import * as THREE from 'three/webgpu';
import { texture } from 'three/tsl';
import { canvasTexture, type ArtMaterials } from '../materials';
import type { Kit } from './geometry-kit';
import { STOREFRONT_TENANTS } from './business-signs';
export const DISTRICT_TENANTS = [
  ...STOREFRONT_TENANTS.map((t) => [t[0], t[1]] as readonly string[]),
  ['星空カラオケ', 'HOSHIZORA KARAOKE'],
  ['夜風バー', 'YOKAZE BAR'],
  ['港ジャズ', 'MINATO JAZZ'],
  ['彩りダイニング', 'IRODORI DINING'],
  ['光電器', 'HIKARI ELECTRONICS'],
  ['ゲーム銀河', 'GALAXY GAMES'],
  ['未来パーツ', 'MIRAI PARTS'],
  ['音楽基地', 'MUSIC BASE'],
  ['街角ファッション', 'STREET FASHION'],
  ['青空スニーカー', 'AOZORA SNEAKERS'],
  ['ポップスタジオ', 'POP STUDIO'],
  ['レコード横丁', 'RECORD ALLEY'],
  ['夜空ラウンジ', 'YOZORA LOUNGE'],
  ['炭火食堂', 'SUMIBI KITCHEN'],
  ['星劇場', 'STAR THEATRE'],
  ['虹ゲームセンター', 'RAINBOW ARCADE'],
  ['市場鮮魚', 'MARKET SEAFOOD'],
  ['果実商店', 'FRUIT MARKET'],
  ['お茶の小路', 'TEA LANE'],
  ['屋台らーめん', 'RAMEN STALL'],
  ['古着百貨', 'VINTAGE GOODS'],
  ['新星カメラ', 'NOVA CAMERA'],
  ['文庫横丁', 'BOOK ALLEY'],
  ['旅の道具', 'TRAVEL GOODS'],
] as const;
export const DISTRICT_SIGN_COLORS = [
  '#c93932',
  '#154c9c',
  '#e6b52a',
  '#bf3676',
  '#15898a',
  '#f1e9d1',
  '#344c91',
  '#d56c25',
] as const;
const atlases = new WeakMap<ArtMaterials, THREE.CanvasTexture>();
export function districtSignCell(index: number, vertical = false) {
  const i = ((index % (vertical ? 16 : 48)) + (vertical ? 16 : 48)) % (vertical ? 16 : 48),
    x = vertical ? 768 + (i % 4) * 64 : (i % 3) * 256,
    y = vertical ? Math.floor(i / 4) * 256 : Math.floor(i / 3) * 64,
    w = vertical ? 64 : 256,
    h = vertical ? 256 : 64;
  return {
    u0: (x + 2) / 1024,
    u1: (x + w - 2) / 1024,
    v0: 1 - (y + h - 2) / 1024,
    v1: 1 - (y + 2) / 1024,
  };
}
function surface(art: ArtMaterials, light: 'closed' | 'retail' | 'nightlife') {
  return art.shared(`district-signs/${light}`, () => {
    let atlas = atlases.get(art);
    if (!atlas) {
      atlas = canvasTexture(1024, 1024, (ctx) => {
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        for (let i = 0; i < 48; i++) {
          const x = (i % 3) * 256,
            y = Math.floor(i / 3) * 64,
            bg = DISTRICT_SIGN_COLORS[i % 8];
          ctx.fillStyle = bg;
          ctx.fillRect(x, y, 256, 64);
          ctx.fillStyle = i % 8 === 2 || i % 8 === 5 ? '#242b35' : '#fff7e4';
          ctx.font = `${i % 3 === 0 ? 900 : 700} ${i % 3 === 1 ? 25 : 23}px "Hiragino Kaku Gothic ProN",sans-serif`;
          ctx.fillText(DISTRICT_TENANTS[i][0], x + 128, y + 23, 244);
          ctx.font = '700 10px sans-serif';
          ctx.fillText(DISTRICT_TENANTS[i][1], x + 128, y + 49, 240);
          ctx.strokeStyle = ctx.fillStyle;
          ctx.lineWidth = 2;
          if (i % 3 === 0) ctx.strokeRect(x + 5, y + 5, 246, 54);
        }
        for (let i = 0; i < 16; i++) {
          const x = 768 + (i % 4) * 64,
            y = Math.floor(i / 4) * 256;
          ctx.fillStyle = DISTRICT_SIGN_COLORS[(i + 1) % 8];
          ctx.fillRect(x, y, 64, 256);
          ctx.fillStyle = (i + 1) % 8 === 2 || (i + 1) % 8 === 5 ? '#242b35' : '#fff7e4';
          const label = DISTRICT_TENANTS[24 + i][0].slice(0, 7);
          ctx.font = '900 28px "Hiragino Kaku Gothic ProN",sans-serif';
          Array.from(label).forEach((char, n) => ctx.fillText(char, x + 32, y + 22 + n * 31, 55));
        }
      });
      atlases.set(art, atlas);
      const owned = atlas;
      art.onDispose(() => {
        owned.dispose();
        atlases.delete(art);
      });
    }
    const anime = new THREE.MeshToonNodeMaterial({ map: atlas, gradientMap: art.toonRamp }),
      miniature = new THREE.MeshStandardNodeMaterial({ map: atlas, roughness: 0.45 });
    const emission = texture(atlas)
      .rgb.mul(art.night)
      .mul(light === 'nightlife' ? 1.2 : light === 'retail' ? art.litShop : 0.0);
    for (const mat of [anime, miniature])
      (mat as unknown as { emissiveNode: typeof emission }).emissiveNode = emission;
    return { anime, miniature };
  });
}
export function addDistrictSign(
  kit: Kit,
  art: ArtMaterials,
  m: {
    x: number;
    y: number;
    z: number;
    width: number;
    height: number;
    tenant: number;
    vertical?: boolean;
    nightlife?: boolean;
    active: boolean;
    rotation?: number;
  },
) {
  const c = districtSignCell(m.tenant, m.vertical),
    g = new THREE.PlaneGeometry(m.width, m.height),
    uv = g.getAttribute('uv');
  for (let i = 0; i < uv.count; i++)
    uv.setXY(i, c.u0 + uv.getX(i) * (c.u1 - c.u0), c.v0 + uv.getY(i) * (c.v1 - c.v0));
  g.rotateY(m.rotation ?? 0);
  g.translate(m.x, m.y, m.z);
  kit.add(g, surface(art, !m.active ? 'closed' : m.nightlife ? 'nightlife' : 'retail'));
}
