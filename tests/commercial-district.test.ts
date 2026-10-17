import {beforeEach,afterEach,it,expect,vi} from 'vitest';
import * as THREE from 'three/webgpu';
import {COMMERCIAL_DISTRICTS,applyCommercialDistrict,districtAt,parseCommercialDistrict} from '../web/src/render/architecture/commercial-district';
import {shoppingStreetStudy} from '../web/src/render/architecture/shopping-street-study';
import {BUSINESS_RECIPES,selectBusinessRecipe,businessCanHostAdvertising} from '../web/src/render/architecture/business-recipes';
import {buildBusinessBuilding} from '../web/src/render/architecture/business';
import {ApartmentResources} from '../web/src/render/architecture/resources';
import {disposeGroup} from '../web/src/render/kit';
import {DISTRICT_TENANTS,districtSignCell} from '../web/src/render/architecture/district-signs';
import {parseViewerPreferences} from '../web/src/storage/saves';
beforeEach(()=>vi.stubGlobal('document',{createElement:()=>({width:0,height:0,getContext:()=>new Proxy({} as Record<PropertyKey,unknown>,{get:(t,k)=>t[k]??(()=>undefined),set:(t,k,v)=>{t[k]=v;return true;}})})}));
afterEach(()=>vi.unstubAllGlobals());
it('offers five researched profiles, a coherent mixed mode and backward compatible preferences',()=>{
 expect(COMMERCIAL_DISTRICTS).toHaveLength(7);expect(parseCommercialDistrict('invalid')).toBe('standard');expect(parseViewerPreferences(null).commercialDistrict).toBe('standard');expect(parseViewerPreferences(JSON.stringify({version:1,commercialDistrict:'roppongi'})).commercialDistrict).toBe('roppongi');
 const world=shoppingStreetStudy(),b=world.buildings[0],r=selectBusinessRecipe(b)!;expect(applyCommercialDistrict(b,r,'standard')).toBe(r);expect(districtAt({...b,x:1,z:1},'tokyo-mix')).toBe(districtAt({...b,x:159,z:159},'tokyo-mix'));
 for(const extra of [{anchor:true},{floors:24},{width:50},{kind:'apartment'}])expect(applyCommercialDistrict({...b,...extra},r,'shibuya')).toBe(r);
 expect(businessCanHostAdvertising(applyCommercialDistrict(b,r,'ameyoko'))).toBe(false);
});
it('generates packed opposing rows without parcel overlap and preserves access',()=>{
 for(const mode of ['shibuya','roppongi','akihabara','shinjuku','ameyoko'] as const){const world=shoppingStreetStudy(42,mode);expect(world.buildings).toHaveLength(24);expect(shoppingStreetStudy(42,mode)).toEqual(world);
 for(let i=0;i<24;i++){const b=world.buildings[i];if(i%12<11){const next=world.buildings[i+1];expect(next.x-next.width/2-(b.x+b.width/2)).toBeCloseTo(.65);}
 expect(Math.abs(b.z)-b.depth/2).toBeCloseTo(7);expect(b.rotation).toBe(i<12?0:Math.PI);expect(world.building_access?.find(a=>a.building_id===b.id)?.road_id).toBe(1);}
 }
});
it('builds district facades within parcels at all LODs and lifecycle phases with shared ownership',()=>{
 const owner=new ApartmentResources(),errors=vi.spyOn(console,'error'),shapes=new Set<string>();
 try{for(const mode of ['shibuya','roppongi','akihabara','shinjuku','ameyoko'] as const)for(const detail of ['close','district','far'] as const)for(const stage of ['open','structure','surface','demolition']){
 const sample=shoppingStreetStudy(32,mode).buildings[2],b={...sample,stage,progress:.76},before=JSON.stringify(b),recipe=applyCommercialDistrict(b,selectBusinessRecipe(b)!,mode);
 const g=buildBusinessBuilding(b,owner.art,{recipe,detail,signage:{proximity:2,feature:'screen'}})!,box=new THREE.Box3().setFromObject(g);expect(box.min.x).toBeGreaterThanOrEqual(-b.width/2-.001);expect(box.max.x).toBeLessThanOrEqual(b.width/2+.001);expect(box.min.z).toBeGreaterThanOrEqual(-b.depth/2-.001);expect(box.max.z).toBeLessThanOrEqual(b.depth/2+.001);expect(box.max.y).toBeLessThanOrEqual(.58+b.floors*3.25+7);
 let vertices=0;g.traverse(o=>{if(o instanceof THREE.Mesh){vertices+=o.geometry.getAttribute('position').count;expect(owner.owns(o.material as THREE.Material)).toBe(true);expect(Array.from(o.geometry.getAttribute('position').array).every(Number.isFinite)).toBe(true);}});if(stage==='open'&&detail==='close')shapes.add(`${recipe.form}:${vertices}`);expect(g.userData.businessArchitecture.district).toBe(mode);expect(JSON.stringify(b)).toBe(before);disposeGroup(g);
 }expect(shapes.size).toBeGreaterThanOrEqual(4);expect(errors).not.toHaveBeenCalled();}finally{errors.mockRestore();owner.dispose();}
},30000);
it('keeps all 48 horizontal and 16 portrait tenant cells in their single atlas',()=>{
 expect(DISTRICT_TENANTS).toHaveLength(48);expect(new Set(DISTRICT_TENANTS.map(t=>t[0])).size).toBe(48);
 for(const vertical of [false,true])for(let i=0;i<(vertical?16:48);i++){const c=districtSignCell(i,vertical);expect(c.u0).toBeGreaterThan(0);expect(c.u1).toBeLessThan(1);expect(c.v0).toBeGreaterThan(0);expect(c.v1).toBeLessThan(1);expect(c.u1).toBeGreaterThan(c.u0);expect(c.v1).toBeGreaterThan(c.v0);}
});

it('reserves an independent upper-floor entrance for district offices above shops',()=>{
 const owner=new ApartmentResources(),b={...shoppingStreetStudy().buildings[0],kind:'office',floors:7},base=BUSINESS_RECIPES.find(r=>r.id==='deep-frame')!,recipe=applyCommercialDistrict(b,base,'shibuya');
 try{const g=buildBusinessBuilding(b,owner.art,{recipe,detail:'close'})!,meta=g.userData.businessArchitecture;for(const slot of meta.frontageSlots)expect(Math.abs(meta.entryX-slot.x)).toBeGreaterThanOrEqual(slot.width/2);disposeGroup(g);
 expect(applyCommercialDistrict({...b,floors:1},base,'shibuya')).toBe(base);
 }finally{owner.dispose();}
});
