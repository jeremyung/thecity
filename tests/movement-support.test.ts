import { describe, expect, it } from 'vitest';
import { decodeMovementPacket } from '../web/src/movement-support';

const bits = (value: number) => new Uint32Array(new Float32Array([value]).buffer)[0];
const row = (id: number, roads: number[], platforms: number[] = [], resolved = 1, kind = 0) =>
  [id, bits(1.25), bits(-2.5), bits(-0), kind, bits(8), resolved, roads.length, platforms.length, ...roads, ...platforms];

describe('movement support packet', () => {
  it('keeps pose order, float bits and full integer dependency IDs together', () => {
    const packet = new Uint32Array([1, 2, ...row(12, [0xffffffff], [40]), ...row(15, [3], [], 0)]);
    const result = decodeMovementPacket(packet);
    expect(Array.from(result.movement)).toEqual([12, 1.25, -2.5, -0, 0, 8, 15, 1.25, -2.5, -0, 0, 8]);
    expect(result.support.get(12)).toEqual({ resolved: true, roads: [0xffffffff], platforms: [40] });
    expect(result.support.get(15)?.resolved).toBe(false);
    packet.fill(0);
    expect(result.support.get(12)?.roads).toEqual([0xffffffff]);
    expect(result.movement[0]).toBe(12);
  });

  it('accepts an empty frame and typed-array subviews', () => {
    expect(decodeMovementPacket(new Uint32Array([1, 0])).movement.length).toBe(0);
    const backing = new Uint32Array([99, 1, 1, ...row(12, [3]), 99]);
    expect(decodeMovementPacket(backing.subarray(1, -1)).support.get(12)?.roads).toEqual([3]);
  });

  it('preserves all pedestrian, transit and road vehicle kinds in the six-float pose', () => {
    const packet = new Uint32Array([1, 9, ...Array.from({ length: 9 }, (_, kind) => row(kind + 1, [3], [], 1, kind)).flat()]);
    const result = decodeMovementPacket(packet);
    expect(Array.from({ length: 9 }, (_, i) => result.movement[i * 6 + 4])).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
    expect(Array.from({ length: 9 }, (_, i) => result.movement[i * 6 + 5])).toEqual(Array(9).fill(8));
  });

  it.each([
    [], [2, 0], [1, 0xffffffff], [1, 1, ...row(1, [3]).slice(0, -1)],
    [1, 0, 99], [1, 2, ...row(1, [3]), ...row(1, [4])],
    [1, 1, ...row(16777217, [])], [1, 1, ...row(1, [], [], 2)],
  ])('rejects malformed or ambiguous packets', (...words: number[]) => {
    expect(() => decodeMovementPacket(new Uint32Array(words))).toThrow();
  });

  it('rejects nonfinite poses and unknown actor kinds', () => {
    const pose = new Uint32Array([1, 1, ...row(12, [3])]);
    pose[3] = bits(Infinity);
    expect(() => decodeMovementPacket(pose)).toThrow(/Nonfinite/);
    pose[3] = bits(1); pose[8] = 2;
    expect(() => decodeMovementPacket(pose)).toThrow(/Invalid/);
    const unknownKind = new Uint32Array([1, 1, ...row(12, [3], [], 1, 9)]);
    expect(() => decodeMovementPacket(unknownKind)).toThrow(/Invalid/);
  });

  it('requires dependencies for resolved poses but permits unresolved poses', () => {
    expect(() => decodeMovementPacket(new Uint32Array([1, 1, ...row(12, [])]))).toThrow(/no support/);
    expect(decodeMovementPacket(new Uint32Array([1, 1, ...row(12, [], [], 0)]))
      .support.get(12)?.resolved).toBe(false);
  });
});
