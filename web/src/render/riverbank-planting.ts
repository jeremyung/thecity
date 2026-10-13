import type { Point } from '../types';

/** Small bank trees fit between the water edge and the promenade railing.
 * The far foliage LOD bounds the rotated crown at 2.38 m before scaling;
 * s=1.8 uses the existing tree transform s/1.8, for a radius below 2.38 m.
 * At offset 2.7 m the crown ends before the railing at offset 5.4 m.
 * Existing world clearance still removes trees at crossings and development.
 */
export function riverbankTreePoints(river: readonly Point[], halfWidth: number) {
  const points: Array<Point & { s: number }> = [];
  if (river.length < 2) return points;
  const start = Math.max(-372, river[0].z + 10);
  const end = Math.min(372, river[river.length - 1].z - 10);
  for (const side of [-1, 1]) {
    for (let z = start; z <= end; z += 18) {
      const segment = river.findIndex((p, i) => i > 0 && z <= p.z);
      if (segment < 1) continue;
      const a = river[segment - 1], b = river[segment];
      const x = a.x + (b.x - a.x) * (z - a.z) / (b.z - a.z);
      const bankX = x + side * (halfWidth + 2.7);
      if (Math.abs(bankX) + 2.38 > 400) continue;
      points.push({ x: bankX, z, s: 1.8 });
    }
  }
  return points;
}
