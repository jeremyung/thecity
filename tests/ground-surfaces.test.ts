import { ActorsLayer } from '../web/src/render/actors-layer';
import { LandscapeLayer } from '../web/src/render/landscape-layer';
import { RoadsLayer } from '../web/src/render/roads-layer';
import * as THREE from 'three/webgpu';
import { describe, expect, it, vi } from 'vitest';
import { CityRenderer } from '../web/src/render/city';
import { buildGroundSurfaces } from '../web/src/render/ground-surfaces';
import type { World } from '../web/src/types';

function world(ground: NonNullable<World['ground']>, progress = 0): World {
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
    terrain: { river: [], protected: [], gateways: [] },
    weather: { kind: 'clear', intensity: 0 },
    nodes: [],
    roads: [
      {
        id: 1,
        from: 1,
        to: 2,
        kind: 'local',
        stage: 'building',
        progress,
        connected: false,
        length: 10,
        lanes: 1,
      },
    ],
    parcels: [],
    buildings: [],
    ground,
    projects: [],
    trains: [],
    events: [],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
  };
}

const patch = (x: number, block = 0) => ({
  block,
  points: [
    [x, 0],
    [x + 8, 0],
    [x + 8, 8],
    [x, 8],
  ] as [number, number][],
});

function rendererHarness() {
  const renderer = Object.create(CityRenderer.prototype) as any;
  renderer.actors = Object.assign(Object.create(ActorsLayer.prototype), { host: renderer });
  renderer.landscape = new LandscapeLayer(renderer);
  renderer.roadsLayer = new RoadsLayer(renderer);
  renderer.worldState = null;
  renderer.hour = 0;
  renderer.rain = 0;
  renderer.terrainSeed = 17;
  renderer.worldTick = 0;
  renderer.roadsLayer.roadSignature = '';
  renderer.groundSignature = '';
  renderer.scene = new THREE.Scene();
  renderer.ground = new THREE.Group();
  renderer.scene.add(renderer.ground);
  renderer.buildings = new Map();
  renderer.objectQueue = new Map();
  renderer.chunks = { remove: vi.fn() };
  const materials = new Map<string, THREE.Material>();
  renderer.art = {
    get: (tone: string) =>
      materials.get(tone) ??
      (materials.set(tone, new THREE.MeshBasicMaterial({ color: tone })), materials.get(tone)!),
  };
  // This harness isolates road/ground publication; park geometry and seasonal
  // painting have their own tests and need the full foliage/material setup.
  renderer.landscape.rebuildParks = vi.fn();
  renderer.landscape.paintSeason = vi.fn();
  renderer.rig = { world: vi.fn() };
  renderer.sidewalks = { setWorld: vi.fn() };
  renderer.actors.buses = { setWorld: vi.fn() };
  renderer.actors.trains = { setWorld: vi.fn() };
  renderer.layers = [];
  renderer.landscape.updateTrees = vi.fn();
  renderer.roadsLayer.rebuildRoads = vi.fn();
  renderer.pendingCompiles = 0;
  renderer.retiredResources = [];
  return renderer as Pick<CityRenderer, 'setWorld'> & {
    roadsLayer: { rebuildRoads: ReturnType<typeof vi.fn> };
    updateTrees: ReturnType<typeof vi.fn>;
  };
}

describe('progressive ground renderer isolation', () => {
  it('updates moved ground polygons at the same count without rebuilding roads', () => {
    const renderer = rendererHarness();
    renderer.setWorld(world([patch(0)]));
    const roads = renderer.roadsLayer.rebuildRoads;
    roads.mockClear();
    const previous = (renderer as any).ground;

    renderer.setWorld(world([patch(24)]));

    expect((renderer as any).ground).not.toBe(previous);
    expect(renderer.roadsLayer.rebuildRoads).not.toHaveBeenCalled();
    expect((renderer as any).ground.children[0].geometry.boundingBox).toBeNull();
    (renderer as any).ground.children[0].geometry.computeBoundingBox();
    expect((renderer as any).ground.children[0].geometry.boundingBox!.min.x).toBe(24);
  });

  it('retains the ground group for an identical snapshot', () => {
    const renderer = rendererHarness();
    const snapshot = world([patch(0)]);
    renderer.setWorld(snapshot);
    const ground = (renderer as any).ground;

    renderer.setWorld({ ...snapshot, tick: 2 });

    expect((renderer as any).ground).toBe(ground);
  });

  it('rebuilds roads on road progress while retaining unchanged ground', () => {
    const renderer = rendererHarness();
    renderer.setWorld(world([patch(0)], 0.1));
    const ground = (renderer as any).ground;
    renderer.roadsLayer.rebuildRoads.mockClear();

    renderer.setWorld(world([patch(0)], 0.2));

    expect(renderer.roadsLayer.rebuildRoads).toHaveBeenCalledOnce();
    expect((renderer as any).ground).toBe(ground);
  });
});

describe('ground surface geometry', () => {
  it('places polygon surfaces at the configured height and batches patches by at most four materials', () => {
    const materials = new Map<string, THREE.Material>();
    const art = {
      get: (tone: string) =>
        materials.get(tone) ??
        (materials.set(tone, new THREE.MeshBasicMaterial({ color: tone })), materials.get(tone)!),
    } as any;
    const group = buildGroundSurfaces(
      Array.from({ length: 12 }, (_, i) => patch(i * 10, i)),
      art,
    );

    expect(group.children).toHaveLength(4);
    for (const child of group.children as THREE.Mesh[]) {
      child.geometry.computeBoundingBox();
      expect(child.geometry.boundingBox!.min.y).toBeCloseTo(0.035);
      expect(child.geometry.boundingBox!.max.y).toBeCloseTo(0.035);
    }
    for (const child of group.children as THREE.Mesh[]) child.geometry.dispose();
    group.clear();
  });

  it('disposes replaced ground geometry when a changed snapshot arrives', () => {
    const renderer = rendererHarness();
    renderer.setWorld(world([patch(0)]));
    const oldGround = (renderer as any).ground as THREE.Group;
    const geometry = (oldGround.children[0] as THREE.Mesh).geometry;
    const disposed = vi.fn();
    geometry.addEventListener('dispose', disposed);

    renderer.setWorld(world([patch(16)]));

    expect(disposed).toHaveBeenCalledOnce();
  });
});
