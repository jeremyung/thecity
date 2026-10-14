import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { SEASON_DAYS, seasonAt, seasonKey, seasonalColor, treeColor } from '../web/src/render/seasons';

describe('city seasons', () => {
  it('cycles four seven-day seasons from spring, starting a new city fully in spring', () => {
    expect(seasonAt(0)).toEqual({ season: 'spring', previous: 'winter', blend: 1 });
    expect(seasonAt(SEASON_DAYS - 1, 1439).season).toBe('spring');
    expect(seasonAt(SEASON_DAYS).season).toBe('summer');
    expect(seasonAt(2 * SEASON_DAYS + 3).season).toBe('autumn');
    expect(seasonAt(3 * SEASON_DAYS).season).toBe('winter');
    expect(seasonAt(4 * SEASON_DAYS).season).toBe('spring');
    for (const bad of [Number.NaN, -5]) expect(seasonAt(bad).season).toBe('spring');
  });

  it('blends across the first day of each later season only', () => {
    expect(seasonAt(SEASON_DAYS, 0).blend).toBe(0);
    expect(seasonAt(SEASON_DAYS, 720).blend).toBeCloseTo(.5);
    expect(seasonAt(SEASON_DAYS + 1, 0).blend).toBe(1);
    expect(seasonKey(seasonAt(SEASON_DAYS, 0))).not.toBe(seasonKey(seasonAt(SEASON_DAYS, 720)));
    expect(seasonKey(seasonAt(SEASON_DAYS + 2, 0))).toBe(seasonKey(seasonAt(SEASON_DAYS + 3, 600)));
  });

  it('gives each species its seasonal crown', () => {
    const hsl = (c: THREE.Color) => c.getHSL({ h: 0, s: 0, l: 0 });
    const pinkish = (c: THREE.Color) => c.r > c.g && c.r > .7;
    expect(pinkish(seasonalColor('blossom', '#e5bdba', 'spring', .2))).toBe(true);
    expect(pinkish(seasonalColor('blossom', '#e5bdba', 'summer', .2))).toBe(false);
    const ginkgo = seasonalColor('ginkgo', '#7f9a5e', 'autumn', .5);
    expect(hsl(ginkgo).h).toBeGreaterThan(.07); expect(hsl(ginkgo).h).toBeLessThan(.17);
    expect(seasonalColor('broadleaf', '#758a67', 'summer', .4).getHexString()).toBe('758a67');
    expect(seasonalColor('evergreen', '#4f6e57', 'autumn', .4).getHexString()).toBe('4f6e57');
    // Winter mutes deciduous crowns toward bare branches but leaves evergreens dark green.
    expect(hsl(seasonalColor('broadleaf', '#758a67', 'winter', .4)).s).toBeLessThan(hsl(new THREE.Color('#758a67')).s);
    expect(hsl(seasonalColor('evergreen', '#4f6e57', 'winter', .4)).h).toBeGreaterThan(.3);
    const half = treeColor('ginkgo', '#7f9a5e', { season: 'autumn', previous: 'summer', blend: .5 }, .5);
    const summer = seasonalColor('ginkgo', '#7f9a5e', 'summer', .5), autumn = seasonalColor('ginkgo', '#7f9a5e', 'autumn', .5);
    expect(half.r).toBeCloseTo((summer.r + autumn.r) / 2, 5);
  });
});
