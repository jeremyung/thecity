import * as THREE from 'three/webgpu';
import { color, mix, uniform, positionWorld, positionLocal, vec2, vec3, sin, reflector, texture, uv, smoothstep, mx_noise_float, transformNormalToView, fwidth } from 'three/tsl';
import type { Point, Style } from '../types';

export class RiverWater {
  readonly mesh: THREE.Mesh;
  readonly time=uniform(0);
  readonly night=uniform(0);
  readonly style=uniform(0);
  private grid=new Float32Array(64*64);
  private previous=new Float32Array(64*64);
  private next=new Float32Array(64*64);
  private pixels=new Float32Array(64*64*4);
  private ripple:THREE.DataTexture;
  private reflection:ReturnType<typeof reflector>;
  private elapsed=0;
  private surface:THREE.MeshStandardNodeMaterial;
  private reflectionReview:THREE.MeshBasicNodeMaterial|null=null;
  private random=123456789;
  constructor(points:Point[],halfWidth=32) {
    const vertices:number[]=[],uvs:number[]=[],indices:number[]=[];
    const width=halfWidth*2;
    for(let i=0;i<points.length-1;i++){
      const a=points[i],b=points[i+1];
      for(let j=0;j<=32;j++){
        const t=j/32,z=THREE.MathUtils.lerp(a.z,b.z,t),x=THREE.MathUtils.lerp(a.x,b.x,t);
        for(const side of [-1,1]){vertices.push(x+side*width/2,-.7,z);uvs.push((side+1)/2,(z+400)/800);}
      }
      const offset=i*66;
      for(let j=0;j<32;j++){const n=offset+j*2;indices.push(n,n+2,n+1,n+1,n+2,n+3);}
    }
    const geo=new THREE.BufferGeometry();geo.setAttribute('position',new THREE.Float32BufferAttribute(vertices,3));geo.setAttribute('uv',new THREE.Float32BufferAttribute(uvs,2));geo.setIndex(indices);geo.computeVertexNormals();
    this.ripple=new THREE.DataTexture(this.pixels,64,64,THREE.RGBAFormat,THREE.FloatType);this.ripple.minFilter=THREE.LinearFilter;this.ripple.magFilter=THREE.LinearFilter;this.ripple.needsUpdate=true;
    const mat=this.surface=new THREE.MeshStandardNodeMaterial({roughness:.25,metalness:0,side:THREE.DoubleSide});
    const t=this.time,w=positionWorld;
    const reflection=this.reflection=reflector({resolutionScale:.35,bounces:false,samples:0});
    // River coordinates follow each seeded bend. World-X sampling clamped most
    // of an eastern river to one texel, making its simulated rain invisible.
    const ripple=texture(this.ripple,uv());
    const wave=sin(positionLocal.z.mul(.16).sub(t.mul(.65))).mul(.08).add(sin(positionLocal.x.mul(.25).add(t.mul(.3))).mul(.055));
    mat.positionNode=positionLocal.add(vec3(0,wave.add(ripple.r.mul(.06)),0));
    const phaseX=w.x.mul(2.8).add(w.z.mul(1.1)).sub(t.mul(1.2));
    const phaseZ=w.z.mul(3.4).sub(w.x.mul(.8)).sub(t.mul(.9));
    // MSAA resolves polygon edges, not undersampled procedural highlights.
    // Attenuate fine detail as its screen footprint grows, transferring
    // unresolved variation to roughness rather than distant shimmer.
    const footprintX=fwidth(phaseX),footprintZ=fwidth(phaseZ);
    const resolvedX=footprintX.mul(footprintX).mul(-.5).exp();
    const resolvedZ=footprintZ.mul(footprintZ).mul(-.5).exp();
    // Independent two-dimensional detail breaks the coherent river-wide sine
    // bands. Both phase footprints attenuate the lower-frequency field;
    // motion is continuous advection, never frame randomness.
    const fineFlow=vec2(phaseX,phaseZ).mul(.35);
    const fineResolved=resolvedX.mul(resolvedZ);
    const smallX=mx_noise_float(fineFlow).mul(.055).mul(fineResolved);
    const smallZ=mx_noise_float(fineFlow.add(vec2(19.7,43.1))).mul(.045).mul(fineResolved);
    // Broad slopes vary across both axes: a single axial sine made the dusk
    // sun reflect as regularly spaced bright bars across the entire river.
    // Continuous drifting noise breaks that alignment without frame-random
    // flicker; the resolved fine waves and simulated rain remain independent.
    const flow=vec3(w.x.mul(.085).add(t.mul(.035)),w.z.mul(.065).sub(t.mul(.045)),0);
    const broadX=mx_noise_float(flow).mul(.12);
    const broadZ=mx_noise_float(flow.add(vec3(19.7,43.1,7.3))).mul(.1);
    mat.normalNode=transformNormalToView(vec3(broadX.add(smallX).add(ripple.g.mul(.14)),1,broadZ.add(smallZ).add(ripple.b.mul(.14))).normalize()).normalize();
    const base=mix(color('#578f94'),color('#1a354a'),this.night);
    const shore=uv().x.sub(.5).abs().mul(2);
    const wash=smoothstep(.89,.99,shore).mul(mx_noise_float(w.mul(vec3(.18,0,.13)).add(vec3(0,0,t.mul(-.05)))).mul(.3).add(.7));
    const reflected=mix(base,reflection.rgb,mix(.24,.45,this.style));
    mat.colorNode=mix(reflected,mix(color('#b7c9b3'),color('#486876'),this.night),wash.mul(.32));
    mat.roughnessNode=mix(.32,.18,this.style).add(resolvedX.mul(resolvedZ).oneMinus().mul(.18));
    mat.emissiveNode=base.mul(.16);
    this.mesh=new THREE.Mesh(geo,mat);this.mesh.receiveShadow=true;
    reflection.target.rotation.x=-Math.PI/2;reflection.target.position.y=-.7;this.mesh.add(reflection.target);
  }
  /** Development probe: inspect reflected radiance before surface lighting. */
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
  dispose(){this.reflection.dispose();this.mesh.geometry.dispose();this.surface.dispose();this.reflectionReview?.dispose();this.ripple.dispose();this.mesh.removeFromParent();}
}
