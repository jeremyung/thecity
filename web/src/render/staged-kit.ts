import * as THREE from 'three/webgpu';
import { Kit } from './kit';
import { boxGeometry } from './box-geometry';
import { IncrementalGeometryMerge } from './incremental-geometry-merge';

type Primitive =
  | { kind: 'box'; args: [number, number, number, number, number, number, number]; material: THREE.Material; offset: THREE.Vector3; order: number }
  | { kind: 'cylinder' | 'cone'; args: [number, number, number, number, number, number]; material: THREE.Material; segments: number; offset: THREE.Vector3; order: number };
type Part = { geometry: THREE.BufferGeometry; owned: boolean; order: number };
type MaterialJob = { material: THREE.Material; parts: Part[]; merger: IncrementalGeometryMerge | null };

/** Kit whose primitive construction and per-material concatenation can be advanced in bounded calls. */
export class StagedKit extends Kit {
  private commands: Primitive[] = [];
  private commandIndex = 0;
  private stagedParts = new Map<THREE.Material, Part[]>();
  private jobs: MaterialJob[] = [];
  private jobIndex = 0;
  private stagedOffset = new THREE.Vector3();
  private phase: 'queue' | 'merge' | 'complete' | 'disposed' = 'queue';
  private output: THREE.Group | null = null;
  private taken = false;
  private started = false;
  private nextOrder = 0;

  override withOffset<T>(x: number, y: number, z: number, build: () => T): T {
    this.assertQueue();
    const previous = this.stagedOffset;
    this.stagedOffset = previous.clone().add(new THREE.Vector3(x, y, z));
    try { return build(); } finally { this.stagedOffset = previous; }
  }

  override box(x: number, y: number, z: number, w: number, h: number, d: number, mat: THREE.Material, rotation = 0): void {
    this.enqueue({ kind: 'box', args: [x, y, z, w, h, d, rotation], material: mat, offset: this.stagedOffset.clone(), order: this.nextOrder++ });
  }
  override cylinder(x: number, y: number, z: number, r: number, h: number, mat: THREE.Material, segments = 8): void {
    this.enqueue({ kind: 'cylinder', args: [x, y, z, r, h, 0], material: mat, segments, offset: this.stagedOffset.clone(), order: this.nextOrder++ });
  }
  override cone(x: number, y: number, z: number, radius: number, h: number, mat: THREE.Material, segments = 6): void {
    this.enqueue({ kind: 'cone', args: [x, y, z, radius, h, 0], material: mat, segments, offset: this.stagedOffset.clone(), order: this.nextOrder++ });
  }
  override add(geo: THREE.BufferGeometry, mat: THREE.Material): void {
    this.assertQueue();
    if (this.stagedOffset.lengthSq()) geo.translate(this.stagedOffset.x, this.stagedOffset.y, this.stagedOffset.z);
    const list = this.stagedParts.get(mat) ?? [];
    list.push({ geometry: geo, owned: true, order: this.nextOrder++ });
    this.stagedParts.set(mat, list);
  }
  override finish(_cast = true): THREE.Group { throw new Error('StagedKit.finish() is synchronous; use step() and takeGroup()'); }

  /** Process up to 128 queued primitives, or advance exactly one material merge step. */
  step(): boolean {
    if (this.phase === 'disposed') throw new Error('StagedKit has been disposed');
    if (this.phase === 'complete') return true;
    this.started = true;
    if (this.phase === 'queue') {
      const count = Math.min(this.commands.length - this.commandIndex, 128);
      for (let i = 0; i < count; i++) {
        const command = this.commands[this.commandIndex++];
        this.construct(command);
      }
      if (this.commandIndex < this.commands.length) return false;
      this.commands = [];
      this.commandIndex = 0;
      this.phase = 'merge';
      this.jobs = [...this.stagedParts].map(([material, parts]) => ({ material, parts: parts.sort((a, b) => a.order - b.order), merger: null }));
      this.stagedParts.clear();
      if (!this.jobs.length) { this.phase = 'complete'; this.output = new THREE.Group(); return true; }
      // Construction calls stay exclusive to this step, even when the last primitive was reached.
      return false;
    }
    const job = this.jobs[this.jobIndex];
    if (!job.merger) job.merger = new IncrementalGeometryMerge(job.parts.map(part => part.geometry));
    const done = job.merger.step(12000, 36000);
    if (done) {
      const geometry = job.merger.result!;
      const mesh = new THREE.Mesh(geometry, job.material);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      (this.output ??= new THREE.Group()).add(mesh);
      for (const part of job.parts) if (part.owned) part.geometry.dispose();
      job.parts = [];
      job.merger = null;
      this.jobIndex++;
      if (this.jobIndex >= this.jobs.length) { this.jobs = []; this.phase = 'complete'; return true; }
    }
    return false;
  }

  takeGroup(): THREE.Group | null {
    if (this.phase !== 'complete' || this.taken) return null;
    this.taken = true;
    const group = this.output;
    this.output = null;
    return group;
  }

  dispose(): void {
    if (this.phase === 'disposed') return;
    this.phase = 'disposed';
    this.commands = [];
    this.commandIndex = 0;
    for (const parts of this.stagedParts.values()) for (const part of parts) if (part.owned) part.geometry.dispose();
    this.stagedParts.clear();
    for (const job of this.jobs) {
      job.merger?.dispose();
      for (const part of job.parts) if (part.owned) part.geometry.dispose();
      job.parts = [];
    }
    this.jobs = [];
    if (this.output) {
      this.output.traverse(object => { if (object instanceof THREE.Mesh) object.geometry.dispose(); });
      this.output.removeFromParent();
      this.output = null;
    }
  }

  private enqueue(command: Primitive) {
    this.assertQueue();
    // Reserve material order now: explicit geometry may arrive before this
    // primitive is constructed on a later frame.
    if (!this.stagedParts.has(command.material)) this.stagedParts.set(command.material, []);
    this.commands.push(command);
  }
  private assertQueue() { if (this.phase !== 'queue' || this.started) throw new Error('StagedKit no longer accepts geometry'); }
  private construct(command: Primitive) {
    const [x, y, z] = command.args;
    let geometry: THREE.BufferGeometry;
    if (command.kind === 'box') geometry = boxGeometry(x, y, z, command.args[3], command.args[4], command.args[5], command.args[6]);
    else {
      geometry = command.kind === 'cylinder'
        ? new THREE.CylinderGeometry(command.args[3], command.args[3], command.args[4], command.segments)
        : new THREE.ConeGeometry(command.args[3], command.args[4], command.segments);
      geometry.translate(x, y, z);
    }
    if (command.offset.lengthSq()) geometry.translate(command.offset.x, command.offset.y, command.offset.z);
    const list = this.stagedParts.get(command.material) ?? [];
    list.push({ geometry, owned: true, order: command.order });
    this.stagedParts.set(command.material, list);
  }
}
