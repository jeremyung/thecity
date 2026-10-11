import { describe, expect, it } from 'vitest';
import { compactMidriseEnvelope, midriseMassing, type BuildingMass } from '../web/src/render/building-massing';

function building(overrides: Partial<{
  kind: string;
  floors: number;
  width: number;
  depth: number;
  appearance_seed: number;
  anchor: boolean;
}> = {}) {
  return {
    kind: 'apartment', floors: 7, width: 18, depth: 14, appearance_seed: 2468, anchor: false,
    ...overrides,
  };
}

function expectInsideEnvelope(masses: BuildingMass[], width: number, depth: number) {
  for (const mass of masses) {
    expect(mass.x - mass.width / 2).toBeGreaterThanOrEqual(-width / 2 - 1e-8);
    expect(mass.x + mass.width / 2).toBeLessThanOrEqual(width / 2 + 1e-8);
    expect(mass.z - mass.depth / 2).toBeGreaterThanOrEqual(-depth / 2 - 1e-8);
    expect(mass.z + mass.depth / 2).toBeLessThanOrEqual(depth / 2 + 1e-8);
    expect(Number.isInteger(mass.baseFloor)).toBe(true);
    expect(Number.isInteger(mass.floors)).toBe(true);
    expect(mass.baseFloor).toBeGreaterThanOrEqual(0);
    expect(mass.floors).toBeGreaterThan(0);
  }
}

function expectSupported(masses: BuildingMass[]) {
  for (const upper of masses.filter((mass) => mass.baseFloor > 0)) {
    const supports = masses.filter((lower) => lower.baseFloor + lower.floors === upper.baseFloor);
    const containedBySupport = supports.some((lower) =>
      upper.x - upper.width / 2 >= lower.x - lower.width / 2 - 1e-8
      && upper.x + upper.width / 2 <= lower.x + lower.width / 2 + 1e-8
      && upper.z - upper.depth / 2 >= lower.z - lower.depth / 2 - 1e-8
      && upper.z + upper.depth / 2 <= lower.z + lower.depth / 2 + 1e-8,
    );
    expect(containedBySupport).toBe(true);
  }
}

describe('footprint-bounded midrise massing', () => {
  it('widens only supported compact-midrise envelopes to the bounded parcel clearance', () => {
    const input = building({ width: 40, depth: 32 });
    expect(compactMidriseEnvelope(input, 33.6, 26.88)).toEqual({ width: 38.4, depth: 30.2 });
    // Keep the existing envelope on narrow lots instead of shrinking their bodies.
    expect(compactMidriseEnvelope(building({ width: 8, depth: 8 }), 6.72, 6.72)).toEqual({ width: 6.72, depth: 6.72 });
    expect(compactMidriseEnvelope(building({ width: 14, depth: 12 }), 11.76, 10.08)).toEqual({ width: 12.4, depth: 10.2 });
  });

  it('leaves anchors, low-rise, towers, and undersized lots at their exact input dimensions', () => {
    const cases = [
      building({ anchor: true, width: 40, depth: 32 }),
      building({ kind: 'home', floors: 2, width: 40, depth: 32 }),
      building({ floors: 12, width: 40, depth: 32 }),
      building({ width: 7.99, depth: 32 }),
      building({ width: 40, depth: 7.99 }),
    ];
    for (const input of cases) {
      const width = input.width * .84, depth = input.depth * .84;
      expect(compactMidriseEnvelope(input, width, depth)).toEqual({ width, depth });
    }
  });

  it('supports every tier across floor counts, narrow plots and seeded variations', () => {
    for (const kind of ['apartment', 'office']) {
      for (let floors = 4; floors <= 11; floors++) {
        for (let seed = 0; seed < 64; seed++) {
          const width = 6 + seed % 19;
          const depth = 6 + (seed * 7) % 23;
          const result = midriseMassing(building({ kind, floors, appearance_seed: Math.imul(seed, 2654435761) >>> 0 }), width, depth)!;
          expectInsideEnvelope(result.masses, width, depth);
          expectSupported(result.masses);
          expect(Math.max(...result.masses.map(mass => mass.baseFloor + mass.floors))).toBe(floors);
        }
      }
    }
  });

  it('composes apartment walk-ups with a full-height side core and supported two-floor setback', () => {
    const input = building({ floors: 7 });
    const result = midriseMassing(input, 18, 14)!;
    const core = result.masses.find((mass) => mass.role === 'core')!;
    const main = result.masses.filter((mass) => mass.role === 'main');

    expect(result.family).toBe('walkup');
    expect(core.baseFloor).toBe(0);
    expect(core.floors).toBe(7);
    expect(core.width / 18).toBeGreaterThanOrEqual(.2);
    expect(core.width / 18).toBeLessThanOrEqual(.24);
    expect(main.map((mass) => [mass.baseFloor, mass.floors])).toEqual([[0, 5], [5, 2]]);
    expect(main[1].width).toBeLessThan(main[0].width);
    expect(main[1].depth).toBeLessThan(main[0].depth);
    expect(main[1].x).not.toBe(main[0].x);
    expect(main[1].z).toBeLessThan(main[0].z);
    expect(core.floors + main[0].floors).toBeGreaterThan(7); // Core and main overlap vertically.
    expectInsideEnvelope(result.masses, 18, 14);
    expectSupported(result.masses);
  });

  it('composes offices with a full-footprint podium, off-center shaft, and further-setback crown', () => {
    for (const floors of [4, 5, 8, 11]) {
      const result = midriseMassing(building({ kind: 'office', floors }), 22, 16)!;
      const [podium] = result.masses;
      const shaft = result.masses.find((mass) => mass.baseFloor === 2 && mass.floors > 2);
      const crown = result.masses[result.masses.length - 1];

      expect(result.family).toBe('setback-office');
      expect(podium).toMatchObject({ x: 0, z: 0, width: 22, depth: 16, baseFloor: 0, floors: 2, role: 'main' });
      expect(crown.baseFloor + crown.floors).toBe(floors);
      expect(crown.floors).toBe(2);
      expect(crown.width).toBeLessThan(shaft?.width ?? podium.width);
      expect(crown.depth).toBeLessThan(shaft?.depth ?? podium.depth);
      if (shaft) {
        expect(shaft.x).not.toBe(0);
        expect(shaft.width).toBeLessThan(podium.width);
        expect(shaft.baseFloor + shaft.floors).toBe(crown.baseFloor);
      }
      expect(result.masses.reduce((sum, mass) => sum + mass.floors, 0)).toBe(floors);
      expectInsideEnvelope(result.masses, 22, 16);
      expectSupported(result.masses);
    }
  });

  it('uses bounded seed-stable variation for core side and setbacks', () => {
    const first = midriseMassing(building({ appearance_seed: 2468 }), 18, 14)!;
    const repeat = midriseMassing(building({ appearance_seed: 2468 }), 18, 14)!;
    const oppositeSide = midriseMassing(building({ appearance_seed: 2469 }), 18, 14)!;
    expect(repeat).toEqual(first);
    expect(oppositeSide.masses.find((mass) => mass.role === 'core')!.x)
      .toBe(-first.masses.find((mass) => mass.role === 'core')!.x);
    expect(oppositeSide.masses).not.toEqual(first.masses);
    expectInsideEnvelope(oppositeSide.masses, 18, 14);
    expectSupported(oppositeSide.masses);
  });

  it('keeps unsupported uses, anchors, out-of-band floors, and undersized envelopes unchanged', () => {
    expect(midriseMassing(building({ kind: 'home' }), 18, 14)).toBeNull();
    expect(midriseMassing(building({ kind: 'shop' }), 18, 14)).toBeNull();
    expect(midriseMassing(building({ anchor: true }), 18, 14)).toBeNull();
    expect(midriseMassing(building({ floors: 3 }), 18, 14)).toBeNull();
    expect(midriseMassing(building({ floors: 12 }), 18, 14)).toBeNull();
    expect(midriseMassing(building({ floors: 4.5 }), 18, 14)).toBeNull();
    expect(midriseMassing(building(), 5.99, 14)).toBeNull();
    expect(midriseMassing(building({ kind: 'office' }), 18, Number.NaN)).toBeNull();
  });
});
