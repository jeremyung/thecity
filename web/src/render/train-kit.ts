import * as THREE from 'three/webgpu';
import { mergeVertices } from 'three/addons/utils/BufferGeometryUtils.js';
import { Kit } from './kit';
import { COACH_LENGTH, COACH_WIDTH, RAIL_CENTER_OFFSET, PANTOGRAPH_CONTACT_HEIGHT } from './rail-profile';

type TrainArt = { get(color: string, roughness?: number, emissive?: boolean): THREE.Material };

/** A compact stainless commuter coach, authored around railhead y=0 and +z forward. */
export function buildCommuterCoach(art: TrainArt, role: 'lead' | 'middle' | 'tail', cabLights = true, doors = true): THREE.Group {
  const kit = new Kit();
  const silver = art.get('#c5c9c8', .38), roofMat = art.get('#8f9998', .72);
  const dark = art.get('#202d31', .28), glass = art.get('#243c45', .18);
  const green = art.get('#168b61', .42), orange = art.get('#e8752b', .44);
  const iron = art.get('#343d3f', .75), wheel = art.get('#252b2d', .7);
  const warm = art.get('#ffcf76', .28, true), red = art.get('#e5352f', .3, true);
  const addGeometry = (geometry: THREE.BufferGeometry, material: THREE.Material) => {
    const compatible = geometry.index ? geometry : mergeVertices(geometry);
    if (compatible !== geometry) geometry.dispose();
    kit.add(compatible, material);
  };

  // Continuous rounded aluminum shell. The cross section is deliberately arched,
  // with shoulders and a flatter crown instead of a rectangular extrusion.
  const shell = new THREE.Shape();
  shell.moveTo(-COACH_WIDTH / 2, 2.92);
  shell.lineTo(COACH_WIDTH / 2, 2.92);
  shell.lineTo(COACH_WIDTH / 2, 3.05);
  shell.quadraticCurveTo(COACH_WIDTH / 2, 3.42, 1.20, 3.62);
  shell.quadraticCurveTo(.82, 3.88, .40, 3.88);
  shell.lineTo(-.40, 3.88);
  shell.quadraticCurveTo(-.82, 3.88, -1.20, 3.62);
  shell.quadraticCurveTo(-COACH_WIDTH / 2, 3.42, -COACH_WIDTH / 2, 3.05);
  shell.closePath();
  const body = new THREE.ExtrudeGeometry(shell, { depth: COACH_LENGTH, bevelEnabled: false, curveSegments: 5 });
  body.translate(0, 0, -COACH_LENGTH / 2);
  addGeometry(body, silver);

  // Lower body is built around actual door openings, not an opaque extrusion.
  kit.box(0, .9675, 0, COACH_WIDTH, .195, COACH_LENGTH, roofMat);
  for (const z of [-COACH_LENGTH / 2 + .04, COACH_LENGTH / 2 - .04])
    kit.box(0, 1.975, z, COACH_WIDTH, 2.21, .08, silver);
  const spans = [[-7.5, -5.06], [-3.94, -.56], [.56, 3.94], [5.06, 7.5]];
  // Painted waist bands and wall sections stop at each sliding doorway.
  for (const side of [-1, 1]) {
    const x = side * (COACH_WIDTH / 2 + .012);
    for (const [lo, hi] of spans) {
      const center = (lo + hi) / 2, length = hi - lo;
      kit.box(side * 1.51, 1.975, center, .08, 1.95, length, silver);
      kit.box(x, 2.22, center, .035, .20, length, green);
      kit.box(x, 2.075, center, .04, .045, length, orange);
    }
    for (const z of [-4.5, 0, 4.5]) {
      for (const edge of [-.56, .56]) kit.box(side * 1.52, 1.99, z + edge, .09, 1.86, .035, dark);
      if (doors) for (const leaf of [-1, 1]) addDoorLeaf(kit, {silver, glass, green, orange}, side, side * 1.5, z + leaf * .28);
    }
    for (const z of [-6.05, -2.55, 2.55, 6.05]) {
      kit.box(x + side * .013, 2.62, z, .04, 1.05, 1.16, dark);
      kit.box(x + side * .039, 2.62, z, .028, .90, 1.01, glass);
      kit.box(x + side * .055, 2.145, z, .025, .045, 1.10, silver);
    }
    // Low continuous sill and subtle side glazing frames.
    kit.box(x + side * .02, 1.00, 0, .05, .11, COACH_LENGTH - .55, roofMat);
  }
  // Low underfloor battery, compressor and brake cabinets.
  for (const z of [-1.6, 1.6]) {
    kit.box(0, .67, z, 1.38, .38, 1.45, iron);
    kit.box(0, .45, z, 1.10, .08, 1.18, roofMat);
  }
  // Roof air-conditioning units and ventilator grilles.
  for (const z of [-5.35, -2.0, 2.0, 5.35]) {
    kit.box(0, 4.00, z, 1.38, .23, 1.65, roofMat);
    kit.box(0, 4.13, z, .94, .045, 1.24, iron);
  }

  // Bogies: two two-axle trucks with correctly gauged wheels and exposed axle hubs.
  for (const bz of [-4.7, 4.7]) {
    kit.box(0, .43, bz, 2.35, .20, 2.04, iron);
    kit.box(0, .30, bz, 1.90, .12, 1.68, roofMat);
    for (const az of [-.65, .65]) {
      for (const x of [-RAIL_CENTER_OFFSET, RAIL_CENTER_OFFSET]) {
        const geom = new THREE.CylinderGeometry(.32, .32, .14, 12);
        geom.rotateZ(Math.PI / 2);
        geom.translate(x, .32, bz + az);
        addGeometry(geom, wheel);
        // Hub cap on the outside face, matching the wheel axle direction.
        const hub = new THREE.CylinderGeometry(.105, .105, .018, 10);
        hub.rotateZ(Math.PI / 2);
        hub.translate(x + Math.sign(x) * .078, .32, bz + az);
        addGeometry(hub, roofMat);
      }
    }
    // Springs and dampers under the carbody.
    for (const x of [-.78, .78]) kit.box(x, .59, bz, .18, .42, .24, roofMat);
  }

  // End faces remain closed and flat; only the outer cab ends receive glazing.
  const cabZ = role === 'lead' ? COACH_LENGTH / 2 + .012 : role === 'tail' ? -COACH_LENGTH / 2 - .012 : null;
  if (cabZ !== null) {
    const facing = role === 'lead' ? 1 : -1;
    kit.box(0, 2.90, cabZ, 1.92, .76, .035, dark);
    kit.box(0, 2.93, cabZ + facing * .025, 1.65, .56, .025, glass);
    // Destination panel is a solid dark amber field with no generated lettering.
    kit.box(0, 3.48, cabZ + facing * .025, .92, .19, .035, dark);
    if (cabLights) for (const x of [-.83, .83]) kit.box(x, 1.17, cabZ + facing * .035, .18, .17, .04, role === 'lead' ? warm : red);
    kit.box(0, 2.22, cabZ + facing * .032, 2.85, .20, .035, green);
    kit.box(0, 2.075, cabZ + facing * .032, 2.85, .045, .035, orange);
  }

  if (role === 'middle') {
    // A compact articulated pantograph on the roof, kept below 5.2 m.
    const roofZ = -.55;
    const beam = (a: THREE.Vector3, b: THREE.Vector3, width: number, mat: THREE.Material) => {
      const delta = b.clone().sub(a), geo = new THREE.BoxGeometry(width, delta.length(), width * .72);
      const transform = new THREE.Object3D();
      transform.position.copy(a).add(b).multiplyScalar(.5);
      transform.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), delta.clone().normalize());
      transform.updateMatrix();
      geo.applyMatrix4(transform.matrix);
      addGeometry(geo, mat);
    };
    const p0 = new THREE.Vector3(-.72, 4.15, roofZ), p1 = new THREE.Vector3(.72, 4.15, roofZ);
    const p2 = new THREE.Vector3(-.42, 4.86, roofZ), p3 = new THREE.Vector3(.42, 4.86, roofZ);
    beam(p0, p3, .075, iron); beam(p1, p2, .075, iron);
    kit.box(0, PANTOGRAPH_CONTACT_HEIGHT - .0225, roofZ, 1.10, .045, .16, iron);
    kit.box(0, 4.16, roofZ, 1.78, .09, .48, roofMat);
  }
  return kit.finish(false);
}


type DoorPalette = { silver: THREE.Material; glass: THREE.Material; green: THREE.Material; orange: THREE.Material };
function addDoorLeaf(kit: Kit, p: DoorPalette, side: number, x: number, z: number) {
  kit.box(x, 1.9925, z, .04, 1.855, .55, p.silver);
  kit.box(x + side * .027, 2.59, z, .015, .55, .35, p.glass);
  kit.box(x + side * .027, 2.22, z, .015, .20, .55, p.green);
  kit.box(x + side * .027, 2.075, z, .015, .045, .55, p.orange);
}

/** One leaf, shared by all doors; it slides inside the side-wall pocket. */
export function buildCommuterDoorLeaf(art: TrainArt): THREE.Group {
  const kit = new Kit();
  addDoorLeaf(kit, { silver: art.get('#c5c9c8', .38), glass: art.get('#243c45', .18),
    green: art.get('#168b61', .42), orange: art.get('#e8752b', .44) }, 1, 0, 0);
  return kit.finish(false);
}
