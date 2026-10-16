import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import { CameraDirector } from '../web/src/camera/director';
import { cameraPathClearance } from '../web/src/camera/clearance';
import { verticalFovDegrees } from '../web/src/camera/lens';
import type { World } from '../web/src/types';
// @ts-expect-error Node built-in types are not included in the app TypeScript config.
import { readFileSync } from 'node:fs';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const orbitMocks = vi.hoisted(() => ({
  instances: [] as Array<{
    start(): void;
    disposed: boolean;
    bendOnUpdate: ((camera: THREE.Camera) => void) | null;
  }>,
}));

vi.mock('three/addons/controls/OrbitControls.js', async () => {
  const Three = await import('three');
  class MockOrbitControls {
    readonly camera: THREE.Camera;
    target = new Three.Vector3();
    enableDamping = false;
    dampingFactor = 0;
    screenSpacePanning = true;
    minDistance = 0;
    maxDistance = Infinity;
    minPolarAngle = 0;
    maxPolarAngle = Math.PI;
    disposed = false;
    bendOnUpdate: ((camera: THREE.Camera) => void) | null = null;
    private listeners = new Map<string, Set<(event: Event) => void>>();

    constructor(camera: THREE.Camera, _element: HTMLElement) {
      this.camera = camera;
      orbitMocks.instances.push(this);
    }
    addEventListener(type: string, listener: (event: Event) => void): void {
      const listeners = this.listeners.get(type) ?? new Set();
      listeners.add(listener);
      this.listeners.set(type, listeners);
    }
    removeEventListener(type: string, listener: (event: Event) => void): void {
      this.listeners.get(type)?.delete(listener);
    }
    update(): boolean {
      this.bendOnUpdate?.(this.camera);
      return false;
    }
    dispose(): void {
      this.disposed = true;
    }
    start(): void {
      for (const listener of this.listeners.get('start') ?? []) listener(new Event('start'));
    }
  }
  return { OrbitControls: MockOrbitControls };
});

function makeWorld(): World {
  return {
    schema_version: 1,
    engine_version: 'test',
    seed: 7,
    tick: 0,
    day: 0,
    minute_of_day: 500,
    development_rate: 4,
    revision: 0,
    stage: 'Founding',
    bounds: { min_x: -400, max_x: 400, min_z: -400, max_z: 400 },
    terrain: {
      river: [
        { x: 10, z: 0 },
        { x: 20, z: 0 },
        { x: 30, z: 0 },
      ],
      protected: [],
      gateways: [],
    },
    weather: { kind: 'clear', intensity: 0 },
    nodes: [
      { id: 1, x: 0, z: 0, kind: 'street' },
      { id: 2, x: 100, z: 0, kind: 'station' },
    ],
    roads: [
      {
        id: 4,
        from: 1,
        to: 2,
        kind: 'rail',
        stage: 'open',
        progress: 1,
        connected: true,
        length: 100,
        lanes: 2,
      },
    ],
    parcels: [],
    buildings: [],
    projects: [],
    trains: [{ id: 500, route: [1, 2], progress: 15 }],
    events: [],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
  };
}

function makeBuilding(id: number, x: number, z: number): World['buildings'][number] {
  return {
    id,
    parcel_id: id + 1000,
    x,
    z,
    rotation: 0,
    width: 18,
    depth: 14,
    floors: 2,
    kind: 'home',
    stage: 'open',
    progress: 1,
    capacity: 8,
    occupancy: 4,
    built_day: 0,
    name: `Home ${id}`,
    appearance_seed: id,
    history: [],
    anchor: true,
    vacant_days: 0,
  };
}

type ResidentFixture = {
  id: number;
  name: string;
  home_building: number;
  work_building: number | null;
  activity: string;
  transfer?: string;
  train_id?: number | null;
  bus_id?: number | null;
  bus_origin_stop?: number | null;
  bus_destination_stop?: number | null;
  bus_stop_node?: number | null;
  location_node?: number | null;
};

function withResident(world: World, resident: ResidentFixture): World {
  return { ...world, residents: [resident] } as World;
}

function makeElement(): HTMLElement {
  return Object.assign(new EventTarget(), { tabIndex: -1 }) as unknown as HTMLElement;
}

beforeEach(() => {
  orbitMocks.instances = [];
  vi.stubGlobal('window', {
    matchMedia: () => ({
      matches: false,
      addEventListener() {},
      removeEventListener() {},
      addListener() {},
      removeListener() {},
    }),
  });
});

describe('CameraDirector', () => {
  it('frames a followed building as a volume and grows the frame after redevelopment', () => {
    const camera = new THREE.PerspectiveCamera(42, 0.7, 0.1, 2000);
    const director = new CameraDirector(camera, makeElement());
    const world = makeWorld();
    const building = {
      ...makeBuilding(41, 100, 20),
      anchor: false,
      kind: 'office',
      floors: 6,
      stage: 'demolition',
      progress: 0.5,
    };
    world.buildings = [building];
    director.world(world);
    director.follow(41);
    const state = director as unknown as {
      desiredTarget: THREE.Vector3;
      desiredPosition: THREE.Vector3;
    };
    const originalDistance = state.desiredPosition.distanceTo(state.desiredTarget);
    expect(state.desiredTarget.x).toBe(100);
    expect(state.desiredTarget.z).toBe(20);
    expect(state.desiredTarget.y).toBeGreaterThan(9);
    world.buildings = [{ ...building, floors: 18, stage: 'structure' }];
    world.revision++;
    director.world(world);
    director.update(0.1, 1);
    expect(state.desiredTarget.y).toBeGreaterThan(29);
    expect(director.focusTarget.y).toBeCloseTo(state.desiredTarget.y);
    expect(state.desiredPosition.distanceTo(state.desiredTarget)).toBeGreaterThan(originalDistance);
    director.dispose();
  });

  it('focuses a followed train at its center instead of the look-ahead target', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.follow(500);
    const state = director as unknown as { desiredTarget: THREE.Vector3 };
    expect(state.desiredTarget.x).toBeCloseTo(25);
    expect(director.focusTarget.x).toBeCloseTo(15);
    expect(director.focusTarget.y).toBeCloseTo(8.735);
    director.dispose();
  });

  it('focuses movement followers at their published subject point and holds it during loss grace', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.follow(812);
    director.setPositions(new Float32Array([812, 30, 18, 0, 1, 11.15]));
    director.update(0.1, 0.1);
    const state = director as unknown as { desiredTarget: THREE.Vector3 };
    expect(state.desiredTarget.x).toBeCloseTo(36);
    expect(director.focusTarget.x).toBeCloseTo(30);
    expect(director.focusTarget.y).toBeCloseTo(11.15);
    expect(director.focusTarget.z).toBeCloseTo(18);

    director.setPositions(new Float32Array());
    for (let i = 0; i < 10; i++) director.update(0.1, 0.2 + i * 0.1);
    expect(director.focusTarget.x).toBeCloseTo(30);
    expect(director.focusTarget.y).toBeCloseTo(11.15);
    expect(director.focusTarget.z).toBeCloseTo(18);

    // Manual input ends Follow immediately, so focus returns to the live camera target.
    orbitMocks.instances.at(-1)?.start();
    expect(director.focusTarget).toBe(director.target);
    director.dispose();
  });

  it('uses the published bus position for bus look-ahead focus', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.follow(813);
    director.setPositions(new Float32Array([813, 48, -22, Math.PI / 2, 3, 2.4]));
    director.update(0.1, 0.1);
    const state = director as unknown as { desiredTarget: THREE.Vector3 };
    expect(state.desiredTarget.z).toBeCloseTo(-14);
    expect(director.focusTarget.x).toBeCloseTo(48);
    expect(director.focusTarget.y).toBeCloseTo(2.4);
    expect(director.focusTarget.z).toBeCloseTo(-22);
    director.dispose();
  });

  it('starts at the intended scene and keeps a bounded set of real anchors', () => {
    const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 2_000);
    const director = new CameraDirector(camera, makeElement());
    expect(camera.position.toArray()).toEqual([240, 170, 290]);
    expect(director.target.toArray()).toEqual([50, 0, 20]);

    const world = makeWorld();
    world.buildings = Array.from({ length: 60 }, (_, index) =>
      makeBuilding(index + 1, index * 9, index * -4),
    );
    director.world(world);
    const anchors = (director as unknown as { anchors: unknown[] }).anchors;
    expect(anchors.length).toBeLessThanOrEqual(32);
    expect(anchors.length).toBeGreaterThan(1);
    director.dispose();
  });

  it("frames a neighborhood's real street interior around a blocker and holds the shot under Explore", () => {
    const world = makeWorld();
    world.terrain.river = [];
    world.nodes = [
      { id: 3, x: 0, z: 0, kind: 'street' },
      { id: 4, x: 0, z: 120, kind: 'street' },
    ];
    world.roads = [
      {
        id: 20,
        from: 3,
        to: 4,
        kind: 'local',
        stage: 'open',
        progress: 1,
        connected: true,
        length: 120,
        lanes: 1,
      },
    ];
    const localHomes = [
      makeBuilding(101, 20, 50),
      makeBuilding(102, 24, 60),
      makeBuilding(103, 22, 70),
    ].map((building) => ({ ...building, anchor: false }));
    const blocker = {
      ...makeBuilding(104, 15, 82),
      anchor: false,
      floors: 16,
      width: 24,
      depth: 18,
    };
    blocker.stage = 'structure';
    world.buildings = [...localHomes, blocker];
    world.parcels = localHomes.map((building) => ({
      id: building.parcel_id,
      x: building.x,
      z: building.z,
      width: building.width,
      depth: building.depth,
      access_node: 3,
      suitability: 1,
      protected: false,
      reservation: null,
    }));

    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(world);
    const state = director as unknown as {
      selectedAnchor: { id: string; label: string } | null;
      desiredPosition: THREE.Vector3;
    };
    expect(state.selectedAnchor?.id).toBe('district:0:0');
    expect(state.selectedAnchor?.label).toBe('Neighborhood street');
    // The target is the midpoint of the open local road, not the homes' (22,60) centroid.
    expect(director.target.x).toBeCloseTo(0);
    expect(director.target.z).toBeCloseTo(60);
    expect(director.target.distanceTo(new THREE.Vector3(22, 0, 60))).toBeGreaterThan(20);

    const obstacleSet = world.buildings.map((building) => ({
      center: { x: building.x, z: building.z },
      rotation: building.rotation,
      width: building.width + 2,
      depth: building.depth + 2,
      minY: 0,
      maxY: 0.58 + building.floors * 3.25 + 4.05,
    }));
    const preferredCamera = director.target
      .clone()
      .add(
        new THREE.Vector3(0.65, 0.28, 0.7)
          .normalize()
          .multiplyScalar(state.desiredPosition.distanceTo(director.target)),
      );
    expect(state.desiredPosition.distanceTo(preferredCamera)).toBeGreaterThan(5);
    expect(
      cameraPathClearance(state.desiredPosition, state.desiredPosition, obstacleSet, {
        terrainY: 0,
        safetyMarginM: 1.5,
      }).collides,
    ).toBe(false);
    expect(
      cameraPathClearance(
        { x: director.target.x, y: Math.max(director.target.y, 1), z: director.target.z },
        state.desiredPosition,
        obstacleSet,
        { terrainY: 0, safetyMarginM: 0.3 },
      ).collides,
    ).toBe(false);

    const heldCamera = state.desiredPosition.clone();
    const heldTarget = director.target.clone();
    for (let frame = 0; frame < 12; frame++) director.update(1 / 60, frame / 60);
    const next = { ...world, tick: 1, revision: 1 };
    director.world(next);
    for (let frame = 0; frame < 12; frame++) director.update(1 / 60, 1 + frame / 60);
    expect(state.desiredPosition.distanceTo(heldCamera)).toBeLessThan(0.001);
    expect(director.target.distanceTo(heldTarget)).toBeLessThan(0.001);

    orbitMocks.instances.at(-1)?.start();
    expect(director.mode).toBe('explore');
    expect(director.camera.position.distanceTo(heldCamera)).toBeLessThan(0.001);
    expect(director.target.distanceTo(heldTarget)).toBeLessThan(0.001);
    director.update(0.1, 2);
    expect(director.camera.position.distanceTo(heldCamera)).toBeLessThan(0.001);
    expect(director.target.distanceTo(heldTarget)).toBeLessThan(0.001);
    director.dispose();
  });

  it("leaves a closed street shot and preserves the user's camera in Explore", () => {
    const world = makeWorld();
    world.terrain.river = [];
    world.nodes = [
      { id: 3, x: 0, z: 0, kind: 'street' },
      { id: 4, x: 0, z: 120, kind: 'street' },
    ];
    world.roads = [
      {
        id: 20,
        from: 3,
        to: 4,
        kind: 'local',
        stage: 'open',
        progress: 1,
        connected: true,
        length: 120,
        lanes: 1,
      },
    ];
    const localHomes = [
      makeBuilding(101, 20, 50),
      makeBuilding(102, 24, 60),
      makeBuilding(103, 22, 70),
    ].map((building) => ({ ...building, anchor: false }));
    world.buildings = localHomes;
    world.parcels = localHomes.map((building) => ({
      id: building.parcel_id,
      x: building.x,
      z: building.z,
      width: building.width,
      depth: building.depth,
      access_node: 3,
      suitability: 1,
      protected: false,
      reservation: null,
    }));

    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(world);
    const state = director as unknown as {
      selectedAnchor: { id: string } | null;
      desiredTarget: THREE.Vector3;
    };
    expect(state.selectedAnchor?.id).toBe('district:0:0');
    expect(director.target.x).toBeCloseTo(0);
    expect(director.target.z).toBeCloseTo(60);

    const closed = {
      ...world,
      tick: 1,
      revision: 1,
      roads: world.roads.map((road) => ({ ...road, stage: 'closed' })),
    };
    director.world(closed);
    // There is no replacement street candidate, so the fallback is the surviving district itself.
    expect(state.desiredTarget.x).toBeCloseTo(22);
    expect(state.desiredTarget.z).toBeCloseTo(60);
    for (let frame = 0; frame < 150; frame++) director.update(0.1, frame * 0.1);
    expect(director.target.distanceTo(new THREE.Vector3(0, 1, 60))).toBeGreaterThan(5);

    orbitMocks.instances.at(-1)?.start();
    expect(director.mode).toBe('explore');
    const manualPosition = director.camera.position.clone();
    const manualTarget = director.target.clone();
    const later = { ...closed, tick: 2, revision: 2 };
    director.world(later);
    for (let frame = 0; frame < 30; frame++) director.update(0.1, 20 + frame * 0.1);
    expect(director.camera.position.distanceTo(manualPosition)).toBeLessThan(0.001);
    expect(director.target.distanceTo(manualTarget)).toBeLessThan(0.001);
    director.dispose();
  });

  it('changes focal length in place and takes persistent Explore control', () => {
    const camera = new THREE.PerspectiveCamera(37.85, 1, 0.1, 2_000);
    const director = new CameraDirector(camera, makeElement());
    const modeChanges: string[] = [];
    director.onModeChange = (mode) => modeChanges.push(mode);
    director.world(makeWorld());
    director.update(0.1, 0);
    const position = camera.position.clone();
    const target = director.target.clone();

    director.setLens('detail');
    expect(director.lens).toBe('detail');
    expect(director.mode).toBe('explore');
    expect(modeChanges).toContain('explore');
    expect(camera.fov).toBeCloseTo(verticalFovDegrees(85), 10);
    expect(camera.position.distanceTo(position)).toBe(0);
    expect(director.target.distanceTo(target)).toBe(0);

    // Watch transitions are canceled, so the director does not undo the chosen framing.
    director.update(0.1, 0.1);
    expect(camera.position.distanceTo(position)).toBe(0);
    expect(director.target.distanceTo(target)).toBe(0);
    director.dispose();
  });

  it('does not lift a clear camera when the lens changes near a building', () => {
    const camera = new THREE.PerspectiveCamera(37.85, 1, 0.1, 2_000);
    const director = new CameraDirector(camera, makeElement());
    const world = makeWorld();
    const nearby = makeBuilding(41, 0, 0);
    nearby.anchor = false;
    world.buildings = [nearby];
    director.world(world);
    camera.position.set(180, 110, 190);
    director.target.set(0, 0, 0);
    director.setMode('explore');
    const clearPose = camera.position.clone();

    director.setLens('detail');

    expect(camera.position.distanceTo(clearPose)).toBeLessThan(1e-8);
    expect(director.mode).toBe('explore');
    director.dispose();
  });

  it("can apply a saved lens without changing the user's camera mode", () => {
    const camera = new THREE.PerspectiveCamera(37.85, 1, 0.1, 2_000);
    const director = new CameraDirector(camera, makeElement());
    const position = camera.position.clone();
    director.setLens('neighborhood', false);
    expect(director.mode).toBe('watch');
    expect(camera.fov).toBeCloseTo(verticalFovDegrees(50), 10);
    expect(camera.position.distanceTo(position)).toBe(0);
    director.dispose();
  });

  it('sets the same held waterfront review pose after different camera histories', () => {
    const world = makeWorld();
    world.terrain.river = [
      { x: 0, z: 0 },
      { x: 20, z: 0 },
      { x: 20, z: 40 },
    ];
    const create = () =>
      new CameraDirector(new THREE.PerspectiveCamera(42, 1, 0.1, 2_000), makeElement());
    const first = create();
    const second = create();
    first.world(world);
    second.world(world);
    second.setLens('detail');
    second.setMode('overview');
    second.update(1, 3);

    expect(first.reviewWaterfront()).toBe(true);
    expect(second.reviewWaterfront()).toBe(true);
    expect(first.target.toArray()).toEqual([20, 0, 10]);
    expect(first.camera.position.toArray()).toEqual([130, 145, 190]);
    expect(second.camera.position.distanceTo(first.camera.position)).toBeLessThan(1e-8);
    expect(second.target.distanceTo(first.target)).toBeLessThan(1e-8);
    expect(second.lens).toBe('detail');
    expect(first.mode).toBe('explore');
    expect(first.label).toBe('Explore');

    const pose = first.camera.position.clone();
    const target = first.target.clone();
    for (let i = 0; i < 20; i++) first.update(0.1, i * 0.1);
    expect(first.camera.position.distanceTo(pose)).toBeLessThan(1e-8);
    expect(first.target.distanceTo(target)).toBeLessThan(1e-8);
    first.dispose();
    second.dispose();
  });

  it('sets and holds the repeatable low waterfront pose without changing the lens', () => {
    const world = makeWorld();
    world.terrain.river = [
      { x: 0, z: 0 },
      { x: 20, z: 0 },
      { x: 20, z: 40 },
    ];
    const create = () =>
      new CameraDirector(new THREE.PerspectiveCamera(42, 1, 0.1, 2_000), makeElement());
    const first = create();
    const second = create();
    first.world(world);
    second.world(world);
    first.setMode('overview');
    first.update(0.7, 2);
    second.setLens('detail');
    second.setMode('watch');
    second.update(1.3, 4);

    expect(first.reviewWaterfront('low')).toBe(true);
    expect(second.reviewWaterfront('low')).toBe(true);
    expect(first.target.toArray()).toEqual([20, 0, 10]);
    expect(first.camera.position.toArray()).toEqual([150, 55, 225]);
    expect(second.camera.position.distanceTo(first.camera.position)).toBeLessThan(1e-8);
    expect(second.target.distanceTo(first.target)).toBeLessThan(1e-8);
    expect(second.lens).toBe('detail');
    const pose = first.camera.position.clone();
    for (let i = 0; i < 20; i++) first.update(0.1, i * 0.1);
    expect(first.camera.position.distanceTo(pose)).toBeLessThan(1e-8);
    first.dispose();
    second.dispose();
  });

  it.each([
    ['overhead', 130, 145, 180],
    ['low', 150, 55, 215],
  ] as const)(
    'recovers the %s waterfront camera above an obstructing building',
    (preset, x, y, z) => {
      const world = makeWorld();
      world.terrain.river = [
        { x: 0, z: 0 },
        { x: 40, z: 0 },
      ];
      const tower = makeBuilding(90, x, z);
      tower.anchor = false;
      tower.floors = 60;
      tower.kind = 'office';
      world.buildings = [tower];
      const director = new CameraDirector(
        new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
        makeElement(),
      );
      director.world(world);
      // A final ordinary controls update must not undo the guarded recovery.
      orbitMocks.instances.at(-1)!.bendOnUpdate = (camera) => {
        if (director.controls.maxDistance < 1_200) camera.position.y = 0;
      };
      expect(director.reviewWaterfront(preset)).toBe(true);
      expect(director.camera.position.y).toBeGreaterThan(y);
      expect(
        cameraPathClearance(
          director.camera.position,
          director.camera.position,
          (director as unknown as { cameraObstacles: Parameters<typeof cameraPathClearance>[2] })
            .cameraObstacles,
          { terrainY: 0, safetyMarginM: 1.5 },
        ).collides,
      ).toBe(false);
      director.dispose();
    },
  );

  it('rejects waterfront review without a usable river or after disposal', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    expect(director.reviewWaterfront()).toBe(false);
    const world = makeWorld();
    world.terrain.river = [
      { x: 1, z: 1 },
      { x: 1, z: 1 },
    ];
    director.world(world);
    expect(director.reviewWaterfront()).toBe(false);
    world.terrain.river = [
      { x: 1, z: 1 },
      { x: 2, z: 1 },
    ];
    director.dispose();
    expect(director.reviewWaterfront()).toBe(false);
  });

  it('reframes a held founder when neighboring growth blocks its view', () => {
    const world = makeWorld();
    world.buildings = [makeBuilding(10, 0, 0)];
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1.5, 0.1, 2000),
      makeElement(),
    );
    director.world(world);
    const initial = director.camera.position.clone();
    const next = {
      ...world,
      tick: 1,
      buildings: [
        ...world.buildings,
        { ...makeBuilding(11, 30, 30), anchor: false, floors: 16, width: 28, depth: 28 },
      ],
    };
    director.world(next);
    const pose = director as unknown as { desiredPosition: THREE.Vector3 };
    expect(pose.desiredPosition.distanceTo(initial)).toBeGreaterThan(5);
    expect(director.label).toBe('Home 10');
    director.dispose();
  });

  it('separates sightline footprints from travel safety aprons', () => {
    const world = makeWorld();
    world.buildings = [
      { ...makeBuilding(113, -31.894, -207.1536), width: 9.75268, depth: 10.535443 },
      {
        ...makeBuilding(731, -43.07999, -208.82758),
        anchor: false,
        width: 14.86827,
        depth: 11.691514,
        floors: 3,
        rotation: -1.4944199,
      },
    ];
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1.5, 0.1, 2000),
      makeElement(),
    );
    director.world(world);
    const bounds = director as unknown as {
      cameraObstacles: import('../web/src/camera/clearance').CameraObstacleBox[];
      sightObstacles: import('../web/src/camera/clearance').CameraObstacleBox[];
    };
    const probe = { x: -34.819804, y: 2, z: -201.8858785 };
    expect(
      cameraPathClearance(probe, probe, [bounds.cameraObstacles[1]], { safetyMarginM: 0.3 })
        .collides,
    ).toBe(true);
    expect(
      cameraPathClearance(probe, probe, [bounds.sightObstacles[1]], { safetyMarginM: 0.3 })
        .collides,
    ).toBe(false);
    expect(bounds.cameraObstacles[1].width).toBeCloseTo(14.86827 + 2.4);
    expect(bounds.cameraObstacles[1].depth).toBeCloseTo(11.691514 + 5.2);
    director.dispose();
  });

  it('keeps a founder visible when a tall neighbor blocks the default Watch direction', () => {
    const world = makeWorld();
    world.buildings = [
      makeBuilding(10, 0, 0),
      { ...makeBuilding(11, 30, 30), anchor: false, floors: 16, width: 28, depth: 28 },
    ];
    const camera = new THREE.PerspectiveCamera(42, 1.5, 0.1, 2000);
    const director = new CameraDirector(camera, makeElement());
    director.world(world);
    expect(director.label).toBe('Home 10');
    const neighbor = {
      center: { x: 30, z: 30 },
      width: 28,
      depth: 28,
      rotation: 0,
      minY: 0,
      maxY: 60,
    };
    const oldPosition = new THREE.Vector3(0.65, 0.28, 0.7).normalize().multiplyScalar(78);
    expect(
      cameraPathClearance({ x: 0, y: 3.25, z: 0 }, oldPosition, [neighbor], {
        terrainY: 0,
        safetyMarginM: 0.3,
      }).collides,
    ).toBe(true);
    expect(
      cameraPathClearance({ x: 0, y: 3.25, z: 0 }, camera.position, [neighbor], {
        terrainY: 0,
        safetyMarginM: 0.3,
      }).collides,
    ).toBe(false);
    director.dispose();
  });

  it('opens the actual seed 87231 WASM world directly on a readable founding shop', () => {
    const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
    initSync({ module: wasm });
    const engine = new CityEngine(87_231);
    try {
      const world = JSON.parse(engine.snapshot()) as World;
      const founder = world.buildings.find((building) => building.name === 'Kissa Aoba');
      expect(founder).toBeDefined();
      expect(founder?.anchor).toBe(true);

      const camera = new THREE.PerspectiveCamera(42, 817 / 753, 0.1, 2_000);
      const director = new CameraDirector(camera, makeElement());
      director.world(world);
      expect(director.label).toBe('Kissa Aoba');
      expect(director.target.distanceTo(new THREE.Vector3(founder!.x, 0, founder!.z))).toBeLessThan(
        5,
      );
      expect(camera.position.distanceTo(director.target)).toBeLessThan(135);

      camera.lookAt(director.target);
      camera.updateMatrixWorld(true);
      const corners: THREE.Vector3[] = [];
      for (const x of [founder!.x - founder!.width / 2, founder!.x + founder!.width / 2]) {
        for (const z of [founder!.z - founder!.depth / 2, founder!.z + founder!.depth / 2]) {
          for (const y of [0, founder!.floors * 3.25])
            corners.push(new THREE.Vector3(x, y, z).project(camera));
        }
      }
      const widthNdc = Math.max(...corners.map((p) => p.x)) - Math.min(...corners.map((p) => p.x));
      const heightNdc = Math.max(...corners.map((p) => p.y)) - Math.min(...corners.map((p) => p.y));
      expect(widthNdc).toBeGreaterThan(0.16);
      expect(heightNdc).toBeGreaterThan(0.08);
      expect(corners.every((p) => Math.abs(p.x) < 0.94 && Math.abs(p.y) < 0.94)).toBe(true);
      director.dispose();
    } finally {
      engine.free();
    }
  });

  it('hands control to the viewer and does not reclaim it until Watch is chosen', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    world.buildings = [makeBuilding(10, -30, 12), makeBuilding(11, 100, 90)];
    director.world(world);
    director.update(0.1, 0.1);

    director.camera.position.x += 18;
    director.target.x += 18;
    orbitMocks.instances[0].start();
    expect(director.mode).toBe('explore');
    const manualTarget = director.target.clone();
    for (let i = 0; i < 80; i++) director.update(0.1, i * 0.1);
    expect(director.target.distanceTo(manualTarget)).toBeLessThan(0.001);

    director.setMode('watch');
    expect(director.mode).toBe('watch');
    director.dispose();
    expect(orbitMocks.instances[0].disposed).toBe(true);
  });

  it('rejects a manual sweep through a rotated tower and retains the last clear pose', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const emptyWorld = makeWorld();
    director.world(emptyWorld);
    director.setMode('explore');
    director.camera.position.set(-60, 50, 0);
    director.target.set(0, 0, 0);
    director.update(1 / 60, 1 / 60);

    const city = makeWorld();
    city.tick = 1;
    const tower = makeBuilding(22, 0, 0);
    tower.anchor = false;
    tower.width = 20;
    tower.depth = 10;
    tower.floors = 20;
    tower.kind = 'office';
    tower.rotation = Math.PI / 4;
    city.buildings = [tower];
    director.world(city);
    const savedBuildings = JSON.stringify(city.buildings);

    director.camera.position.set(60, 50, 0);
    director.update(1 / 60, 2 / 60);

    expect(director.camera.position.x).toBeCloseTo(-60, 8);
    expect(director.camera.position.y).toBeCloseTo(50, 8);
    expect(director.target.toArray()).toEqual([0, 0, 0]);
    expect(director.mode).toBe('explore');
    expect(JSON.stringify(city.buildings)).toBe(savedBuildings);
    director.dispose();
  });

  it('keeps clear manual travel, but restores the prior pose when a path crosses the ground plane', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.setMode('explore');
    const target = director.target.clone();

    director.camera.position.set(260, 180, 300);
    director.update(1 / 60, 1 / 60);
    expect(director.camera.position.distanceTo(new THREE.Vector3(260, 180, 300))).toBeLessThan(
      1e-8,
    );
    expect(director.target.distanceTo(target)).toBeLessThan(1e-8);

    const lastClear = director.camera.position.clone();
    const lastTarget = director.target.clone();
    director.target.y = -30;
    director.camera.position.y = -10;
    director.update(1 / 60, 2 / 60);
    expect(director.camera.position.distanceTo(lastClear)).toBeLessThan(1e-8);
    expect(director.target.distanceTo(lastTarget)).toBeLessThan(1e-8);
    expect(director.mode).toBe('explore');
    director.dispose();
  });

  it('lifts a camera already inside a newly loaded building once, without changing XZ or target', () => {
    const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 2_000);
    const director = new CameraDirector(camera, makeElement());
    camera.position.set(0, 12, 0);
    director.target.set(0, 0, 10);
    const target = director.target.clone();
    const world = makeWorld();
    const building = makeBuilding(31, 0, 0);
    building.anchor = false;
    building.floors = 20;
    building.stage = 'construction';
    world.terrain.river = [];
    world.roads = [];
    world.buildings = [building];

    director.world(world);

    expect(camera.position.x).toBe(0);
    expect(camera.position.z).toBe(0);
    expect(camera.position.y).toBeGreaterThan(60);
    expect(director.target.distanceTo(target)).toBe(0);
    expect(director.mode).toBe('watch');

    // Recovery includes an epsilon above the collision plane, allowing the
    // viewer to move out of the building without the next sweep being rejected.
    const savedWorld = JSON.stringify(world);
    director.world({ ...world, tick: 1 });
    expect(JSON.stringify(world)).toBe(savedWorld);
    director.setMode('explore');
    camera.position.x = 60;
    director.update(1 / 60, 1 / 60);
    expect(camera.position.x).toBeCloseTo(60, 8);
    expect(director.mode).toBe('explore');
    expect(JSON.stringify(world)).toBe(savedWorld);
    director.dispose();
  });

  it('eases every ordinary automatic camera segment around a tower without corner cutting', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    world.terrain.river = [];
    const tower = makeBuilding(90, 0, 0);
    tower.anchor = false;
    tower.kind = 'office';
    tower.floors = 20;
    tower.width = 24;
    tower.depth = 14;
    world.buildings = [tower];
    director.world(world);
    director.camera.position.set(-80, 60, 0);
    director.target.set(-80, 0, 0);
    const internals = director as unknown as {
      desiredPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
      automaticRoute: { lifted: boolean } | null;
      applyPose(dt: number, seconds: number): void;
      cameraObstacles: Parameters<typeof cameraPathClearance>[2];
    };
    internals.desiredPosition.set(80, 60, 0);
    internals.desiredTarget.set(80, 0, 0);

    let previous = director.camera.position.clone();
    for (let frame = 0; frame < 320; frame++) {
      internals.applyPose(0.1, 0.5);
      const current = director.camera.position.clone();
      expect(
        cameraPathClearance(
          { x: previous.x, y: previous.y, z: previous.z },
          { x: current.x, y: current.y, z: current.z },
          internals.cameraObstacles,
          { terrainY: 0, safetyMarginM: 1.5 },
        ).collides,
      ).toBe(false);
      previous = current;
      if (!internals.automaticRoute) break;
    }
    expect(director.camera.position.distanceTo(new THREE.Vector3(80, 60, 0))).toBeLessThan(0.36);
    expect(director.target.distanceTo(new THREE.Vector3(80, 0, 0))).toBeLessThan(0.01);
    director.dispose();
  });

  it('clears a lifted route on manual interruption and keeps Explore', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    world.terrain.river = [];
    const tower = makeBuilding(91, 0, 0);
    tower.anchor = false;
    tower.kind = 'office';
    tower.floors = 20;
    tower.width = 24;
    tower.depth = 14;
    world.buildings = [tower];
    director.world(world);
    director.camera.position.set(-80, 60, 0);
    director.target.set(-80, 0, 0);
    const internals = director as unknown as {
      desiredPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
      automaticRoute: { lifted: boolean } | null;
      applyPose(dt: number, seconds: number): void;
    };
    internals.desiredPosition.set(80, 60, 0);
    internals.desiredTarget.set(80, 0, 0);
    internals.applyPose(0.1, 0.5);
    expect(internals.automaticRoute?.lifted).toBe(true);

    orbitMocks.instances.at(-1)!.start();

    expect(internals.automaticRoute).toBeNull();
    expect(director.mode).toBe('explore');
    director.dispose();
  });

  it('rejects a collision introduced by OrbitControls after the planned point was clear', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    world.terrain.river = [];
    const tower = makeBuilding(94, 0, 0);
    tower.anchor = false;
    tower.kind = 'office';
    tower.floors = 20;
    tower.width = 24;
    tower.depth = 14;
    world.buildings = [tower];
    director.world(world);
    director.camera.position.set(-80, 60, 0);
    director.target.set(-80, 0, 0);
    const internals = director as unknown as {
      desiredPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
      automaticRoute: unknown;
      applyPose(dt: number, seconds: number): void;
    };
    internals.desiredPosition.set(80, 60, 0);
    internals.desiredTarget.set(80, 0, 0);
    const clearPose = director.camera.position.clone();
    let bendOnce = true;
    orbitMocks.instances.at(-1)!.bendOnUpdate = (camera) => {
      if (!bendOnce) return;
      bendOnce = false;
      camera.position.set(0, 50, 0); // Inside the authored tower envelope.
    };

    internals.applyPose(0.1, 0.5);

    expect(director.camera.position.distanceTo(clearPose)).toBeLessThan(1e-8);
    expect(director.target.toArray()).toEqual([-80, 0, 0]);
    expect(internals.automaticRoute).toBeNull();
    director.dispose();
  });

  it('guards settled automatic frames and restores the prior camera orientation on rollback', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    world.terrain.river = [];
    const tower = makeBuilding(95, 0, 0);
    tower.anchor = false;
    tower.kind = 'office';
    tower.floors = 20;
    tower.width = 24;
    tower.depth = 14;
    world.buildings = [tower];
    director.world(world);
    director.camera.position.set(-80, 60, 0);
    director.target.set(-80, 0, 0);
    const internals = director as unknown as {
      desiredPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
      automaticRoute: unknown;
      applyPose(dt: number, seconds: number): void;
    };
    internals.desiredPosition.set(80, 60, 0);
    internals.desiredTarget.set(80, 0, 0);
    for (let frame = 0; frame < 320; frame++) {
      internals.applyPose(0.1, 0.5);
      if (!internals.automaticRoute) break;
    }
    const clearPosition = director.camera.position.clone();
    const clearTarget = director.target.clone();
    const clearOrientation = director.camera.quaternion.clone();
    expect(clearPosition.distanceTo(new THREE.Vector3(80, 60, 0))).toBeLessThan(0.36);

    orbitMocks.instances.at(-1)!.bendOnUpdate = (camera) => {
      camera.position.set(0, 50, 0);
      camera.lookAt(0, 0, 0);
    };
    director.update(1 / 60, 1);

    expect(director.camera.position.distanceTo(clearPosition)).toBeLessThan(1e-8);
    expect(director.target.distanceTo(clearTarget)).toBeLessThan(1e-8);
    expect(director.camera.quaternion.angleTo(clearOrientation)).toBeLessThan(1e-8);
    expect(internals.automaticRoute).toBeNull();
    director.dispose();
  });

  it('uses the ordinary one-segment smoothing path when no obstacle blocks it', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.camera.position.set(-100, 90, 0);
    director.target.set(-100, 0, 0);
    const internals = director as unknown as {
      desiredPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
      automaticRoute: { lifted: boolean; waypoints: THREE.Vector3[] } | null;
      applyPose(dt: number, seconds: number): void;
    };
    internals.desiredPosition.set(-30, 90, 0);
    internals.desiredTarget.set(-30, 0, 0);

    internals.applyPose(0.1, 5.5);

    expect(internals.automaticRoute?.lifted).toBe(false);
    expect(internals.automaticRoute?.waypoints).toHaveLength(1);
    expect(director.camera.position.x).toBeGreaterThan(-100);
    expect(director.camera.position.x).toBeLessThan(-30);
    director.dispose();
  });

  it('uses an explicit static cut for reduced-motion automatic transitions', () => {
    vi.stubGlobal('window', {
      matchMedia: () => ({
        matches: true,
        addEventListener() {},
        removeEventListener() {},
        addListener() {},
        removeListener() {},
      }),
    });
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    world.terrain.river = [];
    const tower = makeBuilding(93, 0, 0);
    tower.anchor = false;
    tower.kind = 'office';
    tower.floors = 20;
    tower.width = 24;
    tower.depth = 14;
    world.buildings = [tower];
    director.world(world);
    director.camera.position.set(-80, 60, 0);
    director.target.set(-80, 0, 0);
    const internals = director as unknown as {
      desiredPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
      automaticRoute: unknown;
      applyPose(dt: number, seconds: number): void;
    };
    internals.desiredPosition.set(80, 60, 0);
    internals.desiredTarget.set(80, 0, 0);

    internals.applyPose(0.1, 0.5);

    expect(director.camera.position.toArray()).toEqual([80, 60, 0]);
    expect(director.target.toArray()).toEqual([80, 0, 0]);
    expect(internals.automaticRoute).toBeNull();
    director.dispose();
  });

  it('invalidates a planned route when a world snapshot changes obstacles', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    director.world(world);
    const internals = director as unknown as {
      desiredPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
      automaticRoute: unknown;
      applyPose(dt: number, seconds: number): void;
    };
    director.camera.position.set(-100, 90, 0);
    director.target.set(-100, 0, 0);
    internals.desiredPosition.set(100, 90, 0);
    internals.desiredTarget.set(100, 0, 0);
    internals.applyPose(0.1, 5.5);
    expect(internals.automaticRoute).not.toBeNull();

    const changed = makeWorld();
    changed.tick = 1;
    const newTower = makeBuilding(92, 20, 0);
    newTower.anchor = false;
    newTower.floors = 18;
    newTower.kind = 'office';
    changed.buildings = [newTower];
    director.world(changed);
    expect(internals.automaticRoute).toBeNull();
    director.dispose();
  });

  it('recovers a settled Watch pose when a new building encloses it, then resumes clear travel', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const first = makeWorld();
    first.terrain.river = [];
    director.world(first);
    director.camera.position.set(0, 42, 0);
    director.target.set(0, 0, 12);
    director.camera.lookAt(director.target);
    const preservedTarget = director.target.clone();

    const changed = {
      ...first,
      tick: 1,
      buildings: [
        {
          ...makeBuilding(111, 0, 0),
          kind: 'office' as const,
          anchor: false,
          floors: 20,
          width: 24,
          depth: 14,
        },
      ],
    };
    director.world(changed);

    const state = director as unknown as {
      cameraObstacles: Parameters<typeof cameraPathClearance>[2];
      automaticRoute: unknown;
    };
    const recovered = {
      x: director.camera.position.x,
      y: director.camera.position.y,
      z: director.camera.position.z,
    };
    expect(
      cameraPathClearance(recovered, recovered, state.cameraObstacles, {
        terrainY: 0,
        safetyMarginM: 1.5,
      }).collides,
    ).toBe(false);
    expect(director.camera.position.y).toBeGreaterThan(0.58 + 20 * 3.25 + 4.05 + 1.5);
    expect(director.target.distanceTo(preservedTarget)).toBeLessThan(1e-8);
    expect(director.mode).toBe('watch');

    const before = director.camera.position.clone();
    director.update(1 / 60, 1 / 60);
    expect(director.camera.position.distanceTo(before)).toBeGreaterThan(0);
    expect(state.automaticRoute).not.toBeNull();
    const direction = new THREE.Vector3();
    director.camera.getWorldDirection(direction);
    expect(
      direction.dot(director.target.clone().sub(director.camera.position).normalize()),
    ).toBeGreaterThan(0.999);
    director.dispose();
  });

  it('recovers a Follow pose on a changed obstacle revision without losing the subject', () => {
    const first = makeWorld();
    first.terrain.river = [];
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(first);
    director.follow(500);
    director.camera.position.set(15, 42, 0);
    director.target.set(15, 0, 6);
    director.camera.lookAt(director.target);

    const changed = {
      ...first,
      tick: 1,
      buildings: [
        {
          ...makeBuilding(112, 15, 0),
          kind: 'office' as const,
          anchor: false,
          floors: 20,
          width: 24,
          depth: 14,
        },
      ],
    };
    director.world(changed);
    const state = director as unknown as {
      cameraObstacles: Parameters<typeof cameraPathClearance>[2];
      automaticRoute: unknown;
    };
    const recovered = {
      x: director.camera.position.x,
      y: director.camera.position.y,
      z: director.camera.position.z,
    };
    expect(
      cameraPathClearance(recovered, recovered, state.cameraObstacles, {
        terrainY: 0,
        safetyMarginM: 1.5,
      }).collides,
    ).toBe(false);
    expect(director.label).toBe('Train');
    expect(director.mode).toBe('watch');

    const before = director.camera.position.clone();
    director.update(1 / 60, 1 / 60);
    expect(director.camera.position.distanceTo(before)).toBeGreaterThan(0);
    expect(state.automaticRoute).not.toBeNull();
    expect(director.label).toBe('Train');
    const position = {
      x: director.camera.position.x,
      y: director.camera.position.y,
      z: director.camera.position.z,
    };
    expect(
      cameraPathClearance(recovered, position, state.cameraObstacles, {
        terrainY: 0,
        safetyMarginM: 1.5,
      }).collides,
    ).toBe(false);
    director.dispose();
  });

  it('keeps following a moving subject finite while the destination changes', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.follow(500);
    for (let frame = 0; frame < 30; frame++) {
      const x = 20 + frame * 4;
      director.setPositions(new Float32Array([500, x, 10, 0.2, 2, -1]));
      director.update(1 / 60, frame / 60);
      expect(
        [...director.camera.position.toArray(), ...director.target.toArray()].every(
          Number.isFinite,
        ),
      ).toBe(true);
    }
    expect(director.mode).toBe('watch');
    director.dispose();
  });

  it('returns Watch toward the city after a regional car disappears through the follow grace period', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const world = makeWorld();
    world.buildings = [makeBuilding(10, 20, 12)];
    director.world(world);
    // ID 812 exists only in movement publications; the structural train 500
    // remains present and cannot provide a fallback for this regional car.
    director.setMode('explore');
    director.follow(812);
    director.setPositions(new Float32Array([812, 320, 280, 0, 1, 11.15]));
    director.update(0.1, 0.1);

    director.setPositions(new Float32Array());
    const state = director as unknown as { followId: number | null; desiredTarget: THREE.Vector3 };
    const heldTarget = state.desiredTarget.clone();
    // Exercise more than one second of the grace period with nonzero steps.
    for (let i = 0; i < 15; i++) director.update(0.1, 0.2 + i * 0.1);
    expect(state.desiredTarget.distanceTo(heldTarget)).toBeLessThan(0.001);

    // Continue past the four-second grace and let the eased Watch return finish.
    for (let i = 0; i < 85; i++) director.update(0.1, 1.7 + i * 0.1);

    expect(state.followId).toBeNull();
    expect(director.mode).toBe('watch');
    expect(Math.hypot(director.target.x - 20, director.target.z - 12)).toBeLessThan(250);
    director.dispose();
  });

  it('resumes a moving follow when the subject reappears within its grace period', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.setMode('explore');
    director.follow(812);
    director.setPositions(new Float32Array([812, 30, 18, 0, 1, 11.15]));
    director.update(0.1, 0.1);
    director.setPositions(new Float32Array());
    for (let i = 0; i < 15; i++) director.update(0.1, 0.2 + i * 0.1);

    director.setPositions(new Float32Array([812, 76, -24, Math.PI / 2, 1, 11.15]));
    director.update(0.1, 1.7);

    const state = director as unknown as { followId: number | null; followPosition: THREE.Vector3 };
    expect(state.followId).toBe(812);
    expect(state.followPosition.x).toBe(76);
    expect(state.followPosition.z).toBe(-24);
    director.dispose();
  });

  it('does not snap to the founder when the viewer took manual control before the first snapshot', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const cameraBeforeWorld = director.camera.position.clone();
    const targetBeforeWorld = director.target.clone();
    orbitMocks.instances[0].start();

    const world = makeWorld();
    world.buildings = [makeBuilding(10, -30, 12)];
    world.buildings[0].anchor = true;
    director.world(world);

    expect(director.mode).toBe('explore');
    expect(director.camera.position.distanceTo(cameraBeforeWorld)).toBeLessThan(0.001);
    expect(director.target.distanceTo(targetBeforeWorld)).toBeLessThan(0.001);
    director.dispose();
  });

  it('keeps an explicit follow across world snapshots and releases a vanished subject smoothly', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    const first = makeWorld();
    director.world(first);
    director.follow(500);
    director.setPositions(new Float32Array([91, 80, -30, 0.2, 0, -1, 500, 42, 26, 0.1, 2, -1]));
    director.update(0.1, 0.1);
    expect(director.label).toBe('Train');

    const next = makeWorld();
    next.revision = 2;
    next.trains[0].progress = 28;
    director.world(next);
    expect(director.label).toBe('Train');
    director.update(0.1, 0.2);

    next.trains = [];
    next.revision++;
    director.world(next);
    director.setPositions(new Float32Array());
    for (let i = 0; i < 45; i++) director.update(0.1, 0.3 + i * 0.1);
    expect(director.label).not.toBe('Train');
    expect(director.target.x).toBeGreaterThan(-430);
    expect(director.target.x).toBeLessThan(430);
    director.dispose();
  });

  it('keeps the founding shop readable on desktop and narrow screens without fitting the river span', () => {
    vi.stubGlobal('window', {
      matchMedia: () => ({
        matches: true,
        addEventListener() {},
        removeEventListener() {},
        addListener() {},
        removeListener() {},
      }),
    });
    for (const aspect of [16 / 9, 0.68]) {
      const camera = new THREE.PerspectiveCamera(42, aspect, 0.1, 2_000);
      const director = new CameraDirector(camera, makeElement());
      const world = makeWorld();
      world.buildings = [makeBuilding(10, 116, 6)];
      world.buildings[0].anchor = true;
      // The real founding shop sits near the river. This stream is 55 m away,
      // while its full span is intentionally much larger than the opening frame.
      world.terrain.river = [
        { x: 61, z: -190 },
        { x: 61, z: 6 },
        { x: 61, z: 210 },
      ];
      director.world(world);
      director.update(0.1, 0.1);

      const founder = (
        director as unknown as {
          anchors: Array<{ id: string; x: number; z: number; frameRadius?: number }>;
        }
      ).anchors.find((anchor) => anchor.id === 'founder:10');
      expect(founder).toBeDefined();
      expect(Math.hypot(founder!.x - 116, founder!.z - 6)).toBeLessThan(5);
      expect(founder!.frameRadius).toBeLessThan(30);

      const distance = director.camera.position.distanceTo(director.target);
      expect(distance).toBeGreaterThanOrEqual(78 - 1e-9);
      expect(distance).toBeLessThan(135);
      camera.lookAt(director.target);
      camera.updateMatrixWorld(true);

      const projected = [] as THREE.Vector3[];
      for (const x of [107, 125])
        for (const z of [-1, 13])
          for (const y of [0, 6.5]) {
            projected.push(new THREE.Vector3(x, y, z).project(camera));
          }
      const bounds = {
        minX: Math.min(...projected.map((p) => p.x)),
        maxX: Math.max(...projected.map((p) => p.x)),
        minY: Math.min(...projected.map((p) => p.y)),
        maxY: Math.max(...projected.map((p) => p.y)),
      };
      expect(bounds.minX).toBeGreaterThan(-0.92);
      expect(bounds.maxX).toBeLessThan(0.92);
      expect(bounds.minY).toBeGreaterThan(-0.92);
      expect(bounds.maxY).toBeLessThan(0.92);
      expect(bounds.maxX - bounds.minX).toBeGreaterThan(0.12);
      expect(bounds.maxY - bounds.minY).toBeGreaterThan(0.08);

      const nearRiver = new THREE.Vector3(61, 0, 6).project(camera);
      expect(Math.abs(nearRiver.x)).toBeLessThan(1.15);
      expect(Math.abs(nearRiver.y)).toBeLessThan(1.15);
      director.dispose();
    }
  });

  it('frames a real tall-building cluster from its river side and keeps roofs inside the shot', () => {
    const camera = new THREE.PerspectiveCamera(42, 16 / 9, 0.1, 2_000);
    const director = new CameraDirector(camera, makeElement());
    const world = makeWorld();
    world.roads = [];
    world.terrain.river = [
      { x: 0, z: -80 },
      { x: 0, z: 0 },
      { x: 0, z: 80 },
    ];
    const tower = (id: number, x: number, z: number, floors: number, kind = 'office') => ({
      ...makeBuilding(id, x, z),
      anchor: false,
      floors,
      kind,
      width: 24,
      depth: 22,
    });
    world.buildings = [
      tower(10, 100, 0, 20),
      tower(11, 112, 10, 18),
      tower(12, 94, -12, 16),
      tower(13, 50, 2, 3),
    ];

    director.world(world);
    const internals = director as unknown as {
      anchors: Array<{
        id: string;
        label: string;
        x: number;
        z: number;
        targetY?: number;
        frameRadius?: number;
        cameraDirection?: THREE.Vector3;
      }>;
      anchorDistance: (anchor: {
        id: string;
        targetY?: number;
        frameRadius?: number;
        cameraDirection?: THREE.Vector3;
      }) => number;
      setAnchorPose: (
        anchor: { x: number; z: number; targetY?: number; cameraDirection?: THREE.Vector3 },
        distance: number,
      ) => void;
      applyPose: (dt: number, seconds: number) => void;
    };
    const skyline = internals.anchors.find((anchor) => anchor.id.startsWith('skyline:station:'));
    expect(skyline?.label).toBe('Station skyline');
    expect(skyline?.cameraDirection?.x).toBeLessThan(-0.8); // View in from the actual river side.
    expect(skyline?.targetY).toBeGreaterThan(20);
    expect(skyline?.targetY).toBeLessThan(40);
    expect(skyline?.frameRadius).toBeLessThan(100); // Local tower group, never full-district extent.

    internals.setAnchorPose(skyline!, internals.anchorDistance(skyline!));
    for (let frame = 0; frame < 160; frame++) internals.applyPose(0.1, 0.1);
    camera.lookAt(director.target);
    camera.updateMatrixWorld(true);
    expect(camera.position.x).toBeLessThan(skyline!.x);
    expect(camera.position.y).toBeGreaterThan(20 * 3.25); // Elevated above the focal roofline.

    const projected: THREE.Vector3[] = [];
    for (const building of world.buildings.slice(0, 3)) {
      for (const x of [building.x - building.width / 2, building.x + building.width / 2]) {
        for (const z of [building.z - building.depth / 2, building.z + building.depth / 2]) {
          for (const y of [0, building.floors * 3.25])
            projected.push(new THREE.Vector3(x, y, z).project(camera));
        }
      }
    }
    expect(projected.every((point) => Number.isFinite(point.x) && Number.isFinite(point.y))).toBe(
      true,
    );
    expect(Math.max(...projected.map((point) => point.y))).toBeLessThan(0.98); // Tallest roof remains visible.
    expect(Math.min(...projected.map((point) => point.y))).toBeGreaterThan(-0.98);
    const nearestFocalBuilding = Math.min(
      ...world.buildings
        .slice(0, 3)
        .map((building) =>
          Math.hypot(camera.position.x - building.x, camera.position.z - building.z),
        ),
    );
    expect(nearestFocalBuilding).toBeGreaterThan(70); // Camera stays outside the tower cluster.
    director.dispose();
  });

  it('fits the actual mature skyline roof and footprint corners across wide and narrow viewports', () => {
    const wasm = readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
    const save = readFileSync(
      new URL('./fixtures/access-joins/mature-world.json', import.meta.url),
      'utf8',
    );
    initSync({ module: wasm });
    const engine = CityEngine.restore(save);
    try {
      const world = JSON.parse(engine.snapshot()) as World;
      const tallest = Math.max(
        ...world.buildings
          .filter((building) => building.stage === 'open')
          .map((building) => building.floors),
      );
      expect(tallest).toBeGreaterThanOrEqual(5);
      for (const aspect of [817 / 753, 0.62]) {
        const camera = new THREE.PerspectiveCamera(42, aspect, 0.1, 2_000);
        const director = new CameraDirector(camera, makeElement());
        director.world(world);
        const internals = director as unknown as {
          anchors: Array<{
            id: string;
            label: string;
            x: number;
            z: number;
            targetY?: number;
            frameRadius?: number;
            framePoints?: THREE.Vector3[];
            cameraDirection?: THREE.Vector3;
          }>;
        };
        const skyline = internals.anchors.find((anchor) => anchor.id.startsWith('skyline:'));
        expect(skyline).toBeDefined();
        const station = world.nodes
          .filter((node) => node.kind === 'station')
          .sort(
            (a, b) =>
              Math.hypot(a.x - skyline!.x, a.z - skyline!.z) -
              Math.hypot(b.x - skyline!.x, b.z - skyline!.z),
          )[0];
        expect(station).toBeDefined();
        expect(Math.hypot(station!.x - skyline!.x, station!.z - skyline!.z)).toBeLessThanOrEqual(
          150,
        );
        expect(skyline!.label).toBe('Station skyline');
        expect(skyline!.id).toBe(`skyline:station:${station!.id}`);
        expect(skyline!.cameraDirection).toBeDefined();
        expect(skyline!.framePoints?.length).toBeGreaterThan(0);
        expect(skyline!.frameRadius).toBeLessThan(180);
        const nearbyTwentyFloor = world.buildings.find(
          (building) =>
            building.stage === 'open' &&
            building.floors === 20 &&
            Math.hypot(building.x - skyline!.x, building.z - skyline!.z) <= 90,
        );
        expect(nearbyTwentyFloor).toBeDefined();
        const expectedTwentyBounds = (
          director as unknown as { skylineBounds(buildings: World['buildings']): THREE.Vector3[] }
        ).skylineBounds([nearbyTwentyFloor!]);
        expect(
          expectedTwentyBounds.every((corner) =>
            skyline!.framePoints!.some((point) => point.distanceTo(corner) < 0.001),
          ),
        ).toBe(true);

        director.reviewSkyline();
        // Check the authored eased path as it enters the shot, not just its destination.
        let transitionExtrema = {
          minX: Infinity,
          maxX: -Infinity,
          minY: Infinity,
          maxY: -Infinity,
        };
        for (let frame = 0; frame < 300; frame++) {
          director.update(0.1, frame * 0.1);
          if (!((frame >= 48 && frame <= 60) || (frame >= 230 && frame <= 250))) continue;
          camera.lookAt(director.target);
          camera.updateMatrixWorld(true);
          const projected = skyline!.framePoints!.map((point) => point.clone().project(camera));
          transitionExtrema.minX = Math.min(
            transitionExtrema.minX,
            ...projected.map((point) => point.x),
          );
          transitionExtrema.maxX = Math.max(
            transitionExtrema.maxX,
            ...projected.map((point) => point.x),
          );
          transitionExtrema.minY = Math.min(
            transitionExtrema.minY,
            ...projected.map((point) => point.y),
          );
          transitionExtrema.maxY = Math.max(
            transitionExtrema.maxY,
            ...projected.map((point) => point.y),
          );
        }
        camera.lookAt(director.target);
        camera.updateMatrixWorld(true);
        const finalPoints = skyline!.framePoints!.map((point) => point.clone().project(camera));
        expect(
          finalPoints.every((point) => Number.isFinite(point.x) && Number.isFinite(point.y)),
        ).toBe(true);
        expect(Math.max(...finalPoints.map((point) => point.x))).toBeLessThan(0.88);
        expect(Math.min(...finalPoints.map((point) => point.x))).toBeGreaterThan(-0.88);
        expect(Math.max(...finalPoints.map((point) => point.y))).toBeLessThan(0.76);
        expect(Math.min(...finalPoints.map((point) => point.y))).toBeGreaterThan(-0.76);
        expect(transitionExtrema.maxY).toBeLessThan(0.92);
        expect(transitionExtrema.minY).toBeGreaterThan(-0.92);
        expect(camera.position.distanceTo(director.target)).toBeLessThan(600);
        director.dispose();
      }
    } finally {
      engine.free();
    }
  });

  it('keeps a near 32-floor foreground tower and 45-floor skyline roof within landscape and portrait safe areas', () => {
    vi.stubGlobal('window', {
      matchMedia: () => ({
        matches: true,
        addEventListener() {},
        removeEventListener() {},
        addListener() {},
        removeListener() {},
      }),
    });
    const world = makeWorld();
    world.roads = [];
    world.terrain.river = [
      { x: -40, z: -70 },
      { x: -40, z: 0 },
      { x: -40, z: 70 },
    ];
    const tower = (id: number, x: number, z: number, floors: number, rotation = 0) => ({
      ...makeBuilding(id, x, z),
      anchor: false,
      floors,
      kind: 'office',
      width: 30,
      depth: 26,
      rotation,
    });
    world.buildings = [
      tower(70, 34, 0, 45, 0.12),
      tower(71, 72, 4, 32, -0.08),
      tower(72, 95, -18, 20),
    ];
    for (const aspect of [16 / 9, 0.58]) {
      const camera = new THREE.PerspectiveCamera(42, aspect, 0.1, 2_000);
      const director = new CameraDirector(camera, makeElement());
      director.world(world);
      const state = director as unknown as {
        anchors: Array<{
          id: string;
          x: number;
          z: number;
          targetY?: number;
          cameraDirection?: THREE.Vector3;
          framePoints?: THREE.Vector3[];
        }>;
      };
      const skyline = state.anchors.find((anchor) => anchor.id.startsWith('skyline:'));
      expect(skyline).toBeDefined();
      expect(skyline!.framePoints!.some((point) => point.y > 45 * 3.25 + 3)).toBe(true); // Roof equipment included.
      expect(skyline!.framePoints!.some((point) => point.x > 72 && point.y > 32 * 3.25)).toBe(true); // Nearer 32F tower included.
      director.reviewSkyline();
      director.update(0.1, 0.1);
      camera.lookAt(director.target);
      camera.updateMatrixWorld(true);
      const projected = skyline!.framePoints!.map((point) => point.clone().project(camera));
      expect(projected.every((point) => Number.isFinite(point.x) && Number.isFinite(point.y))).toBe(
        true,
      );
      expect(Math.max(...projected.map((point) => point.x))).toBeLessThan(0.88);
      expect(Math.min(...projected.map((point) => point.x))).toBeGreaterThan(-0.88);
      expect(Math.max(...projected.map((point) => point.y))).toBeLessThan(0.76);
      expect(Math.min(...projected.map((point) => point.y))).toBeGreaterThan(-0.76);
      expect(camera.position.distanceTo(director.target)).toBeLessThan(600);
      director.dispose();
    }
  });

  it('exposes an explicit eased skyline review that leaves world state intact and yields to manual control', () => {
    const world = makeWorld();
    world.roads = [];
    world.terrain.river = [
      { x: -20, z: -80 },
      { x: -20, z: 0 },
      { x: -20, z: 80 },
    ];
    world.buildings = [
      { ...makeBuilding(31, 80, 0), anchor: false, floors: 16, kind: 'office' },
      { ...makeBuilding(32, 88, 12), anchor: false, floors: 14, kind: 'office' },
      { ...makeBuilding(33, 30, 0), anchor: false, floors: 2, kind: 'shop' },
    ];
    const original = JSON.stringify(world);
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1.5, 0.1, 2_000),
      makeElement(),
    );
    director.world(world);
    director.setMode('explore');
    // Selection may already open on the skyline; test easing from a distinct safe pose.
    expect(director.reviewPose({ x: -120, y: 150, z: 100 }, { x: 0, y: 0, z: 0 })).toBe(true);
    const modeChanges: string[] = [];
    director.onModeChange = (mode) => modeChanges.push(mode);
    const before = director.target.clone();

    expect(director.reviewSkyline()).toBe(true);
    expect(director.mode).toBe('watch');
    expect(modeChanges).toEqual(['watch']);
    expect(JSON.stringify(world)).toBe(original);
    director.update(0.1, 0.1);
    expect(director.target.distanceTo(before)).toBeGreaterThan(0);

    const current = director.target.clone();
    orbitMocks.instances.at(-1)!.start();
    expect(director.mode).toBe('explore');
    for (let i = 0; i < 20; i++) director.update(0.1, 0.2 + i * 0.1);
    expect(director.target.distanceTo(current)).toBeLessThan(0.001);
    expect(JSON.stringify(world)).toBe(original);
    director.dispose();
  });

  it('keeps neighborhood scenes close and sizes them to their developed footprint', () => {
    vi.stubGlobal('window', {
      matchMedia: () => ({
        matches: true,
        addEventListener() {},
        removeEventListener() {},
        addListener() {},
        removeListener() {},
      }),
    });
    const world = makeWorld();
    world.roads = [];
    world.buildings = [
      makeBuilding(1, 0, 0),
      makeBuilding(2, 65, 30),
      makeBuilding(3, 110, 110),
    ].map((building) => ({ ...building, anchor: false }));
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 817 / 753, 0.1, 2_000),
      makeElement(),
    );
    director.world(world);
    director.update(0.1, 0.1);

    const distance = director.camera.position.distanceTo(director.target);
    expect(director.label).toBe('Neighborhood street');
    expect(distance).toBeGreaterThanOrEqual(160);
    expect(distance).toBeLessThanOrEqual(200);
    director.dispose();
  });

  it('uses a lower Watch horizon while keeping Overview at the higher survey angle', () => {
    vi.stubGlobal('window', {
      matchMedia: () => ({
        matches: true,
        addEventListener() {},
        removeEventListener() {},
        addListener() {},
        removeListener() {},
      }),
    });
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1.4, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.update(0.1, 0.1);
    const watchElevation = director.camera.position.clone().sub(director.target).normalize().y;
    expect(watchElevation).toBeCloseTo(0.28, 2);
    expect(director.controls.maxPolarAngle).toBeGreaterThan(Math.acos(0.28));

    director.setMode('overview');
    director.update(0.1, 0.2);
    const overviewElevation = director.camera.position.clone().sub(director.target).normalize().y;
    expect(overviewElevation).toBeCloseTo(0.4, 1);
    expect(overviewElevation).toBeGreaterThan(watchElevation);
    director.dispose();
  });

  it.each([
    { name: 'east', heading: 0, expectedX: 86, expectedZ: 90 },
    { name: 'north', heading: Math.PI / 2, expectedX: 80, expectedZ: 96 },
  ])('uses engine heading convention when following $name', ({ heading, expectedX, expectedZ }) => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.follow(500);
    director.setPositions(new Float32Array([500, 80, 90, heading, 0, -1]));
    for (let i = 0; i < 45; i++) director.update(0.1, i * 0.1);

    expect(director.target.x).toBeCloseTo(expectedX, 0);
    expect(director.target.z).toBeCloseTo(expectedZ, 0);
    director.dispose();
  });

  it('uses an elevated movement surface as the follow target height', () => {
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(makeWorld());
    director.follow(500);
    const state = director as unknown as {
      followPosition: THREE.Vector3;
      desiredTarget: THREE.Vector3;
    };

    director.setPositions(new Float32Array([500, 80, 90, 0, 0, -1]));
    director.update(0.1, 0.1);
    expect(state.desiredTarget.y).toBe(0);
    director.setPositions(new Float32Array([500, 80, 90, 0, 0, 8]));
    director.update(0.1, 0.2);
    expect(state.followPosition.y).toBe(8);
    expect(state.desiredTarget.y).toBe(8);
    director.dispose();
  });

  it('uses snapshot direction for look-ahead without moving the train center at a reversal', () => {
    const centers: number[] = [];
    for (const direction of [1, -1]) {
      const world = makeWorld();
      world.trains[0].direction = direction;
      const director = new CameraDirector(
        new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
        makeElement(),
      );
      director.world(world);
      director.follow(500);

      const state = director as unknown as { desiredTarget: THREE.Vector3; followHeading: number };
      centers.push(state.desiredTarget.x - Math.cos(state.followHeading) * 10);
      expect(state.desiredTarget.x).toBeCloseTo(direction > 0 ? 25 : 5, 0);
      director.dispose();
    }

    // Engine progress remains 15m from route[0] in both directions; only heading flips.
    expect(centers[0]).toBeCloseTo(15, 5);
    expect(centers[1]).toBeCloseTo(15, 5);
  });

  it('settles train follow on a visible view beyond a foreground tower', () => {
    vi.stubGlobal('window', {
      matchMedia: () => ({
        matches: true,
        addEventListener() {},
        removeEventListener() {},
        addListener() {},
        removeListener() {},
      }),
    });
    const camera = new THREE.PerspectiveCamera(39, 16 / 9, 0.1, 2000),
      director = new CameraDirector(camera, makeElement());
    const world = makeWorld();
    world.buildings = [{ ...makeBuilding(900, 25, 50), width: 60, depth: 24, floors: 20 }];
    director.world(world);
    director.follow(500);
    director.update(0.1, 0);
    const state = director as unknown as {
      cameraObstacles: Parameters<typeof cameraPathClearance>[2];
    };
    for (const x of [-5, 15, 35])
      expect(
        cameraPathClearance({ x, y: 8.735, z: 0 }, camera.position, state.cameraObstacles, {
          safetyMarginM: 0,
        }).collides,
      ).toBe(false);
    const settled = camera.position.clone();
    for (let i = 0; i < 50; i++) director.update(0.1, i * 0.1);
    expect(camera.position.distanceTo(settled)).toBeLessThan(0.01);
    director.dispose();
  });

  it('frames the full train closely from the same side through a reversal', () => {
    vi.stubGlobal('window', {
      matchMedia: () => ({
        matches: true,
        addEventListener() {},
        removeEventListener() {},
        addListener() {},
        removeListener() {},
      }),
    });
    const camera = new THREE.PerspectiveCamera(39, 817 / 753, 0.1, 2000),
      director = new CameraDirector(camera, makeElement());
    director.world(makeWorld());
    director.follow(500);
    director.setPositions(new Float32Array([500, 15, 0, 0, 2, -1]));
    director.update(0.1, 0);
    const firstSide = camera.position.clone().sub(director.target).normalize();
    expect(camera.position.distanceTo(director.target)).toBeLessThan(190);
    // The lightweight controls mock does not aim the camera like real OrbitControls.
    camera.lookAt(director.target);
    camera.updateMatrixWorld(true);
    for (const x of [-8.5, 38.5]) {
      const screen = new THREE.Vector3(x, 3, 0).project(camera);
      expect(Math.abs(screen.x)).toBeLessThan(0.9);
      expect(Math.abs(screen.y)).toBeLessThan(0.9);
    }
    director.setPositions(new Float32Array([500, 15, 0, Math.PI, 2, -1]));
    director.update(0.1, 0.1);
    const reversedSide = camera.position.clone().sub(director.target).normalize();
    expect(firstSide.dot(reversedSide)).toBeGreaterThan(0.99);
    director.dispose();
  });

  it('keeps a live resident movement tuple when a structural snapshot updates indoor fallback data', () => {
    const world = makeWorld();
    world.buildings = [makeBuilding(10, -30, 10), makeBuilding(11, 95, 20)];
    const resident: ResidentFixture = {
      id: 701,
      name: 'Nana Mori',
      home_building: 10,
      work_building: 11,
      activity: 'walking_to_work',
      location_node: 1,
    };
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(withResident(world, resident));
    director.follow(resident.id);
    director.setPositions(new Float32Array([resident.id, 82, 37, 0, 0, -1]));

    const nextWorld = withResident(
      { ...world, revision: world.revision + 1 },
      { ...resident, location_node: 2, activity: 'at_work' },
    );
    director.world(nextWorld);

    const state = director as unknown as { followPosition: THREE.Vector3; followId: number | null };
    expect(state.followPosition.x).toBe(82);
    expect(state.followPosition.z).toBe(37);
    expect(state.followId).toBe(resident.id);
    expect(director.label).toBe('Nana Mori');
    director.dispose();
  });

  it('keeps the resident identity and follows an indoor location node across snapshots', () => {
    const world = makeWorld();
    world.buildings = [makeBuilding(10, -30, 10), makeBuilding(11, 95, 20)];
    const resident: ResidentFixture = {
      id: 702,
      name: 'Mika Sato',
      home_building: 10,
      work_building: 11,
      activity: 'at_work',
      location_node: 2,
    };
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(withResident(world, resident));
    director.follow(resident.id);
    const state = director as unknown as { followPosition: THREE.Vector3; followId: number | null };

    expect(director.label).toBe('Mika Sato');
    expect(state.followPosition.x).toBe(100);
    expect(state.followPosition.z).toBe(0);
    expect(state.followId).toBe(resident.id);

    director.world(
      withResident(
        { ...world, revision: 1 },
        { ...resident, activity: 'at_home', location_node: 2 },
      ),
    );
    expect(director.label).toBe('Mika Sato');
    expect(state.followPosition.x).toBe(100);
    director.dispose();
  });

  it('follows an onboard resident at the train center through reversal without changing identity', () => {
    const resident: ResidentFixture = {
      id: 703,
      name: 'Yuki Tanaka',
      home_building: 10,
      work_building: 11,
      activity: 'riding_train',
      train_id: 500,
    };
    const world = withResident(makeWorld(), resident);
    world.trains[0].direction = 1;
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(world);
    director.follow(resident.id);
    const state = director as unknown as {
      followPosition: THREE.Vector3;
      followId: number | null;
      followHeading: number;
      desiredTarget: THREE.Vector3;
    };

    expect(director.label).toBe('Yuki Tanaka');
    expect(state.followId).toBe(resident.id);
    expect(state.followPosition.x).toBeCloseTo(15, 5);
    expect(state.desiredTarget.x).toBeCloseTo(25, 5);

    const reversed = withResident({ ...world, revision: 1 }, resident);
    reversed.trains[0].direction = -1;
    director.world(reversed);
    director.update(0.1, 0.1);
    expect(state.followPosition.x).toBeCloseTo(15, 5);
    expect(state.desiredTarget.x).toBeCloseTo(5, 5);
    expect(director.label).toBe('Yuki Tanaka');
    expect(state.followId).toBe(resident.id);

    // When the movement buffer publishes the train tuple, follow it while the
    // resident's persistent ID and label stay selected.
    director.setPositions(new Float32Array([500, 22, 5, Math.PI, 2, -1]));
    const next = withResident({ ...reversed, revision: 2 }, { ...resident, location_node: 2 });
    next.trains[0].direction = 1;
    next.trains[0].progress = 17;
    director.world(next);
    expect(state.followPosition.x).toBe(22);
    expect(state.followPosition.z).toBe(5);
    expect(state.followId).toBe(resident.id);
    expect(director.label).toBe('Yuki Tanaka');
    director.dispose();
  });

  it('keeps resident identity while following an onboard bus and returns to the curb after alighting', () => {
    const world = makeWorld();
    world.roads = [
      {
        id: 30,
        from: 1,
        to: 2,
        kind: 'local',
        stage: 'open',
        progress: 1,
        connected: true,
        length: 100,
        lanes: 2,
      },
    ];
    world.bus_lines = [
      {
        id: 8,
        stops: [1, 2],
        route: [1, 2],
        active: true,
        stop_positions: [
          { node_id: 1, x: 0, z: 6.2, heading: 0, direction: 1 },
          { node_id: 1, x: 0, z: -6.2, heading: Math.PI, direction: -1 },
          { node_id: 2, x: 100, z: 6.2, heading: 0, direction: 1 },
          { node_id: 2, x: 100, z: -6.2, heading: Math.PI, direction: -1 },
        ],
      },
    ];
    world.buses = [
      {
        id: 600,
        line_id: 8,
        progress: 20,
        direction: -1,
        dwelling: false,
        passengers: 4,
        capacity: 18,
        next_stop: 1,
        waiting: 2,
      },
    ];
    const resident: ResidentFixture = {
      id: 91,
      name: 'Yuki Tanaka',
      home_building: 700,
      work_building: null,
      activity: 'commuting',
      transfer: 'riding_bus',
      train_id: null,
      bus_id: 600,
      bus_origin_stop: 2,
      bus_destination_stop: 1,
      bus_stop_node: 2,
    };
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(withResident(world, resident));
    director.follow(resident.id);

    const state = director as unknown as {
      followPosition: THREE.Vector3;
      followHeading: number;
      followKind: number;
      followId: number | null;
    };
    expect(state.followPosition.x).toBeCloseTo(20);
    expect(state.followHeading).toBeCloseTo(Math.PI);
    expect(state.followKind).toBe(3);
    expect(director.label).toBe('Yuki Tanaka');

    // Movement is keyed by bus ID in transit, but the camera continues to track resident ID.
    director.setPositions(new Float32Array([600, 45, 0, Math.PI, 3, -1]));
    expect(state.followPosition.x).toBe(45);
    expect(state.followId).toBe(resident.id);

    // When the resident is queued, use the directional curb marker rather than the road-node center.
    const waiting = withResident(
      { ...world, revision: 2, buses: [] },
      {
        ...resident,
        activity: 'waiting_for_bus',
        transfer: 'waiting_bus',
        bus_id: null,
        bus_stop_node: 2,
      },
    );
    director.setPositions(new Float32Array());
    director.world(waiting);
    expect(state.followPosition.x).toBe(100);
    expect(state.followPosition.z).toBeCloseTo(-6.2);
    expect(state.followHeading).toBeCloseTo(Math.PI);
    expect(director.label).toBe('Yuki Tanaka');

    const alighted = withResident(
      { ...waiting, revision: 3 },
      {
        ...resident,
        activity: 'walking_from_bus',
        transfer: 'walking_from_bus',
        bus_id: null,
        bus_stop_node: 1,
        location_node: null,
      },
    );
    director.world(alighted);
    expect(state.followPosition.x).toBe(0);
    expect(state.followPosition.z).toBeCloseTo(-6.2);
    expect(state.followId).toBe(resident.id);
    expect(director.label).toBe('Yuki Tanaka');
    director.dispose();
  });

  it('follows a stranded passenger at the waiting position after their vehicle departs', () => {
    const world = makeWorld();
    const resident: ResidentFixture = {
      id: 91,
      name: 'Yuki Tanaka',
      home_building: 700,
      work_building: null,
      activity: 'commuting',
      transfer: 'riding_inbound',
      train_id: 500,
    };
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(withResident(world, resident));
    director.follow(resident.id);
    director.setPositions(new Float32Array([500, 80, 0, Math.PI, 2, -1]));

    const waiting = withResident(
      { ...world, revision: 2 },
      {
        ...resident,
        activity: 'waiting_for_route',
        transfer: 'waiting_alight',
        train_id: null,
        bus_id: null,
        location_node: 2,
      },
    );
    director.world(waiting);
    director.setPositions(new Float32Array([500, 65, 0, Math.PI, 2, -1, 91, 98, 6, 0, 0, 8]));
    const state = director as unknown as {
      followPosition: THREE.Vector3;
      followId: number;
      followKind: number;
    };
    expect(state.followPosition.toArray()).toEqual([98, 8, 6]);
    expect(state.followId).toBe(91);
    expect(state.followKind).toBe(0);
    expect(director.label).toBe('Yuki Tanaka');

    // A structural update must not replace the precise platform tuple with
    // either the road node or the departing train's position.
    director.world({ ...waiting, revision: 3 });
    expect(state.followPosition.toArray()).toEqual([98, 8, 6]);
    director.setPositions(new Float32Array([500, 40, 0, Math.PI, 2, -1, 91, 98, 6, 0, 0, 8]));
    expect(state.followPosition.toArray()).toEqual([98, 8, 6]);
    director.dispose();
  });

  it('uses the authoritative directional curb marker for a queued bus passenger fallback', () => {
    const world = makeWorld();
    world.bus_lines = [
      // This line shares the queued stop, but not both trip endpoints.
      {
        id: 7,
        stops: [2, 3],
        route: [2, 3],
        active: true,
        stop_positions: [{ node_id: 2, x: 100, z: 9, heading: 0, direction: 1 }],
      },
      {
        id: 8,
        stops: [1, 2],
        route: [1, 2],
        active: true,
        stop_positions: [
          { node_id: 2, x: 100, z: 6.2, heading: 0, direction: 1 },
          { node_id: 2, x: 100, z: -6.2, heading: Math.PI, direction: -1 },
        ],
      },
    ];
    const resident: ResidentFixture = {
      id: 92,
      name: 'Mika Sato',
      home_building: 700,
      work_building: null,
      activity: 'waiting_for_bus',
      transfer: 'waiting_bus',
      bus_id: null,
      bus_origin_stop: 2,
      bus_destination_stop: 1,
      bus_stop_node: 2,
      location_node: 2,
    };
    const director = new CameraDirector(
      new THREE.PerspectiveCamera(42, 1, 0.1, 2_000),
      makeElement(),
    );
    director.world(withResident(world, resident));
    director.follow(resident.id);
    const state = director as unknown as {
      followPosition: THREE.Vector3;
      followHeading: number;
      followKind: number;
    };

    expect(state.followPosition.x).toBe(100);
    expect(state.followPosition.z).toBeCloseTo(-6.2);
    expect(state.followHeading).toBeCloseTo(Math.PI);
    expect(state.followKind).toBe(0);
    director.dispose();
  });
});
