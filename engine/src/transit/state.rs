use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Trains, buses, expressway traffic and stations. Flattened into the save, so its fields keep their top-level keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TransitState {
    #[serde(default)]
    pub(crate) expressway_traffic: transit::expressway_traffic::TrafficState,
    pub(crate) trains: Vec<Train>,
    #[serde(default)]
    pub(crate) bus_line: Option<BusLine>,
    #[serde(default)]
    pub(crate) buses: Vec<Bus>,
    #[serde(default)]
    pub(crate) next_bus_evaluation_tick: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) pending_alight_ids: Vec<u32>,
    #[serde(default)]
    pub(crate) pending_alight_cursor: usize,
    pub(crate) station_online: bool,
    #[serde(default)]
    pub(crate) station_ready: bool,
}
