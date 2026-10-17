import { describe, expect, it } from 'vitest';
import { createPavedGroundTest } from '../web/src/render/paved-ground';

const square = [
  [-1, -1],
  [1, -1],
  [1, 1],
  [-1, 1],
] as const;

describe('paved ground containment', () => {
  it('uses the actual rotated polygon rather than its bounding box', () => {
    // A diamond has a broad axis-aligned box, but its corners are unpaved.
    const isPaved = createPavedGroundTest([
      {
        points: [
          [0, -2],
          [2, 0],
          [0, 2],
          [-2, 0],
        ],
      },
    ]);

    expect(isPaved({ x: 0, z: 1.5 })).toBe(true);
    expect(isPaved({ x: 1.8, z: 1.8 })).toBe(false);
  });

  it('treats a concave polygon notch as exterior', () => {
    const isPaved = createPavedGroundTest([
      {
        points: [
          [0, 0],
          [4, 0],
          [4, 4],
          [3, 4],
          [3, 1],
          [1, 1],
          [1, 4],
          [0, 4],
        ],
      },
    ]);

    expect(isPaved({ x: 0.5, z: 3 })).toBe(true);
    expect(isPaved({ x: 2, z: 3 })).toBe(false);
  });

  it('includes polygon edges and vertices', () => {
    const isPaved = createPavedGroundTest([{ points: square }]);

    expect(isPaved({ x: 0, z: -1 })).toBe(true);
    expect(isPaved({ x: 1, z: 1 })).toBe(true);
  });

  it('does not create fill for empty or underspecified patches', () => {
    const isPaved = createPavedGroundTest([
      { points: [] },
      {
        points: [
          [0, 0],
          [2, 0],
        ],
      },
    ]);

    expect(isPaved({ x: 0, z: 0 })).toBe(false);
    expect(isPaved({ x: 1, z: 0 })).toBe(false);
  });

  it('handles clockwise and counterclockwise point order', () => {
    const ccw = createPavedGroundTest([{ points: square }]);
    const cw = createPavedGroundTest([{ points: [...square].reverse() }]);

    expect(ccw({ x: 0.25, z: -0.5 })).toBe(true);
    expect(cw({ x: 0.25, z: -0.5 })).toBe(true);
    expect(ccw({ x: 2, z: 0 })).toBe(false);
    expect(cw({ x: 2, z: 0 })).toBe(false);
  });

  it('checks each patch independently and leaves the gap between patches unpaved', () => {
    const isPaved = createPavedGroundTest([
      {
        points: [
          [-4, -1],
          [-2, -1],
          [-2, 1],
          [-4, 1],
        ],
      },
      {
        points: [
          [2, -1],
          [4, -1],
          [4, 1],
          [2, 1],
        ],
      },
    ]);

    expect(isPaved({ x: -3, z: 0 })).toBe(true);
    expect(isPaved({ x: 3, z: 0 })).toBe(true);
    expect(isPaved({ x: 0, z: 0 })).toBe(false);
  });
});
