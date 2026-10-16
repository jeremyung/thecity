import { expect, it } from 'vitest';
import { Vector3 } from 'three';
import { CameraDirector } from '../web/src/camera/director';

it('does not use structural train or onboard-resident fallbacks for withheld movement', () => {
  const director = Object.create(CameraDirector.prototype) as any;
  director.followId = null;
  director.disposed = false;
  director.currentWorld = {
    residents: [{ id: 50, train_id: 10 }],
    buildings: [],
    roads: [],
    nodes: [
      { id: 1, x: 0, z: 0 },
      { id: 2, x: 100, z: 0 },
    ],
    trains: [{ id: 10, route: [1, 2], progress: 20, direction: 1 }],
  };
  const position = new Vector3();
  director.setPositions(new Float32Array(), new Set());
  expect(director.findEntityPoint(10, position)).toBe(true);
  expect(director.findEntityPoint(50, position)).toBe(true);
  director.setPositions(new Float32Array(), new Set([10]));
  expect(director.findEntityPoint(10, position)).toBe(false);
  expect(director.findEntityPoint(50, position)).toBe(false);
  director.setPositions(new Float32Array(), new Set());
  expect(director.findEntityPoint(50, position)).toBe(true);
});
