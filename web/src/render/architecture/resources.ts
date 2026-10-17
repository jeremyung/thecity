import * as THREE from 'three/webgpu';
import {apartmentTextureVersions} from '../generated/apartment-textures';
import {attribute,positionWorld,normalWorld,triplanarTexture,texture,float,vec3,color,mx_noise_float,bumpMap,mix,smoothstep,fract,fwidth} from 'three/tsl';
import {ArtMaterials} from '../materials';
import {APARTMENT_RECIPES} from './recipes';
import {ARCHITECTURE_SURFACES,ARCHITECTURE_TEXTURES} from './surface-catalog';

export const APARTMENT_SURFACES={wall:[...new Set(Object.values(APARTMENT_RECIPES).flatMap(r=>r.wallTints))],concrete:[...new Set(Object.values(APARTMENT_RECIPES).map(r=>r.concreteTint))],glass:['#59717a','#40565f','#7c8c8b','#526c73']} as const;
/** One owner per renderer/session, never per building. Its palette avoids changing legacy materials. */
export class ApartmentResources {
 readonly art=new ArtMaterials();
 private disposed=false;
 private textures:THREE.Texture[]=[];
 private loading:Promise<boolean>|null=null;
 private batches=new Map<string,THREE.Material>();
 constructor(){
  const get=this.art.get.bind(this.art),shared=this.art.shared.bind(this.art);
  this.art.get=(...args)=>this.mark(get(...args));
  this.art.shared=(...args)=>this.mark(shared(...args));
  const registered=new Set<string>();
  for(const binding of ARCHITECTURE_SURFACES){
   const {hex,role,roughness}=binding,key=`${hex}/${roughness}`;if(registered.has(key))continue;registered.add(key);
   const material=this.art.get(hex,roughness);
   for(const style of ['anime','miniature'] as const){
    const surface=this.art.variant(material,style) as THREE.MeshStandardNodeMaterial;surface.userData.apartmentSurface=true;surface.userData.apartmentRole=role;surface.userData.apartmentTexture=binding.texture;
    if(role==='glass'){
     surface.transparent=true;surface.opacity=.72;surface.depthWrite=false;
     if(style==='miniature'){const physical=surface as THREE.MeshStandardNodeMaterial;physical.envMapIntensity=.8;physical.roughness=.24;}
    }else{
     // World-space relief survives transformed/merged city batches; no per-building maps.
     const grain=mx_noise_float(positionWorld.mul(role==='plaster'?45:28));
     const wash=mx_noise_float(positionWorld.mul(vec3(1.5,.13,1.5)));
     const aging=attribute<'vec2'>('architectureWear','vec2'),wear=aging.x,phase=aging.y;
     // Domain-warped, nonperiodic fields: no floor-height or repeated panel masks.
     const p=positionWorld.add(vec3(phase,0,phase.mul(.37)));
     const warp=vec3(mx_noise_float(p.mul(.19)),mx_noise_float(p.mul(.23).add(17)),mx_noise_float(p.mul(.17).add(43))).mul(.85);
     const q=p.add(warp);
     const patch=smoothstep(.02,.28,mx_noise_float(q.mul(vec3(.38,.23,.38))));
     const vertical=smoothstep(.08,.31,mx_noise_float(q.mul(vec3(2.7,.11,2.7))));
     const breakup=smoothstep(-.1,.24,mx_noise_float(q.mul(vec3(.8,.65,.8))));
     const runoff=vertical.mul(breakup);
     const splash=smoothstep(1.2,.08,positionWorld.y).mul(patch);
     const materialFactor=surface.userData.apartmentRole==='brick'?.48:surface.userData.apartmentRole==='tile'?.35:1;
     const patina=float(1).sub(patch.mul(.09).add(runoff.mul(.12)).add(splash.mul(.16)).mul(wear).mul(materialFactor));
     const original=surface.colorNode;
     surface.colorNode=vec3(original??color(surface.color)).mul(wash.mul(.035).add(.965)).mul(patina);
     if(style==='miniature'){
      const physical=surface as THREE.MeshStandardNodeMaterial;
      physical.normalNode=bumpMap(grain,float(role==='metal'?.003:role==='plaster'?.012:.016));
      physical.roughnessNode=mix(float(roughness).add(grain.mul(.035)).add(wear.mul(.06)),float(Math.max(.18,roughness-.19)),this.art.wet);
     }
    }
   }
  }
  for(const recipe of Object.values(APARTMENT_RECIPES))if(recipe.finish)for(const hex of recipe.wallTints){
   const material=this.art.get(hex,recipe.wallRoughness??.85),brick=recipe.finish==='brick';
   for(const style of ['anime','miniature'] as const){
    const surface=this.art.variant(material,style) as THREE.MeshStandardNodeMaterial,p=positionWorld;
    const rows=p.y.div(brick?.075:.15),stagger=brick?rows.floor().mod(2).mul(.5):float(0);
    const u=fract(mix(p.x,p.z,normalWorld.x.abs().greaterThan(normalWorld.z.abs()).select(1,0)).div(brick?.24:.15).add(stagger)),v=fract(rows);
    const seam=smoothstep(.025,.075,u).mul(smoothstep(.025,.075,v));
    const fade=smoothstep(.2,.7,fwidth(rows)).oneMinus();
    surface.colorNode=vec3(surface.colorNode??color(surface.color)).mul(mix(float(1),seam.mul(.15).add(.85),fade));
    if(style==='miniature')(surface as THREE.MeshStandardNodeMaterial).normalNode=bumpMap(seam.mul(fade),float(.012));
   }
  }
 }
 private mark<T extends THREE.Material>(material:T):T{for(const style of ['anime','miniature'] as const)this.art.variant(material,style).userData.architectureOwner=true;return material;}
 owns(material:THREE.Material){return material.userData.architectureOwner===true;}
 variant(material:THREE.Material,style?:'anime'|'miniature'){return this.art.variant(material,style);}

 /** Shared surface shaders use exact per-building vertex tints, preserving relief and glass. */
 batch(material:THREE.Material){
  if(!material.userData.apartmentSurface){const result=this.art.buildingBatch(material);this.mark(result.material);return result;}
  const source=material as THREE.MeshStandardNodeMaterial,role=String(source.userData.apartmentRole),key=`apartment-batch-${role}-${source.userData.apartmentTexture??'plain'}`;
  if(!this.batches.has(key)){
   this.batches.set(key,source);
   this.art.shared(key,()=>({anime:this.batchVariant(source,'anime'),miniature:this.batchVariant(source,'miniature')}));
  }
  const shader=this.art.shared(key,()=>{throw Error('Missing architecture shader');});
  return {material:shader,color:source.color};
 }
 private batchVariant(source:THREE.Material,style:'anime'|'miniature'){
  const original=this.art.variant(source,style) as THREE.MeshStandardNodeMaterial,clone=original.clone();
  clone.colorNode=vec3(original.colorNode??color(original.color)).div(vec3(color(original.color)).max(vec3(.001)));
  clone.vertexColors=true;return clone;
 }
 private refreshBatches(){
  for(const [key,source] of this.batches)for(const style of ['anime','miniature'] as const){
   const shader=this.art.variant(this.art.shared(key,()=>{throw Error('Missing architecture shader');}),style) as THREE.MeshStandardNodeMaterial;
   const original=this.art.variant(source,style) as THREE.MeshStandardNodeMaterial;
   shader.colorNode=vec3(original.colorNode??color(original.color)).div(vec3(color(original.color)).max(vec3(.001)));shader.needsUpdate=true;
  }
 }

 loadTextures(baseURL='/art/apartment-pilot/'):Promise<boolean>{
  if(this.disposed)return Promise.resolve(false);
  return this.loading??=this.load(baseURL);
 }
 private async load(base:string):Promise<boolean>{
  const loader=new THREE.TextureLoader();
  const results=await Promise.allSettled(ARCHITECTURE_TEXTURES.map(async name=>({name,map:await loader.loadAsync(`${base}${name}.png?v=${apartmentTextureVersions[name]}`)})));
  const loaded=results.flatMap(r=>r.status==='fulfilled'?[r.value]:[]);
  if(this.disposed){loaded.forEach(({map})=>map.dispose());return false;}
  this.textures.push(...loaded.map(({map})=>map));
  for(const {name,map} of loaded){
   map.colorSpace=THREE.SRGBColorSpace;map.wrapS=map.wrapT=THREE.RepeatWrapping;map.anisotropy=4;
   const updated=new Set<string>();
   for(const binding of ARCHITECTURE_SURFACES){
    if(binding.texture!==name)continue;
    const key=`${binding.hex}/${binding.roughness}`;if(updated.has(key))continue;updated.add(key);
    const material=this.art.get(binding.hex,binding.roughness);
    for(const style of ['anime','miniature'] as const){
     const variant=this.art.variant(material,style) as THREE.MeshStandardNodeMaterial;
     if(variant.userData.apartmentTexture!==name)continue;
     const sample=triplanarTexture(texture(map),null,null,float(2.5),positionWorld,normalWorld).rgb;
     // Palette supplies macro colour; generated maps supply restrained material grain.
     variant.colorNode=vec3(variant.colorNode??color(variant.color)).mul(sample.mul(.32).add(.84));
     variant.needsUpdate=true;
    }
   }
  }
  this.refreshBatches();return loaded.length===ARCHITECTURE_TEXTURES.length;
 }
 dispose(){if(this.disposed)return;this.disposed=true;this.textures.forEach(t=>t.dispose());this.textures=[];this.batches.clear();this.art.dispose();}
}
