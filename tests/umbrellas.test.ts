import { describe, expect, it, vi } from "vitest";
import * as THREE from "three/webgpu";
import { Umbrellas } from "../web/src/render/umbrellas";

function tuples(...actors: Array<[number, number, number, number, number]>): Float32Array {
  return new Float32Array(actors.flat());
}

function meshesOfType(root: THREE.Group): THREE.InstancedMesh[] {
  const meshes: THREE.InstancedMesh[] = [];
  root.traverse((object) => { if (object instanceof THREE.InstancedMesh) meshes.push(object); });
  return meshes;
}

function selectedXs(umbrellas: Umbrellas): number[] {
  const xs: number[] = [];
  const matrix = new THREE.Matrix4();
  const meshes = meshesOfType(umbrellas.root);
  const shaftGeometry = meshes.at(-1)!.geometry;
  for (const mesh of meshes) {
    if (mesh.geometry === shaftGeometry) continue;
    for (let i = 0; i < mesh.count; i++) {
      mesh.getMatrixAt(i, matrix);
      xs.push(matrix.elements[12]);
    }
  }
  return xs.sort((a, b) => a - b);
}

function canopyScale(umbrellas: Umbrellas): number {
  const meshes = meshesOfType(umbrellas.root);
  const shaftGeometry = meshes.at(-1)!.geometry;
  const canopy = meshes.find((mesh) => mesh.count > 0 && mesh.geometry !== shaftGeometry)!;
  const matrix = new THREE.Matrix4();
  const scale = new THREE.Vector3();
  canopy.getMatrixAt(0, matrix);
  matrix.decompose(new THREE.Vector3(), new THREE.Quaternion(), scale);
  return scale.x;
}

describe("Umbrellas", () => {
  it("renders only pedestrians, stays within capacity, and uses finite reusable instance geometry", () => {
    const umbrellas = new Umbrellas(2);
    const stream = tuples(
      [10, 10, 2, 0.2, 0],
      [11, 20, 3, 0.6, 0],
      [12, 30, 4, 0.8, 1],
      [13, 40, 5, 1.1, 2],
    );
    umbrellas.update(stream, 0.2, 1);
    const meshes = meshesOfType(umbrellas.root);
    const canopyMeshes = meshes.slice(0, 6);
    const shaft = meshes[6];
    expect(canopyMeshes.reduce((sum, mesh) => sum + mesh.count, 0)).toBe(2);
    expect(shaft.count).toBe(2);
    expect(selectedXs(umbrellas).every((x) => x === 10 || x === 20)).toBe(true);
    expect(umbrellas.root.children).toHaveLength(7);
    expect(meshes.every((mesh) => mesh.frustumCulled)).toBe(true);
    expect(meshes.filter((mesh) => mesh.count > 0).every((mesh) => mesh.boundingSphere && Number.isFinite(mesh.boundingSphere.radius))).toBe(true);
    for (const mesh of meshes) {
      const positions = mesh.geometry.getAttribute("position");
      for (let index = 0; index < positions.count; index++) {
        expect(Number.isFinite(positions.getX(index))).toBe(true);
        expect(Number.isFinite(positions.getY(index))).toBe(true);
        expect(Number.isFinite(positions.getZ(index))).toBe(true);
      }
    }
    const canopyNormals = meshes[0].geometry.getAttribute("normal");
    expect(Array.from({ length: canopyNormals.count }, (_, index) => canopyNormals.getY(index)).reduce((sum, y) => sum + y, 0) / canopyNormals.count).toBeGreaterThan(0);

    const geometries = meshes.map((mesh) => mesh.geometry);
    const matrixBuffers = meshes.map((mesh) => mesh.instanceMatrix.array);
    umbrellas.update(stream, 0.9, 1.1);
    expect(meshesOfType(umbrellas.root).map((mesh) => mesh.geometry)).toEqual(geometries);
    meshesOfType(umbrellas.root).forEach((mesh, index) => expect(mesh.instanceMatrix.array).toBe(matrixBuffers[index]));
    expect(meshesOfType(umbrellas.root).reduce((sum, mesh) => sum + mesh.count, 0)).toBeLessThanOrEqual(4);
    umbrellas.dispose();
  });

  it("keeps the same seeded pedestrian subset when movement order changes", () => {
    const umbrellas = new Umbrellas(3);
    const actors: Array<[number, number, number, number, number]> = Array.from({ length: 12 }, (_, index) => [index + 1, (index + 1) * 10, 0, index * 0.1, 0] as [number, number, number, number, number]);
    umbrellas.update(tuples(...actors), 0.7, 10);
    const initial = selectedXs(umbrellas);
    expect(initial).toHaveLength(3);
    umbrellas.update(tuples(...actors.slice().reverse()), 0.7, 10.1);
    expect(selectedXs(umbrellas)).toEqual(initial);
    umbrellas.dispose();
  });

  it("opens smoothly by elapsed presentation time and keeps injected art materials owned by the caller", () => {
    const cached = new Map<string, THREE.Material>();
    const materialForColor = (hex: string) => {
      let material = cached.get(hex);
      if (!material) { material = new THREE.MeshStandardMaterial({ color: hex }); cached.set(hex, material); }
      return material;
    };
    const first = new Umbrellas(1, materialForColor);
    const second = new Umbrellas(1, materialForColor);
    const actor = tuples([52, 4, -8, 0.4, 0]);
    first.update(actor, 0, 0);
    second.update(actor, 0, 0);
    first.update(actor, 1, 0.05);
    second.update(actor, 1, 0.1);
    const partlyOpen = canopyScale(first);
    for (let frame = 2; frame <= 10; frame++) first.update(actor, 1, frame * 0.05);
    for (let frame = 2; frame <= 5; frame++) second.update(actor, 1, frame * 0.1);
    const firstOpen = canopyScale(first);
    const secondOpen = canopyScale(second);
    expect(firstOpen).toBeGreaterThan(partlyOpen);
    expect(firstOpen).toBeCloseTo(secondOpen, 5);

    const materialDisposals = [...cached.values()].map((material) => {
      const dispose = vi.fn();
      material.addEventListener("dispose", dispose);
      return dispose;
    });
    first.dispose(); second.dispose();
    expect(materialDisposals.every((dispose) => !dispose.mock.calls.length)).toBe(true);
    for (const material of cached.values()) material.dispose();
  });

  it("shows an initially rainy paused world fully open, then hides canopies after dry weather settles", () => {
    const umbrellas = new Umbrellas(2);
    const actor = tuples([7, 2, 3, 0.4, 0], [8, 5, 6, 0.8, 0]);
    umbrellas.update(actor, 0.8, 44); // The first snapshot has no elapsed presentation delta.
    expect(canopyScale(umbrellas)).toBeCloseTo(0.8, 5);
    expect(meshesOfType(umbrellas.root).at(-1)!.count).toBe(2);

    for (let frame = 1; frame <= 40; frame++) umbrellas.update(actor, 0, 44 + frame * 0.1);
    expect(meshesOfType(umbrellas.root).every((mesh) => mesh.count === 0)).toBe(true);

    umbrellas.reset();
    expect(meshesOfType(umbrellas.root).every((mesh) => mesh.count === 0)).toBe(true);
    umbrellas.update(actor, 1, 44); // A new world's first rainy snapshot opens immediately again.
    expect(canopyScale(umbrellas)).toBeCloseTo(1, 5);
    umbrellas.dispose();
  });
});
