use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Road network and master-plan realization. Flattened into the save, so its fields keep their top-level keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NetworkState {
    /// Monotonic generation for geometry/eligibility inputs used by live
    /// access proposal validation. Separate from the UI snapshot revision.
    #[serde(default)]
    pub(crate) access_geometry_revision: u64,
    /// Once exhausted, equality can no longer prove freshness; never wrap.
    #[serde(default)]
    pub(crate) access_geometry_revision_exhausted: bool,
    pub(crate) nodes: Vec<Node>,
    pub(crate) roads: Vec<Road>,
    /// Master-plan realization progress. Absent for legacy-planner worlds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) masterplan: Option<infrastructure::masterplan_growth::MasterplanProgress>,
}

/// Buildings, projects and renewal. Flattened into the save, so its fields keep their top-level keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ConstructionState {
    pub(crate) buildings: Vec<Building>,
    pub(crate) projects: Vec<Project>,
    pub(crate) growth_cursor: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) pending_rear_extensions: Vec<infrastructure::rear_extensions::RearExtensionPlan>,
    #[serde(default)]
    pub(crate) rear_extension_cursor: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) building_original_footprints: Vec<BuildingOriginalFootprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) height_redevelopment:
        Option<infrastructure::height_redevelopment::HeightRedevelopment>,
    #[serde(default)]
    pub(crate) height_redevelopment_cursor: usize,
    pub(crate) last_renewal_day: u32,
    #[serde(default)]
    pub(crate) last_business_turnover_day: u32,
}
