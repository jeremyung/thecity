import * as THREE from 'three/webgpu';
import { Kit, disposeGroup } from './kit';
import { ArtMaterials } from './materials';
import type { World } from '../types';

/** Permanent frontage paving, using the simulation's own walking connections. */
export class BuildingAccessPaths {
  readonly root = new THREE.Group();
  private signature = '';
  private paving = new THREE.Group();

  constructor(private art: ArtMaterials, private retire: (release: () => void) => void) {
    this.root.add(this.paving);
  }

  setWorld(world: World) {
    const buildings = new Map(world.buildings.map(b => [b.id, b]));
    const paths = (world.building_access ?? []).filter(path => {
      const b = buildings.get(path.building_id);
      return b && ['open', 'renovation', 'vacant'].includes(b.stage);
    });
    const signature = JSON.stringify(paths);
    if (signature === this.signature) return;
    this.signature = signature;
    const previous = this.paving;
    previous.removeFromParent();
    this.retire(() => disposeGroup(previous));
    const kit = new Kit();
    for (const path of paths) {
      const [door, curb] = path.points;
      const dx = curb.x - door.x, dz = curb.z - door.z;
      const length = Math.hypot(dx, dz);
      if (!Number.isFinite(length) || length < .05) continue;
      // Meet the parcel's paving and the raised sidewalk without a floating
      // strip. Slight overlap at both ends avoids seams at oblique viewpoints.
      const rise = .425, angle = Math.atan2(dx, dz);
      const geometry = new THREE.BoxGeometry(1.65, .09, Math.hypot(length, rise) + .18);
      geometry.rotateX(-Math.atan2(rise, length));
      geometry.rotateY(angle);
      geometry.translate((door.x + curb.x) / 2, .2175, (door.z + curb.z) / 2);
      kit.add(geometry, this.art.get('#b5b3a2'));
      // A small level landing joins the street edge even beside a junction,
      // where the long sidewalk strips stop short of the corner.
      kit.box(curb.x, .425, curb.z, 1.85, .1, 1.8, this.art.get('#b5b3a2'), angle);
    }
    this.paving = kit.finish(false);
    this.root.add(this.paving);
  }

  reset() {
    const previous = this.paving;
    previous.removeFromParent();
    this.retire(() => disposeGroup(previous));
    this.paving = new THREE.Group();
    this.root.add(this.paving);
    this.signature = '';
  }
}
