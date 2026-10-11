import * as THREE from 'three/webgpu';
import type { Building, BuildingOriginalFootprint } from '../types';
import { ArtMaterials } from './materials';
import { buildingPalette } from './building-palette';
import { compactMidriseEnvelope, midriseMassing } from './building-massing';
import { lowriseBuildingWidth, lowriseMassing } from './lowrise-massing';
import { buildingKit, Kit, type BuildingKitReviewOptions } from './kit';

const FLOOR_HEIGHT = 3.25;
const BASE_HEIGHT = .58;

export interface ExtendedAdditionEnvelope {
  /** Center of the extension in the expanded building's local XZ frame. */
  x: number;
  z: number;
  width: number;
  depth: number;
  /** Front and rear faces in expanded building local Z; rear is negative. */
  frontZ: number;
  rearZ: number;
  /** Retained source mass's actual rear face in the same frame. */
  retainedRearZ: number;
}

function originalBuilding(b: Building, original: BuildingOriginalFootprint): Building {
  return { ...b, x: original.x, z: original.z, width: original.width,
    depth: original.depth, rotation: original.rotation };
}

function localDisplacement(b: Building, original: BuildingOriginalFootprint): THREE.Vector2 {
  const dx = original.x - b.x, dz = original.z - b.z;
  const c = Math.cos(b.rotation), s = Math.sin(b.rotation);
  // Inverse of Three's Y rotation: world displacement -> expanded-local XZ.
  return new THREE.Vector2(c * dx - s * dz, s * dx + c * dz);
}

/**
 * Locate the retained kit's true rear mass and the additive area that reaches
 * the expanded parcel's rear edge. The kit's .84 depth inset and low-rise
 * service wing are included, so construction can use the same join plane.
 */
export function extendedAdditionEnvelope(
  b: Building,
  original: BuildingOriginalFootprint,
  review: BuildingKitReviewOptions = {},
): ExtendedAdditionEnvelope | null {
  if (![b.x, b.z, b.rotation, b.width, b.depth, original.x, original.z, original.rotation, original.width, original.depth]
    .every(Number.isFinite) || b.depth <= original.depth || b.width <= 0 || original.width <= 0) return null;

  const source = originalBuilding(b, original);
  const initialWidth = Math.max(6, source.width * (source.anchor ? .66 : .84));
  const initialDepth = Math.max(6, source.depth * .84);
  const envelope = review.compactMidrise
    ? compactMidriseEnvelope(source, initialWidth, initialDepth)
    : { width: initialWidth, depth: initialDepth };
  const d = envelope.depth;
  const w = lowriseBuildingWidth(source, envelope.width, d);
  const lowrise = lowriseMassing(source, w, d);
  const midrise = midriseMassing(source, w, d);

  let localMassX = 0, localMassZ = -d / 2, massWidth = w;
  if (lowrise) {
    const service = lowrise.masses.find(mass => mass.role === 'service');
    if (service) {
      localMassX = service.x;
      localMassZ = service.z - service.depth / 2;
      massWidth = service.width;
    }
  } else if (midrise) {
    const rear = Math.min(...midrise.masses.map(mass => mass.z - mass.depth / 2));
    const rearMasses = midrise.masses.filter(mass => mass.z - mass.depth / 2 <= rear + .02);
    const left = Math.min(...rearMasses.map(mass => mass.x - mass.width / 2));
    const right = Math.max(...rearMasses.map(mass => mass.x + mass.width / 2));
    localMassX = (left + right) / 2;
    localMassZ = rear;
    massWidth = right - left;
  }

  const sourceOffset = localDisplacement(b, original);
  const yawDelta = original.rotation - b.rotation;
  const c = Math.cos(yawDelta), s = Math.sin(yawDelta);
  const retainedX = sourceOffset.x + c * localMassX + s * localMassZ;
  const retainedRearZ = sourceOffset.y - s * localMassX + c * localMassZ;
  const frontZ = retainedRearZ + .025; // slight hidden overlap prevents a seam
  const rearZ = -b.depth / 2 + .22; // keep roof and wall faces inside the expanded lot
  const width = Math.min(massWidth, b.width - .4);
  if (width <= .5 || frontZ <= rearZ) return null;
  const x = THREE.MathUtils.clamp(retainedX, -b.width / 2 + width / 2 + .2, b.width / 2 - width / 2 - .2);
  return { x, z: (frontZ + rearZ) / 2, width, depth: frontZ - rearZ, frontZ, rearZ, retainedRearZ };
}

function occupancyWindowLit(b: Building, floor: number, bay: number, count: number): boolean {
  if (b.stage !== 'open') return false;
  const residential = b.kind === 'home' || b.kind === 'apartment';
  if (residential ? b.occupancy <= 0 : b.business_active === false) return false;
  const seed = b.appearance_seed >>> 0;
  const guaranteed = seed % Math.max(1, count);
  const mask = Math.imul(seed ^ Math.imul(floor + 1, 0x9e3779b9) ^ Math.imul(bay + 3, 0x85ebca6b), 0xc2b2ae35) >>> 0;
  return (residential && floor === 0 && bay === guaranteed) || mask % (residential ? 5 : 4) === 0;
}

function additionKit(b: Building, envelope: ExtendedAdditionEnvelope, art: ArtMaterials): THREE.Group {
  const kit = new Kit();
  const palette = buildingPalette(b, art.palette);
  const wall = art.get(palette.wall), trim = art.get(palette.trim);
  const shadow = art.get('#344346'), glass = art.get('#55757a', .25);
  const use = b.kind === 'office' ? 'office' : b.kind === 'home' || b.kind === 'apartment' ? 'home' : 'shop';
  const glow = art.get('#e9aa58', .42, true, use);
  const concrete = art.get('#aaa99e'), roof = art.get(['#48575a', '#59676a', '#555d60', '#625d55'][(b.appearance_seed >>> 3) % 4]);
  const { x, z, width, depth } = envelope;
  const floors = Math.max(1, Math.min(30, Math.floor(b.floors)));
  const bodyHeight = floors * FLOOR_HEIGHT;
  const centerY = BASE_HEIGHT + bodyHeight / 2;
  const left = x - width / 2, right = x + width / 2;
  const front = envelope.frontZ, back = envelope.rearZ;

  // The front wall is intentionally a short overlap with the retained rear
  // facade. Every outer face remains within the expanded parcel.
  kit.box(x, BASE_HEIGHT / 2, z, width, BASE_HEIGHT, depth, concrete);
  kit.box(x, centerY, z, width, bodyHeight, depth, wall);
  for (let floor = 0; floor < floors; floor++) {
    const y = BASE_HEIGHT + floor * FLOOR_HEIGHT + 1.7;
    const bayCount = Math.max(1, Math.floor(depth / 3.4));
    for (let bay = 0; bay < bayCount; bay++) {
      const bayZ = front - (bay + .5) * depth / bayCount;
      const pane = occupancyWindowLit(b, floor, bay, bayCount) ? glow : glass;
      for (const side of [-1, 1]) {
        const sideX = side < 0 ? left : right;
        kit.box(sideX - side * .025, y, bayZ, .075, 1.55, 1.12, shadow);
        kit.box(sideX + side * .018, y, bayZ, .035, 1.28, .86, pane);
        kit.box(sideX + side * .05, y, bayZ, .035, 1.33, .055, trim);
        kit.box(sideX + side * .05, y - .69, bayZ, .035, .07, .94, trim);
      }
    }
    // Quiet rear elevation bays make the new depth legible from above and
    // from the service lane without adding a second street-facing facade.
    for (const bayX of [x - width * .25, x + width * .25]) {
      const pane = occupancyWindowLit(b, floor, bayX < x ? 0 : 1, 2) ? glow : glass;
      kit.box(bayX, y, back, Math.min(1.12, width * .24), 1.55, .075, shadow);
      kit.box(bayX, y, back - .045, Math.min(.86, width * .19), 1.28, .035, pane);
      kit.box(bayX, y - .69, back - .07, Math.min(.96, width * .22), .07, .04, trim);
    }
    kit.box(x, BASE_HEIGHT + (floor + 1) * FLOOR_HEIGHT - .12, z, width, .12, depth, concrete);
  }

  const top = BASE_HEIGHT + bodyHeight;
  kit.box(x, top + .08, z, width, .16, depth, roof);
  for (const xx of [left + .08, right - .08]) kit.box(xx, top + .24, z, .12, .32, depth - .16, trim);
  for (const zz of [front - .08, back + .08]) kit.box(x, top + .24, zz, width - .16, .32, .12, trim);
  // Downpipe and drain shoe are restrained details in the same shared iron.
  const iron = art.get('#536164', .62);
  const drainX = right - .18;
  kit.box(drainX, BASE_HEIGHT + bodyHeight / 2, back + .055, .07, bodyHeight, .07, iron);
  kit.box(drainX, .42, back + .18, .08, .12, .32, iron);
  return kit.finish();
}

/**
 * Render the original building unchanged in expanded-building local space,
 * then add the completed rear volume. Without a persisted original footprint,
 * the authoritative buildingKit output is returned directly.
 */
export function buildExtendedBuilding(
  b: Building,
  original: BuildingOriginalFootprint | undefined,
  art: ArtMaterials,
  review: BuildingKitReviewOptions = {},
): THREE.Group {
  if (!original) return buildingKit(b, art, review);
  const source = originalBuilding(b, original);
  const root = new THREE.Group();
  root.userData.buildingId = b.id;
  const retained = buildingKit(source, art, review);
  const offset = localDisplacement(b, original);
  retained.position.set(offset.x, 0, offset.y);
  retained.rotation.y = original.rotation - b.rotation;
  retained.userData.retainedOriginalMass = true;
  root.add(retained);
  const envelope = extendedAdditionEnvelope(b, original, review);
  if (envelope) {
    const addition = additionKit(b, envelope, art);
    addition.name = 'completed-rear-addition';
    addition.userData.extendedAddition = true;
    addition.userData.buildingId = b.id;
    root.add(addition);
    root.userData.additionEnvelope = envelope;
  }
  return root;
}
