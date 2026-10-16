import { describe, expect, it, vi } from 'vitest';
import { LENS_PRESETS, signedCircleOfConfusionPixels } from '../web/src/camera/lens';

// This adapter evaluates the scalar TSL expression's method chain on JS
// numbers. It does not compile a shader or exercise WebGPU/WebGL execution.
vi.mock('three/tsl', () => {
  class ScalarNode {
    constructor(readonly value: number) {}
    valueOf() {
      return this.value;
    }
    negate() {
      return new ScalarNode(-this.value);
    }
    mul(other: number | ScalarNode) {
      return new ScalarNode(this.value * unwrap(other));
    }
    sub(other: number | ScalarNode) {
      return new ScalarNode(this.value - unwrap(other));
    }
    div(other: number | ScalarNode) {
      return new ScalarNode(this.value / unwrap(other));
    }
    max(other: number | ScalarNode) {
      return new ScalarNode(Math.max(this.value, unwrap(other)));
    }
    greaterThan(other: number | ScalarNode) {
      return new ScalarNode(this.value > unwrap(other) ? 1 : 0);
    }
    and(other: number | ScalarNode) {
      return new ScalarNode(this.value !== 0 && unwrap(other) !== 0 ? 1 : 0);
    }
    clamp(low: number, high: number) {
      return new ScalarNode(Math.max(low, Math.min(high, this.value)));
    }
    select<T extends number | ScalarNode>(whenTrue: T, whenFalse: T) {
      return new ScalarNode(this.value !== 0 ? unwrap(whenTrue) : unwrap(whenFalse));
    }
  }
  function unwrap(value: number | ScalarNode): number {
    return value instanceof ScalarNode ? value.value : value;
  }
  return { float: (value: number) => new ScalarNode(value) };
});

import { physicalCoCNode } from '../web/src/render/postprocessing/physical-coc';

function evaluateExpression(input: {
  depthM: number;
  focusM: number;
  focalLengthMm: number;
  fNumber: number;
  opticalScale: number;
  outputHeightPx: number;
  maxGatherRadiusPx: number;
  enabled?: number;
}): number {
  // The production expression only calls these TSL methods; this structural
  // adapter keeps the mock isolated from test and oracle code.
  const wrap = (value: number) => scalarNode(value);
  const result = physicalCoCNode({
    viewZ: wrap(-input.depthM) as never,
    focusDistanceM: wrap(input.focusM) as never,
    focalLengthM: wrap(input.focalLengthMm / 1000) as never,
    fNumber: wrap(input.fNumber) as never,
    opticalScale: wrap(input.opticalScale) as never,
    outputHeightPx: wrap(input.outputHeightPx) as never,
    maxGatherRadiusPx: wrap(input.maxGatherRadiusPx) as never,
    enabled: wrap(input.enabled ?? 1) as never,
  });
  return scalar(result as unknown as ScalarAdapter);
}

// Share the mock's scalar implementation without importing a test-only
// production hook. The vi.mock factory's methods are represented here too.
interface ScalarAdapter {
  negate(): ScalarAdapter;
  mul(other: number | ScalarAdapter): ScalarAdapter;
  sub(other: number | ScalarAdapter): ScalarAdapter;
  div(other: number | ScalarAdapter): ScalarAdapter;
  max(other: number | ScalarAdapter): ScalarAdapter;
  greaterThan(other: number | ScalarAdapter): ScalarAdapter;
  and(other: number | ScalarAdapter): ScalarAdapter;
  clamp(low: number, high: number): ScalarAdapter;
  select(yes: number | ScalarAdapter, no: number | ScalarAdapter): ScalarAdapter;
  valueOf(): number;
}

function scalarNode(value: number): ScalarAdapter {
  const node = {
    negate: () => scalarNode(-value),
    mul: (other: number | ScalarAdapter) => scalarNode(value * scalar(other)),
    sub: (other: number | ScalarAdapter) => scalarNode(value - scalar(other)),
    div: (other: number | ScalarAdapter) => scalarNode(value / scalar(other)),
    max: (other: number | ScalarAdapter) => scalarNode(Math.max(value, scalar(other))),
    greaterThan: (other: number | ScalarAdapter) => scalarNode(value > scalar(other) ? 1 : 0),
    and: (other: number | ScalarAdapter) => scalarNode(value !== 0 && scalar(other) !== 0 ? 1 : 0),
    clamp: (low: number, high: number) => scalarNode(Math.max(low, Math.min(high, value))),
    select: (yes: number | ScalarAdapter, no: number | ScalarAdapter) =>
      scalarNode(value !== 0 ? scalar(yes) : scalar(no)),
    valueOf: () => value,
  };
  return node;
}

function scalar(value: number | ScalarAdapter): number {
  return typeof value === 'number' ? value : value.valueOf();
}

function oracleNormalized(input: Parameters<typeof evaluateExpression>[0]): number {
  try {
    const diameterPx = signedCircleOfConfusionPixels({
      objectDistanceM: input.depthM,
      focusDistanceM: input.focusM,
      focalLengthMm: input.focalLengthMm,
      fNumber: input.fNumber,
      viewportHeightPx: input.outputHeightPx,
      opticalModelScale: input.opticalScale,
      maxBlurPx: 2 * input.maxGatherRadiusPx,
    });
    return (
      Math.sign(diameterPx) *
      Math.min(1, (Math.abs(diameterPx) * 0.5) / input.maxGatherRadiusPx) *
      (input.enabled ?? 1)
    );
  } catch {
    return 0;
  }
}

describe('physical CoC scalar expression', () => {
  it('matches the independent thin-lens oracle across presets, distances, scales, and output sizes', () => {
    for (const preset of Object.values(LENS_PRESETS)) {
      for (const opticalScale of [1, 0.04, 0.01]) {
        for (const [depthM, focusM] of [
          [180, 260],
          [260, 260],
          [410, 260],
        ] as const) {
          for (const outputHeightPx of [480, 900, 1440]) {
            const input = {
              depthM,
              focusM,
              focalLengthMm: preset.focalLengthMm,
              fNumber: preset.fNumber,
              opticalScale,
              outputHeightPx,
              maxGatherRadiusPx: 24,
            };
            expect(evaluateExpression(input)).toBeCloseTo(oracleNormalized(input), 10);
          }
        }
      }
    }
  });

  it('keeps sign, normalizes diameter to radius, caps at the gather budget, and honors disable', () => {
    const base = {
      depthM: 5,
      focusM: 10,
      focalLengthMm: 50,
      fNumber: 2.8,
      opticalScale: 1,
      outputHeightPx: 1200,
      maxGatherRadiusPx: 12,
    };
    const near = evaluateExpression(base);
    const cappedNear = evaluateExpression({ ...base, maxGatherRadiusPx: 0.05 });
    const far = evaluateExpression({ ...base, depthM: 20 });
    expect(near).toBeLessThan(0);
    expect(Math.abs(near)).toBeLessThan(1);
    expect(Math.abs(cappedNear)).toBe(1);
    expect(far).toBeGreaterThan(0);
    expect(far).toBeLessThanOrEqual(1);
    expect(evaluateExpression({ ...base, depthM: 10 })).toBe(0);
    expect(Math.abs(evaluateExpression({ ...base, enabled: 0 }))).toBe(0);
    expect(Math.abs(evaluateExpression({ ...base, maxGatherRadiusPx: 24 }))).toBeLessThan(
      Math.abs(near),
    );
  });

  it('returns zero when the modeled object or focus distance is at or inside the focal length', () => {
    const base = {
      depthM: 0.05,
      focusM: 2,
      focalLengthMm: 50,
      fNumber: 4,
      opticalScale: 1,
      outputHeightPx: 800,
      maxGatherRadiusPx: 20,
    };
    expect(evaluateExpression(base)).toBe(0);
    expect(evaluateExpression({ ...base, depthM: 5, focusM: 0.05 })).toBe(0);
  });
});
