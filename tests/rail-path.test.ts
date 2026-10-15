import fixtureText from './fixtures/rail-alignment-native.json?raw';
import { describe, expect, it } from 'vitest';
import { sampledRail } from '../web/src/render/rail-path';
import type { RailAlignment } from '../web/src/types';

const fixture = JSON.parse(fixtureText) as {
  cases: (RailAlignment & { seed: number; probes: [number, number, number, number, number][] })[];
};

describe('authoritative rail path', () => {
  it('matches native positions and tangents on both arms in three seeds', () => {
    expect(fixture.cases).toHaveLength(6);
    for (const alignment of fixture.cases) {
      const path = sampledRail(alignment);
      for (const [distance, x, z, ux, uz] of alignment.probes) {
        const p = path.at(distance);
        expect(Math.hypot(p.x - x, p.z - z)).toBeLessThan(.0001);
        expect(Math.hypot(p.ux - ux, p.uz - uz)).toBeLessThan(.0001);
        expect(Math.abs(path.distanceAt(p) - distance)).toBeLessThan(.0001);
      }
    }
  });

  it('gives coaches distinct bend tangents and clamps both terminal ends', () => {
    for (const alignment of fixture.cases) {
      const path = sampledRail(alignment);
      const [first, last] = [alignment.samples[0], alignment.samples.at(-1)!];
      expect(path.at(-100).x).toBeCloseTo(first[1], 6);
      expect(path.at(path.length + 100).z).toBeCloseTo(last[2], 6);
      let greatestTurn = 0;
      for (let distance = 24; distance < path.length - 24; distance += 5) {
        const front = path.at(distance + 16), rear = path.at(distance - 16);
        greatestTurn = Math.max(greatestTurn, Math.hypot(front.ux - rear.ux, front.uz - rear.uz));
      }
      expect(greatestTurn).toBeGreaterThan(.01);
      expect(Math.abs(path.at(47).uz)).toBeLessThan(.0001);
      expect(Math.abs(path.at(path.length - 47).uz)).toBeLessThan(.0001);
    }
  });
});
