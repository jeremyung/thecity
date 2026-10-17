use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Residents, travelers, errands and demand. Flattened into the save, so its fields keep their top-level keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PeopleState {
    pub(crate) travelers: Vec<Traveler>,
    pub(crate) neighborhood_errands: population::neighborhood_errands::ErrandState,
    #[serde(default)]
    pub(crate) residents: Vec<Resident>,
    pub(crate) population: u32,
    pub(crate) demand: f32,
    pub(crate) last_departure_day: u32,
    #[serde(default)]
    pub(crate) next_delivery_tick: u64,
    #[serde(default)]
    pub(crate) delivery_cursor: usize,
    #[serde(
        default,
        skip_serializing_if = "population::district_controller::DistrictControllerState::is_empty"
    )]
    pub(crate) district_controller: population::district_controller::DistrictControllerState,
}
