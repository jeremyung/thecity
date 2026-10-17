import type { Building, BuildingOriginalFootprint } from '../types';
import type { SignageContext } from './signage';
import { windowLived } from './building-surfaces';
/** Fields that affect a building's rendered group. Occupancy is binary: every
 * window pane bakes whether anyone lives or works behind it (`windowLived`), so
 * crossing the vacant boundary rebuilds, while other occupancy changes do not. */
export function buildingRenderSignature(
  b: Building,
  original?: BuildingOriginalFootprint,
  signage?: SignageContext,
): string {
  const homeOccupancy = windowLived(b) ? 'occupied' : 'vacant';
  const retained = original
    ? `${original.x}:${original.z}:${original.width}:${original.depth}:${original.rotation}`
    : '-';
  const signs =
    signage && (signage.proximity || signage.feature)
      ? `:s${signage.proximity}${signage.feature ?? ''}`
      : '';
  const stagedTower =
    b.floors >= 12 &&
    (b.kind === 'office' || b.kind === 'apartment') &&
    b.stage !== 'open' &&
    b.stage !== 'renovation' &&
    b.stage !== 'demolition';
  const towerPhase = `${Math.ceil(b.floors * Math.max(0, Math.min(1, (b.progress - 0.12) / 0.57)))}:${Math.ceil(b.floors * Math.max(0, Math.min(1, (b.progress - 0.69) / 0.23)))}`;
  const phase = stagedTower
    ? towerPhase
    : b.stage === 'demolition'
      ? `${Math.ceil(b.floors * (1 - Math.max(0, Math.min(1, b.progress))))}:${b.progress > 0}`
      : Math.floor(b.progress * 5);
  return `${b.stage}:${b.floors}:${phase}:${b.kind}:${b.name}:${b.appearance_seed}:${b.business_active ?? 'legacy'}:${homeOccupancy}:${b.x}:${b.z}:${b.width}:${b.depth}:${b.rotation}:${retained}${signs}`;
}
