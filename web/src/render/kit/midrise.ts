import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../materials';
import { type BuildingMass, type MidriseMassing } from '../building-massing';
import { renderRoofServices } from '../roof-services';
import { type Building } from '../../types';
import { Kit, baseValue, pick, windowPane } from './core';

export type MidriseOpenMaterials = {
  w: number;
  d: number;
  base: number;
  floor: number;
  wall: THREE.Material;
  roof: THREE.Material;
  trim: THREE.Material;
  concrete: THREE.Material;
  paleConcrete: THREE.Material;
  iron: THREE.Material;
  dark: THREE.Material;
  glass: THREE.Material;
  lightGlass: THREE.Material;
  wood: THREE.Material;
  glow: THREE.Material;
};

export function renderMidriseOpen(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  layout: MidriseMassing,
  m: MidriseOpenMaterials,
  installedThrough = Infinity,
  includeRoof = true,
) {
  const {
    base,
    floor,
    wall,
    roof,
    trim,
    concrete,
    paleConcrete,
    iron,
    dark,
    glass,
    lightGlass,
    wood,
    glow,
  } = m;
  const mains = layout.masses.filter((mass) => mass.role === 'main');
  const roofMass = mains.reduce((a, c) =>
    a.baseFloor + a.floors > c.baseFloor + c.floors ? a : c,
  );

  for (const source of layout.masses) {
    const admittedFloors = Math.min(
      source.floors,
      Math.max(0, installedThrough - source.baseFloor),
    );
    if (admittedFloors <= 0) continue;
    const mass = { ...source, floors: admittedFloors };
    const massBase = base + mass.baseFloor * floor;
    const massTop = base + (mass.baseFloor + mass.floors) * floor;
    kit.withOffset(mass.x, 0, mass.z, () => {
      kit.box(
        0,
        massBase + (mass.floors * floor) / 2,
        0,
        mass.width,
        mass.floors * floor,
        mass.depth,
        wall,
      );
      if (mass.role === 'core') {
        renderMidriseCoreFacade(kit, {
          outerSide: mass.x < 0 ? -1 : 1,
          width: mass.width,
          depth: mass.depth,
          base: massBase,
          floor,
          floors: mass.floors,
          wood,
          dark,
          glass,
          iron,
          concrete,
          trim,
        });
        // A compact stair headhouse sits on the full-height circulation core.
        if (source.baseFloor + source.floors <= installedThrough) {
          kit.box(0, massTop + 0.16, 0, mass.width * 0.72, 0.32, mass.depth * 0.7, trim);
          kit.box(0, massTop + 0.54, 0, mass.width * 0.42, 0.48, mass.depth * 0.42, concrete);
        }
      } else {
        const core = layout.masses.find((candidate) => candidate.role === 'core');
        const omitSideWindows = core ? (core.x > mass.x ? 1 : -1) : undefined;
        const terraceEntry = layout.family === 'terrace-home' && mass.baseFloor > 0;
        const entryX =
          (mass.baseFloor === 0 || terraceEntry) && Math.abs(mass.x) <= mass.width / 2
            ? -mass.x
            : undefined;
        renderUrbanFacade(kit, b, art, {
          w: mass.width,
          d: mass.depth,
          base: massBase,
          top: massTop,
          floor,
          floors: mass.floors,
          baseFloor: mass.baseFloor,
          balconyStyle:
            layout.family === 'terrace-apartment' || layout.family === 'slab-apartment'
              ? 'continuous'
              : undefined,
          omitSideWindows,
          entryX,
          terraceEntry,
          parcelD: b.depth,
          massOffsetZ: mass.z,
          parcelW: b.width,
          massOffsetX: mass.x,
          wood,
          dark,
          glass,
          lightGlass,
          trim,
          glow,
          iron,
          concrete,
          paleConcrete,
          isApartment: b.kind === 'apartment' || layout.family === 'terrace-home',
        });
        // Only the podium/main facade owns the building entrance. Upper office
        // tiers use their global floor index and never repeat a ground trim.
        if (mass.baseFloor === 0 || terraceEntry) {
          const doorX = -mass.x;
          if (Math.abs(doorX) < mass.width * 0.32) {
            renderMidriseEntry(kit, doorX, mass.depth, massBase, dark, glass, trim, concrete);
          }
        }
      }
    });
  }

  // Exposed terrace strips appear only where a main volume steps inward.
  for (let i = 0; i < mains.length - 1; i++) {
    const lower = mains[i],
      upper = mains[i + 1];
    if (
      lower.baseFloor + lower.floors !== upper.baseFloor ||
      lower.baseFloor + lower.floors > installedThrough ||
      upper.baseFloor + upper.floors > installedThrough
    )
      continue;
    const y = base + upper.baseFloor * floor - 0.045;
    const plate = 0.12,
      overlap = 0.08;
    if (layout.family === 'terrace-apartment' || layout.family === 'terrace-home') {
      const front = lower.z + lower.depth / 2 - 0.06;
      const rear = upper.z + upper.depth / 2;
      const width = lower.width - 0.12;
      // Thin walking finish covers only the exposed strip, rests on the lower
      // mass, and avoids inheriting its vertical facade material.
      kit.box(
        lower.x,
        base + upper.baseFloor * floor + 0.025,
        (front + rear) / 2,
        width,
        0.05,
        front - rear,
        concrete,
      );
      const guardY = base + upper.baseFloor * floor + 0.55;
      kit.box(lower.x, guardY, front, width, 1.1, 0.12, paleConcrete);
      for (const side of [-1, 1]) {
        kit.box(
          lower.x + side * (width / 2 - 0.06),
          guardY,
          (front + rear) / 2,
          0.12,
          1.1,
          front - rear,
          paleConcrete,
        );
      }
      continue;
    }
    kit.withOffset(lower.x, 0, lower.z, () => {
      for (const side of [-1, 1]) {
        kit.box(0, y, side * (lower.depth / 2 - plate / 2), lower.width, plate, plate, concrete);
        kit.box(side * (lower.width / 2 - plate / 2), y, 0, plate, plate, lower.depth, concrete);
      }
      // Seeded slim rails stop short at the family corners, retaining a quiet
      // terrace line rather than outlining another full box.
      for (const side of [-1, 1]) {
        const lowerEdge = lower.z + (side * lower.depth) / 2;
        const upperEdge = upper.z + (side * upper.depth) / 2;
        if (side * (lowerEdge - upperEdge) < 0.25) continue;
        const z = side * (lower.depth / 2 - overlap);
        kit.box(0, y + 0.42, z, lower.width * 0.72, 0.055, 0.055, iron);
        for (const x of [-lower.width * 0.36, 0, lower.width * 0.36])
          kit.box(x, y + 0.22, z, 0.045, 0.42, 0.045, iron);
      }
    });
  }

  // Roof equipment belongs to one highest main mass; the core receives only
  // its slim stair cap above. This avoids buried duplicate equipment at tiers.
  if (includeRoof) {
    const roofTop = base + (roofMass.baseFloor + roofMass.floors) * floor;
    kit.withOffset(roofMass.x, 0, roofMass.z, () =>
      renderFlatRoof(kit, {
        w: roofMass.width,
        d: roofMass.depth,
        top: roofTop,
        roof,
        trim,
        concrete,
        iron,
        paleConcrete,
        seed: b.appearance_seed,
        art,
      }),
    );
  }
}

export function renderMidriseCoreFacade(
  kit: Kit,
  m: {
    outerSide: -1 | 1;
    width: number;
    depth: number;
    base: number;
    floor: number;
    floors: number;
    wood: THREE.Material;
    dark: THREE.Material;
    glass: THREE.Material;
    iron: THREE.Material;
    concrete: THREE.Material;
    trim: THREE.Material;
  },
) {
  const { width, depth, base, floor, floors, wood, dark, glass, iron, concrete, trim } = m;
  const slotW = Math.max(0.32, Math.min(0.62, width * 0.38));
  for (let f = 0; f < floors; f++) {
    const y = base + f * floor + 1.9;
    for (const side of [-1, 1]) {
      const z = side * (depth / 2 + 0.08);
      kit.box(0, y, z, slotW + 0.16, 2.05, 0.13, dark);
      kit.box(0, y, z + side * 0.08, slotW, 1.82, 0.04, glass);
      kit.box(0, y, z + side * 0.13, 0.055, 1.84, 0.04, iron);
      kit.box(0, base + (f + 1) * floor - 0.13, z + side * 0.08, width * 0.86, 0.14, 0.18, trim);
    }
    // The circulation core spans the full depth. Give its exposed flank a
    // restrained service-window rhythm; the attached apartment side stays solid.
    const outer = m.outerSide,
      x = (outer * width) / 2;
    const bays = Math.max(1, Math.floor((depth - 1.6) / 5));
    for (let bay = 0; bay < bays; bay++) {
      const z = ((bay + 0.5) * depth) / bays - depth / 2,
        landingY = base + f * floor + 2.15;
      kit.box(x + outer * 0.045, landingY, z, 0.12, 1.31, 0.84, dark);
      kit.box(x + outer * 0.12, landingY, z, 0.035, 1.15, 0.68, glass);
      kit.box(x + outer * 0.15, landingY, z, 0.035, 1.15, 0.045, trim);
    }
    if (f === 0) {
      for (const side of [-1, 1]) {
        const z = side * (depth / 2 + 0.11);
        kit.box(0, base + 1.06, z, Math.min(1.1, width * 0.62), 2.05, 0.16, dark);
        kit.box(0, base + 1.06, z + side * 0.1, Math.min(0.88, width * 0.5), 1.78, 0.04, wood);
        kit.box(
          0,
          base + 2.18,
          z + side * 0.17,
          Math.min(1.35, width * 0.82),
          0.13,
          0.42,
          concrete,
        );
      }
    }
  }
}

export function renderMidriseEntry(
  kit: Kit,
  doorX: number,
  depth: number,
  base: number,
  dark: THREE.Material,
  glass: THREE.Material,
  trim: THREE.Material,
  concrete: THREE.Material,
) {
  kit.box(doorX, base + 1.13, depth / 2 + 0.075, 0.98, 2.18, 0.16, dark);
  kit.box(doorX, base + 1.13, depth / 2 + 0.17, 0.76, 1.92, 0.045, glass);
  kit.box(doorX + 0.29, base + 1.12, depth / 2 + 0.205, 0.045, 0.1, 0.025, trim);
  kit.box(doorX, base + 2.31, depth / 2 + 0.22, 1.24, 0.13, 0.46, concrete);
}

export function renderMassingConstruction(
  kit: Kit,
  layout: Pick<MidriseMassing, 'masses'>,
  m: {
    w: number;
    d: number;
    parcelW: number;
    base: number;
    floor: number;
    floors: number;
    builtFloors: number;
    progress: number;
    authoredEnvelope?: boolean;
    wall: THREE.Material;
    concrete: THREE.Material;
    iron: THREE.Material;
    glass: THREE.Material;
    dark: THREE.Material;
    wood: THREE.Material;
    scaffold: THREE.Material;
    caution: THREE.Material;
    paleConcrete: THREE.Material;
  },
) {
  const {
    base,
    floor,
    builtFloors,
    progress,
    wall,
    concrete,
    iron,
    glass,
    dark,
    wood,
    scaffold,
    caution,
    paleConcrete,
  } = m;
  const installBudget = Math.ceil(THREE.MathUtils.clamp((progress - 0.69) / 0.23, 0, 1) * m.floors);
  for (const mass of layout.masses) {
    // A tier cannot acquire a scaffold before work reaches its base level.
    if (mass.baseFloor > builtFloors) continue;
    const erected = Math.max(0, Math.min(mass.floors, builtFloors - mass.baseFloor));
    const massBase = base + mass.baseFloor * floor;
    const frameTop = massBase + erected * floor;
    const scaffoldTop = Math.min(
      massBase + mass.floors * floor + 0.45,
      Math.max(massBase + 2.1, frameTop + 1.05),
    );
    kit.withOffset(mass.x, 0, mass.z, () => {
      const installed = m.authoredEnvelope
        ? Math.min(erected, Math.max(0, installBudget - mass.baseFloor))
        : 0;
      for (let f = installed; f < erected; f++) {
        const y0 = massBase + f * floor;
        for (const x of [-mass.width / 2 + 0.18, mass.width / 2 - 0.18])
          for (const z of [-mass.depth / 2 + 0.18, mass.depth / 2 - 0.18]) {
            kit.box(x, y0 + floor / 2, z, 0.18, floor, 0.18, iron);
          }
        kit.box(0, y0 + floor - 0.08, 0, mass.width, 0.16, mass.depth, concrete);
      }
      const cladFloors = Math.min(
        erected,
        Math.max(0, Math.min(mass.floors, installBudget - mass.baseFloor)),
      );
      for (let f = 0; f < (m.authoredEnvelope ? 0 : cladFloors); f++) {
        const y = massBase + f * floor + 1.62;
        const panelH = 2.42;
        for (const side of [-1, 1]) {
          const z = side * (mass.depth / 2 - 0.07);
          kit.box(-mass.width * 0.3, y, z, mass.width * 0.38, panelH, 0.12, wall);
          kit.box(mass.width * 0.3, y, z, mass.width * 0.38, panelH, 0.12, wall);
          kit.box(0, y + 0.73, z, mass.width * 0.19, 0.72, 0.12, wall);
          if (progress > 0.84) {
            kit.box(0, y, z + side * 0.075, mass.width * 0.25, 1.05, 0.035, dark);
            kit.box(0, y, z + side * 0.1, mass.width * 0.22, 0.92, 0.025, glass);
          }
        }
        for (const side of [-1, 1]) {
          const x = side * (mass.width / 2 - 0.07);
          kit.box(x, y, -mass.depth * 0.28, 0.12, panelH, mass.depth * 0.38, wall);
          kit.box(x, y, mass.depth * 0.28, 0.12, panelH, mass.depth * 0.38, wall);
          kit.box(x + side * 0.075, y + 0.73, 0, 0.035, 0.72, mass.depth * 0.19, wall);
        }
      }
      // Inset working scaffold stays within each authored mass outline.
      if (scaffoldTop > massBase + 1.3) {
        const insetX = Math.max(0.12, mass.width / 2 - 0.17),
          insetZ = Math.max(0.12, mass.depth / 2 - 0.17);
        for (const x of [-insetX, insetX]) {
          for (const z of [-insetZ, insetZ])
            kit.box(
              x,
              (massBase + scaffoldTop) / 2,
              z,
              0.055,
              scaffoldTop - massBase,
              0.055,
              scaffold,
            );
          for (let y = massBase + 1.15; y < scaffoldTop; y += 2.3)
            kit.box(x, y, 0, 0.045, 0.055, mass.depth - 0.3, scaffold);
        }
        for (const z of [-insetZ, insetZ])
          for (let y = massBase + 1.15; y < scaffoldTop; y += 2.3)
            kit.box(0, y, z, mass.width - 0.3, 0.055, 0.045, scaffold);
      }
    });
  }

  // Site logistics remain readable at every construction stage, including
  // foundation-only progress, and never exceed the supplied parcel envelope.
  const palletW = Math.min(1.7, m.w * 0.22),
    palletD = Math.min(1.2, m.d * 0.18);
  const palletX = Math.max(0, m.w / 2 - palletW / 2 - 0.25),
    palletZ = Math.max(0, m.d / 2 - palletD / 2 - 0.25);
  kit.box(-palletX, 0.5, -palletZ, palletW, 0.72, palletD, wood);
  kit.box(palletX, 0.4, palletZ, palletW * 0.85, 0.5, palletD * 0.8, paleConcrete);
  for (let i = -1; i <= 1; i++)
    kit.box(-palletX + i * palletW * 0.28, 0.89, -palletZ, 0.035, 0.08, palletD, iron);

  const craneX = Math.min(m.w * 0.3, Math.max(0.5, m.w / 2 - 0.35));
  const craneZ = -Math.min(m.d * 0.22, Math.max(0.45, m.d / 2 - 0.35));
  const craneHeight = Math.min(
    m.floors * floor + 4,
    Math.max(4.4, base + Math.max(erectedMax(layout, builtFloors), 0) * floor + 2.8),
  );
  kit.box(craneX, craneHeight / 2, craneZ, 0.16, craneHeight, 0.16, caution);
  const jibW = Math.min(Math.max(1.4, m.w * 0.58), 2 * (m.parcelW / 2 - craneX - 0.02));
  kit.box(craneX, craneHeight - 0.18, craneZ, jibW, 0.14, 0.15, caution);
  kit.box(craneX - jibW * 0.24, craneHeight - 0.65, craneZ, 0.1, 1, 0.1, iron);
}

export function erectedMax(layout: Pick<MidriseMassing, 'masses'>, builtFloors: number) {
  return layout.masses.reduce((max, mass) => {
    const reached = Math.max(0, Math.min(mass.floors, builtFloors - mass.baseFloor));
    return Math.max(max, reached > 0 ? mass.baseFloor + reached : 0);
  }, 0);
}

export function renderMidriseRenovationScaffold(
  kit: Kit,
  masses: BuildingMass[],
  m: { height: number; floor: number },
  material: THREE.Material,
) {
  const seen = new Set<string>();
  for (const mass of masses) {
    // The initial renovation works from street level; elevated tiers remain
    // quiet until a distinct upper-floor access scaffold is authored.
    if (mass.baseFloor > 0) continue;
    const h = Math.min(
      m.height,
      mass.floors * m.floor,
      Math.max(2.6, m.height - mass.baseFloor * m.floor),
    );
    if (h <= 0) continue;
    const key = `${mass.x.toFixed(2)}:${mass.z.toFixed(2)}:${mass.width.toFixed(2)}:${mass.depth.toFixed(2)}`;
    if (seen.has(key)) continue;
    seen.add(key);
    kit.withOffset(mass.x, 0, mass.z, () => {
      const x = Math.max(0.1, mass.width / 2 - 0.16),
        z = Math.max(0.1, mass.depth / 2 - 0.16);
      for (const xx of [-x, x])
        for (const zz of [-z, z]) kit.box(xx, h / 2, zz, 0.06, h, 0.06, material);
      for (let y = 1.2; y < h; y += 2.35) {
        for (const xx of [-x, x]) kit.box(xx, y, 0, 0.045, 0.055, mass.depth - 0.28, material);
        for (const zz of [-z, z]) kit.box(0, y, zz, mass.width - 0.28, 0.055, 0.045, material);
      }
    });
  }
}

export function renderUrbanFacade(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  m: {
    w: number;
    d: number;
    base: number;
    top: number;
    floor: number;
    floors: number;
    baseFloor?: number;
    balconyStyle?: 'continuous';
    omitSideWindows?: 1 | -1;
    entryX?: number;
    terraceEntry?: boolean;
    parcelD?: number;
    massOffsetZ?: number;
    parcelW?: number;
    massOffsetX?: number;
    wood: THREE.Material;
    dark: THREE.Material;
    glass: THREE.Material;
    lightGlass: THREE.Material;
    trim: THREE.Material;
    glow: THREE.Material;
    iron: THREE.Material;
    concrete: THREE.Material;
    paleConcrete: THREE.Material;
    isApartment: boolean;
  },
) {
  const { w, d, base, floor, floors, wood, dark, trim, iron, concrete, paleConcrete, isApartment } =
    m;
  const pane = windowPane(art, b);
  const baseFloor = m.baseFloor ?? 0;
  const spacing = isApartment ? (w > 17 ? 3.35 : 3.05) : w > 17 ? 4.5 : 3.25;
  for (let f = 0; f < floors; f++) {
    const y = base + f * floor + 1.95;
    for (const side of [1, -1]) {
      const z = side * (d / 2 + 0.07);
      const doorOpening =
        (baseFloor + f === 0 || (m.terraceEntry && f === 0)) &&
        side === 1 &&
        m.entryX !== undefined;
      if (b.kind === 'office') {
        // Offices use one calm curtain-wall ribbon per floor. Slim mullions and
        // opaque spandrels create a civic rhythm distinct from apartment bays.
        const bandW = w * 0.84;
        const segments: Array<[number, number]> = doorOpening
          ? [
              [-bandW / 2, Math.min(bandW / 2, m.entryX! - 0.62)],
              [Math.max(-bandW / 2, m.entryX! + 0.62), bandW / 2],
            ]
          : [[-bandW / 2, bandW / 2]];
        for (const [left, right] of segments) {
          const segmentW = right - left;
          if (segmentW < 0.12) continue;
          const center = (right + left) / 2;
          kit.box(center, y, z, segmentW, 2.02, 0.13, dark);
          kit.box(center, y + 0.04, z + side * 0.08, segmentW - 0.08, 1.78, 0.04, pane);
          const mullions = Math.max(1, Math.floor(segmentW / 1.65));
          for (let i = 0; i <= mullions; i++) {
            const x = left + (segmentW * i) / mullions;
            kit.box(x, y + 0.04, z + side * 0.13, 0.055, 1.8, 0.045, trim);
          }
          kit.box(center, y - 1.05, z + side * 0.11, segmentW, 0.2, 0.16, paleConcrete);
        }
      } else {
        for (let x = -w / 2 + 1.5; x < w / 2 - 0.8; x += spacing) {
          const ww = isApartment ? Math.min(spacing - 0.5, 1.85) : Math.min(spacing - 0.55, 2.35);
          if (doorOpening && x + ww / 2 > m.entryX! - 0.62 && x - ww / 2 < m.entryX! + 0.62)
            continue;
          const balconyDoor = m.balconyStyle === 'continuous' && side === 1 && baseFloor + f > 0;
          const openingY = balconyDoor ? base + f * floor + 1.3 : y;
          const openingH = balconyDoor ? 2.22 : 1.51;
          kit.box(x, openingY, z, ww + 0.22, openingH + 0.24, 0.13, dark);
          kit.box(x, openingY, z + side * 0.08, ww, openingH, 0.04, pane);
          kit.box(x, openingY, z + side * 0.15, 0.065, openingH, 0.045, trim);
          if (!balconyDoor) kit.box(x, y, z + side * 0.15, ww, 0.065, 0.045, trim);
          if (
            isApartment &&
            m.balconyStyle !== 'continuous' &&
            (f + pick(b.appearance_seed, 2)) % 2 === 0
          ) {
            // Fit the projection to this facade's actual parcel clearance;
            // broad plots retain deep balconies instead of using the narrow kit.
            const clearance =
              m.parcelD === undefined
                ? Infinity
                : m.parcelD / 2 - side * (m.massOffsetZ ?? 0) - d / 2;
            const balconyDepth = Math.max(0.12, Math.min(0.92, clearance - 0.17));
            const balconyProjection = 0.07 + balconyDepth / 2;
            const balconyZ = z + side * balconyProjection;
            const balconyW = ww + 0.3;
            const railZ = balconyZ + side * (balconyDepth / 2 - 0.05);
            kit.box(x, y - 0.94, balconyZ, balconyW, 0.13, balconyDepth, concrete);
            kit.box(x, y - 0.47, railZ, balconyW, 0.1, 0.12, paleConcrete);
            for (let railX = x - balconyW / 2; railX <= x + balconyW / 2 + 0.01; railX += 0.62) {
              kit.box(railX, y - 0.38, railZ, 0.045, 0.82, 0.045, iron);
            }
            kit.box(x, y + 0.08, railZ, balconyW, 0.055, 0.055, iron);
          }
        }
      }
      if (m.balconyStyle === 'continuous' && side === 1 && baseFloor + f > 0) {
        // One floor-level slab and closed guard, not overlapping window-sized
        // boxes. The slab embeds into the wall and stays inside the parcel.
        const front = Math.min(
          d / 2 + 0.92,
          (m.parcelD ?? d + 2) / 2 - (m.massOffsetZ ?? 0) - 0.08,
        );
        const back = d / 2 - 0.04,
          depth = front - back,
          width = w - 0.24;
        const slabY = base + f * floor + 0.08;
        kit.box(0, slabY, (front + back) / 2, width, 0.13, depth, concrete);
        const guardY = slabY + 0.065 + 0.535;
        kit.box(0, guardY, front - 0.05, width, 1.07, 0.1, paleConcrete);
        kit.box(0, guardY + 0.535, front - 0.05, width, 0.055, 0.12, iron);
        for (const end of [-1, 1]) {
          kit.box(
            end * (width / 2 - 0.05),
            guardY,
            (front + back) / 2,
            0.1,
            1.07,
            depth,
            paleConcrete,
          );
        }
      }
      // Continuous lintels, sills, and floor bands lend the narrow facade a built rhythm.
      if (!doorOpening && b.kind !== 'office')
        kit.box(0, y - 1.02, z + side * 0.08, w + 0.18, 0.14, 0.2, paleConcrete);
      kit.box(0, base + (f + 1) * floor - 0.12, z + side * 0.08, w + 0.28, 0.18, 0.22, trim);
      if (!doorOpening && baseFloor + f === 0 && b.kind === 'office') {
        kit.box(0, base + 1.12, z + side * 0.14, w * 0.68, 0.28, 0.08, wood);
      }
    }
  }
  // Side windows and fire stairs give block-shaped buildings a back and a service side.
  for (let f = 0; f < floors; f++) {
    for (const side of [-1, 1] as const) {
      if (m.omitSideWindows === side) continue;
      const sideClearance =
        m.parcelW === undefined
          ? Infinity
          : Math.max(0, m.parcelW / 2 - (side * (m.massOffsetX ?? 0) + w / 2));
      const sideOffset = Math.min(0.08, Math.max(0, sideClearance - 0.1));
      const x = side * (w / 2 + sideOffset),
        y = base + f * floor + 1.95;
      if (b.kind === 'office' && sideClearance >= 0.24) {
        // Carry the office's ribbon glazing around its side elevations instead
        // of reverting to residential punched windows. Keep every projection
        // inside the same parcel clearance used by the original side windows.
        const bandD = d * 0.84;
        kit.box(x, y, 0, 0.13, 2.02, bandD, dark);
        kit.box(x + side * 0.08, y + 0.04, 0, 0.04, 1.78, bandD - 0.08, pane);
        const mullions = Math.max(1, Math.floor(bandD / 1.65));
        for (let i = 0; i <= mullions; i++) {
          const z = -bandD / 2 + (bandD * i) / mullions;
          kit.box(x + side * 0.13, y + 0.04, z, 0.045, 1.8, 0.055, trim);
        }
        kit.box(x + side * 0.11, y - 1.05, 0, 0.16, 0.2, bandD, paleConcrete);
        continue;
      }
      for (let z = -d / 2 + 1.3; z < d / 2 - 0.6; z += 3.1) {
        kit.box(x, y, z, 0.13, 1.6, 1.5, dark);
        kit.box(x + side * 0.08, y, z, 0.04, 1.38, 1.28, pane);
        if (side === 1 && z > 0 && f < floors - 1 && b.kind !== 'office') {
          const stairWidth = Math.min(0.42, Math.max(0, (sideClearance - sideOffset - 0.1) * 0.72));
          if (stairWidth > 0.12) {
            const stairOffset = stairWidth / 2 + 0.04;
            kit.box(x + side * stairOffset, y - 0.92, z, stairWidth, 0.11, 1.5, concrete);
            kit.box(
              x + side * (stairOffset + stairWidth / 2 + 0.045),
              y + 0.18,
              z,
              0.05,
              1.05,
              1.5,
              iron,
            );
          }
        }
      }
    }
  }
}

export function flatRoofMembrane(art: ArtMaterials): THREE.Material {
  return art.get('#8b908b', 0.92);
}

export function renderFlatRoof(
  kit: Kit,
  m: {
    w: number;
    d: number;
    top: number;
    roof: THREE.Material;
    trim: THREE.Material;
    concrete: THREE.Material;
    iron: THREE.Material;
    paleConcrete: THREE.Material;
    seed: number;
    art: ArtMaterials;
  },
) {
  const { w, d, top, trim, concrete, iron, paleConcrete, seed, art } = m;
  const roof = flatRoofMembrane(art);
  kit.box(0, top + 0.22, 0, w + 0.45, 0.42, d + 0.45, roof);
  // Keep the waterproof roof field visible. A full-width pale trim slab here
  // used to cover it, making every roof read as the same concrete platform.
  kit.box(0, top + 0.48, 0, w + 0.15, 0.1, d + 0.15, roof);
  for (const z of [-d / 2, d / 2]) kit.box(0, top + 0.82, z, w + 0.45, 0.7, 0.16, concrete);
  for (const x of [-w / 2, w / 2]) kit.box(x, top + 0.82, 0, 0.16, 0.7, d + 0.35, concrete);
  for (const z of [-d / 2, d / 2]) kit.box(0, top + 1.2, z, w + 0.45, 0.06, 0.23, trim);
  for (const x of [-w / 2, w / 2])
    kit.box(x, top + 1.2, 0, 0.23, 0.06, Math.max(0.1, d - 0.23), trim);
  renderRoofServices(kit, {
    w,
    d,
    top,
    seed,
    concrete,
    iron,
    paleConcrete,
    dark: art.get('#344346'),
  });
  // Aerial and rooftop laundry vary by use and seed.
  if (pick(seed >>> 8, 3) === 0) {
    const aerialX = -w / 2 + 0.2;
    const aerialW = Math.max(0.35, Math.min(1.8, w - 0.4));
    kit.box(aerialX, top + 1.75, d / 2 - 0.2, 0.08, 2.5, 0.08, iron);
    kit.box(aerialX + aerialW / 2, top + 2.95, d / 2 - 0.2, aerialW, 0.07, 0.07, iron);
  }
  if (pick(seed >>> 10, 4) === 1) {
    const laundry = art.get('#e7e0d1');
    const width = Math.max(0.24, Math.min(0.45, (w - 0.3) / 3));
    const spacing = (w - width) / 4;
    for (let i = 0; i < 3; i++)
      kit.box(-w / 2 + spacing * (i + 1), top + 1.15, d / 2 - 0.16, width, 0.78, 0.06, laundry);
  }
}

export function renderExternalUnits(
  kit: Kit,
  m: {
    w: number;
    d: number;
    top: number;
    floors: number;
    rearClearance?: number;
    concrete: THREE.Material;
    iron: THREE.Material;
    dark: THREE.Material;
    paleConcrete: THREE.Material;
  },
) {
  const { w, d, top, floors, concrete, iron, dark, paleConcrete } = m;
  // Three low-noise AC clusters, each with a grille, pipe and vibration stand.
  const count = floors > 4 ? 2 : 1;
  // Keep the casing, grille, stand and pipe together, with a gap behind the wall.
  // Only their depth adapts; full-size units remain unchanged where they fit.
  const depthScale = Math.min(1, Math.max(0, (m.rearClearance ?? Infinity) - 0.02) / 0.6475);
  const rear = (distance: number) => -d / 2 - distance * depthScale;
  for (let i = 0; i < count; i++) {
    const z = rear(0.32);
    const x = (i - (count - 1) / 2) * Math.min(w * 0.4, 3.2);
    const y = baseValue(top, floors, i);
    kit.box(x, y, z, 1.05, 0.72, 0.5 * depthScale, paleConcrete);
    kit.box(x, y, z - 0.27 * depthScale, 0.68, 0.44, 0.055 * depthScale, dark);
    for (let slat = -2; slat <= 2; slat++)
      kit.box(x + slat * 0.1, y, z - 0.31 * depthScale, 0.035, 0.37, 0.035 * depthScale, iron);
    kit.box(x, y - 0.42, z, 1.24, 0.08, 0.62 * depthScale, iron);
    kit.box(x + 0.65, y + 0.25, z + 0.04 * depthScale, 0.08, 0.5, 0.08 * depthScale, iron);
  }
  kit.box(w / 2 + 0.08, top * 0.68, -d * 0.18, 0.13, top * 0.64, 0.13, iron);
  kit.box(w / 2 + 0.22, top * 0.8, -d * 0.18, 0.18, 0.32, 0.2, concrete);
}
