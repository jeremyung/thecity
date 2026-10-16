import * as THREE from 'three/webgpu';

/**
 * Applies a static transform to borrowed geometry over bounded vertex/index
 * quotas. The output is indexed (unindexed triangles receive an identity
 * index), which lets downstream chunk merging use one topology consistently.
 *
 * Destination arrays are allocated up front, but no source vertex or index
 * values are read/copied until `step`. Each call touches at most its two
 * quotas, apart from per-attribute constant work and final O(1) bounds setup.
 * Inputs remain owned by the caller and are never modified or disposed.
 */
export class IncrementalGeometryTransform {
  private source: THREE.BufferGeometry | null;
  private matrix: THREE.Matrix4 | null;
  private normalMatrix: THREE.Matrix3 | null;
  private output: THREE.BufferGeometry | null;
  private readonly count: number;
  private readonly indexCount: number;
  private readonly indexed: boolean;
  private readonly attributeNames: string[];
  private vertexCursor = 0;
  private indexCursor = 0;
  private completed = false;
  private cancelled = false;
  private minX = Infinity;
  private minY = Infinity;
  private minZ = Infinity;
  private maxX = -Infinity;
  private maxY = -Infinity;
  private maxZ = -Infinity;
  private readonly point = new THREE.Vector3();

  constructor(source: THREE.BufferGeometry, matrix: THREE.Matrix4, linearColor?: THREE.Color) {
    const schema = validateGeometry(source);
    this.source = source;
    this.matrix = matrix.clone();
    this.normalMatrix = new THREE.Matrix3().getNormalMatrix(this.matrix);
    this.count = schema.count;
    this.indexed = schema.indexed;
    this.attributeNames = schema.keys.filter((name) => !(linearColor && name === 'color'));
    this.indexCount = schema.indexed ? schema.indexCount : schema.count;
    this.colorValues = linearColor ? [linearColor.r, linearColor.g, linearColor.b] : null;
    if (this.colorValues?.some((value) => !Number.isFinite(value)))
      throw new TypeError('Linear color must be finite');

    const output = new THREE.BufferGeometry();
    this.output = output;
    output.name = source.name;
    output.drawRange = { ...source.drawRange };
    output.groups = source.groups.map((group) => ({ ...group }));
    output.userData = { ...source.userData };

    for (const name of schema.keys) {
      if (linearColor && name === 'color') continue;
      const attribute = schema.attributes[name];
      const ArrayType = attribute.array.constructor as TypedArrayConstructor;
      const copy = new ArrayType(attribute.array.length);
      const target = new THREE.BufferAttribute(copy, attribute.itemSize, attribute.normalized);
      target.name = attribute.name;
      target.usage = attribute.usage;
      target.gpuType = attribute.gpuType;
      output.setAttribute(name, target);
    }

    if (linearColor) {
      const colorData = new Float32Array(schema.count * 3);
      output.setAttribute('color', new THREE.BufferAttribute(colorData, 3));
    }

    if (schema.indexed) {
      const sourceIndex = source.index!;
      const IndexType = sourceIndex.array.constructor as TypedArrayConstructor;
      output.setIndex(new THREE.BufferAttribute(new IndexType(sourceIndex.count), 1));
    } else {
      // Keep 0xffff out of Uint16 indices (Three treats it as primitive restart).
      const IndexType = schema.count > 65534 ? Uint32Array : Uint16Array;
      output.setIndex(new THREE.BufferAttribute(new IndexType(this.indexCount), 1));
    }
  }

  get done(): boolean {
    return this.completed;
  }
  get result(): THREE.BufferGeometry | null {
    return this.completed && !this.cancelled ? this.output : null;
  }
  get processedVertices(): number {
    return this.vertexCursor;
  }
  get processedIndices(): number {
    return this.indexCursor;
  }

  /** Read and transform no more than each supplied quota. */
  step(maxVertices: number, maxIndices: number): boolean {
    if (this.cancelled) throw new Error('Geometry transform was disposed before completion');
    validateQuota(maxVertices, 'maxVertices');
    validateQuota(maxIndices, 'maxIndices');
    if (this.completed) return true;

    const source = this.source!;
    const output = this.output!;
    const matrix = this.matrix!;
    const normalMatrix = this.normalMatrix!;
    const endVertex = Math.min(this.count, this.vertexCursor + maxVertices);
    for (let i = this.vertexCursor; i < endVertex; i++) {
      for (const name of this.attributeNames) {
        const input = source.getAttribute(name) as THREE.BufferAttribute;
        const target = output.getAttribute(name) as THREE.BufferAttribute;
        copyTuple(input, target, i);
      }

      const position = source.getAttribute('position') as THREE.BufferAttribute;
      const targetPosition = output.getAttribute('position') as THREE.BufferAttribute;
      this.point.fromBufferAttribute(position, i).applyMatrix4(matrix);
      assertFiniteVector(this.point, 'Transformed positions must be finite');
      targetPosition.setXYZ(i, this.point.x, this.point.y, this.point.z);
      // BufferGeometry.computeBoundingBox reads the stored attribute values;
      // include Float32/normalized quantization exactly the same way.
      const px = targetPosition.getX(i),
        py = targetPosition.getY(i),
        pz = targetPosition.getZ(i);
      this.minX = Math.min(this.minX, px);
      this.minY = Math.min(this.minY, py);
      this.minZ = Math.min(this.minZ, pz);
      this.maxX = Math.max(this.maxX, px);
      this.maxY = Math.max(this.maxY, py);
      this.maxZ = Math.max(this.maxZ, pz);

      const normal = source.getAttribute('normal') as THREE.BufferAttribute | undefined;
      if (normal) {
        this.point.fromBufferAttribute(normal, i).applyNormalMatrix(normalMatrix);
        assertFiniteVector(this.point, 'Transformed normals must be finite');
        (output.getAttribute('normal') as THREE.BufferAttribute).setXYZ(
          i,
          this.point.x,
          this.point.y,
          this.point.z,
        );
      }

      const tangent = source.getAttribute('tangent') as THREE.BufferAttribute | undefined;
      if (tangent) {
        this.point.fromBufferAttribute(tangent, i).transformDirection(matrix);
        assertFiniteVector(this.point, 'Transformed tangents must be finite');
        (output.getAttribute('tangent') as THREE.BufferAttribute).setXYZ(
          i,
          this.point.x,
          this.point.y,
          this.point.z,
        );
      }

      if (this.colorValues) {
        const color = output.getAttribute('color') as THREE.BufferAttribute;
        const rgb = this.colorValues!;
        color.setXYZ(i, rgb[0], rgb[1], rgb[2]);
      }
    }
    this.vertexCursor = endVertex;

    const endIndex = Math.min(this.indexCount, this.indexCursor + maxIndices);
    if (this.indexed) {
      const sourceIndex = source.index!;
      const targetIndex = output.index!;
      for (let i = this.indexCursor; i < endIndex; i++) {
        const value = sourceIndex.getX(i);
        if (!Number.isInteger(value) || value < 0 || value >= this.count) {
          throw new RangeError('Geometry has an out-of-range index');
        }
        targetIndex.array[i] = value;
      }
    } else {
      const targetIndex = output.index!;
      for (let i = this.indexCursor; i < endIndex; i++) targetIndex.array[i] = i;
    }
    this.indexCursor = endIndex;

    if (this.vertexCursor === this.count && this.indexCursor === this.indexCount) {
      this.finishBounds();
      this.completed = true;
      this.releaseSources();
    }
    return this.completed;
  }

  dispose(): void {
    if (this.cancelled) return;
    this.cancelled = true;
    this.output?.dispose();
    this.output = null;
    this.releaseSources();
  }

  private colorValues: [number, number, number] | null = null;

  private finishBounds(): void {
    if (this.count === 0) return;
    const box = new THREE.Box3(
      new THREE.Vector3(this.minX, this.minY, this.minZ),
      new THREE.Vector3(this.maxX, this.maxY, this.maxZ),
    );
    this.output!.boundingBox = box;
    this.output!.boundingSphere = new THREE.Sphere(
      box.getCenter(new THREE.Vector3()),
      box.getSize(new THREE.Vector3()).length() * 0.5,
    );
  }

  private releaseSources(): void {
    this.source = null;
    this.matrix = null;
    this.normalMatrix = null;
    this.attributeNames.length = 0;
    this.colorValues = null;
  }
}

type TypedArrayConstructor = { new (length: number): THREE.TypedArray };

function validateGeometry(geometry: THREE.BufferGeometry): {
  keys: string[];
  attributes: Record<string, THREE.BufferAttribute>;
  count: number;
  indexed: boolean;
  indexCount: number;
} {
  if (!geometry?.isBufferGeometry) throw new TypeError('Geometry must be a BufferGeometry');
  if (Object.keys(geometry.morphAttributes).length || geometry.morphTargetsRelative)
    throw new TypeError('Morph attributes are unsupported');
  const keys = Object.keys(geometry.attributes).sort();
  if (!keys.includes('position')) throw new TypeError('Geometry has no position attribute');
  const attributes: Record<string, THREE.BufferAttribute> = {};
  let count = -1;
  for (const name of keys) {
    const attribute = geometry.getAttribute(name) as THREE.BufferAttribute & {
      isInterleavedBufferAttribute?: boolean;
    };
    if (!attribute?.isBufferAttribute || attribute.isInterleavedBufferAttribute)
      throw new TypeError(`Attribute ${name} must be a non-interleaved BufferAttribute`);
    if (attribute.itemSize < 1 || attribute.array.length !== attribute.count * attribute.itemSize)
      throw new TypeError(`Attribute ${name} has an invalid layout`);
    if (count < 0) count = attribute.count;
    if (attribute.count !== count) throw new TypeError('Attributes have different vertex counts');
    if ((name === 'position' || name === 'normal' || name === 'tangent') && attribute.itemSize < 3)
      throw new TypeError(`${name} needs at least three components`);
    attributes[name] = attribute;
  }
  const index = geometry.index;
  if (index && (!index.isBufferAttribute || index.itemSize !== 1 || index.count % 3 !== 0))
    throw new TypeError('Geometry has an invalid triangle index');
  if (!index && count % 3 !== 0)
    throw new TypeError('Geometry has an incomplete unindexed triangle');
  return {
    keys,
    attributes,
    count: Math.max(0, count),
    indexed: index !== null,
    indexCount: index?.count ?? 0,
  };
}

function copyTuple(
  source: THREE.BufferAttribute,
  target: THREE.BufferAttribute,
  index: number,
): void {
  const from = index * source.itemSize;
  const to = index * target.itemSize;
  for (let component = 0; component < source.itemSize; component++)
    target.array[to + component] = source.array[from + component];
}

function validateQuota(value: number, name: string): void {
  if (!Number.isSafeInteger(value) || value < 0)
    throw new RangeError(`${name} must be a nonnegative safe integer`);
}

function assertFiniteVector(value: THREE.Vector3, message: string): void {
  if (!Number.isFinite(value.x) || !Number.isFinite(value.y) || !Number.isFinite(value.z))
    throw new TypeError(message);
}
