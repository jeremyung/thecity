import * as THREE from 'three/webgpu';
import { color, mix, uniform, positionWorld, positionLocal, positionView, vec2, vec3, sin, reflector, texture, uv, smoothstep, mx_noise_float, screenUV, viewportSafeUV, viewportSharedTexture, viewportDepthTexture, perspectiveDepthToViewZ, attribute, cameraNear, cameraFar, cameraPosition, normalize, dot, max, pow, float } from 'three/tsl';
import type { Point, Style } from '../types';
import { createWaterNormalTexture } from './water-normals';
import { waterFlowPhase, WATER_FLOW_CYCLE } from './water-flow';

/**
 * River surface after three.js Water2Mesh: two scrolling ripple normal layers
 * distort both the planar reflection and the refracted riverbed, which is
 * tinted by the water thickness behind each pixel. Fresnel blends the two, so
 * aerial views look into dark water while grazing views approach a mirror.
 * The surface is unlit; a narrow sun lobe supplies glints instead.
 *
 * Earlier rejected trials (validation/water-*.md) kept an undistorted, lit
 * reflection at a fixed 24-45% share and had nothing beneath the surface.
 */
export class RiverWater {
  readonly mesh: THREE.Mesh;
  readonly time=uniform(0);
  readonly night=uniform(0);
  readonly style=uniform(0);
  private sunDirection=uniform(new THREE.Vector3(0,1,0));
  private sunColor=uniform(new THREE.Color('#fff1d9'));
  private grid=new Float32Array(64*64);
  private previous=new Float32Array(64*64);
  private next=new Float32Array(64*64);
  private pixels=new Float32Array(64*64*4);
  private ripple:THREE.DataTexture;
  private normals:THREE.DataTexture;
  private reflection:ReturnType<typeof reflector>;
  private elapsed=0;
  private flowPhase=uniform(new THREE.Vector3(0,.5,1));
  private surface:THREE.MeshBasicNodeMaterial;
  private bed:THREE.Mesh;
  private reflectionReview:THREE.MeshBasicNodeMaterial|null=null;
  private random=123456789;
  constructor(points:Point[],halfWidth=32) {
    this.mesh=new THREE.Mesh(riverStrip(points,halfWidth,6,()=>-.7),this.surface=new THREE.MeshBasicNodeMaterial({side:THREE.DoubleSide,transparent:true}));
    // A shallow bowl below the surface gives refraction and depth tint
    // something real to show: pale shallows at the walls, darker mid-channel.
    // It stays above the flat backdrop at y=-3 that underlies the basin.
    const bed=new THREE.MeshStandardNodeMaterial({roughness:1});
    bed.colorNode=mix(color('#6f6a52'),color('#4d5545'),mx_noise_float(positionLocal.mul(.08)).mul(.5).add(.5));
    this.bed=new THREE.Mesh(riverStrip(points,halfWidth,10,(across)=>-1.2-1.6*Math.pow(1-across*across,.7)),bed);
    this.bed.receiveShadow=true;this.mesh.add(this.bed);

    this.ripple=new THREE.DataTexture(this.pixels,64,64,THREE.RGBAFormat,THREE.FloatType);this.ripple.minFilter=THREE.LinearFilter;this.ripple.magFilter=THREE.LinearFilter;this.ripple.needsUpdate=true;
    this.normals=createWaterNormalTexture();
    const mat=this.surface,t=this.time,w=positionWorld;
    const wave=sin(positionLocal.z.mul(.16).sub(t.mul(.65))).mul(.08).add(sin(positionLocal.x.mul(.25).add(t.mul(.3))).mul(.055));
    // River coordinates follow each seeded bend; world-X sampling clamped most
    // of an eastern river to one texel, making its simulated rain invisible.
    const rain=texture(this.ripple,uv());
    mat.positionNode=positionLocal.add(vec3(0,wave.add(rain.r.mul(.06)),0));

    // Flow map (Water2 style): each ripple layer is advected along the local
    // current for one cycle, sampled twice half a cycle apart, and the pair is
    // cross-faded so the texture never stretches. The current follows bends
    // and is slowest along the walls (per-vertex 'flow', metres/second).
    const ground=vec2(w.x,w.z),current=vec2(attribute('flow','vec2'));
    const cycle=float(WATER_FLOW_CYCLE),phase0=this.flowPhase.x,phase1=this.flowPhase.y;
    const blend=this.flowPhase.z;
    const advected=(scale:number,rate:number,shift:number)=>{
      const base=ground.div(scale);
      const a=texture(this.normals,base.sub(current.mul(phase0.mul(cycle).mul(rate/scale))));
      const b=texture(this.normals,base.add(shift).sub(current.mul(phase1.mul(cycle).mul(rate/scale))));
      // The wrapping sample must be invisible at each half-cycle boundary.
      return mix(a,b,blend).rgb.mul(2).sub(1);
    };
    const layerA=advected(38,1,.37);
    const layerB=advected(13,1.35,.61);
    // A calm urban river: gentle slopes, not open-water chop.
    const strength=mix(float(.38),float(.3),this.style);
    const normal=normalize(vec3(
      layerA.x.add(layerB.x.mul(.7)).mul(strength).add(rain.g.mul(.14)),
      layerA.z.add(layerB.z),
      layerA.y.add(layerB.y.mul(.7)).mul(strength).add(rain.b.mul(.14)),
    ));
    const toEye=normalize(cameraPosition.sub(w));
    const facing=max(dot(toEye,normal),0);
    const f0=mix(float(.035),float(.025),this.style);
    const fresnel=f0.add(f0.oneMinus().mul(pow(facing.oneMinus(),5)));

    const reflection=this.reflection=reflector({resolutionScale:.35,bounces:false,samples:0});
    reflection.uvNode=reflection.uvNode!.add(normal.xz.mul(.035));
    reflection.target.rotation.x=-Math.PI/2;reflection.target.position.y=-.7;this.mesh.add(reflection.target);

    const refractUv=viewportSafeUV(screenUV.add(normal.xz.mul(.018)));
    const behind=viewportSharedTexture(refractUv).rgb;
    const sceneZ=perspectiveDepthToViewZ(viewportDepthTexture(refractUv).x,cameraNear,cameraFar);
    const thickness=positionView.z.sub(sceneZ).max(0);
    // Red is absorbed first: shallows keep the bed's colour, depth turns teal.
    const transmitted=vec3(thickness.mul(-.95).exp(),thickness.mul(-.42).exp(),thickness.mul(-.32).exp());
    const scatter=mix(color('#24504f'),color('#07131c'),this.night);
    const body=behind.mul(transmitted).add(scatter.mul(transmitted.oneMinus()));
    // A broken lip of foam where the surface meets walls and piers.
    const lip=smoothstep(.45,.05,thickness).mul(smoothstep(-.2,.35,mx_noise_float(vec3(w.x.mul(.7),w.z.mul(.7),t.mul(.4)))));
    const surface=mix(body,mix(color('#d9e2dc'),color('#3d5563'),this.night),lip.mul(.45));
    const glint=pow(max(dot(normal,normalize(this.sunDirection.add(toEye))),0),mix(float(1600),float(2200),this.style)).mul(3.5);
    mat.colorNode=mix(surface,reflection.rgb,fresnel.mul(lip.mul(.6).oneMinus())).add(this.sunColor.mul(glint));
    this.mesh.receiveShadow=false;
  }
  /** Key light direction (toward the sun, world space) and colour·intensity. */
  setSun(direction:THREE.Vector3,light:THREE.Color){this.sunDirection.value.copy(direction).normalize();this.sunColor.value.copy(light);}
  /** Development probe: inspect reflected radiance before surface blending. */
  toggleReflectionForReview():boolean {
    if(this.mesh.material!==this.surface){this.mesh.material=this.surface;return false;}
    if(!this.reflectionReview){
      this.reflectionReview=new THREE.MeshBasicNodeMaterial({side:THREE.DoubleSide});
      this.reflectionReview.colorNode=this.reflection.rgb;
      this.reflectionReview.positionNode=this.surface.positionNode;
    }
    this.mesh.material=this.reflectionReview;return true;
  }
  setReflectionScale(scale:number){this.reflection.reflector.resolutionScale=THREE.MathUtils.clamp(scale,.15,.5);}
  update(dt:number,t:number,rain:number,style:Style,night:number){
    waterFlowPhase(t,this.flowPhase.value);
    this.time.value=t;this.style.value=style==='miniature'?1:0;this.night.value=night;
    this.elapsed+=dt;
    if(this.elapsed<.05)return;this.elapsed=0;
    const rand=()=>{this.random^=this.random<<13;this.random^=this.random>>>17;this.random^=this.random<<5;return(this.random>>>0)/4294967296;};
    if(rain>.05)for(let i=0;i<rain*12;i++){const x=1+Math.floor(rand()*62),y=1+Math.floor(rand()*62);this.grid[y*64+x]+=.2;}
    for(let y=1;y<63;y++)for(let x=1;x<63;x++){
      const n=y*64+x;this.next[n]=((this.grid[n-1]+this.grid[n+1]+this.grid[n-64]+this.grid[n+64])*.5-this.previous[n])*.98;
      this.pixels[n*4]=this.next[n];this.pixels[n*4+1]=this.grid[n+1]-this.grid[n-1];this.pixels[n*4+2]=this.grid[n+64]-this.grid[n-64];this.pixels[n*4+3]=1;
    }
    const tmp=this.previous;this.previous=this.grid;this.grid=this.next;this.next=tmp;this.ripple.needsUpdate=true;
  }
  dispose(){this.reflection.dispose();this.mesh.geometry.dispose();this.surface.dispose();this.bed.geometry.dispose();(this.bed.material as THREE.Material).dispose();this.reflectionReview?.dispose();this.ripple.dispose();this.normals.dispose();this.mesh.removeFromParent();}
}

/** Strip following the river centreline; `height(across)` gets -1..1 bank to bank. */
function riverStrip(points:Point[],halfWidth:number,acrossSegments:number,height:(across:number)=>number){
  const vertices:number[]=[],uvs:number[]=[],flows:number[]=[],indices:number[]=[];
  const columns=acrossSegments+1;
  for(let i=0;i<points.length-1;i++){
    const a=points[i],b=points[i+1],offset=vertices.length/3;
    const length=Math.hypot(b.x-a.x,b.z-a.z)||1,dx=(b.x-a.x)/length,dz=(b.z-a.z)/length;
    for(let j=0;j<=32;j++){
      const t=j/32,z=THREE.MathUtils.lerp(a.z,b.z,t),x=THREE.MathUtils.lerp(a.x,b.x,t);
      for(let k=0;k<columns;k++){const across=k/acrossSegments*2-1;vertices.push(x+across*halfWidth,height(across),z);uvs.push((across+1)/2,(z+400)/800);
        // ~0.6 m/s mid-channel, a quarter of that against the walls.
        const speed=.15+.45*(1-across*across);flows.push(dx*speed,dz*speed);}
    }
    for(let j=0;j<32;j++)for(let k=0;k<acrossSegments;k++){const n=offset+j*columns+k;indices.push(n,n+columns,n+1,n+1,n+columns,n+columns+1);}
  }
  const geo=new THREE.BufferGeometry();geo.setAttribute('position',new THREE.Float32BufferAttribute(vertices,3));geo.setAttribute('uv',new THREE.Float32BufferAttribute(uvs,2));geo.setAttribute('flow',new THREE.Float32BufferAttribute(flows,2));geo.setIndex(indices);geo.computeVertexNormals();
  return geo;
}
