import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import { WINDOW_ATTRIBUTE } from '../web/src/render/building-surfaces';
import type { Building, Style } from '../web/src/types';

function tower(floors: number, profile: number, kind: 'office' | 'apartment' = 'office'): Building {
  return {
    id: floors * 10 + profile, parcel_id: floors * 100 + profile, x: 0, z: 0, rotation: 0,
    width: 24, depth: 20, floors, kind, stage: 'open', progress: 1, capacity: 80,
    occupancy: 35, built_day: 0, name: `${kind} tower`, appearance_seed: profile,
    history: [], anchor: false, vacant_days: 0,
  };
}

function meshes(root: THREE.Object3D): THREE.Mesh[] {
  const result: THREE.Mesh[] = [];
  root.traverse((object) => { if (object instanceof THREE.Mesh) result.push(object); });
  return result;
}

function signature(root: THREE.Object3D): string[] {
  return meshes(root).map((mesh) => {
    const position = Array.from(mesh.geometry.getAttribute('position').array).join(',');
    const index = mesh.geometry.index ? Array.from(mesh.geometry.index.array).join(',') : '';
    return `${position}|${index}`;
  }).sort();
}

function expectFiniteGeometry(root: THREE.Object3D): void {
  for (const mesh of meshes(root)) {
    const position = mesh.geometry.getAttribute('position');
    for (let i = 0; i < position.count; i++) {
      expect(Number.isFinite(position.getX(i))).toBe(true);
      expect(Number.isFinite(position.getY(i))).toBe(true);
      expect(Number.isFinite(position.getZ(i))).toBe(true);
    }
  }
}

function boxes(mesh: THREE.Mesh): Array<{ minX: number; maxX: number; minY: number; maxY: number; minZ: number; maxZ: number }> {
  const p = mesh.geometry.getAttribute('position');
  const result = [];
  for (let start = 0; start + 23 < p.count; start += 24) {
    const box = { minX: Infinity, maxX: -Infinity, minY: Infinity, maxY: -Infinity, minZ: Infinity, maxZ: -Infinity };
    for (let i = start; i < start + 24; i++) {
      box.minX = Math.min(box.minX, p.getX(i)); box.maxX = Math.max(box.maxX, p.getX(i));
      box.minY = Math.min(box.minY, p.getY(i)); box.maxY = Math.max(box.maxY, p.getY(i));
      box.minZ = Math.min(box.minZ, p.getZ(i)); box.maxZ = Math.max(box.maxZ, p.getZ(i));
    }
    result.push(box);
  }
  return result;
}

describe('tower building kit', () => {
  it('keeps tall profiles within parcel bounds and floor-count roof height', () => {
    const art = new ArtMaterials();
    for (const floors of [12, 20, 45]) {
      for (const profile of [0, 1, 2]) {
       for (const kind of ['office', 'apartment'] as const) {
        const group = buildingKit(tower(floors, profile, kind), art);
        const bounds = new THREE.Box3().setFromObject(group);
        const expectedTop = 0.58 + floors * 3.25;

        expectFiniteGeometry(group);
        expect(bounds.min.x).toBeGreaterThanOrEqual(-12.5);
        expect(bounds.max.x).toBeLessThanOrEqual(12.5);
        // The entry canopy may project a little beyond the parcel edge.
        expect(bounds.min.z).toBeGreaterThanOrEqual(-10.5);
        expect(bounds.max.z).toBeLessThanOrEqual(10.5);
        expect(bounds.max.y).toBeGreaterThan(expectedTop);
        expect(bounds.max.y).toBeLessThanOrEqual(expectedTop + 5);
        if (kind === 'apartment') {
          const vertices = meshes(group).reduce((sum, mesh) => sum + mesh.geometry.getAttribute('position').count, 0);
          expect(vertices).toBeLessThan(160_000);
        }

        // Above the podium, every vertex remains in the recessed shaft/crown,
        // including roof service equipment and the occasional aerial.
        const upperVertices: Array<{ x: number; z: number }> = [];
        for (const mesh of meshes(group)) {
          const p = mesh.geometry.getAttribute('position');
          for (let i = 0; i < p.count; i++) {
            const y = p.getY(i);
            if (y > 0.58 + 3 * 3.25 + 0.5 && y < expectedTop) upperVertices.push({ x: p.getX(i), z: p.getZ(i) });
          }
        }
        expect(upperVertices.length).toBeGreaterThan(0);
        expect(upperVertices.reduce((max, p) => Math.max(max, Math.abs(p.x)), 0)).toBeLessThan(10.5);
        expect(upperVertices.reduce((max, p) => Math.max(max, Math.abs(p.z)), 0)).toBeLessThan(9);
        disposeGroup(group);
       }
      }
    }
    art.dispose();
  }, 20_000);

  it('keeps authored geometry identical between anime and miniature material styles', () => {
    const art = new ArtMaterials();
    const building = tower(20, 2, 'apartment');
    const groups: THREE.Group[] = [];
    for (const style of ['anime', 'miniature'] as Style[]) {
      art.setStyle(style);
      groups.push(buildingKit(building, art));
    }
    expect(signature(groups[0])).toEqual(signature(groups[1]));
    groups.forEach(disposeGroup);
    art.dispose();
  });

  it('gives apartment towers opaque piers and individual bays while offices retain glass ribbons', () => {
    const art = new ArtMaterials();
    const apartment = buildingKit(tower(20, 0, 'apartment'), art);
    const office = buildingKit(tower(20, 0, 'office'), art);
    // Tower panes are window panes with rooms behind them, on each tower's light schedule.
    const glassMesh = (root: THREE.Object3D) => meshes(root).find((mesh) => (mesh.material as THREE.Material).userData.windowPane)!;
    const maxBoxWidth = (mesh: THREE.Mesh) => {
      const p = mesh.geometry.getAttribute('position');
      let max = 0;
      // Kit merges independent BoxGeometry parts in insertion order. Each box
      // contributes 24 non-indexed vertices, so measure each authored bay.
      for (let start = 0; start < p.count; start += 24) {
        let minX = Infinity, maxX = -Infinity;
        for (let i = start; i < Math.min(start + 24, p.count); i++) {
          minX = Math.min(minX, p.getX(i)); maxX = Math.max(maxX, p.getX(i));
        }
        max = Math.max(max, maxX - minX);
      }
      return max;
    };

    expect(maxBoxWidth(glassMesh(apartment))).toBeLessThan(5);
    expect(maxBoxWidth(glassMesh(office))).toBeGreaterThan(15);
    expectFiniteGeometry(apartment);
    const bounds = new THREE.Box3().setFromObject(apartment);
    expect(bounds.min.x).toBeGreaterThanOrEqual(-12.5);
    expect(bounds.max.x).toBeLessThanOrEqual(12.5);
    expect(bounds.min.z).toBeGreaterThanOrEqual(-10.5);
    expect(bounds.max.z).toBeLessThanOrEqual(10.5);
    disposeGroup(apartment); disposeGroup(office); art.dispose();
  });

  it('releases tower geometry while retaining shared art materials', () => {
    const art = new ArtMaterials();
    const group = buildingKit(tower(12, 1), art);
    const [mesh] = meshes(group);
    const geometryDisposed = vi.fn();
    const materialDisposed = vi.fn();
    mesh.geometry.addEventListener('dispose', geometryDisposed);
    const sharedMaterial = mesh.material as THREE.Material;
    sharedMaterial.addEventListener('dispose', materialDisposed);

    disposeGroup(group);

    expect(geometryDisposed).toHaveBeenCalledTimes(1);
    expect(materialDisposed).not.toHaveBeenCalled();
    art.dispose();
  });

  it('puts occupied apartment panes on all four elevations above the podium', () => {
    const art = new ArtMaterials();
    const building = tower(20, 0, 'apartment');
    const group = buildingKit(building, art);
    // Occupied towers flag every pane as lived; the shader lights rooms from that and the hour.
    const litMesh = meshes(group).find((mesh) => mesh.material === art.window('home'));
    expect(litMesh).toBeDefined();
    const info = litMesh!.geometry.getAttribute(WINDOW_ATTRIBUTE);
    for (let i = 0; i < info.count; i++) expect(info.getY(i)).toBe(1);
    const p = litMesh!.geometry.getAttribute('position');
    const shaftW = building.width * .84 * .86;
    const shaftD = building.depth * .84 * .79;
    const abovePodium = 0.58 + 3 * 3.25;
    const facades = new Set<string>();
    for (let start = 0; start < p.count; start += 24) {
      let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity, minZ = Infinity, maxZ = -Infinity;
      for (let i = start; i < start + 24; i++) {
        minX = Math.min(minX, p.getX(i)); maxX = Math.max(maxX, p.getX(i));
        minY = Math.min(minY, p.getY(i)); maxY = Math.max(maxY, p.getY(i));
        minZ = Math.min(minZ, p.getZ(i)); maxZ = Math.max(maxZ, p.getZ(i));
      }
      if ((minY + maxY) / 2 <= abovePodium) continue;
      if (minZ > shaftD / 2 + .04) facades.add('+z');
      if (maxZ < -shaftD / 2 - .04) facades.add('-z');
      if (minX > shaftW / 2 + .04) facades.add('+x');
      if (maxX < -shaftW / 2 - .04) facades.add('-x');
    }
    expect(facades).toEqual(new Set(['+z', '-z', '+x', '-x']));
    expectFiniteGeometry(group);
    disposeGroup(group); art.dispose();
  });

  it('uses tall residential sash bays and continuous seed-selected balcony columns', () => {
    const art = new ArtMaterials();
    const building = tower(20, 1, 'apartment');
    const group = buildingKit(building, art);
    const glass = art.window('home');
    const panes = boxes(meshes(group).find((mesh) => mesh.material === glass)!);
    const shaftW = building.width * .84 * .86;
    const shaftD = building.depth * .84 * .87;
    const frontPanes = panes.filter((box) => Math.abs((box.minZ + box.maxZ) / 2) > shaftD / 2 + .07 && box.maxY - box.minY > 1.8);

    expect(frontPanes.length).toBeGreaterThan(20);
    for (const pane of frontPanes) {
      expect(pane.maxY - pane.minY).toBeGreaterThan((pane.maxX - pane.minX) * 1.25);
    }

    const stone = art.get('#aab6b8', .85, false, 'ambient', 'tile');
    const stoneBoxes = boxes(meshes(group).find((mesh) => mesh.material === stone)!);
    const bayWidth = (shaftW - 1.1) / Math.max(4, Math.floor(shaftW / 3.1));
    const shaftFirstFloorCenter = 0.58 + 3 * 3.25 + .46;
    const columnsByFloor: number[][] = [];
    const shaftFloors = building.floors - 3 - 3; // profile 1 uses a three-floor crown
    for (let floor = 0; floor < shaftFloors; floor++) {
      const y = shaftFirstFloorCenter + floor * 3.25;
      const decks = stoneBoxes.filter((box) =>
        Math.abs((box.minY + box.maxY) / 2 - y) < .002 &&
        Math.abs((box.maxY - box.minY) - .12) < .002 &&
        box.maxZ - box.minZ > .8 && box.maxZ - box.minZ < 1.11 &&
        Math.abs((box.minZ + box.maxZ) / 2) > shaftD / 2 + .05,
      );
      const centers = [...new Set(decks.map((box) => Math.round((box.minX + box.maxX) * 500) / 1000))].sort((a, b) => a - b);
      columnsByFloor.push(centers);
    }
    expect(columnsByFloor.every((centers) => centers.length >= 1 && centers.length <= 2)).toBe(true);
    for (const centers of columnsByFloor.slice(1)) expect(centers).toEqual(columnsByFloor[0]);
    expect(columnsByFloor[0].length).toBeLessThan(Math.floor(shaftW / 3.1) / 2);

    const apartmentVertices = meshes(group).reduce((sum, mesh) => sum + mesh.geometry.getAttribute('position').count, 0);
    expect(apartmentVertices).toBeLessThan(160_000);
    disposeGroup(group); art.dispose();
  });
});
