import { describe, expect, it } from 'vitest';
import { cameraPathClearance, type CameraObstacleBox, type CameraPoint3 } from '../web/src/camera/clearance';
import { chooseTrainFollowPosition, chooseBuildingFollowPosition } from '../web/src/camera/follow-visibility';

const target: CameraPoint3 = { x: 0, y: 12, z: 0 };

describe('building follow visibility', () => {
  it('looks past surrounding blocks without rejecting the subject itself', () => {
    const subject: CameraObstacleBox = { center:{x:0,z:0},rotation:.3,width:20,depth:20,minY:0,maxY:24 };
    const blockers: CameraObstacleBox[] = [
      { center:{x:30,z:0},rotation:0,width:12,depth:85,minY:0,maxY:40 },
      { center:{x:-30,z:0},rotation:0,width:12,depth:85,minY:0,maxY:40 },
      { center:{x:0,z:30},rotation:0,width:85,depth:12,minY:0,maxY:40 },
      { center:{x:0,z:-30},rotation:0,width:85,depth:12,minY:0,maxY:40 },
    ];
    const result = chooseBuildingFollowPosition(target, 20, 110, [subject,...blockers]);
    expect(result.visible).toBe(true);
    expect(result.position.y).toBeGreaterThan(75);
    expect(cameraPathClearance(target,result.position,blockers,{terrainY:0,safetyMarginM:.3}).collides).toBe(false);
    // A clear roof/centre is insufficient: the demolition work remains near ground.
    for (const y of [1, 24]) {
      expect(cameraPathClearance({ ...target, y }, result.position, blockers,
        { terrainY: 0, safetyMarginM: .3 }).collides).toBe(false);
    }
    expect(chooseBuildingFollowPosition(target,20,110,[subject,...blockers])).toEqual(result);
  });
});
const forward = { x: 0, z: 1 };
// Preferred is a direction from the target; distance supplies its magnitude.
const preferred: CameraPoint3 = { x: 0, y: 0.28, z: -0.96 };
const distance = 75;
const preferredPosition: CameraPoint3 = {
  x: target.x + preferred.x * distance,
  y: target.y + preferred.y * distance,
  z: target.z + preferred.z * distance,
};

function sightPoints(): CameraPoint3[] {
  return [-20, 0, 20].map((along) => ({
    x: target.x + forward.x * along,
    y: target.y,
    z: target.z + forward.z * along,
  }));
}

function expectVisibleLinesClear(position: CameraPoint3, obstacles: readonly CameraObstacleBox[]): void {
  for (const subject of sightPoints()) {
    expect(cameraPathClearance(position, subject, obstacles, { terrainY: 0, safetyMarginM: 0 }).collides).toBe(false);
  }
}

function expectWithinBounds(position: CameraPoint3): void {
  expect(Math.abs(position.x - target.x)).toBeLessThanOrEqual(430);
  expect(Math.abs(position.z - target.z)).toBeLessThanOrEqual(430);
  expect(position.y).toBeLessThanOrEqual(700);
  expect(Object.values(position).every(Number.isFinite)).toBe(true);
}

describe('train follow visibility', () => {
  it('preserves a preferred pose when it already has a clear view', () => {
    const result = chooseTrainFollowPosition(target, forward, preferred, distance, []);

    expect(result.visible).toBe(true);
    for (const axis of ['x','y','z'] as const) expect(result.position[axis]).toBeCloseTo(preferredPosition[axis]);
    expectVisibleLinesClear(result.position, []);
  });

  it('moves to a clear alternate view when a building blocks the preferred sightline', () => {
    const blocker: CameraObstacleBox = {
      center: { x: 0, z: -38 }, rotation: 0, width: 42, depth: 8, minY: 0, maxY: 46,
    };
    const result = chooseTrainFollowPosition(target, forward, preferred, distance, [blocker]);

    expect(result.visible).toBe(true);
    expect(result.position).not.toEqual(preferredPosition);
    expectVisibleLinesClear(result.position, [blocker]);
    expectWithinBounds(result.position);
  });

  it('uses a higher angle when low views from either side are blocked', () => {
    const barriers: CameraObstacleBox[] = [
      { center:{x:-38,z:0},rotation:0,width:6,depth:82,minY:0,maxY:36 },
      { center:{x:38,z:0},rotation:0,width:6,depth:82,minY:0,maxY:36 },
      { center:{x:0,z:-38},rotation:0,width:82,depth:6,minY:0,maxY:36 },
      { center:{x:0,z:38},rotation:0,width:82,depth:6,minY:0,maxY:36 },
    ];
    const result = chooseTrainFollowPosition(target, forward, preferred, distance, barriers);

    expect(result.visible).toBe(true);
    expect(result.position.y).toBeGreaterThan(preferredPosition.y);
    expectVisibleLinesClear(result.position, barriers);
    expectWithinBounds(result.position);
  });

  it('returns a finite bounded invisible result when every view is boxed in', () => {
    const enclosure: CameraObstacleBox = {
      center: { x: target.x, z: target.z }, rotation: 0,
      width: 860, depth: 860, minY: 0, maxY: 700,
    };
    const result = chooseTrainFollowPosition(target, forward, preferred, distance, [enclosure]);

    expect(result.visible).toBe(false);
    expectWithinBounds(result.position);
  });
});
