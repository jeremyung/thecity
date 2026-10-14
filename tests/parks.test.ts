import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { Kit, disposeGroup } from '../web/src/render/kit';
import { addPocketPark, addShrineGrove, pocketParks, streetTreePlacements, STREET_TREE_OFFSET, STREET_TREE_SPACING, MAX_STREET_TREES } from '../web/src/render/parks';
import { streetLightPlacements } from '../web/src/render/street-lighting';
import type { Building, Parcel, World } from '../web/src/types';

type StreetWorld=Pick<World,'nodes'|'roads'|'building_access'|'bus_lines'>;
function streetWorld(segments:Array<{id:number;from:number;to:number;kind?:string;stage?:string;connected?:boolean}>,points:Array<{id:number;x:number;z:number}>):StreetWorld{
  return {
    nodes:points.map(point=>({...point,kind:'junction'})),
    roads:segments.map(segment=>{
      const from=points.find(point=>point.id===segment.from)!,to=points.find(point=>point.id===segment.to)!;
      return {id:segment.id,from:segment.from,to:segment.to,kind:segment.kind??'arterial',stage:segment.stage??'open',progress:1,connected:segment.connected??true,length:Math.hypot(to.x-from.x,to.z-from.z),lanes:2};
    }),
  };
}
const parcel=(id:number,overrides:Partial<Parcel>={}):Parcel=>({id,x:0,z:0,width:12,depth:16,rotation:0,access_node:1,suitability:.8,protected:true,reservation:null,...overrides});

describe('street trees',()=>{
  it('lines both sidewalks of open connected arterials only, deterministically and clear of lamps',()=>{
    const world=streetWorld([
      {id:1,from:1,to:2},
      {id:2,from:2,to:3,kind:'local'},
      {id:3,from:3,to:4,stage:'construction'},
      {id:4,from:4,to:5,connected:false},
    ],[{id:1,x:0,z:0},{id:2,x:200,z:0},{id:3,x:200,z:200},{id:4,x:400,z:200},{id:5,x:400,z:400}]);
    const trees=streetTreePlacements(world);
    expect(trees).toEqual(streetTreePlacements(world));
    expect(trees.length).toBeGreaterThan(30);
    for(const tree of trees){
      expect(Math.abs(tree.z)).toBeCloseTo(STREET_TREE_OFFSET,5);
      expect(tree.x).toBeGreaterThanOrEqual(12-1e-6);
      expect(tree.x).toBeLessThanOrEqual(188+1e-6);
      expect(tree.s).toBeGreaterThan(1.3);expect(tree.s).toBeLessThan(1.7);
    }
    const north=trees.filter(tree=>tree.z>0).map(tree=>tree.x).sort((a,b)=>a-b);
    for(let i=1;i<north.length;i++)expect(north[i]-north[i-1]).toBeGreaterThanOrEqual(STREET_TREE_SPACING-1e-6);
    for(const lamp of streetLightPlacements(world))for(const tree of trees)expect(Math.hypot(lamp.x-tree.x,lamp.z-tree.z)).toBeGreaterThanOrEqual(3);
  });

  it('keeps entries, bus stops, cross streets and viaducts clear and respects the cap',()=>{
    const base=streetWorld([{id:1,from:1,to:2}],[{id:1,x:0,z:0},{id:2,x:200,z:0}]);
    const all=streetTreePlacements(base);
    const target=all[Math.floor(all.length/2)];
    const blocked:StreetWorld={...base,
      building_access:[{building_id:9,road_id:1,points:[{x:target.x,z:target.z+Math.sign(target.z)*3},{x:target.x,z:target.z}]}],
      bus_lines:[{id:1,stops:[],route:[],active:true,stop_positions:[{node_id:1,x:60,z:STREET_TREE_OFFSET,heading:0,direction:1}]}],
    };
    const kept=streetTreePlacements(blocked);
    expect(kept.some(tree=>tree.x===target.x&&tree.z===target.z)).toBe(false);
    expect(kept.some(tree=>Math.hypot(tree.x-60,tree.z-STREET_TREE_OFFSET)<7)).toBe(false);

    const crossed=streetWorld([{id:1,from:1,to:2},{id:2,from:3,to:4,kind:'local'},{id:3,from:5,to:6,kind:'rail'}],
      [{id:1,x:0,z:0},{id:2,x:200,z:0},{id:3,x:100,z:-30},{id:4,x:100,z:30},{id:5,x:150,z:-30},{id:6,x:150,z:30}]);
    const near=streetTreePlacements(crossed).filter(tree=>Math.abs(tree.x-100)<4.75||Math.abs(tree.x-150)<6.5);
    expect(near).toEqual([]);

    const points=Array.from({length:120},(_,index)=>({id:index+1,x:0,z:index*400}));
    const long=streetWorld(points.slice(1).map((point,index)=>({id:index+1,from:index+1,to:point.id})),points);
    expect(streetTreePlacements(long)).toHaveLength(MAX_STREET_TREES);
  });
});

describe('pocket parks',()=>{
  it('are protected lots with no building',()=>{
    const building={parcel_id:2} as Building;
    const parks=pocketParks({parcels:[parcel(3),parcel(1,{protected:false}),parcel(2),parcel(4,{width:3})],buildings:[building]});
    expect(parks.map(park=>park.id)).toEqual([3]);
  });

  it('keeps furniture and trees inside the rotated lot with a street-facing entry',()=>{
    const art=new ArtMaterials();
    for(const lot of [parcel(1,{x:40,z:-20,rotation:.7}),parcel(2,{x:-10,z:30,width:7,depth:8,rotation:Math.PI})]){
      const kit=new Kit(),trees=addPocketPark(kit,lot,art),group=kit.finish();
      const inverse=(x:number,z:number)=>{const dx=x-lot.x,dz=z-lot.z,c=Math.cos(lot.rotation!),s=Math.sin(lot.rotation!);return {x:c*dx-s*dz,z:s*dx+c*dz};};
      expect(trees.length).toBeGreaterThanOrEqual(2);
      for(const tree of trees){const local=inverse(tree.x,tree.z);expect(Math.abs(local.x)).toBeLessThan(lot.width/2);expect(Math.abs(local.z)).toBeLessThan(lot.depth/2);}
      const box=new THREE.Box3();
      group.traverse(object=>{
        if(!(object instanceof THREE.Mesh))return;
        const position=object.geometry.getAttribute('position');
        for(let i=0;i<position.count;i++){const local=inverse(position.getX(i),position.getZ(i));box.expandByPoint(new THREE.Vector3(local.x,position.getY(i),local.z));}
      });
      expect(box.min.x).toBeGreaterThanOrEqual(-lot.width/2-1e-3);expect(box.max.x).toBeLessThanOrEqual(lot.width/2+1e-3);
      expect(box.min.z).toBeGreaterThanOrEqual(-lot.depth/2-1e-3);expect(box.max.z).toBeLessThanOrEqual(lot.depth/2+1e-3);
      expect(box.min.y).toBeGreaterThanOrEqual(-1e-4);
      disposeGroup(group);
    }
  });

  it('plants a dense shrine grove that leaves the approach open toward the city',()=>{
    const kit=new Kit(),trees=addShrineGrove(kit,[{x:-350,z:-350},{x:-260,z:-350},{x:-260,z:-270},{x:-350,z:-270}],new ArtMaterials());
    expect(trees.length).toBeGreaterThan(80);
    expect(trees.every(tree=>tree.x>-350&&tree.x<-260&&tree.z>-350&&tree.z<-270)).toBe(true);
    // The longer edge nearest the city center (+z here) holds the torii; its approach stays clear.
    expect(trees.some(tree=>Math.abs(tree.x+305)<4&&tree.z>-305)).toBe(false);
    expect(addShrineGrove(new Kit(),[{x:0,z:0},{x:1,z:1}],new ArtMaterials())).toEqual([]);
    disposeGroup(kit.finish());
  });
});
