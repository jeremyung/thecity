import * as THREE from 'three/webgpu';
import {
  float,
  normalLocal,
  positionLocal,
  texture as textureNode,
  triplanarTexture,
} from 'three/tsl';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { ArtMaterials, signTexture } from './materials';
import { paintedSurfaceTint } from './painted-surface';
import { buildingKit, disposeGroup, Kit } from './kit';
import type { BuildingKind, Building, Style } from '../types';
import { VEHICLE_KINDS, VehicleFleet } from './vehicles';
import { CityBuses } from './buses';
import { TrainFleet } from './trains';

/** Isolated development viewer for the same authored building kits used by the city. */
export class ArchitectureAssetReview {
  private readonly compactMidriseReview =
    import.meta.env.DEV && new URLSearchParams(window.location.search).get('midrise') === 'compact';
  readonly scene = new THREE.Scene();
  readonly camera = new THREE.PerspectiveCamera(38, 1, 0.1, 1400);
  readonly art = new ArtMaterials();
  readonly renderer: THREE.WebGPURenderer;
  readonly controls: OrbitControls;
  private readonly models = new THREE.Group();
  private readonly street = new THREE.Group();
  private readonly fleet: VehicleFleet;
  private readonly buses: CityBuses;
  private readonly trainFleet: TrainFleet;
  private traffic: Array<{
    id: number;
    kind: number;
    x: number;
    z: number;
    dir: number;
    speed: number;
  }> = [];
  private lastFrame = 0;
  private trafficPaused = false;
  private readonly materialStudy = new THREE.Group();
  private lowriseSamples: Array<{ building: Building; model: THREE.Group }> = [];
  private readonly sun = new THREE.DirectionalLight('#fff1d1', 3);
  private readonly hemisphere = new THREE.HemisphereLight('#c8dce3', '#748998', 2);
  private readonly ground: THREE.Mesh;
  private readonly resize: ResizeObserver;
  private disposed = false;
  private started = false;
  private candidateTexture: THREE.Texture | null = null;
  private shopFacadeMesh: THREE.Mesh | null = null;
  private shopFacadeSource: THREE.Material | null = null;
  private shopFacadeTexture: THREE.Texture | null = null;
  private shopFacadeVariants: Partial<Record<Style, THREE.Material>> = {};
  private style: Style = 'anime';
  private shopPlasterEnabled = true;
  private lowriseStage: 'open' | 'construction' | 'renovation' = 'open';
  private lowriseProgress = 1;
  onMaterialStatus?: (status: 'loading' | 'ready' | 'error') => void;

  constructor(
    readonly element: HTMLElement,
    style: Style = 'anime',
    private readonly materialCandidate: 'plaster' | 'concrete' = 'plaster',
  ) {
    const forceWebGL = new URLSearchParams(window.location.search).has('webgl');
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
    this.element.appendChild(this.renderer.domElement);
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.075;
    this.controls.target.set(0, 24, -8);
    this.controls.minDistance = 35;
    this.controls.maxDistance = 420;
    this.controls.maxPolarAngle = 1.42;

    this.scene.background = new THREE.Color('#afc4ca');
    this.scene.fog = new THREE.Fog('#afc4ca', 300, 1000);
    this.scene.add(
      this.hemisphere,
      this.sun,
      this.sun.target,
      this.street,
      this.models,
      this.materialStudy,
    );
    this.sun.position.set(-190, 280, 210);
    this.sun.target.position.set(0, 0, 0);
    this.sun.castShadow = true;
    this.sun.shadow.mapSize.set(2048, 2048);
    this.sun.shadow.camera.left = -180;
    this.sun.shadow.camera.right = 180;
    this.sun.shadow.camera.top = 180;
    this.sun.shadow.camera.bottom = -180;
    this.sun.shadow.camera.near = 1;
    this.sun.shadow.camera.far = 650;
    this.sun.shadow.bias = -0.0002;
    this.sun.shadow.normalBias = 0.35;

    this.ground = new THREE.Mesh(new THREE.PlaneGeometry(360, 260), this.art.get('#88958a'));
    this.ground.rotation.x = -Math.PI / 2;
    this.ground.position.set(-2, -0.12, -18);
    this.ground.receiveShadow = true;
    this.scene.add(this.ground);
    this.buildStreet();
    this.fleet = new VehicleFleet(this.art, 1000);
    this.buses = new CityBuses(this.art, (release) => release());
    this.trainFleet = new TrainFleet(this.art, 8);
    this.scene.add(this.fleet.root, this.buses.root, this.trainFleet.root);
    this.buildTraffic();
    if (import.meta.env.DEV) Object.assign(window, { __review: this });
    this.buildArchitecture();
    this.buildMaterialStudy();
    this.art.setStyle(style);
    this.resize = new ResizeObserver(() => this.setSize());
    this.resize.observe(element);
    window.addEventListener('resize', this.onWindowResize);
    this.camera.position.set(20, 104, 210);
    this.camera.lookAt(this.controls.target);
    this.controls.update();
  }

  async start(): Promise<void> {
    if (this.disposed || this.started) return;
    await this.renderer.init();
    if (this.disposed) {
      this.renderer.dispose();
      return;
    }
    this.setSize();
    this.started = true;
    this.renderer.setAnimationLoop((now: number) => {
      if (this.disposed) return;
      this.updateTraffic(now);
      this.controls.update();
      this.renderer.render(this.scene, this.camera);
    });
  }

  setStyle(style: Style): void {
    this.style = style;
    this.art.setStyle(style);
    this.scene.traverse((object) => {
      if (object instanceof THREE.Mesh) {
        if (object === this.shopFacadeMesh && this.shopFacadeSource) {
          object.material = this.currentShopFacadeMaterial(style);
          return;
        }
        object.material = Array.isArray(object.material)
          ? object.material.map((material) => this.art.variant(material, style))
          : this.art.variant(object.material, style);
      }
    });
  }

  setNight(enabled: boolean): void {
    this.art.night.value = enabled ? 1 : 0;
    this.sun.intensity = enabled ? 0.08 : 3;
    this.hemisphere.intensity = enabled ? 0.3 : 2;
    this.scene.background = new THREE.Color(enabled ? '#142331' : '#afc4ca');
    if (this.scene.fog instanceof THREE.Fog) this.scene.fog.color.copy(this.scene.background);
  }

  setShopPlaster(enabled: boolean): void {
    this.shopPlasterEnabled = enabled;
    if (this.shopFacadeMesh && this.shopFacadeSource) {
      this.shopFacadeMesh.material = this.currentShopFacadeMaterial(this.style);
    }
  }

  resetCamera(): void {
    this.resetShadowView();
    this.controls.minDistance = 8;
    this.controls.target.set(0, 24, -8);
    this.camera.position.set(20, 104, 210);
    this.controls.update();
  }

  inspectMaterial(): void {
    this.resetShadowView();
    this.controls.minDistance = 8;
    this.controls.target.set(116, 3.2, -38);
    this.camera.position.set(118, 4.8, -23);
    this.controls.update();
  }

  inspectShop(): void {
    this.controls.minDistance = 8;
    this.controls.target.set(-94, 4, -28);
    this.camera.position.set(-77, 12, 1);
    this.controls.update();
    // Concentrate shadow texels on the inspected facade instead of the entire
    // specimen field. This reveals recesses and contact at architectural scale.
    this.sun.target.position.set(-94, 3, -28);
    this.sun.position.set(-154, 95, 42);
    Object.assign(this.sun.shadow.camera, { left: -26, right: 26, top: 26, bottom: -26 });
    this.sun.shadow.camera.updateProjectionMatrix();
  }

  inspectApartment(): void {
    this.resetShadowView();
    this.controls.minDistance = 8;
    this.controls.target.set(-47, 34, -30);
    this.camera.position.set(29, 77, 108);
    this.controls.update();
  }

  inspectOffice(): void {
    this.resetShadowView();
    this.controls.minDistance = 8;
    this.controls.target.set(10, 7, -21);
    this.camera.position.set(29, 12, 2);
    this.controls.update();
  }

  inspectLowrise(rear = false): void {
    this.resetShadowView();
    this.controls.minDistance = 8;
    this.controls.target.set(-14, 4, -102);
    this.camera.position.set(rear ? -54 : 7, 28, rear ? -145 : -59);
    this.controls.update();
    this.sun.target.position.set(-14, 3, -102);
    this.sun.position.set(-74, 95, -32);
    Object.assign(this.sun.shadow.camera, { left: -38, right: 38, top: 38, bottom: -38 });
    this.sun.shadow.camera.updateProjectionMatrix();
  }

  /** Rebuild only the three low-rise review specimens at the requested lifecycle stage. */
  setLowriseStage(stage: 'open' | 'construction' | 'renovation', progress: number): void {
    if (this.disposed) return;
    const boundedProgress = Number.isFinite(progress) ? THREE.MathUtils.clamp(progress, 0, 1) : 0.5;
    if (
      stage === this.lowriseStage &&
      (stage === 'open' || boundedProgress === this.lowriseProgress)
    )
      return;
    this.lowriseStage = stage;
    this.lowriseProgress = boundedProgress;
    this.lowriseSamples = this.lowriseSamples.map(({ building, model: previous }) => {
      this.models.remove(previous);
      disposeGroup(previous);
      // The review's generic 'construction' preview uses a stage no kit branch special-cases.
      const kitStage = stage === 'construction' ? 'foundation' : stage;
      const next = buildingKit(
        { ...building, stage: kitStage, progress: boundedProgress },
        this.art,
        {
          compactMidrise: this.compactMidriseReview,
        },
      );
      next.position.set(building.x, 0, building.z);
      next.traverse((object) => {
        if (object instanceof THREE.Mesh) {
          object.castShadow = true;
          object.receiveShadow = true;
        }
      });
      this.models.add(next);
      return { building, model: next };
    });
  }

  private resetShadowView(): void {
    this.sun.position.set(-190, 280, 210);
    this.sun.target.position.set(0, 0, 0);
    Object.assign(this.sun.shadow.camera, { left: -180, right: 180, top: 180, bottom: -180 });
    this.sun.shadow.camera.updateProjectionMatrix();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.renderer.setAnimationLoop(null);
    this.resize.disconnect();
    window.removeEventListener('resize', this.onWindowResize);
    this.controls.dispose();
    // A texture may still be in flight. Dispose it here; its late callback
    // checks `disposed` and will never attach it to a retired material.
    this.candidateTexture?.dispose();
    this.candidateTexture = null;
    for (const material of Object.values(this.shopFacadeVariants)) material?.dispose();
    this.shopFacadeVariants = {};
    this.shopFacadeTexture?.dispose();
    this.shopFacadeTexture = null;
    disposeGroup(this.models);
    disposeGroup(this.street);
    this.fleet.dispose();
    this.buses.dispose();
    this.trainFleet.dispose();
    disposeGroup(this.materialStudy);
    this.ground.geometry.dispose();
    this.art.dispose();
    this.renderer.dispose();
    this.renderer.domElement.remove();
  }

  private readonly onWindowResize = () => this.setSize();

  /** Mixed traffic in both lanes, driving slowly so variants, wheels and lamps read. */
  private buildTraffic(): void {
    const mix = [1, 4, 1, 5, 4, 6, 1, 7, 4, 8, 1, 5];
    for (const [lane, dir, base] of [
      [3.4, 1, 100],
      [12.6, -1, 300],
    ] as const) {
      mix.forEach((kind, i) =>
        this.traffic.push({
          id: base + i * 7,
          kind,
          x: -56 + i * 8.6 + (dir < 0 ? 4 : 0),
          z: lane,
          dir,
          speed: 2.2,
        }),
      );
      this.traffic.push({ id: 900 + base, kind: 3, x: 52, z: lane, dir, speed: 1.6 });
    }
  }

  /** Review controls for captures: freeze traffic at a pose. */
  setTrafficPaused(paused: boolean): void {
    this.trafficPaused = paused;
  }

  private updateTraffic(now: number): void {
    const dt = this.lastFrame ? Math.min(0.1, (now - this.lastFrame) / 1000) : 1 / 60;
    this.lastFrame = now;
    // The review street surface is 0.14 m below the city's road slab top.
    const lift = 0.16 - 0.3;
    this.fleet.begin();
    this.buses.begin(now);
    let bus = 0;
    for (const v of this.traffic) {
      if (!this.trafficPaused) {
        v.x += v.dir * v.speed * dt;
        if (v.x > 60) v.x = -60;
        if (v.x < -60) v.x = 60;
      }
      const yaw = v.dir > 0 ? Math.PI / 2 : -Math.PI / 2;
      if (v.kind === 3) this.buses.set(bus++, v.id, v.x, v.z, yaw, null, 0.16);
      else this.fleet.add(v.id, v.kind, v.x, v.z, yaw, lift, 0, this.trafficPaused ? 0 : dt);
    }
    this.fleet.finish();
    this.buses.finish(bus);
    // A consist on the review station's near rail (rails run along x at y=0.44).
    this.trainFleet.begin();
    this.trainFleet.add(1, -119, -73, Math.PI / 2, 0.44, 0, dt);
    this.trainFleet.finish(1);
  }

  private setSize(): void {
    if (this.disposed) return;
    const width = this.element.clientWidth,
      height = this.element.clientHeight;
    if (!width || !height) return;
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height);
  }

  private buildArchitecture(): void {
    const samples: Array<{
      name: string;
      x: number;
      z: number;
      kind: BuildingKind;
      width: number;
      depth: number;
      floors: number;
      anchor?: boolean;
      seed?: number;
    }> = [
      {
        name: 'Kissa Aoba · corner shop',
        x: -94,
        z: -28,
        kind: 'shop',
        width: 22,
        depth: 16,
        floors: 2,
        anchor: true,
      },
      {
        name: 'Apartment tower · 20 floors',
        x: -47,
        z: -30,
        kind: 'apartment',
        width: 24,
        depth: 20,
        floors: 20,
      },
      { name: 'Office block', x: 3, z: -31, kind: 'office', width: 27, depth: 23, floors: 8 },
      {
        name: 'Mixed-use tower · 28 floors',
        x: 62,
        z: -38,
        kind: 'office',
        width: 32,
        depth: 28,
        floors: 28,
      },
      {
        name: 'Narrow machiya',
        x: -32,
        z: -103.04,
        kind: 'home',
        width: 9.5,
        depth: 12,
        floors: 2,
        seed: 8,
      },
      {
        name: 'Three-floor house',
        x: -18.55,
        z: -102.2,
        kind: 'home',
        width: 13,
        depth: 10,
        floors: 3,
        seed: 23,
      },
      {
        name: 'Neighborhood shop-house',
        x: -3.85,
        z: -101.78,
        kind: 'shop',
        width: 12,
        depth: 9,
        floors: 2,
        seed: 73,
      },
    ];
    samples.forEach((sample, index) => {
      const building: Building = {
        id: index + 1,
        parcel_id: index + 101,
        x: sample.x,
        z: sample.z,
        rotation: 0,
        width: sample.width,
        depth: sample.depth,
        floors: sample.floors,
        kind: sample.kind,
        stage: 'open',
        progress: 1,
        business_active: true,
        capacity: sample.floors * 8,
        occupancy: Math.round(sample.floors * 2.5),
        built_day: 0,
        name: sample.name,
        appearance_seed: sample.seed ?? 4100 + index * 137 + (index === 2 ? 16 : 0),
        history: [],
        anchor: sample.anchor ?? false,
        vacant_days: 0,
      };
      const model = buildingKit(building, this.art, { compactMidrise: this.compactMidriseReview });
      if (index === 0) {
        // Match the kit's deterministic wall material exactly. Other facade
        // parts (glass, trim, timber, roof and signage) keep their own materials.
        this.shopFacadeSource = this.art.get(
          this.art.palette[(building.appearance_seed >>> 0) % this.art.palette.length],
        );
        model.traverse((object) => {
          if (object instanceof THREE.Mesh && object.material === this.shopFacadeSource)
            this.shopFacadeMesh = object;
        });
      }
      model.position.set(sample.x, 0, sample.z);
      model.traverse((object) => {
        if (object instanceof THREE.Mesh) {
          object.castShadow = true;
          object.receiveShadow = true;
        }
      });
      this.models.add(model);
      if (index >= 4) this.lowriseSamples.push({ building, model });
    });
    this.buildStationFront();
  }

  private buildStreet(): void {
    const kit = new Kit();
    const asphalt = this.art.get('#666f6c', 0.96);
    const curb = this.art.get('#aaa99e');
    const sidewalk = this.art.get('#b5b3a2');
    const paint = this.art.get('#e8dfc6');
    const lamp = this.art.get('#4f5c59');
    kit.box(-8, 0.08, 8, 260, 0.16, 18, asphalt);
    kit.box(-8, 0.19, -3.1, 260, 0.09, 3, curb);
    kit.box(-8, 0.19, 19.1, 260, 0.09, 3, curb);
    kit.box(-8, 0.14, -10.3, 260, 0.08, 11, sidewalk);
    kit.box(-8, 0.14, 26.3, 260, 0.08, 11, sidewalk);
    kit.box(-8, 0.17, 8, 260, 0.035, 0.18, paint);
    for (let x = -120; x <= 120; x += 24) {
      kit.box(x, 0.18, -3.05, 0.16, 0.04, 0.22, paint);
      kit.box(x, 0.18, 19.05, 0.16, 0.04, 0.22, paint);
    }
    for (const x of [-110, -55, 0, 55, 110]) {
      kit.box(x, 4.3, 23.5, 0.18, 8.4, 0.18, lamp);
      kit.box(x, 8.42, 22.4, 0.18, 0.14, 2.6, lamp);
      kit.box(x, 8.36, 21.15, 1.1, 0.08, 1.35, this.art.get('#d3bd93', 0.5, true, 'shop'));
    }
    // Controlled neighborhood comparison: the same 2.2 m parcel gaps used
    // by coordinated blocks, with aligned entrances facing a local street.
    // This is an authored review arrangement, not a generated-city result.
    kit.box(-18, 0.07, -92, 57, 0.14, 6, asphalt);
    kit.box(-18, 0.13, -96, 57, 0.1, 2, sidewalk);
    kit.box(-18, 0.13, -88, 57, 0.1, 2, sidewalk);
    kit.box(-18, 0.2, -95.1, 57, 0.14, 0.18, curb);
    kit.box(-18, 0.2, -88.9, 57, 0.14, 0.18, curb);
    for (const x of [-40, 4]) {
      kit.box(x, 0.15, -94.65, 3, 0.025, 0.1, paint);
      kit.box(x, 0.15, -89.35, 3, 0.025, 0.1, paint);
    }
    this.street.add(kit.finish());
  }

  private buildStationFront(): void {
    const kit = new Kit();
    const platform = this.art.get('#aaa99e');
    const edge = this.art.get('#d3b962');
    const iron = this.art.get('#71817e', 0.58);
    const canopy = this.art.get('#556b6d');
    const shelter = this.art.get('#d0cbbc');
    const glass = this.art.get('#7d9899', 0.32);
    const rail = this.art.get('#6d6457');
    // A compact station apron and its actual platform geometry anchor the assembled street.
    kit.box(-119, 0.38, -41, 36, 0.76, 50, platform);
    kit.box(-119, 0.8, -16, 35.5, 0.12, 0.7, edge);
    kit.box(-119, 5.2, -41, 39, 0.24, 47, canopy);
    for (const x of [-135, -103]) {
      for (const z of [-61, -41, -21]) {
        kit.cylinder(x, 2.8, z, 0.15, 4.8, iron);
      }
    }
    kit.box(-119, 2.2, -66, 13, 4.4, 10, shelter);
    kit.box(-119, 4.48, -66, 14, 0.2, 11, canopy);
    kit.box(-119, 2.45, -60.86, 7.8, 2.5, 0.08, glass);
    kit.box(-119, 2.2, -14, 4.8, 4.1, 2.3, shelter);
    kit.box(-119, 4.32, -14, 5.4, 0.18, 2.9, canopy);

    // Rail pair and sleepers run behind the street-front platform.
    kit.box(-119, 0.44, -73, 120, 0.14, 0.15, iron);
    kit.box(-119, 0.44, -79, 120, 0.14, 0.15, iron);
    for (let x = -175; x <= -60; x += 2.6) kit.box(x, 0.27, -76, 0.35, 0.12, 8, rail);
    this.street.add(kit.finish());

    const texture = signTexture('青葉川  AOBAGAWA');
    const material = new THREE.MeshBasicNodeMaterial({
      map: texture,
      transparent: true,
      side: THREE.DoubleSide,
    });
    const sign = new THREE.Mesh(new THREE.BoxGeometry(0.12, 1.1, 5.2), material);
    sign.position.set(-119, 5.05, -13.9);
    sign.userData.ownedMaterial = true;
    sign.userData.ownedTexture = texture;
    this.street.add(sign);
  }

  private buildMaterialStudy(): void {
    const x = 116,
      z = -38;
    const wallMaterial = new THREE.MeshStandardNodeMaterial({ color: '#d8cdb5', roughness: 0.94 });
    const wall = new THREE.Mesh(new THREE.BoxGeometry(6, 6, 0.18), wallMaterial);
    wall.position.set(x, 3, z);
    wall.castShadow = true;
    wall.receiveShadow = true;
    wall.userData.ownedMaterial = true;
    this.materialStudy.add(wall);

    const plaqueTexture = signTexture(
      this.materialCandidate === 'concrete' ? 'CONCRETE' : 'PLASTER',
      '#f1eadb',
      '#384d4c',
    );
    const plaqueMaterial = new THREE.MeshBasicNodeMaterial({
      map: plaqueTexture,
      side: THREE.DoubleSide,
    });
    const plaque = new THREE.Mesh(new THREE.PlaneGeometry(5.6, 0.62), plaqueMaterial);
    plaque.position.set(x, 6.55, z + 0.13);
    plaque.userData.ownedMaterial = true;
    plaque.userData.ownedTexture = plaqueTexture;
    this.materialStudy.add(plaque);

    // A thin baseline keeps the sample legible against the street without
    // turning it into another building or a backdrop.
    const plinthMaterial = this.art.get('#898b7d');
    const plinth = new THREE.Mesh(new THREE.BoxGeometry(6.3, 0.22, 0.65), plinthMaterial);
    plinth.position.set(x, 0.11, z + 0.18);
    plinth.receiveShadow = true;
    this.materialStudy.add(plinth);

    const candidate = new THREE.TextureLoader().load(
      this.materialCandidate === 'concrete'
        ? '/art/textures/cast-concrete-pilot-512.png'
        : '/art/textures/ivory-plaster-v1.png',
      (texture) => {
        if (this.disposed) return;
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.wrapS = THREE.RepeatWrapping;
        texture.wrapT = THREE.RepeatWrapping;
        texture.repeat.set(3, 3);
        texture.anisotropy = 4;
        texture.needsUpdate = true;
        wallMaterial.map = texture;
        wallMaterial.needsUpdate = true;
        wallMaterial.color.set('#ffffff');
        wall.userData.ownedTexture = texture;
        this.applyPlasterToShop(texture);
        this.candidateTexture = null; // The wall now owns its disposal.
        this.onMaterialStatus?.('ready');
      },
      undefined,
      () => {
        if (this.candidateTexture === candidate) {
          candidate.dispose();
          this.candidateTexture = null;
        }
        if (!this.disposed) this.onMaterialStatus?.('error');
      },
    );
    this.candidateTexture = candidate;
    this.onMaterialStatus?.('loading');
  }

  private currentShopFacadeMaterial(style: Style): THREE.Material {
    return this.shopPlasterEnabled && this.shopFacadeVariants[style]
      ? this.shopFacadeVariants[style]!
      : this.art.variant(this.shopFacadeSource!, style);
  }

  private applyPlasterToShop(candidate: THREE.Texture): void {
    if (!this.shopFacadeMesh || !this.shopFacadeSource) return;

    // The swatch uses 3 repeats across its 6m face. The facade instead maps
    // the source's authored 2m tile directly through local-space triplanar UVs.
    const facadeTexture = candidate.clone();
    facadeTexture.wrapS = THREE.RepeatWrapping;
    facadeTexture.wrapT = THREE.RepeatWrapping;
    facadeTexture.repeat.set(1, 1);
    facadeTexture.anisotropy = 4;
    facadeTexture.colorSpace = THREE.SRGBColorSpace;
    facadeTexture.needsUpdate = true;
    this.shopFacadeTexture = facadeTexture;

    for (const style of ['anime', 'miniature'] as const) {
      const source = this.art.variant(this.shopFacadeSource, style) as
        THREE.MeshToonNodeMaterial | THREE.MeshStandardNodeMaterial;
      const material = source.clone() as
        THREE.MeshToonNodeMaterial | THREE.MeshStandardNodeMaterial;
      delete material.userData.artKey;
      delete material.userData.artBuildingClass;
      delete material.userData.artBatchMaterial;
      material.color.set('#ffffff');
      const baseColor = triplanarTexture(
        textureNode(facadeTexture),
        null,
        null,
        float(0.5), // one texture period per 2m of local facade
        positionLocal,
        normalLocal,
      ).rgb;
      // A base-colour candidate must retain the selected style's pigment treatment.
      material.colorNode = style === 'anime' ? baseColor.mul(paintedSurfaceTint()) : baseColor;
      material.needsUpdate = true;
      this.shopFacadeVariants[style] = material;
    }
    this.shopFacadeMesh.material = this.currentShopFacadeMaterial(this.style);
  }
}
