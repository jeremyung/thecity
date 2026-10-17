import { ActorsLayer } from '../web/src/render/actors-layer';
import { LandscapeLayer } from '../web/src/render/landscape-layer';
import { RoadsLayer } from '../web/src/render/roads-layer';
import * as THREE from 'three/webgpu';
import { uniform } from 'three/tsl';
import { describe, expect, it, vi } from 'vitest';
import { CityRenderer } from '../web/src/render/city';
import { COMPACT_EXPRESSWAY_PROFILE } from '../web/src/render/expressway-profile';
import { roadBatchSignatures } from '../web/src/render/road-batch-signature';
import { Kit } from '../web/src/render/kit';
import type { World } from '../web/src/types';

vi.mock('../web/src/render/materials', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../web/src/render/materials')>();
  return { ...actual, signTexture: () => new THREE.Texture() };
});

const stats = { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 };
function world(overrides: Partial<World> = {}): World {
  return {
    schema_version: 1,
    engine_version: 'test',
    seed: 17,
    tick: 1,
    day: 1,
    minute_of_day: 360,
    development_rate: 1,
    revision: 1,
    stage: 'running',
    bounds: { min_x: -100, max_x: 100, min_z: -100, max_z: 100 },
    terrain: {
      river: [
        { x: 0, z: -500 },
        { x: 0, z: 500 },
      ],
      protected: [],
      gateways: [],
    },
    weather: { kind: 'clear', intensity: 0 },
    nodes: [
      { id: 1, x: -60, z: 0, kind: 'station' },
      { id: 2, x: 60, z: 0, kind: 'street' },
      { id: 3, x: 0, z: -60, kind: 'street' },
    ],
    roads: [
      {
        id: 10,
        from: 1,
        to: 2,
        kind: 'local',
        stage: 'open',
        progress: 1,
        connected: true,
        length: 120,
        lanes: 1,
      },
      {
        id: 11,
        from: 1,
        to: 3,
        kind: 'bridge',
        stage: 'building',
        progress: 0.2,
        connected: false,
        length: 85,
        lanes: 1,
      },
      {
        id: 12,
        from: 2,
        to: 3,
        kind: 'local',
        stage: 'building',
        progress: 0.2,
        connected: false,
        length: 85,
        lanes: 1,
      },
    ],
    parcels: [],
    buildings: [],
    projects: [],
    trains: [],
    events: [],
    stats,
    ...overrides,
  };
}

function harness() {
  const renderer = Object.create(CityRenderer.prototype) as any;
  renderer.actors = Object.assign(Object.create(ActorsLayer.prototype), { host: renderer });
  renderer.landscape = new LandscapeLayer(renderer);
  renderer.roadsLayer = new RoadsLayer(renderer);
  renderer.scene = new THREE.Scene();
  renderer.roadsLayer.roads = new THREE.Group();
  renderer.scene.add(renderer.roadsLayer.roads);
  renderer.roadsLayer.constructionRoads = new THREE.Group();
  renderer.scene.add(renderer.roadsLayer.constructionRoads);
  renderer.roadsLayer.station = new THREE.Group();
  renderer.scene.add(renderer.roadsLayer.station);
  renderer.roadsLayer.roadSignature = '';
  renderer.roadsLayer.completedRoadSignature = '';
  renderer.riverHalfWidth = 32;
  renderer.worldState = null;
  renderer.worldTick = 0;
  renderer.terrainSeed = 17;
  renderer.hour = 0;
  renderer.rain = 0;
  renderer.groundSignature = '';
  renderer.retiredResources = [];
  renderer.pendingCompiles = 0;
  const materials = new Map<string, THREE.Material>();
  renderer.art = {
    night: uniform(1),
    style: 'day',
    setStyle(style: string) {
      this.style = style;
    },
    variant: vi.fn((material: THREE.Material) => material),
    get: (tone: string, emission = 0, lamp = false) => {
      const key = `${tone}:${emission}:${lamp}`;
      if (!materials.has(key)) materials.set(key, new THREE.MeshBasicMaterial({ color: tone }));
      return materials.get(key)!;
    },
    asphalt() {
      return this.get('#747f7b');
    },
  };
  // This harness isolates road/ground publication; park geometry and seasonal
  // painting have their own tests and need the full foliage/material setup.
  renderer.landscape.rebuildParks = vi.fn();
  renderer.landscape.paintSeason = vi.fn();
  renderer.rig = { world: vi.fn() };
  renderer.actors.buses = { setWorld: vi.fn() };
  renderer.actors.trains = { setWorld: vi.fn() };
  renderer.layers = [];
  renderer.sidewalks = { setWorld: vi.fn() };
  renderer.chunks = { remove: vi.fn() };
  renderer.groundSignature = JSON.stringify([]);
  renderer.buildings = new Map();
  renderer.objectQueue = new Map();
  renderer.landscape.updateTrees = vi.fn();
  return renderer;
}

function submitPending(renderer: any, snapshot: World) {
  renderer.setWorld(snapshot);
}

// Most legacy assertions describe the settled picture. Keep that convenience
// explicit so tests that exercise staging can inspect the pending frames.
function submit(renderer: any, snapshot: World) {
  submitPending(renderer, snapshot);
  for (
    let i = 0;
    i < 10000 && (renderer.roadsLayer.roadBuild || renderer.roadsLayer.queuedRoadWorld);
    i++
  )
    renderer.roadsLayer.updateRoads();
  if (renderer.roadsLayer.roadBuild || renderer.roadsLayer.queuedRoadWorld)
    throw new Error('road build did not settle');
}

function geometryFingerprint(group: THREE.Object3D): string[] {
  const result: string[] = [];
  group.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    const position = object.geometry.getAttribute('position');
    result.push(`${position.count}:${Array.from(position.array).join(',')}`);
  });
  return result;
}
function vertexCount(group: THREE.Object3D): number {
  return geometryFingerprint(group).reduce(
    (sum, value) => sum + Number(value.slice(0, value.indexOf(':'))),
    0,
  );
}

function firstGroundHit(group: THREE.Object3D, x: number, z: number) {
  group.updateMatrixWorld(true);
  const raycaster = new THREE.Raycaster(new THREE.Vector3(x, 2, z), new THREE.Vector3(0, -1, 0));
  return raycaster.intersectObject(group, true)[0];
}

const platform = {
  node_id: 1,
  x: 0,
  z: 0,
  ux: 0,
  uz: 1,
  surface_y: 8,
  stair_x: 10,
  stair_bottom_z: -18,
  stair_top_z: 0,
  ground_y: 0,
  stair_steps: 12,
  ground_path: [
    { x: 0, z: 0 },
    { x: 7.5, z: -1.5 },
    { x: 7.5, z: -21.5 },
    { x: 4.05, z: -21.5 },
    { x: 4.05, z: -20 },
  ],
  ground_width: 1.6,
  construction_phase: 4,
};

describe('road batch retention', () => {
  it('publishes persistent lamp transforms only with the corresponding completed road batch', () => {
    const renderer = harness(),
      first = world();
    submit(renderer, first);
    const pool = renderer.roadsLayer.streetLighting,
      meshes = [...pool.meshes];
    const before = meshes.map((mesh) => Array.from(mesh.instanceMatrix.array));
    const oldRoads = renderer.roadsLayer.roads;
    const next = {
      ...first,
      tick: 2,
      roads: first.roads.map((road) =>
        road.id === 12 ? { ...road, stage: 'open', progress: 1, connected: true } : road,
      ),
    };
    submitPending(renderer, next);
    expect(renderer.roadsLayer.roads).toBe(oldRoads);
    expect(
      pool.meshes.map((mesh: THREE.InstancedMesh) => Array.from(mesh.instanceMatrix.array)),
    ).toEqual(before);
    let committed = false;
    for (let step = 0; step < 10000; step++) {
      renderer.roadsLayer.updateRoads();
      if (renderer.roadsLayer.roads !== oldRoads) {
        committed = true;
        break;
      }
      expect(
        pool.meshes.map((mesh: THREE.InstancedMesh) => Array.from(mesh.instanceMatrix.array)),
      ).toEqual(before);
    }
    expect(committed).toBe(true);
    expect(renderer.roadsLayer.streetLighting).toBe(pool);
    expect(pool.meshes).toEqual(meshes);
    expect(pool.root.parent).toBe(renderer.scene);
    expect(
      pool.root.userData.streetLightSources.some((source: { key: string }) =>
        source.key.startsWith('12:'),
      ),
    ).toBe(true);
    expect(
      pool.meshes.map((mesh: THREE.InstancedMesh) => Array.from(mesh.instanceMatrix.array)),
    ).not.toEqual(before);
  });

  it('assigns the captured edge overlap to one pavement surface through construction and opening', () => {
    const renderer = harness();
    const snapshot = world({
      nodes: [
        { id: 1736, x: 145.64136, z: 12.112568, kind: 'street' },
        { id: 1272, x: 115.08585, z: 13.294467, kind: 'street' },
        { id: 1777, x: 104.16684, z: 19.800463, kind: 'street' },
        { id: 1806, x: 123.55672, z: 25.20845, kind: 'street' },
      ],
      roads: [
        {
          id: 1787,
          from: 1736,
          to: 1272,
          kind: 'local',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 30.58,
          lanes: 1,
        },
        {
          id: 1807,
          from: 1777,
          to: 1806,
          kind: 'arterial',
          stage: 'structure',
          progress: 0.7151829,
          connected: false,
          length: 20.13,
          lanes: 2,
        },
      ],
    });
    let cachedPartition: unknown;
    let completedClaims: ReturnType<typeof vi.spyOn> | undefined;
    for (const progress of [0.7151829, 0.8582195, 1]) {
      snapshot.roads[1] = {
        ...snapshot.roads[1],
        progress,
        stage: progress === 1 ? 'open' : 'surface',
      };
      snapshot.revision++;
      submit(renderer, snapshot);
      if (progress === 0.7151829) {
        cachedPartition = renderer.roadsLayer.completedRoadSurfaces;
        completedClaims = vi.spyOn(renderer.roadsLayer.completedRoadSurfaces, 'claim');
      } else if (progress === 0.8582195) {
        expect(renderer.roadsLayer.completedRoadSurfaces).toBe(cachedPartition);
        expect(completedClaims).not.toHaveBeenCalled();
        completedClaims!.mockRestore();
      } else expect(renderer.roadsLayer.completedRoadSurfaces).not.toBe(cachedPartition);
      renderer.scene.updateMatrixWorld(true);
      // Interior of the measured overlap triangle, away from paint and mesh edges.
      const ray = new THREE.Raycaster(
        new THREE.Vector3(115.7, 1, 15.7),
        new THREE.Vector3(0, -1, 0),
      );
      const hits = ray
        .intersectObjects([renderer.roadsLayer.roads, renderer.roadsLayer.constructionRoads], true)
        .filter((hit) => Math.abs(hit.point.y - 0.3) < 1e-5);
      expect(hits, `one surface at progress ${progress}`).toHaveLength(1);
      expect(
        ((hits[0].object as THREE.Mesh).material as THREE.MeshBasicMaterial).color.getHexString(),
      ).toBe('747f7b');
    }
  });

  it('trims local edge paint through a junction and resumes it beyond the shared carriageway', () => {
    const renderer = harness();
    const snapshot = world({
      nodes: [
        { id: 1, x: 0, z: 0, kind: 'street' },
        { id: 2, x: 0, z: 60, kind: 'street' },
        { id: 3, x: 60, z: 0, kind: 'street' },
      ],
      roads: [
        {
          id: 10,
          from: 1,
          to: 2,
          kind: 'local',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 60,
          lanes: 1,
        },
        {
          id: 11,
          from: 1,
          to: 3,
          kind: 'arterial',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 60,
          lanes: 2,
        },
      ],
    });

    submit(renderer, snapshot);

    // Local edge stripe is absent in the 8.2m arterial opening, then resumes.
    const junction = firstGroundHit(renderer.roadsLayer.roads, 2.8, 4);
    expect(junction).toBeDefined();
    expect(
      ((junction!.object as THREE.Mesh).material as THREE.MeshBasicMaterial).color.getHexString(),
    ).toBe('747f7b');
    expect(junction!.point.y).toBeCloseTo(0.3, 5);

    const stripe = firstGroundHit(renderer.roadsLayer.roads, 2.8, 14);
    expect(stripe).toBeDefined();
    expect(
      ((stripe!.object as THREE.Mesh).material as THREE.MeshBasicMaterial).color.getHexString(),
    ).toBe('dedaca');
    expect(stripe!.point.y).toBeGreaterThan(0.3);
  });

  it('does not render construction cones on an open local road', () => {
    const renderer = harness();
    const snapshot = world({
      nodes: [
        { id: 1, x: 0, z: 0, kind: 'street' },
        { id: 2, x: 0, z: 60, kind: 'street' },
      ],
      roads: [
        {
          id: 10,
          from: 1,
          to: 2,
          kind: 'local',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 60,
          lanes: 1,
        },
      ],
    });

    submit(renderer, snapshot);

    // Open-road construction markers are placed every 12 m at the road edge.
    const edge = firstGroundHit(renderer.roadsLayer.roads, 3.1, 24);
    expect(edge).toBeDefined();
    expect(
      ((edge!.object as THREE.Mesh).material as THREE.MeshBasicMaterial).color.getHexString(),
    ).toBe('747f7b');
    expect(edge!.point.y).toBeCloseTo(0.3, 5);
  });

  it('keeps local and lane road surfaces level through a ground-level bend', () => {
    const renderer = harness();
    const snapshot = world({
      nodes: [
        { id: 1, x: 0, z: 0, kind: 'street' },
        { id: 2, x: 0, z: 60, kind: 'street' },
        { id: 3, x: 60, z: 0, kind: 'street' },
      ],
      roads: [
        {
          id: 10,
          from: 1,
          to: 2,
          kind: 'local',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 60,
          lanes: 1,
        },
        {
          id: 11,
          from: 1,
          to: 3,
          kind: 'lane',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 60,
          lanes: 1,
        },
      ],
    });

    submit(renderer, snapshot);

    const localSurface = firstGroundHit(renderer.roadsLayer.roads, 0, 30);
    const laneSurface = firstGroundHit(renderer.roadsLayer.roads, 30, 0);
    expect(localSurface).toBeDefined();
    expect(laneSurface).toBeDefined();
    expect(localSurface!.point.y).toBeCloseTo(0.3, 5);
    expect(laneSurface!.point.y).toBeCloseTo(0.3, 5);
  });

  it('holds platform movement support until the replacement station batch commits', () => {
    const renderer = harness();
    const initial = world({ rail_platforms: [platform] });
    const support = { resolved: true, roads: [10], platforms: [1] };
    submitPending(renderer, initial);
    expect(renderer.roadsLayer.infrastructure().canPresentMovement(support)).toBe(false);
    submit(renderer, initial);
    expect(renderer.roadsLayer.infrastructure().canPresentMovement(support)).toBe(true);

    const changed = world({ rail_platforms: [{ ...platform, surface_y: 9 }] });
    submitPending(renderer, changed);
    expect(renderer.roadsLayer.infrastructure().canPresentMovement(support)).toBe(false);
    submit(renderer, changed);
    expect(renderer.roadsLayer.infrastructure().canPresentMovement(support)).toBe(true);
  });

  it('schedules a new commit when lane identity changes under an existing road ID', () => {
    const renderer = harness();
    const initial = world();
    const support = { resolved: true, roads: [10], platforms: [] };
    submit(renderer, initial);
    const changed = world({
      roads: initial.roads.map((road) => (road.id === 10 ? { ...road, lanes: 3 } : road)),
    });
    submitPending(renderer, changed);
    expect(renderer.roadsLayer.roadBuild || renderer.roadsLayer.queuedRoadWorld).toBeTruthy();
    expect(renderer.roadsLayer.infrastructure().canPresentMovement(support)).toBe(false);
    submit(renderer, changed);
    expect(renderer.roadsLayer.infrastructure().canPresentMovement(support)).toBe(true);
  });

  it('admits a queued building only after its supporting road batch commits', () => {
    const renderer = harness();
    const snapshot = world({
      building_access: [
        {
          building_id: 500,
          road_id: 10,
          points: [
            { x: 0, z: 0 },
            { x: 1, z: 1 },
          ],
        },
      ],
    });
    renderer.chunks.set = vi.fn();
    renderer.chunks.update = vi.fn();
    submitPending(renderer, snapshot);
    const createGroup = vi.fn(() => new THREE.Group());
    renderer.objectQueue.set(500, {
      building: { x: 0, z: 0 },
      signature: 'new',
      source: { createGroup },
    });
    renderer.updateBuildingBatches();
    expect(createGroup).not.toHaveBeenCalled();
    expect(renderer.objectQueue.has(500)).toBe(true);
    expect(renderer.chunks.update).toHaveBeenLastCalledWith(new Set());

    for (let i = 0; i < 10000 && renderer.roadsLayer.roadBuild; i++)
      renderer.roadsLayer.updateRoads();
    expect(renderer.roadsLayer.roadBuild).toBeNull();
    renderer.updateBuildingBatches();
    expect(createGroup).toHaveBeenCalledOnce();
    expect(renderer.objectQueue.has(500)).toBe(false);
    expect(renderer.buildings.has(500)).toBe(true);
  });

  it('defers road kit generation until the road update phase', () => {
    const renderer = harness();
    const roadKit = vi.spyOn(renderer.roadsLayer, 'roadKit');

    submitPending(renderer, world());

    expect(roadKit).not.toHaveBeenCalled();
    expect(renderer.roadsLayer.roadBuild?.preparation).toBeTruthy();
    for (let i = 0; i < 100 && !roadKit.mock.calls.length; i++) renderer.roadsLayer.updateRoads();
    expect(roadKit).toHaveBeenCalled();
  });

  it('cancels partially prepared geometry while keeping the visible batches committed', () => {
    const renderer = harness();
    const initial = world();
    submit(renderer, initial);
    const committed = [
      renderer.roadsLayer.roads,
      renderer.roadsLayer.constructionRoads,
      renderer.roadsLayer.station,
    ];
    const manyNodes = Array.from({ length: 504 }, (_, i) => ({
      id: 100 + i,
      x: (i % 22) * 12,
      z: Math.floor(i / 22) * 12,
      kind: 'street' as const,
    }));
    const next = {
      ...initial,
      tick: 2,
      nodes: manyNodes,
      roads: Array.from({ length: 500 }, (_, i) => ({
        id: 200 + i,
        from: 100 + i,
        to: 100 + ((i + 1) % 504),
        kind: 'local' as const,
        stage: 'building' as const,
        progress: 0.3,
        connected: false,
        length: 12,
        lanes: 1,
      })),
    };

    submitPending(renderer, next);
    const candidate = renderer.roadsLayer.roadBuild;
    expect(candidate?.preparation).toBeTruthy();
    const returnPreparation = vi.spyOn(candidate.preparation!, 'return');
    const disposals: ReturnType<typeof vi.fn>[] = [];
    const originalAdd = Kit.prototype.add;
    const add = vi.spyOn(Kit.prototype, 'add').mockImplementation(function (
      this: Kit,
      geometry,
      material,
    ) {
      const disposed = vi.fn();
      geometry.addEventListener('dispose', disposed);
      disposals.push(disposed);
      originalAdd.call(this, geometry, material);
    });
    for (let i = 0; i < 100 && disposals.length === 0; i++) renderer.roadsLayer.updateRoads();
    add.mockRestore();
    expect(disposals.length).toBeGreaterThan(0);
    expect(renderer.roadsLayer.roadBuild?.preparation).toBeTruthy();
    renderer.roadsLayer.cancelRoadBuild();

    expect([
      renderer.roadsLayer.roads,
      renderer.roadsLayer.constructionRoads,
      renderer.roadsLayer.station,
    ]).toEqual(committed);
    expect(returnPreparation).toHaveBeenCalledOnce();
    for (const disposed of disposals) expect(disposed).toHaveBeenCalledOnce();
  });

  it('builds from the submitted snapshot even if its source objects are later mutated', () => {
    const renderer = harness();
    const submitted = world();
    const expected = harness();
    submit(expected, submitted);

    submitPending(renderer, submitted);
    submitted.nodes.find((n) => n.id === 3)!.x = 41;
    submitted.roads[0].kind = 'arterial';
    for (let i = 0; i < 10000 && renderer.roadsLayer.roadBuild; i++)
      renderer.roadsLayer.updateRoads();

    expect(geometryFingerprint(renderer.roadsLayer.roads)).toEqual(
      geometryFingerprint(expected.roadsLayer.roads),
    );
    expect(geometryFingerprint(renderer.roadsLayer.constructionRoads)).toEqual(
      geometryFingerprint(expected.roadsLayer.constructionRoads),
    );
  });

  it('keeps committed groups visible while a replacement is staged', () => {
    const renderer = harness();
    const first = world();
    submit(renderer, first);
    const oldRoads = renderer.roadsLayer.roads,
      oldConstruction = renderer.roadsLayer.constructionRoads,
      oldStation = renderer.roadsLayer.station;

    submitPending(renderer, {
      ...first,
      tick: 2,
      nodes: first.nodes.map((n) => (n.id === 3 ? { ...n, x: 15 } : n)),
    });

    expect(renderer.roadsLayer.roads).toBe(oldRoads);
    expect(renderer.roadsLayer.constructionRoads).toBe(oldConstruction);
    expect(renderer.roadsLayer.station).toBe(oldStation);
    expect(renderer.roadsLayer.roadBuild).toBeTruthy();
    expect(renderer.roadsLayer.completedRoadSignature).not.toBe(
      renderer.roadsLayer.roadBuild.signature,
    );
  });

  it('coalesces queued worlds to the latest snapshot without restarting the active build', () => {
    const renderer = harness();
    const first = world();
    submitPending(renderer, first);
    const active = renderer.roadsLayer.roadBuild;
    expect(active).toBeTruthy();
    const second = {
      ...first,
      tick: 2,
      nodes: first.nodes.map((n) => (n.id === 3 ? { ...n, x: 10 } : n)),
    };
    const latest = {
      ...second,
      tick: 3,
      nodes: second.nodes.map((n) => (n.id === 3 ? { ...n, x: 18 } : n)),
    };

    submitPending(renderer, second);
    submitPending(renderer, latest);
    expect(renderer.roadsLayer.roadBuild === active).toBe(true);
    expect(renderer.roadsLayer.queuedRoadWorld.world.tick).toBe(3);
    expect(
      renderer.roadsLayer.queuedRoadWorld.world.nodes.find(
        (n: World['nodes'][number]) => n.id === 3,
      )?.x,
    ).toBe(18);

    while (renderer.roadsLayer.roadBuild) renderer.roadsLayer.updateRoads();
    expect(renderer.roadsLayer.completedRoadSignature).toBe(roadBatchSignatures(first).completed);
    expect(renderer.roadsLayer.roads).not.toBeNull();
    expect(renderer.roadsLayer.queuedRoadWorld.world.tick).toBe(3);
    renderer.roadsLayer.updateRoads(); // queued preparation starts on the following frame
    expect(renderer.roadsLayer.roadBuild).toBeTruthy();
    expect(renderer.roadsLayer.roadBuild.signature).toBe(roadBatchSignatures(latest).completed);
    submit(renderer, latest);
    expect(renderer.roadsLayer.queuedRoadWorld).toBeNull();
  });

  it('cancels pending candidates while retaining the committed batches', () => {
    const renderer = harness();
    const first = world();
    submit(renderer, first);
    const oldRoads = renderer.roadsLayer.roads,
      oldConstruction = renderer.roadsLayer.constructionRoads,
      oldStation = renderer.roadsLayer.station;
    submitPending(renderer, {
      ...first,
      tick: 2,
      nodes: first.nodes.map((n) => (n.id === 3 ? { ...n, x: 15 } : n)),
    });
    for (
      let i = 0;
      i < 100 && geometryFingerprint(renderer.roadsLayer.roadBuild.construction).length === 0;
      i++
    )
      renderer.roadsLayer.updateRoads();
    const candidate = renderer.roadsLayer.roadBuild;
    const pendingGeometries: THREE.BufferGeometry[] = [];
    candidate.construction.traverse((object: THREE.Object3D) => {
      if (object instanceof THREE.Mesh) pendingGeometries.push(object.geometry);
    });
    const disposed = vi.fn();
    expect(pendingGeometries.length).toBeGreaterThan(0);
    pendingGeometries[0].addEventListener('dispose', disposed);

    renderer.roadsLayer.cancelRoadBuild();

    expect(renderer.roadsLayer.roadBuild).toBeNull();
    expect(renderer.roadsLayer.queuedRoadWorld).toBeNull();
    expect(renderer.roadsLayer.roads).toBe(oldRoads);
    expect(renderer.roadsLayer.constructionRoads).toBe(oldConstruction);
    expect(renderer.roadsLayer.station).toBe(oldStation);
    expect(disposed).toHaveBeenCalledOnce();
  });

  it('allows retry after queued preparation fails without retiring committed roads', () => {
    const renderer = harness();
    submit(renderer, world());
    const committed = renderer.roadsLayer.roads;
    renderer.roadsLayer.queuedRoadWorld = { world: world(), signature: 'queued' };
    renderer.roadsLayer.roadSignature = 'queued';
    const prepare = vi.spyOn(renderer.roadsLayer, 'prepareRoads').mockImplementationOnce(() => {
      throw new Error('preparation failed');
    });
    expect(() => renderer.roadsLayer.updateRoads()).toThrow('preparation failed');
    expect(renderer.roadsLayer.roadSignature).toBe('');
    expect(renderer.roadsLayer.queuedRoadWorld).toBeNull();
    expect(renderer.roadsLayer.roads === committed).toBe(true);
    prepare.mockRestore();
  });

  it('applies the style selected at commit time to staged road meshes', () => {
    const renderer = harness();
    const first = world();
    const nightMaterial = new THREE.MeshBasicMaterial({ color: 'black' });
    const stylesSeen: string[] = [];
    renderer.art.variant.mockImplementation((material: THREE.Material) => {
      stylesSeen.push(renderer.art.style);
      return renderer.art.style === 'night' ? nightMaterial : material;
    });

    submitPending(renderer, first);
    renderer.art.setStyle('night');
    while (renderer.roadsLayer.roadBuild) renderer.roadsLayer.updateRoads();

    expect(renderer.art.variant).toHaveBeenCalledWith(expect.any(THREE.Material));
    expect(stylesSeen.length).toBeGreaterThan(0);
    expect(stylesSeen.every((style) => style === 'night')).toBe(true);
    const committedMeshes: THREE.Mesh[] = [];
    renderer.roadsLayer.roads.traverse((child: THREE.Object3D) => {
      if (child instanceof THREE.Mesh) committedMeshes.push(child);
    });
    expect(committedMeshes.length).toBeGreaterThan(0);
    expect(committedMeshes.every((mesh) => mesh.material === nightMaterial)).toBe(true);
    nightMaterial.dispose();
  });

  it('keys completed geometry on its dependencies while ignoring construction progress', () => {
    const base = world({
      rail_platforms: [platform],
      road_decks: [
        {
          road_id: 20,
          deck_height: 10,
          controls: [
            [-20, 0],
            [20, 0],
          ],
        },
      ],
    });
    const mutations: Array<[string, (snapshot: World) => World]> = [
      [
        'connected state',
        (snapshot) => ({
          ...snapshot,
          roads: snapshot.roads.map((r) => (r.id === 10 ? { ...r, connected: false } : r)),
        }),
      ],
      [
        'road kind',
        (snapshot) => ({
          ...snapshot,
          roads: snapshot.roads.map((r) => (r.id === 10 ? { ...r, kind: 'arterial' } : r)),
        }),
      ],
      [
        'expressway cross section',
        (snapshot) => ({ ...snapshot, expressway_profile: COMPACT_EXPRESSWAY_PROFILE }),
      ],
      [
        'deck height',
        (snapshot) => ({
          ...snapshot,
          road_decks: [
            {
              road_id: 20,
              deck_height: 11,
              controls: [
                [-20, 0],
                [20, 0],
              ],
            },
          ],
        }),
      ],
      [
        'deck controls',
        (snapshot) => ({
          ...snapshot,
          road_decks: [
            {
              road_id: 20,
              deck_height: 10,
              controls: [
                [-20, 8],
                [20, 0],
              ],
            },
          ],
        }),
      ],
      [
        'platform surface',
        (snapshot) => ({ ...snapshot, rail_platforms: [{ ...platform, surface_y: 9 }] }),
      ],
      [
        'platform stairs',
        (snapshot) => ({
          ...snapshot,
          rail_platforms: [
            {
              ...platform,
              stair_steps: 13,
              ground_path: [
                { x: 0, z: 0 },
                { x: 7.5, z: -1.5 },
                { x: 7.5, z: -21.5 },
                { x: 4.05, z: -21.5 },
                { x: 4.05, z: -20 },
              ],
              ground_width: 1.6,
              construction_phase: 4,
            },
          ],
        }),
      ],
      [
        'river points',
        (snapshot) => ({
          ...snapshot,
          terrain: {
            ...snapshot.terrain,
            river: [
              { x: 2, z: -500 },
              { x: 0, z: 500 },
            ],
          },
        }),
      ],
      [
        'river width',
        (snapshot) => ({ ...snapshot, terrain: { ...snapshot.terrain, river_half_width: 35 } }),
      ],
    ];

    for (const [name, mutate] of mutations) {
      const before = roadBatchSignatures(base);
      const after = roadBatchSignatures(mutate(base));
      expect(after.completed, name).not.toBe(before.completed);
    }

    const before = roadBatchSignatures(base);
    const progressOnly = roadBatchSignatures({
      ...base,
      roads: base.roads.map((r) => (r.id === 11 ? { ...r, progress: 0.3 } : r)),
    });
    expect(progressOnly.completed).toBe(before.completed);
    expect(progressOnly.construction).not.toBe(before.construction);
  });

  it('retains completed roads and station during construction progress while replacing construction geometry', () => {
    const renderer = harness();
    const first = world({
      rail_platforms: [platform],
      projects: [
        {
          id: 90,
          kind: 'station',
          from: 1,
          to: 2,
          parcel_id: null,
          stage: 'building',
          progress: 0.1,
          reason: '',
          duration: 1,
          started_tick: 0,
        },
      ],
    });
    submit(renderer, first);
    const completed = renderer.roadsLayer.roads,
      station = renderer.roadsLayer.station,
      construction = renderer.roadsLayer.constructionRoads;
    const completedGeometry = geometryFingerprint(completed);
    const constructionGeometry = geometryFingerprint(construction);

    submit(renderer, {
      ...first,
      tick: 2,
      roads: first.roads.map((r) => (r.id === 11 ? { ...r, progress: 0.3 } : r)),
    });

    expect(renderer.roadsLayer.roads).toBe(completed);
    expect(renderer.roadsLayer.station).toBe(station);
    expect(geometryFingerprint(renderer.roadsLayer.roads)).toEqual(completedGeometry);
    expect(geometryFingerprint(renderer.roadsLayer.constructionRoads)).not.toEqual(
      constructionGeometry,
    );
    expect(renderer.roadsLayer.constructionRoads).not.toBe(construction);
  });

  it('invalidates completed batches for unfinished crossing topology and same-count node relocation', () => {
    const renderer = harness();
    const first = world();
    submit(renderer, first);
    const completed = renderer.roadsLayer.roads;

    const changedTopology = {
      ...first,
      tick: 2,
      roads: first.roads.map((r) => (r.id === 11 ? { ...r, from: 2 } : r)),
    };
    submit(renderer, changedTopology);
    expect(renderer.roadsLayer.roads).not.toBe(completed);
    const afterTopology = renderer.roadsLayer.roads;

    submit(renderer, {
      ...changedTopology,
      tick: 3,
      nodes: changedTopology.nodes.map((n) => (n.id === 3 ? { ...n, x: 12 } : n)),
    });
    expect(renderer.roadsLayer.roads).not.toBe(afterTopology);
  });

  it('rebuilds only when the authoritative station phase changes', () => {
    const renderer = harness();
    const base = world({
      rail_platforms: [{ ...platform, construction_phase: 2 }],
      projects: [
        {
          id: 90,
          kind: 'station',
          from: 1,
          to: 2,
          parcel_id: null,
          stage: 'surface',
          progress: 0.8,
          reason: '',
          duration: 1,
          started_tick: 0,
        },
      ],
    });
    submit(renderer, base);
    const rooflessStation = renderer.roadsLayer.station;
    expect(rooflessStation.children[0]).toBeDefined();
    const rooflessVertices = vertexCount(rooflessStation);

    submit(renderer, {
      ...base,
      tick: 2,
      projects: base.projects.map((p) => ({ ...p, progress: 0.84 })),
    });
    expect(renderer.roadsLayer.station).toBe(rooflessStation);
    submit(renderer, {
      ...base,
      tick: 3,
      rail_platforms: [{ ...platform, construction_phase: 3 }],
    });

    expect(renderer.roadsLayer.station).not.toBe(rooflessStation);
    expect(renderer.roadsLayer.station.children[0]).toBeDefined();
    expect(vertexCount(renderer.roadsLayer.station)).toBeGreaterThan(rooflessVertices);
  });

  it('moves a completed road from construction into the completed batch', () => {
    const renderer = harness();
    const first = world();
    submit(renderer, first);
    const completedBefore = vertexCount(renderer.roadsLayer.roads);
    const constructionBefore = vertexCount(renderer.roadsLayer.constructionRoads);

    submit(renderer, {
      ...first,
      tick: 2,
      roads: first.roads.map((r) =>
        r.id === 11 ? { ...r, stage: 'open', progress: 1, connected: true } : r,
      ),
    });

    expect(vertexCount(renderer.roadsLayer.roads)).toBeGreaterThan(completedBefore);
    expect(vertexCount(renderer.roadsLayer.constructionRoads)).toBeLessThan(constructionBefore);
  });

  it('defers replaced geometry disposal while retaining persistent lights and shared Art materials', () => {
    const renderer = harness();
    const first = world();
    submit(renderer, first);
    const oldRoads = renderer.roadsLayer.roads as THREE.Group;
    const meshes: THREE.Mesh[] = [];
    oldRoads.traverse((object: THREE.Object3D) => {
      if (object instanceof THREE.Mesh) meshes.push(object);
    });
    const geometry = meshes[0].geometry;
    const lighting = renderer.roadsLayer.streetLighting;
    const lightMeshes = [...lighting.meshes];
    const ownedMaterialMesh = lightMeshes.find((mesh) => mesh.userData.ownedMaterial)!;
    const ownedMaterial = ownedMaterialMesh.material as THREE.Material;
    const sharedMesh = meshes.find((mesh) => !mesh.userData.ownedMaterial)!;
    const sharedMaterial = sharedMesh.material as THREE.Material;
    const geometryDisposed = vi.fn(),
      ownedDisposed = vi.fn(),
      sharedDisposed = vi.fn();
    geometry.addEventListener('dispose', geometryDisposed);
    ownedMaterial.addEventListener('dispose', ownedDisposed);
    sharedMaterial.addEventListener('dispose', sharedDisposed);
    renderer.pendingCompiles = 1;

    submit(renderer, {
      ...first,
      tick: 2,
      nodes: first.nodes.map((n) => (n.id === 3 ? { ...n, x: 15 } : n)),
    });

    expect(renderer.roadsLayer.roads).not.toBe(oldRoads);
    expect(geometryDisposed).not.toHaveBeenCalled();
    expect(ownedDisposed).not.toHaveBeenCalled();
    renderer.pendingCompiles = 0;
    for (const dispose of renderer.retiredResources.splice(0)) dispose();
    expect(geometryDisposed).toHaveBeenCalledOnce();
    expect(renderer.roadsLayer.streetLighting).toBe(lighting);
    expect(renderer.roadsLayer.streetLighting.meshes).toEqual(lightMeshes);
    expect(ownedDisposed).not.toHaveBeenCalled();
    expect(sharedDisposed).not.toHaveBeenCalled();
  });
});
