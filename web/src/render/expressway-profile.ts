import profiles from '../../../shared/expressway-profiles.json';
import type { ExpresswayProfile } from '../generated';

/** Published by the engine so geometry and moving traffic use the same section. */
export type { ExpresswayProfile };
export const CURRENT_EXPRESSWAY_PROFILE = profiles.current as ExpresswayProfile;
export const COMPACT_EXPRESSWAY_PROFILE = profiles.compact as ExpresswayProfile;
