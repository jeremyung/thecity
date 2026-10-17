import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../materials';
import { type MidriseMassing } from '../building-massing';
import { type LowriseMassing } from '../lowrise-massing';
import {
  addBladeSign,
  addFacadeFeature,
  addRooftopBillboard,
  hasRooftopBillboard,
  type SignageContext,
} from '../signage';
import { type Building } from '../../types';
import { Kit, addRod, pick } from './core';
import { renderSideElevation } from './lowrise';

export function renderCommercialSignage(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  m: {
    w: number;
    d: number;
    base: number;
    floor: number;
    height: number;
    frame: THREE.Material;
    midrise: MidriseMassing | null;
    lowrise: LowriseMassing | null;
    signage?: SignageContext;
    displays?: import('../architecture/business-recipes').DisplayMode;
  },
) {
  if (m.displays === 'off') return;
  const { w, d, base, floor, height, frame, midrise, lowrise } = m;
  const seed = b.appearance_seed >>> 0,
    proximity = m.signage?.proximity ?? 0;
  const shop = b.kind === 'shop' || b.kind === 'cafe',
    tower = b.floors >= 12 && (b.kind === 'office' || b.kind === 'apartment');
  // Blades: offices of 3–15 floors as before, plus shop-houses of two or more floors.
  if (b.kind === 'office' && b.floors >= 3 && b.floors < 16 && pick(seed >>> 4, 3) !== 0) {
    const shaft =
      midrise?.family === 'setback-office'
        ? midrise.masses.find((mass) => mass.baseFloor > 0 && mass.depth < d)
        : undefined;
    if (shaft) {
      const bottom = base + shaft.baseFloor * floor + 0.3;
      addBladeSign(kit, art, frame, {
        x: shaft.x + shaft.width / 2 - 0.48,
        frontZ: shaft.z + shaft.depth / 2,
        maxDepth: b.depth / 2 - (shaft.z + shaft.depth / 2) - 0.14,
        bottom,
        maxHeight: shaft.floors * floor - 0.6,
        panels: 4,
        seed: seed >>> 9,
      });
    } else if (!tower) {
      addBladeSign(kit, art, frame, {
        x: w / 2 - 0.48,
        frontZ: d / 2,
        maxDepth: (b.depth - d) / 2 - 0.14,
        bottom: 3.2,
        maxHeight: height - 3.4,
        panels: Math.min(4, b.floors - 2),
        seed: seed >>> 9,
      });
    }
  } else if (shop && b.floors >= 2 && pick(seed >>> 6, 4) < 1 + proximity) {
    const frontage = lowrise?.masses.find((mass) => mass.role === 'frontage');
    const frontZ = frontage ? frontage.z + frontage.depth / 2 : d / 2,
      halfW = frontage ? frontage.width / 2 : w / 2,
      cx = frontage?.x ?? 0;
    const side = pick(seed >>> 7, 2) ? 1 : -1;
    addBladeSign(kit, art, frame, {
      x: cx + side * (halfW - 0.48),
      frontZ,
      maxDepth: b.depth / 2 - frontZ - 0.14,
      bottom: 3.2,
      maxHeight: height - 3.4,
      panels: 1,
      seed: seed >>> 9,
    });
  }
  // Rooftop boards stand on the highest flat roof.
  if (!tower && hasRooftopBillboard(b, proximity)) {
    const mains = midrise?.masses.filter((mass) => mass.role === 'main');
    const roof = mains?.length
      ? mains.reduce((a, c) => (a.baseFloor + a.floors > c.baseFloor + c.floors ? a : c))
      : null;
    if (roof)
      addRooftopBillboard(
        kit,
        art,
        frame,
        {
          x: roof.x,
          z: roof.z,
          width: roof.width,
          depth: roof.depth,
          top: base + (roof.baseFloor + roof.floors) * floor,
        },
        seed >>> 15,
      );
    else if (!lowrise)
      addRooftopBillboard(
        kit,
        art,
        frame,
        { x: 0, z: 0, width: w, depth: d, top: base + height },
        seed >>> 15,
      );
  }
  // Station-area podium displays sit above the entrance floor.
  const feature = m.signage?.feature;
  if (feature) {
    if (tower)
      addFacadeFeature(kit, art, frame, feature, {
        frontZ: d / 2,
        width: w * 0.72,
        bottom: base + floor + 0.2,
        height: 2 * floor - 0.6,
        seed,
      });
    else if (midrise?.family === 'setback-office')
      addFacadeFeature(kit, art, frame, feature, {
        frontZ: d / 2,
        width: w * 0.8,
        bottom: base + floor + 0.2,
        height: floor - 0.5,
        seed,
      });
  }
}

export function renderShopDisplay(
  kit: Kit,
  zone: { left: number; right: number },
  m: {
    base: number;
    shopTop: number;
    frontZ: (offset: number, depth: number) => number;
    dark: THREE.Material;
    glass: THREE.Material;
    wood: THREE.Material;
    trim: THREE.Material;
    ivory: THREE.Material;
    latticeCount?: number;
  },
) {
  const { base, shopTop, frontZ, dark, glass, wood, trim, ivory } = m;
  const openingW = Math.max(0.9, zone.right - zone.left),
    center = (zone.left + zone.right) / 2;
  const displayWidth = openingW - 0.18;
  // Recessed glazing keeps a clear depth step behind the timber frame.
  kit.box(center, (base + shopTop) / 2, frontZ(0, 0.13), openingW, shopTop - base, 0.13, dark);
  kit.box(
    center,
    (base + shopTop) / 2,
    frontZ(0.08, 0.035),
    displayWidth,
    shopTop - base - 0.18,
    0.035,
    glass,
  );
  // Repeated bays, a kickboard, and sill make each pane read as a shop window.
  const bayCount = Math.max(3, Math.ceil(displayWidth / 1.6));
  kit.box(center, base + 0.27, frontZ(0.12, 0.12), displayWidth, 0.54, 0.12, wood);
  kit.box(center, base + 0.57, frontZ(0.23, 0.34), displayWidth + 0.08, 0.09, 0.34, trim);
  for (let bay = 1; bay < bayCount; bay++) {
    const x = center - displayWidth / 2 + (displayWidth * bay) / bayCount;
    kit.box(x, (base + shopTop) / 2, frontZ(0.13, 0.09), 0.065, shopTop - base, 0.09, wood);
  }
  kit.box(center, shopTop - 0.43, frontZ(0.13, 0.09), displayWidth, 0.07, 0.09, wood);
  kit.box(center, shopTop + 0.61, frontZ(0.13, 0.13), displayWidth, 0.42, 0.13, ivory);
  const latticeCount = m.latticeCount ?? Math.max(2, Math.floor(displayWidth / 1.2));
  for (let index = 0; index <= latticeCount; index++) {
    const x = center - displayWidth / 2 + (displayWidth * index) / latticeCount;
    kit.box(x, shopTop + 0.61, frontZ(0.22, 0.07), 0.055, 0.38, 0.07, wood);
  }
}

export function renderShopFront(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  m: {
    w: number;
    d: number;
    top: number;
    base: number;
    wood: THREE.Material;
    dark: THREE.Material;
    glass: THREE.Material;
    trim: THREE.Material;
    glow: THREE.Material;
    green: THREE.Material;
    ivory: THREE.Material;
    lantern: THREE.Material;
    concrete: THREE.Material;
    quietSides?: Array<-1 | 1>;
    frontBoundary?: number;
    frontWidthBoundary?: number;
    sideBoundary?: number;
    canopyBoundary?: number;
    centerDoor?: boolean;
  },
) {
  const { w, d, top, base, wood, dark, glass, trim, glow, green, ivory, lantern, concrete } = m;
  // Low-rise shop-houses live above the shop: their upper and side windows keep household hours.
  const livingPane = art.window('home');
  renderSideElevation(kit, b, {
    w,
    d,
    base,
    dark,
    glass: livingPane,
    trim,
    wood,
    concrete,
    quietSides: m.quietSides,
    sideBoundary: m.sideBoundary,
  });
  const front = d / 2 + 0.08;
  const frontZ = (offset: number, depth: number) => {
    if (m.frontBoundary === undefined) return front + offset;
    const face = m.frontBoundary - depth / 2 - 0.08 + offset * 0.22;
    return Math.min(m.frontBoundary - depth / 2, face);
  };
  const doorW = Math.min(1.35, w * 0.17);
  const doorX = m.centerDoor ? 0 : Math.min(w * 0.34, w / 2 - doorW / 2 - 0.12);
  const doorGap = 0.12;
  const displayZones = m.centerDoor
    ? [
        { left: -w * 0.42, right: doorX - doorW / 2 - doorGap },
        { left: doorX + doorW / 2 + doorGap, right: w * 0.42 },
      ]
    : [{ left: -w * 0.42, right: doorX - doorW / 2 - doorGap }];
  const shopTop = Math.min(3.05, top - 0.4);
  const isCafe = b.kind === 'cafe';
  // Display windows balance around the centered entry on low-rise shops, while
  // keeping the founding shop's established single-sided composition intact.
  for (const zone of displayZones) {
    renderShopDisplay(kit, zone, {
      base,
      shopTop,
      frontZ,
      dark,
      glass,
      wood,
      trim,
      ivory,
      latticeCount: m.centerDoor ? undefined : Math.max(3, Math.floor(w / 1.2)),
    });
    // One paper notice sits within an outer glazing bay, clear of entry and mullions.
    const displayWidth = zone.right - zone.left - 0.18;
    if (displayWidth > 1.1) {
      const bays = Math.max(3, Math.ceil(displayWidth / 1.6)),
        bayWidth = displayWidth / bays;
      const width = Math.min(0.55, bayWidth * 0.68),
        height = width * 1.35;
      const face = new THREE.PlaneGeometry(width, height);
      const slot = (isCafe ? 0 : 2) + pick(b.appearance_seed >>> 4, 2),
        uv = face.getAttribute('uv');
      for (let i = 0; i < uv.count; i++) {
        uv.setX(i, ((slot % 2) + 0.03 + uv.getX(i) * 0.94) / 2);
        uv.setY(i, (1 - Math.floor(slot / 2) + 0.03 + uv.getY(i) * 0.94) / 2);
      }
      face.translate(zone.right - 0.09 - bayWidth / 2, base + 1.2, frontZ(0.08, 0.035) + 0.021);
      kit.add(face, art.storefrontPoster());
    }
  }
  const jambs = m.centerDoor
    ? [-w * 0.42, doorX - doorW / 2 - doorGap, doorX + doorW / 2 + doorGap, w * 0.42]
    : [displayZones[0].left, displayZones[0].right, doorX + doorW / 2 + doorGap];
  for (const x of jambs) {
    kit.box(x, (base + shopTop) / 2, frontZ(0.15, 0.2), 0.16, shopTop - base + 0.12, 0.2, wood);
  }
  const doorZ = frontZ(0.2, 0.22);
  if (m.centerDoor)
    kit.box(doorX, base + 0.07, frontZ(0.18, 0.24), doorW + 0.2, 0.14, 0.24, concrete);
  kit.box(doorX, (base + shopTop) / 2, doorZ, doorW, shopTop - base, 0.22, wood);
  kit.box(
    doorX,
    (base + shopTop) / 2,
    frontZ(0.33, 0.035),
    doorW - 0.17,
    shopTop - base - 0.2,
    0.035,
    glow,
  );
  kit.box(
    doorX - doorW * 0.32,
    (base + shopTop) / 2,
    frontZ(0.37, 0.04),
    0.045,
    shopTop - base - 0.5,
    0.04,
    dark,
  );
  kit.cylinder(doorX + doorW * 0.32, 1.45, frontZ(0.39, 0.14), 0.07, 0.07, trim, 6);
  // Warm timber fascia, noren-inspired valance and striped canvas canopy.
  kit.box(0, shopTop + 0.28, frontZ(0.22, 0.32), w * 0.98, 0.46, 0.32, wood);
  const canopyY = Math.min(3.7, top - 0.18);
  const canopyW = Math.min(
    isCafe ? w * 0.84 : w + 0.62,
    m.frontWidthBoundary === undefined ? Infinity : 2 * m.frontWidthBoundary - 0.08,
  );
  // Low-rise frontBoundary is the wall, not the parcel edge. Use the real
  // remaining frontage for an exposed canopy, with a shallow wall embed.
  const projecting = m.canopyBoundary !== undefined && m.frontBoundary !== undefined;
  const canopyD = projecting
    ? Math.min(isCafe ? 0.98 : 1.35, m.canopyBoundary! - m.frontBoundary! + 0.08)
    : m.frontBoundary === undefined
      ? isCafe
        ? 0.98
        : 1.35
      : Math.min(isCafe ? 0.98 : 1.35, Math.max(0.6, d * 0.3));
  const canopyZ = projecting
    ? m.frontBoundary! - 0.08 + canopyD / 2
    : m.frontBoundary === undefined
      ? d / 2 + (isCafe ? 0.53 : 0.72)
      : m.frontBoundary - canopyD / 2;
  const valanceZ = projecting
    ? canopyZ + canopyD / 2 - 0.065
    : frontZ(canopyZ + canopyD / 2 - front, 0.12);
  kit.box(0, canopyY, canopyZ, canopyW, 0.2, canopyD, green);
  kit.box(0, canopyY - 0.19, valanceZ, canopyW, 0.24, 0.12, green);
  const stripe = art.get(isCafe ? '#e4d5b7' : '#ead6af');
  const stripeW = Math.max(0.42, canopyW / (isCafe ? 11 : 9));
  for (let x = -canopyW / 2 + stripeW / 2; x <= canopyW / 2 - stripeW / 2; x += stripeW * 2) {
    kit.box(
      x,
      canopyY - 0.18,
      projecting ? valanceZ + 0.003 : frontZ(canopyZ + canopyD / 2 - front + 0.02, 0.125),
      stripeW,
      0.23,
      0.125,
      stripe,
    );
  }
  for (const x of [-canopyW / 2 + 0.2, canopyW / 2 - 0.2]) {
    kit.box(
      x,
      canopyY - 0.9,
      frontZ(canopyZ + canopyD * 0.18 - front, 0.07),
      0.07,
      1.45,
      0.07,
      wood,
    );
  }
  // Lattice transom panels and paper lamps make the narrow frontage legible from above.
  const transomY = shopTop + 0.61;
  for (const zone of displayZones) {
    const width = Math.max(0.5, zone.right - zone.left - 0.18);
    const center = (zone.left + zone.right) / 2;
    kit.box(center, transomY, frontZ(0.13, 0.13), width, 0.42, 0.13, ivory);
    const latticeCount = Math.max(2, Math.floor(width / 1.2));
    for (let i = 0; i <= latticeCount; i++) {
      const x = center - width / 2 + (width * i) / latticeCount;
      kit.box(x, transomY, frontZ(0.22, 0.07), 0.055, 0.38, 0.07, wood);
    }
  }
  kit.box(-w / 2 + 0.43, shopTop + 0.45, frontZ(0.48, 0.28), 0.42, 0.68, 0.28, dark);
  kit.box(-w / 2 + 0.43, shopTop + 0.45, frontZ(0.64, 0.06), 0.27, 0.48, 0.06, lantern);
  kit.box(w / 2 - 0.38, shopTop + 0.45, frontZ(0.43, 0.24), 0.26, 0.72, 0.24, wood);
  kit.box(w / 2 - 0.38, shopTop + 0.45, frontZ(0.57, 0.055), 0.12, 0.48, 0.055, lantern);
  // Wall-mounted lanterns flank the entry under the eaves.
  for (const x of [-w / 2 + 0.44, w / 2 - 0.44]) {
    kit.box(x, shopTop + 0.95, frontZ(0.22, 0.13), 0.13, 0.5, 0.13, dark);
    kit.box(x, shopTop + 0.89, frontZ(0.29, 0.25), 0.25, 0.22, 0.25, lantern);
  }
  // Slim tiled side shutters and end pilasters.
  for (const x of [-w / 2 + 0.18, w / 2 - 0.18]) {
    kit.box(x, (base + top) / 2, frontZ(0.1, 0.2), 0.24, top - base, 0.2, wood);
  }
  kit.box(0, base + 0.12, frontZ(0.35, 0.24), w * 0.98, 0.16, 0.24, dark);
  for (let floorIndex = 1; floorIndex < b.floors; floorIndex++) {
    const y = base + floorIndex * 3.25 + 1.78;
    for (const x of [-w * 0.27, w * 0.25]) {
      kit.box(x, y, frontZ(-0.02, 0.14), Math.min(2.5, w * 0.25), 1.52, 0.14, dark);
      kit.box(x, y, frontZ(0.06, 0.04), Math.min(2.28, w * 0.23), 1.3, 0.04, livingPane);
      kit.box(x, y, frontZ(0.13, 0.05), 0.06, 1.32, 0.05, trim);
      if (b.kind === 'apartment' || b.kind === 'home') {
        kit.box(x, y - 0.82, frontZ(0.34, 0.62), Math.min(2.8, w * 0.28), 0.12, 0.62, concrete);
        kit.box(x, y - 0.24, frontZ(0.62, 0.06), Math.min(2.8, w * 0.28), 0.06, 0.06, dark);
        for (const rx of [-0.9, 0, 0.9])
          kit.box(x + rx, y - 0.26, frontZ(0.62, 0.05), 0.05, 0.66, 0.05, dark);
      }
    }
  }
}

/** Secondary elevations need the same inhabited scale as the shop front.
 * All details share existing materials and stay inside the normal roof overhang. */

export function renderStreetLife(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  m: {
    w: number;
    d: number;
    parcelW: number;
    parcelD: number;
    frontageEdge?: number;
    wood: THREE.Material;
    dark: THREE.Material;
    green: THREE.Material;
    paleConcrete: THREE.Material;
    iron: THREE.Material;
    trim: THREE.Material;
    seed: number;
  },
) {
  const { w, d, parcelW, parcelD, wood, dark, green, paleConcrete, iron, trim, seed } = m;
  const frontage = m.frontageEdge ?? d / 2 + 0.08;
  const parcelFront = parcelD / 2;
  const clearDepth = parcelFront - frontage;
  const parcelHalfWidth = parcelW / 2;
  const planters = art.get('#817f70');
  const soil = art.get('#4b493e');
  const foliage = art.get('#587363');
  const leafLight = art.get('#718166');

  // Two compact planters soften the shop corners. Their outside edges are
  // clamped inward so narrow parcels never spill into the neighboring lot.
  const planterX = Math.min(w / 2 - 0.48, parcelHalfWidth - 0.43);
  const planterZ = Math.min(frontage + 0.48, parcelFront - 0.31);
  if (planterX > 0.6 && planterZ >= frontage + 0.31) {
    for (const side of [-1, 1]) {
      const x = side * planterX;
      kit.box(x, 0.24, planterZ, 0.76, 0.48, 0.62, planters);
      kit.box(x, 0.49, planterZ, 0.65, 0.045, 0.51, soil);
      for (const offset of [-0.2, 0, 0.2]) {
        const h = 0.38 + pick(seed + Math.round((offset + 0.25) * 100), 3) * 0.08;
        kit.cone(
          x + offset,
          0.52 + h / 2,
          planterZ + (offset === 0 ? 0.02 : 0),
          0.2,
          h,
          offset === 0 ? leafLight : foliage,
          6,
        );
      }
    }
  }

  // A vending machine is a small frontage object, with a shaded display and a
  // warm service panel instead of a large luminous sign.
  const hasFrontage = clearDepth >= 0.78;
  const vendingX = -Math.min(w * 0.32, parcelHalfWidth - 0.48);
  const vendingZ = Math.min(frontage + Math.max(0.48, clearDepth * 0.66), parcelFront - 0.29);
  if (hasFrontage && (b.anchor || pick(seed >>> 2, 3) !== 0) && Math.abs(vendingX) > 0.8) {
    const body = art.get('#c7c1b2');
    const display = art.get('#344a4d', 0.38);
    const accent = art.get('#bd6953');
    kit.box(vendingX, 0.91, vendingZ, 0.78, 1.82, 0.46, body);
    kit.box(vendingX - 0.07, 1.11, vendingZ + 0.238, 0.49, 1.22, 0.035, display);
    for (let row = 0; row < 3; row++) {
      for (let col = 0; col < 2; col++) {
        kit.box(
          vendingX - 0.22 + col * 0.15,
          1.49 - row * 0.3,
          vendingZ + 0.263,
          0.095,
          0.14,
          0.04,
          (row + col) % 2 ? paleConcrete : trim,
        );
      }
    }
    kit.box(vendingX + 0.25, 0.78, vendingZ + 0.24, 0.15, 0.42, 0.04, accent);
    kit.box(vendingX, 0.36, vendingZ + 0.24, 0.44, 0.18, 0.045, dark);
  }

  // One parked bicycle gives the frontage a human scale. The slim rack and
  // frame use shared materials and are merged with the rest of the building.
  const bikeX = Math.min(w * 0.34, parcelHalfWidth - 1.04);
  const bikeZ = Math.min(frontage + Math.max(0.58, clearDepth * 0.72), parcelFront - 0.43);
  const hasBike = parcelW >= 12 && hasFrontage && (b.anchor || pick(seed >>> 5, 2) === 0);
  if (hasBike && bikeX > 0.95) {
    const tire = art.get('#303b3d');
    const frame = art.get('#65766e', 0.58);
    const centerX = bikeX;
    const centerZ = bikeZ;
    const rear = { x: centerX - 0.63, y: 0.39, z: centerZ };
    const front = { x: centerX + 0.63, y: 0.39, z: centerZ };
    const crank = { x: centerX - 0.02, y: 0.42, z: centerZ };
    const seat = { x: centerX - 0.12, y: 0.91, z: centerZ };
    const stem = { x: centerX + 0.4, y: 0.83, z: centerZ };
    for (const wheelX of [rear.x, front.x]) {
      const wheel = new THREE.TorusGeometry(0.35, 0.035, 5, 12);
      wheel.translate(wheelX, 0.39, centerZ);
      kit.add(wheel, tire);
      kit.cylinder(wheelX, 0.39, centerZ, 0.045, 0.08, iron, 6);
    }
    addRod(kit, rear, crank, 0.035, frame);
    addRod(kit, crank, seat, 0.035, frame);
    addRod(kit, seat, rear, 0.035, frame);
    addRod(kit, seat, stem, 0.035, frame);
    addRod(kit, stem, front, 0.035, frame);
    addRod(kit, crank, front, 0.035, frame);
    addRod(kit, stem, { x: stem.x + 0.24, y: stem.y + 0.03, z: centerZ }, 0.035, iron);
    kit.box(centerX - 0.16, 0.98, centerZ, 0.38, 0.08, 0.13, dark);
    kit.box(centerX - 0.02, 0.35, centerZ, 0.14, 0.06, 0.08, iron);

    // A short inverted-U stand sits beside the wheels, contained within the lot.
    const rackX = Math.min(parcelHalfWidth - 0.46, bikeX + 1.15);
    if (rackX > bikeX + 0.55) {
      kit.box(rackX, 0.045, bikeZ, 0.9, 0.09, 0.16, iron);
      addRod(
        kit,
        { x: rackX - 0.34, y: 0.05, z: bikeZ },
        { x: rackX - 0.34, y: 0.63, z: bikeZ },
        0.035,
        iron,
      );
      addRod(
        kit,
        { x: rackX + 0.34, y: 0.05, z: bikeZ },
        { x: rackX + 0.34, y: 0.63, z: bikeZ },
        0.035,
        iron,
      );
      addRod(
        kit,
        { x: rackX - 0.34, y: 0.63, z: bikeZ },
        { x: rackX + 0.34, y: 0.63, z: bikeZ },
        0.035,
        iron,
      );
    }
  }

  // Cafes get one tiny table and two stools tucked under the canopy.
  if (b.kind === 'cafe' && hasFrontage && parcelW >= 10) {
    const tableX = Math.max(-parcelHalfWidth + 0.84, -w * 0.08);
    const tableZ = Math.min(frontage + 0.52, parcelFront - 0.42);
    const tabletop = art.get('#8b6950');
    kit.cylinder(tableX, 0.66, tableZ, 0.34, 0.1, tabletop, 10);
    kit.cylinder(tableX, 0.35, tableZ, 0.055, 0.58, iron, 6);
    for (const dx of [-0.62, 0.62]) {
      kit.cylinder(tableX + dx, 0.46, tableZ, 0.2, 0.09, wood, 8);
      for (const dz of [-0.13, 0.13]) {
        kit.box(tableX + dx, 0.23, tableZ + dz, 0.045, 0.46, 0.045, iron);
      }
    }
  }
}
