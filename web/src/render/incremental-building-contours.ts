import * as THREE from 'three/webgpu';

type Edge = { index0: number; index1: number; normal: THREE.Vector3 };
type PositionAttribute = THREE.BufferAttribute | THREE.InterleavedBufferAttribute;
type MeshJob = {
  matrix: THREE.Matrix4;
  position: PositionAttribute;
  index: THREE.BufferAttribute | null;
  vertexCursor: number;
  vertexCount: number;
  min: THREE.Vector3;
  max: THREE.Vector3;
  triangleCursor: number;
  triangleCount: number;
  edges: Map<string, Edge | null>;
  edgeIterator: IterableIterator<[string, Edge | null]> | null;
  skip: boolean;
  geometryMorph: boolean;
};
const finite = (v: THREE.Vector3) =>
  Number.isFinite(v.x) && Number.isFinite(v.y) && Number.isFinite(v.z);

/** Incrementally reproduces BuildingChunks' EdgesGeometry outline extraction. */
export class IncrementalBuildingContours {
  private work = 0;
  get processedWork() {
    return this.work;
  }
  private readonly jobs: MeshJob[] = [];
  private jobCursor = 0;
  private output: number[] = [];
  private outputBuffer: Float32Array | null = null;
  private outputCursor = 0;
  private finished = false;
  private cancelled = false;
  private readonly thresholdDot: number;
  constructor(
    group: THREE.Group,
    private minEdgeLengthSq = 12.25,
    thresholdAngle = 42,
  ) {
    this.thresholdDot = Math.cos(THREE.MathUtils.degToRad(thresholdAngle));
    group.updateMatrixWorld(true);
    group.traverse((o) => {
      if (!(o instanceof THREE.Mesh)) return;
      const geometry = o.geometry,
        position = geometry.getAttribute('position');
      if (!position) return;
      this.jobs.push({
        matrix: o.matrixWorld.clone(),
        position,
        index: geometry.getIndex(),
        vertexCursor: 0,
        vertexCount: position.count,
        min: new THREE.Vector3(Infinity, Infinity, Infinity),
        max: new THREE.Vector3(-Infinity, -Infinity, -Infinity),
        triangleCursor: 0,
        triangleCount: (geometry.index?.count ?? position.count) / 3,
        edges: new Map(),
        edgeIterator: null,
        skip: false,
        geometryMorph: !!geometry.morphAttributes.position?.length,
      });
    });
  }
  get result(): Float32Array | null {
    return this.finished && !this.cancelled ? this.outputBuffer : null;
  }
  step(workQuota: number): boolean {
    this.work = 0;
    if (this.finished || this.cancelled) return true;
    const quota = Math.max(0, Math.floor(workQuota));
    while (this.work < quota) {
      if (this.jobCursor >= this.jobs.length) {
        if (!this.outputBuffer) this.outputBuffer = new Float32Array(this.output.length);
        if (this.outputCursor < this.output.length) {
          this.outputBuffer[this.outputCursor] = this.output[this.outputCursor];
          this.outputCursor++;
          this.work++;
          continue;
        }
        this.output = [];
        this.finished = true;
        break;
      }
      const job = this.jobs[this.jobCursor];
      if (job.vertexCursor < job.vertexCount) {
        const p = new THREE.Vector3().fromBufferAttribute(job.position, job.vertexCursor++);
        job.min.min(p);
        job.max.max(p);
        this.work++;
        continue;
      }
      if (job.triangleCursor === 0) {
        // Match computeBoundingBox + Box3.applyMatrix4 over all eight corners.
        if (!job.position.count || !finite(job.min) || !finite(job.max)) {
          job.triangleCursor = -3;
        } else {
          const box = new THREE.Box3(job.min.clone(), job.max.clone()).applyMatrix4(job.matrix),
            maxAbs = Math.max(
              Math.abs(box.min.x),
              Math.abs(box.min.y),
              Math.abs(box.min.z),
              Math.abs(box.max.x),
              Math.abs(box.max.y),
              Math.abs(box.max.z),
            ),
            margin = maxAbs * 2 * 1.1920928955078125e-7,
            dx = box.max.x - box.min.x + margin,
            dy = box.max.y - box.min.y + margin,
            dz = box.max.z - box.min.z + margin;
          if (
            !job.geometryMorph &&
            finite(box.min) &&
            finite(box.max) &&
            [dx, dy, dz].every(Number.isFinite) &&
            dx * dx + dy * dy + dz * dz < this.minEdgeLengthSq
          ) {
            this.jobCursor++;
            continue;
          }
          job.triangleCursor = -3;
        }
      }
      if (job.triangleCursor === -3) job.triangleCursor = 0;
      if (job.triangleCursor >= 0 && job.triangleCursor < job.triangleCount) {
        this.triangle(job);
        this.work++;
        continue;
      }
      if (job.triangleCursor >= job.triangleCount && job.edgeIterator === null)
        job.edgeIterator = job.edges.entries();
      const entry = job.edgeIterator!.next();
      if (!entry.done) {
        this.work++;
        const edge = entry.value[1];
        if (edge) this.emit(job, edge.index0, edge.index1);
        continue;
      }
      this.jobCursor++;
    }
    return this.finished;
  }
  private triangle(j: MeshJob) {
    const base = j.triangleCursor++ * 3,
      ids = j.index
        ? [j.index.getX(base), j.index.getX(base + 1), j.index.getX(base + 2)]
        : [base, base + 1, base + 2],
      a = new THREE.Vector3().fromBufferAttribute(j.position, ids[0]),
      b = new THREE.Vector3().fromBufferAttribute(j.position, ids[1]),
      c = new THREE.Vector3().fromBufferAttribute(j.position, ids[2]),
      triangle = new THREE.Triangle(a, b, c),
      normal = new THREE.Vector3();
    triangle.getNormal(normal);
    const hashes = [a, b, c].map(
      (p) => `${Math.round(p.x * 1e4)},${Math.round(p.y * 1e4)},${Math.round(p.z * 1e4)}`,
    );
    if (hashes[0] === hashes[1] || hashes[1] === hashes[2] || hashes[2] === hashes[0]) return;
    for (let k = 0; k < 3; k++) {
      const n = (k + 1) % 3,
        x = hashes[k],
        y = hashes[n],
        key = `${x}_${y}`,
        reverse = `${y}_${x}`,
        sibling = j.edges.get(reverse);
      if (sibling) {
        if (normal.dot(sibling.normal) <= this.thresholdDot) this.emit(j, ids[k], ids[n]);
        j.edges.set(reverse, null);
      } else if (!j.edges.has(key))
        j.edges.set(key, { index0: ids[k], index1: ids[n], normal: normal.clone() });
    }
  }
  private emit(j: MeshJob, a: number, b: number) {
    const p = new THREE.Vector3(
        Math.fround(j.position.getX(a)),
        Math.fround(j.position.getY(a)),
        Math.fround(j.position.getZ(a)),
      ).applyMatrix4(j.matrix),
      q = new THREE.Vector3(
        Math.fround(j.position.getX(b)),
        Math.fround(j.position.getY(b)),
        Math.fround(j.position.getZ(b)),
      ).applyMatrix4(j.matrix),
      vals = [
        Math.fround(p.x),
        Math.fround(p.y),
        Math.fround(p.z),
        Math.fround(q.x),
        Math.fround(q.y),
        Math.fround(q.z),
      ],
      dx = vals[3] - vals[0],
      dy = vals[4] - vals[1],
      dz = vals[5] - vals[2];
    if (dx * dx + dy * dy + dz * dz >= this.minEdgeLengthSq) this.output.push(...vals);
  }
  dispose() {
    this.cancelled = true;
    this.finished = true;
    this.output = [];
    this.outputBuffer = null;
    this.jobs.length = 0;
  }
}
