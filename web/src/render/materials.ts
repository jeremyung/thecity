import * as THREE from 'three/webgpu';
import { float, floor, fwidth, color, materialColor, positionWorld, positionView, smoothstep, positionLocal, instanceIndex, vec3, sin, mx_noise_float, uniform, mix, vertexColor, normalWorld, cameraPosition, reflect, texture, varyingProperty } from 'three/tsl';
import type { Style } from '../types';
import type { BuildingBatchSurface } from './building-chunks';
import { paintedSurfaceTint } from './painted-surface';
import { claddingShade, claddingShadeBatched, windowMaterials, type Cladding, type WindowActivity } from './building-surfaces';
type Surface=THREE.MeshStandardNodeMaterial|THREE.MeshToonNodeMaterial;
type BuildingArtClass={key:string,roughness:number,emissive:boolean,painted:boolean,activity:'home'|'office'|'shop'|'ambient',cladding?:Cladding};
type BuildingSurfaceVariants={anime:Surface,miniature:Surface};

/** Anime keeps cladding lighter so its painted surface leads; Miniature shows it fully. */
const ANIME_CLADDING=.55;

export class ArtMaterials {
  readonly contours=new THREE.LineBasicNodeMaterial({color:'#283e4c',transparent:true,opacity:.3,depthWrite:false});
  readonly night = uniform(0);
  readonly wet = uniform(0);
  readonly time=uniform(0);
  readonly litHome=uniform(1);
  readonly litOffice=uniform(1);
  readonly litShop=uniform(1);
  /** Toward the sun, world space; set each frame with the key light. */
  readonly sunDirection=uniform(new THREE.Vector3(0,1,0));
  /** 0 disables cloud shadows (night, overcast); ~.4 on a clear day. */
  readonly cloudShadow=uniform(0);
  /** Share of rooms with a light on, per window schedule; set from the time of day. */
  readonly occupancy={home:uniform(.5),office:uniform(.5),shop:uniform(.5)} as const;
  readonly palette = ['#d8d4c4', '#c2c5ba', '#b8beb8', '#e2d9c6', '#b7ad9f', '#c9c8c1'];
  private style:Style='anime';
  private cache = new Map<string,{anime:Surface,miniature:Surface}>();
  private buildingCache = new Map<string,BuildingSurfaceVariants>();
  private bands:THREE.DataTexture;
  private tenantAtlas:THREE.CanvasTexture|null=null;
  private storefrontAtlas:THREE.CanvasTexture|null=null;
  private leafMask:THREE.CanvasTexture|null=null;
  private shopSignAtlas:THREE.CanvasTexture|null=null;
  private shopSignMaterial:THREE.MeshBasicNodeMaterial|null=null;
  constructor(){
    this.contours.opacityNode=smoothstep(100,420,positionView.z.negate()).mul(-1).add(1).mul(.27);
    this.bands=new THREE.DataTexture(new Uint8Array([95,145,195,230,255]),5,1,THREE.RedFormat);
    this.bands.minFilter=THREE.NearestFilter;this.bands.magFilter=THREE.NearestFilter;this.bands.needsUpdate=true;
  }
  /**
   * Drifting cloud shadows as an albedo factor on sun-facing surfaces. Faces
   * turned from the sun keep their ambient look, which approximates removing
   * only the direct term without a per-light cookie.
   */
  cloudShade(){
    const drift=vec3(this.time.mul(3.2),0,this.time.mul(1.3));
    const p=positionWorld.add(drift).mul(vec3(.0045,0,.0045));
    // MaterialX noise rarely leaves ±.35; this band covers roughly 40% of the ground.
    const cover=smoothstep(-.02,.2,mx_noise_float(p).mul(.72).add(mx_noise_float(p.mul(2.9)).mul(.28)));
    const sunlit=normalWorld.dot(this.sunDirection).max(0).sqrt();
    return float(1).sub(cover.mul(sunlit).mul(this.cloudShadow));
  }
  get(hex: string, roughness = 0.85, emissive = false, activity:'home'|'office'|'shop'|'ambient'='ambient', cladding:Cladding='plaster'):Surface {
    const key = `${hex}/${roughness}/${emissive}/${emissive?activity:'ambient'}${cladding==='plaster'?'':`/${cladding}`}`;
    let variants=this.cache.get(key);
    if(!variants){
      const anime=new THREE.MeshToonNodeMaterial({color:hex,gradientMap:this.bands});
      const miniature=new THREE.MeshStandardNodeMaterial({color:hex,roughness,metalness:roughness<.4?.18:0});
      const brush=mx_noise_float(positionWorld.mul(.65)).mul(.055).add(.985);
      const pigment=mx_noise_float(positionWorld.mul(.05)).mul(.08).add(.97);
      anime.colorNode=color(hex).mul(brush.mul(pigment));
      const painted=roughness>=.65&&!emissive&&hex!=='#fffffe'&&hex!=='#ffffff';
      if(painted)anime.colorNode=vec3(color(hex)).mul(paintedSurfaceTint());
      miniature.colorNode=color(hex).mul(brush.mul(.3).add(.7));
      const clad=painted&&cladding!=='plaster'?cladding:undefined;
      if(clad){anime.colorNode=anime.colorNode.mul(claddingShade(clad,ANIME_CLADDING));miniature.colorNode=miniature.colorNode.mul(claddingShade(clad,1));}
      if(roughness<.4&&!emissive){
        const grazing=normalWorld.dot(cameraPosition.sub(positionWorld).normalize()).abs().oneMinus().pow(3);
        const sheen=grazing.mul(.6).add(.08).mul(this.night.oneMinus()).add(1);
        anime.colorNode=anime.colorNode.mul(sheen);miniature.colorNode=miniature.colorNode.mul(sheen);
        this.applyGlassSky(anime,miniature);
      }
      const shade=this.cloudShade();anime.colorNode=anime.colorNode.mul(shade);miniature.colorNode=miniature.colorNode.mul(shade);
      miniature.roughnessNode=mix(roughness,Math.max(.1,roughness*.67),this.wet);
      if(hex==='#fffffe'){
        anime.vertexColors=true;miniature.vertexColors=true;
        this.leafMask??=foliageMask();
        for(const material of [anime,miniature]){material.alphaMap=this.leafMask;material.alphaTest=.38;material.alphaToCoverage=true;material.side=THREE.DoubleSide;}
        // Standard shading preserves the miniature's material response, but
        // shadowed cards can collapse to black. Keep a small, night-aware fill
        // and multiply by the same per-tree tint as the diffuse pass; without
        // that tint, blossom and foliage fill would both skew toward pale gray.
        const treeTint=varyingProperty('vec3','vInstanceColor');
        const canopyFill=vertexColor().mul(treeTint).mul(this.night.oneMinus().mul(.11).add(.025));
        (miniature as unknown as {emissiveNode:typeof canopyFill}).emissiveNode=canopyFill;
        const sway=sin(this.time.mul(.55).add(instanceIndex.toFloat().mul(.73))).mul(positionLocal.y.sub(2).max(0)).mul(.035);
        anime.positionNode=positionLocal.add(vec3(sway,0,sway.mul(.35)));miniature.positionNode=anime.positionNode;
      }
      if(hex==='#ffffff'){
        const sway=sin(this.time.mul(.7).add(instanceIndex.toFloat().mul(.73))).mul(positionLocal.y.max(0)).mul(.1);
        const position=positionLocal.add(vec3(sway,0,sway.mul(.4)));
        anime.positionNode=position;miniature.positionNode=position;
      }
      for(const mat of [anime,miniature]){
        mat.userData.artKey=key;
        // Every clad wall shares one batch class; each vertex carries its cladding kind.
        mat.userData.artBuildingClass={key:`${roughness}/${emissive}/${emissive?activity:'ambient'}/${painted}${clad?'/clad':''}`,roughness,emissive,painted,activity,...(clad?{cladding:clad}:{})} satisfies BuildingArtClass;
        if(emissive){const use=activity==='home'?this.litHome:activity==='office'?this.litOffice:activity==='shop'?this.litShop:1;const emission=color(hex).mul(this.night.mul(1.4).add(.12)).mul(use);(mat as unknown as {emissiveNode:typeof emission}).emissiveNode=emission;}
      }
      variants={anime,miniature};this.cache.set(key,variants);
    }
    return variants[this.style];
  }
  /** One atlas/material pair for every vertical tenant sign, shared across chunks. */
  tenantSign():Surface {
    const key='tenant-sign-atlas';
    let variants=this.cache.get(key);
    if(!variants){
      this.tenantAtlas=canvasTexture(128,4096,ctx=>{
        const labels=['青葉商会','中央書房','東都写真','山田時計','光文印刷','喫茶白鳥','銀河音楽','朝日旅館'];
        const backgrounds=['#b94336','#ede4c9','#326779','#e4dbbf','#315a60','#b54b37','#3b596e','#eee4cb'];
        for(let slot=0;slot<8;slot++){
          const y=slot*512;
          ctx.fillStyle=backgrounds[slot];ctx.fillRect(0,y,128,512);
          ctx.strokeStyle=slot===1||slot===3||slot===7?'#8f856e':'#e8d9b7';
          ctx.lineWidth=2;ctx.strokeRect(7,y+7,114,498);
          ctx.fillStyle=slot===1||slot===3||slot===7?'#263e43':'#fff0ca';
          ctx.font='600 76px "Hiragino Kaku Gothic ProN", sans-serif';
          ctx.textAlign='center';ctx.textBaseline='middle';
          [...labels[slot]].forEach((character,index)=>ctx.fillText(character,64,y+82+index*115));
        }
      });
      const anime=new THREE.MeshToonNodeMaterial({map:this.tenantAtlas,gradientMap:this.bands});
      const miniature=new THREE.MeshStandardNodeMaterial({map:this.tenantAtlas,roughness:.55});
      for(const material of [anime,miniature]){
        material.userData.artKey=key;
        const emission=texture(this.tenantAtlas).rgb.mul(this.night).mul(this.litShop).mul(.65);
        (material as unknown as {emissiveNode:typeof emission}).emissiveNode=emission;
      }
      variants={anime,miniature};this.cache.set(key,variants);
    }
    return variants[this.style];
  }
  /** Four quiet window notices share one atlas across all storefronts and styles. */
  storefrontPoster():Surface {
    const key='storefront-poster-atlas';let variants=this.cache.get(key);
    if(!variants){
      this.storefrontAtlas=canvasTexture(512,512,ctx=>{
        const headings=['珈琲','喫茶','新入荷','お知らせ'];
        const details=['自家焙煎','モーニング','季節の品','ご案内'];
        for(let slot=0;slot<4;slot++){
          const x=(slot%2)*256,y=Math.floor(slot/2)*256;
          ctx.fillStyle=['#e4d5af','#d8ddd0','#d6c8a7','#e5dcc8'][slot];ctx.fillRect(x,y,256,256);
          ctx.fillStyle=slot===2?'#8e4437':'#354b46';ctx.fillRect(x+18,y+18,220,7);
          ctx.textAlign='center';ctx.textBaseline='middle';ctx.font='600 38px "Hiragino Mincho ProN",serif';ctx.fillText(headings[slot],x+128,y+62);
          ctx.font='22px "Hiragino Kaku Gothic ProN",sans-serif';ctx.fillText(details[slot],x+128,y+108);
          if(slot<2){
            ctx.fillStyle='#826650';ctx.beginPath();ctx.ellipse(x+120,y+161,35,17,0,0,Math.PI*2);ctx.fill();
            ctx.fillStyle='#f1e5c9';ctx.fillRect(x+85,y+157,70,30);
            ctx.strokeStyle='#f1e5c9';ctx.lineWidth=7;ctx.strokeRect(x+154,y+161,18,15);
            ctx.fillStyle='#354b46';ctx.font='19px serif';ctx.fillText(slot===0?'COFFEE':'モーニング',x+128,y+224);
          }else{
            for(let row=0;row<4;row++)ctx.fillRect(x+44,y+145+row*18,168-(row%2)*30,3);
            ctx.font='18px serif';ctx.fillText(slot===2?'暮らしの道具':'店内禁煙',x+128,y+230);
          }
        }
      });
      const anime=new THREE.MeshToonNodeMaterial({map:this.storefrontAtlas,gradientMap:this.bands});
      const miniature=new THREE.MeshStandardNodeMaterial({map:this.storefrontAtlas,roughness:.9});
      for(const material of [anime,miniature]){
        material.userData.artKey=key;
        // Same diffuse expression as NodeMaterial's default, explicitly live:
        // r183's static observer misses shadow texture replacement on otherwise
        // unchanged atlas materials shared by many objects and render passes.
        material.colorNode=materialColor as unknown as NonNullable<Surface['colorNode']>;
      }
      variants={anime,miniature};this.cache.set(key,variants);
    }
    return variants[this.style];
  }
  /**
   * One opaque atlas/material for every storefront sign. A material per sign
   * kept each one out of the neighborhood batches: ~600 extra draws per pass.
   * Returns the shared material and the atlas cell's v range for `label`.
   */
  shopSign(label:typeof SHOP_SIGN_LABELS[number]):{material:THREE.MeshBasicNodeMaterial,v0:number,v1:number}{
    const slot=SHOP_SIGN_LABELS.indexOf(label);
    if(slot<0)throw new Error(`Unknown shop sign: ${label}`);
    if(!this.shopSignMaterial){
      this.shopSignAtlas=canvasTexture(SIGN_WIDTH,SIGN_ATLAS_HEIGHT,ctx=>{
        // Background fills the padding too, so mip levels only blend in sign paper.
        ctx.fillStyle=SIGN_PAPER;ctx.fillRect(0,0,SIGN_WIDTH,SIGN_ATLAS_HEIGHT);
        SHOP_SIGN_LABELS.forEach((text,slot)=>drawSign(ctx,text,SIGN_PAPER,SIGN_INK,shopSignCell(slot).top));
      });
      this.shopSignMaterial=new THREE.MeshBasicNodeMaterial({map:this.shopSignAtlas,side:THREE.DoubleSide});
      this.shopSignMaterial.userData.artKey='shop-sign-atlas';
    }
    const{v0,v1}=shopSignCell(slot);
    return{material:this.shopSignMaterial,v0,v1};
  }
  /** Window panes with rooms behind them, one shared pair per light schedule. */
  window(activity:WindowActivity):Surface{
    return this.shared(`window/${activity}`,()=>windowMaterials(activity,{night:this.night,occupancy:this.occupancy},this.bands) as unknown as {anime:Surface,miniature:Surface});
  }
  setStyle(style:Style){this.style=style;}
  private disposers:Array<()=>void>=[];
  /** Resources owned by `shared` materials (textures) released with the art set. */
  onDispose(release:()=>void){this.disposers.push(release);}
  /** Resolve a source building surface to its shared shader class and exact linear base color. */
  buildingBatch(material:THREE.Material):BuildingBatchSurface{
    // Panes read their schedule from vertex data, so all of them share one batch.
    if(material.userData.windowPane)return{material:this.window('home')};
    const artClass=material.userData.artBuildingClass as BuildingArtClass|undefined;
    const baseColor=(material as THREE.Material&{color?:THREE.Color}).color;
    if(!artClass||!baseColor)return{material:this.variant(material)};
    return{material:this.buildingVariants(artClass)[this.style],color:baseColor};
  }
  variant(material:THREE.Material,style:Style=this.style){
    const artClass=material.userData.artBatchMaterial?material.userData.artBuildingClass as BuildingArtClass|undefined:undefined;
    if(artClass)return this.buildingVariants(artClass)[style];
    const key=material.userData.artKey as string|undefined;
    return key?this.cache.get(key)?.[style]??material:material;
  }
  dispose(){this.disposers.splice(0).forEach(release=>release());for(const pair of this.cache.values()){pair.anime.dispose();pair.miniature.dispose();}for(const pair of this.buildingCache.values()){pair.anime.dispose();pair.miniature.dispose();}this.cache.clear();this.buildingCache.clear();this.bands.dispose();this.leafMask?.dispose();this.tenantAtlas?.dispose();this.storefrontAtlas?.dispose();this.shopSignMaterial?.dispose();this.shopSignAtlas?.dispose();this.shopSignMaterial=null;this.shopSignAtlas=null;this.contours.dispose();}

  /** Toon ramp shared by every Anime surface. */
  get toonRamp(){return this.bands;}
  /**
   * Cache a style pair under `key` so style switches swap it like any other
   * art material. `create` runs once; later calls return the current style.
   */
  shared(key:string,create:()=>{anime:Surface,miniature:Surface}):Surface{
    let variants=this.cache.get(key);
    if(!variants){variants=create();variants.anime.userData.artKey=key;variants.miniature.userData.artKey=key;this.cache.set(key,variants);}
    return variants[this.style];
  }
  /** World-space puddle mask shared by the asphalt albedo and the SSR pass. */
  static puddles(x:ReturnType<typeof float>,z:ReturnType<typeof float>){
    return smoothstep(.1,.28,mx_noise_float(vec3(x,0,z).mul(.09)));
  }
  /**
   * Road asphalt. Roads are merged into shared batches without lane UVs, so
   * every detail is world-anchored: aggregate grain (faded once unresolved),
   * rectangular repair patches, hairline cracks and broad wear. Rain darkens
   * the surface and collects in puddles that the wet-weather SSR pass mirrors.
   */
  asphalt():Surface{
    return this.shared('asphalt',()=>{
      const p=vec3(positionWorld.x,0,positionWorld.z);
      const grainP=p.mul(2.6);
      const grainVisible=smoothstep(.4,1.4,fwidth(grainP).length()).oneMinus();
      const grain=mx_noise_float(grainP).mul(.08).mul(grainVisible);
      const patch=smoothstep(.2,.26,mx_noise_float(vec3(floor(p.x.div(5.5)),0,floor(p.z.div(9))).mul(.71)));
      const crackP=p.mul(.32);
      const crackVisible=smoothstep(.05,.2,fwidth(crackP).length()).oneMinus();
      // Hairlines only in scattered stretches, faint: continuous isolines read as a map.
      const crackZone=smoothstep(.15,.3,mx_noise_float(p.mul(.05)));
      const crack=smoothstep(.012,0,mx_noise_float(crackP).abs()).mul(crackVisible).mul(crackZone).mul(.14);
      const wear=mx_noise_float(p.mul(.028)).mul(.07);
      const puddle=ArtMaterials.puddles(positionWorld.x,positionWorld.z).mul(this.wet);
      const wetness=mix(float(1),float(.8),this.wet.mul(.6).add(puddle.mul(.4)));
      const albedo=color('#747f7b').mul(grain.add(wear).add(1)).mul(mix(float(1),float(.87),patch)).mul(crack.oneMinus()).mul(wetness).mul(this.cloudShade());
      const anime=new THREE.MeshToonNodeMaterial({color:'#747f7b',gradientMap:this.bands});
      const miniature=new THREE.MeshStandardNodeMaterial({color:'#747f7b',roughness:.9});
      anime.colorNode=albedo;miniature.colorNode=albedo;
      miniature.roughnessNode=mix(mix(float(.9),float(.5),this.wet),float(.08),puddle);
      return{anime,miniature};
    });
  }
  private applyGlassSky(anime:Surface,miniature:Surface){
    const radiance=this.glassRadiance();
    (anime as unknown as {emissiveNode:typeof radiance}).emissiveNode=radiance.mul(.55);
    (miniature as unknown as {emissiveNode:typeof radiance}).emissiveNode=radiance;
  }
  /** Sky radiance seen in glass: Miniature strength; Anime uses 0.55 of it. */
  glassRadiance(){
    // A bounded analytic sky reflection, not scene reflections or transparent
    // interiors. Shared by source and batched surfaces to preserve appearance.
    const incident=positionWorld.sub(cameraPosition).normalize();
    const reflected=reflect(incident,normalWorld);
    const skyAmount=smoothstep(-.12,.55,reflected.y);
    const fresnel=normalWorld.dot(incident).abs().oneMinus().pow(5).mul(.72).add(.08);
    const daySky=mix(color('#536c73'),color('#c4d8df'),skyAmount);
    const nightSky=mix(color('#090f1c'),color('#23364c'),skyAmount);
    return mix(daySky,nightSky,this.night).mul(fresnel);
  }

  private buildingVariants(artClass:BuildingArtClass):BuildingSurfaceVariants{
    let variants=this.buildingCache.get(artClass.key);if(variants)return variants;
    const anime=new THREE.MeshToonNodeMaterial({color:'#ffffff',gradientMap:this.bands,vertexColors:true});
    const miniature=new THREE.MeshStandardNodeMaterial({color:'#ffffff',roughness:artClass.roughness,metalness:artClass.roughness<.4?.18:0,vertexColors:true});
    const brush=mx_noise_float(positionWorld.mul(.65)).mul(.055).add(.985);
    const pigment=mx_noise_float(positionWorld.mul(.05)).mul(.08).add(.97);
    // NodeMaterial multiplies colorNode by the vertex color attribute. Keeping
    // the original world-space noise nodes preserves both authored styles.
    anime.colorNode=vec3(brush.mul(pigment));
    if(artClass.painted)anime.colorNode=paintedSurfaceTint();
    miniature.colorNode=vec3(brush.mul(.3).add(.7));
    if(artClass.cladding){anime.colorNode=anime.colorNode.mul(claddingShadeBatched(ANIME_CLADDING));miniature.colorNode=miniature.colorNode.mul(claddingShadeBatched(1));}
    if(artClass.roughness<.4&&!artClass.emissive){
      const grazing=normalWorld.dot(cameraPosition.sub(positionWorld).normalize()).abs().oneMinus().pow(3);
      // Batch colorNode is multiplied by the original facade vertex color.
      // Keep the sheen multiplicative so batching preserves the source tint.
      const sheen=grazing.mul(.6).add(.08).mul(this.night.oneMinus()).add(1);
      anime.colorNode=anime.colorNode.mul(sheen);miniature.colorNode=miniature.colorNode.mul(sheen);
      this.applyGlassSky(anime,miniature);
    }
    {const shade=this.cloudShade();anime.colorNode=anime.colorNode.mul(shade);miniature.colorNode=miniature.colorNode.mul(shade);}
    miniature.roughnessNode=mix(artClass.roughness,Math.max(.1,artClass.roughness*.67),this.wet);
    if(artClass.emissive){
      const use=artClass.activity==='home'?this.litHome:artClass.activity==='office'?this.litOffice:artClass.activity==='shop'?this.litShop:1;
      const emission=vertexColor().mul(this.night.mul(1.4).add(.12)).mul(use);
      (anime as unknown as {emissiveNode:typeof emission}).emissiveNode=emission;
      (miniature as unknown as {emissiveNode:typeof emission}).emissiveNode=emission;
    }
    anime.userData.artBuildingClass=artClass;miniature.userData.artBuildingClass=artClass;anime.userData.artBatchMaterial=true;miniature.userData.artBatchMaterial=true;
    variants={anime,miniature};this.buildingCache.set(artClass.key,variants);return variants;
  }
}
/** Fine leaf edges live in a shared coverage mask instead of giant solid lobes. */
function foliageMask(){
  const canvas=document.createElement('canvas');canvas.width=256;canvas.height=256;
  const ctx=canvas.getContext('2d')!;ctx.fillStyle='#000';ctx.fillRect(0,0,256,256);
  let seed=71831;const random=()=>{seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return(seed>>>0)/4294967296;};
  ctx.strokeStyle='#bbb';ctx.lineWidth=2;
  for(let branch=0;branch<9;branch++){
    const angle=branch*2.39996,radius=50+random()*46;
    const x=128+Math.cos(angle)*radius,y=128+Math.sin(angle)*radius*.86;
    ctx.beginPath();ctx.moveTo(128,146);ctx.quadraticCurveTo((x+128)/2,120,x,y);ctx.stroke();
    for(let leaf=0;leaf<27;leaf++){
      const a=random()*Math.PI*2,r=Math.sqrt(random())*32;
      const lx=x+Math.cos(a)*r,ly=y+Math.sin(a)*r*.8;
      ctx.fillStyle=random()>.15?'#fff':'#dedede';
      ctx.beginPath();ctx.ellipse(lx,ly,4+random()*5,2.8+random()*3,a,0,Math.PI*2);ctx.fill();
    }
  }
  const texture=new THREE.CanvasTexture(canvas);texture.colorSpace=THREE.NoColorSpace;texture.anisotropy=4;return texture;
}
export function canvasTexture(width:number,height:number,draw:(ctx:CanvasRenderingContext2D)=>void){
  const canvas=document.createElement('canvas');canvas.width=width;canvas.height=height;draw(canvas.getContext('2d')!);
  const tex=new THREE.CanvasTexture(canvas);tex.colorSpace=THREE.SRGBColorSpace;tex.anisotropy=4;return tex;
}
const SIGN_WIDTH=512,SIGN_HEIGHT=160,SIGN_PADDING=8,SIGN_PAPER='#f2e9ce',SIGN_INK='#314247';
export const SHOP_SIGN_LABELS=['喫茶 青葉','珈琲','商店','喫茶','甘味処','八百屋','酒店','花屋','パン','文具','時計店'] as const;
const CAFE_SIGNS=['珈琲','喫茶','甘味処'] as const;
const SHOP_SIGNS=['商店','八百屋','酒店','花屋','パン','文具','時計店'] as const;
/** Storefront label for a building: the founding anchor keeps its name; others vary by seed. */
export function shopSignLabel(b:{anchor:boolean;kind:string;appearance_seed:number}):typeof SHOP_SIGN_LABELS[number]{
  if(b.anchor)return '喫茶 青葉';
  const labels=b.kind==='cafe'?CAFE_SIGNS:SHOP_SIGNS;
  return labels[((b.appearance_seed>>>11)>>>0)%labels.length];
}
// Cells share 8 px paper gutters: 8 + 11×(160+8) = 1856 rows, padded with
// paper to a power-of-two 2048-row atlas.
export const SIGN_ATLAS_HEIGHT=2048;
/** Canvas rows and texture v range of one atlas cell. CanvasTexture is flipped: canvas row 0 is v=1. */
export function shopSignCell(slot:number){
  const top=SIGN_PADDING+slot*(SIGN_HEIGHT+SIGN_PADDING),bottom=top+SIGN_HEIGHT;
  return{top,bottom,v0:1-bottom/SIGN_ATLAS_HEIGHT,v1:1-top/SIGN_ATLAS_HEIGHT};
}
function drawSign(ctx:CanvasRenderingContext2D,text:string,background:string,ink:string,top=0){
  ctx.fillStyle=background;ctx.fillRect(0,top,SIGN_WIDTH,SIGN_HEIGHT);ctx.fillStyle=ink;ctx.textAlign='center';ctx.textBaseline='middle';ctx.font='500 64px "Hiragino Mincho ProN", serif';ctx.fillText(text,256,top+85);ctx.strokeStyle='#b0a78a';ctx.lineWidth=3;ctx.strokeRect(8,top+8,496,144);
}
export function signTexture(text:string,background=SIGN_PAPER,ink=SIGN_INK){
  return canvasTexture(SIGN_WIDTH,SIGN_HEIGHT,ctx=>drawSign(ctx,text,background,ink));
}
