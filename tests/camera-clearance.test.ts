import { describe, expect, it } from 'vitest';
import {
  cameraPathClearance,
  segmentIntersectsOrientedBoxXZ,
  type CameraObstacleBox,
  type CameraPoint3,
} from '../web/src/camera/clearance';

const box: CameraObstacleBox = {
  center: { x: 0, z: 0 }, rotation: 0, width: 10, depth: 4, minY: 0, maxY: 24,
};

describe('camera path clearance geometry', () => {
  it('intersects a rotated nonsquare envelope in its actual Three.js Y-rotation frame', () => {
    const angle = Math.PI / 4;
    const world = (localX: number, localZ: number): CameraPoint3 => ({
      x: Math.cos(angle) * localX + Math.sin(angle) * localZ,
      y: 12,
      z: -Math.sin(angle) * localX + Math.cos(angle) * localZ,
    });
    const rotated = { ...box, rotation: angle, width: 10, depth: 2 };
    expect(segmentIntersectsOrientedBoxXZ(world(-8, 0), world(8, 0), rotated)).toBe(true);
    // A parallel local-X segment offset beyond the narrow local-Z side is clear.
    expect(segmentIntersectsOrientedBoxXZ(world(-8, 1.6), world(8, 1.6), rotated)).toBe(false);
  });

  it('treats tangency to an expanded footprint as intersection and respects the margin', () => {
    const start = { x: -8, y: 12, z: 3 };
    const end = { x: 8, y: 12, z: 3 };
    expect(segmentIntersectsOrientedBoxXZ(start, end, box)).toBe(false);
    expect(segmentIntersectsOrientedBoxXZ(start, end, box, 1)).toBe(true);
  });

  it('handles starts, ends, and zero-length paths inside the footprint', () => {
    const inside = { x: 0, y: 12, z: 0 };
    const outside = { x: 12, y: 12, z: 0 };
    expect(segmentIntersectsOrientedBoxXZ(inside, outside, box)).toBe(true);
    expect(segmentIntersectsOrientedBoxXZ(outside, inside, box)).toBe(true);
    expect(segmentIntersectsOrientedBoxXZ(inside, inside, box)).toBe(true);
    expect(segmentIntersectsOrientedBoxXZ(outside, outside, box)).toBe(false);
  });

  it('reports roof clearance separately from obstacle-volume collision', () => {
    const lowPath = { start: { x: -12, y: 8, z: 0 }, end: { x: 12, y: 8, z: 0 } };
    expect(cameraPathClearance(lowPath.start, lowPath.end, [box], { terrainY: -1, safetyMarginM: 2 }))
      .toEqual({ requiredCameraHeight: 26, collides: true });

    const elevated = { start: { x: -12, y: 30, z: 0 }, end: { x: 12, y: 30, z: 0 } };
    expect(cameraPathClearance(elevated.start, elevated.end, [box], { safetyMarginM: 2 }))
      .toEqual({ requiredCameraHeight: 26, collides: false });

    const empty = cameraPathClearance(lowPath.start, lowPath.end, [], { terrainY: -3, safetyMarginM: 2 });
    expect(empty).toEqual({ requiredCameraHeight: -1, collides: false });
  });

  it('treats the flat terrain safety plane as a collision boundary', () => {
    const below = cameraPathClearance(
      { x: 0, y: -2, z: 0 }, { x: 8, y: -2, z: 0 }, [],
      { terrainY: -1, safetyMarginM: 1 },
    );
    expect(below).toEqual({ requiredCameraHeight: 0, collides: true });

    const above = cameraPathClearance(
      { x: 0, y: 2, z: 0 }, { x: 8, y: 2, z: 0 }, [],
      { terrainY: -1, safetyMarginM: 1 },
    );
    expect(above).toEqual({ requiredCameraHeight: 0, collides: false });

    const tangent = cameraPathClearance(
      { x: 0, y: 0, z: 0 }, { x: 0, y: 4, z: 0 }, [],
      { terrainY: -1, safetyMarginM: 1 },
    );
    expect(tangent.collides).toBe(true);

    const descending = cameraPathClearance(
      { x: 0, y: 4, z: 0 }, { x: 0, y: -2, z: 0 }, [],
      { terrainY: -1, safetyMarginM: 1 },
    );
    expect(descending.collides).toBe(true);
  });

  it('detects an ascending segment that begins inside a building volume', () => {
    const result = cameraPathClearance(
      { x: 0, y: 12, z: 0 },
      { x: 0, y: 40, z: 0 },
      [box],
      { safetyMarginM: 0 },
    );
    expect(result.collides).toBe(true);
    expect(result.requiredCameraHeight).toBe(24);
  });

  it('rejects invalid geometry and keeps all reported values finite', () => {
    expect(() => segmentIntersectsOrientedBoxXZ(
      { x: Number.NaN, y: 0, z: 0 }, { x: 1, y: 1, z: 1 }, box,
    )).toThrow(RangeError);
    expect(() => cameraPathClearance(
      { x: 0, y: 4, z: 0 }, { x: 1, y: 4, z: 1 }, [{ ...box, rotation: Infinity }],
    )).toThrow(RangeError);
    const result = cameraPathClearance(
      { x: -20, y: 30, z: 0 }, { x: 20, y: 30, z: 0 }, [box],
    );
    expect(Number.isFinite(result.requiredCameraHeight)).toBe(true);
    expect(result.collides).toBe(false);
  });
});
