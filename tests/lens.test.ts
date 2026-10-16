import { describe, expect, it } from 'vitest';
import {
  circleOfConfusionPixels,
  LENS_PRESETS,
  normalizedGatherRadiusFromDiameterPx,
  signedCircleOfConfusionPixels,
  verticalFovDegrees,
} from '../web/src/camera/lens';

describe('camera lens model', () => {
  it('derives the expected vertical field of view for the 35 mm and 50 mm presets', () => {
    expect(verticalFovDegrees(LENS_PRESETS.environment.focalLengthMm)).toBeCloseTo(37.85, 1);
    expect(verticalFovDegrees(LENS_PRESETS.neighborhood.focalLengthMm)).toBeCloseTo(26.99, 1);
    expect(verticalFovDegrees(LENS_PRESETS.detail.focalLengthMm)).toBeLessThan(
      verticalFovDegrees(50),
    );
  });

  it('has zero blur on the focus plane and increases blur away from focus', () => {
    const shared = {
      focusDistanceM: 10,
      focalLengthMm: 50,
      fNumber: 5.6,
      viewportHeightPx: 1000,
    } as const;
    expect(circleOfConfusionPixels({ ...shared, objectDistanceM: 10 })).toBe(0);
    const near = circleOfConfusionPixels({ ...shared, objectDistanceM: 5 });
    const far = circleOfConfusionPixels({ ...shared, objectDistanceM: 20 });
    expect(near).toBeGreaterThan(far);
    expect(far).toBeGreaterThan(0);
  });

  it('preserves near/far sign while the legacy helper stays unsigned', () => {
    const shared = {
      focusDistanceM: 10,
      focalLengthMm: 50,
      fNumber: 5.6,
      viewportHeightPx: 1000,
    } as const;
    const near = signedCircleOfConfusionPixels({ ...shared, objectDistanceM: 5 });
    const far = signedCircleOfConfusionPixels({ ...shared, objectDistanceM: 20 });
    expect(near).toBeLessThan(0);
    expect(far).toBeGreaterThan(0);
    expect(signedCircleOfConfusionPixels({ ...shared, objectDistanceM: 10 })).toBe(0);
    expect(circleOfConfusionPixels({ ...shared, objectDistanceM: 5 })).toBeCloseTo(
      Math.abs(near),
      12,
    );
  });

  it('applies optical model scale only to scene distances and follows aperture trends', () => {
    const input = {
      objectDistanceM: 5,
      focusDistanceM: 10,
      focalLengthMm: 50,
      fNumber: 5.6,
      viewportHeightPx: 1000,
      maxBlurPx: 1000,
    } as const;
    const physical = Math.abs(signedCircleOfConfusionPixels(input));
    const miniature = Math.abs(signedCircleOfConfusionPixels({ ...input, opticalModelScale: 0.5 }));
    const wideAperture = Math.abs(signedCircleOfConfusionPixels({ ...input, fNumber: 2.8 }));
    expect(miniature).toBeGreaterThan(physical);
    expect(wideAperture).toBeCloseTo(physical * 2, 10);
    expect(signedCircleOfConfusionPixels({ ...input, opticalModelScale: 1 })).toBe(
      signedCircleOfConfusionPixels(input),
    );
  });

  it('supports each lens preset and rejects scaled distances that reach the focal length', () => {
    for (const preset of Object.values(LENS_PRESETS)) {
      expect(
        Number.isFinite(
          signedCircleOfConfusionPixels({
            objectDistanceM: 3,
            focusDistanceM: 8,
            focalLengthMm: preset.focalLengthMm,
            fNumber: preset.fNumber,
            viewportHeightPx: 1200,
          }),
        ),
      ).toBe(true);
    }
    expect(() =>
      signedCircleOfConfusionPixels({
        objectDistanceM: 5,
        focusDistanceM: 10,
        focalLengthMm: 50,
        fNumber: 5.6,
        viewportHeightPx: 1000,
        opticalModelScale: 0.005,
      }),
    ).toThrow(/must exceed focal length/);
    expect(() =>
      signedCircleOfConfusionPixels({
        objectDistanceM: 5,
        focusDistanceM: 10,
        focalLengthMm: 50,
        fNumber: 5.6,
        viewportHeightPx: 1000,
        opticalModelScale: Number.NaN,
      }),
    ).toThrow(RangeError);
  });

  it('uses scene meters with optical millimeters consistently', () => {
    const pixels = circleOfConfusionPixels({
      objectDistanceM: 5,
      focusDistanceM: 10,
      focalLengthMm: 50,
      fNumber: 5.6,
      sensorHeightMm: 24,
      viewportHeightPx: 1000,
      maxBlurPx: 1000,
    });

    // Explicit thin-lens calculation in millimeters. Convert scene meters once.
    const f = 50;
    const objectMm = 5_000;
    const focusMm = 10_000;
    const objectImageMm = (f * objectMm) / (objectMm - f);
    const focusImageMm = (f * focusMm) / (focusMm - f);
    const sensorBlurMm = ((f / 5.6) * Math.abs(objectImageMm - focusImageMm)) / objectImageMm;
    expect(pixels).toBeCloseTo((sensorBlurMm / 24) * 1000, 10);
  });

  it('bounds the result to the configured blur and image resolution', () => {
    const input = {
      objectDistanceM: 0.051,
      focusDistanceM: 10,
      focalLengthMm: 50,
      fNumber: 1.4,
      viewportHeightPx: 300,
    } as const;
    expect(circleOfConfusionPixels({ ...input, maxBlurPx: 12 })).toBe(12);
    expect(circleOfConfusionPixels({ ...input, maxBlurPx: 900 })).toBe(300);
    expect(circleOfConfusionPixels({ ...input, viewportHeightPx: 100 })).toBeLessThanOrEqual(25);
  });

  it('normalizes diameter to a bounded gather radius with the diameter-to-radius half factor', () => {
    expect(normalizedGatherRadiusFromDiameterPx(20, 20)).toBe(0.5);
    expect(normalizedGatherRadiusFromDiameterPx(-40, 20)).toBe(1);
    expect(normalizedGatherRadiusFromDiameterPx(400, 20)).toBe(1);
    expect(normalizedGatherRadiusFromDiameterPx(0, 20)).toBe(0);
    expect(() => normalizedGatherRadiusFromDiameterPx(Number.NaN, 20)).toThrow(RangeError);
    expect(() => normalizedGatherRadiusFromDiameterPx(10, 0)).toThrow(RangeError);
  });

  it('rejects invalid optical values and distances inside the focal plane', () => {
    expect(() => verticalFovDegrees(0)).toThrow(RangeError);
    expect(() => verticalFovDegrees(50, Number.NaN)).toThrow(RangeError);
    expect(() =>
      circleOfConfusionPixels({
        objectDistanceM: 0.05,
        focusDistanceM: 10,
        focalLengthMm: 50,
        fNumber: 4,
        viewportHeightPx: 1000,
      }),
    ).toThrow(/must exceed focal length/);
    expect(() =>
      circleOfConfusionPixels({
        objectDistanceM: 5,
        focusDistanceM: 10,
        focalLengthMm: 50,
        fNumber: 0,
        viewportHeightPx: 1000,
      }),
    ).toThrow(RangeError);
  });
});
