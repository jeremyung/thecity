import {expect,it} from 'vitest';
import {selectApartmentArchitecture,apartmentArchitectureKey,parseApartmentEra} from '../web/src/render/architecture/era';
import {architectureCityStudy} from '../web/src/render/architecture/city-study';
const apartment=()=>({...architectureCityStudy().buildings.find(b=>b.kind==='apartment')!,width:18,depth:18,floors:6,stage:'open',progress:1});
it('selects diverse, deterministic eras without changing world records',()=>{
 const b=apartment(),before=JSON.stringify(b),mixed=new Set(),older=new Set(),newer=new Set();
 for(let seed=0;seed<128;seed++){
  const specimen={...b,appearance_seed:seed};
  const selection=selectApartmentArchitecture(specimen);mixed.add(selection.kind==='legacy'?'legacy':selection.era);
  older.add(apartmentArchitectureKey(selectApartmentArchitecture(specimen,'older')));
  newer.add(apartmentArchitectureKey(selectApartmentArchitecture(specimen,'contemporary')));
  expect(selectApartmentArchitecture({...specimen,occupancy:0,built_day:100,stage:'renovation'})).toEqual(selection);
 }
 expect([...mixed].sort()).toEqual(['contemporary','older']);expect(older.size).toBe(5);expect(newer.size).toBe(5);expect(JSON.stringify(b)).toBe(before);
});
it('routes contemporary towers to both kits and preserves legacy fallbacks',()=>{
 const b={...apartment(),floors:32,width:26,depth:24},recipes=new Set();
 for(let appearance_seed=0;appearance_seed<32;appearance_seed++){
  const choice=selectApartmentArchitecture({...b,appearance_seed},'contemporary');expect(choice.kind).toBe('tower');recipes.add(apartmentArchitectureKey(choice));
 }
 expect(recipes.size).toBe(5);expect(selectApartmentArchitecture(b,'older')).toEqual({kind:'legacy'});
 for(const specimen of [{...b,kind:'office'},{...b,width:4}])expect(selectApartmentArchitecture(specimen,'contemporary')).toEqual({kind:'legacy'});
 expect(selectApartmentArchitecture(b,'contemporary',true)).toEqual({kind:'legacy'});
});
it('defaults old and invalid preferences to mixed and accepts each supported mode',()=>{
 for(const value of [undefined,null,'new',{},12])expect(parseApartmentEra(value)).toBe('mixed');
 for(const mode of ['mixed','older','contemporary'])expect(parseApartmentEra(mode)).toBe(mode);
});
it('the five-tower city fixture covers each contemporary recipe with nonoverlapping parcels',()=>{
 const world=architectureCityStudy(32,true),choices=world.buildings.map(b=>selectApartmentArchitecture(b,'contemporary'));
 expect(new Set(choices.map(apartmentArchitectureKey)).size).toBe(5);
 for(const b of world.buildings)expect(b.floors).toBeGreaterThanOrEqual(12);
 for(let i=0;i<world.buildings.length;i++)for(let j=i+1;j<world.buildings.length;j++){const a=world.buildings[i],b=world.buildings[j];expect(Math.abs(a.x-b.x)).toBeGreaterThan((a.width+b.width)/2);}
});
