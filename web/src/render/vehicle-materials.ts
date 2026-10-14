import * as THREE from 'three/webgpu';
import { attribute, color, cos, float, floor, mix, mod, normalGeometry, positionGeometry, positionLocal, select, sin, smoothstep, step, texture, uv, vec2, vec3 } from 'three/tsl';
import type { ArtMaterials } from './materials';

/**
 * One material pair for every road vehicle and the bus, plus one for livery
 * panels. Per-instance attributes (set by VehicleFleet / CityBuses):
 *  - vstate: (odometer m, front-wheel steer rad, unused, brake 0..1)
 *  - vtint:  paint colour applied through the atlas paint mask
 *  - vlivery: livery tile index, or -1 for none (livery panels only)
 */
const ATLAS_URL = '/art/vehicles/vehicle-atlas.png';
const MASK_URL = '/art/vehicles/vehicle-mask.png';
type Surface = THREE.MeshStandardNodeMaterial | THREE.MeshToonNodeMaterial;

type BitmapLoad = (url: string, onLoad: (bitmap: ImageBitmap) => void, onError: () => void) => void;

function loadBitmap(url: string, onLoad: (bitmap: ImageBitmap) => void, onError: () => void) {
  new THREE.ImageBitmapLoader().setOptions({ imageOrientation: 'flipY' }).load(url, onLoad, undefined, onError);
}

/** Free a texture and close the decoded bitmap it owns, if any. */
function release(texture: THREE.Texture) {
  const image = texture.image as { close?: () => void } | null;
  texture.dispose();
  image?.close?.();
}

/**
 * A texture node that starts on a neutral 1x1 placeholder and switches to
 * the decoded image. GPU storage is sized on first upload, so the image goes
 * into a new Texture rather than into the placeholder. The bitmap stays open
 * while its texture is live, so a lost device can re-upload it; it is closed
 * when ArtMaterials is disposed. A load that completes after that is closed
 * at once and never installed on the retired node.
 */
export function streamedTexture(url: string, colorSpace: THREE.ColorSpace, art: Pick<ArtMaterials, 'onDispose'>, fallback: [number, number, number, number], load: BitmapLoad | null = defaultLoad()) {
  const first = placeholder(fallback);
  const node = texture(first);
  let current: THREE.Texture = first, disposed = false;
  art.onDispose(() => { disposed = true; release(current); });
  load?.(url, bitmap => {
    if (disposed) { bitmap.close(); return; }
    const loaded = new THREE.Texture(bitmap as unknown as HTMLImageElement);
    loaded.flipY = false; loaded.colorSpace = colorSpace; loaded.anisotropy = 8; loaded.needsUpdate = true;
    node.value = loaded; release(current); current = loaded;
  }, () => { /* Missing art keeps the placeholder rather than failing the city. */ });
  return node;
}

/** Tests and headless tools have no image decoding; the placeholder stays. */
function defaultLoad(): BitmapLoad | null {
  return typeof createImageBitmap === 'function' && typeof fetch === 'function' ? loadBitmap : null;
}

function placeholder(rgba: [number, number, number, number] = [200, 200, 200, 255]) {
  const t = new THREE.DataTexture(new Uint8Array(rgba), 1, 1);
  t.needsUpdate = true;
  return t;
}

/**
 * Wheel roll and steer in body space. The POSITION sub-build sees the
 * geometry before the instance transform, which is applied afterwards.
 */
export function wheelPosition() {
  const w = attribute<'vec4'>('wheel', 'vec4'), st = attribute<'vec4'>('vstate', 'vec4'), g = positionLocal;
  const angle = st.x.div(w.z.max(.01)), ca = cos(angle), sa = sin(angle);
  const dy = g.y.sub(w.x), dz = g.z.sub(w.y);
  const ry = w.x.add(dy.mul(ca)).sub(dz.mul(sa));
  const rz = w.y.add(dy.mul(sa)).add(dz.mul(ca));
  const steer = select(w.w.notEqual(0), st.y, float(0)), cs = cos(steer), ss = sin(steer);
  const dx = g.x.sub(w.w), dz2 = rz.sub(w.y);
  const sx = w.w.add(dx.mul(cs)).add(dz2.mul(ss));
  const sz = w.y.sub(dx.mul(ss)).add(dz2.mul(cs));
  return select(w.z.greaterThan(0), vec3(sx, ry, sz), g);
}

export function vehicleMaterial(art: ArtMaterials): Surface {
  return art.shared('vehicle-atlas', () => {
    // Until the mask arrives: plain paint, no lamps or glass.
    const atlas = streamedTexture(ATLAS_URL, THREE.SRGBColorSpace, art, [200, 200, 200, 255]);
    const masks = streamedTexture(MASK_URL, THREE.NoColorSpace, art, [255, 0, 0, 255]);
    const base = atlas.sample(uv()), mask = masks.sample(uv());
    const tint = attribute<'vec3'>('vtint', 'vec3'), state = attribute<'vec4'>('vstate', 'vec4');
    const part = attribute<'float'>('vpart', 'float'), lateral = attribute<'float'>('lateral', 'float').abs();
    // Paint was stored as grey at 0.8 of its reference luminance.
    const painted = mix(base.rgb, base.rgb.mul(tint).mul(1.25), mask.r);
    // End faces carry the elevation's lamp row across the whole width: keep
    // lamps at the corners and turn the middle of that row into grille.
    // Includes sloped noses and tails, which carry the same lamp rows.
    const endFace = part.equal(1).and(normalGeometry.z.abs().greaterThan(.35));
    const corner = smoothstep(.36, .5, lateral).mul(smoothstep(.99, .9, lateral));
    const lampShape = select(endFace, corner, float(1));
    const lamp = mask.g.mul(lampShape);
    // On painted bodies the elevation's nose and tail columns are lens and
    // chrome; spread across the width they read as a pale bar. Render any
    // non-paint, non-lamp end pixel as dark trim: grille, bumper, housings.
    // Fronts only: tails keep the elevation (box bodies are unpainted there).
    const trim = endFace.and(positionGeometry.z.greaterThan(0)).and(attribute<'float'>('tinted', 'float').greaterThan(.5)).and(mask.r.lessThan(.5));
    const shaped = painted.mul(select(endFace, mask.g.mul(corner.oneMinus()).mul(.7).oneMinus(), float(1)));
    // Lens and chrome survive only at the corners, where real lamps sit; the
    // middle is one flat grille colour rather than a stretched texture row.
    const surface = select(trim, mix(color('#1c2022'), shaped, corner), shaped);
    const front = positionGeometry.z.greaterThan(0);
    const lampColor = select(front, color('#fff0c8'), color('#ff3b26'));
    const lampPower = select(front, art.night.mul(2.2), art.night.mul(1.3).add(state.w.mul(1.6)));
    const sign = select(part.equal(3), color('#fff2cf').mul(art.night.mul(1.4).add(.2)), vec3(0, 0, 0));
    const glass = mask.b.mul(select(part.equal(2), float(0), float(1)));
    const sky = art.glassRadiance();
    const pair = {
      anime: new THREE.MeshToonNodeMaterial({ gradientMap: art.toonRamp }),
      miniature: new THREE.MeshStandardNodeMaterial({ roughness: .4, metalness: .08 }),
    };
    const position = wheelPosition();
    for (const [style, material] of Object.entries(pair) as [keyof typeof pair, Surface][]) {
      material.colorNode = surface;
      material.positionNode = position;
      const emission = lampColor.mul(lamp).mul(lampPower).add(sign).add(sky.mul(glass).mul(style === 'anime' ? .55 : 1));
      (material as unknown as { emissiveNode: typeof emission }).emissiveNode = emission;
    }
    // Glass and fresh paint are glossier; rain glosses everything a little more.
    // Perimeter strips are flat across the full width, so a sun highlight on
    // them becomes a hard stripe instead of a curved glint: keep them satin.
    const strip = part.equal(1);
    const dry = select(strip, mix(float(.72), float(.12), mask.b), mix(float(.42), float(.1), mask.b));
    const wet = select(strip, mix(float(.5), float(.08), mask.b), mix(float(.24), float(.06), mask.b));
    pair.miniature.roughnessNode = mix(dry, wet, art.wet);
    return pair;
  });
}

/**
 * Livery tiles: 2 columns x 8 rows. Artwork is authored on a 512x256 tile and
 * drawn at LIVERY_SCALE: half scale is legible at street distance and keeps the
 * canvas texture near 2.7 MiB with mips (canvas textures escape the file audit).
 */
export const LIVERIES = ['まちタクシー', '青葉交通', 'ひだまり便', 'つばめ急便', 'まち郵便', 'あおば運輸', 'やまびこ引越センター', 'ふじ冷凍便', 'みどり交通', 'さくら無線', '白鳥クリーニング', 'まちの八百屋', 'こばやし建材'] as const;
const LIVERY_ROWS = 8, LIVERY_SCALE = .5;
export type Livery = typeof LIVERIES[number];
const TILE_W = 512, TILE_H = 256;

function drawLivery(ctx: CanvasRenderingContext2D, livery: Livery, x: number, y: number) {
  const gothic = '"Hiragino Kaku Gothic ProN", "Noto Sans JP", "Yu Gothic", sans-serif';
  ctx.save(); ctx.translate(x, y);
  ctx.textBaseline = 'middle'; ctx.textAlign = 'left';
  const text = (s: string, px: number, tx: number, ty: number, fill: string, weight = 700, family = gothic) => {
    ctx.font = `${weight} ${px}px ${family}`; ctx.fillStyle = fill; ctx.fillText(s, tx, ty);
  };
  switch (livery) {
    case 'まちタクシー':
      ctx.fillStyle = '#2f9e5b'; ctx.beginPath(); ctx.arc(70, 128, 34, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = '#f5f1e4'; ctx.beginPath(); ctx.arc(70, 128, 14, 0, Math.PI * 2); ctx.fill();
      text('まちタクシー', 54, 128, 130, '#f2eee2'); break;
    case '青葉交通':
      text('青葉交通', 64, 120, 128, '#cfb26a', 600);
      ctx.strokeStyle = '#cfb26a'; ctx.lineWidth = 4; ctx.strokeRect(36, 88, 60, 80); break;
    case 'ひだまり便':
      ctx.fillStyle = '#e9a92c'; ctx.fillRect(0, 196, TILE_W, 44);
      ctx.fillStyle = '#ef7d2d'; ctx.beginPath(); ctx.arc(96, 104, 58, 0, Math.PI * 2); ctx.fill();
      text('ひだまり便', 70, 180, 108, '#4b3424'); break;
    case 'つばめ急便':
      ctx.fillStyle = '#1f4f8f'; ctx.fillRect(0, 206, TILE_W, 10); ctx.fillRect(0, 224, TILE_W, 10);
      ctx.save(); ctx.translate(96, 100); ctx.beginPath();
      ctx.moveTo(-60, -10); ctx.quadraticCurveTo(-10, -6, 10, 18); ctx.quadraticCurveTo(30, -12, 66, -34);
      ctx.quadraticCurveTo(34, 0, 16, 34); ctx.lineTo(-6, 26); ctx.quadraticCurveTo(-26, 4, -60, -10); ctx.fill(); ctx.restore();
      text('つばめ急便', 66, 180, 104, '#1f4f8f'); break;
    case 'まち郵便':
      ctx.fillStyle = '#f6f1e6'; ctx.fillRect(0, 150, TILE_W, 22);
      text('まち郵便', 62, 40, 96, '#f6f1e6');
      ctx.strokeStyle = '#f6f1e6'; ctx.lineWidth = 6; ctx.strokeRect(330, 66, 86, 60);
      ctx.beginPath(); ctx.moveTo(330, 66); ctx.lineTo(373, 102); ctx.lineTo(416, 66); ctx.stroke(); break;
    case 'あおば運輸':
      ctx.fillStyle = '#2f9a4a'; ctx.fillRect(0, 214, TILE_W, 30);
      ctx.save(); ctx.translate(92, 108); ctx.rotate(-.6); ctx.beginPath(); ctx.ellipse(0, 0, 64, 32, 0, 0, Math.PI * 2); ctx.fill(); ctx.restore();
      ctx.strokeStyle = '#f4f4ee'; ctx.lineWidth = 4; ctx.beginPath(); ctx.moveTo(52, 146); ctx.lineTo(130, 72); ctx.stroke();
      text('あおば運輸', 66, 180, 96, '#2f8a44');
      text('AOBA TRANSPORT', 28, 184, 156, '#2f8a44', 700, 'Helvetica, Arial, sans-serif'); break;
    case 'やまびこ引越センター':
      ctx.fillStyle = '#e8743a'; ctx.beginPath(); ctx.moveTo(300, 256); ctx.lineTo(380, 256); ctx.lineTo(512, 40); ctx.lineTo(512, -20); ctx.fill();
      ctx.fillStyle = '#23324f'; ctx.beginPath(); ctx.moveTo(30, 96); ctx.lineTo(70, 46); ctx.lineTo(96, 76); ctx.lineTo(116, 54); ctx.lineTo(150, 96); ctx.fill();
      text('やまびこ', 54, 30, 140, '#23324f'); text('引越センター', 40, 30, 196, '#23324f'); break;
    case 'ふじ冷凍便':
      ctx.strokeStyle = '#3a8fd0'; ctx.lineWidth = 14; ctx.beginPath(); ctx.moveTo(20, 70);
      ctx.bezierCurveTo(140, 10, 260, 110, 400, 46); ctx.bezierCurveTo(440, 30, 470, 36, 500, 50); ctx.stroke();
      text('ふじ冷凍便', 80, 40, 160, '#2b77b8'); break;
    case 'みどり交通':
      // Two-tone fleet: a cream waist band with the name on the door.
      ctx.fillStyle = '#efe3b4'; ctx.fillRect(0, 150, TILE_W, 44);
      text('みどり交通', 60, 70, 96, '#f3ecd2', 600);
      ctx.fillStyle = '#efe3b4'; ctx.beginPath(); ctx.arc(40, 96, 18, 0, Math.PI * 2); ctx.fill(); break;
    case 'さくら無線':
      ctx.fillStyle = '#d97a93';
      for (let i = 0; i < 5; i++) { const a = i / 5 * Math.PI * 2 - Math.PI / 2; ctx.beginPath(); ctx.ellipse(70 + Math.cos(a) * 22, 120 + Math.sin(a) * 22, 16, 11, a, 0, Math.PI * 2); ctx.fill(); }
      text('さくら無線', 60, 130, 124, '#3a2a30', 600); break;
    case '白鳥クリーニング':
      ctx.fillStyle = '#6fa6c8'; ctx.fillRect(0, 200, TILE_W, 34);
      ctx.strokeStyle = '#3d6f92'; ctx.lineWidth = 8; ctx.beginPath(); ctx.moveTo(40, 120);
      ctx.bezierCurveTo(60, 40, 120, 60, 96, 100); ctx.quadraticCurveTo(80, 140, 150, 140); ctx.stroke();
      text('白鳥', 74, 180, 92, '#2f5a78'); text('クリーニング', 46, 180, 160, '#2f5a78'); break;
    case 'まちの八百屋':
      ctx.fillStyle = '#5f9a3c'; ctx.fillRect(0, 196, TILE_W, 40);
      ctx.fillStyle = '#e07a2e'; ctx.beginPath(); ctx.arc(80, 110, 44, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = '#5f9a3c'; ctx.beginPath(); ctx.ellipse(90, 62, 22, 10, -.5, 0, Math.PI * 2); ctx.fill();
      text('まちの八百屋', 56, 150, 110, '#3f6a28'); break;
    case 'こばやし建材':
      ctx.fillStyle = '#23324f'; ctx.fillRect(0, 0, TILE_W, 36); ctx.fillRect(0, 220, TILE_W, 36);
      text('こばやし建材', 70, 36, 130, '#23324f'); break;
  }
  ctx.restore();
}

export function liveryMaterial(art: ArtMaterials): Surface {
  return art.shared('vehicle-livery', () => {
    const canvas = typeof document !== 'undefined' ? document.createElement('canvas') : null;
    let map: THREE.Texture = placeholder();
    if (canvas && typeof canvas.getContext === 'function') {
      canvas.width = TILE_W * 2 * LIVERY_SCALE; canvas.height = TILE_H * LIVERY_ROWS * LIVERY_SCALE;
      const ctx = canvas.getContext('2d');
      if (ctx && typeof ctx.clearRect === 'function') {
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.scale(LIVERY_SCALE, LIVERY_SCALE);
        LIVERIES.forEach((livery, i) => drawLivery(ctx, livery, (i % 2) * TILE_W, Math.floor(i / 2) * TILE_H));
        map = new THREE.CanvasTexture(canvas); map.colorSpace = THREE.SRGBColorSpace; map.anisotropy = 8;
      }
    }
    art.onDispose(() => map.dispose());
    // The per-instance index reaches the fragment stage interpolated, as
    // 11.9999 or 12.0001; floor/mod on that flips between neighbouring tiles
    // row by row. Snap to the nearest integer first.
    const index = floor(attribute<'float'>('vlivery', 'float').add(.5));
    const col = mod(index, 2), row = floor(index.div(2).add(.01));
    // CanvasTexture is flipped: canvas row 0 is v=1.
    const at = vec2(col.add(uv().x).div(2), float(1).sub(row.add(float(1).sub(uv().y)).div(LIVERY_ROWS)));
    const sample = texture(map, at);
    const pair = {
      // Blended rather than alpha-tested: a hard cut drops thin glyph strokes
      // once the panel is small on screen. Panels lie on opaque bodies, so
      // the transparent pass draws them after the bodies they sit on.
      anime: new THREE.MeshToonNodeMaterial({ gradientMap: art.toonRamp, transparent: true, depthWrite: false }),
      miniature: new THREE.MeshStandardNodeMaterial({ roughness: .45, transparent: true, depthWrite: false }),
    };
    for (const material of Object.values(pair)) {
      material.colorNode = sample.rgb;
      material.opacityNode = sample.a.mul(step(-.5, index));
      // Decals sit 12 mm proud of the body: bias them toward the camera.
      material.polygonOffset = true; material.polygonOffsetFactor = -2; material.polygonOffsetUnits = -2;
    }
    return pair;
  });
}

export const LIVERY_INDEX = Object.fromEntries(LIVERIES.map((name, i) => [name, i])) as Record<Livery, number>;
