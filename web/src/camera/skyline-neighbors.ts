import type { Building } from '../types';

/** Exact skyline neighborhoods, in original building order for stable scoring. */
export function skylineNeighborhoods(buildings: Building[], radius: number) {
  const cells = new Map<string, number[]>();
  for (let index = 0; index < buildings.length; index++) {
    const building = buildings[index];
    const key = `${Math.floor(building.x / radius)}:${Math.floor(building.z / radius)}`;
    const cell = cells.get(key);
    if (cell) cell.push(index);
    else cells.set(key, [index]);
  }
  return (seed: Building): Building[] => {
    const x = Math.floor(seed.x / radius),
      z = Math.floor(seed.z / radius);
    const candidates: number[] = [];
    for (let dx = -1; dx <= 1; dx++)
      for (let dz = -1; dz <= 1; dz++) {
        const cell = cells.get(`${x + dx}:${z + dz}`);
        if (cell)
          for (const index of cell) {
            const building = buildings[index];
            if (Math.hypot(building.x - seed.x, building.z - seed.z) <= radius)
              candidates.push(index);
          }
      }
    // Preserve reduction order and equal-score winner selection from the full scan.
    candidates.sort((a, b) => a - b);
    return candidates.map((index) => buildings[index]);
  };
}
