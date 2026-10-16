import * as THREE from 'three/webgpu';
import { Kit, disposeGroup } from './kit';
import { ArtMaterials } from './materials';
import type { World } from '../types';

/** Permanent frontage paving, using the simulation's own walking connections. */
export class BuildingAccessPaths {
  readonly root = new THREE.Group();
  private signature = '';
  private paving = new THREE.Group();

  constructor(
    private art: ArtMaterials,
    private retire: (release: () => void) => void,
  ) {
    this.root.add(this.paving);
  }

  setWorld(world: World) {
    const buildings = new Map(world.buildings.map((b) => [b.id, b]));
    const paths = (world.building_access ?? []).filter((path) => {
      const b = buildings.get(path.building_id);
      return b && ['open', 'renovation', 'vacant'].includes(b.stage);
    });
    const roads = new Map(world.roads.map((road) => [road.id, road]));
    // Shared streets have no raised sidewalk. Keep their landings just above
    // the 0.3m carriageway, rather than introducing a 17.5cm step into it.
    const landingCenter = (roadId: number) => {
      const kind = roads.get(roadId)?.kind;
      return kind === 'local' || kind === 'lane' ? 0.255 : 0.425;
    };
    const signature = JSON.stringify(paths.map((path) => [path, landingCenter(path.road_id)]));
    if (signature === this.signature) return;
    this.signature = signature;
    const previous = this.paving;
    previous.removeFromParent();
    this.retire(() => disposeGroup(previous));
    const kit = new Kit();
    for (const path of paths) {
      if (path.points.some((p) => !Number.isFinite(p.x) || !Number.isFinite(p.z))) continue;
      const points = path.points.filter(
        (p, i) =>
          i === 0 || Math.hypot(p.x - path.points[i - 1].x, p.z - path.points[i - 1].z) >= 0.05,
      );
      if (points.length < 2) continue;
      const curb = points[points.length - 1];
      let angle = 0;
      for (let i = 1; i < points.length; i++) {
        const start = points[i - 1],
          end = points[i];
        const dx = end.x - start.x,
          dz = end.z - start.z;
        const length = Math.hypot(dx, dz);
        // Meet the parcel's paving and the raised sidewalk without a floating
        // strip. Slight overlap at both ends avoids seams at oblique viewpoints.
        // Interior passage legs stay at ground level; only the final leg ramps
        // to the curb. Never replace bends with a shortcut through a front lot.
        const rise = i === points.length - 1 ? landingCenter(path.road_id) : 0;
        angle = Math.atan2(dx, dz);
        const geometry = new THREE.BoxGeometry(1.65, 0.09, Math.hypot(length, rise) + 0.18);
        geometry.rotateX(-Math.atan2(rise, length));
        geometry.rotateY(angle);
        geometry.translate((start.x + end.x) / 2, 0.005 + rise / 2, (start.z + end.z) / 2);
        kit.add(geometry, this.art.get('#b5b3a2'));
      }
      // A small level landing joins the street edge even beside a junction,
      // where the long sidewalk strips stop short of the corner.
      kit.box(
        curb.x,
        landingCenter(path.road_id),
        curb.z,
        1.85,
        0.1,
        1.8,
        this.art.get('#b5b3a2'),
        angle,
      );
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
