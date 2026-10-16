import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import { CameraDirector } from '../web/src/camera/director';
import { cameraPathClearance } from '../web/src/camera/clearance';
import type { World } from '../web/src/types';

const controlsMocks = vi.hoisted(() => ({ instances: [] as unknown[] }));
vi.mock('three/addons/controls/OrbitControls.js', async () => {
  const Three = await import('three');
  class MockOrbitControls {
    target = new Three.Vector3();
    enableDamping = false;
    dampingFactor = 0;
    screenSpacePanning = true;
    minDistance = 0;
    maxDistance = Infinity;
    minPolarAngle = 0;
    maxPolarAngle = Math.PI;
    private listeners = new Map<string, Set<(event: Event) => void>>();
    constructor(
      readonly camera: THREE.Camera,
      _element: HTMLElement,
    ) {
      controlsMocks.instances.push(this);
    }
    addEventListener(type: string, listener: (event: Event) => void) {
      const listeners = this.listeners.get(type) ?? new Set();
      listeners.add(listener);
      this.listeners.set(type, listeners);
    }
    removeEventListener(type: string, listener: (event: Event) => void) {
      this.listeners.get(type)?.delete(listener);
    }
    update() {
      return false;
    }
    dispose() {}
  }
  return { OrbitControls: MockOrbitControls };
});

function worldWithStation(): World {
  return {
    schema_version: 1,
    engine_version: 'test',
    seed: 87231,
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
        { x: 30, z: 0 },
      ],
      protected: [],
      gateways: [],
    },
    weather: { kind: 'clear', intensity: 0 },
    nodes: [{ id: 42, kind: 'station', x: 80, z: -30 }],
    roads: [],
    parcels: [],
    buildings: [],
    projects: [],
    trains: [],
    events: [],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
  };
}
function office(
  id: number,
  x: number,
  z: number,
  width: number,
  depth: number,
  floors: number,
): World['buildings'][number] {
  return {
    id,
    parcel_id: id + 1000,
    x,
    z,
    rotation: 0,
    width,
    depth,
    floors,
    kind: 'office',
    stage: 'open',
    progress: 1,
    capacity: 24,
    occupancy: 15,
    built_day: 0,
    name: `Tower ${id}`,
    appearance_seed: id,
    history: [],
    anchor: false,
    vacant_days: 0,
  };
}
function director() {
  return new CameraDirector(
    new THREE.PerspectiveCamera(42, 1, 0.1, 2000),
    Object.assign(new EventTarget(), { tabIndex: -1 }) as unknown as HTMLElement,
  );
}
beforeEach(() => {
  controlsMocks.instances = [];
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

describe('CameraDirector.reviewStation', () => {
  it('returns the same held station framing from different prior poses and preserves lens', () => {
    const world = worldWithStation(),
      first = director(),
      second = director();
    first.world(world);
    second.world(world);
    first.setLens('detail');
    second.setLens('detail');
    second.setMode('overview');
    second.update(0.8, 1);
    second.camera.position.set(-200, 90, 140);
    second.target.set(0, 0, 0);
    expect(first.reviewStation(42)).toBe(true);
    expect(second.reviewStation(42)).toBe(true);
    expect(first.camera.position.toArray()).toEqual(second.camera.position.toArray());
    expect(first.target.toArray()).toEqual(second.target.toArray());
    expect(first.target.x).toBe(80);
    expect(first.target.y).toBeCloseTo(6.935);
    expect(first.target.z).toBe(-30);
    expect(first.lens).toBe('detail');
    expect(first.mode).toBe('explore');
    expect(first.label).toBe('Explore');
    const p = first.camera.position.clone(),
      t = first.target.clone();
    for (let i = 0; i < 20; i++) first.update(0.1, i * 0.1);
    expect(first.camera.position.distanceTo(p)).toBeLessThan(1e-8);
    expect(first.target.distanceTo(t)).toBeLessThan(1e-8);
    first.dispose();
    second.dispose();
  });
  it('uses the platform center and height when the world provides one', () => {
    const world = worldWithStation();
    world.rail_platforms = [
      {
        node_id: 42,
        x: 84,
        z: -27,
        ux: 1,
        uz: 0,
        surface_y: 8.2,
        stair_x: 0,
        stair_bottom_z: 0,
        stair_top_z: 0,
        ground_y: 0,
        stair_steps: 8,
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
    ];
    const d = director();
    d.world(world);
    expect(d.reviewStation(42)).toBe(true);
    expect(d.target.toArray()).toEqual([84, 8.2, -27]);
    d.dispose();
  });
  it('chooses a clear station sightline around a nearby building that blocks a close follow view', () => {
    const world = worldWithStation();
    // The old node follow looked from this direction; this tower sits directly
    // between that close camera position and the station.
    world.buildings = [office(70, 97, -12, 26, 20, 22)];
    const d = director();
    d.world(world);
    const oldCloseFollow = new THREE.Vector3(80, 6.935, -30).add(
      new THREE.Vector3(0.65, 0.28, 0.7).normalize().multiplyScalar(95),
    );
    const obstacles = (
      d as unknown as { cameraObstacles: Parameters<typeof cameraPathClearance>[2] }
    ).cameraObstacles;
    expect(
      cameraPathClearance(new THREE.Vector3(80, 6.935, -30), oldCloseFollow, obstacles, {
        terrainY: 0,
        safetyMarginM: 0.3,
      }).collides,
    ).toBe(true);
    expect(d.reviewStation(42)).toBe(true);
    expect(
      cameraPathClearance(d.target, d.camera.position, obstacles, {
        terrainY: 0,
        safetyMarginM: 0.3,
      }).collides,
    ).toBe(false);
    expect(
      cameraPathClearance(d.camera.position, d.camera.position, obstacles, {
        terrainY: 0,
        safetyMarginM: 1.5,
      }).collides,
    ).toBe(false);
    expect(Math.hypot(d.camera.position.x - 97, d.camera.position.z + 12)).toBeGreaterThan(15);
    d.dispose();
  });
  it('rejects an infeasible station view without mutating the existing pose', () => {
    const world = worldWithStation();
    world.buildings = [office(71, 80, -30, 180, 180, 200)];
    const d = director();
    d.world(world);
    d.setMode('overview');
    d.update(0.2, 0.2);
    const pos = d.camera.position.clone(),
      target = d.target.clone(),
      mode = d.mode,
      label = d.label,
      lens = d.lens;
    expect(d.reviewStation(42)).toBe(false);
    expect(d.camera.position.toArray()).toEqual(pos.toArray());
    expect(d.target.toArray()).toEqual(target.toArray());
    expect(d.mode).toBe(mode);
    expect(d.label).toBe(label);
    expect(d.lens).toBe(lens);
    d.dispose();
  });
  it('returns false for an absent station without changing the current view', () => {
    const d = director();
    d.world(worldWithStation());
    d.setMode('overview');
    d.update(0.2, 0.2);
    const pos = d.camera.position.clone(),
      target = d.target.clone(),
      mode = d.mode,
      label = d.label,
      lens = d.lens;
    expect(d.reviewStation(999)).toBe(false);
    expect(d.camera.position.toArray()).toEqual(pos.toArray());
    expect(d.target.toArray()).toEqual(target.toArray());
    expect(d.mode).toBe(mode);
    expect(d.label).toBe(label);
    expect(d.lens).toBe(lens);
    d.dispose();
  });
});
