import * as THREE from 'three/webgpu';
import { ArtMaterials } from './materials';
import type { Building } from '../types';
import type { Kit } from './kit';
import type { BuildingMass } from './building-massing';

/** Shared silhouette for finished towers, construction and top-down demolition. */
export function towerMassing(b: Pick<Building, 'appearance_seed' | 'floors'>, w: number, d: number): BuildingMass[] {
  const profile = (b.appearance_seed >>> 0) % 3;
  const crownFloors = profile === 1 ? 3 : 2;
  const shaftW = w * (profile === 2 ? .77 : .86), shaftD = d * (profile === 0 ? .79 : .87);
  const x = profile === 2 ? -w * .07 : 0;
  return [
    { x: 0, z: 0, width: w, depth: d, baseFloor: 0, floors: 3, role: 'main' },
    { x, z: 0, width: shaftW, depth: shaftD, baseFloor: 3, floors: b.floors - 3 - crownFloors, role: 'main' },
    { x, z: 0, width: shaftW * .83, depth: shaftD * .82, baseFloor: b.floors - crownFloors, floors: crownFloors, role: 'main' },
  ];
}

/** A tower is a podium, a slender occupied shaft, and a recessed service crown.
 * Its floor count and footprint remain those of the authoritative building. */
export function renderTower(kit: Kit, b: Building, art: ArtMaterials, w: number, d: number, base: number, floor: number, finishedFloors = b.floors) {
  const seed = b.appearance_seed >>> 0, apartment = b.kind === 'apartment';
  const profile = seed % 3;
  // Apartment towers are tile-faced; office towers show formed concrete between their ribbons.
  const stone = art.get(['#c4c3b8', '#aab6b8', '#d3cec0'][profile], .85, false, 'ambient', apartment ? 'tile' : 'concrete');
  const frame = art.get(apartment ? ['#87999c', '#829294', '#99a5a3'][profile] : ['#768b93', '#647e8a', '#8e9da1'][profile], .6);
  const shadow = art.get(apartment ? '#59666b' : '#304752'), roof = art.get('#64767d');
  // Every pane shows a room; lights switch per room with the time of day.
  const glass = art.window(apartment ? 'home' : 'office');
  const silver = art.get('#becacd', .32);
  const operational = b.stage === 'open' || b.stage === 'renovation';
  const lobby = operational ? art.get('#e2bd7f', .45, true, 'shop') : glass;
  const podiumFloors = 3, crownFloors = profile === 1 ? 3 : 2;
  const shaftFloors = b.floors - podiumFloors - crownFloors;
  const shaftW = w * (profile === 2 ? .77 : .86), shaftD = d * (profile === 0 ? .79 : .87);
  const offsetX = profile === 2 ? -w * .07 : 0;
  const bands = (width: number, depth: number, first: number, count: number, x = 0) => {
    const height = count * floor, bottom = base + first * floor;
    kit.box(x, bottom + height / 2, 0, width, height, depth, stone);
    for (let f = 0; f < count; f++) {
      const level = first + f, y = bottom + f * floor + 1.8;
      if (apartment) {
        // Apartment elevations are masonry and narrow window bays, rather than
        // the continuous glazed ribbons used by offices. The shallow reveals
        // remain within the shaft footprint and merge into the shared kit.
        const baysW = Math.max(4, Math.floor(width / 3.1));
        const baysD = Math.max(3, Math.floor(depth / 3.0));
        // Residential balconies form one or two continuous columns selected
        // by the building seed. Keeping the bay choice stable up the elevation
        // gives the facade a readable rhythm instead of scattered loggias.
        const balconyColumns = new Set<number>();
        const balconyCount = baysW >= 6 && (seed % 3 === 1) ? 2 : 1;
        const firstBalcony = 1 + (seed % Math.max(1, baysW - 2));
        balconyColumns.add(firstBalcony);
        if (balconyCount === 2) balconyColumns.add(Math.min(baysW - 2, firstBalcony + Math.max(2, Math.floor(baysW / 2))));
        for (const side of [-1, 1]) {
          const faceZ = side * (depth / 2 + .025);
          for (let bay = 0; bay < baysW; bay++) {
            const bayWidth = (width - 1.1) / baysW;
            const bx = x - width / 2 + .55 + bayWidth * (bay + .5);
            const balcony = balconyColumns.has(bay);
            const paneHeight = balcony ? 2.24 : 2.08;
            const paneY = y + .02;
            const paneW = bayWidth * (balcony ? .52 : .46);
            // A deep, narrow reveal makes the tall sash read as an opening
            // within the pier rhythm instead of another square on a grid.
            kit.box(bx, y + .02, side * (depth / 2 + .045), paneW + .14, paneHeight + .14, .055, shadow);
            kit.box(bx, paneY, side * (depth / 2 + .09), paneW, paneHeight, .035, glass);
            // Fine jambs, sill and head trim articulate each vertically
            // proportioned opening without adding per-building materials.
            for (const edge of [-1, 1]) {
              kit.box(bx + edge * bayWidth * .36, y + .02, faceZ + side * .07, .065, 2.48, .14, frame);
            }
            kit.box(bx, y - 1.22, faceZ + side * .07, bayWidth * .74, .105, .14, stone);
            kit.box(bx, y + 1.25, faceZ + side * .07, bayWidth * .74, .075, .14, stone);
            // A slim central sash line gives scale to the glazing. Balcony
            // bays retain a clear door-sized center opening behind the rail.
            kit.box(bx, y + .02, side * (depth / 2 + .115), .045, paneHeight * .92, .018, frame);
            kit.box(bx, y - .08, side * (depth / 2 + .116), paneW * .92, .045, .018, frame);
            if (balcony) {
              // Shallow slab and slender guard repeat on each floor in the
              // same column. The projection stays inside the tower parcel.
              const deckDepth = Math.min(1.1, Math.max(.82, (d - shaftD) * .42));
              const deckEdge = depth / 2 + deckDepth;
              kit.box(bx, y - 1.34, side * (depth / 2 + deckDepth / 2), bayWidth * .7, .12, deckDepth, stone);
              // A low, light parapet gives each balcony column a solid
              // horizontal break; the glazing above remains a door-sized
              // opening. Its projection stays inside the parcel envelope.
              kit.box(bx, y - 1.02, side * (deckEdge - .035), bayWidth * .68, .48, .07, stone);
              kit.box(bx, y - .74, side * (deckEdge - .035), bayWidth * .68, .035, .06, frame);
              for (const edge of [-1, 1]) {
                kit.box(bx + edge * bayWidth * .32, y - .84, side * (deckEdge - .035), .045, .34, .06, frame);
              }
            }
          }
        }
        for (const side of [-1, 1]) {
          const faceX = x + side * (width / 2 + .025);
          for (let bay = 0; bay < baysD; bay++) {
            const bayDepth = (depth - .9) / baysD;
            const bz = -depth / 2 + .45 + bayDepth * (bay + .5);
            const sidePaneW = bayDepth * .48;
            kit.box(faceX + side * .02, y + .02, bz, .055, 2.32, sidePaneW + .14, shadow);
            kit.box(faceX + side * .06, y + .02, bz, .035, 2.16, sidePaneW, glass);
            kit.box(faceX + side * .055, y - 1.18, bz, .12, .11, sidePaneW + .12, stone);
            kit.box(faceX + side * .055, y + 1.16, bz, .12, .075, sidePaneW + .12, stone);
            kit.box(faceX + side * .085, y + .02, bz, .018, .035, sidePaneW * .9, frame);
          }
        }
        continue;
      }
      for (const side of [-1, 1]) {
        kit.box(x, y, side * (depth / 2 + .04), width - .45, 2.45, .075, glass);
        kit.box(x + side * (width / 2 + .04), y, 0, .075, 2.45, depth - .45, glass);
        const projection = apartment ? .38 : .11;
        kit.box(x, y - 1.32, side * (depth / 2 + projection / 2), width + .18, apartment ? .13 : .2, projection + .1, apartment ? stone : frame);
        kit.box(x + side * (width / 2 + projection / 2), y - 1.32, 0, projection + .1, apartment ? .13 : .2, depth + .18, apartment ? stone : frame);
      }
    }
    if (apartment) {
      // Vertical structural piers are authored once per tower mass, rather
      // than once per floor, keeping the merged geometry budget predictable.
      const baysW = Math.max(4, Math.floor(width / 3.1));
      const baysD = Math.max(3, Math.floor(depth / 3.0));
      for (const side of [-1, 1]) {
        for (let pier = 0; pier <= baysW; pier++) {
          const px = x - width / 2 + .55 + (width - 1.1) * pier / baysW;
          kit.box(px, bottom + height / 2, side * (depth / 2 + .05), .16, height, .12, frame);
        }
        for (let pier = 0; pier <= baysD; pier++) {
          const pz = -depth / 2 + .45 + (depth - .9) * pier / baysD;
          kit.box(x + side * (width / 2 + .045), bottom + height / 2, pz, .12, height, .16, frame);
        }
      }
    } else {
      // Office curtain walls keep the former continuous mullion rhythm.
      const baysW = Math.max(4, Math.round(width / 1.55));
      const baysD = Math.max(4, Math.round(depth / 1.55));
      for (const side of [-1, 1]) {
        for (let bay = 0; bay <= baysW; bay++) kit.box(x - width / 2 + width * bay / baysW, bottom + height / 2, side * (depth / 2 + .1), bay === 0 || bay === baysW ? .26 : .075, height, .13, silver);
        for (let bay = 0; bay <= baysD; bay++) kit.box(x + side * (width / 2 + .1), bottom + height / 2, -depth / 2 + depth * bay / baysD, .13, height, bay === 0 || bay === baysD ? .26 : .075, silver);
      }
    }
  };
  for (const mass of towerMassing(b, w, d)) {
    const installed = Math.max(0, Math.min(mass.floors, finishedFloors - mass.baseFloor));
    if (installed > 0) bands(mass.width, mass.depth, mass.baseFloor, installed, mass.x);
  }
  const podiumTop = base + podiumFloors * floor, top = base + b.floors * floor;
  if (finishedFloors >= podiumFloors) kit.box(0, podiumTop + .1, 0, w + .3, .22, d + .3, stone);
  if (finishedFloors >= b.floors) {
  kit.box(offsetX, top + .13, 0, shaftW * .86, .26, shaftD * .85, roof);
  // Rooftop mechanical screen and an occasional restrained aerial.
  kit.box(offsetX, top + .85, -shaftD * .13, shaftW * .42, 1.45, shaftD * .36, shadow);
  for (let row = 0; row < 5; row++) kit.box(offsetX, top + .28 + row * .27, shaftD * .052, shaftW * .43, .075, .08, frame);
  if (profile !== 1) {
    kit.box(offsetX + shaftW * .2, top + 2, -shaftD * .15, .12, 3.8, .12, silver);
    kit.box(offsetX + shaftW * .2, top + 3.6, -shaftD * .15, .22, .16, .22, art.get('#b86a5b', .6, operational));
  }
  }
  if (finishedFloors < 2) return;
  // Double-height entry, canopy, and occupied ground-floor frontage.
  kit.box(0, base + 2, d / 2 + .18, Math.min(5, w * .36), 3.8, .2, shadow);
  kit.box(0, base + 2, d / 2 + .3, Math.min(3.8, w * .28), 3.5, .04, lobby);
  kit.box(0, base + 4.15, d / 2 + 1, Math.min(7, w * .52), .18, 2, frame);
  for (const side of [-1, 1]) kit.box(side * w * .35, base + 1.65, d / 2 + .15, w * .2, 2.6, .08, lobby);
}
