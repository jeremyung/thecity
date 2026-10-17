import type * as THREE from 'three/webgpu';
import type { LensPresetId } from '../optics/lens';
import type { World } from '../types';

/**
 * What the renderer needs from whatever moves the camera. The app's composition
 * root supplies the implementation (the camera director), so render/ never
 * depends on camera/.
 */
export interface CameraRig {
  readonly lens: LensPresetId;
  /** Orbit target: the point the viewer is looking at. */
  readonly target: THREE.Vector3;
  /** Stable subject for depth of field. */
  readonly focusTarget: THREE.Vector3;
  setLens(id: LensPresetId, takeManualControl?: boolean): void;
  world(world: World): void;
  update(dt: number, time: number): void;
  /** Presented actor rows (`MOVEMENT_STRIDE` floats each) and ids withheld from view. */
  setPositions(data: Float32Array, withheld?: ReadonlySet<number>): void;
  dispose(): void;
}

export type CameraRigFactory = (camera: THREE.PerspectiveCamera, element: HTMLElement) => CameraRig;
