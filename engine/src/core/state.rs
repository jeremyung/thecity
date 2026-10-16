use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: u32,
    pub tick: u64,
    pub kind: EventKind,
    pub entity_id: u32,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub population: u32,
    pub active_travelers: u32,
    pub buildings: u32,
    pub active_projects: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weather {
    pub kind: WeatherKind,
    pub intensity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorldState {
    pub(crate) schema_version: u32,
    #[serde(default)]
    pub(crate) activity_schema_version: u8,
    pub(crate) engine_version: String,
    pub(crate) seed: u32,
    pub(crate) rng_growth: u64,
    pub(crate) rng_schedule: u64,
    pub(crate) rng_weather: u64,
    pub(crate) tick: u64,
    pub(crate) development_rate: u32,
    pub(crate) development_accum: u64,
    pub(crate) revision: u64,
    /// Monotonic generation for geometry/eligibility inputs used by live
    /// access proposal validation. Separate from the UI snapshot revision.
    #[serde(default)]
    pub(crate) access_geometry_revision: u64,
    /// Once exhausted, equality can no longer prove freshness; never wrap.
    #[serde(default)]
    pub(crate) access_geometry_revision_exhausted: bool,
    pub(crate) next_id: u32,
    pub(crate) next_event_id: u32,
    pub(crate) nodes: Vec<Node>,
    pub(crate) roads: Vec<Road>,
    pub(crate) parcels: Vec<Parcel>,
    #[serde(default)]
    pub(crate) parcel_frontages: Vec<ParcelFrontage>,
    pub(crate) buildings: Vec<Building>,
    pub(crate) projects: Vec<Project>,
    pub(crate) travelers: Vec<Traveler>,
    pub(crate) neighborhood_errands: population::neighborhood_errands::ErrandState,
    #[serde(default)]
    pub(crate) expressway_traffic: transit::expressway_traffic::TrafficState,
    #[serde(default)]
    pub(crate) residents: Vec<Resident>,
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
    pub(crate) events: VecDeque<Event>,
    pub(crate) population: u32,
    pub(crate) demand: f32,
    pub(crate) growth_cursor: usize,
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
    pub(crate) station_online: bool,
    #[serde(default)]
    pub(crate) station_ready: bool,
    pub(crate) weather_day: u32,
    pub(crate) weather_kind: WeatherKind,
    pub(crate) weather_intensity: f32,
    pub(crate) last_departure_day: u32,
    pub(crate) last_renewal_day: u32,
    #[serde(default)]
    pub(crate) last_business_turnover_day: u32,
    #[serde(default)]
    pub(crate) next_delivery_tick: u64,
    #[serde(default)]
    pub(crate) delivery_cursor: usize,
    #[serde(
        default,
        skip_serializing_if = "population::district_controller::DistrictControllerState::is_empty"
    )]
    pub(crate) district_controller: population::district_controller::DistrictControllerState,
    /// Master-plan realization progress. Absent for legacy-planner worlds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) masterplan: Option<infrastructure::masterplan_growth::MasterplanProgress>,
}

impl WorldState {
    pub(crate) fn bump_access_geometry_revision(&mut self) {
        if self.access_geometry_revision_exhausted {
            return;
        }
        match self.access_geometry_revision.checked_add(1) {
            Some(next) => self.access_geometry_revision = next,
            None => {
                self.access_geometry_revision = u64::MAX;
                self.access_geometry_revision_exhausted = true;
            }
        }
    }
}
