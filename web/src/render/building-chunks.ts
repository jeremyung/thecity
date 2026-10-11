import * as THREE from 'three/webgpu';
import { IncrementalGeometryMerge } from './incremental-geometry-merge';
import { IncrementalGeometryTransform } from './incremental-geometry-transform';

export type BuildingBatchSurface = { material: THREE.Material; color?: THREE.Color };
type BuildingSurfaceResolver = (material: THREE.Material) => THREE.Material | BuildingBatchSurface;
export interface RebuildableBuildingSource { createGroup():THREE.Group; disposeGroup(group:THREE.Group):void }
type Source = { id:number; key:string; contours:Float32Array; descriptor:RebuildableBuildingSource|null; pendingGroup:THREE.Group|null; legacyGroup:THREE.Group|null; leases:number; retired:boolean; released:boolean };
type MeshInput = { id: number; mesh: THREE.Mesh|null; matrix: THREE.Matrix4; vertices: number };
type PreparedPart = { geometry: THREE.BufferGeometry; id: number; shadow: boolean; triangles: number };
type MaterialJob = { material: THREE.Material; parts: PreparedPart[]; merger: IncrementalGeometryMerge | null };
type TransformJob = { input:MeshInput; transform:IncrementalGeometryTransform; material:THREE.Material; shadow:boolean; triangles:number };
type RetiredSource = { afterRevision: number; release: () => void };
type Build = {
  key: string;
  revision: number;
  sources:Source[];
  sourceCursor:number;
  currentSource:Source|null;
  currentGroup:THREE.Group|null;
  groupMeshes:MeshInput[];
  groupMeshCursor:number;
  transform: TransformJob | null;
  grouped: Map<THREE.Material, PreparedPart[]>;
  jobs: MaterialJob[] | null;
  jobCursor: number;
  next: THREE.Group;
  contourSources: Float32Array[];
  contourSourceCursor: number;
  contourOffset: number;
  contourOutput: Float32Array;
  contourCursor: number;
  contourMin: THREE.Vector3;
  contourMax: THREE.Vector3;
  contoursDone: boolean;
  ownedResources:Array<THREE.Material|THREE.Texture>;
  ownedResourceSet:Set<THREE.Material|THREE.Texture>;
};

export const buildingChunkKey=(x:number,z:number)=>`${Math.floor((x+400)/200)}:${Math.floor((z+400)/200)}`;
const MAX_DEFERRED_UPDATES=12;
const MAX_VERTICES_PER_UPDATE=12_000;
const MAX_INDICES_PER_UPDATE=36_000;
const MAX_CONTOUR_FLOATS_PER_UPDATE=24_000;
const MAX_PREPARED_VERTICES_PER_UPDATE=12_000;
const MAX_PREPARED_MESHES_PER_UPDATE=16;
const MIN_CONTOUR_EDGE_LENGTH_SQ=12.25;

/** Retains authored detail while rebuilding static neighborhood batches over multiple updates. */
export class BuildingChunks {
  readonly root=new THREE.Group();
  private sources=new Map<number,Source>();
  private batches=new Map<string,THREE.Group>();
  private dirty=new Set<string>();
  private dirtySince=new Map<string,number>();
  private revisions=new Map<string,number>();
  private updateSequence=0;
  private retired=new Map<string,RetiredSource[]>();
  private active:Build|null=null;
  private contoursVisible=true;

  constructor(private material:BuildingSurfaceResolver,private retire:(dispose:()=>void)=>void,private contourMaterial?:THREE.LineBasicNodeMaterial){}

  get pendingCount(){return this.dirty.size+(this.active&&!this.dirty.has(this.active.key)?1:0);}
  setContoursVisible(visible:boolean){this.contoursVisible=visible;this.root.traverse(o=>{if(o instanceof THREE.LineSegments)o.visible=visible;});if(this.active)this.active.next.traverse(o=>{if(o instanceof THREE.LineSegments)o.visible=visible;});}
  private key(group:THREE.Group){return buildingChunkKey(group.position.x,group.position.z);}
  private markDirty(key:string){if(!this.dirty.has(key))this.dirtySince.set(key,this.updateSequence);this.dirty.add(key);this.revisions.set(key,(this.revisions.get(key)??0)+1);}

  set(id:number,group:THREE.Group,sourceOrRelease?:RebuildableBuildingSource|(()=>void)){
    const old=this.sources.get(id),key=this.key(group);
    if(old){
      this.markDirty(old.key);
      if(typeof sourceOrRelease==='function')this.queueRetirement(old.key,this.revisions.get(old.key)??0,sourceOrRelease);
      this.retireSource(old);
    }
    const descriptor=typeof sourceOrRelease==='function'?null:sourceOrRelease??null;
    this.sources.set(id,{id,key,contours:this.outlines(group),descriptor,pendingGroup:descriptor?group:null,legacyGroup:descriptor?null:group,leases:0,retired:false,released:false});
    this.markDirty(key);
  }

  private retireSource(source:Source){source.retired=true;this.releaseSourceIfPossible(source);}
  private releaseSourceIfPossible(source:Source){
    if(!source.retired||source.leases!==0||source.released)return;
    if(source.pendingGroup&&source.descriptor)source.descriptor.disposeGroup(source.pendingGroup);
    source.pendingGroup=null;source.released=true;
  }

  private queueRetirement(key:string,afterRevision:number,release:()=>void){const list=this.retired.get(key)??[];list.push({afterRevision,release});this.retired.set(key,list);}

  private outlines(group:THREE.Group){
    if(!this.contourMaterial)return new Float32Array(0);
    const values:number[]=[];group.updateMatrixWorld(true);
    group.traverse(o=>{if(!(o instanceof THREE.Mesh))return;if(maxWorldBoundsDiagonalBelow(o.geometry,o.matrixWorld,MIN_CONTOUR_EDGE_LENGTH_SQ))return;const edges=new THREE.EdgesGeometry(o.geometry,42);edges.applyMatrix4(o.matrixWorld);const points=edges.getAttribute('position');
      for(let i=0;i<points.count;i+=2){const dx=points.getX(i+1)-points.getX(i),dy=points.getY(i+1)-points.getY(i),dz=points.getZ(i+1)-points.getZ(i);
        // Major roof and facade breaks only: tiny fittings stay unoutlined.
        if(dx*dx+dy*dy+dz*dz<MIN_CONTOUR_EDGE_LENGTH_SQ)continue;
        for(let j=0;j<2;j++)values.push(points.getX(i+j),points.getY(i+j),points.getZ(i+j));
      }edges.dispose();
    });return new Float32Array(values);
  }

  remove(id:number,release?:()=>void){const old=this.sources.get(id);if(!old){release?.();return;}this.sources.delete(id);this.markDirty(old.key);if(release)this.queueRetirement(old.key,this.revisions.get(old.key)??0,release);this.retireSource(old);}

  /**
   * Advances at most one chunk build. Source transforms and merge copies are
   * quota-stepped with no source-size exception; contours use a separate quota.
   */
  update(pendingKeys?:ReadonlySet<string>){
    this.updateSequence++;
    if(!this.active){
      let selected:string|undefined;
      for(const key of this.dirty){if(!pendingKeys?.has(key)||(this.updateSequence-(this.dirtySince.get(key)??0))>=MAX_DEFERRED_UPDATES){selected=key;break;}}
      if(selected!==undefined)this.beginBuild(selected);
    }
    const build=this.active;if(!build)return;
    try{
      this.prepareMeshes(build);
      this.copyContours(build,MAX_CONTOUR_FLOATS_PER_UPDATE);
      if(build.sourceCursor>=build.sources.length&&build.currentGroup===null&&build.transform===null&&build.jobs===null)this.createJobs(build);
      if(build.jobs){this.advanceMergers(build,MAX_VERTICES_PER_UPDATE,MAX_INDICES_PER_UPDATE);}
      if(this.ready(build))this.commit(build);
    }catch(error){this.cancelBuild();this.markDirty(build.key);throw error;}
  }

  private beginBuild(key:string){
    const sources:Source[]=[],contourSources:Float32Array[]=[];
    for(const source of this.sources.values()){
      if(source.key!==key)continue;
      source.leases++;sources.push(source);
      if(source.contours.length)contourSources.push(source.contours);
    }
    const contourLength=contourSources.reduce((sum,values)=>sum+values.length,0);
    this.active={key,revision:this.revisions.get(key)??0,sources,sourceCursor:0,currentSource:null,currentGroup:null,groupMeshes:[],groupMeshCursor:0,transform:null,grouped:new Map(),jobs:null,jobCursor:0,
      next:Object.assign(new THREE.Group(),{name:`Neighborhood ${key}`}),contourSources,contourSourceCursor:0,contourOffset:0,
      contourOutput:new Float32Array(contourLength),contourCursor:0,contourMin:new THREE.Vector3(Infinity,Infinity,Infinity),
      contourMax:new THREE.Vector3(-Infinity,-Infinity,-Infinity),contoursDone:contourLength===0,ownedResources:[],ownedResourceSet:new Set()};
    this.dirty.delete(key);this.dirtySince.delete(key);
  }

  private prepareMeshes(build:Build){
    let verticesLeft=MAX_PREPARED_VERTICES_PER_UPDATE,indicesLeft=MAX_INDICES_PER_UPDATE,started=0,regenerated=0;
    while((build.transform!==null||build.currentGroup!==null||build.sourceCursor<build.sources.length)&&started<MAX_PREPARED_MESHES_PER_UPDATE){
      if(!build.currentGroup){
        const source=build.sources[build.sourceCursor];
        if(!source)break;
        let group:THREE.Group;
        if(source.descriptor){
          if(source.pendingGroup){group=source.pendingGroup;source.pendingGroup=null;}
          else{if(regenerated>=1)break;group=source.descriptor.createGroup();regenerated++;}
        }else group=source.legacyGroup!;
        build.sourceCursor++;
        build.currentSource=source;build.currentGroup=group;build.groupMeshCursor=0;build.groupMeshes=[];
        group.updateMatrixWorld(true);
        group.traverse(object=>{if(object instanceof THREE.Mesh)build.groupMeshes.push({id:source.id,mesh:object,matrix:object.matrixWorld.clone(),vertices:object.geometry.getAttribute('position').count});});
      }
      if(build.groupMeshCursor>=build.groupMeshes.length){this.finishCurrentGroup(build);continue;}
      if(!build.transform){
        const input=build.groupMeshes[build.groupMeshCursor],mesh=input.mesh!;started++;
        const materials=Array.isArray(mesh.material)?mesh.material:[mesh.material];
        if(materials.length!==1)throw new Error('Building surfaces require one material per authored mesh');
        const resolved=this.material(materials[0]),batch=resolved instanceof THREE.Material?{material:resolved}:resolved;
        build.transform={input,material:batch.material,shadow:mesh.castShadow,
          triangles:(mesh.geometry.index?.count??mesh.geometry.getAttribute('position').count)/3,
          transform:new IncrementalGeometryTransform(mesh.geometry,input.matrix,batch.color)};
      }
      const job=build.transform,transform=job.transform;
      const beforeVertices=transform.processedVertices,beforeIndices=transform.processedIndices;
      const complete=transform.step(verticesLeft,indicesLeft);
      verticesLeft-=transform.processedVertices-beforeVertices;
      indicesLeft-=transform.processedIndices-beforeIndices;
      if(!complete)break;
      const part:PreparedPart={geometry:transform.result!,id:job.input.id,shadow:job.shadow,triangles:job.triangles};
      const list=build.grouped.get(job.material)??[];list.push(part);build.grouped.set(job.material,list);
      job.input.mesh=null;build.groupMeshCursor++;build.transform=null;
      if(verticesLeft===0&&indicesLeft===0)break;
    }
  }

  private finishCurrentGroup(build:Build){
    const source=build.currentSource,group=build.currentGroup;
    if(!source||!group)return;
    build.currentSource=null;build.currentGroup=null;build.groupMeshes=[];build.groupMeshCursor=0;
    // Legacy callers explicitly own their groups. Production descriptors transfer
    // tagged sign resources to the eventual batch before freeing source geometry.
    if(source.descriptor){
      group.traverse(object=>{
        if(!(object instanceof THREE.Mesh))return;
        if(object.userData.ownedMaterial){
          for(const material of Array.isArray(object.material)?object.material:[object.material])this.retainOwnedResource(build,material);
          delete object.userData.ownedMaterial;
        }
        if(object.userData.ownedTexture){this.retainOwnedResource(build,object.userData.ownedTexture as THREE.Texture);delete object.userData.ownedTexture;}
      });
      source.descriptor.disposeGroup(group);
    }
  }

  private retainOwnedResource(build:Build,resource:THREE.Material|THREE.Texture){
    if(build.ownedResourceSet.has(resource))return;
    build.ownedResourceSet.add(resource);build.ownedResources.push(resource);
  }

  private createJobs(build:Build){
    build.jobs=[];
    for(const [material,parts]of build.grouped)build.jobs.push({material,parts,merger:null});
    build.grouped.clear();
  }

  private advanceMergers(build:Build,vertexQuota:number,indexQuota:number){
    let verticesLeft=vertexQuota,indicesLeft=indexQuota;
    while(build.jobCursor<build.jobs!.length){
      const job=build.jobs![build.jobCursor];
      if(!job.merger)job.merger=new IncrementalGeometryMerge(job.parts.map(part=>part.geometry));
      const merger=job.merger;
      const beforeVertices=merger.processedVertices,beforeIndices=merger.processedIndices;
      const complete=merger.step(verticesLeft,indicesLeft);
      verticesLeft-=merger.processedVertices-beforeVertices;indicesLeft-=merger.processedIndices-beforeIndices;
      if(!complete)break;
      const geometry=merger.result!;
      const mesh=new THREE.Mesh(geometry,job.material);mesh.castShadow=job.parts.some(part=>part.shadow);mesh.receiveShadow=true;
      let face=0;mesh.userData.buildingSpans=job.parts.map(part=>{const first=face;face+=part.triangles;return{first,end:face,id:part.id};});
      build.next.add(mesh);
      for(const part of job.parts)part.geometry.dispose();
      job.parts.length=0;job.merger=null;build.jobCursor++;
      if(verticesLeft===0&&indicesLeft===0)break;
    }
  }

  private copyContours(build:Build,floatQuota:number){
    if(build.contoursDone)return;
    let remaining=floatQuota;
    while(remaining>0&&build.contourSourceCursor<build.contourSources.length){
      const source=build.contourSources[build.contourSourceCursor],count=Math.min(remaining,source.length-build.contourOffset);
      build.contourOutput.set(source.subarray(build.contourOffset,build.contourOffset+count),build.contourCursor);
      const vertexStart=build.contourOffset/3,vertexEnd=(build.contourOffset+count)/3;
      for(let i=vertexStart;i<vertexEnd;i++){const offset=i*3,x=source[offset],y=source[offset+1],z=source[offset+2];
        build.contourMin.x=Math.min(build.contourMin.x,x);build.contourMin.y=Math.min(build.contourMin.y,y);build.contourMin.z=Math.min(build.contourMin.z,z);
        build.contourMax.x=Math.max(build.contourMax.x,x);build.contourMax.y=Math.max(build.contourMax.y,y);build.contourMax.z=Math.max(build.contourMax.z,z);}
      build.contourOffset+=count;build.contourCursor+=count;remaining-=count;
      if(build.contourOffset===source.length){build.contourSourceCursor++;build.contourOffset=0;}
    }
    if(build.contourSourceCursor<build.contourSources.length)return;
    build.contoursDone=true;
    const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.BufferAttribute(build.contourOutput,3));
    const center=build.contourMin.clone().add(build.contourMax).multiplyScalar(.5);
    geometry.boundingBox=new THREE.Box3(build.contourMin.clone(),build.contourMax.clone());
    geometry.boundingSphere=new THREE.Sphere(center,build.contourMax.clone().sub(build.contourMin).length()*.5);
    const lines=new THREE.LineSegments(geometry,this.contourMaterial!);lines.visible=this.contoursVisible;lines.raycast=()=>{};build.next.add(lines);
  }

  private ready(build:Build){return build.sourceCursor>=build.sources.length&&build.currentGroup===null&&build.transform===null&&build.jobs!==null&&build.jobCursor>=build.jobs.length&&build.contoursDone;}

  private commit(build:Build){
    // A style switch can happen while this staged build is detached from the
    // scene. Resolve shared render materials again at the atomic commit, while
    // keeping authored per-vertex colors (and all geometry) untouched.
    build.next.traverse(object=>{if(object instanceof THREE.Mesh){const resolved=this.material(object.material);object.material=resolved instanceof THREE.Material?resolved:resolved.material;}});
    if(build.ownedResources.length)build.next.userData.buildingOwnedResources=build.ownedResources;
    const old=this.batches.get(build.key);
    if(old){old.removeFromParent();this.retire(()=>disposeGroup(old));}
    this.root.add(build.next);this.batches.set(build.key,build.next);
    const releases=this.retired.get(build.key);
    if(releases){const pending:RetiredSource[]=[];for(const retired of releases){if(retired.afterRevision<=build.revision)retired.release();else pending.push(retired);}if(pending.length)this.retired.set(build.key,pending);else this.retired.delete(build.key);}
    this.releaseBuildSources(build);this.active=null;
  }

  private cancelBuild(){
    const build=this.active;if(!build)return;
    build.transform?.transform.dispose();
    if(build.currentGroup&&build.currentSource?.descriptor)build.currentSource.descriptor.disposeGroup(build.currentGroup);
    for(const job of build.jobs??[]){if(job.merger&&!job.merger.done)job.merger.dispose();for(const part of job.parts)part.geometry.dispose();}
    for(const parts of build.grouped.values())for(const part of parts)part.geometry.dispose();
    disposeGroup(build.next);disposeOwnedResources(build.ownedResources);this.releaseBuildSources(build);this.active=null;
  }

  private releaseBuildSources(build:Build){
    for(const source of build.sources){source.leases=Math.max(0,source.leases-1);this.releaseSourceIfPossible(source);}
    build.sources=[];
  }

  buildingAt(hit:THREE.Intersection){if(hit.faceIndex===undefined||hit.faceIndex===null)return undefined;const spans=hit.object.userData.buildingSpans as Array<{first:number,end:number,id:number}>|undefined;if(!spans)return undefined;let low=0,high=spans.length-1;while(low<=high){const middle=(low+high)>>>1,span=spans[middle];if(hit.faceIndex<span.first)high=middle-1;else if(hit.faceIndex>=span.end)low=middle+1;else return span.id;}return undefined;}

  clear(){
    this.cancelBuild();
    for(const group of this.batches.values()){group.removeFromParent();this.retire(()=>disposeGroup(group));}
    for(const source of this.sources.values())this.retireSource(source);
    this.batches.clear();this.sources.clear();this.dirty.clear();this.dirtySince.clear();this.revisions.clear();this.updateSequence=0;
    for(const releases of this.retired.values())for(const {release}of releases)release();this.retired.clear();
  }
}

function disposeGroup(group:THREE.Group){
  group.traverse(object=>{if(object instanceof THREE.Mesh||object instanceof THREE.Line)object.geometry.dispose();});
  const resources=group.userData.buildingOwnedResources as Array<THREE.Material|THREE.Texture>|undefined;
  if(resources){delete group.userData.buildingOwnedResources;disposeOwnedResources(resources);}
}
function disposeOwnedResources(resources:readonly(THREE.Material|THREE.Texture)[]){for(const resource of resources)resource.dispose();}

function maxWorldBoundsDiagonalBelow(geometry:THREE.BufferGeometry,matrix:THREE.Matrix4,thresholdSq:number){
  if(geometry.morphAttributes.position?.length)return false;
  // Positions can change without invalidating Three's cached bounding box.
  geometry.computeBoundingBox();
  const local=geometry.boundingBox;
  if(!local||local.isEmpty()||!finitePoint(local.min)||!finitePoint(local.max))return false;
  // Transform a copy: EdgesGeometry is still evaluated against the untouched
  // local geometry and its cached bounds remain in local coordinates.
  const world=local.clone().applyMatrix4(matrix);
  if(world.isEmpty()||!finitePoint(world.min)||!finitePoint(world.max))return false;
  // EdgesGeometry writes transformed endpoints to Float32 attributes before
  // the legacy length test. Account for endpoint rounding at large world
  // coordinates; an uncertain near-threshold bound must take the exact path.
  const maxAbs=Math.max(Math.abs(world.min.x),Math.abs(world.min.y),Math.abs(world.min.z),Math.abs(world.max.x),Math.abs(world.max.y),Math.abs(world.max.z));
  const roundingMargin=maxAbs*2*1.1920928955078125e-7;
  const dx=world.max.x-world.min.x+roundingMargin,dy=world.max.y-world.min.y+roundingMargin,dz=world.max.z-world.min.z+roundingMargin;
  return Number.isFinite(dx)&&Number.isFinite(dy)&&Number.isFinite(dz)&&dx*dx+dy*dy+dz*dz<thresholdSq;
}

function finitePoint(point:THREE.Vector3){return Number.isFinite(point.x)&&Number.isFinite(point.y)&&Number.isFinite(point.z);}
