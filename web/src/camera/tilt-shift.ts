/** Artistic tilted-focus optics. Distances remain scene metres. */
export interface TiltShiftSettings {
  enabled: boolean;
  /** Band offset along its normal; top=0/bottom=1 at zero angle. */
  position: number;
  angle: number;
  tilt: number;
  /** Aperture multiplier: larger values widen the sharp region. */
  width: number;
  /** Maximum blur radius in output pixels. */
  blur: number;
}
export const DEFAULT_TILT_SHIFT: Readonly<TiltShiftSettings> = {
  enabled: false,
  position: 0.5,
  angle: 0,
  tilt: 60,
  width: 0.5,
  blur: 20,
};
export function normalizeTiltShift(value: Partial<TiltShiftSettings>): TiltShiftSettings {
  const bounded = (v: number | undefined, fallback: number, min: number, max: number) =>
    Number.isFinite(v) ? Math.min(max, Math.max(min, v!)) : fallback;
  return {
    enabled: value.enabled === true,
    position: bounded(value.position, 0.5, 0.1, 0.9),
    angle: bounded(value.angle, 0, -90, 90),
    tilt: bounded(value.tilt, 60, 0, 80),
    width: bounded(value.width, 0.5, 0.1, 2),
    blur: bounded(value.blur, 20, 0, 32),
  };
}
/** Write a plane without frame-loop allocations. Settings must already be normalized. */
export function writeTiltedFocusPlane(
  target: { set: (x: number, y: number, z: number, w: number) => unknown },
  focusDepth: number,
  verticalFov: number,
  s: Readonly<TiltShiftSettings>,
): void {
  const t = (s.tilt * Math.PI) / 180,
    a = (s.angle * Math.PI) / 180;
  const nx = Math.sin(t) * Math.sin(a),
    ny = Math.sin(t) * Math.cos(a),
    nz = Math.cos(t);
  const anchorOffset = (0.5 - s.position) * 2 * Math.tan((verticalFov * Math.PI) / 360);
  target.set(nx, ny, nz, Math.max(0.1, focusDepth) * (nz + Math.sin(t) * anchorOffset));
}
/** Plane normal in camera coordinates with positive forward depth; w is plane distance. */
export function tiltedFocusPlane(
  focusDepth: number,
  verticalFov: number,
  settings: TiltShiftSettings,
): [number, number, number, number] {
  const result: [number, number, number, number] = [0, 0, 0, 0];
  writeTiltedFocusPlane(
    {
      set: (x, y, z, w) => {
        result[0] = x;
        result[1] = y;
        result[2] = z;
        result[3] = w;
      },
    },
    focusDepth,
    verticalFov,
    normalizeTiltShift(settings),
  );
  return result;
}
/** Intersection with a camera ray (x/depth,y/depth,1); null means no forward focus. */
export function tiltedRayFocus(
  plane: readonly number[],
  rayX: number,
  rayY: number,
): number | null {
  const denominator = plane[0] * rayX + plane[1] * rayY + plane[2];
  const depth = plane[3] / denominator;
  return Number.isFinite(depth) && Math.abs(denominator) > 1e-5 && depth > 0.1 ? depth : null;
}
