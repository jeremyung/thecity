import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { disposeGroup } from '../web/src/render/kit';
import {
  buildStreetLighting,
  PersistentStreetLighting,
  prepareStreetLighting,
  streetLightPlacements,
} from '../web/src/render/street-lighting';
import { roadWidth } from '../web/src/render/road-elevation';
import type { World, RoadKind, Stage, NodeKind } from '../web/src/types';

function streetWorld(
  segments: Array<{
    id: number;
    from: number;
    to: number;
    kind?: RoadKind;
    stage?: Stage;
    connected?: boolean;
  }>,
  points: Array<{ id: number; x: number; z: number }>,
): Pick<World, 'nodes' | 'roads'> {
  return {
    nodes: points.map((point) => ({ ...point, kind: 'junction' })),
    roads: segments.map((segment) => {
      const from = points.find((point) => point.id === segment.from)!,
        to = points.find((point) => point.id === segment.to)!;
      return {
        id: segment.id,
        from: segment.from,
        to: segment.to,
        kind: segment.kind ?? 'local',
        stage: segment.stage ?? 'open',
        progress: 1,
        connected: segment.connected ?? true,
        length: Math.hypot(to.x - from.x, to.z - from.z),
        lanes: 2,
      };
    }),
  };
}

describe('procedural street lighting', () => {
  it('places deterministic poles along open connected ordinary streets with safe spacing and curb offset', () => {
    const world = streetWorld(
      [
        { id: 4, from: 1, to: 2 },
        { id: 7, from: 2, to: 3, kind: 'arterial' as const },
        { id: 8, from: 3, to: 4, kind: 'bridge' as const },
        { id: 9, from: 4, to: 5, kind: 'rail' as const },
        { id: 10, from: 5, to: 6, stage: 'foundation' },
        { id: 11, from: 6, to: 7, connected: false },
      ],
      [
        { id: 1, x: -170, z: 20 },
        { id: 2, x: -50, z: 20 },
        { id: 3, x: 60, z: 20 },
        { id: 4, x: 60, z: 140 },
        { id: 5, x: 60, z: 260 },
        { id: 6, x: 180, z: 260 },
        { id: 7, x: 300, z: 260 },
      ],
    );
    const first = streetLightPlacements(world),
      again = streetLightPlacements(world);
    expect(first).toEqual(again);
    expect(first.every((light) => light.roadId === 4 || light.roadId === 7)).toBe(true);
    for (const road of world.roads.filter((road) => road.id === 4 || road.id === 7)) {
      const a = world.nodes.find((node) => node.id === road.from)!,
        b = world.nodes.find((node) => node.id === road.to)!;
      const dx = b.x - a.x,
        dz = b.z - a.z,
        length = Math.hypot(dx, dz);
      const lights = first.filter((light) => light.roadId === road.id);
      const along = lights
        .map((light) => ((light.x - a.x) * dx + (light.z - a.z) * dz) / length)
        .sort((x, y) => x - y);
      const side = lights.map((light) => ((light.x - a.x) * dz - (light.z - a.z) * dx) / length);
      expect(along[0]).toBeGreaterThanOrEqual(10 - 1e-6);
      expect(length - along.at(-1)!).toBeGreaterThanOrEqual(10 - 1e-6);
      for (let i = 1; i < along.length; i++)
        expect(along[i] - along[i - 1]).toBeGreaterThanOrEqual(28 - 1e-6);
      for (let i = 1; i < along.length; i++)
        expect(along[i] - along[i - 1]).toBeLessThanOrEqual(34 + 1e-6);
      const expectedOffset = roadWidth(road.kind) / 2 + 0.65;
      expect(
        side.every(
          (offset, index) =>
            Math.abs(Math.abs(offset) - expectedOffset) < 1e-5 &&
            Math.sign(offset) !== Math.sign(side[index - 1] ?? -offset),
        ),
      ).toBe(true);
      expect(lights.every((light) => light.y === 0.3 && Number.isFinite(light.heading))).toBe(true);
    }
  });

  it('rejects short, missing-node and non-finite segments and respects the global cap', () => {
    const short = streetWorld(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 2, to: 3 },
        { id: 3, from: 3, to: 4 },
      ],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 19.9, z: 0 },
        { id: 3, x: 40, z: 0 },
        { id: 4, x: Number.NaN, z: 20 },
      ],
    );
    short.roads.push({
      id: 4,
      from: 4,
      to: 99,
      kind: 'local' as const,
      stage: 'open',
      progress: 1,
      connected: true,
      length: 150,
      lanes: 2,
    });
    expect(streetLightPlacements(short)).toHaveLength(1); // exactly 20 m permits one midpoint placement

    const points = Array.from({ length: 201 }, (_, index) => ({
      id: index + 1,
      x: 0,
      z: index * 140,
    }));
    const roads = Array.from({ length: 200 }, (_, index) => ({
      id: index + 1,
      from: index + 1,
      to: index + 2,
    }));
    expect(streetLightPlacements(streetWorld(roads, points))).toHaveLength(1000);
  });

  it('builds finite instanced geometry, shares night control, and disposes only owned materials', () => {
    const art = new ArtMaterials();
    const world = streetWorld(
      [{ id: 3, from: 1, to: 2 }],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 0, z: 130 },
      ],
    );
    const group = buildStreetLighting(world, art);
    const instanced = group.children.filter(
      (child): child is THREE.InstancedMesh => child instanceof THREE.InstancedMesh,
    );
    expect(instanced).toHaveLength(4);
    expect(instanced.map((mesh) => mesh.count)).toEqual(
      Array(4).fill(streetLightPlacements(world).length),
    );
    for (const mesh of instanced) {
      const position = mesh.geometry.getAttribute('position');
      for (let i = 0; i < position.count; i++)
        expect([position.getX(i), position.getY(i), position.getZ(i)].every(Number.isFinite)).toBe(
          true,
        );
      const matrix = new THREE.Matrix4();
      for (let i = 0; i < mesh.count; i++) {
        mesh.getMatrixAt(i, matrix);
        expect(matrix.elements.every(Number.isFinite)).toBe(true);
      }
    }
    const pool = instanced.find((mesh) => mesh.name === 'street-lamp-light-pools')!;
    const lamp = instanced.find((mesh) => mesh.name === 'street-lamp-heads')!;
    const poles = instanced.find((mesh) => mesh.name === 'street-lamp-poles')!;
    const poolMaterial = pool.material as THREE.Material & { userData: Record<string, unknown> };
    const lampMaterial = lamp.material as THREE.Material;
    const sharedMaterial = poles.material as THREE.Material;
    expect(poolMaterial.userData.streetLightNight).toBe(art.night);
    art.night.value = 0;
    expect((poolMaterial.userData.streetLightNight as typeof art.night).value).toBe(0);
    const poolDisposed = vi.fn(),
      lampDisposed = vi.fn(),
      sharedDisposed = vi.fn();
    poolMaterial.addEventListener('dispose', poolDisposed);
    lampMaterial.addEventListener('dispose', lampDisposed);
    sharedMaterial.addEventListener('dispose', sharedDisposed);
    disposeGroup(group);
    expect(poolDisposed).toHaveBeenCalledOnce();
    expect(lampDisposed).toHaveBeenCalledOnce();
    expect(sharedDisposed).not.toHaveBeenCalled();
    art.dispose();
  });

  it('keeps four mesh and material identities while atomically replacing prepared placements', () => {
    const art = new ArtMaterials();
    const first = streetWorld(
      [{ id: 3, from: 1, to: 2 }],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 0, z: 130 },
      ],
    );
    const second = streetWorld(
      [{ id: 4, from: 1, to: 2 }],
      [
        { id: 1, x: 20, z: 10 },
        { id: 2, x: 20, z: 210 },
      ],
    );
    const lighting = new PersistentStreetLighting(art);
    const meshes = [...lighting.meshes],
      materials = meshes.map((mesh) => mesh.material);
    const firstData = prepareStreetLighting(first);
    lighting.commit(firstData);
    expect(lighting.meshes).toEqual(meshes);
    expect(lighting.meshes.map((mesh) => mesh.count)).toEqual(Array(4).fill(firstData.count));
    expect(lighting.root.userData.streetLightSources).toEqual(firstData.sources);

    const secondData = prepareStreetLighting(second);
    expect(secondData.count).toBeGreaterThan(firstData.count);
    lighting.commit(secondData);
    expect(lighting.meshes).toEqual(meshes);
    expect(lighting.meshes.map((mesh) => mesh.material)).toEqual(materials);
    expect(lighting.meshes.map((mesh) => mesh.count)).toEqual(Array(4).fill(secondData.count));
    const matrix = new THREE.Matrix4();
    for (let meshIndex = 0; meshIndex < 4; meshIndex++) {
      for (let index = 0; index < secondData.count; index++) {
        lighting.meshes[meshIndex].getMatrixAt(index, matrix);
        expect(matrix.elements).toEqual(
          Array.from(secondData.matrices[meshIndex].subarray(index * 16, index * 16 + 16)),
        );
      }
    }

    lighting.commit({
      count: 0,
      matrices: [new Float32Array(), new Float32Array(), new Float32Array(), new Float32Array()],
      sources: [],
    });
    expect(lighting.meshes.map((mesh) => mesh.count)).toEqual([0, 0, 0, 0]);
    expect(lighting.root.userData.streetLightSources).toEqual([]);
    expect(lighting.meshes).toEqual(meshes);
    const sharedDisposed = vi.fn(),
      ownedDisposed = [vi.fn(), vi.fn()];
    (materials[0] as THREE.Material).addEventListener('dispose', sharedDisposed);
    (materials[2] as THREE.Material).addEventListener('dispose', ownedDisposed[0]);
    (materials[3] as THREE.Material).addEventListener('dispose', ownedDisposed[1]);
    lighting.dispose();
    expect(sharedDisposed).not.toHaveBeenCalled();
    expect(ownedDisposed.map((listener) => listener.mock.calls.length)).toEqual([1, 1]);
    art.dispose();
  });

  it('keeps warm pools above the street and inside each carriageway, with lamps at the pole tops', () => {
    const world = streetWorld(
      [
        { id: 2, from: 1, to: 2 },
        { id: 5, from: 3, to: 4, kind: 'arterial' as const },
      ],
      [
        { id: 1, x: -80, z: 15 },
        { id: 2, x: 80, z: 15 },
        { id: 3, x: -70, z: 110 },
        { id: 4, x: 70, z: 180 },
      ],
    );
    const placements = streetLightPlacements(world);
    const art = new ArtMaterials();
    const group = buildStreetLighting(world, art);
    const findMesh = (name: string) =>
      group.children.find(
        (child): child is THREE.InstancedMesh =>
          child instanceof THREE.InstancedMesh && child.name === name,
      )!;
    const pools = findMesh('street-lamp-light-pools');
    const poles = findMesh('street-lamp-poles');
    const lamps = findMesh('street-lamp-heads');
    const matrix = new THREE.Matrix4(),
      position = new THREE.Vector3(),
      rotation = new THREE.Quaternion(),
      scale = new THREE.Vector3();

    expect(pools.count).toBe(placements.length);
    for (let index = 0; index < placements.length; index++) {
      const placement = placements[index];
      const road = world.roads.find((candidate) => candidate.id === placement.roadId)!;
      const a = world.nodes.find((node) => node.id === road.from)!,
        b = world.nodes.find((node) => node.id === road.to)!;
      const dx = b.x - a.x,
        dz = b.z - a.z,
        length = Math.hypot(dx, dz),
        ux = dx / length,
        uz = dz / length;
      const carriagewayWidth = roadWidth(road.kind);
      const transverseRadius = Math.min(1.8, carriagewayWidth * 0.17);
      const expectedOffset = carriagewayWidth / 2 - transverseRadius - 0.08;

      pools.getMatrixAt(index, matrix);
      matrix.decompose(position, rotation, scale);
      expect(position.y).toBeCloseTo(0.325, 6); // 2.5 cm above the road deck
      expect(scale.x).toBeCloseTo(transverseRadius, 6);
      expect(scale.z).toBeCloseTo(4, 6); // 8 m road-parallel diameter
      const centerAlong = (position.x - a.x) * ux + (position.z - a.z) * uz;
      const centerSide = ((position.x - a.x) * dz - (position.z - a.z) * dx) / length;
      expect(Math.abs(Math.abs(centerSide) - expectedOffset)).toBeLessThan(1e-5);
      expect(Math.abs(centerSide) + scale.x).toBeLessThanOrEqual(carriagewayWidth / 2 - 0.079);
      expect(centerAlong - scale.z).toBeGreaterThanOrEqual(-1e-5);
      expect(centerAlong + scale.z).toBeLessThanOrEqual(length + 1e-5);

      // The baked plane's local Z axis is the 4 m semi-axis; it must follow
      // the road despite the instance yaw, including diagonal arterials.
      const longAxis = new THREE.Vector3(0, 0, -1).applyQuaternion(rotation);
      expect(Math.abs(Math.abs(longAxis.x * ux + longAxis.z * uz) - 1)).toBeLessThan(1e-5);

      const vertices = pools.geometry.getAttribute('position');
      const alongExtents: number[] = [];
      const sideExtents: number[] = [];
      for (let vertex = 0; vertex < vertices.count; vertex++) {
        const point = new THREE.Vector3()
          .fromBufferAttribute(vertices, vertex)
          .applyMatrix4(matrix);
        const along = (point.x - a.x) * ux + (point.z - a.z) * uz;
        const side = (point.x - a.x) * dz - (point.z - a.z) * dx;
        alongExtents.push(along);
        sideExtents.push(side / length);
        expect(point.y).toBeCloseTo(0.325, 5);
        expect(Math.abs(side / length)).toBeLessThanOrEqual(carriagewayWidth / 2 - 0.079 + 1e-5);
      }
      expect(Math.max(...alongExtents) - Math.min(...alongExtents)).toBeCloseTo(8, 5);
      expect(Math.min(...alongExtents)).toBeGreaterThanOrEqual(-1e-5);
      expect(Math.max(...alongExtents)).toBeLessThanOrEqual(length + 1e-5);
      expect(Math.max(...sideExtents) - Math.min(...sideExtents)).toBeCloseTo(
        2 * transverseRadius,
        5,
      );
      poles.getMatrixAt(index, matrix);
      matrix.decompose(position, rotation, scale);
      expect(position.y).toBeCloseTo(road.kind === 'arterial' ? 0.475 : 0, 6);
      const poleTop = position.y + 5;
      lamps.getMatrixAt(index, matrix);
      matrix.decompose(position, rotation, scale);
      const lampTop = position.y + 0.06;
      expect(Math.abs(lampTop - poleTop)).toBeLessThanOrEqual(0.01);
    }
    disposeGroup(group);
    art.dispose();
  });
});
