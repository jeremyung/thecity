import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import * as THREE from 'three/webgpu';
import {ApartmentResources} from '../web/src/render/architecture/resources';
import {BUSINESS_RECIPES,businessPool,selectBusinessRecipe,resolveBusinessRecipes,parseDisplayMode,businessCanHostAdvertising} from '../web/src/render/architecture/business-recipes';
import {buildBusinessBuilding} from '../web/src/render/architecture/business';
import {businessCityStudy} from '../web/src/render/architecture/business-study';
import {STOREFRONT_TENANTS,storefrontCell,storefrontSurface} from '../web/src/render/architecture/business-signs';
import {videoScreenSurface,signageContexts} from '../web/src/render/signage';
import {WINDOW_ATTRIBUTE} from '../web/src/render/building-surfaces';
import {disposeGroup} from '../web/src/render/kit';
import {parseViewerPreferences} from '../web/src/storage/saves';
import type {Building} from '../web/src/types';
beforeEach(()=>{vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>new Proxy({} as Record<PropertyKey,unknown>,{get:(t,k)=>t[k]??(()=>undefined),set:(t,k,v)=>{t[k]=v;return true;}})})});});
afterEach(()=>vi.unstubAllGlobals());
const fixture=()=>businessCityStudy();
it('makes every recipe reachable via the production selector and preserves snapshot data',()=>{
 const world=fixture(),before=JSON.stringify(world);expect(world.buildings).toHaveLength(30);
 expect(new Set(world.buildings.map(b=>selectBusinessRecipe(b)?.id)).size).toBe(30);
 for(const family of ['office','commercial','mixed-use'] as const)for(const era of ['older','contemporary'])expect(BUSINESS_RECIPES.filter(r=>r.family===family&&r.era===era)).toHaveLength(5);
 for(const b of world.buildings){expect(selectBusinessRecipe(b,'legacy')).toBeUndefined();expect(selectBusinessRecipe(b,'mixed',true)).toBeUndefined();for(const era of ['older','contemporary'] as const)for(const r of businessPool(b,era))expect(r.era).toBe(era);}
 expect(JSON.stringify(world)).toBe(before);
});
it('is stable across ordering, occupancy and later infill, while reducing adjacent repeats',()=>{
 const b=fixture().buildings[0],list=Array.from({length:5},(_,i)=>({...b,id:i+1,x:i*9})),a=resolveBusinessRecipes(list,'mixed'),rev=resolveBusinessRecipes([...list].reverse(),'mixed');expect([...a]).toEqual([...rev]);
 for(let i=1;i<list.length;i++)expect(a.get(i)?.id).not.toBe(a.get(i+1)?.id);
 const more=resolveBusinessRecipes([...list,{...b,id:100,x:3}],'mixed');for(const [id,r] of a)expect(more.get(id)).toEqual(r);
 expect(selectBusinessRecipe({...b,occupancy:0,business_active:false,stage:'demolition'})?.id).toBe(selectBusinessRecipe(b)?.id);
});
it('keeps all recipes and construction phases within the authoritative parcel, with finite geometry',()=>{
 const owner=new ApartmentResources();try{for(const b of fixture().buildings){const recipe=selectBusinessRecipe(b)!;for(const detail of ['close','district','far'] as const)for(const stage of ['foundation','structure','surface','open','renovation','demolition']){
  const group=buildBusinessBuilding({...b,stage,progress:.76},owner.art,{recipe,detail,signage:{proximity:2,feature:'screen'}})!;
  const bounds=new THREE.Box3().setFromObject(group);expect(bounds.max.y).toBeLessThanOrEqual(.58+b.floors*3.25+7);expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width/2-.001);expect(bounds.max.x).toBeLessThanOrEqual(b.width/2+.001);expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth/2-.001);expect(bounds.max.z).toBeLessThanOrEqual(b.depth/2+.001);
  group.traverse(o=>{if(o instanceof THREE.Mesh){const p=o.geometry.getAttribute('position');expect(Array.from(p.array).every(Number.isFinite)).toBe(true);expect(owner.owns(o.material as THREE.Material)).toBe(true);}});disposeGroup(group);
 }}}finally{owner.dispose();}
},30000);
it('fits narrow parcels and varies facade geometry while keeping primary entrances on the access axis',()=>{
 const owner=new ApartmentResources();try{const signatures=new Set<string>(),entries=new Set<number>();for(const recipe of BUSINESS_RECIPES){const b={...fixture().buildings[0],width:5,depth:6,floors:7};const g=buildBusinessBuilding(b,owner.art,{recipe,detail:'close',signage:{proximity:2,feature:'screen'}})!;let vertices=0;g.traverse(o=>{if(o instanceof THREE.Mesh)vertices+=o.geometry.getAttribute('position').count;});const bounds=new THREE.Box3().setFromObject(g);expect(bounds.max.x).toBeLessThanOrEqual(b.width/2+.001);expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width/2-.001);expect(bounds.max.z).toBeLessThanOrEqual(b.depth/2+.001);expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth/2-.001);signatures.add(`${vertices}:${g.userData.businessArchitecture.bands.map((v:any)=>v.use)}`);disposeGroup(g);}
 expect(signatures.size).toBeGreaterThanOrEqual(10);
 const b=fixture().buildings.find(b=>selectBusinessRecipe(b)?.family==='mixed-use')!;for(let seed=0;seed<30;seed++){const g=buildBusinessBuilding({...b,appearance_seed:seed},owner.art,{recipe:selectBusinessRecipe(b)!})!;entries.add(g.userData.businessArchitecture.entryX);disposeGroup(g);}expect([...entries]).toEqual([0]);
 }finally{owner.dispose();}
});
it('uses separate home/office/retail room schedules and keeps unfinished and vacant rooms dark',()=>{
 const owner=new ApartmentResources();try{for(const b of fixture().buildings)for(const stage of ['open','surface','demolition'])for(const active of [true,false]){
  const g=buildBusinessBuilding({...b,stage,business_active:active,occupancy:active?50:0,progress:.8},owner.art,{recipe:selectBusinessRecipe(b)!,detail:'far'})!;
  g.traverse(o=>{if(!(o instanceof THREE.Mesh)||(o.material as THREE.Material).userData.windowPane!==true)return;const a=o.geometry.getAttribute(WINDOW_ATTRIBUTE);expect(a).toBeDefined();const expected=stage==='open'&&active?1:0;let valid=true;for(let i=0;i<a.count;i++)valid&&=a.getY(i)%2===expected&&a.getZ(i)>0&&a.getW(i)>0;expect(valid).toBe(true);});disposeGroup(g);
 }}finally{owner.dispose();}
});
it('removes floors monotonically during demolition and never leaves signs or a crown behind',()=>{
 const owner=new ApartmentResources();try{for(const b of fixture().buildings){let previous=Infinity;for(const progress of [0,.2,.5,.8,1]){
  const g=buildBusinessBuilding({...b,stage:'demolition',progress},owner.art,{recipe:selectBusinessRecipe(b)!,signage:{proximity:2,feature:'screen'}})!;const bounds=new THREE.Box3().setFromObject(g);expect(bounds.max.y).toBeLessThanOrEqual(previous+.001);previous=bounds.max.y;expect(g.userData.businessArchitecture.roofInstalled).toBe(false);expect(g.userData.businessArchitecture.display).toBeUndefined();disposeGroup(g);
 }}}finally{owner.dispose();}
});
it('honours off/static/animated displays, activity and LOD without per-building materials',()=>{
 const owner=new ApartmentResources();try{const b=fixture().buildings[0],recipe=selectBusinessRecipe(b)!;
 for(const mode of ['off','static','animated'] as const){const g=buildBusinessBuilding(b,owner.art,{recipe,displays:mode,signage:{proximity:2,feature:'screen'}})!;expect(g.userData.businessArchitecture.display).toBe(mode==='off'?undefined:mode==='static'?'ad':'screen');disposeGroup(g);}
 for(const update of [{stage:'surface'},{business_active:false}]){const g=buildBusinessBuilding({...b,...update},owner.art,{recipe,signage:{proximity:2,feature:'screen'}})!;expect(g.userData.businessArchitecture.display).toBeUndefined();disposeGroup(g);}
 const g=buildBusinessBuilding(b,owner.art,{recipe,detail:'far',signage:{proximity:2,feature:'screen'}})!;expect(g.userData.businessArchitecture.display).toBeUndefined();disposeGroup(g);
 expect(videoScreenSurface(owner.art,0)).toBe(videoScreenSurface(owner.art,12));expect(videoScreenSurface(owner.art,0)).not.toBe(videoScreenSurface(owner.art,1));expect(storefrontSurface(owner.art,true)).toBe(storefrontSurface(owner.art,true));expect(storefrontSurface(owner.art,true)).not.toBe(storefrontSurface(owner.art,false));expect(storefrontSurface(owner.art,true).map).toBe(storefrontSurface(owner.art,false).map);
 }finally{owner.dispose();}
});
it('uses a bounded 24-tenant atlas with gutters and preserves old preference defaults',()=>{
 expect(STOREFRONT_TENANTS).toHaveLength(24);for(let i=0;i<24;i++){const c=storefrontCell(i);expect(c.u0).toBeGreaterThan(0);expect(c.u1).toBeLessThan(1);expect(c.v0).toBeGreaterThan(0);expect(c.v1).toBeLessThan(1);}expect(storefrontCell(-1)).toEqual(storefrontCell(23));
 expect(parseViewerPreferences(null).commercialDisplays).toBe('animated');expect(parseViewerPreferences(JSON.stringify({version:1,commercialDisplays:'static'})).commercialDisplays).toBe('static');expect(parseDisplayMode('invalid')).toBe('animated');
 const world=fixture();expect([...signageContexts(world).values()].filter(c=>c.feature).length).toBeLessThanOrEqual(6);
});
it('reviews exactly five reachable designs for each family and era',()=>{
 for(const family of ['office','commercial','mixed-use'] as const)for(const era of ['older','contemporary'] as const){
  const world=businessCityStudy(32,family,era);expect(world.buildings).toHaveLength(5);
  expect(new Set(world.buildings.map(b=>selectBusinessRecipe(b,era)?.id)).size).toBe(5);
  for(const b of world.buildings)expect(selectBusinessRecipe(b,era)?.era).toBe(era);
 }
});
it('varies older tenant widths, preserves nonoverlap and reserves separate upper-floor access',()=>{
 const owner=new ApartmentResources();try{
  const widths=new Set<string>();for(const source of fixture().buildings){const recipe=selectBusinessRecipe(source)!;
   const g=buildBusinessBuilding(source,owner.art,{recipe,detail:'far'})!,meta=g.userData.businessArchitecture,slots=meta.frontageSlots as {x:number;width:number}[];
   for(let i=0;i<slots.length;i++){expect(slots[i].width).toBeGreaterThan(0);if(i)expect(slots[i].x-slots[i].width/2).toBeGreaterThanOrEqual(slots[i-1].x+slots[i-1].width/2-.00001);}
   if(recipe.family==='mixed-use'&&recipe.retailFloors>0){for(const slot of slots)expect(Math.abs(slot.x-meta.entryX)-slot.width/2).toBeGreaterThanOrEqual(1.09);}
   if(recipe.era==='older'&&slots.length>1)widths.add(slots.map(s=>s.width.toFixed(2)).join(':'));
   if(recipe.family!=='mixed-use'&&['screen','curtain','department'].includes(recipe.form))expect(slots).toHaveLength(1);
   disposeGroup(g);
  }expect(widths.size).toBeGreaterThan(3);
 }finally{owner.dispose();}
});
it('fits varied generated heights and widths without negative meshes or escaping the lot',()=>{
 const owner=new ApartmentResources();try{for(const recipe of BUSINESS_RECIPES)for(const [width,depth,floors] of [[5,6,1],[8,12,4],[18,20,12],[35,28,35],[90,80,60]]){
  const b={...fixture().buildings[0],width,depth,floors},g=buildBusinessBuilding(b,owner.art,{recipe,detail:'far'})!,box=new THREE.Box3().setFromObject(g);
  expect(box.min.x).toBeGreaterThanOrEqual(-width/2-.001);expect(box.max.x).toBeLessThanOrEqual(width/2+.001);expect(box.min.z).toBeGreaterThanOrEqual(-depth/2-.001);expect(box.max.z).toBeLessThanOrEqual(depth/2+.001);
  g.traverse(o=>{if(o instanceof THREE.Mesh)expect(Array.from(o.geometry.getAttribute('position').array).every(Number.isFinite)).toBe(true);});disposeGroup(g);
 }}finally{owner.dispose();}
},30000);

it('reserves city advertising slots for media and tenant facades instead of restrained showrooms',()=>{
 const world=fixture(),choices=resolveBusinessRecipes(world.buildings,'mixed'),eligible=new Set(world.buildings.filter(b=>businessCanHostAdvertising(choices.get(b.id)!)).map(b=>b.id));
 const contexts=signageContexts(world,eligible);
 for(const [id,context] of contexts)if(context.feature)expect(eligible.has(id)).toBe(true);
 for(const recipe of BUSINESS_RECIPES.filter(r=>r.era==='contemporary'&&['screen','fins','curtain','stepped'].includes(r.form)))expect(businessCanHostAdvertising(recipe)).toBe(false);
 expect(businessCanHostAdvertising(BUSINESS_RECIPES.find(r=>r.id==='media-corner')!)).toBe(true);
});
it('selects five distinct corporate skyscrapers and preserves their geometry across lifecycle and LOD',async()=>{
 const {CORPORATE_SKYSCRAPERS}=await import('../web/src/render/architecture/corporate-tower-recipes');
 const world=businessCityStudy(32,'skyscrapers'),owner=new ApartmentResources(),errors=vi.spyOn(console,'error');
 try{expect(world.buildings).toHaveLength(5);for(const b of world.buildings){const recipe=selectBusinessRecipe(b)!;expect(CORPORATE_SKYSCRAPERS).toContain(recipe);expect(businessCanHostAdvertising(recipe)).toBe(false);expect(selectBusinessRecipe(b,'older')?.towerForm).toBeUndefined();expect(selectBusinessRecipe(b,'legacy')).toBeUndefined();
 let close=0,far=0;for(const detail of ['close','district','far'] as const)for(const stage of ['foundation','structure','surface','open','renovation','demolition']){
 const g=buildBusinessBuilding({...b,stage,progress:.72},owner.art,{recipe,detail})!,bounds=new THREE.Box3().setFromObject(g);let vertices=0;
 expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width/2-.001);expect(bounds.max.x).toBeLessThanOrEqual(b.width/2+.001);expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth/2-.001);expect(bounds.max.z).toBeLessThanOrEqual(b.depth/2+.001);expect(bounds.max.y).toBeLessThanOrEqual(.58+b.floors*3.25+3.01);
 g.traverse(o=>{if(o instanceof THREE.Mesh){vertices+=o.geometry.getAttribute('position').count;expect(Array.from(o.geometry.getAttribute('position').array).every(Number.isFinite)).toBe(true);expect(owner.owns(o.material as THREE.Material)).toBe(true);if((o.material as THREE.Material).userData.windowPane){expect(o.geometry.hasAttribute(WINDOW_ATTRIBUTE)).toBe(true);if((o.material as THREE.Material).userData.corporateGlass)expect(owner.batch(o.material as THREE.Material).material===o.material).toBe(true);}}});
 if(stage==='open'){if(detail==='close')close=vertices;if(detail==='far')far=vertices;}expect(g.userData.businessArchitecture.roofInstalled).toBe(stage==='open'||stage==='renovation');disposeGroup(g);
 }expect(far).toBeLessThan(close);}expect(errors).not.toHaveBeenCalled();
 }finally{owner.dispose();}
},30000);
it('keeps corporate eligibility bounded and supports variable engine dimensions',async()=>{
 const {CORPORATE_SKYSCRAPERS}=await import('../web/src/render/architecture/corporate-tower-recipes');const owner=new ApartmentResources(),base=businessCityStudy(32,'skyscrapers').buildings[0];
 try{for(const dims of [{width:24,depth:20,floors:24},{width:90,depth:80,floors:60}])for(const recipe of CORPORATE_SKYSCRAPERS){const b={...base,...dims};expect(businessPool(b,'contemporary')).toContain(recipe);const g=buildBusinessBuilding(b,owner.art,{recipe,detail:'far'})!,bounds=new THREE.Box3().setFromObject(g);expect(bounds.max.x).toBeLessThanOrEqual(b.width/2);expect(bounds.max.z).toBeLessThanOrEqual(b.depth/2);expect(bounds.max.y).toBeLessThanOrEqual(.58+b.floors*3.25+3.01);disposeGroup(g);}
 for(const b of [{...base,width:23},{...base,depth:19},{...base,floors:23},{...base,kind:'apartment'},{...base,anchor:true}])expect(businessPool(b,'contemporary').every(r=>!r.towerForm)).toBe(true);
 expect(businessCityStudy(32,'skyscrapers','older').buildings).toHaveLength(5);
 }finally{owner.dispose();}
});
