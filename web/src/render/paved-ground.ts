type Point = { x: number; z: number };
type Patch = { points: ReadonlyArray<readonly [number, number]> };

/** Root clearance uses the actual filled polygon, not its bounding rectangle.
 * Canopies rooted outside may overhang paving. Bounds are built once per world
 * update, then reused for every tree; this never changes simulation geometry.
 */
export function createPavedGroundTest(patches: ReadonlyArray<Patch>): (point: Point) => boolean {
  const polygons = patches
    .filter((patch) => patch.points.length >= 3)
    .map(({ points }) => ({
      points,
      minX: Math.min(...points.map((p) => p[0])),
      maxX: Math.max(...points.map((p) => p[0])),
      minZ: Math.min(...points.map((p) => p[1])),
      maxZ: Math.max(...points.map((p) => p[1])),
    }));
  return ({ x, z }) =>
    polygons.some(({ points, minX, maxX, minZ, maxZ }) => {
      if (x < minX || x > maxX || z < minZ || z > maxZ) return false;
      let inside = false;
      for (let i = 0, j = points.length - 1; i < points.length; j = i++) {
        const [ax, az] = points[j],
          [bx, bz] = points[i];
        const dx = bx - ax,
          dz = bz - az;
        const cross = (x - ax) * dz - (z - az) * dx;
        if (
          Math.abs(cross) < 1e-8 &&
          x >= Math.min(ax, bx) &&
          x <= Math.max(ax, bx) &&
          z >= Math.min(az, bz) &&
          z <= Math.max(az, bz)
        )
          return true;
        if (az > z !== bz > z && x < ax + ((z - az) * dx) / dz) inside = !inside;
      }
      return inside;
    });
}
