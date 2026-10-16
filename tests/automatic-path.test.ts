import { describe, expect, it } from 'vitest';
import {
  cameraPathClearance,
  type CameraObstacleBox,
  type CameraPoint3,
} from '../web/src/camera/clearance';
import { planAutomaticCameraPath } from '../web/src/camera/automatic-path';

const tower: CameraObstacleBox = {
  center: { x: 0, z: 0 },
  rotation: Math.PI / 4,
  width: 18,
  depth: 8,
  minY: 0,
  maxY: 62,
};

function assertPathClear(
  start: CameraPoint3,
  path: ReturnType<typeof planAutomaticCameraPath>,
  obstacles: readonly CameraObstacleBox[],
): void {
  let previous = path.recoveryPoint ?? start;
  for (const waypoint of path.waypoints) {
    expect(Object.values(waypoint).every(Number.isFinite)).toBe(true);
    expect(
      cameraPathClearance(previous, waypoint, obstacles, { terrainY: 0, safetyMarginM: 2 })
        .collides,
    ).toBe(false);
    previous = waypoint;
  }
  expect(path.waypoints.length).toBeGreaterThanOrEqual(1);
  expect(path.waypoints.length).toBeLessThanOrEqual(3);
}

describe('automatic camera path planner', () => {
  it('keeps a clear destination unchanged for an unobstructed route', () => {
    const start = { x: -100, y: 30, z: 50 };
    const end = { x: -40, y: 45, z: 55 };
    const path = planAutomaticCameraPath(start, end, [tower]);
    expect(path.kind).toBe('straight');
    expect(path.recoverStart).toBe(false);
    expect(path.waypoints).toEqual([end]);
    expect(path.correctedDestination).toEqual(end);
  });

  it('lifts over a rotated narrow tower with clear rise, traverse, and descent', () => {
    const start = { x: -45, y: 28, z: 0 };
    const end = { x: 45, y: 26, z: 0 };
    const path = planAutomaticCameraPath(start, end, [tower]);
    expect(path.kind).toBe('lifted');
    expect(path.recoverStart).toBe(false);
    expect(path.waypoints).toHaveLength(3);
    expect(path.waypoints[0]).toMatchObject({ x: start.x, z: start.z });
    expect(path.waypoints[1]).toMatchObject({ x: end.x, z: end.z });
    expect(path.waypoints[0].y).toBeGreaterThan(tower.maxY + 2);
    expect(path.correctedDestination).toEqual(end);
    assertPathClear(start, path, [tower]);
  });

  it('handles a high start and low destination without forcing the destination upward', () => {
    const start = { x: -45, y: 120, z: 0 };
    const end = { x: 45, y: 30, z: 0 };
    const path = planAutomaticCameraPath(start, end, [{ ...tower, maxY: 100 }]);
    expect(path.kind).toBe('lifted');
    expect(path.correctedDestination).toEqual(end);
    expect(path.waypoints.at(-1)).toEqual(end);
    assertPathClear(start, path, [tower]);
  });

  it('raises an obstructed destination above the roof envelope', () => {
    const start = { x: -50, y: 30, z: 0 };
    const end = { x: 0, y: 20, z: 0 };
    const path = planAutomaticCameraPath(start, end, [tower]);
    expect(path.kind).toBe('lifted');
    expect(path.correctedDestination.y).toBeGreaterThan(tower.maxY + 2);
    expect(path.waypoints.at(-1)).toEqual(path.correctedDestination);
    assertPathClear(start, path, [tower]);
  });

  it('marks an obstructed initial pose for explicit recovery, then clears the route', () => {
    const start = { x: 0, y: 12, z: 0 };
    const end = { x: 50, y: 24, z: 0 };
    const path = planAutomaticCameraPath(start, end, [tower]);
    expect(path.recoverStart).toBe(true);
    expect(path.recoveryPoint).toBeDefined();
    expect(path.recoveryPoint!.y).toBeGreaterThan(tower.maxY + 2);
    assertPathClear(start, path, [tower]);
    // The planner intentionally does not call the recovery segment itself clear.
    expect(
      cameraPathClearance(start, path.recoveryPoint!, [tower], { safetyMarginM: 2 }).collides,
    ).toBe(true);
  });

  it('clears the highest of multiple swept roofs and the terrain margin', () => {
    const second: CameraObstacleBox = {
      center: { x: 30, z: 4 },
      rotation: -0.2,
      width: 14,
      depth: 12,
      minY: 0,
      maxY: 110,
    };
    const start = { x: -60, y: 40, z: 0 };
    const end = { x: 70, y: -10, z: 0 };
    const path = planAutomaticCameraPath(start, end, [tower, second]);
    expect(path.kind).toBe('lifted');
    expect(path.waypoints[0].y).toBeGreaterThan(second.maxY + 2);
    expect(path.correctedDestination.y).toBeGreaterThan(2);
    assertPathClear(start, path, [tower, second]);
  });
});
