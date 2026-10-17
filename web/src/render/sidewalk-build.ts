import * as THREE from 'three/webgpu';
import type { World } from '../types';
import { arterialSidewalkSteps } from './arterial-sidewalks';
import { sidewalkGeometry } from './sidewalk-kit';
import { IncrementalGeometryMerge } from './incremental-geometry-merge';

/** Detached, frame-budgeted sidewalk construction. Owns every intermediate and final geometry. */
export class SidewalkBuild {
  private iterator: Generator<void, { x: number; z: number }[][], void> | null;
  private polygons: { x: number; z: number }[][] | null = null;
  private polygonIndex = 0;
  private fragments: THREE.BufferGeometry[] = [];
  private merger: IncrementalGeometryMerge | null = null;
  private output: THREE.BufferGeometry | null = null;
  private completed = false;
  private disposed = false;
  private transferred = false;

  constructor(world: Pick<World, 'nodes' | 'roads'>) {
    this.iterator = arterialSidewalkSteps(world);
  }

  /** Advances at most 2048 clipping yields, 16 extrusions, or one bounded merge step. */
  step(): boolean {
    if (this.disposed || this.completed) return this.completed;
    if (this.iterator) {
      for (let i = 0; i < 2048; i++) {
        const next = this.iterator.next();
        if (next.done) {
          this.iterator = null;
          this.polygons = next.value;
          if (!this.polygons.length) this.finish(null);
          break;
        }
      }
      return this.completed;
    }
    if (this.polygons) {
      const end = Math.min(this.polygonIndex + 16, this.polygons.length);
      while (this.polygonIndex < end)
        this.fragments.push(sidewalkGeometry(this.polygons[this.polygonIndex++]));
      if (this.polygonIndex < this.polygons.length) return false;
      this.polygons = null;
      return false;
    }
    if (!this.merger) {
      if (!this.fragments.length) {
        this.finish(null);
        return true;
      }
      this.merger = new IncrementalGeometryMerge(this.fragments);
    }
    const done = this.merger.step(12000, 36000);
    if (done) {
      this.output = this.merger.result;
      this.merger = null;
      this.disposeFragments();
      this.finish(this.output);
    }
    return this.completed;
  }

  /** Transfers the completed geometry to the caller exactly once. */
  takeGeometry(): THREE.BufferGeometry | null {
    if (!this.completed || this.disposed || this.transferred) return null;
    this.transferred = true;
    const result = this.output;
    this.output = null;
    return result;
  }

  /** Cancels pending work and disposes all geometry still owned by this job. */
  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.iterator?.return(undefined as never);
    this.iterator = null;
    this.merger?.dispose();
    this.merger = null;
    this.disposeFragments();
    this.output?.dispose();
    this.output = null;
    this.polygons = null;
    this.completed = false;
  }

  private finish(geometry: THREE.BufferGeometry | null): void {
    this.output = geometry;
    this.completed = true;
    this.polygons = null;
  }

  private disposeFragments(): void {
    for (const fragment of this.fragments) fragment.dispose();
    this.fragments = [];
  }
}
