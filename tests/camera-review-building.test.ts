import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import { CameraDirector } from '../web/src/camera/director';
import { cameraPathClearance } from '../web/src/camera/clearance';
import type { World } from '../web/src/types';

const controlsMocks=vi.hoisted(()=>({instances:[] as Array<{start():void}>}));
vi.mock('three/addons/controls/OrbitControls.js',async()=>{
  const Three=await import('three');
  class MockOrbitControls{
    target=new Three.Vector3();enableDamping=false;dampingFactor=0;screenSpacePanning=true;minDistance=0;maxDistance=Infinity;minPolarAngle=0;maxPolarAngle=Math.PI;
    private listeners=new Map<string,Set<(event:Event)=>void>>();
    constructor(readonly camera:THREE.Camera,_element:HTMLElement){controlsMocks.instances.push(this);}
    addEventListener(type:string,listener:(event:Event)=>void){const listeners=this.listeners.get(type)??new Set();listeners.add(listener);this.listeners.set(type,listeners);}
    removeEventListener(type:string,listener:(event:Event)=>void){this.listeners.get(type)?.delete(listener);}
    update(){return false;}
    dispose(){}
    start(){for(const listener of this.listeners.get('start')??[])listener(new Event('start'));}
  }
  return{OrbitControls:MockOrbitControls};
});

function worldWithBuildings(buildings:World['buildings']):World{return{
  schema_version:1,engine_version:'test',seed:7,tick:0,day:0,minute_of_day:500,development_rate:4,revision:0,stage:'Founding',
  bounds:{min_x:-400,max_x:400,min_z:-400,max_z:400},terrain:{river:[{x:10,z:0},{x:30,z:0}],protected:[],gateways:[]},weather:{kind:'clear',intensity:0},
  nodes:[],roads:[],parcels:[],buildings,projects:[],trains:[],events:[],stats:{population:0,active_travelers:0,buildings:buildings.length,active_projects:0},
};}
function building(id:number,x:number,z:number):World['buildings'][number]{return{id,parcel_id:id+1000,x,z,rotation:.35,width:22,depth:16,floors:18,kind:'office',stage:'open',progress:1,capacity:24,occupancy:15,built_day:0,name:`Tower ${id}`,appearance_seed:id,history:[],anchor:false,vacant_days:0};}
function element(){return Object.assign(new EventTarget(),{tabIndex:-1}) as unknown as HTMLElement;}
function director(){return new CameraDirector(new THREE.PerspectiveCamera(42,1,0.1,2000),element());}
beforeEach(()=>{controlsMocks.instances=[];vi.stubGlobal('window',{matchMedia:()=>({matches:false,addEventListener(){},removeEventListener(){},addListener(){},removeListener(){}})});});

describe('CameraDirector.reviewBuilding',()=>{
  it('returns the same held building pose after different camera histories and preserves lens',()=>{
    const world=worldWithBuildings([building(72,80,-30)]),first=director(),second=director();first.world(world);second.world(world);first.setLens('detail');
    second.setLens('detail');second.setMode('overview');second.update(.8,1);second.camera.position.set(-200,90,140);second.target.set(0,0,0);
    expect(first.reviewBuilding(72)).toBe(true);expect(second.reviewBuilding(72)).toBe(true);
    expect(first.camera.position.toArray()).toEqual(second.camera.position.toArray());expect(first.target.toArray()).toEqual(second.target.toArray());
    expect(second.lens).toBe('detail');expect(first.mode).toBe('explore');expect(second.mode).toBe('explore');expect(first.label).toBe('Explore');
    const p=first.camera.position.clone(),t=first.target.clone();for(let i=0;i<20;i++)first.update(.1,i*.1);
    expect(first.camera.position.distanceTo(p)).toBeLessThan(1e-8);expect(first.target.distanceTo(t)).toBeLessThan(1e-8);
    first.dispose();second.dispose();
  });
  it('places the camera above the building and at a collision-safe destination',()=>{
    const tower=building(73,0,0);tower.floors=35;tower.rotation=Math.PI/4;const world=worldWithBuildings([tower,building(74,65,35)]),d=director();d.world(world);
    expect(d.reviewBuilding(73)).toBe(true);expect(d.target.toArray()).toEqual([0,(.58+35*3.25+4.05)/2,0]);
    expect(d.camera.position.y).toBeGreaterThan(.58+35*3.25+4.05+1.5);
    const obstacles=(d as unknown as {cameraObstacles:Parameters<typeof cameraPathClearance>[2]}).cameraObstacles;
    expect(cameraPathClearance(d.camera.position,d.camera.position,obstacles,{terrainY:0,safetyMarginM:1.5}).collides).toBe(false);
    expect(Math.abs(d.target.x)).toBeLessThanOrEqual(430);expect(Math.abs(d.target.z)).toBeLessThanOrEqual(430);
    d.dispose();
  });
  it('returns false for an unknown building without mutating camera or mode',()=>{
    const d=director();d.world(worldWithBuildings([building(72,10,10)]));d.setMode('overview');d.update(.2,.2);
    const pos=d.camera.position.clone(),target=d.target.clone(),mode=d.mode,label=d.label,lens=d.lens;
    expect(d.reviewBuilding(999)).toBe(false);expect(d.camera.position.toArray()).toEqual(pos.toArray());expect(d.target.toArray()).toEqual(target.toArray());expect(d.mode).toBe(mode);expect(d.label).toBe(label);expect(d.lens).toBe(lens);
    d.dispose();
  });
});
