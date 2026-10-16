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
pub(crate) use core::limits::*;
pub use core::state::*;
pub(crate) use infrastructure::blocks;
pub use infrastructure::construction::*;
pub(crate) use infrastructure::development;
pub(crate) use infrastructure::frontage;
pub(crate) use infrastructure::growth::*;
pub(crate) use infrastructure::masterplan;
pub use infrastructure::network::*;
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
pub use persistence::snapshot::*;
pub(crate) use population::access::*;
pub(crate) use population::district_controller;
pub(crate) use population::district_density;
pub(crate) use population::residents::*;
pub(crate) use population::travelers::*;
pub(crate) use transit::bus::*;
pub(crate) use transit::rail::*;
pub(crate) use transit::rail_alignment;
pub(crate) use transit::traffic::*;
pub(crate) use transit::transit_recovery;
pub(crate) use world::clearance::*;
pub(crate) use world::geometry;
pub use world::parcels::*;
pub(crate) use world::river::*;
pub(crate) use world::river_course;
pub use world::types::*;

pub use transit::rail_platform::RailPlatformView;
pub use transit::rail_runtime::RailAlignmentView;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::Engine;
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    pub struct CityEngine {
        inner: Engine,
    }
    #[wasm_bindgen]
    impl CityEngine {
        #[wasm_bindgen(constructor)]
        pub fn new(seed: u32) -> CityEngine {
            CityEngine {
                inner: Engine::new(seed),
            }
        }
        /// Legacy incremental planner, kept for comparison fixtures only.
        #[cfg(feature = "legacy")]
        #[wasm_bindgen(js_name=newLegacy)]
        pub fn new_legacy(seed: u32) -> CityEngine {
            CityEngine {
                inner: Engine::new_legacy(seed),
            }
        }
        pub fn step(&mut self, ticks: u32) {
            self.inner.step(ticks)
        }
        #[wasm_bindgen(js_name=snapshot)]
        pub fn snapshot(&self) -> String {
            self.inner.snapshot_json()
        }
        pub fn presentation_revision(&self) -> String {
            self.inner.presentation_revision()
        }
        #[wasm_bindgen(js_name=movements)]
        pub fn movements(&self) -> Vec<f32> {
            self.inner.movement_buffer()
        }
        #[wasm_bindgen(js_name=movement_packet)]
        pub fn movement_packet(&self) -> Vec<u32> {
            self.inner.movement_packet()
        }
        #[wasm_bindgen(js_name=train_doors)]
        pub fn train_doors(&self) -> Vec<f32> {
            self.inner.train_doors()
        }
        #[wasm_bindgen(js_name=inspect)]
        pub fn inspect(&self, id: u32) -> String {
            self.inner.inspect_json(id)
        }
        #[wasm_bindgen(js_name=save)]
        pub fn save(&self) -> String {
            self.inner.save_json()
        }
        #[wasm_bindgen(js_name=restore)]
        pub fn restore(json: &str) -> Result<CityEngine, JsValue> {
            Engine::restore_json(json)
                .map(|inner| CityEngine { inner })
                .map_err(|e| JsValue::from_str(&e))
        }
        /// Opt-in benchmark boundary. Actor preflight and commit remain in one
        /// atomic call; city restore and save serialization are timed separately.
        #[cfg(feature = "profiling")]
        pub fn split_open_road(&mut self, road_id: u32, fraction: f32) -> Result<String, JsValue> {
            self.inner
                .try_split_open_road(road_id, fraction)
                .map(|outcome| {
                    serde_json::json!({
                    "split_node_id": outcome.split_node_id,
                    "retained_road_id": outcome.retained_road_id,
                    "new_road_id": outcome.new_road_id,
                    })
                    .to_string()
                })
                .map_err(|error| JsValue::from_str(&format!("{error:?}")))
        }
        #[wasm_bindgen(js_name=set_development_rate)]
        pub fn set_development_rate(&mut self, rate: u32) {
            self.inner.set_development_rate(rate)
        }
    }
}

#[cfg(test)]
mod tests;
