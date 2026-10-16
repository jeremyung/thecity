import { MOVEMENT_STRIDE } from '../movement';
import type { MovementSupport } from '../movement-support';
import type { MovementGeometryIdentity, VisibleInfrastructure } from './visible-infrastructure';

export interface ActorPose {
  x: number;
  z: number;
  h: number;
  k: number;
  y: number;
}
interface Target {
  pose: ActorPose;
  support: MovementSupport;
}
interface Presented {
  target: Target;
  from: ActorPose;
  pose: ActorPose;
  started: number;
  support: MovementSupport;
  identity: MovementGeometryIdentity;
  frozen: boolean;
}

/** One latest packet and one visible pose per live actor; no replay queue.
 * Re-evaluated after infrastructure commits, including while simulation is paused.
 */
export class MovementPresentation {
  private latest = new Map<number, Target>();
  private shown = new Map<number, Presented>();
  private duration = 50;
  private frames = [new Map<number, ActorPose>(), new Map<number, ActorPose>()];
  private frameIndex = 0;

  receive(
    data: Float32Array,
    support: ReadonlyMap<number, MovementSupport>,
    duration: number,
  ): void {
    this.duration = Math.max(20, Math.min(250, duration));
    this.latest.clear();
    for (let i = 0; i + MOVEMENT_STRIDE <= data.length; i += MOVEMENT_STRIDE) {
      const dependency = support.get(data[i]);
      if (!dependency) continue;
      this.latest.set(data[i], {
        pose: { x: data[i + 1], z: data[i + 2], h: data[i + 3], k: data[i + 4], y: data[i + 5] },
        support: {
          resolved: dependency.resolved,
          roads: [...dependency.roads],
          platforms: [...dependency.platforms],
        },
      });
    }
    for (const id of this.shown.keys()) if (!this.latest.has(id)) this.shown.delete(id);
  }

  sample(now: number, infrastructure: VisibleInfrastructure): Map<number, ActorPose> {
    // Borrowed frame data: valid until this buffer is reused two samples later.
    const result = this.frames[this.frameIndex];
    this.frameIndex = 1 - this.frameIndex;
    for (const id of result.keys()) if (!this.latest.has(id)) result.delete(id);
    for (const [id, target] of this.latest) {
      let shown = this.shown.get(id);
      if (shown && !infrastructure.matchesMovementIdentity(shown.support, shown.identity)) {
        this.shown.delete(id);
        shown = undefined;
      }
      if (infrastructure.canPresentMovement(target.support)) {
        if (!shown || shown.target !== target || shown.frozen) {
          // Start from the last rendered pose, never an unseen future target.
          // Until interpolation ends, both the old and new support must remain
          // visible. Sets are bounded by the finite world's dependency IDs.
          const support = shown
            ? {
                resolved: true,
                roads: [...new Set([...shown.support.roads, ...target.support.roads])],
                platforms: [...new Set([...shown.support.platforms, ...target.support.platforms])],
              }
            : target.support;
          shown = {
            target,
            from: { ...(shown?.pose ?? target.pose) },
            pose: shown?.pose ?? { ...target.pose },
            started: now,
            support,
            identity: infrastructure.captureMovementIdentity(support),
            frozen: false,
          };
          this.shown.set(id, shown);
        }
        const t = Math.min(1, Math.max(0, (now - shown.started) / this.duration));
        const a = shown.from,
          b = target.pose;
        const dh = Math.atan2(Math.sin(b.h - a.h), Math.cos(b.h - a.h));
        shown.pose.x = a.x + (b.x - a.x) * t;
        shown.pose.z = a.z + (b.z - a.z) * t;
        shown.pose.h = a.h + dh * t;
        shown.pose.k = b.k;
        // A ground sentinel is a policy, not an elevation to interpolate.
        shown.pose.y = a.y < 0 || b.y < 0 ? b.y : a.y + (b.y - a.y) * t;
        if (t === 1 && shown.support !== target.support) {
          shown.support = target.support;
          shown.identity = infrastructure.captureMovementIdentity(target.support);
        }
      } else if (shown) {
        shown.frozen = true;
      }
      if (shown) {
        const pose = result.get(id);
        if (pose) Object.assign(pose, shown.pose);
        else result.set(id, { ...shown.pose });
      } else result.delete(id);
    }
    return result;
  }

  reset(): void {
    this.latest.clear();
    this.shown.clear();
    for (const frame of this.frames) frame.clear();
  }

  withheld(visible: ReadonlyMap<number, ActorPose>): ReadonlySet<number> {
    return new Set([...this.latest.keys()].filter((id) => !visible.has(id)));
  }
}
