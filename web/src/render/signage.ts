import * as THREE from 'three/webgpu';
import { color, cos, length, mix, sin, smoothstep, texture, uv, vec2, vec3 } from 'three/tsl';
import { canvasTexture, type ArtMaterials } from './materials';
import type { Kit } from './kit';
import type { Building, World } from '../types';

/**
 * Commercial signage: vertical blade signs, rooftop billboard frames and a
 * few large station-area wall ads and video screens. Every name and brand is
 * invented for the city; none imitate real businesses.
 */
export const BLADE_LABELS = [
  'カラオケ', 'パチンコ', '薬局', '学習塾', '居酒屋', 'ラーメン', '歯科', '英会話',
  '整体院', '書店', '焼肉', '麻雀', '漫画喫茶', '質屋', '美容室', '不動産',
] as const;
const BLADE_COLORS: Array<[string, string]> = [
  ['#c4402f', '#fff1cf'], ['#f0e6c8', '#a8352a'], ['#2f6f8f', '#f6efd9'], ['#f3ead2', '#25525f'],
  ['#b9452f', '#fff4d8'], ['#e8b84a', '#5a2a1c'], ['#e9f0ee', '#2b6a63'], ['#365e83', '#fff3d6'],
  ['#4f7a52', '#f7f0d8'], ['#efe6cc', '#3a4a52'], ['#7a2e2a', '#ffe9c2'], ['#26473f', '#f2e8c9'],
  ['#e05a3a', '#fffaf0'], ['#3b3b44', '#f1d79a'], ['#f4d9de', '#8a2f45'], ['#e7ecef', '#1f4d7a'],
];
export const BILLBOARDS = [
  { title: '星屑サイダー', sub: 'HOSHIKUZU CIDER', bg: '#2b6f9e', ink: '#fff4d6', accent: '#f2c14e' },
  { title: 'つばめ不動産', sub: 'TSUBAME ESTATE', bg: '#f2e3b3', ink: '#b2392b', accent: '#2f5d73' },
  { title: '夏祭り', sub: 'RIVERSIDE FESTIVAL 7.27', bg: '#c7402f', ink: '#fff1cf', accent: '#f6c445' },
  { title: '青葉英語学院', sub: 'AOBA ENGLISH SCHOOL', bg: '#e9eef0', ink: '#24525f', accent: '#d2583c' },
] as const;

export const SIGN_ATLAS_WIDTH = 512, SIGN_ATLAS_HEIGHT = 1024;
const BLADE_W = 64, BLADE_H = 256, BILLBOARD_H = 128;
/** Half a texel of gutter keeps mip levels from sampling the neighboring cell. */
const INSET = 2;

export interface AtlasCell { u0: number; u1: number; v0: number; v1: number }
function cell(x: number, y: number, w: number, h: number): AtlasCell {
  // CanvasTexture rows are flipped: canvas row 0 is v=1.
  return {
    u0: (x + INSET) / SIGN_ATLAS_WIDTH, u1: (x + w - INSET) / SIGN_ATLAS_WIDTH,
    v0: 1 - (y + h - INSET) / SIGN_ATLAS_HEIGHT, v1: 1 - (y + INSET) / SIGN_ATLAS_HEIGHT,
  };
}
/** Blade cells fill the top half of the atlas: 8 columns by 2 rows of 64×256. */
export function bladeCell(slot: number): AtlasCell {
  const i = ((slot % BLADE_LABELS.length) + BLADE_LABELS.length) % BLADE_LABELS.length;
  return cell((i % 8) * BLADE_W, Math.floor(i / 8) * BLADE_H, BLADE_W, BLADE_H);
}
/** Billboard cells fill the bottom half: four 512×128 rows (4:1). */
export function billboardCell(slot: number): AtlasCell {
  const i = ((slot % BILLBOARDS.length) + BILLBOARDS.length) % BILLBOARDS.length;
  return cell(0, 2 * BLADE_H + i * BILLBOARD_H, SIGN_ATLAS_WIDTH, BILLBOARD_H);
}

function drawAtlas(ctx: CanvasRenderingContext2D) {
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  BLADE_LABELS.forEach((label, i) => {
    const x = (i % 8) * BLADE_W, y = Math.floor(i / 8) * BLADE_H, [bg, ink] = BLADE_COLORS[i];
    ctx.fillStyle = bg; ctx.fillRect(x, y, BLADE_W, BLADE_H);
    ctx.strokeStyle = ink; ctx.lineWidth = 2; ctx.strokeRect(x + 5, y + 5, BLADE_W - 10, BLADE_H - 10);
    const chars = [...label], size = Math.min(46, 216 / chars.length);
    ctx.fillStyle = ink; ctx.font = `700 ${size}px "Hiragino Kaku Gothic ProN", sans-serif`;
    chars.forEach((character, index) => ctx.fillText(character, x + BLADE_W / 2, y + BLADE_H / 2 + (index - (chars.length - 1) / 2) * size * 1.08));
  });
  BILLBOARDS.forEach((board, i) => {
    const y = 2 * BLADE_H + i * BILLBOARD_H;
    ctx.fillStyle = board.bg; ctx.fillRect(0, y, SIGN_ATLAS_WIDTH, BILLBOARD_H);
    // A stacked-bar brand mark; plain rectangles keep the atlas cheap to draw.
    ctx.fillStyle = board.accent; for (let bar = 0; bar < 3; bar++) ctx.fillRect(28 + bar * 6, y + 28 + bar * 26, 72 - bar * 12, 18);
    ctx.fillRect(0, y + BILLBOARD_H - 14, SIGN_ATLAS_WIDTH, 6);
    ctx.fillStyle = board.ink; ctx.font = '800 50px "Hiragino Kaku Gothic ProN", sans-serif';
    ctx.fillText(board.title, 300, y + 52);
    ctx.font = '600 20px sans-serif'; ctx.fillText(board.sub, 300, y + 98);
    ctx.strokeStyle = board.ink; ctx.lineWidth = 3; ctx.strokeRect(4, y + 4, SIGN_ATLAS_WIDTH - 8, BILLBOARD_H - 8);
  });
}

/** One shared atlas surface for every blade, rooftop board and wall ad; lit after dark. */
export function streetSignSurface(art: ArtMaterials) {
  return art.shared('street-sign-atlas', () => {
    const atlas = canvasTexture(SIGN_ATLAS_WIDTH, SIGN_ATLAS_HEIGHT, drawAtlas);
    art.onDispose(() => atlas.dispose());
    const anime = new THREE.MeshToonNodeMaterial({ map: atlas, gradientMap: art.toonRamp, side: THREE.DoubleSide });
    const miniature = new THREE.MeshStandardNodeMaterial({ map: atlas, roughness: .5, side: THREE.DoubleSide });
    for (const material of [anime, miniature]) {
      const emission = texture(atlas).rgb.mul(art.night.mul(.85).add(.04)).mul(art.litShop);
      (material as unknown as { emissiveNode: typeof emission }).emissiveNode = emission;
    }
    return { anime, miniature };
  });
}

/**
 * Self-lit video wall: slow drifting color fields and a soft highlight with
 * faint scanlines. It is generated from city time, not a video file.
 */
export function videoScreenSurface(art: ArtMaterials, program=0) {
  const variant=((program%3)+3)%3;
  return art.shared(`video-screen-${variant}`, () => {
    const t = art.time.mul(.035+variant*.012).add(variant*1.7), p = uv();
    const hue = p.x.mul(.55).add(p.y.mul(.3)).add(t);
    const field = vec3(cos(hue.mul(6.2832)), cos(hue.add(.33).mul(6.2832)), cos(hue.add(.67).mul(6.2832))).mul(.5).add(.5);
    const centre = vec2(sin(t.mul(3.1)).mul(.28).add(.5), cos(t.mul(2.3)).mul(.22).add(.5));
    const highlight = smoothstep(.42, 0, length(p.sub(centre)));
    const scan = sin(p.y.mul(260)).mul(.035).add(.965);
    const atlas=streetSignSurface(art).map!,cell=billboardCell(variant);
    const poster=texture(atlas,p.mul(vec2(cell.u1-cell.u0,cell.v1-cell.v0)).add(vec2(cell.u0,cell.v0))).rgb;
    const motion=mix(field,vec3(color('#fff4dc')),highlight.mul(.35));
    const picture=mix(poster,motion,.12).mul(scan).mul(art.night.mul(.45).add(.8)).mul(art.litShop);
    // A black base keeps the screen independent of sun and shadow; the
    // picture is pure emission in both styles.
    const anime = new THREE.MeshToonNodeMaterial({ color: '#000000', gradientMap: art.toonRamp });
    const miniature = new THREE.MeshStandardNodeMaterial({ color: '#000000', roughness: .35 });
    for (const material of [anime, miniature]) (material as unknown as { emissiveNode: typeof picture }).emissiveNode = picture;
    return { anime, miniature };
  });
}

function mapPlane(width: number, height: number, c: AtlasCell) {
  const plane = new THREE.PlaneGeometry(width, height), uvs = plane.getAttribute('uv');
  for (let i = 0; i < uvs.count; i++) uvs.setXY(i, c.u0 + uvs.getX(i) * (c.u1 - c.u0), c.v0 + uvs.getY(i) * (c.v1 - c.v0));
  return plane;
}

/** Station-area intensity: 2 inside the station core, 1 in its fringe, 0 elsewhere. */
export function stationProximity(distance: number): 0 | 1 | 2 {
  return distance < 140 ? 2 : distance < 300 ? 1 : 0;
}
export type FacadeFeature = 'screen' | 'ad';
/** World context a building's signage needs beyond the building itself. */
export interface SignageContext { proximity: 0 | 1 | 2; feature?: FacadeFeature }

/**
 * Stacked blade panels on a bracketed frame projecting from the facade at
 * local (x, frontZ), clear of the street-side lot line. Each panel is a
 * different tenant; panel height follows the 1:4 atlas cell.
 */
export function addBladeSign(kit: Kit, art: ArtMaterials, frame: THREE.Material, m: { x: number; frontZ: number; maxDepth: number; bottom: number; maxHeight: number; panels: number; seed: number }) {
  const depth = Math.min(.85, m.maxDepth, m.maxHeight / 4);
  if (depth < .35) return 0;
  const panelH = depth * 4, gap = .12;
  const panels = Math.max(1, Math.min(m.panels, Math.floor((m.maxHeight + gap) / (panelH + gap))));
  const total = panels * panelH + (panels - 1) * gap, z = m.frontZ + .07 + depth / 2, y = m.bottom + total / 2;
  kit.box(m.x, y, z, .19, total + .12, depth + .06, frame);
  for (const end of [-1, 1]) kit.box(m.x, y + end * (total / 2 - .24), m.frontZ + .02, .13, .1, .2, frame);
  const material = streetSignSurface(art);
  for (let p = 0; p < panels; p++) {
    const c = bladeCell(m.seed + p * 5), py = m.bottom + p * (panelH + gap) + panelH / 2;
    for (const side of [-1, 1]) {
      const face = mapPlane(depth - .045, panelH - .04, c);
      face.rotateY(side * Math.PI / 2);
      face.translate(m.x + side * .101, py, z);
      kit.add(face, material);
    }
  }
  return panels;
}

/**
 * Steel-frame billboard standing on a flat roof. The panel clears the water
 * tank and service boxes (≤ 2.1 m above the roof deck) and faces the street.
 */
export function addRooftopBillboard(kit: Kit, art: ArtMaterials, frame: THREE.Material, roof: { x: number; z: number; width: number; depth: number; top: number }, seed: number) {
  const width = Math.min(roof.width - 1, 10);
  if (width < 4 || roof.depth < 5) return false;
  const height = width / 4, bottom = roof.top + 2.4, z = roof.z - roof.depth * .05;
  for (const x of [-width / 2 + .3, width / 2 - .3]) for (const dz of [-.55, .1]) kit.box(roof.x + x, roof.top + 1.2 + height / 2 + .3, z + dz, .12, bottom - roof.top + height - .4, .12, frame);
  kit.box(roof.x, bottom - .1, z - .2, width + .2, .12, .9, frame);
  kit.box(roof.x, bottom + height / 2, z, width + .24, height + .24, .2, frame);
  const face = mapPlane(width, height, billboardCell(seed));
  face.translate(roof.x, bottom + height / 2, z + .105);
  kit.add(face, streetSignSurface(art));
  const lamp = art.get('#fff1c4', .5, true, 'shop');
  for (let i = 0; i < 3; i++) {
    const lx = roof.x + (i - 1) * width * .33;
    kit.box(lx, bottom + height + .3, z + .35, .06, .06, .6, frame);
    kit.box(lx, bottom + height + .27, z + .66, .34, .12, .14, lamp);
  }
  return true;
}

/** Large podium-front display: a bezelled video wall or a printed wall ad. */
export function addFacadeFeature(kit: Kit, art: ArtMaterials, frame: THREE.Material, feature: FacadeFeature, m: { frontZ: number; width: number; bottom: number; height: number; seed: number; program?: number }) {
  const height = m.height, width = Math.min(m.width, feature === 'screen' ? height * 16 / 9 : height * 4);
  if (width < 3 || height < 2) return false;
  const y = m.bottom + height / 2;
  kit.box(0, y, m.frontZ + .17, width + .4, height + .4, .3, frame);
  const face = feature === 'screen' ? new THREE.PlaneGeometry(width, height) : mapPlane(width, height, billboardCell(m.seed + 2));
  face.translate(0, y, m.frontZ + .33);
  kit.add(face, feature === 'screen' ? videoScreenSurface(art,m.program??0) : streetSignSurface(art));
  return true;
}

/** Commercial buildings of 4–11 floors may carry a rooftop board, more often near the station. */
export function hasRooftopBillboard(b: Pick<Building, 'kind' | 'floors' | 'anchor' | 'appearance_seed'>, proximity: number) {
  if (b.anchor || !['shop', 'cafe', 'office'].includes(b.kind) || b.floors < 4 || b.floors > 11) return false;
  const chance = [.15, .4, .7][proximity] ?? .15;
  return ((b.appearance_seed >>> 13) % 100) / 100 < chance;
}

export const MAX_FACADE_FEATURES = 6;
const FEATURE_RADIUS = 220, FEATURE_SPACING = 30;

/** A building can host a podium display: towers, and setback offices of 4–11 floors. */
export function canHostFacadeFeature(b: Building) {
  if (b.stage !== 'open' || b.anchor || b.business_active === false) return false;
  return (b.floors >= 12 && (b.kind === 'office' || b.kind === 'apartment')) || (b.kind==='office' && b.floors>=6 && b.floors<=11)||(['shop','cafe'].includes(b.kind)&&b.floors>=3&&b.floors<=11);
}

/**
 * Up to six station-area buildings carry a podium display, nearest first and
 * at least 30 m apart. Ranks alternate video wall and printed ad.
 */
export function selectFacadeFeatures(world: Pick<World, 'nodes' | 'buildings'>): Map<number, FacadeFeature> {
  const result = new Map<number, FacadeFeature>();
  const station = world.nodes.find(node => node.kind === 'station');
  if (!station) return result;
  const chosen: Building[] = [];
  const candidates = world.buildings.filter(canHostFacadeFeature)
    .map(b => ({ b, distance: Math.hypot(b.x - station.x, b.z - station.z) }))
    .filter(entry => entry.distance <= FEATURE_RADIUS)
    .sort((a, b) => a.distance - b.distance || a.b.id - b.b.id);
  for (const { b } of candidates) {
    if (chosen.some(other => Math.hypot(other.x - b.x, other.z - b.z) < FEATURE_SPACING)) continue;
    result.set(b.id, chosen.length % 2 === 0 ? 'screen' : 'ad');
    chosen.push(b);
    if (chosen.length >= MAX_FACADE_FEATURES) break;
  }
  return result;
}

/** Per-building signage context for one world snapshot. */
export function signageContexts(world: Pick<World, 'nodes' | 'buildings'>): Map<number, SignageContext> {
  const station = world.nodes.find(node => node.kind === 'station');
  const features = selectFacadeFeatures(world);
  const contexts = new Map<number, SignageContext>();
  for (const b of world.buildings) {
    const proximity = station ? stationProximity(Math.hypot(b.x - station.x, b.z - station.z)) : 0;
    const feature = features.get(b.id);
    contexts.set(b.id, feature ? { proximity, feature } : { proximity });
  }
  return contexts;
}
