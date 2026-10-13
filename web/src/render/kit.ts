import * as THREE from 'three/webgpu';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { ArtMaterials } from './materials';
import { compactMidriseEnvelope, midriseMassing, type BuildingMass, type MidriseMassing } from './building-massing';
import { lowriseBuildingWidth, lowriseMassing, type LowriseMass, type LowriseMassing } from './lowrise-massing';
import { buildingPalette } from './building-palette';
import { renderTower, towerMassing } from './tower-kit';
import { renderDemolition } from './demolition-kit';
import { addCeramicRoofDetail } from './ceramic-roof';
import { renderRoofServices } from './roof-services';
import { boxGeometry } from './box-geometry';
import type { Building } from '../types';

// Every repeatable detail is baked into one geometry per shared material.
export class Kit {
  private parts = new Map<THREE.Material, THREE.BufferGeometry[]>();
  private offset = new THREE.Vector3();

  withOffset<T>(x: number, y: number, z: number, build: () => T): T {
    const previous = this.offset;
    this.offset = previous.clone().add(new THREE.Vector3(x, y, z));
    try {
      return build();
    } finally {
      this.offset = previous;
    }
  }

  box(x: number, y: number, z: number, w: number, h: number, d: number, mat: THREE.Material, rotation = 0) {
    this.add(boxGeometry(x, y, z, w, h, d, rotation), mat);
  }
  cylinder(x: number, y: number, z: number, r: number, h: number, mat: THREE.Material, segments = 8) {
    const geo = new THREE.CylinderGeometry(r, r, h, segments);
    geo.translate(x, y, z);
    this.add(geo, mat);
  }
  cone(x: number, y: number, z: number, radius: number, h: number, mat: THREE.Material, segments = 6) {
    const geo = new THREE.ConeGeometry(radius, h, segments);
    geo.translate(x, y, z);
    this.add(geo, mat);
  }
  add(geo: THREE.BufferGeometry, mat: THREE.Material) {
    if (this.offset.lengthSq() > 0) geo.translate(this.offset.x, this.offset.y, this.offset.z);
    const list = this.parts.get(mat) ?? [];
    list.push(geo);
    this.parts.set(mat, list);
  }
  finish(cast = true) {
    const group = new THREE.Group();
    for (const [mat, parts] of this.parts) {
      const merged = mergeGeometries(parts, false);
      if (merged) {
        const mesh = new THREE.Mesh(merged, mat);
        mesh.castShadow = cast;
        mesh.receiveShadow = true;
        group.add(mesh);
      }
      parts.forEach(g => g.dispose());
    }
    this.parts.clear();
    return group;
  }
  /** Cancel unfinished geometry; materials remain owned by the shared palette. */
  dispose() {
    for (const parts of this.parts.values()) for (const geometry of parts) geometry.dispose();
    this.parts.clear();
  }
}

/**
 * Master-plan lots share lot lines, so the body keeps a fixed side margin for
 * eaves, sills and the foundation lip instead of a proportional inset.
 * Neighbours end up about 0.9 m apart, close to Tokyo's 0.5 m-per-side norm.
 * Depth keeps its proportional front setback and rear yard.
 */
export function partyWallBodyWidth(width: number, anchor = false, floors = 2, kind = 'home'): number {
  if (anchor || floors >= 14) return Math.max(6, width * (anchor ? .66 : .84));
  // Apartment and office families carry balconies and entry reveals.
  const lowrise = floors < 4 && (kind === 'home' || kind === 'shop' || kind === 'cafe');
  return Math.max(6, width - (lowrise ? .9 : 1.3));
}
export function partyWallBodyDepth(depth: number): number {
  return Math.max(6, depth * .84);
}

const pick = (seed: number, count: number) => Math.abs(Math.trunc(seed)) % count;

/** About half of non-anchor two- and three-floor homes receive restrained modern service details. */
export function isModernHomeVariant(b:Pick<Building,'kind'|'floors'|'appearance_seed'|'anchor'>):boolean {
  return !b.anchor&&b.kind==='home'&&(b.floors===2||b.floors===3)&&(((b.appearance_seed>>>2)&1)===0);
}

export interface BuildingKitReviewOptions { compactMidrise?: boolean }

export function buildingKit(b: Building, art: ArtMaterials, review: BuildingKitReviewOptions = {}) {
  const seed = b.appearance_seed >>> 0;
  // Master-plan lots tile blocks without gaps, so bodies run to the shared lot
  // lines (party walls) and keep only a shallow front setback and rear yard.
  const initialWidth = partyWallBodyWidth(b.width, b.anchor, b.floors, b.kind);
  const initialDepth = partyWallBodyDepth(b.depth), floor = 3.25;
  const envelope = review.compactMidrise ? compactMidriseEnvelope(b, initialWidth, initialDepth) : {width:initialWidth,depth:initialDepth};
  const d = envelope.depth;
  const w = lowriseBuildingWidth(b, envelope.width, d);
  const floors = Math.max(1, b.floors);
  const lowrise = lowriseMassing(b, w, d);
  const kit = new Kit();
  const height = Math.max(3.25, floors * floor);
  const base = .58, top = base + height;
  const isShop = b.kind === 'shop' || b.kind === 'cafe';
  const isHome = b.kind === 'home';
  const isApartment = b.kind === 'apartment';
  const lightUsage = b.kind === 'office' ? 'office' : (isHome || isApartment) ? 'home' : isShop ? 'shop' : 'ambient';
  const facadePalette=buildingPalette(b,art.palette);
  const wall = art.get(facadePalette.wall);
  const concrete = art.get('#aaa99e'), paleConcrete = art.get('#c3b9a5');
  const roof = art.get(['#48575a', '#59676a', '#555d60', '#625d55'][pick(seed >>> 3, 4)]);
  const roofTile = art.get('#78817a'), trim = art.get(facadePalette.trim);
  const wood = art.get(['#644e3e', '#755d49', '#56483d'][pick(seed >>> 5, 3)]);
  const dark = art.get('#344346'), iron = art.get('#536164', .62);
  const glass = art.get('#55757a', .25), lightGlass = art.get('#94a7a0', .3);
  const glow = art.get('#e9aa58', .42, true, lightUsage), lantern = art.get('#ffbd62', .5, true, lightUsage);
  const green = art.get(b.anchor ? '#426c5c' : ['#527366', '#a4654f', '#526c76'][pick(seed >>> 7, 3)]);
  const ivory = art.get('#eee1c1'), scaffold = art.get('#8c8070', .72), caution = art.get('#d6b25e');
  const isOpen = b.stage === 'open';
  const renovating = b.stage === 'renovation';
  const foundation = .3;
  const midrise = midriseMassing(b, w, d);

  // Occupied parcels have a small domestic/service edge rather than reading as
  // freestanding blocks on an uninterrupted lawn. Keep paths and planting
  // inside the cadastral footprint and leave the centered entry unobstructed.
  if (isOpen || renovating) {
    const paving = art.get(isHome ? '#a6a797' : '#b5b3a2');
    kit.box(0, .025, 0, b.width * .96, .05, b.depth * .96, paving);
    const margin = Math.max(0, (b.width - w) / 2 - .2);
    if ((isHome || isApartment) && margin > .55) {
      const garden = art.get('#748368'), hedge = art.get('#4d7057');
      const gardenX = w / 2 + margin / 2 + .12;
      const gardenD = d * (isHome ? .8 : .6);
      const side = pick(seed >>> 6, 2) ? 1 : -1;
      kit.box(side * gardenX, .065, -d * .08, margin, .08, gardenD, garden);
      kit.box(side * (b.width / 2 - .22), .26, -d * .08, .14, .45, gardenD + .25, concrete);
      for (let z = -gardenD / 2; z < gardenD / 2; z += 1.4) {
        const bush = new THREE.IcosahedronGeometry(1, 1);
        bush.scale(margin * .43, .38 + pick(seed + Math.round(z * 10), 3) * .09, .7);
        bush.translate(side * gardenX, .5, z - d * .08);
        kit.add(bush, hedge);
      }
    }
    // Fine paving joints establish scale without a texture repeating across
    // the whole city. The path remains aligned with the building's front.
    const apron = (b.depth - d) / 2;
    if (apron > .55) {
      for (let x = -w / 2; x < w / 2; x += 2.8) {
        kit.box(x, .058, d / 2 + apron / 2, .025, .008, apron * .8, concrete);
      }
    }
  }

  // Foundation plinth and entry steps keep the small buildings grounded.
  kit.box(0, foundation / 2, 0, lowrise ? Math.min(w + .65, b.width - .08) : w + .65, foundation, lowrise ? Math.min(d + .65, b.depth - .08) : d + .65, concrete);
  if (isShop || isHome) {
    const stepDepth = lowrise ? Math.min(.8, Math.max(.24, (b.depth - d) / 2 + .22)) : .8;
    const stepZ = lowrise ? Math.min(d / 2 + .35, b.depth / 2 - stepDepth / 2 - .04) : d / 2 + .35;
    kit.box(0, foundation + .08, stepZ, Math.min(w * .48, 3.7), .16, stepDepth, paleConcrete);
  }

  if (b.stage === 'demolition') {
    renderDemolition(kit, midrise?.masses ?? (floors >= 12 ? towerMassing(b, w, d) : [{ x: 0, z: 0, width: w, depth: d, baseFloor: 0, floors, role: 'main' }]), {
      progress: b.progress, floors, parcelW: b.width, parcelD: b.depth,
      wall, concrete, dark, scaffold, caution,
    });
    return finishBuilding(kit, b, w, d, height, art, false);
  }

  if (!isOpen && !renovating) {
    const towerConstruction = floors >= 12 && (isApartment || b.kind === 'office');
    const constructionLayout = midrise ?? (towerConstruction
      ? { masses: towerMassing(b, w, d) } : null);
    if (constructionLayout) {
      const progress = THREE.MathUtils.clamp(b.progress, 0, 1);
      const builtFloors = Math.ceil(floors * THREE.MathUtils.clamp((progress - .12) / .57, 0, 1));
      const authoredMidrise = Boolean(midrise);
      renderMassingConstruction(kit, constructionLayout, { w, d, parcelW: b.width, base, floor, floors, builtFloors, progress, authoredEnvelope: towerConstruction || authoredMidrise, wall, concrete, iron, glass, dark, wood, scaffold, caution, paleConcrete });
      if (towerConstruction) {
        const cladFloors = Math.min(builtFloors, Math.ceil(floors * THREE.MathUtils.clamp((progress - .69) / .23, 0, 1)));
        if (cladFloors > 0) renderTower(kit, b, art, w, d, base, floor, cladFloors);
      } else if (midrise) {
        const cladFloors = Math.min(builtFloors, Math.ceil(floors * THREE.MathUtils.clamp((progress - .69) / .23, 0, 1)));
        if (cladFloors > 0) {
          const roofFloor = Math.max(...midrise.masses.filter((mass) => mass.role === 'main').map((mass) => mass.baseFloor + mass.floors));
          renderMidriseOpen(kit, b, art, midrise, {
            w, d, base, floor, wall, roof, trim, concrete, paleConcrete, iron, dark, glass, lightGlass, wood, glow: glass,
          }, cladFloors, cladFloors >= roofFloor);
        }
      }
      return finishBuilding(kit, b, w, d, height, art, false);
    }
    if (lowrise) {
      renderLowriseConstruction(kit, b, art, lowrise, { w, d, base, top, floor, floors, parcelW:b.width, parcelD:b.depth, progress: b.progress, wall, roof, roofTile, trim, concrete, iron, scaffold, wood, paleConcrete, caution, dark, glass, lightGlass, green });
      return finishBuilding(kit, b, w, d, height, art, false);
    }
    // A real work site: columns, partial floor plates and open scaffolding remain visible
    // until the envelope stage. Cladding arrives only near exterior completion.
    const progress = THREE.MathUtils.clamp(b.progress, 0, 1);
    const builtFloors = Math.max(0, Math.ceil(floors * THREE.MathUtils.clamp((progress - .12) / .57, 0, 1)));
    const frameTop = Math.max(1.1, Math.min(height, builtFloors * floor));
    const columns = Math.max(1, Math.min(floors, builtFloors));
    for (let f = 0; f < columns; f++) {
      const y0 = base + f * floor;
      for (const x of [-w / 2 + .28, w / 2 - .28]) {
        for (const z of [-d / 2 + .28, d / 2 - .28]) {
          kit.box(x, y0 + floor / 2, z, .22, floor, .22, iron);
        }
      }
      kit.box(0, y0 + floor - .08, 0, w, .2, d, concrete);
    }
    // Exterior infill arrives as individual floor panels late in the build, leaving
    // openings and exposed columns readable until glazing and finish work begin.
    const installedFloors = Math.ceil(THREE.MathUtils.clamp((progress - .69) / .23, 0, 1) * floors);
    for (let f = 0; f < installedFloors; f++) {
      const y = base + f * floor + 1.68;
      const panelH = 2.58;
      for (const side of [-1, 1]) {
        const z = side * (d / 2 - .08);
        kit.box(-w * .32, y, z, w * .34, panelH, .16, wall);
        kit.box(w * .32, y, z, w * .34, panelH, .16, wall);
        kit.box(0, y + .83, z, w * .3, .92, .16, wall);
        kit.box(-w * .17, y - .05, side * (d / 2 - .14), .08, panelH + .12, .1, wood);
        kit.box(w * .17, y - .05, side * (d / 2 - .14), .08, panelH + .12, .1, wood);
        if (progress > .84) {
          kit.box(0, y, side * (d / 2 + .015), w * .29, 1.2, .08, dark);
          kit.box(0, y, side * (d / 2 + .065), w * .26, 1.03, .04, glass);
        }
      }
      for (const side of [-1, 1]) {
        const x = side * (w / 2 - .08);
        kit.box(x, y, -d * .32, .16, panelH, d * .34, wall);
        kit.box(x, y, d * .32, .16, panelH, d * .34, wall);
        kit.box(side * (w / 2 + .015), y + .83, 0, .16, .92, d * .3, wall);
        if (progress > .84) {
          kit.box(side * (w / 2 + .03), y, 0, .08, 1.2, d * .29, dark);
          kit.box(side * (w / 2 + .08), y, 0, .04, 1.03, d * .26, glass);
        }
      }
    }
    // Scaffold rises in step with the frame; braces and planks read at miniature scale.
    const scaffoldTop = Math.max(2.5, Math.min(height + .8, frameTop + 1.3));
    for (const x of [-w / 2 - .62, w / 2 + .62]) {
      for (const z of [-d / 2 - .62, d / 2 + .62]) {
        kit.box(x, scaffoldTop / 2, z, .075, scaffoldTop, .075, scaffold);
      }
      for (let y = 1.15; y < scaffoldTop; y += 2.4) {
        kit.box(x, y, 0, .07, .07, d + 1.35, scaffold);
        kit.box(0, y, zForSide(x, w, d), w + 1.25, .07, .07, scaffold);
      }
    }
    for (let x = -w / 2; x <= w / 2; x += 2.2) {
      kit.box(x, 1.42, d / 2 + 2.25, .07, 2.8, .07, scaffold);
      kit.box(x, 2.72, d / 2 + 2.25, .07, .08, .07, caution);
    }
    kit.box(0, .88, d / 2 + 2.25, w + 1, .16, .16, scaffold);
    kit.box(0, 1.2, d / 2 + 2.27, w + 1, .32, .06, art.get('#c8c3b1'));
    // Timber packs, covered aggregate and a compact crane distinguish active sites.
    kit.box(-w * .24, .58, d / 2 + 1.05, Math.min(2.7, w * .3), .48, .72, wood);
    kit.box(w * .25, .72, -d / 2 - .9, 1.6, .7, 1.05, paleConcrete);
    if (floors > 2 || b.kind === 'office' || isApartment) {
      const mastX = w / 2 + 1.7;
      const mastH = Math.min(height + 5, Math.max(6, frameTop + 2.5));
      kit.box(mastX, mastH / 2, -d * .2, .24, mastH, .24, caution);
      kit.box(mastX - .15, mastH - .5, -d * .2, w * .68, .18, .2, caution);
      kit.box(mastX - w * .43, mastH - .75, -d * .2, .12, mastH * .33, .12, iron);
      kit.box(mastX + .6, 1.1, -d * .2, .8, .3, .7, art.get('#a95f48'));
    }
    return finishBuilding(kit, b, w, d, height, art, false);
  }

  const shellHeight = isHome && floors <= 2 ? height - .28 : height;
  const steppedApartment = isApartment && floors >= 5;

  if (floors >= 12 && (isApartment || b.kind === 'office')) {
    renderTower(kit, b, art, w, d, base, floor);
  } else if (midrise) {
    renderMidriseOpen(kit, b, art, midrise, { w, d, base, floor, wall, roof, trim, concrete, paleConcrete, iron, dark, glass, lightGlass, wood, glow });
  } else if (lowrise) {
      renderLowriseOpen(kit, b, art, lowrise, {
      w, d, base, top, floor, wall, roof, roofTile, trim, wood, dark, glass, lightGlass, iron,
      green, concrete, paleConcrete, glow, lantern, seed,
    });
  } else if (steppedApartment) {
    // The top apartment floor pulls back on every side, leaving a clear roof terrace
    // line at miniature scale while keeping the same overall building height.
    const lowerHeight = (floors - 1) * floor;
    const capW = w * .84, capD = d * .84;
    kit.box(0, base + lowerHeight / 2, 0, w, lowerHeight, d, wall);
    renderUrbanFacade(kit, b, art, { w, d, base, top: base + lowerHeight, floor, floors: floors - 1, wood, dark, glass, lightGlass, trim, glow, iron, concrete, paleConcrete, isApartment: true });
    kit.box(0, base + lowerHeight + floor / 2, 0, capW, floor, capD, wall);
    renderUrbanFacade(kit, b, art, { w: capW, d: capD, base: base + lowerHeight, top, floor, floors: 1, wood, dark, glass, lightGlass, trim, glow, iron, concrete, paleConcrete, isApartment: true });
    renderFlatRoof(kit, { w: capW, d: capD, top, roof, trim, concrete, iron, paleConcrete, seed, art });
    for (const x of [-capW / 2 + .24, capW / 2 - .24]) kit.box(x, top / 2, capD / 2 + .055, .075, top, .075, iron);
    kit.box(0, top - .04, capD / 2 + .2, capW + .5, .12, .13, dark);
    kit.box(0, top - .04, -capD / 2 - .2, capW + .5, .12, .13, dark);
    if (isOpen || renovating) renderExternalUnits(kit, { w: capW, d: capD, top, floors: 1, concrete, iron, dark, paleConcrete });
  } else {
    kit.box(0, base + shellHeight / 2, 0, w, shellHeight, d, wall);

    if (isShop) {
      renderShopFront(kit, b, art, { w, d, top, base, wood, dark, glass, trim, glow, green, ivory, lantern, concrete });
    } else if (isHome && floors <= 2) {
      renderHouseFacade(kit, b, art, { w, d, base, top, floor, wood, dark, glass, lightGlass, trim, glow, iron, green, ivory, concrete });
    } else {
      const compactUrbanEntry = floors >= 2 && floors <= 3 && (isApartment || b.kind === 'office');
      renderUrbanFacade(kit, b, art, {
        w, d, base, top, floor, floors, entryX: compactUrbanEntry ? 0 : undefined,
        parcelD: compactUrbanEntry ? b.depth : undefined,
        parcelW: compactUrbanEntry ? b.width : undefined,
        massOffsetZ: 0, massOffsetX: 0,
        wood, dark, glass, lightGlass, trim, glow, iron, concrete, paleConcrete, isApartment,
      });
      if (compactUrbanEntry) renderMidriseEntry(kit, 0, d, base, dark, glass, trim, concrete);
    }

    if (floors <= 3 && (isHome || isShop || (isApartment && pick(seed, 3) !== 0))) {
      renderGabledRoof(kit, w, d, top, roof, roofTile, trim, iron, dark, seed, b.width);
    } else {
      renderFlatRoof(kit, { w, d, top, roof, trim, concrete, iron, paleConcrete, seed, art });
    }

    // Rain chains, downpipes, roof-edge gutters and small utility equipment.
    for (const x of [-w / 2 + .24, w / 2 - .24]) {
      kit.box(x, top / 2, d / 2 + .055, .075, top, .075, iron);
    }
    kit.box(0, top - .04, d / 2 + .2, w + .5, .12, .13, dark);
    kit.box(0, top - .04, -d / 2 - .2, w + .5, .12, .13, dark);
    if (isOpen || renovating) renderExternalUnits(kit, { w, d, top, floors, concrete, iron, dark, paleConcrete });
  }

  // A few low-key signs of daily life appear only after a storefront opens.
  // Everything stays inside the parcel; the unfinished stages remain uncluttered.
  if (isOpen && (isShop || b.anchor)) {
    const frontage=lowrise?.masses.find(mass=>mass.role==='frontage');
    const frontageEdge=frontage?frontage.z+frontage.depth/2:undefined;
    renderStreetLife(kit, b, art, { w, d, parcelW: b.width, parcelD: b.depth, frontageEdge, wood, dark, green, paleConcrete, iron, trim, seed });
  }

  if (isOpen && b.business_active !== false && b.kind === 'office' && floors >= 3 && floors < 16 && pick(seed >>> 4, 3) !== 0) {
    renderTenantBlade(kit, b, art, w, d, height, iron);
  }

  if (renovating) {
    if (midrise) renderMidriseRenovationScaffold(kit, midrise.masses, { height: Math.min(height, 6.5), floor }, scaffold);
    else if (lowrise) renderLowriseRenovationScaffold(kit, lowrise.masses, Math.min(height, 6.5), scaffold, caution, b.width, b.depth);
    else renderRenovationScaffold(kit, w, d, Math.min(height, 6.5), scaffold, caution);
  }
  return finishBuilding(kit, b, w, d, height, art, true);
}

/** Street-facing blades give commercial midrises a readable, narrow vertical accent. */
function renderTenantBlade(kit:Kit,b:Building,art:ArtMaterials,w:number,d:number,height:number,frame:THREE.Material){
  const depth=Math.min(.85,(b.depth-d)/2-.14);
  if(depth<.35)return;
  const signHeight=Math.min(depth*4.5,height-3.4);
  const x=w/2-.48,y=3.2+signHeight/2,z=d/2+.07+depth/2;
  kit.box(x,y,z,.19,signHeight+.12,depth+.06,frame);
  for(const end of [-1,1])kit.box(x,y+end*(signHeight/2-.24),d/2+.09,.13,.10,.28,frame);
  const slot=pick(b.appearance_seed>>>9,8);
  const material=art.tenantSign();
  for(const side of [-1,1]){
    const face=new THREE.PlaneGeometry(depth-.045,signHeight);
    const uv=face.getAttribute('uv');
    for(let i=0;i<uv.count;i++)uv.setY(i,(uv.getY(i)*.96+.02+7-slot)/8);
    face.rotateY(side*Math.PI/2);
    face.translate(x+side*.101,y,z);
    kit.add(face,material);
  }
}

type MidriseOpenMaterials = {
  w:number; d:number; base:number; floor:number; wall:THREE.Material; roof:THREE.Material; trim:THREE.Material;
  concrete:THREE.Material; paleConcrete:THREE.Material; iron:THREE.Material; dark:THREE.Material;
  glass:THREE.Material; lightGlass:THREE.Material; wood:THREE.Material; glow:THREE.Material;
};

function renderMidriseOpen(kit:Kit,b:Building,art:ArtMaterials,layout:MidriseMassing,m:MidriseOpenMaterials,installedThrough=Infinity,includeRoof=true){
  const {base,floor,wall,roof,trim,concrete,paleConcrete,iron,dark,glass,lightGlass,wood,glow}=m;
  const mains=layout.masses.filter(mass=>mass.role==='main');
  const roofMass=mains.reduce((a,c)=>a.baseFloor+a.floors>c.baseFloor+c.floors?a:c);

  for(const source of layout.masses){
    const admittedFloors=Math.min(source.floors,Math.max(0,installedThrough-source.baseFloor));
    if(admittedFloors<=0)continue;
    const mass={...source,floors:admittedFloors};
    const massBase=base+mass.baseFloor*floor;
    const massTop=base+(mass.baseFloor+mass.floors)*floor;
    kit.withOffset(mass.x,0,mass.z,()=>{
      kit.box(0,massBase+mass.floors*floor/2,0,mass.width,mass.floors*floor,mass.depth,wall);
      if(mass.role==='core'){
        renderMidriseCoreFacade(kit,{outerSide:mass.x<0?-1:1,width:mass.width,depth:mass.depth,base:massBase,floor,floors:mass.floors,wood,dark,glass,iron,concrete,trim});
        // A compact stair headhouse sits on the full-height circulation core.
        if (source.baseFloor + source.floors <= installedThrough) {
          kit.box(0,massTop+.16,0,mass.width*.72,.32,mass.depth*.7,trim);
          kit.box(0,massTop+.54,0,mass.width*.42,.48,mass.depth*.42,concrete);
        }
      }else{
        const core=layout.masses.find(candidate=>candidate.role==='core');
        const omitSideWindows=core&&layout.family==='walkup'?(core.x>mass.x?1:-1):undefined;
        const entryX=mass.baseFloor===0&&Math.abs(mass.x)<=mass.width/2?-mass.x:undefined;
        renderUrbanFacade(kit,b,art,{w:mass.width,d:mass.depth,base:massBase,top:massTop,floor,floors:mass.floors,baseFloor:mass.baseFloor,omitSideWindows,entryX,parcelD:b.depth,massOffsetZ:mass.z,parcelW:b.width,massOffsetX:mass.x,wood,dark,glass,lightGlass,trim,glow,iron,concrete,paleConcrete,isApartment:b.kind==='apartment'});
        // Only the podium/main facade owns the building entrance. Upper office
        // tiers use their global floor index and never repeat a ground trim.
        if(mass.baseFloor===0){
          const doorX=-mass.x;
          if(Math.abs(doorX)<mass.width*.32){
            renderMidriseEntry(kit,doorX,mass.depth,massBase,dark,glass,trim,concrete);
          }
        }
      }
    });
  }

  // Exposed terrace strips appear only where a main volume steps inward.
  for(let i=0;i<mains.length-1;i++){
    const lower=mains[i],upper=mains[i+1];
    if(lower.baseFloor+lower.floors!==upper.baseFloor || lower.baseFloor+lower.floors>installedThrough || upper.baseFloor+upper.floors>installedThrough)continue;
    const y=base+upper.baseFloor*floor-.045;
    const plate=.12, overlap=.08;
    kit.withOffset(lower.x,0,lower.z,()=>{
      for(const side of [-1,1]){
        kit.box(0,y,side*(lower.depth/2-plate/2),lower.width,plate,plate,concrete);
        kit.box(side*(lower.width/2-plate/2),y,0,plate,plate,lower.depth,concrete);
      }
      // Seeded slim rails stop short at the family corners, retaining a quiet
      // terrace line rather than outlining another full box.
      for(const side of [-1,1]){
        const z=side*(lower.depth/2-overlap);
        kit.box(0,y+.42,z,lower.width*.72,.055,.055,iron);
        for(const x of [-lower.width*.36,0,lower.width*.36])kit.box(x,y+.22,z,.045,.42,.045,iron);
      }
    });
  }

  // Roof equipment belongs to one highest main mass; the core receives only
  // its slim stair cap above. This avoids buried duplicate equipment at tiers.
  if(includeRoof){
    const roofTop=base+(roofMass.baseFloor+roofMass.floors)*floor;
    kit.withOffset(roofMass.x,0,roofMass.z,()=>renderFlatRoof(kit,{w:roofMass.width,d:roofMass.depth,top:roofTop,roof,trim,concrete,iron,paleConcrete,seed:b.appearance_seed,art}));
  }
}

function renderMidriseCoreFacade(kit:Kit,m:{outerSide:-1|1;width:number;depth:number;base:number;floor:number;floors:number;wood:THREE.Material;dark:THREE.Material;glass:THREE.Material;iron:THREE.Material;concrete:THREE.Material;trim:THREE.Material}){
  const {width,depth,base,floor,floors,wood,dark,glass,iron,concrete,trim}=m;
  const slotW=Math.max(.32,Math.min(.62,width*.38));
  for(let f=0;f<floors;f++){
    const y=base+f*floor+1.9;
    for(const side of [-1,1]){
      const z=side*(depth/2+.08);
      kit.box(0,y,z,slotW+.16,2.05,.13,dark);
      kit.box(0,y,z+side*.08,slotW,1.82,.04,glass);
      kit.box(0,y,z+side*.13,.055,1.84,.04,iron);
      kit.box(0,base+(f+1)*floor-.13,z+side*.08,width*.86,.14,.18,trim);
    }
    // The circulation core spans the full depth. Give its exposed flank a
    // restrained service-window rhythm; the attached apartment side stays solid.
    const outer=m.outerSide,x=outer*width/2;
    const bays=Math.max(1,Math.floor((depth-1.6)/5));
    for(let bay=0;bay<bays;bay++){
      const z=(bay+.5)*depth/bays-depth/2,landingY=base+f*floor+2.15;
      kit.box(x+outer*.045,landingY,z,.12,1.31,.84,dark);
      kit.box(x+outer*.12,landingY,z,.035,1.15,.68,glass);
      kit.box(x+outer*.15,landingY,z,.035,1.15,.045,trim);
    }
    if(f===0){
      for(const side of [-1,1]){
        const z=side*(depth/2+.11);
        kit.box(0,base+1.06,z,Math.min(1.1,width*.62),2.05,.16,dark);
        kit.box(0,base+1.06,z+side*.1,Math.min(.88,width*.5),1.78,.04,wood);
        kit.box(0,base+2.18,z+side*.17,Math.min(1.35,width*.82),.13,.42,concrete);
      }
    }
  }
}

function renderMidriseEntry(kit:Kit,doorX:number,depth:number,base:number,dark:THREE.Material,glass:THREE.Material,trim:THREE.Material,concrete:THREE.Material){
  kit.box(doorX,base+1.13,depth/2+.075,.98,2.18,.16,dark);
  kit.box(doorX,base+1.13,depth/2+.17,.76,1.92,.045,glass);
  kit.box(doorX+.29,base+1.12,depth/2+.205,.045,.1,.025,trim);
  kit.box(doorX,base+2.31,depth/2+.22,1.24,.13,.46,concrete);
}

function renderMassingConstruction(kit:Kit,layout:Pick<MidriseMassing,'masses'>,m:{w:number;d:number;parcelW:number;base:number;floor:number;floors:number;builtFloors:number;progress:number;authoredEnvelope?:boolean;wall:THREE.Material;concrete:THREE.Material;iron:THREE.Material;glass:THREE.Material;dark:THREE.Material;wood:THREE.Material;scaffold:THREE.Material;caution:THREE.Material;paleConcrete:THREE.Material}){
  const {base,floor,builtFloors,progress,wall,concrete,iron,glass,dark,wood,scaffold,caution,paleConcrete}=m;
  const installBudget=Math.ceil(THREE.MathUtils.clamp((progress-.69)/.23,0,1)*m.floors);
  for(const mass of layout.masses){
    // A tier cannot acquire a scaffold before work reaches its base level.
    if(mass.baseFloor>builtFloors)continue;
    const erected=Math.max(0,Math.min(mass.floors,builtFloors-mass.baseFloor));
    const massBase=base+mass.baseFloor*floor;
    const frameTop=massBase+erected*floor;
    const scaffoldTop=Math.min(massBase+mass.floors*floor+.45,Math.max(massBase+2.1,frameTop+1.05));
    kit.withOffset(mass.x,0,mass.z,()=>{
      const installed = m.authoredEnvelope ? Math.min(erected, Math.max(0, installBudget - mass.baseFloor)) : 0;
      for(let f=installed;f<erected;f++){
        const y0=massBase+f*floor;
        for(const x of [-mass.width/2+.18,mass.width/2-.18])for(const z of [-mass.depth/2+.18,mass.depth/2-.18]){
          kit.box(x,y0+floor/2,z,.18,floor,.18,iron);
        }
        kit.box(0,y0+floor-.08,0,mass.width,.16,mass.depth,concrete);
      }
      const cladFloors=Math.min(erected,Math.max(0,Math.min(mass.floors,installBudget-mass.baseFloor)));
      for(let f=0;f<(m.authoredEnvelope ? 0 : cladFloors);f++){
        const y=massBase+f*floor+1.62;
        const panelH=2.42;
        for(const side of [-1,1]){
          const z=side*(mass.depth/2-.07);
          kit.box(-mass.width*.3,y,z,mass.width*.38,panelH,.12,wall);
          kit.box(mass.width*.3,y,z,mass.width*.38,panelH,.12,wall);
          kit.box(0,y+.73,z,mass.width*.19,.72,.12,wall);
          if(progress>.84){kit.box(0,y,z+side*.075,mass.width*.25,1.05,.035,dark);kit.box(0,y,z+side*.1,mass.width*.22,.92,.025,glass);}
        }
        for(const side of [-1,1]){
          const x=side*(mass.width/2-.07);
          kit.box(x,y,-mass.depth*.28,.12,panelH,mass.depth*.38,wall);
          kit.box(x,y,mass.depth*.28,.12,panelH,mass.depth*.38,wall);
          kit.box(x+side*.075,y+.73,0,.035,.72,mass.depth*.19,wall);
        }
      }
      // Inset working scaffold stays within each authored mass outline.
      if(scaffoldTop>massBase+1.3){
        const insetX=Math.max(.12,mass.width/2-.17),insetZ=Math.max(.12,mass.depth/2-.17);
        for(const x of [-insetX,insetX]){
          for(const z of [-insetZ,insetZ])kit.box(x,(massBase+scaffoldTop)/2,z,.055,scaffoldTop-massBase,.055,scaffold);
          for(let y=massBase+1.15;y<scaffoldTop;y+=2.3)kit.box(x,y,0,.045,.055,mass.depth-.3,scaffold);
        }
        for(const z of [-insetZ,insetZ])for(let y=massBase+1.15;y<scaffoldTop;y+=2.3)kit.box(0,y,z,mass.width-.3,.055,.045,scaffold);
      }
    });
  }

  // Site logistics remain readable at every construction stage, including
  // foundation-only progress, and never exceed the supplied parcel envelope.
  const palletW=Math.min(1.7,m.w*.22),palletD=Math.min(1.2,m.d*.18);
  const palletX=Math.max(0,m.w/2-palletW/2-.25),palletZ=Math.max(0,m.d/2-palletD/2-.25);
  kit.box(-palletX,.5,-palletZ,palletW,.72,palletD,wood);
  kit.box(palletX,.4,palletZ,palletW*.85,.5,palletD*.8,paleConcrete);
  for(let i=-1;i<=1;i++)kit.box(-palletX+i*palletW*.28,.89,-palletZ,.035,.08,palletD,iron);

  const craneX=Math.min(m.w*.3,Math.max(.5,m.w/2-.35));
  const craneZ=-Math.min(m.d*.22,Math.max(.45,m.d/2-.35));
  const craneHeight=Math.min(m.floors*floor+4,Math.max(4.4,base+Math.max(erectedMax(layout,builtFloors),0)*floor+2.8));
  kit.box(craneX,craneHeight/2,craneZ,.16,craneHeight,.16,caution);
  const jibW=Math.min(Math.max(1.4,m.w*.58),2*(m.parcelW/2-craneX-.02));
  kit.box(craneX,craneHeight-.18,craneZ,jibW,.14,.15,caution);
  kit.box(craneX-jibW*.24,craneHeight-.65,craneZ,.1,1,.1,iron);
}

function erectedMax(layout:Pick<MidriseMassing,'masses'>,builtFloors:number){
  return layout.masses.reduce((max,mass)=>{
    const reached=Math.max(0,Math.min(mass.floors,builtFloors-mass.baseFloor));
    return Math.max(max,reached>0?mass.baseFloor+reached:0);
  },0);
}

function renderMidriseRenovationScaffold(kit:Kit,masses:BuildingMass[],m:{height:number;floor:number},material:THREE.Material){
  const seen=new Set<string>();
  for(const mass of masses){
    // The initial renovation works from street level; elevated tiers remain
    // quiet until a distinct upper-floor access scaffold is authored.
    if(mass.baseFloor>0)continue;
    const h=Math.min(m.height, mass.floors*m.floor, Math.max(2.6,m.height-mass.baseFloor*m.floor));
    if(h<=0)continue;
    const key=`${mass.x.toFixed(2)}:${mass.z.toFixed(2)}:${mass.width.toFixed(2)}:${mass.depth.toFixed(2)}`;
    if(seen.has(key))continue;seen.add(key);
    kit.withOffset(mass.x,0,mass.z,()=>{
      const x=Math.max(.1,mass.width/2-.16),z=Math.max(.1,mass.depth/2-.16);
      for(const xx of [-x,x])for(const zz of [-z,z])kit.box(xx,h/2,zz,.06,h,.06,material);
      for(let y=1.2;y<h;y+=2.35){
        for(const xx of [-x,x])kit.box(xx,y,0,.045,.055,mass.depth-.28,material);
        for(const zz of [-z,z])kit.box(0,y,zz,mass.width-.28,.055,.045,material);
      }
    });
  }
}

function zForSide(x: number, w: number, d: number) {
  return x < 0 ? d / 2 + .62 : -d / 2 - .62;
}


type LowriseOpenMaterials = {
  w:number; d:number; base:number; top:number; floor:number; wall:THREE.Material; roof:THREE.Material;
  roofTile:THREE.Material; trim:THREE.Material; wood:THREE.Material; dark:THREE.Material; glass:THREE.Material;
  lightGlass:THREE.Material; iron:THREE.Material; green:THREE.Material; concrete:THREE.Material;
  paleConcrete:THREE.Material; glow:THREE.Material; lantern:THREE.Material; seed:number;
};

function renderLowriseOpen(kit:Kit,b:Building,art:ArtMaterials,layout:LowriseMassing,m:LowriseOpenMaterials){
  const {base,floor,wall,roof,roofTile,trim,wood,dark,glass,lightGlass,iron,green,concrete,paleConcrete,glow,lantern,seed}=m;
  const frontage=layout.masses.find(mass=>mass.role==='frontage')!;
  const service=layout.masses.find(mass=>mass.role==='service')!;
  const top=base+frontage.floors*floor;
  kit.withOffset(frontage.x,0,frontage.z,()=>{
    // Hold the solid body back by a shallow facade reveal. The authored front
    // elevation can sit in this reveal while the frontage still reaches the lot line.
    kit.box(0,base+frontage.floors*floor/2,-.06,frontage.width,frontage.floors*floor,frontage.depth-.12,wall);
    if(b.kind==='home'){
      renderHouseFacade(kit,b,art,{w:frontage.width,d:frontage.depth,base,top,floor,wood,dark,glass,lightGlass,trim,glow,iron,green,ivory:art.get('#eee1c1'),concrete,entrySide:1,quietSides:layout.quietSides,frontOnly:true,centerEntry:true,lowriseDetail:true,frontBoundary:frontage.depth/2,frontWidthBoundary:b.width/2,sideBoundary:b.width/2-Math.abs(frontage.x)});
      if(isModernHomeVariant(b))renderModernHomeRearWindows(kit,frontage,{base,floor,dark,lightGlass,trim,concrete});
    }else{
      renderShopFront(kit,b,art,{w:frontage.width,d:frontage.depth,top,base,wood,dark,glass,trim,glow,green,ivory:art.get('#eee1c1'),lantern,concrete,quietSides:layout.quietSides,frontBoundary:frontage.depth/2,frontWidthBoundary:b.width/2,sideBoundary:b.width/2-Math.abs(frontage.x),centerDoor:true});
    }
  });
  renderLowriseRoof(kit,frontage,top,roof,roofTile,trim,iron,dark,seed,b.width,b.depth);

  // A single-storey rear utility volume joins the main wall but keeps its own
  // roofline; its louver and service door sit on the quiet rear elevation.
  const serviceWall=art.get(['#a99983','#a5a08e','#9eaa9d'][pick(seed>>>8,3)]);
  const serviceTop=base+service.floors*floor;
  kit.box(service.x,base+service.floors*floor/2,service.z,service.width,service.floors*floor,service.depth,serviceWall);
  const rearZ=service.z-service.depth/2+.025;
  kit.box(service.x,base+1.04,rearZ,.82,1.86,.07,dark);
  kit.box(service.x,base+1.04,rearZ-.035,.64,1.62,.035,wood);
  kit.box(service.x+service.width*.27,base+2.18,rearZ,.42,.34,.055,iron);
  kit.box(service.x+service.width*.27,base+2.18,rearZ-.035,.28,.2,.035,dark);
  for(let slat=-1;slat<=1;slat++)kit.box(service.x+service.width*.27+slat*.085,base+2.18,rearZ-.06,.025,.17,.025,trim);
  if(isModernHomeVariant(b))renderModernHomeCondenser(kit,service,layout.quietSides[0],base,paleConcrete,dark,iron,trim);
  renderLowriseRoof(kit,service,serviceTop,roof,roofTile,trim,iron,dark,seed>>>3,b.width,b.depth);

  // A shallow eave marks the frontage without adding another full-height
  // corner element to the compact silhouette.
  if(b.kind==='home'){
    const eave=frontage.z+frontage.depth/2-.05;
    kit.box(0,top-.12,eave,.12,.1,.1,wood);
  }
  void paleConcrete;void glow;void lantern;
}

/** Compact rear utility windows add scale to selected homes without repeating the front composition. */
function renderModernHomeRearWindows(kit:Kit,mass:LowriseMass,m:{base:number;floor:number;dark:THREE.Material;lightGlass:THREE.Material;trim:THREE.Material;concrete:THREE.Material}){
  const {base,floor,dark,lightGlass,trim,concrete}=m;
  const rear= -mass.depth/2;
  const windowW=Math.min(1.38,mass.width*.16),windowH=1.16;
  for(let level=1;level<mass.floors;level++){
    const y=base+level*floor+1.62;
    for(const x of [-mass.width*.22,mass.width*.22]){
      // Rearward is negative Z. The pane stands clearly outside the dark frame,
      // while the service-wing roof remains well below the first upper window.
      kit.box(x,y,rear-.015,windowW+.14,windowH+.16,.13,dark);
      kit.box(x,y,rear-.095,windowW,windowH,.035,lightGlass);
      kit.box(x,y,rear-.12,.045,windowH,.025,trim);
      kit.box(x,y-windowH/2-.1,rear-.1,windowW+.22,.1,.22,concrete);
    }
  }
}

/** An attached split-system condenser and short line set on the quiet service side. */
function renderModernHomeCondenser(kit:Kit,service:LowriseMass,side:-1|1,base:number,pale:THREE.Material,dark:THREE.Material,iron:THREE.Material,trim:THREE.Material){
  const centerX=service.x+side*(service.width/2+.36),centerY=base+1.3,centerZ=service.z+service.depth*.16;
  // Its inner edge touches the service wall; the outer edge stays inside the lot.
  kit.box(centerX,centerY,centerZ,.72,.56,.48,pale);
  kit.box(centerX+side*.37,centerY,centerZ,.025,.38,.38,dark);
  for(let slat=-2;slat<=2;slat++)kit.box(centerX+side*.39,centerY+slat*.065,centerZ,.025,.025,.32,iron);
  // Two small wall brackets support the unit without suggesting a balcony deck.
  for(const dz of [-.16,.16])kit.box(centerX-side*.19,base+1.02,centerZ+dz,.36,.055,.055,iron);
  const pipeX=service.x+side*(service.width/2+.035),pipeZ=centerZ-.39;
  kit.cylinder(pipeX,base+2.03,pipeZ,.022,1.08,trim,6);
  kit.box(pipeX,base+1.5,centerZ-.3,.045,.045,.18,trim);
}

function renderLowriseRoof(kit:Kit,mass:LowriseMass,top:number,roof:THREE.Material,tiles:THREE.Material,trim:THREE.Material,iron:THREE.Material,dark:THREE.Material,seed:number,parcelW:number,parcelD:number){
  kit.withOffset(mass.x,0,mass.z,()=>{
    if(mass.roof==='gable'){
      renderLowriseGable(kit,mass,top,roof,tiles,trim,iron,dark,seed,parcelW/2-Math.abs(mass.x),parcelD/2-mass.z,parcelD/2+mass.z);
    }else if(mass.roof==='shed'){
      const slope=new THREE.BoxGeometry(Math.min(mass.width-.24,2*(parcelW/2-Math.abs(mass.x))),.14,Math.min(mass.depth-.24,parcelD/2-mass.z+parcelD/2+mass.z));
      slope.rotateX(.075);
      slope.translate(0,top+.11,0);
      kit.add(slope,roof);
      kit.box(0,top+.2,0,Math.min(mass.width*.9,2*(parcelW/2-Math.abs(mass.x))),.08,.12,trim);
    }else{
      kit.box(0,top+.08,0,mass.width-.1,.16,mass.depth-.1,roof);
      for(const z of [-mass.depth/2+.09,mass.depth/2-.09])kit.box(0,top+.28,z,mass.width-.12,.4,.12,trim);
      for(const x of [-mass.width/2+.09,mass.width/2-.09])kit.box(x,top+.28,0,.12,.4,mass.depth-.12,trim);
      // A small raised roof hatch breaks the flat parapet without implying an
      // additional occupied floor.
      const hatchW=Math.min(1.1,mass.width*.24),hatchD=Math.min(.9,mass.depth*.2);
      kit.box(-mass.width*.18,top+.3,-mass.depth*.15,hatchW,.28,hatchD,dark);
    }
  });
}

function renderLowriseGable(kit:Kit,mass:LowriseMass,top:number,roof:THREE.Material,tiles:THREE.Material,trim:THREE.Material,iron:THREE.Material,dark:THREE.Material,seed:number,halfX:number,frontLimit:number,backLimit:number){
  const half=Math.min(mass.depth/2+.16,frontLimit,backLimit);
  const roofW=Math.min(mass.width+.32,Math.max(.4,halfX*2));
  const rise=Math.min(1.15,half*.32),angle=Math.atan2(rise,half),slopeLength=Math.hypot(half,rise);
  for(const side of [-1,1]){
    const panel=new THREE.BoxGeometry(roofW,.18,slopeLength);
    panel.rotateX(side*angle);panel.translate(0,top+rise/2,side*half/2);kit.add(panel,roof);
    kit.box(0,top-.015,side*half,roofW,.13,.1,trim);
  }
  addCeramicRoofDetail(kit,{width:roofW,halfDepth:half,rise,top,roof,tile:roof});
  kit.box(0,top+rise+.12,0,roofW,.22,.22,roof);
  if(pick(seed,4)===0)kit.box(0,top+rise+.2,0,Math.min(.6,roofW*.2),.08,.28,tiles);
}

function renderLowriseConstruction(kit:Kit,b:Building,art:ArtMaterials,layout:LowriseMassing,m:{w:number;d:number;base:number;top:number;floor:number;floors:number;parcelW:number;parcelD:number;progress:number;wall:THREE.Material;roof:THREE.Material;roofTile:THREE.Material;trim:THREE.Material;concrete:THREE.Material;iron:THREE.Material;scaffold:THREE.Material;wood:THREE.Material;paleConcrete:THREE.Material;caution:THREE.Material;dark:THREE.Material;glass:THREE.Material;lightGlass:THREE.Material;green:THREE.Material}){
  const {w,d,base,top,floor,progress,wall,roof,roofTile,trim,concrete,iron,scaffold,wood,paleConcrete,caution,dark,glass,lightGlass,green}=m;
  const amount=Number.isFinite(progress)?THREE.MathUtils.clamp(progress,0,1):0;
  const builtFloors=Math.ceil(m.floors*THREE.MathUtils.clamp((amount-.12)/.57,0,1));
  const installed=Math.ceil(THREE.MathUtils.clamp((amount-.68)/.25,0,1)*builtFloors);
  // The last cladding pass becomes the exact open-stage facade and roof family.
  // Keep a low work scaffold and grounded supplies visible, but do not stack a
  // second partial shell or activate commercial lighting before the building opens.
  if(amount>=.86){
    renderLowriseOpen(kit,b,art,layout,{w,d,base,top,floor,wall,roof,roofTile,trim,wood,dark,glass,lightGlass,iron,green,concrete,paleConcrete,glow:art.get('#e9aa58',.42),lantern:art.get('#ffbd62',.5),seed:b.appearance_seed>>>0});
    renderLowriseWorksite(kit,layout,{base,floor,builtFloors,parcelW:m.parcelW,parcelD:m.parcelD,wood,paleConcrete,caution,scaffold});
    return;
  }
  for(const mass of layout.masses){
    const levels=Math.min(mass.floors,builtFloors);
    const frameHeight=levels*floor;
    if(levels>0){
      for(let level=0;level<levels;level++){
        const y0=base+level*floor;
        for(const x of [mass.x-mass.width/2+.14,mass.x+mass.width/2-.14])for(const z of [mass.z-mass.depth/2+.14,mass.z+mass.depth/2-.14])
          kit.box(x,y0+floor/2,z,.18,floor,.18,iron);
        kit.box(mass.x,y0+floor-.09,mass.z,mass.width,.18,mass.depth,concrete);
        if(level<installed){
          // Cladding stays in the same mass footprint and leaves the frontage
          // doorway visibly open while the frame is still a work site.
          for(const side of [-1,1]){
            const z=mass.z+side*(mass.depth/2-.08);
            kit.box(mass.x-mass.width*.34,y0+floor*.56,z,mass.width*.28,floor*.72,.12,wall);
            kit.box(mass.x+mass.width*.34,y0+floor*.56,z,mass.width*.28,floor*.72,.12,wall);
            kit.box(mass.x,y0+floor*.85,z,mass.width*.65,.12,.12,wood);
          }
          for(const side of [-1,1]){
            const x=mass.x+side*(mass.width/2-.07);
            kit.box(x,y0+floor*.5,mass.z,.12,floor*.78,mass.depth*.72,wall);
          }
        }
      }
    }
    const scaffoldTop=Math.max(2.3,base+frameHeight+.5);
    const halfX=Math.min(mass.width/2+.16,m.parcelW/2-Math.abs(mass.x)-.04);
    const halfZ=Math.min(mass.depth/2+.16,m.parcelD/2-Math.abs(mass.z)-.04);
    const xs=[mass.x-halfX,mass.x+halfX],zs=[mass.z-halfZ,mass.z+halfZ];
    for(const x of xs){
      for(const z of zs)kit.box(x,scaffoldTop/2,z,.055,scaffoldTop,.055,scaffold);
      for(let y=1.1;y<scaffoldTop;y+=2.1){
        kit.box(x,y,mass.z,.045,.045,halfZ*2,scaffold);
        kit.box(mass.x,y,x===xs[0]?zs[0]:zs[1],halfX*2,.045,.045,scaffold);
      }
    }
  }
  renderLowriseWorksite(kit,layout,{base,floor,builtFloors,parcelW:m.parcelW,parcelD:m.parcelD,wood,paleConcrete,caution,scaffold});
}

function renderLowriseWorksite(kit:Kit,layout:LowriseMassing,m:{base:number;floor:number;builtFloors:number;parcelW:number;parcelD:number;wood:THREE.Material;paleConcrete:THREE.Material;caution:THREE.Material;scaffold:THREE.Material}){
  const {base,floor,builtFloors,parcelW,parcelD,wood,paleConcrete,caution,scaffold}=m;
  // Grounded supplies and a tarp give the foundation stage a legible cause
  // without introducing a floating crane or material above the parcel.
  kit.box(-.8,.38,.1,1.35,.62,.7,wood);
  kit.box(.9,.28,-.1,1.25,.42,.82,paleConcrete);
  const frontage=layout.masses.find(mass=>mass.role==='frontage')!;
  kit.box(0,.72,Math.min(frontage.z+frontage.depth/2+.12,parcelD/2-.24),Math.min(1.7,parcelW-.5),.08,.4,caution);
  renderLowriseRenovationScaffold(kit,layout.masses,Math.min(6.5,base+builtFloors*floor+.4),scaffold,caution,parcelW,parcelD);
}

function renderLowriseRenovationScaffold(kit:Kit,masses:LowriseMass[],height:number,scaffold:THREE.Material,caution:THREE.Material,parcelW=Infinity,parcelD=Infinity){
  const bounds=masses.map(mass=>({minX:mass.x-mass.width/2,maxX:mass.x+mass.width/2,minZ:mass.z-mass.depth/2,maxZ:mass.z+mass.depth/2}));
  const minX=Math.min(...bounds.map(b=>b.minX)),maxX=Math.max(...bounds.map(b=>b.maxX));
  const minZ=Math.min(...bounds.map(b=>b.minZ)),maxZ=Math.max(...bounds.map(b=>b.maxZ));
  const h=Math.max(1.8,height);
  const left=Math.max(minX-.12,-parcelW/2+.04),right=Math.min(maxX+.12,parcelW/2-.04);
  const back=Math.max(minZ-.12,-parcelD/2+.04),front=Math.min(maxZ+.12,parcelD/2-.04);
  for(const x of [left,right])for(const z of [back,front])kit.box(x,h/2,z,.055,h,.055,scaffold);
  kit.box((left+right)/2,Math.min(1.45,h/2),front-.07,right-left,.12,.12,caution);
}

function renderShopDisplay(
  kit:Kit,
  zone:{left:number;right:number},
  m:{base:number;shopTop:number;frontZ:(offset:number,depth:number)=>number;dark:THREE.Material;glass:THREE.Material;wood:THREE.Material;trim:THREE.Material;ivory:THREE.Material;latticeCount?:number},
) {
  const {base,shopTop,frontZ,dark,glass,wood,trim,ivory}=m;
  const openingW=Math.max(.9,zone.right-zone.left),center=(zone.left+zone.right)/2;
  const displayWidth=openingW-.18;
  // Recessed glazing keeps a clear depth step behind the timber frame.
  kit.box(center,(base+shopTop)/2,frontZ(0,.13),openingW,shopTop-base,.13,dark);
  kit.box(center,(base+shopTop)/2,frontZ(.08,.035),displayWidth,shopTop-base-.18,.035,glass);
  // Repeated bays, a kickboard, and sill make each pane read as a shop window.
  const bayCount=Math.max(3,Math.ceil(displayWidth/1.6));
  kit.box(center,base+.27,frontZ(.12,.12),displayWidth,.54,.12,wood);
  kit.box(center,base+.57,frontZ(.23,.34),displayWidth+.08,.09,.34,trim);
  for(let bay=1;bay<bayCount;bay++){
    const x=center-displayWidth/2+displayWidth*bay/bayCount;
    kit.box(x,(base+shopTop)/2,frontZ(.13,.09),.065,shopTop-base,.09,wood);
  }
  kit.box(center,shopTop-.43,frontZ(.13,.09),displayWidth,.07,.09,wood);
  kit.box(center,shopTop+.61,frontZ(.13,.13),displayWidth,.42,.13,ivory);
  const latticeCount=m.latticeCount??Math.max(2,Math.floor(displayWidth/1.2));
  for(let index=0;index<=latticeCount;index++){
    const x=center-displayWidth/2+(displayWidth*index)/latticeCount;
    kit.box(x,shopTop+.61,frontZ(.22,.07),.055,.38,.07,wood);
  }
}

function renderShopFront(
  kit: Kit, b: Building, art: ArtMaterials,
  m: { w: number; d: number; top: number; base: number; wood: THREE.Material; dark: THREE.Material; glass: THREE.Material; trim: THREE.Material; glow: THREE.Material; green: THREE.Material; ivory: THREE.Material; lantern: THREE.Material; concrete: THREE.Material; quietSides?:Array<-1|1>; frontBoundary?:number; frontWidthBoundary?:number; sideBoundary?:number; centerDoor?:boolean },
) {
  const { w, d, top, base, wood, dark, glass, trim, glow, green, ivory, lantern, concrete } = m;
  renderSideElevation(kit,b,{w,d,base,dark,glass,trim,wood,concrete,quietSides:m.quietSides,sideBoundary:m.sideBoundary});
  const front = d / 2 + .08;
  const frontZ=(offset:number,depth:number)=>{
    if(m.frontBoundary===undefined)return front+offset;
    const face=m.frontBoundary-depth/2-.08+offset*.22;
    return Math.min(m.frontBoundary-depth/2,face);
  };
  const doorW = Math.min(1.35, w * .17);
  const doorX = m.centerDoor ? 0 : Math.min(w * .34,w/2-doorW/2-.12);
  const doorGap = .12;
  const displayZones = m.centerDoor
    ? [
      { left: -w * .42, right: doorX - doorW / 2 - doorGap },
      { left: doorX + doorW / 2 + doorGap, right: w * .42 },
    ]
    : [{ left: -w * .42, right: doorX - doorW / 2 - doorGap }];
  const shopTop = Math.min(3.05, top - .4);
  const isCafe = b.kind === 'cafe';
  // Display windows balance around the centered entry on low-rise shops, while
  // keeping the founding shop's established single-sided composition intact.
  for (const zone of displayZones) {
    renderShopDisplay(kit, zone, { base, shopTop, frontZ, dark, glass, wood, trim, ivory, latticeCount:m.centerDoor?undefined:Math.max(3,Math.floor(w/1.2)) });
    // One paper notice sits within an outer glazing bay, clear of entry and mullions.
    const displayWidth=zone.right-zone.left-.18;
    if(displayWidth>1.1){
      const bays=Math.max(3,Math.ceil(displayWidth/1.6)),bayWidth=displayWidth/bays;
      const width=Math.min(.55,bayWidth*.68),height=width*1.35;
      const face=new THREE.PlaneGeometry(width,height);
      const slot=(isCafe?0:2)+pick(b.appearance_seed>>>4,2),uv=face.getAttribute('uv');
      for(let i=0;i<uv.count;i++){uv.setX(i,((slot%2)+.03+uv.getX(i)*.94)/2);uv.setY(i,(1-Math.floor(slot/2)+.03+uv.getY(i)*.94)/2);}
      face.translate(zone.right-.09-bayWidth/2,base+1.2,frontZ(.08,.035)+.021);
      kit.add(face,art.storefrontPoster());
    }
  }
  const jambs = m.centerDoor
    ? [-w * .42, doorX - doorW / 2 - doorGap, doorX + doorW / 2 + doorGap, w * .42]
    : [displayZones[0].left, displayZones[0].right, doorX + doorW / 2 + doorGap];
  for (const x of jambs) {
    kit.box(x, (base + shopTop) / 2, frontZ(.15,.2), .16, shopTop - base + .12, .2, wood);
  }
  const doorZ = frontZ(.2,.22);
  if(m.centerDoor)kit.box(doorX,base+.07,frontZ(.18,.24),doorW+.2,.14,.24,concrete);
  kit.box(doorX, (base + shopTop) / 2, doorZ, doorW, shopTop - base, .22, wood);
  kit.box(doorX, (base + shopTop) / 2, frontZ(.33,.035), doorW - .17, shopTop - base - .2, .035, glow);
  kit.box(doorX - doorW * .32, (base + shopTop) / 2, frontZ(.37,.04), .045, shopTop - base - .5, .04, dark);
  kit.cylinder(doorX + doorW * .32, 1.45, frontZ(.39,.14), .07, .07, trim, 6);
  // Warm timber fascia, noren-inspired valance and striped canvas canopy.
  kit.box(0, shopTop + .28, frontZ(.22,.32), w * .98, .46, .32, wood);
  const canopyY = Math.min(3.7, top - .18);
  const canopyW = Math.min(isCafe ? w * .84 : w + .62,m.frontWidthBoundary===undefined?Infinity:2*m.frontWidthBoundary-.08);
  const canopyD = m.frontBoundary===undefined ? (isCafe ? .98 : 1.35) : Math.min(isCafe ? .98 : 1.35,Math.max(.6,d*.3));
  const canopyZ = m.frontBoundary===undefined ? d / 2 + (isCafe ? .53 : .72) : m.frontBoundary-canopyD/2;
  kit.box(0, canopyY, canopyZ, canopyW, .2, canopyD, green);
  kit.box(0, canopyY - .19, frontZ(canopyZ + canopyD/2-front,.12), canopyW, .24, .12, green);
  const stripe = art.get(isCafe ? '#e4d5b7' : '#ead6af');
  const stripeW = Math.max(.42, canopyW / (isCafe ? 11 : 9));
  for (let x = -canopyW / 2 + stripeW/2; x <= canopyW / 2 - stripeW/2; x += stripeW * 2) {
    kit.box(x, canopyY - .18, frontZ(canopyZ + canopyD/2-front+.02,.125), stripeW, .23, .125, stripe);
  }
  for (const x of [-canopyW / 2 + .2, canopyW / 2 - .2]) {
    kit.box(x, canopyY - .9, frontZ(canopyZ + canopyD * .18-front,.07), .07, 1.45, .07, wood);
  }
  // Lattice transom panels and paper lamps make the narrow frontage legible from above.
  const transomY = shopTop + .61;
  for (const zone of displayZones) {
    const width = Math.max(.5, zone.right - zone.left - .18);
    const center = (zone.left + zone.right) / 2;
    kit.box(center, transomY, frontZ(.13,.13), width, .42, .13, ivory);
    const latticeCount = Math.max(2, Math.floor(width / 1.2));
    for (let i = 0; i <= latticeCount; i++) {
      const x = center - width / 2 + (width * i) / latticeCount;
      kit.box(x, transomY, frontZ(.22,.07), .055, .38, .07, wood);
    }
  }
  kit.box(-w / 2 + .43, shopTop + .45, frontZ(.48,.28), .42, .68, .28, dark);
  kit.box(-w / 2 + .43, shopTop + .45, frontZ(.64,.06), .27, .48, .06, lantern);
  kit.box(w / 2 - .38, shopTop + .45, frontZ(.43,.24), .26, .72, .24, wood);
  kit.box(w / 2 - .38, shopTop + .45, frontZ(.57,.055), .12, .48, .055, glow);
  // Wall-mounted lanterns flank the entry under the eaves.
  for (const x of [-w / 2 + .44, w / 2 - .44]) {
    kit.box(x, shopTop + .95, frontZ(.22,.13), .13, .5, .13, dark);
    kit.box(x, shopTop + .89, frontZ(.29,.25), .25, .22, .25, lantern);
  }
  // Slim tiled side shutters and end pilasters.
  for (const x of [-w / 2 + .18, w / 2 - .18]) {
    kit.box(x, (base + top) / 2, frontZ(.1,.2), .24, top - base, .2, wood);
  }
  kit.box(0, base + .12, frontZ(.35,.24), w * .98, .16, .24, dark);
  for (let floorIndex = 1; floorIndex < b.floors; floorIndex++) {
    const y = base + floorIndex * 3.25 + 1.78;
    for (const x of [-w * .27, w * .25]) {
      kit.box(x, y, frontZ(-.02,.14), Math.min(2.5, w * .25), 1.52, .14, dark);
      kit.box(x, y, frontZ(.06,.04), Math.min(2.28, w * .23), 1.3, .04, ((seedInt(b.appearance_seed) + floorIndex + Math.round(x)) % 3) ? glass : glow);
      kit.box(x, y, frontZ(.13,.05), .06, 1.32, .05, trim);
      if (b.kind === 'apartment' || b.kind === 'home') {
        kit.box(x, y - .82, frontZ(.34,.62), Math.min(2.8, w * .28), .12, .62, concrete);
        kit.box(x, y - .24, frontZ(.62,.06), Math.min(2.8, w * .28), .06, .06, dark);
        for (const rx of [-.9, 0, .9]) kit.box(x + rx, y - .26, frontZ(.62,.05), .05, .66, .05, dark);
      }
    }
  }
}

/** Secondary elevations need the same inhabited scale as the shop front.
 * All details share existing materials and stay inside the normal roof overhang. */
function renderSideElevation(kit:Kit,b:Building,m:{w:number;d:number;base:number;dark:THREE.Material;glass:THREE.Material;trim:THREE.Material;wood:THREE.Material;concrete:THREE.Material;quietSides?:Array<-1|1>;sideBoundary?:number}){
  const {w,d,base,dark,glass,trim,wood,concrete,quietSides}=m;
  const seed=b.appearance_seed>>>0;
  for(const side of [-1,1]){
    if(quietSides?.includes(side as -1|1))continue;
    const sideX=(offset:number,width:number)=>{
      const desired=side*(w/2+offset);
      if(m.sideBoundary===undefined)return desired;
      return side>0?Math.min(desired,m.sideBoundary-width/2):Math.max(desired,-m.sideBoundary+width/2);
    };
    const x=sideX(.045,.13);
    for(let level=0;level<b.floors;level++){
      const y=base+level*3.25+1.72;
      for(const [index,z]of [-d*.24,d*.23].entries()){
        const width=Math.min(1.6,d*.24),height=level===0&&index===0?1.05:1.4;
        kit.box(x,y,z,.13,height+.18,width+.18,wood);
        kit.box(sideX(.08,.035),y,z,.035,height,width,glass);
        kit.box(sideX(.13,.035),y,z,.035,height,.055,trim);
        kit.box(sideX(.14,.3),y-height/2-.14,z,.3,.12,width+.3,concrete);
        kit.box(sideX(.2,.45),y+height/2+.2,z,.45,.1,width+.4,dark);
        if((seed+level+index+(side>0?1:0))%3===0){
          kit.box(sideX(.1,.1),y,z+width/2+.24,.1,height+.12,.32,wood);
          for(let slat=0;slat<4;slat++)kit.box(sideX(.165,.035),y-height*.35+slat*height*.23,z+width/2+.24,.035,.025,.28,trim);
        }
      }
    }
    // A narrow service strip visually grounds the elevation without adding lawn.
    kit.box(sideX(0,.17),base+.1,0,.17,.2,d*.92,concrete);
  }
}

function renderHouseFacade(
  kit: Kit, b: Building, _art: ArtMaterials,
  m: { w: number; d: number; base: number; top: number; floor: number; wood: THREE.Material; dark: THREE.Material; glass: THREE.Material; lightGlass: THREE.Material; trim: THREE.Material; glow: THREE.Material; iron: THREE.Material; green: THREE.Material; ivory: THREE.Material; concrete: THREE.Material; entrySide?: -1 | 1; quietSides?:Array<-1|1>; frontOnly?:boolean; centerEntry?:boolean; lowriseDetail?:boolean; frontBoundary?:number; frontWidthBoundary?:number; sideBoundary?:number },
) {
  const { w, d, base, floor, wood, dark, glass, lightGlass, trim, glow, iron, green, ivory, concrete, entrySide = 1 } = m;
  const occupiedHome = b.stage === 'open' && b.occupancy > 0;
  const windowGlass = (floorIndex:number,windowIndex:number,windowCount:number,unlit:THREE.Material=lightGlass) => {
    if(!occupiedHome)return unlit;
    const seed=b.appearance_seed>>>0;
    // Keep one stable occupied window lit, then add only a sparse seeded subset.
    // Material assignment changes with occupancy; pane geometry stays fixed.
    const guaranteedWindow=seed%windowCount;
    const mask=Math.imul(seed ^ Math.imul(floorIndex+1,0x9e3779b9) ^ Math.imul(windowIndex+3,0x85ebca6b),0xc2b2ae35)>>>0;
    return floorIndex===0&&windowIndex===guaranteedWindow||mask%5===0?glow:unlit;
  };
  renderSideElevation(kit,b,{w,d,base,dark,glass,trim,wood,concrete,quietSides:m.quietSides,sideBoundary:m.sideBoundary});
  const fronts = m.frontOnly ? [1] : [1, -1];
  for (const side of fronts) {
    const z = side * (d / 2 + .08);
    const frontZ=(offset:number,depth:number)=>{
      if(side!==1||m.frontBoundary===undefined)return z+side*offset;
      const face=m.frontBoundary-depth/2-.08+offset*.22;
      return Math.min(m.frontBoundary-depth/2,face);
    };
    if (b.floors === 1) {
      // A modest machiya-like frontage: broad sliding panels, timber lintels and a porch.
      const doorX = m.centerEntry ? 0 : -entrySide * w * .25;
      const windowW = Math.min(2.35, w * .42);
      if(m.centerEntry){
        const sideWindowW=Math.min(1.3,Math.max(.6,(w-1.25-.7)/2));
        const windowX=.625+.2+sideWindowW/2;
        for(const [windowIndex,x] of [-windowX,windowX].entries()){
          const pane=windowGlass(0,windowIndex,2);
          if(m.lowriseDetail){
            renderLowriseHouseWindow(kit,{x,y:base+1.58,width:sideWindowW-.18,height:1.48,frontZ,pane,wood,dark,trim,concrete,horizontalSash:false});
          }else{
            kit.box(x,base+1.57,frontZ(0,.12),sideWindowW,1.75,.12,wood);
            kit.box(x,base+1.58,frontZ(.08,.035),sideWindowW-.18,1.48,.035,pane);
            for(let k=-1;k<=1;k++)kit.box(x+k*.42,base+1.58,frontZ(.14,.035),.045,1.48,.035,trim);
          }
        }
      }else{
        const windowX = entrySide * w * .19;
        kit.box(windowX, base + 1.57, frontZ(0,.12), windowW, 1.75, .12, wood);
        kit.box(windowX, base + 1.58, frontZ(.08,.035), windowW - .22, 1.48, .035, windowGlass(0,0,1,glass));
        for (let k = -1; k <= 1; k++) kit.box(windowX + k * .52, base + 1.58, frontZ(.14,.035), .045, 1.48, .035, trim);
      }
      // The door sits behind a deep timber frame, giving the narrow entry a
      // genuine shadow line rather than another flat facade panel.
      kit.box(doorX, base + 1.45, frontZ(.055,.12), 1.25, 2.52, .12, dark);
      kit.box(doorX, base + 1.45, frontZ(.11,.07), 1.0, 2.28, .07, wood);
      if(!m.centerEntry)kit.box(doorX, base + 1.45, frontZ(.16,.035), .76, 2.05, .035, lightGlass);
      if(m.centerEntry)kit.box(doorX,base+.07,frontZ(.18,.24),1.45,.14,.24,concrete);
      const porchW=Math.min(w+.7,m.frontWidthBoundary===undefined?Infinity:2*m.frontWidthBoundary-.08);
      kit.box(0, base + 2.58, frontZ(.15,.22), Math.min(w+.2,porchW), .24, .22, dark);
      kit.box(0, base + 2.92, frontZ(.54,1.05), porchW, .16, 1.05, wood);
      kit.box(0, base + 2.72, frontZ(1.02,.13), porchW, .34, .13, dark);
      kit.box(-w * .33, base + .4, frontZ(.42,.12), .12, .8, .12, wood);
      kit.box(w * .33, base + .4, frontZ(.42,.12), .12, .8, .12, wood);
    } else if (m.centerEntry && side === 1) {
      const doorW=Math.min(1.12,w*.24),doorH=Math.min(2.38,floor-.58),doorY=base+doorH/2+.16;
      kit.box(0,doorY,frontZ(.055,.11),doorW+.22,doorH+.18,.11,dark);
      kit.box(0,doorY,frontZ(.105,.06),doorW,doorH,.06,wood);
      kit.box(0,base+doorH+.24,frontZ(.11,.22),doorW+.62,.14,.22,green);
      kit.box(0,base+.07,frontZ(.18,.24),doorW+.2,.14,.24,concrete);
      const windowW=Math.min(1.42,Math.max(.72,(w-doorW-.7)/2));
      const windowX=doorW/2+.2+windowW/2;
      for(const [windowIndex,x] of [-windowX,windowX].entries()){
        const pane=windowGlass(0,windowIndex,2);
        if(m.lowriseDetail){
          renderLowriseHouseWindow(kit,{x,y:base+1.8,width:windowW-.18,height:1.34,frontZ,pane,wood,dark,trim,concrete,horizontalSash:true});
        }else{
          kit.box(x,base+1.8,frontZ(0,.1),windowW,1.55,.1,dark);
          kit.box(x,base+1.8,frontZ(.07,.035),windowW-.18,1.34,.035,pane);
          kit.box(x,base+1.8,frontZ(.12,.045),.045,1.36,.045,trim);
          kit.box(x,base+.91,frontZ(.24,.42),windowW+.12,.12,.42,concrete);
        }
      }
    } else {
      for (const x of [-w * .26, w * .24]) {
        kit.box(x, base + 1.9, z, Math.min(2.8, w * .3), 1.75, .13, dark);
        kit.box(x, base + 1.9, z + side * .08, Math.min(2.55, w * .27), 1.48, .04, glass);
        kit.box(x, base + 1.9, z + side * .15, .07, 1.5, .05, trim);
        kit.box(x, base + 1.9, z + side * .15, 2.5, .07, .05, trim);
        kit.box(x, base + .74, z + side * .35, 3.0, .13, .64, concrete);
        for (const railX of [x - 1.28, x + 1.28]) kit.box(railX, base + 1.12, z + side * .62, .055, .78, .055, iron);
        kit.box(x, base + 1.45, z + side * .62, 2.65, .06, .06, iron);
      }
    }
    for(let level=1;level<b.floors;level++){
      const y=base+level*floor+1.65;
      for(const [windowIndex,x] of [-w*.24,w*.24].entries()){
        const opening=Math.min(2.3,w*.32);
        const pane=windowGlass(level,windowIndex,2);
        if(m.lowriseDetail&&m.centerEntry&&side===1){
          renderLowriseHouseWindow(kit,{x,y,width:opening-.18,height:1.28,frontZ,pane,wood,dark,trim,concrete,horizontalSash:true});
        }else{
          kit.box(x,y,frontZ(0,.13),opening,1.46,.13,wood);
          kit.box(x,y,frontZ(.08,.035),opening-.18,1.28,.035,pane);
          kit.box(x,y,frontZ(.13,.035),.055,1.3,.035,trim);
          kit.box(x,y-.82,frontZ(.16,.32),opening+.2,.12,.32,concrete);
          kit.box(x,y+.83,frontZ(.22,.48),opening+.32,.1,.48,dark);
        }
        if(isModernHomeVariant(b)&&m.centerEntry&&side===1){
          // A shallow window guard gives only the modern-home subset a light
          // metal rhythm; it is mounted to the sill, not a walkable balcony.
          const guardWidth=opening-.3,guardZ=frontZ(.28,.045),guardBottom=y-.63,guardTop=y-.3;
          kit.box(x,(guardBottom+guardTop)/2,guardZ,guardWidth,.045,.045,iron);
          for(let bar=0;bar<5;bar++){
            const guardX=x-guardWidth/2+guardWidth*(bar+.5)/5;
            kit.box(guardX,(guardBottom+guardTop)/2,guardZ,.035,guardTop-guardBottom,.045,iron);
          }
        }
      }
    }
    // Eaves carry a thin wood beam and small downturned corner brackets.
    kit.box(0, base + 3.03, frontZ(.22,.28), Math.min(w+.48,m.frontWidthBoundary===undefined?Infinity:2*m.frontWidthBoundary-.08), .16, .28, wood);
    for (const x of [-w / 2 + .25, w / 2 - .25]) kit.box(x, base + 2.72, frontZ(.28,.12), .12, .48, .12, wood);
  }
  // A short side entry canopy with two brackets breaks up the plain end wall.
  kit.box(-w / 2 + .43, base + 2.3, 0, .82, .14, 2.4, green);
  for (const z of [-.86, .86]) kit.box(-w / 2 + .12, base + 1.58, z, .1, 1.42, .1, wood);
  void ivory;
}

/** A shallow recessed sash assembly for low-rise homes; every part stays inside the authored frontage. */
function renderLowriseHouseWindow(
  kit:Kit,
  m:{x:number;y:number;width:number;height:number;frontZ:(offset:number,depth:number)=>number;pane:THREE.Material;wood:THREE.Material;dark:THREE.Material;trim:THREE.Material;concrete:THREE.Material;horizontalSash:boolean},
){
  const {x,y,width,height,frontZ,pane,wood,dark,trim,concrete,horizontalSash}=m;
  const paneW=Math.max(.28,width),paneH=height;

  // Dark backing sits just ahead of the plaster wall. The lighter pane is
  // recessed behind the projecting timber sash, so each edge reads in relief.
  kit.box(x,y,frontZ(-.10,.045),paneW+.12,paneH+.12,.045,dark);
  kit.box(x,y,frontZ(.015,.035),paneW,paneH,.035,pane);

  const jamb=.075,rail=.075;
  const frameZ=frontZ(.16,.065);
  kit.box(x-paneW/2-jamb/2,y,frameZ,jamb,paneH+.14,.065,wood);
  kit.box(x+paneW/2+jamb/2,y,frameZ,jamb,paneH+.14,.065,wood);
  kit.box(x,y-paneH/2-rail/2,frameZ,paneW,rail,.065,wood);
  kit.box(x,y+paneH/2+rail/2,frameZ,paneW,rail,.065,wood);

  // Two sliding-sash rails give the glazing a readable domestic scale without
  // adding per-window objects or changing its silhouette.
  const sashZ=frontZ(.24,.035);
  kit.box(x,y,sashZ,.038,paneH,.035,trim);
  if(horizontalSash)kit.box(x,y,sashZ,paneW,.038,.035,trim);

  // A narrow stone sill projects beneath the reveal; its top supports the frame.
  kit.box(x,y-paneH/2-.045,frontZ(.22,.22),paneW+.2,.085,.22,concrete);
}

function renderUrbanFacade(
  kit: Kit, b: Building, _art: ArtMaterials,
  m: { w: number; d: number; base: number; top: number; floor: number; floors: number; baseFloor?: number; omitSideWindows?:1|-1; entryX?:number; parcelD?:number; massOffsetZ?:number; parcelW?:number; massOffsetX?:number; wood: THREE.Material; dark: THREE.Material; glass: THREE.Material; lightGlass: THREE.Material; trim: THREE.Material; glow: THREE.Material; iron: THREE.Material; concrete: THREE.Material; paleConcrete: THREE.Material; isApartment: boolean },
) {
  const { w, d, base, floor, floors, wood, dark, glass, lightGlass, trim, glow, iron, concrete, paleConcrete, isApartment } = m;
  const baseFloor = m.baseFloor ?? 0;
  const spacing = isApartment ? (w > 17 ? 3.35 : 3.05) : (w > 17 ? 4.5 : 3.25);
  for (let f = 0; f < floors; f++) {
    const y = base + f * floor + 1.95;
    for (const side of [1, -1]) {
      const z = side * (d / 2 + .07);
      const doorOpening=baseFloor+f===0&&side===1&&m.entryX!==undefined;
      if (b.kind === 'office') {
        // Offices use one calm curtain-wall ribbon per floor. Slim mullions and
        // opaque spandrels create a civic rhythm distinct from apartment bays.
        const bandW = w * .84;
        const glassTint = (f + pick(seedInt(b.appearance_seed), 3)) % 4 === 0 ? lightGlass : glass;
        const segments:Array<[number,number]>=doorOpening
          ? [[-bandW/2,Math.min(bandW/2,m.entryX!-0.62)],[Math.max(-bandW/2,m.entryX!+0.62),bandW/2]]
          : [[-bandW/2,bandW/2]];
        for(const [left,right] of segments){
          const segmentW=right-left;
          if(segmentW<.12)continue;
          const center=(right+left)/2;
          kit.box(center, y, z, segmentW, 2.02, .13, dark);
          kit.box(center, y + .04, z + side * .08, segmentW-.08, 1.78, .04, glassTint);
          const mullions = Math.max(1, Math.floor(segmentW / 1.65));
          for (let i = 0; i <= mullions; i++) {
            const x = left + (segmentW * i) / mullions;
            kit.box(x, y + .04, z + side * .13, .055, 1.8, .045, trim);
          }
          kit.box(center, y - 1.05, z + side * .11, segmentW, .2, .16, paleConcrete);
        }
      } else {
        for (let x = -w / 2 + 1.5; x < w / 2 - .8; x += spacing) {
          const lit = ((Math.floor(x * 3) + f * 7 + seedInt(b.appearance_seed)) % 6 + 6) % 6 < (isApartment ? 3 : 2);
          const ww = isApartment ? Math.min(spacing - .5, 1.85) : Math.min(spacing - .55, 2.35);
          if(doorOpening&&x+ww/2>m.entryX!-.62&&x-ww/2<m.entryX!+.62)continue;
          kit.box(x, y, z, ww + .22, 1.75, .13, dark);
          kit.box(x, y, z + side * .08, ww, 1.51, .04, lit ? glow : (f % 3 === 0 ? lightGlass : glass));
          kit.box(x, y, z + side * .15, .065, 1.52, .045, trim);
          kit.box(x, y, z + side * .15, ww, .065, .045, trim);
          if (isApartment && ((f + pick(b.appearance_seed, 2)) % 2 === 0)) {
            // Fit the projection to this facade's actual parcel clearance;
            // broad plots retain deep balconies instead of using the narrow kit.
            const clearance=m.parcelD===undefined?Infinity:m.parcelD/2-side*(m.massOffsetZ??0)-d/2;
            const balconyDepth=Math.max(.12,Math.min(.92,clearance-.17));
            const balconyProjection=.07+balconyDepth/2;
            const balconyZ = z + side * balconyProjection;
            const balconyW = ww + .3;
            const railZ=balconyZ+side*(balconyDepth/2-.05);
            kit.box(x, y - .94, balconyZ, balconyW, .13, balconyDepth, concrete);
            kit.box(x, y - .47, railZ, balconyW, .1, .12, paleConcrete);
            for (let railX = x - balconyW / 2; railX <= x + balconyW / 2 + .01; railX += .62) {
              kit.box(railX, y - .38, railZ, .045, .82, .045, iron);
            }
            kit.box(x, y + .08, railZ, balconyW, .055, .055, iron);
          }
        }
      }
      // Continuous lintels, sills, and floor bands lend the narrow facade a built rhythm.
      if(!doorOpening&&b.kind!=='office')kit.box(0, y - 1.02, z + side * .08, w + .18, .14, .2, paleConcrete);
      kit.box(0, base + (f + 1) * floor - .12, z + side * .08, w + .28, .18, .22, trim);
      if (!doorOpening&&baseFloor + f === 0 && (b.kind === 'office' || b.kind === 'station')) {
        kit.box(0, base + 1.12, z + side * .14, w * .68, .28, .08, wood);
      }
    }
  }
  // Side windows and fire stairs give block-shaped buildings a back and a service side.
  for (let f = 0; f < floors; f++) {
    for (const side of [-1, 1] as const) {
      if(m.omitSideWindows===side)continue;
      const sideClearance=m.parcelW===undefined?Infinity:Math.max(0,m.parcelW/2-(side*(m.massOffsetX??0)+w/2));
      const sideOffset=Math.min(.08,Math.max(0,sideClearance-.1));
      const x = side * (w / 2 + sideOffset), y = base + f * floor + 1.95;
      for (let z = -d / 2 + 1.3; z < d / 2 - .6; z += 3.1) {
        kit.box(x, y, z, .13, 1.6, 1.5, dark);
        kit.box(x + side * .08, y, z, .04, 1.38, 1.28, glass);
        if (side === 1 && z > 0 && f < floors - 1 && b.kind !== 'office') {
          const stairWidth=Math.min(.42,Math.max(0,(sideClearance-sideOffset-.1)*.72));
          if(stairWidth>.12){
            const stairOffset=stairWidth/2+.04;
            kit.box(x + side * stairOffset, y - .92, z, stairWidth, .11, 1.5, concrete);
            kit.box(x + side * (stairOffset+stairWidth/2+.045), y + .18, z, .05, 1.05, 1.5, iron);
          }
        }
      }
    }
  }
}

function renderGabledRoof(
  kit: Kit, w: number, d: number, top: number, roof: THREE.Material, tiles: THREE.Material, trim: THREE.Material, iron: THREE.Material, dark: THREE.Material, seed: number, parcelWidth: number,
) {
  const half = d / 2 + .58;
  const rise = THREE.MathUtils.clamp(half * .48, .82, 2.1);
  const angle = Math.atan2(rise, half);
  const slopeLength = Math.hypot(half, rise);
  const roofW = Math.min(w + 1.15, parcelWidth);
  // Two correctly mirrored planes meet at a straight ridge with their eaves at the walls.
  for (const side of [-1, 1]) {
    const panel = new THREE.BoxGeometry(roofW, .25, slopeLength);
    panel.rotateX(side * angle);
    panel.translate(0, top + rise / 2, side * half / 2);
    kit.add(panel, roof);
    // Eaves and their short exposed rafter tails.
    kit.box(0, top - .02, side * half, Math.min(roofW + .22, parcelWidth), .18, .2, trim);
    for (let x = -w / 2; x <= w / 2; x += 1.65) {
      kit.box(x, top - .1, side * (d / 2 + .32), .1, .24, .46, iron);
    }
  }
  addCeramicRoofDetail(kit,{width:roofW,halfDepth:half,rise,top,roof,tile:roof,surfaceOffset:.15});
  kit.box(0, top + rise + .12, 0, Math.min(roofW + .12, parcelWidth), .26, .3, roof);
  // Gable-end fascia frames a small attic vent and painted end boards.
  for (const side of [-1, 1]) {
    kit.box(0, top + rise * .5, side * (d / 2 + .32), w * .88, .13, .12, trim);
    kit.box(0, top + rise * .61, side * (d / 2 + .38), .52, .4, .08, dark);
    kit.box(0, top + rise * .61, side * (d / 2 + .43), .34, .08, .035, iron);
  }
  // Compact soil-colored tile accent varies by seed while the palette remains finite.
  if (pick(seed, 4) === 0) kit.box(0, top + rise + .31, 0, .8, .1, .42, tiles);
}

function renderFlatRoof(
  kit: Kit, m: { w: number; d: number; top: number; roof: THREE.Material; trim: THREE.Material; concrete: THREE.Material; iron: THREE.Material; paleConcrete: THREE.Material; seed: number; art: ArtMaterials },
) {
  const { w, d, top, roof, trim, concrete, iron, paleConcrete, seed, art } = m;
  kit.box(0, top + .22, 0, w + .45, .42, d + .45, roof);
  // Keep the waterproof roof field visible. A full-width pale trim slab here
  // used to cover it, making every roof read as the same concrete platform.
  kit.box(0, top + .48, 0, w + .15, .1, d + .15, roof);
  for (const z of [-d / 2, d / 2]) kit.box(0, top + .82, z, w + .45, .7, .16, concrete);
  for (const x of [-w / 2, w / 2]) kit.box(x, top + .82, 0, .16, .7, d + .35, concrete);
  for (const z of [-d / 2, d / 2]) kit.box(0, top + 1.2, z, w + .45, .06, .23, trim);
  for (const x of [-w / 2, w / 2]) kit.box(x, top + 1.2, 0, .23, .06, Math.max(.1,d-.23), trim);
  renderRoofServices(kit,{w,d,top,seed,concrete,iron,paleConcrete,dark:art.get('#344346')});
  // Aerial and rooftop laundry vary by use and seed.
  if (pick(seed >>> 8, 3) === 0) {
    const aerialX=-w/2+.2;
    const aerialW=Math.max(.35,Math.min(1.8,w-.4));
    kit.box(aerialX, top + 1.75, d/2-.2, .08, 2.5, .08, iron);
    kit.box(aerialX+aerialW/2, top + 2.95, d/2-.2, aerialW, .07, .07, iron);
  }
  if (pick(seed >>> 10, 4) === 1) {
    const laundry = art.get('#e7e0d1');
    const width=Math.max(.24,Math.min(.45,(w-.3)/3));
    const spacing=(w-width)/4;
    for (let i = 0; i < 3; i++) kit.box(-w/2+spacing*(i+1), top + 1.15, d/2-.16, width, .78, .06, laundry);
  }
}

function renderExternalUnits(
  kit: Kit, m: { w: number; d: number; top: number; floors: number; concrete: THREE.Material; iron: THREE.Material; dark: THREE.Material; paleConcrete: THREE.Material },
) {
  const { w, d, top, floors, concrete, iron, dark, paleConcrete } = m;
  // Three low-noise AC clusters, each with a grille, pipe and vibration stand.
  const count = floors > 4 ? 2 : 1;
  for (let i = 0; i < count; i++) {
    const z = -d / 2 - .32;
    const x = (i - (count - 1) / 2) * Math.min(w * .4, 3.2);
    const y = baseValue(top, floors, i);
    kit.box(x, y, z, 1.05, .72, .5, paleConcrete);
    kit.box(x, y, z - .27, .68, .44, .055, dark);
    for (let slat = -2; slat <= 2; slat++) kit.box(x + slat * .1, y, z - .31, .035, .37, .035, iron);
    kit.box(x, y - .42, z, 1.24, .08, .62, iron);
    kit.box(x + .65, y + .25, z + .04, .08, .5, .08, iron);
  }
  kit.box(w / 2 + .08, top * .68, -d * .18, .13, top * .64, .13, iron);
  kit.box(w / 2 + .22, top * .8, -d * .18, .18, .32, .2, concrete);
}

function renderStreetLife(
  kit: Kit,
  b: Building,
  art: ArtMaterials,
  m: { w: number; d: number; parcelW: number; parcelD: number; frontageEdge?:number; wood: THREE.Material; dark: THREE.Material; green: THREE.Material; paleConcrete: THREE.Material; iron: THREE.Material; trim: THREE.Material; seed: number },
) {
  const { w, d, parcelW, parcelD, wood, dark, green, paleConcrete, iron, trim, seed } = m;
  const frontage = m.frontageEdge ?? d / 2 + .08;
  const parcelFront = parcelD / 2;
  const clearDepth = parcelFront - frontage;
  const parcelHalfWidth = parcelW / 2;
  const planters = art.get('#817f70');
  const soil = art.get('#4b493e');
  const foliage = art.get('#587363');
  const leafLight = art.get('#718166');

  // Two compact planters soften the shop corners. Their outside edges are
  // clamped inward so narrow parcels never spill into the neighboring lot.
  const planterX = Math.min(w / 2 - .48, parcelHalfWidth - .43);
  const planterZ = Math.min(frontage + .48, parcelFront - .31);
  if (planterX > .6 && planterZ >= frontage + .31) {
    for (const side of [-1, 1]) {
      const x = side * planterX;
      kit.box(x, .24, planterZ, .76, .48, .62, planters);
      kit.box(x, .49, planterZ, .65, .045, .51, soil);
      for (const offset of [-.2, 0, .2]) {
        const h = .38 + pick(seed + Math.round((offset + .25) * 100), 3) * .08;
        kit.cone(x + offset, .52 + h / 2, planterZ + (offset === 0 ? .02 : 0), .2, h, offset === 0 ? leafLight : foliage, 6);
      }
    }
  }

  // A vending machine is a small frontage object, with a shaded display and a
  // warm service panel instead of a large luminous sign.
  const hasFrontage = clearDepth >= .78;
  const vendingX = -Math.min(w * .32, parcelHalfWidth - .48);
  const vendingZ = Math.min(frontage + Math.max(.48, clearDepth * .66), parcelFront - .29);
  if (hasFrontage && (b.anchor || pick(seed >>> 2, 3) !== 0) && Math.abs(vendingX) > .8) {
    const body = art.get('#c7c1b2');
    const display = art.get('#344a4d', .38);
    const accent = art.get('#bd6953');
    kit.box(vendingX, .91, vendingZ, .78, 1.82, .46, body);
    kit.box(vendingX - .07, 1.11, vendingZ + .238, .49, 1.22, .035, display);
    for (let row = 0; row < 3; row++) {
      for (let col = 0; col < 2; col++) {
        kit.box(vendingX - .22 + col * .15, 1.49 - row * .3, vendingZ + .263, .095, .14, .04,
          (row + col) % 2 ? paleConcrete : trim);
      }
    }
    kit.box(vendingX + .25, .78, vendingZ + .24, .15, .42, .04, accent);
    kit.box(vendingX, .36, vendingZ + .24, .44, .18, .045, dark);
  }

  // One parked bicycle gives the frontage a human scale. The slim rack and
  // frame use shared materials and are merged with the rest of the building.
  const bikeX = Math.min(w * .34, parcelHalfWidth - 1.04);
  const bikeZ = Math.min(frontage + Math.max(.58, clearDepth * .72), parcelFront - .43);
  const hasBike = parcelW >= 12 && hasFrontage && (b.anchor || pick(seed >>> 5, 2) === 0);
  if (hasBike && bikeX > .95) {
    const tire = art.get('#303b3d');
    const frame = art.get('#65766e', .58);
    const centerX = bikeX;
    const centerZ = bikeZ;
    const rear = { x: centerX - .63, y: .39, z: centerZ };
    const front = { x: centerX + .63, y: .39, z: centerZ };
    const crank = { x: centerX - .02, y: .42, z: centerZ };
    const seat = { x: centerX - .12, y: .91, z: centerZ };
    const stem = { x: centerX + .4, y: .83, z: centerZ };
    for (const wheelX of [rear.x, front.x]) {
      const wheel = new THREE.TorusGeometry(.35, .035, 5, 12);
      wheel.translate(wheelX, .39, centerZ);
      kit.add(wheel, tire);
      kit.cylinder(wheelX, .39, centerZ, .045, .08, iron, 6);
    }
    addRod(kit, rear, crank, .035, frame);
    addRod(kit, crank, seat, .035, frame);
    addRod(kit, seat, rear, .035, frame);
    addRod(kit, seat, stem, .035, frame);
    addRod(kit, stem, front, .035, frame);
    addRod(kit, crank, front, .035, frame);
    addRod(kit, stem, { x: stem.x + .24, y: stem.y + .03, z: centerZ }, .035, iron);
    kit.box(centerX - .16, .98, centerZ, .38, .08, .13, dark);
    kit.box(centerX - .02, .35, centerZ, .14, .06, .08, iron);

    // A short inverted-U stand sits beside the wheels, contained within the lot.
    const rackX = Math.min(parcelHalfWidth - .46, bikeX + 1.15);
    if (rackX > bikeX + .55) {
      kit.box(rackX, .045, bikeZ, .9, .09, .16, iron);
      addRod(kit, { x: rackX - .34, y: .05, z: bikeZ }, { x: rackX - .34, y: .63, z: bikeZ }, .035, iron);
      addRod(kit, { x: rackX + .34, y: .05, z: bikeZ }, { x: rackX + .34, y: .63, z: bikeZ }, .035, iron);
      addRod(kit, { x: rackX - .34, y: .63, z: bikeZ }, { x: rackX + .34, y: .63, z: bikeZ }, .035, iron);
    }
  }

  // Cafes get one tiny table and two stools tucked under the canopy.
  if (b.kind === 'cafe' && hasFrontage && parcelW >= 10) {
    const tableX = Math.max(-parcelHalfWidth + .84, -w * .08);
    const tableZ = Math.min(frontage + .52, parcelFront - .42);
    const tabletop = art.get('#8b6950');
    kit.cylinder(tableX, .66, tableZ, .34, .1, tabletop, 10);
    kit.cylinder(tableX, .35, tableZ, .055, .58, iron, 6);
    for (const dx of [-.62, .62]) {
      kit.cylinder(tableX + dx, .46, tableZ, .2, .09, wood, 8);
      for (const dz of [-.13, .13]) {
        kit.box(tableX + dx, .23, tableZ + dz, .045, .46, .045, iron);
      }
    }
  }

  // Wider corner lots can carry a discreet pair of service poles and a single
  // taut wire. Keeping it above the roofline avoids tangling the shop facade.
  if (b.anchor && parcelW >= 16 && parcelD >= 16) {
    const poleX = parcelHalfWidth - .39;
    const wireZ = parcelFront - .3;
    const poleY = Math.max(5.1, Math.min(8.2, (b.floors * 3.25) + 1.4));
    for (const side of [-1, 1]) {
      kit.box(side * poleX, poleY / 2, wireZ, .13, poleY, .13, iron);
      kit.box(side * poleX, poleY - .35, wireZ, .72, .075, .075, wood);
      for (const dx of [-.23, .23]) kit.cylinder(side * poleX + dx, poleY - .27, wireZ, .045, .16, trim, 6);
    }
    addRod(kit,
      { x: -poleX, y: poleY - .22, z: wireZ },
      { x: poleX, y: poleY - .22, z: wireZ },
      .018,
      dark,
    );
  }

}

function addRod(
  kit: Kit,
  from: { x: number; y: number; z: number },
  to: { x: number; y: number; z: number },
  radius: number,
  material: THREE.Material,
) {
  const start = new THREE.Vector3(from.x, from.y, from.z);
  const end = new THREE.Vector3(to.x, to.y, to.z);
  const direction = end.clone().sub(start);
  const geometry = new THREE.CylinderGeometry(radius, radius, direction.length(), 6);
  geometry.applyQuaternion(new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 1, 0), direction.normalize()));
  geometry.translate((start.x + end.x) / 2, (start.y + end.y) / 2, (start.z + end.z) / 2);
  kit.add(geometry, material);
}

function baseValue(top: number, floors: number, index: number) {
  return Math.min(top - .3, 3.2 + (index % Math.max(1, Math.min(3, floors - 1))) * 3.25);
}

function renderRenovationScaffold(kit: Kit, w: number, d: number, h: number, material: THREE.Material, caution: THREE.Material) {
  for (const x of [-w / 2 - .45, w / 2 + .45]) {
    for (const z of [-d / 2 - .45, d / 2 + .45]) kit.box(x, h / 2, z, .07, h, .07, material);
    for (let y = 1.2; y < h; y += 2.4) kit.box(x, y, 0, .06, .06, d + 1, material);
  }
  kit.box(0, 1.5, d / 2 + 1.55, w + .8, 2.8, .13, caution);
}

function seedInt(seed: number) { return Math.trunc(seed) >>> 0; }

function finishBuilding(kit: Kit, b: Building, w: number, d: number, height: number, art: ArtMaterials, completed: boolean) {
  const group = kit.finish();
  if (completed && (b.anchor || b.kind === 'shop' || b.kind === 'cafe')) {
    const { material, v0, v1 } = art.shopSign(b.anchor ? '喫茶 青葉' : (b.kind === 'cafe' ? '珈琲' : '商店'));
    const signW = Math.min(w * .7, b.anchor ? 5.5 : 4.8);
    const signH = signW / 3.7;
    const plane = new THREE.PlaneGeometry(signW, signH), uvs = plane.getAttribute('uv');
    for (let i = 0; i < uvs.count; i++) uvs.setY(i, v0 + uvs.getY(i) * (v1 - v0));
    const sign = new THREE.Mesh(plane, material);
    const signY = height < 5.2 ? height - signH / 2 - .06 : Math.max(4.45, Math.min(height - .25, 4.85));
    sign.position.set(0, signY, Math.min(d / 2 + .31, b.depth / 2 - .03));
    group.add(sign);
  }
  group.userData.buildingId = b.id;
  group.traverse(o => o.userData.buildingId = b.id);
  return group;
}

export function disposeGroup(group: THREE.Object3D) {
  group.traverse(o => {
    if (o instanceof THREE.Mesh) {
      if(o instanceof THREE.InstancedMesh)o.dispose();
      o.geometry.dispose();
      if (o.userData.ownedMaterial) (o.material as THREE.Material).dispose();
    }
    if (o.userData.ownedTexture) (o.userData.ownedTexture as THREE.Texture).dispose();
  });
  group.removeFromParent();
}
