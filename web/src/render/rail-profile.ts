import trainManifest from './train-manifest.json';
/** Authored commuter consist. Dimensions are metres, shared by kits and track. */
export const RAIL_GAUGE = 1.067;
export const RAIL_HEAD_WIDTH = 0.07;
export const RAIL_CENTER_OFFSET = (RAIL_GAUGE + RAIL_HEAD_WIDTH) / 2;
// Keep coach floor aligned with the engine's 8 m platform walking surface.
export const RAIL_DECK = 5.4;
export const RAIL_HEAD_Y = 1 + RAIL_DECK + 0.45 + 0.085;
export const COACH_LENGTH = trainManifest.classes.commuter.length;
export const COACH_WIDTH = trainManifest.classes.commuter.width;
export const COACH_SPACING = COACH_LENGTH + trainManifest.classes.commuter.gap;
export const TRAIN_LENGTH =
  COACH_SPACING * (trainManifest.classes.commuter.consist.length - 1) + COACH_LENGTH;
export const TRAIN_HALF_LENGTH = TRAIN_LENGTH / 2;
export const RAIL_PLATFORM_END = TRAIN_LENGTH + 1;
export const RAIL_GATEWAY_STAIR_BOTTOM = RAIL_PLATFORM_END + 19;
export const COACH_FLOOR_HEIGHT = 1.065;
export const PLATFORM_EDGE_X = COACH_WIDTH / 2 + 0.1;
/** Top of the middle coach's pantograph shoe above the rail head. */
export const PANTOGRAPH_CONTACT_HEIGHT = 4.9425;
