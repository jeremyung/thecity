// Every repeatable detail is baked into one geometry per shared material.
// Facade: implementation lives in kit/.
export {
  Kit,
  partyWallBodyWidth,
  partyWallBodyDepth,
  isModernHomeVariant,
  disposeGroup,
} from './kit/core';
export { buildingKit } from './kit/building';
export type { BuildingKitReviewOptions } from './kit/building';
