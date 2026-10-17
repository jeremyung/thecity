import type { Building } from '../../types';
import type { ArtMaterials } from '../materials';
import type { Kit } from './geometry-kit';
/** Shared exact construction state drives geometry and cache invalidation. */
export function apartmentConstruction(b: Building) {
  const p = Math.max(0, Math.min(1, Number.isFinite(b.progress) ? b.progress : 0)),
    complete = b.stage === 'open' || b.stage === 'renovation';
  const installed =
    b.stage === 'demolition'
      ? Math.ceil(b.floors * (1 - p))
      : complete
        ? b.floors
        : ['foundation', 'structure'].includes(b.stage)
          ? 0
          : Math.max(0, Math.min(b.floors, Math.floor(((p - 0.4) / 0.6) * b.floors)));
  const framed =
    b.stage === 'demolition'
      ? installed
      : b.stage === 'foundation'
        ? 0
        : b.stage === 'structure'
          ? Math.min(b.floors, Math.max(1, Math.ceil((p / 0.65) * b.floors)))
          : b.floors;
  return { complete, installed, framed };
}
export function apartmentPhase(b: Building) {
  const s = apartmentConstruction(b);
  return `${s.installed}:${s.framed}:${b.stage === 'demolition' && b.progress > 0}`;
}
export function addDemolitionSite(kit: Kit, b: Building, art: ArtMaterials) {
  const fence = art.get('#66746f'),
    caution = art.get('#c1a16a'),
    concrete = art.get('#9a9b90');
  for (const side of [-1, 1]) {
    kit.box(side * (b.width / 2 - 0.12), 0.85, 0, 0.1, 1.7, b.depth - 0.14, fence);
    kit.box(0, 0.85, side * (b.depth / 2 - 0.12), b.width - 0.14, 1.7, 0.1, fence);
    kit.box(side * b.width * 0.25, 1.25, b.depth / 2 - 0.055, b.width * 0.18, 0.12, 0.02, caution);
  }
  if (b.progress > 0)
    for (let i = 0; i < 8; i++)
      kit.box(
        (((i * 7) % 11) / 10 - 0.5) * b.width * 0.6,
        0.25,
        (((i * 3) % 7) / 6 - 0.5) * b.depth * 0.6,
        0.3 + (i % 3) * 0.1,
        0.2 + (i % 2) * 0.15,
        0.35,
        concrete,
      );
}
