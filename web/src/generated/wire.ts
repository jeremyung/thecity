// Generated from engine/src/core/wire.rs. Do not edit.
/** Floats per actor in the movement buffer: id, x, z, heading, kind, surface y. */
export const MOVEMENT_STRIDE = 6;
export const MOVEMENT_FIELD = { id: 0, x: 1, z: 2, heading: 3, kind: 4, surfaceY: 5 } as const;
/** Actor kinds carried in the movement buffer's kind field. */
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
/** Floats per train in the door buffer: id, terminal node (-1 away), opening 0..1. */
export const TRAIN_DOOR_STRIDE = 3;
/** Version word that leads every movement support packet. */
export const MOVEMENT_PACKET_VERSION = 1;
