import * as THREE from 'three/webgpu';

/** Keep the GPU instance-buffer identity when a rebuilt static kit still fits.
 * Geometry ownership remains with the caller; only instance values are copied.
 */
export function reuseInstances(
  previous: THREE.InstancedMesh | undefined,
  next: THREE.InstancedMesh,
): THREE.InstancedMesh {
  if (
    !previous ||
    previous.material !== next.material ||
    previous.instanceMatrix.count < next.count ||
    Boolean(previous.instanceColor) !== Boolean(next.instanceColor) ||
    previous.morphTexture ||
    next.morphTexture
  )
    return next;
  previous.count = next.count;
  previous.instanceMatrix.array.set(next.instanceMatrix.array.subarray(0, next.count * 16));
  previous.instanceMatrix.needsUpdate = true;
  if (previous.instanceColor && next.instanceColor) {
    previous.instanceColor.array.set(next.instanceColor.array.subarray(0, next.count * 3));
    previous.instanceColor.needsUpdate = true;
  }
  previous.boundingBox = null;
  previous.boundingSphere = null;
  next.dispose();
  return previous;
}

/** Geometric growth avoids rebuilding instance shader bindings for each new tree/pole. */
export function instanceCapacity(count: number): number {
  return 2 ** Math.ceil(Math.log2(Math.max(1, count)));
}
