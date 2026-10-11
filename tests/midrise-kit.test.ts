import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import { compactMidriseEnvelope } from '../web/src/render/building-massing';
import type { Building } from '../web/src/types';

function midrise(kind:'apartment'|'office',stage:'open'|'construction'|'renovation',progress=1,floors=8,width=24,depth=19,appearance_seed=9812):Building{
  return {id:71,parcel_id:18,x:0,z:0,rotation:0,width,depth,floors,kind,stage,progress,capacity:48,occupancy:21,business_active:false,built_day:4,name:`${kind} test`,appearance_seed,history:[],anchor:false,vacant_days:0};
}

function positions(group:THREE.Object3D,filter?:(mesh:THREE.Mesh)=>boolean){
  const points:Array<THREE.Vector3>=[];
  group.traverse(object=>{
    if(!(object instanceof THREE.Mesh)||filter&&!filter(object))return;
    const position=object.geometry.getAttribute('position');
    for(let i=0;i<position.count;i++)points.push(new THREE.Vector3(position.getX(i),position.getY(i),position.getZ(i)));
  });
  return points;
}

function expectFinite(group:THREE.Object3D){
  for(const point of positions(group))expect([point.x,point.y,point.z].every(Number.isFinite)).toBe(true);
}

function expectParcelContained(group:THREE.Object3D,b:Building){
  const bounds=new THREE.Box3().setFromObject(group);
  expect([bounds.min.x,bounds.max.x,bounds.min.y,bounds.max.y,bounds.min.z,bounds.max.z].every(Number.isFinite)).toBe(true);
  expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width/2-1e-6);
  expect(bounds.max.x).toBeLessThanOrEqual(b.width/2+1e-6);
  expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth/2-1e-6);
  expect(bounds.max.z).toBeLessThanOrEqual(b.depth/2+1e-6);
}

function boundsAt(points:THREE.Vector3[],minY:number,maxY:number){
  const selected=points.filter(point=>point.y>=minY&&point.y<=maxY);
  expect(selected.length).toBeGreaterThan(0);
  const box=new THREE.Box3().setFromPoints(selected);
  return box;
}

describe('midrise family geometry integration',()=>{
  it('keeps stepped apartment and office masses inside their parcel and retains all floors',()=>{
    const art=new ArtMaterials();
    for(const kind of ['apartment','office'] as const){
      const b=midrise(kind,'open');
      const group=buildingKit(b,art),bounds=new THREE.Box3().setFromObject(group);
      expectFinite(group);
      // The foundation plinth has a small intentional edge beyond the mass,
      // but remains within the authoritative parcel in this fixture.
      expect(bounds.min.x).toBeGreaterThanOrEqual(-b.width/2);
      expect(bounds.max.x).toBeLessThanOrEqual(b.width/2);
      expect(bounds.min.z).toBeGreaterThanOrEqual(-b.depth/2);
      expect(bounds.max.z).toBeLessThanOrEqual(b.depth/2);
      expect(bounds.max.y).toBeGreaterThan(0.58+b.floors*3.25);
      expect(bounds.max.y).toBeLessThan(0.58+b.floors*3.25+4);
      disposeGroup(group);
    }
    art.dispose();
  });

  it('fits roof equipment and facade details inside every narrow supported parcel envelope',()=>{
    const art=new ArtMaterials();
    const floorCounts=[4,8,11],seeds=[3,27,104];
    for(const kind of ['apartment','office'] as const){
      for(const width of [8,10,12])for(const depth of [8,10,14]){
        for(const floors of floorCounts)for(const seed of seeds){
          const b=midrise(kind,'open',1,floors,width,depth,seed);
          const group=buildingKit(b,art),bounds=new THREE.Box3().setFromObject(group);
          expectFinite(group);
          const context=`${kind} ${width}x${depth} ${floors} floors seed ${seed}`;
          expect(bounds.min.x,context).toBeGreaterThanOrEqual(-width/2-1e-6);
          expect(bounds.max.x,context).toBeLessThanOrEqual(width/2+1e-6);
          expect(bounds.min.z,context).toBeGreaterThanOrEqual(-depth/2-1e-6);
          expect(bounds.max.z,context).toBeLessThanOrEqual(depth/2+1e-6);
          disposeGroup(group);
        }
      }
    }
    art.dispose();
  }, 20_000);

  it('contains compact-midrise geometry across stages and parcel sizes',()=>{
    const art=new ArtMaterials();
    const parcels=[[8,8],[14,12],[24,19],[40,32]] as const;
    const stages=[['open',1],['construction',0],['construction',.8],['construction',1],['renovation',.5]] as const;
    for(const kind of ['apartment','office'] as const){
      for(const [width,depth] of parcels)for(const floors of [4,8,11])for(const seed of [3,27,104]){
        for(const [stage,progress] of stages){
          const b=midrise(kind,stage,progress,floors,width,depth,seed);
          const group=buildingKit(b,art,{compactMidrise:true});
          expectParcelContained(group,b);
          disposeGroup(group);
        }
      }
    }
    art.dispose();
  },60_000);

  it('keeps compact-midrise geometry identical across both surface styles',()=>{
    const art=new ArtMaterials();
    const b=midrise('apartment','open',1,8,24,19,27);
    const positionsFor=(style:'anime'|'miniature')=>{
      art.setStyle(style);
      const group=buildingKit(b,art,{compactMidrise:true});
      const vertices=positions(group).map(point=>point.toArray());
      disposeGroup(group);
      return vertices;
    };
    expect(positionsFor('anime')).toEqual(positionsFor('miniature'));
    art.dispose();
  });

  it('leaves the authored ground entrance clear of front window panels while retaining rear glazing',()=>{
    const art=new ArtMaterials();
    for(const compactMidrise of [false,true])for(const kind of ['apartment','office'] as const){
      const b=midrise(kind,'open',1,8,12,14,17),group=buildingKit(b,art,{compactMidrise});
      const frontZ=(compactMidrise?compactMidriseEnvelope(b,b.width*.84,b.depth*.84).depth:b.depth*.84)/2;
      const paneMaterials=[art.get('#55757a',.25),art.get('#94a7a0',.3)];
      let doorwayGlassVertices=0,rearGlassVertices=0;
      group.traverse(object=>{
        if(!(object instanceof THREE.Mesh)||!paneMaterials.some(material=>material===object.material))return;
        const position=object.geometry.getAttribute('position');
        for(let i=0;i<position.count;i++){
          const x=position.getX(i),y=position.getY(i),z=position.getZ(i);
          if(Math.abs(x)<.4&&y>=.74&&y<=2.68&&z>frontZ+.12)doorwayGlassVertices++;
          if(y>1.2&&y<2.7&&z< -frontZ+.05)rearGlassVertices++;
        }
      });
      // The centered door glazing contributes one box (24 face vertices); additional
      // front glazing in this aperture would indicate a window crossing it.
      expect(doorwayGlassVertices).toBe(24);
      expect(rearGlassVertices).toBeGreaterThan(8);
      disposeGroup(group);
    }
    art.dispose();
  });

  it('clips upper construction to the same office-family tiers and keeps the early scaffold at street level',()=>{
    const art=new ArtMaterials();
    const open=buildingKit(midrise('office','open'),art);
    const construction=buildingKit(midrise('office','construction',.8),art);
    const base=.58,floor=3.25;
    const openPoints=positions(open),constructionPoints=positions(construction);
    const openPodium=boundsAt(openPoints,base+.3,base+2*floor-.2);
    const openShaft=boundsAt(openPoints,base+2*floor+.2,base+6*floor-.2);
    const constructionPodium=boundsAt(constructionPoints,base+.3,base+2*floor-.2);
    const constructionShaft=boundsAt(constructionPoints,base+2*floor+.2,base+6*floor-.2);
    expect(openShaft.max.x-openShaft.min.x).toBeLessThan(openPodium.max.x-openPodium.min.x);
    expect(constructionShaft.max.x-constructionShaft.min.x).toBeLessThan(constructionPodium.max.x-constructionPodium.min.x);
    expect(constructionShaft.max.z-constructionShaft.min.z).toBeLessThan(constructionPodium.max.z-constructionPodium.min.z);

    const early=buildingKit(midrise('office','construction',0),art);
    const scaffold=art.get('#8c8070',.72);
    const upperScaffold=positions(early,mesh=>mesh.material===scaffold).filter(point=>point.y>base+2*floor);
    expect(upperScaffold).toHaveLength(0);
    expectFinite(early);
    [open,construction,early].forEach(disposeGroup);
    art.dispose();
  });

  it('does not duplicate ground-level renovation scaffold on recessed tiers',()=>{
    const art=new ArtMaterials();
    const group=buildingKit(midrise('office','renovation',.5),art);
    const scaffold=art.get('#8c8070',.72);
    const points=positions(group,mesh=>mesh.material===scaffold);
    expect(points.length).toBeGreaterThan(0);
    expect(Math.max(...points.map(point=>point.y))).toBeLessThanOrEqual(6.5+.08);
    expectFinite(group);
    disposeGroup(group);
    art.dispose();
  });
});
