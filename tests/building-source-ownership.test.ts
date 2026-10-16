import * as THREE from 'three/webgpu';
import { describe, expect, it, vi } from 'vitest';
import { BuildingChunks, type RebuildableBuildingSource } from '../web/src/render/building-chunks';

function groupAt(
  x: number,
  material: THREE.Material,
  owned = false,
  largeVertices = 0,
  largeMaterial = material,
): { group: THREE.Group; geometries: THREE.BufferGeometry[]; texture?: THREE.Texture } {
  const group = new THREE.Group();
  group.position.x = x;
  const geometries: THREE.BufferGeometry[] = [];
  let texture: THREE.Texture | undefined;
  if (owned) {
    texture = new THREE.DataTexture(new Uint8Array([255, 255, 255, 255]), 1, 1);
    texture.needsUpdate = true;
    const signMaterial = new THREE.MeshBasicMaterial({ map: texture });
    const sign = new THREE.Mesh(new THREE.BoxGeometry(2, 2, 0.2), signMaterial);
    sign.userData.ownedMaterial = true;
    sign.userData.ownedTexture = texture;
    group.add(sign);
    geometries.push(sign.geometry);
  } else {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(2, 3, 4), material);
    group.add(mesh);
    geometries.push(mesh.geometry);
  }
  if (largeVertices) {
    const geometry = new THREE.BufferGeometry(),
      positions = new Float32Array(largeVertices * 3),
      indices = new Uint32Array(largeVertices);
    const normals = new Float32Array(largeVertices * 3),
      uv = new Float32Array(largeVertices * 2);
    for (let i = 0; i < largeVertices; i++) {
      positions[i * 3] = i % 100;
      positions[i * 3 + 1] = Math.floor(i / 100) % 100;
      indices[i] = i;
    }
    for (let i = 0; i < largeVertices; i++) normals[i * 3 + 1] = 1;
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
    geometry.setAttribute('uv', new THREE.BufferAttribute(uv, 2));
    geometry.setIndex(new THREE.BufferAttribute(indices, 1));
    group.add(new THREE.Mesh(geometry, largeMaterial));
    geometries.push(geometry);
  }
  return { group, geometries, texture };
}

function descriptor(
  make: () => ReturnType<typeof groupAt>,
  disposed: THREE.Group[] = [],
): RebuildableBuildingSource {
  return {
    createGroup: () => make().group,
    disposeGroup(group) {
      disposed.push(group);
      group.traverse((object) => {
        if (!(object instanceof THREE.Mesh)) return;
        object.geometry.dispose();
        if (object.userData.ownedMaterial) {
          for (const material of Array.isArray(object.material)
            ? object.material
            : [object.material])
            material.dispose();
          delete object.userData.ownedMaterial;
        }
        if (object.userData.ownedTexture) {
          (object.userData.ownedTexture as THREE.Texture).dispose();
          delete object.userData.ownedTexture;
        }
      });
      group.removeFromParent();
    },
  };
}

function drain(chunks: BuildingChunks, max = 500) {
  let n = 0;
  while (chunks.pendingCount && n < max) {
    chunks.update();
    n++;
  }
  if (chunks.pendingCount) throw new Error('source build did not drain');
  return n;
}
function sourceRecord(chunks: BuildingChunks, id: number) {
  return (
    chunks as unknown as {
      sources: Map<number, { pendingGroup: THREE.Group | null; legacyGroup: THREE.Group | null }>;
    }
  ).sources.get(id)!;
}
function meshes(chunks: BuildingChunks) {
  const result: THREE.Mesh[] = [];
  chunks.root.traverse((object) => {
    if (object instanceof THREE.Mesh) result.push(object);
  });
  return result;
}
function trianglesFor(chunks: BuildingChunks, id: number) {
  const values: number[][] = [];
  for (const mesh of meshes(chunks)) {
    const positions = mesh.geometry.getAttribute('position'),
      index = mesh.geometry.index!;
    for (let face = 0; face < index.count / 3; face++)
      if (
        chunks.buildingAt({ object: mesh, faceIndex: face } as unknown as THREE.Intersection) === id
      ) {
        const triangle: number[] = [];
        for (let corner = 0; corner < 3; corner++) {
          const vertex = index.getX(face * 3 + corner);
          triangle.push(positions.getX(vertex), positions.getY(vertex), positions.getZ(vertex));
        }
        values.push(triangle);
      }
  }
  return values;
}

describe('BuildingChunks source ownership', () => {
  it('releases consumed source geometry but keeps owned sign material and texture alive until deferred batch retirement', () => {
    const sourceMaterial = new THREE.MeshBasicMaterial(),
      retired: Array<() => void> = [],
      disposed: THREE.Group[] = [];
    const chunks = new BuildingChunks(
      (material) => (material === sourceMaterial ? sourceMaterial : material),
      (release) => retired.push(release),
    );
    const first = groupAt(240, sourceMaterial, true),
      source = descriptor(() => groupAt(240, sourceMaterial, true), disposed);
    const sourceGeometryDisposed = vi.fn(),
      materialDisposed = vi.fn(),
      textureDisposed = vi.fn();
    first.geometries[0].addEventListener('dispose', sourceGeometryDisposed);
    const firstSign = first.group.children[0] as THREE.Mesh,
      firstMaterial = firstSign.material as THREE.Material,
      firstTexture = first.texture!;
    firstMaterial.addEventListener('dispose', materialDisposed);
    firstTexture.addEventListener('dispose', textureDisposed);

    chunks.set(1, first.group, source);
    drain(chunks);
    expect(sourceGeometryDisposed).toHaveBeenCalledTimes(1);
    expect(disposed).toContain(first.group);
    expect(sourceRecord(chunks, 1).pendingGroup).toBeNull();
    expect(sourceRecord(chunks, 1).legacyGroup).toBeNull();
    expect(meshes(chunks)[0].material).toBe(firstMaterial);
    expect(meshes(chunks)[0].parent?.userData.buildingOwnedResources).toContain(firstMaterial);
    expect(materialDisposed).not.toHaveBeenCalled();
    expect(textureDisposed).not.toHaveBeenCalled();

    const second = groupAt(240, sourceMaterial, true),
      secondMaterial = (second.group.children[0] as THREE.Mesh).material as THREE.Material;
    chunks.set(
      1,
      second.group,
      descriptor(() => groupAt(240, sourceMaterial, true), disposed),
    );
    drain(chunks);
    // Retirement can be held while async style compilation still references the old batch.
    expect(retired).toHaveLength(1);
    expect(materialDisposed).not.toHaveBeenCalled();
    expect(textureDisposed).not.toHaveBeenCalled();
    retired.shift()!();
    expect(materialDisposed).toHaveBeenCalledTimes(1);
    expect(textureDisposed).toHaveBeenCalledTimes(1);
    expect(meshes(chunks)[0].material).toBe(secondMaterial);

    chunks.clear();
    expect(retired).toHaveLength(1);
    retired.shift()!();
    sourceMaterial.dispose();
  });

  it('rebuilds committed sources one factory per update with identical triangles and current style at commit', () => {
    const sourceMaterial = new THREE.MeshBasicMaterial(),
      largeMaterial = new THREE.MeshBasicMaterial(),
      anime = new THREE.MeshBasicMaterial(),
      miniature = new THREE.MeshBasicMaterial();
    let mini = false;
    const resolver = (material: THREE.Material) =>
      material === sourceMaterial ||
      material === largeMaterial ||
      material === anime ||
      material === miniature
        ? mini
          ? miniature
          : anime
        : material;
    const chunks = new BuildingChunks(resolver, (dispose) => dispose());
    const generated = new Map<number, number>(),
      groups: THREE.Group[] = [];
    const makeSource = (id: number, x: number, large = 0): RebuildableBuildingSource =>
      descriptor(() => {
        generated.set(id, (generated.get(id) ?? 0) + 1);
        const result = groupAt(x, sourceMaterial, false, large, largeMaterial);
        groups.push(result.group);
        return result;
      });
    for (const [id, x] of [
      [10, 240],
      [11, 260],
    ] as const) {
      const initial = groupAt(x, sourceMaterial);
      chunks.set(id, initial.group, makeSource(id, x));
    }
    drain(chunks);
    const before10 = trianglesFor(chunks, 10),
      before11 = trianglesFor(chunks, 11);
    expect(before10).toHaveLength(12);
    expect(before11).toHaveLength(12);

    const added = groupAt(280, sourceMaterial, false, 14_001, largeMaterial);
    chunks.set(12, added.group, makeSource(12, 280, 14_001));
    let iterations = 0;
    while (chunks.pendingCount && iterations++ < 100) {
      const old = (generated.get(10) ?? 0) + (generated.get(11) ?? 0) + (generated.get(12) ?? 0);
      chunks.update();
      const now = (generated.get(10) ?? 0) + (generated.get(11) ?? 0) + (generated.get(12) ?? 0);
      expect(now - old).toBeLessThanOrEqual(1);
      if (iterations === 1) mini = true; // A staged source build must commit with the newly selected material.
    }
    expect(chunks.pendingCount).toBe(0);
    expect(generated.get(10)).toBe(1);
    expect(generated.get(11)).toBe(1);
    expect(generated.get(12) ?? 0).toBe(0);
    expect(trianglesFor(chunks, 10)).toEqual(before10);
    expect(trianglesFor(chunks, 11)).toEqual(before11);
    expect(meshes(chunks).every((mesh) => mesh.material === miniature)).toBe(true);
    expect(
      chunks.buildingAt({
        object: meshes(chunks)[0],
        faceIndex: 0,
      } as unknown as THREE.Intersection),
    ).toBe(10);
    chunks.clear();
    sourceMaterial.dispose();
    largeMaterial.dispose();
    anime.dispose();
    miniature.dispose();
  });

  it('disposes transferred resources on build cancellation and never disposes a legacy material at batch retirement', () => {
    const shared = new THREE.MeshBasicMaterial(),
      retired: Array<() => void> = [],
      chunks = new BuildingChunks(
        (material) => material,
        (dispose) => retired.push(dispose),
      );
    const owned = groupAt(240, shared, true, 54_000),
      disposed: THREE.Group[] = [];
    const ownedMaterial = (owned.group.children[0] as THREE.Mesh).material as THREE.Material,
      ownedTexture = owned.texture!;
    const materialDisposed = vi.fn(),
      textureDisposed = vi.fn();
    ownedMaterial.addEventListener('dispose', materialDisposed);
    ownedTexture.addEventListener('dispose', textureDisposed);
    chunks.set(
      1,
      owned.group,
      descriptor(() => groupAt(240, shared, true, 54_000), disposed),
    );
    // Sign is fully transformed before the oversized second mesh finishes; its owned resources now belong to the detached build.
    for (let i = 0; i < 6; i++) chunks.update();
    expect(materialDisposed).not.toHaveBeenCalled();
    expect(textureDisposed).not.toHaveBeenCalled();
    chunks.clear();
    expect(materialDisposed).toHaveBeenCalledTimes(1);
    expect(textureDisposed).toHaveBeenCalledTimes(1);
    expect(disposed).toContain(owned.group);

    const legacy = groupAt(250, shared, true),
      legacyMaterial = (legacy.group.children[0] as THREE.Mesh).material as THREE.Material;
    const legacyDisposed = vi.fn();
    legacyMaterial.addEventListener('dispose', legacyDisposed);
    chunks.set(2, legacy.group);
    drain(chunks);
    chunks.clear();
    retired.splice(0).forEach((release) => release());
    expect(legacyDisposed).not.toHaveBeenCalled();
    legacy.group.traverse((object) => {
      if (object instanceof THREE.Mesh) {
        object.geometry.dispose();
        for (const material of Array.isArray(object.material) ? object.material : [object.material])
          material.dispose();
      }
    });
    shared.dispose();
  });
});
