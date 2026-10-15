import { describe, expect, it } from 'vitest';
import { RenderQualityController, renderQualitySettings } from '../web/src/render/quality';

describe('matched review quality', () => {
  it('keeps resolution, shadows and reflections identical across displays and adaptive histories', () => {
    for (const profile of ['high', 'balanced'] as const) {
      for (const level of [0, 1, 2] as const) {
        for (const dpr of [1, 1.25, 2, 3]) {
          expect(renderQualitySettings(profile, level, dpr, true)).toEqual({
            pixelRatio: 1, shadowSize: 2048, reflectionScale: .35,
          });
        }
      }
    }
  });
  it('returns to the selected adaptive profile when the review lock is released', () => {
    const controller = new RenderQualityController('balanced');
    feed(controller, 60, 0, 30_000);
    expect(controller.level).toBe(2);
    expect(renderQualitySettings('balanced', controller.level, 2, true).pixelRatio).toBe(1);
    controller.reset();
    expect(renderQualitySettings('balanced', controller.level, 2, false)).toEqual({
      pixelRatio: .8, shadowSize: 512, reflectionScale: .15,
    });
    expect(controller.snapshot().reason).toBe('startup');
  });
});

function feed(controller: RenderQualityController, frameMs: number, start: number, end: number, step = 50): number[] {
  const changes: number[] = [];
  for (let now = start; now <= end; now += step) {
    const level = controller.record(frameMs, now);
    if (level !== null) changes.push(level);
  }
  return changes;
}

describe('RenderQualityController', () => {
  it('ignores startup cost and downgrades only after several seconds of sustained slow frames', () => {
    const quality = new RenderQualityController('high');
    quality.record(34, 0);
    expect(feed(quality, 34, 50, 1_950)).toEqual([]);
    expect(quality.snapshot(1_950)).toMatchObject({ level: 0, reason: 'startup', warmupRemainingMs: 50 });

    expect(feed(quality, 34, 2_000, 5_300)).toEqual([]); // Less than the slow dwell.
    expect(quality.level).toBe(0);
    expect(feed(quality, 34, 5_350, 6_000)).toEqual([1]);
    expect(quality.snapshot(6_000)).toMatchObject({ level: 1, reason: 'cooldown', cooldownRemainingMs: expect.any(Number) });
  });

  it('does not drop quality for one hitch or frames excluded by a visibility gap', () => {
    const quality = new RenderQualityController('high');
    quality.record(15, 0);
    feed(quality, 15, 50, 2_000);
    quality.record(220, 2_016);
    expect(quality.level).toBe(0);
    feed(quality, 15, 2_032, 7_000);
    expect(quality.level).toBe(0);

    feed(quality, 30, 7_050, 9_000);
    expect(quality.snapshot(9_000).slowForMs).toBeGreaterThan(0);
    quality.record(30, 9_300); // A throttled/background gap clears dwell evidence.
    expect(quality.snapshot(9_300)).toMatchObject({ reason: 'gap', slowForMs: 0, recoveryForMs: 0 });
    expect(quality.level).toBe(0);
  });

  it('does not oscillate near the threshold when render intervals vary', () => {
    const quality = new RenderQualityController('high');
    let now = 0;
    quality.record(16.7, now);
    const intervals = [8, 33, 17, 49, 11, 24, 16];
    let frame = 0;
    while (now < 22_000) {
      now += intervals[frame % intervals.length];
      const frameMs = frame++ % 2 === 0 ? 14 : 20;
      expect(quality.record(frameMs, now)).toBeNull();
    }
    expect(quality.snapshot(now)).toMatchObject({ level: 0, slowForMs: 0, recoveryForMs: 0, reason: 'stable' });
  });

  it('restores one quality level only after sustained near-target performance and observes cooldown', () => {
    const quality = new RenderQualityController('high');
    quality.record(30, 0);
    feed(quality, 30, 50, 2_000);
    const downgrade = feed(quality, 30, 2_050, 6_000);
    expect(downgrade).toEqual([1]);
    expect(quality.snapshot(6_100).cooldownRemainingMs).toBeGreaterThan(0);

    const cooldownEnd = 6_100 + quality.snapshot(6_100).cooldownRemainingMs;
    feed(quality, 10, 6_050, cooldownEnd - 50);
    expect(quality.level).toBe(1); // Recovery during cooldown is not banked.
    expect(quality.snapshot(cooldownEnd).recoveryForMs).toBe(0);
    expect(feed(quality, 10, cooldownEnd, cooldownEnd + 17_950)).toEqual([]);
    expect(quality.level).toBe(1);
    expect(feed(quality, 10, cooldownEnd + 18_000, cooldownEnd + 18_000)).toEqual([0]);
    expect(quality.snapshot(cooldownEnd + 18_000)).toMatchObject({ level: 0, reason: 'cooldown' });
  });

  it('recovers two levels at a capped 60 Hz and remains stable through alternating near-target load', () => {
    const quality = new RenderQualityController('high');
    quality.record(30, 0);
    expect(feed(quality, 30, 50, 30_000)).toEqual([1, 2]);
    expect(quality.level).toBe(2);

    // A refresh-capped display cannot produce frame intervals below 16.67ms.
    // It should still recover each level after the cooldown and dwell periods.
    const recovery = 1000 / 60;
    const changes = feed(quality, recovery, 30_050, 90_000);
    expect(changes).toEqual([1, 0]); // One-step recovery; no skipped levels.
    expect(quality.snapshot(90_000)).toMatchObject({ level: 0, reason: 'stable' });

    let now = 90_000;
    const alternating: number[] = [];
    let index = 0;
    while (now < 120_000) {
      now += 50;
      const changed = quality.record(index++ % 2 === 0 ? 16 : 20, now);
      if (changed !== null) alternating.push(changed);
    }
    expect(alternating).toEqual([]);
    expect(quality.level).toBe(0);
  });

  it('can recover Balanced at a refresh-capped 30 Hz interval', () => {
    const quality = new RenderQualityController('balanced');
    quality.record(50, 0);
    expect(feed(quality, 50, 50, 7_000)).toEqual([1]);
    expect(feed(quality, 1000 / 30, 7_050, 45_000)).toEqual([0]);
    expect(quality.level).toBe(0);
  });

  it('uses the selected frame-rate profile and resets evidence and cooldown on profile changes', () => {
    const quality = new RenderQualityController('high');
    quality.record(35, 0);
    feed(quality, 35, 50, 2_000);
    expect(feed(quality, 35, 2_050, 6_000)).toEqual([1]);

    quality.setProfile('balanced');
    expect(quality.snapshot(6_000)).toMatchObject({
      level: 1,
      profile: 'balanced',
      reason: 'startup',
      slowForMs: 0,
      recoveryForMs: 0,
      cooldownRemainingMs: 0,
      warmupRemainingMs: 2_000,
    });
    quality.record(38, 6_000);
    expect(feed(quality, 38, 6_050, 10_000)).toEqual([]); // ~26 fps is adequate for Balanced's 30 fps target tolerance.
    expect(quality.level).toBe(1);
  });

  it('resets measurement when ineligible and handles a backwards clock without changing level', () => {
    const quality = new RenderQualityController('high');
    quality.record(28, 100);
    feed(quality, 28, 150, 2_400);
    const dwellBefore = quality.snapshot(2_400).slowForMs;
    expect(dwellBefore).toBeGreaterThan(0);
    quality.record(28, 2_416, false);
    expect(quality.snapshot(2_416)).toMatchObject({ reason: 'ineligible', slowForMs: 0, smoothedFrameMs: null });
    quality.record(28, 2_500);
    quality.record(28, 1_000);
    expect(quality.snapshot(1_000)).toMatchObject({ reason: 'clock-reset', level: 0, slowForMs: 0 });
  });
});
