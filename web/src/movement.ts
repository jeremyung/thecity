/** Worker protocol v2: [id, x, z, heading, kind, surfaceY].
 * Kind remains 0 person / 1 car / 2 train / 3 bus.
 * A negative surfaceY delegates to the existing ground/road surface policy.
 * Nonnegative values are absolute walking-surface heights from the engine.
 */
export const MOVEMENT_STRIDE = 6;
export const PEDESTRIAN_GROUND_SURFACE = .27;

export function pedestrianSurface(surfaceY: number, groundLift = 0): number {
  return surfaceY >= 0 ? surfaceY : PEDESTRIAN_GROUND_SURFACE + groundLift;
}
