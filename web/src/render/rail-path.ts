import type { Point, RailAlignment } from '../types';
import type { RoadCurve } from './road-curve';

export interface RailPath extends RoadCurve {
  /** Project an interpolated movement position back onto authoritative arc distance. */
  distanceAt(point: Point): number;
}

/** No client-generated spline: each coach samples the engine-owned path. */
export function sampledRail(alignment: RailAlignment): RailPath {
  const samples = alignment.samples;
  if (
    samples.length < 2 ||
    samples[0][0] !== 0 ||
    samples.some(
      (s, i) => s.some((v) => !Number.isFinite(v)) || (i > 0 && s[0] <= samples[i - 1][0]),
    )
  ) {
    throw new Error('Invalid authoritative rail samples');
  }
  const length = samples[samples.length - 1][0];
  return {
    length,
    at(distance) {
      const d = Math.max(0, Math.min(length, distance));
      let lo = 0,
        hi = samples.length - 1;
      while (lo + 1 < hi) {
        const mid = (lo + hi) >>> 1;
        if (samples[mid][0] < d) lo = mid;
        else hi = mid;
      }
      const a = samples[lo],
        b = samples[hi],
        t = (d - a[0]) / (b[0] - a[0]);
      const ux = a[3] + (b[3] - a[3]) * t,
        uz = a[4] + (b[4] - a[4]) * t;
      const norm = Math.hypot(ux, uz);
      return {
        x: a[1] + (b[1] - a[1]) * t,
        z: a[2] + (b[2] - a[2]) * t,
        ux: ux / norm,
        uz: uz / norm,
      };
    },
    distanceAt(point) {
      let best = Infinity,
        distance = 0;
      for (let i = 1; i < samples.length; i++) {
        const a = samples[i - 1],
          b = samples[i],
          dx = b[1] - a[1],
          dz = b[2] - a[2];
        const t = Math.max(
          0,
          Math.min(1, ((point.x - a[1]) * dx + (point.z - a[2]) * dz) / (dx * dx + dz * dz)),
        );
        const error = (point.x - a[1] - dx * t) ** 2 + (point.z - a[2] - dz * t) ** 2;
        if (error < best) {
          best = error;
          distance = a[0] + (b[0] - a[0]) * t;
        }
      }
      return distance;
    },
  };
}
