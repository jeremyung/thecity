import { describe, expect, it } from 'vitest';
import { buildingPalette, BUILDING_ACCENT_WALLS } from '../web/src/render/building-palette';
import type { Building } from '../web/src/types';
import matureSave from '../tests/fixtures/validation/early-reservation-wasm-87231-save.json?raw';

const neutralWalls = ['#d8d4c4', '#c2c5ba', '#b8beb8', '#e2d9c6', '#b7ad9f', '#c9c8c1'];
function sample(
  kind: string,
  appearance_seed: number,
  anchor = false,
): Pick<Building, 'kind' | 'appearance_seed' | 'anchor'> {
  return { kind, appearance_seed, anchor };
}

describe('seeded Tokyo facade palettes', () => {
  it('keeps selection deterministic, bounded, and anchored to the existing founder palette', () => {
    const chosen = buildingPalette(sample('home', 0x8675309), neutralWalls);
    expect(buildingPalette(sample('home', 0x8675309), neutralWalls)).toEqual(chosen);
    expect(buildingPalette(sample('shop', 0x8675309), neutralWalls)).not.toEqual(chosen);
    expect(buildingPalette(sample('shop', 1547540591, true), neutralWalls)).toEqual({
      family: 'neutral',
      wall: neutralWalls[1547540591 % neutralWalls.length],
      trim: '#d8cdb5',
    });
    const observed = new Set<string>();
    for (const kind of ['home', 'shop', 'cafe', 'apartment', 'office'])
      for (let seed = 0; seed < 512; seed++) {
        const palette = buildingPalette(sample(kind, seed), neutralWalls);
        observed.add(palette.wall);
        expect(neutralWalls.concat(BUILDING_ACCENT_WALLS)).toContain(palette.wall);
      }
    expect(observed.size).toBeLessThanOrEqual(11);
    expect(BUILDING_ACCENT_WALLS).toHaveLength(5);
  });

  it('keeps neutrals dominant and gives each building kind a restrained accent mix in the 850-building review save', () => {
    const save = JSON.parse(matureSave) as { buildings: Building[] };
    const counts = new Map<string, number>();
    const colors = new Set<string>();
    for (const building of save.buildings) {
      const palette = buildingPalette(building, neutralWalls);
      counts.set(palette.family, (counts.get(palette.family) ?? 0) + 1);
      colors.add(palette.wall);
    }
    expect(save.buildings).toHaveLength(850);
    expect(colors.size).toBeLessThanOrEqual(11);
    expect(counts.get('neutral') ?? 0).toBeGreaterThan(500);
    for (const family of ['terracotta', 'blue-gray', 'sage', 'brick', 'warm-stone'])
      expect(counts.get(family)).toBeGreaterThan(0);
  });
});
