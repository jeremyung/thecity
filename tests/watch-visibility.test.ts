import { describe, expect, it } from 'vitest';
import { cameraPathClearance, type CameraObstacleBox, type CameraPoint3 } from '../web/src/camera/clearance';
import { chooseStreetWatchPosition } from '../web/src/camera/watch-visibility';

const target: CameraPoint3 = { x: 0, y: 0, z: 0 };
const forward = { x: 0, z: 1 };
const preferred: CameraPoint3 = { x: 0, y: .28, z: -.96 };
const distance = 75;

function sightPoints(): CameraPoint3[] {
  return [-12, 0, 12].map(along => ({ x: 0, y: 1, z: along }));
}

function expectSightlinesClear(position: CameraPoint3, obstacles: readonly CameraObstacleBox[]): void {
  for (const subject of sightPoints()) {
    expect(cameraPathClearance(subject, position, obstacles, { terrainY: 0, safetyMarginM: .3 }).collides).toBe(false);
  }
}

function expectSafeDestination(position: CameraPoint3, obstacles: readonly CameraObstacleBox[]): void {
  expect(cameraPathClearance(position, position, obstacles, { terrainY: 0, safetyMarginM: 1.5 }).collides).toBe(false);
}

describe('street Watch visibility', () => {
  it('preserves the preferred street view when its target sightlines are clear', () => {
    const result = chooseStreetWatchPosition(target, forward, preferred, distance, []);
    const expected = { x: 0, y: 21, z: -72 };
    expect(result.visible).toBe(true);
    expect(result.position.x).toBeCloseTo(expected.x);
    expect(result.position.y).toBeCloseTo(expected.y);
    expect(result.position.z).toBeCloseTo(expected.z);
    expectSightlinesClear(result.position, []);
    expectSafeDestination(result.position, []);
  });

  it('caps elevation at a 0.65 slope even when the preferred angle is steeper', () => {
    const steepPreferred = { x: 0, y: .9, z: -.4 };
    const result = chooseStreetWatchPosition(target, forward, steepPreferred, distance, []);
    const horizontal = Math.hypot(result.position.x - target.x, result.position.z - target.z);
    expect(result.visible).toBe(true);
    expect((result.position.y - target.y) / horizontal).toBeCloseTo(.65);
  });

  it('reports no clear shot when tall street walls block every candidate below the elevation cap', () => {
    const walls: CameraObstacleBox[] = [
      { center: { x: 0, z: -24 }, rotation: 0, width: 60, depth: 8, minY: 0, maxY: 100 },
      { center: { x: 0, z: 24 }, rotation: 0, width: 60, depth: 8, minY: 0, maxY: 100 },
      { center: { x: -24, z: 0 }, rotation: 0, width: 8, depth: 60, minY: 0, maxY: 100 },
      { center: { x: 24, z: 0 }, rotation: 0, width: 8, depth: 60, minY: 0, maxY: 100 },
    ];
    const result = chooseStreetWatchPosition(target, forward, preferred, distance, walls);
    expect(result.visible).toBe(false);
    expect(Object.values(result.position).every(Number.isFinite)).toBe(true);
    expectSafeDestination(result.position, walls);
    const horizontal = Math.hypot(result.position.x - target.x, result.position.z - target.z);
    expect((result.position.y - target.y) / horizontal).toBeLessThanOrEqual(.65 + 1e-9);
  });

  it('chooses a clear alternate view when a building blocks the preferred shot', () => {
    const blocker: CameraObstacleBox = {
      center: { x: 0, z: -38 }, rotation: 0, width: 42, depth: 8, minY: 0, maxY: 46,
    };
    const result = chooseStreetWatchPosition(target, forward, preferred, distance, [blocker]);
    expect(result.visible).toBe(true);
    expect(result.position).not.toEqual({ x: 0, y: 21, z: -72 });
    expectSightlinesClear(result.position, [blocker]);
    expectSafeDestination(result.position, [blocker]);
  });

  it('returns the same bounded shot for repeated inputs', () => {
    const blockers: CameraObstacleBox[] = [
      { center: { x: -38, z: 0 }, rotation: 0, width: 6, depth: 82, minY: 0, maxY: 36 },
      { center: { x: 38, z: 0 }, rotation: 0, width: 6, depth: 82, minY: 0, maxY: 36 },
    ];
    const first = chooseStreetWatchPosition(target, forward, preferred, distance, blockers);
    const second = chooseStreetWatchPosition(target, forward, preferred, distance, blockers);
    expect(second).toEqual(first);
    expect(Math.abs(first.position.x)).toBeLessThanOrEqual(430);
    expect(Math.abs(first.position.z)).toBeLessThanOrEqual(430);
    expect(first.position.y).toBeLessThanOrEqual(700);
    expect(Object.values(first.position).every(Number.isFinite)).toBe(true);
  });

  it('reports invisible when no destination is camera-safe', () => {
    const enclosure: CameraObstacleBox = {
      center: { x: 0, z: 0 }, rotation: 0, width: 860, depth: 860, minY: 0, maxY: 700,
    };
    const result = chooseStreetWatchPosition(target, forward, preferred, distance, [enclosure]);
    expect(result.visible).toBe(false);
    expect(Object.values(result.position).every(Number.isFinite)).toBe(true);
    expect(Math.abs(result.position.x)).toBeLessThanOrEqual(430);
    expect(Math.abs(result.position.z)).toBeLessThanOrEqual(430);
    expect(result.position.y).toBeLessThanOrEqual(700);
  });

  it('rejects invalid street directions and distance', () => {
    expect(() => chooseStreetWatchPosition(target, { x: 0, z: 0 }, preferred, distance, [])).toThrow(RangeError);
    expect(() => chooseStreetWatchPosition(target, forward, preferred, 0, [])).toThrow(RangeError);
  });
});
