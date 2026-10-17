import type { World, RailPlatform } from '../types';
import { TRAIN_DOOR_STRIDE } from '../movement';
import { consistHalfLength } from './train-kit';

/** Movement owns door timing; committed infrastructure owns the opening side. */
export class TrainDoorState {
  private samples = new Map<number, { from: number; to: number; terminal: number }>();
  private platforms = new Map<number, RailPlatform>();
  private hasMovement = false;
  setWorld(world: World) {
    this.platforms = new Map((world.rail_platforms ?? []).map((p) => [p.node_id, p]));
    if (!this.hasMovement) {
      this.samples.clear();
      for (const train of world.trains ?? []) {
        const opening = Math.max(0, Math.min(1, train.door_open_fraction ?? 0));
        this.samples.set(train.id, {
          from: opening,
          to: opening,
          terminal: train.door_terminal ?? -1,
        });
      }
    }
  }
  setMovement(data: Float32Array) {
    const next = new Map<number, { from: number; to: number; terminal: number }>();
    for (let i = 0; i + TRAIN_DOOR_STRIDE <= data.length; i += TRAIN_DOOR_STRIDE) {
      const id = data[i],
        terminal = data[i + 1],
        to = Math.max(0, Math.min(1, data[i + 2]));
      const old = this.samples.get(id);
      next.set(id, {
        from: this.hasMovement && old ? old.to : to,
        to,
        terminal: terminal < 0 && old ? old.terminal : terminal,
      });
    }
    this.samples = next;
    this.hasMovement = true;
  }
  /** Signed fraction: positive opens local +x, negative opens local -x. */
  opening(id: number, x: number, z: number, carYaw: number, alpha: number) {
    const sample = this.samples.get(id),
      p = sample && this.platforms.get(sample.terminal);
    if (
      !sample ||
      !p ||
      Math.hypot(x - p.x - p.ux * consistHalfLength(), z - p.z - p.uz * consistHalfLength()) >= 0.03
    )
      return 0;
    const fraction = sample.from + (sample.to - sample.from) * Math.max(0, Math.min(1, alpha));
    return Math.sign(p.uz * Math.cos(carYaw) + p.ux * Math.sin(carYaw)) * fraction;
  }
  reset() {
    this.samples.clear();
    this.platforms.clear();
    this.hasMovement = false;
  }
}
