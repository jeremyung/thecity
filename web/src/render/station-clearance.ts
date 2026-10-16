import type { Point, RailPlatform } from '../types';
import { PLATFORM_EDGE_X } from './rail-profile';

/** Conservative plan clearance for the reserved station kit, including canopy
 * overhang and either signed stair run. Clearing the reservation before the deck
 * is erected prevents foliage from appearing through subsequent phases. */
export function overlapsStation(point: Point, canopy: number, platform: RailPlatform): boolean {
  const dx = point.x - platform.x,
    dz = point.z - platform.z;
  const x = dx * platform.uz - dz * platform.ux;
  const z = dx * platform.ux + dz * platform.uz;
  const inside = (minX: number, maxX: number, minZ: number, maxZ: number) =>
    x >= minX - canopy && x <= maxX + canopy && z >= minZ - canopy && z <= maxZ + canopy;
  const approach = platform.ground_path.slice(1).some((b, i) => {
    const a = platform.ground_path[i],
      dx = b.x - a.x,
      dz = b.z - a.z;
    const t = Math.max(0, Math.min(1, ((x - a.x) * dx + (z - a.z) * dz) / (dx * dx + dz * dz)));
    return Math.hypot(x - a.x - t * dx, z - a.z - t * dz) <= canopy + platform.ground_width / 2;
  });
  return (
    approach ||
    inside(PLATFORM_EDGE_X, 9, -2, 49) ||
    inside(
      platform.stair_x - 1.2,
      platform.stair_x + 1.2,
      Math.min(platform.stair_bottom_z, platform.stair_top_z) - 0.1,
      Math.max(platform.stair_bottom_z, platform.stair_top_z) + 0.1,
    )
  );
}
