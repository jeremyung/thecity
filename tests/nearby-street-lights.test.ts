import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildStreetLighting } from '../web/src/render/street-lighting';
import { NearbyStreetLights, type StreetLightSource } from '../web/src/render/nearby-street-lights';

function source(key: string, x: number, z: number, y = 5): StreetLightSource {
  return { key, x, y, z, targetX: x + 1, targetZ: z };
}

function spotlights(system: NearbyStreetLights) {
  return system.root.children.filter(
    (child): child is THREE.SpotLight => child instanceof THREE.SpotLight,
  );
}

function tick(system: NearbyStreetLights, focus: { x: number; z: number }, night = 1, steps = 1) {
  for (let i = 0; i < steps; i++) system.update(0.1, focus, night);
}

describe('nearby street-light sources', () => {
  it('keeps exactly four fixed spotlights and limits source candidates to 1000', () => {
    const system = new NearbyStreetLights();
    expect(spotlights(system)).toHaveLength(4);
    const candidates = Array.from({ length: 1001 }, (_, i) =>
      source(`s${i}`, i === 1000 ? 0 : 500 + i, 0),
    );
    system.setSources(candidates);
    tick(system, { x: 0, z: 0 }, 1, 5);
    expect(spotlights(system).every((light) => light.intensity === 0)).toBe(true);
  });

  it('rejects non-finite sources and illuminates the nearest valid candidates', () => {
    const system = new NearbyStreetLights();
    const candidates = [
      source('bad-x', Number.NaN, 0),
      source('bad-y', 0, 0, Number.POSITIVE_INFINITY),
      source('a', 2, 0),
      source('b', -3, 0),
      source('c', 0, 4),
      source('d', 0, -5),
      source('outside', 89.9, 20),
    ];
    system.setSources(candidates);
    tick(system, { x: 0, z: 0 }, 1, 5);
    const lights = spotlights(system);
    expect(lights.filter((light) => light.intensity > 0)).toHaveLength(4);
    expect(
      lights
        .filter((light) => light.intensity > 0)
        .map((light) => `${light.position.x},${light.position.z}`)
        .sort(),
    ).toEqual(['-3,0', '0,-5', '0,4', '2,0']);
  });

  it('does not assign duplicate positions for the committed fixture sources', () => {
    const art = new ArtMaterials();
    const world = {
      nodes: [
        { id: 1, x: -100, z: 0, kind: 'junction' },
        { id: 2, x: 100, z: 0, kind: 'junction' },
      ],
      roads: [
        {
          id: 7,
          from: 1,
          to: 2,
          kind: 'local',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 200,
          lanes: 2,
        },
      ],
    } as never;
    const group = buildStreetLighting(world, art);
    const system = new NearbyStreetLights();
    system.setSources(group.userData.streetLightSources as StreetLightSource[]);
    tick(system, { x: 0, z: 0 }, 1, 5);
    const active = spotlights(system).filter((light) => light.intensity > 0);
    const positions = active.map(
      (light) => `${light.position.x.toFixed(5)},${light.position.z.toFixed(5)}`,
    );
    expect(new Set(positions).size).toBe(positions.length);
    system.reset();
    art.dispose();
  });

  it('does not assign one physical position to multiple light slots', () => {
    const system = new NearbyStreetLights();
    system.setSources([
      source('same-a', 10, 10),
      source('same-b', 10, 10),
      source('same-c', 10, 10),
      source('same-d', 10, 10),
      source('distinct', 12, 10),
    ]);
    tick(system, { x: 10, z: 10 }, 1, 5);
    const active = spotlights(system).filter((light) => light.intensity > 0);
    const positions = active.map((light) => `${light.position.x},${light.position.z}`);
    expect(new Set(positions).size).toBe(positions.length);
  });

  it('keeps zero intensity in daytime despite selecting nearby sources', () => {
    const system = new NearbyStreetLights();
    system.setSources([source('near', 0, 0)]);
    tick(system, { x: 0, z: 0 }, 0, 5);
    expect(spotlights(system).every((light) => light.intensity === 0)).toBe(true);
  });

  it('turns only the light contribution off for a held-camera review', () => {
    const system = new NearbyStreetLights();
    system.setSources([source('near', 0, 0)]);
    tick(system, { x: 0, z: 0 }, 1, 10);
    const lights = spotlights(system),
      positions = lights.map((light) => light.position.clone());
    expect(lights.some((light) => light.intensity > 0)).toBe(true);
    system.enabled = false;
    tick(system, { x: 0, z: 0 });
    expect(lights.every((light) => light.intensity === 0)).toBe(true);
    expect(lights.every((light, i) => light.position.equals(positions[i]))).toBe(true);
    system.enabled = true;
    tick(system, { x: 0, z: 0 });
    expect(lights.some((light) => light.intensity > 0)).toBe(true);
    expect(spotlights(system)).toHaveLength(4);
  });

  it('fades old positions to zero before moving a slot to a distant focus', () => {
    const system = new NearbyStreetLights();
    system.setSources([
      source('near-a', -3, 0),
      source('near-b', -2, 0),
      source('near-c', 2, 0),
      source('near-d', 3, 0),
      source('far-a', 500, 0),
      source('far-b', 510, 0),
      source('far-c', 520, 0),
      source('far-d', 530, 0),
    ]);
    tick(system, { x: 0, z: 0 }, 1, 5);
    const lights = spotlights(system);
    expect(lights.every((light) => light.intensity > 0)).toBe(true);
    const oldPositions = lights.map((light) => light.position.clone());
    const oldIntensities = lights.map((light) => light.intensity);
    tick(system, { x: 500, z: 0 }, 1); // New desired set, old source starts fading.
    expect(lights.every((light, index) => light.position.equals(oldPositions[index]))).toBe(true);
    expect(
      lights.every(
        (light, index) => light.intensity > 0 && light.intensity < oldIntensities[index],
      ),
    ).toBe(true);
    tick(system, { x: 500, z: 0 }, 1, 4);
    expect(lights.every((light) => light.intensity === 0)).toBe(true);
    expect(lights.every((light) => light.position.x >= 500 && light.position.x <= 530)).toBe(true);
    tick(system, { x: 500, z: 0 }, 1);
    expect(lights.every((light) => light.intensity > 0)).toBe(true);
  });

  it('extinguishes and removes assigned sources immediately on reset', () => {
    const system = new NearbyStreetLights();
    system.setSources([source('near', 0, 0)]);
    tick(system, { x: 0, z: 0 }, 1, 5);
    const lights = spotlights(system);
    expect(lights.some((light) => light.intensity > 0)).toBe(true);
    system.reset();
    expect(lights.every((light) => light.intensity === 0)).toBe(true);
    tick(system, { x: 0, z: 0 }, 1, 10);
    expect(lights.every((light) => light.intensity === 0)).toBe(true);
  });

  it('places each source 8 cm below the lamp center at the fixture position', () => {
    const art = new ArtMaterials();
    const world = {
      nodes: [
        { id: 1, x: -100, z: 0, kind: 'junction' },
        { id: 2, x: 100, z: 0, kind: 'junction' },
      ],
      roads: [
        {
          id: 7,
          from: 1,
          to: 2,
          kind: 'local',
          stage: 'open',
          progress: 1,
          connected: true,
          length: 200,
          lanes: 2,
        },
      ],
    } as never;
    const group = buildStreetLighting(world, art);
    const lamps = group.children.find(
      (child): child is THREE.InstancedMesh =>
        child instanceof THREE.InstancedMesh && child.name === 'street-lamp-heads',
    )!;
    const sources = group.userData.streetLightSources as StreetLightSource[];
    expect(sources).toHaveLength(lamps.count);
    const matrix = new THREE.Matrix4(),
      position = new THREE.Vector3();
    for (let i = 0; i < lamps.count; i++) {
      lamps.getMatrixAt(i, matrix);
      position.setFromMatrixPosition(matrix);
      expect(sources[i].x).toBeCloseTo(position.x, 6);
      expect(sources[i].z).toBeCloseTo(position.z, 6);
      expect(sources[i].y).toBeCloseTo(position.y - 0.08, 6);
    }
    art.dispose();
  });
});
