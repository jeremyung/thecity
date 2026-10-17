import { miniatureOpticalScale } from '../optics/miniature-scale';
import {
  DEFAULT_TILT_SHIFT,
  normalizeTiltShift,
  writeTiltedFocusPlane,
  type TiltShiftSettings,
} from '../optics/tilt-shift';
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
import {
  color,
  uniform,
  mix,
  positionLocal,
  smoothstep,
  mx_noise_float,
  pass,
  add,
  vec3,
  attribute,
  positionView,
  float,
  floor,
  fract,
  step,
  vec2,
  vec4,
  mrt,
  output,
  normalView,
  directionToColor,
  colorToDirection,
  sample,
  getViewPosition,
  uv,
  fog,
  rangeFogFactor,
  positionWorld,
} from 'three/tsl';
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
import {
  prepareStreetLighting,
  PersistentStreetLighting,
  type StreetLightingData,
} from './street-lighting';
import { bezierRoad, sweepRoad, type RoadCurve } from './road-curve';
import { CURRENT_EXPRESSWAY_PROFILE } from './expressway-profile';
import { addExpresswayFurniture } from './expressway-kit';
import { windowLived, windowOccupancy } from './building-surfaces';
import { createFoliageGeometry, createTreeTrunkGeometry } from './foliage';
import { signageContexts, type SignageContext } from './signage';
import { seasonAt, seasonKey, treeColor, type SeasonState, type TreeSpecies } from './seasons';
import {
  addNeighborhoodShrine,
  addPocketPark,
  addScrambleCrossing,
  addShrineGrove,
  addStationPlaza,
  openSpaceKind,
  openSpaces,
  parkSignature,
  streetTreePlacements,
  type PlantedTree,
} from './parks';
import { createBackdropGeometry } from './backdrop';
import { createSkylineGeometry } from './skyline';
import { createPavedGroundTest } from './paved-ground';
import { buildGroundSurfaces } from './ground-surfaces';
import { CitySidewalks } from './sidewalks';
import { roadBatchSignatures } from './road-batch-signature';
import {
  ARTERIAL_SIDEWALK,
  bridgeDeckHeight,
  bridgeLift,
  bridgeVehiclePose,
  vehicleSurfacePose,
  EXPRESSWAY_DECK,
  RAIL_DECK,
  roadWidth,
  expresswayPierClear,
  type CrossingDeck,
  type BridgeSpan,
} from './road-elevation';
import { RenderQualityController, renderQualitySettings } from './quality';
import { FrameTiming, type FrameTimingSnapshot } from './frame-timing';
import { createRenderStartupLifecycle, type RenderStartupLifecycle } from './render-startup';
import type { CameraRig, CameraRigFactory } from './camera-rig';
import { RoadsLayer } from './roads-layer';
import { LandscapeLayer } from './landscape-layer';
import { ActorsLayer } from './actors-layer';
import { dampFocusDepth, focusDepthFromViewZ } from '../optics/focus';
import type { LensPresetId } from '../optics/lens';
import { LENS_PRESETS, verticalFovDegrees } from '../optics/lens';
import type { World, Style, Point, Road, Building, BuildingOriginalFootprint } from '../types';

export interface StyleSwitchMetrics {
  target: Style;
  samples: number;
  compiledSamples: number;
  maxSubmitMs: number;
  prepareMs: number;
  submitMs: number;
  waitMs: number;
  applyMs: number;
  outcome: 'compiling' | 'applied' | 'superseded' | 'failed';
}
export type PresentedActor = { x: number; z: number; h: number; k: number; y: number };

export interface PresentationView {
  world: World | null;
  listener: THREE.Vector3;
  camera: THREE.PerspectiveCamera;
  current: ReadonlyMap<number, PresentedActor>;
  previous: ReadonlyMap<number, PresentedActor>;
  motionDelta: number;
  motionTick: number;
  riverHalfWidth: number;
  hour: number;
  rain: number;
}

export interface RenderMetrics {
  timing?: FrameTimingSnapshot;
  styleSwitch?: StyleSwitchMetrics;
  fps: number;
  frameMs: number;
  calls: number;
  triangles: number;
  textures: number;
  geometries: number;
  backend: string;
  pendingBuildings: number;
  pendingChunks: number;
  pendingSidewalks: number;
  pendingRoads: number;
}
/** Fields that affect a building's rendered group. Occupancy is binary: every
 * window pane bakes whether anyone lives or works behind it (`windowLived`), so
 * crossing the vacant boundary rebuilds, while other occupancy changes do not. */
export function buildingRenderSignature(
  b: Building,
  original?: BuildingOriginalFootprint,
  signage?: SignageContext,
): string {
  const homeOccupancy = windowLived(b) ? 'occupied' : 'vacant';
  const retained = original
    ? `${original.x}:${original.z}:${original.width}:${original.depth}:${original.rotation}`
    : '-';
  const signs =
    signage && (signage.proximity || signage.feature)
      ? `:s${signage.proximity}${signage.feature ?? ''}`
      : '';
  const stagedTower =
    b.floors >= 12 &&
    (b.kind === 'office' || b.kind === 'apartment') &&
    b.stage !== 'open' &&
    b.stage !== 'renovation' &&
    b.stage !== 'demolition';
  const towerPhase = `${Math.ceil(b.floors * Math.max(0, Math.min(1, (b.progress - 0.12) / 0.57)))}:${Math.ceil(b.floors * Math.max(0, Math.min(1, (b.progress - 0.69) / 0.23)))}`;
  const phase = stagedTower
    ? towerPhase
    : b.stage === 'demolition'
      ? `${Math.ceil(b.floors * (1 - Math.max(0, Math.min(1, b.progress))))}:${b.progress > 0}`
      : Math.floor(b.progress * 5);
  return `${b.stage}:${b.floors}:${phase}:${b.kind}:${b.name}:${b.appearance_seed}:${b.business_active ?? 'legacy'}:${homeOccupancy}:${b.x}:${b.z}:${b.width}:${b.depth}:${b.rotation}:${retained}${signs}`;
}
export type TreeBatch = {
  indices: number[];
  trees: THREE.InstancedMesh;
  trunks: THREE.InstancedMesh;
  center: THREE.Vector3;
  far: boolean;
};
export type TreeLook = { species: TreeSpecies; color: string; variation: number };
export const v = new THREE.Vector3(),
  dummy = new THREE.Object3D();
// Range haze (m from camera): clear through the city, veiling the backdrop.
// Miniature keeps crisper air, matching its tilt-shift photographic read.
const HAZE_NEAR: Record<Style, number> = { anime: 380, miniature: 460 },
  HAZE_FAR: Record<Style, number> = { anime: 3200, miniature: 3700 };
export const personColors = ['#e8d9b6', '#bc7361', '#89a589', '#64839d', '#e3b35f', '#a897b0'].map(
  (c) => new THREE.Color(c),
);
export function random(seed: number) {
  return () => {
    seed ^= seed << 13;
    seed ^= seed >>> 17;
    seed ^= seed << 5;
    return (seed >>> 0) / 4294967296;
  };
}

export class CityRenderer {
  readonly landscape = new LandscapeLayer(this);
  readonly roadsLayer = new RoadsLayer(this);
  // Explicit development study; never selected by production or a saved world.
  private readonly compactMidriseReview =
    import.meta.env.DEV && new URLSearchParams(window.location.search).get('midrise') === 'compact';
  readonly scene = new THREE.Scene();
  readonly camera = new THREE.PerspectiveCamera(
    verticalFovDegrees(LENS_PRESETS.environment.focalLengthMm),
    1,
    0.5,
    3500,
  );
  readonly art = new ArtMaterials();
  readonly actors = new ActorsLayer(this);
  readonly rig: CameraRig;
  readonly renderer: THREE.WebGPURenderer;
  worldState: World | null = null;
  onPick: ((id: number, kind: 'building' | 'traveler') => void) | null = null;
  onMetrics: ((m: RenderMetrics) => void) | null = null;
  onError: ((error: Error) => void) | null = null;
  style: Style = 'anime';
  private styleRequest = 0;
  private styleWarmupTail: Promise<void> = Promise.resolve();
  private pendingCompiles = 0;
  private styleSwitchMetrics: StyleSwitchMetrics | undefined;
  private retiredResources: Array<() => void> = [];
  private pipeline: THREE.RenderPipeline | null = null;
  private focusedPipeline: THREE.RenderPipeline | null = null;
  private unblurredPipeline: THREE.RenderPipeline | null = null;
  private occlusion: ReturnType<typeof ao> | null = null;
  private occlusionStrength = uniform(0.22);
  private hazeColor = uniform(new THREE.Color('#d2d6cc'));
  private hazeNear = uniform(HAZE_NEAR.anime);
  private hazeFar = uniform(HAZE_FAR.anime);
  private mist = uniform(0);
  private reflections: ReturnType<typeof ssr> | null = null;
  private wetReflections = false;
  private shafts: ReturnType<typeof godrays> | null = null;
  private lightShafts = false;
  private shaftColor = uniform(new THREE.Color(0, 0, 0));
  private shaftStrength = 0;
  private skyEnvironment: SkyEnvironment | null = null;
  private renderStartup: RenderStartupLifecycle | null = null;
  private scenePass: ReturnType<typeof pass> | null = null;
  private focus = uniform(300);
  private readonly focusTargetView = new THREE.Vector3();
  private focusResetPending = true;
  private opticalScale = uniform(0.04);
  private lensFocalM = uniform(0.035);
  private lensFNumber = uniform(8);
  private outputHeight = uniform(1);
  private dofEnabled = uniform(1);
  private bokeh = uniform(4);
  private tiltShift: TiltShiftSettings = { ...DEFAULT_TILT_SHIFT };
  private tiltEnabled = uniform(0);
  private tiltWidth = uniform(0.5);
  private tiltOpticalScale = uniform(0.01);
  private tiltPlane = uniform(new THREE.Vector4(0, 0, 1, 300));
  private shine: ReturnType<typeof bloom> | null = null;
  private depthOfField: ReturnType<typeof signedCoCDof> | null = null;
  skyHigh = uniform(new THREE.Color('#9bbac8'));
  skyLow = uniform(new THREE.Color('#f1dcb9'));
  private skyCloud = uniform(new THREE.Color('#fff5df'));
  private skyCloudShade = uniform(new THREE.Color('#809aaf'));
  hillColor = uniform(new THREE.Color('#637f85'));
  skylineColor = uniform(new THREE.Color('#8496a3'));
  skylineNight = uniform(0);
  private sunDisc: THREE.Mesh;
  private moonDisc: THREE.Mesh;
  private sun = new THREE.DirectionalLight('#fff1d1', 3);
  private hemi = new THREE.HemisphereLight('#c8dce3', '#748998', 2);
  private lanternLights: Array<THREE.PointLight> = [];

  private sidewalks = new CitySidewalks(
    () => this.art.get('#b4b8a6'),
    (dispose) => this.retire(dispose),
  );
  private chunks = new ConstructionBatches(
    (m) => this.art.buildingBatch(m),
    (dispose) => this.retire(dispose),
    this.art.contours,
  );
  private buildings = new Map<number, { source: RebuildableBuildingSource; signature: string }>();
  water: RiverWater | null = null;
  riverHalfWidth = 32;
  private parkKey = '';
  /** Latest per-building signage context; part of each building's render signature. */
  private signage = new Map<number, SignageContext>();
  private accessPaths = new BuildingAccessPaths(this.art, (release) => this.retire(release));
  private rearExtensions = new RearExtensions(
    this.art,
    (release) => this.retire(release),
    (building) => {
      const current = this.buildings.get(building.id);
      const original = this.worldState?.building_original_footprints?.find(
        (form) => form.building_id === building.id,
      );
      return (
        !!current &&
        current.signature ===
          buildingRenderSignature(building, original, this.signage.get(building.id)) &&
        this.chunks.isSourceVisible(building.id, current.source)
      );
    },
  );
  private umbrellas = new Umbrellas(2000, (hex) => this.art.get(hex));
  private lastTick = 0;
  hour = 6;
  private preview: number | null = null;
  private frame = 0;
  private disposed = false;
  private lastFrame = 0;
  private frameTiming = new FrameTiming();
  private lastMetricsAt = 0;
  private onVisibility = () => {
    this.frameTiming.record(performance.now(), 0, 'loading', false);
    this.lastFrame = 0;
  };
  private groundSignature = '';
  private ground = new THREE.Group();
  private terrainSeed = -1;
  private resize: ResizeObserver;
  private pointerStart = { x: 0, y: 0 };
  private selected: number | null = null;
  private highlight: THREE.Mesh;
  private daylight = 1;
  private rain = 0;
  private focusEnabled = true;
  private quality: 'high' | 'balanced' = 'high';
  private qualityController = new RenderQualityController();
  private fixedReviewQuality = false;
  private worldTick = -1;
  private waterTime = 0;
  private rainTime = uniform(0);
  private rainOpacity = uniform(0);
  private rainLines: THREE.LineSegments;
  private objectQueue = new Map<
    number,
    { building: Building; signature: string; source: RebuildableBuildingSource }
  >();
  constructor(
    readonly element: HTMLElement,
    createRig: CameraRigFactory,
    forceWebGL = false,
  ) {
    document.addEventListener('visibilitychange', this.onVisibility);
    this.renderer = new THREE.WebGPURenderer({
      antialias: true,
      samples: 4,
      forceWebGL,
      powerPreference: 'high-performance',
    });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.6));
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.05;
    element.appendChild(this.renderer.domElement);
    this.rig = createRig(this.camera, this.renderer.domElement);
    // Range haze (clear city, veiled backdrop) plus low-lying mist: it builds
    // with distance but only near the ground and water, strongest at first
    // light and in rain, so towers rise out of it as in the dawn concept.
    const mist = positionView.z
      .negate()
      .mul(-0.0032)
      .exp()
      .oneMinus()
      .mul(positionWorld.y.add(1.5).div(-8).exp())
      .mul(this.mist);
    this.scene.fogNode = fog(this.hazeColor, rangeFogFactor(this.hazeNear, this.hazeFar).max(mist));
    this.scene.add(
      this.landscape.terrain,
      this.roadsLayer.roads,
      this.landscape.parks,
      this.roadsLayer.constructionRoads,
      this.sidewalks.root,
      this.ground,
      this.roadsLayer.station,
      this.chunks.root,
      this.umbrellas.root,
      this.actors.buses.root,
      this.actors.trains.root,
      this.actors.fleet.root,
      this.accessPaths.root,
      this.rearExtensions.root,
      this.hemi,
      this.sun,
    );
    this.sun.castShadow = true;
    this.sun.shadow.mapSize.set(2048, 2048);
    this.sun.shadow.camera.left = -230;
    this.sun.shadow.camera.right = 230;
    this.sun.shadow.camera.top = 230;
    this.sun.shadow.camera.bottom = -230;
    this.sun.shadow.camera.near = 1;
    this.sun.shadow.camera.far = 1100;
    this.sun.shadow.bias = -0.0002;
    this.sun.shadow.normalBias = 0.35;
    this.scene.add(this.sun.target);
    for (let i = 0; i < 8; i++) {
      const light = new THREE.PointLight('#ffd6a0', 0, 22, 2);
      this.lanternLights.push(light);
      this.scene.add(light);
    }
    this.scene.add(this.roadsLayer.nearbyStreetLights.root);
    const sky = new THREE.Mesh(
      new THREE.SphereGeometry(1800, 32, 20),
      new THREE.MeshBasicNodeMaterial({ side: THREE.BackSide, depthWrite: false }),
    );
    const direction = positionLocal.normalize();
    const skyBase = mix(this.skyLow, this.skyHigh, smoothstep(-0.05, 0.75, direction.y));
    // Stable, layered cloud masses: broad silhouettes, broken edges, and cool
    // undersides read as an environmental painting rather than a flat sky wash.
    const cloudPoint = direction
      .mul(vec3(8, 23, 8))
      .add(vec3(this.art.time.mul(0.0015), 0, this.art.time.mul(0.0005)));
    const cloudField = mx_noise_float(cloudPoint)
      .mul(0.64)
      .add(mx_noise_float(cloudPoint.mul(2.1)).mul(0.25))
      .add(mx_noise_float(cloudPoint.mul(4.3)).mul(0.11));
    const clouds = smoothstep(-0.035, 0.16, cloudField).mul(smoothstep(0.015, 0.09, direction.y));
    const cloudRelief = smoothstep(-0.04, 0.2, cloudField.add(direction.y.mul(0.13)));
    const cloudPaint = mix(this.skyCloudShade, this.skyCloud, cloudRelief);
    (sky.material as THREE.MeshBasicNodeMaterial).colorNode = mix(skyBase, cloudPaint, clouds);
    (sky.material as THREE.MeshBasicNodeMaterial).fog = false;
    sky.renderOrder = -10;
    this.scene.add(sky);
    this.skyEnvironment = new SkyEnvironment(sky);
    this.sunDisc = new THREE.Mesh(
      new THREE.SphereGeometry(15, 16, 8),
      new THREE.MeshBasicNodeMaterial({ color: '#ffe5bc', fog: false }),
    );
    this.scene.add(this.sunDisc);
    this.moonDisc = new THREE.Mesh(
      new THREE.SphereGeometry(7, 16, 8),
      new THREE.MeshBasicNodeMaterial({ color: '#cbdbe2', fog: false }),
    );
    this.scene.add(this.moonDisc);
    const rainPositions: number[] = [],
      rainPhases: number[] = [];
    const rainRandom = random(123);
    for (let i = 0; i < 700; i++) {
      const x = rainRandom() * 180 - 90,
        z = rainRandom() * 180 - 90,
        phase = rainRandom() * 90;
      rainPositions.push(x, 0, z, x + 0.06, 1.1, z);
      rainPhases.push(phase, phase);
    }
    const rainGeo = new THREE.BufferGeometry();
    rainGeo.setAttribute('position', new THREE.Float32BufferAttribute(rainPositions, 3));
    rainGeo.setAttribute('rainPhase', new THREE.Float32BufferAttribute(rainPhases, 1));
    const rainMat = new THREE.LineBasicNodeMaterial({
      color: '#b8d0d4',
      transparent: true,
      depthWrite: false,
    });
    // Both endpoints wrap together. Wrapping each endpoint separately made a
    // short drop stretch through the entire 90 m volume at its wrap boundary.
    const rainHeight = float(attribute('rainPhase', 'float')).sub(this.rainTime.mul(16)).mod(90);
    rainMat.positionNode = vec3(positionLocal.x, positionLocal.y.add(rainHeight), positionLocal.z);
    rainMat.opacityNode = this.rainOpacity.mul(smoothstep(8, 28, positionView.z.negate()));
    this.rainLines = new THREE.LineSegments(rainGeo, rainMat);
    this.rainLines.visible = false;
    // Shader displacement moves drops beyond the small undeformed geometry.
    rainGeo.boundingSphere = new THREE.Sphere(new THREE.Vector3(0, 45, 0), 140);
    this.scene.add(this.rainLines);
    this.actors.pedestrians = this.instances(
      new THREE.CapsuleGeometry(0.33, 0.9, 3, 5),
      this.art.get('#e7e5d9'),
      2000,
    );
    this.actors.heads = this.instances(
      new THREE.SphereGeometry(0.28, 6, 4),
      this.art.get('#dfbe97'),
      2000,
    );
    this.highlight = new THREE.Mesh(
      new THREE.RingGeometry(8, 8.2, 64),
      new THREE.MeshBasicNodeMaterial({ color: '#f2d79b', side: THREE.DoubleSide }),
    );
    this.highlight.rotation.x = -Math.PI / 2;
    this.highlight.visible = false;
    this.scene.add(this.highlight);
    this.resize = new ResizeObserver(() => this.setSize());
    this.resize.observe(element);
    this.renderer.domElement.addEventListener('pointerdown', this.pointerDown);
    this.renderer.domElement.addEventListener('pointerup', this.pointerUp);
  }
  private instances(geo: THREE.BufferGeometry, mat: THREE.Material, count: number) {
    const mesh = new THREE.InstancedMesh(geo, mat, count);
    mesh.count = 0;
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    mesh.frustumCulled = false;
    this.scene.add(mesh);
    return mesh;
  }
  async start(initialStyle: Style = 'anime') {
    this.applyStyle(initialStyle);
    this.renderer.domElement.dataset.graphicsPhase = 'device-initialization';
    try {
      await this.renderer.init();
    } catch (error) {
      this.renderer.domElement.dataset.graphicsPhase = 'device-error';
      throw error;
    }
    this.renderer.domElement.dataset.graphicsPhase = 'pipeline-setup';
    // A slow GPU initialization can outlive unmount or hot reload. Never
    // restart an animation loop or allocate a pipeline for a retired scene.
    if (this.disposed) {
      this.renderer.dispose();
      return;
    }
    this.setSize();
    this.buildPipeline();
    const startup = (this.renderStartup = createRenderStartupLifecycle({
      stopLoop: () => this.renderer.setAnimationLoop(null),
      setPhase: (phase) => {
        this.renderer.domElement.dataset.graphicsPhase = phase;
      },
      onError: (error) => this.onError?.(error),
    }));
    this.renderer.setAnimationLoop(() => startup.runFrame(() => this.animate()));
    this.renderer.domElement.dataset.graphicsPhase = 'loop-started';
    await startup.promise;
  }
  private readonly sceneFrame = new THREE.NodeFrame();
  private buildPipeline() {
    this.disposePipeline();
    this.scenePass = pass(this.scene, this.camera, { samples: 4 });
    this.scenePass.updateBeforeType = THREE.NodeUpdateType.NONE;
    // Explicit resolved normals avoid Three's depth-normal reconstruction path,
    // which emits an invalid textureDimensions mip argument for MSAA WebGPU depth.
    // Keep four-sample scene AA; AO and wet-weather SSR share the normal target.
    const wet = this.wetReflections;
    this.scenePass.setMRT(mrt({ output, normal: directionToColor(normalView) }));
    const depthTexture = this.scenePass.getTextureNode('depth');
    const normalTexture = this.scenePass.getTextureNode('normal');
    const viewNormal = sample((uv) => colorToDirection(normalTexture.sample(uv)));
    const occlusion = (this.occlusion = ao(depthTexture, viewNormal, this.camera));
    occlusion.resolutionScale = 0.5;
    occlusion.radius.value = 3;
    occlusion.thickness.value = 2.5;
    occlusion.distanceExponent.value = 1.4;
    occlusion.samples.value = 12;
    // Fade AO beyond ~700 m: distant depth loses precision and can band
    // across the hills, skyline and mountain ring.
    const aoReach = smoothstep(1300, 700, this.scenePass.getViewZNode().negate());
    let sceneColor = this.scenePass
      .getTextureNode('output')
      .mul(
        vec4(
          vec3(mix(float(1), occlusion.getTextureNode().r, this.occlusionStrength.mul(aoReach))),
          1,
        ),
      );
    if (wet) {
      // Reflectivity is derived in the SSR pass itself: flat ground near street
      // level, in the same world-space puddles the asphalt darkens. Camera
      // matrices are explicit uniforms because the pass renders a screen quad.
      const viewPosition = getViewPosition(
        uv(),
        depthTexture.r,
        uniform(this.camera.projectionMatrixInverse),
      );
      const cameraWorld = uniform(this.camera.matrixWorld);
      const world = cameraWorld.mul(vec4(viewPosition, 1)).xyz;
      const up = cameraWorld.mul(vec4(colorToDirection(normalTexture.rgb), 0)).xyz.normalize().y;
      const reflectivity = smoothstep(0.86, 0.97, up)
        .mul(smoothstep(1.6, 0.5, world.y))
        .mul(ArtMaterials.puddles(world.x, world.z).mul(0.55).add(0.1))
        .mul(this.art.wet);
      // SSR samples its colour input, so it reflects the raw pass texture.
      const reflections = (this.reflections = ssr(
        this.scenePass.getTextureNode('output'),
        depthTexture,
        viewNormal,
        reflectivity,
        float(0.32),
        this.camera,
      ));
      reflections.resolutionScale = 0.5;
      reflections.maxDistance.value = 60;
      reflections.thickness.value = 0.6;
      reflections.quality.value = 0.45;
      // SSR writes reflected colour already scaled by reflectivity with alpha 1,
      // so add it; alpha blending would replace the street with a dark mirror.
      sceneColor = sceneColor.add(vec4(reflections.rgb, 0));
    }
    if (this.lightShafts) {
      // Raymarched shafts through the key light's shadow map, only while the
      // sun is low enough for them to read (rebuilt when that changes).
      const shafts = (this.shafts = godrays(
        this.scenePass.getTextureNode('depth'),
        this.camera,
        this.sun,
      ));
      shafts.resolutionScale = 0.5; // Aerial views see sunlit air almost everywhere: keep density low so
      // only the contrast behind occluders reads, not a uniform glow.
      shafts.raymarchSteps.value = 40;
      shafts.density.value = 0.09;
      shafts.maxDensity.value = 0.22;
      shafts.distanceAttenuation.value = 2;
      sceneColor = sceneColor.add(vec4(shafts.getTextureNode().rgb.mul(this.shaftColor), 0));
    }
    const depth = this.scenePass.getViewZNode();
    // Intersect each visible camera ray with the tilted plane. Unlike a
    // screen-space stripe, roof height and scene depth affect the focus.
    const focusView = getViewPosition(
      uv(),
      depthTexture.r,
      uniform(this.camera.projectionMatrixInverse),
    );
    const focusRay = vec3(focusView.xy.div(depth.negate().max(0.0001)), 1);
    const planeDenominator = focusRay.dot(this.tiltPlane.xyz);
    const planeValid = planeDenominator.abs().greaterThan(0.00001);
    const planeDepth = this.tiltPlane.w.div(planeValid.select(planeDenominator, float(1)));
    const tiltedDepth = planeValid
      .and(planeDepth.greaterThan(0.1))
      .select(planeDepth, float(1000000));
    const signedCoC = physicalCoCNode({
      viewZ: depth,
      focusDistanceM: mix(this.focus, tiltedDepth, this.tiltEnabled),
      focalLengthM: this.lensFocalM,
      fNumber: this.lensFNumber.mul(mix(float(1), this.tiltWidth, this.tiltEnabled)),
      opticalScale: mix(this.opticalScale, this.tiltOpticalScale, this.tiltEnabled),
      outputHeightPx: this.outputHeight,
      maxGatherRadiusPx: this.bokeh,
      enabled: this.dofEnabled,
    });
    // Keep two output graphs sharing the scene/AO/bloom resources. Switching
    // focus must not rebuild the scene pass and invalidate its compiled work.
    // The unblurred graph omits DOF entirely, so its seven passes do not run.
    const focus = (this.depthOfField = signedCoCDof(sceneColor, signedCoC, this.bokeh));
    const shine = bloom(sceneColor, 0.1, 0.5, 0.9);
    this.shine = shine;
    this.focusedPipeline = new THREE.RenderPipeline(this.renderer);
    this.focusedPipeline.outputNode = add(
      focus as unknown as THREE.Node<'vec4'>,
      shine as unknown as THREE.Node<'vec4'>,
    );
    this.unblurredPipeline = new THREE.RenderPipeline(this.renderer);
    this.unblurredPipeline.outputNode = add(sceneColor, shine as unknown as THREE.Node<'vec4'>);
    this.pipeline = this.focusEnabled ? this.focusedPipeline : this.unblurredPipeline;
  }

  private disposePipeline() {
    // Warmup borrows scene attachments. Detach immediately, but keep the old
    // resources alive until pending compiler work releases its references.
    const resources = new Set([
      this.occlusion,
      this.reflections,
      this.shafts,
      this.pipeline,
      this.focusedPipeline,
      this.unblurredPipeline,
      this.depthOfField,
      this.shine,
      this.scenePass,
    ]);
    this.occlusion = null;
    this.reflections = null;
    this.shafts = null;
    this.pipeline = null;
    this.focusedPipeline = null;
    this.unblurredPipeline = null;
    this.depthOfField = null;
    this.shine = null;
    this.scenePass = null;
    this.retire(() => {
      for (const resource of resources) resource?.dispose();
    });
  }
  async setStyle(style: Style) {
    const request = ++this.styleRequest;
    const previous = this.styleWarmupTail;
    let release!: () => void;
    this.styleWarmupTail = new Promise<void>((resolve) => {
      release = resolve;
    });
    await previous;
    try {
      if (this.disposed || request !== this.styleRequest || style === this.style) return;
      // Serialize warmup and skip superseded requests before allocating samples.
      const prepareStart = performance.now();
      const prepared = styleWarmup(this.scene, (m) => this.art.variant(m, style));
      const timing: StyleSwitchMetrics = {
        target: style,
        samples: prepared.scene.children.filter((o) => o instanceof THREE.Mesh).length,
        prepareMs: performance.now() - prepareStart,
        compiledSamples: 0,
        maxSubmitMs: 0,
        submitMs: 0,
        waitMs: 0,
        applyMs: 0,
        outcome: 'compiling',
      };
      this.styleSwitchMetrics = timing;
      this.pendingCompiles++;
      try {
        await compileStyleSamples(
          prepared.scene,
          (scene) =>
            this.scenePass
              ? compileForScenePass(
                  this.renderer,
                  this.scenePass,
                  scene,
                  this.camera,
                  prepared.targetScene,
                )
              : this.renderer.compileAsync(scene, this.camera, prepared.targetScene),
          () => !this.disposed && request === this.styleRequest,
          timing,
        );
      } catch (error) {
        timing.outcome = 'failed';
        throw error;
      } finally {
        prepared.dispose();
        this.pendingCompiles--;
        if (!this.pendingCompiles) {
          const retired = this.retiredResources.splice(0);
          for (const dispose of retired) dispose();
        }
      }
      if (this.disposed || request !== this.styleRequest) {
        timing.outcome = 'superseded';
        return;
      }
      const applyStart = performance.now();
      this.applyStyle(style);
      timing.applyMs = performance.now() - applyStart;
      timing.outcome = 'applied';
    } finally {
      release();
    }
  }
  private applyStyle(style: Style) {
    this.style = style;
    this.art.setStyle(style);
    this.chunks.setContoursVisible(style === 'anime');
    this.scene.traverse((o) => {
      if (o instanceof THREE.Mesh)
        o.material = Array.isArray(o.material)
          ? o.material.map((m) => this.art.variant(m))
          : this.art.variant(o.material);
    });
    this.opticalScale.value = style === 'miniature' ? 0.01 : 0.04;
    this.bokeh.value = this.tiltShift.enabled
      ? this.tiltShift.blur
      : style === 'miniature'
        ? 16
        : 4;
  }
  retire(dispose: () => void) {
    if (this.pendingCompiles) this.retiredResources.push(dispose);
    else dispose();
  }
  retireGroup(group: THREE.Group) {
    group.removeFromParent();
    this.retire(() => disposeGroup(group));
  }
  setPreview(hour: number | null) {
    this.preview = hour;
  }
  toggleWaterReflectionForReview() {
    return this.water?.toggleReflectionForReview() ?? false;
  }
  toggleStreetLightsForReview() {
    return (this.roadsLayer.nearbyStreetLights.enabled =
      !this.roadsLayer.nearbyStreetLights.enabled);
  }
  setLens(id: LensPresetId, takeManualControl = true) {
    this.rig.setLens(id, takeManualControl);
    const lens = LENS_PRESETS[id];
    this.lensFocalM.value = lens.focalLengthMm / 1000;
    this.lensFNumber.value = lens.fNumber;
  }
  setTiltShift(settings: Partial<TiltShiftSettings>) {
    this.tiltShift = normalizeTiltShift({ ...this.tiltShift, ...settings });
    this.tiltEnabled.value = this.tiltShift.enabled ? 1 : 0;
    this.tiltWidth.value = this.tiltShift.width;
    this.bokeh.value = this.tiltShift.enabled
      ? this.tiltShift.blur
      : this.style === 'miniature'
        ? 16
        : 4;
  }
  setFocus(value: boolean) {
    if (this.focusEnabled === value) return;
    this.focusEnabled = value;
    this.dofEnabled.value = value ? 1 : 0;
    if (this.pipeline) this.pipeline = value ? this.focusedPipeline : this.unblurredPipeline;
  }
  setQuality(quality: 'high' | 'balanced') {
    this.quality = quality;
    this.qualityController.setProfile(quality);
    this.applyQuality();
  }
  setFixedReviewQuality(enabled: boolean) {
    if (!import.meta.env.DEV) return;
    this.fixedReviewQuality = enabled;
    this.qualityController.reset();
    this.applyQuality();
  }
  reviewQualitySnapshot() {
    const buffer = this.renderer.getDrawingBufferSize(new THREE.Vector2());
    return {
      mode: this.fixedReviewQuality ? 'fixed-review' : 'adaptive',
      profile: this.quality,
      adaptiveLevel: this.qualityController.level,
      ...renderQualitySettings(
        this.quality,
        this.qualityController.level,
        window.devicePixelRatio,
        this.fixedReviewQuality,
      ),
      actualBuffer: buffer.toArray(),
    };
  }
  applyQuality() {
    // Reduce reflection/shadow cost before lowering the focal scene's resolution.
    const settings = renderQualitySettings(
      this.quality,
      this.qualityController.level,
      window.devicePixelRatio,
      this.fixedReviewQuality,
    );
    this.renderer.setPixelRatio(settings.pixelRatio);
    this.sun.shadow.mapSize.set(settings.shadowSize, settings.shadowSize);
    this.water?.setReflectionScale(settings.reflectionScale);
    this.setSize();
  }
  setSelected(id: number | null) {
    this.selected = id;
  }
  private setSize() {
    const w = this.element.clientWidth,
      h = this.element.clientHeight;
    if (!w || !h) return;
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(w, h);
    this.outputHeight.value = this.renderer.getDrawingBufferSize(new THREE.Vector2()).y;
  }
  setWorld(world: World, replaced = false) {
    this.worldState = world;
    this.landscape.season = seasonAt(world.day, world.minute_of_day);
    this.hour = world.minute_of_day / 60;
    this.rain = world.weather?.intensity ?? 0;
    if (replaced || this.terrainSeed !== world.seed || world.tick < this.worldTick) {
      this.resetWorld();
      this.landscape.buildTerrain(world);
      this.terrainSeed = world.seed;
    }
    this.roadsLayer.infrastructure().request(world);
    this.worldTick = world.tick;
    this.rig.world(world);
    this.actors.buses.setWorld(world);
    this.actors.trains.setWorld(
      trainWorldWithInfrastructure(world, this.roadsLayer.committedRoadWorld),
    );
    this.accessPaths.setWorld(world);
    this.rearExtensions.setWorld(world);

    const signatures = roadBatchSignatures(world);
    const signature = signatures.completed + signatures.construction;
    if (signature !== this.roadsLayer.roadSignature) {
      this.roadsLayer.rebuildRoads(world, signatures.completed);
      this.roadsLayer.roadSignature = signature;
    }
    this.sidewalks.setWorld(world);
    const groundSignature = JSON.stringify(world.ground ?? []);
    if (groundSignature !== this.groundSignature) {
      this.groundSignature = groundSignature;
      this.retireGroup(this.ground);
      this.ground = buildGroundSurfaces(world.ground ?? [], this.art);
      this.scene.add(this.ground);
    }
    const parkKey = `${signature}#${parkSignature(world)}`;
    if (parkKey !== this.parkKey) {
      this.parkKey = parkKey;
      this.landscape.rebuildParks(world);
    }
    if (seasonKey(this.landscape.season) !== this.landscape.paintedSeason)
      this.landscape.paintSeason();
    const ids = new Set(world.buildings.map((b) => b.id));
    for (const id of this.buildings.keys())
      if (!ids.has(id)) {
        this.chunks.remove(id);
        this.buildings.delete(id);
      }
    for (const id of this.objectQueue.keys()) if (!ids.has(id)) this.objectQueue.delete(id);
    const originals = new Map(
      (world.building_original_footprints ?? []).map((original) => [
        original.building_id,
        original,
      ]),
    );
    const signage = (this.signage = signageContexts(world));
    for (const b of world.buildings) {
      const original = originals.get(b.id);
      const context = signage.get(b.id);
      const signature = buildingRenderSignature(b, original, context);
      if (this.buildings.get(b.id)?.signature === signature) {
        this.objectQueue.delete(b.id);
        continue;
      }
      // One pending update per authoritative building. Repeated snapshots replace
      // its data without appending duplicate closures or starving older parcels.
      const snapshot: Building = { ...b, history: [...b.history] };
      const originalSnapshot = original ? { ...original } : undefined;
      const source: RebuildableBuildingSource = {
        createGroup: () => {
          const group = buildExtendedBuilding(snapshot, originalSnapshot, this.art, {
            compactMidrise: this.compactMidriseReview,
            signage: context,
          });
          group.position.set(snapshot.x, 0, snapshot.z);
          group.rotation.y = snapshot.rotation;
          return group;
        },
        disposeGroup: (group) => disposeGroup(group),
      };
      this.objectQueue.set(b.id, { building: snapshot, signature, source });
    }
    this.landscape.updateTrees(world);
  }

  private resetWorld() {
    this.frameTiming = new FrameTiming();
    this.lastMetricsAt = 0;
    this.retireGroup(this.ground);
    this.ground = new THREE.Group();
    this.scene.add(this.ground);
    this.groundSignature = '';
    this.focusResetPending = true;
    this.umbrellas.reset();
    this.actors.buses.reset();
    this.actors.fleet.reset();
    this.actors.trains.reset();
    this.accessPaths.reset();
    this.rearExtensions.reset();
    this.actors.umbrellaCount = 0;
    this.qualityController.reset();
    this.chunks.clear();
    this.sidewalks.reset();
    this.roadsLayer.cancelRoadBuild();
    this.roadsLayer.infrastructure().reset();
    this.roadsLayer.nearbyStreetLights?.reset();
    this.roadsLayer.streetLighting?.reset();
    this.buildings.clear();
    this.landscape.clearParks();
    this.parkKey = '';
    this.landscape.paintedSeason = '';
    this.retireGroup(this.landscape.terrain);
    const near = this.landscape.treeNearGeometry,
      far = this.landscape.treeFarGeometry,
      trunk = this.landscape.treeTrunkGeometry;
    this.retire(() => {
      near?.dispose();
      far?.dispose();
      trunk?.dispose();
    });
    this.landscape.treeNearGeometry = null;
    this.landscape.treeFarGeometry = null;
    this.landscape.treeTrunkGeometry = null;
    this.landscape.treeBatches = [];
    this.landscape.terrain = new THREE.Group();
    this.scene.add(this.landscape.terrain);
    this.retireGroup(this.roadsLayer.roads);
    this.roadsLayer.roads = new THREE.Group();
    this.scene.add(this.roadsLayer.roads);
    this.retireGroup(this.roadsLayer.constructionRoads);
    this.roadsLayer.constructionRoads = new THREE.Group();
    this.scene.add(this.roadsLayer.constructionRoads);
    this.roadsLayer.completedRoadSignature = '';
    this.roadsLayer.completedRoadSurfaces = null;
    const oldWater = this.water;
    oldWater?.mesh.removeFromParent();
    this.retire(() => oldWater?.dispose());
    this.water = null;
    this.waterTime = 0;
    this.actors.motionTick = 0;
    this.actors.motionDelta = 0.05;
    this.actors.received = performance.now();
    this.actors.current.clear();
    this.actors.previous.clear();
    this.actors.movementPresentation?.reset();
    this.actors.actorSampleAt = 0;
    this.roadsLayer.committedRoadWorld = null;
    this.objectQueue.clear();
    this.roadsLayer.roadSignature = '';
    this.selected = null;
    this.landscape.treePoints = [];
  }

  private updateBuildingBatches() {
    let builtCount = 0;
    for (const [id, item] of this.objectQueue) {
      if (!this.roadsLayer.infrastructure().canPresentBuilding(id)) continue;
      if (builtCount++ >= 2) break;
      this.objectQueue.delete(id);
      const group = item.source.createGroup();
      this.chunks.set(id, group, item.source, item.building.stage !== 'open');
      this.buildings.set(id, { source: item.source, signature: item.signature });
    }
    const pendingChunks = new Set<string>();
    for (const [id, { building }] of this.objectQueue)
      if (this.roadsLayer.infrastructure().canPresentBuilding(id))
        pendingChunks.add(buildingChunkKey(building.x, building.z));
    this.chunks.update(pendingChunks);
  }
  private animate() {
    if (this.disposed) return;
    const now = performance.now(),
      frameInterval = this.lastFrame ? now - this.lastFrame : 1000 / 60,
      dt = Math.min(0.1, frameInterval / 1000);
    this.lastFrame = now;
    const begin = performance.now();
    const timingPhase =
      this.objectQueue.size ||
      this.chunks.pendingCount ||
      this.sidewalks.pendingCount ||
      this.roadsLayer.pendingRoads ||
      this.pendingCompiles
        ? 'loading'
        : 'ready';
    this.updateBuildingBatches();
    this.sidewalks.update();
    this.roadsLayer.updateRoads();
    if (this.worldState) this.rearExtensions.flushCompleted(this.worldState);
    this.actors.updateActors(now);
    this.rig.update(dt, now / 1000);
    const activeLens = LENS_PRESETS[this.rig.lens];
    this.lensFocalM.value = activeLens.focalLengthMm / 1000;
    this.lensFNumber.value = activeLens.fNumber;
    this.landscape.updateTreeDetail();
    this.updateLight(dt);
    const alpha = Math.min(1, (now - this.actors.received) / this.actors.interval);
    const time = Math.max(0, (this.actors.motionTick - 1 + alpha) * 0.05);
    const waterDt = Math.max(0, Math.min(0.1, time - this.waterTime));
    this.water?.update(waterDt, time, this.rain, this.style, 1 - this.daylight);
    this.umbrellas.update(
      this.actors.umbrellaTuples.subarray(0, this.actors.umbrellaCount * MOVEMENT_STRIDE),
      this.rain,
      time,
    );
    this.art.time.value = time;
    this.waterTime = time;
    this.lastTick = this.actors.motionTick;
    this.rainTime.value = time;
    this.rainLines.visible = this.rain > 0.1;
    this.rainOpacity.value = 0.035 + this.rain * 0.13;
    this.rainLines.position.set(this.rig.target.x, 0, this.rig.target.z);
    this.camera.updateMatrixWorld();
    this.focusTargetView.copy(this.rig.focusTarget).applyMatrix4(this.camera.matrixWorldInverse);
    const focusTarget = focusDepthFromViewZ(
      this.focusTargetView.z,
      this.camera.near,
      this.camera.far,
    );
    if (this.focusResetPending) {
      this.focus.value = focusTarget;
      this.focusResetPending = false;
    } else this.focus.value = dampFocusDepth(this.focus.value, focusTarget, dt);
    if (this.tiltShift.enabled) {
      writeTiltedFocusPlane(
        this.tiltPlane.value,
        this.focus.value,
        this.camera.fov,
        this.tiltShift,
      );
      this.tiltOpticalScale.value = miniatureOpticalScale(this.focus.value);
    }
    if (this.selected !== null) {
      const b = this.worldState?.buildings.find((b) => b.id === this.selected);
      this.highlight.visible = !!b;
      if (b) this.highlight.position.set(b.x, 0.22, b.z);
    } else this.highlight.visible = false;
    if (this.scenePass && this.pipeline)
      submitSceneAndEffects(this.renderer, this.scenePass, this.pipeline, this.sceneFrame);
    const cost = performance.now() - begin;
    if (
      !this.fixedReviewQuality &&
      this.qualityController.record(
        frameInterval,
        now,
        !document.hidden &&
          this.objectQueue.size === 0 &&
          this.chunks.pendingCount === 0 &&
          this.sidewalks.pendingCount === 0 &&
          this.roadsLayer.pendingRoads === 0 &&
          this.pendingCompiles === 0,
      ) !== null
    )
      this.applyQuality();
    this.frame++;
    this.frameTiming.record(now, cost, timingPhase, !document.hidden);
    if (now - this.lastMetricsAt >= 1000 && this.onMetrics) {
      this.lastMetricsAt = now;
      const timing = this.frameTiming.snapshot();
      this.onMetrics({
        timing,
        fps: timing.fps,
        frameMs: cost,
        calls: this.renderer.info.render.drawCalls,
        triangles: this.renderer.info.render.triangles,
        textures: this.renderer.info.memory.textures,
        geometries: this.renderer.info.memory.geometries,
        backend: this.renderer.backend.constructor.name,
        styleSwitch: this.styleSwitchMetrics ? { ...this.styleSwitchMetrics } : undefined,
        pendingBuildings: this.objectQueue.size,
        pendingChunks: this.chunks.pendingCount,
        pendingSidewalks: this.sidewalks.pendingCount,
        pendingRoads: this.roadsLayer.pendingRoads,
      });
    }
  }
  private updateLight(dt: number) {
    const hour = this.preview ?? this.hour,
      angle = ((hour - 6) / 12) * Math.PI;
    const day = THREE.MathUtils.smoothstep(Math.sin(angle), -0.2, 0.12),
      dusk = 1 - THREE.MathUtils.smoothstep(Math.abs(Math.sin(angle)), 0.05, 0.5);
    this.daylight = THREE.MathUtils.lerp(this.daylight, day, Math.min(1, dt * 3));
    const top = new THREE.Color('#659bbd')
      .lerp(new THREE.Color('#9aa5bd'), dusk * 0.5)
      .lerp(new THREE.Color('#11253e'), 1 - this.daylight);
    const low = new THREE.Color('#d5ded3')
      .lerp(new THREE.Color('#efb185'), dusk)
      .lerp(new THREE.Color('#334860'), 1 - this.daylight);
    // Steer the painted gradient toward Preetham scattering for this sun:
    // deeper zenith and a warm sunward horizon at low sun. Painted colours
    // still own night, dusk mood and the clouds; scattering only leads by day.
    const solar = new THREE.Vector3(
      Math.cos(angle),
      Math.max(-0.05, Math.sin(angle)),
      -0.45,
    ).normalize();
    const lead = this.daylight * 0.42;
    if (lead > 0.01) {
      const zenith = skyRadiance(solar, new THREE.Vector3(0, 1, 0));
      const toward = new THREE.Vector3(solar.x, 0, solar.z).normalize().setY(0.04);
      const horizon = skyRadiance(solar, toward).lerp(
        skyRadiance(solar, toward.clone().set(-toward.x, 0.04, -toward.z)),
        0.65,
      );
      top.lerp(zenith, lead);
      low.lerp(horizon, lead * 0.85);
    }
    this.skyHigh.value.copy(top);
    this.skyLow.value.copy(low);
    this.skyCloudShade.value
      .set('#8ba5bc')
      .lerp(new THREE.Color('#9c829b'), dusk * 0.7)
      .lerp(new THREE.Color('#283c58'), 1 - this.daylight);
    this.hillColor.value
      .set('#577773')
      .lerp(new THREE.Color('#747386'), dusk * 0.55)
      .lerp(new THREE.Color('#20384a'), 1 - this.daylight);
    this.skyCloud.value
      .set('#fff3dc')
      .lerp(new THREE.Color('#d29e91'), dusk * 0.5)
      .lerp(new THREE.Color('#45526a'), 1 - this.daylight);
    this.skylineColor.value
      .set('#8193a1')
      .lerp(new THREE.Color('#8a7f93'), dusk * 0.6)
      .lerp(new THREE.Color('#1c2a3c'), 1 - this.daylight);
    this.skylineNight.value = 1 - this.daylight;
    // Haze matches the horizon so hills, skyline and mountains recede into the
    // sky. The range keeps the city itself clear (overview contrast, spec 05)
    // while the backdrop gains aerial perspective. Rain pulls the veil closer.
    this.hazeColor.value.copy(low).lerp(top, 0.25);
    const veil = 1 - this.rain * 0.45;
    this.hazeNear.value = HAZE_NEAR[this.style] * veil;
    this.hazeFar.value = HAZE_FAR[this.style] * veil;
    // Morning mist peaks around 05:30-07:00 and burns off by mid-morning.
    const morning =
      THREE.MathUtils.smoothstep(hour % 24, 4.5, 5.8) *
      (1 - THREE.MathUtils.smoothstep(hour % 24, 7, 9.5));
    this.mist.value = Math.min(0.7, morning * 0.5 + this.rain * 0.3);
    const solarY = Math.sin(angle) * 450;
    this.sunDisc.position.set(Math.cos(angle) * 1050, solarY * 3, -480);
    this.sunDisc.visible = solarY > -25;
    this.moonDisc.position.set(-Math.cos(angle) * 1050, -solarY * 3, 480);
    this.moonDisc.visible = solarY < 25;
    const keyBlend = this.daylight * 2 - 1;
    // Snap only the shadow camera's light-plane translation. The key's
    // direction and elevation still follow the clock, not camera movement.
    // A lower key (about 50 degrees at 14:00) draws longer modelling shadows.
    this.sun.position.set(
      Math.cos(angle) * 350 * keyBlend,
      Math.max(30, Math.abs(Math.sin(angle)) * 270),
      -160 * keyBlend,
    );
    const shadowCamera = this.sun.shadow.camera;
    snapShadowAnchor(
      this.rig.target,
      this.sun.position,
      shadowCamera.right - shadowCamera.left,
      shadowCamera.top - shadowCamera.bottom,
      this.sun.shadow.mapSize.x,
      this.sun.shadow.mapSize.y,
      this.sun.target.position,
    );
    this.sun.position.add(this.sun.target.position);
    // Clear daytime key separates sunlit neutral facades from cool sky fill.
    // Fade the extra energy out with dusk so the established night/golden-hour
    // treatment stays intact; ambient and exposure are deliberately unchanged.
    const clearDayKey = this.daylight * (1 - dusk) * (1 - THREE.MathUtils.clamp(this.rain, 0, 1));
    this.sun.intensity = 0.16 + this.daylight * 2.05 + clearDayKey * 1.19;
    this.shaftStrength =
      dusk * THREE.MathUtils.smoothstep(this.daylight, 0.15, 0.6) * (1 - this.rain);
    this.shaftColor.value
      .set(dusk > 0.4 ? '#ffc58a' : '#ffe2b4')
      .multiplyScalar(this.shaftStrength * 0.45);
    this.sun.color
      .set('#aac8e3')
      .lerp(new THREE.Color(dusk > 0.4 ? '#ffcc9a' : '#ffe6c2'), this.daylight);
    this.sun.color.lerp(new THREE.Color('#fff1db'), clearDayKey);
    this.water?.setSun(
      v.copy(this.sun.position).sub(this.sun.target.position),
      new THREE.Color().copy(this.sun.color).multiplyScalar(this.sun.intensity * 0.5),
    );
    // Soft urban bounce keeps opaque facades and lanes readable between local
    // emitters. The visible sky is not an environment light: retain enough
    // daytime fill to read shaded facades while the sun describes orientation.
    this.hemi.intensity = 1.05 - this.daylight * 0.27;
    this.hemi.groundColor.set('#a0a09a').lerp(new THREE.Color('#748998'), this.daylight);
    // A painted moonlit fill keeps rooflines and quiet lanes readable between
    // pools of warm shop light; the sky itself can remain a deep midnight blue.
    this.hemi.color.set('#829eb9').lerp(new THREE.Color('#d8e6e7'), this.daylight);
    // Ambient follows the live sky: warm at dusk, cool at noon, for both the
    // toon (Anime) and standard (Miniature) paths. Standard surfaces also get
    // a pre-filtered sky environment for reflections.
    this.hemi.color.lerp(new THREE.Color().copy(top).lerp(low, 0.55), 0.4);
    const environment = this.skyEnvironment?.update(this.renderer, performance.now(), [
      top,
      low,
      this.skyCloud.value,
    ]);
    if (environment && this.scene.environment !== environment) this.scene.environment = environment;
    // The environment adds to the hemisphere fill on standard surfaces; trade
    // some of that fill for sky-shaped ambient rather than stacking both.
    this.scene.environmentIntensity = 0.1 + this.daylight * 0.16;
    if (this.style === 'miniature') this.hemi.intensity *= 0.78;
    this.art.sunDirection.value.copy(this.sun.position).sub(this.sun.target.position).normalize();
    this.art.cloudShadow.value =
      0.42 * Math.max(0, this.daylight * 2 - 1) * (1 - dusk * 0.6) * (1 - this.rain);
    if (this.frame % 30 === 0) {
      const target = this.rig.target;
      const lit =
        this.worldState?.buildings
          .filter((b) => b.stage === 'open' && (b.anchor || b.kind === 'shop' || b.kind === 'cafe'))
          .sort(
            (a, b) =>
              Math.hypot(a.x - target.x, a.z - target.z) -
              Math.hypot(b.x - target.x, b.z - target.z),
          )
          .slice(0, 8) ?? [];
      for (let i = 0; i < this.lanternLights.length; i++) {
        const b = lit[i],
          lamp = this.lanternLights[i];
        lamp.userData.active = !!b;
        if (b)
          lamp.position.set(
            b.x + Math.sin(b.rotation) * (b.depth * 0.45 + 1.5),
            3.1,
            b.z + Math.cos(b.rotation) * (b.depth * 0.45 + 1.5),
          );
      }
    }
    const activityHour = hour % 24,
      lightBlend = Math.min(1, dt * 0.8);
    this.art.litHome.value = THREE.MathUtils.lerp(
      this.art.litHome.value,
      activityHour >= 23 || activityHour < 5 ? 0.32 : 1,
      lightBlend,
    );
    this.art.litOffice.value = THREE.MathUtils.lerp(
      this.art.litOffice.value,
      activityHour >= 7 && activityHour < 20 ? 1 : 0.1,
      lightBlend,
    );
    this.art.litShop.value = THREE.MathUtils.lerp(
      this.art.litShop.value,
      activityHour >= 6 && activityHour < 23 ? 1 : 0.18,
      lightBlend,
    );
    // Rooms switch on and off individually as each schedule's occupancy passes their threshold.
    for (const activity of ['home', 'office', 'shop'] as const) {
      const share = this.art.occupancy[activity];
      share.value = THREE.MathUtils.lerp(
        share.value,
        windowOccupancy(activity, activityHour),
        lightBlend,
      );
    }
    for (const lamp of this.lanternLights)
      lamp.intensity = lamp.userData.active ? (1 - this.daylight) * 36 * this.art.litShop.value : 0;
    this.art.night.value = 1 - this.daylight;
    this.art.wet.value = THREE.MathUtils.lerp(
      this.art.wet.value,
      this.rain,
      Math.min(1, dt * (this.rain > this.art.wet.value ? 1 : 0.08)),
    );
    // Hysteresis: rebuild the pipeline only when streets clearly get or lose a sheen.
    const wetStreets = this.wetReflections ? this.art.wet.value > 0.03 : this.art.wet.value > 0.08;
    if (wetStreets !== this.wetReflections && this.pipeline) {
      this.wetReflections = wetStreets;
      this.buildPipeline();
    }
    // Golden-hour shafts: low sun, daylight, little rain.
    const golden = this.lightShafts ? this.shaftStrength > 0.02 : this.shaftStrength > 0.06;
    if (golden !== this.lightShafts && this.pipeline) {
      this.lightShafts = golden;
      this.buildPipeline();
    }
    this.roadsLayer.nearbyStreetLights.update(dt, this.rig.target, 1 - this.daylight);
    if (this.shine) this.shine.strength.value = 0.06 + (1 - this.daylight) * 0.2;
    this.renderer.toneMappingExposure = 1;
  }
  /** Read-only presentation state for consumers outside the renderer (audio). */
  presentationView(): PresentationView {
    return {
      world: this.worldState,
      listener: this.rig.target,
      camera: this.camera,
      current: this.actors.current,
      previous: this.actors.previous,
      motionDelta: this.actors.motionDelta,
      motionTick: this.actors.motionTick,
      riverHalfWidth: this.riverHalfWidth,
      hour: this.hour,
      rain: this.rain,
    };
  }
  private pointerDown = (e: PointerEvent) => {
    this.pointerStart = { x: e.clientX, y: e.clientY };
  };
  private pointerUp = (e: PointerEvent) => {
    if (Math.hypot(e.clientX - this.pointerStart.x, e.clientY - this.pointerStart.y) > 4) return;
    const bounds = this.renderer.domElement.getBoundingClientRect(),
      ray = new THREE.Raycaster();
    ray.setFromCamera(
      new THREE.Vector2(
        ((e.clientX - bounds.left) / bounds.width) * 2 - 1,
        (-(e.clientY - bounds.top) / bounds.height) * 2 + 1,
      ),
      this.camera,
    );
    const hits = ray.intersectObjects([this.chunks.root, ...this.actors.actorIds.keys()], true);
    const hit = hits.find((candidate) => {
      for (let object: THREE.Object3D | null = candidate.object; object; object = object.parent)
        if (!object.visible) return false;
      return true;
    });
    if (!hit) return;
    const building = this.chunks.buildingAt(hit);
    if (building !== undefined) this.onPick?.(building, 'building');
    else if (hit.instanceId !== undefined) {
      const id = this.actors.actorIds.get(hit.object)?.[hit.instanceId];
      if (id !== undefined) this.onPick?.(id, 'traveler');
    }
  };
  dispose() {
    this.disposed = true;
    document.removeEventListener('visibilitychange', this.onVisibility);
    this.renderStartup?.dispose();
    this.renderer.setAnimationLoop(null);
    this.resize.disconnect();
    this.rig.dispose();
    this.renderer.domElement.removeEventListener('pointerdown', this.pointerDown);
    this.renderer.domElement.removeEventListener('pointerup', this.pointerUp);
    this.resetWorld();
    this.roadsLayer.streetLighting?.root.removeFromParent();
    this.retire(() => this.roadsLayer.streetLighting?.dispose());
    this.umbrellas.root.removeFromParent();
    this.retire(() => this.umbrellas.dispose());
    this.actors.buses.root.removeFromParent();
    this.retire(() => this.actors.buses.dispose());
    this.actors.trains.root.removeFromParent();
    this.retire(() => this.actors.trains.dispose());
    this.actors.fleet.root.removeFromParent();
    this.retire(() => this.actors.fleet.dispose());
    this.disposePipeline();
    this.scene.environment = null;
    const environment = this.skyEnvironment;
    this.skyEnvironment = null;
    this.retire(() => environment?.dispose());
    const materials = new Set<THREE.Material>(),
      geometries = new Set<THREE.BufferGeometry>();
    this.scene.traverse((o) => {
      if (o instanceof THREE.Mesh || o instanceof THREE.Line) {
        geometries.add(o.geometry);
        for (const m of Array.isArray(o.material) ? o.material : [o.material])
          if (!m.userData.artKey && m !== this.art.contours) materials.add(m);
      }
    });
    this.retire(() => {
      for (const g of geometries) g.dispose();
      for (const m of materials) m.dispose();
      this.art.dispose();
      this.renderer.dispose();
    });
    this.renderer.domElement.remove();
  }
}

export function distanceToSegment(p: Point, a: Point, b: Point) {
  const dx = b.x - a.x,
    dz = b.z - a.z;
  const t = Math.max(
    0,
    Math.min(1, ((p.x - a.x) * dx + (p.z - a.z) * dz) / (dx * dx + dz * dz || 1)),
  );
  return Math.hypot(p.x - a.x - dx * t, p.z - a.z - dz * t);
}
