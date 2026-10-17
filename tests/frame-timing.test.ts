import { describe, expect, it } from 'vitest';
import { FrameTiming } from '../web/src/render/frame-timing';

describe('FrameTiming diagnostic sampler', () => {
  it('keeps only the newest 120 paired samples', () => {
    const timing = new FrameTiming();
    timing.record(0, 0, 'ready', true);
    for (let i = 1; i <= 150; i++) timing.record(i * 10, i, 'ready', true);
    expect(timing.snapshot()).toEqual({
      samples: 120,
      fps: 100,
      frameP95Ms: 10,
      cpuP95Ms: 144,
      maxFrameMs: 10,
      phase: 'ready',
    });
  });

  it('does not count the first frame of a cold phase', () => {
    const timing = new FrameTiming();
    timing.record(20, 4, 'loading', true);
    expect(timing.snapshot()).toEqual({
      samples: 0,
      fps: 0,
      frameP95Ms: 0,
      cpuP95Ms: 0,
      maxFrameMs: 0,
      phase: 'loading',
    });
    timing.record(36, 3, 'loading', true);
    expect(timing.snapshot().samples).toBe(1);
    expect(timing.snapshot().cpuP95Ms).toBe(3);
  });

  it('drops the hidden interval and starts with a fresh visible frame', () => {
    const timing = new FrameTiming();
    timing.record(0, 0, 'ready', true);
    timing.record(16, 2, 'ready', true);
    timing.record(17, 0, 'ready', false);
    expect(timing.snapshot()).toEqual({
      samples: 0,
      fps: 0,
      frameP95Ms: 0,
      cpuP95Ms: 0,
      maxFrameMs: 0,
      phase: 'hidden',
    });
    timing.record(10000, 0, 'ready', true);
    timing.record(10016, 1, 'ready', true);
    expect(timing.snapshot().samples).toBe(1);
    expect(timing.snapshot().frameP95Ms).toBe(16);
  });

  it('resets samples on loading/ready transitions', () => {
    const timing = new FrameTiming();
    timing.record(0, 0, 'loading', true);
    timing.record(10, 1, 'loading', true);
    timing.record(20, 2, 'ready', true);
    expect(timing.snapshot()).toEqual({
      samples: 0,
      fps: 0,
      frameP95Ms: 0,
      cpuP95Ms: 0,
      maxFrameMs: 0,
      phase: 'ready',
    });
    timing.record(36, 3, 'ready', true);
    expect(timing.snapshot().samples).toBe(1);
  });

  it('retains genuine slow visible frames and uses nearest-rank quantiles', () => {
    const timing = new FrameTiming();
    timing.record(0, 0, 'ready', true);
    const intervals = [10, 20, 30, 40, 50, 60, 70, 80, 90, 600];
    let now = 0;
    intervals.forEach((interval, index) => {
      now += interval;
      timing.record(now, index + 1, 'ready', true);
    });
    const snapshot = timing.snapshot();
    expect(snapshot.samples).toBe(10);
    expect(snapshot.frameP95Ms).toBe(600);
    expect(snapshot.cpuP95Ms).toBe(10);
    expect(snapshot.maxFrameMs).toBe(600);
    expect(snapshot.fps).toBeCloseTo(1000 / 105);
  });

  it('safely resets a backwards clock and ignores invalid costs', () => {
    const timing = new FrameTiming();
    timing.record(10, 1, 'ready', true);
    timing.record(20, -1, 'ready', true);
    timing.record(30, 2, 'ready', true);
    expect(timing.snapshot().samples).toBe(1);
    timing.record(25, 2, 'ready', true);
    expect(timing.snapshot().samples).toBe(0);
    timing.record(40, 2, 'ready', true);
    expect(timing.snapshot().samples).toBe(1);
  });
});
