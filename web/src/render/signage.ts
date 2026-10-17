import * as THREE from 'three/webgpu';
import { color, cos, fract, length, mix, sin, smoothstep, texture, uv, vec2, vec3 } from 'three/tsl';
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
  { title: '月影レコード', sub: 'TSUKIKAGE RECORDS · NEW RELEASE', bg: '#34234f', ink: '#fff0ec', accent: '#ed659e' },
  { title: '空色通信', sub: 'SORAIRO MOBILE · CONNECT', bg: '#126c81', ink: '#f3ffff', accent: '#63e7db' },
  { title: '夜風シネマ', sub: 'YOKAZE CINEMA · TONIGHT', bg: '#182038', ink: '#fff0ce', accent: '#e97048' },
  { title: '花道スタジオ', sub: 'HANAMICHI LIVE · ON STAGE', bg: '#c63267', ink: '#fff8ed', accent: '#f7ce47' },
  { title: 'こもれび茶', sub: 'KOMOREBI TEA · FRESH', bg: '#315e43', ink: '#fff5d7', accent: '#b4d369' },
  { title: '流星ゲームズ', sub: 'RYUSEI GAMES · PLAY', bg: '#243d91', ink: '#effbff', accent: '#f47dca' },
  { title: '朝凪デザイン', sub: 'ASANAGI DESIGN · COLLECTION', bg: '#e9b274', ink: '#392a42', accent: '#f8edca' },
  { title: '灯台トラベル', sub: 'TODAI TRAVEL · EXPLORE', bg: '#286c9a', ink: '#fff8e7', accent: '#edca65' },
] as const;

export const SIGN_ATLAS_WIDTH = 512, SIGN_ATLAS_HEIGHT = 2048;
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
/** Billboard cells fill the bottom half: twelve 512×128 rows (4:1). */
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
  const variant=((program%BILLBOARDS.length)+BILLBOARDS.length)%BILLBOARDS.length;
  return art.shared(`video-screen-${variant}`, () => {
    const t = art.displayTime.mul(.035+variant*.012).add(variant*1.7), p = uv();
    const hue = p.x.mul(.55).add(p.y.mul(.3)).add(t);
    const field = vec3(cos(hue.mul(6.2832)), cos(hue.add(.33).mul(6.2832)), cos(hue.add(.67).mul(6.2832))).mul(.5).add(.5);
    const centre = vec2(sin(t.mul(3.1)).mul(.28).add(.5), cos(t.mul(2.3)).mul(.22).add(.5));
    const highlight = smoothstep(0, .42, length(p.sub(centre))).oneMinus();
    const scan = sin(p.y.mul(260)).mul(.035).add(.965);
    const atlas=streetSignSurface(art).map!,cell=billboardCell(variant);
    const poster=texture(atlas,p.mul(vec2(cell.u1-cell.u0,cell.v1-cell.v0)).add(vec2(cell.u0,cell.v0))).rgb;
    // Four distinct campaign treatments, shared by twelve fictional advertisers.
    // Global UVs and city time keep tiled panels synchronized without timers.
    const treatment=variant%4;
    const sweep=smoothstep(0,.12,length(p.x.sub(fract(t.mul(.4))))).oneMinus().mul(.3);
    const beat=sin(t.mul(4)).mul(.06).add(.94);
    const motion=treatment===0?mix(field,vec3(color('#fff4dc')),highlight.mul(.35))
      :treatment===1?vec3(color(BILLBOARDS[variant].accent)).mul(sweep.add(.25))
      :treatment===2?field.mul(smoothstep(.2,.5,length(p.sub(centre))).oneMinus())
      :vec3(color(BILLBOARDS[variant].accent)).mul(sin(p.x.mul(18).sub(t.mul(4))).mul(.3).add(.5));
    const picture=mix(poster.mul(beat),motion,treatment===1?.28:.18).mul(scan).mul(art.night.mul(1.3).add(1)).mul(art.litShop);
    // A black base keeps the screen independent of sun and shadow; the
    // picture is pure emission in both styles.
    const anime = new THREE.MeshToonNodeMaterial({ color: '#000000', gradientMap: art.toonRamp });
    const miniature = new THREE.MeshStandardNodeMaterial({ color: '#000000', roughness: .35 });
    for (const material of [anime, miniature]) (material as unknown as { emissiveNode: typeof picture }).emissiveNode = picture;
    return { anime, miniature };
  });
}

/** Building-scale graphic campaigns: sculptural orbit, ribbons, constellation and equalizer. */
export function mediaArtworkSurface(art:ArtMaterials,program=0){
 const variant=((program%4)+4)%4;
 return art.shared(`media-artwork-${variant}`,()=>{
  const p=uv(),t=art.displayTime.mul(.17+variant*.05).add(variant*1.7),q=p.sub(vec2(.5,.55));
  const palettes=[['#0d234a','#28d8ed','#fa614e'],['#33144f','#ff56ae','#f4d366'],['#072e34','#5ef1c9','#f6c26f'],['#1d1743','#9280ff','#f76c88']];
  const [bg,ink,accent]=palettes[variant],radius=length(q.mul(vec2(1, .8)));
  let hero;
  if(variant===0){
   const orbit=length(q.add(vec2(sin(t).mul(.12),cos(t.mul(.7)).mul(.08))).mul(vec2(1,1.4)));
   hero=smoothstep(.012,.035,orbit.sub(.25).abs()).oneMinus().add(smoothstep(.04,.12,length(q.sub(vec2(cos(t).mul(.27),sin(t).mul(.18))))).oneMinus());
  }else if(variant===1){
   const wave=sin(p.y.mul(12).add(t)).mul(.16).add(.5);
   hero=smoothstep(.03,.1,p.x.sub(wave).abs()).oneMinus().add(smoothstep(.02,.06,p.x.sub(wave.add(.22)).abs()).oneMinus().mul(.7));
  }else if(variant===2){
   const cell=fract(p.mul(vec2(6,9)).add(vec2(sin(t).mul(.12),t.mul(.08)))).sub(.5);
   hero=smoothstep(.035,.12,length(cell)).oneMinus().mul(sin(p.x.mul(9).add(p.y.mul(6)).add(t)).mul(.35).add(.65));
  }else{
   const column=fract(p.x.mul(9)).sub(.5).abs(),height=sin(p.x.mul(13).add(t)).mul(.2).add(.5);
   hero=smoothstep(.25,.4,column).oneMinus().mul(smoothstep(height,height.add(.025),p.y).oneMinus()).mul(smoothstep(.19,.22,p.y));
  }
  const background=mix(color(bg),color(accent),smoothstep(0,.5,radius).oneMinus().mul(.3));
  const graphic=mix(background,color(ink),hero.clamp(0,1));
  const atlas=streetSignSurface(art).map!,cell=billboardCell(variant*3);
  const labelUV=vec2(p.x,p.y.div(.17).clamp(0,1));
  const label=texture(atlas,labelUV.mul(vec2(cell.u1-cell.u0,cell.v1-cell.v0)).add(vec2(cell.u0,cell.v0))).rgb;
  const picture=mix(label,graphic,smoothstep(.16,.18,p.y)).mul(art.night.mul(1.3).add(1.1)).mul(art.litShop);
  const anime=new THREE.MeshToonNodeMaterial({color:'#000000',gradientMap:art.toonRamp}),miniature=new THREE.MeshStandardNodeMaterial({color:'#000000',roughness:.4});
  for(const material of [anime,miniature]){(material as unknown as {emissiveNode:typeof picture}).emissiveNode=picture;material.userData.mediaArtwork=variant;}
  return {anime,miniature};
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

export type ScreenLayout = 'flat' | 'curved' | 'triptych';
/** Global UVs span all panels; sag stays behind the front envelope. */
export function screenPanels(width:number,height:number,layout:ScreenLayout):THREE.BufferGeometry[]{
 const panels:THREE.BufferGeometry[]=[];
 const count=layout==='triptych'?3:1;
 for(let panel=0;panel<count;panel++){
  const gap=count===1?0:.08,pw=(width-gap*(count-1))/count;
  const left=-width/2+panel*(pw+gap),g=new THREE.PlaneGeometry(pw,height,layout==='curved'?24:1,1);
  const p=g.getAttribute('position'),tex=g.getAttribute('uv');
  for(let i=0;i<p.count;i++){
   const x=left+(p.getX(i)+pw/2),z=layout==='curved'?-Math.min(.28,width*.025)*(1-Math.cos(x/width*Math.PI)):0;
   p.setXYZ(i,x,p.getY(i),z);tex.setX(i,Math.max(0,Math.min(1,(x+width/2)/width)));
  }
  g.computeVertexNormals();panels.push(g);
 }
 return panels;
}

/** Large podium-front display: a bezelled video wall or a printed wall ad. */
export function addFacadeFeature(kit: Kit, art: ArtMaterials, frame: THREE.Material, feature: FacadeFeature, m: { frontZ: number; width: number; bottom: number; height: number; seed: number; program?: number; layout?: ScreenLayout; screenPoster?: boolean }) {
  const height = m.height, width = Math.min(m.width, feature === 'screen'||m.screenPoster ? height * 16 / 9 : height * 4);
  if (width < 3 || height < 2) return false;
  const y = m.bottom + height / 2;
  const layout=m.layout??'flat';
  if(layout==='flat')kit.box(0, y, m.frontZ + .17, width + .4, height + .4, .3, frame);
  const faces=screenPanels(width,height,layout);
  if(feature==='ad'){
   const c=billboardCell(m.program??m.seed+2);
   for(const face of faces){const tex=face.getAttribute('uv');for(let i=0;i<tex.count;i++)tex.setXY(i,c.u0+tex.getX(i)*(c.u1-c.u0),c.v0+tex.getY(i)*(c.v1-c.v0));}
  }
  for(const face of faces){
   if(layout!=='flat'){
    const backing=face.clone().scale(1.015,1.04,1).translate(0,y,m.frontZ+.30);kit.add(backing,frame);
   }
   face.translate(0,y,m.frontZ+.33);
   kit.add(face,feature==='screen'?videoScreenSurface(art,m.program??0):streetSignSurface(art));
  }
  // Tiled screens retain dark seams; all tiles share one material and clock.
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
  return (b.floors >= 12 && (b.kind === 'office' || b.kind === 'apartment')) || (b.kind==='office' && b.floors>=6 && b.floors<=11)||(['shop','cafe'].includes(b.kind)&&b.floors>=3&&b.floors<=20);
}

/**
 * Up to six station-area buildings carry a podium display, nearest first and
 * at least 30 m apart. Ranks alternate video wall and printed ad.
 */
export function selectFacadeFeatures(world: Pick<World, 'nodes' | 'buildings'> & Partial<Pick<World,'roads'>>, eligible?:ReadonlySet<number>,preferredScreens?:ReadonlySet<number>): Map<number, FacadeFeature> {
  const result = new Map<number, FacadeFeature>();
  const station = world.nodes.find(node => node.kind === 'station');
  const centers=station?[station]:[];
  if(preferredScreens?.size){
    const degree=new Map<number,number>();for(const r of world.roads??[]){degree.set(r.from,(degree.get(r.from)??0)+1);degree.set(r.to,(degree.get(r.to)??0)+1);}
    centers.push(...world.nodes.filter(n=>(degree.get(n.id)??0)>=3&&n.id!==station?.id));
  }
  if (!centers.length) return result;
  const chosen: Building[] = [];
  const candidates = world.buildings.filter(b=>canHostFacadeFeature(b)&&(!eligible||eligible.has(b.id)))
    .map(b => ({ b, distance: Math.min(...centers.map(n=>Math.hypot(b.x-n.x,b.z-n.z))) }))
    .filter(entry => entry.distance <= FEATURE_RADIUS)
    .sort((a, b) => a.distance - b.distance || a.b.id - b.b.id);
  for (const { b } of candidates) {
    if (chosen.some(other => Math.hypot(other.x - b.x, other.z - b.z) < FEATURE_SPACING)) continue;
    result.set(b.id, chosen.length % 2 === 0 ? 'screen' : 'ad');
    chosen.push(b);
    if (chosen.length >= MAX_FACADE_FEATURES) break;
  }
  if(preferredScreens?.size){
    const ids=[...result.keys()],preferred=ids.filter(id=>preferredScreens.has(id));
    const count=Math.min(Math.ceil(MAX_FACADE_FEATURES/2),Math.max(preferred.length,[...result.values()].filter(v=>v==='screen').length));
    const order=[...preferred,...ids.filter(id=>!preferredScreens.has(id)&&result.get(id)==='screen'),...ids.filter(id=>!preferredScreens.has(id)&&result.get(id)==='ad')];
    const screens=new Set(order.slice(0,count));for(const id of ids)result.set(id,screens.has(id)?'screen':'ad');
  }
  return result;
}

/** Per-building signage context for one world snapshot. */
export function signageContexts(world: Pick<World, 'nodes' | 'buildings'> & Partial<Pick<World,'roads'>>, eligible?:ReadonlySet<number>,preferredScreens?:ReadonlySet<number>): Map<number, SignageContext> {
  const station = world.nodes.find(node => node.kind === 'station');
  const features = selectFacadeFeatures(world,eligible,preferredScreens);
  const contexts = new Map<number, SignageContext>();
  for (const b of world.buildings) {
    const feature = features.get(b.id);
    const proximity = station ? stationProximity(Math.hypot(b.x - station.x, b.z - station.z)) : feature?2:0;
    contexts.set(b.id, feature ? { proximity, feature } : { proximity });
  }
  return contexts;
}
