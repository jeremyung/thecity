import * as THREE from 'three/webgpu';
import { texture } from 'three/tsl';
import { canvasTexture, type ArtMaterials } from '../materials';
import type { Kit } from './geometry-kit';
export const STOREFRONT_TENANTS = [
  ['青葉珈琲', 'AOBA COFFEE', '#394f43'],
  ['つばめ書房', 'TSUBAME BOOKS', '#3f596b'],
  ['小川ベーカリー', 'OGAWA BAKERY', '#9b6946'],
  ['日々食品', 'HIBI MARKET', '#a6533e'],
  ['みどり薬局', 'MIDORI PHARMACY', '#4b7361'],
  ['山手食堂', 'YAMATE KITCHEN', '#74443b'],
  ['花と暮らし', 'FLOWERS & LIVING', '#65775b'],
  ['灯りレコード', 'AKARI RECORDS', '#505068'],
  ['東町サイクル', 'HIGASHI CYCLE', '#3f6974'],
  ['まちの文具', 'MACHI STATIONERY', '#6b6354'],
  ['喫茶こもれび', 'KOMOREBI CAFE', '#745c48'],
  ['橙デリ', 'DAIDAI DELI', '#af703e'],
  ['晴海スタジオ', 'HARUMI STUDIO', '#536776'],
  ['緑川設計', 'MIDORIKAWA DESIGN', '#606c58'],
  ['中央クリニック', 'CHUO CLINIC', '#587b81'],
  ['空色旅行', 'SORAIRO TRAVEL', '#4b6f91'],
  ['ふじ写真館', 'FUJI PHOTO', '#625b74'],
  ['港テーラー', 'MINATO TAILOR', '#454e59'],
  ['青空スポーツ', 'AOZORA SPORTS', '#496655'],
  ['日和雑貨', 'HIYORI GOODS', '#947b63'],
  ['朝市マート', 'ASAICHI MART', '#6c7c43'],
  ['星川眼鏡', 'HOSHIKAWA OPTICS', '#48616c'],
  ['つきの美容室', 'TSUKI SALON', '#8b6e79'],
  ['川辺ギャラリー', 'KAWABE GALLERY', '#6a635b'],
] as const;
export const PRICE_LABELS = [
  'SALE',
  '¥198',
  '¥580',
  '本日特売',
  'OPEN',
  '¥980',
  '新入荷',
  '¥350',
] as const;
const WIDTH = 512,
  HEIGHT = 1024,
  CELL_W = 256,
  CELL_H = 80;
export function storefrontCell(index: number) {
  const i =
      ((index % STOREFRONT_TENANTS.length) + STOREFRONT_TENANTS.length) % STOREFRONT_TENANTS.length,
    x = (i % 2) * CELL_W,
    y = Math.floor(i / 2) * CELL_H;
  return {
    u0: (x + 3) / WIDTH,
    u1: (x + CELL_W - 3) / WIDTH,
    v0: 1 - (y + CELL_H - 3) / HEIGHT,
    v1: 1 - (y + 3) / HEIGHT,
  };
}
const atlases = new WeakMap<ArtMaterials, THREE.CanvasTexture>();
/** Two bounded shared materials: occupied and vacant. No per-building canvas. */
export function storefrontSurface(art: ArtMaterials, active: boolean) {
  return art.shared(`business-storefront-${active}`, () => {
    let atlas = atlases.get(art);
    if (!atlas) {
      atlas = canvasTexture(WIDTH, HEIGHT, (ctx) => {
        ctx.fillStyle = '#ddd6c5';
        ctx.fillRect(0, 0, WIDTH, HEIGHT);
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        STOREFRONT_TENANTS.forEach(([jp, en, bg], i) => {
          const x = (i % 2) * CELL_W,
            y = Math.floor(i / 2) * CELL_H;
          ctx.fillStyle = bg;
          ctx.fillRect(x, y, CELL_W, CELL_H);
          ctx.fillStyle = '#f2eddd';
          ctx.font = '700 26px "Hiragino Kaku Gothic ProN",sans-serif';
          ctx.fillText(jp, x + 128, y + 29, 240);
          ctx.font = '600 12px sans-serif';
          ctx.fillText(en, x + 128, y + 59, 240);
        });
        PRICE_LABELS.forEach((label, i) => {
          ctx.fillStyle = i % 2 ? '#f0d75e' : '#f4eee1';
          ctx.fillRect(i * 64, 960, 64, 64);
          ctx.fillStyle = '#a53228';
          ctx.font = '800 18px sans-serif';
          ctx.fillText(label, i * 64 + 32, 992, 58);
        });
      });
      atlases.set(art, atlas);
      const owned = atlas;
      art.onDispose(() => {
        owned.dispose();
        atlases.delete(art);
      });
    }
    const anime = new THREE.MeshToonNodeMaterial({ map: atlas, gradientMap: art.toonRamp }),
      miniature = new THREE.MeshStandardNodeMaterial({ map: atlas, roughness: 0.6 });
    const emission = texture(atlas)
      .rgb.mul(art.night)
      .mul(art.litShop)
      .mul(active ? 0.55 : 0);
    for (const material of [anime, miniature])
      (material as unknown as { emissiveNode: typeof emission }).emissiveNode = emission;
    return { anime, miniature };
  });
}
export function addStorefrontSign(
  kit: Kit,
  art: ArtMaterials,
  m: {
    x: number;
    y: number;
    z: number;
    width: number;
    height: number;
    tenant: number;
    active: boolean;
    side?: boolean;
  },
) {
  const c = storefrontCell(m.tenant),
    geo = new THREE.PlaneGeometry(m.width, m.height),
    uv = geo.getAttribute('uv');
  for (let i = 0; i < uv.count; i++)
    uv.setXY(i, c.u0 + uv.getX(i) * (c.u1 - c.u0), c.v0 + uv.getY(i) * (c.v1 - c.v0));
  if (m.side) geo.rotateY(Math.PI / 2);
  geo.translate(m.x, m.y, m.z);
  kit.add(geo, storefrontSurface(art, m.active));
}

/** Price placards reuse unused pixels in the storefront atlas. */
export function addPriceCard(
  kit: Kit,
  art: ArtMaterials,
  x: number,
  y: number,
  z: number,
  index: number,
) {
  const i = ((index % 8) + 8) % 8,
    geo = new THREE.PlaneGeometry(0.32, 0.32),
    uv = geo.getAttribute('uv');
  for (let n = 0; n < uv.count; n++)
    uv.setXY(n, (i * 64 + 3 + uv.getX(n) * 58) / 512, (3 + uv.getY(n) * 58) / 1024);
  geo.translate(x, y, z);
  kit.add(geo, storefrontSurface(art, false));
}
