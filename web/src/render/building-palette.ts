import type { Building } from '../types';

export type BuildingPaletteFamily = 'neutral' | 'terracotta' | 'blue-gray' | 'sage' | 'brick' | 'warm-stone';

export interface BuildingPalette {
  readonly wall: string;
  readonly trim: string;
  readonly family: BuildingPaletteFamily;
}

interface AccentPalette extends BuildingPalette { readonly family: Exclude<BuildingPaletteFamily, 'neutral'> }

const ACCENTS: readonly AccentPalette[] = [
  { family: 'terracotta', wall: '#b77b68', trim: '#ded0bd' },
  { family: 'blue-gray', wall: '#788b96', trim: '#d0d2c8' },
  { family: 'sage', wall: '#85917c', trim: '#d7d4c2' },
  { family: 'brick', wall: '#a96f61', trim: '#d2c5ae' },
  { family: 'warm-stone', wall: '#c2b69d', trim: '#e0d5c0' },
];

const ACCENTS_BY_KIND: Readonly<Record<string, readonly BuildingPaletteFamily[]>> = {
  home: ['terracotta', 'blue-gray', 'sage', 'warm-stone'],
  shop: ['terracotta', 'blue-gray', 'sage', 'brick', 'warm-stone'],
  cafe: ['terracotta', 'blue-gray', 'sage', 'brick', 'warm-stone'],
  apartment: ['blue-gray', 'sage', 'brick', 'warm-stone'],
  office: ['blue-gray', 'terracotta', 'brick', 'warm-stone'],
};

const NEUTRAL_WEIGHT: Readonly<Record<string, number>> = {
  home: 8,
  shop: 6,
  cafe: 5,
  apartment: 7,
  office: 6,
};

function seededHash(seed: number): number {
  let value = seed >>> 0;
  value ^= value >>> 16;
  value = Math.imul(value, 0x7feb352d);
  value ^= value >>> 15;
  value = Math.imul(value, 0x846ca68b);
  value ^= value >>> 16;
  return value >>> 0;
}

/** Select a bounded, kind-aware facade palette from stable appearance data. */
export function buildingPalette(
  building: Pick<Building, 'kind' | 'appearance_seed' | 'anchor'>,
  neutralWalls: readonly string[],
): BuildingPalette {
  if (neutralWalls.length === 0) throw new RangeError('neutralWalls must contain at least one color');

  const seed = building.appearance_seed >>> 0;
  // Founder and named anchor colors are part of the experience's identity.
  if (building.anchor) {
    return { family: 'neutral', wall: neutralWalls[seed % neutralWalls.length], trim: '#d8cdb5' };
  }

  const colors = ACCENTS_BY_KIND[building.kind];
  if (!colors) return { family: 'neutral', wall: neutralWalls[seed % neutralWalls.length], trim: '#d8cdb5' };

  const hash = seededHash(seed);
  const neutralWeight = NEUTRAL_WEIGHT[building.kind] ?? 7;
  if (hash % 10 < neutralWeight) {
    return {
      family: 'neutral',
      wall: neutralWalls[(hash >>> 8) % neutralWalls.length],
      trim: '#d8cdb5',
    };
  }

  const accentFamily = colors[(hash >>> 8) % colors.length];
  const accent = ACCENTS.find(item => item.family === accentFamily)!;
  return { family: accent.family, wall: accent.wall, trim: accent.trim };
}

/** All supported runtime wall swatches: six existing neutrals and five muted accents. */
export const BUILDING_ACCENT_WALLS: readonly string[] = ACCENTS.map(palette => palette.wall);
