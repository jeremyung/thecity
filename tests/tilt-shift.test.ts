import { describe, it, expect } from 'vitest';
import {
  DEFAULT_TILT_SHIFT,
  normalizeTiltShift,
  tiltedFocusPlane,
  tiltedRayFocus,
} from '../web/src/optics/tilt-shift';
describe('tilted focus plane', () => {
  it('reduces to ordinary axial focus at zero tilt', () => {
    const p = tiltedFocusPlane(100, 40, { ...DEFAULT_TILT_SHIFT, tilt: 0 });
    for (const [x, y] of [
      [0, 0],
      [0.3, -0.2],
      [-0.5, 0.4],
    ])
      expect(tiltedRayFocus(p, x, y)).toBeCloseTo(100);
  });
  it('keeps the chosen band centre at subject depth', () => {
    for (const position of [0.2, 0.5, 0.8]) {
      const s = { ...DEFAULT_TILT_SHIFT, position };
      const y = (0.5 - position) * 2 * Math.tan((40 * Math.PI) / 360);
      expect(tiltedRayFocus(tiltedFocusPlane(120, 40, s), 0, y)).toBeCloseTo(120);
    }
  });
  it('rotates the focus gradient rather than changing the central focus', () => {
    const horizontal = tiltedFocusPlane(100, 40, { ...DEFAULT_TILT_SHIFT, angle: 0 });
    const vertical = tiltedFocusPlane(100, 40, { ...DEFAULT_TILT_SHIFT, angle: 90 });
    expect(tiltedRayFocus(horizontal, 0.1, 0)).toBeCloseTo(100);
    expect(tiltedRayFocus(vertical, 0, 0.1)).toBeCloseTo(100);
    expect(tiltedRayFocus(horizontal, 0, 0.1)).toBeCloseTo(tiltedRayFocus(vertical, 0.1, 0)!);
  });
  it('rejects intersections behind the camera or at the horizon', () => {
    expect(tiltedRayFocus([0, 1, 0, 100], 0, 0)).toBeNull();
    expect(tiltedRayFocus([0, 1, 0, 100], 0, -1)).toBeNull();
  });
  it('bounds malformed control values', () => {
    expect(
      normalizeTiltShift({ position: NaN, angle: Infinity, blur: 80, width: -1 }),
    ).toMatchObject({ position: 0.5, angle: 0, blur: 32, width: 0.1 });
  });
  it('moves a vertical band horizontally when position changes', () => {
    const settings = { ...DEFAULT_TILT_SHIFT, position: 0.3, angle: 90 };
    const offset = (0.5 - settings.position) * 2 * Math.tan((40 * Math.PI) / 360);
    const p = tiltedFocusPlane(100, 40, settings);
    expect(tiltedRayFocus(p, offset, 0)).toBeCloseTo(100);
    expect(tiltedRayFocus(p, 0, 0)).not.toBeCloseTo(100);
  });
  it('preserves intersections when both sides of a plane equation change sign', () => {
    const p = tiltedFocusPlane(100, 40, DEFAULT_TILT_SHIFT);
    const reversed = p.map((x) => -x);
    expect(tiltedRayFocus(reversed, 0.1, 0.2)).toBeCloseTo(tiltedRayFocus(p, 0.1, 0.2)!);
  });
});
