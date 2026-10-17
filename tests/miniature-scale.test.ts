import { describe, expect, it } from 'vitest';
import { miniatureOpticalScale } from '../web/src/optics/miniature-scale';
import { signedCircleOfConfusionPixels } from '../web/src/optics/lens';

function blur(focus: number, ratio: number) {
  return signedCircleOfConfusionPixels({
    objectDistanceM: focus * ratio,
    focusDistanceM: focus,
    focalLengthMm: 50,
    fNumber: 2.8,
    viewportHeightPx: 800,
    maxBlurPx: 40,
    opticalModelScale: miniatureOpticalScale(focus),
  });
}
describe('miniature distance scale', () => {
  it('keeps the established close-view optics', () => {
    for (const focus of [0.1, 10, 100, 116.1, 120]) expect(miniatureOpticalScale(focus)).toBe(0.01);
  });
  it('retains near/far separation at equivalent relative scene depths as the camera pulls back', () => {
    for (const ratio of [0.5, 1, 2]) {
      expect(blur(692, ratio)).toBeCloseTo(blur(160.4, ratio), 10);
      expect(blur(2000, ratio)).toBeCloseTo(blur(160.4, ratio), 10);
    }
    expect(blur(692, 0.5)).toBeLessThan(0);
    expect(blur(692, 1)).toBe(0);
    expect(blur(692, 2)).toBeGreaterThan(10);
  });
  it('has no jump at the close-to-overview transition', () => {
    expect(blur(120.00001, 2)).toBeCloseTo(blur(119.99999, 2), 5);
  });
  it('returns a finite positive fallback for invalid focus inputs', () => {
    for (const focus of [NaN, Infinity, -Infinity, 0, -1])
      expect(miniatureOpticalScale(focus)).toBe(0.01);
  });
});
