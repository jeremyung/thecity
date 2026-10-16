/** Decoded rendering tuple: [id, x, z, heading, kind, surfaceY].
 * Kind is 0 person / 1 compact / 2 train / 3 bus / 4 kei / 5 taxi /
 * 6 van / 7 truck / 8 mail vehicle. Road vehicle kinds are shared with
 * render/vehicles.ts; surfaceY stays the final tuple field for every actor.
 * A negative surfaceY delegates to the existing ground/road surface policy.
 * Nonnegative values are absolute walking or vehicle road-surface heights from the engine.
 */
export const MOVEMENT_STRIDE = 6;
export const PEDESTRIAN_GROUND_SURFACE = 0.27;
export const MOVEMENT_KINDS = {
  person: 0,
  compact: 1,
  train: 2,
  bus: 3,
  kei: 4,
  taxi: 5,
  van: 6,
  truck: 7,
  mail: 8,
} as const;

/** Recognize the stable movement wire values, including every road vehicle. */
export function isMovementKind(kind: number): boolean {
  return Number.isInteger(kind) && kind >= MOVEMENT_KINDS.person && kind <= MOVEMENT_KINDS.mail;
}

export function pedestrianSurface(surfaceY: number, groundLift = 0): number {
  return surfaceY >= 0 ? surfaceY : PEDESTRIAN_GROUND_SURFACE + groundLift;
}

/** Train presentation tuples published at the same tick as movement: id, terminal (-1 away), opening [0,1]. */
export const TRAIN_DOOR_STRIDE = 3;
