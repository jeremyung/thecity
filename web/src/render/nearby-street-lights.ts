import * as THREE from 'three/webgpu';

export interface StreetLightSource {
  key: string;
  x: number;
  y: number;
  z: number;
  targetX: number;
  targetZ: number;
}

/** Fixed shader light count. Selection changes fade out before repositioning. */
export class NearbyStreetLights {
  readonly root = new THREE.Group();
  enabled = true;
  private sources: StreetLightSource[] = [];
  private slots = Array.from({ length: 4 }, () => ({
    light: new THREE.SpotLight('#ffd5a2', 0, 18, Math.PI * 0.32, 0.85, 2),
    source: null as StreetLightSource | null,
    gain: 0,
  }));
  private desired = new Set<string>();
  private elapsed = 1;
  constructor() {
    this.root.name = 'nearby-street-lights';
    for (const slot of this.slots) {
      slot.light.castShadow = false;
      this.root.add(slot.light, slot.light.target);
    }
  }
  setSources(sources: readonly StreetLightSource[]) {
    const keys = new Set<string>(),
      positions = new Set<string>();
    this.sources = [];
    for (const p of sources) {
      if (![p.x, p.y, p.z, p.targetX, p.targetZ].every(Number.isFinite)) continue;
      const position = `${p.x}:${p.y}:${p.z}`;
      if (keys.has(p.key) || positions.has(position)) continue;
      keys.add(p.key);
      positions.add(position);
      this.sources.push(p);
      if (this.sources.length === 1000) break;
    }
    const byKey = new Map(this.sources.map((p) => [p.key, p]));
    for (const slot of this.slots)
      if (slot.source) {
        const next = byKey.get(slot.source.key);
        if (next) slot.source = next;
        else {
          slot.source = null;
          slot.gain = 0;
          slot.light.intensity = 0;
        }
      }
    this.elapsed = 1;
  }
  reset() {
    this.setSources([]);
    this.desired.clear();
  }
  update(dt: number, focus: { x: number; z: number }, night: number) {
    const step = Math.max(0, Math.min(0.1, dt));
    this.elapsed += step;
    if (this.elapsed >= 0.5) {
      this.elapsed = 0;
      const retained = new Set(this.slots.flatMap((s) => (s.source ? [s.source.key] : [])));
      // Slight preference for current fixtures prevents ties from thrashing.
      this.desired = new Set(
        this.sources
          .map((source) => ({
            source,
            distance: Math.hypot(source.x - focus.x, source.z - focus.z),
          }))
          .filter((p) => p.distance < 90)
          .sort(
            (a, b) =>
              a.distance * (retained.has(a.source.key) ? 0.8 : 1) -
                b.distance * (retained.has(b.source.key) ? 0.8 : 1) ||
              a.source.key.localeCompare(b.source.key),
          )
          .slice(0, 4)
          .map((p) => p.source.key),
      );
    }
    for (const slot of this.slots) {
      const keep = slot.source !== null && this.desired.has(slot.source.key);
      slot.gain = THREE.MathUtils.clamp(slot.gain + (keep ? 1 : -1) * step * 2, 0, 1);
      if (slot.gain === 0 && !keep) {
        const occupied = new Set(this.slots.flatMap((s) => (s.source ? [s.source.key] : [])));
        slot.source =
          this.sources.find((p) => this.desired.has(p.key) && !occupied.has(p.key)) ?? null;
        if (slot.source) {
          const p = slot.source;
          slot.light.position.set(p.x, p.y, p.z);
          slot.light.target.position.set(p.targetX, 0, p.targetZ);
        }
      }
      slot.light.intensity = this.enabled ? 65 * slot.gain * THREE.MathUtils.clamp(night, 0, 1) : 0;
    }
  }
}
