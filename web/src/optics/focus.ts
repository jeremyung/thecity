/** Positive camera-space distance used by the renderer's axial depth-of-field pass. */
export function focusDepthFromViewZ(viewZ: number, near = 0.1, far = 3_500): number {
  const minimum = Number.isFinite(near) && near > 0 ? near : 0.1;
  const maximum = Number.isFinite(far) && far > minimum ? far : Math.max(3_500, minimum + 1);
  const depth = Number.isFinite(viewZ) ? -viewZ : minimum;
  return Math.min(maximum, Math.max(minimum, depth));
}

/** Exponential damping with a rate in inverse seconds, independent of frame rate. */
export function dampFocusDepth(
  current: number,
  target: number,
  deltaSeconds: number,
  rate = 8,
): number {
  if (!Number.isFinite(target)) return Number.isFinite(current) ? current : 0;
  if (!Number.isFinite(current)) return target;
  const dt = Number.isFinite(deltaSeconds) ? Math.max(0, deltaSeconds) : 0;
  const response = Number.isFinite(rate) && rate > 0 ? rate : 8;
  const difference = target - current;
  if (!Number.isFinite(difference)) return target;
  return current + difference * -Math.expm1(-response * dt);
}
