use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[cfg(test)]
mod experiments;

mod core;
mod infrastructure;
mod legacy;
mod persistence;
mod population;
mod transit;
mod world;
// Transitional root re-exports of relocated modules.
pub(crate) use core::clock::*;
pub use core::engine::*;
pub use core::kinds::*;
pub(crate) use core::limits::*;
pub use core::state::*;
pub(crate) use core::systems;
pub use core::wire;
pub(crate) use infrastructure::blocks;
pub use infrastructure::construction::*;
pub(crate) use infrastructure::development;
pub(crate) use infrastructure::frontage;
pub(crate) use infrastructure::growth::*;
pub(crate) use infrastructure::masterplan;
pub use infrastructure::network::*;
pub(crate) use infrastructure::state::*;
pub(crate) use infrastructure::station_towers::*;
pub(crate) use legacy::access_candidates;
pub(crate) use legacy::access_capture;
pub(crate) use legacy::access_descriptor_search;
pub(crate) use legacy::access_discovery_job;
#[cfg(target_arch = "wasm32")]
#[cfg(feature = "legacy")]
pub(crate) use legacy::access_discovery_wasm;
pub(crate) use legacy::access_live;
pub(crate) use legacy::access_planning_job;
pub(crate) use legacy::access_proposal_capture;
pub(crate) use legacy::access_search;
#[cfg(feature = "legacy")]
pub(crate) use legacy::district_policy;
pub(crate) use legacy::face_target_ranking;
pub(crate) use legacy::face_target_selection;
pub(crate) use legacy::face_target_viability;
pub(crate) use legacy::frontage_planner::*;
#[cfg(feature = "legacy")]
pub(crate) use legacy::frontier;
pub(crate) use legacy::interior_access;
#[cfg(feature = "legacy")]
pub(crate) use legacy::neighborhood_admission;
pub(crate) use legacy::neighborhood_planner::*;
#[cfg(feature = "legacy")]
pub(crate) use legacy::project_scoring;
pub(crate) use legacy::road_face_trace;
pub(crate) use legacy::road_faces;
pub use legacy::road_planner::*;
pub(crate) use legacy::spine_blocks;
pub(crate) use legacy::spine_plan;
pub(crate) use legacy::state::*;
pub use persistence::snapshot::*;
pub(crate) use population::access::*;
pub(crate) use population::district_controller;
pub(crate) use population::district_density;
pub(crate) use population::residents::*;
pub(crate) use population::state::*;
pub(crate) use population::travelers::*;
pub(crate) use transit::bus::*;
pub(crate) use transit::rail::*;
pub(crate) use transit::rail_alignment;
pub(crate) use transit::state::*;
pub(crate) use transit::traffic::*;
pub(crate) use transit::transit_recovery;
pub(crate) use world::clearance::*;
pub(crate) use world::geometry;
pub use world::parcels::*;
pub(crate) use world::river::*;
pub(crate) use world::river_course;
pub(crate) use world::state::*;
pub use world::types::*;

pub use transit::rail_platform::RailPlatformView;
pub use transit::rail_runtime::RailAlignmentView;

#[cfg(test)]
mod tests;
