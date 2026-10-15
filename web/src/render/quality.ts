export type RenderQualityProfile = 'high' | 'balanced';
export type RenderQualityLevel = 0 | 1 | 2;
/** Review captures use one physical pixel per CSS pixel on every display. */
export function renderQualitySettings(profile: RenderQualityProfile, level: RenderQualityLevel, devicePixelRatio: number, fixedReview = false) {
  if (fixedReview) return { pixelRatio: 1, shadowSize: 2048, reflectionScale: .35 };
  const high = profile === 'high';
  return {
    pixelRatio: Math.min(devicePixelRatio, (high ? [1.6, 1.6, 1.25] : [1, 1, .8])[level]),
    shadowSize: (high ? [2048, 1536, 1024] : [1024, 768, 512])[level],
    reflectionScale: (high ? [.35, .23, .16] : [.25, .19, .15])[level],
  };
}
export type RenderQualityReason = 'startup' | 'stable' | 'slow' | 'recovery' | 'cooldown' | 'ineligible' | 'gap' | 'clock-reset';

export type RenderQualitySnapshot = {
  level: RenderQualityLevel;
  profile: RenderQualityProfile;
  reason: RenderQualityReason;
  frameMs: number | null;
  smoothedFrameMs: number | null;
  slowForMs: number;
  recoveryForMs: number;
  cooldownRemainingMs: number;
  warmupRemainingMs: number;
};

const WARMUP_MS = 2_000;
const SLOW_HOLD_MS = 3_500;
const RECOVERY_HOLD_MS = 18_000;
const COOLDOWN_MS = 8_000;
const MAX_FRAME_GAP_MS = 250;
const EWMA_TIME_CONSTANT_MS = 1_500;

/**
 * Hysteretic, bounded render-quality selection. It observes presentation frame
 * cost only; callers decide which startup/build/hidden frames are eligible.
 */
export class RenderQualityController {
  private levelValue: RenderQualityLevel = 0;
  private profileValue: RenderQualityProfile;
  private reasonValue: RenderQualityReason = 'startup';
  private lastNow: number | null = null;
  private warmupStart: number | null = null;
  private cooldownUntil = 0;
  private smoothed: number | null = null;
  private latestFrame: number | null = null;
  private slowFor = 0;
  private recoveryFor = 0;

  constructor(profile: RenderQualityProfile = 'high') {
    this.profileValue = profile;
  }

  get level(): RenderQualityLevel { return this.levelValue; }

  /** Profile changes start a fresh measurement and cooldown window. */
  setProfile(profile: RenderQualityProfile): void {
    if (profile !== 'high' && profile !== 'balanced') throw new Error(`Unknown render quality profile: ${String(profile)}`);
    this.profileValue = profile;
    this.reset();
  }

  /** Clear evidence and startup timing while keeping the chosen quality level. */
  reset(): void {
    this.lastNow = null;
    this.warmupStart = null;
    this.cooldownUntil = 0;
    this.smoothed = null;
    this.latestFrame = null;
    this.slowFor = 0;
    this.recoveryFor = 0;
    this.reasonValue = 'startup';
  }

  /**
   * Record a rendered frame. `nowMs` is monotonic presentation time. Returns
   * the new level only when a single step change occurs.
   */
  record(frameMs: number, nowMs: number, eligible = true): RenderQualityLevel | null {
    if (!eligible || !Number.isFinite(frameMs) || frameMs < 0 || !Number.isFinite(nowMs)) {
      this.reset();
      this.reasonValue = 'ineligible';
      return null;
    }

    if (this.lastNow !== null && nowMs < this.lastNow) {
      this.reset();
      this.lastNow = nowMs;
      this.warmupStart = nowMs;
      this.reasonValue = 'clock-reset';
      return null;
    }

    this.latestFrame = frameMs;
    if (this.warmupStart === null) this.warmupStart = nowMs;
    if (frameMs >= MAX_FRAME_GAP_MS) {
      this.clearEvidence();
      this.smoothed = null;
      this.lastNow = nowMs;
      this.reasonValue = 'gap';
      return null;
    }

    if (this.lastNow === null) {
      this.lastNow = nowMs;
      this.smoothed = frameMs;
      this.reasonValue = 'startup';
      return null;
    }

    const elapsed = nowMs - this.lastNow;
    this.lastNow = nowMs;
    if (elapsed >= MAX_FRAME_GAP_MS) {
      this.clearEvidence();
      this.smoothed = frameMs;
      this.reasonValue = 'gap';
      return null;
    }
    if (elapsed <= 0) return null;

    const alpha = 1 - Math.exp(-elapsed / EWMA_TIME_CONSTANT_MS);
    this.smoothed = this.smoothed === null ? frameMs : this.smoothed + (frameMs - this.smoothed) * alpha;

    if (nowMs - this.warmupStart < WARMUP_MS) {
      this.clearEvidence();
      this.reasonValue = 'startup';
      return null;
    }

    if (nowMs < this.cooldownUntil) {
      this.clearEvidence();
      this.reasonValue = 'cooldown';
      return null;
    }

    const evidenceStart = Math.max(nowMs - elapsed, this.warmupStart ?? nowMs, this.cooldownUntil);
    const evidenceElapsed = Math.max(0, nowMs - evidenceStart);

    const targetMs = this.profileValue === 'high' ? 1000 / 60 : 1000 / 30;
    const slowThreshold = targetMs * 1.2;
    // Recovery asks for sustained performance near the selected target. A
    // 60 Hz display cannot demonstrate >73 fps, so requiring large spare
    // capacity would prevent it from ever restoring quality.
    const recoveryThreshold = targetMs * 1.03;
    if (this.smoothed > slowThreshold) {
      this.slowFor += evidenceElapsed;
      this.recoveryFor = 0;
      this.reasonValue = 'slow';
      if (this.slowFor >= SLOW_HOLD_MS && this.levelValue < 2) {
        this.levelValue = (this.levelValue + 1) as RenderQualityLevel;
        this.cooldownUntil = nowMs + COOLDOWN_MS;
        this.clearEvidence();
        this.reasonValue = 'cooldown';
        return this.levelValue;
      }
    } else if (this.smoothed <= recoveryThreshold && this.levelValue > 0) {
      this.recoveryFor += evidenceElapsed;
      this.slowFor = 0;
      this.reasonValue = 'recovery';
      if (this.recoveryFor >= RECOVERY_HOLD_MS && this.levelValue > 0) {
        this.levelValue = (this.levelValue - 1) as RenderQualityLevel;
        this.cooldownUntil = nowMs + COOLDOWN_MS;
        this.clearEvidence();
        this.reasonValue = 'cooldown';
        return this.levelValue;
      }
    } else {
      this.clearEvidence();
      this.reasonValue = 'stable';
    }
    return null;
  }

  snapshot(nowMs = this.lastNow ?? 0): RenderQualitySnapshot {
    return {
      level: this.levelValue,
      profile: this.profileValue,
      reason: this.reasonValue,
      frameMs: this.latestFrame,
      smoothedFrameMs: this.smoothed,
      slowForMs: this.slowFor,
      recoveryForMs: this.recoveryFor,
      cooldownRemainingMs: Math.max(0, this.cooldownUntil - nowMs),
      warmupRemainingMs: this.warmupStart === null ? WARMUP_MS : Math.max(0, WARMUP_MS - (nowMs - this.warmupStart)),
    };
  }

  private clearEvidence(): void {
    this.slowFor = 0;
    this.recoveryFor = 0;
  }
}
