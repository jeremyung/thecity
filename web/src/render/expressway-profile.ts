import profiles from '../../../shared/expressway-profiles.json';
/** Published by the engine so geometry and moving traffic use the same section. */
export interface ExpresswayProfile {
  width:number; lane_centers:readonly number[];
  inner_mark:number; lane_mark:number; outer_mark:number; median_width:number;
}
export const CURRENT_EXPRESSWAY_PROFILE:ExpresswayProfile=profiles.current;
export const COMPACT_EXPRESSWAY_PROFILE:ExpresswayProfile=profiles.compact;
