import * as THREE from 'three/webgpu';
import { BuildingChunks, type RebuildableBuildingSource } from './building-chunks';

type Entry = {
  changing: boolean;
  descriptor: RebuildableBuildingSource | null;
  showingDynamic: boolean;
  migrationPending: boolean;
};

/** Routes ordinary buildings through spatial batches and active work through one keyed batch per building. */
export class ConstructionBatches {
  readonly root = new THREE.Group();
  private entries = new Map<number, Entry>();
  private turn: 'static' | 'dynamic' = 'static';
  private migrationTurn = true;
  private staticChunks: BuildingChunks;
  private dynamicChunks: BuildingChunks;
  constructor(
    material: (
      material: THREE.Material,
    ) => THREE.Material | { material: THREE.Material; color?: THREE.Color },
    retire: (dispose: () => void) => void,
    contourMaterial?: THREE.LineBasicNodeMaterial,
  ) {
    this.staticChunks = new BuildingChunks(
      material,
      retire,
      contourMaterial,
      undefined,
      (sources, previous) => {
        for (const [id, entry] of this.entries) {
          // A detached static snapshot must not resurrect geometry already handed
          // to the isolated pool. Superseded builds are cancelled by BuildingChunks.
          if (
            (entry.changing || entry.migrationPending) &&
            sources.has(id) &&
            sources.get(id) !== entry.descriptor &&
            entry.descriptor &&
            this.dynamicChunks.hasCommittedSource(id, entry.descriptor)
          )
            return false;
          if (
            !previous.has(id) ||
            (!entry.changing && !entry.migrationPending) ||
            !entry.descriptor ||
            sources.get(id) === previous.get(id)
          )
            continue;
          if (!this.dynamicChunks.hasCommittedSource(id, entry.descriptor)) return false;
        }
        return true;
      },
    );
    this.dynamicChunks = new BuildingChunks(
      material,
      retire,
      contourMaterial,
      (id) => `building:${id}`,
      (sources, previous) => {
        for (const [id, entry] of this.entries) {
          if (!previous.has(id) || sources.has(id) || !entry.descriptor) continue;
          if (entry.changing && !this.staticChunks.hasVisibleSource(id)) return false;
          if (!entry.changing && !this.staticChunks.isSourceVisible(id, entry.descriptor))
            return false;
        }
        return true;
      },
    );
    this.root.add(this.staticChunks.root, this.dynamicChunks.root);
  }
  get pendingCount() {
    return (
      this.staticChunks.pendingCount +
      this.dynamicChunks.pendingCount +
      [...this.entries.values()].filter((entry) => entry.migrationPending).length
    );
  }
  setContoursVisible(visible: boolean) {
    this.staticChunks.setContoursVisible(visible);
    this.dynamicChunks.setContoursVisible(visible);
  }
  isSourceVisible(id: number, descriptor: RebuildableBuildingSource) {
    return (
      this.staticChunks.isSourceVisible(id, descriptor) ||
      this.dynamicChunks.isSourceVisible(id, descriptor)
    );
  }
  hasVisibleSource(id: number) {
    return this.staticChunks.hasVisibleSource(id) || this.dynamicChunks.hasVisibleSource(id);
  }
  buildingAt(hit: THREE.Intersection) {
    return this.dynamicChunks.buildingAt(hit) ?? this.staticChunks.buildingAt(hit);
  }
  set(id: number, group: THREE.Group, descriptor: RebuildableBuildingSource, changing = false) {
    const previous = this.entries.get(id);
    const isolated =
      changing ||
      !!previous?.changing ||
      !!previous?.migrationPending ||
      this.dynamicChunks.hasCommittedSource(id);
    const entry: Entry = {
      changing,
      descriptor,
      showingDynamic: previous?.showingDynamic ?? false,
      migrationPending: !changing && isolated,
    };
    this.entries.set(id, entry);
    if (isolated) {
      this.dynamicChunks.set(id, group, descriptor);
      this.staticChunks.remove(id);
    } else {
      this.staticChunks.set(id, group, descriptor);
      this.dynamicChunks.remove(id);
    }
  }
  remove(id: number) {
    this.entries.delete(id);
    this.staticChunks.remove(id);
    this.dynamicChunks.remove(id);
  }
  update(pendingKeys?: ReadonlySet<string>) {
    // Spend this scheduler slot on one bounded committed-source retirement.
    // The old source stays intact if it exceeds the quota; its ordinary static
    // rebuild remains the fallback. Never mask a shared material mesh wholesale.
    for (const [id, entry] of this.entries) {
      if (
        (entry.changing || entry.migrationPending) &&
        entry.descriptor &&
        this.staticChunks.hasVisibleSource(id) &&
        this.dynamicChunks.hasCommittedSource(id, entry.descriptor) &&
        this.staticChunks.tryRetireCommittedSource(id)
      ) {
        this.reconcileVisibility();
        return;
      }
    }
    // A return to static first publishes the final facade in the isolated batch.
    // Regenerate its static source in a scheduler slot, never in addition to a
    // chunk update (which may itself regenerate one source).
    const migration = [...this.entries].find(
      ([id, entry]) =>
        entry.migrationPending &&
        entry.descriptor &&
        this.dynamicChunks.isSourceVisible(id, entry.descriptor),
    );
    if (migration && this.migrationTurn) {
      const [id, entry] = migration;
      this.staticChunks.set(id, entry.descriptor!.createGroup(), entry.descriptor!);
      entry.migrationPending = false;
      this.migrationTurn = false;
      this.reconcileVisibility();
      return;
    }
    this.migrationTurn = true;
    const staticPending = this.staticChunks.pendingCount > 0,
      dynamicPending = this.dynamicChunks.pendingCount > 0;
    if (staticPending && dynamicPending) {
      const selected = this.turn;
      this.turn = selected === 'static' ? 'dynamic' : 'static';
      (selected === 'static' ? this.staticChunks : this.dynamicChunks).update(pendingKeys);
    } else if (staticPending) this.staticChunks.update(pendingKeys);
    else if (dynamicPending) this.dynamicChunks.update(pendingKeys);
    this.reconcileVisibility();
  }
  private reconcileVisibility() {
    for (const [id, entry] of this.entries) {
      const dynamicCommitted = entry.changing && this.dynamicChunks.hasCommittedSource(id);
      const dynamicReady = dynamicCommitted && !this.staticChunks.hasVisibleSource(id);
      const staticReady =
        !entry.changing &&
        !!entry.descriptor &&
        this.staticChunks.isSourceVisible(id, entry.descriptor);
      if (entry.changing) {
        if (dynamicReady) {
          entry.showingDynamic = true;
          this.dynamicChunks.setBuildingVisible(id, true);
        } else if (
          entry.showingDynamic &&
          this.dynamicChunks.hasCommittedSource(id) &&
          !this.staticChunks.hasVisibleSource(id)
        ) {
          this.dynamicChunks.setBuildingVisible(id, true);
        } else this.dynamicChunks.setBuildingVisible(id, false);
      } else if (staticReady) {
        this.dynamicChunks.setBuildingVisible(id, false);
        this.dynamicChunks.remove(id);
        if (!this.dynamicChunks.hasCommittedSource(id)) {
          entry.showingDynamic = false;
          this.entries.delete(id);
        }
      } else {
        // An in-flight dynamic update may commit after the desired state has
        // returned to static. Retain it until the static replacement is visible.
        this.dynamicChunks.setBuildingVisible(id, !this.staticChunks.hasVisibleSource(id));
      }
    }
  }
  clear() {
    this.entries.clear();
    this.staticChunks.clear();
    this.dynamicChunks.clear();
  }
}
