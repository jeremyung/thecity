/** Independent deterministic streams: adding a prop does not reshuffle the architecture. */
export function architecturalChoice(seed: number, channel: number, index = 0): number {
  let x = (seed ^ Math.imul(channel + 1, 0x9e3779b9) ^ Math.imul(index + 1, 0x85ebca6b)) >>> 0;
  x = Math.imul(x ^ (x >>> 16), 0x7feb352d);
  x = Math.imul(x ^ (x >>> 15), 0x846ca68b);
  return (x ^ (x >>> 16)) >>> 0;
}
export function apartmentFeatures(seed: number) {
  const pick = (channel: number, n: number) => architecturalChoice(seed, channel) % n;
  return {
    windows: pick(0, 3),
    stairSide: pick(1, 2) ? 1 : -1,
    stairFinish: pick(2, 2),
    stairScreen: pick(3, 3),
    canopy: pick(4, 3),
    roof: pick(5, 3),
    shutters: pick(6, 3) === 0,
    utilityColumns: pick(7, 2) + 1,
    balconyScreens: pick(8, 3),
  };
}
