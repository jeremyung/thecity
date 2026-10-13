/** Compact fictional commuter consist. Dimensions are metres, shared by kits and track. */
export const RAIL_GAUGE = 1.067;
export const RAIL_HEAD_WIDTH = .07;
export const RAIL_CENTER_OFFSET = (RAIL_GAUGE + RAIL_HEAD_WIDTH) / 2;
// Keep coach floor aligned with the engine's 8 m platform walking surface.
export const RAIL_DECK = 5.4;
export const RAIL_HEAD_Y = 1 + RAIL_DECK + .45 + .085;
export const COACH_LENGTH = 15;
export const COACH_WIDTH = 3.1;
export const COACH_SPACING = 16;
export const COACH_FLOOR_HEIGHT = 1.065;
export const PLATFORM_EDGE_X = COACH_WIDTH / 2 + .1;
/** Top of the middle coach's pantograph shoe above the rail head. */
export const PANTOGRAPH_CONTACT_HEIGHT = 4.9425;
