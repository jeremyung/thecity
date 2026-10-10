import * as THREE from 'three/webgpu';

type AttributeSchema = {
  readonly keys: string[];
  readonly attributes: Record<string, THREE.BufferAttribute>;
  readonly positionCount: number;
  readonly indexed: boolean;
  readonly indexCount: number;
};

/**
 * Incrementally concatenates compatible, ordinary BufferGeometries.
 *
 * The result matches `mergeGeometries(parts, false)` for vertex attributes,
 * indexed/unindexed topology and triangle order. Inputs are borrowed: they are
 * never modified or disposed. Morph targets and interleaved attributes are
 * rejected up front because this utility is for static authored surfaces.
 *
 * Output typed arrays are allocated up front (the unavoidable contiguous
 * destination allocation); `step` bounds the actual vertex and index copying.
 * Bounds are accumulated as positions are copied. The sphere is the
 * conservative sphere of the resulting AABB, avoiding a final vertex scan.
 */
export class IncrementalGeometryMerge {
  private parts: readonly THREE.BufferGeometry[];
  private schemas: AttributeSchema[];
  private output: THREE.BufferGeometry | null;
  private readonly vertexTotal: number;
  private vertexOffsets: number[];
  private readonly indexTotal: number;
  private indexed: boolean;
  private keys: string[];
  private vertexPart = 0;
  private vertexInPart = 0;
  private indexPart = 0;
  private indexInPart = 0;
  private completed = false;
  private cancelled = false;
  private minX = Infinity;
  private minY = Infinity;
  private minZ = Infinity;
  private maxX = -Infinity;
  private maxY = -Infinity;
  private maxZ = -Infinity;
  private verticesCopied = 0;
  private indicesCopied = 0;

  constructor(parts: readonly THREE.BufferGeometry[]) {
    if (parts.length === 0) throw new RangeError('At least one geometry is required');
    this.parts = parts.slice();
    this.schemas = parts.map((geometry, partIndex) => validateGeometry(geometry, partIndex));
    this.indexed = this.schemas[0].indexed;
    this.keys = this.schemas[0].keys;

    let vertexTotal = 0;
    let indexTotal = 0;
    this.vertexOffsets = [];
    for (let i = 0; i < this.schemas.length; i++) {
      const schema = this.schemas[i];
      if (schema.indexed !== this.indexed) throw new TypeError(`Geometry ${i} has an incompatible indexing mode`);
      if (!sameKeys(this.keys, schema.keys)) throw new TypeError(`Geometry ${i} has incompatible attributes`);
      for (const key of this.keys) {
        const expected = this.schemas[0].attributes[key];
        const actual = schema.attributes[key];
        if (actual.array.constructor !== expected.array.constructor || actual.itemSize !== expected.itemSize ||
          actual.normalized !== expected.normalized || actual.gpuType !== expected.gpuType) {
          throw new TypeError(`Geometry ${i} attribute ${key} has an incompatible layout`);
        }
      }
      this.vertexOffsets.push(vertexTotal);
      vertexTotal += schema.positionCount;
      indexTotal += schema.indexCount;
    }
    if (!Number.isSafeInteger(vertexTotal) || !Number.isSafeInteger(indexTotal)) {
      throw new RangeError('Merged geometry is too large');
    }
    this.vertexTotal = vertexTotal;
    this.indexTotal = indexTotal;

    // All schema checks finish before any destination geometry or arrays exist.
    this.output = new THREE.BufferGeometry();
    for (const key of this.keys) {
      const first = this.schemas[0].attributes[key];
      const ArrayType = first.array.constructor as { new(length: number): THREE.TypedArray };
      const array = new ArrayType(vertexTotal * first.itemSize);
      const merged = new THREE.BufferAttribute(array, first.itemSize, first.normalized);
      merged.gpuType = first.gpuType;
      merged.name = first.name;
      this.output.setAttribute(key, merged);
    }
    if (this.indexed) {
      // Choose conservatively from the complete vertex domain so large offsets
      // are never truncated; Three reserves 0xffff for primitive restart.
      const IndexType = vertexTotal > 65535 ? Uint32Array : Uint16Array;
      this.output.setIndex(new THREE.BufferAttribute(new IndexType(indexTotal), 1));
    }
  }

  get done(): boolean { return this.completed; }
  get result(): THREE.BufferGeometry | null { return this.completed && !this.cancelled ? this.output : null; }
  get processedVertices(): number { return this.verticesCopied; }
  get processedIndices(): number { return this.indicesCopied; }

  /** Copy no more than each supplied quota. Either quota may be zero. */
  step(maxVertices: number, maxIndices: number): boolean {
    if (this.cancelled) throw new Error('Geometry merge was disposed before completion');
    validateQuota(maxVertices, 'maxVertices');
    validateQuota(maxIndices, 'maxIndices');
    if (this.completed) return true;

    let vertexBudget = maxVertices;
    while (vertexBudget > 0 && this.vertexPart < this.parts.length) {
      const schema = this.schemas[this.vertexPart];
      const remaining = schema.positionCount - this.vertexInPart;
      if (remaining <= 0) {
        this.vertexPart++;
        this.vertexInPart = 0;
        continue;
      }
      const count = Math.min(remaining, vertexBudget);
      const globalStart = this.vertexOffset(this.vertexPart) + this.vertexInPart;
      for (const key of this.keys) {
        const source = schema.attributes[key];
        const target = this.output!.getAttribute(key) as THREE.BufferAttribute;
        const tupleSize = source.itemSize;
        target.array.set(
          source.array.subarray(this.vertexInPart * tupleSize, (this.vertexInPart + count) * tupleSize),
          globalStart * tupleSize,
        );
      }
      this.accumulateBounds(schema.attributes.position, this.vertexInPart, count);
      this.vertexInPart += count;
      this.verticesCopied += count;
      vertexBudget -= count;
    }

    let indexBudget = maxIndices;
    while (indexBudget > 0 && this.indexPart < this.parts.length) {
      const schema = this.schemas[this.indexPart];
      const remaining = schema.indexCount - this.indexInPart;
      if (remaining <= 0) {
        this.indexPart++;
        this.indexInPart = 0;
        continue;
      }
      const count = Math.min(remaining, indexBudget);
      const source = this.parts[this.indexPart].index!;
      const target = this.output!.index!;
      const vertexOffset = this.vertexOffset(this.indexPart);
      for (let i = 0; i < count; i++) {
        const sourceIndex = source.getX(this.indexInPart + i);
        if (!Number.isInteger(sourceIndex) || sourceIndex < 0 || sourceIndex >= schema.positionCount) {
          throw new RangeError(`Geometry ${this.indexPart} has an out-of-range index`);
        }
        target.array[this.indicesCopied + i] = sourceIndex + vertexOffset;
      }
      this.indexInPart += count;
      this.indicesCopied += count;
      indexBudget -= count;
    }

    if (this.verticesCopied === this.vertexTotal && this.indicesCopied === this.indexTotal) {
      this.finishBounds();
      this.completed = true;
      this.releaseSources();
    }
    return this.completed;
  }

  /** Dispose only this unfinished/finished destination; borrowed sources remain owned by callers. */
  dispose(): void {
    if (this.cancelled) return;
    this.cancelled = true;
    this.output?.dispose();
    this.output = null;
    this.releaseSources();
  }

  private releaseSources(): void {
    this.parts = [];
    this.schemas = [];
    this.vertexOffsets = [];
    this.keys = [];
  }

  private vertexOffset(partIndex: number): number {
    return this.vertexOffsets[partIndex];
  }

  private accumulateBounds(position: THREE.BufferAttribute, start: number, count: number): void {
    for (let i = start; i < start + count; i++) {
      const x = position.getX(i), y = position.getY(i), z = position.getZ(i);
      if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) {
        throw new TypeError('Position data must be finite');
      }
      this.minX = Math.min(this.minX, x); this.minY = Math.min(this.minY, y); this.minZ = Math.min(this.minZ, z);
      this.maxX = Math.max(this.maxX, x); this.maxY = Math.max(this.maxY, y); this.maxZ = Math.max(this.maxZ, z);
    }
  }

  private finishBounds(): void {
    if (this.vertexTotal === 0) return;
    const min = new THREE.Vector3(this.minX, this.minY, this.minZ);
    const max = new THREE.Vector3(this.maxX, this.maxY, this.maxZ);
    const box = new THREE.Box3(min, max);
    this.output!.boundingBox = box;
    const center = box.getCenter(new THREE.Vector3());
    this.output!.boundingSphere = new THREE.Sphere(center, box.getSize(new THREE.Vector3()).length() * 0.5);
  }
}

function validateGeometry(geometry: THREE.BufferGeometry, partIndex: number): AttributeSchema {
  if (!geometry || !geometry.isBufferGeometry) throw new TypeError(`Geometry ${partIndex} is not a BufferGeometry`);
  if (Object.keys(geometry.morphAttributes).length || geometry.morphTargetsRelative) {
    throw new TypeError(`Geometry ${partIndex} has unsupported morph attributes`);
  }
  const keys = Object.keys(geometry.attributes).sort();
  if (!keys.includes('position')) throw new TypeError(`Geometry ${partIndex} has no position attribute`);
  const attributes: Record<string, THREE.BufferAttribute> = {};
  let positionCount = -1;
  for (const key of keys) {
    const attribute = geometry.getAttribute(key) as THREE.BufferAttribute;
    if (!attribute?.isBufferAttribute || (attribute as THREE.BufferAttribute & { isInterleavedBufferAttribute?: boolean }).isInterleavedBufferAttribute) {
      throw new TypeError(`Geometry ${partIndex} attribute ${key} must be a non-interleaved BufferAttribute`);
    }
    if (attribute.itemSize <= 0 || attribute.array.length !== attribute.count * attribute.itemSize) {
      throw new TypeError(`Geometry ${partIndex} attribute ${key} has an invalid layout`);
    }
    if (positionCount < 0) positionCount = attribute.count;
    if (attribute.count !== positionCount) throw new TypeError(`Geometry ${partIndex} attributes have different vertex counts`);
    if (key === 'position' && attribute.itemSize < 3) throw new TypeError(`Geometry ${partIndex} position needs three components`);
    attributes[key] = attribute;
  }
  const index = geometry.index;
  if (index && (!index.isBufferAttribute || index.itemSize !== 1 || index.count % 3 !== 0)) {
    throw new TypeError(`Geometry ${partIndex} has an invalid triangle index`);
  }
  if (!index && positionCount % 3 !== 0) {
    throw new TypeError(`Geometry ${partIndex} has an incomplete unindexed triangle`);
  }
  return { keys, attributes, positionCount: Math.max(0, positionCount), indexed: index !== null, indexCount: index?.count ?? 0 };
}

function sameKeys(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((key, i) => key === b[i]);
}

function validateQuota(value: number, name: string): void {
  if (!Number.isSafeInteger(value) || value < 0) throw new RangeError(`${name} must be a nonnegative safe integer`);
}
