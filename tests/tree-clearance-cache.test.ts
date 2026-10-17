import { LandscapeLayer } from '../web/src/render/landscape-layer';
import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { CityRenderer } from '../web/src/render/city';
import type { World } from '../web/src/types';

function world(overrides: Partial<World> = {}): World {
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
    nodes: [],
    roads: [],
    parcels: [],
    buildings: [],
    projects: [],
    trains: [],
    events: [],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
    ...overrides,
  };
}

function building(x: number, z: number) {
  return {
    id: 1,
    parcel_id: 1,
    x,
    z,
    rotation: 0,
    width: 10,
    depth: 10,
    floors: 1,
    kind: 'home' as const,
    stage: 'open' as const,
    progress: 1,
    capacity: 1,
    occupancy: 1,
    built_day: 0,
    name: 'Home',
    appearance_seed: 1,
    history: [],
    anchor: false,
    vacant_days: 0,
  };
}

function harness(
  points = [
    { x: 0, z: 0, s: 2 },
    { x: 20, z: 0, s: 2 },
  ],
) {
  const geometry = new THREE.BoxGeometry(1, 1, 1);
  const material = new THREE.MeshBasicMaterial();
  const trees = new THREE.InstancedMesh(geometry, material, points.length);
  const trunks = new THREE.InstancedMesh(geometry, material, points.length);
  trees.computeBoundingSphere = vi.fn(trees.computeBoundingSphere.bind(trees));
  trunks.computeBoundingSphere = vi.fn(trunks.computeBoundingSphere.bind(trunks));
  const renderer = Object.create(LandscapeLayer.prototype) as LandscapeLayer;
  Object.assign(renderer, {
    treePoints: points,
    treeClearanceKey: '',
    treeBatches: [
      { indices: points.map((_, i) => i), trees, trunks, center: new THREE.Vector3(), far: false },
    ],
  });
  return { renderer, trees, trunks };
}

function matrixAt(mesh: THREE.InstancedMesh, index: number) {
  const matrix = new THREE.Matrix4();
  mesh.getMatrixAt(index, matrix);
  return matrix.elements;
}

function hiddenAt(mesh: THREE.InstancedMesh, index: number) {
  return matrixAt(mesh, index)
    .slice(0, 12)
    .every((value) => value === 0);
}

afterEach(() => vi.restoreAllMocks());

describe('tree clearance cache', () => {
  it('clears station reservations in rotated frames and invalidates moved geometry', () => {
    for (const [ux, uz] of [
      [1, 0],
      [-1, 0],
      [0.6, 0.8],
    ]) {
      for (const gateway of [false, true]) {
        const platform = {
          node_id: 1,
          x: 30,
          z: -10,
          ux,
          uz,
          surface_y: 8,
          stair_x: 4.05,
          stair_bottom_z: gateway ? 67 : -20,
          stair_top_z: gateway ? 48 : -1,
          ground_y: 0.27,
          stair_steps: 44,
          ground_path: [
            { x: 0, z: 0 },
            { x: 7.5, z: -1.5 },
            { x: 7.5, z: gateway ? 68.5 : -21.5 },
            { x: 4.05, z: gateway ? 68.5 : -21.5 },
            { x: 4.05, z: gateway ? 67 : -20 },
          ],
          ground_width: 1.6,
          construction_phase: 0,
        };
        const point = (x: number, z: number) => ({
          x: 30 + uz * x + ux * z,
          z: -10 - ux * x + uz * z,
          s: 1,
        });
        const { renderer, trees } = harness([
          point(8, 24),
          point(4.05, gateway ? 60 : -12),
          point(25, 24),
        ]);
        LandscapeLayer.prototype['updateTrees'].call(
          renderer,
          world({ rail_platforms: [platform] }),
        );
        expect(hiddenAt(trees, 0)).toBe(true);
        expect(hiddenAt(trees, 1)).toBe(true);
        expect(hiddenAt(trees, 2)).toBe(false);
        const version = trees.instanceMatrix.version;
        LandscapeLayer.prototype['updateTrees'].call(
          renderer,
          world({ rail_platforms: [{ ...platform, construction_phase: 4 }] }),
        );
        expect(trees.instanceMatrix.version).toBe(version);
        LandscapeLayer.prototype['updateTrees'].call(
          renderer,
          world({ rail_platforms: [{ ...platform, x: 400 }] }),
        );
        expect(trees.instanceMatrix.version).toBeGreaterThan(version);
        expect(hiddenAt(trees, 0)).toBe(false);
        expect(hiddenAt(trees, 1)).toBe(false);
      }
    }
  });

  it('skips matrix uploads and bounds work when clearance inputs are unchanged', () => {
    const { renderer, trees, trunks } = harness();
    LandscapeLayer.prototype['updateTrees'].call(renderer, world({ buildings: [building(0, 0)] }));
    const treeVersion = trees.instanceMatrix.version,
      trunkVersion = trunks.instanceMatrix.version;
    vi.mocked(trees.computeBoundingSphere).mockClear();
    vi.mocked(trunks.computeBoundingSphere).mockClear();

    LandscapeLayer.prototype['updateTrees'].call(
      renderer,
      world({
        tick: 25,
        revision: 99,
        weather: { kind: 'rain', intensity: 0.7 },
        buildings: [{ ...building(0, 0), progress: 0.5, floors: 8 }],
      }),
    );

    expect(trees.instanceMatrix.version).toBe(treeVersion);
    expect(trunks.instanceMatrix.version).toBe(trunkVersion);
    expect(trees.computeBoundingSphere).not.toHaveBeenCalled();
    expect(trunks.computeBoundingSphere).not.toHaveBeenCalled();
  });

  it('recomputes hidden and visible matrices for same-count building and ground geometry changes', () => {
    const { renderer, trees, trunks } = harness();
    LandscapeLayer.prototype['updateTrees'].call(renderer, world({ buildings: [building(0, 0)] }));
    expect(hiddenAt(trees, 0)).toBe(true);
    expect(hiddenAt(trees, 1)).toBe(false);
    const initialVersion = trees.instanceMatrix.version;

    LandscapeLayer.prototype['updateTrees'].call(renderer, world({ buildings: [building(20, 0)] }));
    expect(trees.instanceMatrix.version).toBeGreaterThan(initialVersion);
    expect(hiddenAt(trees, 0)).toBe(false);
    expect(hiddenAt(trees, 1)).toBe(true);
    expect(hiddenAt(trunks, 0)).toBe(false);
    expect(hiddenAt(trunks, 1)).toBe(true);

    const buildingVersion = trees.instanceMatrix.version;
    LandscapeLayer.prototype['updateTrees'].call(
      renderer,
      world({
        ground: [
          {
            block: 1,
            points: [
              [15, -5],
              [25, -5],
              [25, 5],
              [15, 5],
            ],
          },
        ],
      }),
    );
    expect(trees.instanceMatrix.version).toBeGreaterThan(buildingVersion);
    expect(hiddenAt(trees, 0)).toBe(false);
    expect(hiddenAt(trees, 1)).toBe(true);
  });

  it('rebuilds fresh tree point buffers after regeneration resets the clearance key', () => {
    const { renderer, trees } = harness();
    LandscapeLayer.prototype['updateTrees'].call(renderer, world());
    const firstMatrix = new THREE.Matrix4();
    trees.getMatrixAt(0, firstMatrix);

    Object.assign(renderer, {
      treePoints: [
        { x: 100, z: 30, s: 3 },
        { x: 120, z: 30, s: 3 },
      ],
      treeClearanceKey: '',
    });
    LandscapeLayer.prototype['updateTrees'].call(renderer, world());

    const regeneratedMatrix = new THREE.Matrix4();
    trees.getMatrixAt(0, regeneratedMatrix);
    expect(regeneratedMatrix.elements[12]).toBe(100);
    expect(regeneratedMatrix.elements[14]).toBe(30);
    expect(regeneratedMatrix.elements).not.toEqual(firstMatrix.elements);
  });
});
