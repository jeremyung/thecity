import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import * as THREE from 'three/webgpu';
import {ApartmentResources} from '../web/src/render/architecture/resources';
import {BUSINESS_RECIPES,businessPool,selectBusinessRecipe,resolveBusinessRecipes,parseDisplayMode} from '../web/src/render/architecture/business-recipes';
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
 const world=fixture(),before=JSON.stringify(world);expect(world.buildings).toHaveLength(15);
 expect(new Set(world.buildings.map(b=>selectBusinessRecipe(b)?.id)).size).toBe(15);
 for(const family of ['office','commercial','mixed-use'] as const)expect(BUSINESS_RECIPES.filter(r=>r.family===family)).toHaveLength(5);
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
it('fits narrow parcels and varies facade geometry and entrances across recipes and seeds',()=>{
 const owner=new ApartmentResources();try{const signatures=new Set<string>(),entries=new Set<number>();for(const recipe of BUSINESS_RECIPES){const b={...fixture().buildings[0],width:5,depth:6,floors:7};const g=buildBusinessBuilding(b,owner.art,{recipe,detail:'close',signage:{proximity:2,feature:'screen'}})!;let vertices=0;g.traverse(o=>{if(o instanceof THREE.Mesh)vertices+=o.geometry.getAttribute('position').count;});const bounds=new THREE.Box3().setFromObject(g);expect(bounds.max.x).toBeLessThanOrEqual(b.width/2+.001);expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width/2-.001);expect(bounds.max.z).toBeLessThanOrEqual(b.depth/2+.001);expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth/2-.001);signatures.add(`${vertices}:${g.userData.businessArchitecture.bands.map((v:any)=>v.use)}`);disposeGroup(g);}
 expect(signatures.size).toBeGreaterThanOrEqual(10);
 const b=fixture().buildings.find(b=>selectBusinessRecipe(b)?.family==='mixed-use')!;for(let seed=0;seed<30;seed++){const g=buildBusinessBuilding({...b,appearance_seed:seed},owner.art,{recipe:selectBusinessRecipe(b)!})!;entries.add(g.userData.businessArchitecture.entryX);disposeGroup(g);}expect(entries.size).toBe(2);
 }finally{owner.dispose();}
});
it('uses separate home/office/retail room schedules and keeps unfinished and vacant rooms dark',()=>{
 const owner=new ApartmentResources();try{for(const b of fixture().buildings)for(const stage of ['open','surface','demolition'])for(const active of [true,false]){
  const g=buildBusinessBuilding({...b,stage,business_active:active,occupancy:active?50:0,progress:.8},owner.art,{recipe:selectBusinessRecipe(b)!,detail:'far'})!;
  g.traverse(o=>{if(!(o instanceof THREE.Mesh)||(o.material as THREE.Material).userData.windowPane!==true)return;const a=o.geometry.getAttribute(WINDOW_ATTRIBUTE);expect(a).toBeDefined();for(let i=0;i<a.count;i++){expect(a.getY(i)%2).toBe(stage==='open'&&active?1:0);expect(a.getZ(i)).toBeGreaterThan(0);expect(a.getW(i)).toBeGreaterThan(0);}});disposeGroup(g);
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
 expect(videoScreenSurface(owner.art,0)).toBe(videoScreenSurface(owner.art,3));expect(videoScreenSurface(owner.art,0)).not.toBe(videoScreenSurface(owner.art,1));expect(storefrontSurface(owner.art,true)).toBe(storefrontSurface(owner.art,true));expect(storefrontSurface(owner.art,true)).not.toBe(storefrontSurface(owner.art,false));expect(storefrontSurface(owner.art,true).map).toBe(storefrontSurface(owner.art,false).map);
 }finally{owner.dispose();}
});
it('uses a bounded 24-tenant atlas with gutters and preserves old preference defaults',()=>{
 expect(STOREFRONT_TENANTS).toHaveLength(24);for(let i=0;i<24;i++){const c=storefrontCell(i);expect(c.u0).toBeGreaterThan(0);expect(c.u1).toBeLessThan(1);expect(c.v0).toBeGreaterThan(0);expect(c.v1).toBeLessThan(1);}expect(storefrontCell(-1)).toEqual(storefrontCell(23));
 expect(parseViewerPreferences(null).commercialDisplays).toBe('animated');expect(parseViewerPreferences(JSON.stringify({version:1,commercialDisplays:'static'})).commercialDisplays).toBe('static');expect(parseDisplayMode('invalid')).toBe('animated');
 const world=fixture();expect([...signageContexts(world).values()].filter(c=>c.feature).length).toBeLessThanOrEqual(6);
});
