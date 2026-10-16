import type { Point } from '../types';

export type JunctionArm = { end: Point; width: number };
const cross = (a: Point, b: Point, c: Point) =>
  (b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x);
function hull(points: Point[]): Point[] {
  const sorted = [...points].sort((a, b) => a.x - b.x || a.z - b.z);
  const unique = sorted.filter(
    (p, i) => !i || Math.hypot(p.x - sorted[i - 1].x, p.z - sorted[i - 1].z) > 1e-7,
  );
  if (unique.length < 3) return [];
  const half = (ps: Point[]) => {
    const out: Point[] = [];
    for (const p of ps) {
      while (out.length > 1 && cross(out[out.length - 2], out[out.length - 1], p) <= 1e-8)
        out.pop();
      out.push(p);
    }
    return out;
  };
  const a = half(unique),
    b = half([...unique].reverse());
  a.pop();
  b.pop();
  return a.length + b.length >= 3 ? [...a, ...b] : [];
}

/**
 * Convex corner fill bounded by the former junction radius. Cross-section ends
 * determine the footprint, so a wide road cannot create a full-width bulb on a
 * narrow side street. Quarter-ellipse samples round exposed connecting corners.
 * Straight collinear roads already overlap and need no extra pad.
 */
export function junctionFootprint(node: Point, arms: readonly JunctionArm[]): Point[] {
  const ends: Point[] = [];
  let count = 0,
    maxRadius = 0;
  for (const { end, width } of arms) {
    const dx = end.x - node.x,
      dz = end.z - node.z,
      length = Math.hypot(dx, dz);
    if (!Number.isFinite(length) || length < 1e-6 || !Number.isFinite(width) || width <= 0)
      continue;
    count++;
    const half = width / 2;
    maxRadius = Math.max(maxRadius, half);
    for (const sign of [-1, 1])
      ends.push({ x: (-dz / length) * half * sign, z: (dx / length) * half * sign });
  }
  if (count < 2) return [];
  const boundary = hull(ends);
  if (!boundary.length) return [];
  const samples: Point[] = [];
  for (let i = 0; i < boundary.length; i++) {
    const p = boundary[i],
      q = boundary[(i + 1) % boundary.length];
    for (let j = 0; j < 8; j++) {
      const t = (j * Math.PI) / 16;
      let x = p.x * Math.cos(t) + q.x * Math.sin(t),
        z = p.z * Math.cos(t) + q.z * Math.sin(t);
      const radius = Math.hypot(x, z),
        scale = radius > maxRadius ? maxRadius / radius : 1;
      x *= scale;
      z *= scale;
      samples.push({ x, z });
    }
  }
  return hull(samples).map((p) => ({ x: p.x + node.x, z: p.z + node.z }));
}
