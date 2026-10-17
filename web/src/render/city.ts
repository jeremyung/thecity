import {buildBusinessBuilding} from './architecture/business';
import {resolveBusinessRecipes,BUSINESS_RECIPE_VERSION,parseDisplayMode,type BusinessRecipe,type DisplayMode} from './architecture/business-recipes';
import {selectApartmentArchitecture,resolveApartmentArchitectures,type ApartmentArchitecture,apartmentArchitectureKey,buildSelectedApartment,parseApartmentEra,type ApartmentEra} from './architecture/era';
import {apartmentPhase,APARTMENT_RECIPE_VERSION} from './architecture/recipes';
import {ApartmentResources} from './architecture/resources';
import {apartmentDetail,type ApartmentDetail} from './architecture/detail';
import { miniatureOpticalScale } from '../camera/miniature-scale';
import { DEFAULT_TILT_SHIFT, normalizeTiltShift, writeTiltedFocusPlane, type TiltShiftSettings } from '../camera/tilt-shift';
import { instanceCapacity, reuseInstances } from './reuse-instances';
import { riverbankTreePoints } from './riverbank-planting';
import { groundRoadFootprint, groundRoadSurfaceGeometry } from './ground-road-footprint';
import { RoadSurfacePartition } from './road-surface-partition';
import { roadMarkingTrims, type MarkingTrim } from './road-markings';
import { NearbyStreetLights } from './nearby-street-lights';
import { junctionFootprint, type JunctionArm } from './road-junction';
import { MovementPresentation } from './movement-presentation';
import { overlapsStation } from './station-clearance';
import { treeClearanceSignature } from './tree-clearance-signature';
import { createTreeClearanceGrid } from './tree-clearance-grid';
import type { MovementSupport } from '../movement-support';
import { VisibleInfrastructure, type InfrastructureCommit } from './visible-infrastructure';
import { buildStationPlatform } from './station-kit';
import { MOVEMENT_STRIDE, PEDESTRIAN_GROUND_SURFACE, pedestrianSurface } from '../movement';
import { snapShadowAnchor } from './shadow-anchor';
import { styleWarmup, compileStyleSamples, compileForScenePass } from './style-warmup';
import * as THREE from 'three/webgpu';
import { submitSceneAndEffects } from './scene-submission';
import { color, uniform, mix, positionLocal, smoothstep, mx_noise_float, pass, add, vec3, attribute, positionView, float, floor, fract, step, vec2, vec4, mrt, output, normalView, directionToColor, colorToDirection, sample, getViewPosition, uv, fog, rangeFogFactor, positionWorld } from 'three/tsl';
import { bloom } from 'three/addons/tsl/display/BloomNode.js';
import { ownedAO as ao } from './postprocessing/owned-ao';
import { ssr } from 'three/addons/tsl/display/SSRNode.js';
import { godrays } from 'three/addons/tsl/display/GodraysNode.js';
import { SkyEnvironment } from './sky-environment';
import { skyRadiance } from './sky-scattering';
import { backdropForestMatrices } from './backdrop-forest';
import { signedCoCDof } from './postprocessing/SignedCoCDepthOfFieldNode.js';
import { physicalCoCNode } from './postprocessing/physical-coc';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { ArtMaterials, signTexture } from './materials';
import { Kit, disposeGroup } from './kit';
import { buildOverheadLines } from './overhead-lines';
import { StagedKit } from './staged-kit';
import { RiverWater } from './water';
import { createRiverbankGeometry } from './riverbank';
import { buildingChunkKey, type RebuildableBuildingSource } from './building-chunks';
import { Umbrellas } from './umbrellas';
import { CityBuses } from './buses';
import { isRoadVehicle, ROAD_SURFACE_Y, VehicleFleet } from './vehicles';
import { TrainFleet, trainWorldWithInfrastructure } from './trains';
import { addRailCorridor } from './rail-corridor';
import { sampledRail, type RailPath } from './rail-path';
import { RAIL_CENTER_OFFSET, RAIL_HEAD_WIDTH, RAIL_HEAD_Y } from './rail-profile';
import { BuildingAccessPaths } from './building-access';
import { RearExtensions } from './rear-extensions';
import { ConstructionBatches } from './construction-batches';
import { buildExtendedBuilding } from './extended-building';
import { prepareStreetLighting, PersistentStreetLighting, type StreetLightingData } from './street-lighting';
import { bezierRoad, sweepRoad, type RoadCurve } from './road-curve';
import { CURRENT_EXPRESSWAY_PROFILE } from './expressway-profile';
import { addExpresswayFurniture } from './expressway-kit';
import { windowLived, windowOccupancy } from './building-surfaces';
import { createFoliageGeometry, createTreeTrunkGeometry } from './foliage';
import { signageContexts, type SignageContext } from './signage';
import { seasonAt, seasonKey, treeColor, type SeasonState, type TreeSpecies } from './seasons';
import { addNeighborhoodShrine, addPocketPark, addScrambleCrossing, addShrineGrove, addStationPlaza, openSpaceKind, openSpaces, parkSignature, streetTreePlacements, type PlantedTree } from './parks';
import { createBackdropGeometry } from './backdrop';
import { createSkylineGeometry } from './skyline';
import { createPavedGroundTest } from './paved-ground';
import { buildGroundSurfaces } from './ground-surfaces';
import { CitySidewalks } from './sidewalks';
import { roadBatchSignatures } from './road-batch-signature';
import { ARTERIAL_SIDEWALK, bridgeDeckHeight, bridgeLift, bridgeVehiclePose, vehicleSurfacePose, EXPRESSWAY_DECK, RAIL_DECK, roadWidth, expresswayPierClear, type CrossingDeck, type BridgeSpan } from './road-elevation';
import { RenderQualityController, renderQualitySettings } from './quality';
import { FrameTiming, type FrameTimingSnapshot } from './frame-timing';
import { createRenderStartupLifecycle, type RenderStartupLifecycle } from './render-startup';
import { CameraDirector } from '../camera/director';
import { dampFocusDepth, focusDepthFromViewZ } from '../camera/focus';
import type { LensPresetId } from '../camera/lens';
import { LENS_PRESETS, verticalFovDegrees } from '../camera/lens';
import { resolveAudioEventContext } from '../audio/event-context';
import type { World, Style, Point, Road, Building, BuildingOriginalFootprint } from '../types';

export interface StyleSwitchMetrics { target:Style; samples:number; compiledSamples:number; maxSubmitMs:number; prepareMs:number; submitMs:number; waitMs:number; applyMs:number; outcome:'compiling'|'applied'|'superseded'|'failed' }
export interface RenderMetrics { timing?:FrameTimingSnapshot; styleSwitch?:StyleSwitchMetrics; fps:number; frameMs:number; calls:number; triangles:number; textures:number; geometries:number; backend:string; pendingBuildings:number; pendingChunks:number; pendingSidewalks:number; pendingRoads:number }
/** Fields that affect a building's rendered group. Occupancy is binary: every
 * window pane bakes whether anyone lives or works behind it (`windowLived`), so
 * crossing the vacant boundary rebuilds, while other occupancy changes do not. */
export function buildingRenderSignature(b:Building,original?:BuildingOriginalFootprint,signage?:SignageContext):string {
  const homeOccupancy=windowLived(b)?'occupied':'vacant';
  const retained=original?`${original.x}:${original.z}:${original.width}:${original.depth}:${original.rotation}`:'-';
  const signs=signage&&(signage.proximity||signage.feature)?`:s${signage.proximity}${signage.feature??''}`:'';
  const stagedTower=b.floors>=12&&(b.kind==='office'||b.kind==='apartment')&&b.stage!=='open'&&b.stage!=='renovation'&&b.stage!=='demolition';
  const towerPhase=`${Math.ceil(b.floors*Math.max(0,Math.min(1,(b.progress-.12)/.57)))}:${Math.ceil(b.floors*Math.max(0,Math.min(1,(b.progress-.69)/.23)))}`;
  const phase=stagedTower?towerPhase:b.stage==='demolition'?`${Math.ceil(b.floors*(1-Math.max(0,Math.min(1,b.progress))))}:${b.progress>0}`:Math.floor(b.progress*5);
  return `${b.stage}:${b.floors}:${phase}:${b.kind}:${b.name}:${b.appearance_seed}:${b.business_active ?? 'legacy'}:${homeOccupancy}:${b.x}:${b.z}:${b.width}:${b.depth}:${b.rotation}:${retained}${signs}`;
}
type TreeBatch={indices:number[],trees:THREE.InstancedMesh,trunks:THREE.InstancedMesh,center:THREE.Vector3,far:boolean};
type TreeLook={species:TreeSpecies,color:string,variation:number};
const v=new THREE.Vector3(),dummy=new THREE.Object3D();
// Range haze (m from camera): clear through the city, veiling the backdrop.
// Miniature keeps crisper air, matching its tilt-shift photographic read.
const HAZE_NEAR:Record<Style,number>={anime:380,miniature:460},HAZE_FAR:Record<Style,number>={anime:3200,miniature:3700};
const personColors=['#e8d9b6','#bc7361','#89a589','#64839d','#e3b35f','#a897b0'].map(c=>new THREE.Color(c));
function random(seed:number){return()=>{seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return(seed>>>0)/4294967296;};}

export class CityRenderer {
  // Explicit development study; never selected by production or a saved world.
  private readonly compactMidriseReview = import.meta.env.DEV && new URLSearchParams(window.location.search).get('midrise') === 'compact';
  readonly scene=new THREE.Scene();
  readonly camera=new THREE.PerspectiveCamera(verticalFovDegrees(LENS_PRESETS.environment.focalLengthMm),1,.5,3500);
  readonly art=new ArtMaterials();
  private readonly apartmentResources=new ApartmentResources();
  private apartmentDetails=new Map<number,ApartmentDetail>();
  private apartmentDetailAt=0;
  private apartmentEra:ApartmentEra='mixed';
  private businessChoices=new Map<number,BusinessRecipe>();
  private commercialDisplays:DisplayMode='animated';
  private apartmentChoices=new Map<number,ApartmentArchitecture>();
  private apartmentLayout='';
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
  private focusedPipeline:THREE.RenderPipeline|null=null;
  private unblurredPipeline:THREE.RenderPipeline|null=null;
  private occlusion:ReturnType<typeof ao>|null=null;
  private occlusionStrength=uniform(.22);
  private hazeColor=uniform(new THREE.Color('#d2d6cc'));
  private hazeNear=uniform(HAZE_NEAR.anime);
  private hazeFar=uniform(HAZE_FAR.anime);
  private mist=uniform(0);
  private reflections:ReturnType<typeof ssr>|null=null;
  private wetReflections=false;
  private shafts:ReturnType<typeof godrays>|null=null;
  private lightShafts=false;
  private shaftColor=uniform(new THREE.Color(0,0,0));
  private shaftStrength=0;
  private skyEnvironment:SkyEnvironment|null=null;
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
  private tiltShift: TiltShiftSettings={...DEFAULT_TILT_SHIFT};
  private tiltEnabled=uniform(0);
  private tiltWidth=uniform(.5);
  private tiltOpticalScale=uniform(.01);
  private tiltPlane=uniform(new THREE.Vector4(0,0,1,300));
  private shine: ReturnType<typeof bloom> | null = null;
  private depthOfField:ReturnType<typeof signedCoCDof>|null=null;
  private skyHigh=uniform(new THREE.Color('#9bbac8'));
  private skyLow=uniform(new THREE.Color('#f1dcb9'));
  private skyCloud=uniform(new THREE.Color('#fff5df'));
  private skyCloudShade=uniform(new THREE.Color('#809aaf'));
  private hillColor=uniform(new THREE.Color('#637f85'));
  private skylineColor=uniform(new THREE.Color('#8496a3'));
  private skylineNight=uniform(0);
  private sunDisc:THREE.Mesh;
  private moonDisc:THREE.Mesh;
  private sun=new THREE.DirectionalLight('#fff1d1',3);
  private hemi=new THREE.HemisphereLight('#c8dce3','#748998',2);
  private lanternLights:Array<THREE.PointLight>=[];
  private nearbyStreetLights=new NearbyStreetLights();
  private streetLighting:PersistentStreetLighting|undefined;
  private terrain=new THREE.Group();
  private roads=new THREE.Group();
  private constructionRoads=new THREE.Group();
  private completedRoadSignature='';
  private completedRoadSurfaces:RoadSurfacePartition|null=null;
  private roadBuild:{kit:StagedKit;construction:THREE.Group;station:THREE.Group;lighting:{data:StreetLightingData|null};signature:string;surfaces:RoadSurfacePartition;identities:InfrastructureCommit;world:World;preparation:Generator<void,void,unknown>|null}|null=null;
  private visibleInfrastructure?:VisibleInfrastructure;
  private infrastructure(){return this.visibleInfrastructure??=new VisibleInfrastructure();}
  private queuedRoadWorld:{world:World;signature:string}|null=null;
  private get pendingRoads(){return Number(Boolean(this.roadBuild||this.queuedRoadWorld));}

  private station=new THREE.Group();
  private sidewalks=new CitySidewalks(()=>this.art.get('#b4b8a6'),dispose=>this.retire(dispose));
  private chunks=new ConstructionBatches(m=>this.apartmentResources.owns(m)?this.apartmentResources.batch(m):this.art.buildingBatch(m),dispose=>this.retire(dispose),this.art.contours);
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
  // Crown look per tree (species, summer color, variation) for seasonal repainting.
  private groveLook:TreeLook[]=[];
  private parkLook:TreeLook[]=[];
  private season:SeasonState=seasonAt(0);
  private paintedSeason='';
  /** Latest per-building signage context; part of each building's render signature. */
  private signage=new Map<number,SignageContext>();
  private treePoints:Array<{x:number,z:number,s:number,h?:number}> = [];
  private buses=new CityBuses(this.art,release=>this.retire(release));
  private accessPaths=new BuildingAccessPaths(this.art,release=>this.retire(release));
  private rearExtensions=new RearExtensions(this.art,release=>this.retire(release),building=>{
    const current=this.buildings.get(building.id);
    const original=this.worldState?.building_original_footprints?.find(form=>form.building_id===building.id);
    return !!current&&current.signature===this.apartmentSignature(building,original)&&this.chunks.isSourceVisible(building.id,current.source);
  });
  private umbrellas=new Umbrellas(2000,hex=>this.art.get(hex));
  private umbrellaTuples=new Float32Array(2000*MOVEMENT_STRIDE);
  private umbrellaCount=0;
  private pedestrians:THREE.InstancedMesh;
  private heads:THREE.InstancedMesh;
  private fleet=new VehicleFleet(this.art);
  private trains=new TrainFleet(this.art);
  private committedRoadWorld:World|null=null;
  private movementPresentation=new MovementPresentation();
  private actorSampleAt=0;
  private treeClearanceKey='';
  private previous=new Map<number,{x:number,z:number,h:number,k:number,y:number}>();
  private current=new Map<number,{x:number,z:number,h:number,k:number,y:number}>();
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
  private frameTiming=new FrameTiming();
  private lastMetricsAt=0;
  private onVisibility=()=>{this.frameTiming.record(performance.now(),0,'loading',false);this.lastFrame=0;};
  private roadSignature='';
  private groundSignature='';
  private ground=new THREE.Group();
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
  private fixedReviewQuality=false;
  private actorIds=new Map<THREE.Object3D,number[]>();
  private worldTick=-1;
  private waterTime=0;
  private rainTime=uniform(0);
  private rainOpacity=uniform(0);
  private rainLines:THREE.LineSegments;
  private objectQueue=new Map<number,{building:Building,signature:string,source:RebuildableBuildingSource}>();
  constructor(readonly element:HTMLElement,forceWebGL=false){
    void this.apartmentResources.loadTextures().then(ok=>{if(!ok&&!this.disposed)console.warn("Some apartment textures unavailable; using procedural fallback");}).catch(error=>console.warn("Apartment texture fallback",error));
    document.addEventListener('visibilitychange',this.onVisibility);
    this.renderer=new THREE.WebGPURenderer({antialias:true,samples:4,forceWebGL,powerPreference:'high-performance'});
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio,1.6));
    this.renderer.shadowMap.enabled=true;this.renderer.shadowMap.type=THREE.PCFSoftShadowMap;
    this.renderer.toneMapping=THREE.ACESFilmicToneMapping;this.renderer.toneMappingExposure=1.05;
    element.appendChild(this.renderer.domElement);
    this.director=new CameraDirector(this.camera,this.renderer.domElement);
    // Range haze (clear city, veiled backdrop) plus low-lying mist: it builds
    // with distance but only near the ground and water, strongest at first
    // light and in rain, so towers rise out of it as in the dawn concept.
    const mist=positionView.z.negate().mul(-.0032).exp().oneMinus().mul(positionWorld.y.add(1.5).div(-8).exp()).mul(this.mist);
    this.scene.fogNode=fog(this.hazeColor,rangeFogFactor(this.hazeNear,this.hazeFar).max(mist));
    this.scene.add(this.terrain,this.roads,this.parks,this.constructionRoads,this.sidewalks.root,this.ground,this.station,this.chunks.root,this.umbrellas.root,this.buses.root,this.trains.root,this.fleet.root,this.accessPaths.root,this.rearExtensions.root,this.hemi,this.sun);
    this.sun.castShadow=true;this.sun.shadow.mapSize.set(2048,2048);
    this.sun.shadow.camera.left=-230;this.sun.shadow.camera.right=230;this.sun.shadow.camera.top=230;this.sun.shadow.camera.bottom=-230;
    this.sun.shadow.camera.near=1;this.sun.shadow.camera.far=1100;this.sun.shadow.bias=-.0002;this.sun.shadow.normalBias=.35;
    this.scene.add(this.sun.target);
    for(let i=0;i<8;i++){const light=new THREE.PointLight('#ffd6a0',0,22,2);this.lanternLights.push(light);this.scene.add(light);}
    this.scene.add(this.nearbyStreetLights.root);
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
    (sky.material as THREE.MeshBasicNodeMaterial).fog=false;sky.renderOrder=-10;this.scene.add(sky);this.skyEnvironment=new SkyEnvironment(sky);
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
  private readonly sceneFrame=new THREE.NodeFrame();
  private buildPipeline(){
    this.disposePipeline();
    this.scenePass=pass(this.scene,this.camera,{samples:4});
    this.scenePass.updateBeforeType=THREE.NodeUpdateType.NONE;
    // Explicit resolved normals avoid Three's depth-normal reconstruction path,
    // which emits an invalid textureDimensions mip argument for MSAA WebGPU depth.
    // Keep four-sample scene AA; AO and wet-weather SSR share the normal target.
    const wet=this.wetReflections;
    this.scenePass.setMRT(mrt({output,normal:directionToColor(normalView)}));
    const depthTexture=this.scenePass.getTextureNode('depth');
    const normalTexture=this.scenePass.getTextureNode('normal');
    const viewNormal=sample(uv=>colorToDirection(normalTexture.sample(uv)));
    const occlusion=this.occlusion=ao(depthTexture,viewNormal,this.camera);
    occlusion.resolutionScale=.5;occlusion.radius.value=3;occlusion.thickness.value=2.5;occlusion.distanceExponent.value=1.4;occlusion.samples.value=12;
    // Fade AO beyond ~700 m: distant depth loses precision and can band
    // across the hills, skyline and mountain ring.
    const aoReach=smoothstep(1300,700,this.scenePass.getViewZNode().negate());
    let sceneColor=this.scenePass.getTextureNode('output').mul(vec4(vec3(mix(float(1),occlusion.getTextureNode().r,this.occlusionStrength.mul(aoReach))),1));
    if(wet){
      // Reflectivity is derived in the SSR pass itself: flat ground near street
      // level, in the same world-space puddles the asphalt darkens. Camera
      // matrices are explicit uniforms because the pass renders a screen quad.
      const viewPosition=getViewPosition(uv(),depthTexture.r,uniform(this.camera.projectionMatrixInverse));
      const cameraWorld=uniform(this.camera.matrixWorld);
      const world=cameraWorld.mul(vec4(viewPosition,1)).xyz;
      const up=cameraWorld.mul(vec4(colorToDirection(normalTexture.rgb),0)).xyz.normalize().y;
      const reflectivity=smoothstep(.86,.97,up).mul(smoothstep(1.6,.5,world.y)).mul(ArtMaterials.puddles(world.x,world.z).mul(.55).add(.1)).mul(this.art.wet);
      // SSR samples its colour input, so it reflects the raw pass texture.
      const reflections=this.reflections=ssr(this.scenePass.getTextureNode('output'),depthTexture,viewNormal,reflectivity,float(.32),this.camera);
      reflections.resolutionScale=.5;reflections.maxDistance.value=60;reflections.thickness.value=.6;reflections.quality.value=.45;
      // SSR writes reflected colour already scaled by reflectivity with alpha 1,
      // so add it; alpha blending would replace the street with a dark mirror.
      sceneColor=sceneColor.add(vec4(reflections.rgb,0));
    }
    if(this.lightShafts){
      // Raymarched shafts through the key light's shadow map, only while the
      // sun is low enough for them to read (rebuilt when that changes).
      const shafts=this.shafts=godrays(this.scenePass.getTextureNode('depth'),this.camera,this.sun);
      shafts.resolutionScale=.5;// Aerial views see sunlit air almost everywhere: keep density low so
      // only the contrast behind occluders reads, not a uniform glow.
      shafts.raymarchSteps.value=40;shafts.density.value=.09;shafts.maxDensity.value=.22;shafts.distanceAttenuation.value=2;
      sceneColor=sceneColor.add(vec4(shafts.getTextureNode().rgb.mul(this.shaftColor),0));
    }
    const depth=this.scenePass.getViewZNode();
    // Intersect each visible camera ray with the tilted plane. Unlike a
    // screen-space stripe, roof height and scene depth affect the focus.
    const focusView=getViewPosition(uv(),depthTexture.r,uniform(this.camera.projectionMatrixInverse));
    const focusRay=vec3(focusView.xy.div(depth.negate().max(.0001)),1);
    const planeDenominator=focusRay.dot(this.tiltPlane.xyz);
    const planeValid=planeDenominator.abs().greaterThan(.00001);
    const planeDepth=this.tiltPlane.w.div(planeValid.select(planeDenominator,float(1)));
    const tiltedDepth=planeValid.and(planeDepth.greaterThan(.1))
      .select(planeDepth,float(1000000));
    const signedCoC=physicalCoCNode({viewZ:depth,focusDistanceM:mix(this.focus,tiltedDepth,this.tiltEnabled),
      focalLengthM:this.lensFocalM,fNumber:this.lensFNumber.mul(mix(float(1),this.tiltWidth,this.tiltEnabled)),opticalScale:mix(this.opticalScale,this.tiltOpticalScale,this.tiltEnabled),
      outputHeightPx:this.outputHeight,maxGatherRadiusPx:this.bokeh,enabled:this.dofEnabled});
    // Keep two output graphs sharing the scene/AO/bloom resources. Switching
    // focus must not rebuild the scene pass and invalidate its compiled work.
    // The unblurred graph omits DOF entirely, so its seven passes do not run.
    const focus=this.depthOfField=signedCoCDof(sceneColor,signedCoC,this.bokeh);
    const shine=bloom(sceneColor,.1,.5,.9);this.shine=shine;
    this.focusedPipeline=new THREE.RenderPipeline(this.renderer);
    this.focusedPipeline.outputNode=add(focus as unknown as THREE.Node<'vec4'>,shine as unknown as THREE.Node<'vec4'>);
    this.unblurredPipeline=new THREE.RenderPipeline(this.renderer);
    this.unblurredPipeline.outputNode=add(sceneColor,shine as unknown as THREE.Node<'vec4'>);
    this.pipeline=this.focusEnabled?this.focusedPipeline:this.unblurredPipeline;
  }

  private disposePipeline(){
    // Warmup borrows scene attachments. Detach immediately, but keep the old
    // resources alive until pending compiler work releases its references.
    const resources=new Set([this.occlusion,this.reflections,this.shafts,this.pipeline,this.focusedPipeline,this.unblurredPipeline,this.depthOfField,this.shine,this.scenePass]);
    this.occlusion=null;this.reflections=null;this.shafts=null;this.pipeline=null;
    this.focusedPipeline=null;this.unblurredPipeline=null;
    this.depthOfField=null;this.shine=null;this.scenePass=null;
    this.retire(()=>{for(const resource of resources)resource?.dispose();});
  }
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
      const prepared=styleWarmup(this.scene,m=>this.materialVariant(m,style));
      const timing:StyleSwitchMetrics={target:style,samples:prepared.scene.children.filter(o=>o instanceof THREE.Mesh).length,prepareMs:performance.now()-prepareStart,compiledSamples:0,maxSubmitMs:0,submitMs:0,waitMs:0,applyMs:0,outcome:'compiling'};
      this.styleSwitchMetrics=timing;
      this.pendingCompiles++;
      try{
        await compileStyleSamples(prepared.scene,
          scene=>this.scenePass
            ?compileForScenePass(this.renderer,this.scenePass,scene,this.camera,prepared.targetScene)
            :this.renderer.compileAsync(scene,this.camera,prepared.targetScene),
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
    this.style=style;this.art.setStyle(style);this.apartmentResources.art.setStyle(style);this.chunks.setContoursVisible(style==='anime');this.scene.traverse(o=>{if(o instanceof THREE.Mesh)o.material=Array.isArray(o.material)?o.material.map(m=>this.materialVariant(m)):this.materialVariant(o.material);});
    this.opticalScale.value=style==='miniature'?.01:.04;this.bokeh.value=this.tiltShift.enabled?this.tiltShift.blur:(style==='miniature'?16:4);
  }
  private materialVariant(material:THREE.Material,style:Style=this.style){return this.apartmentResources.owns(material)?this.apartmentResources.variant(material,style):this.art.variant(material,style);}
  private apartmentSignature(b:Building,original?:BuildingOriginalFootprint){
    const choice=this.apartmentChoices.get(b.id)??selectApartmentArchitecture(b,this.apartmentEra,!!original);
    const business=this.businessChoices?.get(b.id);
    if(business)return buildingRenderSignature(b,original,this.signage.get(b.id))+`:business-${BUSINESS_RECIPE_VERSION}:${business.id}:${this.commercialDisplays}:${this.apartmentDetails.get(b.id)??'district'}:${apartmentPhase(b)}`;
    return buildingRenderSignature(b,original,this.signage.get(b.id))+`:displays-${this.commercialDisplays}`+(choice.kind!=='legacy'?`:architecture-${APARTMENT_RECIPE_VERSION}:${apartmentArchitectureKey(choice)}:${this.apartmentDetails.get(b.id)??'district'}:${apartmentPhase(b)}`:'');
  }
  setCommercialDisplays(value:DisplayMode){const mode=parseDisplayMode(value);if(mode===this.commercialDisplays)return;this.commercialDisplays=mode;if(this.worldState)this.setWorld(this.worldState);}
  setApartmentEra(value:ApartmentEra){const mode=parseApartmentEra(value);if(mode===this.apartmentEra)return;this.apartmentEra=mode;this.apartmentLayout='';if(this.worldState)this.setWorld(this.worldState);}
  private updateApartmentDetail(now:number){
    if(!this.worldState||now-this.apartmentDetailAt<1000)return;this.apartmentDetailAt=now;
    const originals=new Set(this.worldState.building_original_footprints?.map(b=>b.building_id));let changed=false;
    for(const b of this.worldState.buildings)if(this.businessChoices?.has(b.id)||(this.apartmentChoices.get(b.id)??selectApartmentArchitecture(b,this.apartmentEra,originals.has(b.id))).kind!=='legacy'){
      const previous=this.apartmentDetails.get(b.id),next=apartmentDetail(Math.hypot(b.x-this.camera.position.x,b.z-this.camera.position.z,b.floors*1.625-this.camera.position.y),previous);
      if(previous!==next){this.apartmentDetails.set(b.id,next);changed=true;}
    }
    if(changed)this.setWorld(this.worldState);
  }
  private retire(dispose:()=>void){if(this.pendingCompiles)this.retiredResources.push(dispose);else dispose();}
  private retireGroup(group:THREE.Group){group.removeFromParent();this.retire(()=>disposeGroup(group));}
  setPreview(hour:number|null){this.preview=hour;}
  toggleWaterReflectionForReview(){return this.water?.toggleReflectionForReview()??false;}
  toggleStreetLightsForReview(){return this.nearbyStreetLights.enabled=!this.nearbyStreetLights.enabled;}
  setLens(id:LensPresetId,takeManualControl=true){this.director.setLens(id,takeManualControl);const lens=LENS_PRESETS[id];this.lensFocalM.value=lens.focalLengthMm/1000;this.lensFNumber.value=lens.fNumber;}
  setTiltShift(settings:Partial<TiltShiftSettings>){
    this.tiltShift=normalizeTiltShift({...this.tiltShift,...settings});
    this.tiltEnabled.value=this.tiltShift.enabled?1:0;
    this.tiltWidth.value=this.tiltShift.width;
    this.bokeh.value=this.tiltShift.enabled?this.tiltShift.blur:(this.style==='miniature'?16:4);
  }
  setFocus(value:boolean){
    if(this.focusEnabled===value)return;
    this.focusEnabled=value;this.dofEnabled.value=value?1:0;
    if(this.pipeline)this.pipeline=value?this.focusedPipeline:this.unblurredPipeline;
  }
  setQuality(quality:'high'|'balanced'){this.quality=quality;this.qualityController.setProfile(quality);this.applyQuality();}
  setFixedReviewQuality(enabled:boolean){
    if(!import.meta.env.DEV)return;
    this.fixedReviewQuality=enabled;this.qualityController.reset();this.applyQuality();
  }
  reviewQualitySnapshot(){
    const buffer=this.renderer.getDrawingBufferSize(new THREE.Vector2());
    return {mode:this.fixedReviewQuality?'fixed-review':'adaptive',profile:this.quality,
      adaptiveLevel:this.qualityController.level,
      ...renderQualitySettings(this.quality,this.qualityController.level,window.devicePixelRatio,this.fixedReviewQuality),
      actualBuffer:buffer.toArray()};
  }
  private applyQuality(){
    // Reduce reflection/shadow cost before lowering the focal scene's resolution.
    const settings=renderQualitySettings(this.quality,this.qualityController.level,window.devicePixelRatio,this.fixedReviewQuality);
    this.renderer.setPixelRatio(settings.pixelRatio);
    this.sun.shadow.mapSize.set(settings.shadowSize,settings.shadowSize);
    this.water?.setReflectionScale(settings.reflectionScale);
    this.setSize();
  }
  setSelected(id:number|null){this.selected=id;}
  private setSize(){const w=this.element.clientWidth,h=this.element.clientHeight;if(!w||!h)return;this.camera.aspect=w/h;this.camera.updateProjectionMatrix();this.renderer.setSize(w,h);this.outputHeight.value=this.renderer.getDrawingBufferSize(new THREE.Vector2()).y;}
  setWorld(world:World,replaced=false){
    this.worldState=world;this.season=seasonAt(world.day,world.minute_of_day);this.hour=world.minute_of_day/60;this.rain=world.weather?.intensity??0;
    if(replaced||this.terrainSeed!==world.seed||world.tick<this.worldTick){this.resetWorld();this.buildTerrain(world);this.terrainSeed=world.seed;}
    this.infrastructure().request(world);
    this.worldTick=world.tick;this.director.world(world);this.buses.setWorld(world);this.trains.setWorld(trainWorldWithInfrastructure(world,this.committedRoadWorld));this.accessPaths.setWorld(world);this.rearExtensions.setWorld(world);

    const signatures=roadBatchSignatures(world);
    const signature=signatures.completed+signatures.construction;
    if(signature!==this.roadSignature){this.rebuildRoads(world,signatures.completed);this.roadSignature=signature;}
    this.sidewalks.setWorld(world);
    const groundSignature=JSON.stringify(world.ground??[]);
    if(groundSignature!==this.groundSignature){
      this.groundSignature=groundSignature;
      this.retireGroup(this.ground);
      this.ground=buildGroundSurfaces(world.ground??[],this.art);
      this.scene.add(this.ground);
    }
    const parkKey=`${signature}#${parkSignature(world)}`;
    if(parkKey!==this.parkKey){this.parkKey=parkKey;this.rebuildParks(world);}
    if(seasonKey(this.season)!==this.paintedSeason)this.paintSeason();
    const ids=new Set(world.buildings.map(b=>b.id));
    for(const id of this.buildings.keys())if(!ids.has(id)){this.chunks.remove(id);this.buildings.delete(id);this.apartmentDetails.delete(id);}
    for(const id of this.objectQueue.keys())if(!ids.has(id))this.objectQueue.delete(id);
    const originals=new Map((world.building_original_footprints??[]).map(original=>[original.building_id,original]));
    const layout=world.buildings.map(b=>`${b.id}:${b.kind}:${b.anchor}:${b.appearance_seed}:${b.floors}:${b.width}:${b.depth}:${b.x}:${b.z}:${b.stage}:${originals.has(b.id)}`).join('|');
    if(layout!==this.apartmentLayout){this.apartmentLayout=layout;this.apartmentChoices=resolveApartmentArchitectures(world.buildings,this.apartmentEra,new Set(originals.keys()));this.businessChoices=resolveBusinessRecipes(world.buildings,this.apartmentEra,new Set(originals.keys()));}
    const signage=this.signage=signageContexts(world);
    for(const b of world.buildings){
      const original=originals.get(b.id);
      const context=signage.get(b.id);
      const architecture=this.apartmentChoices.get(b.id)??selectApartmentArchitecture(b,this.apartmentEra,!!original);
      const business=this.businessChoices.get(b.id);
      if((business||architecture.kind!=='legacy')&&!this.apartmentDetails.has(b.id))this.apartmentDetails.set(b.id,apartmentDetail(Math.hypot(b.x-this.camera.position.x,b.z-this.camera.position.z,b.floors*1.625-this.camera.position.y)));
      const signature=this.apartmentSignature(b,original);
      if(this.buildings.get(b.id)?.signature===signature){this.objectQueue.delete(b.id);continue;}
      // One pending update per authoritative building. Repeated snapshots replace
      // its data without appending duplicate closures or starving older parcels.
      const snapshot:Building={...b,history:[...b.history]};
      const originalSnapshot=original?{...original}:undefined;
      const detail=this.apartmentDetails.get(b.id)??'district',displays=this.commercialDisplays;
      const source:RebuildableBuildingSource={
        createGroup:()=>{const group=(business?buildBusinessBuilding(snapshot,this.apartmentResources.art,{recipe:business,detail,displays,signage:context}):null)??buildSelectedApartment(snapshot,this.apartmentResources.art,architecture,detail)??buildExtendedBuilding(snapshot,originalSnapshot,this.art,{compactMidrise:this.compactMidriseReview,signage:context,commercialDisplays:displays});group.position.set(snapshot.x,0,snapshot.z);group.rotation.y=snapshot.rotation;return group;},
        disposeGroup:group=>disposeGroup(group),
      };
      this.objectQueue.set(b.id,{building:snapshot,signature,source});
    }
    this.updateTrees(world);
  }
  setMovement(data:Float32Array,tick:number,minute:number,trainDoors:Float32Array,support:ReadonlyMap<number,MovementSupport>){
    this.trains.setMovement(trainDoors);
    const now=performance.now();this.interval=Math.max(20,Math.min(250,now-this.received));this.received=now;this.motionTick=tick;this.hour=minute/60;
    this.movementPresentation.receive(data,support,this.interval);
  }

  private resetWorld(){
    this.frameTiming=new FrameTiming();this.lastMetricsAt=0;
    this.retireGroup(this.ground);this.ground=new THREE.Group();this.scene.add(this.ground);this.groundSignature='';
    this.focusResetPending=true;
    this.umbrellas.reset();this.buses.reset();this.fleet.reset();this.trains.reset();this.accessPaths.reset();this.rearExtensions.reset();this.umbrellaCount=0;this.qualityController.reset();
    this.chunks.clear();
    this.sidewalks.reset();
    this.cancelRoadBuild();
    this.infrastructure().reset();
    this.nearbyStreetLights?.reset();this.streetLighting?.reset();
    this.buildings.clear();this.apartmentDetails.clear();this.apartmentChoices.clear();this.businessChoices.clear();this.apartmentLayout='';
    this.clearParks();this.parkKey='';this.paintedSeason='';
    this.retireGroup(this.terrain);const near=this.treeNearGeometry,far=this.treeFarGeometry,trunk=this.treeTrunkGeometry;this.retire(()=>{near?.dispose();far?.dispose();trunk?.dispose();});this.treeNearGeometry=null;this.treeFarGeometry=null;this.treeTrunkGeometry=null;this.treeBatches=[];this.terrain=new THREE.Group();this.scene.add(this.terrain);
    this.retireGroup(this.roads);this.roads=new THREE.Group();this.scene.add(this.roads);
    this.retireGroup(this.constructionRoads);this.constructionRoads=new THREE.Group();this.scene.add(this.constructionRoads);this.completedRoadSignature='';this.completedRoadSurfaces=null;
    const oldWater=this.water;oldWater?.mesh.removeFromParent();this.retire(()=>oldWater?.dispose());this.water=null;this.waterTime=0;this.motionTick=0;this.motionDelta=.05;this.received=performance.now();this.current.clear();this.previous.clear();this.movementPresentation?.reset();this.actorSampleAt=0;this.committedRoadWorld=null;this.objectQueue.clear();this.roadSignature='';this.selected=null;this.treePoints=[];
  }
  private buildTerrain(world:World){
    const river=world.terrain.river;const halfWidth=this.riverHalfWidth=world.terrain.river_half_width??32;
    // Natural land is the underlay for near-coplanar block/parcel paving.
    // Bias only its depth away from the camera: moving the actual surface would
    // leave tree roots and banks floating, while biasing paving can hide details.
    const land=new THREE.MeshStandardNodeMaterial({roughness:1,polygonOffset:true,polygonOffsetFactor:2,polygonOffsetUnits:2});const meadow=mx_noise_float(positionLocal.mul(.024)).mul(.5).add(.5);
    const scrub=mx_noise_float(positionLocal.mul(.007)).mul(.5).add(.5);
    const landBase=mix(mix(color('#577d61'),color('#94ab6d'),meadow),color('#c2bc8c'),smoothstep(.59,.79,scrub).mul(.55)).mul(mx_noise_float(positionLocal.mul(.6)).mul(.075).add(.965));
    land.colorNode=landBase.mul(this.art.cloudShade());
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
    backdropMaterial.colorNode=mix(landBase,this.outerLandscapeColor(),smoothstep(420,520,positionLocal.x.abs().max(positionLocal.z.abs()))).mul(this.art.cloudShade());
    const backdrop=new THREE.Mesh(createBackdropGeometry(world.seed,river,halfWidth),backdropMaterial);
    backdrop.userData.ownedMaterial=true;backdrop.receiveShadow=true;this.terrain.add(backdrop);
    // Low-poly crowns give the wooded hills a broken silhouette; haze softens
    // them with distance. One instanced draw, no shadows (outside the frustum).
    const forest=backdropForestMatrices(backdrop.geometry,world.seed);
    if(forest.length){
      const crowns=new THREE.InstancedMesh(new THREE.IcosahedronGeometry(1,0),this.art.get('#e9eee5',.95),forest.length);
      const tones=['#4a6347','#58704c','#3f5843','#66784f'].map(c=>new THREE.Color(c));
      forest.forEach((matrix,i)=>{crowns.setMatrixAt(i,matrix);crowns.setColorAt(i,tones[i%tones.length]);});
      crowns.instanceMatrix.needsUpdate=true;crowns.computeBoundingSphere();this.terrain.add(crowns);
    }
    const bankKit=new Kit();
    // Sweep the same X-offset boundary used by land and water. Shared bend
    // stations keep the wall and path continuous without overlapping boxes.
    for(const side of [-1,1] as const){
      bankKit.add(createRiverbankGeometry(river,halfWidth,side,0,1.4,-1.3,.04),this.art.get('#9b9e8b'));
      bankKit.add(createRiverbankGeometry(river,halfWidth,side,0,1.6,.04,.18),this.art.get('#b0b2a0'));
      bankKit.add(createRiverbankGeometry(river,halfWidth,side,5.5,10.5,-.02,.1),this.art.get('#c2c2a6'));
    }
    this.terrain.add(bankKit.finish(false));
    const rng=random(world.seed+937);
    const positions:Array<{x:number,z:number,s:number,h?:number}>=[];
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
    positions.push(...riverbankTreePoints(river,halfWidth));
    this.treePoints=positions;this.treeClearanceKey='';
    // Small overlapping leaf masses give a broken canopy silhouette at both
    // observation distances, rather than a row of identical smooth spheres.
    const foliage=createFoliageGeometry('near');
    this.treeNearGeometry=foliage;this.treeFarGeometry=createFoliageGeometry('far');
    this.treeTrunkGeometry=createTreeTrunkGeometry();
    this.groveLook=positions.map((p,i)=>{
      const blossom=Math.abs(p.x-this.riverX(river,p.z)-halfWidth-17)<.01,variation=((i*2654435761)>>>0)/4294967296;
      return blossom?{species:'blossom',color:'#e5bdba',variation}:{species:'broadleaf',color:['#758a67','#899b70','#607e67','#a0aa7a'][i%4],variation};
    });
    this.treeBatches=this.createTreeBatches(positions,(_,local,i)=>treeColor(this.groveLook[i].species,this.groveLook[i].color,this.season,this.groveLook[i].variation));
    for(const batch of this.treeBatches)this.terrain.add(batch.trees,batch.trunks);
    this.updateTrees(world);
    this.buildMountains(rng);
    this.buildSkyline(world.seed,river);
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
  /** Distant land as a working landscape rather than one mottled lawn: a
   * suburban roofscape continues the city past its edge, farmland fills the
   * valley floor and forest climbs the hills. All of it sits behind range haze. */
  private outerLandscapeColor(){
    const p=positionLocal;
    // Field parcels on a slightly rotated grid, each with its own crop tone.
    const u=p.x.mul(.92).add(p.z.mul(.39)).div(74),v=p.z.mul(.92).sub(p.x.mul(.39)).div(46);
    const parcel=mx_noise_float(vec3(floor(u),floor(v),3.1).mul(.61)).mul(.5).add(.5);
    const crops=mix(mix(color('#7d985c'),color('#a7a272'),smoothstep(.3,.62,parcel)),color('#5d7a4e'),smoothstep(.7,.86,parcel));
    const fu=fract(u),fv=fract(v);
    const hedge=smoothstep(.0,.05,fu.min(fu.oneMinus()).min(fv.min(fv.oneMinus())));
    const fields=mix(color('#566b47'),crops,hedge.mul(.35).add(.65));
    // Forest on higher ground and in irregular woods on the plain. A warped,
    // multi-octave mask gives ragged woodland edges rather than smooth ovals.
    const warp=vec2(mx_noise_float(vec2(p.x,p.z).mul(.02)),mx_noise_float(vec2(p.z,p.x).mul(.02).add(7.3))).mul(24);
    const wp=vec2(p.x,p.z).add(warp);
    const woods=mx_noise_float(wp.mul(.005)).mul(.7).add(mx_noise_float(wp.mul(.021)).mul(.3)).mul(.5).add(.5);
    const forestMask=smoothstep(26,58,p.y).max(smoothstep(.58,.7,woods));
    const crowns=mx_noise_float(p.mul(.21)).mul(.5).add(.5);
    const canopy=mix(color('#4a6248'),color('#6c8053'),crowns.mul(crowns));
    // Distant buildings are silhouettes/geometry, not a translucent roof grid
    // painted over fields. That grid produced ghost rectangles and moire at
    // grazing angles, even with shadows and AO disabled.
    return mix(fields,canopy,forestMask);
  }
  private buildSkyline(seed:number,river:Point[]){
    const material=new THREE.MeshBasicNodeMaterial({side:THREE.DoubleSide});
    // World-space window cells: sparse warm lights after dusk, flat haze by day.
    const cell=floor(vec3(positionLocal.x.add(positionLocal.z).mul(.31),positionLocal.y.mul(.29),0));
    const lit=step(.48,mx_noise_float(cell.mul(.73))).mul(step(6,positionLocal.y));
    material.colorNode=mix(this.skylineColor,color('#e8b878'),lit.mul(this.skylineNight).mul(.55)).mul(mx_noise_float(positionLocal.mul(.02)).mul(.06).add(.97));
    const skyline=new THREE.Mesh(createSkylineGeometry(seed,river),material);
    skyline.userData.ownedMaterial=true;this.terrain.add(skyline);
  }
  /** Instanced trees in 200 m cells so each cell can switch foliage detail by camera distance. */
  private createTreeBatches<T extends {x:number,z:number}>(points:readonly T[],colorOf:(p:T,local:number,index:number)=>THREE.Color):TreeBatch[]{
    if(!this.treeNearGeometry||!this.treeTrunkGeometry)return [];
    const cells=new Map<string,number[]>();for(let i=0;i<points.length;i++){const p=points[i],key=`${Math.floor((p.x+400)/200)}:${Math.floor((p.z+400)/200)}`;const list=cells.get(key)??[];list.push(i);cells.set(key,list);}
    const batches:TreeBatch[]=[];
    for(const [key,indices] of cells){
      const trunks=new THREE.InstancedMesh(this.treeTrunkGeometry,this.art.get('#746b5c'),instanceCapacity(indices.length)),trees=new THREE.InstancedMesh(this.treeNearGeometry,this.art.get('#fffffe',.96),instanceCapacity(indices.length)),center=new THREE.Vector3();
      trunks.name=`tree-trunks-${key}`;trees.name=`tree-crowns-${key}`;trunks.count=trees.count=indices.length;
      trunks.instanceMatrix.setUsage(THREE.DynamicDrawUsage);trees.instanceMatrix.setUsage(THREE.DynamicDrawUsage);trees.castShadow=true;trunks.castShadow=true;
      for(let local=0;local<indices.length;local++){const p=points[indices[local]];center.x+=p.x;center.z+=p.z;trees.setColorAt(local,colorOf(p,local,indices[local]));}
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
    // Retain instance identity across nearby development; fresh InstancedMesh
    // objects otherwise rebuild hundreds of node programs on a world update.
    const previousBatches=this.parkBatches;
    const previousTrees=new Map(previousBatches.map(b=>[b.trees.name,b]));
    for(const batch of previousBatches){batch.trees.removeFromParent();batch.trunks.removeFromParent();}
    const previousWires=this.parks.getObjectByName('overhead-lines') as THREE.Group|undefined;
    previousWires?.removeFromParent();
    this.parkBatches=[];
    this.clearParks();
    const streetTrees=streetTreePlacements(world),kit=new Kit(),planted:PlantedTree[]=[...streetTrees];
    for(const parcel of openSpaces(world)){
      const kind=openSpaceKind(parcel);
      planted.push(...(kind==='plaza'?addStationPlaza:kind==='shrine'?addNeighborhoodShrine:addPocketPark)(kit,parcel,this.art));
    }
    addScrambleCrossing(kit,world,this.art);
    for(const polygon of world.terrain.protected)planted.push(...addShrineGrove(kit,polygon,this.art));
    this.parks.add(kit.finish(),buildOverheadLines(world,this.art,streetTrees,previousWires));
    this.parkLook=planted;
    this.parkBatches=this.createTreeBatches(planted,p=>treeColor(p.species,p.color,this.season,p.variation));
    for(const batch of this.parkBatches){
      for(let local=0;local<batch.indices.length;local++){const i=batch.indices[local],p=planted[i];dummy.position.set(p.x,0,p.z);dummy.rotation.set(0,i*2.39996,0);dummy.scale.setScalar(p.s/1.8);dummy.updateMatrix();batch.trees.setMatrixAt(local,dummy.matrix);batch.trunks.setMatrixAt(local,dummy.matrix);}
      const previous=previousTrees.get(batch.trees.name);
      const nearGeometry=batch.trees.geometry;
      batch.trees=reuseInstances(previous?.trees,batch.trees);
      batch.trunks=reuseInstances(previous?.trunks,batch.trunks);
      batch.trees.geometry=nearGeometry;
      batch.trees.instanceMatrix.needsUpdate=true;batch.trunks.instanceMatrix.needsUpdate=true;
      batch.trees.computeBoundingSphere();batch.trunks.computeBoundingSphere();
      this.parks.add(batch.trees,batch.trunks);
    }
    const retained=new Set(this.parkBatches.flatMap(b=>[b.trees,b.trunks]));
    for(const batch of previousBatches)for(const mesh of [batch.trees,batch.trunks])if(!retained.has(mesh))this.retire(()=>mesh.dispose());
    if(previousWires)this.retireGroup(previousWires);
  }
  /** Repaint every crown for the current season; called only when its quantized key changes. */
  private paintSeason(){
    this.paintedSeason=seasonKey(this.season);
    const paint=(batches:TreeBatch[],looks:TreeLook[])=>{for(const batch of batches){
      for(let local=0;local<batch.indices.length;local++){const look=looks[batch.indices[local]];if(look)batch.trees.setColorAt(local,treeColor(look.species,look.color,this.season,look.variation));}
      if(batch.trees.instanceColor)batch.trees.instanceColor.needsUpdate=true;
    }};
    paint(this.treeBatches,this.groveLook);paint(this.parkBatches,this.parkLook);
  }
  private updateTrees(world:World){
    const clearanceKey=treeClearanceSignature(world);
    if(clearanceKey===this.treeClearanceKey)return;
    const nodes=new Map(world.nodes.map(n=>[n.id,n]));
    const onPaving=createPavedGroundTest(world.ground??[]);
    const maxCanopy=Math.max(0,...this.treePoints.map(p=>p.s*1.7+2));
    const candidatesAt=createTreeClearanceGrid(world.buildings,world.roads,(world.rear_extensions??[]).slice(0,2),world.nodes,maxCanopy,kind=>roadWidth(kind)/2+(kind==='bridge'?2.5+maxCanopy:kind==='lane'?1.5:3.5),world.rail_alignments);
    const railPaths=new Map((world.rail_alignments??[]).map(a=>[a.road_id,sampledRail(a)]));
    for(const batch of this.treeBatches){for(let local=0;local<batch.indices.length;local++){
      const i=batch.indices[local];
      const p=this.treePoints[i];
      const candidates=candidatesAt(p);
      let hidden=onPaving(p)||candidates.buildings.some(b=>{const dx=p.x-b.x,dz=p.z-b.z,c=Math.cos(b.rotation),s=Math.sin(b.rotation),localX=c*dx-s*dz,localZ=s*dx+c*dz,canopy=p.s*1.7+2;
        return Math.abs(localX)<b.width/2+canopy&&localZ>-b.depth/2-canopy&&localZ<b.depth/2+canopy+(b.anchor?4:0);
      });
      if(!hidden)hidden=candidates.roads.some(r=>{const a=nodes.get(r.from),b=nodes.get(r.to);const path=railPaths.get(r.id),nearest=path?.at(path.distanceAt(p));return a&&b&&(nearest?Math.hypot(p.x-nearest.x,p.z-nearest.z):distanceToSegment(p,a,b))<roadWidth(r.kind)/2+(r.kind==='bridge'?2.5+2.38*p.s/1.8:r.kind==='lane'?1.5:3.5);});
      if(!hidden)hidden=(world.rail_platforms??[]).some(platform=>overlapsStation(p,p.s*1.7+2,platform));
      if(!hidden)hidden=candidates.rearExtensions.some(e=>{const dx=p.x-e.x,dz=p.z-e.z,c=Math.cos(e.rotation),s=Math.sin(e.rotation),canopy=p.s*1.7+2;return Math.abs(c*dx-s*dz)<e.width/2+canopy&&Math.abs(s*dx+c*dz)<e.depth/2+canopy;});
      dummy.position.set(p.x,0,p.z);dummy.rotation.set(0,i*2.39996,0);dummy.scale.set(hidden?0:p.s/1.8,hidden?0:p.s/1.8*(p.h??1),hidden?0:p.s/1.8);dummy.updateMatrix();batch.trunks.setMatrixAt(local,dummy.matrix);batch.trees.setMatrixAt(local,dummy.matrix);
    }
    batch.trees.instanceMatrix.needsUpdate=true;batch.trunks.instanceMatrix.needsUpdate=true;batch.trees.computeBoundingSphere();batch.trunks.computeBoundingSphere();}
    this.treeClearanceKey=clearanceKey;
  }
  private updateTreeDetail(){
    if(!this.treeNearGeometry||!this.treeFarGeometry)return;
    for(const batch of [...this.treeBatches,...this.parkBatches]){const distance=this.camera.position.distanceTo(batch.center),far=batch.far?distance>280:distance>330;if(far!==batch.far){batch.far=far;batch.trees.geometry=far?this.treeFarGeometry:this.treeNearGeometry;batch.trees.computeBoundingSphere();}}
  }
  private rebuildRoads(world:World,completedSignature=roadBatchSignatures(world).completed){
    const copy=structuredClone({nodes:world.nodes,roads:world.roads,projects:world.projects,
      expressway_profile:world.expressway_profile,road_decks:world.road_decks,rail_platforms:world.rail_platforms,terrain:world.terrain});
    world={...world,...copy};
    if(this.roadBuild||this.queuedRoadWorld){
      // Keep one active coherent build and one latest input. Restarting an
      // active job on every snapshot can prevent it from ever becoming visible.
      this.queuedRoadWorld={world,signature:completedSignature};
      return;
    }
    this.prepareRoads(world,completedSignature);
  }
  private cancelRoadBuild(){
    if(this.roadBuild){const job=this.roadBuild;job.preparation?.return();job.kit.dispose();disposeGroup(job.construction);disposeGroup(job.station);}
    this.roadBuild=null;this.queuedRoadWorld=null;
  }
  private updateRoads(){
    if(!this.roadBuild){
      if(this.queuedRoadWorld){
        const next=this.queuedRoadWorld;this.queuedRoadWorld=null;
        try{this.prepareRoads(next.world,next.signature);}
        catch(error){this.roadSignature='';throw error;}
      }
      return;
    }
    const job=this.roadBuild;let built:THREE.Group|null=null;
    try{
      if(job.preparation){
        const deadline=performance.now()+2;
        for(let units=0;units<128;units++){
          if(job.preparation.next().done){job.preparation=null;break;}
          if(performance.now()>=deadline)break;
        }
        return;
      }
      const deadline=performance.now()+2;
      let complete=false;
      // Allow cheap slices to share a frame, but never drain a whole job here.
      // A single primitive/merge slice may exceed this soft time budget.
      for(let slices=0;slices<8;slices++){
        if(job.kit.step()){complete=true;break;}
        if(performance.now()>=deadline)break;
      }
      if(!complete)return;
      built=job.kit.takeGroup()!;
      const roads=new THREE.Group();roads.add(built);
      // Detached candidates miss scene-wide style traversal during preparation.
      for(const group of [roads,job.construction,job.station])group.traverse(o=>{if(o instanceof THREE.Mesh)o.material=Array.isArray(o.material)?o.material.map(m=>this.art.variant(m)):this.art.variant(o.material);});
      this.retireGroup(this.roads);this.retireGroup(this.constructionRoads);this.retireGroup(this.station);
      this.roads=roads;this.constructionRoads=job.construction;this.station=job.station;
      this.scene.add(this.roads,this.constructionRoads,this.station);
      if(job.lighting.data){
        if(!this.streetLighting){this.streetLighting=new PersistentStreetLighting(this.art);this.scene.add(this.streetLighting.root);}
        this.streetLighting.commit(job.lighting.data);
        this.nearbyStreetLights?.setSources(job.lighting.data.sources);
      }
      this.infrastructure().commit(job.identities);
    const bridgeNodes=new Map(job.world.nodes.map(n=>[n.id,n]));this.bridgeSpans=job.world.roads.filter(r=>r.kind==='bridge'&&r.stage==='open').flatMap(r=>{const a=bridgeNodes.get(r.from),b=bridgeNodes.get(r.to);return a&&b?[{a,b}]:[];});
      this.committedRoadWorld=job.world;
      this.trains.setWorld(trainWorldWithInfrastructure(this.worldState??job.world,job.world));
      this.completedRoadSignature=job.signature;this.completedRoadSurfaces=job.surfaces;this.roadBuild=null;
    }catch(error){if(built)disposeGroup(built);this.cancelRoadBuild();this.roadSignature='';throw error;}
  }
  private prepareRoads(world:World,completedSignature:string){
    const kit=new StagedKit(),constructionKit=new Kit();
    const construction=new THREE.Group(),station=new THREE.Group(),lighting:{data:StreetLightingData|null}={data:null};
    const rebuildCompleted=completedSignature!==this.completedRoadSignature;
    const surfaces=rebuildCompleted?new RoadSurfacePartition():(this.completedRoadSurfaces??new RoadSurfacePartition());
    const preparation=this.populateRoads(world,rebuildCompleted,kit,constructionKit,construction,station,lighting,surfaces);
    if(rebuildCompleted){
      this.roadBuild={kit,construction,station,lighting,signature:completedSignature,surfaces,identities:this.infrastructure().capture(world),world,preparation};
    }else{
      try{
        // Construction-only refreshes contain no preparation yields.
        preparation.next();
        kit.dispose();this.retireGroup(this.constructionRoads);
        this.constructionRoads=construction;this.scene.add(construction);
      }catch(error){kit.dispose();disposeGroup(construction);disposeGroup(station);throw error;}
    }
  }
  private *populateRoads(world:World,rebuildCompleted:boolean,kit:StagedKit,constructionKit:Kit,
    construction:THREE.Group,station:THREE.Group,lighting:{data:StreetLightingData|null},surfaces:RoadSurfacePartition):Generator<void,void,unknown>{
    const nodes=new Map(world.nodes.map(n=>[n.id,n]));
    try{
    if(rebuildCompleted){
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
        yield;
      }
    }
    const deckHeights=new Map((world.road_decks??[]).map(d=>[d.road_id,d.deck_height]));
    const railAlignments=new Map((world.rail_alignments??[]).map(a=>[a.road_id,a]));
    const crossings:CrossingDeck[]=world.roads.flatMap(road=>{
      const alignment=railAlignments.get(road.id);
      if(alignment)return alignment.samples.slice(1).map((s,i)=>({a:{x:alignment.samples[i][1],z:alignment.samples[i][2]},b:{x:s[1],z:s[2]},width:roadWidth(road.kind),height:1+RAIL_DECK}));
      const a=nodes.get(road.from),b=nodes.get(road.to);
      return a&&b?[{a,b,width:road.kind==='expressway'?(world.expressway_profile??CURRENT_EXPRESSWAY_PROFILE).width:roadWidth(road.kind),height:road.kind==='expressway'?(deckHeights.get(road.id)??EXPRESSWAY_DECK):road.kind==='rail'?1+RAIL_DECK:0}]:[];
    });
    const curves=new Map<number,RoadCurve>();
    for(const alignment of world.rail_alignments??[])curves.set(alignment.road_id,sampledRail(alignment));
    const deckById=new Map((world.road_decks??[]).map(d=>[d.road_id,d]));
    for(const road of world.roads){
      const a=nodes.get(road.from),b=nodes.get(road.to),deck=deckById.get(road.id);
      if(a&&b&&deck){const [c,d]=deck.controls;curves.set(road.id,bezierRoad(a,{x:c[0],z:c[1]},{x:d[0],z:d[1]},b));}
    }
    const markingTrims=rebuildCompleted?roadMarkingTrims(world):new Map<number,MarkingTrim>();
    // Existing pavement owns its footprint before construction claims the remainder.
    // Stable ordering also removes coplanar overlap between neighboring open roads.
    const orderedRoads=[...world.roads].sort((a,b)=>a.id-b.id);
    if(rebuildCompleted)for(const road of orderedRoads){
      if(road.stage!=='open')continue;
      const a=nodes.get(road.from),b=nodes.get(road.to);if(!a||!b)continue;
      this.roadKit(kit,a,b,road,deckHeights.get(road.id),crossings,curves.get(road.id),world.rail_platforms,markingTrims.get(road.id),surfaces,world.expressway_profile);
      yield;
    }
    if(rebuildCompleted){
      const junctions=new Map<number,JunctionArm[]>();
      for(const road of world.roads){
        if(road.stage!=='open'||road.kind==='rail'||road.kind==='bridge'||road.kind==='expressway')continue;
        const a=nodes.get(road.from),b=nodes.get(road.to);if(!a||!b)continue;
        for(const [node,end] of [[a,b],[b,a]]){const arms=junctions.get(node.id)??[];arms.push({end,width:roadWidth(road.kind)});junctions.set(node.id,arms);}
      }
      for(const [id,arms] of junctions){
        const node=nodes.get(id)!;const points=junctionFootprint(node,arms);
        const pieces=surfaces.claim(points);
        this.addGroundRoadSurface(kit,pieces,this.art.asphalt());
        yield;
      }
    }
    const constructionSurfaces=surfaces.fork();
    for(const road of orderedRoads){
      if(road.stage==='open')continue;
      const a=nodes.get(road.from),b=nodes.get(road.to);
      if(a&&b)this.roadKit(constructionKit,a,b,road,deckHeights.get(road.id),crossings,curves.get(road.id),world.rail_platforms,undefined,constructionSurfaces,world.expressway_profile);
      if(rebuildCompleted)yield;
    }
    for(const project of world.projects.filter(p=>p.kind==='road'&&p.progress<1)){
      const a=nodes.get(project.from!),b=nodes.get(project.to!);if(!a||!b)continue;
      if(world.roads.some(r=>r.from===a.id&&r.to===b.id&&r.stage!=='open'))continue;
      this.roadKit(constructionKit,a,b,{id:project.id,from:a.id,to:b.id,kind:world.roads.find(r=>(r.from===a.id&&r.to===b.id)||(r.from===b.id&&r.to===a.id))?.kind??(Math.abs(a.x-b.x)>100?'bridge':'local'),stage:project.stage,progress:project.progress,connected:false,length:Math.hypot(a.x-b.x,a.z-b.z),lanes:1},undefined,[],undefined,world.rail_platforms,undefined,constructionSurfaces,world.expressway_profile);
    }
    construction.add(constructionKit.finish());
    if(rebuildCompleted)yield;
    if(rebuildCompleted){
      for(const node of world.nodes.filter(n=>n.kind==='market'||n.kind==='station')){for(let stripe=-3;stripe<=3;stripe++)kit.box(node.x+stripe*1.1,.37,node.z, .65,.025,6,this.art.get('#dedaca'));}
      lighting.data=prepareStreetLighting(world);
      yield;
      for(const layout of world.rail_platforms??[]){
        const platform=buildStationPlatform(layout,this.art);
        if(layout.construction_phase===4){
        const signMaterial=new THREE.MeshBasicNodeMaterial({map:signTexture('青葉川  AOBAGAWA'),color:'#fff4d9'});
        const board=new THREE.Mesh(new THREE.BoxGeometry(.08,.72,3.4),signMaterial);board.userData.ownedMaterial=true;board.userData.ownedTexture=signMaterial.map;board.position.set(3.8,layout.surface_y+2.6,24);platform.add(board);
        }
        station.add(platform);
        yield;
      }
    }
    }finally{constructionKit.dispose();}
  }

  private addGroundRoadSurface(kit:Kit,pieces:Point[][],material:THREE.Material){
    for(const points of pieces){
      kit.add(groundRoadSurfaceGeometry(points), material);
    }
  }
  private roadKit(kit:Kit,a:Point & {kind?:string},b:Point & {kind?:string},r:Road,deckHeight=EXPRESSWAY_DECK,crossings:readonly CrossingDeck[]=[],curve?:RoadCurve,platforms=this.worldState?.rail_platforms,markingTrim:MarkingTrim={start:0,end:0},surfaces?:RoadSurfacePartition,expresswayProfile=CURRENT_EXPRESSWAY_PROFILE){
    if(r.kind==='rail'&&curve){
      const open=r.stage==='open',end=curve.length*(open?1:THREE.MathUtils.clamp((r.progress-.12)/.88,0,1));
      const y=1+RAIL_DECK,width=roadWidth('rail'),concrete=this.art.get('#9a9d93');
      if(!open)for(let d=0;d<curve.length;d+=18){const p=curve.at(d);for(const side of [-1,1])kit.box(p.x+p.uz*side*5,.7,p.z-p.ux*side*5,.12,1.4,.12,this.art.get('#b79b73'));}
      if(end<.01)return;
      sweepRoad(kit,curve,0,end,0,y-.35,width+.6,.5,this.art.get('#a3a69c'));
      sweepRoad(kit,curve,0,end,0,y,width,.3,this.art.get(open?'#747f7b':'#ac9a78'));
      for(let d=11;d<end;d+=22){const p=curve.at(d),yaw=Math.atan2(p.ux,p.uz);
        for(const side of [-1,1])kit.box(p.x+p.uz*side*2.4,y/2,p.z-p.ux*side*2.4,1.1,y,1.1,concrete,yaw);
        kit.box(p.x,y-.55,p.z,width+1.2,.7,1.4,concrete,yaw);
      }
      for(const side of [-1,1])sweepRoad(kit,curve,0,end,side*RAIL_CENTER_OFFSET,y+.45,RAIL_HEAD_WIDTH,.17,this.art.get('#a3aca7',.4));
      for(let d=0;d<end;d+=.75){const p=curve.at(d);kit.box(p.x,y+.29,p.z,2.2,.15,.24,this.art.get('#6d6457'),Math.atan2(p.ux,p.uz));}
      if(open)addRailCorridor(kit,a,b,this.art,platforms,curve as RailPath);
      return;
    }
    if(r.kind==='expressway'&&curve){
      const open=r.stage==='open',end=curve.length*(open?1:THREE.MathUtils.clamp((r.progress-.12)/.88,0,1));
      if(!open)for(let distance=0;distance<curve.length;distance+=18){const p=curve.at(distance);for(const side of [-1,1])kit.box(p.x+p.uz*side*5,.7,p.z-p.ux*side*5,.12,1.4,.12,this.art.get('#b79b73'));}
      if(end<.01)return;
      const width=expresswayProfile.width,y=deckHeight,concrete=this.art.get('#9a9d93');
      sweepRoad(kit,curve,0,end,0,y-.35,width+.6,.5,this.art.get('#a3a69c'));
      sweepRoad(kit,curve,0,end,0,y,width,.3,(open?this.art.asphalt():this.art.get('#ac9a78')));
      for(const side of [-1,1])sweepRoad(kit,curve,0,end,side*(width/2+.25),y+.75,.3,1.3,this.art.get('#b8bbb2'));
      for(let distance=14;distance<end;distance+=28){const p=curve.at(distance),angle=Math.atan2(p.ux,p.uz);
        if(!expresswayPierClear(p,{x:p.x-p.ux,z:p.z-p.uz},{x:p.x+p.ux,z:p.z+p.uz},y,crossings))continue;
        kit.box(p.x,y/2,p.z,2.2,y,2.2,concrete,angle);kit.box(p.x,y-.55,p.z,width+1.2,.7,1.4,concrete,angle);
      }
      if(open)addExpresswayFurniture(kit,a,b,{paint:this.art.get('#dedaca'),concrete,metal:this.art.get('#53696a'),lamp:this.art.get('#ead5a3',.5,true)},y,curve,expresswayProfile);
      return;
    }
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
    const width=expressway?expresswayProfile.width:roadWidth(r.kind),y=bridge?1.4:rail?1+RAIL_DECK:expressway?deckHeight:.15;
    if(rail||expressway){
      // Viaduct piers: single columns under the expressway, paired under rail.
      const spacing=expressway?28:22,pierColor=this.art.get('#9a9d93');
      for(let distance=spacing/2;distance<len;distance+=spacing){const t=distance/len,px=a.x+dx*t,pz=a.z+dz*t;
        if(expressway&&!expresswayPierClear({x:px,z:pz},a,b,y,crossings))continue;
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
      const deck=new THREE.ExtrudeGeometry(shape,{depth:width+2.8,bevelEnabled:false,steps:1});deck.setIndex(Array.from({length:deck.getAttribute('position').count},(_,i)=>i));deck.rotateY(-Math.PI/2);deck.translate((width+2.8)/2,0,0);deck.rotateY(angle);deck.translate(mid.x,0,mid.z);kit.add(deck,(open?this.art.asphalt():this.art.get('#ac9a78')));
    }else if(surfaces&&!rail&&!expressway){
      this.addGroundRoadSurface(kit,surfaces.claim(groundRoadFootprint(a,b,width)),(open?this.art.asphalt():this.art.get('#ac9a78')));
    }else kit.box(mid.x,y,mid.z,width,.3,len,(open?this.art.asphalt():this.art.get('#ac9a78')),angle);
    if(expressway){
      if(open)addExpresswayFurniture(kit,a,b,{paint:this.art.get('#dedaca'),concrete:this.art.get('#9a9d93'),metal:this.art.get('#53696a'),lamp:this.art.get('#ead5a3',.5,true)},y,undefined,expresswayProfile);
      return;
    }
    if(rail){
      if(open)addRailCorridor(kit,a,b,this.art,platforms);
      for(const side of [-1,1])kit.box(mid.x+Math.cos(angle)*side*RAIL_CENTER_OFFSET,y+.45,mid.z-Math.sin(angle)*side*RAIL_CENTER_OFFSET,RAIL_HEAD_WIDTH,.17,len,this.art.get('#a3aca7',.4),angle);
      for(let distance=0;distance<len;distance+=.75){const t=distance/len;kit.box(a.x+dx*t,y+.29,a.z+dz*t,2.2,.15,.24,this.art.get('#6d6457'),angle);}
    }else if(lane){
      // Lanes are shared-surface roji: no center line and no raised sidewalk.
    }else{
      if(open){if(bridge||r.kind==='arterial')for(let distance=8;distance<len-4;distance+=12){
        if(!bridge&&(distance-2<markingTrim.start||distance+2>len-markingTrim.end))continue;
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
      // Open arterial sidewalks are committed separately by CitySidewalks.
      // Local streets retain painted edges on their shared surface.
      if(!bridge&&r.kind==='arterial'&&!open)for(const side of [-1,1])kit.box(mid.x+Math.cos(angle)*side*(width/2+ARTERIAL_SIDEWALK/2),y+.15,mid.z-Math.sin(angle)*side*(width/2+ARTERIAL_SIDEWALK/2),ARTERIAL_SIDEWALK,.35,Math.max(1,len-width),this.art.get('#b4b8a6'),angle);
      else if(!bridge&&open&&r.kind!=='arterial'){
        const start=Math.max(width/2,markingTrim.start),end=Math.min(len-width/2,len-markingTrim.end);
        if(end>start)for(const side of [-1,1]){
          const t=(start+end)/(2*len);
          kit.box(a.x+dx*t+Math.cos(angle)*side*(width/2-.45),y+.16,a.z+dz*t-Math.sin(angle)*side*(width/2-.45),.14,.03,end-start,this.art.get('#dedaca'),angle);
        }
      }
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
      // Ordinary streets use buildStreetLighting's shared fixtures and pools.
      // Retain this separate bridge kit until its sloped deck has a light model.
      if(open&&bridge&&len>25){for(let distance=12;distance<len-8;distance+=44){const t=distance/len,x=a.x+dx*t+Math.cos(angle)*(width/2+1.5),z=a.z+dz*t-Math.sin(angle)*(width/2+1.5);const baseY=bridgeDeckHeight(distance,len);kit.cylinder(x,baseY+3.5,z,.1,7,this.art.get('#53696a'),6);kit.box(x-.7,baseY+7,z,1.8,.2,.5,this.art.get('#ead5a3',.5,true));}}
    }
  }
  private updateBuildingBatches(){
    let builtCount=0;const started=performance.now();
    for(const [id,item] of this.objectQueue){
      if(!this.infrastructure().canPresentBuilding(id))continue;
      // A complex apartment may consume the frame budget by itself; never pair
      // it with another expensive build just to meet a fixed object count.
      if(builtCount>=2||(builtCount>0&&performance.now()-started>=3))break;
      builtCount++;
      this.objectQueue.delete(id);
      const group=item.source.createGroup();
      this.chunks.set(id,group,item.source,item.building.stage!=='open');this.buildings.set(id,{source:item.source,signature:item.signature});
    }
    const pendingChunks=new Set<string>();
    for(const [id,{building}] of this.objectQueue)if(this.infrastructure().canPresentBuilding(id))pendingChunks.add(buildingChunkKey(building.x,building.z));
    this.chunks.update(pendingChunks);
  }
  private animate(){
    if(this.disposed)return;
    const now=performance.now(),frameInterval=this.lastFrame?now-this.lastFrame:1000/60,dt=Math.min(.1,frameInterval/1000);this.lastFrame=now;
    const begin=performance.now();
    const timingPhase=this.objectQueue.size||this.chunks.pendingCount||this.sidewalks.pendingCount||this.pendingRoads||this.pendingCompiles?'loading':'ready';
    this.updateApartmentDetail(now);
    this.updateBuildingBatches();
    this.sidewalks.update();
    this.updateRoads();
    if(this.worldState)this.rearExtensions.flushCompleted(this.worldState);
    this.updateActors(now);
    this.director.update(dt,now/1000);
    const activeLens=LENS_PRESETS[this.director.lens];
    this.lensFocalM.value=activeLens.focalLengthMm/1000;this.lensFNumber.value=activeLens.fNumber;
    this.updateTreeDetail();
    this.updateLight(dt);
    const alpha=Math.min(1,(now-this.received)/this.interval);const time=Math.max(0,(this.motionTick-1+alpha)*.05);const waterDt=Math.max(0,Math.min(.1,time-this.waterTime));this.water?.update(waterDt,time,this.rain,this.style,1-this.daylight);this.umbrellas.update(this.umbrellaTuples.subarray(0,this.umbrellaCount*MOVEMENT_STRIDE),this.rain,time);this.art.time.value=time;this.waterTime=time;this.lastTick=this.motionTick;this.rainTime.value=time;this.rainLines.visible=this.rain>.1;this.rainOpacity.value=.035+this.rain*.13;this.rainLines.position.set(this.director.target.x,0,this.director.target.z);
    this.camera.updateMatrixWorld();
    this.focusTargetView.copy(this.director.focusTarget).applyMatrix4(this.camera.matrixWorldInverse);
    const focusTarget=focusDepthFromViewZ(this.focusTargetView.z,this.camera.near,this.camera.far);
    if(this.focusResetPending){this.focus.value=focusTarget;this.focusResetPending=false;}
    else this.focus.value=dampFocusDepth(this.focus.value,focusTarget,dt);
    if(this.tiltShift.enabled){
      writeTiltedFocusPlane(this.tiltPlane.value,this.focus.value,this.camera.fov,this.tiltShift);
      this.tiltOpticalScale.value=miniatureOpticalScale(this.focus.value);
    }
    if(this.selected!==null){const b=this.worldState?.buildings.find(b=>b.id===this.selected);this.highlight.visible=!!b;if(b)this.highlight.position.set(b.x,.22,b.z);}else this.highlight.visible=false;
    if(this.scenePass&&this.pipeline)submitSceneAndEffects(this.renderer,this.scenePass,this.pipeline,this.sceneFrame);
    const cost=performance.now()-begin;
    if(!this.fixedReviewQuality&&this.qualityController.record(frameInterval,now,!document.hidden&&this.objectQueue.size===0&&this.chunks.pendingCount===0&&this.sidewalks.pendingCount===0&&this.pendingRoads===0&&this.pendingCompiles===0)!==null)this.applyQuality();
    this.frame++;
    this.frameTiming.record(now,cost,timingPhase,!document.hidden);
    if(now-this.lastMetricsAt>=1000&&this.onMetrics){this.lastMetricsAt=now;const timing=this.frameTiming.snapshot();this.onMetrics({timing,fps:timing.fps,frameMs:cost,calls:this.renderer.info.render.drawCalls,triangles:this.renderer.info.render.triangles,textures:this.renderer.info.memory.textures,geometries:this.renderer.info.memory.geometries,backend:this.renderer.backend.constructor.name,styleSwitch:this.styleSwitchMetrics?{...this.styleSwitchMetrics}:undefined,pendingBuildings:this.objectQueue.size,pendingChunks:this.chunks.pendingCount,pendingSidewalks:this.sidewalks.pendingCount,pendingRoads:this.pendingRoads});}
  }
  private updateActors(now:number){
    const t=Math.min(1,(now-this.received)/this.interval);let ped=0,train=0,bus=0;this.buses.begin(now);this.trains.begin();this.fleet.begin();const pedIds:number[]=[];
    this.previous=this.current;this.current=this.movementPresentation.sample(now,this.infrastructure());
    this.motionDelta=this.actorSampleAt?Math.max(.001,(now-this.actorSampleAt)/1000):.05;this.actorSampleAt=now;
    const cameraPositions:number[]=[];
    for(const [id,current]of this.current){const old=this.previous.get(id)??current,x=current.x,z=current.z,h=Math.PI/2-current.h;
      const lift=current.k===0?pedestrianSurface(current.y,bridgeLift(current,this.bridgeSpans))-PEDESTRIAN_GROUND_SURFACE:current.k===2?0:bridgeLift({x,z},this.bridgeSpans);
      let surfaceY=current.y;

      if(current.k===0&&ped<2000){surfaceY=lift+PEDESTRIAN_GROUND_SURFACE;const offset=ped*MOVEMENT_STRIDE;this.umbrellaTuples[offset]=id;this.umbrellaTuples[offset+1]=x;this.umbrellaTuples[offset+2]=z;this.umbrellaTuples[offset+3]=Math.PI/2-h;this.umbrellaTuples[offset+4]=0;this.umbrellaTuples[offset+5]=lift;const moving=Math.hypot(current.x-old.x,current.z-old.z)>.01;const step=moving?Math.sin(this.motionTick*.31+id)*.045:0;dummy.position.set(x,1.05+step+lift,z);dummy.rotation.set(0,h,moving?step*.4:0);dummy.scale.setScalar(1);dummy.updateMatrix();this.pedestrians.setMatrixAt(ped,dummy.matrix);this.pedestrians.setColorAt(ped,personColors[id%personColors.length]);dummy.position.y=1.95+step+lift;dummy.updateMatrix();this.heads.setMatrixAt(ped,dummy.matrix);pedIds[ped]=id;ped++;}
      else if(isRoadVehicle(current.k)){const vehicle=vehicleSurfacePose({x,z},h,2.7,this.bridgeSpans,current.y);surfaceY=vehicle.lift+ROAD_SURFACE_Y;this.fleet.add(id,current.k,x,z,h,vehicle.lift,vehicle.pitch,this.motionDelta);}
      else if(current.k===3&&bus<2){const vehicle=bridgeVehiclePose({x,z},h,5.1,this.bridgeSpans);surfaceY=vehicle.lift+ROAD_SURFACE_Y;this.buses.set(bus++,id,x,z,h,this.worldState,vehicle.lift+ROAD_SURFACE_Y,vehicle.pitch);}
      else if(current.k===2&&train<8){this.trains.add(id,x,z,h,RAIL_HEAD_Y,0,this.motionDelta,t);train++;}
      else {this.current.delete(id);continue;}
      cameraPositions.push(id,x,z,current.h,current.k,surfaceY);
    }
    this.director.setPositions(new Float32Array(cameraPositions),this.movementPresentation.withheld(this.current));
    this.umbrellaCount=ped;
    this.trains.finish(train);for(const {mesh,ids} of this.trains.pickable)this.actorIds.set(mesh,ids);
    this.buses.finish(bus);for(const mesh of this.buses.meshes)this.actorIds.set(mesh,this.buses.ids);
    this.actorIds.set(this.pedestrians,pedIds);this.actorIds.set(this.heads,pedIds);this.fleet.finish();for(const {mesh,ids} of this.fleet.pickable)this.actorIds.set(mesh,ids);
    for(const mesh of [this.pedestrians,this.heads]){mesh.count=ped;mesh.instanceMatrix.needsUpdate=true;}
    if(this.pedestrians.instanceColor)this.pedestrians.instanceColor.needsUpdate=true;
  }
  private updateLight(dt:number){
    const hour=this.preview??this.hour,angle=(hour-6)/12*Math.PI;
    const day=THREE.MathUtils.smoothstep(Math.sin(angle),-.2,.12),dusk=1-THREE.MathUtils.smoothstep(Math.abs(Math.sin(angle)),.05,.5);
    this.daylight=THREE.MathUtils.lerp(this.daylight,day,Math.min(1,dt*3));
    const top=new THREE.Color('#659bbd').lerp(new THREE.Color('#9aa5bd'),dusk*.5).lerp(new THREE.Color('#11253e'),1-this.daylight);
    const low=new THREE.Color('#d5ded3').lerp(new THREE.Color('#efb185'),dusk).lerp(new THREE.Color('#334860'),1-this.daylight);
    // Steer the painted gradient toward Preetham scattering for this sun:
    // deeper zenith and a warm sunward horizon at low sun. Painted colours
    // still own night, dusk mood and the clouds; scattering only leads by day.
    const solar=new THREE.Vector3(Math.cos(angle),Math.max(-.05,Math.sin(angle)),-.45).normalize();
    const lead=this.daylight*.42;
    if(lead>.01){
      const zenith=skyRadiance(solar,new THREE.Vector3(0,1,0));
      const toward=new THREE.Vector3(solar.x,0,solar.z).normalize().setY(.04);
      const horizon=skyRadiance(solar,toward).lerp(skyRadiance(solar,toward.clone().set(-toward.x,.04,-toward.z)),.65);
      top.lerp(zenith,lead);low.lerp(horizon,lead*.85);
    }
    this.skyHigh.value.copy(top);this.skyLow.value.copy(low);this.skyCloudShade.value.set('#8ba5bc').lerp(new THREE.Color('#9c829b'),dusk*.7).lerp(new THREE.Color('#283c58'),1-this.daylight);this.hillColor.value.set('#577773').lerp(new THREE.Color('#747386'),dusk*.55).lerp(new THREE.Color('#20384a'),1-this.daylight);this.skyCloud.value.set('#fff3dc').lerp(new THREE.Color('#d29e91'),dusk*.5).lerp(new THREE.Color('#45526a'),1-this.daylight);
    this.skylineColor.value.set('#8193a1').lerp(new THREE.Color('#8a7f93'),dusk*.6).lerp(new THREE.Color('#1c2a3c'),1-this.daylight);this.skylineNight.value=1-this.daylight;
    // Haze matches the horizon so hills, skyline and mountains recede into the
    // sky. The range keeps the city itself clear (overview contrast, spec 05)
    // while the backdrop gains aerial perspective. Rain pulls the veil closer.
    this.hazeColor.value.copy(low).lerp(top,.25);
    const veil=1-this.rain*.45;
    this.hazeNear.value=HAZE_NEAR[this.style]*veil;this.hazeFar.value=HAZE_FAR[this.style]*veil;
    // Morning mist peaks around 05:30-07:00 and burns off by mid-morning.
    const morning=THREE.MathUtils.smoothstep(hour%24,4.5,5.8)*(1-THREE.MathUtils.smoothstep(hour%24,7,9.5));
    this.mist.value=Math.min(.7,morning*.5+this.rain*.3);
    const solarY=Math.sin(angle)*450;this.sunDisc.position.set(Math.cos(angle)*1050,solarY*3,-480);this.sunDisc.visible=solarY>-25;this.moonDisc.position.set(-Math.cos(angle)*1050,-solarY*3,480);this.moonDisc.visible=solarY<25;
    const keyBlend=this.daylight*2-1;
    // Snap only the shadow camera's light-plane translation. The key's
    // direction and elevation still follow the clock, not camera movement.
    // A lower key (about 50 degrees at 14:00) draws longer modelling shadows.
    this.sun.position.set(Math.cos(angle)*350*keyBlend,Math.max(30,Math.abs(Math.sin(angle))*270),-160*keyBlend);
    const shadowCamera=this.sun.shadow.camera;
    snapShadowAnchor(this.director.controls.target,this.sun.position,
      shadowCamera.right-shadowCamera.left,shadowCamera.top-shadowCamera.bottom,
      this.sun.shadow.mapSize.x,this.sun.shadow.mapSize.y,this.sun.target.position);
    this.sun.position.add(this.sun.target.position);
    // Clear daytime key separates sunlit neutral facades from cool sky fill.
    // Fade the extra energy out with dusk so the established night/golden-hour
    // treatment stays intact; ambient and exposure are deliberately unchanged.
    const clearDayKey=this.daylight*(1-dusk)*(1-THREE.MathUtils.clamp(this.rain,0,1));
    this.sun.intensity=.16+this.daylight*2.05+clearDayKey*1.19;
    this.shaftStrength=dusk*THREE.MathUtils.smoothstep(this.daylight,.15,.6)*(1-this.rain);
    this.shaftColor.value.set(dusk>.4?'#ffc58a':'#ffe2b4').multiplyScalar(this.shaftStrength*.45);this.sun.color.set('#aac8e3').lerp(new THREE.Color(dusk>.4?'#ffcc9a':'#ffe6c2'),this.daylight);
    this.sun.color.lerp(new THREE.Color('#fff1db'),clearDayKey);
    this.water?.setSun(v.copy(this.sun.position).sub(this.sun.target.position),new THREE.Color().copy(this.sun.color).multiplyScalar(this.sun.intensity*.5));
    // Soft urban bounce keeps opaque facades and lanes readable between local
    // emitters. The visible sky is not an environment light: retain enough
    // daytime fill to read shaded facades while the sun describes orientation.
    this.hemi.intensity=1.05-this.daylight*.27;
    this.hemi.groundColor.set('#a0a09a').lerp(new THREE.Color('#748998'),this.daylight);
    // A painted moonlit fill keeps rooflines and quiet lanes readable between
    // pools of warm shop light; the sky itself can remain a deep midnight blue.
    this.hemi.color.set('#829eb9').lerp(new THREE.Color('#d8e6e7'),this.daylight);
    // Ambient follows the live sky: warm at dusk, cool at noon, for both the
    // toon (Anime) and standard (Miniature) paths. Standard surfaces also get
    // a pre-filtered sky environment for reflections.
    this.hemi.color.lerp(new THREE.Color().copy(top).lerp(low,.55),.4);
    const environment=this.skyEnvironment?.update(this.renderer,performance.now(),[top,low,this.skyCloud.value]);
    if(environment&&this.scene.environment!==environment)this.scene.environment=environment;
    // The environment adds to the hemisphere fill on standard surfaces; trade
    // some of that fill for sky-shaped ambient rather than stacking both.
    this.scene.environmentIntensity=.1+this.daylight*.16;
    if(this.style==='miniature')this.hemi.intensity*=.78;
    this.art.sunDirection.value.copy(this.sun.position).sub(this.sun.target.position).normalize();
    this.art.cloudShadow.value=.42*Math.max(0,this.daylight*2-1)*(1-dusk*.6)*(1-this.rain);
    if(this.frame%30===0){const target=this.director.target;
      const lit=this.worldState?.buildings.filter(b=>b.stage==='open'&&(b.anchor||b.kind==='shop'||b.kind==='cafe')).sort((a,b)=>Math.hypot(a.x-target.x,a.z-target.z)-Math.hypot(b.x-target.x,b.z-target.z)).slice(0,8)??[];
      for(let i=0;i<this.lanternLights.length;i++){const b=lit[i],lamp=this.lanternLights[i];lamp.userData.active=!!b;if(b)lamp.position.set(b.x+Math.sin(b.rotation)*(b.depth*.45+1.5),3.1,b.z+Math.cos(b.rotation)*(b.depth*.45+1.5));}
    }
    const activityHour=hour%24,lightBlend=Math.min(1,dt*.8);this.art.litHome.value=THREE.MathUtils.lerp(this.art.litHome.value,activityHour>=23||activityHour<5?.32:1,lightBlend);this.art.litOffice.value=THREE.MathUtils.lerp(this.art.litOffice.value,activityHour>=7&&activityHour<20?1:.1,lightBlend);this.art.litShop.value=THREE.MathUtils.lerp(this.art.litShop.value,activityHour>=6&&activityHour<23?1:.18,lightBlend);
    // Rooms switch on and off individually as each schedule's occupancy passes their threshold.
    for(const activity of ['home','office','shop'] as const){const share=this.art.occupancy[activity];share.value=THREE.MathUtils.lerp(share.value,windowOccupancy(activity,activityHour),lightBlend);}
    for(const lamp of this.lanternLights)lamp.intensity=lamp.userData.active?(1-this.daylight)*36*this.art.litShop.value:0;
    this.art.night.value=1-this.daylight;this.art.wet.value=THREE.MathUtils.lerp(this.art.wet.value,this.rain,Math.min(1,dt*(this.rain>this.art.wet.value?1:.08)));
    const apartmentArt=this.apartmentResources.art;
    apartmentArt.night.value=this.art.night.value;apartmentArt.wet.value=this.art.wet.value;apartmentArt.time.value=this.art.time.value;
    apartmentArt.sunDirection.value.copy(this.art.sunDirection.value);apartmentArt.cloudShadow.value=this.art.cloudShadow.value;
    for(const activity of ['home','office','shop'] as const)apartmentArt.occupancy[activity].value=this.art.occupancy[activity].value;
    apartmentArt.litHome.value=this.art.litHome.value;apartmentArt.litOffice.value=this.art.litOffice.value;apartmentArt.litShop.value=this.art.litShop.value;
    // Hysteresis: rebuild the pipeline only when streets clearly get or lose a sheen.
    const wetStreets=this.wetReflections?this.art.wet.value>.03:this.art.wet.value>.08;
    if(wetStreets!==this.wetReflections&&this.pipeline){this.wetReflections=wetStreets;this.buildPipeline();}
    // Golden-hour shafts: low sun, daylight, little rain.
    const golden=this.lightShafts?this.shaftStrength>.02:this.shaftStrength>.06;
    if(golden!==this.lightShafts&&this.pipeline){this.lightShafts=golden;this.buildPipeline();}
    this.nearbyStreetLights.update(dt,this.director.target,1-this.daylight);
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
    const hits=ray.intersectObjects([this.chunks.root,...this.actorIds.keys()],true);const hit=hits.find(candidate=>{for(let object:THREE.Object3D|null=candidate.object;object;object=object.parent)if(!object.visible)return false;return true;});if(!hit)return;const building=this.chunks.buildingAt(hit);if(building!==undefined)this.onPick?.(building,'building');else if(hit.instanceId!==undefined){const id=this.actorIds.get(hit.object)?.[hit.instanceId];if(id!==undefined)this.onPick?.(id,'traveler');}
  };
  dispose(){this.disposed=true;document.removeEventListener('visibilitychange',this.onVisibility);this.renderStartup?.dispose();this.renderer.setAnimationLoop(null);this.resize.disconnect();this.director.dispose();this.renderer.domElement.removeEventListener('pointerdown',this.pointerDown);this.renderer.domElement.removeEventListener('pointerup',this.pointerUp);this.resetWorld();this.streetLighting?.root.removeFromParent();this.retire(()=>this.streetLighting?.dispose());this.umbrellas.root.removeFromParent();this.retire(()=>this.umbrellas.dispose());this.buses.root.removeFromParent();this.retire(()=>this.buses.dispose());this.trains.root.removeFromParent();this.retire(()=>this.trains.dispose());this.fleet.root.removeFromParent();this.retire(()=>this.fleet.dispose());this.disposePipeline();this.scene.environment=null;const environment=this.skyEnvironment;this.skyEnvironment=null;this.retire(()=>environment?.dispose());const materials=new Set<THREE.Material>(),geometries=new Set<THREE.BufferGeometry>();this.scene.traverse(o=>{if(o instanceof THREE.Mesh||o instanceof THREE.Line){geometries.add(o.geometry);for(const m of Array.isArray(o.material)?o.material:[o.material])if(!m.userData.artKey&&m!==this.art.contours)materials.add(m);}});this.retire(()=>{for(const g of geometries)g.dispose();for(const m of materials)m.dispose();this.apartmentResources.dispose();this.art.dispose();this.renderer.dispose();});this.renderer.domElement.remove();}
}

function distanceToSegment(p:Point,a:Point,b:Point){const dx=b.x-a.x,dz=b.z-a.z;const t=Math.max(0,Math.min(1,((p.x-a.x)*dx+(p.z-a.z)*dz)/(dx*dx+dz*dz||1)));return Math.hypot(p.x-a.x-dx*t,p.z-a.z-dz*t);}
