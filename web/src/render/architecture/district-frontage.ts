import type { Building } from '../../types';
import type { ArtMaterials } from '../materials';
import type { Kit } from './geometry-kit';
import type { BusinessRecipe } from './business-recipes';
import type { BusinessDetail } from './business-components';
import { addDistrictSign } from './district-signs';
import { architecturalChoice as pick } from './variation';
/** Tenant identities can be dense without multiplying animated billboard allocations. */
export function districtFrontage(
  kit: Kit,
  art: ArtMaterials,
  b: Building,
  r: BusinessRecipe,
  detail: BusinessDetail,
  reservedDisplay = false,
  frontage: readonly { x: number; width: number }[] = [],
) {
  if (!r.district || b.stage !== 'open') return;
  const w = b.width - 1,
    front = (b.depth - 2.2) / 2,
    seed = b.appearance_seed,
    nightlife = r.district === 'roppongi' || r.district === 'shinjuku',
    active = b.business_active !== false,
    metal = art.get('#303b40', 0.45);
  const pool =
    r.district === 'akihabara'
      ? [28, 29, 30, 39, 45, 31]
      : r.district === 'roppongi'
        ? [25, 26, 27, 36, 37, 24]
        : r.district === 'shinjuku'
          ? [24, 25, 38, 39, 37, 36]
          : r.district === 'ameyoko'
            ? [40, 41, 42, 43, 44, 3, 20]
            : [32, 33, 34, 35, 7, 18, 22, 31];
  const portraitPool =
    r.district === 'roppongi'
      ? [1, 2, 3, 12, 13, 0]
      : r.district === 'akihabara'
        ? [4, 5, 6, 7, 15]
        : r.district === 'shinjuku'
          ? [0, 1, 12, 14, 15, 13]
          : [8, 9, 10, 11, 7, 0];
  const first = pick(seed, 530) % pool.length,
    maxFloor = Math.min(b.floors, r.district === 'ameyoko' ? 2 : 9),
    columns = Math.max(1, Math.min(3, Math.floor(w / 3.2)));
  for (let f = 0; f < maxFloor; f++) {
    if (reservedDisplay && (f === 1 || f === 2)) continue;
    if (detail === 'far' && f > 0 && f % 2 === 1) continue;
    const count =
      f === 0
        ? frontage.length || columns
        : Math.min(columns, r.district === 'akihabara' ? 2 : 1 + (pick(seed, 531, f) % 2));
    for (let n = 0; n < count; n++) {
      const width =
          Math.min((f === 0 ? frontage[n]?.width : undefined) ?? w / count, f === 0 ? 4.8 : 6.5) -
          0.28,
        x =
          f === 0 && frontage[n]
            ? frontage[n].x
            : count === 1
              ? ((pick(seed, 532, f) % 3) - 1) * Math.max(0, (w - width) / 2 - 0.45)
              : -w / 2 + ((n + 0.5) * w) / count;
      const height = f === 0 ? 0.64 : r.district === 'akihabara' ? 1.05 : 0.68,
        y = 0.58 + f * 3.25 + (f === 0 ? 2.82 : 2.65);
      kit.box(x, y, front + 0.23, width + 0.1, height + 0.1, 0.25, metal);
      addDistrictSign(kit, art, {
        x,
        y,
        z: front + 0.365,
        width,
        height,
        tenant: pool[(first + f * 5 + n) % pool.length],
        nightlife,
        active,
      });
    }
  }
  if (detail !== 'far' && b.floors >= 3 && r.district !== 'ameyoko') {
    const side = pick(seed, 533) % 2 ? -1 : 1,
      panels = Math.min(3, Math.floor((Math.min(b.floors, 10) - 1) / 2));
    for (let i = 0; i < panels; i++) {
      const height = 4.5,
        x = side * (w / 2 - 0.47),
        y = 0.58 + 4.1 + i * 5.1;
      const tenant = portraitPool[(first + i) % portraitPool.length];
      if (pick(seed, 536, i) % 2 === 0) {
        // Two readable faces on a projecting blade, contained in the frontage reserve.
        kit.box(x, y, front + 0.46, 0.16, height + 0.1, 0.92, metal);
        for (const direction of [-1, 1])
          addDistrictSign(kit, art, {
            x: x + direction * 0.085,
            y,
            z: front + 0.46,
            width: 0.8,
            height,
            tenant,
            vertical: true,
            nightlife,
            active,
            rotation: (direction * Math.PI) / 2,
          });
      } else {
        kit.box(x, y, front + 0.3, 0.8, height + 0.1, 0.28, metal);
        addDistrictSign(kit, art, {
          x,
          y,
          z: front + 0.455,
          width: 0.69,
          height,
          tenant,
          vertical: true,
          nightlife,
          active,
        });
      }
    }
  }
  // Rear service plant keeps the public frontage for doors and shop access.
  if (detail === 'close')
    for (let f = 1; f < Math.min(b.floors, 6); f++)
      if (pick(seed, 534, f) % 3 === 0) {
        kit.box(
          -w * 0.3,
          0.58 + f * 3.25 + 1.4,
          -front - 0.22,
          0.8,
          0.48,
          0.38,
          art.get('#cecdc2'),
        );
      }
}
