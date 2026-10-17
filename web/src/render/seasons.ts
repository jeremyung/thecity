import * as THREE from 'three/webgpu';

/**
 * A compressed city year: four seven-day seasons starting in spring, so a new
 * city opens under blossom. Tree crowns change color by species; the switch
 * blends over the first day of each season.
 */
export const SEASONS = ['spring', 'summer', 'autumn', 'winter'] as const;
export type Season = (typeof SEASONS)[number];
export const SEASON_DAYS = 7;
export type TreeSpecies = 'broadleaf' | 'blossom' | 'ginkgo' | 'evergreen';

export interface SeasonState {
  season: Season;
  previous: Season;
  blend: number;
}

/** Season at a city day and minute; `blend` rises 0→1 across the season's first day. */
export function seasonAt(day: number, minuteOfDay = 0): SeasonState {
  const safeDay = Number.isFinite(day) ? Math.max(0, Math.floor(day)) : 0;
  const index = Math.floor(safeDay / SEASON_DAYS) % SEASONS.length;
  const intoSeason = safeDay % SEASON_DAYS;
  const minute = Number.isFinite(minuteOfDay) ? Math.min(1440, Math.max(0, minuteOfDay)) : 0;
  // A brand-new city starts fully in spring rather than blending in from winter.
  const blend = safeDay < SEASON_DAYS || intoSeason > 0 ? 1 : minute / 1440;
  return {
    season: SEASONS[index],
    previous: SEASONS[(index + SEASONS.length - 1) % SEASONS.length],
    blend,
  };
}

const AUTUMN_BROADLEAF = ['#c4793a', '#b9963f', '#a8533a', '#c9a24a'];
const WINTER_BARE = ['#8d7f70', '#84796c', '#978a78'];

/** Crown color for one tree in one season; `variation` is the tree's stable 0..1 hash. */
export function seasonalColor(
  species: TreeSpecies,
  base: string,
  season: Season,
  variation: number,
): THREE.Color {
  const color = new THREE.Color(base),
    pick = (list: readonly string[]) =>
      new THREE.Color(list[Math.floor(variation * list.length) % list.length]);
  switch (species) {
    case 'evergreen':
      return season === 'winter' ? color.lerp(new THREE.Color('#3f5a49'), 0.25) : color;
    case 'blossom':
      if (season === 'spring') return pick(['#e5bdba', '#edcfc4', '#cfa9b0']);
      if (season === 'summer')
        return new THREE.Color('#6f8c5c').lerp(new THREE.Color('#7f9a5e'), variation);
      if (season === 'autumn') return pick(['#b8743f', '#c98b4a', '#a9603d']);
      return pick(WINTER_BARE);
    case 'ginkgo':
      if (season === 'spring') return color.lerp(new THREE.Color('#a9c56f'), 0.45);
      if (season === 'autumn') return pick(['#e2b93b', '#d9a92e', '#e8c454']);
      if (season === 'winter') return pick(WINTER_BARE);
      return color;
    default:
      if (season === 'spring') return color.lerp(new THREE.Color('#a9c27a'), 0.35);
      if (season === 'autumn')
        return variation < 0.75
          ? pick(AUTUMN_BROADLEAF)
          : color.lerp(new THREE.Color('#b9963f'), 0.4);
      if (season === 'winter') return color.lerp(pick(WINTER_BARE), 0.65);
      return color;
  }
}

/** Blended crown color for the current season state. */
export function treeColor(
  species: TreeSpecies,
  base: string,
  state: SeasonState,
  variation: number,
): THREE.Color {
  const current = seasonalColor(species, base, state.season, variation);
  return state.blend >= 1
    ? current
    : seasonalColor(species, base, state.previous, variation).lerp(current, state.blend);
}

/** Quantized key so instance colors are rewritten only when they visibly change. */
export function seasonKey(state: SeasonState) {
  return `${state.season}:${Math.round(state.blend * 24)}`;
}
