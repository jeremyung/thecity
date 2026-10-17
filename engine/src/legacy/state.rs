use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Persisted cursors and plans of the retired incremental planners. Flattened into the save, so its fields keep their top-level keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LegacyPlannerState {
    #[serde(default)]
    pub(crate) road_origin_cursor: usize,
    #[serde(default)]
    pub(crate) road_infill_due: u8,
    #[serde(default)]
    pub(crate) frontage_road_cursor: usize,
    #[serde(default)]
    pub(crate) frontage_slot_cursor: u16,
    #[serde(default)]
    pub(crate) neighborhood_plan: Option<NeighborhoodPlan>,
    #[serde(default)]
    pub(crate) pending_interior_access: Option<legacy::interior_access::InteriorAccessPlan>,
    #[serde(default)]
    pub(crate) interior_access_cursor: usize,
    #[serde(default)]
    pub(crate) next_face_access_tick: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) exhausted_access_faces: Vec<Vec<u32>>,
    // Runtime jobs are copied here only at explicit save boundaries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) live_access_checkpoint: Option<legacy::access_live::LiveAccessCheckpoint>,
    /// Road ids that belonged to completed or active compact neighborhood
    /// blocks. Kept after a plan retires so historic block frontage keeps its
    /// parcel geometry and infill rules.
    #[serde(default)]
    pub(crate) neighborhood_road_ids: BTreeSet<u32>,
    #[serde(default)]
    pub(crate) neighborhood_anchor_cursor: usize,
}
