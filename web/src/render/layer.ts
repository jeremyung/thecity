import type * as THREE from 'three/webgpu';
import type { ArtMaterials } from './materials';
import type { Building, World } from '../types';

/** Renderer services a layer may use. */
export interface LayerHost {
  readonly art: ArtMaterials;
  /** Defer releasing GPU resources until the frame that stops drawing them. */
  retire(release: () => void): void;
  /** True once a building's committed geometry is on screen. */
  isBuildingVisible(building: Building): boolean;
}

/**
 * One visual feature. CityRenderer adds `root` to the scene and calls the
 * hooks in registration order (see render/layers.ts). Adding a visual means a
 * new layer file plus one registry line, not an edit to city.ts.
 */
export interface RenderLayer {
  readonly name: string;
  readonly root?: THREE.Object3D;
  /** A new structural snapshot arrived. */
  setWorld?(world: World): void;
  /** Once per rendered frame. */
  frame?(world: World | null): void;
  /** The world was replaced: drop everything derived from the old one. */
  reset?(): void;
  dispose?(): void;
}

export type LayerFactory = (host: LayerHost) => RenderLayer;
