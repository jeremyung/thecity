import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { BuildingAccessPaths } from '../web/src/render/building-access';
import { ArtMaterials } from '../web/src/render/materials';
import type { Building, World } from '../web/src/types';

function building(id: number, stage = 'open'): Building {
  return {
    id, parcel_id: id + 100, x: 400 + id, z: -200, rotation: 0, width: 18, depth: 14,
    floors: 2, kind: 'shop', stage, progress: 1, capacity: 12, occupancy: 4, built_day: 0,
    name: `Shop ${id}`, appearance_seed: id, history: [], anchor: false, vacant_days: 0,
  };
}

function world(buildings: Building[], buildingAccess?: World['building_access']): World {
  return {
    schema_version: 1, engine_version: 'test', seed: 41, tick: 0, day: 0, minute_of_day: 360,
    development_rate: 1, revision: 0, stage: 'Founding',
    bounds: { min_x: -400, max_x: 400, min_z: -400, max_z: 400 },
    terrain: { river: [], protected: [], gateways: [] }, weather: { kind: 'clear', intensity: 0 },
    nodes: [], roads: [], parcels: [], buildings, projects: [], trains: [], events: [],
    ...(buildingAccess === undefined ? {} : { building_access: buildingAccess }),
    stats: { population: 0, active_travelers: 0, buildings: buildings.length, active_projects: 0 },
  };
}

function meshes(root: THREE.Object3D): THREE.Mesh[] {
  const found: THREE.Mesh[] = [];
  root.traverse((object) => { if (object instanceof THREE.Mesh) found.push(object); });
  return found;
}

describe('BuildingAccessPaths', () => {
  it('joins shared streets at carriageway height and rebuilds when the road becomes arterial', () => {
    const art = new ArtMaterials();
    const paths = new BuildingAccessPaths(art, release => release());
    const state = world([building(1)], [{ building_id: 1, road_id: 9,
      points: [{ x: 0, z: 0 }, { x: 0, z: 10 }] }]);
    state.roads = [{ id: 9, from: 1, to: 2, kind: 'local', stage: 'open',
      progress: 1, connected: true, length: 20, lanes: 1 }];
    const landingTop = () => {
      const p = meshes(paths.root)[0].geometry.getAttribute('position');
      let top = -Infinity;
      for (let i = 0; i < p.count; i++) if (p.getZ(i) > 10.5) top = Math.max(top, p.getY(i));
      return top;
    };
    for (const kind of ['local', 'lane']) {
      state.roads[0].kind = kind;
      paths.setWorld(state);
      expect(landingTop()).toBeCloseTo(.305, 5);
    }
    const shared = paths.root.children[0];
    state.roads[0].kind = 'arterial';
    paths.setWorld(state);
    expect(paths.root.children[0]).not.toBe(shared);
    expect(landingTop()).toBeCloseTo(.475, 5);
    paths.reset(); art.dispose();
  });

  it('follows a bent passage without paving a diagonal through the front lot', () => {
    const art = new ArtMaterials();
    const paths = new BuildingAccessPaths(art, release => release());
    paths.setWorld(world([building(1)], [{ building_id: 1, road_id: 9,
      points: [{ x: 0, z: 0 }, { x: 0, z: 10 }, { x: 10, z: 10 }] }]));
    const [mesh] = meshes(paths.root);
    expect(meshes(paths.root)).toHaveLength(1);
    const geometry = mesh.geometry.toNonIndexed();
    const p = geometry.getAttribute('position');
    let reachesBend = false, reachesCurb = false;
    for (let i = 0; i < p.count; i += 3) {
      const x = (p.getX(i) + p.getX(i + 1) + p.getX(i + 2)) / 3;
      const z = (p.getZ(i) + p.getZ(i + 1) + p.getZ(i + 2)) / 3;
      expect(Math.abs(x) < .95 || Math.abs(z - 10) < .95).toBe(true);
      if (z > 9 && x < 1) reachesBend = true;
      if (x > 9) reachesCurb = true;
      if (z < 8) expect(Math.max(p.getY(i), p.getY(i + 1), p.getY(i + 2))).toBeLessThan(.06);
    }
    expect(reachesBend && reachesCurb).toBe(true);
    geometry.dispose(); paths.reset(); art.dispose();
  });

  it('places a diagonal path from authoritative door-to-curb points and merges same-material paths', () => {
    const art = new ArtMaterials();
    const retired: Array<() => void> = [];
    const paths = new BuildingAccessPaths(art, (release) => retired.push(release));
    const buildings = Array.from({ length: 48 }, (_, i) => building(i + 1));
    const diagonal = [{ building_id: buildings[0].id, road_id: 700, points: [{ x: 1, z: 2 }, { x: 13, z: 22 }] as const }];
    paths.setWorld(world(buildings, diagonal));
    const pathMeshes = meshes(paths.root);
    expect(pathMeshes).toHaveLength(1);
    const [mesh] = pathMeshes;
    const geometry = mesh.geometry;
    const length = Math.hypot(12, 20);
    const axis = new THREE.Vector2(12 / length, 20 / length);
    const side = new THREE.Vector2(-axis.y, axis.x);
    const positions = geometry.getAttribute('position');
    const along: number[] = [], lateral: number[] = [], rampLateral: number[] = [];
    for (let i = 0; i < positions.count; i++) {
      const x = positions.getX(i) - 1;
      const z = positions.getZ(i) - 2;
      const distance = x * axis.x + z * axis.y;
      const offset = x * side.x + z * side.y;
      along.push(distance);
      lateral.push(offset);
      if (distance < length - .91) rampLateral.push(offset);
    }
    // Both authoritative endpoints fall on the rendered access path. The ramp
    // overlaps the door slightly and the level curb landing extends 0.9m past it.
    expect(Math.min(...along)).toBeCloseTo(-.09, 1);
    expect(Math.max(...along)).toBeCloseTo(length + .9, 1);
    expect(Math.min(...along)).toBeLessThan(0);
    expect(Math.max(...along)).toBeGreaterThan(length);
    expect(Math.max(...rampLateral.map(Math.abs))).toBeLessThan(.94);
    expect(Math.max(...lateral) - Math.min(...lateral)).toBeLessThan(1.9);

    const manyPaths = buildings.map((b, i) => ({
      building_id: b.id, road_id: 800 + i,
      points: [{ x: i * 3, z: 40 }, { x: i * 3 + 12, z: 60 }] as const,
    }));
    paths.setWorld(world(buildings, manyPaths));
    expect(meshes(paths.root)).toHaveLength(1);

    paths.reset();
    retired.forEach((release) => release());
    art.dispose();
  });

  it('renders no paths when the contract is unavailable and filters buildings still under construction', () => {
    const art = new ArtMaterials();
    const retired: Array<() => void> = [];
    const paths = new BuildingAccessPaths(art, (release) => retired.push(release));
    const open = building(1, 'open');
    const construction = building(2, 'structure');
    const access = [
      { building_id: open.id, road_id: 30, points: [{ x: 2, z: 3 }, { x: 4, z: 5 }] as const },
      { building_id: construction.id, road_id: 31, points: [{ x: 10, z: 11 }, { x: 12, z: 13 }] as const },
    ];

    paths.setWorld(world([open]));
    expect(meshes(paths.root)).toHaveLength(0);
    paths.setWorld(world([open, construction], access));
    expect(meshes(paths.root)).toHaveLength(1);

    paths.setWorld(world([open]));
    expect(meshes(paths.root)).toHaveLength(0);
    paths.reset();
    retired.forEach((release) => release());
    art.dispose();
  });

  it('reuses unchanged geometry and retires replaced or removed geometry without disposing shared art materials', () => {
    const art = new ArtMaterials();
    const retired: Array<() => void> = [];
    const paths = new BuildingAccessPaths(art, (release) => retired.push(release));
    const shop = building(1);
    const firstWorld = world([shop], [{ building_id: shop.id, road_id: 90, points: [{ x: 2, z: 3 }, { x: 18, z: 9 }] }]);
    paths.setWorld(firstWorld);
    retired.splice(0).forEach((release) => release()); // Release the initial empty group.

    const initialGroup = paths.root.children[0];
    const initialGeometry = meshes(initialGroup)[0].geometry;
    const initialGeometryDisposed = vi.fn();
    initialGeometry.addEventListener('dispose', initialGeometryDisposed);
    const sharedMaterial = meshes(initialGroup)[0].material as THREE.Material;
    const sharedMaterialDisposed = vi.fn();
    sharedMaterial.addEventListener('dispose', sharedMaterialDisposed);

    paths.setWorld({ ...firstWorld, revision: 1 });
    expect(paths.root.children[0]).toBe(initialGroup);
    expect(retired).toHaveLength(0);

    const replacement = world([shop], [{ building_id: shop.id, road_id: 90, points: [{ x: 2, z: 3 }, { x: 26, z: 15 }] }]);
    paths.setWorld(replacement);
    expect(paths.root.children[0]).not.toBe(initialGroup);
    expect(initialGeometryDisposed).not.toHaveBeenCalled();
    expect(sharedMaterialDisposed).not.toHaveBeenCalled();
    retired.shift()!();
    expect(initialGeometryDisposed).toHaveBeenCalledTimes(1);
    expect(sharedMaterialDisposed).not.toHaveBeenCalled();

    const replacementGroup = paths.root.children[0];
    const replacementGeometry = meshes(replacementGroup)[0].geometry;
    const replacementDisposed = vi.fn();
    replacementGeometry.addEventListener('dispose', replacementDisposed);
    paths.setWorld(world([shop])); // Removal of the access record.
    expect(meshes(paths.root)).toHaveLength(0);
    expect(replacementDisposed).not.toHaveBeenCalled();
    retired.splice(0).forEach((release) => release());
    expect(replacementDisposed).toHaveBeenCalledTimes(1);
    expect(sharedMaterialDisposed).not.toHaveBeenCalled();

    paths.reset();
    retired.splice(0).forEach((release) => release());
    art.dispose();
    expect(sharedMaterialDisposed).toHaveBeenCalledTimes(1);
  });
});
