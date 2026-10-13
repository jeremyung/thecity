import { isMovementKind, MOVEMENT_STRIDE } from './movement';

export interface MovementSupport {
  readonly resolved: boolean;
  readonly roads: readonly number[];
  readonly platforms: readonly number[];
}

export interface SupportedMovement {
  readonly movement: Float32Array;
  readonly support: ReadonlyMap<number, MovementSupport>;
}

/**
 * Engine packet v1: [version, rows], then variable-length records:
 * [id, xBits, zBits, headingBits, kind, surfaceYBits, resolved, roads, platforms,
 *  ...roadIds, ...platformNodeIds]. IDs remain integer words on the wire.
 * Worker protocol v5 transfers this packet; the client decodes before recycling.
 */
export function decodeMovementPacket(packet: Uint32Array): SupportedMovement {
  if (packet.length < 2 || packet[0] !== 1) throw new Error('Unsupported movement support packet');
  const rows = packet[1];
  // Validate advertised size before allocating from an untrusted row count.
  if (rows > Math.floor((packet.length - 2) / 9)) throw new Error('Truncated movement support packet');
  const movement = new Float32Array(rows * MOVEMENT_STRIDE);
  const floats = new Float32Array(packet.buffer, packet.byteOffset, packet.length);
  const support = new Map<number, MovementSupport>();
  let cursor = 2;
  for (let row = 0; row < rows; row++) {
    if (cursor + 9 > packet.length) throw new Error('Truncated movement support record');
    const id = packet[cursor], kind = packet[cursor + 4], resolved = packet[cursor + 6];
    const roadCount = packet[cursor + 7], platformCount = packet[cursor + 8];
    if (!isMovementKind(kind) || resolved > 1 || support.has(id)) throw new Error('Invalid movement support record');
    // Existing rendering tuples still use f32 actor IDs. Reject aliasing rather
    // than assigning another actor's presentation; dependencies themselves are u32.
    if (Math.fround(id) !== id) throw new Error('Actor ID is not exactly representable');
    const end = cursor + 9 + roadCount + platformCount;
    if (end > packet.length) throw new Error('Truncated movement support dependencies');
    if (resolved === 1 && roadCount + platformCount === 0) throw new Error('Resolved pose has no support');
    const x = floats[cursor + 1], z = floats[cursor + 2];
    const heading = floats[cursor + 3], surface = floats[cursor + 5];
    if (![x, z, heading, surface].every(Number.isFinite)) throw new Error('Nonfinite movement pose');
    movement.set([id, x, z, heading, kind, surface], row * MOVEMENT_STRIDE);
    support.set(id, {
      resolved: resolved === 1,
      roads: Array.from(packet.subarray(cursor + 9, cursor + 9 + roadCount)),
      platforms: Array.from(packet.subarray(cursor + 9 + roadCount, end)),
    });
    cursor = end;
  }
  if (cursor !== packet.length) throw new Error('Trailing movement support data');
  return { movement, support };
}
