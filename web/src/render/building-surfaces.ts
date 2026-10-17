import * as THREE from 'three/webgpu';
import {
  abs, attribute, cameraPosition, clamp, color, cross, dot, float, floor, fract, fwidth, max, min, mix, mx_noise_float, normalize,
  normalWorld, positionWorld, select, sin, smoothstep, step, uv, vec3, vec4,
} from 'three/tsl';
import type { Building } from '../types';

/**
 * Building surface realism: wall cladding and windows with rooms behind them.
 *
 * Both are shader work on the existing geometry. Cladding is a pattern drawn
 * along each facade (horizontal siding, tile, ALC panel joints or formed
 * concrete), so it costs no texture memory and fades to its mean colour where
 * its lines would be smaller than a pixel. Windows use interior mapping: each
 * pane shows a box room behind the glass, with curtains or blinds, and its
 * light switches with a city-wide occupancy level for the building's use.
 */

/** Wall finishes common in Tokyo; plaster keeps the existing painted surface. */
export type Cladding = 'plaster' | 'siding' | 'tile' | 'alc' | 'concrete';
export type WindowActivity = 'home' | 'office' | 'shop';

/** Seeded pick from weighted options; the weights need not sum to one. */
function weighted<T>(value: number, options: ReadonlyArray<readonly [T, number]>): T {
  const total = options.reduce((sum, [, weight]) => sum + weight, 0);
  let target = value * total;
  for (const [option, weight] of options) { if ((target -= weight) < 0) return option; }
  return options[options.length - 1][0];
}
function unit(seed: number, salt: number) {
  let h = Math.imul((seed >>> 0) ^ Math.imul(salt + 1, 0x9e3779b9), 0x85ebca6b);
  h ^= h >>> 13; h = Math.imul(h, 0xc2b2ae35); h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}

/**
 * The finish a building's walls carry, by use and height. Detached homes are
 * mostly lap siding or plaster; apartment blocks favour tile; offices mix
 * tile, ALC panel and exposed concrete. Stable per building seed.
 */
export function claddingFor(b: Pick<Building, 'kind' | 'floors' | 'appearance_seed'>): Cladding {
  const value = unit(b.appearance_seed, 211);
  if (b.kind === 'home') return weighted(value, [['siding', 55], ['plaster', 45]]);
  if (b.kind === 'apartment') return b.floors <= 2
    ? weighted(value, [['siding', 45], ['plaster', 30], ['tile', 25]])
    : weighted(value, [['tile', 55], ['plaster', 25], ['concrete', 20]]);
  if (b.kind === 'office') return weighted(value, [['tile', 35], ['alc', 35], ['concrete', 30]]);
  if (b.kind === 'shop' || b.kind === 'cafe') return b.floors <= 2
    ? weighted(value, [['plaster', 45], ['siding', 30], ['tile', 25]])
    : weighted(value, [['tile', 40], ['plaster', 35], ['alc', 25]]);
  return 'plaster';
}

/** Which window light schedule a building follows. */
export function windowActivity(kind: string): WindowActivity {
  return kind === 'home' || kind === 'apartment' ? 'home' : kind === 'shop' || kind === 'cafe' ? 'shop' : 'office';
}

/** 1 when someone lives or works behind the windows; vacant and unfinished buildings keep bare, dark rooms. */
export function windowLived(b: Pick<Building, 'stage' | 'kind' | 'occupancy' | 'business_active'>): number {
  if (b.stage !== 'open') return 0;
  const residential = b.kind === 'home' || b.kind === 'apartment';
  return (residential ? b.occupancy > 0 : b.business_active !== false) ? 1 : 0;
}

/** Piecewise-linear schedule through (hour, value) points, wrapping at midnight. */
function schedule(points: ReadonlyArray<readonly [number, number]>, hour: number) {
  const h = ((hour % 24) + 24) % 24;
  for (let i = 0; i < points.length; i++) {
    const [h0, v0] = points[i], [h1, v1] = points[(i + 1) % points.length];
    const end = h1 <= h0 ? h1 + 24 : h1, at = h < h0 ? h + 24 : h;
    if (at >= h0 && at <= end) return v0 + (v1 - v0) * (end === h0 ? 0 : (at - h0) / (end - h0));
  }
  return points[0][1];
}

/**
 * Share of rooms with their light on at a given hour. Homes fill up after
 * dusk and empty after midnight; offices are lit through the working day and
 * thin out in the evening; shops follow trading hours.
 */
export function windowOccupancy(activity: WindowActivity, hour: number): number {
  if (activity === 'home') return schedule([[0, .12], [5, .06], [6.5, .26], [8.5, .08], [16, .08], [18.5, .48], [21.5, .52], [23.5, .18]], hour);
  if (activity === 'office') return schedule([[0, .05], [7, .06], [9, .86], [18, .84], [21, .32], [23, .08]], hour);
  return schedule([[0, .04], [9, .05], [10, .72], [20.5, .7], [22.5, .1]], hour);
}

/**
 * Per-vertex window data: room seed, lived flag plus twice the schedule index,
 * pane width and height in metres. Packing the schedule lets every pane in a
 * neighbourhood share one batch.
 */
export const WINDOW_ATTRIBUTE = 'windowInfo';
const ACTIVITY_INDEX: Record<WindowActivity, number> = { home: 0, office: 1, shop: 2 };
/** Per-vertex cladding kind, so every clad wall in a neighbourhood shares one batch. */
export const CLADDING_ATTRIBUTE = 'claddingKind';
export const CLADDING_INDEX: Record<Exclude<Cladding, 'plaster'>, number> = { siding: 1, tile: 2, alc: 3, concrete: 4 };
export function tagCladding(geometry: THREE.BufferGeometry, kind: Exclude<Cladding, 'plaster'>) {
  const count = geometry.getAttribute('position').count;
  geometry.setAttribute(CLADDING_ATTRIBUTE, new THREE.BufferAttribute(new Float32Array(count).fill(CLADDING_INDEX[kind]), 1));
}

/** Deterministic pane seed from the building seed and the pane's local position. */
export function paneSeed(buildingSeed: number, x: number, y: number, z: number): number {
  return unit((buildingSeed >>> 0) ^ Math.imul(Math.round(x * 10), 73856093) ^ Math.imul(Math.round(y * 10), 19349663) ^ Math.imul(Math.round(z * 10), 83492791), 7);
}

/** Attach window data to a pane box; the thin dimension is its depth. */
export function tagWindowPane(geometry: THREE.BufferGeometry, seed: number, lived: number, width: number, height: number, activity: WindowActivity = 'home') {
  const count = geometry.getAttribute('position').count, data = new Float32Array(count * 4), state = lived + 2 * ACTIVITY_INDEX[activity];
  for (let i = 0; i < count; i++) { data[i * 4] = seed; data[i * 4 + 1] = state; data[i * 4 + 2] = width; data[i * 4 + 3] = height; }
  geometry.setAttribute(WINDOW_ATTRIBUTE, new THREE.BufferAttribute(data, 4));
}

type FloatNode = ReturnType<typeof float>;
type Vec3Node = ReturnType<typeof vec3>;

const UP = vec3(0, 1, 0);
/** 1 on walls, 0 on roofs and soffits. */
function verticalMask() { return smoothstep(float(.55), float(.35), abs(normalWorld.y)); }
/** Facade tangent: horizontal, along the wall. Matches BoxGeometry's u direction on side faces. */
function facadeTangent() { return normalize(cross(UP, normalWorld).add(vec3(1e-4, 0, 1e-4))); }
const hash = (x: FloatNode) => fract(sin(x.mul(12.9898).add(78.233)).mul(43758.5453));
/** Line mask: 1 on a line of half-width `half` repeating every `period`, faded where the pixel footprint outgrows it. */
function lines(coord: FloatNode, period: number, half: number) {
  const cell = coord.div(period), fw = max(fwidth(cell), float(1e-5));
  const distance = abs(fract(cell.add(.5)).sub(.5));
  const line = smoothstep(float(half / period).add(fw), float(half / period), distance);
  // Lines vanish once a repeat spans fewer than ~4 pixels, before they can beat into moire.
  return line.mul(smoothstep(float(.25), float(.08), fw));
}
/** Fade for per-element variation: full below a quarter element per pixel, gone above half. */
function detail(coord: FloatNode, period: number) { return smoothstep(float(.3), float(.12), fwidth(coord.div(period))); }

/**
 * Multiplier for a cladding pattern. `strength` scales its contrast: Anime
 * keeps it lighter so the painted surface leads; Miniature shows it fully.
 */
export function claddingShade(kind: Cladding, strength: number): Vec3Node {
  if (kind === 'plaster') return vec3(1, 1, 1);
  const masked = mix(float(1), claddingPattern(kind), verticalMask().mul(strength));
  return vec3(masked, masked, masked);
}

/** The cladding multiplier for a shared batch: the kind comes from each vertex. */
export function claddingShadeBatched(strength: number): Vec3Node {
  const kind = float(attribute(CLADDING_ATTRIBUTE, 'float'));
  const pick = (index: number) => abs(kind.sub(index)).lessThan(.5);
  // Keep derivative-based patterns outside conditional shader blocks. The
  // shared shader serves every finish; evaluating all patterns before masking
  // avoids backend-dependent darkening from conditional pattern evaluation.
  const shade = float(1)
    .add(claddingPattern('siding').sub(1).mul(float(pick(1))))
    .add(claddingPattern('tile').sub(1).mul(float(pick(2))))
    .add(claddingPattern('alc').sub(1).mul(float(pick(3))))
    .add(claddingPattern('concrete').sub(1).mul(float(pick(4))));
  const masked = mix(float(1), shade, verticalMask().mul(strength));
  return vec3(masked, masked, masked);
}

function claddingPattern(kind: Exclude<Cladding, 'plaster'>): FloatNode {
  const u = dot(positionWorld, facadeTangent()), v = positionWorld.y;
  let shade: FloatNode;
  if (kind === 'siding') {
    // Cement siding panels: 0.455 m boards with a shadow line under each lap,
    // in 3.03 m panels whose vertical joints line up.
    const board = v.div(.455), within = fract(board);
    const fade = detail(v, .455);
    const lap = smoothstep(float(.1), float(0), within).mul(.2).add(smoothstep(float(.88), float(1), within).mul(-.04));
    const joints = lines(u, 3.03, .005).mul(.12);
    shade = float(1).sub(lap.mul(fade)).sub(joints).add(within.sub(.5).mul(.05).mul(fade));
  } else if (kind === 'tile') {
    // Running-bond facing tile, 227 x 60 mm with grout, each tile a slightly different fired tone.
    const rowCoord = v.div(.07), row = floor(rowCoord);
    const along = u.add(row.mod(2).mul(.1175)), col = floor(along.div(.235));
    const grout = max(lines(v, .07, .005), lines(along, .235, .005));
    const tone = hash(row.mul(17.31).add(col.mul(3.17))).sub(.5).mul(.12).mul(detail(u, .235));
    const panel = mx_noise_float(vec3(u, v, 0).mul(.35)).mul(.04);
    shade = float(1).add(tone).add(panel).sub(grout.mul(.2));
  } else if (kind === 'alc') {
    // Autoclaved concrete panels 0.6 m wide with sealed vertical joints, and a joint at each floor.
    const joints = max(lines(u, .6, .008), lines(v.sub(.58), 3.25, .012));
    const speckle = mx_noise_float(vec3(u, v, 0).mul(3.1)).mul(.025).mul(detail(u, .3));
    shade = float(1).sub(joints.mul(.24)).add(speckle);
  } else {
    // Formed concrete: 0.9 x 1.8 m form panels, six tie holes each, and a cloudy pour.
    const pu = fract(u.div(.9)), pv = fract(v.sub(.58).div(1.8));
    const seams = max(lines(u, .9, .006), lines(v.sub(.58), 1.8, .006));
    const holeU = min(abs(pu.sub(.18)), abs(pu.sub(.82))).mul(.9);
    const holeV = min(min(abs(pv.sub(.17)), abs(pv.sub(.5))), abs(pv.sub(.83))).mul(1.8);
    const holes = smoothstep(float(.022), float(.014), vec3(holeU, holeV, 0).length()).mul(detail(u, .2));
    const pour = mx_noise_float(vec3(u, v, 0).mul(.45)).mul(.07).add(mx_noise_float(vec3(u, v, 0).mul(2.3)).mul(.025));
    shade = float(1).sub(seams.mul(.16)).sub(holes.mul(.35)).add(pour);
  }
  return shade;
}

interface WindowUniforms { night: FloatNode; occupancy: { home: FloatNode; office: FloatNode; shop: FloatNode } }
const choose3 = (h: FloatNode, a: Vec3Node, b: Vec3Node, c: Vec3Node) => select(h.lessThan(.34), a, select(h.lessThan(.67), b, c));

/**
 * Radiance seen through a window pane: the room behind it, curtains or
 * blinds, its light, and the sky reflected in the glass. Horizontal faces of
 * the pane box show only the reflection.
 */
export function windowRadiance(u: WindowUniforms, corporateTint?:string): Vec3Node {
  const info = vec4(attribute(WINDOW_ATTRIBUTE, 'vec4'));
  // Interpolating a per-pane constant can wobble by an ulp between pixels; the
  // hash would amplify that into speckle, so snap the seed before using it.
  const seed = floor(float(info.x).mul(4096).add(.5)).div(4096), paneW = max(float(info.z), float(.2)), paneH = max(float(info.w), float(.2));
  const n = normalWorld, t = facadeTangent();
  const view = normalize(positionWorld.sub(cameraPosition));
  const dx = dot(view, t), dy = float(view.y), dz = min(dot(view, n), float(-1e-3));
  // Schedule and occupancy are packed together: lived + 2 x (home 0, office 1, shop 2).
  const state = floor(float(info.y).add(.5)), activityIndex = floor(state.div(2)), lived = state.sub(activityIndex.mul(2));
  const home = activityIndex.lessThan(.5), office = activityIndex.greaterThan(.5).and(activityIndex.lessThan(1.5));

  // Wide panes (office ribbons, broad sashes) hold several rooms side by side.
  const module = select(home, float(3.6), float(3.2));
  const cells = max(floor(paneW.div(module).add(.5)), float(1)), cellW = paneW.div(cells);
  const px0 = uv().x.mul(paneW), cell = min(floor(px0.div(cellW)), cells.sub(1));
  const px = px0.sub(cell.mul(cellW)), py = uv().y.mul(paneH);
  const rs = fract(seed.add(cell.mul(.6180339)));
  const h = (k: number) => hash(rs.add(k * .1371));

  // The room: wider than the pane, floor below the sill, ceiling above the head.
  const xMin = float(-.45), xMax = cellW.add(.45), yMin = select(home, float(-.9), float(-.8)), yMax = paneH.add(.35);
  const depth = mix(float(3), float(5.5), h(1));
  const tx = select(dx.greaterThan(0), xMax.sub(px).div(max(dx, float(1e-4))), xMin.sub(px).div(min(dx, float(-1e-4))));
  const ty = select(dy.greaterThan(0), yMax.sub(py).div(max(dy, float(1e-4))), yMin.sub(py).div(min(dy, float(-1e-4))));
  const tz = depth.div(dz.negate());
  const hit = min(min(tx, ty), tz);
  const hx = px.add(dx.mul(hit)), hy = py.add(dy.mul(hit)), hz = dz.mul(hit);
  const back = tz.lessThanEqual(min(tx, ty));
  const floorOrCeiling = back.not().and(ty.lessThanEqual(tx));
  const onFloor = floorOrCeiling.and(dy.lessThan(0)), onCeiling = floorOrCeiling.and(dy.greaterThanEqual(0));

  // Surfaces. Vacant rooms are bare and grey.
  const wall = select(home,
    choose3(h(2), vec3(color('#e9e0cc')), vec3(color('#e2e4d4')), vec3(color('#efe8dc'))),
    choose3(h(2), vec3(color('#dadcd8')), vec3(color('#d3d6d2')), vec3(color('#e2ddd2'))));
  const tatami = vec3(color('#c4b682')).mul(float(1).sub(max(lines(hx, .9, .012), lines(hz, 1.8, .012)).mul(.25)));
  const floorAlbedo = select(home, select(h(3).lessThan(.4), tatami, vec3(color('#8b6a4b'))), vec3(color('#5f6669')));
  const ceiling = vec3(color('#f1eee6'));
  // Back-wall furniture: a wardrobe or shelf at home, partitions and desks at work.
  const furnitureX = mix(xMin, xMax.sub(1.3), h(4)), furnitureW = mix(float(.8), float(1.3), h(5));
  const inFurniture = back.and(hx.greaterThan(furnitureX)).and(hx.lessThan(furnitureX.add(furnitureW))).and(hy.lessThan(yMin.add(mix(float(1.5), float(1.9), h(6)))));
  const inPartition = back.and(hy.lessThan(yMin.add(1.15)));
  let albedo = select(onFloor, floorAlbedo, select(onCeiling, ceiling, wall));
  // Retail gets shallow stocked shelves rather than office partitions. Geometry
  // behind an opaque interior-mapped pane would be invisible; keep it in the room shader.
  const shelfHeight=hy.sub(yMin),shelf=back.and(shelfHeight.mod(.58).lessThan(.055)).and(shelfHeight.lessThan(2.1));
  const goods=back.and(shelfHeight.mod(.58).greaterThan(.08)).and(shelfHeight.mod(.58).lessThan(.44)).and(shelfHeight.lessThan(2.1)).and(hx.mul(4).fract().lessThan(.78));
  const goodsColor=choose3(fract(floor(hx.mul(4)).mul(.37).add(h(10))),vec3(color('#bd7956')),vec3(color('#8caa8d')),vec3(color('#d3bc79')));
  const stockedShelves=select(shelf,vec3(color('#5d5145')),select(goods,goodsColor,albedo));
  const counter=back.and(shelfHeight.lessThan(.95)),display=back.and(shelfHeight.greaterThan(.95)).and(shelfHeight.lessThan(1.45)).and(hx.mul(1.4).fract().lessThan(.45));
  const cafeInterior=select(counter,vec3(color('#8c6950')),select(display,vec3(color('#d3bc79')),albedo));
  const garment=back.and(shelfHeight.greaterThan(.65)).and(shelfHeight.lessThan(1.8)).and(hx.mul(2).fract().lessThan(.72));
  const boutiqueInterior=select(garment,goodsColor,albedo);
  const retailAlbedo=select(h(11).lessThan(.4),stockedShelves,select(h(11).lessThan(.7),cafeInterior,boutiqueInterior));
  albedo = select(home, select(inFurniture, vec3(color('#5b4636')), albedo), select(office,select(inPartition, vec3(color('#8a9094')), albedo),retailAlbedo));
  albedo = mix(vec3(color('#bdbcb6')).mul(select(onFloor, float(.62), float(1))), albedo, lived);
  const geometryShade = select(back, float(1), select(onFloor, float(.74), select(onCeiling, float(.92), float(.8))))
    .mul(mix(float(1), float(.62), hz.negate().div(depth)));

  // Light: daylight from the window, and a ceiling lamp when the room is occupied.
  const occupancy = select(home, u.occupancy.home, select(office, u.occupancy.office, u.occupancy.shop));
  const lit = step(h(7), occupancy).mul(lived);
  const warm = vec3(color('#ffd6a2')), cool = vec3(color('#e6eeff'));
  const lampColor = select(home, select(h(8).lessThan(.45), cool, warm), select(office, cool, warm));
  const lampStrength = lit.mul(mix(float(.3), float(1.1), u.night));
  const toLamp = vec3(hx.sub(cellW.mul(.5)), hy.sub(yMax), hz.add(depth.mul(.5)));
  const falloff = float(1).div(toLamp.dot(toLamp).mul(.12).add(1));
  // Daylight through the window: brightest near the glass, falling off into the room.
  const daylight = u.night.oneMinus().mul(mix(float(.32), float(.12), hz.negate().div(depth))).add(.012);
  const lampDisk = onCeiling.and(vec3(toLamp.x, 0, toLamp.z).length().lessThan(.32));
  let interior = albedo.mul(geometryShade).mul(vec3(.9, .95, 1).mul(daylight).add(lampColor.mul(lampStrength).mul(falloff)))
    .add(select(lampDisk, lampColor.mul(lampStrength).mul(2.2), vec3(0, 0, 0)));

  // Window dressing just behind the glass.
  const dressing = h(9), tone = h(10);
  {
    const curtain = choose3(tone, vec3(color('#eee4cf')), vec3(color('#cdbd9b')), vec3(color('#b7c4c2')));
    const folds = mix(float(1), sin(px.mul(24)).mul(.06).add(.95), detail(px, .26));
    // Fabric just behind the glass catches the window light but stays below a sunlit wall.
    const curtainLight = curtain.mul(folds).mul(vec3(u.night.oneMinus().mul(.42).add(.012)).add(lampColor.mul(lampStrength).mul(.7)));
    const sideWidth = cellW.mul(mix(float(.14), float(.32), tone));
    const sides = px.lessThan(sideWidth).or(px.greaterThan(cellW.sub(sideWidth)));
    const sheer = dressing.greaterThan(.3).and(dressing.lessThan(.62));
    const drawn = dressing.greaterThanEqual(.62).and(dressing.lessThan(.9)).and(sides);
    const closed = dressing.greaterThanEqual(.9);
    const cover = select(sheer, float(.55), select(drawn.or(closed), float(1), float(0))).mul(lived).mul(select(home, float(1), float(0)));
    interior = mix(interior, curtainLight, cover);
  }
  {
    // Office and shop blinds, lowered from the head by a seeded amount.
    const lowered = mix(float(.15), float(1), tone);
    const blinds = home.not().and(dressing.lessThan(select(office, float(.45), float(.25)))).and(py.greaterThan(paneH.mul(lowered.oneMinus())));
    const slats = mix(float(1), lines(py, .08, .012).oneMinus().mul(.2).add(.8), detail(py, .08));
    const blindLight = vec3(color('#d8dad6')).mul(slats).mul(vec3(u.night.oneMinus().mul(.4).add(.012)).add(lampColor.mul(lampStrength).mul(.6)));
    interior = mix(interior, blindLight, select(blinds, float(1), float(0)).mul(lived));
  }

  // Far away the room collapses to its average so parallax cannot sparkle.
  const average = mix(wall, floorAlbedo, .3).mul(vec3(daylight).add(lampColor.mul(lampStrength).mul(.55)));
  const far = smoothstep(float(.14), float(.45), fwidth(px0));
  interior = mix(interior, mix(vec3(color('#9a9a94')).mul(daylight), average, lived), far);

  // Glass: tinted transmission and a Fresnel sky reflection.
  const facing = abs(dot(view, n));
  const reflectance=corporateTint?float(.52):select(home,float(.12),select(office,float(.25),float(.16)));
  const fresnel = clamp(facing.oneMinus().pow(5).mul(.72).add(reflectance), 0, 1);
  const incident = view, reflected = incident.sub(n.mul(dot(incident, n).mul(2)));
  const sky = smoothstep(float(-.12), float(.55), reflected.y);
  const reflection = mix(mix(vec3(color('#4f676e')), vec3(color('#c4d8df')), sky), mix(vec3(color('#090f1c')), vec3(color('#23364c')), sky), u.night).mul(fresnel);
  const tint=select(home,vec3(.78,.86,.86),select(office,choose3(h(12),vec3(.42,.62,.7),vec3(.5,.64,.68),vec3(.58,.65,.65)),vec3(.64,.74,.75)));
  const through = interior.mul(corporateTint?vec3(color(corporateTint)):tint).mul(fresnel.oneMinus()).mul(corporateTint?mix(float(.025),float(.8),u.night):float(1));
  return mix(reflection, through.add(reflection), verticalMask());
}

/** Window pane materials for both styles; the pane colour is the glass itself, the room is emission. */
export function windowMaterials(activity: WindowActivity, uniforms: WindowUniforms, toonRamp: THREE.Texture, corporateTint?:string) {
  const radiance = windowRadiance(uniforms,corporateTint);
  const anime = new THREE.MeshToonNodeMaterial({ color: '#1d2629', gradientMap: toonRamp });
  const miniature = new THREE.MeshStandardNodeMaterial({ color: '#172023', roughness: .08, metalness: 0 });
  for (const material of [anime, miniature]) {
    (material as unknown as { emissiveNode: Vec3Node }).emissiveNode = radiance;
    material.userData.windowPane = true;
    if(corporateTint)material.userData.corporateGlass=true;
    material.userData.windowActivity = activity;
  }
  return { anime, miniature };
}
