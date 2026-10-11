import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildExtendedBuilding, extendedAdditionEnvelope } from '../web/src/render/extended-building';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import { buildingPalette } from '../web/src/render/building-palette';
import type { Building, BuildingOriginalFootprint, Style } from '../web/src/types';

const canvasContext = { fillRect() {}, fillText() {}, strokeRect() {}, beginPath() {}, moveTo() {}, quadraticCurveTo() {}, stroke() {}, ellipse() {}, fill() {} };
beforeAll(() => vi.stubGlobal('document', { createElement: () => ({ width: 0, height: 0, getContext: () => canvasContext }) }));
afterAll(() => vi.unstubAllGlobals());

function building(kind: 'home' | 'shop' | 'apartment', rotation: number, depth: number, extensionDepth = 0): Building {
  const originalCenter = new THREE.Vector2(17, -23);
  const expandedCenter = originalCenter.clone().add(new THREE.Vector2(-Math.sin(rotation), -Math.cos(rotation)).multiplyScalar(extensionDepth / 2));
  return {
    id: 31, parcel_id: 18, x: expandedCenter.x, z: expandedCenter.y, rotation, width: 14, depth: depth + extensionDepth,
    floors: kind === 'apartment' ? 6 : 2, kind, stage: 'open', progress: 1, capacity: 20, occupancy: 16,
    business_active: true, built_day: 3, name: `${kind} original`, appearance_seed: 1521,
    history: [], anchor: false, vacant_days: 0,
  };
}

function footprint(rotation: number, depth: number): BuildingOriginalFootprint {
  return { building_id: 31, x: 17, z: -23, width: 14, depth, rotation };
}

function meshes(root: THREE.Object3D): THREE.Mesh[] {
  const found: THREE.Mesh[] = [];
  root.traverse(o => { if (o instanceof THREE.Mesh) found.push(o); });
  return found;
}

function localGeometrySignature(root: THREE.Object3D): string[] {
  return meshes(root).map(mesh => Array.from(mesh.geometry.getAttribute('position').array as ArrayLike<number>).join(',' )).sort();
}

function worldVertices(root: THREE.Object3D): string[] {
  root.updateWorldMatrix(true, true);
  const points: string[] = [];
  root.traverse(object => {
    if (!(object instanceof THREE.Mesh)) return;
    const p = object.geometry.getAttribute('position');
    for (let i = 0; i < p.count; i++) {
      const v = new THREE.Vector3(p.getX(i), p.getY(i), p.getZ(i)).applyMatrix4(object.matrixWorld);
      points.push(`${v.x.toFixed(5)},${v.y.toFixed(5)},${v.z.toFixed(5)}`);
    }
  });
  return points.sort();
}

function retained(root: THREE.Group): THREE.Group {
  return root.children.find(child => child.userData.retainedOriginalMass) as THREE.Group;
}

function addition(root: THREE.Group): THREE.Group {
  return root.children.find(child => child.userData.extendedAddition) as THREE.Group;
}

describe('completed additive rear annex', () => {
  it('keeps original kit geometry byte-identical and world-fixed through completion and later extension', () => {
    const art = new ArtMaterials();
    const versions = [8, 12];
    for (const kind of ['home', 'shop', 'apartment'] as const) for (const rotation of [0, .63, -1.17]) {
      const original = footprint(rotation, 12);
      const current = building(kind, rotation, original.depth, versions[0]);
      const sourceDescriptor = { ...current, x: original.x, z: original.z, width: original.width, depth: original.depth };
      const before = buildingKit(sourceDescriptor, art);
      before.position.set(original.x, 0, original.z); before.rotation.y = rotation;
      const once = buildExtendedBuilding(current, original, art);
      once.position.set(current.x, 0, current.z); once.rotation.y = current.rotation;
      expect(localGeometrySignature(retained(once))).toEqual(localGeometrySignature(before));
      expect(worldVertices(retained(once))).toEqual(worldVertices(before));

      const later = building(kind, rotation, original.depth, versions[1]);
      const twice = buildExtendedBuilding(later, original, art);
      twice.position.set(later.x, 0, later.z); twice.rotation.y = later.rotation;
      expect(localGeometrySignature(retained(twice))).toEqual(localGeometrySignature(before));
      expect(worldVertices(retained(twice))).toEqual(worldVertices(before));
      expect(addition(once)).toBeDefined();
      expect(addition(twice)).toBeDefined();
      disposeGroup(before); disposeGroup(once); disposeGroup(twice);
    }
    art.dispose();
  });

  it('joins the real kit rear edge and keeps addition faces inside the expanded footprint', () => {
    const art = new ArtMaterials();
    for (const kind of ['home', 'shop', 'apartment'] as const) for (const rotation of [.2, 1.1]) {
      const original = footprint(rotation, 12), current = building(kind, rotation, original.depth, 8);
      const envelope = extendedAdditionEnvelope(current, original)!;
      expect(envelope.frontZ).toBeGreaterThan(envelope.retainedRearZ);
      expect(envelope.frontZ - envelope.retainedRearZ).toBeCloseTo(.025, 8);
      const group = buildExtendedBuilding(current, original, art);
      const newMass = addition(group);
      group.updateMatrixWorld(true);
      const inverse = group.matrixWorld.clone().invert();
      let minY = Infinity;
      for (const mesh of meshes(newMass)) {
        const p = mesh.geometry.getAttribute('position');
        for (let i = 0; i < p.count; i++) {
          const v = new THREE.Vector3(p.getX(i), p.getY(i), p.getZ(i)).applyMatrix4(mesh.matrixWorld).applyMatrix4(inverse);
          minY = Math.min(minY, v.y);
          expect(Math.abs(v.x)).toBeLessThanOrEqual(current.width / 2 + 1e-6);
          expect(Math.abs(v.z)).toBeLessThanOrEqual(current.depth / 2 + 1e-6);
        }
      }
      expect(minY).toBeCloseTo(0, 6);
      if (kind === 'apartment') {
        const source = retained(group);
        source.updateWorldMatrix(true, true);
        const wall = art.get(buildingPalette(current, art.palette).wall);
        let measuredRearWall = Infinity;
        for (const mesh of meshes(source).filter(mesh => mesh.material === wall)) {
          const p = mesh.geometry.getAttribute('position');
          for (let i = 0; i < p.count; i++) {
            const v = new THREE.Vector3(p.getX(i), p.getY(i), p.getZ(i)).applyMatrix4(mesh.matrixWorld);
            if (Math.abs(v.y - .58) < 1e-5) measuredRearWall = Math.min(measuredRearWall, v.z);
          }
        }
        expect(measuredRearWall).toBeLessThan(Infinity);
        expect(envelope.frontZ).toBeCloseTo(measuredRearWall + .025, 6);
      }
      disposeGroup(group);
    }
    art.dispose();
  });

  it('uses the ordinary building kit when no original footprint exists and keeps style geometry shared', () => {
    const art = new ArtMaterials();
    const current = building('home', .5, 12, 8), original = footprint(.5, 12);
    art.setStyle('anime' satisfies Style);
    const anime = buildExtendedBuilding(current, original, art);
    const animeSignature = localGeometrySignature(anime);
    const noHistory = buildExtendedBuilding(current, undefined, art);
    const ordinary = buildingKit(current, art);
    expect(localGeometrySignature(noHistory)).toEqual(localGeometrySignature(ordinary));
    art.setStyle('miniature');
    const miniature = buildExtendedBuilding(current, original, art);
    expect(localGeometrySignature(miniature)).toEqual(animeSignature);
    expect(meshes(addition(miniature))[0].material).not.toBe(meshes(addition(anime))[0].material);
    [anime, noHistory, ordinary, miniature].forEach(disposeGroup);
    art.dispose();
  });

  it('lights residential addition windows only for occupied open hosts', () => {
    const art = new ArtMaterials(), original = footprint(.4, 12);
    const occupied = buildExtendedBuilding(building('home', .4, 12, 8), original, art);
    const vacantHost = { ...building('home', .4, 12, 8), occupancy: 0 };
    const vacant = buildExtendedBuilding(vacantHost, original, art);
    const hasEmissiveAdditionMaterial = (group: THREE.Group) => meshes(addition(group)).some(mesh => {
      const material = Array.isArray(mesh.material) ? mesh.material[0] : mesh.material;
      return material.userData.artBuildingClass?.emissive === true;
    });
    expect(hasEmissiveAdditionMaterial(occupied)).toBe(true);
    expect(hasEmissiveAdditionMaterial(vacant)).toBe(false);
    [occupied, vacant].forEach(disposeGroup);
    art.dispose();
  });
});
