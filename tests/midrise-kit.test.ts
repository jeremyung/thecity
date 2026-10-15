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
  it('carries office ribbon glazing along both side elevations while retaining parcel clearance',()=>{
    const art=new ArtMaterials();
    try {
      for(const style of ['anime','miniature'] as const){
        art.setStyle(style);
        const b=midrise('office','open',1,6,24,19,3);
        const group=buildingKit(b,art);group.updateMatrixWorld(true);
        expectParcelContained(group,b);
        for(const side of [-1,1])for(const z of [-4.7,-2.8,.6,3.7]){
          const hit=new THREE.Raycaster(new THREE.Vector3(side*20,.58+3.25+1.99,z),new THREE.Vector3(-side,0,0)).intersectObject(group,true)[0];
          expect(hit,`${style} side ${side} at ${z}`).toBeDefined();
          const material=(hit.object as THREE.Mesh).material;
          expect(material).toBe(art.window('office'));
        }
        disposeGroup(group);
      }
    } finally {art.dispose();}
  });

  it('gives tall homes a centered ground door instead of an uninterrupted window facade',()=>{
    const art=new ArtMaterials();
    try {
      for(const style of ['anime','miniature'] as const){
        art.setStyle(style);
        for(const floors of [4,5,8])for(const stage of ['open','renovation'] as const){
          const b:Building={...midrise('apartment',stage,1,floors,8,12,32),kind:'home'};
          const group=buildingKit(b,art);group.updateMatrixWorld(true);
          const hits=new THREE.Raycaster(new THREE.Vector3(0,1.2,10),new THREE.Vector3(0,0,-1)).intersectObject(group,true);
          // Renovation retains the door behind temporary construction fencing.
          const hit=stage==='open'?hits[0]:hits.find(h=>(h.object as THREE.Mesh).material===art.get('#55757a',.25));
          expect(hit,`${style}/${floors}/${stage} door`).toBeDefined();
          expect((hit!.object as THREE.Mesh).material===art.get('#55757a',.25)).toBe(true);
          expect(hit!.point.z).toBeCloseTo(12*.84/2+.1925,5);
          if(stage==='open')expectParcelContained(group,b);
          disposeGroup(group);
        }
      }
    } finally {art.dispose();}
  });

  it('gives terrace apartments solid front guards above ground without blocking the entrance',()=>{
    const art=new ArtMaterials();
    try {
      for(const style of ['anime','miniature'] as const){
        art.setStyle(style);
        const b=midrise('apartment','open',1,8,12,14,32);
        const group=buildingKit(b,art);group.updateMatrixWorld(true);
        const frontHit=(x:number,y:number)=>new THREE.Raycaster(new THREE.Vector3(x,y,10),new THREE.Vector3(0,0,-1)).intersectObject(group,true)[0];
        const guard=art.get('#c3b9a5');
        // Solid balcony guards should read as a continuous horizontal surface
        // across multiple bays, above (not across) the street entrance.
        const hits=[-2,-1,0,1,2].map(x=>frontHit(x,.58+3.25+.7));
        expect(hits.every(hit=>hit!==undefined)).toBe(true);
        expect(new Set(hits.map(hit=>(hit.object as THREE.Mesh).material)).size).toBe(1);
        expect((hits[0].object as THREE.Mesh).material).toBe(guard);
        expect(hits[0].point.z).toBeGreaterThan(b.depth*.84/2+.3);
        const door=frontHit(0,1.5);
        expect(door.point.z).toBeLessThan(hits[0].point.z);
        expect((door.object as THREE.Mesh).material).not.toBe((hits[0].object as THREE.Mesh).material);
        // The balcony floor must meet the storey's floor rather than sit a
        // metre above it. Test support across the band and both closed ends.
        const balconyZ=b.depth*.84/2+.35;
        for(const x of [-2,0,2]){
          const floorHit=new THREE.Raycaster(new THREE.Vector3(x,.58+3.25+.5,balconyZ),new THREE.Vector3(0,-1,0)).intersectObject(group,true)[0];
          expect(floorHit.point.y).toBeCloseTo(.58+3.25+.145,5);
        }
        for(const direction of [-1,1]){
          const end=new THREE.Raycaster(new THREE.Vector3(0,.58+3.25+.7,balconyZ),new THREE.Vector3(direction,0,0)).intersectObject(group,true)[0];
          expect((end.object as THREE.Mesh).material).toBe(guard);
          expect(Math.abs(end.point.x)).toBeLessThan(b.width/2);
        }
        const terraceFloor=.58+7*3.25;
        for(const x of [-2,0,2]){
          const surface=new THREE.Raycaster(new THREE.Vector3(x,terraceFloor+.5,4),new THREE.Vector3(0,-1,0)).intersectObject(group,true)[0];
          expect((surface.object as THREE.Mesh).material).toBe(art.get('#aaa99e'));
          expect(surface.point.y).toBeCloseTo(terraceFloor+.05,5);
        }
        for(const direction of [new THREE.Vector3(0,0,1),new THREE.Vector3(-1,0,0),new THREE.Vector3(1,0,0)]){
          const boundary=new THREE.Raycaster(new THREE.Vector3(0,terraceFloor+.7,4),direction).intersectObject(group,true)[0];
          expect(boundary,'terrace closes its front and side edges').toBeDefined();
          expect((boundary.object as THREE.Mesh).material).toBe(guard);
        }
        disposeGroup(group);
      }
    }finally{art.dispose();}
  });

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
    const floorCounts=[4,8,11],seeds=[3,27,32,104];
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
      for(const [width,depth] of parcels)for(const floors of [4,8,11])for(const seed of [3,27,32,104]){
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
    for(const seed of [17,32])for(const compactMidrise of [false,true])for(const kind of ['apartment','office'] as const){
      const b=midrise(kind,'open',1,8,12,14,seed),group=buildingKit(b,art,{compactMidrise});
      const frontZ=(compactMidrise?compactMidriseEnvelope(b,b.width*.84,b.depth*.84).depth:b.depth*.84)/2;
      const paneMaterials=[art.get('#55757a',.25),art.get('#94a7a0',.3),art.window('home'),art.window('office'),art.window('shop')];
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

  it('keeps centered entrances clear on 8x12 two- and three-floor apartment and office variants',()=>{
    const art=new ArtMaterials();
    const styleVertices:Record<'anime'|'miniature',number[][][]>={anime:[],miniature:[]};
    const cases=[
      ['apartment',2,2714839168],['apartment',3,1441647569],
      ['office',2,2128358164],['office',2,878632987],['office',3,2471553678],
    ] as const;
    for(const style of ['anime','miniature'] as const){
      art.setStyle(style);
      for(const [kind,floors,appearance_seed] of cases){
        const b=midrise(kind,'open',1,floors,8,12,appearance_seed);
        const group=buildingKit(b,art),context=`${style} ${kind} ${floors} floors seed ${appearance_seed}`;
        let doorwayGlassVertices=0;
        const paneMaterials=[art.get('#55757a',.25),art.get('#94a7a0',.3),art.window('home'),art.window('office'),art.window('shop')];
        group.traverse(object=>{
          if(!(object instanceof THREE.Mesh)||!paneMaterials.some(material=>material===object.material))return;
          const position=object.geometry.getAttribute('position');
          for(let i=0;i<position.count;i++){
            const x=position.getX(i),y=position.getY(i),z=position.getZ(i);
            if(Math.abs(x)<.4&&y>=.74&&y<=2.68&&z>12*.84/2+.12)doorwayGlassVertices++;
          }
        });
        expect(doorwayGlassVertices,`${context} has only the centered door glazing`).toBe(24);
        expectParcelContained(group,b);
        styleVertices[style].push(positions(group).map(point=>point.toArray()));
        disposeGroup(group);
      }
    }
    expect(styleVertices.miniature).toEqual(styleVertices.anime);
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

  it('erects the slab office at full width until its single inset top floor in both styles',()=>{
    const art=new ArtMaterials(),base=.58,floor=3.25;
    for(const style of ['anime','miniature'] as const){
      art.setStyle(style);
      for(const floors of [4,8,11]){
        // All structural floors exist before authored cladding replaces plates.
        const b=midrise('office','construction',.69,floors,24,19,27);
        const group=buildingKit(b,art);
        const concrete=positions(group,mesh=>mesh.material===art.get('#aaa99e'));
        const atFloor=(n:number)=>boundsAt(concrete,base+n*floor-.17,base+n*floor+.01);
        const first=atFloor(1),lastFull=atFloor(floors-1),top=atFloor(floors);
        expect(lastFull.min.x).toBeCloseTo(first.min.x,5);
        expect(lastFull.max.x).toBeCloseTo(first.max.x,5);
        expect(lastFull.min.z).toBeCloseTo(first.min.z,5);
        expect(lastFull.max.z).toBeCloseTo(first.max.z,5);
        expect(top.min.x).toBeGreaterThan(lastFull.min.x);
        expect(top.max.x).toBeLessThan(lastFull.max.x);
        expect(top.min.z).toBeGreaterThan(lastFull.min.z);
        expect(top.max.z).toBeLessThan(lastFull.max.z);
        expectParcelContained(group,b);
        disposeGroup(group);
      }
    }
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
