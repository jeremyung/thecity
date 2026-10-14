import { snapShadowAnchor } from './shadow-anchor';
import { styleWarmup, compileStyleSamples } from './style-warmup';
import * as THREE from 'three/webgpu';
import { color, uniform, mix, positionLocal, smoothstep, mx_noise_float, pass, add, vec3, attribute, positionView, float } from 'three/tsl';
import { bloom } from 'three/addons/tsl/display/BloomNode.js';
import { signedCoCDof } from './postprocessing/SignedCoCDepthOfFieldNode.js';
import { physicalCoCNode } from './postprocessing/physical-coc';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { ArtMaterials, signTexture } from './materials';
import { Kit, disposeGroup } from './kit';
import { RiverWater } from './water';
import { BuildingChunks, buildingChunkKey, type RebuildableBuildingSource } from './building-chunks';
import { Umbrellas } from './umbrellas';
import { CityBuses } from './buses';
import { isRoadVehicle, ROAD_SURFACE_Y, VehicleFleet } from './vehicles';
import { TRAIN_HALF_LENGTH, TrainFleet } from './trains';
import { BuildingAccessPaths } from './building-access';
import { RearExtensions } from './rear-extensions';
import { buildExtendedBuilding } from './extended-building';
import { buildStreetLighting } from './street-lighting';
import { createFoliageGeometry, createTreeTrunkGeometry } from './foliage';
import { signageContexts, type SignageContext } from './signage';
import { addNeighborhoodShrine, addPocketPark, addScrambleCrossing, addShrineGrove, addStationPlaza, openSpaceKind, openSpaces, parkSignature, streetTreePlacements, type PlantedTree } from './parks';
import { createBackdropGeometry } from './backdrop';
import { treeOcclusion } from './tree-occlusion';
import { ARTERIAL_SIDEWALK, bridgeDeckHeight, bridgeLift, bridgeVehiclePose, EXPRESSWAY_DECK, RAIL_DECK, roadWidth, type BridgeSpan } from './road-elevation';
import { RenderQualityController } from './quality';
import { createRenderStartupLifecycle, type RenderStartupLifecycle } from './render-startup';
import { CameraDirector } from '../camera/director';
import { dampFocusDepth, focusDepthFromViewZ } from '../camera/focus';
import type { LensPresetId } from '../camera/lens';
import { LENS_PRESETS, verticalFovDegrees } from '../camera/lens';
import { resolveAudioEventContext } from '../audio/event-context';
import type { World, Style, Point, Road, Building, BuildingOriginalFootprint } from '../types';

export interface StyleSwitchMetrics { target:Style; samples:number; compiledSamples:number; maxSubmitMs:number; prepareMs:number; submitMs:number; waitMs:number; applyMs:number; outcome:'compiling'|'applied'|'superseded'|'failed' }
export interface RenderMetrics { styleSwitch?:StyleSwitchMetrics; fps:number; frameMs:number; calls:number; triangles:number; textures:number; geometries:number; backend:string; pendingBuildings:number; pendingChunks:number }
/** Fields that affect a building's rendered group. Home occupancy is binary
 * because only the seeded window-light subset depends on whether anyone lives there. */
export function buildingRenderSignature(b:Building,original?:BuildingOriginalFootprint,signage?:SignageContext):string {
  const homeOccupancy=(b.kind==='home'||(original&&b.kind==='apartment'))?(b.occupancy>0?'occupied':'vacant'):'-';
  const retained=original?`${original.x}:${original.z}:${original.width}:${original.depth}:${original.rotation}`:'-';
  const signs=signage&&(signage.proximity||signage.feature)?`:s${signage.proximity}${signage.feature??''}`:'';
  return `${b.stage}:${b.floors}:${Math.floor(b.progress*5)}:${b.kind}:${b.name}:${b.appearance_seed}:${b.business_active ?? 'legacy'}:${homeOccupancy}:${b.x}:${b.z}:${b.width}:${b.depth}:${b.rotation}:${retained}${signs}`;
}
type TreeBatch={indices:number[],trees:THREE.InstancedMesh,trunks:THREE.InstancedMesh,center:THREE.Vector3,far:boolean};
const v=new THREE.Vector3(),dummy=new THREE.Object3D();
const personColors=['#e8d9b6','#bc7361','#89a589','#64839d','#e3b35f','#a897b0'].map(c=>new THREE.Color(c));
function random(seed:number){return()=>{seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return(seed>>>0)/4294967296;};}

export class CityRenderer {
  // Explicit development study; never selected by production or a saved world.
  private readonly compactMidriseReview = import.meta.env.DEV && new URLSearchParams(window.location.search).get('midrise') === 'compact';
  readonly scene=new THREE.Scene();
  readonly camera=new THREE.PerspectiveCamera(verticalFovDegrees(LENS_PRESETS.environment.focalLengthMm),1,.5,3500);
  readonly art=new ArtMaterials();
  readonly director:CameraDirector;
  readonly renderer:THREE.WebGPURenderer;
  worldState:World|null=null;
  onPick:((id:number,kind:'building'|'traveler')=>void)|null=null;
  onMetrics:((m:RenderMetrics)=>void)|null=null;
  onError:((error:Error)=>void)|null=null;
  style:Style='anime';
  private styleRequest=0;
  private styleWarmupTail:Promise<void>=Promise.resolve();
  private pendingCompiles=0;
  private styleSwitchMetrics:StyleSwitchMetrics|undefined;
  private retiredResources:Array<()=>void>=[];
  private pipeline:THREE.RenderPipeline|null=null;
  private renderStartup:RenderStartupLifecycle|null=null;
  private scenePass:ReturnType<typeof pass>|null=null;
  private focus=uniform(300);
  private readonly focusTargetView=new THREE.Vector3();
  private focusResetPending=true;
  private opticalScale=uniform(.04);
  private lensFocalM=uniform(.035);
  private lensFNumber=uniform(8);
  private outputHeight=uniform(1);
  private dofEnabled=uniform(1);
  private bokeh=uniform(4);
  private shine: ReturnType<typeof bloom> | null = null;
  private depthOfField:ReturnType<typeof signedCoCDof>|null=null;
  private skyHigh=uniform(new THREE.Color('#9bbac8'));
  private skyLow=uniform(new THREE.Color('#f1dcb9'));
  private skyCloud=uniform(new THREE.Color('#fff5df'));
  private skyCloudShade=uniform(new THREE.Color('#809aaf'));
  private hillColor=uniform(new THREE.Color('#637f85'));
  private sunDisc:THREE.Mesh;
  private moonDisc:THREE.Mesh;
  private sun=new THREE.DirectionalLight('#fff1d1',3);
  private hemi=new THREE.HemisphereLight('#c8dce3','#748998',2);
  private lanternLights:Array<THREE.PointLight>=[];
  private terrain=new THREE.Group();
  private roads=new THREE.Group();
  private station=new THREE.Group();
  private chunks=new BuildingChunks(m=>this.art.buildingBatch(m),dispose=>this.retire(dispose),this.art.contours);
  private buildings=new Map<number,{source:RebuildableBuildingSource,signature:string}>();
  private water:RiverWater|null=null;
  private riverHalfWidth=32;
  private bridgeSpans:BridgeSpan[]=[];
  private treeBatches:TreeBatch[]=[];
  private treeNearGeometry:THREE.BufferGeometry|null=null;
  private treeFarGeometry:THREE.BufferGeometry|null=null;
  private treeTrunkGeometry:THREE.BufferGeometry|null=null;
  // Street trees, pocket parks and the shrine grove follow roads and parcels,
  // so they rebuild separately from the seeded terrain groves.
  private parks=new THREE.Group();
  private parkBatches:TreeBatch[]=[];
  private parkKey='';
  /** Latest per-building signage context; part of each building's render signature. */
  private signage=new Map<number,SignageContext>();
  private treePoints:Array<{x:number,z:number,s:number}> = [];
  private buses=new CityBuses(this.art,release=>this.retire(release));
  private accessPaths=new BuildingAccessPaths(this.art,release=>this.retire(release));
  private rearExtensions=new RearExtensions(this.art,release=>this.retire(release),building=>{
    const current=this.buildings.get(building.id);
    const original=this.worldState?.building_original_footprints?.find(form=>form.building_id===building.id);
    return !!current&&current.signature===buildingRenderSignature(building,original,this.signage.get(building.id))&&this.chunks.isSourceVisible(building.id,current.source);
  });
  private umbrellas=new Umbrellas(2000,hex=>this.art.get(hex));
  private umbrellaTuples=new Float32Array(2000*5);
  private umbrellaCount=0;
  private pedestrians:THREE.InstancedMesh;
  private heads:THREE.InstancedMesh;
  private fleet=new VehicleFleet(this.art);
  private lastActorFrame=0;
  private trainFleet=new TrainFleet(this.art);
  private previous=new Map<number,{x:number,z:number,h:number,k:number}>();
  private current=new Map<number,{x:number,z:number,h:number,k:number}>();
  private received=performance.now();
  private interval=50;
  private lastTick=0;
  private motionTick=0;
  private motionDelta=.05;
  private hour=6;
  private preview:number|null=null;
  private frame=0;
  private disposed=false;
  private lastFrame=0;
  private frames:number[]=[];
  private roadSignature='';
  private terrainSeed=-1;
  private resize:ResizeObserver;
  private pointerStart={x:0,y:0};
  private selected:number|null=null;
  private highlight:THREE.Mesh;
  private daylight=1;
  private rain=0;
  private focusEnabled=true;
  private quality:'high'|'balanced'='high';
  private qualityController=new RenderQualityController();
  private roadGroups=new Map<string,THREE.Group>();
  private actorIds=new Map<THREE.Object3D,number[]>();
  private worldTick=-1;
  private waterTime=0;
  private rainTime=uniform(0);
  private rainOpacity=uniform(0);
  private rainLines:THREE.LineSegments;
  private objectQueue=new Map<number,{building:Building,signature:string,source:RebuildableBuildingSource}>();
  constructor(readonly element:HTMLElement,forceWebGL=false){
    this.renderer=new THREE.WebGPURenderer({antialias:true,samples:4,forceWebGL,powerPreference:'high-performance'});
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio,1.6));
    this.renderer.shadowMap.enabled=true;this.renderer.shadowMap.type=THREE.PCFSoftShadowMap;
    this.renderer.toneMapping=THREE.ACESFilmicToneMapping;this.renderer.toneMappingExposure=1.05;
    element.appendChild(this.renderer.domElement);
    this.director=new CameraDirector(this.camera,this.renderer.domElement);
    this.scene.fog=new THREE.FogExp2('#bfcbd0',.0008);
    this.scene.add(this.terrain,this.roads,this.parks,this.station,this.chunks.root,this.umbrellas.root,this.buses.root,this.fleet.root,this.trainFleet.root,this.accessPaths.root,this.rearExtensions.root,this.hemi,this.sun);
    this.sun.castShadow=true;this.sun.shadow.mapSize.set(2048,2048);
    this.sun.shadow.camera.left=-230;this.sun.shadow.camera.right=230;this.sun.shadow.camera.top=230;this.sun.shadow.camera.bottom=-230;
    this.sun.shadow.camera.near=1;this.sun.shadow.camera.far=1100;this.sun.shadow.bias=-.0002;this.sun.shadow.normalBias=.35;
    this.scene.add(this.sun.target);
    for(let i=0;i<8;i++){const light=new THREE.PointLight('#ffd6a0',0,22,2);this.lanternLights.push(light);this.scene.add(light);}
    const sky=new THREE.Mesh(new THREE.SphereGeometry(1800,32,20),new THREE.MeshBasicNodeMaterial({side:THREE.BackSide,depthWrite:false}));
    const direction=positionLocal.normalize();
    const skyBase=mix(this.skyLow,this.skyHigh,smoothstep(-.05,.75,direction.y));
    // Stable, layered cloud masses: broad silhouettes, broken edges, and cool
    // undersides read as an environmental painting rather than a flat sky wash.
    const cloudPoint=direction.mul(vec3(8,23,8)).add(vec3(this.art.time.mul(.0015),0,this.art.time.mul(.0005)));
    const cloudField=mx_noise_float(cloudPoint).mul(.64)
      .add(mx_noise_float(cloudPoint.mul(2.1)).mul(.25))
      .add(mx_noise_float(cloudPoint.mul(4.3)).mul(.11));
    const clouds=smoothstep(-.035,.16,cloudField)
      .mul(smoothstep(.015,.09,direction.y));
    const cloudRelief=smoothstep(-.04,.2,cloudField.add(direction.y.mul(.13)));
    const cloudPaint=mix(this.skyCloudShade,this.skyCloud,cloudRelief);
    (sky.material as THREE.MeshBasicNodeMaterial).colorNode=mix(skyBase,cloudPaint,clouds);
    (sky.material as THREE.MeshBasicNodeMaterial).fog=false;sky.renderOrder=-10;this.scene.add(sky);
    this.sunDisc=new THREE.Mesh(new THREE.SphereGeometry(15,16,8),new THREE.MeshBasicNodeMaterial({color:'#ffe5bc',fog:false}));this.scene.add(this.sunDisc);
    this.moonDisc=new THREE.Mesh(new THREE.SphereGeometry(7,16,8),new THREE.MeshBasicNodeMaterial({color:'#cbdbe2',fog:false}));this.scene.add(this.moonDisc);
    const rainPositions:number[]=[],rainPhases:number[]=[];const rainRandom=random(123);
    for(let i=0;i<700;i++){
      const x=rainRandom()*180-90,z=rainRandom()*180-90,phase=rainRandom()*90;
      rainPositions.push(x,0,z,x+.06,1.1,z);rainPhases.push(phase,phase);
    }
    const rainGeo=new THREE.BufferGeometry();
    rainGeo.setAttribute('position',new THREE.Float32BufferAttribute(rainPositions,3));
    rainGeo.setAttribute('rainPhase',new THREE.Float32BufferAttribute(rainPhases,1));
    const rainMat=new THREE.LineBasicNodeMaterial({color:'#b8d0d4',transparent:true,depthWrite:false});
    // Both endpoints wrap together. Wrapping each endpoint separately made a
    // short drop stretch through the entire 90 m volume at its wrap boundary.
    const rainHeight=float(attribute('rainPhase','float')).sub(this.rainTime.mul(16)).mod(90);
    rainMat.positionNode=vec3(positionLocal.x,positionLocal.y.add(rainHeight),positionLocal.z);
    rainMat.opacityNode=this.rainOpacity.mul(smoothstep(8,28,positionView.z.negate()));
    this.rainLines=new THREE.LineSegments(rainGeo,rainMat);this.rainLines.visible=false;
    // Shader displacement moves drops beyond the small undeformed geometry.
    rainGeo.boundingSphere=new THREE.Sphere(new THREE.Vector3(0,45,0),140);
    this.scene.add(this.rainLines);
    this.pedestrians=this.instances(new THREE.CapsuleGeometry(.33,.9,3,5),this.art.get('#e7e5d9'),2000);
    this.heads=this.instances(new THREE.SphereGeometry(.28,6,4),this.art.get('#dfbe97'),2000);
    this.highlight=new THREE.Mesh(new THREE.RingGeometry(8,8.2,64),new THREE.MeshBasicNodeMaterial({color:'#f2d79b',side:THREE.DoubleSide}));this.highlight.rotation.x=-Math.PI/2;this.highlight.visible=false;this.scene.add(this.highlight);
    this.resize=new ResizeObserver(()=>this.setSize());this.resize.observe(element);
    this.renderer.domElement.addEventListener('pointerdown',this.pointerDown);
    this.renderer.domElement.addEventListener('pointerup',this.pointerUp);
  }
  private instances(geo:THREE.BufferGeometry,mat:THREE.Material,count:number){const mesh=new THREE.InstancedMesh(geo,mat,count);mesh.count=0;mesh.castShadow=true;mesh.receiveShadow=true;mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);mesh.frustumCulled=false;this.scene.add(mesh);return mesh;}
  async start(initialStyle:Style='anime'){
    this.applyStyle(initialStyle);
    this.renderer.domElement.dataset.graphicsPhase='device-initialization';
    try { await this.renderer.init(); } catch(error) {
      this.renderer.domElement.dataset.graphicsPhase='device-error';throw error;
    }
    this.renderer.domElement.dataset.graphicsPhase='pipeline-setup';
    // A slow GPU initialization can outlive unmount or hot reload. Never
    // restart an animation loop or allocate a pipeline for a retired scene.
    if(this.disposed){this.renderer.dispose();return;}
    this.setSize();this.buildPipeline();
    const startup=this.renderStartup=createRenderStartupLifecycle({
      stopLoop:()=>this.renderer.setAnimationLoop(null),
      setPhase:phase=>{this.renderer.domElement.dataset.graphicsPhase=phase;},
      onError:error=>this.onError?.(error),
    });
    this.renderer.setAnimationLoop(()=>startup.runFrame(()=>this.animate()));
    this.renderer.domElement.dataset.graphicsPhase='loop-started';
    await startup.promise;
  }
  private buildPipeline(){
    this.disposePipeline();
    this.scenePass=pass(this.scene,this.camera,{samples:4});
    const sceneColor=this.scenePass.getTextureNode('output');
    const depth=this.scenePass.getViewZNode();
    const signedCoC=physicalCoCNode({viewZ:depth,focusDistanceM:this.focus,
      focalLengthM:this.lensFocalM,fNumber:this.lensFNumber,opticalScale:this.opticalScale,
      outputHeightPx:this.outputHeight,maxGatherRadiusPx:this.bokeh,enabled:this.dofEnabled});
    const focus=this.depthOfField=signedCoCDof(sceneColor,signedCoC,this.bokeh);
    const shine=bloom(sceneColor,.1,.5,.9);this.shine=shine;
    this.pipeline=new THREE.RenderPipeline(this.renderer);
    this.pipeline.outputNode=add(focus as unknown as THREE.Node<'vec4'>, shine as unknown as THREE.Node<'vec4'>);
  }
  private disposePipeline(){this.pipeline?.dispose();this.depthOfField?.dispose();this.shine?.dispose();this.scenePass?.dispose();this.pipeline=null;this.depthOfField=null;this.shine=null;this.scenePass=null;}
  async setStyle(style:Style){
    const request=++this.styleRequest;
    const previous=this.styleWarmupTail;
    let release!:()=>void;
    this.styleWarmupTail=new Promise<void>(resolve=>{release=resolve;});
    await previous;
    try{
      if(this.disposed||request!==this.styleRequest||style===this.style)return;
      // Serialize warmup and skip superseded requests before allocating samples.
      const prepareStart=performance.now();
      const prepared=styleWarmup(this.scene,m=>this.art.variant(m,style));
      const timing:StyleSwitchMetrics={target:style,samples:prepared.scene.children.filter(o=>o instanceof THREE.Mesh).length,prepareMs:performance.now()-prepareStart,compiledSamples:0,maxSubmitMs:0,submitMs:0,waitMs:0,applyMs:0,outcome:'compiling'};
      this.styleSwitchMetrics=timing;
      this.pendingCompiles++;
      try{
        await compileStyleSamples(prepared.scene,
          scene=>this.renderer.compileAsync(scene,this.camera,prepared.targetScene),
          ()=>!this.disposed&&request===this.styleRequest,timing);
      }catch(error){timing.outcome='failed';throw error;}finally{
        prepared.dispose();this.pendingCompiles--;
        if(!this.pendingCompiles){const retired=this.retiredResources.splice(0);for(const dispose of retired)dispose();}
      }
      if(this.disposed||request!==this.styleRequest){timing.outcome='superseded';return;}
      const applyStart=performance.now();
      this.applyStyle(style);
      timing.applyMs=performance.now()-applyStart;timing.outcome='applied';
    }finally{release();}
  }
  private applyStyle(style:Style){
    this.style=style;this.art.setStyle(style);this.chunks.setContoursVisible(style==='anime');this.scene.traverse(o=>{if(o instanceof THREE.Mesh)o.material=Array.isArray(o.material)?o.material.map(m=>this.art.variant(m)):this.art.variant(o.material);});
    this.opticalScale.value=style==='miniature'?.01:.04;this.bokeh.value=style==='miniature'?8:4;
  }
  private retire(dispose:()=>void){if(this.pendingCompiles)this.retiredResources.push(dispose);else dispose();}
  private retireGroup(group:THREE.Group){group.removeFromParent();this.retire(()=>disposeGroup(group));}
  setPreview(hour:number|null){this.preview=hour;}
  setLens(id:LensPresetId,takeManualControl=true){this.director.setLens(id,takeManualControl);const lens=LENS_PRESETS[id];this.lensFocalM.value=lens.focalLengthMm/1000;this.lensFNumber.value=lens.fNumber;}
  setFocus(value:boolean){this.focusEnabled=value;this.dofEnabled.value=value?1:0;}
  setQuality(quality:'high'|'balanced'){this.quality=quality;this.qualityController.setProfile(quality);this.applyQuality();}
  private applyQuality(){
    const level=this.qualityController.level,high=this.quality==='high';
    // Reduce reflection/shadow cost before lowering the focal scene's resolution.
    const ratio=(high?[1.6,1.6,1.25]:[1,1,.8])[level];
    const shadow=(high?[2048,1536,1024]:[1024,768,512])[level];
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio,ratio));
    this.sun.shadow.mapSize.set(shadow,shadow);
    this.water?.setReflectionScale((high?[.35,.23,.16]:[.25,.19,.15])[level]);
    this.setSize();
  }
  setSelected(id:number|null){this.selected=id;}
  private setSize(){const w=this.element.clientWidth,h=this.element.clientHeight;if(!w||!h)return;this.camera.aspect=w/h;this.camera.updateProjectionMatrix();this.renderer.setSize(w,h);this.outputHeight.value=this.renderer.getDrawingBufferSize(new THREE.Vector2()).y;}
  setWorld(world:World,replaced=false){
    this.worldState=world;this.hour=world.minute_of_day/60;this.rain=world.weather?.intensity??0;
    if(replaced||this.terrainSeed!==world.seed||world.tick<this.worldTick){this.resetWorld();this.buildTerrain(world);this.terrainSeed=world.seed;}
    this.worldTick=world.tick;this.director.world(world);this.buses.setWorld(world);this.accessPaths.setWorld(world);this.rearExtensions.setWorld(world);
    const bridgeNodes=new Map(world.nodes.map(n=>[n.id,n]));this.bridgeSpans=world.roads.filter(r=>r.kind==='bridge'&&r.stage==='open').flatMap(r=>{const a=bridgeNodes.get(r.from),b=bridgeNodes.get(r.to);return a&&b?[{a,b}]:[];});
    const signatures=world.roads.map(r=>`${r.id}:${r.stage}:${Math.floor(r.progress*12)}`).join('|')+`-g${world.ground?.length??0}-`+world.projects.filter(p=>['road','rail','bridge','station'].includes(p.kind)).map(p=>`${p.id}:${p.stage}:${Math.floor(p.progress*12)}`).join('|');
    if(signatures!==this.roadSignature){this.roadSignature=signatures;this.rebuildRoads(world);}
    const parkKey=`${signatures}#${parkSignature(world)}`;
    if(parkKey!==this.parkKey){this.parkKey=parkKey;this.rebuildParks(world);}
    const ids=new Set(world.buildings.map(b=>b.id));
    for(const id of this.buildings.keys())if(!ids.has(id)){this.chunks.remove(id);this.buildings.delete(id);}
    for(const id of this.objectQueue.keys())if(!ids.has(id))this.objectQueue.delete(id);
    const originals=new Map((world.building_original_footprints??[]).map(original=>[original.building_id,original]));
    const signage=this.signage=signageContexts(world);
    for(const b of world.buildings){
      const original=originals.get(b.id);
      const context=signage.get(b.id);
      const signature=buildingRenderSignature(b,original,context);
      if(this.buildings.get(b.id)?.signature===signature){this.objectQueue.delete(b.id);continue;}
      // One pending update per authoritative building. Repeated snapshots replace
      // its data without appending duplicate closures or starving older parcels.
      const snapshot:Building={...b,history:[...b.history]};
      const originalSnapshot=original?{...original}:undefined;
      const source:RebuildableBuildingSource={
        createGroup:()=>{const group=buildExtendedBuilding(snapshot,originalSnapshot,this.art,{compactMidrise:this.compactMidriseReview,signage:context});group.position.set(snapshot.x,0,snapshot.z);group.rotation.y=snapshot.rotation;return group;},
        disposeGroup:group=>disposeGroup(group),
      };
      this.objectQueue.set(b.id,{building:snapshot,signature,source});
    }
    this.updateTrees(world);
  }
  setMovement(data:Float32Array,tick:number,minute:number){
    this.previous=this.current;this.current=new Map();
    for(let i=0;i<data.length;i+=5)this.current.set(data[i],{x:data[i+1],z:data[i+2],h:data[i+3],k:data[i+4]});
    const now=performance.now();this.interval=Math.max(20,Math.min(250,now-this.received));this.received=now;this.motionDelta=Math.max(.05,(tick-this.motionTick)*.05);this.motionTick=tick;this.hour=minute/60;
    this.director.setPositions(data);
  }
  private resetWorld(){
    this.focusResetPending=true;
    this.umbrellas.reset();this.buses.reset();this.fleet.reset();this.trainFleet.reset();this.accessPaths.reset();this.rearExtensions.reset();this.umbrellaCount=0;this.qualityController.reset();
    this.chunks.clear();
    this.buildings.clear();
    this.clearParks();this.parkKey='';
    this.retireGroup(this.terrain);const near=this.treeNearGeometry,far=this.treeFarGeometry,trunk=this.treeTrunkGeometry;this.retire(()=>{near?.dispose();far?.dispose();trunk?.dispose();});this.treeNearGeometry=null;this.treeFarGeometry=null;this.treeTrunkGeometry=null;this.treeBatches=[];this.terrain=new THREE.Group();this.scene.add(this.terrain);
    this.retireGroup(this.roads);this.roads=new THREE.Group();this.scene.add(this.roads);
    const oldWater=this.water;oldWater?.mesh.removeFromParent();this.retire(()=>oldWater?.dispose());this.water=null;this.waterTime=0;this.motionTick=0;this.motionDelta=.05;this.received=performance.now();this.current.clear();this.previous.clear();this.objectQueue.clear();this.roadSignature='';this.selected=null;this.treePoints=[];
  }
  private buildTerrain(world:World){
    const river=world.terrain.river;const halfWidth=this.riverHalfWidth=world.terrain.river_half_width??32;
    const land=new THREE.MeshStandardNodeMaterial({roughness:1});const meadow=mx_noise_float(positionLocal.mul(.024)).mul(.5).add(.5);
    const scrub=mx_noise_float(positionLocal.mul(.007)).mul(.5).add(.5);
    land.colorNode=mix(mix(color('#577d61'),color('#94ab6d'),meadow),color('#c2bc8c'),smoothstep(.59,.79,scrub).mul(.55)).mul(mx_noise_float(positionLocal.mul(.6)).mul(.075).add(.965));
    for(const side of [-1,1]){
      const shape=new THREE.Shape();const edge=side<0?-420:420;shape.moveTo(edge,-420);
      const pts=side<0?river:[...river].reverse();
      if(side<0){for(const p of pts)shape.lineTo(p.x-halfWidth,p.z);shape.lineTo(edge,420);}else{shape.lineTo(edge,420);for(const p of pts)shape.lineTo(p.x+halfWidth,p.z);}
      shape.closePath();const geo=new THREE.ShapeGeometry(shape);geo.rotateX(Math.PI/2);
      // Shape XY -> XZ with positive z, normals need both sides for this winding.
      land.side=THREE.DoubleSide;land.userData.ownedMaterial=true;const mesh=new THREE.Mesh(geo,land);mesh.userData.ownedMaterial=true;mesh.receiveShadow=true;this.terrain.add(mesh);
    }
    const visibleRiver=[{x:river[0].x,z:-1400},...river,{x:river.at(-1)!.x,z:1400}];
    this.water=new RiverWater(visibleRiver,halfWidth);this.scene.add(this.water.mesh);this.applyQuality();
    const backdropMaterial=new THREE.MeshStandardNodeMaterial({roughness:1});
    const outerLandscape=smoothstep(420,1000,positionLocal.x.abs().max(positionLocal.z.abs()));
    const woodland=mix(color('#526e60'),color('#78816b'),mx_noise_float(positionLocal.mul(.012)).mul(.5).add(.5));
    backdropMaterial.colorNode=mix(land.colorNode!,woodland,outerLandscape);
    const backdrop=new THREE.Mesh(createBackdropGeometry(world.seed,river,halfWidth),backdropMaterial);
    backdrop.userData.ownedMaterial=true;backdrop.receiveShadow=true;this.terrain.add(backdrop);
    const bankKit=new Kit();
    for(let i=0;i<river.length-1;i++){
      const a=river[i],b=river[i+1],dx=b.x-a.x,dz=b.z-a.z,len=Math.hypot(dx,dz),angle=Math.atan2(dx,dz);
      for(const side of [-1,1]){
        bankKit.box((a.x+b.x)/2+side*(halfWidth+2),-.45,(a.z+b.z)/2,3,.9,len,this.art.get('#9b9e8b'),angle);
        bankKit.box((a.x+b.x)/2+side*(halfWidth+8),.04,(a.z+b.z)/2,5,.12,len,this.art.get('#c2c2a6'),angle);
      }
    }
    // Low irregular stone shoulders break the ruler-straight bank edge.
    for(let z=-395;z<400;z+=4.5)for(const side of [-1,1]){const stone=new THREE.IcosahedronGeometry(1,0),scale=1.2+(Math.sin(z*7+world.seed)*.5+.5)*1.5;stone.scale(scale,.65,scale*.8);stone.rotateY(z);stone.translate(this.riverX(river,z)+side*halfWidth,-.2,z);bankKit.add(stone,this.art.get(z%9<4.5?'#8c9b8b':'#b0b2a0'));}
    this.terrain.add(bankKit.finish(false));
    const rng=random(world.seed+937);
    const positions:Array<{x:number,z:number,s:number}>=[];
    // Groves leave clear meadows between them; the city gradually clears real
    // development footprints, while riverbank vegetation stays recognizable.
    for(let grove=0;grove<26;grove++){const cx=rng()*740-370,cz=rng()*740-370,radius=24+rng()*40;
      for(let tree=0;tree<24;tree++){const a=rng()*Math.PI*2,r=Math.sqrt(rng())*radius,x=cx+Math.cos(a)*r,z=cz+Math.sin(a)*r;
        if(Math.abs(x)>397||Math.abs(z)>397||Math.abs(x-this.riverX(river,z))<halfWidth+17)continue;
        positions.push({x,z,s:1.5+rng()*2.8});
      }
    }
    // A line of blossoms along the river creates a recognizable foreground rhythm.
    for(let z=-330;z<350;z+=24)positions.push({x:this.riverX(river,z)+halfWidth+17,z,s:2.6});
    for(let i=0;i<360;i++){const z=rng()*800-400,side=rng()>.5?1:-1;positions.push({x:this.riverX(river,z)+side*(halfWidth+21+rng()*28),z,s:1.8+rng()*2.5});}
    this.treePoints=positions;
    // Small overlapping leaf masses give a broken canopy silhouette at both
    // observation distances, rather than a row of identical smooth spheres.
    const foliage=createFoliageGeometry('near');
    this.treeNearGeometry=foliage;this.treeFarGeometry=createFoliageGeometry('far');
    this.treeTrunkGeometry=createTreeTrunkGeometry();
    this.treeBatches=this.createTreeBatches(positions,(p,local)=>Math.abs(p.x-this.riverX(river,p.z)-halfWidth-17)<.01?['#e5bdba','#edcfc4','#cfa9b0'][local%3]:['#758a67','#899b70','#607e67','#a0aa7a'][local%4]);
    for(const batch of this.treeBatches)this.terrain.add(batch.trees,batch.trunks);
    this.updateTrees(world);
    this.buildMountains(rng);
    const detail=new Kit();
    // Park benches along the riverside walk.
    for(let z=-150;z<300;z+=80){const x=this.riverX(river,z)+halfWidth+12;detail.box(x,.9,z,1.2,.15,3.1,this.art.get('#7a6754'));detail.box(x+.5,1.3,z,.12,.8,3.1,this.art.get('#7a6754'));}
    this.terrain.add(detail.finish(false));
  }
  private riverX(points:Point[],z:number){for(let i=0;i<points.length-1;i++)if(z<=points[i+1].z){const t=(z-points[i].z)/(points[i+1].z-points[i].z);return THREE.MathUtils.lerp(points[i].x,points[i+1].x,t);}return points.at(-1)!.x;}
  private buildMountains(rng:()=>number){
    for(let ring=0;ring<3;ring++){
      const vertices:number[]=[],indices:number[]=[];
      const phase=rng()*Math.PI*2;
      for(let i=0;i<=96;i++){const a=i/96*Math.PI*2,r=2500+ring*250,h=160+ring*32+Math.sin(a*5+phase)*38+Math.sin(a*11+phase*.3)*17+Math.sin(a*23+phase)*7;vertices.push(Math.cos(a)*r,-8,Math.sin(a)*r,Math.cos(a)*r,h,Math.sin(a)*r);if(i<96){const n=i*2;indices.push(n,n+2,n+1,n+1,n+2,n+3);}}
      const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(vertices,3));g.setIndex(indices);g.computeVertexNormals();const m=new THREE.MeshBasicNodeMaterial({side:THREE.DoubleSide});m.colorNode=mix(this.hillColor,mix(this.skyLow,this.skyHigh,.55),.2+ring*.24).mul(mx_noise_float(positionLocal.mul(.025)).mul(.055).add(.97));const mountain=new THREE.Mesh(g,m);mountain.userData.ownedMaterial=true;this.terrain.add(mountain);
    }
  }
  /** Instanced trees in 200 m cells so each cell can switch foliage detail by camera distance. */
  private createTreeBatches<T extends {x:number,z:number}>(points:readonly T[],colorOf:(p:T,local:number)=>string):TreeBatch[]{
    if(!this.treeNearGeometry||!this.treeTrunkGeometry)return [];
    const cells=new Map<string,number[]>();for(let i=0;i<points.length;i++){const p=points[i],key=`${Math.floor((p.x+400)/200)}:${Math.floor((p.z+400)/200)}`;const list=cells.get(key)??[];list.push(i);cells.set(key,list);}
    const batches:TreeBatch[]=[];
    for(const indices of cells.values()){
      const trunks=new THREE.InstancedMesh(this.treeTrunkGeometry,this.art.get('#746b5c'),indices.length),trees=new THREE.InstancedMesh(this.treeNearGeometry,this.art.get('#fffffe',.96),indices.length),center=new THREE.Vector3();
      trunks.instanceMatrix.setUsage(THREE.DynamicDrawUsage);trees.instanceMatrix.setUsage(THREE.DynamicDrawUsage);trees.castShadow=true;trunks.castShadow=true;
      for(let local=0;local<indices.length;local++){const p=points[indices[local]];center.x+=p.x;center.z+=p.z;trees.setColorAt(local,new THREE.Color(colorOf(p,local)));}
      center.divideScalar(indices.length);batches.push({indices,trees,trunks,center,far:false});
    }
    return batches;
  }
  private clearParks(){
    // Instanced trees share the terrain's foliage and trunk geometry; detach
    // them so retiring the park group disposes only park-owned geometry.
    for(const batch of this.parkBatches){batch.trees.removeFromParent();batch.trunks.removeFromParent();batch.trees.dispose();batch.trunks.dispose();}
    this.parkBatches=[];this.retireGroup(this.parks);this.parks=new THREE.Group();this.parks.name='parks';this.scene.add(this.parks);
  }
  private rebuildParks(world:World){
    this.clearParks();
    const kit=new Kit(),planted:PlantedTree[]=[...streetTreePlacements(world)];
    for(const parcel of openSpaces(world)){
      const kind=openSpaceKind(parcel);
      planted.push(...(kind==='plaza'?addStationPlaza:kind==='shrine'?addNeighborhoodShrine:addPocketPark)(kit,parcel,this.art));
    }
    addScrambleCrossing(kit,world,this.art);
    for(const polygon of world.terrain.protected)planted.push(...addShrineGrove(kit,polygon,this.art));
    this.parks.add(kit.finish());
    this.parkBatches=this.createTreeBatches(planted,p=>p.color);
    for(const batch of this.parkBatches){
      for(let local=0;local<batch.indices.length;local++){const i=batch.indices[local],p=planted[i];dummy.position.set(p.x,0,p.z);dummy.rotation.set(0,i*2.39996,0);dummy.scale.setScalar(p.s/1.8);dummy.updateMatrix();batch.trees.setMatrixAt(local,dummy.matrix);batch.trunks.setMatrixAt(local,dummy.matrix);}
      batch.trees.computeBoundingSphere();batch.trunks.computeBoundingSphere();
      this.parks.add(batch.trees,batch.trunks);
    }
  }
  private updateTrees(world:World){
    const hides=treeOcclusion(world,this.treePoints);
    for(const batch of this.treeBatches){for(let local=0;local<batch.indices.length;local++){
      const i=batch.indices[local];
      const p=this.treePoints[i];
      const hidden=hides(p);
      dummy.position.set(p.x,0,p.z);dummy.rotation.set(0,i*2.39996,0);dummy.scale.setScalar(hidden?0:p.s/1.8);dummy.updateMatrix();batch.trunks.setMatrixAt(local,dummy.matrix);batch.trees.setMatrixAt(local,dummy.matrix);
    }
    batch.trees.instanceMatrix.needsUpdate=true;batch.trunks.instanceMatrix.needsUpdate=true;batch.trees.computeBoundingSphere();batch.trunks.computeBoundingSphere();}
  }
  private updateTreeDetail(){
    if(!this.treeNearGeometry||!this.treeFarGeometry)return;
    for(const batch of [...this.treeBatches,...this.parkBatches]){const distance=this.camera.position.distanceTo(batch.center),far=batch.far?distance>280:distance>330;if(far!==batch.far){batch.far=far;batch.trees.geometry=far?this.treeFarGeometry:this.treeNearGeometry;batch.trees.computeBoundingSphere();}}
  }
  private rebuildRoads(world:World){
    this.retireGroup(this.roads);this.roads=new THREE.Group();this.scene.add(this.roads);this.roadGroups.clear();
    const nodes=new Map(world.nodes.map(n=>[n.id,n]));
    const kit=new Kit();
    // Continuous pedestrian-scale river railings. Reserve openings for real
    // bridges, including construction, and rebuild when infrastructure changes.
    const bankMetal=this.art.get('#73827b',.66);
    for(const side of [-1,1])for(let z=-390;z<390;z+=6){
      const a={x:this.riverX(world.terrain.river,z)+side*(this.riverHalfWidth+5.4),z};
      const b={x:this.riverX(world.terrain.river,z+6)+side*(this.riverHalfWidth+5.4),z:z+6};
      const middle={x:(a.x+b.x)/2,z:z+3};
      const opening=world.roads.some(road=>{if(road.kind!=='bridge')return false;const from=nodes.get(road.from),to=nodes.get(road.to);return from&&to&&distanceToSegment(middle,from,to)<10;});
      if(opening)continue;
      const length=Math.hypot(b.x-a.x,6),angle=Math.atan2(b.x-a.x,6);
      kit.box(a.x,.73,a.z,.08,1.46,.08,bankMetal);
      for(const y of [.64,1.28])kit.box(middle.x,y,middle.z,.055,.065,length,bankMetal,angle);
    }
    // Developed master-plan blocks are paved urban ground (concrete, gravel and
    // service yards) rather than lawn; parcel paving and buildings sit above.
    const groundTones=['#a7a59a','#9d9c92','#aaa69a','#a19f95'];
    for(const patch of world.ground??[]){
      if(patch.points.length<3)continue;
      const shape=new THREE.Shape(patch.points.map(([x,z])=>new THREE.Vector2(x,-z)));
      const geometry=new THREE.ShapeGeometry(shape);geometry.rotateX(-Math.PI/2);geometry.translate(0,.035,0);
      kit.add(geometry,this.art.get(groundTones[patch.block%groundTones.length]));
    }
    for(const road of world.roads){const a=nodes.get(road.from),b=nodes.get(road.to);if(a&&b)this.roadKit(kit,a,b,road);}
    const junctions=new Map<number,{count:number,radius:number}>();
    for(const road of world.roads.filter(r=>r.stage==='open'&&r.kind!=='rail'&&r.kind!=='bridge'&&r.kind!=='expressway'))for(const id of [road.from,road.to]){const entry=junctions.get(id)??{count:0,radius:0};entry.count++;entry.radius=Math.max(entry.radius,roadWidth(road.kind)/2+.3);junctions.set(id,entry);}
    for(const [id,junction]of junctions){const node=nodes.get(id);if(node&&junction.count>1)kit.cylinder(node.x,.33,node.z,junction.radius,.07,this.art.get('#747f7b'),24);}
    for(const project of world.projects.filter(p=>p.kind==='road'&&p.progress<1)){
      const a=nodes.get(project.from!),b=nodes.get(project.to!);if(!a||!b)continue;
      if(world.roads.some(r=>r.from===a.id&&r.to===b.id&&r.stage!=='open'))continue;
      this.roadKit(kit,a,b,{id:project.id,from:a.id,to:b.id,kind:world.roads.find(r=>(r.from===a.id&&r.to===b.id)||(r.from===b.id&&r.to===a.id))?.kind??(Math.abs(a.x-b.x)>100?'bridge':'local'),stage:project.stage,progress:project.progress,connected:false,length:Math.hypot(a.x-b.x,a.z-b.z),lanes:1});
    }
    for(const node of world.nodes.filter(n=>n.kind==='market'||n.kind==='station')){for(let stripe=-3;stripe<=3;stripe++)kit.box(node.x+stripe*1.1,.37,node.z, .65,.025,6,this.art.get('#dedaca'));}
    this.roads.add(kit.finish(),buildStreetLighting(world,this.art));
    this.retireGroup(this.station);this.station=new THREE.Group();this.scene.add(this.station);
    const stationNode=world.nodes.find(n=>n.kind==='station');
    const stationProject=world.projects.find(p=>p.kind==='station');
    if(stationNode){
      const s=new Kit(),rail=world.roads.find(r=>r.kind==='rail'&&(r.from===stationNode.id||r.to===stationNode.id));
      const other=rail?nodes.get(rail.from===stationNode.id?rail.to:rail.from):null;
      const bearing=other?Math.atan2(other.x-stationNode.x,other.z-stationNode.z):0;
      // The 47 m train stops alongside a platform aligned to its real corridor.
      // Local z points down the line; the platform stays beside the tracks.
      // Platform length follows the consist so the whole train sits alongside it.
      const platformMid=TRAIN_HALF_LENGTH,platformLen=TRAIN_HALF_LENGTH*2;
      s.box(5.2,1,platformMid,5.2,2,platformLen,this.art.get('#b9bbb1'));
      s.box(2.9,2.08,platformMid,.45,.1,platformLen-1,this.art.get('#d3b962'));
      if((stationProject?.progress??1)>.2){
        s.box(5.5,6,platformMid,7,.22,platformLen+2,this.art.get('#546d6c'));
        for(let d=3;d<=platformLen-3;d+=10.5){s.cylinder(6.7,3.95,d,.14,3.9,this.art.get('#d1d3c5'));s.box(5.4,5.55,d,3.8,.12,.12,this.art.get('#94a4a0'));s.box(4.7,5.7,d,1.7,.08,.3,this.art.get('#ffdda5',.5,true));}
      }
      for(const z of [12,34,56,78]){s.box(5.7,2.6,z,1.15,.15,3.1,this.art.get('#7a6754'));s.box(6.2,3,z,.13,.75,3.1,this.art.get('#7a6754'));}
      s.box(5.6,2.85,43,1.1,1.7,1.1,this.art.get('#557777'));s.box(5.58,3.15,42.42,.72,.85,.04,this.art.get('#ceddd8',.35,true));
      // Entry ramp and its small ticket shelter connect to the station node.
      s.box(6,.6,-4,4.5,1.2,7,this.art.get('#aaa99e'));s.box(6,2.7,-6,3,3.2,2.4,this.art.get('#c9c8c1'));s.box(6,4.45,-6,3.8,.2,3.1,this.art.get('#59676a'));s.box(5.96,2.8,-7.23,1.5,1.4,.08,this.art.get('#52757a',.35));
      for(const z of [2,platformMid,platformLen-3])s.box(5.2,-RAIL_DECK/2,z,2.4,RAIL_DECK,2.4,this.art.get('#9a9d93'));
      s.box(9.4,-RAIL_DECK/2+.2,-2,2.6,RAIL_DECK+.4,8,this.art.get('#aaa99e'));
      const platform=s.finish();platform.position.set(stationNode.x,RAIL_DECK,stationNode.z);platform.rotation.y=bearing;
      const signMaterial=new THREE.MeshBasicNodeMaterial({map:signTexture('青葉川  AOBAGAWA'),color:'#fff4d9'});
      const board=new THREE.Mesh(new THREE.BoxGeometry(.08,.72,3.4),signMaterial);board.userData.ownedMaterial=true;board.userData.ownedTexture=signMaterial.map;board.position.set(3.8,4.6,platformMid);platform.add(board);
      this.station.add(platform);
    }
  }
  private roadKit(kit:Kit,a:Point,b:Point,r:Road){
    if(r.stage!=='open'){
      const fullDx=b.x-a.x,fullDz=b.z-a.z,fullLength=Math.hypot(fullDx,fullDz),bearing=Math.atan2(fullDx,fullDz);
      // Survey stakes show the reserved corridor before any usable deck exists.
      for(let distance=0;distance<fullLength;distance+=18){const t=distance/fullLength;for(const side of [-1,1])kit.box(a.x+fullDx*t+Math.cos(bearing)*side*5,.7,a.z+fullDz*t-Math.sin(bearing)*side*5,.12,1.4,.12,this.art.get('#b79b73'));}
      const fraction=THREE.MathUtils.clamp((r.progress-.12)/.88,0,1);
      if(fraction<.015)return;
      b={x:a.x+fullDx*fraction,z:a.z+fullDz*fraction};
    }
    const dx=b.x-a.x,dz=b.z-a.z,len=Math.hypot(dx,dz),angle=Math.atan2(dx,dz),mid={x:(a.x+b.x)/2,z:(a.z+b.z)/2},rail=r.kind==='rail',open=r.stage==='open',bridge=r.kind==='bridge';
    const expressway=r.kind==='expressway',lane=r.kind==='lane';
    const width=roadWidth(r.kind),y=bridge?1.4:rail?1+RAIL_DECK:expressway?EXPRESSWAY_DECK:lane?.12:.15;
    if(rail||expressway){
      // Viaduct piers: single columns under the expressway, paired under rail.
      const spacing=expressway?28:22,pierColor=this.art.get('#9a9d93');
      for(let distance=spacing/2;distance<len;distance+=spacing){const t=distance/len,px=a.x+dx*t,pz=a.z+dz*t;
        if(expressway)kit.box(px,y/2,pz,2.2,y,2.2,pierColor,angle);
        else for(const side of [-1,1])kit.box(px+Math.cos(angle)*side*2.4,y/2,pz-Math.sin(angle)*side*2.4,1.1,y,1.1,pierColor,angle);
        kit.box(px,y-.55,pz,width+1.2,.7,1.4,pierColor,angle);
      }
      kit.box(mid.x,y-.35,mid.z,width+.6,.5,len,this.art.get('#a3a69c'),angle);
      if(expressway)for(const side of [-1,1])kit.box(mid.x+Math.cos(angle)*side*(width/2+.25),y+.75,mid.z-Math.sin(angle)*side*(width/2+.25),.3,1.3,len,this.art.get('#b8bbb2'),angle);
    }
    if(bridge){
      const ramp=Math.min(12,len*.22),shape=new THREE.Shape();
      shape.moveTo(-len/2,.3);shape.lineTo(-len/2+ramp,2);shape.lineTo(len/2-ramp,2);shape.lineTo(len/2,.3);
      shape.lineTo(len/2,-.15);shape.lineTo(len/2-ramp,.8);shape.lineTo(-len/2+ramp,.8);shape.lineTo(-len/2,-.15);shape.closePath();
      const deck=new THREE.ExtrudeGeometry(shape,{depth:width+2.8,bevelEnabled:false,steps:1});deck.setIndex(Array.from({length:deck.getAttribute('position').count},(_,i)=>i));deck.rotateY(-Math.PI/2);deck.translate((width+2.8)/2,0,0);deck.rotateY(angle);deck.translate(mid.x,0,mid.z);kit.add(deck,this.art.get(open?'#747f7b':'#ac9a78'));
    }else kit.box(mid.x,y,mid.z,width,.3,len,this.art.get(open?'#747f7b':'#ac9a78'),angle);
    if(rail){
      for(const side of [-1,1])kit.box(mid.x+Math.cos(angle)*side*1.35,y+.45,mid.z-Math.sin(angle)*side*1.35,.14,.17,len,this.art.get('#a3aca7',.4),angle);
      for(let distance=0;distance<len;distance+=2.5){const t=distance/len;kit.box(a.x+dx*t,y+.29,a.z+dz*t,5,.15,.4,this.art.get('#6d6457'),angle);}
    }else if(lane){
      // Lanes are shared-surface roji: no center line and no raised sidewalk.
    }else{
      if(open&&(bridge||r.kind==='arterial')){for(let distance=8;distance<len-4;distance+=12){
        if(bridge){
          // Split painted marks at ramp creases so neither end floats or sinks.
          const ramp=Math.min(12,len*.22),cuts=[distance-2,...[ramp,len-ramp].filter(v=>v>distance-2&&v<distance+2),distance+2];
          for(let i=0;i<cuts.length-1;i++){const start=cuts[i],end=cuts[i+1],fromY=bridgeDeckHeight(start,len),toY=bridgeDeckHeight(end,len),along=(start+end)/2;
            const mark=new THREE.BoxGeometry(.18,.035,Math.hypot(end-start,toY-fromY));mark.rotateX(-Math.atan2(toY-fromY,end-start));mark.rotateY(angle);mark.translate(a.x+dx*along/len,(fromY+toY)/2+.035,a.z+dz*along/len);kit.add(mark,this.art.get('#dedaca'));
          }
        }else{const t=distance/len;kit.box(a.x+dx*t,y+.19,a.z+dz*t,.18,.035,4,this.art.get('#dedaca'),angle);
          // Four-lane arterials: dashed lane dividers between the two lanes each way.
          if(r.kind==='arterial')for(const side of [-1,1])kit.box(a.x+dx*t+Math.cos(angle)*side*width/4,y+.19,a.z+dz*t-Math.sin(angle)*side*width/4,.14,.03,2.4,this.art.get('#dedaca'),angle);}
      }
      // Arterials keep raised sidewalks; local streets are shared surfaces with
      // painted edge lines, as on Tokyo residential streets.
      if(!bridge&&r.kind==='arterial')for(const side of [-1,1])kit.box(mid.x+Math.cos(angle)*side*(width/2+ARTERIAL_SIDEWALK/2),y+.15,mid.z-Math.sin(angle)*side*(width/2+ARTERIAL_SIDEWALK/2),ARTERIAL_SIDEWALK,.35,Math.max(1,len-width),this.art.get('#b4b8a6'),angle);
      else if(!bridge&&open)for(const side of [-1,1])kit.box(mid.x+Math.cos(angle)*side*(width/2-.45),y+.16,mid.z-Math.sin(angle)*side*(width/2-.45),.14,.03,Math.max(1,len-width),this.art.get('#dedaca'),angle);
      }else{
        for(let distance=0;distance<len;distance+=12){const t=distance/len,x=a.x+dx*t,z=a.z+dz*t;kit.cylinder(x+width/2,y+.55,z,.25,.8,this.art.get('#c17d4f'),5);}
      }
      if(bridge){
        const ramp=Math.min(12,len*.22),distances=[0,ramp,len-ramp,len];
        for(const side of [-1,1])for(let i=0;i<3;i++){
          const start=distances[i],end=distances[i+1],fromY=bridgeDeckHeight(start,len),toY=bridgeDeckHeight(end,len),along=(start+end)/2;
          const railing=new THREE.BoxGeometry(.14,1.0,Math.hypot(end-start,toY-fromY));railing.rotateX(-Math.atan2(toY-fromY,end-start));railing.rotateY(angle);
          railing.translate(a.x+dx*along/len+Math.cos(angle)*side*(width/2+1.28),(fromY+toY)/2+.5,a.z+dz*along/len-Math.sin(angle)*side*(width/2+1.28));kit.add(railing,this.art.get('#668084'));
        }
        for(const t of [.28,.55,.76])kit.box(a.x+dx*t,-1,a.z+dz*t,width*.8,5,2,this.art.get('#a2aa9e'));
      }
      if(open&&len>25){for(let distance=12;distance<len-8;distance+=44){const t=distance/len,x=a.x+dx*t+Math.cos(angle)*(width/2+1.5),z=a.z+dz*t-Math.sin(angle)*(width/2+1.5);const baseY=bridge?bridgeDeckHeight(distance,len):0;kit.cylinder(x,baseY+3.5,z,.1,7,this.art.get('#53696a'),6);kit.box(x-.7,baseY+7,z,1.8,.2,.5,this.art.get('#ead5a3',.5,true));}}
    }
  }
  private animate(){
    if(this.disposed)return;
    const now=performance.now(),frameInterval=this.lastFrame?now-this.lastFrame:1000/60,dt=Math.min(.1,frameInterval/1000);this.lastFrame=now;
    const begin=performance.now();
    for(let i=0;i<2&&this.objectQueue.size;i++){
      const [id,item]=this.objectQueue.entries().next().value!;this.objectQueue.delete(id);
      const group=item.source.createGroup();
      this.chunks.set(id,group,item.source);this.buildings.set(id,{source:item.source,signature:item.signature});
    }
    const pendingChunks=new Set<string>();
    for(const {building} of this.objectQueue.values())pendingChunks.add(buildingChunkKey(building.x,building.z));
    this.chunks.update(pendingChunks);
    if(this.worldState)this.rearExtensions.flushCompleted(this.worldState);
    this.director.update(dt,now/1000);
    const activeLens=LENS_PRESETS[this.director.lens];
    this.lensFocalM.value=activeLens.focalLengthMm/1000;this.lensFNumber.value=activeLens.fNumber;
    this.updateTreeDetail();
    this.updateActors(now);
    this.updateLight(dt);
    const alpha=Math.min(1,(now-this.received)/this.interval);const time=Math.max(0,(this.motionTick-1+alpha)*.05);const waterDt=Math.max(0,Math.min(.1,time-this.waterTime));this.water?.update(waterDt,time,this.rain,this.style,1-this.daylight);this.umbrellas.update(this.umbrellaTuples.subarray(0,this.umbrellaCount*5),this.rain,time);this.art.time.value=time;this.waterTime=time;this.lastTick=this.motionTick;this.rainTime.value=time;this.rainLines.visible=this.rain>.1;this.rainOpacity.value=.035+this.rain*.13;this.rainLines.position.set(this.director.target.x,0,this.director.target.z);
    this.camera.updateMatrixWorld();
    this.focusTargetView.copy(this.director.controls.target).applyMatrix4(this.camera.matrixWorldInverse);
    const focusTarget=focusDepthFromViewZ(this.focusTargetView.z,this.camera.near,this.camera.far);
    if(this.focusResetPending){this.focus.value=focusTarget;this.focusResetPending=false;}
    else this.focus.value=dampFocusDepth(this.focus.value,focusTarget,dt);
    if(this.selected!==null){const b=this.worldState?.buildings.find(b=>b.id===this.selected);this.highlight.visible=!!b;if(b)this.highlight.position.set(b.x,.22,b.z);}else this.highlight.visible=false;
    this.pipeline?.render();
    const cost=performance.now()-begin;
    if(this.qualityController.record(frameInterval,now,!document.hidden&&this.objectQueue.size===0&&this.pendingCompiles===0)!==null)this.applyQuality();
    this.frames.push(frameInterval);if(this.frames.length>120)this.frames.shift();
    if(this.frame++%90===0&&this.onMetrics){const avg=this.frames.reduce((a,b)=>a+b,0)/this.frames.length;this.onMetrics({fps:1000/avg,frameMs:cost,calls:this.renderer.info.render.drawCalls,triangles:this.renderer.info.render.triangles,textures:this.renderer.info.memory.textures,geometries:this.renderer.info.memory.geometries,backend:this.renderer.backend.constructor.name,styleSwitch:this.styleSwitchMetrics?{...this.styleSwitchMetrics}:undefined,pendingBuildings:this.objectQueue.size,pendingChunks:this.chunks.pendingCount});}
  }
  private updateActors(now:number){
    const t=Math.min(1,(now-this.received)/this.interval);let ped=0,train=0,bus=0;const dt=this.lastActorFrame?Math.min(.1,(now-this.lastActorFrame)/1000):1/60;this.lastActorFrame=now;this.buses.begin(now);this.fleet.begin();this.trainFleet.begin();const pedIds:number[]=[],trainIds:number[]=[];
    for(const [id,current]of this.current){const old=this.previous.get(id)??current,x=THREE.MathUtils.lerp(old.x,current.x,t),z=THREE.MathUtils.lerp(old.z,current.z,t);let h=current.h;const dh=Math.atan2(Math.sin(current.h-old.h),Math.cos(current.h-old.h));h=Math.PI/2-(old.h+dh*t);const lift=current.k===2?0:bridgeLift({x,z},this.bridgeSpans);
      if(current.k===0&&ped<2000){const offset=ped*5;this.umbrellaTuples[offset]=id;this.umbrellaTuples[offset+1]=x;this.umbrellaTuples[offset+2]=z;this.umbrellaTuples[offset+3]=Math.PI/2-h;this.umbrellaTuples[offset+4]=0;const moving=Math.hypot(current.x-old.x,current.z-old.z)>.01;const step=moving?Math.sin(this.motionTick*.31+id)*.045:0;dummy.position.set(x,1.05+step+lift,z);dummy.rotation.set(0,h,moving?step*.4:0);dummy.scale.setScalar(1);dummy.updateMatrix();this.pedestrians.setMatrixAt(ped,dummy.matrix);this.pedestrians.setColorAt(ped,personColors[id%personColors.length]);dummy.position.y=1.95+step+lift;dummy.updateMatrix();this.heads.setMatrixAt(ped,dummy.matrix);pedIds[ped]=id;ped++;}
      else if(isRoadVehicle(current.k)){const vehicle=bridgeVehiclePose({x,z},h,2.7,this.bridgeSpans);this.fleet.add(id,current.k,x,z,h,vehicle.lift,vehicle.pitch,dt);}
      else if(current.k===3&&bus<2){const vehicle=bridgeVehiclePose({x,z},h,5.1,this.bridgeSpans);this.buses.set(bus++,id,x,z,h,this.worldState,vehicle.lift+ROAD_SURFACE_Y,vehicle.pitch);}
      else if(current.k===2&&train<this.trainFleet.capacity){this.trainFleet.add(id,x,z,h,RAIL_DECK,0,dt);trainIds[train]=id;train++;}
    }
    this.umbrellaCount=ped;
    this.buses.finish(bus);for(const mesh of this.buses.meshes)this.actorIds.set(mesh,this.buses.ids);
    this.actorIds.set(this.pedestrians,pedIds);this.actorIds.set(this.heads,pedIds);this.fleet.finish();for(const {mesh,ids} of this.fleet.pickable)this.actorIds.set(mesh,ids);this.trainFleet.finish(train);for(const {mesh,ids} of this.trainFleet.pickable)this.actorIds.set(mesh,ids);
    for(const [mesh,count]of [[this.pedestrians,ped],[this.heads,ped]] as const){mesh.count=count;mesh.instanceMatrix.needsUpdate=true;}
    if(this.pedestrians.instanceColor)this.pedestrians.instanceColor.needsUpdate=true;
  }
  private updateLight(dt:number){
    const hour=this.preview??this.hour,angle=(hour-6)/12*Math.PI;
    const day=THREE.MathUtils.smoothstep(Math.sin(angle),-.2,.12),dusk=1-THREE.MathUtils.smoothstep(Math.abs(Math.sin(angle)),.05,.5);
    this.daylight=THREE.MathUtils.lerp(this.daylight,day,Math.min(1,dt*3));
    const top=new THREE.Color('#659bbd').lerp(new THREE.Color('#9aa5bd'),dusk*.5).lerp(new THREE.Color('#11253e'),1-this.daylight);
    const low=new THREE.Color('#d5ded3').lerp(new THREE.Color('#efb185'),dusk).lerp(new THREE.Color('#334860'),1-this.daylight);
    this.skyHigh.value.copy(top);this.skyLow.value.copy(low);this.skyCloudShade.value.set('#8ba5bc').lerp(new THREE.Color('#9c829b'),dusk*.7).lerp(new THREE.Color('#283c58'),1-this.daylight);this.hillColor.value.set('#577773').lerp(new THREE.Color('#747386'),dusk*.55).lerp(new THREE.Color('#20384a'),1-this.daylight);this.skyCloud.value.set('#fff3dc').lerp(new THREE.Color('#d29e91'),dusk*.5).lerp(new THREE.Color('#45526a'),1-this.daylight);
    const atmosphere=this.scene.fog as THREE.FogExp2;
    atmosphere.color.copy(top).lerp(low,.35);
    // Clear air preserves facade contrast at neighborhood scale. Rain still
    // veils the distant city; stylized skies retain a little more atmosphere.
    const clearHaze=this.style==='miniature'?.00022:.0003;
    atmosphere.density=clearHaze+this.rain*.00025;
    const solarY=Math.sin(angle)*450;this.sunDisc.position.set(Math.cos(angle)*1050,solarY*3,-480);this.sunDisc.visible=solarY>-25;this.moonDisc.position.set(-Math.cos(angle)*1050,-solarY*3,480);this.moonDisc.visible=solarY<25;
    const keyBlend=this.daylight*2-1;
    // Snap only the shadow camera's light-plane translation. The key's
    // direction and elevation still follow the clock, not camera movement.
    this.sun.position.set(Math.cos(angle)*350*keyBlend,Math.max(30,Math.abs(Math.sin(angle))*450),-160*keyBlend);
    const shadowCamera=this.sun.shadow.camera;
    snapShadowAnchor(this.director.controls.target,this.sun.position,
      shadowCamera.right-shadowCamera.left,shadowCamera.top-shadowCamera.bottom,
      this.sun.shadow.mapSize.x,this.sun.shadow.mapSize.y,this.sun.target.position);
    this.sun.position.add(this.sun.target.position);
    this.sun.intensity=.16+this.daylight*1.8;this.sun.color.set('#aac8e3').lerp(new THREE.Color(dusk>.4?'#ffcc9a':'#fff1d9'),this.daylight);
    // Soft urban bounce keeps opaque facades and lanes readable between local
    // emitters. Preserve the daytime fill and ground tint so the sun continues
    // to describe building orientation without flattening daytime massing.
    this.hemi.intensity=1.05-this.daylight*.55;
    this.hemi.groundColor.set('#a0a09a').lerp(new THREE.Color('#748998'),this.daylight);
    // A painted moonlit fill keeps rooflines and quiet lanes readable between
    // pools of warm shop light; the sky itself can remain a deep midnight blue.
    this.hemi.color.set('#829eb9').lerp(new THREE.Color('#d8e6e7'),this.daylight);
    if(this.frame%30===0){const target=this.director.target;
      const lit=this.worldState?.buildings.filter(b=>b.stage==='open'&&(b.anchor||b.kind==='shop'||b.kind==='cafe')).sort((a,b)=>Math.hypot(a.x-target.x,a.z-target.z)-Math.hypot(b.x-target.x,b.z-target.z)).slice(0,8)??[];
      for(let i=0;i<this.lanternLights.length;i++){const b=lit[i],lamp=this.lanternLights[i];lamp.userData.active=!!b;if(b)lamp.position.set(b.x+Math.sin(b.rotation)*(b.depth*.45+1.5),3.1,b.z+Math.cos(b.rotation)*(b.depth*.45+1.5));}
    }
    const activityHour=hour%24,lightBlend=Math.min(1,dt*.8);this.art.litHome.value=THREE.MathUtils.lerp(this.art.litHome.value,activityHour>=23||activityHour<5?.32:1,lightBlend);this.art.litOffice.value=THREE.MathUtils.lerp(this.art.litOffice.value,activityHour>=7&&activityHour<20?1:.1,lightBlend);this.art.litShop.value=THREE.MathUtils.lerp(this.art.litShop.value,activityHour>=6&&activityHour<23?1:.18,lightBlend);
    for(const lamp of this.lanternLights)lamp.intensity=lamp.userData.active?(1-this.daylight)*36*this.art.litShop.value:0;
    this.art.night.value=1-this.daylight;this.art.wet.value=THREE.MathUtils.lerp(this.art.wet.value,this.rain,Math.min(1,dt*(this.rain>this.art.wet.value?1:.08)));
    if(this.shine)this.shine.strength.value=.06+(1-this.daylight)*.2;
    this.renderer.toneMappingExposure=1;
  }
  getAudioContext(){
    const world=this.worldState,listener=this.director.target;this.camera.getWorldDirection(v);
    const trains:Array<{id:number,x:number,z:number,speed:number}>=[],buses:Array<{id:number,x:number,z:number,speed:number}>=[],traffic:Array<{id:number,x:number,z:number,speed:number,distance:number}>=[];let pedestrians=0;
    for(const [id,p]of this.current){const old=this.previous.get(id)??p,speed=Math.hypot(p.x-old.x,p.z-old.z)/this.motionDelta,distance=Math.hypot(p.x-listener.x,p.z-listener.z);if(p.k===2)trains.push({id,x:p.x,z:p.z,speed});else if(p.k===3)buses.push({id,x:p.x,z:p.z,speed});else if(isRoadVehicle(p.k)&&distance<130)traffic.push({id,x:p.x,z:p.z,speed,distance});else if(p.k===0&&distance<45)pedestrians++;}
    traffic.sort((a,b)=>a.distance-b.distance||a.id-b.id);
    const events=resolveAudioEventContext(world,this.current,this.motionTick);
    let riverDistance=800;const river=world?.terrain.river??[];for(let i=0;i<river.length-1;i++)riverDistance=Math.min(riverDistance,Math.max(0,distanceToSegment({x:listener.x,z:listener.z},river[i],river[i+1])-this.riverHalfWidth));
    // Listen at the observed scene, with compressed height appropriate to a
    // miniature world, rather than silencing it from a distant aerial camera.
    const construction=[...(world?.rear_extensions??[]).slice(0,2).map(e=>({id:e.project_id,x:e.x,z:e.z})),...(world?.buildings.filter(b=>b.stage!=='open').map(b=>({id:b.id,x:b.x,z:b.z}))??[])].slice(0,12);
    return{hour:this.hour,rain:this.rain,riverDistance,tick:this.motionTick,worldKey:world?.seed,developmentRate:world?.development_rate,traffic:traffic.slice(0,24),pedestrians,events,camera:{x:listener.x,y:this.camera.position.y*.15,z:listener.z,forward:{x:v.x,z:v.z}},trains,buses,construction};
  }
  private pointerDown=(e:PointerEvent)=>{this.pointerStart={x:e.clientX,y:e.clientY};};
  private pointerUp=(e:PointerEvent)=>{
    if(Math.hypot(e.clientX-this.pointerStart.x,e.clientY-this.pointerStart.y)>4)return;
    const bounds=this.renderer.domElement.getBoundingClientRect(),ray=new THREE.Raycaster();ray.setFromCamera(new THREE.Vector2((e.clientX-bounds.left)/bounds.width*2-1,-(e.clientY-bounds.top)/bounds.height*2+1),this.camera);
    const hits=ray.intersectObjects([this.chunks.root,...this.actorIds.keys()],true);if(!hits.length)return;const hit=hits[0],building=this.chunks.buildingAt(hit);if(building!==undefined)this.onPick?.(building,'building');else if(hit.instanceId!==undefined){const id=this.actorIds.get(hit.object)?.[hit.instanceId];if(id!==undefined)this.onPick?.(id,'traveler');}
  };
  dispose(){this.disposed=true;this.renderStartup?.dispose();this.renderer.setAnimationLoop(null);this.resize.disconnect();this.director.dispose();this.renderer.domElement.removeEventListener('pointerdown',this.pointerDown);this.renderer.domElement.removeEventListener('pointerup',this.pointerUp);this.resetWorld();this.umbrellas.root.removeFromParent();this.retire(()=>this.umbrellas.dispose());this.buses.root.removeFromParent();this.retire(()=>this.buses.dispose());this.retire(()=>this.fleet.dispose());this.retire(()=>this.trainFleet.dispose());this.disposePipeline();const materials=new Set<THREE.Material>(),geometries=new Set<THREE.BufferGeometry>();this.scene.traverse(o=>{if(o instanceof THREE.Mesh||o instanceof THREE.Line){geometries.add(o.geometry);for(const m of Array.isArray(o.material)?o.material:[o.material])if(!m.userData.artKey&&m!==this.art.contours)materials.add(m);}});this.retire(()=>{for(const g of geometries)g.dispose();for(const m of materials)m.dispose();this.art.dispose();this.renderer.dispose();});this.renderer.domElement.remove();}
}

function distanceToSegment(p:Point,a:Point,b:Point){const dx=b.x-a.x,dz=b.z-a.z;const t=Math.max(0,Math.min(1,((p.x-a.x)*dx+(p.z-a.z)*dz)/(dx*dx+dz*dz||1)));return Math.hypot(p.x-a.x-dx*t,p.z-a.z-dz*t);}
