import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../materials';
import { type LowriseMass, type LowriseMassing } from '../lowrise-massing';
import { addCeramicRoofDetail } from '../ceramic-roof';
import { type Building } from '../../types';
import { Kit, isModernHomeVariant, pick, windowPane } from './core';
import { flatRoofMembrane } from './midrise';
import { renderShopFront } from './storefront';

export type LowriseOpenMaterials = {
  w: number;
  d: number;
  base: number;
  top: number;
  floor: number;
  wall: THREE.Material;
  roof: THREE.Material;
  roofTile: THREE.Material;
  trim: THREE.Material;
  wood: THREE.Material;
  dark: THREE.Material;
  glass: THREE.Material;
  lightGlass: THREE.Material;
  iron: THREE.Material;
  green: THREE.Material;
  concrete: THREE.Material;
  paleConcrete: THREE.Material;
  glow: THREE.Material;
  lantern: THREE.Material;
  seed: number;
};

export function renderLowriseOpen(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  layout: LowriseMassing,
  m: LowriseOpenMaterials,
) {
  const {
    base,
    floor,
    wall,
    roof,
    roofTile,
    trim,
    wood,
    dark,
    glass,
    lightGlass,
    iron,
    green,
    concrete,
    paleConcrete,
    glow,
    lantern,
    seed,
  } = m;
  const frontage = layout.masses.find((mass) => mass.role === 'frontage')!;
  const service = layout.masses.find((mass) => mass.role === 'service')!;
  const top = base + frontage.floors * floor;
  kit.withOffset(frontage.x, 0, frontage.z, () => {
    // Hold the solid body back by a shallow facade reveal. The authored front
    // elevation can sit in this reveal while the frontage still reaches the lot line.
    kit.box(
      0,
      base + (frontage.floors * floor) / 2,
      -0.06,
      frontage.width,
      frontage.floors * floor,
      frontage.depth - 0.12,
      wall,
    );
    if (b.kind === 'home') {
      renderHouseFacade(kit, b, art, {
        w: frontage.width,
        d: frontage.depth,
        base,
        top,
        floor,
        wood,
        dark,
        glass,
        lightGlass,
        trim,
        glow,
        iron,
        green,
        ivory: art.get('#eee1c1'),
        concrete,
        entrySide: 1,
        quietSides: layout.quietSides,
        frontOnly: true,
        centerEntry: true,
        lowriseDetail: true,
        frontBoundary: frontage.depth / 2,
        frontWidthBoundary: b.width / 2,
        sideBoundary: b.width / 2 - Math.abs(frontage.x),
      });
    } else {
      renderShopFront(kit, b, art, {
        w: frontage.width,
        d: frontage.depth,
        top,
        base,
        wood,
        dark,
        glass,
        trim,
        glow,
        green,
        ivory: art.get('#eee1c1'),
        lantern,
        concrete,
        quietSides: layout.quietSides,
        frontBoundary: frontage.depth / 2,
        frontWidthBoundary: b.width / 2,
        sideBoundary: b.width / 2 - Math.abs(frontage.x),
        canopyBoundary: b.depth / 2 - frontage.z - 0.04,
        centerDoor: true,
      });
    }
  });
  renderLowriseServiceElevation(kit, frontage, {
    base,
    floor,
    pane: art.window('home'),
    trim,
    concrete,
    dark,
    iron,
  });
  renderLowriseRoof(
    kit,
    frontage,
    top,
    roof,
    flatRoofMembrane(art),
    roofTile,
    trim,
    iron,
    dark,
    seed,
    b.width,
    b.depth,
    wall,
  );

  // A single-storey rear utility volume joins the main wall but keeps its own
  // roofline; its louver and service door sit on the quiet rear elevation.
  const serviceWall = art.get(['#a99983', '#a5a08e', '#9eaa9d'][pick(seed >>> 8, 3)]);
  const serviceTop = base + service.floors * floor;
  kit.box(
    service.x,
    base + (service.floors * floor) / 2,
    service.z,
    service.width,
    service.floors * floor,
    service.depth,
    serviceWall,
  );
  const rearZ = service.z - service.depth / 2 + 0.025;
  kit.box(service.x, base + 1.04, rearZ, 0.82, 1.86, 0.07, dark);
  kit.box(service.x, base + 1.04, rearZ - 0.035, 0.64, 1.62, 0.035, wood);
  kit.box(service.x + service.width * 0.27, base + 2.18, rearZ, 0.42, 0.34, 0.055, iron);
  kit.box(service.x + service.width * 0.27, base + 2.18, rearZ - 0.035, 0.28, 0.2, 0.035, dark);
  for (let slat = -1; slat <= 1; slat++)
    kit.box(
      service.x + service.width * 0.27 + slat * 0.085,
      base + 2.18,
      rearZ - 0.06,
      0.025,
      0.17,
      0.025,
      trim,
    );
  if (isModernHomeVariant(b))
    renderModernHomeCondenser(
      kit,
      service,
      layout.quietSides[0],
      base,
      paleConcrete,
      dark,
      iron,
      trim,
    );
  renderLowriseRoof(
    kit,
    service,
    serviceTop,
    roof,
    flatRoofMembrane(art),
    roofTile,
    trim,
    iron,
    dark,
    seed >>> 3,
    b.width,
    b.depth,
    serviceWall,
  );

  // A shallow eave marks the frontage without adding another full-height
  // corner element to the compact silhouette.
  if (b.kind === 'home') {
    const eave = frontage.z + frontage.depth / 2 - 0.05;
    kit.box(0, top - 0.12, eave, 0.12, 0.1, 0.1, wood);
  }
  void paleConcrete;
  void glow;
  void lantern;
}

/** Rear rooms and roof drainage complete the occupied low-rise elevation.
 * The attached one-storey service wing owns ground-level openings; upper rooms
 * remain above its roof. All details stay within the original parcel envelope. */

export function renderLowriseServiceElevation(
  kit: Kit,
  mass: LowriseMass,
  m: {
    base: number;
    floor: number;
    pane: THREE.Material;
    trim: THREE.Material;
    concrete: THREE.Material;
    dark: THREE.Material;
    iron: THREE.Material;
  },
) {
  const { base, floor, pane, trim, concrete, dark, iron } = m;
  const rear = mass.z - mass.depth / 2;
  const bays = Math.max(1, Math.floor(mass.width / 3.2));
  const pitch = (mass.width - 0.8) / bays;
  const windowW = Math.min(1.8, pitch - 0.45),
    windowH = 1.35;
  for (let level = 1; level < mass.floors; level++) {
    const y = base + level * floor + 1.65;
    for (let bay = 0; bay < bays; bay++) {
      const x = mass.x - mass.width / 2 + 0.4 + pitch * (bay + 0.5);
      kit.box(x, y, rear - 0.015, windowW + 0.16, windowH + 0.18, 0.13, dark);
      kit.box(x, y, rear - 0.095, windowW, windowH, 0.035, pane);
      kit.box(x, y, rear - 0.125, 0.045, windowH, 0.025, trim);
      kit.box(x, y - windowH / 2 - 0.09, rear - 0.1, windowW + 0.22, 0.1, 0.2, concrete);
      // A shallow metal hood follows each opening, not a balcony slab.
      kit.box(x, y + windowH / 2 + 0.12, rear - 0.13, windowW + 0.28, 0.07, 0.28, iron);
    }
  }
  const top = base + mass.floors * floor;
  // Downpipes are tied to the roof edge, avoiding the centered service wing.
  for (const side of [-1, 1]) {
    const x = mass.x + side * (mass.width / 2 - 0.2);
    const pipeZ = rear - 0.11;
    kit.cylinder(x, (base + top) / 2, pipeZ, 0.045, top - base, iron, 6);
    for (let level = 0; level < mass.floors; level++)
      kit.box(x, base + level * floor + 0.5, pipeZ, 0.13, 0.06, 0.13, trim);
  }
  // The existing gable sheds to front/rear; flat and shed roofs retain coping.
  if (mass.roof === 'gable') {
    kit.box(mass.x, top - 0.06, rear - 0.14, mass.width - 0.16, 0.13, 0.18, iron);
  }
}

/** An attached split-system condenser and short line set on the quiet service side. */

export function renderModernHomeCondenser(
  kit: Kit,
  service: LowriseMass,
  side: -1 | 1,
  base: number,
  pale: THREE.Material,
  dark: THREE.Material,
  iron: THREE.Material,
  trim: THREE.Material,
) {
  const centerX = service.x + side * (service.width / 2 + 0.36),
    centerY = base + 1.3,
    centerZ = service.z + service.depth * 0.16;
  // Its inner edge touches the service wall; the outer edge stays inside the lot.
  kit.box(centerX, centerY, centerZ, 0.72, 0.56, 0.48, pale);
  kit.box(centerX + side * 0.37, centerY, centerZ, 0.025, 0.38, 0.38, dark);
  for (let slat = -2; slat <= 2; slat++)
    kit.box(centerX + side * 0.39, centerY + slat * 0.065, centerZ, 0.025, 0.025, 0.32, iron);
  // Two small wall brackets support the unit without suggesting a balcony deck.
  for (const dz of [-0.16, 0.16])
    kit.box(centerX - side * 0.19, base + 1.02, centerZ + dz, 0.36, 0.055, 0.055, iron);
  const pipeX = service.x + side * (service.width / 2 + 0.035),
    pipeZ = centerZ - 0.39;
  kit.cylinder(pipeX, base + 2.03, pipeZ, 0.022, 1.08, trim, 6);
  kit.box(pipeX, base + 1.5, centerZ - 0.3, 0.045, 0.045, 0.18, trim);
}

export function renderLowriseRoof(
  kit: Kit,
  mass: LowriseMass,
  top: number,
  roof: THREE.Material,
  membrane: THREE.Material,
  tiles: THREE.Material,
  trim: THREE.Material,
  iron: THREE.Material,
  dark: THREE.Material,
  seed: number,
  parcelW: number,
  parcelD: number,
  gableWall: THREE.Material,
) {
  kit.withOffset(mass.x, 0, mass.z, () => {
    if (mass.roof === 'gable') {
      renderLowriseGable(
        kit,
        mass,
        top,
        roof,
        tiles,
        trim,
        iron,
        dark,
        seed,
        parcelW / 2 - Math.abs(mass.x),
        parcelD / 2 - mass.z,
        parcelD / 2 + mass.z,
        gableWall,
      );
    } else if (mass.roof === 'shed') {
      const slope = new THREE.BoxGeometry(
        Math.min(mass.width - 0.24, 2 * (parcelW / 2 - Math.abs(mass.x))),
        0.14,
        Math.min(mass.depth - 0.24, parcelD / 2 - mass.z + parcelD / 2 + mass.z),
      );
      slope.rotateX(0.075);
      slope.translate(0, top + 0.11, 0);
      kit.add(slope, roof);
      kit.box(
        0,
        top + 0.2,
        0,
        Math.min(mass.width * 0.9, 2 * (parcelW / 2 - Math.abs(mass.x))),
        0.08,
        0.12,
        trim,
      );
    } else {
      kit.box(0, top + 0.08, 0, mass.width - 0.1, 0.16, mass.depth - 0.1, membrane);
      for (const z of [-mass.depth / 2 + 0.09, mass.depth / 2 - 0.09])
        kit.box(0, top + 0.28, z, mass.width - 0.12, 0.4, 0.12, trim);
      for (const x of [-mass.width / 2 + 0.09, mass.width / 2 - 0.09])
        kit.box(x, top + 0.28, 0, 0.12, 0.4, mass.depth - 0.12, trim);
      // A small raised roof hatch breaks the flat parapet without implying an
      // additional occupied floor.
      const hatchW = Math.min(1.1, mass.width * 0.24),
        hatchD = Math.min(0.9, mass.depth * 0.2);
      kit.box(-mass.width * 0.18, top + 0.3, -mass.depth * 0.15, hatchW, 0.28, hatchD, dark);
    }
  });
}

export function renderLowriseGable(
  kit: Kit,
  mass: LowriseMass,
  top: number,
  roof: THREE.Material,
  tiles: THREE.Material,
  trim: THREE.Material,
  iron: THREE.Material,
  dark: THREE.Material,
  seed: number,
  halfX: number,
  frontLimit: number,
  backLimit: number,
  gableWall: THREE.Material,
) {
  const half = Math.min(mass.depth / 2 + 0.16, frontLimit, backLimit);
  const roofW = Math.min(mass.width + 0.32, Math.max(0.4, halfX * 2));
  const rise = Math.min(2.6, half * 0.42),
    angle = Math.atan2(rise, half),
    slopeLength = Math.hypot(half, rise);
  for (const side of [-1, 1]) {
    const panel = new THREE.BoxGeometry(roofW, 0.18, slopeLength);
    panel.rotateX(side * angle);
    panel.translate(0, top + rise / 2, (side * half) / 2);
    kit.add(panel, roof);
    kit.box(0, top - 0.015, side * half, roofW, 0.13, 0.1, trim);
  }
  // Close both gable ends with the facade material. The slope continues
  // above the wall edge into the eave; the infill stops at the body footprint.
  const wallHalf = mass.depth / 2,
    eaveRise = rise * (1 - wallHalf / half);
  const positions: number[] = [];
  for (const side of [-1, 1]) {
    const x = (side * mass.width) / 2;
    const a = [x, top, -wallHalf],
      b = [x, top, wallHalf];
    const c = [x, top + eaveRise, wallHalf],
      d = [x, top + rise, 0],
      e = [x, top + eaveRise, -wallHalf];
    for (const triangle of [
      [a, b, c],
      [a, c, d],
      [a, d, e],
    ]) {
      const vertices = side > 0 ? [triangle[0], triangle[2], triangle[1]] : triangle;
      positions.push(...vertices.flat());
    }
  }
  const infill = new THREE.BufferGeometry();
  infill.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  infill.setIndex(Array.from({ length: positions.length / 3 }, (_, i) => i));
  const uv: number[] = [];
  for (let i = 0; i < positions.length; i += 3)
    uv.push((positions[i + 2] + wallHalf) / mass.depth, (positions[i + 1] - top) / rise);
  infill.setAttribute('uv', new THREE.Float32BufferAttribute(uv, 2));
  infill.computeVertexNormals();
  kit.add(infill, gableWall);
  addCeramicRoofDetail(kit, { width: roofW, halfDepth: half, rise, top, roof, tile: roof });
  kit.box(0, top + rise + 0.12, 0, roofW, 0.22, 0.22, roof);
  if (pick(seed, 4) === 0)
    kit.box(0, top + rise + 0.2, 0, Math.min(0.6, roofW * 0.2), 0.08, 0.28, tiles);
}

export function renderLowriseConstruction(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  layout: LowriseMassing,
  m: {
    w: number;
    d: number;
    base: number;
    top: number;
    floor: number;
    floors: number;
    parcelW: number;
    parcelD: number;
    progress: number;
    wall: THREE.Material;
    roof: THREE.Material;
    roofTile: THREE.Material;
    trim: THREE.Material;
    concrete: THREE.Material;
    iron: THREE.Material;
    scaffold: THREE.Material;
    wood: THREE.Material;
    paleConcrete: THREE.Material;
    caution: THREE.Material;
    dark: THREE.Material;
    glass: THREE.Material;
    lightGlass: THREE.Material;
    green: THREE.Material;
  },
) {
  const {
    w,
    d,
    base,
    top,
    floor,
    progress,
    wall,
    roof,
    roofTile,
    trim,
    concrete,
    iron,
    scaffold,
    wood,
    paleConcrete,
    caution,
    dark,
    glass,
    lightGlass,
    green,
  } = m;
  const amount = Number.isFinite(progress) ? THREE.MathUtils.clamp(progress, 0, 1) : 0;
  const builtFloors = Math.ceil(m.floors * THREE.MathUtils.clamp((amount - 0.12) / 0.57, 0, 1));
  const installed = Math.ceil(THREE.MathUtils.clamp((amount - 0.68) / 0.25, 0, 1) * builtFloors);
  // The last cladding pass becomes the exact open-stage facade and roof family.
  // Keep a low work scaffold and grounded supplies visible, but do not stack a
  // second partial shell or activate commercial lighting before the building opens.
  if (amount >= 0.86) {
    renderLowriseOpen(kit, b, art, layout, {
      w,
      d,
      base,
      top,
      floor,
      wall,
      roof,
      roofTile,
      trim,
      wood,
      dark,
      glass,
      lightGlass,
      iron,
      green,
      concrete,
      paleConcrete,
      glow: glass,
      lantern: art.get('#ffbd62', 0.5),
      seed: b.appearance_seed >>> 0,
    });
    renderLowriseWorksite(kit, layout, {
      base,
      floor,
      builtFloors,
      parcelW: m.parcelW,
      parcelD: m.parcelD,
      wood,
      paleConcrete,
      caution,
      scaffold,
    });
    return;
  }
  for (const mass of layout.masses) {
    const levels = Math.min(mass.floors, builtFloors);
    const frameHeight = levels * floor;
    if (levels > 0) {
      for (let level = 0; level < levels; level++) {
        const y0 = base + level * floor;
        for (const x of [mass.x - mass.width / 2 + 0.14, mass.x + mass.width / 2 - 0.14])
          for (const z of [mass.z - mass.depth / 2 + 0.14, mass.z + mass.depth / 2 - 0.14])
            kit.box(x, y0 + floor / 2, z, 0.18, floor, 0.18, iron);
        kit.box(mass.x, y0 + floor - 0.09, mass.z, mass.width, 0.18, mass.depth, concrete);
        if (level < installed) {
          // Cladding stays in the same mass footprint and leaves the frontage
          // doorway visibly open while the frame is still a work site.
          for (const side of [-1, 1]) {
            const z = mass.z + side * (mass.depth / 2 - 0.08);
            kit.box(
              mass.x - mass.width * 0.34,
              y0 + floor * 0.56,
              z,
              mass.width * 0.28,
              floor * 0.72,
              0.12,
              wall,
            );
            kit.box(
              mass.x + mass.width * 0.34,
              y0 + floor * 0.56,
              z,
              mass.width * 0.28,
              floor * 0.72,
              0.12,
              wall,
            );
            kit.box(mass.x, y0 + floor * 0.85, z, mass.width * 0.65, 0.12, 0.12, wood);
          }
          for (const side of [-1, 1]) {
            const x = mass.x + side * (mass.width / 2 - 0.07);
            kit.box(x, y0 + floor * 0.5, mass.z, 0.12, floor * 0.78, mass.depth * 0.72, wall);
          }
        }
      }
    }
    const scaffoldTop = Math.max(2.3, base + frameHeight + 0.5);
    const halfX = Math.min(mass.width / 2 + 0.16, m.parcelW / 2 - Math.abs(mass.x) - 0.04);
    const halfZ = Math.min(mass.depth / 2 + 0.16, m.parcelD / 2 - Math.abs(mass.z) - 0.04);
    const xs = [mass.x - halfX, mass.x + halfX],
      zs = [mass.z - halfZ, mass.z + halfZ];
    for (const x of xs) {
      for (const z of zs) kit.box(x, scaffoldTop / 2, z, 0.055, scaffoldTop, 0.055, scaffold);
      for (let y = 1.1; y < scaffoldTop; y += 2.1) {
        kit.box(x, y, mass.z, 0.045, 0.045, halfZ * 2, scaffold);
        kit.box(mass.x, y, x === xs[0] ? zs[0] : zs[1], halfX * 2, 0.045, 0.045, scaffold);
      }
    }
  }
  renderLowriseWorksite(kit, layout, {
    base,
    floor,
    builtFloors,
    parcelW: m.parcelW,
    parcelD: m.parcelD,
    wood,
    paleConcrete,
    caution,
    scaffold,
  });
}

export function renderLowriseWorksite(
  kit: Kit,
  layout: LowriseMassing,
  m: {
    base: number;
    floor: number;
    builtFloors: number;
    parcelW: number;
    parcelD: number;
    wood: THREE.Material;
    paleConcrete: THREE.Material;
    caution: THREE.Material;
    scaffold: THREE.Material;
  },
) {
  const { base, floor, builtFloors, parcelW, parcelD, wood, paleConcrete, caution, scaffold } = m;
  // Grounded supplies and a tarp give the foundation stage a legible cause
  // without introducing a floating crane or material above the parcel.
  kit.box(-0.8, 0.38, 0.1, 1.35, 0.62, 0.7, wood);
  kit.box(0.9, 0.28, -0.1, 1.25, 0.42, 0.82, paleConcrete);
  const frontage = layout.masses.find((mass) => mass.role === 'frontage')!;
  kit.box(
    0,
    0.72,
    Math.min(frontage.z + frontage.depth / 2 + 0.12, parcelD / 2 - 0.24),
    Math.min(1.7, parcelW - 0.5),
    0.08,
    0.4,
    caution,
  );
  renderLowriseRenovationScaffold(
    kit,
    layout.masses,
    Math.min(6.5, base + builtFloors * floor + 0.4),
    scaffold,
    caution,
    parcelW,
    parcelD,
  );
}

export function renderLowriseRenovationScaffold(
  kit: Kit,
  masses: LowriseMass[],
  height: number,
  scaffold: THREE.Material,
  caution: THREE.Material,
  parcelW = Infinity,
  parcelD = Infinity,
) {
  const bounds = masses.map((mass) => ({
    minX: mass.x - mass.width / 2,
    maxX: mass.x + mass.width / 2,
    minZ: mass.z - mass.depth / 2,
    maxZ: mass.z + mass.depth / 2,
  }));
  const minX = Math.min(...bounds.map((b) => b.minX)),
    maxX = Math.max(...bounds.map((b) => b.maxX));
  const minZ = Math.min(...bounds.map((b) => b.minZ)),
    maxZ = Math.max(...bounds.map((b) => b.maxZ));
  const h = Math.max(1.8, height);
  const left = Math.max(minX - 0.12, -parcelW / 2 + 0.04),
    right = Math.min(maxX + 0.12, parcelW / 2 - 0.04);
  const back = Math.max(minZ - 0.12, -parcelD / 2 + 0.04),
    front = Math.min(maxZ + 0.12, parcelD / 2 - 0.04);
  for (const x of [left, right])
    for (const z of [back, front]) kit.box(x, h / 2, z, 0.055, h, 0.055, scaffold);
  kit.box(
    (left + right) / 2,
    Math.min(1.45, h / 2),
    front - 0.07,
    right - left,
    0.12,
    0.12,
    caution,
  );
}

export function renderSideElevation(
  kit: Kit,
  b: Building,
  m: {
    w: number;
    d: number;
    base: number;
    dark: THREE.Material;
    glass: THREE.Material;
    trim: THREE.Material;
    wood: THREE.Material;
    concrete: THREE.Material;
    quietSides?: Array<-1 | 1>;
    sideBoundary?: number;
  },
) {
  const { w, d, base, dark, glass, trim, wood, concrete, quietSides } = m;
  const seed = b.appearance_seed >>> 0;
  for (const side of [-1, 1]) {
    if (quietSides?.includes(side as -1 | 1)) continue;
    const sideX = (offset: number, width: number) => {
      const desired = side * (w / 2 + offset);
      if (m.sideBoundary === undefined) return desired;
      return side > 0
        ? Math.min(desired, m.sideBoundary - width / 2)
        : Math.max(desired, -m.sideBoundary + width / 2);
    };
    for (let level = 0; level < b.floors; level++) {
      const y = base + level * 3.25 + 1.72;
      for (const [index, z] of [-d * 0.24, d * 0.23].entries()) {
        const width = Math.min(1.6, d * 0.24),
          height = level === 0 && index === 0 ? 1.05 : 1.4;
        // The frame sits just behind the pane's face so the glass shows within it,
        // including where the wall is clamped against the lot line.
        const paneX = sideX(0.08, 0.035);
        kit.box(
          paneX - side * (0.0175 + 0.012 + 0.065),
          y,
          z,
          0.13,
          height + 0.18,
          width + 0.18,
          wood,
        );
        kit.box(paneX, y, z, 0.035, height, width, glass);
        kit.box(sideX(0.13, 0.035), y, z, 0.035, height, 0.055, trim);
        kit.box(sideX(0.14, 0.3), y - height / 2 - 0.14, z, 0.3, 0.12, width + 0.3, concrete);
        kit.box(sideX(0.2, 0.45), y + height / 2 + 0.2, z, 0.45, 0.1, width + 0.4, dark);
        if ((seed + level + index + (side > 0 ? 1 : 0)) % 3 === 0) {
          kit.box(sideX(0.1, 0.1), y, z + width / 2 + 0.24, 0.1, height + 0.12, 0.32, wood);
          for (let slat = 0; slat < 4; slat++)
            kit.box(
              sideX(0.165, 0.035),
              y - height * 0.35 + slat * height * 0.23,
              z + width / 2 + 0.24,
              0.035,
              0.025,
              0.28,
              trim,
            );
        }
      }
    }
    // A narrow service strip visually grounds the elevation without adding lawn.
    kit.box(sideX(0, 0.17), base + 0.1, 0, 0.17, 0.2, d * 0.92, concrete);
  }
}

export function renderHouseFacade(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  m: {
    w: number;
    d: number;
    base: number;
    top: number;
    floor: number;
    wood: THREE.Material;
    dark: THREE.Material;
    glass: THREE.Material;
    lightGlass: THREE.Material;
    trim: THREE.Material;
    glow: THREE.Material;
    iron: THREE.Material;
    green: THREE.Material;
    ivory: THREE.Material;
    concrete: THREE.Material;
    entrySide?: -1 | 1;
    quietSides?: Array<-1 | 1>;
    frontOnly?: boolean;
    centerEntry?: boolean;
    lowriseDetail?: boolean;
    frontBoundary?: number;
    frontWidthBoundary?: number;
    sideBoundary?: number;
  },
) {
  const {
    w,
    d,
    base,
    floor,
    wood,
    dark,
    lightGlass,
    trim,
    iron,
    green,
    ivory,
    concrete,
    entrySide = 1,
  } = m;
  // Every sash shows a room; whether its light is on is decided per room in
  // the shader from the time of day and the building's occupancy.
  const pane = windowPane(art, b);
  const windowGlass = (
    _floorIndex: number,
    _windowIndex: number,
    _windowCount: number,
    _unlit?: THREE.Material,
  ) => pane;
  renderSideElevation(kit, b, {
    w,
    d,
    base,
    dark,
    glass: pane,
    trim,
    wood,
    concrete,
    quietSides: m.quietSides,
    sideBoundary: m.sideBoundary,
  });
  const fronts = m.frontOnly ? [1] : [1, -1];
  for (const side of fronts) {
    const z = side * (d / 2 + 0.08);
    const frontZ = (offset: number, depth: number) => {
      if (side !== 1 || m.frontBoundary === undefined) return z + side * offset;
      const face = m.frontBoundary - depth / 2 - 0.08 + offset * 0.22;
      return Math.min(m.frontBoundary - depth / 2, face);
    };
    if (b.floors === 1) {
      // A modest machiya-like frontage: broad sliding panels, timber lintels and a porch.
      const doorX = m.centerEntry ? 0 : -entrySide * w * 0.25;
      const windowW = Math.min(2.35, w * 0.42);
      if (m.centerEntry) {
        const sideWindowW = Math.min(1.3, Math.max(0.6, (w - 1.25 - 0.7) / 2));
        const windowX = 0.625 + 0.2 + sideWindowW / 2;
        for (const [windowIndex, x] of [-windowX, windowX].entries()) {
          const pane = windowGlass(0, windowIndex, 2);
          if (m.lowriseDetail) {
            renderLowriseHouseWindow(kit, {
              x,
              y: base + 1.58,
              width: sideWindowW - 0.18,
              height: 1.48,
              frontZ,
              pane,
              wood,
              dark,
              trim,
              concrete,
              horizontalSash: false,
            });
          } else {
            kit.box(x, base + 1.57, frontZ(0, 0.12), sideWindowW, 1.75, 0.12, wood);
            kit.box(x, base + 1.58, frontZ(0.08, 0.035), sideWindowW - 0.18, 1.48, 0.035, pane);
            for (let k = -1; k <= 1; k++)
              kit.box(x + k * 0.42, base + 1.58, frontZ(0.14, 0.035), 0.045, 1.48, 0.035, trim);
          }
        }
      } else {
        const windowX = entrySide * w * 0.19;
        kit.box(windowX, base + 1.57, frontZ(0, 0.12), windowW, 1.75, 0.12, wood);
        kit.box(
          windowX,
          base + 1.58,
          frontZ(0.08, 0.035),
          windowW - 0.22,
          1.48,
          0.035,
          windowGlass(0, 0, 1),
        );
        for (let k = -1; k <= 1; k++)
          kit.box(windowX + k * 0.52, base + 1.58, frontZ(0.14, 0.035), 0.045, 1.48, 0.035, trim);
      }
      // The door sits behind a deep timber frame, giving the narrow entry a
      // genuine shadow line rather than another flat facade panel.
      kit.box(doorX, base + 1.45, frontZ(0.055, 0.12), 1.25, 2.52, 0.12, dark);
      kit.box(doorX, base + 1.45, frontZ(0.11, 0.07), 1.0, 2.28, 0.07, wood);
      if (!m.centerEntry)
        kit.box(doorX, base + 1.45, frontZ(0.16, 0.035), 0.76, 2.05, 0.035, lightGlass);
      if (m.centerEntry)
        kit.box(doorX, base + 0.07, frontZ(0.18, 0.24), 1.45, 0.14, 0.24, concrete);
      const porchW = Math.min(
        w + 0.7,
        m.frontWidthBoundary === undefined ? Infinity : 2 * m.frontWidthBoundary - 0.08,
      );
      kit.box(0, base + 2.58, frontZ(0.15, 0.22), Math.min(w + 0.2, porchW), 0.24, 0.22, dark);
      kit.box(0, base + 2.92, frontZ(0.54, 1.05), porchW, 0.16, 1.05, wood);
      kit.box(0, base + 2.72, frontZ(1.02, 0.13), porchW, 0.34, 0.13, dark);
      kit.box(-w * 0.33, base + 0.4, frontZ(0.42, 0.12), 0.12, 0.8, 0.12, wood);
      kit.box(w * 0.33, base + 0.4, frontZ(0.42, 0.12), 0.12, 0.8, 0.12, wood);
    } else if (m.centerEntry && side === 1) {
      const doorW = Math.min(1.12, w * 0.24),
        doorH = Math.min(2.38, floor - 0.58),
        doorY = base + doorH / 2 + 0.16;
      kit.box(0, doorY, frontZ(0.055, 0.11), doorW + 0.22, doorH + 0.18, 0.11, dark);
      kit.box(0, doorY, frontZ(0.105, 0.06), doorW, doorH, 0.06, wood);
      kit.box(0, base + doorH + 0.24, frontZ(0.11, 0.22), doorW + 0.62, 0.14, 0.22, green);
      kit.box(0, base + 0.07, frontZ(0.18, 0.24), doorW + 0.2, 0.14, 0.24, concrete);
      const windowW = Math.min(1.42, Math.max(0.72, (w - doorW - 0.7) / 2));
      const windowX = doorW / 2 + 0.2 + windowW / 2;
      for (const [windowIndex, x] of [-windowX, windowX].entries()) {
        const pane = windowGlass(0, windowIndex, 2);
        if (m.lowriseDetail) {
          renderLowriseHouseWindow(kit, {
            x,
            y: base + 1.8,
            width: windowW - 0.18,
            height: 1.34,
            frontZ,
            pane,
            wood,
            dark,
            trim,
            concrete,
            horizontalSash: true,
          });
        } else {
          kit.box(x, base + 1.8, frontZ(0, 0.1), windowW, 1.55, 0.1, dark);
          kit.box(x, base + 1.8, frontZ(0.07, 0.035), windowW - 0.18, 1.34, 0.035, pane);
          kit.box(x, base + 1.8, frontZ(0.12, 0.045), 0.045, 1.36, 0.045, trim);
          kit.box(x, base + 0.91, frontZ(0.24, 0.42), windowW + 0.12, 0.12, 0.42, concrete);
        }
      }
    } else {
      for (const x of [-w * 0.26, w * 0.24]) {
        kit.box(x, base + 1.9, z, Math.min(2.8, w * 0.3), 1.75, 0.13, dark);
        kit.box(x, base + 1.9, z + side * 0.08, Math.min(2.55, w * 0.27), 1.48, 0.04, pane);
        kit.box(x, base + 1.9, z + side * 0.15, 0.07, 1.5, 0.05, trim);
        kit.box(x, base + 1.9, z + side * 0.15, 2.5, 0.07, 0.05, trim);
        kit.box(x, base + 0.74, z + side * 0.35, 3.0, 0.13, 0.64, concrete);
        for (const railX of [x - 1.28, x + 1.28])
          kit.box(railX, base + 1.12, z + side * 0.62, 0.055, 0.78, 0.055, iron);
        kit.box(x, base + 1.45, z + side * 0.62, 2.65, 0.06, 0.06, iron);
      }
    }
    for (let level = 1; level < b.floors; level++) {
      const y = base + level * floor + 1.65;
      for (const [windowIndex, x] of [-w * 0.24, w * 0.24].entries()) {
        const opening = Math.min(2.3, w * 0.32);
        const pane = windowGlass(level, windowIndex, 2);
        if (m.lowriseDetail && m.centerEntry && side === 1) {
          renderLowriseHouseWindow(kit, {
            x,
            y,
            width: opening - 0.18,
            height: 1.28,
            frontZ,
            pane,
            wood,
            dark,
            trim,
            concrete,
            horizontalSash: true,
          });
        } else {
          kit.box(x, y, frontZ(0, 0.13), opening, 1.46, 0.13, wood);
          kit.box(x, y, frontZ(0.08, 0.035), opening - 0.18, 1.28, 0.035, pane);
          kit.box(x, y, frontZ(0.13, 0.035), 0.055, 1.3, 0.035, trim);
          kit.box(x, y - 0.82, frontZ(0.16, 0.32), opening + 0.2, 0.12, 0.32, concrete);
          kit.box(x, y + 0.83, frontZ(0.22, 0.48), opening + 0.32, 0.1, 0.48, dark);
        }
        if (isModernHomeVariant(b) && m.centerEntry && side === 1) {
          // A shallow window guard gives only the modern-home subset a light
          // metal rhythm; it is mounted to the sill, not a walkable balcony.
          const guardWidth = opening - 0.3,
            guardZ = frontZ(0.28, 0.045),
            guardBottom = y - 0.63,
            guardTop = y - 0.3;
          kit.box(x, (guardBottom + guardTop) / 2, guardZ, guardWidth, 0.045, 0.045, iron);
          for (let bar = 0; bar < 5; bar++) {
            const guardX = x - guardWidth / 2 + (guardWidth * (bar + 0.5)) / 5;
            kit.box(
              guardX,
              (guardBottom + guardTop) / 2,
              guardZ,
              0.035,
              guardTop - guardBottom,
              0.045,
              iron,
            );
          }
        }
      }
    }
    // Eaves carry a thin wood beam and small downturned corner brackets.
    kit.box(
      0,
      base + 3.03,
      frontZ(0.22, 0.28),
      Math.min(
        w + 0.48,
        m.frontWidthBoundary === undefined ? Infinity : 2 * m.frontWidthBoundary - 0.08,
      ),
      0.16,
      0.28,
      wood,
    );
    for (const x of [-w / 2 + 0.25, w / 2 - 0.25])
      kit.box(x, base + 2.72, frontZ(0.28, 0.12), 0.12, 0.48, 0.12, wood);
  }
  // A short side entry canopy with two brackets breaks up the plain end wall.
  kit.box(-w / 2 + 0.43, base + 2.3, 0, 0.82, 0.14, 2.4, green);
  for (const z of [-0.86, 0.86]) kit.box(-w / 2 + 0.12, base + 1.58, z, 0.1, 1.42, 0.1, wood);
  void ivory;
}

/** A shallow recessed sash assembly for low-rise homes; every part stays inside the authored frontage. */

export function renderLowriseHouseWindow(
  kit: Kit,
  m: {
    x: number;
    y: number;
    width: number;
    height: number;
    frontZ: (offset: number, depth: number) => number;
    pane: THREE.Material;
    wood: THREE.Material;
    dark: THREE.Material;
    trim: THREE.Material;
    concrete: THREE.Material;
    horizontalSash: boolean;
  },
) {
  const { x, y, width, height, frontZ, pane, wood, dark, trim, concrete, horizontalSash } = m;
  const paneW = Math.max(0.28, width),
    paneH = height;

  // Dark backing sits just ahead of the plaster wall. The lighter pane is
  // recessed behind the projecting timber sash, so each edge reads in relief.
  kit.box(x, y, frontZ(-0.1, 0.045), paneW + 0.12, paneH + 0.12, 0.045, dark);
  kit.box(x, y, frontZ(0.015, 0.035), paneW, paneH, 0.035, pane);

  const jamb = 0.075,
    rail = 0.075;
  const frameZ = frontZ(0.16, 0.065);
  kit.box(x - paneW / 2 - jamb / 2, y, frameZ, jamb, paneH + 0.14, 0.065, wood);
  kit.box(x + paneW / 2 + jamb / 2, y, frameZ, jamb, paneH + 0.14, 0.065, wood);
  kit.box(x, y - paneH / 2 - rail / 2, frameZ, paneW, rail, 0.065, wood);
  kit.box(x, y + paneH / 2 + rail / 2, frameZ, paneW, rail, 0.065, wood);

  // Two sliding-sash rails give the glazing a readable domestic scale without
  // adding per-window objects or changing its silhouette.
  const sashZ = frontZ(0.24, 0.035);
  kit.box(x, y, sashZ, 0.038, paneH, 0.035, trim);
  if (horizontalSash) kit.box(x, y, sashZ, paneW, 0.038, 0.035, trim);

  // A narrow stone sill projects beneath the reveal; its top supports the frame.
  kit.box(x, y - paneH / 2 - 0.045, frontZ(0.22, 0.22), paneW + 0.2, 0.085, 0.22, concrete);
}

/** The window pane material for a building's light schedule. */

export function renderGabledRoof(
  kit: Kit,
  w: number,
  d: number,
  top: number,
  roof: THREE.Material,
  tiles: THREE.Material,
  trim: THREE.Material,
  iron: THREE.Material,
  dark: THREE.Material,
  seed: number,
  parcelWidth: number,
  wall: THREE.Material,
) {
  const half = d / 2 + 0.58;
  const rise = THREE.MathUtils.clamp(half * 0.48, 0.82, 2.1);
  const angle = Math.atan2(rise, half);
  const slopeLength = Math.hypot(half, rise);
  const roofW = Math.min(w + 1.15, parcelWidth);
  // Two correctly mirrored planes meet at a straight ridge with their eaves at the walls.
  for (const side of [-1, 1]) {
    const panel = new THREE.BoxGeometry(roofW, 0.25, slopeLength);
    panel.rotateX(side * angle);
    panel.translate(0, top + rise / 2, (side * half) / 2);
    kit.add(panel, roof);
    // Eaves and their short exposed rafter tails.
    kit.box(0, top - 0.02, side * half, Math.min(roofW + 0.22, parcelWidth), 0.18, 0.2, trim);
    for (let x = -w / 2; x <= w / 2; x += 1.65) {
      kit.box(x, top - 0.1, side * (d / 2 + 0.32), 0.1, 0.24, 0.46, iron);
    }
  }
  addCeramicRoofDetail(kit, {
    width: roofW,
    halfDepth: half,
    rise,
    top,
    roof,
    tile: roof,
    surfaceOffset: 0.15,
  });
  kit.box(0, top + rise + 0.12, 0, Math.min(roofW + 0.12, parcelWidth), 0.26, 0.3, roof);
  // Close the end walls perpendicular to the ridge. The previous trim sat
  // on the eave faces and left both triangular roof volumes open.
  const endHalf = d / 2,
    eaveRise = rise * (1 - endHalf / half);
  for (const side of [-1, 1]) {
    const shape = new THREE.Shape();
    shape.moveTo(-endHalf, 0);
    shape.lineTo(endHalf, 0);
    shape.lineTo(endHalf, eaveRise);
    shape.lineTo(0, rise);
    shape.lineTo(-endHalf, eaveRise);
    shape.closePath();
    const infill = new THREE.ShapeGeometry(shape);
    // Shape x becomes world z; front faces point outward on each gable end.
    infill.rotateY((side * Math.PI) / 2);
    infill.translate((side * w) / 2, top, 0);
    if (!infill.index)
      infill.setIndex(Array.from({ length: infill.getAttribute('position').count }, (_, i) => i));
    kit.add(infill, wall);
    for (const slopeSide of [-1, 1]) {
      const fascia = new THREE.BoxGeometry(0.1, 0.14, slopeLength);
      fascia.rotateX(slopeSide * angle);
      fascia.translate(side * (roofW / 2 - 0.05), top + rise / 2, (slopeSide * half) / 2);
      kit.add(fascia, trim);
    }
    kit.box(side * (w / 2 + 0.04), top + rise * 0.48, 0, 0.08, 0.4, 0.52, dark);
    kit.box(side * (w / 2 + 0.09), top + rise * 0.48, 0, 0.035, 0.08, 0.34, iron);
  }
  // Compact soil-colored tile accent varies by seed while the palette remains finite.
  if (pick(seed, 4) === 0) kit.box(0, top + rise + 0.31, 0, 0.8, 0.1, 0.42, tiles);
}

/** Flat waterproof fields are distinct from dark ceramic/metal pitched roofs.
 * One shared finish keeps roof fields batchable across appearance seeds. */
