import * as THREE from 'three/webgpu';
import { IncrementalBuildingContours } from './incremental-building-contours';
import { IncrementalGeometryMerge } from './incremental-geometry-merge';
import { IncrementalGeometryTransform } from './incremental-geometry-transform';

export type BuildingBatchSurface = { material: THREE.Material; color?: THREE.Color };
type BuildingSurfaceResolver = (material: THREE.Material) => THREE.Material | BuildingBatchSurface;
export interface RebuildableBuildingSource {
  createGroup(): THREE.Group;
  disposeGroup(group: THREE.Group): void;
}
type Source = {
  id: number;
  key: string;
  contours: Float32Array | null;
  descriptor: RebuildableBuildingSource | null;
  pendingGroup: THREE.Group | null;
  legacyGroup: THREE.Group | null;
  leases: number;
  retired: boolean;
  released: boolean;
};
type MeshInput = { id: number; mesh: THREE.Mesh | null; matrix: THREE.Matrix4; vertices: number };
type PreparedPart = {
  geometry: THREE.BufferGeometry;
  id: number;
  shadow: boolean;
  triangles: number;
};
type MaterialJob = {
  material: THREE.Material;
  parts: PreparedPart[];
  merger: IncrementalGeometryMerge | null;
};
type TransformJob = {
  input: MeshInput;
  transform: IncrementalGeometryTransform;
  material: THREE.Material;
  shadow: boolean;
  triangles: number;
};
type RetiredSource = { afterRevision: number; release: () => void };
type ContourSource = { id: number; values: Float32Array };
type ContourSpan = { id: number; start: number; end: number };
type Build = {
  key: string;
  revision: number;
  sources: Source[];
  sourceCursor: number;
  currentSource: Source | null;
  currentGroup: THREE.Group | null;
  groupMeshes: MeshInput[];
  groupMeshCursor: number;
  transform: TransformJob | null;
  grouped: Map<THREE.Material, PreparedPart[]>;
  jobs: MaterialJob[] | null;
  jobCursor: number;
  next: THREE.Group;
  contourSources: ContourSource[];
  contourSpans: ContourSpan[];
  contourSourceCursor: number;
  contourOffset: number;
  contourOutput: Float32Array;
  contourCursor: number;
  contourMin: THREE.Vector3;
  contourMax: THREE.Vector3;
  contoursDone: boolean;
  contoursInitialized: boolean;
  contourBuilder: IncrementalBuildingContours | null;
  ownedResources: Array<THREE.Material | THREE.Texture>;
  ownedResourceSet: Set<THREE.Material | THREE.Texture>;
};

export const buildingChunkKey = (x: number, z: number) =>
  `${Math.floor((x + 400) / 200)}:${Math.floor((z + 400) / 200)}`;
const MAX_DEFERRED_UPDATES = 12;
const MAX_VERTICES_PER_UPDATE = 12_000;
const MAX_INDICES_PER_UPDATE = 36_000;
const MAX_CONTOUR_FLOATS_PER_UPDATE = 24_000;
const MAX_PREPARED_VERTICES_PER_UPDATE = 12_000;
const MAX_PREPARED_MESHES_PER_UPDATE = 16;
const MAX_CONTOUR_WORK_PER_UPDATE = 8_000;
const CONTOUR_WORK_SLICE = 500;
const CONTOUR_TIME_BUDGET_MS = 1.5;
const MIN_CONTOUR_EDGE_LENGTH_SQ = 12.25;

/** Retains authored detail while rebuilding static neighborhood batches over multiple updates. */
export class BuildingChunks {
  readonly root = new THREE.Group();
  private sources = new Map<number, Source>();
  private batches = new Map<string, THREE.Group>();
  /** Source descriptors whose geometry is represented by each committed batch. */
  private committedSources = new Map<string, Map<number, RebuildableBuildingSource | null>>();
  private dirty = new Set<string>();
  private dirtySince = new Map<string, number>();
  private revisions = new Map<string, number>();
  private updateSequence = 0;
  private retired = new Map<string, RetiredSource[]>();
  private active: Build | null = null;
  private contoursVisible = true;

  constructor(
    private material: BuildingSurfaceResolver,
    private retire: (dispose: () => void) => void,
    private contourMaterial?: THREE.LineBasicNodeMaterial,
    private keySelector?: (id: number, group: THREE.Group) => string,
    private canCommit?: (
      sources: ReadonlyMap<number, RebuildableBuildingSource | null>,
      previous: ReadonlyMap<number, RebuildableBuildingSource | null>,
    ) => boolean,
  ) {}

  get pendingCount() {
    return this.dirty.size + (this.active && !this.dirty.has(this.active.key) ? 1 : 0);
  }
  /** True only when this exact descriptor's source is present in a visible committed chunk. */
  isSourceVisible(id: number, descriptor: RebuildableBuildingSource): boolean {
    for (const [key, sources] of this.committedSources)
      if (sources.get(id) === descriptor && this.batches.get(key)?.visible !== false) return true;
    return false;
  }
  hasVisibleSource(id: number): boolean {
    for (const [key, sources] of this.committedSources)
      if (sources.has(id) && this.batches.get(key)?.visible !== false) return true;
    return false;
  }
  hasCommittedSource(id: number, descriptor?: RebuildableBuildingSource): boolean {
    for (const sources of this.committedSources.values())
      if (sources.has(id) && (descriptor === undefined || sources.get(id) === descriptor))
        return true;
    return false;
  }
  /** Retire one committed source in place when its exact buffer work fits existing per-update quotas. */
  tryRetireCommittedSource(id: number): boolean {
    const batches: Array<{ key: string; group: THREE.Group }> = [];
    for (const [key, sources] of this.committedSources)
      if (sources.has(id)) {
        const group = this.batches.get(key);
        if (group) batches.push({ key, group });
      }
    if (!batches.length) return true;
    let indexWork = 0,
      contourWork = 0,
      unsupported = false;
    const meshSpans: Array<{
      mesh: THREE.Mesh;
      spans: Array<{ first: number; end: number; id: number }>;
      target: Array<{ first: number; end: number }>;
    }> = [];
    const lineSpans: Array<{
      line: THREE.LineSegments;
      spans: ContourSpan[];
      target: ContourSpan[];
    }> = [];
    for (const { group } of batches)
      group.traverse((object) => {
        if (object instanceof THREE.Mesh) {
          const spans = object.userData.buildingSpans as
            Array<{ first: number; end: number; id: number }> | undefined;
          const target = (spans ?? []).filter((span) => span.id === id);
          if (!target.length) return;
          const index = object.geometry.index;
          if (
            !index?.isBufferAttribute ||
            !spans ||
            target.some(
              (span) =>
                !Number.isInteger(span.first) ||
                !Number.isInteger(span.end) ||
                span.first < 0 ||
                span.end < span.first ||
                span.end * 3 > index.count,
            )
          ) {
            unsupported = true;
            return;
          }
          indexWork += target.reduce((sum, span) => sum + (span.end - span.first) * 3, 0);
          meshSpans.push({ mesh: object, spans, target });
        } else if (object instanceof THREE.LineSegments) {
          const spans = object.userData.buildingContourSpans as ContourSpan[] | undefined;
          const target = (spans ?? []).filter((span) => span.id === id);
          if (!target.length) return;
          const position = object.geometry.getAttribute('position');
          if (
            !position?.isBufferAttribute ||
            !spans ||
            target.some(
              (span) =>
                !Number.isInteger(span.start) ||
                !Number.isInteger(span.end) ||
                span.start < 0 ||
                span.end < span.start ||
                span.end > position.count * 3 ||
                span.start % 6 !== 0 ||
                span.end % 6 !== 0,
            )
          ) {
            unsupported = true;
            return;
          }
          contourWork += target.reduce((sum, span) => sum + span.end - span.start, 0);
          lineSpans.push({ line: object, spans, target });
        }
      });
    if (
      unsupported ||
      indexWork > MAX_INDICES_PER_UPDATE ||
      contourWork > MAX_CONTOUR_FLOATS_PER_UPDATE
    )
      return false;
    for (const { mesh, spans, target } of meshSpans) {
      const index = mesh.geometry.index!;
      // Preserve pending ranges if another retirement occurs before a draw.
      for (const span of target) {
        for (let face = span.first; face < span.end; face++) {
          const offset = face * 3,
            value = index.getX(offset);
          index.setX(offset, value);
          index.setX(offset + 1, value);
          index.setX(offset + 2, value);
        }
        index.addUpdateRange(span.first * 3, (span.end - span.first) * 3);
      }
      index.needsUpdate = true;
      mesh.userData.buildingSpans = spans.filter((span) => span.id !== id);
    }
    for (const { line, spans, target } of lineSpans) {
      const position = line.geometry.getAttribute('position') as THREE.BufferAttribute;
      // Renderer upload clears accumulated update ranges.
      for (const span of target) {
        for (let offset = span.start; offset < span.end; offset += 6) {
          position.setXYZ(
            offset / 3 + 1,
            position.getX(offset / 3),
            position.getY(offset / 3),
            position.getZ(offset / 3),
          );
        }
        position.addUpdateRange(span.start, span.end - span.start);
      }
      position.needsUpdate = true;
      line.userData.buildingContourSpans = spans.filter((span) => span.id !== id);
    }
    for (const { key } of batches) this.committedSources.get(key)?.delete(id);
    return true;
  }
  setBuildingVisible(id: number, visible: boolean) {
    if (!this.keySelector) throw new Error('Visibility gating requires isolated building batches');
    const source = this.sources.get(id);
    if (source) {
      const batch = this.batches.get(source.key);
      if (batch) batch.visible = visible;
      return;
    }
    for (const [key, sources] of this.committedSources)
      if (sources.has(id)) {
        const batch = this.batches.get(key);
        if (batch) batch.visible = visible;
      }
  }
  setContoursVisible(visible: boolean) {
    this.contoursVisible = visible;
    this.root.traverse((o) => {
      if (o instanceof THREE.LineSegments) o.visible = visible;
    });
    if (this.active)
      this.active.next.traverse((o) => {
        if (o instanceof THREE.LineSegments) o.visible = visible;
      });
  }
  private key(id: number, group: THREE.Group) {
    return this.keySelector?.(id, group) ?? buildingChunkKey(group.position.x, group.position.z);
  }
  private markDirty(key: string) {
    if (!this.dirty.has(key)) this.dirtySince.set(key, this.updateSequence);
    this.dirty.add(key);
    this.revisions.set(key, (this.revisions.get(key) ?? 0) + 1);
  }

  set(id: number, group: THREE.Group, sourceOrRelease?: RebuildableBuildingSource | (() => void)) {
    const old = this.sources.get(id),
      key = this.key(id, group);
    if (old) {
      this.markDirty(old.key);
      if (typeof sourceOrRelease === 'function')
        this.queueRetirement(old.key, this.revisions.get(old.key) ?? 0, sourceOrRelease);
      this.retireSource(old);
    }
    const descriptor = typeof sourceOrRelease === 'function' ? null : (sourceOrRelease ?? null);
    this.sources.set(id, {
      id,
      key,
      contours: this.contourMaterial ? null : new Float32Array(0),
      descriptor,
      pendingGroup: descriptor ? group : null,
      legacyGroup: descriptor ? null : group,
      leases: 0,
      retired: false,
      released: false,
    });
    this.markDirty(key);
  }

  private retireSource(source: Source) {
    source.retired = true;
    this.releaseSourceIfPossible(source);
  }
  private releaseSourceIfPossible(source: Source) {
    if (!source.retired || source.leases !== 0 || source.released) return;
    if (source.pendingGroup && source.descriptor)
      source.descriptor.disposeGroup(source.pendingGroup);
    source.pendingGroup = null;
    source.released = true;
  }

  private queueRetirement(key: string, afterRevision: number, release: () => void) {
    const list = this.retired.get(key) ?? [];
    list.push({ afterRevision, release });
    this.retired.set(key, list);
  }

  remove(id: number, release?: () => void) {
    const old = this.sources.get(id);
    if (!old) {
      release?.();
      return;
    }
    this.sources.delete(id);
    this.markDirty(old.key);
    if (release) this.queueRetirement(old.key, this.revisions.get(old.key) ?? 0, release);
    this.retireSource(old);
  }

  /**
   * Advances at most one chunk build. Source transforms and merge copies are
   * quota-stepped with no source-size exception; contours use a separate quota.
   */
  update(pendingKeys?: ReadonlySet<string>) {
    this.updateSequence++;
    if (!this.active) {
      let selected: string | undefined;
      for (const key of this.dirty) {
        if (
          !pendingKeys?.has(key) ||
          this.updateSequence - (this.dirtySince.get(key) ?? 0) >= MAX_DEFERRED_UPDATES
        ) {
          selected = key;
          break;
        }
      }
      if (selected !== undefined) this.beginBuild(selected);
    }
    const build = this.active;
    if (!build) return;
    try {
      this.prepareMeshes(build);
      this.copyContours(build, MAX_CONTOUR_FLOATS_PER_UPDATE);
      if (
        build.sourceCursor >= build.sources.length &&
        build.currentGroup === null &&
        build.transform === null &&
        build.jobs === null
      )
        this.createJobs(build);
      if (build.jobs) {
        this.advanceMergers(build, MAX_VERTICES_PER_UPDATE, MAX_INDICES_PER_UPDATE);
      }
      if (this.ready(build)) {
        const permitted =
          !this.canCommit ||
          this.canCommit(
            new Map(build.sources.map((source) => [source.id, source.descriptor])),
            this.committedSources.get(build.key) ?? new Map(),
          );
        if (permitted) this.commit(build);
        else if (this.revisions.get(build.key) !== build.revision) {
          // A superseded removal must not hold the only build slot while the
          // newer source needed to make that handoff safe waits behind it.
          this.cancelBuild();
          this.markDirty(build.key);
        }
      }
    } catch (error) {
      this.cancelBuild();
      this.markDirty(build.key);
      throw error;
    }
  }

  private beginBuild(key: string) {
    const sources: Source[] = [],
      contourSources: ContourSource[] = [],
      contourSpans: ContourSpan[] = [];
    for (const source of this.sources.values()) {
      if (source.key !== key) continue;
      source.leases++;
      sources.push(source);
    }
    this.active = {
      key,
      revision: this.revisions.get(key) ?? 0,
      sources,
      sourceCursor: 0,
      currentSource: null,
      currentGroup: null,
      groupMeshes: [],
      groupMeshCursor: 0,
      transform: null,
      grouped: new Map(),
      jobs: null,
      jobCursor: 0,
      next: Object.assign(new THREE.Group(), { name: `Neighborhood ${key}` }),
      contourSources,
      contourSpans,
      contourSourceCursor: 0,
      contourOffset: 0,
      contourOutput: new Float32Array(0),
      contourCursor: 0,
      contourMin: new THREE.Vector3(Infinity, Infinity, Infinity),
      contourMax: new THREE.Vector3(-Infinity, -Infinity, -Infinity),
      contoursDone: false,
      contoursInitialized: false,
      contourBuilder: null,
      ownedResources: [],
      ownedResourceSet: new Set(),
    };
    this.dirty.delete(key);
    this.dirtySince.delete(key);
  }

  private prepareMeshes(build: Build) {
    let verticesLeft = MAX_PREPARED_VERTICES_PER_UPDATE,
      indicesLeft = MAX_INDICES_PER_UPDATE,
      started = 0,
      regenerated = 0,
      contourWorkLeft = MAX_CONTOUR_WORK_PER_UPDATE;
    // Use spare preparation time without an unbounded full-building outline pass.
    // The deadline is cooperative: a slice (and allocation/GC) can overrun it.
    const contourDeadline = performance.now() + CONTOUR_TIME_BUDGET_MS;
    while (
      (build.transform !== null ||
        build.currentGroup !== null ||
        build.sourceCursor < build.sources.length) &&
      started < MAX_PREPARED_MESHES_PER_UPDATE
    ) {
      if (!build.currentGroup) {
        const source = build.sources[build.sourceCursor];
        if (!source) break;
        let group: THREE.Group;
        if (source.descriptor) {
          if (source.pendingGroup) {
            group = source.pendingGroup;
            source.pendingGroup = null;
          } else {
            if (regenerated >= 1) break;
            group = source.descriptor.createGroup();
            regenerated++;
          }
        } else group = source.legacyGroup!;
        build.sourceCursor++;
        build.currentSource = source;
        build.currentGroup = group;
        build.groupMeshCursor = 0;
        build.groupMeshes = [];
        group.updateMatrixWorld(true);
        group.traverse((object) => {
          if (object instanceof THREE.Mesh)
            build.groupMeshes.push({
              id: source.id,
              mesh: object,
              matrix: object.matrixWorld.clone(),
              vertices: object.geometry.getAttribute('position').count,
            });
        });
      }
      if (build.currentSource!.contours === null) {
        if (contourWorkLeft <= 0) break;
        build.contourBuilder ??= new IncrementalBuildingContours(
          build.currentGroup!,
          MIN_CONTOUR_EDGE_LENGTH_SQ,
        );
        let complete = false;
        do {
          complete = build.contourBuilder.step(Math.min(CONTOUR_WORK_SLICE, contourWorkLeft));
          contourWorkLeft -= build.contourBuilder.processedWork;
        } while (!complete && contourWorkLeft > 0 && performance.now() < contourDeadline);
        if (!complete) break;
        build.currentSource!.contours = build.contourBuilder.result!;
        build.contourBuilder.dispose();
        build.contourBuilder = null;
      }
      if (build.groupMeshCursor >= build.groupMeshes.length) {
        this.finishCurrentGroup(build);
        continue;
      }
      if (!build.transform) {
        const input = build.groupMeshes[build.groupMeshCursor],
          mesh = input.mesh!;
        started++;
        const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
        if (materials.length !== 1)
          throw new Error('Building surfaces require one material per authored mesh');
        const resolved = this.material(materials[0]),
          batch = resolved instanceof THREE.Material ? { material: resolved } : resolved;
        build.transform = {
          input,
          material: batch.material,
          shadow: mesh.castShadow,
          triangles:
            (mesh.geometry.index?.count ?? mesh.geometry.getAttribute('position').count) / 3,
          transform: new IncrementalGeometryTransform(mesh.geometry, input.matrix, batch.color),
        };
      }
      const job = build.transform,
        transform = job.transform;
      const beforeVertices = transform.processedVertices,
        beforeIndices = transform.processedIndices;
      const complete = transform.step(verticesLeft, indicesLeft);
      verticesLeft -= transform.processedVertices - beforeVertices;
      indicesLeft -= transform.processedIndices - beforeIndices;
      if (!complete) break;
      const part: PreparedPart = {
        geometry: transform.result!,
        id: job.input.id,
        shadow: job.shadow,
        triangles: job.triangles,
      };
      const list = build.grouped.get(job.material) ?? [];
      list.push(part);
      build.grouped.set(job.material, list);
      job.input.mesh = null;
      build.groupMeshCursor++;
      build.transform = null;
      if (verticesLeft === 0 && indicesLeft === 0) break;
    }
  }

  private finishCurrentGroup(build: Build) {
    const source = build.currentSource,
      group = build.currentGroup;
    if (!source || !group) return;
    build.currentSource = null;
    build.currentGroup = null;
    build.groupMeshes = [];
    build.groupMeshCursor = 0;
    // Legacy callers explicitly own their groups. Production descriptors transfer
    // tagged sign resources to the eventual batch before freeing source geometry.
    if (source.descriptor) {
      group.traverse((object) => {
        if (!(object instanceof THREE.Mesh)) return;
        if (object.userData.ownedMaterial) {
          for (const material of Array.isArray(object.material)
            ? object.material
            : [object.material])
            this.retainOwnedResource(build, material);
          delete object.userData.ownedMaterial;
        }
        if (object.userData.ownedTexture) {
          this.retainOwnedResource(build, object.userData.ownedTexture as THREE.Texture);
          delete object.userData.ownedTexture;
        }
      });
      source.descriptor.disposeGroup(group);
    }
  }

  private retainOwnedResource(build: Build, resource: THREE.Material | THREE.Texture) {
    if (build.ownedResourceSet.has(resource)) return;
    build.ownedResourceSet.add(resource);
    build.ownedResources.push(resource);
  }

  private createJobs(build: Build) {
    build.jobs = [];
    for (const [material, parts] of build.grouped)
      build.jobs.push({ material, parts, merger: null });
    build.grouped.clear();
  }

  private advanceMergers(build: Build, vertexQuota: number, indexQuota: number) {
    let verticesLeft = vertexQuota,
      indicesLeft = indexQuota;
    while (build.jobCursor < build.jobs!.length) {
      const job = build.jobs![build.jobCursor];
      if (!job.merger)
        job.merger = new IncrementalGeometryMerge(job.parts.map((part) => part.geometry));
      const merger = job.merger;
      const beforeVertices = merger.processedVertices,
        beforeIndices = merger.processedIndices;
      const complete = merger.step(verticesLeft, indicesLeft);
      verticesLeft -= merger.processedVertices - beforeVertices;
      indicesLeft -= merger.processedIndices - beforeIndices;
      if (!complete) break;
      const geometry = merger.result!;
      const mesh = new THREE.Mesh(geometry, job.material);
      mesh.castShadow = job.parts.some((part) => part.shadow);
      mesh.receiveShadow = true;
      let face = 0;
      mesh.userData.buildingSpans = job.parts.map((part) => {
        const first = face;
        face += part.triangles;
        return { first, end: face, id: part.id };
      });
      build.next.add(mesh);
      for (const part of job.parts) part.geometry.dispose();
      job.parts.length = 0;
      job.merger = null;
      build.jobCursor++;
      if (verticesLeft === 0 && indicesLeft === 0) break;
    }
  }

  private copyContours(build: Build, floatQuota: number) {
    if (build.contoursDone) return;
    if (!build.contoursInitialized) {
      if (build.sourceCursor < build.sources.length || build.currentGroup !== null) return;
      let length = 0;
      for (const source of build.sources) {
        const values = source.contours!;
        if (!values.length) continue;
        build.contourSources.push({ id: source.id, values });
        build.contourSpans.push({ id: source.id, start: length, end: length + values.length });
        length += values.length;
      }
      build.contourOutput = new Float32Array(length);
      build.contoursInitialized = true;
      if (!length) {
        build.contoursDone = true;
        return;
      }
    }
    let remaining = floatQuota;
    while (remaining > 0 && build.contourSourceCursor < build.contourSources.length) {
      const source = build.contourSources[build.contourSourceCursor].values,
        count = Math.min(remaining, source.length - build.contourOffset);
      build.contourOutput.set(
        source.subarray(build.contourOffset, build.contourOffset + count),
        build.contourCursor,
      );
      const vertexStart = build.contourOffset / 3,
        vertexEnd = (build.contourOffset + count) / 3;
      for (let i = vertexStart; i < vertexEnd; i++) {
        const offset = i * 3,
          x = source[offset],
          y = source[offset + 1],
          z = source[offset + 2];
        build.contourMin.x = Math.min(build.contourMin.x, x);
        build.contourMin.y = Math.min(build.contourMin.y, y);
        build.contourMin.z = Math.min(build.contourMin.z, z);
        build.contourMax.x = Math.max(build.contourMax.x, x);
        build.contourMax.y = Math.max(build.contourMax.y, y);
        build.contourMax.z = Math.max(build.contourMax.z, z);
      }
      build.contourOffset += count;
      build.contourCursor += count;
      remaining -= count;
      if (build.contourOffset === source.length) {
        build.contourSourceCursor++;
        build.contourOffset = 0;
      }
    }
    if (build.contourSourceCursor < build.contourSources.length) return;
    build.contoursDone = true;
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(build.contourOutput, 3));
    const center = build.contourMin.clone().add(build.contourMax).multiplyScalar(0.5);
    geometry.boundingBox = new THREE.Box3(build.contourMin.clone(), build.contourMax.clone());
    geometry.boundingSphere = new THREE.Sphere(
      center,
      build.contourMax.clone().sub(build.contourMin).length() * 0.5,
    );
    const lines = new THREE.LineSegments(geometry, this.contourMaterial!);
    lines.visible = this.contoursVisible;
    lines.raycast = () => {};
    lines.userData.buildingContourSpans = build.contourSpans;
    build.next.add(lines);
  }

  private ready(build: Build) {
    return (
      build.sourceCursor >= build.sources.length &&
      build.currentGroup === null &&
      build.transform === null &&
      build.jobs !== null &&
      build.jobCursor >= build.jobs.length &&
      build.contoursDone
    );
  }

  private commit(build: Build) {
    // A style switch can happen while this staged build is detached from the
    // scene. Resolve shared render materials again at the atomic commit, while
    // keeping authored per-vertex colors (and all geometry) untouched.
    build.next.traverse((object) => {
      if (object instanceof THREE.Mesh) {
        const resolved = this.material(object.material);
        object.material = resolved instanceof THREE.Material ? resolved : resolved.material;
      }
    });
    if (build.ownedResources.length)
      build.next.userData.buildingOwnedResources = build.ownedResources;
    const old = this.batches.get(build.key);
    if (old) {
      old.removeFromParent();
      this.retire(() => disposeGroup(old));
    }
    const committed = new Map(build.sources.map((source) => [source.id, source.descriptor]));
    if (committed.size) {
      if (this.keySelector) build.next.visible = old?.visible ?? false;
      this.root.add(build.next);
      this.batches.set(build.key, build.next);
      this.committedSources.set(build.key, committed);
    } else {
      this.batches.delete(build.key);
      this.committedSources.delete(build.key);
      this.retire(() => disposeGroup(build.next));
    }
    const releases = this.retired.get(build.key);
    if (releases) {
      const pending: RetiredSource[] = [];
      for (const retired of releases) {
        if (retired.afterRevision <= build.revision) retired.release();
        else pending.push(retired);
      }
      if (pending.length) this.retired.set(build.key, pending);
      else this.retired.delete(build.key);
    }
    this.releaseBuildSources(build);
    this.active = null;
  }

  private cancelBuild() {
    const build = this.active;
    if (!build) return;
    build.contourBuilder?.dispose();
    build.transform?.transform.dispose();
    if (build.currentGroup && build.currentSource?.descriptor)
      build.currentSource.descriptor.disposeGroup(build.currentGroup);
    for (const job of build.jobs ?? []) {
      if (job.merger && !job.merger.done) job.merger.dispose();
      for (const part of job.parts) part.geometry.dispose();
    }
    for (const parts of build.grouped.values()) for (const part of parts) part.geometry.dispose();
    disposeGroup(build.next);
    disposeOwnedResources(build.ownedResources);
    this.releaseBuildSources(build);
    this.active = null;
  }

  private releaseBuildSources(build: Build) {
    for (const source of build.sources) {
      source.leases = Math.max(0, source.leases - 1);
      this.releaseSourceIfPossible(source);
    }
    build.sources = [];
  }

  buildingAt(hit: THREE.Intersection) {
    if (hit.faceIndex === undefined || hit.faceIndex === null) return undefined;
    const spans = hit.object.userData.buildingSpans as
      Array<{ first: number; end: number; id: number }> | undefined;
    if (!spans) return undefined;
    let low = 0,
      high = spans.length - 1;
    while (low <= high) {
      const middle = (low + high) >>> 1,
        span = spans[middle];
      if (hit.faceIndex < span.first) high = middle - 1;
      else if (hit.faceIndex >= span.end) low = middle + 1;
      else return span.id;
    }
    return undefined;
  }

  clear() {
    this.cancelBuild();
    for (const group of this.batches.values()) {
      group.removeFromParent();
      this.retire(() => disposeGroup(group));
    }
    for (const source of this.sources.values()) this.retireSource(source);
    this.batches.clear();
    this.committedSources.clear();
    this.sources.clear();
    this.dirty.clear();
    this.dirtySince.clear();
    this.revisions.clear();
    this.updateSequence = 0;
    for (const releases of this.retired.values()) for (const { release } of releases) release();
    this.retired.clear();
  }
}

function disposeGroup(group: THREE.Group) {
  group.traverse((object) => {
    if (object instanceof THREE.Mesh || object instanceof THREE.Line) object.geometry.dispose();
  });
  const resources = group.userData.buildingOwnedResources as
    Array<THREE.Material | THREE.Texture> | undefined;
  if (resources) {
    delete group.userData.buildingOwnedResources;
    disposeOwnedResources(resources);
  }
}
function disposeOwnedResources(resources: readonly (THREE.Material | THREE.Texture)[]) {
  for (const resource of resources) resource.dispose();
}
