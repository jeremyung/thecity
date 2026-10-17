import { buildApartmentBuilding, type ApartmentBuildOptions } from '../architecture';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../materials';
import { compactMidriseEnvelope, midriseMassing } from '../building-massing';
import { lowriseBuildingWidth, lowriseMassing } from '../lowrise-massing';
import { buildingPalette } from '../building-palette';
import { broadHomeMassing } from '../broad-home-massing';
import { renderTower, towerMassing } from '../tower-kit';
import { renderDemolition } from '../demolition-kit';
import { type SignageContext } from '../signage';
import { claddingFor, windowActivity, windowLived } from '../building-surfaces';
import { type Building } from '../../types';
import {
  Kit,
  finishBuilding,
  partyWallBodyDepth,
  partyWallBodyWidth,
  pick,
  renderRenovationScaffold,
  zForSide,
} from './core';
import {
  renderGabledRoof,
  renderHouseFacade,
  renderLowriseConstruction,
  renderLowriseOpen,
  renderLowriseRenovationScaffold,
} from './lowrise';
import {
  renderExternalUnits,
  renderFlatRoof,
  renderMassingConstruction,
  renderMidriseEntry,
  renderMidriseOpen,
  renderMidriseRenovationScaffold,
  renderUrbanFacade,
} from './midrise';
import { renderCommercialSignage, renderShopFront, renderStreetLife } from './storefront';

export interface BuildingKitReviewOptions {
  compactMidrise?: boolean;
  apartmentComponents?: boolean | ApartmentBuildOptions;
  /** Station proximity and any podium display chosen for this building. */
  signage?: SignageContext;
}

export function buildingKit(b: Building, art: ArtMaterials, review: BuildingKitReviewOptions = {}) {
  if (review.apartmentComponents) {
    const group = buildApartmentBuilding(
      b,
      art,
      review.apartmentComponents === true ? {} : review.apartmentComponents,
    );
    if (group) return group;
  }
  const seed = b.appearance_seed >>> 0;
  // Master-plan lots tile blocks without gaps, so bodies run to the shared lot
  // lines (party walls) and keep only a shallow front setback and rear yard.
  const initialWidth = partyWallBodyWidth(b.width, b.anchor, b.floors, b.kind);
  const initialDepth = partyWallBodyDepth(b.depth),
    floor = 3.25;
  const envelope = review.compactMidrise
    ? compactMidriseEnvelope(b, initialWidth, initialDepth)
    : { width: initialWidth, depth: initialDepth };
  const d = envelope.depth;
  const w = lowriseBuildingWidth(b, envelope.width, d);
  const floors = Math.max(1, b.floors);
  const broadHome = broadHomeMassing(b, w, d);
  const lowrise = broadHome ? null : lowriseMassing(b, w, d);
  const kit = new Kit();
  kit.windowContext = { seed, lived: windowLived(b) };
  const height = Math.max(3.25, floors * floor);
  const base = 0.58,
    top = base + height;
  const isShop = b.kind === 'shop' || b.kind === 'cafe';
  const isHome = b.kind === 'home';
  const isApartment = b.kind === 'apartment';
  const lightUsage =
    b.kind === 'office' ? 'office' : isHome || isApartment ? 'home' : isShop ? 'shop' : 'ambient';
  const facadePalette = buildingPalette(b, art.palette);
  const wall = art.get(facadePalette.wall, 0.85, false, 'ambient', claddingFor(b));
  const concrete = art.get('#aaa99e'),
    paleConcrete = art.get('#c3b9a5');
  const roof = art.get(['#48575a', '#59676a', '#555d60', '#625d55'][pick(seed >>> 3, 4)]);
  const roofTile = art.get('#78817a'),
    trim = art.get(facadePalette.trim);
  const wood = art.get(['#644e3e', '#755d49', '#56483d'][pick(seed >>> 5, 3)]);
  const dark = art.get('#344346'),
    iron = art.get('#536164', 0.62);
  const glass = art.get('#55757a', 0.25),
    lightGlass = art.get('#94a7a0', 0.3);
  const glow = art.window(windowActivity(b.kind)),
    lantern = art.get('#ffbd62', 0.5, true, lightUsage);
  const green = art.get(
    b.anchor ? '#426c5c' : ['#527366', '#a4654f', '#526c76'][pick(seed >>> 7, 3)],
  );
  const ivory = art.get('#eee1c1'),
    scaffold = art.get('#8c8070', 0.72),
    caution = art.get('#d6b25e');
  const isOpen = b.stage === 'open';
  const renovating = b.stage === 'renovation';
  const foundation = 0.3;
  const midrise = broadHome ?? midriseMassing(b, w, d);

  // Occupied parcels have a small domestic/service edge rather than reading as
  // freestanding blocks on an uninterrupted lawn. Keep paths and planting
  // inside the cadastral footprint and leave the centered entry unobstructed.
  if (isOpen || renovating) {
    const paving = art.get(isHome ? '#a6a797' : '#b5b3a2');
    kit.box(0, 0.025, 0, b.width * 0.96, 0.05, b.depth * 0.96, paving);
    const margin = Math.max(0, (b.width - w) / 2 - 0.2);
    if ((isHome || isApartment) && margin > 0.55) {
      const garden = art.get('#748368'),
        hedge = art.get('#4d7057');
      const gardenX = w / 2 + margin / 2 + 0.12;
      const gardenD = d * (isHome ? 0.8 : 0.6);
      const side = pick(seed >>> 6, 2) ? 1 : -1;
      kit.box(side * gardenX, 0.065, -d * 0.08, margin, 0.08, gardenD, garden);
      kit.box(side * (b.width / 2 - 0.22), 0.26, -d * 0.08, 0.14, 0.45, gardenD + 0.25, concrete);
      for (let z = -gardenD / 2; z < gardenD / 2; z += 1.4) {
        const bush = new THREE.IcosahedronGeometry(1, 1);
        bush.scale(margin * 0.43, 0.38 + pick(seed + Math.round(z * 10), 3) * 0.09, 0.7);
        bush.translate(side * gardenX, 0.5, z - d * 0.08);
        kit.add(bush, hedge);
      }
    }
    // Fine paving joints establish scale without a texture repeating across
    // the whole city. The path remains aligned with the building's front.
    const apron = (b.depth - d) / 2;
    if (apron > 0.55) {
      for (let x = -w / 2; x < w / 2; x += 2.8) {
        kit.box(x, 0.058, d / 2 + apron / 2, 0.025, 0.008, apron * 0.8, concrete);
      }
    }
  }

  // Foundation plinth and entry steps keep the small buildings grounded.
  kit.box(
    0,
    foundation / 2,
    0,
    lowrise || broadHome ? Math.min(w + 0.65, b.width - 0.08) : w + 0.65,
    foundation,
    lowrise || broadHome ? Math.min(d + 0.65, b.depth - 0.08) : d + 0.65,
    concrete,
  );
  if (isShop || isHome) {
    const stepDepth = lowrise ? Math.min(0.8, Math.max(0.24, (b.depth - d) / 2 + 0.22)) : 0.8;
    const stepZ = lowrise
      ? Math.min(d / 2 + 0.35, b.depth / 2 - stepDepth / 2 - 0.04)
      : d / 2 + 0.35;
    kit.box(0, foundation + 0.08, stepZ, Math.min(w * 0.48, 3.7), 0.16, stepDepth, paleConcrete);
  }

  if (b.stage === 'demolition') {
    renderDemolition(
      kit,
      midrise?.masses ??
        (floors >= 12
          ? towerMassing(b, w, d)
          : [{ x: 0, z: 0, width: w, depth: d, baseFloor: 0, floors, role: 'main' }]),
      {
        progress: b.progress,
        floors,
        parcelW: b.width,
        parcelD: b.depth,
        wall,
        concrete,
        dark,
        scaffold,
        caution,
      },
    );
    return finishBuilding(kit, b, w, d, height, art, false);
  }

  if (!isOpen && !renovating) {
    const towerConstruction = floors >= 12 && (isApartment || b.kind === 'office');
    const constructionLayout =
      midrise ?? (towerConstruction ? { masses: towerMassing(b, w, d) } : null);
    if (constructionLayout) {
      const progress = THREE.MathUtils.clamp(b.progress, 0, 1);
      const builtFloors = Math.ceil(floors * THREE.MathUtils.clamp((progress - 0.12) / 0.57, 0, 1));
      const authoredMidrise = Boolean(midrise);
      renderMassingConstruction(kit, constructionLayout, {
        w,
        d,
        parcelW: b.width,
        base,
        floor,
        floors,
        builtFloors,
        progress,
        authoredEnvelope: towerConstruction || authoredMidrise,
        wall,
        concrete,
        iron,
        glass,
        dark,
        wood,
        scaffold,
        caution,
        paleConcrete,
      });
      if (towerConstruction) {
        const cladFloors = Math.min(
          builtFloors,
          Math.ceil(floors * THREE.MathUtils.clamp((progress - 0.69) / 0.23, 0, 1)),
        );
        if (cladFloors > 0) renderTower(kit, b, art, w, d, base, floor, cladFloors);
      } else if (midrise) {
        const cladFloors = Math.min(
          builtFloors,
          Math.ceil(floors * THREE.MathUtils.clamp((progress - 0.69) / 0.23, 0, 1)),
        );
        if (cladFloors > 0) {
          const roofFloor = Math.max(
            ...midrise.masses
              .filter((mass) => mass.role === 'main')
              .map((mass) => mass.baseFloor + mass.floors),
          );
          renderMidriseOpen(
            kit,
            b,
            art,
            midrise,
            {
              w,
              d,
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
              glow: glass,
            },
            cladFloors,
            cladFloors >= roofFloor,
          );
        }
      }
      return finishBuilding(kit, b, w, d, height, art, false);
    }
    if (lowrise) {
      renderLowriseConstruction(kit, b, art, lowrise, {
        w,
        d,
        base,
        top,
        floor,
        floors,
        parcelW: b.width,
        parcelD: b.depth,
        progress: b.progress,
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
      });
      return finishBuilding(kit, b, w, d, height, art, false);
    }
    if (b.kind === 'office' && !b.anchor && floors < 12) {
      // Small offices use the same bounded worksite as the tiered family.
      // The older detached-site scaffold/crane assumes several metres of yard.
      const progress = THREE.MathUtils.clamp(b.progress, 0, 1);
      const builtFloors = Math.ceil(floors * THREE.MathUtils.clamp((progress - 0.12) / 0.57, 0, 1));
      renderMassingConstruction(
        kit,
        { masses: [{ x: 0, z: 0, width: w, depth: d, baseFloor: 0, floors, role: 'main' }] },
        {
          w,
          d,
          parcelW: b.width,
          base,
          floor,
          floors,
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
        },
      );
      return finishBuilding(kit, b, w, d, height, art, false);
    }
    // A real work site: columns, partial floor plates and open scaffolding remain visible
    // until the envelope stage. Cladding arrives only near exterior completion.
    const progress = THREE.MathUtils.clamp(b.progress, 0, 1);
    const builtFloors = Math.max(
      0,
      Math.ceil(floors * THREE.MathUtils.clamp((progress - 0.12) / 0.57, 0, 1)),
    );
    const frameTop = Math.max(1.1, Math.min(height, builtFloors * floor));
    const columns = Math.max(1, Math.min(floors, builtFloors));
    for (let f = 0; f < columns; f++) {
      const y0 = base + f * floor;
      for (const x of [-w / 2 + 0.28, w / 2 - 0.28]) {
        for (const z of [-d / 2 + 0.28, d / 2 - 0.28]) {
          kit.box(x, y0 + floor / 2, z, 0.22, floor, 0.22, iron);
        }
      }
      kit.box(0, y0 + floor - 0.08, 0, w, 0.2, d, concrete);
    }
    // Exterior infill arrives as individual floor panels late in the build, leaving
    // openings and exposed columns readable until glazing and finish work begin.
    const installedFloors = Math.ceil(
      THREE.MathUtils.clamp((progress - 0.69) / 0.23, 0, 1) * floors,
    );
    for (let f = 0; f < installedFloors; f++) {
      const y = base + f * floor + 1.68;
      const panelH = 2.58;
      for (const side of [-1, 1]) {
        const z = side * (d / 2 - 0.08);
        kit.box(-w * 0.32, y, z, w * 0.34, panelH, 0.16, wall);
        kit.box(w * 0.32, y, z, w * 0.34, panelH, 0.16, wall);
        kit.box(0, y + 0.83, z, w * 0.3, 0.92, 0.16, wall);
        kit.box(-w * 0.17, y - 0.05, side * (d / 2 - 0.14), 0.08, panelH + 0.12, 0.1, wood);
        kit.box(w * 0.17, y - 0.05, side * (d / 2 - 0.14), 0.08, panelH + 0.12, 0.1, wood);
        if (progress > 0.84) {
          kit.box(0, y, side * (d / 2 + 0.015), w * 0.29, 1.2, 0.08, dark);
          kit.box(0, y, side * (d / 2 + 0.065), w * 0.26, 1.03, 0.04, glass);
        }
      }
      for (const side of [-1, 1]) {
        const x = side * (w / 2 - 0.08);
        kit.box(x, y, -d * 0.32, 0.16, panelH, d * 0.34, wall);
        kit.box(x, y, d * 0.32, 0.16, panelH, d * 0.34, wall);
        kit.box(side * (w / 2 + 0.015), y + 0.83, 0, 0.16, 0.92, d * 0.3, wall);
        if (progress > 0.84) {
          kit.box(side * (w / 2 + 0.03), y, 0, 0.08, 1.2, d * 0.29, dark);
          kit.box(side * (w / 2 + 0.08), y, 0, 0.04, 1.03, d * 0.26, glass);
        }
      }
    }
    // Scaffold rises in step with the frame; braces and planks read at miniature scale.
    const scaffoldTop = Math.max(2.5, Math.min(height + 0.8, frameTop + 1.3));
    for (const x of [-w / 2 - 0.62, w / 2 + 0.62]) {
      for (const z of [-d / 2 - 0.62, d / 2 + 0.62]) {
        kit.box(x, scaffoldTop / 2, z, 0.075, scaffoldTop, 0.075, scaffold);
      }
      for (let y = 1.15; y < scaffoldTop; y += 2.4) {
        kit.box(x, y, 0, 0.07, 0.07, d + 1.35, scaffold);
        kit.box(0, y, zForSide(x, w, d), w + 1.25, 0.07, 0.07, scaffold);
      }
    }
    for (let x = -w / 2; x <= w / 2; x += 2.2) {
      kit.box(x, 1.42, d / 2 + 2.25, 0.07, 2.8, 0.07, scaffold);
      kit.box(x, 2.72, d / 2 + 2.25, 0.07, 0.08, 0.07, caution);
    }
    kit.box(0, 0.88, d / 2 + 2.25, w + 1, 0.16, 0.16, scaffold);
    kit.box(0, 1.2, d / 2 + 2.27, w + 1, 0.32, 0.06, art.get('#c8c3b1'));
    // Timber packs, covered aggregate and a compact crane distinguish active sites.
    kit.box(-w * 0.24, 0.58, d / 2 + 1.05, Math.min(2.7, w * 0.3), 0.48, 0.72, wood);
    kit.box(w * 0.25, 0.72, -d / 2 - 0.9, 1.6, 0.7, 1.05, paleConcrete);
    if (floors > 2 || b.kind === 'office' || isApartment) {
      const mastX = w / 2 + 1.7;
      const mastH = Math.min(height + 5, Math.max(6, frameTop + 2.5));
      kit.box(mastX, mastH / 2, -d * 0.2, 0.24, mastH, 0.24, caution);
      kit.box(mastX - 0.15, mastH - 0.5, -d * 0.2, w * 0.68, 0.18, 0.2, caution);
      kit.box(mastX - w * 0.43, mastH - 0.75, -d * 0.2, 0.12, mastH * 0.33, 0.12, iron);
      kit.box(mastX + 0.6, 1.1, -d * 0.2, 0.8, 0.3, 0.7, art.get('#a95f48'));
    }
    return finishBuilding(kit, b, w, d, height, art, false);
  }

  const shellHeight = isHome && floors <= 2 ? height - 0.28 : height;
  const steppedApartment = isApartment && floors >= 5;

  if (floors >= 12 && (isApartment || b.kind === 'office')) {
    renderTower(kit, b, art, w, d, base, floor);
  } else if (midrise) {
    renderMidriseOpen(kit, b, art, midrise, {
      w,
      d,
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
    });
  } else if (lowrise) {
    renderLowriseOpen(kit, b, art, lowrise, {
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
      glow,
      lantern,
      seed,
    });
  } else if (steppedApartment) {
    // The top apartment floor pulls back on every side, leaving a clear roof terrace
    // line at miniature scale while keeping the same overall building height.
    const lowerHeight = (floors - 1) * floor;
    const capW = w * 0.84,
      capD = d * 0.84;
    kit.box(0, base + lowerHeight / 2, 0, w, lowerHeight, d, wall);
    renderUrbanFacade(kit, b, art, {
      w,
      d,
      base,
      top: base + lowerHeight,
      floor,
      floors: floors - 1,
      wood,
      dark,
      glass,
      lightGlass,
      trim,
      glow,
      iron,
      concrete,
      paleConcrete,
      isApartment: true,
    });
    kit.box(0, base + lowerHeight + floor / 2, 0, capW, floor, capD, wall);
    renderUrbanFacade(kit, b, art, {
      w: capW,
      d: capD,
      base: base + lowerHeight,
      top,
      floor,
      floors: 1,
      wood,
      dark,
      glass,
      lightGlass,
      trim,
      glow,
      iron,
      concrete,
      paleConcrete,
      isApartment: true,
    });
    renderFlatRoof(kit, {
      w: capW,
      d: capD,
      top,
      roof,
      trim,
      concrete,
      iron,
      paleConcrete,
      seed,
      art,
    });
    for (const x of [-capW / 2 + 0.24, capW / 2 - 0.24])
      kit.box(x, top / 2, capD / 2 + 0.055, 0.075, top, 0.075, iron);
    kit.box(0, top - 0.04, capD / 2 + 0.2, capW + 0.5, 0.12, 0.13, dark);
    kit.box(0, top - 0.04, -capD / 2 - 0.2, capW + 0.5, 0.12, 0.13, dark);
    if (isOpen || renovating)
      renderExternalUnits(kit, {
        w: capW,
        d: capD,
        top,
        floors: 1,
        concrete,
        iron,
        dark,
        paleConcrete,
      });
  } else {
    kit.box(0, base + shellHeight / 2, 0, w, shellHeight, d, wall);

    if (isShop) {
      renderShopFront(kit, b, art, {
        w,
        d,
        top,
        base,
        wood,
        dark,
        glass,
        trim,
        glow,
        green,
        ivory,
        lantern,
        concrete,
      });
    } else if (isHome && floors <= 2) {
      renderHouseFacade(kit, b, art, {
        w,
        d,
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
        ivory,
        concrete,
      });
    } else {
      // Tall homes reach this generic facade too; their authoritative front
      // access must terminate at a visible door, not a continuous window row.
      const compactUrbanEntry =
        (floors >= 2 && floors <= 3 && (isApartment || b.kind === 'office')) ||
        (isHome && floors >= 4) ||
        (b.kind === 'office' && !b.anchor && floors < 12);
      renderUrbanFacade(kit, b, art, {
        w,
        d,
        base,
        top,
        floor,
        floors,
        entryX: compactUrbanEntry ? 0 : undefined,
        parcelD: compactUrbanEntry ? b.depth : undefined,
        parcelW: compactUrbanEntry ? b.width : undefined,
        massOffsetZ: 0,
        massOffsetX: 0,
        wood,
        dark,
        glass,
        lightGlass,
        trim,
        glow,
        iron,
        concrete,
        paleConcrete,
        isApartment,
      });
      if (compactUrbanEntry) renderMidriseEntry(kit, 0, d, base, dark, glass, trim, concrete);
    }

    if (floors <= 3 && (isHome || isShop || (isApartment && pick(seed, 3) !== 0))) {
      renderGabledRoof(kit, w, d, top, roof, roofTile, trim, iron, dark, seed, b.width, wall);
    } else {
      renderFlatRoof(kit, { w, d, top, roof, trim, concrete, iron, paleConcrete, seed, art });
    }

    // Rain chains, downpipes, roof-edge gutters and small utility equipment.
    for (const x of [-w / 2 + 0.24, w / 2 - 0.24]) {
      kit.box(x, top / 2, d / 2 + 0.055, 0.075, top, 0.075, iron);
    }
    kit.box(0, top - 0.04, d / 2 + 0.2, w + 0.5, 0.12, 0.13, dark);
    kit.box(0, top - 0.04, -d / 2 - 0.2, w + 0.5, 0.12, 0.13, dark);
    if (isOpen || renovating)
      renderExternalUnits(kit, {
        w,
        d,
        top,
        floors,
        concrete,
        iron,
        dark,
        paleConcrete,
        rearClearance:
          b.kind === 'office' && !b.anchor && floors < 12 ? (b.depth - d) / 2 : undefined,
      });
  }

  // A few low-key signs of daily life appear only after a storefront opens.
  // Everything stays inside the parcel; the unfinished stages remain uncluttered.
  if (isOpen && (isShop || b.anchor)) {
    const frontage = lowrise?.masses.find((mass) => mass.role === 'frontage');
    const frontageEdge = frontage ? frontage.z + frontage.depth / 2 : undefined;
    renderStreetLife(kit, b, art, {
      w,
      d,
      parcelW: b.width,
      parcelD: b.depth,
      frontageEdge,
      wood,
      dark,
      green,
      paleConcrete,
      iron,
      trim,
      seed,
    });
  }

  if (isOpen && b.business_active !== false) {
    renderCommercialSignage(kit, b, art, {
      w,
      d,
      base,
      floor,
      height,
      frame: iron,
      midrise,
      lowrise,
      signage: review.signage,
    });
  }

  if (renovating) {
    if (midrise)
      renderMidriseRenovationScaffold(
        kit,
        midrise.masses,
        { height: Math.min(height, 6.5), floor },
        scaffold,
      );
    else if (lowrise)
      renderLowriseRenovationScaffold(
        kit,
        lowrise.masses,
        Math.min(height, 6.5),
        scaffold,
        caution,
        b.width,
        b.depth,
      );
    else if (b.kind === 'office' && !b.anchor && floors < 12)
      renderMidriseRenovationScaffold(
        kit,
        [{ x: 0, z: 0, width: w, depth: d, baseFloor: 0, floors, role: 'main' }],
        { height: Math.min(height, 6.5), floor },
        scaffold,
      );
    else renderRenovationScaffold(kit, w, d, Math.min(height, 6.5), scaffold, caution);
  }
  return finishBuilding(kit, b, w, d, height, art, true);
}

/**
 * Street-facing commercial signage for an open business: stacked blade signs
 * on offices and shop-houses, rooftop billboard frames on 4–11 floor
 * commercial roofs (likelier near the station), and the podium display the
 * city chose for a few station-area towers and offices.
 */
