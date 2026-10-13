import * as THREE from 'three/webgpu';
import type { Road, World, WorldNode } from '../types';
import { SidewalkBuild } from './sidewalk-build';

type SidewalkWorld = Pick<World, 'nodes' | 'roads'>;

/** Owns the committed sidewalk mesh while rebuilding changed road layouts off-scene.
 * Same-world updates coalesce behind one active build, so frequent growth cannot
 * starve publication. Geometry can briefly lag the latest topology. The caller
 * MUST reset on world replacement; a queued snapshot never crosses that boundary.
 */
export class CitySidewalks {
  readonly root = new THREE.Group();
  private signature = '';
  private activeSignature = '';
  private committedSignature = '';
  private job: SidewalkBuild | null = null;
  private queued: { snapshot: SidewalkWorld; signature: string } | null = null;
  private failed = false;
  private mesh: THREE.Mesh | null = null;

  constructor(
    private readonly material: () => THREE.Material,
    private readonly retire: (dispose: () => void) => void,
  ) {}

  get pendingCount(): 0 | 1 { return this.job || this.queued ? 1 : 0; }

  setWorld(world: SidewalkWorld): void {
    const snapshot = this.snapshot(world);
    const signature = JSON.stringify(snapshot);
    if (signature === this.signature) return;
    this.signature = signature;
    if (signature === this.activeSignature) {
      this.queued = null;
      return;
    }
    if (!this.job && signature === this.committedSignature) {
      this.queued = null;
      return;
    }
    if (this.job) {
      this.queued = { snapshot, signature };
      return;
    }
    this.start(snapshot, signature);
    this.failed = false;
  }

  /** Advances one bounded builder step and swaps only after the whole batch is ready. */
  update(): void {
    if (!this.job || this.failed) return;
    const job = this.job;
    let stagedGeometry: THREE.BufferGeometry | null = null;
    try {
      if (!job.step()) return;
      stagedGeometry = job.takeGeometry();
      job.dispose();
      this.job = null;
      const completedSignature = this.activeSignature;
      this.activeSignature = '';
      if (!stagedGeometry) {
        const old = this.mesh;
        this.mesh = null;
        old?.removeFromParent();
        this.committedSignature = completedSignature;
        if (old) this.retire(() => old.geometry.dispose());
        this.startQueued();
        return;
      }
      const next = new THREE.Mesh(stagedGeometry, this.material());
      stagedGeometry = null;
      next.castShadow = true;
      next.receiveShadow = true;
      const old = this.mesh;
      this.root.add(next);
      this.mesh = next;
      old?.removeFromParent();
      this.committedSignature = completedSignature;
      if (old) this.retire(() => old.geometry.dispose());
      this.startQueued();
    } catch (error) {
      stagedGeometry?.dispose();
      job.dispose();
      this.job = null;
      this.activeSignature = '';
      this.queued = null;
      this.signature = this.committedSignature;
      this.failed = true;
      throw error;
    }
  }

  reset(): void {
    this.job?.dispose();
    this.job = null;
    this.activeSignature = '';
    this.queued = null;
    this.failed = false;
    this.signature = '';
    this.committedSignature = '';
    const old = this.mesh;
    this.mesh = null;
    old?.removeFromParent();
    if (old) this.retire(() => old.geometry.dispose());
  }

  private start(snapshot: SidewalkWorld, signature: string): void {
    this.job = new SidewalkBuild(snapshot);
    this.activeSignature = signature;
  }

  private startQueued(): void {
    const queued = this.queued;
    this.queued = null;
    if (!queued || queued.signature === this.committedSignature) return;
    this.start(queued.snapshot, queued.signature);
  }

  private snapshot(world: SidewalkWorld): SidewalkWorld {
    const roads = world.roads
      .filter(r => r.kind !== 'rail' && r.kind !== 'expressway')
      .map(r => ({ id: r.id, from: r.from, to: r.to, kind: r.kind, stage: r.stage === 'open' ? 'open' : 'closed' }))
      .sort((a, b) => a.id - b.id);
    const endpointIds = new Set(roads.flatMap(r => [r.from, r.to]));
    const nodes = world.nodes
      .filter(n => endpointIds.has(n.id))
      .map(n => ({ id: n.id, x: n.x, z: n.z, kind: n.kind }))
      .sort((a, b) => a.id - b.id);
    // Sidewalk generation reads only id/x/z from nodes and id/from/to/kind/stage from roads.
    return { nodes: nodes as WorldNode[], roads: roads as Road[] };
  }
}
