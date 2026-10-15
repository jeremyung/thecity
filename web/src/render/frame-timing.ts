export type FrameTimingSnapshot = {
  samples: number;
  fps: number;
  frameP95Ms: number;
  cpuP95Ms: number;
  maxFrameMs: number;
  phase: 'loading' | 'ready' | 'hidden';
};

const MAX_SAMPLES = 120;

/**
 * Bounded visible-frame diagnostics only; this class does not make quality decisions.
 * CPU cost passed to record is paired with the interval ending at that call. The
 * first visible call has no preceding interval, so its CPU value is not sampled.
 */
export class FrameTiming {
  private intervals: number[] = [];
  private cpuCosts: number[] = [];
  private lastTime: number | null = null;
  private currentPhase: 'loading' | 'ready' | 'hidden' = 'hidden';

  record(nowMs: number, cpuMs: number, phase: 'loading' | 'ready', visible: boolean): void {
    if (!visible) {
      this.reset();
      this.currentPhase = 'hidden';
      return;
    }
    if (!Number.isFinite(nowMs) || !Number.isFinite(cpuMs) || cpuMs < 0) return;

    if (this.currentPhase !== phase) {
      this.reset();
      this.currentPhase = phase;
      this.lastTime = nowMs;
      return;
    }

    if (this.lastTime === null) {
      this.lastTime = nowMs;
      return;
    }

    const interval = nowMs - this.lastTime;
    if (interval <= 0) {
      this.reset();
      this.currentPhase = phase;
      this.lastTime = nowMs;
      return;
    }

    this.lastTime = nowMs;
    this.intervals.push(interval);
    this.cpuCosts.push(cpuMs);
    if (this.intervals.length > MAX_SAMPLES) {
      this.intervals.shift();
      this.cpuCosts.shift();
    }
  }

  snapshot(): FrameTimingSnapshot {
    const samples = this.intervals.length;
    if (samples === 0) {
      return { samples: 0, fps: 0, frameP95Ms: 0, cpuP95Ms: 0, maxFrameMs: 0, phase: this.currentPhase };
    }
    const total = this.intervals.reduce((sum, value) => sum + value, 0);
    return {
      samples,
      fps: 1000 * samples / total,
      frameP95Ms: nearestRank(this.intervals, 0.95),
      cpuP95Ms: nearestRank(this.cpuCosts, 0.95),
      maxFrameMs: Math.max(...this.intervals),
      phase: this.currentPhase,
    };
  }

  private reset(): void {
    this.intervals = [];
    this.cpuCosts = [];
    this.lastTime = null;
  }
}

function nearestRank(values: number[], percentile: number): number {
  const ordered = [...values].sort((a, b) => a - b);
  return ordered[Math.max(0, Math.ceil(percentile * ordered.length) - 1)];
}
