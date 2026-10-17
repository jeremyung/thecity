import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { VehicleFleet, VEHICLE_KINDS } from '../web/src/render/vehicles';
import { CityRenderer } from '../web/src/render/city';
import type { MovementSupport } from '../web/src/movement-support';
import { MovementPresentation } from '../web/src/render/movement-presentation';
import { VisibleInfrastructure } from '../web/src/render/visible-infrastructure';
import type { World } from '../web/src/types';

function world(): World {
  return {
    schema_version: 1,
    engine_version: 'test',
    seed: 1,
    tick: 0,
    day: 0,
    minute_of_day: 360,
    development_rate: 1,
    revision: 1,
    stage: 'Founding',
    bounds: { min_x: -100, max_x: 100, min_z: -100, max_z: 100 },
    terrain: { river: [], protected: [], gateways: [] },
    weather: { kind: 'clear', intensity: 0 },
    nodes: [
      { id: 1, x: 0, z: 0, kind: 'street' },
      { id: 2, x: 20, z: 0, kind: 'street' },
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
        length: 20,
        lanes: 2,
      },
    ],
    parcels: [],
    buildings: [],
    projects: [],
    trains: [],
    events: [],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
  };
}

function mesh() {
  const matrices: THREE.Matrix4[] = [];
  return {
    matrices,
    count: 0,
    instanceColor: null as THREE.InstancedBufferAttribute | null,
    instanceMatrix: { needsUpdate: false },
    setMatrixAt: (index: number, matrix: THREE.Matrix4) => {
      matrices[index] = matrix.clone();
    },
    setColorAt: vi.fn(),
  };
}

type RendererHarness = {
  fleet: VehicleFleet;
  current: Map<number, { x: number; z: number; h: number; k: number; y: number }>;
  director: { setPositions: ReturnType<typeof vi.fn> };
  visibleInfrastructure: VisibleInfrastructure;
  setMovement(
    data: Float32Array,
    tick: number,
    minute: number,
    trainDoors: Float32Array,
    support: ReadonlyMap<number, MovementSupport>,
  ): void;
  updateActors(now: number): void;
};

const cleanups: Array<() => void> = [];
afterEach(() => cleanups.splice(0).forEach((cleanup) => cleanup()));
function actorMatrix(renderer: RendererHarness, id: number) {
  const entry = renderer.fleet.pickable.find((p) => p.ids.includes(id));
  if (!entry) throw new Error(`Actor ${id} is not rendered`);
  const matrix = new THREE.Matrix4();
  entry.mesh.getMatrixAt(entry.ids.indexOf(id), matrix);
  return matrix;
}
function harness(snapshot: World, commitInfrastructure: boolean): RendererHarness {
  const renderer = Object.create(CityRenderer.prototype) as any;
  renderer.worldState = snapshot;
  renderer.trains = {
    setMovement: vi.fn(),
    begin: vi.fn(),
    finish: vi.fn(),
    batches: [],
    pickable: [],
    lamps: { meshes: [], ids: [] },
    doors: { meshes: [], ids: [] },
  };
  renderer.buses = { begin: vi.fn(), finish: vi.fn(), meshes: [], ids: [] };
  renderer.rig = { setPositions: vi.fn() };
  renderer.movementPresentation = new MovementPresentation();
  renderer.visibleInfrastructure = new VisibleInfrastructure();
  renderer.visibleInfrastructure.request(snapshot);
  if (commitInfrastructure)
    renderer.visibleInfrastructure.commit(renderer.visibleInfrastructure.capture(snapshot));
  renderer.bridgeSpans = [];
  renderer.interval = 50;
  renderer.received = performance.now();
  renderer.motionTick = 0;
  renderer.motionDelta = 0.05;
  renderer.actorSampleAt = 0;
  renderer.current = new Map();
  renderer.previous = new Map();
  renderer.umbrellaTuples = new Float32Array(2000 * 6);
  renderer.actorIds = new Map();
  renderer.pedestrians = mesh();
  renderer.heads = mesh();
  const art = new ArtMaterials();
  renderer.fleet = new VehicleFleet(art, 10);
  cleanups.push(() => {
    renderer.fleet.dispose();
    art.dispose();
  });
  return renderer as RendererHarness;
}

const carSupport: MovementSupport = { resolved: true, roads: [10], platforms: [] };
function car(id: number, x: number, z: number): Float32Array {
  return new Float32Array([id, x, z, 0, 1, -1]);
}
const doors = new Float32Array();

describe('actor presentation integration', () => {
  it('sends the same interpolated car position to the director and instance matrix', () => {
    const renderer = harness(world(), true);
    renderer.setMovement(car(42, 2, 3), 1, 360, doors, new Map([[42, carSupport]]));
    renderer.updateActors(1000);
    renderer.setMovement(car(42, 12, 13), 2, 360, doors, new Map([[42, carSupport]]));
    renderer.updateActors(1001);
    renderer.updateActors(1011);

    const matrix = actorMatrix(renderer, 42);
    const cameraTuple = renderer.rig.setPositions.mock.calls.at(-1)![0] as Float32Array;
    expect(matrix.elements[12]).toBeCloseTo(7);
    expect(matrix.elements[14]).toBeCloseTo(8);
    expect(cameraTuple[1]).toBeCloseTo(matrix.elements[12]);
    expect(cameraTuple[2]).toBeCloseTo(matrix.elements[14]);
  });

  it.each(Object.values(VEHICLE_KINDS))(
    'withholds vehicle kind %s until its road commits, then preserves its elevated surface',
    (kind) => {
      const snapshot = world();
      const renderer = harness(snapshot, false);
      const packet = new Float32Array([99, 6, 7, 0, kind, 11]);
      renderer.setMovement(packet, 1, 360, doors, new Map([[99, carSupport]]));
      renderer.updateActors(1000);
      expect(renderer.current.has(99)).toBe(false);
      expect(renderer.fleet.meshes.reduce((sum, mesh) => sum + mesh.count, 0)).toBe(0);
      expect(renderer.rig.setPositions.mock.calls.at(-1)![0]).toEqual(new Float32Array());

      renderer.visibleInfrastructure.commit(renderer.visibleInfrastructure.capture(snapshot));
      renderer.updateActors(1001);
      expect(renderer.current.get(99)?.x).toBe(6);
      expect(renderer.fleet.meshes.reduce((sum, mesh) => sum + mesh.count, 0)).toBe(1);
      const cameraTuple = renderer.rig.setPositions.mock.calls.at(-1)![0] as Float32Array;
      expect(cameraTuple.slice(0, 3)).toEqual(new Float32Array([99, 6, 7]));
      expect(actorMatrix(renderer, 99).elements[12]).toBeCloseTo(6);
      expect(actorMatrix(renderer, 99).elements[14]).toBeCloseTo(7);
      expect(actorMatrix(renderer, 99).elements[13]).toBeCloseTo(11);
      expect(cameraTuple[4]).toBe(kind);
      expect(cameraTuple[5]).toBeCloseTo(11);
    },
  );
});
