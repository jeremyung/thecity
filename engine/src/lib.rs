use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod access_candidates;
mod access_capture;
mod access_math;
mod access_proposal_validation;
mod access_proposal_capture;
mod access_planning_job;
mod access_live;
mod access_live_engine;
#[cfg(target_arch = "wasm32")]
mod access_discovery_wasm;
#[cfg(test)]
mod access_descriptor_integration_tests;
mod access_descriptor_search;
mod access_discovery_job;
#[cfg(test)]
mod access_discovery_work_diagnostic;
mod access_rays;
#[cfg(test)]
mod access_reference;
mod access_search;
mod blocks;
#[cfg(test)]
mod density_gap_diagnosis;
#[cfg(test)]
mod access_density_diagnostic;
#[cfg(test)]
mod access_bend_diagnostic;
mod development;
mod district_controller;
mod district_density;
#[cfg(test)]
mod district_density_integration_tests;
mod district_policy;
#[cfg(test)]
mod face_access_exact_tests;
mod face_target_containment;
mod face_target_ranking;
mod face_target_sampling;
mod face_target_selection;
mod face_target_viability;
mod frontage;
mod node_coincidence_index;
#[cfg(test)]
mod route_equivalence_tests;
mod road_pair_broadphase;
mod frontier;
mod geometry;
mod interior_access;
mod neighborhood_spine;
mod network_split;
mod project_scoring;
mod road_face_bridges;
mod road_face_preflight;
mod road_face_topology;
mod road_face_trace;
mod road_face_walk;
mod road_faces;
mod road_split;
#[cfg(test)]
mod road_split_profile;
mod spine_blocks;
mod spine_plan;
#[cfg(test)]
mod spine_ranking_experiment_tests;
#[cfg(test)]
mod spine_replay_tests;

const SCHEMA_VERSION: u32 = 1;
const ACTIVITY_SCHEMA_VERSION: u8 = 1;
const ENGINE_VERSION: &str = "0.1.0";
const MIN: f32 = -400.0;
const MAX: f32 = 400.0;
const TICKS_PER_SECOND: u64 = 20;
const TICKS_PER_DAY: u64 = 24_000;
const DEVELOPMENT_TICKS_PER_DAY: u64 = 800;
const MAX_BUILDINGS: usize = 2_000;
const MAX_SEGMENTS: usize = 5_000;
const MAX_PARCELS: usize = 4_000;
const MAX_FRONTAGE_ROADS_PER_EVALUATION: usize = 24;
const MAX_FRONTAGE_SITES_PER_EVALUATION: usize = 48;
const MAX_FRONTAGE_GRID_SITES_PER_ROAD: usize = 34;
const MAX_FRONTAGE_GAP_LOTS_PER_SIDE: usize = 8;
const FRONTAGE_GAP_DEPTH: f32 = 12.0;
const FRONTAGE_GAP_OFFSET: f32 = 15.0;
const FRONTAGE_SHALLOW_DEPTH: f32 = 8.0;
const FRONTAGE_SHALLOW_OFFSET: f32 = 12.0;
const MAX_FRONTAGE_SITES_PER_ROAD: usize =
    MAX_FRONTAGE_GRID_SITES_PER_ROAD + 4 * MAX_FRONTAGE_GAP_LOTS_PER_SIDE;
const MAX_ROAD_ORIGINS_PER_EVALUATION: usize = 6;
const MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION: usize = 24;
const MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION: usize = 24;
const FRONTAGE_SLOT_SPACING: f32 = 16.0;
const FRONTAGE_ENDPOINT_CLEARANCE: f32 = 9.0;
const NEIGHBORHOOD_BLOCK_LENGTH: f32 = 82.0;
const NEIGHBORHOOD_BLOCK_WIDTH: f32 = 50.0;
const NEIGHBORHOOD_PARCEL_SIZE: f32 = 16.0;
const NEIGHBORHOOD_FRONTAGE_SPACING: f32 = 18.0;
const STATION_TOWER_PARCEL_WIDTH: f32 = 24.0;
const STATION_TOWER_PARCEL_DEPTH: f32 = 20.0;
const MIN_STATION_TOWER_FRONTAGE_RESERVATIONS: usize = 2;
const STATION_TOWER_ROAD_OFFSET: f32 = 20.0;
const RIVER_HALF_WIDTH: f32 = 32.0;
const RIVER_BANK_BUFFER: f32 = 6.0;
const MAX_TRAVELERS: usize = 2_000;
const MAX_PROJECTS: usize = 12;
const COMPLETED_PROJECT_HISTORY: usize = 64;
const MAX_POPULATION: u32 = 30_000;
const MAX_EVENTS: usize = 10_000;
const MAX_ROUTE_NODES: usize = 128;
// A bounded cast of named inhabitants gives the scene persistent stories without
// simulating every represented citizen as an individual actor.
const MAX_RESIDENTS: usize = 800;
const MAX_ACTIVE_DELIVERIES: usize = 8;
const RAIL_TRAIN_CAPACITY: usize = 48;
const RAIL_QUEUE_CAPACITY: usize = 96;
const BUS_CAPACITY: usize = 24;
const BUS_QUEUE_CAPACITY: usize = 48;
const MAX_BUSES: usize = 2;
const MAX_BUS_STOPS: usize = 5;
const MAX_BUS_ROUTE_NODES: usize = 128;
const BUS_HEADWAY_TICKS: u64 = 240;
const BUS_DWELL_TICKS: u16 = 80;
const BUS_RETRY_TICKS: u64 = 2_000;
// Three symmetric coaches sit at -16m, 0m, and +16m from the train's
// movement-buffer position. Keep an extra 7.5m of rail under each end.
const TRAIN_HALF_LENGTH: f32 = 23.5;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Terrain {
    pub river: Vec<Point>,
    pub river_half_width: f32,
    pub protected: Vec<Vec<Point>>,
    pub gateways: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Road {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub kind: String,
    pub stage: String,
    pub progress: f32,
    pub connected: bool,
    pub length: f32,
    pub lanes: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RoadSplitOutcome {
    split_node_id: u32,
    retained_road_id: u32,
    new_road_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoadSplitError {
    MissingRoad,
    IneligibleRoad,
    InvalidFraction,
    SegmentTooShort,
    ActivePlan,
    ActiveProject,
    UnsafeFrontage,
    InvalidActorRoute,
    Capacity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Parcel {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    /// Local footprint orientation for coordinated block lots. Legacy parcels
    /// retain their historical axis-aligned admission envelope when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f32>,
    pub access_node: u32,
    pub suitability: f32,
    pub protected: bool,
    pub reservation: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Building {
    pub id: u32,
    pub parcel_id: u32,
    pub x: f32,
    pub z: f32,
    pub rotation: f32,
    pub width: f32,
    pub depth: f32,
    pub floors: u8,
    pub kind: String,
    pub stage: String,
    pub progress: f32,
    pub capacity: u16,
    pub occupancy: u16,
    pub built_day: u32,
    pub name: String,
    pub appearance_seed: u32,
    pub history: Vec<String>,
    pub anchor: bool,
    pub vacant_days: u16,
    #[serde(default)]
    pub business_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: u32,
    pub kind: String,
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub parcel_id: Option<u32>,
    pub stage: String,
    pub progress: f32,
    pub reason: String,
    pub duration: f32,
    pub started_tick: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: u32,
    pub tick: u64,
    pub kind: String,
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
    pub kind: String,
    pub intensity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildingAccessView {
    pub building_id: u32,
    pub road_id: u32,
    pub points: [Point; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub engine_version: String,
    pub seed: u32,
    pub tick: u64,
    pub day: u32,
    pub minute_of_day: f32,
    pub development_rate: u32,
    pub revision: u64,
    pub stage: String,
    pub weather: Weather,
    pub bounds: Bounds,
    pub terrain: Terrain,
    pub nodes: Vec<Node>,
    pub roads: Vec<Road>,
    pub parcels: Vec<Parcel>,
    pub buildings: Vec<Building>,
    #[serde(default)]
    pub building_access: Vec<BuildingAccessView>,
    pub projects: Vec<Project>,
    pub trains: Vec<TrainView>,
    #[serde(default)]
    pub bus_lines: Vec<BusLineView>,
    #[serde(default)]
    pub buses: Vec<BusView>,
    pub residents: Vec<ResidentView>,
    pub events: Vec<Event>,
    pub stats: Stats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusLineView {
    pub id: u32,
    pub stops: Vec<u32>,
    pub route: Vec<u32>,
    pub active: bool,
    #[serde(default)]
    pub stop_positions: Vec<BusStopView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusStopView {
    pub node_id: u32,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub direction: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusView {
    pub id: u32,
    pub line_id: u32,
    pub progress: f32,
    pub direction: i8,
    pub dwelling: bool,
    pub passengers: u16,
    pub capacity: u16,
    pub next_stop: Option<u32>,
    pub waiting: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainView {
    pub id: u32,
    pub route: Vec<u32>,
    pub progress: f32,
    pub direction: i8,
    pub dwelling: bool,
    pub passengers: u16,
    pub capacity: u16,
    pub station_waiting: u16,
    pub gateway_waiting: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResidentView {
    pub id: u32,
    pub name: String,
    pub home_building: u32,
    pub work_building: Option<u32>,
    pub activity: String,
    pub transfer: String,
    pub train_id: Option<u32>,
    pub location_node: Option<u32>,
    #[serde(default)]
    pub bus_id: Option<u32>,
    #[serde(default)]
    pub bus_origin_stop: Option<u32>,
    #[serde(default)]
    pub bus_destination_stop: Option<u32>,
    #[serde(default)]
    pub bus_stop_node: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Traveler {
    id: u32,
    kind: u8,
    route: Vec<u32>,
    route_lengths: Vec<f32>,
    edge_index: usize,
    edge_progress: f32,
    speed: f32,
    destination: u32,
    activity: String,
    resident: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    origin_building: Option<u32>,
    #[serde(default)]
    destination_building: Option<u32>,
    #[serde(default)]
    wait_ticks: u16,
    #[serde(default)]
    origin_access: Option<AccessPath>,
    #[serde(default)]
    destination_access: Option<AccessPath>,
    #[serde(default = "default_access_phase")]
    access_phase: u8,
    #[serde(default)]
    access_progress: f32,
}

fn default_access_phase() -> u8 {
    1
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct WorldPoint {
    x: f32,
    z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccessPath {
    points: Vec<WorldPoint>,
}

impl AccessPath {
    fn length(&self) -> f32 {
        self.points
            .windows(2)
            .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].z - pair[0].z))
            .sum()
    }

    fn sample(&self, mut progress: f32) -> Option<(WorldPoint, f32)> {
        if self.points.len() < 2 {
            return self.points.first().copied().map(|point| (point, 0.0));
        }
        let segments = self.points.len() - 1;
        for index in 0..segments {
            let pair = &self.points[index..=index + 1];
            let dx = pair[1].x - pair[0].x;
            let dz = pair[1].z - pair[0].z;
            let length = dx.hypot(dz);
            if progress <= length || index + 1 == segments {
                let t = if length <= f32::EPSILON {
                    1.0
                } else {
                    (progress / length).clamp(0.0, 1.0)
                };
                return Some((
                    WorldPoint {
                        x: pair[0].x + dx * t,
                        z: pair[0].z + dz * t,
                    },
                    dz.atan2(dx),
                ));
            }
            progress -= length;
        }
        let last = *self.points.last()?;
        let before = self.points[self.points.len() - 2];
        Some((last, (last.z - before.z).atan2(last.x - before.x)))
    }

    fn reversed(&self) -> Self {
        Self {
            points: self.points.iter().rev().copied().collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ParcelFrontage {
    parcel_id: u32,
    road_id: u32,
    along: f32,
    side: i8,
}

/// A small connected street ladder reserved before its roads and frontage lots
/// are built. The planner keeps one bounded block plan per world in this slice.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct NeighborhoodPlan {
    anchor_node: u32,
    /// Ordered perimeter edges. Adjacent edges are scheduled one at a time,
    /// allowing each new junction to join the live graph before the next road.
    segments: Vec<(u32, u32)>,
    next_segment: usize,
    /// Legacy plans keep road-by-road lots; new plans reserve their interior
    /// until the complete connected perimeter can admit coordinated parcels.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    coordinated_lots: bool,
    /// Exact collision-free lots promised by a shared-edge plan. Older and
    /// four-new-edge plans continue to derive their full legacy layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected_lots: Option<Vec<blocks::BlockLot>>,
    /// Optional center-spine lifecycle. Older saves omit this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spine: Option<spine_plan::SpinePlan>,
}

/// A validated, ephemeral block option. Nodes are allocated only after the
/// bounded candidate window has been ranked and this option wins.
struct NeighborhoodCandidate {
    anchor_node: u32,
    corners: [WorldPoint; 4],
    corner_node_ids: [Option<u32>; 4],
    reused_edge: Option<(usize, u32)>,
    selected_lots: Option<Vec<blocks::BlockLot>>,
    spine: Option<spine_plan::SpinePlan>,
    score: development::NeighborhoodBenefitScore,
}

#[cfg(test)]
std::thread_local! {
    static SPINE_RANKING_BY_SITES_PER_ROAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) struct SpineRankingExperimentGuard(bool);

#[cfg(test)]
impl SpineRankingExperimentGuard {
    pub(crate) fn by_sites_per_road() -> Self {
        let previous = SPINE_RANKING_BY_SITES_PER_ROAD.with(|flag| flag.replace(true));
        Self(previous)
    }
}

#[cfg(test)]
impl Drop for SpineRankingExperimentGuard {
    fn drop(&mut self) {
        SPINE_RANKING_BY_SITES_PER_ROAD.with(|flag| flag.set(self.0));
    }
}

fn neighborhood_candidate_is_better(
    candidate: development::NeighborhoodBenefitScore,
    current: development::NeighborhoodBenefitScore,
) -> bool {
    #[cfg(test)]
    if SPINE_RANKING_BY_SITES_PER_ROAD.with(std::cell::Cell::get) {
        return development::neighborhood_sites_per_road_is_better(candidate, current);
    }
    development::neighborhood_score_is_better(candidate, current)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NeighborhoodProfile {
    HistoricCore,
    StationCore,
    StationFringe,
    OuterNeighborhood,
}

fn lane_key(traveler: &Traveler, from: u32, to: u32) -> (u32, u32, u8, u8) {
    let sidewalk = if traveler.kind == 0 {
        (traveler.id & 1) as u8
    } else {
        2
    };
    (from, to, traveler.kind, sidewalk)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Resident {
    id: u32,
    name: String,
    home_building: u32,
    work_building: Option<u32>,
    home_node: u32,
    work_node: Option<u32>,
    mode: u8,
    routine: String,
    activity: String,
    at_work: bool,
    departure_tick: u64,
    return_tick: u64,
    last_departure_day: u32,
    last_return_day: u32,
    history: Vec<String>,
    #[serde(default)]
    rail_commuter: bool,
    #[serde(default)]
    transfer: String,
    #[serde(default)]
    train_id: Option<u32>,
    #[serde(default)]
    queue_slot: Option<u16>,
    #[serde(default)]
    bus_id: Option<u32>,
    #[serde(default)]
    bus_origin_stop: Option<u32>,
    #[serde(default)]
    bus_destination_stop: Option<u32>,
    #[serde(default)]
    bus_stop_node: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Train {
    id: u32,
    route: Vec<u32>,
    distance: f32,
    speed: f32,
    #[serde(default = "default_train_direction")]
    direction: i8,
    #[serde(default)]
    dwell_ticks: u16,
    #[serde(default)]
    passengers: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BusLine {
    id: u32,
    stops: Vec<u32>,
    route: Vec<u32>,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    graph_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Bus {
    id: u32,
    line_id: u32,
    progress: f32,
    direction: i8,
    #[serde(default)]
    dwell_ticks: u16,
    #[serde(default)]
    passengers: Vec<u32>,
    #[serde(default)]
    next_departure_tick: u64,
    #[serde(default)]
    last_stop: Option<u32>,
}

fn default_train_direction() -> i8 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorldState {
    schema_version: u32,
    #[serde(default)]
    activity_schema_version: u8,
    engine_version: String,
    seed: u32,
    rng_growth: u64,
    rng_schedule: u64,
    rng_weather: u64,
    tick: u64,
    development_rate: u32,
    development_accum: u64,
    revision: u64,
    /// Monotonic generation for geometry/eligibility inputs used by live
    /// access proposal validation. Separate from the UI snapshot revision.
    #[serde(default)]
    access_geometry_revision: u64,
    /// Once exhausted, equality can no longer prove freshness; never wrap.
    #[serde(default)]
    access_geometry_revision_exhausted: bool,
    next_id: u32,
    next_event_id: u32,
    nodes: Vec<Node>,
    roads: Vec<Road>,
    parcels: Vec<Parcel>,
    #[serde(default)]
    parcel_frontages: Vec<ParcelFrontage>,
    buildings: Vec<Building>,
    projects: Vec<Project>,
    travelers: Vec<Traveler>,
    #[serde(default)]
    residents: Vec<Resident>,
    trains: Vec<Train>,
    #[serde(default)]
    bus_line: Option<BusLine>,
    #[serde(default)]
    buses: Vec<Bus>,
    #[serde(default)]
    next_bus_evaluation_tick: u64,
    events: VecDeque<Event>,
    population: u32,
    demand: f32,
    growth_cursor: usize,
    #[serde(default)]
    road_origin_cursor: usize,
    #[serde(default)]
    road_infill_due: u8,
    #[serde(default)]
    frontage_road_cursor: usize,
    #[serde(default)]
    frontage_slot_cursor: u16,
    #[serde(default)]
    neighborhood_plan: Option<NeighborhoodPlan>,
    #[serde(default)]
    pending_interior_access: Option<interior_access::InteriorAccessPlan>,
    #[serde(default)]
    interior_access_cursor: usize,
    #[serde(default)]
    next_face_access_tick: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    exhausted_access_faces: Vec<Vec<u32>>,
    // Runtime jobs are copied here only at explicit save boundaries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    live_access_checkpoint: Option<access_live::LiveAccessCheckpoint>,
    /// Road ids that belonged to completed or active compact neighborhood
    /// blocks. Kept after a plan retires so historic block frontage keeps its
    /// parcel geometry and infill rules.
    #[serde(default)]
    neighborhood_road_ids: BTreeSet<u32>,
    #[serde(default)]
    neighborhood_anchor_cursor: usize,
    station_online: bool,
    #[serde(default)]
    station_ready: bool,
    weather_day: u32,
    weather_kind: String,
    weather_intensity: f32,
    last_departure_day: u32,
    last_renewal_day: u32,
    #[serde(default)]
    last_business_turnover_day: u32,
    #[serde(default)]
    next_delivery_tick: u64,
    #[serde(default)]
    delivery_cursor: usize,
    #[serde(
        default,
        skip_serializing_if = "district_controller::DistrictControllerState::is_empty"
    )]
    district_controller: district_controller::DistrictControllerState,
}

/// Authoritative deterministic simulation. Public methods form the coarse WASM boundary.
pub struct Engine {
    state: WorldState,
    planned_progress: Vec<f32>,
    wait_scratch: Vec<u16>,
    lane_order: Vec<usize>,
    district_runtime: district_controller::Runtime,
    live_access: Option<access_live::LiveAccess>,
    /// Non-persisted switch used to compare the first district-policy
    /// experiment against legacy scheduling. Production defaults to false.
    district_policy_experiment: bool,
    #[cfg(test)]
    district_policy_comparisons: u32,
    #[cfg(test)]
    district_policy_access_wins: u32,
    #[cfg(test)]
    district_policy_frontier_wins: u32,
    #[cfg(test)]
    district_policy_diagnostics: district_policy::ExperimentDiagnostics,
    /// Test-only scheduler experiment: offer a real interior-access project
    /// before progressive frontage release, after ordinary usable infill.
    #[cfg(test)]
    early_access_experiment: bool,
    #[cfg(test)]
    early_access_attempted_tick: Option<u64>,
    #[cfg(test)]
    early_access_admissions: u32,
    #[cfg(test)]
    early_access_attempts: u32,
    /// Test-only control for paired comparisons after early block reservation
    /// became the production scheduling behavior.
    #[cfg(test)]
    early_block_reservation_disable_override: bool,
    #[cfg(test)]
    early_block_reservation_attempts: u32,
    #[cfg(test)]
    early_block_reservation_admissions: u32,
    #[cfg(test)]
    early_block_reservation_noops: u32,
    #[cfg(test)]
    early_block_reservation_started_tick: Option<u64>,
    #[cfg(test)]
    early_block_reservation_duration_ticks: u64,
    #[cfg(test)]
    interior_access_completions: u32,
    /// Test-only opening-time access experiment. Production remains unchanged.
    #[cfg(test)]
    road_opening_access_experiment: bool,
    /// Narrow follow-up experiment: permit opening-time access only when the
    /// newly formed face contains sampled unserved interior and a wholly
    /// contained access proposal. The legacy opening experiment remains a
    /// separate control for matched comparisons.
    #[cfg(test)]
    face_conditioned_access_experiment: bool,
    #[cfg(test)]
    road_opening_access_attempts: u32,
    #[cfg(test)]
    road_opening_access_admissions: u32,
    #[cfg(test)]
    road_opening_access_branch: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
struct CollisionSegment {
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
}

/// Immutable geometry inputs reused while evaluating several candidate
/// neighborhood orientations against the same world state.
#[derive(Clone, Debug)]
struct CollisionContext {
    parcel_footprints: Vec<geometry::OrientedRect>,
    open_roads: Vec<CollisionSegment>,
    pending_roads: Vec<CollisionSegment>,
}

fn rng_next(state: &mut u64) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x >> 16) as u32
}
fn random_unit(state: &mut u64) -> f32 {
    (rng_next(state) as f64 / u32::MAX as f64) as f32
}
fn distance(a: &Node, b: &Node) -> f32 {
    geometry::distance_coords(a.x, a.z, b.x, b.z)
}
fn parcel_envelope(parcel: &Parcel) -> geometry::OrientedRect {
    geometry::OrientedRect {
        x: parcel.x,
        z: parcel.z,
        width: parcel.width,
        depth: parcel.depth,
        rotation: parcel.rotation.unwrap_or(0.0),
    }
}
fn parcel_envelopes_clear_of_road(
    context: &CollisionContext,
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
) -> bool {
    context
        .parcel_footprints
        .iter()
        .all(|footprint| geometry::rect_segment_distance(footprint, ax, az, bx, bz) >= 5.0)
}
fn is_housing(building: &Building) -> bool {
    matches!(building.kind.as_str(), "home" | "apartment")
}
fn is_commercial_kind(kind: &str) -> bool {
    matches!(kind, "shop" | "cafe" | "office")
}
fn is_commercial(building: &Building) -> bool {
    is_commercial_kind(&building.kind)
}
fn site_active(building: &Building) -> bool {
    if building.stage != "open" {
        return false;
    }
    if is_housing(building) {
        building.occupancy > 0
    } else if is_commercial(building) {
        building.business_active
    } else {
        false
    }
}
fn site_vacant(building: &Building) -> bool {
    if building.stage != "open" {
        return false;
    }
    if is_housing(building) {
        building.occupancy == 0
    } else if is_commercial(building) {
        !building.business_active
    } else {
        false
    }
}
fn frontage_access_node(a: &Node, b: &Node, along: f32) -> u32 {
    if along < 0.5 {
        a.id
    } else if along > 0.5 {
        b.id
    } else {
        a.id.min(b.id)
    }
}
fn point_segment_distance(x: f32, z: f32, ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    geometry::point_segment_distance(x, z, ax, az, bx, bz)
}
fn segments_distance(a: &Node, b: &Node, c: &Node, d: &Node) -> f32 {
    geometry::segments_distance_coords(a.x, a.z, b.x, b.z, c.x, c.z, d.x, d.z)
}
fn segments_distance_coords(
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
    cx: f32,
    cz: f32,
    dx: f32,
    dz: f32,
) -> f32 {
    geometry::segments_distance_coords(ax, az, bx, bz, cx, cz, dx, dz)
}
fn clock_at(tick: u64) -> (u32, f32) {
    let day = (tick / TICKS_PER_DAY) as u32;
    let elapsed = (tick % TICKS_PER_DAY) as f32 * 1440.0 / TICKS_PER_DAY as f32;
    (day, (360.0 + elapsed) % 1440.0)
}
fn next_clock_deadline(tick: u64, minute: u16) -> u64 {
    let minute = minute as u64 % 1440;
    let minute_from_dawn = (minute + 1440 - 360) % 1440;
    let offset = minute_from_dawn * TICKS_PER_DAY / 1440;
    let day_start = tick / TICKS_PER_DAY * TICKS_PER_DAY;
    let current_offset = tick % TICKS_PER_DAY;
    if current_offset < offset {
        day_start + offset
    } else {
        day_start + TICKS_PER_DAY + offset
    }
}
fn push_history(history: &mut Vec<String>, entry: String) {
    history.push(entry);
    if history.len() > 8 {
        history.remove(0);
    }
}
fn in_protected_land(x: f32, z: f32) -> bool {
    (-350.0..=-260.0).contains(&x) && (-350.0..=-270.0).contains(&z)
}
fn stateless_tower_site_hash(seed: u32, x: f32, z: f32) -> u64 {
    let qx = ((x + 400.0) * 10.0).round() as i64 as u64;
    let qz = ((z + 400.0) * 10.0).round() as i64 as u64;
    let mut value = (seed as u64)
        ^ qx.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ qz.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
fn district_frontier_candidate_id(start_id: u32, target_id: Option<u32>) -> u64 {
    0x4000_0000_0000_0000 | ((start_id as u64) << 32) | target_id.unwrap_or(0) as u64
}

fn district_access_candidate_id(proposal: &interior_access::AccessProposal) -> u64 {
    0x8000_0000_0000_0000
        | ((proposal.score.host_id as u64) << 16)
        | ((proposal.score.fraction_index as u64) << 12)
        | ((proposal.score.side_index as u64) << 8)
        | proposal.score.length_index as u64
}

fn road_demand_is_sufficient(demand: f32) -> bool {
    demand > 0.8
}

fn corridor_crosses_protected(ax: f32, az: f32, bx: f32, bz: f32) -> bool {
    (0..=16).any(|i| {
        let t = i as f32 / 16.0;
        in_protected_land(ax + (bx - ax) * t, az + (bz - az) * t)
    })
}

impl WorldState {
    fn bump_access_geometry_revision(&mut self) {
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

impl Engine {
    /// Advance the persisted generation only after a committed change to the
    /// geometry, connectivity, reservation, or pending-road inputs used by
    /// access proposal validation. Never call from ordinary tick/revision
    /// updates.
    fn mark_access_geometry_changed(&mut self) {
        self.state.bump_access_geometry_revision();
    }

    /// `None` is fail-closed: a proposal cannot be admitted against an
    /// exhausted generation because subsequent mutations cannot be ordered.
    pub(crate) fn access_geometry_revision_token(&self) -> Option<u64> {
        (!self.state.access_geometry_revision_exhausted)
            .then_some(self.state.access_geometry_revision)
    }

    pub fn new(seed: u32) -> Self {
        let seed64 = (seed as u64).wrapping_add(0x9e3779b97f4a7c15).max(1);
        let mut engine = Self {
            state: WorldState {
                schema_version: SCHEMA_VERSION,
                activity_schema_version: ACTIVITY_SCHEMA_VERSION,
                engine_version: ENGINE_VERSION.into(),
                seed,
                rng_growth: seed64 ^ 0xa0761d6478bd642f,
                rng_schedule: seed64 ^ 0xe7037ed1a0b428db,
                rng_weather: seed64 ^ 0x8ebc6af09c88c6e3,
                tick: 0,
                development_rate: 4,
                development_accum: 0,
                revision: 1,
                access_geometry_revision: 0,
                access_geometry_revision_exhausted: false,
                next_id: 1,
                next_event_id: 1,
                nodes: vec![],
                roads: vec![],
                parcels: vec![],
                parcel_frontages: vec![],
                buildings: vec![],
                projects: vec![],
                travelers: vec![],
                residents: vec![],
                trains: vec![],
                bus_line: None,
                buses: vec![],
                next_bus_evaluation_tick: 0,
                events: VecDeque::new(),
                population: 3,
                demand: 0.6,
                growth_cursor: 0,
                road_origin_cursor: 0,
                road_infill_due: 0,
                frontage_road_cursor: 0,
                frontage_slot_cursor: 0,
                neighborhood_plan: None,
                pending_interior_access: None,
                interior_access_cursor: 0,
                next_face_access_tick: 0,
                exhausted_access_faces: Vec::new(),
                live_access_checkpoint: None,
                neighborhood_road_ids: BTreeSet::new(),
                neighborhood_anchor_cursor: 0,
                station_online: false,
                station_ready: false,
                weather_day: 0,
                weather_kind: "clear".into(),
                weather_intensity: 0.0,
                last_departure_day: u32::MAX,
                last_renewal_day: u32::MAX,
                last_business_turnover_day: 0,
                next_delivery_tick: TICKS_PER_DAY / 6,
                delivery_cursor: 0,
                district_controller: district_controller::DistrictControllerState::default(),
            },
            planned_progress: Vec::new(),
            wait_scratch: Vec::new(),
            lane_order: Vec::new(),
            district_runtime: district_controller::Runtime::empty(),
            live_access: None,
            district_policy_experiment: false,
            #[cfg(test)]
            district_policy_comparisons: 0,
            #[cfg(test)]
            district_policy_access_wins: 0,
            #[cfg(test)]
            district_policy_frontier_wins: 0,
            #[cfg(test)]
            district_policy_diagnostics: district_policy::ExperimentDiagnostics::default(),
            #[cfg(test)]
            early_access_experiment: false,
            #[cfg(test)]
            early_access_attempted_tick: None,
            #[cfg(test)]
            early_access_admissions: 0,
            #[cfg(test)]
            early_access_attempts: 0,
            #[cfg(test)]
            early_block_reservation_disable_override: false,
            #[cfg(test)]
            early_block_reservation_attempts: 0,
            #[cfg(test)]
            early_block_reservation_admissions: 0,
            #[cfg(test)]
            early_block_reservation_noops: 0,
            #[cfg(test)]
            early_block_reservation_started_tick: None,
            #[cfg(test)]
            early_block_reservation_duration_ticks: 0,
            #[cfg(test)]
            interior_access_completions: 0,
            #[cfg(test)]
            road_opening_access_experiment: false,
            #[cfg(test)]
            face_conditioned_access_experiment: false,
            #[cfg(test)]
            road_opening_access_attempts: 0,
            #[cfg(test)]
            road_opening_access_admissions: 0,
            #[cfg(test)]
            road_opening_access_branch: None,
        };
        engine.initialize_world();
        engine
    }

    fn id(&mut self) -> u32 {
        let id = self.state.next_id;
        self.state.next_id = self.state.next_id.saturating_add(1);
        id
    }
    fn node(&self, id: u32) -> Option<&Node> {
        self.state.nodes.iter().find(|n| n.id == id)
    }
    fn initialize_world(&mut self) {
        // Every seed shares the river crossing and founding conditions, while its basin bends and parcel fabric vary.
        let gateway = self.add_node(365.0, 42.0, "gateway");
        let east = self.add_node(-18.0, 42.0, "junction");
        let west = self.add_node(-184.0, 42.0, "junction");
        let center = self.add_node(72.0, 42.0, "junction");
        let founder_z = 6.0;
        let founder_x = self.river_center_x_at(founder_z) + 55.0;
        let market = self.add_node(founder_x, 28.0, "market");
        let station = self.add_node(153.0, 58.0, "station");
        let (a, b) = (gateway, center);
        self.add_road(a, b, "arterial", "open", 1.0);
        self.add_road(center, east, "local", "open", 1.0);
        self.add_road(center, market, "local", "open", 1.0);
        // Keep the founding street graph planar: the market already connects
        // to center, so a direct market-to-station chord would cross the
        // center-to-east lane. The station instead joins at center.
        self.add_road(center, station, "local", "open", 1.0);
        self.add_road(east, west, "bridge", "survey", 0.0);
        let bridge_id = self.state.roads.last().unwrap().id;
        let parcel_id = self.add_parcel(founder_x, founder_z, 24.0, 18.0, market, 0.93, false);
        self.remember_nearest_frontage(parcel_id, market);
        let founder = self.add_building(
            parcel_id,
            "shop",
            2,
            "open",
            1.0,
            12,
            "Kissa Aoba",
            true,
            vec!["Opened as the first neighborhood shop".into()],
        );
        let _ = founder;
        self.state.population = 8;
        let parcel = self.add_parcel(91.0, 72.0, 19.0, 16.0, center, 0.82, false);
        self.remember_nearest_frontage(parcel, center);
        let parcel = self.add_parcel(145.0, 18.0, 21.0, 18.0, market, 0.88, false);
        self.remember_nearest_frontage(parcel, market);
        let parcel = self.add_parcel(180.0, 94.0, 25.0, 20.0, station, 0.86, false);
        self.remember_nearest_frontage(parcel, station);
        let rail_b = self.add_node(300.0, 115.0, "rail_gateway");
        let rail_id = self.add_road(station, rail_b, "rail", "foundation", 0.0);
        let rail_project_id = self.id();
        self.state.projects.push(Project {
            id: rail_project_id,
            kind: "rail".into(),
            from: Some(station),
            to: Some(rail_b),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.0,
            reason: "A short rail link is built from the station toward the regional gateway"
                .into(),
            duration: 4.0,
            started_tick: 0,
        });
        let station_id = self.id();
        self.state.projects.push(Project {
            id: station_id,
            kind: "station".into(),
            from: Some(station),
            to: Some(rail_b),
            parcel_id: None,
            stage: "platform".into(),
            progress: 0.0,
            reason: "A rail stop is planned where the growing neighborhood meets the river route"
                .into(),
            duration: 6.0,
            started_tick: 0,
        });
        let bridge_project_id = self.id();
        self.state.projects.push(Project {
            id: bridge_project_id,
            kind: "road".into(),
            from: Some(east),
            to: Some(west),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.0,
            reason: "The first cross-river bridge connects the founding lane to the western bank"
                .into(),
            duration: 8.0,
            started_tick: 0,
        });
        let _ = (bridge_id, rail_id, founder);
        self.record(
            "FoundingShopOpened",
            founder,
            "A small kissa opens beside the first lane".into(),
        );
        self.recompute_connectivity();
        self.state.revision += 1;
    }
    fn add_node(&mut self, x: f32, z: f32, kind: &str) -> u32 {
        let id = self.id();
        self.state.nodes.push(Node {
            id,
            x: x.clamp(MIN, MAX),
            z: z.clamp(MIN, MAX),
            kind: kind.into(),
        });
        self.mark_access_geometry_changed();
        id
    }
    fn add_road(&mut self, from: u32, to: u32, kind: &str, stage: &str, progress: f32) -> u32 {
        let id = self.id();
        let len = match (self.node(from), self.node(to)) {
            (Some(a), Some(b)) => distance(a, b),
            _ => 1.0,
        };
        self.state.roads.push(Road {
            id,
            from,
            to,
            kind: kind.into(),
            stage: stage.into(),
            progress,
            connected: false,
            length: len.max(1.0),
            lanes: if kind == "arterial" { 2 } else { 1 },
        });
        self.mark_access_geometry_changed();
        id
    }

    fn add_parcel(
        &mut self,
        x: f32,
        z: f32,
        w: f32,
        d: f32,
        node: u32,
        s: f32,
        protected: bool,
    ) -> u32 {
        // Keep capacity available for the coordinated lots promised by an
        // active block plan, even when another frontage scan adds a parcel.
        let reserved_lots = self
            .coordinated_plan_lot_count()
            .unwrap_or(0)
            .saturating_add(
                self.state
                    .pending_interior_access
                    .as_ref()
                    .map_or(0, |plan| plan.selected_lots.len()),
            );
        if self
            .state
            .parcels
            .len()
            .saturating_add(1)
            .saturating_add(reserved_lots)
            > MAX_PARCELS
        {
            return 0;
        }
        let id = self.id();
        self.state.parcels.push(Parcel {
            id,
            x,
            z,
            width: w,
            depth: d,
            rotation: None,
            access_node: node,
            suitability: s,
            protected,
            reservation: None,
        });
        self.mark_access_geometry_changed();
        id
    }
    fn remember_nearest_frontage(&mut self, parcel_id: u32, access_node: u32) {
        if parcel_id == 0
            || self
                .state
                .parcel_frontages
                .iter()
                .any(|f| f.parcel_id == parcel_id)
        {
            return;
        }
        let Some(parcel) = self.state.parcels.iter().find(|p| p.id == parcel_id) else {
            return;
        };
        let mut best: Option<(u32, f32, i8, f32)> = None;
        for road in self.state.roads.iter().filter(|road| {
            road.stage == "open"
                && !matches!(road.kind.as_str(), "rail" | "bridge")
                && (road.from == access_node || road.to == access_node)
        }) {
            let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
            else {
                continue;
            };
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let length_sq = dx * dx + dz * dz;
            if length_sq <= f32::EPSILON {
                continue;
            }
            let along =
                (((parcel.x - a.x) * dx + (parcel.z - a.z) * dz) / length_sq).clamp(0.0, 1.0);
            let px = a.x + dx * along;
            let pz = a.z + dz * along;
            let distance = (parcel.x - px).hypot(parcel.z - pz);
            if distance > 34.0
                || best.is_some_and(|(_, _, _, best_distance)| distance >= best_distance)
            {
                continue;
            }
            let nx = -dz / length_sq.sqrt();
            let nz = dx / length_sq.sqrt();
            let side = if (parcel.x - px) * nx + (parcel.z - pz) * nz >= 0.0 {
                1
            } else {
                -1
            };
            best = Some((road.id, along, side, distance));
        }
        if let Some((road_id, along, side, _)) = best {
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
        }
    }
    fn frontage_rotation(&self, parcel_id: u32) -> f32 {
        let Some(frontage) = self
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel_id)
        else {
            return 0.0;
        };
        let Some(road) = self
            .state
            .roads
            .iter()
            .find(|road| road.id == frontage.road_id)
        else {
            return 0.0;
        };
        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
            return 0.0;
        };
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        // Local +Z faces inward from this parcel's frontage side toward the road.
        (frontage.side as f32 * dz).atan2(-frontage.side as f32 * dx)
    }
    fn route_endpoint_position(
        &self,
        route: &[u32],
        at_start: bool,
        traveler_id: u32,
        kind: u8,
    ) -> Option<WorldPoint> {
        if route.len() < 2 {
            return None;
        }
        let edge = if at_start {
            (route[0], route[1])
        } else {
            (route[route.len() - 2], route[route.len() - 1])
        };
        let (Some(a), Some(b)) = (self.node(edge.0), self.node(edge.1)) else {
            return None;
        };
        let length = distance(a, b).max(1.0);
        let width = self
            .state
            .roads
            .iter()
            .find(|road| {
                road.stage == "open"
                    && road.kind != "rail"
                    && ((road.from == edge.0 && road.to == edge.1)
                        || (road.from == edge.1 && road.to == edge.0))
            })
            .map(|road| if road.kind == "arterial" { 14.0 } else { 9.0 })
            .unwrap_or(9.0);
        let lateral = if kind == 1 {
            1.65
        } else if traveler_id % 2 == 0 {
            width * 0.5 + 0.72
        } else {
            -(width * 0.5 + 0.72)
        };
        let nx = -(b.z - a.z) / length;
        let nz = (b.x - a.x) / length;
        let node = if at_start { a } else { b };
        Some(WorldPoint {
            x: node.x + nx * lateral,
            z: node.z + nz * lateral,
        })
    }
    fn building_access_path(
        &self,
        building_id: u32,
        route: &[u32],
        from_building: bool,
        traveler_id: u32,
        kind: u8,
    ) -> Option<AccessPath> {
        let (access_road_id, segment) = self.building_access_segment(building_id)?;
        let building = self.state.buildings.iter().find(|b| b.id == building_id)?;
        let parcel = self
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == building.parcel_id)?;
        let route_position =
            self.route_endpoint_position(route, from_building, traveler_id, kind)?;
        let road = self
            .state
            .roads
            .iter()
            .find(|road| road.id == access_road_id)?;
        let frontage = self
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel.id)?;
        let (a, b) = (self.node(road.from)?, self.node(road.to)?);
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = distance(a, b).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let along = frontage.along.clamp(0.0, 1.0);
        let center = WorldPoint {
            x: a.x + dx * along,
            z: a.z + dz * along,
        };
        let width = if road.kind == "arterial" { 14.0 } else { 9.0 };
        let sidewalk = width * 0.5 + 0.72;
        let frontage_side = frontage.side as f32;
        let frontage_walk = segment[1];
        let access_node = if from_building {
            *route.first()?
        } else {
            *route.last()?
        };
        if access_node != road.from && access_node != road.to {
            return None;
        }
        let access_walk = if access_node == road.from { a } else { b };
        let access_walk = WorldPoint {
            x: access_walk.x + nx * frontage_side * sidewalk,
            z: access_walk.z + nz * frontage_side * sidewalk,
        };
        let toward_frontage = if access_node == road.from { 1.0 } else { -1.0 };
        let travel_nx = -dz * toward_frontage / length;
        let travel_nz = dx * toward_frontage / length;
        let access_lane = WorldPoint {
            x: if access_node == road.from { a.x } else { b.x } + travel_nx * 1.65,
            z: if access_node == road.from { a.z } else { b.z } + travel_nz * 1.65,
        };
        let frontage_lane = WorldPoint {
            x: center.x + travel_nx * 1.65,
            z: center.z + travel_nz * 1.65,
        };
        let (start_walk, end_walk) = if from_building {
            (frontage_walk, access_walk)
        } else {
            (access_walk, frontage_walk)
        };
        let door = if kind != 1 { segment[0] } else { frontage_lane };
        let mut points = if kind == 1 && from_building {
            vec![door, access_lane, route_position]
        } else if kind == 1 {
            vec![route_position, access_lane, door]
        } else if from_building {
            vec![door, start_walk, end_walk, route_position]
        } else {
            vec![route_position, start_walk, end_walk, door]
        };
        points.dedup_by(|right, left| (right.x - left.x).hypot(right.z - left.z) < 0.01);
        (points.len() >= 2).then_some(AccessPath { points })
    }

    fn building_access_segment(&self, building_id: u32) -> Option<(u32, [WorldPoint; 2])> {
        let building = self.state.buildings.iter().find(|b| b.id == building_id)?;
        let parcel = self
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == building.parcel_id)?;
        let frontage = self
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel.id)?;
        let road = self
            .state
            .roads
            .iter()
            .find(|road| road.id == frontage.road_id)?;
        let (a, b) = (self.node(road.from)?, self.node(road.to)?);
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = distance(a, b).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let along = frontage.along.clamp(0.0, 1.0);
        let center = WorldPoint {
            x: a.x + dx * along,
            z: a.z + dz * along,
        };
        let width = if road.kind == "arterial" { 14.0 } else { 9.0 };
        let sidewalk = width * 0.5 + 0.72;
        let side = frontage.side as f32;
        let frontage_walk = WorldPoint {
            x: center.x + nx * side * sidewalk,
            z: center.z + nz * side * sidewalk,
        };
        let door = WorldPoint {
            x: building.x + building.rotation.sin() * (building.depth * 0.5 + 0.2),
            z: building.z + building.rotation.cos() * (building.depth * 0.5 + 0.2),
        };
        Some((road.id, [door, frontage_walk]))
    }
    fn rail_axis(&self) -> Option<(u32, u32, f32, f32)> {
        let (station, gateway) = self.rail_terminals()?;
        let (a, b) = (self.node(station)?, self.node(gateway)?);
        let length = distance(a, b).max(1.0);
        Some((station, gateway, (b.x - a.x) / length, (b.z - a.z) / length))
    }
    fn platform_position(&self, at_station: bool, slot: u16) -> Option<WorldPoint> {
        let (station, gateway, ux, uz) = self.rail_axis()?;
        let terminal = if at_station {
            self.node(station)?
        } else {
            self.node(gateway)?
        };
        let direction = if at_station { 1.0 } else { -1.0 };
        let nx = -uz;
        let nz = ux;
        let along = 12.0 + (slot / 8) as f32 * 2.4;
        let across = 5.2 + (slot % 8) as f32 * 1.3;
        Some(WorldPoint {
            x: terminal.x + ux * direction * along + nx * across,
            z: terminal.z + uz * direction * along + nz * across,
        })
    }
    fn allocate_queue_slot(&self, at_station: bool) -> Option<u16> {
        let used: BTreeSet<u16> = self
            .state
            .residents
            .iter()
            .filter(|resident| {
                if at_station {
                    matches!(
                        resident.transfer.as_str(),
                        "walking_to_station" | "waiting_station"
                    )
                } else {
                    resident.transfer == "waiting_gateway"
                }
            })
            .filter_map(|resident| resident.queue_slot)
            .collect();
        (0..RAIL_QUEUE_CAPACITY as u16).find(|slot| !used.contains(slot))
    }
    fn platform_entry_path(
        &self,
        route: &[u32],
        at_station: bool,
        slot: u16,
        traveler_id: u32,
    ) -> Option<AccessPath> {
        let from = self.route_endpoint_position(route, false, traveler_id, 0)?;
        let to = self.platform_position(at_station, slot)?;
        Some(AccessPath {
            points: vec![from, to],
        })
    }
    fn platform_exit_path(
        &self,
        route: &[u32],
        at_station: bool,
        slot: u16,
        traveler_id: u32,
    ) -> Option<AccessPath> {
        let from = self.platform_position(at_station, slot)?;
        let to = self.route_endpoint_position(route, true, traveler_id, 0)?;
        Some(AccessPath {
            points: vec![from, to],
        })
    }
    fn parcel_geometry_clear(&self, x: f32, z: f32, w: f32, d: f32) -> bool {
        if !self.parcel_clear_of_river(x, z, w, d) {
            return false;
        }
        if x - w * 0.5 < MIN + 4.0
            || x + w * 0.5 > MAX - 4.0
            || z - d * 0.5 < MIN + 4.0
            || z + d * 0.5 > MAX - 4.0
        {
            return false;
        }
        if x + w * 0.5 > -350.0
            && x - w * 0.5 < -260.0
            && z + d * 0.5 > -350.0
            && z - d * 0.5 < -270.0
        {
            return false;
        }
        let candidate = geometry::OrientedRect {
            x,
            z,
            width: w,
            depth: d,
            rotation: 0.0,
        };
        if self
            .state
            .parcels
            .iter()
            .any(|p| geometry::rects_overlap(&candidate, &parcel_envelope(p), 2.0))
        {
            return false;
        }
        let clearance = (w.max(d) * 0.5 + 4.0).max(9.0);
        for road in self.state.roads.iter().filter(|r| r.stage == "open") {
            let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
            else {
                continue;
            };
            if point_segment_distance(x, z, a.x, a.z, b.x, b.z) < clearance {
                return false;
            }
        }
        for project in self
            .state
            .projects
            .iter()
            .filter(|p| p.kind == "road" && p.progress < 1.0)
        {
            let (Some(from), Some(to)) = (project.from, project.to) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                continue;
            };
            if point_segment_distance(x, z, a.x, a.z, b.x, b.z) < clearance + 2.0 {
                return false;
            }
        }
        if !self.neighborhood_corridor_clear_for_parcel(x, z, w, d) {
            return false;
        }
        let candidate = geometry::OrientedRect {
            x,
            z,
            width: w,
            depth: d,
            rotation: 0.0,
        };
        if !self.interior_plan_allows_parcel(&candidate, false) {
            return false;
        }
        true
    }
    fn oriented_parcel_geometry_clear(&self, rect: &geometry::OrientedRect) -> bool {
        if !rect.finite_positive()
            || !self.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth)
        {
            return false;
        }
        let (min_x, min_z, max_x, max_z) = rect.bounds();
        if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
            return false;
        }
        let protected = geometry::OrientedRect {
            x: -305.0,
            z: -310.0,
            width: 90.0,
            depth: 80.0,
            rotation: 0.0,
        };
        if geometry::rects_overlap(rect, &protected, 0.0) {
            return false;
        }
        for parcel in &self.state.parcels {
            // New footprints also respect the actual rendered orientation of
            // legacy frontage, without rewriting historical save geometry.
            let mut existing = parcel_envelope(parcel);
            if parcel.rotation.is_none() {
                // Legacy saves validate their stored axis-aligned envelope.
                // Respect that reservation as well as the rendered footprint;
                // otherwise an admitted lot can fail validation on restore.
                if geometry::rects_overlap(rect, &existing, 2.0) {
                    return false;
                }
                existing.rotation = self
                    .state
                    .buildings
                    .iter()
                    .find(|building| building.parcel_id == parcel.id)
                    .map(|building| building.rotation)
                    .unwrap_or_else(|| self.frontage_rotation(parcel.id));
            }
            if geometry::rects_overlap(rect, &existing, 2.0) {
                return false;
            }
        }
        for road in self.state.roads.iter().filter(|road| road.stage == "open") {
            let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                continue;
            };
            if geometry::rect_segment_distance(rect, a.x, a.z, b.x, b.z) < 5.0 {
                return false;
            }
        }
        for project in self
            .state
            .projects
            .iter()
            .filter(|p| p.kind == "road" && p.progress < 1.0)
        {
            let Some((from, to)) = project.from.zip(project.to) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                continue;
            };
            if geometry::rect_segment_distance(rect, a.x, a.z, b.x, b.z) < 7.0 {
                return false;
            }
        }
        self.neighborhood_corridor_clear_for_parcel(rect.x, rect.z, max_x - min_x, max_z - min_z)
            && self.interior_plan_allows_parcel(rect, false)
    }

    fn collision_context(&self) -> CollisionContext {
        let mut node_positions = BTreeMap::new();
        for node in &self.state.nodes {
            node_positions.entry(node.id).or_insert((node.x, node.z));
        }
        let mut roads_by_id = BTreeMap::new();
        for road in &self.state.roads {
            roads_by_id.entry(road.id).or_insert((road.from, road.to));
        }
        let mut building_rotations = BTreeMap::new();
        for building in &self.state.buildings {
            building_rotations
                .entry(building.parcel_id)
                .or_insert(building.rotation);
        }
        let mut frontages = BTreeMap::new();
        for frontage in &self.state.parcel_frontages {
            frontages
                .entry(frontage.parcel_id)
                .or_insert((frontage.road_id, frontage.side));
        }

        let mut parcel_footprints = Vec::with_capacity(self.state.parcels.len());
        for parcel in &self.state.parcels {
            let mut footprint = parcel_envelope(parcel);
            if parcel.rotation.is_none() {
                parcel_footprints.push(footprint);
                footprint.rotation = building_rotations
                    .get(&parcel.id)
                    .copied()
                    .or_else(|| {
                        let (road_id, side) = *frontages.get(&parcel.id)?;
                        let (from, to) = *roads_by_id.get(&road_id)?;
                        let (ax, az) = *node_positions.get(&from)?;
                        let (bx, bz) = *node_positions.get(&to)?;
                        let dx = bx - ax;
                        let dz = bz - az;
                        Some((side as f32 * dz).atan2(-side as f32 * dx))
                    })
                    .unwrap_or(0.0);
            }
            parcel_footprints.push(footprint);
        }

        let mut open_roads = Vec::new();
        for road in self.state.roads.iter().filter(|road| road.stage == "open") {
            let (Some((ax, az)), Some((bx, bz))) =
                (node_positions.get(&road.from), node_positions.get(&road.to))
            else {
                continue;
            };
            open_roads.push(CollisionSegment {
                ax: *ax,
                az: *az,
                bx: *bx,
                bz: *bz,
            });
        }
        let mut pending_roads = Vec::new();
        for project in self
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "road" && project.progress < 1.0)
        {
            let Some((from, to)) = project.from.zip(project.to) else {
                continue;
            };
            let (Some((ax, az)), Some((bx, bz))) =
                (node_positions.get(&from), node_positions.get(&to))
            else {
                continue;
            };
            pending_roads.push(CollisionSegment {
                ax: *ax,
                az: *az,
                bx: *bx,
                bz: *bz,
            });
        }

        CollisionContext {
            parcel_footprints,
            open_roads,
            pending_roads,
        }
    }

    /// Protect the legacy and rendered parcel envelopes by their exact
    /// centerline distance. `collision_context` retains both orientations for
    /// legacy parcels whose saved rotation is absent.
    fn road_corridor_clear_of_parcels_with_context(
        &self,
        ax: f32,
        az: f32,
        bx: f32,
        bz: f32,
        context: &CollisionContext,
    ) -> bool {
        !self.interior_plan_reserves_road(ax, az, bx, bz)
            && self.interior_plan_lots_clear_of_road(ax, az, bx, bz)
            && parcel_envelopes_clear_of_road(context, ax, az, bx, bz)
    }

    fn oriented_parcel_geometry_clear_with_context(
        &self,
        rect: &geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        self.oriented_parcel_geometry_clear_with_context_mode(rect, context, false)
    }

    fn oriented_parcel_geometry_clear_with_context_mode(
        &self,
        rect: &geometry::OrientedRect,
        context: &CollisionContext,
        completing_reserved_block: bool,
    ) -> bool {
        if !rect.finite_positive()
            || !self.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth)
        {
            return false;
        }
        let (min_x, min_z, max_x, max_z) = rect.bounds();
        if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
            return false;
        }
        let protected = geometry::OrientedRect {
            x: -305.0,
            z: -310.0,
            width: 90.0,
            depth: 80.0,
            rotation: 0.0,
        };
        if geometry::rects_overlap(rect, &protected, 0.0) {
            return false;
        }
        for existing in &context.parcel_footprints {
            if geometry::rects_overlap(rect, existing, 2.0) {
                return false;
            }
        }
        if !self.interior_plan_allows_parcel(rect, completing_reserved_block) {
            return false;
        }
        for road in &context.open_roads {
            if geometry::rect_segment_distance(rect, road.ax, road.az, road.bx, road.bz) < 5.0 {
                return false;
            }
        }
        for project in &context.pending_roads {
            if geometry::rect_segment_distance(rect, project.ax, project.az, project.bx, project.bz)
                < 7.0
            {
                return false;
            }
        }
        completing_reserved_block
            || self.neighborhood_corridor_clear_for_parcel(
                rect.x,
                rect.z,
                max_x - min_x,
                max_z - min_z,
            )
    }

    /// Progressive lots retain the historic centerline clearance while using
    /// their true frontage orientation for parcel, bounds, river, and
    /// protected-land checks. The oriented helper's segment-distance checks
    /// add a footprint safety check; the centerline thresholds below preserve
    /// the ordinary infill spacing policy exactly.
    fn progressive_frontage_geometry_clear(
        &self,
        rect: &geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        if !self.oriented_parcel_geometry_clear_with_context(rect, context) {
            return false;
        }
        self.progressive_frontage_center_clear(rect, context)
    }

    fn progressive_frontage_geometry_clear_for_interior(
        &self,
        rect: &geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        if !self.oriented_parcel_geometry_clear_with_context_mode(rect, context, true) {
            return false;
        }
        self.progressive_frontage_center_clear(rect, context)
    }

    fn progressive_frontage_center_clear(
        &self,
        rect: &geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        let clearance = (rect.width.max(rect.depth) * 0.5 + 4.0).max(9.0);
        context.open_roads.iter().all(|road| {
            point_segment_distance(rect.x, rect.z, road.ax, road.az, road.bx, road.bz) >= clearance
        }) && context.pending_roads.iter().all(|road| {
            point_segment_distance(rect.x, rect.z, road.ax, road.az, road.bx, road.bz)
                >= clearance + 2.0
        })
    }

    fn interior_plan_allows_parcel(
        &self,
        rect: &geometry::OrientedRect,
        completing_own_plan: bool,
    ) -> bool {
        let Some(plan) = &self.state.pending_interior_access else {
            return true;
        };
        if completing_own_plan {
            return true;
        }
        plan.selected_lots.iter().all(|lot| {
            let reserved = geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            !geometry::rects_overlap(rect, &reserved, 2.0)
        })
    }

    fn interior_plan_allows_frontage_candidate(
        &self,
        a: &Node,
        b: &Node,
        x: f32,
        z: f32,
        width: f32,
        depth: f32,
        side: i8,
    ) -> bool {
        if self.state.pending_interior_access.is_none() {
            return true;
        }
        let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
        self.interior_plan_allows_parcel(
            &geometry::OrientedRect {
                x,
                z,
                width,
                depth,
                rotation,
            },
            false,
        )
    }

    fn interior_plan_reserves_road(&self, ax: f32, az: f32, bx: f32, bz: f32) -> bool {
        let Some(plan) = &self.state.pending_interior_access else {
            return false;
        };
        let Some(road) = self
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
        else {
            return false;
        };
        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
            return true;
        };
        geometry::segments_distance_coords(ax, az, bx, bz, a.x, a.z, b.x, b.z) < 8.0
    }

    fn interior_plan_lots_clear_of_road(&self, ax: f32, az: f32, bx: f32, bz: f32) -> bool {
        self.state
            .pending_interior_access
            .as_ref()
            .is_none_or(|plan| {
                plan.selected_lots.iter().all(|lot| {
                    let rect = geometry::OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    };
                    geometry::rect_segment_distance(&rect, ax, az, bx, bz) >= 5.0
                })
            })
    }
    fn parcel_clear_of_river(&self, x: f32, z: f32, width: f32, depth: f32) -> bool {
        let bounding_radius = (width * 0.5).hypot(depth * 0.5);
        self.distance_to_river(x, z) >= RIVER_HALF_WIDTH + RIVER_BANK_BUFFER + bounding_radius
    }
    fn ordinary_road_clear_of_river(
        &self,
        ax: f32,
        az: f32,
        bx: f32,
        bz: f32,
        half_width: f32,
    ) -> bool {
        let required = RIVER_HALF_WIDTH + RIVER_BANK_BUFFER + half_width;
        let points = self.river_control_points();
        (0..points.len() - 1).all(|index| {
            segments_distance_coords(
                ax,
                az,
                bx,
                bz,
                points[index].x,
                points[index].z,
                points[index + 1].x,
                points[index + 1].z,
            ) >= required
        })
    }
    fn distance_to_river(&self, x: f32, z: f32) -> f32 {
        let points = self.river_control_points();
        (0..points.len() - 1)
            .map(|index| {
                point_segment_distance(
                    x,
                    z,
                    points[index].x,
                    points[index].z,
                    points[index + 1].x,
                    points[index + 1].z,
                )
            })
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY)
    }
    fn river_center_x_at(&self, z: f32) -> f32 {
        let points = self.river_control_points();
        for segment in points.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            if z >= a.z && z <= b.z {
                let t = ((z - a.z) / (b.z - a.z)).clamp(0.0, 1.0);
                return a.x + (b.x - a.x) * t;
            }
        }
        if z < points[0].z {
            points[0].x
        } else {
            points[points.len() - 1].x
        }
    }
    fn add_building(
        &mut self,
        parcel: u32,
        kind: &str,
        floors: u8,
        stage: &str,
        progress: f32,
        capacity: u16,
        name: &str,
        anchor: bool,
        history: Vec<String>,
    ) -> u32 {
        let Some(p) = self.state.parcels.iter().find(|p| p.id == parcel).cloned() else {
            return 0;
        };
        let id = self.id();
        let appearance_seed = rng_next(&mut self.state.rng_growth);
        let rotation = if anchor {
            0.0
        } else {
            p.rotation.unwrap_or_else(|| self.frontage_rotation(parcel))
        };
        self.state.buildings.push(Building {
            id,
            parcel_id: parcel,
            x: p.x,
            z: p.z,
            rotation,
            width: p.width,
            depth: p.depth,
            floors,
            kind: kind.into(),
            stage: stage.into(),
            progress,
            capacity,
            occupancy: 0,
            built_day: 0,
            name: name.into(),
            appearance_seed,
            history,
            anchor,
            vacant_days: 0,
            business_active: is_commercial_kind(kind) && stage == "open",
        });
        id
    }

    /// Advance a fixed number of 50ms ticks. Cap batches to keep worker scheduling bounded.
    pub fn step(&mut self, ticks: u32) {
        let count = ticks.min(2_000);
        for _ in 0..count {
            self.step_one();
        }
    }
    fn step_one(&mut self) {
        self.state.tick = self.state.tick.saturating_add(1);
        let (day, _) = clock_at(self.state.tick);
        if day > self.state.weather_day {
            self.state.weather_day = day;
            let roll = random_unit(&mut self.state.rng_weather);
            if roll < 0.56 {
                self.state.weather_kind = "clear".into();
                self.state.weather_intensity = 0.0;
            } else if roll < 0.84 {
                self.state.weather_kind = "overcast".into();
                self.state.weather_intensity =
                    0.2 + random_unit(&mut self.state.rng_weather) * 0.25;
            } else {
                self.state.weather_kind = "rain".into();
                self.state.weather_intensity =
                    0.35 + random_unit(&mut self.state.rng_weather) * 0.65;
            }
            self.state.revision = self.state.revision.saturating_add(1);
        }
        self.state.development_accum = self
            .state
            .development_accum
            .saturating_add(self.state.development_rate as u64);
        while self.state.development_accum >= DEVELOPMENT_TICKS_PER_DAY {
            self.state.development_accum -= DEVELOPMENT_TICKS_PER_DAY;
            self.development_day(day);
        }
        if self.state.tick % 40 == 0 {
            self.evaluate_growth();
        }
        self.trim_completed_projects();
        self.advance_face_access();
        if self.state.tick % 200 == 0 {
            self.ensure_bus_service();
            self.schedule_residents(day);
            self.schedule_deliveries();
        }
        self.advance_travelers();
        self.advance_trains();
        self.advance_buses();
        if self.state.tick % 40 == 0 {
            self.state.revision = self.state.revision.saturating_add(1);
        }
        self.advance_district_measurement();
        #[cfg(test)]
        if self.early_block_reservation_started_tick.is_some() {
            if self.state.neighborhood_plan.is_some() {
                self.early_block_reservation_duration_ticks = self
                    .early_block_reservation_duration_ticks
                    .saturating_add(1);
            } else {
                self.early_block_reservation_started_tick = None;
            }
        }
    }

    fn advance_district_measurement(&mut self) {
        let tick = self.state.tick;
        if self.district_runtime.accumulator.is_none()
            && self.state.district_controller.pending.is_none()
            && tick >= self.state.district_controller.next_refresh_tick
        {
            let (buildings, frontages) = district_controller::make_samples(
                &self.state.buildings,
                &self.state.parcel_frontages,
            );
            let Ok(accumulator) = district_density::MeasurementAccumulator::begin(
                district_controller::DISTRICT_GRID,
                &buildings,
                &frontages,
            ) else {
                self.state.district_controller.next_refresh_tick =
                    tick.saturating_add(district_controller::REFRESH_INTERVAL_TICKS);
                return;
            };
            let Ok(checkpoint) = accumulator.checkpoint() else {
                self.state.district_controller.next_refresh_tick =
                    tick.saturating_add(district_controller::REFRESH_INTERVAL_TICKS);
                return;
            };
            self.state.district_controller.pending = Some(checkpoint);
            self.state.district_controller.pending_capture_tick = tick;
            self.district_runtime.accumulator = Some(accumulator);
        }

        let Some(accumulator) = self.district_runtime.accumulator.as_mut() else {
            return;
        };
        // The input snapshot is frozen at capture. Only its bounded cursor is
        // copied into serialized state on each fixed simulation tick.
        if accumulator
            .advance(district_density::MAX_MEASUREMENT_SAMPLES_PER_ADVANCE)
            .is_err()
        {
            self.district_runtime.accumulator = None;
            self.state.district_controller.pending = None;
            self.state.district_controller.pending_capture_tick = 0;
            self.state.district_controller.next_refresh_tick =
                tick.saturating_add(district_controller::REFRESH_INTERVAL_TICKS);
            return;
        }
        let processed = accumulator.processed_samples();
        let complete = accumulator.is_complete();
        if complete {
            let accumulator = self.district_runtime.accumulator.take().unwrap();
            let mut checkpoint = self
                .state
                .district_controller
                .pending
                .take()
                .expect("active measurement has a persisted checkpoint");
            checkpoint.processed_samples = processed;
            self.district_runtime.metrics = Some(
                accumulator
                    .finish()
                    .expect("successful completed measurement can be finalized"),
            );
            let controller = &mut self.state.district_controller;
            controller.completed_capture_tick = controller.pending_capture_tick;
            controller.completed_tick = tick;
            controller.completed = Some(checkpoint);
            controller.pending_capture_tick = 0;
            controller.next_refresh_tick =
                tick.saturating_add(district_controller::REFRESH_INTERVAL_TICKS);
        } else if let Some(checkpoint) = self.state.district_controller.pending.as_mut() {
            checkpoint.processed_samples = processed;
        }
    }
    fn development_day(&mut self, day: u32) {
        let occupied: usize = self
            .state
            .buildings
            .iter()
            .filter(|b| site_active(b))
            .count();
        let vacancies: usize = self
            .state
            .buildings
            .iter()
            .filter(|b| site_vacant(b))
            .count();
        self.state.demand = (0.35 + self.state.population as f32 / 42.0 + occupied as f32 * 0.055
            - vacancies as f32 * 0.18)
            .clamp(0.12, 4.0);
        let population = self.state.population;
        let mut household_arrivals = 0u32;
        for b in &mut self.state.buildings {
            if site_vacant(b) {
                b.vacant_days = b.vacant_days.saturating_add(1);
            }
            if b.stage == "open" && is_housing(b) && b.occupancy < b.capacity && b.capacity > 0 {
                let arrivals = ((population as f32 / 100.0).ceil() as u16).max(1);
                let previous = b.occupancy;
                b.occupancy = (b.occupancy + arrivals).min(b.capacity);
                household_arrivals += (b.occupancy - previous) as u32;
            }
        }
        self.state.population = self
            .state
            .population
            .saturating_add(household_arrivals)
            .min(MAX_POPULATION);
        self.advance_projects();
        self.sync_residents();
        if day > 0 && day % 3 == 0 && self.state.last_renewal_day != day {
            self.renewal(day);
        }
    }
    fn advance_projects(&mut self) {
        let mut completed = Vec::new();
        let mut spine_road_progress = Vec::new();
        for p in &mut self.state.projects {
            if p.progress >= 1.0 {
                continue;
            }
            let work = 1.0 / p.duration.max(1.0);
            p.progress = (p.progress + work).min(1.0);
            p.stage = if p.progress < 0.22 {
                "survey"
            } else if p.progress < 0.48 {
                "clearing"
            } else if p.progress < 0.73 {
                "structure"
            } else if p.progress < 0.92 {
                "surface"
            } else {
                "finishing"
            }
            .into();
            if p.kind == "building" {
                if let Some(parcel_id) = p.parcel_id {
                    if let Some(building) = self
                        .state
                        .buildings
                        .iter_mut()
                        .find(|b| b.parcel_id == parcel_id && b.stage != "open")
                    {
                        building.stage = p.stage.clone();
                        building.progress = p.progress;
                    }
                }
            }
            if p.kind == "road"
                && self
                    .state
                    .pending_interior_access
                    .as_ref()
                    .is_some_and(|plan| plan.project_id == p.id)
            {
                if let Some(plan) = self.state.pending_interior_access.as_ref() {
                    if let Some(road) = self
                        .state
                        .roads
                        .iter_mut()
                        .find(|road| road.id == plan.branch_road_id)
                    {
                        road.stage = p.stage.clone();
                        road.progress = p.progress;
                    }
                }
            }
            if p.kind == "road" {
                spine_road_progress.push((p.id, p.stage.clone(), p.progress));
            }
            if p.progress >= 1.0 {
                completed.push(p.id);
            }
        }
        for (project_id, stage, progress) in spine_road_progress {
            self.sync_neighborhood_spine_project(project_id, &stage, progress);
        }
        for id in completed {
            self.complete_project(id);
        }
    }
    fn complete_project(&mut self, id: u32) {
        let Some(p) = self.state.projects.iter().find(|p| p.id == id).cloned() else {
            return;
        };
        if self
            .state
            .pending_interior_access
            .as_ref()
            .is_some_and(|plan| plan.project_id == id)
        {
            self.complete_interior_access_project(id);
            return;
        }
        if self.complete_neighborhood_spine_project(id) {
            self.retire_completed_neighborhood_plan();
            self.mark_access_geometry_changed();
            self.state.revision = self.state.revision.saturating_add(1);
            self.trim_completed_projects();
            return;
        }
        match p.kind.as_str() {
            "road" => {
                if let (Some(a), Some(b)) = (p.from, p.to) {
                    let kind = if let (Some(na), Some(nb)) = (self.node(a), self.node(b)) {
                        if (na.x - nb.x).abs() > 100.0 && (na.x < -100.0) != (nb.x < -100.0) {
                            "bridge"
                        } else {
                            "local"
                        }
                    } else {
                        "local"
                    };
                    let rid = if let Some(road) = self.state.roads.iter_mut().find(|r| {
                        ((r.from == a && r.to == b) || (r.from == b && r.to == a))
                            && r.stage != "open"
                    }) {
                        road.stage = "open".into();
                        road.progress = 1.0;
                        road.id
                    } else {
                        self.add_road(a, b, kind, "open", 1.0)
                    };
                    self.recompute_connectivity();
                    self.remember_neighborhood_plan_road(rid, a, b);
                    if let Some(r) = self.state.roads.iter_mut().find(|r| r.id == rid) {
                        if r.connected {
                            self.record(
                                "RoadOpened",
                                rid,
                                format!(
                                    "A {} opens and extends the connected street network",
                                    kind
                                ),
                            );
                        }
                    }
                    if self.state.roads.iter().any(|r| r.id == rid && r.connected) {
                        #[cfg(test)]
                        let frontage_roads = if self.try_plan_access_at_road_opening(rid) {
                            self.state
                                .pending_interior_access
                                .as_ref()
                                .map(|plan| vec![plan.host_road_id, plan.host_second_half_road_id])
                                .unwrap_or_else(|| vec![rid])
                        } else {
                            vec![rid]
                        };
                        #[cfg(not(test))]
                        let frontage_roads = vec![rid];
                        let previous_parcels = self.state.parcels.len();
                        for frontage_road in frontage_roads {
                            self.add_frontage_parcels(frontage_road);
                        }
                        if self.state.parcels.len() > previous_parcels {
                            self.state.road_infill_due =
                                self.state.road_infill_due.saturating_add(2).min(9);
                        }
                    }
                }
            }
            "building" => {
                if let Some(pid) = p.parcel_id {
                    let mut released_parcel_reservation = false;
                    if let Some(parcel) = self.state.parcels.iter_mut().find(|x| x.id == pid) {
                        released_parcel_reservation = parcel.reservation.take().is_some();
                    }
                    if released_parcel_reservation {
                        self.mark_access_geometry_changed();
                    }
                    if let Some(b) = self
                        .state
                        .buildings
                        .iter_mut()
                        .find(|b| b.parcel_id == pid && b.stage != "open")
                    {
                        b.stage = "open".into();
                        b.progress = 1.0;
                        b.built_day = (self.state.tick / TICKS_PER_DAY) as u32;
                        b.vacant_days = 0;
                        let id = b.id;
                        let kind = b.kind.clone();
                        let name = b.name.clone();
                        let reopening = is_commercial(b)
                            && b.history
                                .iter()
                                .any(|entry| entry.starts_with("Business closed:"));
                        let added = if is_housing(b) {
                            b.occupancy = b.capacity.min(4);
                            b.occupancy as u32
                        } else {
                            b.occupancy = 0;
                            b.business_active = is_commercial(b);
                            0
                        };
                        if reopening {
                            self.record(
                                "BusinessReopened",
                                id,
                                format!("{} reopens as a {} after tenant renewal", name, kind),
                            );
                        } else {
                            self.record(
                                "BuildingOpened",
                                id,
                                format!("{} opens as a {}", name, kind),
                            );
                        }
                        self.state.population = self.state.population.saturating_add(added);
                        self.state.population = self.state.population.min(MAX_POPULATION);
                    }
                }
            }
            "rail" => {
                let mut opened_rail = false;
                if let Some(road) = self
                    .state
                    .roads
                    .iter_mut()
                    .find(|r| r.kind == "rail" && Some(r.from) == p.from && Some(r.to) == p.to)
                {
                    opened_rail = road.stage != "open";
                    road.stage = "open".into();
                    road.progress = 1.0;
                }
                if opened_rail {
                    self.mark_access_geometry_changed();
                }
                self.recompute_connectivity();
                self.record(
                    "RailOpened",
                    id,
                    "The completed rail link joins the connected transport network".into(),
                );
                self.state.station_online = self.state.station_ready
                    && self
                        .state
                        .roads
                        .iter()
                        .any(|r| r.kind == "rail" && r.stage == "open" && r.connected);
                if self.state.station_online {
                    self.record(
                        "StationServiceStarted",
                        id,
                        "The completed platform and rail link begin service".into(),
                    );
                }
            }
            "station" => {
                self.state.station_ready = true;
                self.state.station_online = self
                    .state
                    .roads
                    .iter()
                    .any(|r| r.kind == "rail" && r.stage == "open" && r.connected);
                if self.state.station_online {
                    self.record(
                        "StationServiceStarted",
                        id,
                        "The station begins service after its line and platform are ready".into(),
                    );
                }
            }
            _ => {}
        }
        self.retire_completed_neighborhood_plan();
        if p.kind == "road" {
            // The active-road-project corridor is removed at completion even
            // when malformed/legacy project references meant no road opened.
            self.mark_access_geometry_changed();
        }
        self.state.revision = self.state.revision.saturating_add(1);
        self.trim_completed_projects();
    }
    fn trim_completed_projects(&mut self) {
        while self.state.projects.len() > MAX_PROJECTS + COMPLETED_PROJECT_HISTORY {
            let Some(index) = self.state.projects.iter().position(|p| p.progress >= 1.0) else {
                break;
            };
            self.state.projects.remove(index);
        }
    }

    fn complete_interior_access_project(&mut self, project_id: u32) {
        let Some(plan) = self.state.pending_interior_access.clone() else {
            return;
        };
        let Some(project) = self
            .state
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
        else {
            return;
        };
        let Some(branch) = self
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
            .cloned()
        else {
            self.keep_interior_project_retryable(project_id);
            return;
        };
        if project.kind != "road"
            || project.from != Some(branch.from)
            || project.to != Some(branch.to)
            || branch.stage == "open"
            || branch.kind == "rail"
            || self
                .state
                .parcels
                .len()
                .saturating_add(plan.selected_lots.len())
                > MAX_PARCELS
            || self
                .state
                .next_id
                .checked_add(plan.selected_lots.len() as u32)
                .is_none()
        {
            self.keep_interior_project_retryable(project_id);
            return;
        }

        let (Some(a), Some(b)) = (self.node(branch.from), self.node(branch.to)) else {
            self.keep_interior_project_retryable(project_id);
            return;
        };
        let (ax, az, bx, bz) = (a.x, a.z, b.x, b.z);
        let mut context = self.collision_context();
        context.pending_roads.retain(|road| {
            !((road.ax == ax && road.az == az && road.bx == bx && road.bz == bz)
                || (road.bx == ax && road.bz == az && road.ax == bx && road.az == bz))
        });
        context.open_roads.push(CollisionSegment {
            ax: a.x,
            az: a.z,
            bx: b.x,
            bz: b.z,
        });
        let mut promised = Vec::with_capacity(plan.selected_lots.len());
        for lot in &plan.selected_lots {
            let rect = geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if !self.progressive_frontage_geometry_clear_for_interior(&rect, &context)
                || promised.iter().any(|other: &geometry::OrientedRect| {
                    geometry::rects_overlap(&rect, other, 2.0)
                })
            {
                self.keep_interior_project_retryable(project_id);
                return;
            }
            promised.push(rect);
        }

        if let Some(road) = self
            .state
            .roads
            .iter_mut()
            .find(|road| road.id == plan.branch_road_id)
        {
            road.stage = "open".into();
            road.progress = 1.0;
        }
        self.recompute_connectivity();
        if !self
            .state
            .roads
            .iter()
            .any(|road| road.id == plan.branch_road_id && road.connected)
        {
            if let Some(road) = self
                .state
                .roads
                .iter_mut()
                .find(|road| road.id == plan.branch_road_id)
            {
                road.stage = "survey".into();
                road.progress = 0.999;
            }
            self.recompute_connectivity();
            self.keep_interior_project_retryable(project_id);
            return;
        }

        self.state.pending_interior_access = None;
        self.mark_access_geometry_changed();
        for lot in &plan.selected_lots {
            let along_meters = lot.along * branch.length;
            let (Some(from), Some(to)) = (self.node(branch.from), self.node(branch.to)) else {
                unreachable!("preflighted interior access endpoints remain present");
            };
            let access_node = if along_meters <= branch.length * 0.5 {
                from.id
            } else {
                to.id
            };
            let parcel_id =
                self.add_parcel(lot.x, lot.z, lot.width, lot.depth, access_node, 0.72, false);
            assert_ne!(parcel_id, 0, "preflighted access-lot capacity is reserved");
            if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
                parcel.rotation = Some(lot.rotation);
            }
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id: plan.branch_road_id,
                along: lot.along,
                side: lot.side,
            });
            self.mark_access_geometry_changed();
        }
        if let Some(project) = self
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.progress = 1.0;
            project.stage = "finishing".into();
        }
        self.state.road_infill_due = self
            .state
            .road_infill_due
            .saturating_add(plan.selected_lots.len() as u8)
            .min(9);
        self.record(
            "RoadOpened",
            plan.branch_road_id,
            "A new access street opens with reserved, connected neighborhood frontage".into(),
        );
        #[cfg(test)]
        {
            self.interior_access_completions = self.interior_access_completions.saturating_add(1);
        }
        self.state.revision = self.state.revision.saturating_add(1);
        self.trim_completed_projects();
    }

    fn keep_interior_project_retryable(&mut self, project_id: u32) {
        let mut restored_pending_road = false;
        if let Some(project) = self
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            restored_pending_road = project.progress >= 1.0;
            project.progress = 0.999;
            project.stage = "finishing".into();
        }
        if restored_pending_road {
            self.mark_access_geometry_changed();
        }
        if let Some(plan) = &self.state.pending_interior_access {
            if let Some(road) = self
                .state
                .roads
                .iter_mut()
                .find(|road| road.id == plan.branch_road_id)
            {
                if road.stage != "open" {
                    road.progress = 0.999;
                    road.stage = "finishing".into();
                }
            }
        }
    }
    fn project_admission_available(&self) -> bool {
        let active = self
            .state
            .projects
            .iter()
            .filter(|project| project.progress < 1.0)
            .count();
        active < MAX_PROJECTS
            && (self.state.projects.len() < MAX_PROJECTS + COMPLETED_PROJECT_HISTORY
                || self
                    .state
                    .projects
                    .iter()
                    .any(|project| project.progress >= 1.0))
    }
    fn record_project(&mut self, project: Project) {
        let adds_pending_road = project.kind == "road" && project.progress < 1.0;
        while self.state.projects.len() >= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY {
            let Some(index) = self
                .state
                .projects
                .iter()
                .position(|entry| entry.progress >= 1.0)
            else {
                unreachable!("active project capacity must leave completed history to trim");
            };
            self.state.projects.remove(index);
        }
        self.state.projects.push(project);
        if adds_pending_road {
            self.mark_access_geometry_changed();
        }
    }
    fn evaluate_growth(&mut self) {
        if self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .count()
            >= MAX_PROJECTS
        {
            return;
        }
        if self.state.buildings.len() >= MAX_BUILDINGS || self.state.roads.len() >= MAX_SEGMENTS {
            return;
        }
        // Bridge opens before western growth can be considered.
        let day = (self.state.tick / TICKS_PER_DAY) as u32;
        let open_roads = self
            .state
            .roads
            .iter()
            .filter(|road| road.stage == "open")
            .count();
        let homes = self
            .state
            .buildings
            .iter()
            .filter(|b| is_housing(b) && site_active(b))
            .count();
        let businesses = self
            .state
            .buildings
            .iter()
            .filter(|b| is_commercial(b) && site_active(b))
            .count();
        let active = self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .count();
        if open_roads < 3 && active < MAX_PROJECTS {
            return;
        }
        // Finish starter parcels first, then spend each new street's infill quota
        // before considering another frontier extension.
        if self.state.buildings.len() < 5
            && !self.state.parcels.iter().any(|p| p.reservation.is_some())
        {
            self.plan_building(day);
            return;
        }
        let usable_infill = self.state.parcels.iter().any(|parcel| {
            !parcel.protected
                && parcel.reservation.is_none()
                && (self.state.station_online || !self.is_station_tower_lot(parcel))
                && self.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                && !self
                    .state
                    .buildings
                    .iter()
                    .any(|b| b.parcel_id == parcel.id)
                && self.node_connected(parcel.access_node)
        });
        let active_building_projects = self
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "building" && project.progress < 1.0)
            .count();
        let active_road_projects = self
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "road" && project.progress < 1.0)
            .count();
        // Reserve a coordinated block before ordinary infill and building
        // project gates can spend its interior frontage. Reservation only
        // records the plan; perimeter construction remains on the existing
        // later 80-tick scheduler.
        let early_block_reserved = self.try_reserve_early_coordinated_block(active_road_projects);
        if self.state.road_infill_due > 0 {
            if usable_infill {
                if self.state.tick % 80 == 0 {
                    self.plan_building(day);
                }
                return;
            }
            // This quota prioritizes newly opened frontage while usable lots
            // remain. Do not let an exhausted lot pool freeze network growth.
            self.state.road_infill_due = 0;
        }
        if usable_infill || active_building_projects > 0 {
            if usable_infill && self.state.tick % 80 == 0 {
                self.plan_building(day);
            }
            return;
        }
        let connected_ordinary_roads = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .count();
        #[cfg(test)]
        if self.early_access_experiment
            && connected_ordinary_roads >= 40
            && self.state.tick % 160 == 0
            && active_road_projects == 0
            && active == 0
            && self.state.neighborhood_plan.is_none()
            && !self.neighborhood_block_has_unbuilt_frontage()
            && homes + businesses >= 3
            && road_demand_is_sufficient(self.state.demand)
            && self
                .state
                .roads
                .len()
                .saturating_add(self.state.projects.len())
                < MAX_SEGMENTS
        {
            let active_buildings: Vec<Building> = self
                .state
                .buildings
                .iter()
                .filter(|building| site_active(building))
                .cloned()
                .collect();
            let bridgehead_due = !self
                .eligible_bridgehead_headings(&active_buildings)
                .is_empty();
            if !bridgehead_due {
                self.early_access_attempted_tick = Some(self.state.tick);
                self.early_access_attempts = self.early_access_attempts.saturating_add(1);
                if self.try_plan_interior_access() {
                    self.early_access_admissions = self.early_access_admissions.saturating_add(1);
                    return;
                }
            }
        }
        // As established blocks fill, release one additional roadside lot at a
        // time. Candidate work is persisted and capped so older worlds can
        // acquire infill without a full-map scan in a single simulation update.
        if connected_ordinary_roads >= 40
            && self.state.tick % 160 == 0
            && self.try_progressive_frontage_infill()
        {
            return;
        }
        if active_road_projects > 0 {
            return;
        }
        // Give compact blocks the same bounded opportunity cadence as road
        // extensions, after existing usable frontage has had priority. The
        // former 1,600-tick phase made block planning depend on tick alignment
        // and often let frontier roads consume the available geometry first.
        if self.state.neighborhood_plan.is_none()
            && self.state.road_infill_due == 0
            && active == 0
            && !self.neighborhood_block_has_unbuilt_frontage()
            && self.state.tick % 80 == 0
            && self.try_create_neighborhood_plan()
        {
            self.plan_next_neighborhood_segment();
            return;
        }
        if self
            .state
            .neighborhood_plan
            .as_ref()
            .is_some_and(|plan| plan.next_segment < plan.segments.len() || plan.spine.is_some())
            && self.state.tick % 80 == 0
        {
            if early_block_reserved {
                return;
            }
            self.plan_next_neighborhood_segment();
            return;
        }
        let connected_nodes: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|r| r.connected && r.stage == "open")
            .flat_map(|r| [r.from, r.to])
            .collect();
        let isolated = self
            .state
            .roads
            .iter()
            .filter(|r| r.kind == "bridge" && r.stage != "open")
            .count()
            > 0;
        // This is the production demand boundary for every road planner,
        // including the district-policy comparison below. Keep ordinary
        // capacity proposals out of that comparison at demand <= 0.8.
        let needs_road = homes + businesses >= 3
            && self.state.roads.len() + self.state.projects.len() < MAX_SEGMENTS
            && road_demand_is_sufficient(self.state.demand);
        if needs_road && !isolated && connected_nodes.len() > 1 && self.state.tick % 80 == 0 {
            self.plan_road();
            return;
        }
        if (self.state.demand > 0.75 || day < 5) && self.state.tick % 80 == 0 {
            self.plan_building(day);
        }
    }

    fn try_reserve_early_coordinated_block(&mut self, active_road_projects: usize) -> bool {
        #[cfg(test)]
        if self.early_block_reservation_disable_override {
            return false;
        }
        if self.state.tick % 80 != 0
            || self.state.neighborhood_plan.is_some()
            || self.state.pending_interior_access.is_some()
            || active_road_projects > 0
        {
            return false;
        }
        self.record_early_block_attempt();
        if self.try_create_neighborhood_plan() {
            self.record_early_block_admission();
            true
        } else {
            self.record_early_block_noop();
            false
        }
    }

    #[cfg(test)]
    fn record_early_block_attempt(&mut self) {
        self.early_block_reservation_attempts =
            self.early_block_reservation_attempts.saturating_add(1);
    }

    #[cfg(not(test))]
    fn record_early_block_attempt(&mut self) {}

    #[cfg(test)]
    fn record_early_block_admission(&mut self) {
        self.early_block_reservation_admissions =
            self.early_block_reservation_admissions.saturating_add(1);
        self.early_block_reservation_started_tick = Some(self.state.tick);
    }

    #[cfg(not(test))]
    fn record_early_block_admission(&mut self) {}

    #[cfg(test)]
    fn record_early_block_noop(&mut self) {
        self.early_block_reservation_noops = self.early_block_reservation_noops.saturating_add(1);
    }

    #[cfg(not(test))]
    fn record_early_block_noop(&mut self) {}

    fn neighborhood_profile(&self, x: f32, z: f32) -> NeighborhoodProfile {
        let station_distance = self
            .state
            .nodes
            .iter()
            .filter(|node| node.kind == "station")
            .map(|node| (node.x - x).hypot(node.z - z))
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY);
        if self.state.station_online && station_distance <= 112.0 {
            return NeighborhoodProfile::StationCore;
        }
        let market_distance = self
            .state
            .nodes
            .iter()
            .filter(|node| node.kind == "market")
            .map(|node| (node.x - x).hypot(node.z - z))
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY);
        if market_distance <= 118.0 {
            NeighborhoodProfile::HistoricCore
        } else if self.state.station_online && station_distance <= 205.0 {
            NeighborhoodProfile::StationFringe
        } else {
            NeighborhoodProfile::OuterNeighborhood
        }
    }

    fn choose_building_profile(
        &mut self,
        parcel: &Parcel,
        homes: usize,
        shops: usize,
    ) -> (&'static str, u16, u8, &'static str) {
        let profile = self.neighborhood_profile(parcel.x, parcel.z);
        let station_tower = self.is_station_tower_parcel(parcel);
        let mut weights = match profile {
            // Keep the first market streets small, walkable, and mixed with
            // neighborhood commerce even after the rail line opens.
            NeighborhoodProfile::HistoricCore => [48.0, 10.0, 24.0, 4.0, 14.0],
            // Concentrate the tallest fabric around the platform itself.
            NeighborhoodProfile::StationCore => [8.0, 10.0, 12.0, 30.0, 40.0],
            NeighborhoodProfile::StationFringe => [38.0, 10.0, 15.0, 14.0, 23.0],
            NeighborhoodProfile::OuterNeighborhood => [70.0, 7.0, 10.0, 4.0, 9.0],
        };
        if station_tower {
            // Reserve large station-facing lots for substantial employment and
            // housing, rather than letting them become oversized low-rise pads.
            weights = [0.0, 0.0, 0.0, 46.0, 54.0];
        }

        // Local balancing creates a varied street face without making districts
        // look like a checkerboard. Keep the census neighborhood-sized and use
        // exact use classes so homes and apartments can still form coherent runs.
        let mut local_use_counts = [0usize; 5];
        let mut local_floor_counts = [0usize; 46];
        for building in self.state.buildings.iter().filter(|building| {
            building.stage == "open" && (building.x - parcel.x).hypot(building.z - parcel.z) <= 58.0
        }) {
            let index = match building.kind.as_str() {
                "home" => Some(0),
                "shop" => Some(1),
                "cafe" => Some(2),
                "office" => Some(3),
                "apartment" => Some(4),
                _ => None,
            };
            if let Some(index) = index {
                local_use_counts[index] += 1;
            }
            if let Some(count) = local_floor_counts.get_mut(building.floors as usize) {
                *count += 1;
            }
        }
        for (weight, count) in weights.iter_mut().zip(local_use_counts) {
            *weight /= 1.0 + count as f32 * 0.62;
        }
        if homes < 3 {
            weights = [100.0, 0.0, 0.0, 0.0, 0.0];
        } else if shops < 2 {
            weights = [0.0, 58.0, 42.0, 0.0, 0.0];
        }
        let total: f32 = weights.iter().sum();
        let mut choice = random_unit(&mut self.state.rng_growth) * total;
        let mut selected = 0usize;
        for (index, weight) in weights.iter().enumerate() {
            if choice < *weight {
                selected = index;
                break;
            }
            choice -= *weight;
        }
        let kind = ["home", "shop", "cafe", "office", "apartment"][selected];
        let floor_options: &[(u8, f32)] = match (profile, kind) {
            (NeighborhoodProfile::HistoricCore, "home") => &[(1, 18.0), (2, 58.0), (3, 24.0)],
            (NeighborhoodProfile::HistoricCore, "shop" | "cafe") => &[(1, 68.0), (2, 32.0)],
            (NeighborhoodProfile::HistoricCore, "office") => &[(2, 55.0), (3, 45.0)],
            (NeighborhoodProfile::HistoricCore, "apartment") => &[(2, 38.0), (3, 47.0), (4, 15.0)],
            (_, "office") if station_tower => &[
                (20, 12.0),
                (24, 16.0),
                (28, 18.0),
                (32, 18.0),
                (36, 16.0),
                (40, 12.0),
                (45, 8.0),
            ],
            (_, "apartment") if station_tower => &[
                (20, 14.0),
                (24, 17.0),
                (28, 19.0),
                (32, 18.0),
                (36, 14.0),
                (40, 11.0),
                (45, 7.0),
            ],
            (NeighborhoodProfile::StationCore, "home") => &[
                (3, 8.0),
                (4, 20.0),
                (5, 28.0),
                (6, 22.0),
                (7, 14.0),
                (8, 8.0),
            ],
            (NeighborhoodProfile::StationCore, "shop" | "cafe") => {
                &[(2, 48.0), (3, 38.0), (4, 14.0)]
            }
            (NeighborhoodProfile::StationCore, "office") => &[
                (5, 12.0),
                (6, 18.0),
                (7, 18.0),
                (8, 18.0),
                (9, 14.0),
                (10, 10.0),
                (12, 10.0),
            ],
            (NeighborhoodProfile::StationCore, "apartment") => &[
                (6, 12.0),
                (7, 17.0),
                (8, 18.0),
                (9, 18.0),
                (10, 16.0),
                (12, 19.0),
            ],
            (NeighborhoodProfile::StationFringe, "home") => &[
                (2, 8.0),
                (3, 24.0),
                (4, 30.0),
                (5, 22.0),
                (6, 12.0),
                (8, 4.0),
            ],
            (NeighborhoodProfile::StationFringe, "shop" | "cafe") => {
                &[(1, 35.0), (2, 52.0), (3, 13.0)]
            }
            (NeighborhoodProfile::StationFringe, "office") => &[
                (4, 10.0),
                (5, 18.0),
                (6, 20.0),
                (7, 18.0),
                (8, 14.0),
                (10, 12.0),
                (12, 8.0),
            ],
            (NeighborhoodProfile::StationFringe, "apartment") => &[
                (4, 7.0),
                (5, 14.0),
                (6, 18.0),
                (7, 18.0),
                (8, 16.0),
                (10, 15.0),
                (12, 12.0),
            ],
            (NeighborhoodProfile::OuterNeighborhood, "home") => &[(1, 22.0), (2, 55.0), (3, 23.0)],
            (NeighborhoodProfile::OuterNeighborhood, "shop" | "cafe") => &[(1, 72.0), (2, 28.0)],
            (NeighborhoodProfile::OuterNeighborhood, "office") => &[(2, 32.0), (3, 68.0)],
            (NeighborhoodProfile::OuterNeighborhood, "apartment") => {
                &[(3, 44.0), (4, 46.0), (5, 10.0)]
            }
            _ => &[(2, 30.0), (3, 52.0), (4, 18.0)],
        };
        let mut floor_weight: Vec<(u8, f32)> = floor_options
            .iter()
            .map(|(floor, weight)| {
                let neighbors = local_floor_counts
                    .get(*floor as usize)
                    .copied()
                    .unwrap_or(0);
                (*floor, *weight / (1.0 + neighbors as f32 * 0.55))
            })
            .collect();
        let total_floor_weight: f32 = floor_weight.iter().map(|(_, weight)| *weight).sum();
        let mut floor_roll = random_unit(&mut self.state.rng_growth) * total_floor_weight;
        let mut floors = floor_weight.last().map(|(floor, _)| *floor).unwrap_or(2);
        for (floor, weight) in floor_weight.drain(..) {
            if floor_roll < weight {
                floors = floor;
                break;
            }
            floor_roll -= weight;
        }
        let capacity = match kind {
            "home" => 6 + floors as u16 * 2,
            "apartment" => floors as u16 * 5,
            "office" => floors as u16 * 5,
            _ => 6 + floors as u16 * 2,
        };
        let name = match (profile, kind, station_tower) {
            (_, "office", true) => "Station Tower Offices",
            (_, "apartment", true) => "Station Tower Residences",
            (NeighborhoodProfile::HistoricCore, "home", false) => "Machiya House",
            (NeighborhoodProfile::HistoricCore, "shop", false) => "Market Shop",
            (NeighborhoodProfile::HistoricCore, "cafe", false) => "Kissa Komorebi",
            (NeighborhoodProfile::StationCore, "office", false) => "Station Office",
            (NeighborhoodProfile::StationCore, "apartment", false) => "Station Apartments",
            (NeighborhoodProfile::StationCore, "shop", false) => "Station Kiosk",
            (NeighborhoodProfile::StationCore, "cafe", false) => "Platform Cafe",
            (NeighborhoodProfile::StationFringe, "apartment", false) => "Railway Apartments",
            (NeighborhoodProfile::StationFringe, "office", false) => "Neighborhood Office",
            (NeighborhoodProfile::OuterNeighborhood, "home", false) => "Lane House",
            (_, "apartment", false) => "Walk-up Apartments",
            (_, "office", false) => "Small Office",
            (_, "shop", false) => "Corner Store",
            (_, "cafe", false) => "Kissa Aoba",
            _ => "Riverside House",
        };
        (kind, capacity, floors, name)
    }

    fn is_station_tower_parcel(&self, parcel: &Parcel) -> bool {
        if !self.state.station_online
            || self.neighborhood_profile(parcel.x, parcel.z) != NeighborhoodProfile::StationCore
            || !self
                .state
                .parcel_frontages
                .iter()
                .any(|frontage| frontage.parcel_id == parcel.id)
        {
            return false;
        }
        self.is_station_tower_lot(parcel)
    }

    fn is_station_tower_lot(&self, parcel: &Parcel) -> bool {
        if parcel.width < STATION_TOWER_PARCEL_WIDTH || parcel.depth < STATION_TOWER_PARCEL_DEPTH {
            return false;
        }
        self.state
            .nodes
            .iter()
            .filter(|node| node.kind == "station")
            .any(|station| (station.x - parcel.x).hypot(station.z - parcel.z) <= 105.0)
    }

    fn station_tower_frontage_reservations(&self) -> usize {
        let frontage_ids: BTreeSet<u32> = self
            .state
            .parcel_frontages
            .iter()
            .map(|frontage| frontage.parcel_id)
            .collect();
        self.state
            .parcels
            .iter()
            .filter(|parcel| {
                parcel.width >= STATION_TOWER_PARCEL_WIDTH
                    && parcel.depth >= STATION_TOWER_PARCEL_DEPTH
                    && self.is_station_tower_lot(parcel)
            })
            .filter(|parcel| frontage_ids.contains(&parcel.id))
            // Only the first two reservations affect admission. Capping this
            // derived count also avoids scanning the rest of a mature parcel set.
            .take(MIN_STATION_TOWER_FRONTAGE_RESERVATIONS)
            .count()
    }

    fn available_station_tower_envelopes(&self) -> Vec<(f32, f32)> {
        let remaining = MIN_STATION_TOWER_FRONTAGE_RESERVATIONS
            .saturating_sub(self.station_tower_frontage_reservations());
        if remaining == 0 {
            return Vec::new();
        }

        let mut candidates: Vec<(f32, u64, f32, f32)> = Vec::new();
        for road in self.state.roads.iter().filter(|road| {
            road.stage == "open"
                && road.connected
                && !matches!(road.kind.as_str(), "rail" | "bridge")
        }) {
            let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                continue;
            };
            self.push_available_station_tower_candidate(road.id, a, b, &mut candidates);
        }
        // A planned street can reserve its exact future parcel envelope before
        // another frontage slot is filled beside the construction corridor.
        for project in self
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "road" && project.progress < 1.0)
        {
            let (Some(from), Some(to)) = (project.from, project.to) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                continue;
            };
            self.push_available_station_tower_candidate(project.id, a, b, &mut candidates);
        }

        candidates.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.3.total_cmp(&right.3))
        });
        let mut reserved = Vec::with_capacity(remaining);
        for (_, _, x, z) in candidates {
            if reserved.iter().any(|(other_x, other_z): &(f32, f32)| {
                (x - *other_x).abs() < STATION_TOWER_PARCEL_WIDTH + 2.0
                    && (z - *other_z).abs() < STATION_TOWER_PARCEL_DEPTH + 2.0
            }) {
                continue;
            }
            reserved.push((x, z));
            if reserved.len() == remaining {
                break;
            }
        }
        reserved
    }

    fn push_available_station_tower_candidate(
        &self,
        road_id: u32,
        a: &Node,
        b: &Node,
        candidates: &mut Vec<(f32, u64, f32, f32)>,
    ) {
        let sites = self.frontage_candidate_sites(a, b);
        let Some(index) = self.station_tower_candidate_index(road_id, a, b, &sites) else {
            return;
        };
        let (_, _, _, along, side) = sites[index];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let x = a.x + dx * along + nx * side as f32 * STATION_TOWER_ROAD_OFFSET;
        let z = a.z + dz * along + nz * side as f32 * STATION_TOWER_ROAD_OFFSET;
        // Only hold a site that could accept a real large parcel right now;
        // this keeps the reservation finite and lets every unrelated lot grow.
        if !self.parcel_geometry_clear(x, z, STATION_TOWER_PARCEL_WIDTH, STATION_TOWER_PARCEL_DEPTH)
        {
            return;
        }
        let station_distance = self
            .state
            .nodes
            .iter()
            .filter(|node| node.kind == "station")
            .map(|station| (station.x - x).hypot(station.z - z))
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY);
        let tie_break = stateless_tower_site_hash(self.state.seed, x, z);
        candidates.push((station_distance, tie_break, x, z));
    }

    fn overlaps_station_tower_envelopes(
        envelopes: &[(f32, f32)],
        x: f32,
        z: f32,
        width: f32,
        depth: f32,
    ) -> bool {
        envelopes.iter().any(|(tower_x, tower_z)| {
            (x - tower_x).abs() < (width + STATION_TOWER_PARCEL_WIDTH) * 0.5 + 2.0
                && (z - tower_z).abs() < (depth + STATION_TOWER_PARCEL_DEPTH) * 0.5 + 2.0
        })
    }

    fn add_frontage_parcels(&mut self, road_id: u32) {
        let Some(road) = self.state.roads.iter().find(|r| r.id == road_id).cloned() else {
            return;
        };
        if road.kind == "rail" || road.kind == "bridge" {
            return;
        }
        let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
        else {
            return;
        };
        let neighborhood_road = self.is_neighborhood_plan_road(road_id);
        let (lot_width, lot_depth) = if neighborhood_road {
            (NEIGHBORHOOD_PARCEL_SIZE, NEIGHBORHOOD_PARCEL_SIZE)
        } else {
            (14.0, 12.0)
        };
        let candidates = if neighborhood_road {
            self.frontage_candidate_sites_with_layout(
                &a,
                &b,
                lot_width,
                lot_depth,
                NEIGHBORHOOD_FRONTAGE_SPACING,
            )
        } else {
            self.frontage_candidate_sites_with_size(&a, &b, lot_width, lot_depth)
        };
        // Keep a small, bounded station-core pipeline reserved from the first
        // connected streets. Waiting until rail service opens lets ordinary
        // frontage from adjacent roads consume the only viable tower plots.
        // These remain undeveloped until service and local demand support them.
        let reserve_tower_frontage =
            self.station_tower_frontage_reservations() < MIN_STATION_TOWER_FRONTAGE_RESERVATIONS;
        let mut tower_frontage_reserved = false;
        if reserve_tower_frontage {
            if let Some(index) = self.station_tower_candidate_index(road_id, &a, &b, &candidates) {
                let tower_candidate = candidates[index];
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let length = dx.hypot(dz).max(1.0);
                let nx = -dz / length;
                let nz = dx / length;
                let x = a.x
                    + dx * tower_candidate.3
                    + nx * tower_candidate.4 as f32 * STATION_TOWER_ROAD_OFFSET;
                let z = a.z
                    + dz * tower_candidate.3
                    + nz * tower_candidate.4 as f32 * STATION_TOWER_ROAD_OFFSET;
                if self.interior_plan_allows_frontage_candidate(
                    &a,
                    &b,
                    x,
                    z,
                    STATION_TOWER_PARCEL_WIDTH,
                    STATION_TOWER_PARCEL_DEPTH,
                    tower_candidate.4,
                ) && self.parcel_geometry_clear(
                    x,
                    z,
                    STATION_TOWER_PARCEL_WIDTH,
                    STATION_TOWER_PARCEL_DEPTH,
                ) {
                    let parcel_id = self.add_parcel(
                        x,
                        z,
                        STATION_TOWER_PARCEL_WIDTH,
                        STATION_TOWER_PARCEL_DEPTH,
                        tower_candidate.2,
                        0.9,
                        false,
                    );
                    if parcel_id != 0 {
                        self.state.parcel_frontages.push(ParcelFrontage {
                            parcel_id,
                            road_id,
                            along: tower_candidate.3,
                            side: tower_candidate.4,
                        });
                        self.mark_access_geometry_changed();
                        tower_frontage_reserved = true;
                    }
                }
            }
        }
        // A new street gets a small pair of starter lots. Remaining slots are
        // unlocked later as occupied frontage creates sustained local demand.
        let mut center_sites = candidates.clone();
        let reserved_tower_site = tower_frontage_reserved
            .then(|| self.station_tower_candidate_index(road_id, &a, &b, &center_sites))
            .flatten()
            .map(|index| (center_sites[index].3, center_sites[index].4));
        let available_tower_envelopes = self.available_station_tower_envelopes();
        center_sites.sort_by(|left, right| {
            (left.3 - 0.5)
                .abs()
                .total_cmp(&(right.3 - 0.5).abs())
                .then_with(|| left.4.cmp(&right.4))
        });
        for (x, z, access, along, side) in center_sites
            .into_iter()
            .filter(|site| neighborhood_road || (site.3 - 0.5).abs() < 0.001)
        {
            // Leave this midpoint clear for a genuine tower footprint until
            // station service and local demand make it developable.
            if reserved_tower_site.is_some_and(|(tower_along, tower_side)| {
                side == tower_side && (along - tower_along).abs() < 0.001
            }) {
                continue;
            }
            if Self::overlaps_station_tower_envelopes(
                &available_tower_envelopes,
                x,
                z,
                lot_width,
                lot_depth,
            ) {
                continue;
            }
            if !self
                .interior_plan_allows_frontage_candidate(&a, &b, x, z, lot_width, lot_depth, side)
                || !self.parcel_geometry_clear(x, z, lot_width, lot_depth)
            {
                continue;
            }
            let parcel_id = self.add_parcel(x, z, lot_width, lot_depth, access, 0.78, false);
            if parcel_id == 0 {
                break;
            }
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
        }
    }
    fn frontage_candidate_sites(&self, a: &Node, b: &Node) -> Vec<(f32, f32, u32, f32, i8)> {
        self.frontage_candidate_sites_with_depth(a, b, 12.0)
    }

    fn frontage_candidate_sites_with_depth(
        &self,
        a: &Node,
        b: &Node,
        lot_depth: f32,
    ) -> Vec<(f32, f32, u32, f32, i8)> {
        self.frontage_candidate_sites_with_size(a, b, 14.0, lot_depth)
    }

    fn frontage_candidate_sites_with_size(
        &self,
        a: &Node,
        b: &Node,
        lot_width: f32,
        lot_depth: f32,
    ) -> Vec<(f32, f32, u32, f32, i8)> {
        self.frontage_candidate_sites_with_layout(a, b, lot_width, lot_depth, FRONTAGE_SLOT_SPACING)
    }

    fn frontage_candidate_sites_with_layout(
        &self,
        a: &Node,
        b: &Node,
        lot_width: f32,
        lot_depth: f32,
        spacing: f32,
    ) -> Vec<(f32, f32, u32, f32, i8)> {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let max_steps =
            (((length * 0.5 - FRONTAGE_ENDPOINT_CLEARANCE) / spacing).floor() as usize).min(8);
        let positions: Vec<f32> = (-(max_steps as isize)..=max_steps as isize)
            .map(|step| (0.5 + step as f32 * spacing / length).clamp(0.0, 1.0))
            .collect();
        let mut sites = Vec::with_capacity(positions.len() * 2);
        for along in positions {
            for side in [-1.0_f32, 1.0] {
                let x = a.x + dx * along + nx * side * 15.0;
                let z = a.z + dz * along + nz * side * 15.0;
                if !self.parcel_clear_of_river(x, z, lot_width, lot_depth) {
                    continue;
                }
                let access = frontage_access_node(a, b, along);
                sites.push((x, z, access, along, side as i8));
            }
        }
        sites
    }

    fn is_neighborhood_plan_road(&self, road_id: u32) -> bool {
        self.state.neighborhood_road_ids.contains(&road_id)
            || self.state.neighborhood_plan.as_ref().is_some_and(|plan| {
                self.state.roads.iter().any(|road| {
                    road.id == road_id
                        && plan.segments.iter().any(|(from, to)| {
                            (road.from == *from && road.to == *to)
                                || (road.from == *to && road.to == *from)
                        })
                })
            })
    }

    fn neighborhood_block_has_unbuilt_frontage(&self) -> bool {
        let block_parcels: BTreeSet<u32> = self
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| self.state.neighborhood_road_ids.contains(&frontage.road_id))
            .map(|frontage| frontage.parcel_id)
            .collect();
        if block_parcels.is_empty() {
            return false;
        }
        let built_parcels: BTreeSet<u32> = self
            .state
            .buildings
            .iter()
            .map(|building| building.parcel_id)
            .collect();
        self.state.parcels.iter().any(|parcel| {
            block_parcels.contains(&parcel.id)
                && !parcel.protected
                && parcel.reservation.is_none()
                && (self.state.station_online || !self.is_station_tower_lot(parcel))
                && self.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                && self.node_connected(parcel.access_node)
                && !built_parcels.contains(&parcel.id)
        })
    }

    fn remember_neighborhood_plan_road(&mut self, road_id: u32, from: u32, to: u32) {
        let belongs_to_plan = self.state.neighborhood_plan.as_ref().is_some_and(|plan| {
            plan.segments
                .iter()
                .any(|(a, b)| (*a == from && *b == to) || (*a == to && *b == from))
        });
        if belongs_to_plan && self.state.neighborhood_road_ids.insert(road_id) {
            self.mark_access_geometry_changed();
        }
    }

    fn retire_completed_neighborhood_plan(&mut self) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        if plan.segments.len() != 4 || plan.next_segment != plan.segments.len() {
            return false;
        }
        if plan.spine.is_some() {
            let changed = self.advance_neighborhood_spine();
            if changed {
                self.mark_access_geometry_changed();
            }
            return changed;
        }
        let segments = plan.segments.clone();
        let coordinated_lots = plan.coordinated_lots;
        let selected_lots = plan.selected_lots.clone();
        let ready = segments.iter().all(|(from, to)| {
            self.state.roads.iter().any(|road| {
                road.stage == "open"
                    && road.connected
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            }) && !self.state.projects.iter().any(|project| {
                project.kind == "road"
                    && project.progress < 1.0
                    && project
                        .from
                        .zip(project.to)
                        .is_some_and(|(a, b)| (a == *from && b == *to) || (a == *to && b == *from))
            })
        });
        if !ready {
            return false;
        }
        if coordinated_lots
            && selected_lots
                .as_ref()
                .is_some_and(|lots| !self.selected_block_lots_can_be_admitted(&segments, lots))
        {
            // Keep the completed plan and its interior reservation intact if
            // an external mutation invalidated a promised lot.
            return false;
        }
        for (from, to) in &segments {
            if let Some(road) = self.state.roads.iter().find(|road| {
                (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
            }) {
                self.state.neighborhood_road_ids.insert(road.id);
            }
        }
        self.state.neighborhood_plan = None;
        // Clearing a coordinated plan releases its reserved interior even if
        // the plan admitted no new parcels (legacy plans included).
        self.mark_access_geometry_changed();
        if coordinated_lots {
            self.admit_completed_block_lots(&segments, selected_lots.as_deref());
        }
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }

    fn admit_completed_block_lots(
        &mut self,
        segments: &[(u32, u32)],
        selected_lots: Option<&[blocks::BlockLot]>,
    ) {
        if segments.len() != 4 {
            return;
        }
        let mut corners = [blocks::BlockPoint { x: 0.0, z: 0.0 }; 4];
        let mut road_ids = [0_u32; 4];
        for (index, (from, to)) in segments.iter().enumerate() {
            let Some(node) = self.node(*from) else {
                return;
            };
            corners[index] = blocks::BlockPoint {
                x: node.x,
                z: node.z,
            };
            let Some(road) = self.state.roads.iter().find(|road| {
                road.stage == "open"
                    && road.connected
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            }) else {
                return;
            };
            road_ids[index] = road.id;
        }
        let generated_lots;
        let lots = if let Some(selected) = selected_lots {
            selected
        } else {
            generated_lots = blocks::subdivide_rectangular_block(corners, 8.0, 11.0, 2.2);
            &generated_lots
        };
        for lot in lots {
            let rect = geometry::OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if selected_lots.is_none() && !self.oriented_parcel_geometry_clear(&rect) {
                continue;
            }
            let road_id = road_ids[lot.frontage_edge];
            let Some(road) = self.state.roads.iter().find(|road| road.id == road_id) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                continue;
            };
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            // Frontage metadata follows the actual road direction even when
            // the stored block edge was planned with reversed endpoints.
            let along =
                (((rect.x - a.x) * dx + (rect.z - a.z) * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
            let side = if dx * (rect.z - a.z) - dz * (rect.x - a.x) >= 0.0 {
                1
            } else {
                -1
            };
            let access = frontage_access_node(a, b, along);
            let parcel_id =
                self.add_parcel(rect.x, rect.z, rect.width, rect.depth, access, 0.82, false);
            if parcel_id == 0 {
                break;
            }
            if let Some(parcel) = self.state.parcels.last_mut() {
                parcel.rotation = Some(rect.rotation);
            }
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
        }
    }

    fn coordinated_block_lots(&self, segments: &[(u32, u32)]) -> Option<Vec<blocks::BlockLot>> {
        if segments.len() != 4 {
            return None;
        }
        let mut corners = [blocks::BlockPoint { x: 0.0, z: 0.0 }; 4];
        for (index, (from, _)) in segments.iter().enumerate() {
            let node = self.node(*from)?;
            corners[index] = blocks::BlockPoint {
                x: node.x,
                z: node.z,
            };
        }
        Some(blocks::subdivide_rectangular_block(corners, 8.0, 11.0, 2.2))
    }

    fn available_coordinated_block_lots(
        &self,
        corners: &[WorldPoint; 4],
        context: &CollisionContext,
    ) -> Vec<blocks::BlockLot> {
        let canonical = blocks::subdivide_rectangular_block(
            corners.map(|point| blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            8.0,
            11.0,
            2.2,
        );
        canonical
            .into_iter()
            .filter(|lot| {
                self.oriented_parcel_geometry_clear_with_context(
                    &geometry::OrientedRect {
                        x: lot.center.x,
                        z: lot.center.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    },
                    context,
                )
            })
            .collect()
    }

    fn coordinated_plan_lot_count(&self) -> Option<usize> {
        let plan = self.state.neighborhood_plan.as_ref()?;
        if !plan.coordinated_lots {
            return None;
        }
        if let Some(spine) = &plan.spine {
            return spine.validate().ok().map(|layout| layout.lots.len());
        }
        if let Some(lots) = &plan.selected_lots {
            Some(lots.len())
        } else {
            self.coordinated_block_lots(&plan.segments)
                .map(|lots| lots.len())
        }
    }

    fn selected_block_lots_can_be_admitted(
        &self,
        segments: &[(u32, u32)],
        lots: &[blocks::BlockLot],
    ) -> bool {
        if self.state.parcels.len().saturating_add(lots.len()) > MAX_PARCELS
            || !self.selected_block_layout_valid(segments, lots)
        {
            return false;
        }
        segments.iter().all(|(from, to)| {
            self.state.roads.iter().any(|road| {
                road.stage == "open"
                    && road.connected
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            })
        })
    }

    fn selected_block_layout_valid(
        &self,
        segments: &[(u32, u32)],
        lots: &[blocks::BlockLot],
    ) -> bool {
        if lots.len() < 4 || lots.len() > 64 {
            return false;
        }
        let Some(canonical) = self.coordinated_block_lots(segments) else {
            return false;
        };
        let Some((reuse_from, reuse_to)) = segments.first() else {
            return false;
        };
        if !self.state.roads.iter().any(|road| {
            road.stage == "open"
                && road.connected
                && !matches!(road.kind.as_str(), "rail" | "bridge")
                && ((road.from == *reuse_from && road.to == *reuse_to)
                    || (road.from == *reuse_to && road.to == *reuse_from))
        }) {
            return false;
        }
        for index in 1..segments.len() {
            let Some((from, to)) = segments.get(index) else {
                return false;
            };
            let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                return false;
            };
            if distance(a, b) >= 70.0
                && lots.iter().filter(|lot| lot.frontage_edge == index).count() < 2
            {
                return false;
            }
        }
        let context = self.collision_context();
        for (index, lot) in lots.iter().enumerate() {
            if !canonical.contains(lot)
                || !lot.center.x.is_finite()
                || !lot.center.z.is_finite()
                || !lot.width.is_finite()
                || lot.width <= 0.0
                || !lot.depth.is_finite()
                || lot.depth <= 0.0
                || !lot.rotation.is_finite()
                || !lot.frontage_t.is_finite()
                || !(0.0..=1.0).contains(&lot.frontage_t)
                || lot.frontage_edge >= 4
            {
                return false;
            }
            let rect = geometry::OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if !self.oriented_parcel_geometry_clear_with_context_mode(&rect, &context, true)
                || lots[..index].iter().any(|other| {
                    geometry::rects_overlap(
                        &rect,
                        &geometry::OrientedRect {
                            x: other.center.x,
                            z: other.center.z,
                            width: other.width,
                            depth: other.depth,
                            rotation: other.rotation,
                        },
                        2.0,
                    )
                })
            {
                return false;
            }
            let Some((from, to)) = segments.get(lot.frontage_edge) else {
                return false;
            };
            if !self.node(*from).is_some_and(|node| {
                (node.x - lot.center.x).is_finite() && (node.z - lot.center.z).is_finite()
            }) || self.node(*to).is_none()
            {
                return false;
            }
        }
        true
    }

    fn coordinated_block_interior(
        &self,
        segments: &[(u32, u32)],
    ) -> Option<geometry::OrientedRect> {
        if segments.len() != 4 {
            return None;
        }
        let vertices: Vec<_> = segments
            .iter()
            .map(|(from, _)| self.node(*from))
            .collect::<Option<_>>()?;
        let dx = vertices[1].x - vertices[0].x;
        let dz = vertices[1].z - vertices[0].z;
        Some(geometry::OrientedRect {
            x: vertices.iter().map(|node| node.x).sum::<f32>() * 0.25,
            z: vertices.iter().map(|node| node.z).sum::<f32>() * 0.25,
            width: dx.hypot(dz) - 10.0,
            depth: distance(vertices[1], vertices[2]) - 10.0,
            rotation: -dz.atan2(dx),
        })
    }

    /// Selects a midpoint tower-sized site on at most one side of an eligible
    /// street. Its same-side small starter lot is withheld, so the larger
    /// footprint can use the same street frontage without overlapping it.
    fn station_tower_candidate_index(
        &self,
        _road_id: u32,
        a: &Node,
        b: &Node,
        candidates: &[(f32, f32, u32, f32, i8)],
    ) -> Option<usize> {
        let station = self
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "station")?;
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz);
        if length < 42.0 {
            return None;
        }
        let ux = dx / length;
        let uz = dz / length;
        let nx = -uz;
        let nz = ux;
        let station_along =
            (((station.x - a.x) * dx + (station.z - a.z) * dz) / (length * length)).clamp(0.0, 1.0);
        let projected_x = a.x + dx * station_along;
        let projected_z = a.z + dz * station_along;
        let side_offset = (station.x - projected_x) * nx + (station.z - projected_z) * nz;
        let station_side = if side_offset.abs() < 0.01 {
            if stateless_tower_site_hash(self.state.seed, projected_x, projected_z) & 1 == 0 {
                1
            } else {
                -1
            }
        } else if side_offset > 0.0 {
            1
        } else {
            -1
        };

        candidates
            .iter()
            .enumerate()
            .filter(|(_, (_, _, _, along, side))| {
                (*along - 0.5).abs() < 0.001 && *side == station_side
            })
            .map(|(index, (_, _, _, along, side))| {
                let x = a.x + dx * *along + nx * *side as f32 * STATION_TOWER_ROAD_OFFSET;
                let z = a.z + dz * *along + nz * *side as f32 * STATION_TOWER_ROAD_OFFSET;
                (
                    index,
                    (x - station.x).hypot(z - station.z),
                    (*along - station_along).abs(),
                )
            })
            .filter(|(_, distance, _)| *distance <= 100.0)
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.2.total_cmp(&right.2))
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|(index, _, _)| index)
    }

    #[cfg(test)]
    fn frontage_sites(&self, a: &Node, b: &Node) -> Vec<(f32, f32, u32, f32, i8)> {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let mut sites = Vec::with_capacity(2);
        for side in [-1.0_f32, 1.0] {
            let along = 0.5;
            let x = a.x + dx * along + nx * side * 15.0;
            let z = a.z + dz * along + nz * side * 15.0;
            if !self.parcel_geometry_clear(x, z, 14.0, 12.0) {
                continue;
            }
            let access = frontage_access_node(a, b, along);
            sites.push((x, z, access, along, side as i8));
        }
        sites
    }

    /// Count a small number of genuinely usable lots along a proposed road.
    /// The frontage sampler is capped at 34 positions total. Both the legacy
    /// axis-aligned admission envelope and the actual road-facing footprint
    /// must be clear, so each counted site can survive ordinary parcel
    /// admission after construction. Adjacent sample positions have 16 m
    /// center spacing for 14 m frontage, leaving a 2 m gap.
    fn viable_frontage_benefit_count(&self, a: &Node, b: &Node, limit: usize) -> usize {
        self.viable_frontage_benefit_sites(a, b, limit).len()
    }

    /// Return the same mutually compatible opportunities used by road admission.
    /// These are potential lots, not reservations or promised occupied floor area.
    fn viable_frontage_benefit_sites(
        &self,
        a: &Node,
        b: &Node,
        limit: usize,
    ) -> Vec<geometry::OrientedRect> {
        if limit == 0 {
            return Vec::new();
        }
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let candidates = self.frontage_candidate_sites_with_layout(a, b, 14.0, 12.0, 16.0);
        let context = self.collision_context();
        let mut accepted_axis: Vec<geometry::OrientedRect> = Vec::with_capacity(limit.min(2));
        let mut accepted_oriented: Vec<geometry::OrientedRect> = Vec::with_capacity(limit.min(2));
        for (x, z, _, _, side) in candidates {
            let axis = geometry::OrientedRect {
                x,
                z,
                width: 14.0,
                depth: 12.0,
                rotation: 0.0,
            };
            if !self.parcel_geometry_clear(x, z, 14.0, 12.0) {
                continue;
            }
            if accepted_axis
                .iter()
                .any(|existing| geometry::rects_overlap(&axis, existing, 2.0))
                || accepted_oriented
                    .iter()
                    .any(|existing| geometry::rects_overlap(&axis, existing, 2.0))
            {
                continue;
            }
            let oriented = geometry::OrientedRect {
                rotation: (side as f32 * dz).atan2(-side as f32 * dx),
                ..axis
            };
            if !self.oriented_parcel_geometry_clear_with_context(&oriented, &context)
                || accepted_axis
                    .iter()
                    .any(|existing| geometry::rects_overlap(&oriented, existing, 2.0))
                || accepted_oriented
                    .iter()
                    .any(|existing| geometry::rects_overlap(&oriented, existing, 2.0))
            {
                continue;
            }
            accepted_axis.push(axis);
            accepted_oriented.push(oriented);
            if accepted_oriented.len() >= limit {
                return accepted_oriented;
            }
        }
        accepted_oriented
    }

    /// Project reserved footprints into each roadside strip. Subdivision only
    /// proposes sites; admission still checks their complete oriented geometry.
    fn frontage_gap_candidates(
        &self,
        a: &Node,
        b: &Node,
        context: &CollisionContext,
        tower_envelopes: &[(f32, f32)],
    ) -> Vec<((f32, f32, u32, f32, i8), f32)> {
        self.frontage_gap_candidates_with_layout(
            a,
            b,
            context,
            tower_envelopes,
            FRONTAGE_GAP_DEPTH,
            FRONTAGE_GAP_OFFSET,
        )
    }

    fn frontage_gap_candidates_with_layout(
        &self,
        a: &Node,
        b: &Node,
        context: &CollisionContext,
        tower_envelopes: &[(f32, f32)],
        lot_depth: f32,
        center_offset: f32,
    ) -> Vec<((f32, f32, u32, f32, i8), f32)> {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz);
        if !length.is_finite()
            || length <= 0.0
            || !lot_depth.is_finite()
            || lot_depth <= 0.0
            || !center_offset.is_finite()
            || center_offset < lot_depth * 0.5 + 2.0
        {
            return Vec::new();
        }
        let (ux, uz) = (dx / length, dz / length);
        let (nx, nz) = (-uz, ux);
        let mut result = Vec::new();
        for side in [-1_i8, 1] {
            let mut exclusions = Vec::new();
            let towers = tower_envelopes
                .iter()
                .map(|&(x, z)| geometry::OrientedRect {
                    x,
                    z,
                    width: STATION_TOWER_PARCEL_WIDTH,
                    depth: STATION_TOWER_PARCEL_DEPTH,
                    rotation: 0.0,
                });
            for footprint in context.parcel_footprints.iter().copied().chain(towers) {
                let mut along_min = f32::INFINITY;
                let mut along_max = f32::NEG_INFINITY;
                let mut normal_min = f32::INFINITY;
                let mut normal_max = f32::NEG_INFINITY;
                for (x, z) in footprint.corners() {
                    let (rx, rz) = (x - a.x, z - a.z);
                    let along = rx * ux + rz * uz;
                    let normal = (rx * nx + rz * nz) * side as f32;
                    along_min = along_min.min(along);
                    along_max = along_max.max(along);
                    normal_min = normal_min.min(normal);
                    normal_max = normal_max.max(normal);
                }
                // Project obstacles touching the parcel strip, including a
                // 2m safety buffer. The default layout is 12m deep at 15m;
                // diagnostics can compare shallower alternatives without
                // changing production candidate ordering.
                let strip_min = center_offset - lot_depth * 0.5 - 2.0;
                let strip_max = center_offset + lot_depth * 0.5 + 2.0;
                if normal_max >= strip_min && normal_min <= strip_max {
                    exclusions.push(frontage::Interval {
                        start: along_min - 2.0,
                        end: along_max + 2.0,
                    });
                }
            }
            for interval in frontage::subdivide_frontage(
                length,
                &exclusions,
                FRONTAGE_ENDPOINT_CLEARANCE,
                8.0,
                14.0,
                2.2,
                MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
            ) {
                let along = (interval.start + interval.end) * 0.5 / length;
                let x = a.x + dx * along + nx * side as f32 * center_offset;
                let z = a.z + dz * along + nz * side as f32 * center_offset;
                result.push((
                    (x, z, frontage_access_node(a, b, along), along, side),
                    interval.end - interval.start,
                ));
            }
        }
        result
    }

    fn frontage_candidates_with_shallow_fallback(
        &self,
        road_id: u32,
        a: &Node,
        b: &Node,
        collision_context: &CollisionContext,
        tower_envelopes: &[(f32, f32)],
        grid_width: f32,
        grid_depth: f32,
        spacing: f32,
    ) -> (Vec<(f32, f32, u32, f32, i8)>, Vec<(f32, f32)>, bool) {
        let mut candidates =
            self.frontage_candidate_sites_with_layout(a, b, grid_width, grid_depth, spacing);
        let tower_candidate = self.station_tower_candidate_index(road_id, a, b, &candidates);
        // Preserve the existing frontage ordering and station-tower priority.
        if let Some(index) = tower_candidate {
            candidates.swap(0, index);
        }
        let mut dimensions = vec![(grid_width, grid_depth); candidates.len()];
        for (site, width) in self.frontage_gap_candidates(a, b, collision_context, tower_envelopes)
        {
            candidates.push(site);
            dimensions.push((width, FRONTAGE_GAP_DEPTH));
        }
        for (site, width) in self.frontage_gap_candidates_with_layout(
            a,
            b,
            collision_context,
            tower_envelopes,
            FRONTAGE_SHALLOW_DEPTH,
            FRONTAGE_SHALLOW_OFFSET,
        ) {
            let (x, z, _, _, _) = site;
            let duplicate_shape = candidates.iter().zip(dimensions.iter()).any(
                |(candidate, (old_width, old_depth))| {
                    (candidate.0 - x).hypot(candidate.1 - z) <= 1.0e-4
                        && (*old_width - width).abs() <= 1.0e-4
                        && (*old_depth - FRONTAGE_SHALLOW_DEPTH).abs() <= 1.0e-4
                },
            );
            if !duplicate_shape {
                candidates.push(site);
                dimensions.push((width, FRONTAGE_SHALLOW_DEPTH));
            }
        }
        debug_assert!(candidates.len() <= MAX_FRONTAGE_SITES_PER_ROAD);
        (candidates, dimensions, tower_candidate.is_some())
    }

    fn try_progressive_frontage_infill(&mut self) -> bool {
        if self.state.parcels.len() >= MAX_PARCELS
            || self.state.buildings.len() >= MAX_BUILDINGS
            || self.state.demand < 1.2
            || self
                .state
                .buildings
                .iter()
                .filter(|b| site_active(b))
                .count()
                < 8
        {
            return false;
        }

        let mut road_index = self.state.frontage_road_cursor.min(self.state.roads.len());
        let mut slot_index = self.state.frontage_slot_cursor as usize;
        let mut roads_visited = 0;
        let mut sites_visited = 0;
        let mut wrapped = false;
        // This evaluation can inspect up to 48 sites. The envelope set does
        // not change unless a parcel is admitted, which returns immediately.
        let tower_envelopes = self.available_station_tower_envelopes();
        let collision_context = self.collision_context();
        let mut candidate_cache = BTreeMap::new();
        while roads_visited < MAX_FRONTAGE_ROADS_PER_EVALUATION
            && sites_visited < MAX_FRONTAGE_SITES_PER_EVALUATION
        {
            if road_index >= self.state.roads.len() {
                if wrapped {
                    self.state.frontage_road_cursor = 0;
                    self.state.frontage_slot_cursor = 0;
                    return false;
                }
                road_index = 0;
                slot_index = 0;
                wrapped = true;
            }
            let road = &self.state.roads[road_index];
            let road_id = road.id;
            if matches!(road.kind.as_str(), "rail" | "bridge") {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            }
            // This scan is bounded and persistent. Skip temporarily unusable
            // roads so they do not starve later streets; cursor wrap revisits
            // them after they open or reconnect.
            if road.stage != "open" || !road.connected {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            }
            let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
            else {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            };
            let (lot_width, lot_depth) = if self.is_neighborhood_plan_road(road_id) {
                (NEIGHBORHOOD_PARCEL_SIZE, NEIGHBORHOOD_PARCEL_SIZE)
            } else {
                (14.0, 12.0)
            };
            let spacing = if self.is_neighborhood_plan_road(road_id) {
                NEIGHBORHOOD_FRONTAGE_SPACING
            } else {
                FRONTAGE_SLOT_SPACING
            };
            // Reuse each road's projected gaps while visiting its slots. This
            // cache lives only until a single admission or evaluation ends.
            let (candidates, dimensions, has_tower_candidate) =
                candidate_cache.entry(road_id).or_insert_with(|| {
                    self.frontage_candidates_with_shallow_fallback(
                        road_id,
                        &a,
                        &b,
                        &collision_context,
                        &tower_envelopes,
                        lot_width,
                        lot_depth,
                        spacing,
                    )
                });
            if slot_index >= candidates.len() {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            }
            let (x, z, access_node, along, side) = candidates[slot_index];
            let (lot_width, lot_depth) = dimensions[slot_index];
            let tower_site = slot_index == 0 && *has_tower_candidate;
            slot_index += 1;
            sites_visited += 1;
            self.state.frontage_road_cursor = road_index;
            self.state.frontage_slot_cursor = slot_index.min(u16::MAX as usize) as u16;

            // Keep one compact station-frontage opportunity clear until rail
            // service is real. The deterministic cursor will revisit it after
            // the line opens; ordinary sites continue to infill meanwhile.
            if tower_site && !self.state.station_online {
                continue;
            }
            if !tower_site
                && Self::overlaps_station_tower_envelopes(
                    &tower_envelopes,
                    x,
                    z,
                    lot_width,
                    lot_depth,
                )
            {
                continue;
            }

            let (parcel_x, parcel_z, parcel_width, parcel_depth) = if tower_site {
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let length = dx.hypot(dz).max(1.0);
                let nx = -dz / length;
                let nz = dx / length;
                (
                    a.x + dx * along + nx * side as f32 * STATION_TOWER_ROAD_OFFSET,
                    a.z + dz * along + nz * side as f32 * STATION_TOWER_ROAD_OFFSET,
                    STATION_TOWER_PARCEL_WIDTH,
                    STATION_TOWER_PARCEL_DEPTH,
                )
            } else {
                (x, z, 14.0, 12.0)
            };
            let (parcel_width, parcel_depth) = if tower_site {
                (parcel_width, parcel_depth)
            } else {
                (lot_width, lot_depth)
            };
            let parcel_rotation = if tower_site {
                None
            } else {
                Some((side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x)))
            };
            let geometry_clear = if let Some(rotation) = parcel_rotation {
                self.progressive_frontage_geometry_clear(
                    &geometry::OrientedRect {
                        x: parcel_x,
                        z: parcel_z,
                        width: parcel_width,
                        depth: parcel_depth,
                        rotation,
                    },
                    &collision_context,
                )
            } else {
                // Tower opportunities keep their established admission path.
                self.parcel_geometry_clear(parcel_x, parcel_z, parcel_width, parcel_depth)
            };
            if !geometry_clear {
                continue;
            }
            let nearby = self.state.buildings.iter().filter(|building| {
                building.stage == "open"
                    && (building.x - parcel_x).hypot(building.z - parcel_z) <= 78.0
            });
            let mut local_open = 0usize;
            let mut local_occupied = 0usize;
            for building in nearby {
                local_open += 1;
                local_occupied += usize::from(site_active(building));
            }
            if local_open < 4 || local_occupied < 3 {
                continue;
            }
            let parcel_id = self.add_parcel(
                parcel_x,
                parcel_z,
                parcel_width,
                parcel_depth,
                access_node,
                if tower_site { 0.9 } else { 0.74 },
                false,
            );
            if parcel_id == 0 {
                return false;
            }
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
            if let Some(rotation) = parcel_rotation {
                if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
                    parcel.rotation = Some(rotation);
                }
            }
            self.state.road_infill_due = self.state.road_infill_due.saturating_add(1).min(9);
            self.state.revision = self.state.revision.saturating_add(1);
            return true;
        }
        self.state.frontage_road_cursor = road_index.min(self.state.roads.len());
        self.state.frontage_slot_cursor = slot_index.min(u16::MAX as usize) as u16;
        false
    }
    fn ordinary_road_bearings(&self, node_id: u32) -> Vec<f32> {
        let mut bearings = Vec::new();
        for road in self.state.roads.iter().filter(|road| {
            road.connected
                && road.stage == "open"
                && !matches!(road.kind.as_str(), "rail" | "bridge")
                && (road.from == node_id || road.to == node_id)
        }) {
            let neighbor_id = if road.from == node_id {
                road.to
            } else {
                road.from
            };
            if let Some(neighbor) = self.node(neighbor_id) {
                if let Some(node) = self.node(node_id) {
                    bearings.push((neighbor.z - node.z).atan2(neighbor.x - node.x));
                }
            }
        }
        for project in self.state.projects.iter().filter(|project| {
            project.kind == "road"
                && project.progress < 1.0
                && (project.from == Some(node_id) || project.to == Some(node_id))
        }) {
            let neighbor_id = if project.from == Some(node_id) {
                project.to
            } else {
                project.from
            };
            if let (Some(node), Some(neighbor)) =
                (self.node(node_id), neighbor_id.and_then(|id| self.node(id)))
            {
                bearings.push((neighbor.z - node.z).atan2(neighbor.x - node.x));
            }
        }
        bearings
    }

    fn ordinary_road_degree(&self, node_id: u32) -> usize {
        let open = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.connected
                    && road.stage == "open"
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
                    && (road.from == node_id || road.to == node_id)
            })
            .count();
        let planned = self
            .state
            .projects
            .iter()
            .filter(|project| {
                project.kind == "road"
                    && project.progress < 1.0
                    && (project.from == Some(node_id) || project.to == Some(node_id))
            })
            .count();
        open + planned
    }

    fn ordinary_heading_clear(&self, node_id: u32, heading: f32) -> bool {
        const MIN_JUNCTION_ANGLE: f32 = std::f32::consts::PI / 6.0;
        self.ordinary_road_bearings(node_id).iter().all(|existing| {
            let difference = (heading - existing).rem_euclid(std::f32::consts::TAU);
            difference.min(std::f32::consts::TAU - difference) >= MIN_JUNCTION_ANGLE
        })
    }

    fn road_extension_headings(&self, node_id: u32) -> Vec<f32> {
        let mut headings = Vec::new();
        for bearing in self.ordinary_road_bearings(node_id) {
            // A continuation extends the corridor; the perpendicular choices
            // create the cross streets that turn a string of lots into blocks.
            for heading in [
                bearing + std::f32::consts::PI,
                bearing + std::f32::consts::FRAC_PI_2,
                bearing - std::f32::consts::FRAC_PI_2,
            ] {
                let normalized = heading.rem_euclid(std::f32::consts::TAU);
                if !headings.iter().any(|existing: &f32| {
                    let difference = (normalized - existing).abs();
                    difference.min(std::f32::consts::TAU - difference) < 0.12
                }) {
                    headings.push(normalized);
                }
            }
        }
        headings
    }

    fn neighborhood_corridor_clear_for_parcel(
        &self,
        x: f32,
        z: f32,
        width: f32,
        depth: f32,
    ) -> bool {
        let Some(plan) = &self.state.neighborhood_plan else {
            return true;
        };
        if plan.coordinated_lots && plan.segments.len() == 4 {
            if let Some(interior) = self.coordinated_block_interior(&plan.segments) {
                let candidate = geometry::OrientedRect {
                    x,
                    z,
                    width,
                    depth,
                    rotation: 0.0,
                };
                if geometry::rects_overlap(&interior, &candidate, 0.0) {
                    return false;
                }
            }
        }
        let clearance = (width.max(depth) * 0.5 + 5.0).max(9.0);
        plan.segments
            .iter()
            .skip(plan.next_segment)
            .all(|(from, to)| {
                let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                    return true;
                };
                point_segment_distance(x, z, a.x, a.z, b.x, b.z) >= clearance
            })
    }

    fn neighborhood_corridor_clear_for_road(
        &self,
        start: &Node,
        end: &Node,
        start_id: u32,
    ) -> bool {
        let Some(plan) = &self.state.neighborhood_plan else {
            return true;
        };
        if plan.coordinated_lots {
            let Some(interior) = self.coordinated_block_interior(&plan.segments) else {
                return false;
            };
            let same_point =
                |a: &Node, b: &Node| (a.x - b.x).abs() <= 1.0e-4 && (a.z - b.z).abs() <= 1.0e-4;
            let is_perimeter_edge = plan.segments.iter().any(|(from, to)| {
                let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                    return false;
                };
                (same_point(start, a) && same_point(end, b))
                    || (same_point(start, b) && same_point(end, a))
            });
            if is_perimeter_edge {
                return true;
            }
            if !interior.finite_positive()
                || geometry::rect_segment_distance(&interior, start.x, start.z, end.x, end.z) <= 4.5
            {
                return false;
            }
        }
        plan.segments
            .iter()
            .skip(plan.next_segment)
            .all(|(from, to)| {
                let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                    return true;
                };
                let shares_node = *from == start_id || *to == start_id;
                shares_node || segments_distance(start, end, a, b) > 8.0
            })
    }

    fn neighborhood_candidate_clear(
        &self,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        tower_envelopes: &[(f32, f32)],
        collision_context: &CollisionContext,
    ) -> bool {
        let mut corner_ids = [None; 4];
        corner_ids[0] = Some(anchor.id);
        self.neighborhood_candidate_clear_with_edge(
            anchor,
            corners,
            tower_envelopes,
            collision_context,
            corner_ids,
            None,
            None,
        )
    }

    fn neighborhood_candidate_clear_with_edge(
        &self,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        tower_envelopes: &[(f32, f32)],
        collision_context: &CollisionContext,
        corner_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<&[blocks::BlockLot]>,
    ) -> bool {
        self.neighborhood_candidate_clear_with_layout(
            anchor,
            corners,
            tower_envelopes,
            collision_context,
            corner_ids,
            reused_edge,
            selected_lots,
            None,
        )
    }

    fn neighborhood_candidate_clear_with_layout(
        &self,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        tower_envelopes: &[(f32, f32)],
        collision_context: &CollisionContext,
        corner_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<&[blocks::BlockLot]>,
        spine_plan: Option<&spine_plan::SpinePlan>,
    ) -> bool {
        if !corner_ids.contains(&Some(anchor.id)) {
            return false;
        }
        let required_segments = 4 - usize::from(reused_edge.is_some());
        let spine_road_reservation = if spine_plan.is_some() { 3 } else { 0 };
        if self
            .state
            .roads
            .len()
            .saturating_add(self.state.projects.len())
            .saturating_add(required_segments)
            .saturating_add(spine_road_reservation)
            > MAX_SEGMENTS
        {
            return false;
        }
        if selected_lots.is_some() && spine_plan.is_some() {
            return false;
        }
        let mut new_incident_roads = [0usize; 4];
        for index in 0..4 {
            if reused_edge.is_some_and(|(edge_index, _)| edge_index == index) {
                continue;
            }
            new_incident_roads[index] += 1;
            new_incident_roads[(index + 1) % 4] += 1;
        }
        for (index, node_id) in corner_ids.iter().enumerate() {
            if let Some(node_id) = node_id {
                if self.ordinary_road_degree(*node_id) + new_incident_roads[index] > 4 {
                    return false;
                }
            }
        }
        let mut corner_nodes = Vec::with_capacity(4);
        for index in 0..4 {
            if let Some(node) = corner_ids[index].and_then(|node_id| self.node(node_id)) {
                if (node.x - corners[index].x).abs() > 1.0e-4
                    || (node.z - corners[index].z).abs() > 1.0e-4
                {
                    return false;
                }
                corner_nodes.push(node.clone());
            } else {
                corner_nodes.push(Node {
                    id: u32::MAX - index as u32,
                    x: corners[index].x,
                    z: corners[index].z,
                    kind: "candidate".into(),
                });
            }
        }
        let center = WorldPoint {
            x: corners.iter().map(|point| point.x).sum::<f32>() * 0.25,
            z: corners.iter().map(|point| point.z).sum::<f32>() * 0.25,
        };
        let canonical_lots = blocks::subdivide_rectangular_block(
            corners.map(|point| blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            8.0,
            11.0,
            2.2,
        );
        let spine_layout = match spine_plan {
            Some(plan) => {
                if plan.stage != spine_plan::SpineStage::Reserved
                    || plan.corners.iter().zip(corners).any(|(saved, proposed)| {
                        (saved.x - proposed.x).abs() > 1.0e-4
                            || (saved.z - proposed.z).abs() > 1.0e-4
                    })
                {
                    return false;
                }
                match plan.validate() {
                    Ok(layout) => Some(layout),
                    Err(_) => return false,
                }
            }
            None => None,
        };
        let spine_lots: Vec<blocks::BlockLot> = spine_layout
            .as_ref()
            .map(|layout| {
                layout
                    .lots
                    .iter()
                    .map(|lot| blocks::BlockLot {
                        center: lot.center,
                        frontage_edge: match lot.frontage {
                            spine_blocks::SpineFrontage::Perimeter(edge) => edge,
                            spine_blocks::SpineFrontage::Spine(_) => usize::MAX,
                        },
                        frontage_t: lot.frontage_t,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let proposed_lots = selected_lots
            .or_else(|| (!spine_lots.is_empty()).then_some(spine_lots.as_slice()))
            .unwrap_or(&canonical_lots);
        // Reserve the same deterministic lot set that will be admitted once
        // the perimeter opens. Check this before scanning world geometry.
        if proposed_lots.len() < 4
            || proposed_lots.len() > 64
            || spine_plan.is_none()
                && proposed_lots
                    .iter()
                    .any(|lot| !canonical_lots.contains(lot))
            || self.state.parcels.len().saturating_add(proposed_lots.len()) > MAX_PARCELS
            || self
                .state
                .parcel_frontages
                .len()
                .saturating_add(proposed_lots.len())
                > MAX_PARCELS
            || proposed_lots.iter().any(|lot| {
                !self.oriented_parcel_geometry_clear_with_context(
                    &geometry::OrientedRect {
                        x: lot.center.x,
                        z: lot.center.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    },
                    collision_context,
                )
            })
            || spine_plan.is_none()
                && reused_edge.is_none()
                && proposed_lots.len() != canonical_lots.len()
        {
            return false;
        }
        if let (Some(plan), Some(layout)) = (spine_plan, spine_layout.as_ref()) {
            let missing_corners = corner_ids.iter().filter(|id| id.is_none()).count();
            let future_perimeter_ids = required_segments.saturating_mul(2);
            let future_spine_ids = 6_usize.saturating_add(layout.lots.len());
            if self
                .state
                .next_id
                .checked_add(
                    missing_corners
                        .saturating_add(future_perimeter_ids)
                        .saturating_add(future_spine_ids) as u32,
                )
                .is_none()
            {
                return false;
            }
            let temporary_plan = NeighborhoodPlan {
                anchor_node: anchor.id,
                segments: (0..4)
                    .map(|index| {
                        let from = corner_ids[index].unwrap_or(u32::MAX - index as u32);
                        let to_index = (index + 1) % 4;
                        let to = corner_ids[to_index].unwrap_or(u32::MAX - to_index as u32);
                        (from, to)
                    })
                    .collect(),
                next_segment: 0,
                coordinated_lots: true,
                selected_lots: None,
                spine: Some(plan.clone()),
            };
            if !self.spine_world_geometry_safe_with_context(
                &temporary_plan,
                plan,
                layout,
                collision_context,
            ) {
                return false;
            }
        }
        if spine_plan.is_none() {
            if let Some((reused_index, _)) = reused_edge {
                for index in 0..4 {
                    if index == reused_index {
                        continue;
                    }
                    let edge_length =
                        distance(&corner_nodes[index], &corner_nodes[(index + 1) % 4]);
                    if edge_length >= 70.0
                        && proposed_lots
                            .iter()
                            .filter(|lot| lot.frontage_edge == index)
                            .count()
                            < 2
                    {
                        return false;
                    }
                }
            }
        }
        for index in 0..4 {
            let a = &corner_nodes[index];
            let b = &corner_nodes[(index + 1) % 4];
            let is_reused_edge = reused_edge.is_some_and(|(edge_index, road_id)| {
                edge_index == index
                    && self.state.roads.iter().any(|road| {
                        road.id == road_id
                            && road.stage == "open"
                            && road.connected
                            && !matches!(road.kind.as_str(), "rail" | "bridge")
                            && ((road.from == a.id && road.to == b.id)
                                || (road.from == b.id && road.to == a.id))
                    })
            });
            if reused_edge.is_some_and(|(edge_index, _)| edge_index == index) && !is_reused_edge {
                return false;
            }
            if a.x < MIN + 24.0
                || a.x > MAX - 24.0
                || a.z < MIN + 24.0
                || a.z > MAX - 24.0
                || !self.ordinary_road_clear_of_river(a.x, a.z, b.x, b.z, 4.5)
                || corridor_crosses_protected(a.x, a.z, b.x, b.z)
            {
                return false;
            }
            if !is_reused_edge {
                let heading = (b.z - a.z).atan2(b.x - a.x);
                if corner_ids[index]
                    .is_some_and(|node_id| !self.ordinary_heading_clear(node_id, heading))
                    || corner_ids[(index + 1) % 4].is_some_and(|node_id| {
                        !self.ordinary_heading_clear(node_id, (a.z - b.z).atan2(a.x - b.x))
                    })
                {
                    return false;
                }
            }
            // Existing frontage parcels are expected to sit beside a reused
            // open street. Its corridor was already admitted and is not new
            // construction, so do not reject the block because of those
            // lawful neighbors.
            if !is_reused_edge
                && !self.road_corridor_clear_of_parcels_with_context(
                    a.x,
                    a.z,
                    b.x,
                    b.z,
                    collision_context,
                )
            {
                return false;
            }
            for road in self.state.roads.iter().filter(|road| road.stage == "open") {
                let (Some(old_a), Some(old_b)) = (self.node(road.from), self.node(road.to)) else {
                    continue;
                };
                if is_reused_edge && reused_edge.is_some_and(|(_, road_id)| road.id == road_id) {
                    continue;
                }
                let shares_corner = [corner_ids[index], corner_ids[(index + 1) % 4]]
                    .into_iter()
                    .flatten()
                    .any(|node_id| road.from == node_id || road.to == node_id);
                if !shares_corner && segments_distance(a, b, old_a, old_b) <= 8.0 {
                    return false;
                }
            }
            if self.state.projects.iter().any(|project| {
                if project.kind != "road" || project.progress >= 1.0 {
                    return false;
                }
                let (Some(from), Some(to)) = (project.from, project.to) else {
                    return false;
                };
                let (Some(old_a), Some(old_b)) = (self.node(from), self.node(to)) else {
                    return false;
                };
                let shares_corner = [corner_ids[index], corner_ids[(index + 1) % 4]]
                    .into_iter()
                    .flatten()
                    .any(|node_id| from == node_id || to == node_id);
                !shares_corner && segments_distance(a, b, old_a, old_b) <= 8.0
            }) {
                return false;
            }

            let nx = -(b.z - a.z) / distance(a, b).max(1.0);
            let nz = (b.x - a.x) / distance(a, b).max(1.0);
            let midpoint_x = (a.x + b.x) * 0.5;
            let midpoint_z = (a.z + b.z) * 0.5;
            let interior_side =
                if (center.x - midpoint_x) * nx + (center.z - midpoint_z) * nz >= 0.0 {
                    1
                } else {
                    -1
                };
            if selected_lots.is_none() && spine_plan.is_none() {
                let usable_inner_sites = self
                    .frontage_candidate_sites_with_layout(
                        a,
                        b,
                        NEIGHBORHOOD_PARCEL_SIZE,
                        NEIGHBORHOOD_PARCEL_SIZE,
                        NEIGHBORHOOD_FRONTAGE_SPACING,
                    )
                    .into_iter()
                    .filter(|(x, z, _, _, side)| {
                        if *side != interior_side
                            || !self.parcel_geometry_clear(
                                *x,
                                *z,
                                NEIGHBORHOOD_PARCEL_SIZE,
                                NEIGHBORHOOD_PARCEL_SIZE,
                            )
                            || Self::overlaps_station_tower_envelopes(
                                tower_envelopes,
                                *x,
                                *z,
                                NEIGHBORHOOD_PARCEL_SIZE,
                                NEIGHBORHOOD_PARCEL_SIZE,
                            )
                        {
                            return false;
                        }
                        !(0..4).filter(|other| *other != index).any(|other| {
                            let c = &corner_nodes[other];
                            let d = &corner_nodes[(other + 1) % 4];
                            point_segment_distance(*x, *z, c.x, c.z, d.x, d.z)
                                < NEIGHBORHOOD_PARCEL_SIZE * 0.5 + 5.0
                        })
                    })
                    .count();
                let needed_inner_sites = if distance(a, b) >= 70.0 { 2 } else { 1 };
                if usable_inner_sites < needed_inner_sites {
                    return false;
                }
            }
            if tower_envelopes
                .iter()
                .any(|(x, z)| point_segment_distance(*x, *z, a.x, a.z, b.x, b.z) < 24.0)
            {
                return false;
            }
        }
        true
    }

    fn ranked_neighborhood_candidate(
        anchor_node: u32,
        corners: [WorldPoint; 4],
        corner_node_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<Vec<blocks::BlockLot>>,
        score_lots: &[blocks::BlockLot],
        stable_key: [u32; 3],
        spine: Option<spine_plan::SpinePlan>,
    ) -> NeighborhoodCandidate {
        let nominal_area_cm2 = score_lots.iter().fold(0_u64, |area, lot| {
            let lot_area = (lot.width as f64 * lot.depth as f64 * 10_000.0).round() as u64;
            area.saturating_add(lot_area)
        });
        let mut new_road_length_cm = (0..4).fold(0_u32, |length, edge| {
            if reused_edge.is_some_and(|(reused, _)| reused == edge) {
                return length;
            }
            let from = corners[edge];
            let to = corners[(edge + 1) % 4];
            let edge_length_cm =
                ((to.x - from.x).hypot(to.z - from.z) as f64 * 100.0).round() as u32;
            length.saturating_add(edge_length_cm)
        });
        let spine_road_count = if let Some(plan) = &spine {
            if let Ok(layout) = plan.validate() {
                new_road_length_cm = new_road_length_cm.saturating_add(
                    ((layout.spine[1].x - layout.spine[0].x)
                        .hypot(layout.spine[1].z - layout.spine[0].z) as f64
                        * 100.0)
                        .round() as u32,
                );
            }
            3
        } else {
            0
        };
        NeighborhoodCandidate {
            anchor_node,
            corners,
            corner_node_ids,
            reused_edge,
            score: development::NeighborhoodBenefitScore {
                usable_sites: score_lots.len().min(u16::MAX as usize) as u16,
                nominal_area_cm2,
                new_road_length_cm,
                new_road_count: (4 - usize::from(reused_edge.is_some())) as u8 + spine_road_count,
                stable_key,
            },
            selected_lots,
            spine,
        }
    }

    fn spine_candidate_lots(plan: &spine_plan::SpinePlan) -> Option<Vec<blocks::BlockLot>> {
        let layout = plan.validate().ok()?;
        Some(
            layout
                .lots
                .iter()
                .map(|lot| blocks::BlockLot {
                    center: lot.center,
                    frontage_edge: match lot.frontage {
                        spine_blocks::SpineFrontage::Perimeter(edge) => edge,
                        spine_blocks::SpineFrontage::Spine(_) => usize::MAX,
                    },
                    frontage_t: lot.frontage_t,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                })
                .collect(),
        )
    }

    fn try_create_neighborhood_plan(&mut self) -> bool {
        if self.state.neighborhood_plan.is_some()
            || self.state.pending_interior_access.is_some()
            || self.state.demand < 1.1
            || self
                .state
                .buildings
                .iter()
                .filter(|b| site_active(b))
                .count()
                < 10
            || self.state.population < 24
            || self.state.roads.len() + self.state.projects.len() + 3 > MAX_SEGMENTS
        {
            return false;
        }
        let node_count = self.state.nodes.len();
        if node_count == 0 {
            return false;
        }
        let tower_envelopes = self.available_station_tower_envelopes();
        let collision_context = self.collision_context();
        let mut connected = BTreeSet::new();
        let mut reusable_edges_by_node: BTreeMap<u32, Vec<Road>> = BTreeMap::new();
        for road in self
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
        {
            connected.insert(road.from);
            connected.insert(road.to);
            if !matches!(road.kind.as_str(), "rail" | "bridge") {
                reusable_edges_by_node
                    .entry(road.from)
                    .or_default()
                    .push(road.clone());
                reusable_edges_by_node
                    .entry(road.to)
                    .or_default()
                    .push(road.clone());
            }
        }
        let attempts = node_count.min(MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION);
        let first = self.state.neighborhood_anchor_cursor % node_count;
        let mut examined_reuse_edges = BTreeSet::new();
        let mut budget = development::NeighborhoodCandidateBudget::default();
        let mut best_candidate: Option<NeighborhoodCandidate> = None;
        for offset in 0..attempts {
            let index = (first + offset) % node_count;
            self.state.neighborhood_anchor_cursor = (index + 1) % node_count;
            let anchor = self.state.nodes[index].clone();
            if !connected.contains(&anchor.id) || self.ordinary_road_degree(anchor.id) == 0 {
                continue;
            }
            let (mut local_buildings, mut occupied_buildings) = (0, 0);
            for building in self.state.buildings.iter().filter(|b| site_active(b)) {
                if (building.x - anchor.x).hypot(building.z - anchor.z) <= 112.0 {
                    local_buildings += 1;
                    occupied_buildings += usize::from(site_active(building));
                }
            }
            if local_buildings < 8 || occupied_buildings < 4 {
                continue;
            }

            // Evaluate shared-edge and full-block candidates within separate
            // fixed budgets, then compare their usable site benefit per new
            // road length. This keeps either candidate class from hiding the
            // other through iteration order.
            if !budget.shared_limit_reached()
                && examined_reuse_edges.len() < MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION
            {
                if let Some(edges) = reusable_edges_by_node.get(&anchor.id) {
                    for road in edges {
                        if examined_reuse_edges.contains(&road.id) {
                            continue;
                        }
                        if examined_reuse_edges.len() >= MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION
                        {
                            break;
                        }
                        examined_reuse_edges.insert(road.id);
                        let (Some(start), Some(end)) =
                            (self.node(road.from).cloned(), self.node(road.to).cloned())
                        else {
                            continue;
                        };
                        let edge_length = distance(&start, &end);
                        let extrusion = if (70.0..=90.0).contains(&edge_length) {
                            NEIGHBORHOOD_BLOCK_WIDTH
                        } else if (42.0..=58.0).contains(&edge_length) {
                            NEIGHBORHOOD_BLOCK_LENGTH
                        } else {
                            continue;
                        };
                        let perpendicular_x = -(end.z - start.z) / edge_length;
                        let perpendicular_z = (end.x - start.x) / edge_length;
                        let side_seed = stateless_tower_site_hash(
                            self.state.seed ^ road.id,
                            (start.x + end.x) * 0.5,
                            (start.z + end.z) * 0.5,
                        );
                        let sides = if side_seed & 1 == 0 {
                            [1.0_f32, -1.0]
                        } else {
                            [-1.0_f32, 1.0]
                        };
                        for (side_index, side) in sides.into_iter().enumerate() {
                            if !budget.claim_shared() {
                                break;
                            }
                            let offset_x = perpendicular_x * extrusion * side;
                            let offset_z = perpendicular_z * extrusion * side;
                            let corners = [
                                WorldPoint {
                                    x: start.x,
                                    z: start.z,
                                },
                                WorldPoint { x: end.x, z: end.z },
                                WorldPoint {
                                    x: end.x + offset_x,
                                    z: end.z + offset_z,
                                },
                                WorldPoint {
                                    x: start.x + offset_x,
                                    z: start.z + offset_z,
                                },
                            ];
                            let selected_lots =
                                self.available_coordinated_block_lots(&corners, &collision_context);
                            let corner_ids = [Some(road.from), Some(road.to), None, None];
                            if self.neighborhood_candidate_clear_with_edge(
                                &anchor,
                                &corners,
                                &tower_envelopes,
                                &collision_context,
                                corner_ids,
                                Some((0, road.id)),
                                Some(&selected_lots),
                            ) {
                                let candidate = Self::ranked_neighborhood_candidate(
                                    road.from,
                                    corners,
                                    [Some(road.from), Some(road.to), None, None],
                                    Some((0, road.id)),
                                    Some(selected_lots.clone()),
                                    &selected_lots,
                                    [
                                        anchor.id,
                                        0,
                                        road.id.saturating_mul(2).saturating_add(side_index as u32),
                                    ],
                                    None,
                                );
                                if best_candidate.as_ref().map_or(true, |best| {
                                    neighborhood_candidate_is_better(candidate.score, best.score)
                                }) {
                                    best_candidate = Some(candidate);
                                }
                            }

                            // A deep square spine option competes with the
                            // ordinary block on this shared edge using the
                            // same benefit-per-centerline-metre score.
                            if (70.0..=90.0).contains(&edge_length) && budget.claim_spine() {
                                let spine_offset_x =
                                    perpendicular_x * NEIGHBORHOOD_BLOCK_LENGTH * side;
                                let spine_offset_z =
                                    perpendicular_z * NEIGHBORHOOD_BLOCK_LENGTH * side;
                                let spine_corners = [
                                    WorldPoint {
                                        x: start.x,
                                        z: start.z,
                                    },
                                    WorldPoint { x: end.x, z: end.z },
                                    WorldPoint {
                                        x: end.x + spine_offset_x,
                                        z: end.z + spine_offset_z,
                                    },
                                    WorldPoint {
                                        x: start.x + spine_offset_x,
                                        z: start.z + spine_offset_z,
                                    },
                                ];
                                let spine = spine_plan::SpinePlan {
                                    corners: spine_corners.map(|point| blocks::BlockPoint {
                                        x: point.x,
                                        z: point.z,
                                    }),
                                    recipe_version: spine_plan::RECIPE_VERSION,
                                    stage: spine_plan::SpineStage::Reserved,
                                };
                                if self.neighborhood_candidate_clear_with_layout(
                                    &anchor,
                                    &spine_corners,
                                    &tower_envelopes,
                                    &collision_context,
                                    corner_ids,
                                    Some((0, road.id)),
                                    None,
                                    Some(&spine),
                                ) {
                                    if let Some(score_lots) = Self::spine_candidate_lots(&spine) {
                                        let candidate = Self::ranked_neighborhood_candidate(
                                            road.from,
                                            spine_corners,
                                            corner_ids,
                                            Some((0, road.id)),
                                            None,
                                            &score_lots,
                                            [
                                                anchor.id,
                                                2,
                                                road.id
                                                    .saturating_mul(2)
                                                    .saturating_add(side_index as u32),
                                            ],
                                            Some(spine),
                                        );
                                        if best_candidate.as_ref().map_or(true, |best| {
                                            neighborhood_candidate_is_better(
                                                candidate.score,
                                                best.score,
                                            )
                                        }) {
                                            best_candidate = Some(candidate);
                                        }
                                    }
                                }
                            }
                        }
                        if budget.shared_limit_reached() {
                            break;
                        }
                    }
                }
            }

            // Four new streets add two incident roads at this anchor. Keep
            // the older junction-degree constraint for that fallback shape;
            // a reused edge above contributes only one new incident road.
            if self.ordinary_road_degree(anchor.id) > 2 {
                if budget.shared_limit_reached() {
                    break;
                }
                continue;
            }

            let start = stateless_tower_site_hash(self.state.seed, anchor.x, anchor.z) as usize % 8;
            for rotation in 0..8 {
                if !budget.claim_full() {
                    break;
                }
                let option = (start + rotation) % 8;
                let (ux, uz, vx, vz) = match option {
                    0 => (1.0, 0.0, 0.0, 1.0),
                    1 => (1.0, 0.0, 0.0, -1.0),
                    2 => (-1.0, 0.0, 0.0, 1.0),
                    3 => (-1.0, 0.0, 0.0, -1.0),
                    4 => (0.0, 1.0, 1.0, 0.0),
                    5 => (0.0, 1.0, -1.0, 0.0),
                    6 => (0.0, -1.0, 1.0, 0.0),
                    _ => (0.0, -1.0, -1.0, 0.0),
                };
                let corners = [
                    WorldPoint {
                        x: anchor.x,
                        z: anchor.z,
                    },
                    WorldPoint {
                        x: anchor.x + ux * NEIGHBORHOOD_BLOCK_LENGTH,
                        z: anchor.z + uz * NEIGHBORHOOD_BLOCK_LENGTH,
                    },
                    WorldPoint {
                        x: anchor.x
                            + ux * NEIGHBORHOOD_BLOCK_LENGTH
                            + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                        z: anchor.z
                            + uz * NEIGHBORHOOD_BLOCK_LENGTH
                            + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                    },
                    WorldPoint {
                        x: anchor.x + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                        z: anchor.z + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                    },
                ];
                if self.neighborhood_candidate_clear(
                    &anchor,
                    &corners,
                    &tower_envelopes,
                    &collision_context,
                ) {
                    let score_lots = blocks::subdivide_rectangular_block(
                        corners.map(|point| blocks::BlockPoint {
                            x: point.x,
                            z: point.z,
                        }),
                        8.0,
                        11.0,
                        2.2,
                    );
                    let candidate = Self::ranked_neighborhood_candidate(
                        anchor.id,
                        corners,
                        [Some(anchor.id), None, None, None],
                        None,
                        None,
                        &score_lots,
                        [anchor.id, 1, option as u32],
                        None,
                    );
                    if best_candidate.as_ref().map_or(true, |best| {
                        neighborhood_candidate_is_better(candidate.score, best.score)
                    }) {
                        best_candidate = Some(candidate);
                    }
                }

                if budget.claim_spine() {
                    let spine_corners = [
                        WorldPoint {
                            x: anchor.x,
                            z: anchor.z,
                        },
                        WorldPoint {
                            x: anchor.x + ux * NEIGHBORHOOD_BLOCK_LENGTH,
                            z: anchor.z + uz * NEIGHBORHOOD_BLOCK_LENGTH,
                        },
                        WorldPoint {
                            x: anchor.x
                                + ux * NEIGHBORHOOD_BLOCK_LENGTH
                                + vx * NEIGHBORHOOD_BLOCK_LENGTH,
                            z: anchor.z
                                + uz * NEIGHBORHOOD_BLOCK_LENGTH
                                + vz * NEIGHBORHOOD_BLOCK_LENGTH,
                        },
                        WorldPoint {
                            x: anchor.x + vx * NEIGHBORHOOD_BLOCK_LENGTH,
                            z: anchor.z + vz * NEIGHBORHOOD_BLOCK_LENGTH,
                        },
                    ];
                    let spine = spine_plan::SpinePlan {
                        corners: spine_corners.map(|point| blocks::BlockPoint {
                            x: point.x,
                            z: point.z,
                        }),
                        recipe_version: spine_plan::RECIPE_VERSION,
                        stage: spine_plan::SpineStage::Reserved,
                    };
                    let spine_corner_ids = [Some(anchor.id), None, None, None];
                    if self.neighborhood_candidate_clear_with_layout(
                        &anchor,
                        &spine_corners,
                        &tower_envelopes,
                        &collision_context,
                        spine_corner_ids,
                        None,
                        None,
                        Some(&spine),
                    ) {
                        if let Some(score_lots) = Self::spine_candidate_lots(&spine) {
                            let candidate = Self::ranked_neighborhood_candidate(
                                anchor.id,
                                spine_corners,
                                spine_corner_ids,
                                None,
                                None,
                                &score_lots,
                                [anchor.id, 3, option as u32],
                                Some(spine),
                            );
                            if best_candidate.as_ref().map_or(true, |best| {
                                neighborhood_candidate_is_better(candidate.score, best.score)
                            }) {
                                best_candidate = Some(candidate);
                            }
                        }
                    }
                }
            }
            // Stop at the anchor where either class exhausts its allowance.
            // The cursor already points to the next anchor, so candidates
            // beyond that point are reached on the next planning evaluation.
            if budget.any_limit_reached() {
                break;
            }
        }
        let Some(candidate) = best_candidate else {
            return false;
        };
        let mut ids = candidate.corner_node_ids;
        for index in 0..4 {
            if ids[index].is_none() {
                ids[index] = Some(self.add_node(
                    candidate.corners[index].x,
                    candidate.corners[index].z,
                    "junction",
                ));
            }
        }
        let ids = ids.map(|id| id.expect("validated neighborhood corner has a node"));
        let segments = (0..4)
            .map(|index| (ids[index], ids[(index + 1) % 4]))
            .collect();
        if let Some((_, road_id)) = candidate.reused_edge {
            self.state.neighborhood_road_ids.insert(road_id);
        }
        self.state.neighborhood_plan = Some(NeighborhoodPlan {
            anchor_node: candidate.anchor_node,
            segments,
            next_segment: 0,
            coordinated_lots: true,
            selected_lots: candidate.selected_lots,
            spine: candidate.spine,
        });
        self.mark_access_geometry_changed();
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }

    fn plan_next_neighborhood_segment(&mut self) -> bool {
        loop {
            let Some(plan) = self.state.neighborhood_plan.as_ref() else {
                return false;
            };
            if plan.next_segment >= plan.segments.len() {
                let changed = self.advance_neighborhood_spine();
                if changed {
                    self.mark_access_geometry_changed();
                }
                return changed;
            }
            let (from, to) = plan.segments[plan.next_segment];
            if let Some(road_id) = self
                .state
                .roads
                .iter()
                .find(|road| {
                    road.stage == "open"
                        && ((road.from == from && road.to == to)
                            || (road.from == to && road.to == from))
                })
                .map(|road| road.id)
            {
                self.remember_neighborhood_plan_road(road_id, from, to);
                self.state.neighborhood_plan.as_mut().unwrap().next_segment += 1;
                self.retire_completed_neighborhood_plan();
                continue;
            }
            if !self.node_connected(from)
                || self.state.roads.len() + self.state.projects.len() >= MAX_SEGMENTS
                || !self.project_admission_available()
            {
                return true;
            }
            let id = self.id();
            let duration = 3.0 + random_unit(&mut self.state.rng_growth) * 4.0;
            let started_tick = self.state.tick;
            self.record_project(Project {
                id,
                kind: "road".into(),
                from: Some(from),
                to: Some(to),
                parcel_id: None,
                stage: "survey".into(),
                progress: 0.0,
                reason: "A reserved local street opens a compact neighborhood block".into(),
                duration,
                started_tick,
            });
            self.state.neighborhood_plan.as_mut().unwrap().next_segment += 1;
            self.state.revision = self.state.revision.saturating_add(1);
            return true;
        }
    }

    /// Reserve a real local-street chord inside a developed neighborhood
    /// before extending the outer edge. A chord joins two existing connected
    /// junctions, so its construction cannot introduce a disconnected segment
    /// or a speculative endpoint inside a future parcel.
    fn try_plan_local_chord(
        &mut self,
        open_buildings: &[Building],
        collision_context: &CollisionContext,
    ) -> bool {
        if !self.project_admission_available()
            || open_buildings.len() < 12
            || self.state.demand <= 0.8
            || self
                .state
                .projects
                .iter()
                .any(|project| project.kind == "road" && project.progress < 1.0)
        {
            return false;
        }

        let connected: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.connected
                    && road.stage == "open"
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .flat_map(|road| [road.from, road.to])
            .collect();
        let mut origins: Vec<(Node, f32)> = self
            .state
            .nodes
            .iter()
            .filter(|node| connected.contains(&node.id))
            .filter_map(|node| {
                let nearest = open_buildings
                    .iter()
                    .map(|building| (building.x - node.x).hypot(building.z - node.z))
                    .min_by(f32::total_cmp)?;
                (nearest <= 110.0).then_some((node.clone(), nearest))
            })
            .collect();
        if origins.len() < 2 {
            return false;
        }
        let center = open_buildings.iter().fold((0.0, 0.0), |(x, z), building| {
            (
                x + building.x / open_buildings.len() as f32,
                z + building.z / open_buildings.len() as f32,
            )
        });
        origins.sort_by(|(a, _), (b, _)| {
            let a_radius = (a.x - center.0).hypot(a.z - center.1);
            let b_radius = (b.x - center.0).hypot(b.z - center.1);
            b_radius.total_cmp(&a_radius).then_with(|| a.id.cmp(&b.id))
        });

        let first = self.state.road_origin_cursor % origins.len();
        let count = origins.len().min(MAX_ROAD_ORIGINS_PER_EVALUATION);
        for offset in 0..count {
            let origin_index = (first + offset) % origins.len();
            let start = origins[origin_index].0.clone();
            self.state.road_origin_cursor = (origin_index + 1) % origins.len();
            if start.id >= 7 && self.ordinary_road_degree(start.id) >= 4 {
                continue;
            }

            let mut targets: Vec<(Node, f32)> = self
                .state
                .nodes
                .iter()
                .filter(|node| node.id > start.id && connected.contains(&node.id))
                .filter_map(|node| {
                    let length = distance(&start, node);
                    (45.0..=90.0)
                        .contains(&length)
                        .then_some((node.clone(), length))
                })
                .collect();
            targets.sort_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| a.id.cmp(&b.id)));

            for (target, _) in targets {
                if target.id >= 7 && self.ordinary_road_degree(target.id) >= 4 {
                    continue;
                }
                if self.state.roads.iter().any(|road| {
                    (road.from == start.id && road.to == target.id)
                        || (road.from == target.id && road.to == start.id)
                }) || self.state.projects.iter().any(|project| {
                    project.kind == "road"
                        && project.progress < 1.0
                        && ((project.from == Some(start.id) && project.to == Some(target.id))
                            || (project.from == Some(target.id) && project.to == Some(start.id)))
                }) {
                    continue;
                }
                let forward = (target.z - start.z).atan2(target.x - start.x);
                let reverse = (start.z - target.z).atan2(start.x - target.x);
                if !self.ordinary_heading_clear(start.id, forward)
                    || !self.ordinary_heading_clear(target.id, reverse)
                    || in_protected_land(target.x, target.z)
                    || corridor_crosses_protected(start.x, start.z, target.x, target.z)
                    || !self.ordinary_road_clear_of_river(start.x, start.z, target.x, target.z, 4.5)
                {
                    continue;
                }

                let midpoint = WorldPoint {
                    x: (start.x + target.x) * 0.5,
                    z: (start.z + target.z) * 0.5,
                };
                let local_buildings = open_buildings.iter().filter(|building| {
                    (building.x - midpoint.x).hypot(building.z - midpoint.z) <= 100.0
                });
                let mut local_open = 0usize;
                let mut local_occupied = 0usize;
                for building in local_buildings {
                    local_open += 1;
                    local_occupied += usize::from(site_active(building));
                }
                if local_open < 8 || local_occupied < 4 {
                    continue;
                }

                // Leave a clear corridor for the full street width and its
                // construction tolerance. Existing parcels near either
                // junction are safe only when their center lies outside this
                // reserved strip.
                let corridor_clear = self.road_corridor_clear_of_parcels_with_context(
                    start.x,
                    start.z,
                    target.x,
                    target.z,
                    collision_context,
                );
                let road_clear = self
                    .state
                    .roads
                    .iter()
                    .filter(|road| road.stage == "open")
                    .all(|road| {
                        if road.from == start.id
                            || road.to == start.id
                            || road.from == target.id
                            || road.to == target.id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                            return true;
                        };
                        segments_distance(&start, &target, a, b) > 8.0
                    });
                let projects_clear = self
                    .state
                    .projects
                    .iter()
                    .filter(|project| project.kind == "road" && project.progress < 1.0)
                    .all(|project| {
                        let (Some(from), Some(to)) = (project.from, project.to) else {
                            return true;
                        };
                        if from == start.id
                            || to == start.id
                            || from == target.id
                            || to == target.id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                            return true;
                        };
                        segments_distance(&start, &target, a, b) > 8.0
                    });
                if !corridor_clear
                    || !road_clear
                    || !projects_clear
                    || !self.neighborhood_corridor_clear_for_road(&start, &target, start.id)
                {
                    continue;
                }

                // Admission also requires usable lots on the chosen, actual
                // corridor. This uses the same clearance checks as parcels
                // created when construction completes.
                if self.viable_frontage_benefit_count(&start, &target, 2) < 2 {
                    continue;
                }

                let id = self.id();
                let duration = 3.0 + random_unit(&mut self.state.rng_growth) * 4.0;
                self.record_project(Project {
                    id,
                    kind: "road".into(),
                    from: Some(start.id),
                    to: Some(target.id),
                    parcel_id: None,
                    stage: "survey".into(),
                    progress: 0.0,
                    reason:
                        "A local connector closes a developed block and opens new street frontage"
                            .into(),
                    duration,
                    started_tick: self.state.tick,
                });
                self.state.revision = self.state.revision.saturating_add(1);
                return true;
            }
        }
        false
    }

    fn try_plan_interior_access(&mut self) -> bool {
        let mut scan = self.propose_interior_access();
        self.state.interior_access_cursor = scan.next_cursor;
        scan.proposals
            .sort_by(|left, right| right.score.compare(left.score));
        for proposal in scan.proposals {
            if self.admit_interior_access(
                proposal.host_id,
                proposal.host_t,
                WorldPoint {
                    x: proposal.end_x,
                    z: proposal.end_z,
                },
                proposal.selected_lots,
            ) {
                return true;
            }
        }
        false
    }

    #[cfg(test)]
    fn try_plan_access_at_road_opening(&mut self, road_id: u32) -> bool {
        if !self.road_opening_access_experiment
            || self.state.neighborhood_plan.is_some()
            || self.state.pending_interior_access.is_some()
            || self.state.neighborhood_road_ids.contains(&road_id)
        {
            return false;
        }
        let Some(host) = self.state.roads.iter().find(|road| road.id == road_id) else {
            return false;
        };
        if host.stage != "open"
            || !host.connected
            || !matches!(host.kind.as_str(), "local" | "arterial")
        {
            return false;
        }
        let established_roads = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
            })
            .count();
        let active_buildings = self
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .count();
        if established_roads < 40 || active_buildings < 8 {
            return false;
        }
        let open_buildings: Vec<_> = self
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .cloned()
            .collect();
        if !self
            .eligible_bridgehead_headings(&open_buildings)
            .is_empty()
        {
            return false;
        }

        self.road_opening_access_attempts = self.road_opening_access_attempts.saturating_add(1);
        let mut proposals = if self.face_conditioned_access_experiment {
            let Some(proposals) = self.face_conditioned_access_proposals(road_id) else {
                return false;
            };
            proposals
        } else {
            self.propose_interior_access_for_host(road_id).proposals
        };
        proposals.sort_by(|left, right| right.score.compare(left.score));
        for proposal in proposals {
            let proposed_host_id = proposal.host_id;
            let proposed_lot_count = proposal.selected_lots.len();
            if self.admit_interior_access(
                proposal.host_id,
                proposal.host_t,
                WorldPoint {
                    x: proposal.end_x,
                    z: proposal.end_z,
                },
                proposal.selected_lots,
            ) {
                if self.face_conditioned_access_experiment
                    && std::env::var_os("FACE_ACCESS_DEBUG").is_some()
                {
                    eprintln!(
                        "face access admitted trigger={road_id} host={proposed_host_id} lots={proposed_lot_count}"
                    );
                }
                self.road_opening_access_admissions =
                    self.road_opening_access_admissions.saturating_add(1);
                self.road_opening_access_branch = self
                    .state
                    .pending_interior_access
                    .as_ref()
                    .map(|plan| plan.branch_road_id);
                return true;
            }
        }
        false
    }

    /// Enumerate bounded interior-access proposals without changing simulation
    /// state. The caller commits the returned cursor before attempting admission.
    fn propose_interior_access(&self) -> interior_access::AccessScan {
        self.propose_interior_access_from(None, self.state.interior_access_cursor)
    }

    fn propose_interior_access_for_host(&self, host_id: u32) -> interior_access::AccessScan {
        self.propose_interior_access_from(Some(host_id), 0)
    }

    fn propose_interior_access_from(
        &self,
        only_host: Option<u32>,
        cursor: usize,
    ) -> interior_access::AccessScan {
        self.propose_interior_access_from_candidates(only_host, cursor, None)
    }

    fn propose_interior_access_from_candidates(
        &self,
        only_host: Option<u32>,
        cursor: usize,
        explicit_candidates: Option<&[interior_access::AccessCandidate]>,
    ) -> interior_access::AccessScan {
        self.propose_interior_access_scan(only_host, cursor, explicit_candidates, None)
    }

    #[cfg(test)]
    fn propose_interior_access_from_geometry_candidates(
        &self,
        cursor: usize,
        explicit_candidates: &[interior_access::AccessGeometryCandidate],
    ) -> interior_access::AccessScan {
        self.propose_interior_access_scan(None, cursor, None, Some(explicit_candidates))
    }

    fn propose_interior_access_scan(
        &self,
        only_host: Option<u32>,
        cursor: usize,
        explicit_candidates: Option<&[interior_access::AccessCandidate]>,
        geometry_candidates: Option<&[interior_access::AccessGeometryCandidate]>,
    ) -> interior_access::AccessScan {
        use interior_access::{AccessProposal, AccessScan, CandidateScore, InteriorAccessLot};

        let unchanged = || interior_access::AccessScan {
            proposals: Vec::new(),
            next_cursor: cursor,
            #[cfg(test)]
            rejection_counts: [0; 8],
        };

        if self.state.pending_interior_access.is_some()
            || self.state.neighborhood_plan.is_some()
            || self.state.demand <= 0.8
            || !self.project_admission_available()
            || self
                .state
                .roads
                .len()
                .saturating_add(self.state.projects.len())
                .saturating_add(3)
                > MAX_SEGMENTS
            || self.state.parcels.len().saturating_add(2) > MAX_PARCELS
            || self.state.next_id.checked_add(5).is_none()
        {
            return unchanged();
        }
        if self
            .state
            .projects
            .iter()
            .any(|project| project.kind == "road" && project.progress < 1.0)
        {
            return unchanged();
        }

        let explicit_host_ids = explicit_candidates
            .map(|candidates| candidates.iter().map(|candidate| candidate.host_id))
            .into_iter()
            .flatten()
            .chain(
                geometry_candidates
                    .into_iter()
                    .flatten()
                    .map(|candidate| candidate.host_id),
            )
            .collect::<BTreeSet<_>>();
        let explicit_host_ids = (!explicit_host_ids.is_empty()).then_some(explicit_host_ids);
        let mut hosts: Vec<&Road> = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
                    && only_host.is_none_or(|host_id| road.id == host_id)
                    && explicit_host_ids
                        .as_ref()
                        .is_none_or(|host_ids| host_ids.contains(&road.id))
            })
            .collect();
        hosts.sort_by_key(|road| road.id);
        if hosts.is_empty() {
            return unchanged();
        }
        let candidate_count = geometry_candidates
            .map(|candidates| candidates.len())
            .or_else(|| explicit_candidates.map(|candidates| candidates.len()))
            .unwrap_or_else(|| {
                hosts
                    .len()
                    .saturating_mul(interior_access::PROPOSALS_PER_HOST)
            });
        if candidate_count == 0 {
            return unchanged();
        }
        let start_cursor = cursor % candidate_count;
        let inspect_count = candidate_count.min(interior_access::MAX_PROPOSALS_PER_EVALUATION);
        let context = self.collision_context();
        let mut candidates: Vec<AccessProposal> = Vec::new();
        #[cfg(test)]
        let mut rejection_counts = [0usize; 8];
        #[cfg(test)]
        macro_rules! reject_candidate {
            ($bucket:expr) => {{
                rejection_counts[$bucket] += 1;
                continue;
            }};
        }
        let host_indexes =
            (explicit_candidates.is_some() || geometry_candidates.is_some()).then(|| {
                hosts
                    .iter()
                    .enumerate()
                    .map(|(index, road)| (road.id, index))
                    .collect::<BTreeMap<_, _>>()
            });
        for offset in 0..inspect_count {
            let proposal_index = (start_cursor + offset) % candidate_count;
            let (host_index, host_t, fraction_index, side, length_index, explicit_end) =
                if let Some(geometry_candidates) = geometry_candidates {
                    let candidate = geometry_candidates[proposal_index];
                    let Some(host_index) = host_indexes
                        .as_ref()
                        .and_then(|indexes| indexes.get(&candidate.host_id))
                        .copied()
                    else {
                        #[cfg(test)]
                        reject_candidate!(0);
                        #[cfg(not(test))]
                        continue;
                    };
                    (
                        host_index,
                        candidate.host_t,
                        candidate.tie_index as usize,
                        1_i8,
                        0_usize,
                        Some(WorldPoint {
                            x: candidate.end_x,
                            z: candidate.end_z,
                        }),
                    )
                } else if let Some(explicit_candidates) = explicit_candidates {
                    let candidate = explicit_candidates[proposal_index];
                    let Some(host_index) = host_indexes
                        .as_ref()
                        .and_then(|indexes| indexes.get(&candidate.host_id))
                        .copied()
                    else {
                        #[cfg(test)]
                        reject_candidate!(0);
                        #[cfg(not(test))]
                        continue;
                    };
                    (
                        host_index,
                        interior_access::FRACTIONS
                            .get(candidate.fraction_index as usize)
                            .copied()
                            .unwrap_or(f32::NAN),
                        candidate.fraction_index as usize,
                        candidate.side,
                        candidate.length_index as usize,
                        None,
                    )
                } else {
                    let host_index = proposal_index / interior_access::PROPOSALS_PER_HOST;
                    let local = proposal_index % interior_access::PROPOSALS_PER_HOST;
                    let length_index = local % interior_access::LENGTHS.len();
                    let side_index =
                        (local / interior_access::LENGTHS.len()) % interior_access::SIDES.len();
                    let fraction_index =
                        local / (interior_access::LENGTHS.len() * interior_access::SIDES.len());
                    (
                        host_index,
                        interior_access::FRACTIONS[fraction_index],
                        fraction_index,
                        interior_access::SIDES[side_index],
                        length_index,
                        None,
                    )
                };
            let length_from_grid = interior_access::LENGTHS.get(length_index).copied();
            if !host_t.is_finite()
                || (explicit_end.is_none() && length_from_grid.is_none())
                || (explicit_end.is_none() && !interior_access::SIDES.contains(&side))
            {
                #[cfg(test)]
                reject_candidate!(0);
                #[cfg(not(test))]
                continue;
            }
            let side_index = usize::from(side == 1);
            let host = hosts[host_index];
            let (Some(host_a), Some(host_b)) = (self.node(host.from), self.node(host.to)) else {
                continue;
            };
            let dx = host_b.x - host_a.x;
            let dz = host_b.z - host_a.z;
            let host_length = dx.hypot(dz);
            if !host_length.is_finite()
                || host_length * host_t < 10.0
                || host_length * (1.0 - host_t) < 10.0
            {
                continue;
            }
            let ux = dx / host_length;
            let uz = dz / host_length;
            let nx = -uz * side as f32;
            let nz = ux * side as f32;
            let start = WorldPoint {
                x: host_a.x + dx * host_t,
                z: host_a.z + dz * host_t,
            };
            let end = explicit_end.unwrap_or_else(|| WorldPoint {
                x: start.x + nx * length_from_grid.unwrap(),
                z: start.z + nz * length_from_grid.unwrap(),
            });
            let length = if explicit_end.is_some() {
                (end.x - start.x).hypot(end.z - start.z)
            } else {
                length_from_grid.unwrap()
            };
            if !length.is_finite() || length < 24.0 - 0.01 || length > 60.0 + 0.01 {
                #[cfg(test)]
                reject_candidate!(0);
                #[cfg(not(test))]
                continue;
            }
            if [start.x, start.z, end.x, end.z]
                .iter()
                .any(|value| !value.is_finite() || *value < MIN + 24.0 || *value > MAX - 24.0)
                || !self.ordinary_road_clear_of_river(start.x, start.z, end.x, end.z, 4.5)
                || corridor_crosses_protected(start.x, start.z, end.x, end.z)
                || !self.road_corridor_clear_of_parcels_with_context(
                    start.x, start.z, end.x, end.z, &context,
                )
            {
                #[cfg(test)]
                reject_candidate!(1);
                #[cfg(not(test))]
                continue;
            }
            let split_x = start.x;
            let split_z = start.z;
            if self.state.nodes.iter().any(|node| {
                node.id != host.from
                    && node.id != host.to
                    && (node.x - split_x).hypot(node.z - split_z) < 1.0
            }) || self
                .state
                .roads
                .iter()
                .filter(|road| road.id != host.id)
                .any(|road| {
                    let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                        return true;
                    };
                    point_segment_distance(split_x, split_z, a.x, a.z, b.x, b.z) < 0.5
                })
            {
                #[cfg(test)]
                reject_candidate!(2);
                #[cfg(not(test))]
                continue;
            }
            let endpoint_too_close = self
                .state
                .nodes
                .iter()
                .any(|node| (node.x - end.x).hypot(node.z - end.z) < 18.0);
            if endpoint_too_close {
                #[cfg(test)]
                reject_candidate!(3);
                #[cfg(not(test))]
                continue;
            }
            let road_clear = self
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open")
                .all(|road| {
                    if road.id == host.id {
                        return true;
                    }
                    let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                        return false;
                    };
                    geometry::segments_distance_coords(
                        start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z,
                    ) >= 8.0
                });
            let projects_clear = self
                .state
                .projects
                .iter()
                .filter(|project| project.kind == "road" && project.progress < 1.0)
                .all(|project| {
                    let Some((from, to)) = project.from.zip(project.to) else {
                        return true;
                    };
                    let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                        return false;
                    };
                    geometry::segments_distance_coords(
                        start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z,
                    ) >= 8.0
                });
            if !road_clear || !projects_clear {
                #[cfg(test)]
                reject_candidate!(4);
                #[cfg(not(test))]
                continue;
            }

            let mut proposed_context = context.clone();
            proposed_context.open_roads.push(CollisionSegment {
                ax: start.x,
                az: start.z,
                bx: end.x,
                bz: end.z,
            });
            let intervals = frontage::subdivide_frontage(
                length,
                &[],
                FRONTAGE_ENDPOINT_CLEARANCE,
                8.0,
                14.0,
                2.2,
                MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
            );
            // Grid candidates travel along the host-normal `nx/nz` already.
            // Explicit geometry candidates can point at any angle, so their
            // frontage must use the actual branch tangent to place lots
            // perpendicular to the constructed road.
            let (lot_tangent_x, lot_tangent_z) = if explicit_end.is_some() {
                ((end.x - start.x) / length, (end.z - start.z) / length)
            } else {
                (nx, nz)
            };
            let mut lots = Vec::new();
            for lot_side in [-1_i8, 1] {
                for interval in &intervals {
                    let along_meters = (interval.start + interval.end) * 0.5;
                    let x = start.x + (end.x - start.x) * (along_meters / length)
                        - lot_tangent_z * lot_side as f32 * 15.0;
                    let z = start.z
                        + (end.z - start.z) * (along_meters / length)
                        + lot_tangent_x * lot_side as f32 * 15.0;
                    let lot = InteriorAccessLot {
                        x,
                        z,
                        width: interval.end - interval.start,
                        depth: 12.0,
                        rotation: (lot_side as f32 * (end.z - start.z))
                            .atan2(-lot_side as f32 * (end.x - start.x)),
                        along: along_meters / length,
                        side: lot_side,
                    };
                    let rect = geometry::OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    };
                    if self.progressive_frontage_geometry_clear(&rect, &proposed_context)
                        && lots.len() < interior_access::MAX_SELECTED_LOTS
                    {
                        lots.push(lot);
                    }
                }
            }
            if lots.len() < 2
                || self.state.parcels.len().saturating_add(lots.len()) > MAX_PARCELS
                || self.state.parcel_frontages.len().saturating_add(lots.len()) > MAX_PARCELS
            {
                #[cfg(test)]
                reject_candidate!(5);
                #[cfg(not(test))]
                continue;
            }
            let area_cm2 = lots
                .iter()
                .map(|lot| (lot.width * lot.depth * 10_000.0).round() as u32)
                .sum::<u32>();
            let score = CandidateScore {
                site_area: area_cm2,
                road_length_mm: (length * 1_000.0).round() as u32,
                host_id: host.id,
                fraction_index: fraction_index as u8,
                side_index: side_index as u8,
                length_index: length_index as u8,
            };
            candidates.push(AccessProposal {
                score,
                host_id: host.id,
                host_t,
                end_x: end.x,
                end_z: end.z,
                selected_lots: lots,
            });
        }
        AccessScan {
            proposals: candidates,
            next_cursor: (start_cursor + inspect_count) % candidate_count,
            #[cfg(test)]
            rejection_counts,
        }
    }

    /// Commit one geometry-checked proposal through the existing atomic split
    /// preflight. Keeping admission separate lets planners compare proposals
    /// without allocating IDs or changing the network during scoring.
    fn admit_interior_access(
        &mut self,
        host_id: u32,
        host_t: f32,
        end: WorldPoint,
        selected_lots: Vec<interior_access::InteriorAccessLot>,
    ) -> bool {
        // Each proposal has been geometry-checked; split preflight can still
        // reject actor route-cap or frontage remaps. Try the next ranked
        // candidate because split rejection is guaranteed mutation-free.
        if self
            .state
            .roads
            .len()
            .saturating_add(self.state.projects.len())
            .saturating_add(3)
            > MAX_SEGMENTS
            || self.state.parcels.len().saturating_add(selected_lots.len()) > MAX_PARCELS
            || self
                .state
                .parcel_frontages
                .len()
                .saturating_add(selected_lots.len())
                > MAX_PARCELS
            || self
                .state
                .next_id
                .checked_add(5 + selected_lots.len() as u32)
                .is_none()
            || !self.project_admission_available()
        {
            return false;
        }
        let split = match self.try_split_open_road(host_id, host_t) {
            Ok(split) => split,
            Err(_) => return false,
        };
        self.recompute_connectivity();
        let endpoint_id = self.add_node(end.x, end.z, "junction");
        let branch_road_id =
            self.add_road(split.split_node_id, endpoint_id, "local", "survey", 0.0);
        let project_id = self.id();
        let project = Project {
            id: project_id,
            kind: "road".into(),
            from: Some(split.split_node_id),
            to: Some(endpoint_id),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.0,
            reason: "A neighborhood access street is being built from an established corridor"
                .into(),
            duration: (3.0 + ((host_id ^ self.state.seed) % 400) as f32 / 100.0).clamp(3.0, 7.0),
            started_tick: self.state.tick,
        };
        self.record_project(project);
        self.state.pending_interior_access = Some(interior_access::InteriorAccessPlan {
            host_road_id: host_id,
            host_second_half_road_id: split.new_road_id,
            host_t,
            split_node_id: split.split_node_id,
            branch_endpoint_id: endpoint_id,
            branch_road_id,
            project_id,
            selected_lots,
        });
        self.mark_access_geometry_changed();
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }

    fn eligible_bridgehead_headings(&self, open_buildings: &[Building]) -> BTreeMap<u32, f32> {
        let mut headings = BTreeMap::new();
        for bridge in self
            .state
            .roads
            .iter()
            .filter(|road| road.kind == "bridge" && road.stage == "open" && road.connected)
        {
            for (head_id, opposite_id) in [(bridge.from, bridge.to), (bridge.to, bridge.from)] {
                let Some(head) = self.node(head_id) else {
                    continue;
                };
                let Some(opposite) = self.node(opposite_id) else {
                    continue;
                };
                let ordinary_open_or_planned = self.state.roads.iter().any(|road| {
                    (road.from == head_id || road.to == head_id)
                        && road.stage == "open"
                        && road.connected
                        && !matches!(road.kind.as_str(), "rail" | "bridge")
                }) || self.state.projects.iter().any(|project| {
                    let (Some(from), Some(to)) = (project.from, project.to) else {
                        return false;
                    };
                    if project.kind != "road"
                        || project.progress >= 1.0
                        || (from != head_id && to != head_id)
                    {
                        return false;
                    }
                    let matches_nonordinary_road = self.state.roads.iter().any(|road| {
                        matches!(road.kind.as_str(), "rail" | "bridge")
                            && ((road.from == from && road.to == to)
                                || (road.to == from && road.from == to))
                    });
                    !matches_nonordinary_road
                });
                if ordinary_open_or_planned {
                    continue;
                }
                let nearest_opposite_catchment = open_buildings
                    .iter()
                    .map(|building| (building.x - opposite.x).hypot(building.z - opposite.z))
                    .min_by(f32::total_cmp);
                if !nearest_opposite_catchment.is_some_and(|distance| distance <= 110.0) {
                    continue;
                }
                let outward_heading = (head.z - opposite.z).atan2(head.x - opposite.x);
                headings.insert(head_id, outward_heading);
            }
        }
        headings
    }

    /// Enumerate the first viable frontier proposal without mutating the
    /// engine. The local RNG and cursor follow the legacy search order so the
    /// caller can commit precisely the state the mutating planner used.
    fn propose_frontier_road(
        &self,
        open_buildings: &[Building],
        bridgehead_headings: &BTreeMap<u32, f32>,
        collision_context: &CollisionContext,
    ) -> frontier::FrontierScan {
        let center = open_buildings.iter().fold((0.0, 0.0), |(x, z), building| {
            (
                x + building.x / open_buildings.len() as f32,
                z + building.z / open_buildings.len() as f32,
            )
        });
        let connected_nodes: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
            .flat_map(|road| [road.from, road.to])
            .collect();
        // An open bridge can leave its far endpoint connected but outside the
        // normal occupied-frontier radius. Admit exactly one bank-entry origin
        // while the opposite bank has an active catchment and no ordinary road
        // has already claimed the bridgehead.
        let mut candidates: Vec<(Node, f32, Option<f32>)> = self
            .state
            .nodes
            .iter()
            .filter(|node| connected_nodes.contains(&node.id))
            .filter_map(|node| {
                let nearest_occupied_block = open_buildings
                    .iter()
                    .map(|building| (building.x - node.x).hypot(building.z - node.z))
                    .min_by(f32::total_cmp)?;
                // Start at the edge of a developed block, never at an unserved
                // network tip that is already far from the neighborhood. A
                // bridgehead is the sole exception, and only while its opposite
                // bank is developed and this bank has no ordinary connection.
                if nearest_occupied_block <= 110.0 {
                    Some((
                        node.clone(),
                        nearest_occupied_block,
                        bridgehead_headings.get(&node.id).copied(),
                    ))
                } else {
                    bridgehead_headings
                        .get(&node.id)
                        .copied()
                        .map(|heading| (node.clone(), nearest_occupied_block, Some(heading)))
                }
            })
            .collect();
        // Prefer a viable frontage near the developed edge, while randomizing
        // among a few similarly placed nodes to keep seeded layouts distinct.
        candidates.sort_by(|(a, _, _), (b, _, _)| {
            let a_radius = (a.x - center.0).hypot(a.z - center.1);
            let b_radius = (b.x - center.0).hypot(b.z - center.1);
            b_radius.total_cmp(&a_radius).then_with(|| a.id.cmp(&b.id))
        });
        if candidates.is_empty() {
            return frontier::FrontierScan {
                proposal: None,
                next_cursor: self.state.road_origin_cursor,
                next_rng: self.state.rng_growth,
                allow_access_fallback: false,
            };
        }

        let mut rng = self.state.rng_growth;
        let choice_count = candidates.len();
        let first_choice = self.state.road_origin_cursor % choice_count;
        let origin_count = choice_count.min(MAX_ROAD_ORIGINS_PER_EVALUATION);
        let mut next_cursor = (first_choice + origin_count) % choice_count;
        for attempt in 0..origin_count {
            let idx = (first_choice + attempt) % choice_count;
            next_cursor = (idx + 1) % choice_count;
            let start = candidates[idx].0.clone();
            let bridgehead_heading = candidates[idx].2;
            if self
                .state
                .projects
                .iter()
                .any(|p| p.kind == "road" && p.from == Some(start.id) && p.progress < 1.0)
            {
                continue;
            }
            let generated_junction = start.id >= 7;
            if generated_junction && self.ordinary_road_degree(start.id) >= 4 {
                continue;
            }
            let mut headings = bridgehead_heading
                .map(|heading| vec![heading])
                .unwrap_or_else(|| self.road_extension_headings(start.id));
            if headings.is_empty() {
                // Match the legacy seeded fallback for rail/bridge anchors.
                headings.push(random_unit(&mut rng) * std::f32::consts::TAU);
            }
            let rotate = (rng_next(&mut rng) as usize) % headings.len();
            headings.rotate_left(rotate);
            let mut loop_targets: Vec<Node> = self
                .state
                .nodes
                .iter()
                .filter(|node| node.id != start.id && connected_nodes.contains(&node.id))
                .filter(|node| {
                    let distance = (node.x - start.x).hypot(node.z - start.z);
                    distance >= 45.0
                        && distance <= 90.0
                        && !self.state.roads.iter().any(|road| {
                            (road.from == start.id && road.to == node.id)
                                || (road.to == start.id && road.from == node.id)
                        })
                        && !self.state.projects.iter().any(|project| {
                            project.kind == "road"
                                && project.progress < 1.0
                                && ((project.from == Some(start.id) && project.to == Some(node.id))
                                    || (project.from == Some(node.id)
                                        && project.to == Some(start.id)))
                        })
                })
                .cloned()
                .collect();
            for base_angle in headings.iter().copied().flat_map(|heading| [heading; 3]) {
                let angle = base_angle + (random_unit(&mut rng) - 0.5) * 0.18;
                if !self.ordinary_heading_clear(start.id, angle) {
                    continue;
                }
                let length = 50.0 + random_unit(&mut rng) * 38.0;
                let mut x = start.x + angle.cos() * length;
                let mut z = start.z + angle.sin() * length;
                x = x.clamp(MIN + 25.0, MAX - 25.0);
                z = z.clamp(MIN + 25.0, MAX - 25.0);
                if !self.ordinary_heading_clear(start.id, (z - start.z).atan2(x - start.x)) {
                    continue;
                }
                if in_protected_land(x, z) || corridor_crosses_protected(start.x, start.z, x, z) {
                    continue;
                }
                let point = Node {
                    id: u32::MAX,
                    x,
                    z,
                    kind: "candidate".into(),
                };
                let try_loop = !loop_targets.is_empty() && random_unit(&mut rng) < 0.28;
                let mut loop_target = None;
                if try_loop {
                    let target_index = (rng_next(&mut rng) as usize) % loop_targets.len();
                    let target = loop_targets.swap_remove(target_index);
                    let target_angle = (target.z - start.z).atan2(target.x - start.x);
                    if self.ordinary_heading_clear(start.id, target_angle)
                        && self.ordinary_heading_clear(
                            target.id,
                            (start.z - target.z).atan2(start.x - target.x),
                        )
                        && (target.id < 7 || self.ordinary_road_degree(target.id) < 4)
                        && !in_protected_land(target.x, target.z)
                        && !corridor_crosses_protected(start.x, start.z, target.x, target.z)
                    {
                        loop_target = Some(target.clone());
                    }
                }
                let endpoint = loop_target.as_ref().unwrap_or(&point);
                if !self.ordinary_road_clear_of_river(start.x, start.z, endpoint.x, endpoint.z, 4.5)
                {
                    continue;
                }
                if loop_target.is_none()
                    && self
                        .state
                        .nodes
                        .iter()
                        .any(|node| (node.x - x).hypot(node.z - z) < 18.0)
                {
                    continue;
                }
                let corridor_clear = self.road_corridor_clear_of_parcels_with_context(
                    start.x,
                    start.z,
                    endpoint.x,
                    endpoint.z,
                    collision_context,
                );
                let endpoint_id = loop_target.as_ref().map(|node| node.id);
                let road_clear = self
                    .state
                    .roads
                    .iter()
                    .filter(|road| road.stage == "open")
                    .all(|road| {
                        if road.from == start.id
                            || road.to == start.id
                            || Some(road.from) == endpoint_id
                            || Some(road.to) == endpoint_id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                            return true;
                        };
                        segments_distance(&start, endpoint, a, b) > 8.0
                    });
                let projects_clear = self
                    .state
                    .projects
                    .iter()
                    .filter(|project| project.kind == "road" && project.progress < 1.0)
                    .all(|project| {
                        let (Some(from), Some(to)) = (project.from, project.to) else {
                            return true;
                        };
                        if from == start.id
                            || to == start.id
                            || Some(from) == endpoint_id
                            || Some(to) == endpoint_id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                            return true;
                        };
                        segments_distance(&start, endpoint, a, b) > 8.0
                    });
                if !corridor_clear
                    || !road_clear
                    || !projects_clear
                    || !self.neighborhood_corridor_clear_for_road(&start, endpoint, start.id)
                    || self.viable_frontage_benefit_count(&start, endpoint, 2) < 2
                {
                    continue;
                }
                return frontier::FrontierScan {
                    proposal: Some(frontier::FrontierProposal {
                        start_id: start.id,
                        target_id: endpoint_id,
                        end_x: endpoint.x,
                        end_z: endpoint.z,
                        bridgehead: bridgehead_heading.is_some(),
                    }),
                    next_cursor,
                    next_rng: rng,
                    allow_access_fallback: false,
                };
            }
        }
        frontier::FrontierScan {
            proposal: None,
            next_cursor,
            next_rng: rng,
            allow_access_fallback: true,
        }
    }

    fn plan_road(&mut self) {
        self.plan_road_with_policy(self.district_policy_experiment);
    }

    fn plan_road_with_policy(&mut self, use_district_policy: bool) {
        if self.state.roads.len() + self.state.projects.len() + self.spine_reserved_road_records()
            >= MAX_SEGMENTS
            || !self.project_admission_available()
        {
            return;
        }
        let open_buildings: Vec<Building> = self
            .state
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .cloned()
            .collect();
        if open_buildings.is_empty() {
            return;
        }
        // Preserve the existing bridgehead opportunity for frontier planning;
        // interior access remains a fallback after viable outward proposals.
        let bridgehead_headings = self.eligible_bridgehead_headings(&open_buildings);
        let collision_context = self.collision_context();
        if self.try_plan_local_chord(&open_buildings, &collision_context) {
            return;
        }
        let scan =
            self.propose_frontier_road(&open_buildings, &bridgehead_headings, &collision_context);
        // An already-selected bridgehead is an infrastructure prerequisite:
        // keep its admission ahead of density scoring exactly as before.
        if scan
            .proposal
            .as_ref()
            .is_some_and(|proposal| proposal.bridgehead)
        {
            let proposal = scan.proposal.clone().unwrap();
            self.commit_frontier_road(scan, proposal);
            return;
        }
        let can_compare = use_district_policy
            && (scan.proposal.is_some() || scan.allow_access_fallback)
            && self.district_runtime.metrics.is_some()
            && self
                .state
                .tick
                .saturating_sub(self.state.district_controller.completed_capture_tick)
                <= district_policy::METRICS_MAX_AGE_TICKS
            && road_demand_is_sufficient(self.state.demand);
        if can_compare {
            #[cfg(test)]
            {
                self.district_policy_comparisons =
                    self.district_policy_comparisons.saturating_add(1);
            }
            let access_scan = self.propose_interior_access();
            // Both read-only scans advance their deterministic cursors once,
            // even when their proposal loses this comparison.
            self.state.road_origin_cursor = scan.next_cursor;
            self.state.rng_growth = scan.next_rng;
            self.state.interior_access_cursor = access_scan.next_cursor;
            if self.try_admit_ranked_district_road(scan.proposal, access_scan.proposals) {
                return;
            }
            return;
        }
        let Some(proposal) = scan.proposal.clone() else {
            if scan.allow_access_fallback && self.access_fallback_not_attempted_this_tick() {
                self.state.road_origin_cursor = scan.next_cursor;
                self.state.rng_growth = scan.next_rng;
                self.try_plan_interior_access();
            }
            return;
        };
        debug_assert_eq!(
            proposal.bridgehead,
            bridgehead_headings.contains_key(&proposal.start_id)
        );
        self.commit_frontier_road(scan, proposal);
    }

    #[cfg(test)]
    fn access_fallback_not_attempted_this_tick(&self) -> bool {
        self.early_access_attempted_tick != Some(self.state.tick)
    }

    #[cfg(not(test))]
    fn access_fallback_not_attempted_this_tick(&self) -> bool {
        true
    }

    fn commit_frontier_road(
        &mut self,
        scan: frontier::FrontierScan,
        proposal: frontier::FrontierProposal,
    ) {
        self.state.road_origin_cursor = scan.next_cursor;
        self.state.rng_growth = scan.next_rng;
        if !self.project_admission_available() {
            return;
        }
        let to = proposal
            .target_id
            .unwrap_or_else(|| self.add_node(proposal.end_x, proposal.end_z, "junction"));
        let id = self.id();
        let reason = "A connected street extension serves demand from nearby homes and businesses";
        let duration = 3.0 + random_unit(&mut self.state.rng_growth) * 4.0;
        self.record_project(Project {
            id,
            kind: "road".into(),
            from: Some(proposal.start_id),
            to: Some(to),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.0,
            reason: reason.into(),
            duration,
            started_tick: self.state.tick,
        });
        self.state.revision += 1;
    }

    fn try_admit_ranked_district_road(
        &mut self,
        frontier_proposal: Option<frontier::FrontierProposal>,
        access_proposals: Vec<interior_access::AccessProposal>,
    ) -> bool {
        debug_assert!(access_proposals.len() <= district_policy::MAX_ACCESS_PROPOSALS);
        #[cfg(test)]
        let mut diagnostic = district_policy::DecisionRecord {
            tick: self.state.tick,
            frontier_proposal: frontier_proposal.is_some(),
            access_proposals: access_proposals.len(),
            ..district_policy::DecisionRecord::default()
        };
        let Some(metrics) = self.district_runtime.metrics.as_ref() else {
            #[cfg(test)]
            {
                diagnostic.no_admission_reason = Some("no_metrics");
                self.district_policy_diagnostics.observe(diagnostic);
            }
            return false;
        };
        let targets = self.district_targets(metrics);
        let mut score_candidates = Vec::with_capacity(district_policy::MAX_COMPARED_PROJECTS);
        let mut access_by_id = BTreeMap::new();
        let mut frontier_by_id = None;
        let building_samples = self.live_catchment_buildings();
        let demand_fit = district_policy::demand_fit(self.state.demand);

        if let Some(proposal) = frontier_proposal {
            let Some(start) = self.node(proposal.start_id) else {
                #[cfg(test)]
                {
                    diagnostic.no_admission_reason = Some("no_geometry");
                    self.district_policy_diagnostics.observe(diagnostic);
                }
                return false;
            };
            let endpoint = if let Some(target_id) = proposal.target_id {
                let Some(node) = self.node(target_id) else {
                    #[cfg(test)]
                    {
                        diagnostic.no_admission_reason = Some("no_geometry");
                        self.district_policy_diagnostics.observe(diagnostic);
                    }
                    return false;
                };
                node.clone()
            } else {
                Node {
                    id: u32::MAX,
                    x: proposal.end_x,
                    z: proposal.end_z,
                    kind: "candidate".into(),
                }
            };
            let rects = self.viable_frontage_benefit_sites(
                start,
                &endpoint,
                district_policy::MAX_FRONTIER_SITES,
            );
            #[cfg(test)]
            {
                diagnostic.frontier_geometry_candidates = usize::from(!rects.is_empty());
                diagnostic.frontier_sites = rects.len();
                diagnostic.frontier_area_m2 = rects
                    .iter()
                    .map(|rect| (rect.width * rect.depth) as f64)
                    .sum();
            }
            let stable_id = district_frontier_candidate_id(proposal.start_id, proposal.target_id);
            let catchment = district_policy::catchment_at(&building_samples, start.x, start.z);
            let source_market_accepts = district_policy::source_market_accepts_capacity(catchment);
            let feasible = self.state.demand > 0.8 && source_market_accepts;
            #[cfg(test)]
            {
                diagnostic.frontier_source_rejected =
                    diagnostic.frontier_geometry_candidates > 0 && !source_market_accepts;
                diagnostic.frontier_source_feasible_candidates = usize::from(
                    diagnostic.frontier_geometry_candidates > 0 && source_market_accepts,
                );
            }
            let road_cost = (endpoint.x - start.x).hypot(endpoint.z - start.z) as f64;
            if let Some(candidate) = self.make_district_score_candidate(
                stable_id,
                district_density::CandidateKind::Expansion,
                road_cost,
                demand_fit,
                catchment,
                feasible,
                &rects,
                metrics,
            ) {
                score_candidates.push(candidate);
                frontier_by_id = Some((stable_id, proposal));
            }
        }

        for proposal in access_proposals {
            #[cfg(test)]
            {
                diagnostic.access_sites += proposal.selected_lots.len();
                diagnostic.access_geometry_candidates +=
                    usize::from(!proposal.selected_lots.is_empty());
                diagnostic.access_area_m2 += proposal
                    .selected_lots
                    .iter()
                    .map(|lot| (lot.width * lot.depth) as f64)
                    .sum::<f64>();
            }
            let Some(host) = self
                .state
                .roads
                .iter()
                .find(|road| road.id == proposal.host_id)
            else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(host.from), self.node(host.to)) else {
                continue;
            };
            let source_x = a.x + (b.x - a.x) * proposal.host_t;
            let source_z = a.z + (b.z - a.z) * proposal.host_t;
            let catchment = district_policy::catchment_at(&building_samples, source_x, source_z);
            let source_market_accepts = district_policy::source_market_accepts_capacity(catchment);
            let feasible = self.state.demand > 0.8 && source_market_accepts;
            let rects: Vec<_> = proposal
                .selected_lots
                .iter()
                .map(|lot| geometry::OrientedRect {
                    x: lot.x,
                    z: lot.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                })
                .collect();
            #[cfg(test)]
            {
                if !source_market_accepts && !rects.is_empty() {
                    diagnostic.access_vacancy_rejections += 1;
                } else if !rects.is_empty() {
                    diagnostic.access_source_feasible_candidates += 1;
                }
            }
            debug_assert!(rects.len() <= district_policy::MAX_ACCESS_LOTS_PER_PROPOSAL);
            let stable_id = district_access_candidate_id(&proposal);
            let road_cost = proposal.score.road_length_mm as f64 / 1_000.0;
            if let Some(candidate) = self.make_district_score_candidate(
                stable_id,
                district_density::CandidateKind::InteriorAccess,
                road_cost,
                demand_fit,
                catchment,
                feasible,
                &rects,
                metrics,
            ) {
                score_candidates.push(candidate);
                access_by_id.insert(stable_id, proposal);
            }
        }

        if score_candidates.is_empty() {
            #[cfg(test)]
            {
                diagnostic.no_admission_reason = Some(
                    if diagnostic.frontier_sites == 0 && diagnostic.access_sites == 0 {
                        "no_geometry"
                    } else if diagnostic.frontier_source_rejected
                        || diagnostic.access_vacancy_rejections > 0
                    {
                        "no_source_feasible"
                    } else {
                        "no_cell_forecast"
                    },
                );
                self.district_policy_diagnostics.observe(diagnostic);
            }
            return false;
        }
        debug_assert!(score_candidates.len() <= district_policy::MAX_COMPARED_PROJECTS);
        let weights = district_density::ScoreWeights {
            coverage: 1.0,
            floor_area: 0.0,
            frontage: 0.0,
            demand: 0.2,
            road_cost_scale_m: 60.0,
        };
        let ranked = match project_scoring::rank_projects(
            metrics,
            &score_candidates,
            &targets,
            district_policy::vacancy_policy(),
            weights,
        ) {
            Ok(ranked) => ranked,
            Err(_) => {
                #[cfg(test)]
                {
                    diagnostic.no_admission_reason = Some("score_error");
                    self.district_policy_diagnostics.observe(diagnostic);
                }
                return false;
            }
        };
        if ranked.is_empty() {
            #[cfg(test)]
            {
                diagnostic.no_admission_reason = Some(
                    if diagnostic.frontier_source_rejected
                        || diagnostic.access_vacancy_rejections > 0
                    {
                        "no_source_feasible"
                    } else {
                        "no_cell_forecast"
                    },
                );
                self.district_policy_diagnostics.observe(diagnostic);
            }
            return false;
        }
        #[cfg(test)]
        {
            for candidate in &ranked {
                match candidate.kind {
                    district_density::CandidateKind::Expansion => {
                        diagnostic.frontier_ranked += 1;
                        if diagnostic.frontier_top_score.is_none() {
                            diagnostic.frontier_top_score = Some(candidate.score);
                        }
                    }
                    district_density::CandidateKind::InteriorAccess => {
                        diagnostic.access_ranked += 1;
                        if diagnostic.access_top_score.is_none() {
                            diagnostic.access_top_score = Some(candidate.score);
                        }
                    }
                    district_density::CandidateKind::Infill => {}
                }
            }
        }
        for candidate in ranked {
            if candidate.kind == district_density::CandidateKind::InteriorAccess {
                let Some(proposal) = access_by_id.remove(&candidate.stable_id) else {
                    continue;
                };
                if self.admit_interior_access(
                    proposal.host_id,
                    proposal.host_t,
                    WorldPoint {
                        x: proposal.end_x,
                        z: proposal.end_z,
                    },
                    proposal.selected_lots,
                ) {
                    #[cfg(test)]
                    {
                        self.district_policy_access_wins =
                            self.district_policy_access_wins.saturating_add(1);
                        diagnostic.winner = Some("access");
                        self.district_policy_diagnostics.observe(diagnostic);
                    }
                    return true;
                }
            } else if let Some((stable_id, proposal)) = frontier_by_id.take() {
                if stable_id == candidate.stable_id {
                    let scan = frontier::FrontierScan {
                        proposal: None,
                        next_cursor: self.state.road_origin_cursor,
                        next_rng: self.state.rng_growth,
                        allow_access_fallback: false,
                    };
                    self.commit_frontier_road(scan, proposal);
                    #[cfg(test)]
                    {
                        self.district_policy_frontier_wins =
                            self.district_policy_frontier_wins.saturating_add(1);
                        diagnostic.winner = Some("frontier");
                        self.district_policy_diagnostics.observe(diagnostic);
                    }
                    return true;
                }
                frontier_by_id = Some((stable_id, proposal));
            }
        }
        #[cfg(test)]
        {
            diagnostic.no_admission_reason = Some("ranked_admission_rejected");
            self.district_policy_diagnostics.observe(diagnostic);
        }
        false
    }

    fn district_targets(
        &self,
        metrics: &district_density::DistrictMetrics,
    ) -> Vec<district_density::DistrictTargets> {
        let minima: Vec<f64> = metrics
            .cells
            .iter()
            .map(|cell| {
                let x = ((cell.min_x + cell.max_x) * 0.5) as f32;
                let z = ((cell.min_z + cell.max_z) * 0.5) as f32;
                match self.neighborhood_profile(x, z) {
                    NeighborhoodProfile::HistoricCore => 0.32,
                    NeighborhoodProfile::StationCore => 0.40,
                    NeighborhoodProfile::StationFringe => 0.35,
                    NeighborhoodProfile::OuterNeighborhood => 0.28,
                }
            })
            .collect();
        district_policy::targets(metrics, &minima)
    }

    fn live_catchment_buildings(&self) -> Vec<district_policy::ServedBuilding> {
        let connected_nodes: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
            .flat_map(|road| [road.from, road.to])
            .collect();
        let access_nodes: BTreeMap<u32, u32> = self
            .state
            .parcels
            .iter()
            .map(|parcel| (parcel.id, parcel.access_node))
            .collect();
        self.state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .filter(|building| {
                access_nodes
                    .get(&building.parcel_id)
                    .is_some_and(|node_id| connected_nodes.contains(node_id))
            })
            .map(|building| district_policy::ServedBuilding {
                x: building.x,
                z: building.z,
                housing_capacity: if is_housing(building) {
                    building.capacity as f64
                } else {
                    0.0
                },
                housing_vacant: if is_housing(building) {
                    building.capacity.saturating_sub(building.occupancy) as f64
                } else {
                    0.0
                },
                commercial_capacity: if is_commercial(building) {
                    building.capacity as f64
                } else {
                    0.0
                },
                commercial_vacant: if is_commercial(building) && !building.business_active {
                    building.capacity as f64
                } else {
                    0.0
                },
            })
            .collect()
    }

    fn make_district_score_candidate(
        &self,
        stable_id: u64,
        kind: district_density::CandidateKind,
        road_cost_m: f64,
        demand_fit: f64,
        catchment: project_scoring::LiveCatchment,
        feasible: bool,
        rects: &[geometry::OrientedRect],
        metrics: &district_density::DistrictMetrics,
    ) -> Option<project_scoring::ProjectScoreCandidate> {
        if rects.is_empty() {
            return None;
        }
        let mut gains = BTreeMap::<usize, f64>::new();
        for rect in rects {
            let clipped = district_density::clipped_envelope_areas(metrics, *rect).ok()?;
            for (cell_id, area) in clipped {
                *gains.entry(cell_id).or_default() += area;
            }
        }
        if gains.is_empty() {
            return None;
        }
        let cell_impacts = gains
            .into_iter()
            .map(|(cell_id, area)| project_scoring::ForecastCellImpact {
                cell_id,
                forecast_envelope_gain_m2: area * district_policy::FORECAST_UTILIZATION,
                forecast_floor_area_gain_m2: 0.0,
                forecast_occupied_frontage_gain_m: 0.0,
                // Positive one-unit flags activate vacancy relevance; these
                // are not estimates of the future building capacity.
                forecast_housing_capacity_gain: 1.0,
                forecast_commercial_capacity_gain: 1.0,
            })
            .collect();
        Some(project_scoring::ProjectScoreCandidate {
            stable_id,
            kind,
            feasible,
            road_cost_m,
            demand_fit,
            live_catchment: catchment,
            cell_impacts,
        })
    }
    fn plan_building(&mut self, day: u32) {
        if self.state.buildings.len() + self.state.projects.len() >= MAX_BUILDINGS
            || !self.project_admission_available()
        {
            return;
        }
        let mut choices: Vec<usize> = self
            .state
            .parcels
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                !p.protected
                    && p.reservation.is_none()
                    && (self.state.station_online || !self.is_station_tower_lot(p))
                    && self.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                    && self.state.buildings.iter().all(|b| b.parcel_id != p.id)
                    && self.node_connected(p.access_node)
            })
            .map(|(i, _)| i)
            .collect();
        if choices.is_empty() {
            self.create_frontier_parcels();
            choices = self
                .state
                .parcels
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    !p.protected
                        && p.reservation.is_none()
                        && (self.state.station_online || !self.is_station_tower_lot(p))
                        && self.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                        && self.state.buildings.iter().all(|b| b.parcel_id != p.id)
                        && self.node_connected(p.access_node)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if choices.is_empty() {
            return;
        }
        let scored_candidates: Vec<_> = choices
            .iter()
            .map(|&parcel_index| {
                let parcel = &self.state.parcels[parcel_index];
                development::BuildingSiteCandidate {
                    parcel_index,
                    parcel_id: parcel.id,
                    x: parcel.x,
                    z: parcel.z,
                }
            })
            .collect();
        let open_buildings: Vec<_> = self
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .map(|building| development::OpenBuildingPoint {
                x: building.x,
                z: building.z,
                active: site_active(building),
            })
            .collect();
        let Some(selection) = development::select_building_site(
            &scored_candidates,
            &open_buildings,
            self.state.growth_cursor,
        ) else {
            return;
        };
        self.state.growth_cursor = selection.next_cursor;
        let parcel = self.state.parcels[selection.parcel_index].clone();
        let homes = self
            .state
            .buildings
            .iter()
            .filter(|b| matches!(b.kind.as_str(), "home" | "apartment"))
            .count();
        let shops = self
            .state
            .buildings
            .iter()
            .filter(|b| b.kind == "shop" || b.kind == "cafe")
            .count();
        let (kind, capacity, floors, name) = self.choose_building_profile(&parcel, homes, shops);
        let history = vec![format!(
            "Reserved after sustained neighborhood demand on day {}",
            day
        )];
        let building = self.add_building(
            parcel.id,
            kind,
            floors,
            "foundation",
            0.0,
            capacity,
            name,
            false,
            history,
        );
        if let Some(p) = self.state.parcels.iter_mut().find(|p| p.id == parcel.id) {
            p.reservation = Some(building);
        }
        self.mark_access_geometry_changed();
        let project_id = self.id();
        let duration = 2.0 + random_unit(&mut self.state.rng_growth) * 4.0;
        self.record_project(Project {
            id: project_id,
            kind: "building".into(),
            from: None,
            to: None,
            parcel_id: Some(parcel.id),
            stage: "foundation".into(),
            progress: 0.0,
            reason: format!("{} is built to meet nearby demand", name),
            duration,
            started_tick: self.state.tick,
        });
        self.state.road_infill_due = self.state.road_infill_due.saturating_sub(1);
        self.state.revision += 1;
    }
    fn create_frontier_parcels(&mut self) {
        if self.state.parcels.len() >= MAX_PARCELS {
            return;
        }
        let mut candidates: Vec<(Node, f32)> = self
            .state
            .nodes
            .iter()
            .filter(|node| self.node_connected(node.id))
            .filter_map(|node| {
                let distance = self
                    .state
                    .buildings
                    .iter()
                    .filter(|building| site_active(building))
                    .map(|building| (building.x - node.x).hypot(building.z - node.z))
                    .min_by(f32::total_cmp)?;
                (distance <= 110.0).then_some((node.clone(), distance))
            })
            .collect();
        if candidates.is_empty() {
            return;
        }
        candidates.sort_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| a.id.cmp(&b.id)));
        let first = self.state.growth_cursor % candidates.len();
        for attempt in 0..candidates.len() {
            let (node, _) = &candidates[(first + attempt) % candidates.len()];
            let offsets = if attempt % 2 == 0 {
                [-0.5_f32, 0.5]
            } else {
                [0.5_f32, -0.5]
            };
            for offset in offsets {
                let x = (node.x + offset * 27.0).clamp(MIN + 18.0, MAX - 18.0);
                let z = (node.z + if offset < 0.0 { 19.0 } else { -19.0 })
                    .clamp(MIN + 18.0, MAX - 18.0);
                if !self.parcel_geometry_clear(x, z, 14.0, 12.0) {
                    continue;
                }
                let suitability = 0.55 + random_unit(&mut self.state.rng_growth) * 0.4;
                let parcel_id = self.add_parcel(x, z, 14.0, 12.0, node.id, suitability, false);
                self.remember_nearest_frontage(parcel_id, node.id);
                self.state.growth_cursor = self.state.growth_cursor.saturating_add(1);
                return;
            }
        }
    }
    fn node_connected(&self, id: u32) -> bool {
        self.state
            .roads
            .iter()
            .any(|r| r.stage == "open" && r.connected && (r.from == id || r.to == id))
    }
    fn recompute_connectivity(&mut self) {
        let gateway = self
            .state
            .nodes
            .iter()
            .find(|n| n.kind == "gateway")
            .map(|n| n.id);
        let Some(start) = gateway else { return };
        let mut seen = BTreeSet::new();
        let mut q = VecDeque::from([start]);
        seen.insert(start);
        // Preserve source-road neighbor order while avoiding a full road
        // scan for every visited node in a large connected district.
        let mut adjacency: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for road in self.state.roads.iter().filter(|road| road.stage == "open") {
            adjacency.entry(road.from).or_default().push(road.to);
            if road.from != road.to {
                adjacency.entry(road.to).or_default().push(road.from);
            }
        }
        while let Some(n) = q.pop_front() {
            if let Some(links) = adjacency.get(&n) {
                for &next in links {
                    if seen.insert(next) {
                        q.push_back(next);
                    }
                }
            }
        }
        let mut connectivity_changed = false;
        for road in &mut self.state.roads {
            let connected =
                road.stage == "open" && seen.contains(&road.from) && seen.contains(&road.to);
            connectivity_changed |= road.connected != connected;
            road.connected = connected;
        }
        if connectivity_changed {
            self.mark_access_geometry_changed();
        }
    }
    fn renewal(&mut self, day: u32) {
        if !self.project_admission_available() {
            return;
        }
        self.state.last_renewal_day = day;
        self.try_close_business_for_turnover(day);
        let candidates: Vec<u32> = self
            .state
            .buildings
            .iter()
            .filter(|b| !b.anchor && site_vacant(b) && b.vacant_days > 5)
            .map(|b| b.id)
            .collect();
        if candidates.is_empty() {
            return;
        }
        let i = (rng_next(&mut self.state.rng_growth) as usize) % candidates.len();
        let id = candidates[i];
        let (parcel_id, msg) = if let Some(b) = self.state.buildings.iter_mut().find(|b| b.id == id)
        {
            let msg = format!("{} is renovated after a quiet period", b.name);
            b.history.push(msg.clone());
            if b.history.len() > 10 {
                b.history.remove(0);
            }
            b.stage = "renovation".into();
            b.progress = 0.0;
            b.vacant_days = 0;
            b.business_active = false;
            (b.parcel_id, msg)
        } else {
            return;
        };
        if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
            parcel.reservation = Some(id);
        }
        self.mark_access_geometry_changed();
        let pid = self.id();
        self.record_project(Project {
            id: pid,
            kind: "building".into(),
            from: None,
            to: None,
            parcel_id: Some(parcel_id),
            stage: "structure".into(),
            progress: 0.0,
            reason: msg,
            duration: 2.0,
            started_tick: self.state.tick,
        });
        self.record(
            "RenovationStarted",
            id,
            "A vacant property is renewed for a new neighborhood use".into(),
        );
        self.state.revision += 1;
    }
    fn try_close_business_for_turnover(&mut self, day: u32) -> bool {
        if day == 0 || day % 30 != 0 || self.state.last_business_turnover_day == day {
            return false;
        }
        self.state.last_business_turnover_day = day;
        let mut referenced = BTreeSet::new();
        for resident in &self.state.residents {
            if let Some(building_id) = resident.work_building {
                referenced.insert(building_id);
            }
        }
        for traveler in &self.state.travelers {
            if let Some(building_id) = traveler.destination_building {
                referenced.insert(building_id);
            }
            if let Some(building_id) = traveler.origin_building {
                referenced.insert(building_id);
            }
        }
        let mut active_by_kind = BTreeMap::<String, usize>::new();
        for building in &self.state.buildings {
            if is_commercial(building) && site_active(building) {
                *active_by_kind.entry(building.kind.clone()).or_default() += 1;
            }
        }
        let eligible: Vec<u32> = self
            .state
            .buildings
            .iter()
            .filter(|building| {
                is_commercial(building)
                    && building.business_active
                    && building.stage == "open"
                    && !building.anchor
                    && day.saturating_sub(building.built_day) >= 30
                    && !referenced.contains(&building.id)
                    && active_by_kind.get(&building.kind).copied().unwrap_or(0) > 1
            })
            .map(|building| building.id)
            .collect();
        if eligible.is_empty() || rng_next(&mut self.state.rng_growth) % 16 != 0 {
            return false;
        }
        let chosen = eligible[rng_next(&mut self.state.rng_growth) as usize % eligible.len()];
        let Some(building) = self
            .state
            .buildings
            .iter_mut()
            .find(|building| building.id == chosen)
        else {
            return false;
        };
        let reason = format!(
            "Business closed: {} pauses for a seeded tenant change on day {}",
            building.name, day
        );
        push_history(&mut building.history, reason.clone());
        building.business_active = false;
        building.occupancy = 0;
        building.vacant_days = 0;
        self.record("BusinessClosed", chosen, reason);
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }
    fn sync_residents(&mut self) {
        if self.state.residents.len() >= MAX_RESIDENTS {
            return;
        }
        let homes: Vec<(u32, u16, u32)> = self
            .state
            .buildings
            .iter()
            .filter(|b| is_housing(b) && b.stage == "open" && b.occupancy > 0)
            .filter_map(|b| {
                self.state
                    .parcels
                    .iter()
                    .find(|p| p.id == b.parcel_id)
                    .map(|p| (b.id, b.occupancy, p.access_node))
            })
            .collect();
        let destinations: Vec<(u32, u32, String, u16)> = self
            .state
            .buildings
            .iter()
            .filter(|b| is_commercial(b) && site_active(b))
            .filter_map(|b| {
                self.state
                    .parcels
                    .iter()
                    .find(|p| p.id == b.parcel_id)
                    .map(|p| (b.id, p.access_node, b.kind.clone(), b.capacity))
            })
            .collect();
        if destinations.is_empty() {
            return;
        }
        let mut current_by_home = BTreeMap::<u32, u16>::new();
        for resident in &self.state.residents {
            *current_by_home.entry(resident.home_building).or_default() += 1;
        }
        let mut assigned_by_work = BTreeMap::<u32, u16>::new();
        for resident in &self.state.residents {
            if resident.routine == "worker" {
                let Some(work) = resident.work_building else {
                    continue;
                };
                *assigned_by_work.entry(work).or_default() += 1;
            }
        }
        let mut existing_worker_count = self
            .state
            .residents
            .iter()
            .filter(|r| r.routine == "worker")
            .count();
        let max_jobs: usize = destinations
            .iter()
            .map(|(_, _, kind, capacity)| {
                if kind == "office" {
                    *capacity as usize
                } else {
                    (*capacity as usize / 2).max(1)
                }
            })
            .sum();
        for (home_id, occupancy, home_node) in homes {
            let current = *current_by_home.get(&home_id).unwrap_or(&0);
            for ordinal in current..occupancy {
                if self.state.residents.len() >= MAX_RESIDENTS {
                    return;
                }
                let mut choices: Vec<(u32, u32, String, u16)> = destinations.clone();
                choices.sort_by_key(|(id, _, _, _)| {
                    (assigned_by_work.get(id).copied().unwrap_or(0), *id)
                });
                let use_job = existing_worker_count < max_jobs && ordinal % 3 != 2;
                let work = if use_job {
                    choices
                        .iter()
                        .find(|(id, _, kind, capacity)| {
                            let jobs = if kind == "office" {
                                *capacity
                            } else {
                                (*capacity / 2).max(1)
                            };
                            assigned_by_work.get(id).copied().unwrap_or(0) < jobs
                        })
                        .cloned()
                        .or_else(|| choices.first().cloned())
                } else {
                    choices
                        .iter()
                        .find(|(_, _, kind, _)| kind == "shop" || kind == "cafe")
                        .cloned()
                        .or_else(|| choices.first().cloned())
                };
                let Some((local_work_id, local_work_node, _, _)) = work else {
                    continue;
                };
                let first_names = [
                    "Aiko", "Haru", "Mina", "Ren", "Sora", "Yui", "Kai", "Nana", "Tomo", "Emi",
                    "Riku", "Mei",
                ];
                let family_names = [
                    "Sato",
                    "Tanaka",
                    "Suzuki",
                    "Kato",
                    "Ito",
                    "Watanabe",
                    "Kobayashi",
                    "Nakamura",
                ];
                let first = first_names
                    [(rng_next(&mut self.state.rng_schedule) as usize) % first_names.len()];
                let family = family_names
                    [(rng_next(&mut self.state.rng_schedule) as usize) % family_names.len()];
                let is_night = use_job && rng_next(&mut self.state.rng_schedule) % 9 == 0;
                let rail_stops = self.rail_terminals();
                let rail_commuter = use_job
                    && !is_night
                    && rail_stops.is_some_and(|(station, _)| {
                        home_node == station || self.route(home_node, station, false).len() >= 2
                    })
                    && rng_next(&mut self.state.rng_schedule) % 5 == 0;
                let (work_id, work_node) = if rail_commuter {
                    (
                        None,
                        rail_stops
                            .map(|(_, gateway)| gateway)
                            .unwrap_or(local_work_node),
                    )
                } else {
                    (Some(local_work_id), local_work_node)
                };
                let routine = if rail_commuter {
                    "rail_commuter"
                } else if use_job {
                    "worker"
                } else {
                    "neighbor"
                }
                .to_string();
                if use_job && !rail_commuter {
                    existing_worker_count += 1;
                    *assigned_by_work.entry(local_work_id).or_default() += 1;
                }
                let id = self.id();
                let (leave, back) = if is_night {
                    (
                        19 * 60 + rng_next(&mut self.state.rng_schedule) % 180,
                        3 * 60 + rng_next(&mut self.state.rng_schedule) % 180,
                    )
                } else {
                    (
                        7 * 60 + rng_next(&mut self.state.rng_schedule) % 150,
                        16 * 60 + rng_next(&mut self.state.rng_schedule) % 240,
                    )
                };
                let departure_tick = next_clock_deadline(self.state.tick, leave as u16);
                let mut return_tick = next_clock_deadline(self.state.tick, back as u16);
                if return_tick <= departure_tick {
                    return_tick = return_tick.saturating_add(TICKS_PER_DAY);
                }
                let mode = if !rail_commuter && rng_next(&mut self.state.rng_schedule) % 5 == 0 {
                    1
                } else {
                    0
                };
                self.state.residents.push(Resident {
                    id,
                    name: format!("{first} {family}"),
                    home_building: home_id,
                    work_building: work_id,
                    home_node,
                    work_node: Some(work_node),
                    mode,
                    routine,
                    activity: "at_home".into(),
                    at_work: false,
                    departure_tick,
                    return_tick,
                    last_departure_day: u32::MAX,
                    last_return_day: u32::MAX,
                    history: vec![format!("Moved into a home near access node {home_node}")],
                    rail_commuter,
                    transfer: if rail_commuter { "home" } else { "" }.into(),
                    train_id: None,
                    queue_slot: None,
                    bus_id: None,
                    bus_origin_stop: None,
                    bus_destination_stop: None,
                    bus_stop_node: None,
                });
            }
        }
    }

    fn ensure_bus_service(&mut self) {
        if self.state.tick < self.state.next_bus_evaluation_tick {
            return;
        }
        self.state.next_bus_evaluation_tick = self.state.tick.saturating_add(BUS_RETRY_TICKS);
        if self.state.bus_line.as_ref().is_some_and(|line| line.active) {
            if self.state.buses.len() < MAX_BUSES {
                let waiting = self.bus_waiting_count();
                if waiting >= 12 {
                    if let Some((line_id, terminal)) = self
                        .state
                        .bus_line
                        .as_ref()
                        .map(|line| (line.id, line.stops.last().copied()))
                        .and_then(|(line_id, terminal)| {
                            terminal.map(|terminal| (line_id, terminal))
                        })
                    {
                        if let Some(progress) = self.bus_stop_distance(terminal) {
                            let bus_id = self.id();
                            self.state.buses.push(Bus {
                                id: bus_id,
                                line_id,
                                progress,
                                direction: -1,
                                dwell_ticks: BUS_DWELL_TICKS,
                                passengers: Vec::new(),
                                next_departure_tick: self
                                    .state
                                    .tick
                                    .saturating_add(BUS_HEADWAY_TICKS),
                                last_stop: Some(terminal),
                            });
                            self.record(
                                "BusServiceExpanded",
                                bus_id,
                                "A second neighborhood bus joins the line after demand grows"
                                    .into(),
                            );
                        }
                    }
                }
            }
            return;
        }
        let open_roads = self
            .state
            .roads
            .iter()
            .filter(|road| road.kind != "rail" && road.stage == "open" && road.connected)
            .count();
        let developed: Vec<u32> = self
            .state
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .filter_map(|building| {
                self.state
                    .parcels
                    .iter()
                    .find(|parcel| parcel.id == building.parcel_id)
                    .map(|parcel| parcel.access_node)
            })
            .filter(|node| self.node_has_open_road(*node))
            .collect();
        if !self.state.station_online || open_roads < 6 || developed.len() < 3 {
            return;
        }
        let mut candidates: Vec<u32> = developed;
        candidates.sort_unstable();
        candidates.dedup();
        if candidates.len() > 32 {
            let stride = (candidates.len() + 31) / 32;
            candidates = candidates.into_iter().step_by(stride).take(32).collect();
        }
        if candidates.len() < 3 {
            return;
        }
        let Some(station) = self
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "station" && self.node_has_open_road(node.id))
            .map(|node| node.id)
        else {
            return;
        };
        // Build the route around real recurring resident trips. For each distinct
        // home/work access node, cache only short reachable walks to candidate
        // stops; the same nearest-stop rule is used by schedule_bus_trip below.
        let mut stop_candidates = candidates;
        stop_candidates.push(station);
        stop_candidates.sort_unstable();
        stop_candidates.dedup();
        let demand: Vec<(u32, u32)> = self
            .state
            .residents
            .iter()
            .filter(|resident| !resident.rail_commuter)
            .filter_map(|resident| resident.work_node.map(|work| (resident.home_node, work)))
            .filter(|(home, work)| home != work)
            .collect();
        if demand.is_empty() {
            return;
        }
        let endpoints: BTreeSet<u32> = demand
            .iter()
            .flat_map(|(home, work)| [*home, *work])
            .collect();
        let mut endpoint_stop_distance = BTreeMap::<u32, BTreeMap<u32, f32>>::new();
        for endpoint in endpoints {
            let mut reachable = BTreeMap::new();
            for stop in stop_candidates.iter().copied() {
                let (Some(a), Some(b)) = (self.node(endpoint), self.node(stop)) else {
                    continue;
                };
                if distance(a, b) > 75.0 {
                    continue;
                }
                let path = self.route(endpoint, stop, false);
                if path.is_empty() {
                    continue;
                }
                let length: f32 = self.route_lengths(&path).iter().sum();
                if length <= 75.0 {
                    reachable.insert(stop, length);
                }
            }
            endpoint_stop_distance.insert(endpoint, reachable);
        }

        let route_score = |stops: &[u32], route_length: f32| -> (usize, usize, f32) {
            let nearest = |endpoint: u32| -> Option<(usize, f32)> {
                stops
                    .iter()
                    .enumerate()
                    .filter_map(|(index, stop)| {
                        endpoint_stop_distance
                            .get(&endpoint)
                            .and_then(|distances| distances.get(stop))
                            .map(|distance| (index, *distance))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            };
            let mut covered_pairs = 0usize;
            let mut endpoint_count = 0usize;
            let mut walk_cost = 0.0f32;
            for (home, work) in &demand {
                let home_stop = nearest(*home);
                let work_stop = nearest(*work);
                if let Some((_, distance)) = home_stop {
                    endpoint_count += 1;
                    walk_cost += distance;
                }
                if let Some((_, distance)) = work_stop {
                    endpoint_count += 1;
                    walk_cost += distance;
                }
                if let (Some((home_index, _)), Some((work_index, _))) = (home_stop, work_stop) {
                    if home_index != work_index {
                        covered_pairs += 1;
                    }
                }
            }
            (
                covered_pairs,
                endpoint_count,
                walk_cost + route_length * 0.35,
            )
        };

        let mut stops = vec![station];
        let mut route = vec![station];
        let mut remaining = stop_candidates;
        remaining.retain(|candidate| *candidate != station);
        while stops.len() < MAX_BUS_STOPS {
            let last = *stops.last().unwrap_or(&station);
            let visited: BTreeSet<u32> = route.iter().copied().collect();
            let current_length: f32 = self.route_lengths(&route).iter().sum();
            let mut best: Option<((usize, usize, f32), u32, Vec<u32>, Vec<u32>)> = None;
            for candidate in remaining.iter().copied() {
                if candidate == last || visited.contains(&candidate) {
                    continue;
                }
                // Keep the service corridor simple. A shortest connector that doubles back
                // through an earlier stop makes node-based stop distances ambiguous and can
                // leave residents queued on a direction the buses never serve.
                let connector = self.route_avoiding(last, candidate, false, &visited);
                if connector.len() < 2 || connector.len() > MAX_BUS_ROUTE_NODES {
                    continue;
                }
                let Some(a) = self.node(last) else { continue };
                let Some(b) = self.node(candidate) else {
                    continue;
                };
                let separation = distance(a, b);
                if separation < 42.0 {
                    continue;
                }
                if route
                    .len()
                    .saturating_add(connector.len().saturating_sub(1))
                    > MAX_BUS_ROUTE_NODES
                {
                    continue;
                }
                let mut proposed_route = route.clone();
                proposed_route.extend(connector.iter().copied().skip(1));
                let mut proposed_stops = stops.clone();
                proposed_stops.push(candidate);
                let added_length: f32 = self.route_lengths(&connector).iter().sum();
                let score = route_score(&proposed_stops, current_length + added_length);
                let is_better = best.as_ref().is_none_or(|(best_score, best_stop, _, _)| {
                    score.0 > best_score.0
                        || (score.0 == best_score.0 && score.1 > best_score.1)
                        || (score.0 == best_score.0
                            && score.1 == best_score.1
                            && (score.2 < best_score.2
                                || (score.2 == best_score.2 && candidate < *best_stop)))
                });
                if is_better {
                    best = Some((score, candidate, connector, proposed_route));
                }
            }
            let Some((_score, stop, _connector, proposed_route)) = best else {
                break;
            };
            route = proposed_route;
            stops.push(stop);
            remaining.retain(|candidate| *candidate != stop);
        }
        if stops.len() < 3
            || route.len() > MAX_BUS_ROUTE_NODES
            || route_score(&stops, self.route_lengths(&route).iter().sum()).0 == 0
        {
            return;
        }
        let line_id = self.id();
        self.state.bus_line = Some(BusLine {
            id: line_id,
            stops: stops.clone(),
            route,
            active: true,
            graph_revision: open_roads as u64,
        });
        let bus_id = self.id();
        self.state.buses.push(Bus {
            id: bus_id,
            line_id,
            progress: 0.0,
            direction: 1,
            dwell_ticks: BUS_DWELL_TICKS,
            passengers: Vec::new(),
            next_departure_tick: self.state.tick.saturating_add(BUS_HEADWAY_TICKS),
            last_stop: stops.first().copied(),
        });
        self.record(
            "BusServiceStarted",
            bus_id,
            format!(
                "Neighborhood bus service opens with {} connected stops",
                stops.len()
            ),
        );
    }

    fn node_has_open_road(&self, node_id: u32) -> bool {
        self.state.roads.iter().any(|road| {
            road.connected
                && road.stage == "open"
                && road.kind != "rail"
                && (road.from == node_id || road.to == node_id)
        })
    }

    fn bus_stop_distance(&self, stop: u32) -> Option<f32> {
        let line = self.state.bus_line.as_ref()?;
        let index = line.route.iter().position(|node| *node == stop)?;
        Some(self.route_lengths(&line.route[..=index]).iter().sum())
    }

    fn bus_waiting_count(&self) -> usize {
        self.state
            .residents
            .iter()
            .filter(|resident| {
                matches!(resident.transfer.as_str(), "walking_to_bus" | "waiting_bus")
            })
            .count()
    }

    fn bus_platform_waiting_count(&self) -> usize {
        self.state
            .residents
            .iter()
            .filter(|resident| resident.transfer == "waiting_bus")
            .count()
    }

    fn bus_queue_position(&self, node_id: u32, direction: i8, slot: u16) -> Option<WorldPoint> {
        let node = self.node(node_id)?;
        let line = self.state.bus_line.as_ref()?;
        let route_index = line
            .route
            .iter()
            .position(|candidate| *candidate == node_id)?;
        let other_id = if direction >= 0 {
            line.route.get(route_index + 1).or_else(|| {
                route_index
                    .checked_sub(1)
                    .and_then(|index| line.route.get(index))
            })
        } else {
            route_index
                .checked_sub(1)
                .and_then(|index| line.route.get(index))
                .or_else(|| line.route.get(route_index + 1))
        };
        let other_id = *other_id?;
        let other = self.node(other_id)?;
        let length = distance(node, other).max(1.0);
        let along = 3.0 + slot as f32 * 1.25;
        Some(WorldPoint {
            x: node.x + (other.x - node.x) / length * along - (other.z - node.z) / length * 6.2,
            z: node.z + (other.z - node.z) / length * along + (other.x - node.x) / length * 6.2,
        })
    }

    fn bus_stop_views(&self, line: &BusLine) -> Vec<BusStopView> {
        let mut views = Vec::with_capacity(line.stops.len() * 2);
        for stop in &line.stops {
            let Some(index) = line.route.iter().position(|node| node == stop) else {
                continue;
            };
            let Some(node) = self.node(*stop) else {
                continue;
            };
            for direction in [1_i8, -1_i8] {
                let adjacent = if direction >= 0 {
                    line.route
                        .get(index + 1)
                        .or_else(|| index.checked_sub(1).and_then(|i| line.route.get(i)))
                } else {
                    index
                        .checked_sub(1)
                        .and_then(|i| line.route.get(i))
                        .or_else(|| line.route.get(index + 1))
                };
                let Some(next_id) = adjacent else { continue };
                let Some(next) = self.node(*next_id) else {
                    continue;
                };
                let dx = next.x - node.x;
                let dz = next.z - node.z;
                let length = dx.hypot(dz).max(1.0);
                let nx = -dz / length;
                let nz = dx / length;
                let curb = 6.2;
                views.push(BusStopView {
                    node_id: *stop,
                    x: node.x + nx * curb,
                    z: node.z + nz * curb,
                    heading: dz.atan2(dx),
                    direction,
                });
            }
        }
        views
    }

    fn allocate_bus_queue_slot(&self) -> Option<u16> {
        let used: BTreeSet<u16> = self
            .state
            .residents
            .iter()
            .filter(|resident| {
                matches!(resident.transfer.as_str(), "walking_to_bus" | "waiting_bus")
            })
            .filter_map(|resident| resident.queue_slot)
            .collect();
        (0..BUS_QUEUE_CAPACITY as u16).find(|slot| !used.contains(slot))
    }

    fn schedule_bus_trip(
        &mut self,
        index: usize,
        origin: u32,
        goal: u32,
        activity: &str,
        origin_building: u32,
        destination_building: Option<u32>,
    ) -> bool {
        let Some(line) = self.state.bus_line.clone().filter(|line| line.active) else {
            return false;
        };
        if self.rail_service_quiet()
            || self.bus_waiting_count() >= BUS_QUEUE_CAPACITY
            || line.stops.len() < 3
        {
            return false;
        }
        let nearest_stop = |engine: &Self, node: u32| -> Option<(usize, u32)> {
            line.stops
                .iter()
                .enumerate()
                .filter_map(|(stop_index, stop)| {
                    let path = engine.route(node, *stop, false);
                    if path.is_empty() {
                        return None;
                    }
                    let length: f32 = engine.route_lengths(&path).iter().sum();
                    (length <= 75.0).then_some((stop_index, *stop, length))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
                .map(|(stop_index, stop, _)| (stop_index, stop))
        };
        let (Some((origin_index, origin_stop)), Some((destination_index, destination_stop))) =
            (nearest_stop(self, origin), nearest_stop(self, goal))
        else {
            return false;
        };
        if origin_index == destination_index {
            return false;
        }
        let direction = if origin_index < destination_index {
            1
        } else {
            -1
        };
        let Some(queue_slot) = self.allocate_bus_queue_slot() else {
            return false;
        };
        let route = self.route(origin, origin_stop, false);
        if route.is_empty() || route.len() > MAX_ROUTE_NODES {
            return false;
        }
        let id = self.state.residents[index].id;
        let origin_queue = self.bus_queue_position(origin_stop, direction, queue_slot);
        let route_lengths = self.route_lengths(&route);
        let mut traveler = Traveler {
            id,
            kind: 0,
            route,
            route_lengths,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 1.5,
            destination: origin_stop,
            activity: "walking_to_bus".into(),
            resident: id,
            name: self.state.residents[index].name.clone(),
            origin_building: Some(origin_building),
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        };
        traveler.origin_access =
            self.building_access_path(origin_building, &traveler.route, true, id, 0);
        if let Some(queue_position) = origin_queue {
            let end = self.route_endpoint_position(&traveler.route, true, id, 0);
            if let Some(start) = end {
                traveler.destination_access = Some(AccessPath {
                    points: vec![start, queue_position],
                });
            }
        }
        if traveler.origin_access.is_some() {
            traveler.access_phase = 0;
        }
        self.state.travelers.push(traveler);
        if let Some(resident) = self.state.residents.get_mut(index) {
            resident.transfer = "walking_to_bus".into();
            resident.activity = "walking_to_bus".into();
            resident.bus_id = None;
            resident.bus_origin_stop = Some(origin_stop);
            resident.bus_destination_stop = Some(destination_stop);
            resident.bus_stop_node = Some(origin_stop);
            resident.queue_slot = Some(queue_slot);
            if activity == "return" {
                resident.last_return_day = clock_at(self.state.tick).0;
                resident.return_tick = resident.return_tick.saturating_add(TICKS_PER_DAY);
                resident.activity = "returning_home".into();
            } else {
                resident.last_departure_day = clock_at(self.state.tick).0;
                resident.departure_tick = resident.departure_tick.saturating_add(TICKS_PER_DAY);
                resident.activity = "commuting".into();
            }
            push_history(
                &mut resident.history,
                format!(
                    "Started a bus trip toward stop {destination_stop} ({direction}) at tick {}",
                    self.state.tick
                ),
            );
        }
        self.record(
            "BusTripStarted",
            id,
            format!(
                "A resident walks from building {origin_building} toward bus stop {origin_stop}"
            ),
        );
        let _ = destination_building;
        true
    }

    fn schedule_residents(&mut self, day: u32) {
        if self.state.travelers.len() >= MAX_TRAVELERS {
            return;
        }
        let active: BTreeSet<u32> = self
            .state
            .travelers
            .iter()
            .filter(|t| t.resident != 0)
            .map(|t| t.resident)
            .collect();
        for index in 0..self.state.residents.len() {
            if self.state.travelers.len() >= MAX_TRAVELERS {
                break;
            }
            let resident = self.state.residents[index].clone();
            if active.contains(&resident.id) {
                continue;
            }
            if matches!(
                resident.transfer.as_str(),
                "walking_to_bus" | "waiting_bus" | "riding_bus" | "walking_from_bus"
            ) {
                continue;
            }
            if resident.rail_commuter {
                self.schedule_rail_resident(index, day);
                continue;
            }
            let (origin, goal, activity, destination_building) = if resident.at_work {
                if self.state.tick < resident.return_tick {
                    continue;
                }
                let Some(work_node) = resident.work_node else {
                    continue;
                };
                (
                    work_node,
                    resident.home_node,
                    "return",
                    Some(resident.home_building),
                )
            } else {
                if self.state.tick < resident.departure_tick {
                    continue;
                }
                let Some(work_node) = resident.work_node else {
                    continue;
                };
                let activity = if resident.routine == "worker" {
                    if resident.departure_tick % TICKS_PER_DAY > TICKS_PER_DAY / 2 {
                        "night_shift"
                    } else {
                        "commute"
                    }
                } else {
                    "stroll"
                };
                (
                    resident.home_node,
                    work_node,
                    activity,
                    resident.work_building,
                )
            };
            let origin_building = if resident.at_work {
                resident.work_building.unwrap_or(resident.home_building)
            } else {
                resident.home_building
            };
            if self.schedule_bus_trip(
                index,
                origin,
                goal,
                activity,
                origin_building,
                destination_building,
            ) {
                continue;
            }
            let route = self.route(origin, goal, false);
            if route.len() < 2 {
                continue;
            }
            let route_lengths = self.route_lengths(&route);
            let speed = if resident.mode == 1 { 7.0 } else { 1.5 };
            let origin_building = Some(origin_building);
            let mut traveler = Traveler {
                id: resident.id,
                kind: resident.mode,
                route,
                route_lengths,
                edge_index: 0,
                edge_progress: 0.0,
                speed,
                destination: goal,
                activity: activity.into(),
                resident: resident.id,
                name: resident.name.clone(),
                origin_building,
                destination_building,
                wait_ticks: 0,
                origin_access: None,
                destination_access: None,
                access_phase: 1,
                access_progress: 0.0,
            };
            if let Some(building) = origin_building {
                traveler.origin_access = self.building_access_path(
                    building,
                    &traveler.route,
                    true,
                    traveler.id,
                    traveler.kind,
                );
            }
            if let Some(building) = destination_building {
                traveler.destination_access = self.building_access_path(
                    building,
                    &traveler.route,
                    false,
                    traveler.id,
                    traveler.kind,
                );
            }
            if traveler.origin_access.is_some() {
                traveler.access_phase = 0;
            }
            self.state.travelers.push(traveler);
            if let Some(r) = self.state.residents.get_mut(index) {
                if r.at_work {
                    r.last_return_day = day;
                    r.return_tick = r.return_tick.saturating_add(TICKS_PER_DAY);
                    r.activity = "returning_home".into();
                } else {
                    r.last_departure_day = day;
                    r.departure_tick = r.departure_tick.saturating_add(TICKS_PER_DAY);
                    r.activity = if r.routine == "worker" {
                        "commuting"
                    } else {
                        "strolling"
                    }
                    .into();
                }
                push_history(
                    &mut r.history,
                    format!("Departed on {activity} at tick {}", self.state.tick),
                );
            }
            self.record(
                "ResidentTripStarted",
                resident.id,
                format!("{} begins a purposeful {activity} trip", resident.name),
            );
        }
    }

    fn rail_terminals(&self) -> Option<(u32, u32)> {
        if !self.state.station_online {
            return None;
        }
        let rail = self
            .state
            .roads
            .iter()
            .find(|road| road.kind == "rail" && road.stage == "open" && road.connected)?;
        let station = [rail.from, rail.to]
            .into_iter()
            .find(|id| self.node(*id).is_some_and(|node| node.kind == "station"))?;
        let gateway = if station == rail.from {
            rail.to
        } else {
            rail.from
        };
        Some((station, gateway))
    }

    fn rail_service_quiet(&self) -> bool {
        let (_, minute) = clock_at(self.state.tick);
        (60.0..300.0).contains(&minute)
    }

    fn rail_waiting_count(&self, at_station: bool) -> usize {
        let state = if at_station {
            "waiting_station"
        } else {
            "waiting_gateway"
        };
        self.state
            .residents
            .iter()
            .filter(|resident| resident.rail_commuter && resident.transfer == state)
            .count()
    }

    fn rail_platform_demand(&self, at_station: bool) -> usize {
        let waiting = self.rail_waiting_count(at_station);
        let walking = if at_station {
            "walking_to_station"
        } else {
            // Return commuters join the gateway queue directly when their shift ends.
            ""
        };
        waiting
            + self
                .state
                .travelers
                .iter()
                .filter(|traveler| traveler.activity == walking && !walking.is_empty())
                .count()
    }

    fn schedule_rail_resident(&mut self, index: usize, day: u32) {
        let Some((station, _gateway)) = self.rail_terminals() else {
            return;
        };
        let resident = self.state.residents[index].clone();
        let quiet = self.rail_service_quiet();
        match resident.transfer.as_str() {
            "home" if !resident.at_work && self.state.tick >= resident.departure_tick && !quiet => {
                if self.rail_platform_demand(true) >= RAIL_QUEUE_CAPACITY {
                    return;
                }
                let Some(queue_slot) = self.allocate_queue_slot(true) else {
                    return;
                };
                let route = self.route(resident.home_node, station, false);
                if route.is_empty() {
                    return;
                }
                if let Some(current) = self.state.residents.get_mut(index) {
                    current.queue_slot = Some(queue_slot);
                }
                if route.len() == 1 {
                    if let Some(current) = self.state.residents.get_mut(index) {
                        current.transfer = "waiting_station".into();
                        current.activity = "waiting_for_train".into();
                    }
                } else {
                    let route_lengths = self.route_lengths(&route);
                    let mut traveler = Traveler {
                        id: resident.id,
                        kind: 0,
                        route,
                        route_lengths,
                        edge_index: 0,
                        edge_progress: 0.0,
                        speed: 1.5,
                        destination: station,
                        activity: "walking_to_station".into(),
                        resident: resident.id,
                        name: resident.name.clone(),
                        origin_building: Some(resident.home_building),
                        destination_building: None,
                        wait_ticks: 0,
                        origin_access: None,
                        destination_access: None,
                        access_phase: 1,
                        access_progress: 0.0,
                    };
                    traveler.origin_access = self.building_access_path(
                        resident.home_building,
                        &traveler.route,
                        true,
                        traveler.id,
                        0,
                    );
                    traveler.destination_access =
                        self.platform_entry_path(&traveler.route, true, queue_slot, traveler.id);
                    if traveler.origin_access.is_some() {
                        traveler.access_phase = 0;
                    }
                    self.state.travelers.push(traveler);
                    if let Some(current) = self.state.residents.get_mut(index) {
                        current.transfer = "walking_to_station".into();
                        current.activity = "walking_to_station".into();
                    }
                }
                if let Some(current) = self.state.residents.get_mut(index) {
                    current.last_departure_day = day;
                    current.departure_tick = current.departure_tick.saturating_add(TICKS_PER_DAY);
                    push_history(
                        &mut current.history,
                        format!("Started walking to the station at tick {}", self.state.tick),
                    );
                }
                self.record(
                    "RailTripStarted",
                    resident.id,
                    format!(
                        "{} begins a rail commute toward the regional gateway",
                        resident.name
                    ),
                );
            }
            "working_external"
                if resident.at_work && self.state.tick >= resident.return_tick && !quiet =>
            {
                let Some(queue_slot) = self.allocate_queue_slot(false) else {
                    return;
                };
                if let Some(current) = self.state.residents.get_mut(index) {
                    current.transfer = "waiting_gateway".into();
                    current.activity = "waiting_for_train".into();
                    current.queue_slot = Some(queue_slot);
                    current.last_return_day = day;
                    current.return_tick = current.return_tick.saturating_add(TICKS_PER_DAY);
                    push_history(
                        &mut current.history,
                        format!(
                            "Finished an external shift and queued for the return train at tick {}",
                            self.state.tick
                        ),
                    );
                }
                self.record(
                    "RailReturnQueued",
                    resident.id,
                    format!(
                        "{} waits at the regional gateway for the return shuttle",
                        resident.name
                    ),
                );
            }
            _ => {}
        }
    }

    fn schedule_deliveries(&mut self) {
        if self.state.tick < self.state.next_delivery_tick {
            return;
        }
        self.state.next_delivery_tick = self
            .state
            .next_delivery_tick
            .saturating_add(TICKS_PER_DAY / 7);
        let active_deliveries = self
            .state
            .travelers
            .iter()
            .filter(|t| t.activity.starts_with("delivery"))
            .count();
        if active_deliveries >= MAX_ACTIVE_DELIVERIES || self.state.travelers.len() >= MAX_TRAVELERS
        {
            return;
        }
        let Some(gateway) = self
            .state
            .nodes
            .iter()
            .find(|n| n.kind == "gateway")
            .map(|n| n.id)
        else {
            return;
        };
        let businesses: Vec<(u32, u32, String)> = self
            .state
            .buildings
            .iter()
            .filter(|b| matches!(b.kind.as_str(), "shop" | "cafe") && site_active(b))
            .filter_map(|b| {
                self.state
                    .parcels
                    .iter()
                    .find(|p| p.id == b.parcel_id)
                    .map(|p| (b.id, p.access_node, b.name.clone()))
            })
            .collect();
        if businesses.is_empty() {
            return;
        }
        let (building_id, destination, shop_name) =
            businesses[self.state.delivery_cursor % businesses.len()].clone();
        let route = self.route(gateway, destination, false);
        if route.len() < 2 {
            return;
        }
        let route_lengths = self.route_lengths(&route);
        let id = self.id();
        self.state.delivery_cursor = self.state.delivery_cursor.saturating_add(1);
        let mut traveler = Traveler {
            id,
            kind: 1,
            route,
            route_lengths,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 9.0,
            destination,
            activity: "delivery".into(),
            resident: 0,
            name: "Neighborhood Supply Van".into(),
            origin_building: None,
            destination_building: Some(building_id),
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        };
        traveler.destination_access = self.building_access_path(
            building_id,
            &traveler.route,
            false,
            traveler.id,
            traveler.kind,
        );
        self.state.travelers.push(traveler);
        self.record(
            "DeliveryDispatched",
            id,
            format!("A delivery van leaves the gateway for {shop_name}"),
        );
    }

    fn route_lengths(&self, route: &[u32]) -> Vec<f32> {
        route
            .windows(2)
            .map(|edge| match (self.node(edge[0]), self.node(edge[1])) {
                (Some(a), Some(b)) => distance(a, b).max(1.0),
                _ => 1.0,
            })
            .collect()
    }
    fn route(&self, start: u32, goal: u32, rail: bool) -> Vec<u32> {
        self.route_avoiding(start, goal, rail, &BTreeSet::new())
    }
    fn route_avoiding(
        &self,
        start: u32,
        goal: u32,
        rail: bool,
        blocked: &BTreeSet<u32>,
    ) -> Vec<u32> {
        if start == goal {
            return vec![start];
        }
        // Keep source order, including parallel edges, so equal-cost choices
        // match the full-road scan. This index lives only for this immutable
        // query; road openings and splits cannot leave a stale route cache.
        let mut adjacency = Vec::with_capacity(self.state.roads.len() * 2);
        for (index, road) in self.state.roads.iter().enumerate().filter(|(_, road)| {
            road.connected
                && road.stage == "open"
                && ((rail && road.kind == "rail") || (!rail && road.kind != "rail"))
        }) {
            adjacency.push((road.from, index));
            if road.to != road.from {
                adjacency.push((road.to, index));
            }
        }
        adjacency.sort_unstable();
        let mut dist: BTreeMap<u32, f32> = BTreeMap::new();
        let mut prev: BTreeMap<u32, u32> = BTreeMap::new();
        let mut open: BTreeSet<(u32, u32)> = BTreeSet::new();
        dist.insert(start, 0.0);
        open.insert((0, start));
        while let Some((cost_bits, current)) = open.iter().next().copied() {
            open.remove(&(cost_bits, current));
            if current == goal {
                break;
            }
            let base = *dist.get(&current).unwrap_or(&f32::INFINITY);
            let first = adjacency.partition_point(|(node, _)| *node < current);
            for &(_, index) in adjacency[first..].iter().take_while(|(node, _)| *node == current) {
                let r = &self.state.roads[index];
                let next = if r.from == current { r.to } else { r.from };
                if blocked.contains(&next) && next != goal {
                    continue;
                }
                let candidate = base + r.length;
                if candidate < *dist.get(&next).unwrap_or(&f32::INFINITY) {
                    dist.insert(next, candidate);
                    prev.insert(next, current);
                    open.insert((candidate.to_bits(), next));
                }
            }
        }
        if !prev.contains_key(&goal) {
            return vec![];
        }
        let mut path = vec![goal];
        let mut cur = goal;
        while cur != start && path.len() < MAX_ROUTE_NODES {
            let Some(p) = prev.get(&cur).copied() else {
                return vec![];
            };
            cur = p;
            path.push(cur);
        }
        // A capped suffix does not start at the requested origin. Consumers
        // interpret any nonempty path as a usable trip, so fail closed instead
        // of spawning an actor partway along an oversized journey.
        if cur != start {
            return vec![];
        }
        path.reverse();
        path
    }
    fn advance_travelers(&mut self) {
        if self.state.travelers.is_empty() {
            return;
        }
        let count = self.state.travelers.len();
        self.planned_progress.resize(count, 0.0);
        self.wait_scratch.resize(count, 0);
        let mut skip_road = vec![false; count];
        let mut arrivals = Vec::new();
        for index in 0..count {
            let traveler = &mut self.state.travelers[index];
            match traveler.access_phase {
                0 => {
                    skip_road[index] = true;
                    if let Some(path) = &traveler.origin_access {
                        traveler.access_progress += traveler.speed / TICKS_PER_SECOND as f32;
                        if traveler.access_progress >= path.length() {
                            traveler.access_phase = 1;
                            traveler.access_progress = 0.0;
                        }
                    } else {
                        traveler.access_phase = 1;
                        traveler.access_progress = 0.0;
                    }
                }
                2 => {
                    skip_road[index] = true;
                    if let Some(path) = &traveler.destination_access {
                        traveler.access_progress += traveler.speed / TICKS_PER_SECOND as f32;
                        if traveler.access_progress >= path.length() {
                            arrivals.push(traveler.clone());
                        }
                    } else {
                        arrivals.push(traveler.clone());
                    }
                }
                _ => {
                    traveler.access_phase = 1;
                    if traveler.edge_index >= traveler.route_lengths.len() {
                        skip_road[index] = true;
                        if traveler.destination_access.is_some() {
                            traveler.access_phase = 2;
                            traveler.access_progress = 0.0;
                        } else {
                            arrivals.push(traveler.clone());
                        }
                    }
                }
            }
            self.planned_progress[index] = traveler.edge_progress;
            self.wait_scratch[index] = traveler.wait_ticks;
        }
        for index in 0..count {
            let t = &self.state.travelers[index];
            if skip_road[index] {
                continue;
            }
            if t.edge_index >= t.route_lengths.len() {
                self.planned_progress[index] = t.edge_progress;
                self.wait_scratch[index] = t.wait_ticks;
                continue;
            }
            let to = t.route[t.edge_index + 1];
            let len = t.route_lengths[t.edge_index].max(1.0);
            let desired = t.edge_progress + t.speed / TICKS_PER_SECOND as f32;
            let mut target = desired;
            let mut red = false;
            if desired >= len - 3.6 {
                red = Self::junction_red_state(&self.state, t, to);
                if red {
                    target = target.min(len - 3.6).max(t.edge_progress);
                }
            }
            if red {
                self.wait_scratch[index] = t.wait_ticks.saturating_add(1);
            } else {
                self.wait_scratch[index] = 0;
            }
            self.planned_progress[index] = target;
        }
        for index in 0..count {
            self.state.travelers[index].wait_ticks = self.wait_scratch[index];
        }
        // Deterministic car-following queues by directed lane. Followers slow and stop
        // behind the leader before reaching the intersection approach.
        if self.state.tick % 4 == 0 {
            self.lane_order.clear();
            self.lane_order.extend((0..count).filter(|index| {
                !skip_road[*index]
                    && self.state.travelers[*index].access_phase == 1
                    && self.state.travelers[*index].edge_index
                        < self.state.travelers[*index].route_lengths.len()
            }));
            self.lane_order.sort_by(|&a, &b| {
                let ta = &self.state.travelers[a];
                let tb = &self.state.travelers[b];
                let ka = ta
                    .route
                    .get(ta.edge_index..=ta.edge_index + 1)
                    .map(|r| lane_key(ta, r[0], r[1]))
                    .unwrap_or((u32::MAX, u32::MAX, u8::MAX, u8::MAX));
                let kb = tb
                    .route
                    .get(tb.edge_index..=tb.edge_index + 1)
                    .map(|r| lane_key(tb, r[0], r[1]))
                    .unwrap_or((u32::MAX, u32::MAX, u8::MAX, u8::MAX));
                ka.cmp(&kb)
                    .then_with(|| tb.edge_progress.total_cmp(&ta.edge_progress))
                    .then_with(|| ta.id.cmp(&tb.id))
            });
            let mut start = 0;
            while start < self.lane_order.len() {
                let first = self.lane_order[start];
                let Some(key) = self.state.travelers[first]
                    .route
                    .get(
                        self.state.travelers[first].edge_index
                            ..=self.state.travelers[first].edge_index + 1,
                    )
                    .map(|r| lane_key(&self.state.travelers[first], r[0], r[1]))
                else {
                    start += 1;
                    continue;
                };
                let mut end = start + 1;
                while end < self.lane_order.len() {
                    let next = self.lane_order[end];
                    let next_key = self.state.travelers[next]
                        .route
                        .get(
                            self.state.travelers[next].edge_index
                                ..=self.state.travelers[next].edge_index + 1,
                        )
                        .map(|r| lane_key(&self.state.travelers[next], r[0], r[1]));
                    if next_key != Some(key) {
                        break;
                    }
                    end += 1;
                }
                let len = self.state.travelers[first].route_lengths
                    [self.state.travelers[first].edge_index]
                    .max(1.0);
                for i in start + 1..end {
                    let lead = self.lane_order[i - 1];
                    let follower = self.lane_order[i];
                    let gap = if self.state.travelers[lead].kind == 1
                        || self.state.travelers[follower].kind == 1
                    {
                        4.5
                    } else {
                        1.6
                    };
                    let safe = if self.planned_progress[lead] >= len {
                        len - gap
                    } else {
                        self.planned_progress[lead] - gap
                    };
                    self.planned_progress[follower] = self.planned_progress[follower]
                        .min(safe.max(self.state.travelers[follower].edge_progress));
                }
                start = end;
            }
        }
        // Buses are persistent vehicles outside the pedestrian/car traveler list.
        // Include them in the same directed left-lane following relationship so a
        // car cannot advance through a stopped or slow bus at a shared road edge.
        if let Some(line) = &self.state.bus_line {
            let lengths = self.route_lengths(&line.route);
            let bus_lanes: Vec<(u32, u32, f32)> = self
                .state
                .buses
                .iter()
                .filter(|bus| bus.line_id == line.id)
                .filter_map(|bus| {
                    self.bus_segment_at_progress(&line.route, &lengths, bus.progress, bus.direction)
                        .map(|(_segment, from, to, local, _length)| (from, to, local))
                })
                .collect();
            for index in 0..count {
                let traveler = &self.state.travelers[index];
                if traveler.kind != 1
                    || traveler.access_phase != 1
                    || traveler.edge_index >= traveler.route_lengths.len()
                {
                    continue;
                }
                let from = traveler.route[traveler.edge_index];
                let to = traveler.route[traveler.edge_index + 1];
                for (bus_from, bus_to, bus_local) in &bus_lanes {
                    if from != *bus_from || to != *bus_to {
                        continue;
                    }
                    let ahead = *bus_local - traveler.edge_progress;
                    if ahead > 0.0 && ahead < 12.0 {
                        let allowed = traveler.edge_progress + (ahead - 10.0).max(0.0);
                        self.planned_progress[index] = self.planned_progress[index].min(allowed);
                    }
                }
            }
        }
        let (travelers, planned) = (&mut self.state.travelers, &self.planned_progress);
        for (index, t) in travelers.iter_mut().enumerate() {
            if skip_road[index] {
                continue;
            }
            if t.edge_index >= t.route_lengths.len() {
                arrivals.push(t.clone());
                continue;
            }
            let mut remaining = (planned[index] - t.edge_progress).max(0.0);
            t.edge_progress = planned[index];
            while remaining > 0.0 && t.edge_index < t.route_lengths.len() {
                let edge_len = t.route_lengths[t.edge_index].max(1.0);
                let available = edge_len - t.edge_progress;
                if remaining + 0.0001 < available {
                    t.edge_progress += remaining;
                    break;
                }
                remaining = (remaining - available).max(0.0);
                t.edge_index += 1;
                t.edge_progress = 0.0;
            }
            if t.edge_index >= t.route_lengths.len() {
                if t.destination_access.is_some() {
                    t.access_phase = 2;
                    t.access_progress = 0.0;
                } else {
                    arrivals.push(t.clone());
                }
            }
        }
        let arrived_ids: BTreeSet<u32> = arrivals.iter().map(|t| t.id).collect();
        self.state
            .travelers
            .retain(|t| !arrived_ids.contains(&t.id));
        let mut return_vehicles = Vec::new();
        for arrived in arrivals {
            if arrived.resident != 0 {
                if let Some(resident) = self
                    .state
                    .residents
                    .iter_mut()
                    .find(|r| r.id == arrived.resident)
                {
                    match arrived.activity.as_str() {
                        "walking_to_bus" => {
                            resident.transfer = "waiting_bus".into();
                            resident.activity = "waiting_for_bus".into();
                            resident.bus_stop_node = resident.bus_origin_stop;
                        }
                        "commute" if resident.transfer == "walking_from_bus" => {
                            resident.at_work = true;
                            resident.activity = "at_destination".into();
                            resident.transfer.clear();
                            resident.bus_id = None;
                            resident.bus_origin_stop = None;
                            resident.bus_destination_stop = None;
                            resident.bus_stop_node = None;
                            resident.queue_slot = None;
                        }
                        "return" if resident.transfer == "walking_from_bus" => {
                            resident.at_work = false;
                            resident.activity = "at_home".into();
                            resident.transfer.clear();
                            resident.bus_id = None;
                            resident.bus_origin_stop = None;
                            resident.bus_destination_stop = None;
                            resident.bus_stop_node = None;
                            resident.queue_slot = None;
                        }
                        "walking_to_station" => {
                            resident.transfer = "waiting_station".into();
                            resident.activity = "waiting_for_train".into();
                            resident.at_work = false;
                        }
                        "walking_home" => {
                            resident.transfer = "home".into();
                            resident.activity = "at_home".into();
                            resident.at_work = false;
                            resident.train_id = None;
                            resident.queue_slot = None;
                        }
                        _ => {
                            resident.at_work = arrived.activity != "return";
                            resident.activity = if resident.at_work {
                                "at_destination"
                            } else {
                                "at_home"
                            }
                            .into();
                        }
                    }
                    push_history(
                        &mut resident.history,
                        format!(
                            "Arrived after {} at tick {}",
                            arrived.activity, self.state.tick
                        ),
                    );
                }
                self.record(
                    "ResidentTripArrived",
                    arrived.id,
                    format!("{} arrives after {}", arrived.name, arrived.activity),
                );
            } else if arrived.activity == "delivery" {
                let mut returning = arrived.clone();
                returning.route.reverse();
                returning.route_lengths.reverse();
                returning.edge_index = 0;
                returning.edge_progress = 0.0;
                returning.destination = *returning.route.last().unwrap_or(&returning.destination);
                returning.origin_building = arrived.destination_building;
                returning.destination_building = None;
                returning.activity = "delivery_return".into();
                returning.wait_ticks = 0;
                returning.origin_access = arrived
                    .destination_access
                    .as_ref()
                    .map(AccessPath::reversed);
                returning.destination_access = None;
                returning.access_phase = if returning.origin_access.is_some() {
                    0
                } else {
                    1
                };
                returning.access_progress = 0.0;
                return_vehicles.push(returning);
                self.record(
                    "DeliveryArrived",
                    arrived.id,
                    format!(
                        "A supply van reaches {}",
                        arrived.destination_building.unwrap_or(0)
                    ),
                );
            } else if arrived.activity == "delivery_return" {
                self.record(
                    "DeliveryReturned",
                    arrived.id,
                    "A supply van returns to the regional gateway".into(),
                );
            }
        }
        self.state.travelers.extend(return_vehicles);
    }

    fn signalized_state(state: &WorldState, node_id: u32) -> bool {
        state
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .is_some_and(|n| matches!(n.kind.as_str(), "junction" | "market" | "station"))
    }

    fn junction_red_state(state: &WorldState, traveler: &Traveler, node_id: u32) -> bool {
        if !Self::signalized_state(state, node_id) {
            return false;
        }
        if traveler.edge_index + 2 >= traveler.route.len() {
            return false;
        }
        let from = traveler.route[traveler.edge_index];
        let next = traveler.route[traveler.edge_index + 2];
        let (Some(a), Some(b)) = (
            state.nodes.iter().find(|n| n.id == from),
            state.nodes.iter().find(|n| n.id == node_id),
        ) else {
            return false;
        };
        let (Some(c), Some(d)) = (
            state.nodes.iter().find(|n| n.id == node_id),
            state.nodes.iter().find(|n| n.id == next),
        ) else {
            return false;
        };
        if traveler.kind == 0 {
            let phase = (state.tick + node_id as u64 * 19) % 240;
            return phase >= 80;
        }
        let dx = d.x - c.x;
        let dz = d.z - c.z;
        let horizontal = dx.abs() >= dz.abs();
        let approach = (state.tick + node_id as u64 * 37 + from as u64 * 11) % 240;
        let aligned = if horizontal {
            approach < 120
        } else {
            approach >= 120
        };
        let _incoming_heading = (b.z - a.z).atan2(b.x - a.x);
        !aligned
    }
    fn advance_buses(&mut self) {
        let Some(line) = self.state.bus_line.clone().filter(|line| line.active) else {
            return;
        };
        if line.route.len() < 2 || line.route.len() > MAX_BUS_ROUTE_NODES {
            return;
        }
        let lengths = self.route_lengths(&line.route);
        let route_length: f32 = lengths.iter().sum();
        if route_length <= 0.0 {
            return;
        }
        let stop_distances: Vec<(u32, f32)> = line
            .stops
            .iter()
            .filter_map(|stop| {
                self.bus_stop_distance(*stop)
                    .map(|progress| (*stop, progress))
            })
            .collect();
        let quiet = self.rail_service_quiet();
        for bus_index in 0..self.state.buses.len() {
            if self.state.buses[bus_index].dwell_ticks > 0 {
                let at_terminal = self.state.buses[bus_index].progress <= 0.01
                    || self.state.buses[bus_index].progress >= route_length - 0.01;
                let stop = self.state.buses[bus_index].last_stop.or_else(|| {
                    self.stop_at_progress(&stop_distances, self.state.buses[bus_index].progress)
                });
                if let Some(stop) = stop {
                    self.handle_bus_stop(bus_index, stop, &line, quiet);
                }
                if (quiet && self.state.buses[bus_index].passengers.is_empty())
                    || (at_terminal
                        && self.state.tick < self.state.buses[bus_index].next_departure_tick)
                {
                    self.state.buses[bus_index].dwell_ticks = 1;
                } else {
                    self.state.buses[bus_index].dwell_ticks =
                        self.state.buses[bus_index].dwell_ticks.saturating_sub(1);
                }
                continue;
            }
            if quiet && self.state.buses[bus_index].passengers.is_empty() {
                // Empty buses wait at the next stop during the quiet service window.
            }
            let direction = self.state.buses[bus_index].direction;
            let progress = self.state.buses[bus_index].progress;
            let next_stop = Self::next_bus_stop(
                &stop_distances,
                direction,
                progress,
                route_length,
                line.stops.first().copied().unwrap_or(0),
                line.stops.last().copied().unwrap_or(0),
            );
            let speed = 7.5_f32 / TICKS_PER_SECOND as f32;
            let desired = (progress + speed * direction as f32).clamp(0.0, route_length);
            let mut target = desired;
            if let Some((segment, from, to, edge_progress, edge_length)) =
                self.bus_segment_at_progress(&line.route, &lengths, progress, direction)
            {
                if edge_progress >= edge_length - 3.6 {
                    let signalized = Self::signalized_state(&self.state, to);
                    let outgoing_index = if direction >= 0 {
                        segment
                            .checked_add(2)
                            .filter(|index| *index < line.route.len())
                    } else {
                        segment.checked_sub(1)
                    };
                    let at_junction = outgoing_index.is_some();
                    if signalized && at_junction {
                        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                            continue;
                        };
                        let (Some(c), Some(d)) = (
                            self.node(to),
                            self.node(line.route[outgoing_index.unwrap()]),
                        ) else {
                            continue;
                        };
                        let dx = d.x - c.x;
                        let dz = d.z - c.z;
                        let horizontal = dx.abs() >= dz.abs();
                        let phase = (self.state.tick + to as u64 * 37 + from as u64 * 11) % 240;
                        let aligned = if horizontal {
                            phase < 120
                        } else {
                            phase >= 120
                        };
                        let _heading = (b.z - a.z).atan2(b.x - a.x);
                        if !aligned {
                            let stop_progress = if direction >= 0 {
                                progress - edge_progress + edge_length - 3.6
                            } else {
                                progress - (edge_length - edge_progress) + 3.6
                            };
                            target = if direction >= 0 {
                                target.min(progress.max(stop_progress))
                            } else {
                                target.max(progress.min(stop_progress))
                            };
                        }
                    }
                }
                for car in self.state.travelers.iter().filter(|traveler| {
                    traveler.kind == 1
                        && traveler.access_phase == 1
                        && traveler.edge_index < traveler.route_lengths.len()
                        && traveler.route[traveler.edge_index] == from
                        && traveler.route[traveler.edge_index + 1] == to
                }) {
                    let ahead = car.edge_progress - edge_progress;
                    if ahead > 0.0 && ahead < 14.0 {
                        let travel_room = (ahead - 11.0).max(0.0);
                        target = if direction >= 0 {
                            target.min(progress + travel_room)
                        } else {
                            target.max(progress - travel_room)
                        };
                    }
                }
            }
            // Maintain a deterministic gap when buses travel in the same direction.
            for other_index in 0..self.state.buses.len() {
                if other_index == bus_index
                    || self.state.buses[other_index].direction != direction
                    || self.state.buses[other_index].dwell_ticks > 0
                {
                    continue;
                }
                let other = self.state.buses[other_index].progress;
                let gap = if direction > 0 {
                    other - progress
                } else {
                    progress - other
                };
                if gap > 0.0 && gap < 24.0 {
                    target = if direction > 0 {
                        target.min((other - 24.0).max(progress))
                    } else {
                        target.max((other + 24.0).min(progress))
                    };
                }
            }
            let stop_crossed = if direction > 0 {
                next_stop.1 <= target + 0.0001 && next_stop.1 > progress
            } else {
                next_stop.1 >= target - 0.0001 && next_stop.1 < progress
            };
            if stop_crossed {
                target = next_stop.1;
                let bus = &mut self.state.buses[bus_index];
                bus.progress = target;
                bus.last_stop = Some(next_stop.0);
                bus.dwell_ticks = BUS_DWELL_TICKS;
                if target <= 0.01 || target >= route_length - 0.01 {
                    bus.direction = -bus.direction;
                    bus.next_departure_tick = self.state.tick.saturating_add(BUS_HEADWAY_TICKS);
                }
                self.record(
                    "BusStop",
                    self.state.buses[bus_index].id,
                    format!(
                        "Bus {} arrives at stop {}",
                        self.state.buses[bus_index].id, next_stop.0
                    ),
                );
                self.handle_bus_stop(bus_index, next_stop.0, &line, quiet);
            } else {
                self.state.buses[bus_index].progress = target;
            }
        }
    }

    fn stop_at_progress(&self, stops: &[(u32, f32)], progress: f32) -> Option<u32> {
        stops
            .iter()
            .find(|(_, distance)| (*distance - progress).abs() < 0.75)
            .map(|(stop, _)| *stop)
    }

    fn next_bus_stop(
        stops: &[(u32, f32)],
        direction: i8,
        progress: f32,
        route_length: f32,
        first_stop: u32,
        last_stop: u32,
    ) -> (u32, f32) {
        // Exclude only the stop the bus is exactly dwelling at. Any positive
        // tolerance can skip a stop when the remaining distance falls below
        // that tolerance before the bus reaches it.
        if direction >= 0 {
            stops
                .iter()
                .filter(|(_, distance)| *distance > progress)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .copied()
                .unwrap_or((last_stop, route_length))
        } else {
            stops
                .iter()
                .filter(|(_, distance)| *distance < progress)
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .copied()
                .unwrap_or((first_stop, 0.0))
        }
    }

    fn bus_segment_at_progress(
        &self,
        route: &[u32],
        lengths: &[f32],
        progress: f32,
        direction: i8,
    ) -> Option<(usize, u32, u32, f32, f32)> {
        let mut offset = 0.0;
        for (index, length) in lengths.iter().copied().enumerate() {
            let at_or_before_edge_end = if direction >= 0 {
                progress < offset + length - 0.001
            } else {
                progress <= offset + length + 0.001
            };
            if at_or_before_edge_end || index + 1 == lengths.len() {
                if direction >= 0 {
                    return Some((
                        index,
                        route[index],
                        route[index + 1],
                        progress - offset,
                        length,
                    ));
                }
                return Some((
                    index,
                    route[index + 1],
                    route[index],
                    offset + length - progress,
                    length,
                ));
            }
            offset += length;
        }
        None
    }

    fn handle_bus_stop(&mut self, bus_index: usize, stop: u32, line: &BusLine, quiet: bool) {
        let bus_id = self.state.buses[bus_index].id;
        let mut alight: Vec<u32> = self.state.buses[bus_index]
            .passengers
            .iter()
            .copied()
            .filter(|resident_id| {
                self.state
                    .residents
                    .iter()
                    .find(|resident| resident.id == *resident_id)
                    .is_some_and(|resident| resident.bus_destination_stop == Some(stop))
            })
            .collect();
        alight.sort_unstable();
        for resident_id in alight {
            self.alight_bus_passenger(bus_index, resident_id, stop);
        }
        if quiet || self.state.buses[bus_index].passengers.len() >= BUS_CAPACITY {
            return;
        }
        let direction = self.state.buses[bus_index].direction;
        let stop_index = line.stops.iter().position(|candidate| *candidate == stop);
        let mut waiting: Vec<u32> =
            self.state
                .residents
                .iter()
                .filter(|resident| {
                    resident.transfer == "waiting_bus" && resident.bus_origin_stop == Some(stop)
                })
                .filter(|resident| {
                    let Some(origin) = stop_index else {
                        return false;
                    };
                    let Some(destination) = resident.bus_destination_stop.and_then(|node| {
                        line.stops.iter().position(|candidate| *candidate == node)
                    }) else {
                        return false;
                    };
                    if direction >= 0 {
                        destination > origin
                    } else {
                        destination < origin
                    }
                })
                .map(|resident| resident.id)
                .collect();
        waiting.sort_unstable();
        for resident_id in waiting {
            if self.state.buses[bus_index].passengers.len() >= BUS_CAPACITY {
                break;
            }
            self.state.buses[bus_index].passengers.push(resident_id);
            if let Some(resident) = self
                .state
                .residents
                .iter_mut()
                .find(|resident| resident.id == resident_id)
            {
                resident.transfer = "riding_bus".into();
                resident.activity = "aboard_bus".into();
                resident.bus_id = Some(bus_id);
                resident.bus_stop_node = Some(stop);
                resident.queue_slot = None;
                push_history(
                    &mut resident.history,
                    format!(
                        "Boarded bus {bus_id} at stop {stop} at tick {}",
                        self.state.tick
                    ),
                );
            }
            self.record(
                "BusPassengerBoarded",
                resident_id,
                format!("A resident boards bus {bus_id} at stop {stop}"),
            );
        }
    }

    fn alight_bus_passenger(&mut self, bus_index: usize, resident_id: u32, stop: u32) {
        let Some(resident_index) = self
            .state
            .residents
            .iter()
            .position(|resident| resident.id == resident_id)
        else {
            return;
        };
        let resident = self.state.residents[resident_index].clone();
        let target_node = if resident.at_work {
            resident.home_node
        } else {
            resident.work_node.unwrap_or(resident.home_node)
        };
        let destination_building = if resident.at_work {
            resident.home_building
        } else {
            resident.work_building.unwrap_or(resident.home_building)
        };
        let route = self.route(stop, target_node, false);
        if route.is_empty() || route.len() > MAX_ROUTE_NODES {
            return;
        }
        let mut traveler = Traveler {
            id: resident.id,
            kind: 0,
            route_lengths: self.route_lengths(&route),
            route,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 1.5,
            destination: target_node,
            activity: if resident.at_work {
                "return"
            } else {
                "commute"
            }
            .into(),
            resident: resident.id,
            name: resident.name.clone(),
            origin_building: None,
            destination_building: Some(destination_building),
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        };
        if let Some(slot) = resident.queue_slot.or(Some(0)) {
            if let Some(queue) =
                self.bus_queue_position(stop, self.state.buses[bus_index].direction, slot)
            {
                if let Some(endpoint) =
                    self.route_endpoint_position(&traveler.route, false, resident.id, 0)
                {
                    traveler.origin_access = Some(AccessPath {
                        points: vec![queue, endpoint],
                    });
                }
            }
        }
        traveler.destination_access =
            self.building_access_path(destination_building, &traveler.route, false, resident.id, 0);
        if traveler.origin_access.is_some() {
            traveler.access_phase = 0;
        }
        let bus_id = self.state.buses[bus_index].id;
        self.state.buses[bus_index]
            .passengers
            .retain(|id| *id != resident_id);
        if let Some(resident) = self.state.residents.get_mut(resident_index) {
            resident.transfer = "walking_from_bus".into();
            resident.activity = if resident.at_work {
                "walking_home_from_bus"
            } else {
                "walking_to_work_from_bus"
            }
            .into();
            resident.bus_id = None;
            resident.bus_stop_node = Some(stop);
            resident.queue_slot = None;
            push_history(
                &mut resident.history,
                format!(
                    "Alighted from bus {bus_id} at stop {stop} at tick {}",
                    self.state.tick
                ),
            );
        }
        self.state.travelers.push(traveler);
        self.record(
            "BusPassengerAlighted",
            resident_id,
            format!("A resident leaves bus {bus_id} at stop {stop}"),
        );
    }

    fn advance_trains(&mut self) {
        let usable_rail = self
            .state
            .roads
            .iter()
            .find(|r| r.kind == "rail" && r.stage == "open" && r.connected)
            .cloned();
        let Some(rail) = usable_rail
            .filter(|r| self.state.station_online && r.length >= TRAIN_HALF_LENGTH * 2.0)
        else {
            self.state.trains.clear();
            return;
        };
        let station = self
            .state
            .nodes
            .iter()
            .find(|n| n.kind == "station" && (n.id == rail.from || n.id == rail.to))
            .map(|n| n.id);
        let Some(station) = station else {
            self.state.trains.clear();
            return;
        };
        let gateway = if rail.from == station {
            rail.to
        } else {
            rail.from
        };
        let route = vec![station, gateway];
        if self.state.trains.is_empty() {
            let id = self.id();
            self.state.trains.push(Train {
                id,
                route: route.clone(),
                distance: TRAIN_HALF_LENGTH,
                speed: 18.0,
                direction: 1,
                dwell_ticks: 100,
                passengers: Vec::new(),
            });
        }
        let mut stops = Vec::new();
        let quiet = self.rail_service_quiet();
        let station_queue_empty = self.rail_waiting_count(true) == 0;
        for train in &mut self.state.trains {
            if train.route != route {
                train.route = route.clone();
                train.distance = TRAIN_HALF_LENGTH;
                train.direction = 1;
                train.dwell_ticks = 100;
                train.passengers.clear();
            }
            let at_station = train.direction >= 0 && train.distance <= TRAIN_HALF_LENGTH + 0.01;
            if quiet && at_station && train.passengers.is_empty() && station_queue_empty {
                train.dwell_ticks = 1;
                continue;
            }
            if train.dwell_ticks > 0 {
                train.dwell_ticks -= 1;
                continue;
            }
            let step = train.speed / TICKS_PER_SECOND as f32;
            if train.direction >= 0 {
                train.distance += step;
                let far_terminal = rail.length - TRAIN_HALF_LENGTH;
                if train.distance >= far_terminal {
                    train.distance = far_terminal;
                    train.direction = -1;
                    train.dwell_ticks = 180;
                    stops.push((
                        train.id,
                        "TrainGatewayStop",
                        "The regional shuttle pauses at its outer terminus",
                        false,
                    ));
                }
            } else {
                train.distance -= step;
                if train.distance <= TRAIN_HALF_LENGTH {
                    train.distance = TRAIN_HALF_LENGTH;
                    train.direction = 1;
                    train.dwell_ticks = 220;
                    stops.push((
                        train.id,
                        "TrainStationStop",
                        "The shuttle pauses at the neighborhood platform",
                        true,
                    ));
                }
            }
        }
        for (id, kind, description, at_station) in stops {
            self.record(kind, id, description.into());
            self.handle_train_terminal(id, at_station, station, gateway);
        }
        let train_ids: Vec<u32> = self.state.trains.iter().map(|train| train.id).collect();
        for id in train_ids {
            self.board_waiting_train(id, station, gateway);
        }
    }

    fn handle_train_terminal(
        &mut self,
        train_id: u32,
        at_station: bool,
        station: u32,
        gateway: u32,
    ) {
        let passenger_ids = self
            .state
            .trains
            .iter()
            .find(|train| train.id == train_id)
            .map(|train| train.passengers.clone())
            .unwrap_or_default();
        let mut unloaded = BTreeSet::new();
        for resident_id in passenger_ids {
            let Some(resident_index) = self
                .state
                .residents
                .iter()
                .position(|resident| resident.id == resident_id)
            else {
                unloaded.insert(resident_id);
                continue;
            };
            let resident = self.state.residents[resident_index].clone();
            if at_station && resident.transfer == "riding_inbound" {
                let route = self.route(station, resident.home_node, false);
                if route.is_empty() {
                    continue;
                }
                if route.len() == 1 {
                    unloaded.insert(resident_id);
                    let current = &mut self.state.residents[resident_index];
                    current.transfer = "home".into();
                    current.activity = "at_home".into();
                    current.at_work = false;
                    current.train_id = None;
                    current.queue_slot = None;
                    push_history(
                        &mut current.history,
                        format!("Returned home from the gateway at tick {}", self.state.tick),
                    );
                    self.record(
                        "TrainPassengerAlighted",
                        train_id,
                        format!(
                            "{} completes the inbound trip at the station",
                            resident.name
                        ),
                    );
                } else if self.state.travelers.len() < MAX_TRAVELERS {
                    unloaded.insert(resident_id);
                    let mut traveler = Traveler {
                        id: resident.id,
                        kind: 0,
                        route_lengths: self.route_lengths(&route),
                        route,
                        edge_index: 0,
                        edge_progress: 0.0,
                        speed: 1.5,
                        destination: resident.home_node,
                        activity: "walking_home".into(),
                        resident: resident.id,
                        name: resident.name.clone(),
                        origin_building: None,
                        destination_building: Some(resident.home_building),
                        wait_ticks: 0,
                        origin_access: None,
                        destination_access: None,
                        access_phase: 1,
                        access_progress: 0.0,
                    };
                    traveler.origin_access = self.platform_exit_path(
                        &traveler.route,
                        true,
                        (resident.id % RAIL_QUEUE_CAPACITY as u32) as u16,
                        traveler.id,
                    );
                    traveler.destination_access = self.building_access_path(
                        resident.home_building,
                        &traveler.route,
                        false,
                        traveler.id,
                        0,
                    );
                    if traveler.origin_access.is_some() {
                        traveler.access_phase = 0;
                    }
                    self.state.travelers.push(traveler);
                    let current = &mut self.state.residents[resident_index];
                    current.transfer = "walking_home".into();
                    current.activity = "walking_home".into();
                    current.train_id = None;
                    current.queue_slot = None;
                    push_history(
                        &mut current.history,
                        format!(
                            "Arrived by train and started walking home at tick {}",
                            self.state.tick
                        ),
                    );
                    self.record(
                        "TrainPassengerAlighted",
                        train_id,
                        format!(
                            "{} alights at the station and continues on foot",
                            resident.name
                        ),
                    );
                }
            } else if !at_station && resident.transfer == "riding_outbound" {
                unloaded.insert(resident_id);
                let current = &mut self.state.residents[resident_index];
                current.transfer = "working_external".into();
                current.activity = "working_external".into();
                current.at_work = true;
                current.train_id = None;
                current.work_node = Some(gateway);
                push_history(
                    &mut current.history,
                    format!(
                        "Arrived at the regional gateway for work at tick {}",
                        self.state.tick
                    ),
                );
                self.record(
                    "RailWorkStarted",
                    resident_id,
                    format!("{} arrives at an external workplace by rail", resident.name),
                );
                self.record(
                    "TrainPassengerAlighted",
                    train_id,
                    format!("{} alights at the regional gateway", resident.name),
                );
            }
        }
        if !unloaded.is_empty() {
            if let Some(train) = self
                .state
                .trains
                .iter_mut()
                .find(|train| train.id == train_id)
            {
                train
                    .passengers
                    .retain(|resident_id| !unloaded.contains(resident_id));
            }
        }
    }

    fn board_waiting_train(&mut self, train_id: u32, station: u32, gateway: u32) {
        let Some((dwell_ticks, passenger_count, direction, distance)) = self
            .state
            .trains
            .iter()
            .find(|train| train.id == train_id)
            .map(|train| {
                (
                    train.dwell_ticks,
                    train.passengers.len(),
                    train.direction,
                    train.distance,
                )
            })
        else {
            return;
        };
        if dwell_ticks == 0 || passenger_count >= RAIL_TRAIN_CAPACITY {
            return;
        }
        if self.state.tick % 4 != 0 {
            return;
        }
        let rail_length = self
            .state
            .roads
            .iter()
            .find(|road| road.kind == "rail" && road.stage == "open" && road.connected)
            .map(|road| road.length)
            .unwrap_or(0.0);
        let (queue_state, riding_state) = if direction >= 0 && distance <= TRAIN_HALF_LENGTH + 0.01
        {
            ("waiting_station", "riding_outbound")
        } else if direction < 0 && distance >= rail_length - TRAIN_HALF_LENGTH - 0.01 {
            ("waiting_gateway", "riding_inbound")
        } else {
            return;
        };
        let next = self
            .state
            .residents
            .iter()
            .filter(|resident| resident.rail_commuter && resident.transfer == queue_state)
            .map(|resident| resident.id)
            .min();
        let Some(resident_id) = next else { return };
        let Some(index) = self
            .state
            .residents
            .iter()
            .position(|resident| resident.id == resident_id)
        else {
            return;
        };
        let resident_name = self.state.residents[index].name.clone();
        let current = &mut self.state.residents[index];
        current.transfer = riding_state.into();
        current.activity = "riding_train".into();
        current.train_id = Some(train_id);
        current.queue_slot = None;
        push_history(
            &mut current.history,
            format!(
                "Boarded the neighborhood shuttle at tick {}",
                self.state.tick
            ),
        );
        if let Some(train) = self
            .state
            .trains
            .iter_mut()
            .find(|train| train.id == train_id)
        {
            if train.passengers.len() < RAIL_TRAIN_CAPACITY {
                train.passengers.push(resident_id);
            }
        }
        let board_station = queue_state == "waiting_station";
        self.record(
            "TrainPassengerBoarded",
            train_id,
            format!(
                "{resident_name} boards the shuttle at node {}",
                if board_station { station } else { gateway }
            ),
        );
    }
    fn record(&mut self, kind: &str, entity_id: u32, reason: String) {
        let id = self.state.next_event_id;
        self.state.next_event_id = self.state.next_event_id.saturating_add(1);
        self.state.events.push_back(Event {
            id,
            tick: self.state.tick,
            kind: kind.into(),
            entity_id,
            reason,
        });
        while self.state.events.len() > MAX_EVENTS {
            self.state.events.pop_front();
        }
    }
    pub fn set_development_rate(&mut self, rate: u32) {
        self.state.development_rate = rate.clamp(0, 60);
    }
    pub fn tick(&self) -> u64 {
        self.state.tick
    }
    pub fn movement_buffer(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(
            (self.state.travelers.len() + self.state.trains.len() + self.state.buses.len()) * 5,
        );
        for t in &self.state.travelers {
            if t.access_phase == 0 || t.access_phase == 2 {
                let path = if t.access_phase == 0 {
                    t.origin_access.as_ref()
                } else {
                    t.destination_access.as_ref()
                };
                if let Some((point, heading)) = path.and_then(|path| path.sample(t.access_progress))
                {
                    out.extend_from_slice(&[t.id as f32, point.x, point.z, heading, t.kind as f32]);
                }
                continue;
            }
            if t.edge_index + 1 >= t.route.len() {
                continue;
            }
            let Some(a) = self.node(t.route[t.edge_index]) else {
                continue;
            };
            let Some(b) = self.node(t.route[t.edge_index + 1]) else {
                continue;
            };
            let len = distance(a, b).max(1.0);
            let f = (t.edge_progress / len).clamp(0.0, 1.0);
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let mut x = a.x + dx * f;
            let mut z = a.z + dz * f;
            let lateral = self.road_lateral_offset(t, a.id, b.id, dx, dz, len);
            x += lateral.0;
            z += lateral.1;
            out.extend_from_slice(&[
                t.id as f32,
                x,
                z,
                (b.z - a.z).atan2(b.x - a.x),
                t.kind as f32,
            ]);
        }
        if let Some((station, gateway)) = self.rail_terminals() {
            for (at_station, node_id) in [(true, station), (false, gateway)] {
                let Some(node) = self.node(node_id) else {
                    continue;
                };
                let mut waiting: Vec<&Resident> = self
                    .state
                    .residents
                    .iter()
                    .filter(|resident| {
                        resident.transfer
                            == if at_station {
                                "waiting_station"
                            } else {
                                "waiting_gateway"
                            }
                    })
                    .collect();
                waiting.sort_by_key(|resident| resident.id);
                for (slot, resident) in waiting.into_iter().enumerate() {
                    let slot = resident.queue_slot.unwrap_or(slot as u16);
                    let position = self
                        .platform_position(at_station, slot)
                        .unwrap_or(WorldPoint {
                            x: node.x,
                            z: node.z,
                        });
                    out.extend_from_slice(&[resident.id as f32, position.x, position.z, 0.0, 0.0]);
                }
            }
        }
        let mut bus_waiting: Vec<&Resident> = self
            .state
            .residents
            .iter()
            .filter(|resident| resident.transfer == "waiting_bus")
            .collect();
        bus_waiting.sort_by_key(|resident| resident.id);
        for resident in bus_waiting {
            let Some(stop) = resident.bus_stop_node else {
                continue;
            };
            let Some(slot) = resident.queue_slot else {
                continue;
            };
            let direction = self.bus_trip_direction(resident).unwrap_or(1);
            let Some(position) = self.bus_queue_position(stop, direction, slot) else {
                continue;
            };
            out.extend_from_slice(&[resident.id as f32, position.x, position.z, 0.0, 0.0]);
        }
        for t in &self.state.trains {
            if t.route.len() < 2 {
                continue;
            }
            let Some(a) = self.node(t.route[0]) else {
                continue;
            };
            let Some(b) = self.node(t.route[1]) else {
                continue;
            };
            let len = distance(a, b).max(1.0);
            let f = (t.distance / len).clamp(0.0, 1.0);
            let forward_heading = (b.z - a.z).atan2(b.x - a.x);
            let heading = if t.direction >= 0 {
                forward_heading
            } else {
                forward_heading + std::f32::consts::PI
            };
            let x = a.x + (b.x - a.x) * f;
            let z = a.z + (b.z - a.z) * f;
            out.extend_from_slice(&[t.id as f32, x, z, heading, 2.0]);
        }
        if let Some(line) = &self.state.bus_line {
            for bus in &self.state.buses {
                let Some((x, z, heading)) = self.bus_pose(line, bus) else {
                    continue;
                };
                out.extend_from_slice(&[bus.id as f32, x, z, heading, 3.0]);
            }
        }
        out
    }

    fn bus_trip_direction(&self, resident: &Resident) -> Option<i8> {
        let line = self.state.bus_line.as_ref()?;
        let origin = line
            .stops
            .iter()
            .position(|stop| Some(*stop) == resident.bus_origin_stop)?;
        let destination = line
            .stops
            .iter()
            .position(|stop| Some(*stop) == resident.bus_destination_stop)?;
        Some(if destination > origin { 1 } else { -1 })
    }

    fn bus_next_stop(&self, bus: &Bus, line: &BusLine) -> Option<u32> {
        let first = *line.stops.first()?;
        let last = *line.stops.last()?;
        let distances: Vec<(u32, f32)> = line
            .stops
            .iter()
            .filter_map(|stop| {
                self.bus_stop_distance(*stop)
                    .map(|distance| (*stop, distance))
            })
            .collect();
        Some(
            Self::next_bus_stop(
                &distances,
                bus.direction,
                bus.progress,
                self.route_lengths(&line.route).iter().sum(),
                first,
                last,
            )
            .0,
        )
    }

    fn bus_waiting_at(&self, stop: Option<u32>) -> usize {
        let Some(stop) = stop else { return 0 };
        self.state
            .residents
            .iter()
            .filter(|resident| {
                resident.transfer == "waiting_bus" && resident.bus_stop_node == Some(stop)
            })
            .count()
    }

    fn bus_pose(&self, line: &BusLine, bus: &Bus) -> Option<(f32, f32, f32)> {
        let lengths = self.route_lengths(&line.route);
        let mut offset = 0.0;
        for (index, length) in lengths.iter().copied().enumerate() {
            let at_or_before_edge_end = if bus.direction >= 0 {
                bus.progress < offset + length - 0.001
            } else {
                bus.progress <= offset + length + 0.001
            };
            if at_or_before_edge_end || index + 1 == lengths.len() {
                let a = self.node(line.route[index])?;
                let b = self.node(line.route[index + 1])?;
                let f = ((bus.progress - offset) / length.max(1.0)).clamp(0.0, 1.0);
                let mut heading = (b.z - a.z).atan2(b.x - a.x);
                if bus.direction < 0 {
                    heading += std::f32::consts::PI;
                }
                let nx = -heading.sin();
                let nz = heading.cos();
                return Some((
                    a.x + (b.x - a.x) * f + nx * 1.65,
                    a.z + (b.z - a.z) * f + nz * 1.65,
                    heading,
                ));
            }
            offset += length;
        }
        None
    }
    fn road_lateral_offset(
        &self,
        traveler: &Traveler,
        from: u32,
        to: u32,
        dx: f32,
        dz: f32,
        len: f32,
    ) -> (f32, f32) {
        let road = self.state.roads.iter().find(|r| {
            r.stage == "open"
                && r.kind != "rail"
                && ((r.from == from && r.to == to) || (r.from == to && r.to == from))
        });
        let width = road
            .map(|r| if r.kind == "arterial" { 14.0 } else { 9.0 })
            .unwrap_or(9.0);
        let lateral = if traveler.kind == 1 {
            // Left-hand traffic: driver's left is the positive left-normal of the heading.
            1.65
        } else if traveler.id % 2 == 0 {
            width * 0.5 + 0.72
        } else {
            -(width * 0.5 + 0.72)
        };
        let nx = -dz / len;
        let nz = dx / len;
        (nx * lateral, nz * lateral)
    }
    pub fn inspect_json(&self, id: u32) -> String {
        let value = if let Some(b) = self.state.buildings.iter().find(|b| b.id == id) {
            serde_json::json!({"entityType":"building","id":b.id,"name":b.name,"kind":b.kind,"stage":b.stage,"occupancy":b.occupancy,"capacity":b.capacity,"history":b.history})
        } else if let Some(r) = self.state.roads.iter().find(|r| r.id == id) {
            serde_json::json!({"entityType":"road","id":r.id,"kind":r.kind,"stage":r.stage,"connected":r.connected,"length":r.length})
        } else if let Some(t) = self.state.travelers.iter().find(|t| t.id == id) {
            let history: Vec<String> = if t.resident != 0 {
                self.state
                    .residents
                    .iter()
                    .find(|r| r.id == t.resident)
                    .map(|r| r.history.clone())
                    .unwrap_or_default()
            } else {
                self.state
                    .events
                    .iter()
                    .filter(|e| e.entity_id == id)
                    .map(|e| e.reason.clone())
                    .collect()
            };
            serde_json::json!({"entityType":"traveler","id":t.id,"name":t.name,"activity":t.activity,"kind":if t.kind == 1 {"car"} else {"pedestrian"},"origin":t.origin_building,"destination":t.destination_building,"history":history,"wait_ticks":t.wait_ticks})
        } else if let Some(r) = self.state.residents.iter().find(|r| r.id == id) {
            serde_json::json!({"entityType":"resident","id":r.id,"name":r.name,"activity":r.activity,"kind":if r.train_id.is_some() {"rail passenger"} else if r.bus_id.is_some() {"bus passenger"} else if r.mode == 1 {"car"} else {"pedestrian"},"origin":r.home_building,"destination":r.work_building,"home_building":r.home_building,"work_building":r.work_building,"transfer":r.transfer,"train_id":r.train_id,"bus_id":r.bus_id,"bus_origin_stop":r.bus_origin_stop,"bus_destination_stop":r.bus_destination_stop,"bus_stop_node":r.bus_stop_node,"location_node":self.resident_location_node(r),"history":r.history})
        } else if let Some(t) = self.state.trains.iter().find(|t| t.id == id) {
            let terminal = if t.direction >= 0 {
                *t.route.last().unwrap_or(&0)
            } else {
                *t.route.first().unwrap_or(&0)
            };
            let history: Vec<String> = self
                .state
                .events
                .iter()
                .filter(|e| e.entity_id == id)
                .map(|e| e.reason.clone())
                .collect();
            serde_json::json!({"entityType":"train","id":t.id,"name":"Neighborhood Shuttle","activity":if t.dwell_ticks > 0 {"at terminal"} else {"in transit"},"kind":"rail","origin":t.route.first(),"destination":terminal,"history":history,"direction":t.direction,"passengers":t.passengers.len(),"capacity":RAIL_TRAIN_CAPACITY,"station_waiting":self.rail_waiting_count(true),"gateway_waiting":self.rail_waiting_count(false)})
        } else if let Some(bus) = self.state.buses.iter().find(|bus| bus.id == id) {
            let line = self
                .state
                .bus_line
                .as_ref()
                .filter(|line| line.id == bus.line_id);
            let next_stop = line.and_then(|line| self.bus_next_stop(bus, line));
            let history: Vec<String> = self
                .state
                .events
                .iter()
                .filter(|event| event.entity_id == id || bus.passengers.contains(&event.entity_id))
                .map(|event| event.reason.clone())
                .collect();
            serde_json::json!({"entityType":"bus","id":bus.id,"name":"Neighborhood Bus","activity":if bus.dwell_ticks > 0 {"at stop"} else {"in service"},"kind":"bus","line_id":bus.line_id,"next_stop":next_stop,"direction":bus.direction,"passengers":bus.passengers.len(),"capacity":BUS_CAPACITY,"waiting":self.bus_waiting_at(next_stop),"history":history})
        } else if let Some(e) = self.state.events.iter().find(|e| e.id == id) {
            serde_json::json!({"entityType":"event","id":e.id,"kind":e.kind,"reason":e.reason,"tick":e.tick})
        } else {
            serde_json::json!({"entityType":"unknown","id":id})
        };
        value.to_string()
    }

    fn resident_location_node(&self, resident: &Resident) -> Option<u32> {
        if self
            .state
            .travelers
            .iter()
            .any(|traveler| traveler.resident == resident.id)
        {
            return None;
        }
        match resident.transfer.as_str() {
            "home" => Some(resident.home_node),
            "waiting_bus" => resident.bus_stop_node,
            "riding_bus" => None,
            "waiting_station" => self.rail_terminals().map(|(station, _)| station),
            "waiting_gateway" | "working_external" => resident.work_node,
            "riding_outbound" | "riding_inbound" => None,
            _ if resident.activity == "at_home" => Some(resident.home_node),
            _ if resident.activity == "at_destination" => resident.work_node,
            _ => None,
        }
    }
    pub fn snapshot_json(&self) -> String {
        serde_json::to_string(&self.snapshot()).expect("serializing snapshot cannot fail")
    }
    fn building_access_views(&self) -> Vec<BuildingAccessView> {
        self.state
            .buildings
            .iter()
            .filter(|building| building.stage != "demolished")
            .filter_map(|building| {
                let (road_id, access) = self.building_access_segment(building.id)?;
                let road = self.state.roads.iter().find(|road| road.id == road_id)?;
                if road.stage != "open" || !road.connected || road.kind == "rail" {
                    return None;
                }
                Some(BuildingAccessView {
                    building_id: building.id,
                    road_id,
                    points: [
                        Point {
                            x: access[0].x,
                            z: access[0].z,
                        },
                        Point {
                            x: access[1].x,
                            z: access[1].z,
                        },
                    ],
                })
            })
            .collect()
    }
    pub fn snapshot(&self) -> Snapshot {
        let (day, minute) = clock_at(self.state.tick);
        let road_count = self
            .state
            .roads
            .iter()
            .filter(|r| r.stage == "open")
            .count();
        let active_sites = self
            .state
            .buildings
            .iter()
            .filter(|b| site_active(b))
            .count();
        let developable_lots = self
            .state
            .parcels
            .iter()
            .filter(|p| {
                !p.protected
                    && self.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                    && self.node_connected(p.access_node)
            })
            .count();
        let infill_ratio = if developable_lots == 0 {
            0.0
        } else {
            active_sites as f32 / developable_lots as f32
        };
        let stage = if active_sites < 3 || self.state.population < 20 {
            "Founding"
        } else if road_count < 7 || self.state.population < 200 {
            "Settlement"
        } else if road_count >= 100 && active_sites >= 300 && infill_ratio >= 0.9
            || self.state.population >= 24_000
        {
            "Maturity"
        } else if !self.state.station_online || self.state.population < 1500 {
            "Neighborhoods"
        } else if self.state.population < 7000 {
            "TransitCity"
        } else {
            "Metropolis"
        };
        let river = self.river_points();
        Snapshot {
            schema_version: SCHEMA_VERSION,
            engine_version: ENGINE_VERSION.into(),
            seed: self.state.seed,
            tick: self.state.tick,
            day,
            minute_of_day: minute,
            development_rate: self.state.development_rate,
            revision: self.state.revision,
            stage: stage.into(),
            weather: Weather {
                kind: self.state.weather_kind.clone(),
                intensity: self.state.weather_intensity,
            },
            bounds: Bounds {
                min_x: MIN,
                max_x: MAX,
                min_z: MIN,
                max_z: MAX,
            },
            terrain: Terrain {
                river,
                river_half_width: RIVER_HALF_WIDTH,
                protected: vec![vec![
                    Point {
                        x: -350.0,
                        z: -350.0,
                    },
                    Point {
                        x: -260.0,
                        z: -350.0,
                    },
                    Point {
                        x: -260.0,
                        z: -270.0,
                    },
                    Point {
                        x: -350.0,
                        z: -270.0,
                    },
                ]],
                gateways: self
                    .state
                    .nodes
                    .iter()
                    .filter(|n| n.kind == "gateway")
                    .map(|n| n.id)
                    .collect(),
            },
            nodes: self.state.nodes.clone(),
            roads: self.state.roads.clone(),
            parcels: self.state.parcels.clone(),
            buildings: self.state.buildings.clone(),
            building_access: self.building_access_views(),
            projects: self.state.projects.clone(),
            trains: self
                .state
                .trains
                .iter()
                .map(|t| TrainView {
                    id: t.id,
                    route: t.route.clone(),
                    progress: t.distance,
                    direction: t.direction,
                    dwelling: t.dwell_ticks > 0,
                    passengers: t.passengers.len() as u16,
                    capacity: RAIL_TRAIN_CAPACITY as u16,
                    station_waiting: self.rail_waiting_count(true) as u16,
                    gateway_waiting: self.rail_waiting_count(false) as u16,
                })
                .collect(),
            bus_lines: self
                .state
                .bus_line
                .iter()
                .map(|line| BusLineView {
                    id: line.id,
                    stops: line.stops.clone(),
                    route: line.route.clone(),
                    active: line.active,
                    stop_positions: self.bus_stop_views(line),
                })
                .collect(),
            buses: self
                .state
                .buses
                .iter()
                .map(|bus| {
                    let line = self
                        .state
                        .bus_line
                        .as_ref()
                        .filter(|line| line.id == bus.line_id);
                    let next_stop = line.and_then(|line| self.bus_next_stop(bus, line));
                    BusView {
                        id: bus.id,
                        line_id: bus.line_id,
                        progress: bus.progress,
                        direction: bus.direction,
                        dwelling: bus.dwell_ticks > 0,
                        passengers: bus.passengers.len() as u16,
                        capacity: BUS_CAPACITY as u16,
                        next_stop,
                        waiting: self.bus_waiting_at(next_stop) as u16,
                    }
                })
                .collect(),
            residents: self
                .state
                .residents
                .iter()
                .map(|r| ResidentView {
                    id: r.id,
                    name: r.name.clone(),
                    home_building: r.home_building,
                    work_building: r.work_building,
                    activity: r.activity.clone(),
                    transfer: r.transfer.clone(),
                    train_id: r.train_id,
                    location_node: self.resident_location_node(r),
                    bus_id: r.bus_id,
                    bus_origin_stop: r.bus_origin_stop,
                    bus_destination_stop: r.bus_destination_stop,
                    bus_stop_node: r.bus_stop_node,
                })
                .collect(),
            events: self.state.events.iter().cloned().collect(),
            stats: Stats {
                population: self.state.population,
                active_travelers: (self.state.travelers.len()
                    + self.rail_waiting_count(true)
                    + self.rail_waiting_count(false)
                    + self.bus_platform_waiting_count()) as u32,
                buildings: self.state.buildings.len() as u32,
                active_projects: self
                    .state
                    .projects
                    .iter()
                    .filter(|p| p.progress < 1.0)
                    .count() as u32,
            },
        }
    }
    fn river_points(&self) -> Vec<Point> {
        self.river_control_points().to_vec()
    }
    fn river_control_points(&self) -> [Point; 5] {
        let bend = (self.state.seed % 5) as f32 * 9.0 - 18.0;
        [
            Point {
                x: -112.0 + bend,
                z: -400.0,
            },
            Point {
                x: -98.0 + bend,
                z: -190.0,
            },
            Point {
                x: -112.0 + bend,
                z: 20.0,
            },
            Point {
                x: -94.0 + bend,
                z: 210.0,
            },
            Point {
                x: -109.0 + bend,
                z: 400.0,
            },
        ]
    }
    pub fn save_json(&self) -> String {
        if let Some(job) = &self.live_access {
            let mut saved = self.state.clone();
            saved.live_access_checkpoint = Some(job.checkpoint());
            serde_json::to_string(&saved).expect("serializing engine state cannot fail")
        } else {
            serde_json::to_string(&self.state).expect("serializing engine state cannot fail")
        }
    }
    pub fn restore_json(json: &str) -> Result<Self, String> {
        let mut state: WorldState = serde_json::from_str(json).map_err(|e| e.to_string())?;
        access_live_engine::validate_face_history(&state.exhausted_access_faces)?;
        if state.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported engine save schema {}",
                state.schema_version
            ));
        }
        if state.activity_schema_version > ACTIVITY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported business activity schema {}",
                state.activity_schema_version
            ));
        }
        if state.activity_schema_version == 0 {
            for building in &mut state.buildings {
                building.business_active = false;
                if is_commercial_kind(&building.kind) {
                    building.occupancy = 0;
                    building.vacant_days = 0;
                    if building.stage == "open" {
                        building.business_active = true;
                    }
                }
            }
            state.activity_schema_version = ACTIVITY_SCHEMA_VERSION;
        }
        // Validate frozen checkpoint shapes and timeline before replaying any
        // saved prefix to reconstruct derived accumulator state.
        state.district_controller.validate(state.tick)?;
        let district_runtime = district_controller::Runtime::restore(&state.district_controller)?;
        let mut migrated_access_geometry = false;
        if let Some(plan) = &state.neighborhood_plan {
            for (from, to) in &plan.segments {
                if let Some(road) = state.roads.iter().find(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                }) {
                    migrated_access_geometry |= state.neighborhood_road_ids.insert(road.id);
                }
            }
        }
        if migrated_access_geometry {
            state.bump_access_geometry_revision();
        }
        let live_access = state.live_access_checkpoint.take()
            .map(access_live::LiveAccess::restore).transpose()
            .map_err(|error| format!("invalid live access checkpoint: {error:?}"))?;
        let mut engine = Self {
            state,
            live_access,
            planned_progress: Vec::new(),
            wait_scratch: Vec::new(),
            lane_order: Vec::new(),
            district_runtime,
            district_policy_experiment: false,
            #[cfg(test)]
            district_policy_comparisons: 0,
            #[cfg(test)]
            district_policy_access_wins: 0,
            #[cfg(test)]
            district_policy_frontier_wins: 0,
            #[cfg(test)]
            district_policy_diagnostics: district_policy::ExperimentDiagnostics::default(),
            #[cfg(test)]
            early_access_experiment: false,
            #[cfg(test)]
            early_access_attempted_tick: None,
            #[cfg(test)]
            early_access_admissions: 0,
            #[cfg(test)]
            early_access_attempts: 0,
            #[cfg(test)]
            early_block_reservation_disable_override: false,
            #[cfg(test)]
            early_block_reservation_attempts: 0,
            #[cfg(test)]
            early_block_reservation_admissions: 0,
            #[cfg(test)]
            early_block_reservation_noops: 0,
            #[cfg(test)]
            early_block_reservation_started_tick: None,
            #[cfg(test)]
            early_block_reservation_duration_ticks: 0,
            #[cfg(test)]
            interior_access_completions: 0,
            #[cfg(test)]
            road_opening_access_experiment: false,
            #[cfg(test)]
            face_conditioned_access_experiment: false,
            #[cfg(test)]
            road_opening_access_attempts: 0,
            #[cfg(test)]
            road_opening_access_admissions: 0,
            #[cfg(test)]
            road_opening_access_branch: None,
        };
        // Validate the serialized active plan before canonicalizing a
        // completed one; malformed empty plans must not disappear through
        // vacuous all() checks during migration.
        engine.validate().map_err(|e| e.to_string())?;
        if !engine
            .state
            .neighborhood_plan
            .as_ref()
            .is_some_and(|plan| plan.spine.is_some())
        {
            engine.retire_completed_neighborhood_plan();
        }
        engine.validate().map_err(|e| e.to_string())?;
        Ok(engine)
    }
    pub fn validate(&self) -> Result<(), String> {
        access_live_engine::validate_face_history(&self.state.exhausted_access_faces)?;
        if self.state.access_geometry_revision_exhausted
            && self.state.access_geometry_revision != u64::MAX
        {
            return Err("exhausted access geometry revision is not saturated".into());
        }
        self.state.district_controller.validate(self.state.tick)?;
        let building_rotations: BTreeMap<u32, f32> = self
            .state
            .buildings
            .iter()
            .map(|building| (building.parcel_id, building.rotation))
            .collect();
        if self.state.road_infill_due > 9 {
            return Err("road infill quota exceeded its cap".into());
        }
        if self.state.frontage_road_cursor > self.state.roads.len()
            || self.state.frontage_slot_cursor as usize > MAX_FRONTAGE_SITES_PER_ROAD
            || self.state.road_origin_cursor > self.state.nodes.len()
            || self.state.neighborhood_anchor_cursor > self.state.nodes.len()
        {
            return Err("frontage scan cursor is outside its bounded range".into());
        }
        if self.state.interior_access_cursor
            > self
                .state
                .roads
                .len()
                .saturating_mul(interior_access::PROPOSALS_PER_HOST)
        {
            return Err("interior access scan cursor is outside its bounded range".into());
        }
        if let Some(plan) = &self.state.pending_interior_access {
            if self.state.neighborhood_plan.is_some() {
                return Err("neighborhood and interior access plans cannot overlap".into());
            }
            if !plan.host_t.is_finite()
                || !(0.0..1.0).contains(&plan.host_t)
                || !(2..=interior_access::MAX_SELECTED_LOTS).contains(&plan.selected_lots.len())
                || self
                    .state
                    .parcels
                    .len()
                    .saturating_add(plan.selected_lots.len())
                    > MAX_PARCELS
                || self
                    .state
                    .parcel_frontages
                    .len()
                    .saturating_add(plan.selected_lots.len())
                    > MAX_PARCELS
                || self
                    .state
                    .next_id
                    .checked_add(plan.selected_lots.len() as u32)
                    .is_none()
                || self
                    .state
                    .roads
                    .len()
                    .saturating_add(self.state.projects.len())
                    > MAX_SEGMENTS
            {
                return Err(
                    "interior access plan has invalid geometry or capacity reservation".into(),
                );
            }
            let retained = self
                .state
                .roads
                .iter()
                .find(|road| road.id == plan.host_road_id)
                .ok_or_else(|| "interior access plan references a missing host half".to_string())?;
            let second = self
                .state
                .roads
                .iter()
                .find(|road| road.id == plan.host_second_half_road_id)
                .ok_or_else(|| {
                    "interior access plan references a missing second host half".to_string()
                })?;
            let branch = self
                .state
                .roads
                .iter()
                .find(|road| road.id == plan.branch_road_id)
                .ok_or_else(|| "interior access plan references a missing branch".to_string())?;
            if retained.stage != "open"
                || second.stage != "open"
                || !retained.connected
                || !second.connected
                || retained.id == second.id
                || retained.id == branch.id
                || second.id == branch.id
                || retained.to != plan.split_node_id
                || second.from != plan.split_node_id
                || retained.kind != second.kind
                || !matches!(retained.kind.as_str(), "local" | "arterial")
                || branch.from != plan.split_node_id
                || branch.to != plan.branch_endpoint_id
                || branch.kind != "local"
                || branch.stage == "open"
                || !matches!(
                    branch.stage.as_str(),
                    "survey" | "clearing" | "structure" | "surface" | "finishing"
                )
            {
                return Err("interior access plan road references are inconsistent".into());
            }
            let project = self
                .state
                .projects
                .iter()
                .find(|project| project.id == plan.project_id)
                .ok_or_else(|| "interior access plan references a missing project".to_string())?;
            if project.kind != "road"
                || project.progress >= 1.0
                || project.from != Some(branch.from)
                || project.to != Some(branch.to)
                || project.progress != branch.progress
                || project.stage != branch.stage
            {
                return Err("interior access project is not pending on its branch".into());
            }
            let original_a = self
                .node(retained.from)
                .ok_or_else(|| "interior access host origin is missing".to_string())?;
            let split = self
                .node(plan.split_node_id)
                .ok_or_else(|| "interior access split junction is missing".to_string())?;
            let original_b = self
                .node(second.to)
                .ok_or_else(|| "interior access host destination is missing".to_string())?;
            if original_a.id == split.id
                || original_a.id == original_b.id
                || split.id == original_b.id
                || plan.branch_endpoint_id == original_a.id
                || plan.branch_endpoint_id == split.id
                || plan.branch_endpoint_id == original_b.id
                || !(0.0..1.0).contains(&plan.host_t)
            {
                return Err("interior access host split has invalid ids or fraction".into());
            }
            let host_dx = original_b.x - original_a.x;
            let host_dz = original_b.z - original_a.z;
            let host_length = host_dx.hypot(host_dz);
            let expected_split = (
                original_a.x + host_dx * plan.host_t,
                original_a.z + host_dz * plan.host_t,
            );
            if !host_length.is_finite()
                || host_length <= 20.0
                || (split.x - expected_split.0).hypot(split.z - expected_split.1) > 0.01
                || (retained.length - host_length * plan.host_t).abs() > 0.01
                || (second.length - host_length * (1.0 - plan.host_t)).abs() > 0.01
            {
                return Err(
                    "interior access split does not match its original host fraction".into(),
                );
            }
            let (branch_a, branch_b) = (
                self.node(branch.from).unwrap(),
                self.node(branch.to).unwrap(),
            );
            let (dx, dz) = (branch_b.x - branch_a.x, branch_b.z - branch_a.z);
            let branch_length = dx.hypot(dz);
            if !branch_length.is_finite()
                || branch_length < 24.0 - 0.01
                || branch_length > 60.0 + 0.01
                || branch.connected
                || [branch_a.x, branch_a.z, branch_b.x, branch_b.z]
                    .iter()
                    .any(|value| *value < MIN + 24.0 || *value > MAX - 24.0)
                || !self.ordinary_road_clear_of_river(
                    branch_a.x, branch_a.z, branch_b.x, branch_b.z, 4.5,
                )
                || corridor_crosses_protected(branch_a.x, branch_a.z, branch_b.x, branch_b.z)
            {
                return Err("interior access branch has invalid length".into());
            }
            // The saved branch must meet the same minimum junction angle as a
            // live road proposal. Check every existing ordinary approach at
            // the split while excluding this plan's own pending branch project.
            let branch_heading = dz.atan2(dx);
            let junction_heading_clear = |bearing: f32| {
                let difference = (branch_heading - bearing).rem_euclid(std::f32::consts::TAU);
                difference.min(std::f32::consts::TAU - difference) >= std::f32::consts::PI / 6.0
            };
            for road in self.state.roads.iter().filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
                    && (road.from == plan.split_node_id || road.to == plan.split_node_id)
            }) {
                let neighbor_id = if road.from == plan.split_node_id {
                    road.to
                } else {
                    road.from
                };
                let neighbor = self.node(neighbor_id).ok_or_else(|| {
                    "interior access junction has a missing road endpoint".to_string()
                })?;
                let bearing = (neighbor.z - split.z).atan2(neighbor.x - split.x);
                if !junction_heading_clear(bearing) {
                    return Err("interior access branch violates junction angle clearance".into());
                }
            }
            for other in self.state.projects.iter().filter(|other| {
                other.id != plan.project_id
                    && other.kind == "road"
                    && other.progress < 1.0
                    && (other.from == Some(plan.split_node_id)
                        || other.to == Some(plan.split_node_id))
            }) {
                let neighbor_id = if other.from == Some(plan.split_node_id) {
                    other.to
                } else {
                    other.from
                };
                let neighbor = neighbor_id.and_then(|id| self.node(id)).ok_or_else(|| {
                    "interior access junction has a missing project endpoint".to_string()
                })?;
                let bearing = (neighbor.z - split.z).atan2(neighbor.x - split.x);
                if !junction_heading_clear(bearing) {
                    return Err("interior access branch violates junction angle clearance".into());
                }
            }
            if self.state.nodes.iter().any(|node| {
                node.id != plan.branch_endpoint_id
                    && (node.x - branch_b.x).hypot(node.z - branch_b.z) < 18.0
            }) {
                return Err("interior access endpoint violates junction spacing".into());
            }
            for road in self.state.roads.iter().filter(|road| {
                road.stage == "open"
                    && road.id != retained.id
                    && road.id != second.id
                    && road.id != branch.id
            }) {
                let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                    return Err("interior access branch intersects an invalid road".into());
                };
                if geometry::segments_distance_coords(
                    branch_a.x, branch_a.z, branch_b.x, branch_b.z, a.x, a.z, b.x, b.z,
                ) < 8.0
                {
                    return Err("interior access branch intersects an existing road".into());
                }
            }
            let (ux, uz) = (dx / branch_length, dz / branch_length);
            let (nx, nz) = (-uz, ux);
            let branch_context = self.collision_context();
            if !parcel_envelopes_clear_of_road(
                &branch_context,
                branch_a.x,
                branch_a.z,
                branch_b.x,
                branch_b.z,
            ) || self
                .state
                .projects
                .iter()
                .filter(|other| {
                    other.id != plan.project_id && other.kind == "road" && other.progress < 1.0
                })
                .any(|other| {
                    let Some((from, to)) = other.from.zip(other.to) else {
                        return false;
                    };
                    let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                        return true;
                    };
                    geometry::segments_distance_coords(
                        branch_a.x, branch_a.z, branch_b.x, branch_b.z, a.x, a.z, b.x, b.z,
                    ) < 8.0
                })
            {
                return Err(
                    "interior access branch overlaps reserved road or parcel geometry".into(),
                );
            }
            let mut reserved_rects = Vec::with_capacity(plan.selected_lots.len());
            let collision_context = branch_context;
            for lot in &plan.selected_lots {
                let rect = geometry::OrientedRect {
                    x: lot.x,
                    z: lot.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                };
                if !rect.finite_positive()
                    || !lot.along.is_finite()
                    || !(0.0..=1.0).contains(&lot.along)
                    || !matches!(lot.side, -1 | 1)
                    || !(8.0..=14.0).contains(&lot.width)
                    || (lot.depth - 12.0).abs() > 0.001
                    || ((lot.x - branch_a.x) * ux + (lot.z - branch_a.z) * uz
                        - lot.along * branch_length)
                        .abs()
                        > 0.01
                    || (((lot.x - branch_a.x) * nx + (lot.z - branch_a.z) * nz)
                        - lot.side as f32 * 15.0)
                        .abs()
                        > 0.01
                    || (lot.rotation - (lot.side as f32 * dz).atan2(-lot.side as f32 * dx)).abs()
                        > 0.001
                    || !self.oriented_parcel_geometry_clear_with_context_mode(
                        &rect,
                        &collision_context,
                        true,
                    )
                    || reserved_rects.iter().any(|other: &geometry::OrientedRect| {
                        geometry::rects_overlap(&rect, other, 2.0)
                    })
                {
                    return Err("interior access lot reservation is invalid or conflicting".into());
                }
                reserved_rects.push(rect);
            }
        }
        if let Some(plan) = &self.state.neighborhood_plan {
            if plan.segments.len() != 4 || plan.next_segment > plan.segments.len() {
                return Err("neighborhood plan is outside its bounded shape".into());
            }
            if plan.coordinated_lots {
                let mut corners = [blocks::BlockPoint { x: 0.0, z: 0.0 }; 4];
                for index in 0..4 {
                    if plan.segments[index].1 != plan.segments[(index + 1) % 4].0 {
                        return Err("coordinated block perimeter is not closed".into());
                    }
                    let Some(node) = self.node(plan.segments[index].0) else {
                        return Err("coordinated block references a missing junction".into());
                    };
                    corners[index] = blocks::BlockPoint {
                        x: node.x,
                        z: node.z,
                    };
                }
                let canonical_lots = blocks::subdivide_rectangular_block(corners, 8.0, 11.0, 2.2);
                let lot_count = if let Some(spine) = &plan.spine {
                    self.validate_neighborhood_spine(plan)?;
                    spine
                        .validate()
                        .map_err(|_| "neighborhood spine geometry is invalid")?
                        .lots
                        .len()
                } else if let Some(selected) = &plan.selected_lots {
                    if !plan.coordinated_lots
                        || !self.selected_block_layout_valid(&plan.segments, selected)
                    {
                        return Err("coordinated block selected-lot layout is invalid".into());
                    }
                    selected.len()
                } else {
                    canonical_lots.len()
                };
                if lot_count < 4 {
                    return Err("coordinated block has invalid geometry".into());
                }
                if self.state.parcels.len().saturating_add(lot_count) > MAX_PARCELS {
                    return Err("coordinated block parcel reservation exceeds capacity".into());
                }
            } else if plan.selected_lots.is_some() {
                return Err("selected block lots require a coordinated block plan".into());
            }
            if plan.spine.is_some() && !plan.coordinated_lots {
                return Err("neighborhood spine requires coordinated lots".into());
            }
            if self.node(plan.anchor_node).is_none()
                || plan
                    .segments
                    .iter()
                    .any(|(from, to)| self.node(*from).is_none() || self.node(*to).is_none())
            {
                return Err("neighborhood plan references a missing junction".into());
            }
        }
        if self.state.neighborhood_road_ids.len() > MAX_SEGMENTS
            || self
                .state
                .neighborhood_road_ids
                .iter()
                .any(|id| !self.state.roads.iter().any(|road| road.id == *id))
        {
            return Err("historic neighborhood road index is invalid or over capacity".into());
        }
        if self.state.buildings.len() > MAX_BUILDINGS {
            return Err("building cap exceeded".into());
        }
        if self.state.roads.len() > MAX_SEGMENTS {
            return Err("network segment cap exceeded".into());
        }
        if self.state.travelers.len() > MAX_TRAVELERS {
            return Err("traveler cap exceeded".into());
        }
        if self.state.residents.len() > MAX_RESIDENTS {
            return Err("resident cast cap exceeded".into());
        }
        if self.state.parcels.len() > MAX_PARCELS {
            return Err("parcel cap exceeded".into());
        }
        if self.state.parcel_frontages.len() > MAX_PARCELS {
            return Err("parcel frontage cap exceeded".into());
        }
        for frontage in &self.state.parcel_frontages {
            if !self
                .state
                .parcels
                .iter()
                .any(|parcel| parcel.id == frontage.parcel_id)
                || !self
                    .state
                    .roads
                    .iter()
                    .any(|road| road.id == frontage.road_id)
                || !frontage.along.is_finite()
                || !(0.0..=1.0).contains(&frontage.along)
                || !matches!(frontage.side, -1 | 1)
            {
                return Err(format!(
                    "parcel {} has invalid frontage metadata",
                    frontage.parcel_id
                ));
            }
        }
        if self.state.projects.len() > MAX_PROJECTS + COMPLETED_PROJECT_HISTORY {
            return Err("project history cap exceeded".into());
        }
        if self.state.population > MAX_POPULATION {
            return Err("represented population cap exceeded".into());
        }
        let mut resident_ids = BTreeSet::new();
        for resident in &self.state.residents {
            if !resident_ids.insert(resident.id) {
                return Err(format!("duplicate resident id {}", resident.id));
            }
            let home = self
                .state
                .buildings
                .iter()
                .find(|b| b.id == resident.home_building)
                .ok_or_else(|| format!("resident {} references a missing home", resident.id))?;
            if !matches!(home.kind.as_str(), "home" | "apartment") {
                return Err(format!(
                    "resident {} is assigned to a non-residential building",
                    resident.id
                ));
            }
            if !self.state.nodes.iter().any(|n| n.id == resident.home_node) {
                return Err(format!(
                    "resident {} references a missing home access node",
                    resident.id
                ));
            }
            if resident
                .work_node
                .is_some_and(|node| !self.state.nodes.iter().any(|n| n.id == node))
            {
                return Err(format!(
                    "resident {} references a missing work access node",
                    resident.id
                ));
            }
            if resident.history.len() > 8 {
                return Err(format!("resident {} history exceeded its cap", resident.id));
            }
            if resident.rail_commuter
                && !matches!(
                    resident.transfer.as_str(),
                    "home"
                        | "walking_to_station"
                        | "waiting_station"
                        | "riding_outbound"
                        | "working_external"
                        | "waiting_gateway"
                        | "riding_inbound"
                        | "walking_home"
                )
            {
                return Err(format!(
                    "resident {} has an invalid rail transfer state",
                    resident.id
                ));
            }
            let riding = matches!(
                resident.transfer.as_str(),
                "riding_outbound" | "riding_inbound"
            );
            if riding != resident.train_id.is_some() {
                return Err(format!(
                    "resident {} has inconsistent train occupancy",
                    resident.id
                ));
            }
            if resident.transfer == "riding_bus" && resident.bus_id.is_none() {
                return Err(format!(
                    "resident {} is riding without a bus assignment",
                    resident.id
                ));
            }
            if matches!(
                resident.transfer.as_str(),
                "walking_to_bus" | "waiting_bus" | "riding_bus" | "walking_from_bus"
            ) && (resident.bus_origin_stop.is_none() || resident.bus_destination_stop.is_none())
            {
                return Err(format!(
                    "resident {} has an incomplete bus journey",
                    resident.id
                ));
            }
        }
        if self.rail_waiting_count(true) > RAIL_QUEUE_CAPACITY
            || self.rail_waiting_count(false) > RAIL_QUEUE_CAPACITY
        {
            return Err("rail terminal queue exceeded its capacity".into());
        }
        if self.state.buses.len() > MAX_BUSES {
            return Err("bus fleet exceeded its cap".into());
        }
        if self.bus_waiting_count() > BUS_QUEUE_CAPACITY {
            return Err("bus stop queues exceeded their shared capacity".into());
        }
        if let Some(line) = &self.state.bus_line {
            if line.stops.len() > MAX_BUS_STOPS || line.route.len() > MAX_BUS_ROUTE_NODES {
                return Err("bus line exceeded its route or stop cap".into());
            }
            if line.active && line.stops.len() < 3 {
                return Err("active bus line has fewer than three stops".into());
            }
            if line.stops.iter().any(|stop| !line.route.contains(stop)) {
                return Err("bus line contains a stop outside its route".into());
            }
            for edge in line.route.windows(2) {
                if !self.state.roads.iter().any(|road| {
                    road.stage == "open"
                        && road.connected
                        && road.kind != "rail"
                        && ((road.from == edge[0] && road.to == edge[1])
                            || (road.from == edge[1] && road.to == edge[0]))
                }) {
                    return Err("bus line uses a disconnected or non-road edge".into());
                }
            }
        }
        let mut bus_passengers = BTreeSet::new();
        for bus in &self.state.buses {
            if bus.passengers.len() > BUS_CAPACITY {
                return Err(format!("bus {} exceeded passenger capacity", bus.id));
            }
            let line = self
                .state
                .bus_line
                .as_ref()
                .filter(|line| line.id == bus.line_id)
                .ok_or_else(|| format!("bus {} references a missing line", bus.id))?;
            let route_length: f32 = self.route_lengths(&line.route).iter().sum();
            if !bus.progress.is_finite()
                || bus.progress < 0.0
                || bus.progress > route_length + 0.01
                || !matches!(bus.direction, -1 | 1)
            {
                return Err(format!("bus {} has invalid movement state", bus.id));
            }
            for resident_id in &bus.passengers {
                if !bus_passengers.insert(*resident_id) {
                    return Err(format!("resident {resident_id} boarded more than one bus"));
                }
                if !self.state.residents.iter().any(|resident| {
                    resident.id == *resident_id
                        && resident.bus_id == Some(bus.id)
                        && resident.transfer == "riding_bus"
                }) {
                    return Err(format!("bus {} has an unlinked passenger", bus.id));
                }
            }
        }
        for resident in &self.state.residents {
            if (resident.transfer == "riding_bus") != bus_passengers.contains(&resident.id) {
                return Err(format!(
                    "resident {} has inconsistent bus occupancy",
                    resident.id
                ));
            }
        }
        for (at_station, states) in [
            (true, ["walking_to_station", "waiting_station"]),
            (false, ["waiting_gateway", "waiting_gateway"]),
        ] {
            let mut slots = BTreeSet::new();
            for resident in self
                .state
                .residents
                .iter()
                .filter(|resident| states.contains(&resident.transfer.as_str()))
            {
                if let Some(slot) = resident.queue_slot {
                    if slot >= RAIL_QUEUE_CAPACITY as u16 || !slots.insert(slot) {
                        return Err(format!(
                            "resident {} has an invalid or duplicate platform slot at {}",
                            resident.id,
                            if at_station { "station" } else { "gateway" }
                        ));
                    }
                }
            }
        }
        let mut passenger_ids = BTreeSet::new();
        for train in &self.state.trains {
            if train.passengers.len() > RAIL_TRAIN_CAPACITY {
                return Err(format!("train {} exceeded passenger capacity", train.id));
            }
            for resident_id in &train.passengers {
                if !passenger_ids.insert(*resident_id) {
                    return Err(format!(
                        "resident {resident_id} boarded more than one train"
                    ));
                }
                if !self.state.residents.iter().any(|resident| {
                    resident.id == *resident_id && resident.train_id == Some(train.id)
                }) {
                    return Err(format!("train {} has an unlinked passenger", train.id));
                }
            }
        }
        for resident in &self.state.residents {
            if let Some(train_id) = resident.train_id {
                if !passenger_ids.contains(&resident.id)
                    || !self.state.trains.iter().any(|train| train.id == train_id)
                {
                    return Err(format!(
                        "resident {} references a missing passenger seat",
                        resident.id
                    ));
                }
            }
        }
        let mut active_actor_ids = BTreeSet::new();
        for traveler in &self.state.travelers {
            if !active_actor_ids.insert(traveler.id) {
                return Err(format!("duplicate active traveler id {}", traveler.id));
            }
            if traveler.route.is_empty() || traveler.route_lengths.len() + 1 != traveler.route.len()
            {
                return Err(format!("traveler {} has a malformed route", traveler.id));
            }
            if traveler.edge_index > traveler.route_lengths.len()
                || !traveler.edge_progress.is_finite()
                || traveler.edge_progress < 0.0
                || traveler.speed <= 0.0
            {
                return Err(format!(
                    "traveler {} has invalid movement state",
                    traveler.id
                ));
            }
            if !matches!(traveler.access_phase, 0..=2)
                || !traveler.access_progress.is_finite()
                || traveler.access_progress < 0.0
            {
                return Err(format!(
                    "traveler {} has invalid access-path state",
                    traveler.id
                ));
            }
            if traveler.access_phase == 0 && traveler.origin_access.is_none() {
                return Err(format!(
                    "traveler {} is missing its origin connector",
                    traveler.id
                ));
            }
            if traveler.access_phase == 2 && traveler.destination_access.is_none() {
                return Err(format!(
                    "traveler {} is missing its destination connector",
                    traveler.id
                ));
            }
            let active_access_length = match traveler.access_phase {
                0 => traveler.origin_access.as_ref().map(AccessPath::length),
                2 => traveler.destination_access.as_ref().map(AccessPath::length),
                _ => None,
            };
            for path in [
                traveler.origin_access.as_ref(),
                traveler.destination_access.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                if !(2..=4).contains(&path.points.len())
                    || path.points.iter().any(|point| {
                        !point.x.is_finite()
                            || !point.z.is_finite()
                            || point.x < MIN - 1.0
                            || point.x > MAX + 1.0
                            || point.z < MIN - 1.0
                            || point.z > MAX + 1.0
                    })
                {
                    return Err(format!(
                        "traveler {} has invalid access geometry: phase={} progress={:.3} path_length={:.3} points={:?}",
                        traveler.id,
                        traveler.access_phase,
                        traveler.access_progress,
                        path.length(),
                        path.points
                    ));
                }
            }
            if active_access_length
                .is_some_and(|length| traveler.access_progress > length + traveler.speed)
            {
                return Err(format!(
                    "traveler {} exceeded its active access path",
                    traveler.id
                ));
            }
            if traveler.resident != 0 && !resident_ids.contains(&traveler.resident) {
                return Err(format!(
                    "traveler {} references a missing resident",
                    traveler.id
                ));
            }
        }
        if !self.state.station_online && !self.state.trains.is_empty() {
            return Err("train service exists before station and rail are usable".into());
        }
        if self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .count()
            > MAX_PROJECTS
        {
            return Err("project cap exceeded".into());
        }
        let ids: BTreeSet<u32> = self.state.nodes.iter().map(|n| n.id).collect();
        if self
            .state
            .roads
            .iter()
            .any(|r| !ids.contains(&r.from) || !ids.contains(&r.to))
        {
            return Err("road references missing node".into());
        }
        for b in &self.state.buildings {
            if b.occupancy > b.capacity {
                return Err(format!("building {} occupancy exceeds capacity", b.id));
            }
            let p = self
                .state
                .parcels
                .iter()
                .find(|p| p.id == b.parcel_id)
                .ok_or_else(|| format!("building {} references missing parcel", b.id))?;
            if b.stage == "open" && !self.node_connected(p.access_node) {
                return Err(format!(
                    "open building {} lacks connected road access",
                    b.id
                ));
            }
            if !(MIN..=MAX).contains(&b.x) || !(MIN..=MAX).contains(&b.z) {
                return Err(format!("building {} outside boundary", b.id));
            }
            if p.protected {
                return Err(format!("building {} occupies protected land", b.id));
            }
            if self
                .state
                .buildings
                .iter()
                .any(|other| other.id != b.id && other.parcel_id == b.parcel_id)
            {
                return Err(format!(
                    "parcel {} contains multiple buildings",
                    b.parcel_id
                ));
            }
        }
        for (i, p) in self.state.parcels.iter().enumerate() {
            let footprint = parcel_envelope(p);
            if !footprint.finite_positive() {
                return Err(format!("parcel {} has invalid geometry", p.id));
            }
            if let Some(rotation) = p.rotation {
                let facing = self.frontage_rotation(p.id);
                if !self
                    .state
                    .parcel_frontages
                    .iter()
                    .any(|frontage| frontage.parcel_id == p.id)
                    || (rotation.sin() - facing.sin()).abs() > 1e-4
                    || (rotation.cos() - facing.cos()).abs() > 1e-4
                {
                    return Err(format!(
                        "parcel {} orientation does not face its frontage",
                        p.id
                    ));
                }
            }
            let (min_x, min_z, max_x, max_z) = footprint.bounds();
            if min_x < MIN || max_x > MAX || min_z < MIN || max_z > MAX {
                return Err(format!("parcel {} outside boundary", p.id));
            }
            for other in self.state.parcels.iter().skip(i + 1) {
                if geometry::rects_overlap(&footprint, &parcel_envelope(other), 0.0) {
                    return Err(format!("parcels {} and {} overlap", p.id, other.id));
                }
            }
            let mut road_envelopes = vec![footprint];
            let road_clearance = if p.rotation.is_some() { 3.0 } else { 5.0 };
            let legacy_center_clearance = (p.width.max(p.depth) * 0.5 + 3.0).max(8.0);
            if p.rotation.is_none() {
                let rendered_rotation = building_rotations
                    .get(&p.id)
                    .copied()
                    .unwrap_or_else(|| self.frontage_rotation(p.id));
                let mut rendered = footprint;
                rendered.rotation = rendered_rotation;
                road_envelopes.push(rendered);
            }
            for road in self.state.roads.iter().filter(|r| r.stage == "open") {
                let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                    continue;
                };
                let nearest_envelope_distance = road_envelopes
                    .iter()
                    .map(|envelope| geometry::rect_segment_distance(envelope, a.x, a.z, b.x, b.z))
                    .fold(f32::INFINITY, f32::min);
                let exact_clearance_ok = nearest_envelope_distance >= road_clearance;
                let legacy_clearance_ok = p.rotation.is_none()
                    && point_segment_distance(p.x, p.z, a.x, a.z, b.x, b.z)
                        >= legacy_center_clearance;
                if !exact_clearance_ok && !legacy_clearance_ok {
                    return Err(format!(
                        "parcel {} intersects road {} clearance: parcel=({:.2},{:.2}) road=({:.2},{:.2})-({:.2},{:.2}) distance={:.2} clearance={:.2}",
                        p.id,
                        road.id,
                        p.x,
                        p.z,
                        a.x,
                        a.z,
                        b.x,
                        b.z,
                        nearest_envelope_distance,
                        road_clearance
                    ));
                }
            }
            if let Some(building_id) = p.reservation {
                let building = self
                    .state
                    .buildings
                    .iter()
                    .find(|b| b.id == building_id && b.parcel_id == p.id)
                    .ok_or_else(|| format!("parcel {} reserves a missing building", p.id))?;
                if building.stage == "open" {
                    return Err(format!("parcel {} reserves an open building", p.id));
                }
                if !self.state.projects.iter().any(|project| {
                    project.kind == "building"
                        && project.parcel_id == Some(p.id)
                        && project.progress < 1.0
                }) {
                    return Err(format!(
                        "parcel {} has no active construction project",
                        p.id
                    ));
                }
            }
        }
        Ok(())
    }
}

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
        pub fn step(&mut self, ticks: u32) {
            self.inner.step(ticks)
        }
        #[wasm_bindgen(js_name=snapshot)]
        pub fn snapshot(&self) -> String {
            self.inner.snapshot_json()
        }
        #[wasm_bindgen(js_name=movements)]
        pub fn movements(&self) -> Vec<f32> {
            self.inner.movement_buffer()
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
            self.inner.try_split_open_road(road_id, fraction)
                .map(|outcome| serde_json::json!({
                    "split_node_id": outcome.split_node_id,
                    "retained_road_id": outcome.retained_road_id,
                    "new_road_id": outcome.new_road_id,
                }).to_string())
                .map_err(|error| JsValue::from_str(&format!("{error:?}")))
        }
        #[wasm_bindgen(js_name=set_development_rate)]
        pub fn set_development_rate(&mut self, rate: u32) {
            self.inner.set_development_rate(rate)
        }
    }
}

#[cfg(test)]
#[path = "early_access_experiment.rs"]
mod early_access_experiment;
#[cfg(test)]
#[path = "early_block_reservation_experiment.rs"]
mod early_block_reservation_experiment;
#[cfg(test)]
#[path = "mature_void_diagnostic.rs"]
mod mature_void_diagnostic;
#[cfg(test)]
#[path = "road_opening_access_experiment.rs"]
mod road_opening_access_experiment;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug)]
    struct MatureProposalAudit {
        anchors_seen: usize,
        anchors_with_local_demand: usize,
        proposal_count: usize,
        reasons: BTreeMap<&'static str, usize>,
        blocked_lot_sites: BTreeMap<&'static str, usize>,
        eligible_reuse_edges: usize,
        windows: usize,
        shared_candidates: usize,
        full_candidates: usize,
        valid_proposals: usize,
    }

    #[derive(Clone, Copy, Debug)]
    struct MidEdgeLotProbe {
        x: f32,
        z: f32,
        width: f32,
        depth: f32,
        rotation: f32,
        side: i8,
        along: f32,
    }

    #[derive(Clone, Debug)]
    struct MidEdgeAccessProbe {
        host_road_id: u32,
        host_t: f32,
        start: WorldPoint,
        end: WorldPoint,
        length: f32,
        lots: Vec<MidEdgeLotProbe>,
    }

    #[derive(Default, Debug)]
    struct MidEdgeProbeAudit {
        hosts: usize,
        attempted: usize,
        rejected: BTreeMap<&'static str, usize>,
        safe_lot_count: usize,
        feasible: Vec<MidEdgeAccessProbe>,
    }

    fn probe_mid_edge_access(
        engine: &Engine,
        context: &CollisionContext,
        host_road: &Road,
        host_a: &Node,
        host_b: &Node,
        host_t: f32,
        side: i8,
        length: f32,
    ) -> Result<MidEdgeAccessProbe, &'static str> {
        let host_dx = host_b.x - host_a.x;
        let host_dz = host_b.z - host_a.z;
        let host_length = host_dx.hypot(host_dz);
        if !host_length.is_finite() || host_length <= 0.0 {
            return Err("invalid_host");
        }
        let (host_ux, host_uz) = (host_dx / host_length, host_dz / host_length);
        let (nx, nz) = (-host_uz * side as f32, host_ux * side as f32);
        let start = WorldPoint {
            x: host_a.x + host_dx * host_t,
            z: host_a.z + host_dz * host_t,
        };
        let end = WorldPoint {
            x: start.x + nx * length,
            z: start.z + nz * length,
        };
        if [start.x, start.z, end.x, end.z]
            .iter()
            .any(|v| !v.is_finite() || *v < MIN + 24.0 || *v > MAX - 24.0)
        {
            return Err("bounds");
        }
        if !engine.ordinary_road_clear_of_river(start.x, start.z, end.x, end.z, 4.5) {
            return Err("river");
        }
        if corridor_crosses_protected(start.x, start.z, end.x, end.z) {
            return Err("protected");
        }
        if !engine
            .road_corridor_clear_of_parcels_with_context(start.x, start.z, end.x, end.z, context)
        {
            return Err("parcel_corridor");
        }
        for road in engine
            .state
            .roads
            .iter()
            .filter(|road| road.stage == "open")
        {
            if road.id == host_road.id {
                continue;
            }
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                continue;
            };
            if geometry::segments_distance_coords(
                start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z,
            ) < 8.0
            {
                return Err("other_road_intersection");
            }
        }

        let mut proposed_context = context.clone();
        proposed_context.open_roads.push(CollisionSegment {
            ax: start.x,
            az: start.z,
            bx: end.x,
            bz: end.z,
        });
        let mut safe_lots = Vec::new();
        for lot_side in [-1_i8, 1] {
            for interval in frontage::subdivide_frontage(
                length,
                &[],
                FRONTAGE_ENDPOINT_CLEARANCE,
                8.0,
                14.0,
                2.2,
                MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
            ) {
                let along = (interval.start + interval.end) * 0.5;
                let parcel_nx = -nz;
                let parcel_nz = nx;
                let x = start.x
                    + (end.x - start.x) * (along / length)
                    + parcel_nx * lot_side as f32 * 15.0;
                let z = start.z
                    + (end.z - start.z) * (along / length)
                    + parcel_nz * lot_side as f32 * 15.0;
                let width = interval.end - interval.start;
                let rotation = (lot_side as f32 * (end.z - start.z))
                    .atan2(-lot_side as f32 * (end.x - start.x));
                let rect = geometry::OrientedRect {
                    x,
                    z,
                    width,
                    depth: 12.0,
                    rotation,
                };
                if engine.progressive_frontage_geometry_clear(&rect, &proposed_context) {
                    safe_lots.push(MidEdgeLotProbe {
                        x,
                        z,
                        width,
                        depth: 12.0,
                        rotation,
                        side: lot_side,
                        along,
                    });
                }
            }
        }
        if safe_lots.len() < 2 {
            return Err("fewer_than_two_safe_lots");
        }
        Ok(MidEdgeAccessProbe {
            host_road_id: host_road.id,
            host_t,
            start,
            end,
            length,
            lots: safe_lots,
        })
    }

    fn audit_mid_edge_access_candidates(engine: &Engine) -> MidEdgeProbeAudit {
        let context = engine.collision_context();
        let mut audit = MidEdgeProbeAudit::default();
        for host in engine.state.roads.iter().filter(|road| {
            road.stage == "open"
                && road.connected
                && !matches!(road.kind.as_str(), "bridge" | "rail")
        }) {
            let (Some(a), Some(b)) = (engine.node(host.from), engine.node(host.to)) else {
                continue;
            };
            audit.hosts += 1;
            for host_t in [0.25_f32, 0.5, 0.75] {
                for side in [-1_i8, 1] {
                    for length in [24.0_f32, 36.0, 48.0, 60.0] {
                        audit.attempted += 1;
                        match probe_mid_edge_access(
                            engine, &context, host, a, b, host_t, side, length,
                        ) {
                            Ok(candidate) => {
                                audit.safe_lot_count += candidate.lots.len();
                                audit.feasible.push(candidate);
                            }
                            Err(reason) => *audit.rejected.entry(reason).or_default() += 1,
                        }
                    }
                }
            }
        }
        audit.feasible.sort_by(|a, b| {
            b.lots
                .len()
                .cmp(&a.lots.len())
                .then_with(|| a.length.total_cmp(&b.length))
                .then_with(|| a.host_road_id.cmp(&b.host_road_id))
                .then_with(|| a.host_t.total_cmp(&b.host_t))
                .then_with(|| a.start.x.total_cmp(&b.start.x))
                .then_with(|| a.start.z.total_cmp(&b.start.z))
        });
        audit.feasible.truncate(8);
        audit
    }

    fn audit_lot_geometry(
        engine: &Engine,
        lot: &blocks::BlockLot,
        context: &CollisionContext,
    ) -> &'static str {
        let rect = geometry::OrientedRect {
            x: lot.center.x,
            z: lot.center.z,
            width: lot.width,
            depth: lot.depth,
            rotation: lot.rotation,
        };
        if !rect.finite_positive()
            || !engine.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth)
        {
            return "lot_river_or_invalid";
        }
        let (min_x, min_z, max_x, max_z) = rect.bounds();
        if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
            return "lot_bounds";
        }
        let protected = geometry::OrientedRect {
            x: -305.0,
            z: -310.0,
            width: 90.0,
            depth: 80.0,
            rotation: 0.0,
        };
        if geometry::rects_overlap(&rect, &protected, 0.0) {
            return "lot_protected";
        }
        for parcel in &engine.state.parcels {
            let legacy = parcel_envelope(parcel);
            if parcel.rotation.is_none() && geometry::rects_overlap(&rect, &legacy, 2.0) {
                return "lot_legacy_envelope";
            }
            let rendered = if let Some(rotation) = parcel.rotation {
                geometry::OrientedRect { rotation, ..legacy }
            } else {
                let rotation = engine
                    .state
                    .buildings
                    .iter()
                    .find(|building| building.parcel_id == parcel.id)
                    .map(|building| building.rotation)
                    .unwrap_or_else(|| engine.frontage_rotation(parcel.id));
                geometry::OrientedRect { rotation, ..legacy }
            };
            if geometry::rects_overlap(&rect, &rendered, 2.0) {
                return "lot_rendered_footprint";
            }
        }
        if context.open_roads.iter().any(|road| {
            geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 5.0
        }) || context.pending_roads.iter().any(|road| {
            geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 7.0
        }) {
            return "lot_road_corridor";
        }
        "lot_other"
    }

    fn audit_neighborhood_candidate(
        engine: &Engine,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        corner_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<&[blocks::BlockLot]>,
        tower_envelopes: &[(f32, f32)],
        context: &CollisionContext,
    ) -> &'static str {
        if engine.neighborhood_candidate_clear_with_edge(
            anchor,
            corners,
            tower_envelopes,
            context,
            corner_ids,
            reused_edge,
            selected_lots,
        ) {
            return "feasible";
        }
        let required_segments = 4 - usize::from(reused_edge.is_some());
        if engine
            .state
            .roads
            .len()
            .saturating_add(engine.state.projects.len())
            .saturating_add(required_segments)
            > MAX_SEGMENTS
        {
            return "segment_budget";
        }
        let mut new_incident = [0usize; 4];
        for edge in 0..4 {
            if reused_edge.is_some_and(|(reused, _)| reused == edge) {
                continue;
            }
            new_incident[edge] += 1;
            new_incident[(edge + 1) % 4] += 1;
        }
        for (edge, id) in corner_ids.iter().enumerate() {
            if id.is_some_and(|node| engine.ordinary_road_degree(node) + new_incident[edge] > 4) {
                return "junction_degree";
            }
        }
        let canonical = blocks::subdivide_rectangular_block(
            corners.map(|point| blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            8.0,
            11.0,
            2.2,
        );
        let lots = selected_lots.unwrap_or(&canonical);
        if lots.len() < 4 {
            return "fewer_than_four_compatible_lots";
        }
        if lots.len() > 64
            || lots.iter().any(|lot| !canonical.contains(lot))
            || engine.state.parcels.len().saturating_add(lots.len()) > MAX_PARCELS
        {
            return "parcel_capacity_or_noncanonical_lots";
        }
        if let Some(lot) = lots.iter().find(|lot| {
            !engine.oriented_parcel_geometry_clear_with_context(
                &geometry::OrientedRect {
                    x: lot.center.x,
                    z: lot.center.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                },
                context,
            )
        }) {
            return audit_lot_geometry(engine, lot, context);
        }
        if let Some((reused_index, _)) = reused_edge {
            for edge in 0..4 {
                if edge != reused_index
                    && (distance(
                        &Node {
                            id: 0,
                            x: corners[edge].x,
                            z: corners[edge].z,
                            kind: String::new(),
                        },
                        &Node {
                            id: 0,
                            x: corners[(edge + 1) % 4].x,
                            z: corners[(edge + 1) % 4].z,
                            kind: String::new(),
                        },
                    ) >= 70.0
                        && lots.iter().filter(|lot| lot.frontage_edge == edge).count() < 2)
                {
                    return "fewer_than_two_sites_on_long_new_edge";
                }
            }
        }
        let corner_nodes: [Node; 4] = std::array::from_fn(|index| {
            corner_ids[index]
                .and_then(|id| engine.node(id).cloned())
                .unwrap_or(Node {
                    id: u32::MAX - index as u32,
                    x: corners[index].x,
                    z: corners[index].z,
                    kind: "candidate".into(),
                })
        });
        for edge in 0..4 {
            let a = &corner_nodes[edge];
            let b = &corner_nodes[(edge + 1) % 4];
            let is_reused = reused_edge.is_some_and(|(reused, road_id)| {
                reused == edge
                    && engine.state.roads.iter().any(|road| {
                        road.id == road_id
                            && road.stage == "open"
                            && road.connected
                            && !matches!(road.kind.as_str(), "rail" | "bridge")
                            && ((road.from == a.id && road.to == b.id)
                                || (road.from == b.id && road.to == a.id))
                    })
            });
            if reused_edge.is_some_and(|(reused, _)| reused == edge) && !is_reused {
                return "reused_edge_not_open_connected";
            }
            if a.x < MIN + 24.0
                || a.x > MAX - 24.0
                || a.z < MIN + 24.0
                || a.z > MAX - 24.0
                || !engine.ordinary_road_clear_of_river(a.x, a.z, b.x, b.z, 4.5)
                || corridor_crosses_protected(a.x, a.z, b.x, b.z)
            {
                return "road_bounds_river_or_protected";
            }
            if !is_reused {
                let heading = (b.z - a.z).atan2(b.x - a.x);
                if corner_ids[edge].is_some_and(|id| !engine.ordinary_heading_clear(id, heading))
                    || corner_ids[(edge + 1) % 4].is_some_and(|id| {
                        !engine.ordinary_heading_clear(id, (a.z - b.z).atan2(a.x - b.x))
                    })
                {
                    return "junction_heading";
                }
            }
            if !is_reused {
                let legacy = engine.state.parcels.iter().any(|parcel| {
                    point_segment_distance(parcel.x, parcel.z, a.x, a.z, b.x, b.z)
                        <= parcel.width.max(parcel.depth) * 0.5 + 5.0
                });
                let exact = context.parcel_footprints.iter().any(|footprint| {
                    geometry::rect_segment_distance(footprint, a.x, a.z, b.x, b.z) < 5.0
                });
                if legacy {
                    return "road_legacy_parcel_radius";
                }
                if exact {
                    return "road_exact_footprint_clearance";
                }
            }
            for road in engine
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open")
            {
                let (Some(old_a), Some(old_b)) = (engine.node(road.from), engine.node(road.to))
                else {
                    continue;
                };
                if is_reused && reused_edge.is_some_and(|(_, id)| id == road.id) {
                    continue;
                }
                let shares = [corner_ids[edge], corner_ids[(edge + 1) % 4]]
                    .into_iter()
                    .flatten()
                    .any(|id| road.from == id || road.to == id);
                if !shares && segments_distance(a, b, old_a, old_b) <= 8.0 {
                    return "existing_road_segment_intersection";
                }
            }
            if engine.state.projects.iter().any(|project| {
                if project.kind != "road" || project.progress >= 1.0 {
                    return false;
                }
                let Some((from, to)) = project.from.zip(project.to) else {
                    return false;
                };
                let (Some(old_a), Some(old_b)) = (engine.node(from), engine.node(to)) else {
                    return false;
                };
                let shares = [corner_ids[edge], corner_ids[(edge + 1) % 4]]
                    .into_iter()
                    .flatten()
                    .any(|id| from == id || to == id);
                !shares && segments_distance(a, b, old_a, old_b) <= 8.0
            }) {
                return "active_road_project_intersection";
            }
            if !is_reused && engine.state.neighborhood_plan.is_some() {
                return "reserved_neighborhood_corridor";
            }
            if tower_envelopes
                .iter()
                .any(|(x, z)| point_segment_distance(*x, *z, a.x, a.z, b.x, b.z) < 24.0)
            {
                return "station_tower_envelope";
            }
        }
        if selected_lots.is_none() {
            return "legacy_inner_frontage_benefit_or_other";
        }
        "other_geometry_gate"
    }

    fn audit_mature_neighborhood_proposals(engine: &Engine) -> MatureProposalAudit {
        let mut audit = MatureProposalAudit::default();
        let node_count = engine.state.nodes.len();
        if node_count == 0 {
            return audit;
        }
        let active_buildings: Vec<_> = engine
            .state
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .collect();
        let connected: BTreeSet<u32> = engine
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
            .flat_map(|road| [road.from, road.to])
            .collect();
        let mut reusable: BTreeMap<u32, Vec<Road>> = BTreeMap::new();
        for road in engine
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
        {
            if !matches!(road.kind.as_str(), "rail" | "bridge") {
                reusable.entry(road.from).or_default().push(road.clone());
                reusable.entry(road.to).or_default().push(road.clone());
            }
        }
        let tower_envelopes = engine.available_station_tower_envelopes();
        let collision = engine.collision_context();
        let mut cursor = engine.state.neighborhood_anchor_cursor % node_count;
        let mut visited = BTreeSet::new();
        while visited.len() < node_count && audit.windows <= node_count {
            audit.windows += 1;
            let first = cursor;
            let mut budget = development::NeighborhoodCandidateBudget::default();
            let mut examined_edges = BTreeSet::new();
            for offset in 0..node_count.min(MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION) {
                let index = (first + offset) % node_count;
                visited.insert(index);
                cursor = (index + 1) % node_count;
                audit.anchors_seen += 1;
                let anchor = &engine.state.nodes[index];
                if !connected.contains(&anchor.id) || engine.ordinary_road_degree(anchor.id) == 0 {
                    continue;
                }
                let local = active_buildings
                    .iter()
                    .filter(|building| {
                        (building.x - anchor.x).hypot(building.z - anchor.z) <= 112.0
                    })
                    .count();
                if local < 8 || local < 4 {
                    continue;
                }
                audit.anchors_with_local_demand += 1;
                if let Some(edges) = reusable.get(&anchor.id) {
                    for road in edges {
                        if examined_edges.contains(&road.id)
                            || examined_edges.len() >= MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION
                        {
                            continue;
                        }
                        examined_edges.insert(road.id);
                        let (Some(start), Some(end)) =
                            (engine.node(road.from), engine.node(road.to))
                        else {
                            continue;
                        };
                        let length = distance(start, end);
                        let extrusion = if (70.0..=90.0).contains(&length) {
                            NEIGHBORHOOD_BLOCK_WIDTH
                        } else if (42.0..=58.0).contains(&length) {
                            NEIGHBORHOOD_BLOCK_LENGTH
                        } else {
                            continue;
                        };
                        audit.eligible_reuse_edges += 1;
                        let side_seed = stateless_tower_site_hash(
                            engine.state.seed ^ road.id,
                            (start.x + end.x) * 0.5,
                            (start.z + end.z) * 0.5,
                        );
                        let sides = if side_seed & 1 == 0 {
                            [1.0_f32, -1.0]
                        } else {
                            [-1.0_f32, 1.0]
                        };
                        for (side_index, side) in sides.into_iter().enumerate() {
                            if !budget.claim_shared() {
                                break;
                            }
                            let nx = -(end.z - start.z) / length * side;
                            let nz = (end.x - start.x) / length * side;
                            let corners = [
                                WorldPoint {
                                    x: start.x,
                                    z: start.z,
                                },
                                WorldPoint { x: end.x, z: end.z },
                                WorldPoint {
                                    x: end.x + nx * extrusion,
                                    z: end.z + nz * extrusion,
                                },
                                WorldPoint {
                                    x: start.x + nx * extrusion,
                                    z: start.z + nz * extrusion,
                                },
                            ];
                            let lots =
                                engine.available_coordinated_block_lots(&corners, &collision);
                            let canonical = blocks::subdivide_rectangular_block(
                                corners.map(|point| blocks::BlockPoint {
                                    x: point.x,
                                    z: point.z,
                                }),
                                8.0,
                                11.0,
                                2.2,
                            );
                            for rejected in canonical.iter().filter(|lot| !lots.contains(lot)) {
                                let blocker = audit_lot_geometry(engine, rejected, &collision);
                                *audit.blocked_lot_sites.entry(blocker).or_default() += 1;
                            }
                            let reason = audit_neighborhood_candidate(
                                engine,
                                anchor,
                                &corners,
                                [Some(road.from), Some(road.to), None, None],
                                Some((0, road.id)),
                                Some(&lots),
                                &tower_envelopes,
                                &collision,
                            );
                            audit.proposal_count += 1;
                            audit.shared_candidates += 1;
                            if reason == "feasible" {
                                audit.valid_proposals += 1;
                            }
                            *audit.reasons.entry(reason).or_default() += 1;
                            let _stable_key =
                                road.id.saturating_mul(2).saturating_add(side_index as u32);
                        }
                        if budget.shared_limit_reached() {
                            break;
                        }
                    }
                }
                if engine.ordinary_road_degree(anchor.id) <= 2 {
                    let start_option =
                        stateless_tower_site_hash(engine.state.seed, anchor.x, anchor.z) as usize
                            % 8;
                    for rotation in 0..8 {
                        if !budget.claim_full() {
                            break;
                        }
                        let option = (start_option + rotation) % 8;
                        let (ux, uz, vx, vz) = match option {
                            0 => (1.0, 0.0, 0.0, 1.0),
                            1 => (1.0, 0.0, 0.0, -1.0),
                            2 => (-1.0, 0.0, 0.0, 1.0),
                            3 => (-1.0, 0.0, 0.0, -1.0),
                            4 => (0.0, 1.0, 1.0, 0.0),
                            5 => (0.0, 1.0, -1.0, 0.0),
                            6 => (0.0, -1.0, 1.0, 0.0),
                            _ => (0.0, -1.0, -1.0, 0.0),
                        };
                        let corners = [
                            WorldPoint {
                                x: anchor.x,
                                z: anchor.z,
                            },
                            WorldPoint {
                                x: anchor.x + ux * NEIGHBORHOOD_BLOCK_LENGTH,
                                z: anchor.z + uz * NEIGHBORHOOD_BLOCK_LENGTH,
                            },
                            WorldPoint {
                                x: anchor.x
                                    + ux * NEIGHBORHOOD_BLOCK_LENGTH
                                    + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                                z: anchor.z
                                    + uz * NEIGHBORHOOD_BLOCK_LENGTH
                                    + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                            },
                            WorldPoint {
                                x: anchor.x + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                                z: anchor.z + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                            },
                        ];
                        let reason = audit_neighborhood_candidate(
                            engine,
                            anchor,
                            &corners,
                            [Some(anchor.id), None, None, None],
                            None,
                            None,
                            &tower_envelopes,
                            &collision,
                        );
                        audit.proposal_count += 1;
                        audit.full_candidates += 1;
                        if reason == "feasible" {
                            audit.valid_proposals += 1;
                        }
                        *audit.reasons.entry(reason).or_default() += 1;
                    }
                }
                if budget.any_limit_reached() {
                    break;
                }
            }
        }
        audit
    }

    fn building_picker_engine() -> (Engine, u32) {
        let mut engine = Engine::new(731);
        engine.state.parcels.clear();
        engine.state.parcel_frontages.clear();
        engine.state.buildings.clear();
        engine.state.projects.clear();
        engine.state.residents.clear();
        engine.state.travelers.clear();
        engine.state.buses.clear();
        engine.state.trains.clear();
        engine.state.bus_line = None;
        engine.state.neighborhood_plan = None;
        engine.state.station_online = true;
        engine.state.demand = 4.0;

        let access_node = engine
            .state
            .roads
            .iter()
            .find(|road| road.stage == "open" && road.connected && road.kind != "rail")
            .map(|road| road.from)
            .expect("founding street is open and connected");
        for (x, z) in [(210.0, 200.0), (222.0, 200.0), (216.0, 212.0)] {
            let parcel = engine.add_parcel(x, z, 8.0, 8.0, access_node, 1.0, false);
            let building = engine.add_building(
                parcel,
                "home",
                2,
                "open",
                1.0,
                4,
                "Test home",
                false,
                Vec::new(),
            );
            let resident_building = engine
                .state
                .buildings
                .iter_mut()
                .find(|item| item.id == building)
                .unwrap();
            resident_building.occupancy = 2;
            resident_building.built_day = 1;
        }
        (engine, access_node)
    }

    #[test]
    fn building_picker_prefers_connected_infill_and_excludes_invalid_sites() {
        let (mut engine, access_node) = building_picker_engine();
        let isolated = engine.add_parcel(350.0, 350.0, 10.0, 10.0, access_node, 0.8, false);
        let infill = engine.add_parcel(216.0, 232.0, 10.0, 10.0, access_node, 0.8, false);
        let protected = engine.add_parcel(240.0, 240.0, 10.0, 10.0, access_node, 0.8, true);
        let disconnected_node = engine.add_node(390.0, 390.0, "junction");
        let disconnected =
            engine.add_parcel(380.0, 380.0, 10.0, 10.0, disconnected_node, 0.8, false);

        // The former sort-then-cycle picker selected the isolated site at
        // cursor 1 despite having ranked the infill site first.
        engine.state.growth_cursor = 1;
        engine.plan_building(1);
        let project = engine
            .state
            .projects
            .iter()
            .find(|project| project.kind == "building")
            .expect("one building project should be planned");
        assert_eq!(project.parcel_id, Some(infill));
        assert_ne!(project.parcel_id, Some(isolated));
        assert_ne!(project.parcel_id, Some(protected));
        assert_ne!(project.parcel_id, Some(disconnected));

        let saved = engine.save_json();
        let mut replay = Engine::restore_json(&saved).unwrap();
        engine.step(400);
        replay.step(400);
        assert_eq!(engine.save_json(), replay.save_json());
    }

    #[test]
    fn building_picker_bounds_window_wraps_cursor_and_replays() {
        let (mut engine, access_node) = building_picker_engine();
        let mut candidate_ids = Vec::new();
        for index in 0..40 {
            let x = 290.0 + (index % 8) as f32 * 13.0;
            let z = 280.0 + (index / 8) as f32 * 13.0;
            candidate_ids.push(engine.add_parcel(x, z, 8.0, 8.0, access_node, 0.8, false));
        }
        engine.state.growth_cursor = 38;

        engine.plan_building(1);
        assert_eq!(engine.state.growth_cursor, 30);
        let selected = engine
            .state
            .projects
            .iter()
            .find(|project| project.kind == "building")
            .and_then(|project| project.parcel_id)
            .expect("one building project should be planned");
        let selected_index = candidate_ids.iter().position(|id| *id == selected).unwrap();
        assert!(selected_index == 38 || selected_index == 39 || selected_index < 30);

        let saved = engine.save_json();
        let mut replay = Engine::restore_json(&saved).unwrap();
        engine.plan_building(1);
        replay.plan_building(1);
        assert_eq!(engine.save_json(), replay.save_json());
        assert_eq!(engine.state.growth_cursor, replay.state.growth_cursor);
    }

    fn bus_test_engine(residents: usize) -> (Engine, Vec<u32>) {
        let mut engine = Engine::new(217);
        let founder = engine.state.buildings.first().unwrap().id;
        let parcel_id = engine.state.buildings.first().unwrap().parcel_id;
        engine.state.buildings[0].kind = "home".into();
        engine.state.buildings[0].stage = "open".into();
        engine.state.buildings[0].occupancy = 1;
        let start = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == parcel_id)
            .unwrap()
            .access_node;
        let start_node = engine.node(start).unwrap().clone();
        let mut route = vec![start];
        let mut previous = start;
        for index in 1..=4 {
            let x_offset = match index {
                1 => 15.0,
                2 => 55.0,
                3 => 95.0,
                _ => 135.0,
            };
            let id = engine.id();
            engine.state.nodes.push(Node {
                id,
                x: start_node.x + x_offset,
                z: start_node.z + 7.0,
                kind: "junction".into(),
            });
            let from = engine.node(previous).unwrap().clone();
            let to = engine.node(id).unwrap().clone();
            let road_id = engine.id();
            engine.state.roads.push(Road {
                id: road_id,
                from: previous,
                to: id,
                kind: "local".into(),
                stage: "open".into(),
                progress: 1.0,
                connected: true,
                length: distance(&from, &to),
                lanes: 2,
            });
            previous = id;
            route.push(id);
        }
        let stops = vec![route[0], route[2], route[3]];
        let line_id = engine.id();
        engine.state.bus_line = Some(BusLine {
            id: line_id,
            stops: stops.clone(),
            route: route[..=3].to_vec(),
            active: true,
            graph_revision: 1,
        });
        let bus_id = engine.id();
        engine.state.buses.push(Bus {
            id: bus_id,
            line_id,
            progress: 0.0,
            direction: 1,
            dwell_ticks: 20_000,
            passengers: Vec::new(),
            next_departure_tick: 20_000,
            last_stop: Some(stops[0]),
        });
        let mut ids = Vec::new();
        for index in 0..residents {
            let id = engine.id();
            ids.push(id);
            engine.state.residents.push(Resident {
                id,
                name: format!("Bus Rider {index}"),
                home_building: founder,
                work_building: Some(founder),
                home_node: route[1],
                work_node: Some(route[4]),
                mode: 0,
                routine: "worker".into(),
                activity: "at_home".into(),
                at_work: false,
                departure_tick: 0,
                return_tick: 10_000,
                last_departure_day: u32::MAX,
                last_return_day: u32::MAX,
                history: vec!["Settled near a connected bus corridor".into()],
                rail_commuter: false,
                transfer: String::new(),
                train_id: None,
                queue_slot: None,
                bus_id: None,
                bus_origin_stop: None,
                bus_destination_stop: None,
                bus_stop_node: None,
            });
        }
        (engine, ids)
    }

    #[test]
    fn named_resident_walks_rides_alights_and_returns_by_bus_with_replay() {
        let (mut engine, ids) = bus_test_engine(1);
        let resident_id = ids[0];
        engine.schedule_residents(0);
        assert_eq!(engine.state.residents[0].transfer, "walking_to_bus");
        let mut saw_waiting = false;
        for _ in 0..2_000 {
            engine.step(1);
            let resident = engine
                .state
                .residents
                .iter()
                .find(|resident| resident.id == resident_id)
                .unwrap();
            if resident.transfer == "waiting_bus" {
                saw_waiting = true;
                break;
            }
            if resident.transfer == "riding_bus" {
                saw_waiting = true;
                break;
            }
        }
        assert!(saw_waiting, "resident never reached the stop queue");
        for _ in 0..100 {
            if engine.state.residents[0].transfer == "riding_bus" {
                break;
            }
            engine.step(1);
        }
        assert_eq!(engine.state.residents[0].transfer, "riding_bus");
        assert_eq!(engine.state.buses[0].passengers, vec![resident_id]);
        assert!(!engine
            .state
            .travelers
            .iter()
            .any(|traveler| traveler.id == resident_id));
        engine.state.buses[0].dwell_ticks = 0;
        let save = engine.save_json();
        let mut restored = Engine::restore_json(&save).unwrap();
        assert_eq!(restored.save_json(), save);
        let mut saw_alight = false;
        for _ in 0..4_000 {
            engine.step(1);
            restored.step(1);
            assert_eq!(engine.save_json(), restored.save_json());
            if engine.state.residents[0].transfer == "walking_from_bus" {
                saw_alight = true;
                break;
            }
        }
        assert!(saw_alight, "bus never reached the rider's destination stop");
        for _ in 0..4_000 {
            engine.step(1);
            restored.step(1);
            assert_eq!(engine.save_json(), restored.save_json());
            if engine.state.residents[0].at_work {
                break;
            }
        }
        assert!(
            engine.state.residents[0].at_work,
            "rider did not reach the workplace door"
        );
        assert_eq!(engine.state.residents[0].transfer, "");
        engine.state.residents[0].return_tick = engine.state.tick;
        engine.schedule_residents(0);
        assert_eq!(engine.state.residents[0].transfer, "walking_to_bus");
        let mut boarded_return = false;
        let return_walker = engine
            .state
            .travelers
            .iter()
            .find(|traveler| traveler.resident == resident_id)
            .expect("return rider should have a connected walking leg");
        let access_length: f32 = return_walker
            .origin_access
            .as_ref()
            .map(|access| {
                access
                    .points
                    .windows(2)
                    .map(|points| (points[1].x - points[0].x).hypot(points[1].z - points[0].z))
                    .sum()
            })
            .unwrap_or(0.0);
        let walk_ticks = ((return_walker.route_lengths.iter().sum::<f32>() + access_length)
            * TICKS_PER_SECOND as f32
            / return_walker.speed.max(0.1))
        .ceil() as u64;
        let line = engine.state.bus_line.as_ref().unwrap();
        let route_length: f32 = engine.route_lengths(&line.route).iter().sum();
        let service_cycle_ticks = (route_length * 2.0 / (7.5 / TICKS_PER_SECOND as f32)).ceil()
            as u64
            + line.stops.len() as u64 * 2 * BUS_DWELL_TICKS as u64
            + BUS_HEADWAY_TICKS * 2
            + line.route.len() as u64 * 2 * 240;
        // Include the walk to the stop, a full route cycle, per-stop dwell,
        // terminal headway and a complete signal phase at each route junction.
        let return_boarding_ticks = walk_ticks + service_cycle_ticks;
        for _ in 0..return_boarding_ticks {
            engine.step(1);
            if engine.state.residents[0].transfer == "riding_bus" {
                boarded_return = true;
                break;
            }
        }
        assert!(
            boarded_return,
            "resident never boarded the return bus at tick{}: resident={:?}, buses={:?}, line={:?}, stops={:?}, vehicles={:?}",
            engine.state.tick,
            engine
                .state
                .residents
                .iter()
                .find(|resident| resident.id == resident_id),
            engine
                .state
                .buses
                .iter()
                .map(|bus| (
                    bus.progress,
                    bus.direction,
                    bus.dwell_ticks,
                    bus.last_stop,
                    bus.passengers.clone()
                ))
                .collect::<Vec<_>>(),
            engine
                .state
                .bus_line
                .as_ref()
                .map(|line| (&line.route, &line.stops)),
            engine.state.events.iter().filter(|event| event.kind == "BusStop").map(|event| (event.tick, event.reason.clone())).collect::<Vec<_>>(),
            engine.state.travelers.iter().map(|traveler| (traveler.kind, &traveler.route, traveler.edge_index, traveler.edge_progress, &traveler.activity, traveler.wait_ticks)).collect::<Vec<_>>()
        );
        let return_save = engine.save_json();
        let mut return_restored = Engine::restore_json(&return_save).unwrap();
        assert_eq!(return_restored.save_json(), return_save);
        let mut returned = false;
        for _ in 0..10_000 {
            engine.step(1);
            return_restored.step(1);
            assert_eq!(engine.save_json(), return_restored.save_json());
            if !engine.state.residents[0].at_work && engine.state.residents[0].transfer.is_empty() {
                returned = true;
                break;
            }
        }
        assert!(returned, "resident did not return home by the bus line");
        engine.validate().unwrap();
    }

    #[test]
    fn bus_capacity_keeps_overflow_queue_visible_and_bounded() {
        let (mut engine, ids) = bus_test_engine(BUS_CAPACITY + 6);
        engine.schedule_residents(0);
        for _ in 0..3_000 {
            engine.step(1);
            if engine.state.buses[0].passengers.len() == BUS_CAPACITY
                && engine.bus_waiting_count() == 6
            {
                break;
            }
        }
        assert_eq!(engine.state.buses[0].passengers.len(), BUS_CAPACITY);
        assert_eq!(engine.bus_waiting_count(), 6);
        assert!(engine.bus_waiting_count() <= BUS_QUEUE_CAPACITY);
        assert!(engine.state.buses[0]
            .passengers
            .iter()
            .all(|id| ids.contains(id)));
        assert!(engine.state.residents.iter().all(|resident| {
            resident.transfer != "riding_bus"
                || !engine
                    .state
                    .travelers
                    .iter()
                    .any(|traveler| traveler.id == resident.id)
        }));
        let waiting_movement_count = engine
            .movement_buffer()
            .chunks_exact(5)
            .filter(|movement| ids.iter().any(|id| movement[0] == *id as f32))
            .count();
        assert_eq!(
            waiting_movement_count, 6,
            "only the overflow queue should remain individually visible while riders are aboard"
        );
        engine.validate().unwrap();
        let save = engine.save_json();
        let mut restored = Engine::restore_json(&save).unwrap();
        engine.step(800);
        restored.step(800);
        assert_eq!(engine.save_json(), restored.save_json());
    }

    #[test]
    fn quiet_hours_do_not_start_new_bus_rides_but_current_riders_finish() {
        let (mut engine, ids) = bus_test_engine(1);
        engine.schedule_residents(0);
        let resident_id = ids[0];
        engine
            .state
            .travelers
            .retain(|traveler| traveler.id != resident_id);
        let line = engine.state.bus_line.as_ref().unwrap().clone();
        let resident = engine
            .state
            .residents
            .iter_mut()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        resident.transfer = "waiting_bus".into();
        resident.bus_origin_stop = Some(line.stops[0]);
        resident.bus_destination_stop = Some(line.stops[2]);
        resident.bus_stop_node = resident.bus_origin_stop;
        resident.queue_slot = Some(0);
        engine.state.tick = 19_500; // 01:30, inside the quiet service window
        let start_stop = line.stops[0];
        engine.handle_bus_stop(0, start_stop, &line, true);
        assert_eq!(engine.state.residents[0].transfer, "waiting_bus");
        assert!(engine.state.buses[0].passengers.is_empty());

        let destination_stop = line.stops[2];
        engine.state.buses[0].passengers.push(resident_id);
        engine.state.buses[0].dwell_ticks = 0;
        engine.state.buses[0].direction = 1;
        engine.state.buses[0].progress = engine.bus_stop_distance(destination_stop).unwrap() - 0.2;
        let resident = engine
            .state
            .residents
            .iter_mut()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        resident.transfer = "riding_bus".into();
        let bus_id = engine.state.buses[0].id;
        resident.bus_id = Some(bus_id);
        engine.advance_buses();
        assert_eq!(engine.state.residents[0].transfer, "walking_from_bus");
        assert!(engine.state.buses[0].passengers.is_empty());
    }

    #[test]
    fn legacy_save_without_bus_keys_restores_with_empty_service() {
        let (engine, _) = bus_test_engine(1);
        let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        let state = legacy.as_object_mut().unwrap();
        state.remove("bus_line");
        state.remove("buses");
        state.remove("next_bus_evaluation_tick");
        if let Some(residents) = state
            .get_mut("residents")
            .and_then(serde_json::Value::as_array_mut)
        {
            for resident in residents {
                if let Some(object) = resident.as_object_mut() {
                    object.remove("bus_id");
                    object.remove("bus_origin_stop");
                    object.remove("bus_destination_stop");
                    object.remove("bus_stop_node");
                }
            }
        }
        let restored = Engine::restore_json(&legacy.to_string()).unwrap();
        assert!(restored.state.bus_line.is_none());
        assert!(restored.state.buses.is_empty());
        assert_eq!(restored.snapshot().buses.len(), 0);
    }

    #[test]
    fn bus_movement_uses_kind_three_and_left_hand_lane() {
        let (engine, _) = bus_test_engine(0);
        let line = engine.state.bus_line.as_ref().unwrap();
        let bus = &engine.state.buses[0];
        let (x, z, heading) = engine.bus_pose(line, bus).unwrap();
        let movement = engine.movement_buffer();
        let actor = movement
            .chunks_exact(5)
            .find(|entry| entry[0] == bus.id as f32)
            .unwrap();
        assert_eq!(actor[4], 3.0);
        assert!((actor[1] - x).abs() < 0.001);
        assert!((actor[2] - z).abs() < 0.001);
        assert!((actor[3] - heading).abs() < 0.001);
        let route = &line.route;
        let a = engine.node(route[0]).unwrap();
        let b = engine.node(route[1]).unwrap();
        let normal = (-(b.z - a.z) / distance(a, b), (b.x - a.x) / distance(a, b));
        assert!((x - a.x - normal.0 * 1.65).abs() < 0.001);
        assert!((z - a.z - normal.1 * 1.65).abs() < 0.001);
        let views = engine.snapshot().bus_lines[0].stop_positions.clone();
        assert_eq!(views.len(), line.stops.len() * 2);
        assert!(views.iter().any(|view| view.direction == 1));
        assert!(views.iter().any(|view| view.direction == -1));
    }

    #[test]
    fn dwelling_bus_uses_the_departure_curb_on_curved_stops_in_both_directions() {
        let (mut engine, _) = bus_test_engine(0);
        let route = engine.state.bus_line.as_ref().unwrap().route.clone();
        let positions = [
            (0.0, 0.0),
            (20.0, 0.0),
            (2.0, 2.0),
            (42.0, 14.0),
            (62.0, 14.0),
        ];
        for (node_id, (x, z)) in route.iter().zip(positions) {
            let node = engine
                .state
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
                .unwrap();
            node.x = x;
            node.z = z;
        }
        let stop = route[1];
        let line = engine.state.bus_line.as_mut().unwrap();
        line.stops = vec![route[0], stop, route[3]];
        let line = line.clone();
        let progress = distance(engine.node(route[0]).unwrap(), engine.node(stop).unwrap());
        engine.state.buses[0].progress = progress;
        engine.state.buses[0].dwell_ticks = 80;
        engine.state.buses[0].last_stop = Some(stop);

        for direction in [1_i8, -1_i8] {
            engine.state.buses[0].direction = direction;
            let view = engine
                .bus_stop_views(&line)
                .into_iter()
                .find(|view| view.node_id == stop && view.direction == direction)
                .unwrap();
            let (x, z, heading) = engine.bus_pose(&line, &engine.state.buses[0]).unwrap();
            let dx = heading.cos();
            let dz = heading.sin();
            let nx = -dz;
            let nz = dx;
            let node = engine.node(stop).unwrap();
            let bus_lateral = (x - node.x) * nx + (z - node.z) * nz;
            let marker_lateral = (view.x - node.x) * nx + (view.z - node.z) * nz;
            assert!((heading - view.heading).sin().abs() < 0.0001);
            assert!((bus_lateral - 1.65).abs() < 0.001);
            assert!((marker_lateral - 6.2).abs() < 0.001);
            assert!(bus_lateral * marker_lateral > 0.0);
            let queue = engine.bus_queue_position(stop, direction, 0).unwrap();
            let queue_lateral = (queue.x - node.x) * nx + (queue.z - node.z) * nz;
            assert!((queue_lateral - 6.2).abs() < 0.001);
        }
    }

    #[test]
    fn bus_stop_selection_keeps_fractional_forward_and_reverse_stops_pending() {
        let stops = [(6, 0.0), (5, 10.0), (4, 20.0)];
        for remainder in [0.0005_f32, 0.05, 0.125, 0.249] {
            let forward_progress = 10.0 - remainder;
            assert_eq!(
                Engine::next_bus_stop(&stops, 1, forward_progress, 20.0, 6, 4),
                (5, 10.0),
                "forward stop was dropped with {remainder}m remaining"
            );

            let reverse_progress = 10.0 + remainder;
            assert_eq!(
                Engine::next_bus_stop(&stops, -1, reverse_progress, 20.0, 6, 4),
                (5, 10.0),
                "reverse stop was dropped with {remainder}m remaining"
            );
        }
        assert_eq!(
            Engine::next_bus_stop(&stops, 1, 10.0, 20.0, 6, 4),
            (4, 20.0),
            "forward bus must skip its exact current stop"
        );
        assert_eq!(
            Engine::next_bus_stop(&stops, -1, 10.0, 20.0, 6, 4),
            (6, 0.0),
            "reverse bus must skip its exact current stop"
        );
    }

    #[test]
    fn reverse_bus_obeys_signal_for_its_actual_outgoing_route_leg() {
        let (mut engine, _) = bus_test_engine(0);
        let line = engine.state.bus_line.as_ref().unwrap().clone();
        let lengths = engine.route_lengths(&line.route);
        let junction_segment = 1;
        let segment_offset: f32 = lengths[..junction_segment].iter().sum();
        let segment_length = lengths[junction_segment];
        let bus = &mut engine.state.buses[0];
        bus.progress = segment_offset + 3.5;
        bus.direction = -1;
        bus.dwell_ticks = 0;
        bus.last_stop = None;

        let from = line.route[junction_segment + 1];
        let junction = line.route[junction_segment];
        let outgoing = line.route[junction_segment - 1];
        assert!(Engine::signalized_state(&engine.state, junction));
        let (Some(junction_node), Some(outgoing_node)) =
            (engine.node(junction), engine.node(outgoing))
        else {
            panic!("test route nodes must exist");
        };
        let horizontal =
            (outgoing_node.x - junction_node.x).abs() >= (outgoing_node.z - junction_node.z).abs();
        let signal_base = junction as u64 * 37 + from as u64 * 11;
        // Pick a phase that is red for the reverse bus's westbound outgoing leg.
        let red_phase = if horizontal { 120 } else { 0 };
        engine.state.tick = (red_phase + 240 - signal_base % 240) % 240;
        let before = engine.state.buses[0].progress;
        engine.advance_buses();
        assert!(
            (engine.state.buses[0].progress - before).abs() < 0.0001,
            "reverse bus crossed its red stop line"
        );

        // The opposite phase is green, so the same bus can clear the junction.
        engine.state.tick = engine.state.tick.saturating_add(120);
        engine.advance_buses();
        assert!(
            engine.state.buses[0].progress < before,
            "reverse bus did not proceed on green"
        );
        assert!(segment_length > 3.5);
    }

    #[test]
    fn cars_keep_a_following_gap_behind_a_stopped_bus() {
        let (mut engine, _) = bus_test_engine(0);
        let line = engine.state.bus_line.as_ref().unwrap().clone();
        let lengths = engine.route_lengths(&line.route);
        let offset: f32 = lengths[0];
        engine.state.buses[0].progress = offset + 17.0;
        engine.state.buses[0].dwell_ticks = 100;
        engine.state.buses[0].last_stop = Some(line.route[2]);
        let from = line.route[1];
        let to = line.route[2];
        let edge_length = lengths[1];
        engine.state.travelers.push(Traveler {
            id: 900_001,
            kind: 1,
            route: vec![from, to],
            route_lengths: vec![edge_length],
            edge_index: 0,
            edge_progress: 8.0,
            speed: 9.0,
            destination: to,
            activity: "test_bus_following".into(),
            resident: 0,
            name: "Following car".into(),
            origin_building: None,
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        });
        engine.advance_travelers();
        assert!(engine.state.travelers[0].edge_progress <= 8.001);

        engine.state.travelers.clear();
        engine.state.buses[0].progress = 5.0;
        engine.state.buses[0].dwell_ticks = 0;
        engine.state.travelers.push(Traveler {
            id: 900_002,
            kind: 1,
            route: vec![line.route[0], line.route[1]],
            route_lengths: vec![lengths[0]],
            edge_index: 0,
            edge_progress: 12.0,
            speed: 9.0,
            destination: line.route[1],
            activity: "leading_car".into(),
            resident: 0,
            name: "Leading car".into(),
            origin_building: None,
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        });
        engine.advance_buses();
        assert!(engine.state.buses[0].progress <= 5.001);
    }

    #[test]
    fn following_bus_holds_its_position_without_being_forced_backward() {
        let (mut engine, _) = bus_test_engine(0);
        let line = engine.state.bus_line.as_ref().unwrap().clone();
        let mut leader = engine.state.buses[0].clone();
        leader.id = engine.id();
        leader.progress = 0.0;
        leader.direction = -1;
        leader.dwell_ticks = 0;
        leader.last_stop = None;
        engine.state.buses[0].progress = 10.0;
        engine.state.buses[0].direction = -1;
        engine.state.buses[0].dwell_ticks = 0;
        engine.state.buses[0].last_stop = None;
        engine.state.buses.push(leader);

        let before = engine.state.buses[0].progress;
        engine.advance_buses();
        assert!(engine.state.buses[0].progress <= before + 0.0001);
        assert!(engine.state.buses[0].progress >= before - 0.0001);
        assert_eq!(engine.state.buses[1].progress, 0.0);
        assert!(engine
            .state
            .bus_line
            .as_ref()
            .is_some_and(|current| current.id == line.id));
    }

    #[test]
    fn seeded_neighborhood_bus_carries_residents_on_connected_roads() {
        let mut engine = Engine::new(0);
        engine.set_development_rate(60);
        for _ in 0..36 {
            engine.step(2_000);
        }
        let line = engine
            .state
            .bus_line
            .as_ref()
            .expect("mature seed should open a bus line");
        assert!(line.active);
        assert!(line.stops.iter().all(|stop| line.route.contains(stop)));
        assert!(line
            .route
            .windows(2)
            .all(|edge| engine.state.roads.iter().any(|road| {
                road.stage == "open"
                    && road.connected
                    && road.kind != "rail"
                    && ((road.from == edge[0] && road.to == edge[1])
                        || (road.from == edge[1] && road.to == edge[0]))
            })));
        let boardings = engine
            .state
            .events
            .iter()
            .filter(|event| event.kind == "BusPassengerBoarded")
            .count();
        assert!(boardings > 0, "bus line never carried a resident");
        assert!(engine
            .state
            .buses
            .iter()
            .all(|bus| bus.passengers.len() <= BUS_CAPACITY));
        engine.validate().unwrap();
    }

    #[test]
    fn seed_five_reverse_bus_corridor_is_ordered_and_waiting_trip_resolves() {
        let mut engine = Engine::restore_json(include_str!(
            "../../tests/fixtures/bus-seed5-legacy-queue.json"
        ))
        .expect("historical queue fixture should restore with compatible defaults");
        let line = engine
            .state
            .bus_line
            .as_ref()
            .expect("seed should open bus service");
        let unique: BTreeSet<u32> = line.route.iter().copied().collect();
        assert_eq!(
            unique.len(),
            line.route.len(),
            "service route must not double back through itself"
        );
        let stop_positions: Vec<usize> = line
            .stops
            .iter()
            .map(|stop| line.route.iter().position(|node| node == stop).unwrap())
            .collect();
        assert!(stop_positions.windows(2).all(|pair| pair[0] < pair[1]));

        let resident_id = 46;
        let resident = engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .expect("historical reverse-queue resident should exist");
        assert_eq!(resident.transfer, "waiting_bus");
        assert_eq!(engine.bus_trip_direction(resident), Some(-1));
        let mut boarded = false;
        for _ in 0..24_000 {
            engine.step(1);
            let resident = engine
                .state
                .residents
                .iter()
                .find(|resident| resident.id == resident_id)
                .unwrap();
            if resident.transfer == "riding_bus" && resident.bus_id.is_some() {
                boarded = true;
                break;
            }
        }
        assert!(
            boarded,
            "resident waiting for reverse service did not board an available reverse bus"
        );

        let mut returned_home = false;
        for _ in 0..8_000 {
            engine.step(1);
            let resident = engine
                .state
                .residents
                .iter()
                .find(|resident| resident.id == resident_id)
                .unwrap();
            if !resident.at_work && resident.transfer.is_empty() {
                returned_home = true;
                break;
            }
        }
        assert!(
            returned_home,
            "the queued resident did not finish the bus return trip: resident={:?}, buses={:?}",
            engine
                .state
                .residents
                .iter()
                .find(|resident| resident.id == resident_id),
            engine
                .state
                .buses
                .iter()
                .map(|bus| (
                    bus.progress,
                    bus.direction,
                    bus.dwell_ticks,
                    bus.passengers.len()
                ))
                .collect::<Vec<_>>()
        );
        engine.validate().unwrap();
    }

    #[test]
    fn generated_seed_five_bus_shuttle_runs_both_directions_after_bridge_growth() {
        let mut engine = Engine::new(5);
        engine.set_development_rate(60);
        for _ in 0..36 {
            engine.step(2_000);
        }
        let line = engine
            .state
            .bus_line
            .as_ref()
            .expect("seed five should have active bus service")
            .clone();
        assert!(line.active);
        assert!(line
            .route
            .windows(2)
            .all(|edge| engine.state.roads.iter().any(|road| {
                road.stage == "open"
                    && road.connected
                    && ((road.from == edge[0] && road.to == edge[1])
                        || (road.from == edge[1] && road.to == edge[0]))
            })));
        let served_pairs = engine
            .state
            .residents
            .iter()
            .filter(|resident| !resident.rail_commuter)
            .filter_map(|resident| resident.work_node.map(|work| (resident.home_node, work)))
            .filter(|(home, work)| home != work)
            .filter(|(home, work)| {
                let nearest = |endpoint| {
                    line.stops
                        .iter()
                        .enumerate()
                        .filter_map(|(index, stop)| {
                            let path = engine.route(endpoint, *stop, false);
                            if path.is_empty() {
                                return None;
                            }
                            let length: f32 = engine.route_lengths(&path).iter().sum();
                            (length <= 75.0).then_some((index, length))
                        })
                        .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
                };
                matches!((nearest(*home), nearest(*work)), (Some((a, _)), Some((b, _))) if a != b)
            })
            .count();
        assert!(
            served_pairs > 0,
            "generated line has no resident OD pairs within walking range"
        );
        let route_length = line
            .route
            .windows(2)
            .map(|edge| {
                let a = engine.node(edge[0]).unwrap();
                let b = engine.node(edge[1]).unwrap();
                (a.x - b.x).hypot(a.z - b.z)
            })
            .sum::<f32>();
        let round_trip_ticks = (route_length / 7.5 * TICKS_PER_SECOND as f32 * 2.0).ceil() as usize
            + line.stops.len() * BUS_DWELL_TICKS as usize * 2
            + 1_000;
        let mut directions = BTreeSet::new();
        for _ in 0..round_trip_ticks {
            engine.step(1);
            directions.extend(engine.state.buses.iter().map(|bus| bus.direction));
            if directions.contains(&-1) && directions.contains(&1) {
                break;
            }
        }
        assert!(
            directions.contains(&-1) && directions.contains(&1),
            "generated bus service did not traverse its complete reverse and forward cycle within {round_trip_ticks} ticks"
        );
        let boarding_deadline = engine.state.tick.saturating_add(24_000);
        while engine.state.tick < boarding_deadline
            && !engine
                .state
                .residents
                .iter()
                .any(|resident| resident.transfer == "riding_bus")
        {
            engine.step(1);
        }
        assert!(
            engine
                .state
                .residents
                .iter()
                .any(|resident| resident.transfer == "riding_bus"),
            "generated service covered {served_pairs} resident pairs but no natural rider boarded"
        );
        assert!(engine
            .state
            .buses
            .iter()
            .all(|bus| bus.passengers.len() <= BUS_CAPACITY));
        engine.validate().unwrap();
    }

    #[test]
    fn project_history_admission_trims_completed_and_respects_active_cap() {
        let mut engine = Engine::new(88);
        engine.state.projects.clear();
        let make_project = |id, progress| Project {
            id,
            kind: "road".into(),
            from: Some(1),
            to: Some(2),
            parcel_id: None,
            stage: if progress >= 1.0 { "open" } else { "survey" }.into(),
            progress,
            reason: "test admission boundary".into(),
            duration: 1.0,
            started_tick: 0,
        };
        for id in 1..(MAX_PROJECTS + COMPLETED_PROJECT_HISTORY) as u32 {
            engine.state.projects.push(make_project(id, 1.0));
        }
        let active_id = MAX_PROJECTS as u32 + COMPLETED_PROJECT_HISTORY as u32;
        engine.state.projects.push(make_project(active_id, 0.5));
        assert_eq!(
            engine.state.projects.len(),
            MAX_PROJECTS + COMPLETED_PROJECT_HISTORY
        );

        assert!(engine.project_admission_available());
        assert_eq!(
            engine.state.projects.len(),
            MAX_PROJECTS + COMPLETED_PROJECT_HISTORY
        );
        let next_id = active_id + 1;
        engine.record_project(make_project(next_id, 0.0));
        assert!(engine.state.projects.len() <= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY);
        assert!(engine
            .state
            .projects
            .iter()
            .any(|project| project.id == active_id));
        assert!(engine
            .state
            .projects
            .iter()
            .any(|project| project.id == next_id));
        assert!(!engine.state.projects.iter().any(|project| project.id == 1));

        engine.state.projects.clear();
        for id in 1..=MAX_PROJECTS as u32 {
            engine.state.projects.push(make_project(id, 0.0));
        }
        let before: Vec<u32> = engine
            .state
            .projects
            .iter()
            .map(|project| project.id)
            .collect();
        assert!(!engine.project_admission_available());
        assert_eq!(
            engine
                .state
                .projects
                .iter()
                .map(|project| project.id)
                .collect::<Vec<_>>(),
            before
        );
    }

    fn rail_test_engine(seed: u32) -> (Engine, u32, u32) {
        let mut engine = Engine::new(seed);
        let (from, to) = {
            let rail = engine
                .state
                .roads
                .iter_mut()
                .find(|road| road.kind == "rail")
                .unwrap();
            rail.stage = "open".into();
            rail.progress = 1.0;
            (rail.from, rail.to)
        };
        let station = [from, to]
            .into_iter()
            .find(|id| engine.node(*id).is_some_and(|node| node.kind == "station"))
            .unwrap();
        let gateway = if from == station { to } else { from };
        engine.state.station_ready = true;
        engine.state.station_online = true;
        engine.recompute_connectivity();
        (engine, station, gateway)
    }

    fn add_rail_test_resident(engine: &mut Engine, gateway: u32) -> u32 {
        let existing_home = engine
            .state
            .buildings
            .iter()
            .find(|building| building.name == "Test Rail Home")
            .cloned();
        let (home_id, home_node) = if let Some(home) = existing_home {
            let parcel = engine
                .state
                .parcels
                .iter()
                .find(|parcel| parcel.id == home.parcel_id)
                .unwrap();
            if let Some(building) = engine
                .state
                .buildings
                .iter_mut()
                .find(|building| building.id == home.id)
            {
                building.occupancy = building.occupancy.saturating_add(1);
            }
            (home.id, parcel.access_node)
        } else {
            let parcel = engine
                .state
                .parcels
                .iter()
                .find(|parcel| {
                    parcel.reservation.is_none()
                        && engine
                            .state
                            .buildings
                            .iter()
                            .all(|building| building.parcel_id != parcel.id)
                })
                .unwrap()
                .clone();
            let home_id = engine.add_building(
                parcel.id,
                "home",
                2,
                "open",
                1.0,
                100,
                "Test Rail Home",
                false,
                vec!["Opened for rail transfer test".into()],
            );
            engine.state.buildings.last_mut().unwrap().occupancy = 1;
            (home_id, parcel.access_node)
        };
        let id = engine.id();
        engine.state.residents.push(Resident {
            id,
            name: format!("Commuter {id}"),
            home_building: home_id,
            work_building: None,
            home_node,
            work_node: Some(gateway),
            mode: 0,
            routine: "rail_commuter".into(),
            activity: "at_home".into(),
            at_work: false,
            departure_tick: 0,
            return_tick: 0,
            last_departure_day: u32::MAX,
            last_return_day: u32::MAX,
            history: vec!["Moved into a home near the station route".into()],
            rail_commuter: true,
            transfer: "home".into(),
            train_id: None,
            queue_slot: None,
            bus_id: None,
            bus_origin_stop: None,
            bus_destination_stop: None,
            bus_stop_node: None,
        });
        id
    }

    #[test]
    fn same_seed_and_ticks_have_identical_state() {
        let mut a = Engine::new(42);
        let mut b = Engine::new(42);
        for _ in 0..32 {
            a.step(2_000);
            b.step(2_000);
        }
        assert_eq!(a.state.tick, 64_000);
        assert_eq!(b.state.tick, 64_000);
        assert_eq!(a.save_json(), b.save_json());
    }
    #[test]
    fn early_growth_fills_connected_blocks_before_expanding_the_map() {
        let mut engine = Engine::new(87_231);
        engine.set_development_rate(16);
        let milestones = [
            (1_000, 5, 9),
            (3_000, 11, 16),
            (6_000, 16, 22),
            (12_000, 22, 30),
        ];
        let mut previous_tick = 0;
        for (tick, minimum_buildings, maximum_roads) in milestones {
            let mut remaining = tick - previous_tick;
            while remaining > 0 {
                let batch = remaining.min(2_000);
                engine.step(batch);
                remaining -= batch;
            }
            previous_tick = tick;
            assert_eq!(
                engine.state.tick, tick as u64,
                "milestone must advance exactly"
            );
            engine.validate().unwrap();
            let snapshot = engine.snapshot();
            let open_roads = snapshot
                .roads
                .iter()
                .filter(|road| road.stage == "open")
                .count();
            let open_buildings = snapshot
                .buildings
                .iter()
                .filter(|building| building.stage == "open")
                .count();
            let building_projects = snapshot
                .projects
                .iter()
                .filter(|project| project.kind == "building" && project.progress < 1.0)
                .count();
            assert!(
                open_buildings + building_projects >= minimum_buildings,
                "tick {tick}: only {} open and {building_projects} planned buildings",
                open_buildings
            );
            assert!(
                open_roads <= maximum_roads,
                "tick {tick}: {open_roads} open roads outran neighborhood infill"
            );
            println!(
                "growth milestone tick={tick} roads={open_roads} buildings={open_buildings} planned_buildings={building_projects} population={}",
                snapshot.stats.population
            );
        }
    }

    #[test]
    fn accelerated_growth_changes_measured_stage_without_a_calendar_unlock() {
        let mut engine = Engine::new(87231);
        assert_eq!(engine.snapshot().stage, "Founding");
        engine.set_development_rate(60);
        engine.step(1_000);
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.day, 0, "the test must remain within the first day");
        assert_ne!(snapshot.stage, "Founding");
        assert!(snapshot.stats.buildings >= 3);
    }

    #[test]
    fn road_planner_keeps_frontier_candidates_connected_and_bounded() {
        let mut engine = Engine::new(0);
        engine.set_development_rate(60);
        for _ in 0..36 {
            engine.step(2_000);
        }
        engine.plan_road();
        let after = engine
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "road" && project.progress < 1.0)
            .count();
        let admitted: Vec<_> = engine
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "road" && project.progress < 1.0)
            .collect();
        for project in &admitted {
            let endpoints = project.from.zip(project.to).and_then(|(from, to)| {
                Some((engine.node(from)?.clone(), engine.node(to)?.clone()))
            });
            let frontage_count = endpoints
                .as_ref()
                .map(|(from, to)| engine.viable_frontage_benefit_count(from, to, 2))
                .unwrap_or(0);
            assert!(
                frontage_count >= 2,
                "planner admitted a road without viable frontage"
            );
            assert!(
                endpoints.is_some_and(|(from, to)| {
                    engine.node_connected(from.id) || engine.node_connected(to.id)
                }),
                "frontier road must attach to the connected network"
            );
        }
        assert!(
            after <= MAX_PROJECTS,
            "frontier probing exceeded the active project cap"
        );
        let milestone = engine.snapshot();
        let active_sites = engine
            .state
            .buildings
            .iter()
            .filter(|b| site_active(b))
            .count();
        let developable_lots = engine
            .state
            .parcels
            .iter()
            .filter(|p| {
                !p.protected
                    && engine.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                    && engine.node_connected(p.access_node)
            })
            .count();
        let profile = |engine: &Engine| {
            let building_footprint: f32 = engine
                .state
                .buildings
                .iter()
                .filter(|building| building.stage != "demolished")
                .map(|building| building.width * building.depth)
                .sum();
            let open_ordinary: Vec<_> = engine
                .state
                .roads
                .iter()
                .filter(|road| {
                    road.stage == "open"
                        && road.connected
                        && !matches!(road.kind.as_str(), "rail" | "bridge")
                })
                .collect();
            let ordinary_length: f32 = open_ordinary.iter().map(|road| road.length).sum();
            let active = engine
                .state
                .buildings
                .iter()
                .filter(|b| site_active(b))
                .count();
            let usable_empty = engine
                .state
                .parcels
                .iter()
                .filter(|parcel| {
                    !parcel.protected
                        && parcel.reservation.is_none()
                        && engine.parcel_clear_of_river(
                            parcel.x,
                            parcel.z,
                            parcel.width,
                            parcel.depth,
                        )
                        && engine.node_connected(parcel.access_node)
                        && !engine
                            .state
                            .buildings
                            .iter()
                            .any(|building| building.parcel_id == parcel.id)
                })
                .count();
            format!(
                "tick={} buildings={} footprint_m2={:.0} roads={} ordinary_length_m={:.0} active_sites={} unbuilt_usable={} historic_roads={} demand={:.2} population={} plan={:?} infill_due={} road_projects={} building_projects={}",
                engine.state.tick,
                engine.state.buildings.len(),
                building_footprint,
                open_ordinary.len(),
                ordinary_length,
                active,
                usable_empty,
                engine.state.neighborhood_road_ids.len(),
                engine.state.demand,
                engine.state.population,
                engine
                    .state
                    .neighborhood_plan
                    .as_ref()
                    .map(|plan| (plan.next_segment, plan.segments.len())),
                engine.state.road_infill_due,
                engine
                    .state
                    .projects
                    .iter()
                    .filter(|project| project.kind == "road" && project.progress < 1.0)
                    .count(),
                engine
                    .state
                    .projects
                    .iter()
                    .filter(|project| project.kind == "building" && project.progress < 1.0)
                    .count(),
            )
        };
        println!("seed0 density profile at 72k: {}", profile(&engine));
        assert!(
            matches!(milestone.stage.as_str(), "TransitCity" | "Metropolis" | "Maturity"),
            "frontier case did not reach transit-era growth: stage={} population={} active_sites={} developable_lots={} infill={:.3}",
            milestone.stage,
            milestone.stats.population,
            active_sites,
            developable_lots,
            active_sites as f32 / developable_lots.max(1) as f32,
        );
        assert!(active_sites >= 200);
        let ordinary_roads: BTreeSet<u32> = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.connected
                    && road.stage == "open"
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .map(|road| road.id)
            .collect();
        let frontage_count = engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| ordinary_roads.contains(&frontage.road_id))
            .count();
        assert!(frontage_count as f32 / ordinary_roads.len() as f32 > 2.0);
        assert!(engine.state.parcels.iter().all(|parcel| {
            engine.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                && engine.node_connected(parcel.access_node)
        }));
        assert!(engine
            .state
            .roads
            .iter()
            .filter(|road| road.stage == "open")
            .all(|road| { road.connected || road.kind == "bridge" }));
        engine.validate().unwrap();
        for _ in 0..85 {
            engine.step(2_000);
            engine.validate().unwrap();
        }
        println!("seed0 density profile at 242k: {}", profile(&engine));
        let mut probe = Engine::restore_json(&engine.save_json()).unwrap();
        let mut probe_project = false;
        for _ in 0..200 {
            probe.plan_road();
            if probe
                .state
                .projects
                .iter()
                .any(|project| project.kind == "road" && project.progress < 1.0)
            {
                probe_project = true;
                break;
            }
        }
        println!(
            "seed0 bounded road-plan probe found project={probe_project} cursor={}",
            probe.state.road_origin_cursor
        );
        assert!(
            engine.state.travelers.len() > 0,
            "mature city stopped daily trips"
        );
        assert!(engine
            .state
            .buildings
            .iter()
            .any(|building| { is_commercial(building) && site_active(building) }));
        assert!(engine.state.buildings.iter().all(|building| {
            !is_commercial(building) || !building.business_active || building.vacant_days == 0
        }));
    }

    #[test]
    fn road_frontage_benefit_accepts_two_sites_on_one_side() {
        let mut engine = Engine::new(731);
        // A long existing parcel blocks the north side while leaving the south
        // frontage unobstructed. Two same-side lots still represent useful
        // benefit for a road proposal.
        engine.add_parcel(315.0, 315.0, 84.0, 12.0, 1, 0.5, false);
        let a = Node {
            id: u32::MAX - 1,
            x: 280.0,
            z: 300.0,
            kind: "candidate".into(),
        };
        let b = Node {
            id: u32::MAX,
            x: 350.0,
            z: 300.0,
            kind: "candidate".into(),
        };

        assert_eq!(engine.viable_frontage_benefit_count(&a, &b, 2), 2);
    }

    #[test]
    fn rotated_parcel_corner_blocks_a_corridor_that_passes_center_clearance() {
        let mut engine = Engine::new(3);
        engine.state.parcels.clear();
        engine.state.parcels.push(Parcel {
            id: 10_000,
            x: 0.0,
            z: 0.0,
            width: 14.0,
            depth: 12.0,
            rotation: Some(std::f32::consts::FRAC_PI_4),
            access_node: 0,
            suitability: 1.0,
            protected: false,
            reservation: None,
        });
        let context = engine.collision_context();
        let road_z = 12.1;
        let center_clearance = 14.0_f32.max(12.0) * 0.5 + 5.0;
        assert!(road_z > center_clearance);
        let footprint_clearance = geometry::rect_segment_distance(
            &context.parcel_footprints[0],
            -20.0,
            road_z,
            20.0,
            road_z,
        );
        assert!(
            footprint_clearance < 5.0,
            "footprint clearance: {footprint_clearance}"
        );
        assert!(!engine
            .road_corridor_clear_of_parcels_with_context(-20.0, road_z, 20.0, road_z, &context,));
        assert!(
            engine.road_corridor_clear_of_parcels_with_context(-20.0, 15.0, 20.0, 15.0, &context,)
        );
    }

    #[test]
    fn parcel_corridor_uses_exact_clearance_instead_of_max_dimension_radius() {
        let mut engine = Engine::new(3);
        engine.state.parcels.clear();
        engine.state.parcels.push(Parcel {
            id: 10_000,
            x: 0.0,
            z: 0.0,
            width: 20.0,
            depth: 12.0,
            rotation: Some(0.0),
            access_node: 0,
            suitability: 1.0,
            protected: false,
            reservation: None,
        });
        let context = engine.collision_context();

        // The exact edge clearance is 8 m, although the old max-dimension
        // radius was 15 m and rejected this perfectly parallel street.
        assert!(
            engine.road_corridor_clear_of_parcels_with_context(-20.0, 14.0, 20.0, 14.0, &context,)
        );
        assert!(
            !engine.road_corridor_clear_of_parcels_with_context(-20.0, 10.0, 20.0, 10.0, &context,)
        );
        assert!(
            engine.road_corridor_clear_of_parcels_with_context(-20.0, 16.0, 20.0, 16.0, &context,)
        );
    }

    #[test]
    fn missing_parcel_rotation_keeps_legacy_and_rendered_envelopes() {
        let mut engine = Engine::new(4);
        engine.state.parcels.clear();
        engine.state.buildings.clear();
        engine.state.parcels.push(Parcel {
            id: 10_001,
            x: 0.0,
            z: 0.0,
            width: 40.0,
            depth: 12.0,
            rotation: None,
            access_node: 0,
            suitability: 1.0,
            protected: false,
            reservation: None,
        });
        engine.state.buildings.push(Building {
            id: 10_002,
            parcel_id: 10_001,
            x: 0.0,
            z: 0.0,
            rotation: std::f32::consts::FRAC_PI_2,
            width: 40.0,
            depth: 12.0,
            floors: 1,
            kind: "home".into(),
            stage: "open".into(),
            progress: 1.0,
            capacity: 1,
            occupancy: 1,
            built_day: 0,
            name: "Legacy orientation probe".into(),
            appearance_seed: 1,
            history: Vec::new(),
            anchor: false,
            vacant_days: 0,
            business_active: false,
        });

        let context = engine.collision_context();
        assert_eq!(context.parcel_footprints.len(), 2);
        assert_eq!(context.parcel_footprints[0].rotation, 0.0);
        assert_eq!(
            context.parcel_footprints[1].rotation,
            std::f32::consts::FRAC_PI_2
        );

        // The rendered rotated envelope clears this segment by 9 m, but the
        // historical axis-aligned reservation still intersects it. Existing
        // saves therefore keep both geometry constraints during migration.
        assert!(
            geometry::rect_segment_distance(&context.parcel_footprints[1], 15.0, 0.0, 18.0, 0.0,)
                >= 5.0
        );
        assert!(
            geometry::rect_segment_distance(&context.parcel_footprints[0], 15.0, 0.0, 18.0, 0.0,)
                < 5.0
        );
        assert!(
            !engine.road_corridor_clear_of_parcels_with_context(15.0, 0.0, 18.0, 0.0, &context,)
        );
    }

    #[test]
    fn admitted_interior_branch_with_exact_clearance_restores_without_radial_rejection() {
        // This is an organically generated public save captured while an
        // interior-access project is pending. A later infill recipe changes
        // growth timing, so keep the exact-clearance regression tied to the
        // concrete admitted corridor instead of a seed's incidental schedule.
        let saved = include_str!(
            "../../validation/interior-access-verified-87231.json.project-3736.pending.json"
        );
        let engine = Engine::restore_json(saved).expect("recorded organic branch restores");
        let plan = engine
            .state
            .pending_interior_access
            .as_ref()
            .expect("fixture captures a pending access plan");
        let branch_id = plan.branch_road_id;
        let branch = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == branch_id)
            .expect("pending branch retains its road record");
        let (a, b) = (
            engine.node(branch.from).unwrap(),
            engine.node(branch.to).unwrap(),
        );
        let context = engine.collision_context();
        assert!(parcel_envelopes_clear_of_road(&context, a.x, a.z, b.x, b.z));

        // The pending branch save proves exact-clearance plans survive a
        // canonical save/restore. Keep a separate concrete geometry probe for
        // the old max-dimension circle false rejection: its 20x12 envelope is
        // 5.2m from the road, while the old 15m radius rejects its 11.2m
        // center distance.
        let radial_probe = geometry::OrientedRect {
            x: 0.0,
            z: 11.2,
            width: 20.0,
            depth: 12.0,
            rotation: 0.0,
        };
        let radial_probe_context = CollisionContext {
            parcel_footprints: vec![radial_probe],
            open_roads: Vec::new(),
            pending_roads: Vec::new(),
        };
        assert!(
            point_segment_distance(0.0, 11.2, -40.0, 0.0, 40.0, 0.0)
                <= radial_probe.width.max(radial_probe.depth) * 0.5 + 5.0,
            "legacy radial rule should reject the probe"
        );
        assert!(parcel_envelopes_clear_of_road(
            &radial_probe_context,
            -40.0,
            0.0,
            40.0,
            0.0,
        ));

        let canonical = engine.save_json();
        let restored = Engine::restore_json(&canonical)
            .expect("exactly clear admitted access plan should restore");
        assert_eq!(restored.save_json(), canonical);
        let mut completed = restored;
        for _ in 0..12 {
            if completed
                .state
                .roads
                .iter()
                .find(|road| road.id == branch_id)
                .is_some_and(|road| road.stage == "open" && road.connected)
            {
                break;
            }
            completed.advance_projects();
        }
        let branch = completed
            .state
            .roads
            .iter()
            .find(|road| road.id == branch_id)
            .expect("completed branch keeps its road identity");
        assert_eq!(branch.stage, "open");
        assert!(branch.connected);
        let (a, b) = (
            completed.node(branch.from).unwrap(),
            completed.node(branch.to).unwrap(),
        );
        let opened_context = completed.collision_context();
        assert!(parcel_envelopes_clear_of_road(
            &opened_context,
            a.x,
            a.z,
            b.x,
            b.z,
        ));
        completed.validate().expect("opened branch remains valid");
    }

    #[test]
    fn compact_infill_continuation_restores_with_exact_legacy_road_clearance() {
        let mut engine = Engine::restore_json(include_str!(
            "../../validation/infill-oriented-continued.json"
        ))
        .expect("oriented infill fixture restores before continuation");
        let start_tick = engine.state.tick;
        for _ in 0..32 {
            engine.step(2_000);
        }
        assert_eq!(engine.state.tick - start_tick, 64_000);
        engine
            .validate()
            .expect("continued world passes exact geometry");
        let save = engine.save_json();
        let mut restored =
            Engine::restore_json(&save).expect("compact infill continuation canonical-restores");
        assert_eq!(restored.save_json(), save);
        engine.step(2_000);
        restored.step(2_000);
        assert_eq!(engine.save_json(), restored.save_json());
    }

    #[test]
    fn seed_87231_growth_remains_valid_and_canonical_after_corridor_checks() {
        let mut engine = Engine::new(87_231);
        engine.set_development_rate(16);
        for _ in 0..121 {
            engine.step(2_000);
        }
        assert_eq!(engine.state.tick, 242_000);
        engine.validate().unwrap();
        let save = engine.save_json();
        let restored = Engine::restore_json(&save).unwrap();
        assert_eq!(restored.save_json(), save);
    }

    #[test]
    fn mature_city_grows_a_beneficial_road_beyond_old_open_road_stop() {
        let mut engine = Engine::restore_json(include_str!(
            "../../validation/mature-world.bounded-selection.json"
        ))
        .expect("mature public city fixture should restore");
        let initial_next_id = engine.state.next_id;
        let initial_open_ordinary = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .count();
        assert!(initial_open_ordinary > 160);

        let mut opened_road = None;
        let mut occupied_parcel = None;
        for _ in 0..128 {
            engine.step(2_000);
            opened_road = engine
                .state
                .roads
                .iter()
                .find(|road| {
                    road.id >= initial_next_id
                        && road.kind != "bridge"
                        && road.kind != "rail"
                        && road.stage == "open"
                        && road.connected
                })
                .map(|road| road.id);
            if let Some(road_id) = opened_road {
                let accessible: BTreeSet<u32> = engine
                    .state
                    .parcel_frontages
                    .iter()
                    .filter(|frontage| frontage.road_id == road_id)
                    .filter_map(|frontage| {
                        let parcel = engine
                            .state
                            .parcels
                            .iter()
                            .find(|parcel| parcel.id == frontage.parcel_id)?;
                        engine
                            .node_connected(parcel.access_node)
                            .then_some(parcel.id)
                    })
                    .collect();
                occupied_parcel = engine
                    .state
                    .buildings
                    .iter()
                    .find(|building| {
                        building.id >= initial_next_id
                            && accessible.contains(&building.parcel_id)
                            && site_active(building)
                    })
                    .map(|building| building.parcel_id);
                if occupied_parcel.is_some() {
                    break;
                }
            }
        }
        let road_id = opened_road.expect("growth should open an ordinary road past 160");
        let final_open_ordinary = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .count();
        assert!(final_open_ordinary > initial_open_ordinary);
        let occupied_parcel = occupied_parcel.expect(
            "the new road should lead to a connected, occupied building site through construction",
        );
        let new_frontages = engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| frontage.road_id == road_id)
            .count();
        println!(
            "road-benefit continuation tick={} ordinary_roads={} road_id={} frontage_sites={} occupied_parcel={}",
            engine.state.tick,
            engine
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open" && road.connected && road.kind == "local")
                .count(),
            road_id,
            new_frontages,
            occupied_parcel
        );
        let frontage = engine
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.road_id == road_id && frontage.parcel_id == occupied_parcel)
            .expect("occupied building must have frontage on the new road");
        let parcel = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == frontage.parcel_id)
            .unwrap();
        assert!(engine.node_connected(parcel.access_node));
        engine.validate().unwrap();

        let saved = engine.save_json();
        let mut replay = Engine::restore_json(&saved).unwrap();
        engine.step(2_000);
        replay.step(2_000);
        assert_eq!(engine.save_json(), replay.save_json());
    }

    #[test]
    fn exhausted_infill_quota_does_not_block_road_growth_or_skip_available_lots() {
        fn developed_engine() -> Engine {
            let mut engine = Engine::new(5);
            engine.set_development_rate(60);
            for _ in 0..6 {
                engine.step(2_000);
            }
            engine.state.projects.clear();
            engine.state.tick = engine.state.tick.div_ceil(80) * 80;
            engine.state.demand = 4.0;
            engine.state.road_infill_due = 1;
            engine
        }

        // A lot reserved for a future station tower is not currently buildable
        // while the station is offline. It must not leave the infill quota stuck
        // forever and suppress ordinary road planning.
        let mut tower_only = developed_engine();
        tower_only.state.station_online = false;
        let station = tower_only
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "station")
            .unwrap()
            .clone();
        let access_node = tower_only
            .state
            .roads
            .iter()
            .find(|road| road.stage == "open" && road.connected && road.kind != "rail")
            .unwrap()
            .from;
        let tower_parcel = tower_only.add_parcel(
            station.x,
            station.z + 30.0,
            STATION_TOWER_PARCEL_WIDTH,
            STATION_TOWER_PARCEL_DEPTH,
            access_node,
            1.0,
            false,
        );
        tower_only
            .state
            .parcels
            .retain(|parcel| parcel.id == tower_parcel);
        let initial_parcel_count = tower_only.state.parcels.len();
        assert!(tower_only.is_station_tower_lot(&tower_only.state.parcels[0]));
        assert!(!tower_only.state.station_online);
        assert!(tower_only.node_connected(access_node));
        tower_only.evaluate_growth();
        assert!(
            tower_only.state.parcels.len() > initial_parcel_count,
            "an ineligible tower parcel must not freeze the scheduler before it can add ordinary frontage"
        );
        assert!(tower_only.state.parcels.iter().any(|parcel| {
            parcel.id != tower_parcel
                && !tower_only.is_station_tower_lot(parcel)
                && tower_only.node_connected(parcel.access_node)
        }));
        tower_only.evaluate_growth();
        assert!(
            tower_only
                .state
                .projects
                .iter()
                .any(|project| { project.kind == "building" && project.progress < 1.0 }),
            "the newly available ordinary frontage should be developed on the next evaluation"
        );

        // If an ordinary usable lot exists alongside an ineligible tower lot,
        // the scheduler should spend the quota on that lot before road growth.
        let mut mixed = developed_engine();
        mixed.state.station_online = false;
        let connected = mixed
            .state
            .roads
            .iter()
            .find(|road| road.stage == "open" && road.connected && road.kind != "rail")
            .unwrap()
            .from;
        let connected_node = mixed.node(connected).unwrap().clone();
        let usable = mixed.add_parcel(
            connected_node.x + 120.0,
            connected_node.z + 120.0,
            14.0,
            12.0,
            connected,
            1.0,
            false,
        );
        let ineligible = mixed.add_parcel(
            station.x,
            station.z + 30.0,
            STATION_TOWER_PARCEL_WIDTH,
            STATION_TOWER_PARCEL_DEPTH,
            connected,
            1.0,
            false,
        );
        mixed
            .state
            .parcels
            .retain(|parcel| parcel.id == usable || parcel.id == ineligible);
        assert!(!mixed.state.station_online);
        assert!(mixed.state.parcels.iter().any(|parcel| {
            parcel.id == usable
                && !mixed.is_station_tower_lot(parcel)
                && mixed.node_connected(parcel.access_node)
        }));
        mixed.evaluate_growth();
        assert!(
            mixed
                .state
                .projects
                .iter()
                .any(|project| { project.kind == "building" && project.progress < 1.0 }),
            "usable ordinary frontage was bypassed"
        );
        assert_eq!(mixed.state.road_infill_due, 0);
        assert!(
            !mixed
                .state
                .projects
                .iter()
                .any(|project| { project.kind == "road" && project.progress < 1.0 }),
            "road growth bypassed an ordinary lot while infill was due"
        );
    }
    #[test]
    fn bounded_frontier_cursor_reaches_later_viable_origins() {
        let mut engine = Engine::new(87231);
        engine.set_development_rate(60);
        engine.step(12_000);
        assert!(
            engine
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open")
                .count()
                < 40
        );
        let open_buildings: Vec<&Building> = engine
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .collect();
        let center = open_buildings.iter().fold((0.0, 0.0), |(x, z), building| {
            (
                x + building.x / open_buildings.len() as f32,
                z + building.z / open_buildings.len() as f32,
            )
        });
        let connected: BTreeSet<u32> = engine
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
            .flat_map(|road| [road.from, road.to])
            .collect();
        let mut origins: Vec<&Node> = engine
            .state
            .nodes
            .iter()
            .filter(|node| connected.contains(&node.id))
            .filter(|node| {
                open_buildings
                    .iter()
                    .map(|building| (building.x - node.x).hypot(building.z - node.z))
                    .min_by(f32::total_cmp)
                    .is_some_and(|distance| distance <= 110.0)
            })
            .collect();
        origins.sort_by(|a, b| {
            let ar = (a.x - center.0).hypot(a.z - center.1);
            let br = (b.x - center.0).hypot(b.z - center.1);
            br.total_cmp(&ar).then_with(|| a.id.cmp(&b.id))
        });
        let blocked_origin = origins[0].id;
        let neighbor = engine
            .state
            .roads
            .iter()
            .find(|road| {
                road.connected && (road.from == blocked_origin || road.to == blocked_origin)
            })
            .map(|road| {
                if road.from == blocked_origin {
                    road.to
                } else {
                    road.from
                }
            })
            .unwrap();
        let blocker_id = engine.id();
        engine.state.projects.push(Project {
            id: blocker_id,
            kind: "road".into(),
            from: Some(blocked_origin),
            to: Some(neighbor),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.2,
            reason: "test blocker for one frontier origin".into(),
            duration: 2.0,
            started_tick: engine.state.tick,
        });
        engine.state.road_origin_cursor = 0;
        engine.plan_road();
        let later_project = engine
            .state
            .projects
            .iter()
            .find(|project| {
                project.kind == "road" && project.id != blocker_id && project.progress < 1.0
            })
            .expect("bounded origin search should advance past the blocked origin");
        assert_ne!(later_project.from, Some(blocked_origin));
        engine
            .state
            .projects
            .retain(|project| project.id != blocker_id);
        let project = engine
            .state
            .projects
            .iter()
            .find(|project| project.kind == "road" && project.progress < 1.0)
            .cloned()
            .unwrap();
        let from = project.from.and_then(|id| engine.node(id)).unwrap();
        let to = project.to.and_then(|id| engine.node(id)).unwrap();
        assert!(engine.node_connected(from.id));
        assert!(engine.ordinary_road_clear_of_river(from.x, from.z, to.x, to.z, 4.5));
        engine.validate().unwrap();
    }

    #[test]
    fn open_bridge_head_can_seed_only_a_connected_bank_with_opposite_catchment() {
        fn bridge_world(stage: &str, active_opposite: bool) -> (Engine, u32) {
            let mut engine = Engine::new(87231);
            engine.state.nodes.clear();
            engine.state.roads.clear();
            engine.state.parcels.clear();
            engine.state.parcel_frontages.clear();
            engine.state.buildings.clear();
            engine.state.projects.clear();
            engine.state.neighborhood_plan = None;
            engine.state.neighborhood_road_ids.clear();
            engine.state.demand = 3.0;
            engine.state.tick = 8_000;

            let gateway = engine.add_node(70.0, 42.0, "gateway");
            let east = engine.add_node(-18.0, 42.0, "junction");
            let west = engine.add_node(-184.0, 42.0, "junction");
            engine.add_road(gateway, east, "local", "open", 1.0);
            let bridge = engine.add_road(
                east,
                west,
                "bridge",
                stage,
                if stage == "open" { 1.0 } else { 0.0 },
            );
            engine.recompute_connectivity();
            if stage == "open" {
                assert!(engine
                    .state
                    .roads
                    .iter()
                    .any(|road| road.id == bridge && road.connected));
            }

            if active_opposite {
                let parcel = engine.add_parcel(-18.0, 28.0, 14.0, 12.0, east, 1.0, false);
                let building = engine.add_building(
                    parcel,
                    "apartment",
                    3,
                    "open",
                    1.0,
                    12,
                    "East Bank House",
                    false,
                    vec!["Settled beside the bridge approach".into()],
                );
                engine
                    .state
                    .buildings
                    .iter_mut()
                    .find(|site| site.id == building)
                    .unwrap()
                    .occupancy = 6;
            }
            (engine, west)
        }

        let (mut closed, closed_west) = bridge_world("survey", true);
        closed.plan_road();
        assert!(!closed
            .state
            .projects
            .iter()
            .any(|project| { project.kind == "road" && project.from == Some(closed_west) }));

        let (mut disconnected, disconnected_west) = bridge_world("open", true);
        disconnected
            .state
            .roads
            .iter_mut()
            .find(|road| road.kind == "bridge")
            .unwrap()
            .connected = false;
        disconnected.plan_road();
        assert!(!disconnected
            .state
            .projects
            .iter()
            .any(|project| { project.kind == "road" && project.from == Some(disconnected_west) }));

        let (mut quiet, quiet_west) = bridge_world("open", false);
        quiet.plan_road();
        assert!(!quiet
            .state
            .projects
            .iter()
            .any(|project| { project.kind == "road" && project.from == Some(quiet_west) }));

        let (mut active, active_west) = bridge_world("open", true);
        active.plan_road();
        let first_bank_road = active.state.projects.iter().find(|project| {
            project.kind == "road" && project.from == Some(active_west) && project.progress < 1.0
        });
        assert!(
            first_bank_road.is_some(),
            "an open bridge with an opposite active catchment should admit a bank-entry road; access={:?}, roads={:?}",
            active.state.pending_interior_access.as_ref().map(|plan| (plan.host_road_id, plan.host_t)),
            active.state.projects.iter().filter(|project| project.kind == "road" && project.progress < 1.0).map(|project| (project.from, project.to, &project.reason)).collect::<Vec<_>>()
        );
        let first_bank_road = first_bank_road.unwrap();
        let first_bank_project_id = first_bank_road.id;
        let to = first_bank_road.to.unwrap();
        let west = active.node(active_west).unwrap();
        let target = active.node(to).unwrap();
        assert!((target.x - west.x).hypot(target.z - west.z) <= 90.0);
        assert!(active.frontage_sites(west, target).len() > 0);
        active.plan_road();
        assert_eq!(
            active
                .state
                .projects
                .iter()
                .filter(|project| project.kind == "road" && project.from == Some(active_west))
                .count(),
            1,
            "a pending bridgehead connection must suppress repeated pioneer roads"
        );
        active.recompute_connectivity();
        assert!(active.node_connected(active_west));
        let first_bank_duration = active
            .state
            .projects
            .iter()
            .find(|project| project.id == first_bank_project_id)
            .unwrap()
            .duration
            .ceil() as usize;
        for _ in 0..first_bank_duration {
            active.advance_projects();
        }
        let buildings: Vec<Building> = active
            .state
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .cloned()
            .collect();
        assert!(!active
            .eligible_bridgehead_headings(&buildings)
            .contains_key(&active_west));
        active.validate().unwrap();
        assert_eq!(
            active.save_json(),
            Engine::restore_json(&active.save_json())
                .unwrap()
                .save_json()
        );
    }

    #[test]
    fn mature_connected_bridgehead_grows_occupied_far_bank_neighborhood() {
        let mut engine = Engine::restore_json(include_str!(
            "../../validation/mature-world.multiblock-candidate.json"
        ))
        .expect("checked-in public mature city fixture should restore");
        let bridge = engine
            .state
            .roads
            .iter()
            .find(|road| road.kind == "bridge" && road.stage == "open" && road.connected)
            .unwrap()
            .clone();
        let west_id = if engine.node(bridge.from).unwrap().x < engine.node(bridge.to).unwrap().x {
            bridge.from
        } else {
            bridge.to
        };
        let west_x = engine.node(west_id).unwrap().x;
        assert!(!engine.state.buildings.iter().any(|building| {
            building.stage == "open" && is_housing(building) && building.x < west_x
        }));

        for _ in 0..32 {
            engine.step(2_000);
        }
        let bank_roads: Vec<_> = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
                    && ((road.from == west_id
                        && engine.node(road.to).is_some_and(|node| node.x < west_x))
                        || (road.to == west_id
                            && engine.node(road.from).is_some_and(|node| node.x < west_x)))
            })
            .collect();
        let far_bank_housing = engine.state.buildings.iter().filter(|building| {
            building.stage == "open" && is_housing(building) && building.x < west_x
        });
        assert!(
            !bank_roads.is_empty(),
            "bridge opened without a usable road into the far bank"
        );
        assert!(
            far_bank_housing
                .clone()
                .any(|building| building.occupancy > 0),
            "a far-bank road should support occupied housing through ordinary growth"
        );
        assert!(engine
            .state
            .parcels
            .iter()
            .filter(|parcel| parcel.x < west_x)
            .all(|parcel| {
                engine.node_connected(parcel.access_node)
                    && engine.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
            }));
        engine.validate().unwrap();
        let saved = engine.save_json();
        let mut resumed = Engine::restore_json(&saved).unwrap();
        engine.step(2_000);
        resumed.step(2_000);
        assert_eq!(engine.save_json(), resumed.save_json());
    }
    #[test]
    fn train_shuttle_dwells_and_reverses_inside_rail_end_clearances() {
        let mut engine = Engine::new(77);
        let rail = engine
            .state
            .roads
            .iter_mut()
            .find(|road| road.kind == "rail")
            .expect("founding plan includes a rail corridor");
        rail.stage = "open".into();
        rail.connected = true;
        let rail_length = rail.length;
        engine.state.station_ready = true;
        engine.state.station_online = true;
        let min_center = TRAIN_HALF_LENGTH;
        let max_center = rail_length - TRAIN_HALF_LENGTH;
        assert!(max_center > min_center);

        let mut last_position: Option<(f32, f32)> = None;
        let mut last_direction = 1;
        let mut reversals = 0;
        for _ in 0..2_000 {
            engine.state.tick += 1;
            engine.advance_trains();
            let train = &engine.state.trains[0];
            assert!(train.distance >= min_center - 0.001);
            assert!(train.distance <= max_center + 0.001);
            let movement = engine.movement_buffer();
            let position = (movement[1], movement[2]);
            if train.direction != last_direction {
                reversals += 1;
                if let Some(previous) = last_position {
                    assert!(
                        (position.0 - previous.0).hypot(position.1 - previous.1) < 1.0,
                        "consist jumped while reversing"
                    );
                }
                last_direction = train.direction;
            }
            last_position = Some(position);
        }
        assert!(reversals >= 4, "shuttle did not complete a round trip");
        assert!(engine
            .state
            .events
            .iter()
            .any(|event| event.kind == "TrainGatewayStop"));
        assert!(engine
            .state
            .events
            .iter()
            .any(|event| event.kind == "TrainStationStop"));
    }

    #[test]
    fn rail_service_holds_at_station_during_quiet_hours_and_reopens_at_dawn() {
        let (mut engine, station, gateway) = rail_test_engine(84);
        engine.state.tick = 19_500; // 01:30 city time
        engine.advance_trains();
        let train = &engine.state.trains[0];
        assert_eq!(train.distance, TRAIN_HALF_LENGTH);
        assert_eq!(train.dwell_ticks, 1);

        let resident_id = add_rail_test_resident(&mut engine, gateway);
        engine.schedule_residents(0);
        assert_eq!(
            engine
                .state
                .residents
                .iter()
                .find(|resident| resident.id == resident_id)
                .unwrap()
                .transfer,
            "home",
            "a new commute should wait at home during quiet hours"
        );

        engine.state.tick = 23_000; // 05:00 city time
        engine.schedule_residents(0);
        let commuter = engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        assert_eq!(commuter.transfer, "walking_to_station");
        assert!(
            commuter.home_node == station
                || engine.route(commuter.home_node, station, false).len() >= 2
        );
    }

    #[test]
    fn rail_transfer_boards_to_capacity_and_keeps_platform_queue_visible_and_bounded() {
        let (mut engine, _station, gateway) = rail_test_engine(82);
        for _ in 0..RAIL_TRAIN_CAPACITY + 4 {
            let id = add_rail_test_resident(&mut engine, gateway);
            let resident = engine
                .state
                .residents
                .iter_mut()
                .find(|resident| resident.id == id)
                .unwrap();
            resident.transfer = "waiting_station".into();
            resident.activity = "waiting_for_train".into();
        }
        engine.advance_trains();
        engine.state.trains[0].dwell_ticks = 500;
        for _ in 0..200 {
            engine.state.tick += 1;
            engine.advance_trains();
        }
        let train = &engine.state.trains[0];
        assert_eq!(train.passengers.len(), RAIL_TRAIN_CAPACITY);
        assert_eq!(engine.rail_waiting_count(true), 4);
        assert!(engine.rail_waiting_count(true) <= RAIL_QUEUE_CAPACITY);
        assert!(engine.movement_buffer().len() >= (4 + 1) * 5);
        engine.validate().unwrap();
        let save = engine.save_json();
        assert_eq!(Engine::restore_json(&save).unwrap().save_json(), save);
    }

    #[test]
    fn named_rail_commuter_walks_boards_works_returns_and_replays_from_a_save() {
        let (mut engine, station, gateway) = rail_test_engine(83);
        engine.state.development_rate = 0;
        let resident_id = add_rail_test_resident(&mut engine, gateway);
        let home_node = engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .unwrap()
            .home_node;
        assert!(home_node == station || engine.route(home_node, station, false).len() >= 2);

        let mut saved_onboard = None;
        for _ in 0..5_000 {
            engine.step_one();
            if engine
                .state
                .residents
                .iter()
                .any(|resident| resident.id == resident_id && resident.train_id.is_some())
            {
                saved_onboard = Some(engine.save_json());
                break;
            }
        }
        let save = saved_onboard.expect("resident should board a train within the commute window");
        let mut replay = Engine::restore_json(&save).unwrap();
        assert!(replay
            .state
            .trains
            .iter()
            .any(|train| train.passengers.contains(&resident_id)));
        for _ in 0..6_000 {
            engine.step_one();
            replay.step_one();
        }
        assert_eq!(engine.save_json(), replay.save_json());
        let resident = engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        assert_eq!(resident.transfer, "home");
        assert!(resident
            .history
            .iter()
            .any(|entry| entry.contains("Boarded")));
        assert!(resident
            .history
            .iter()
            .any(|entry| entry.contains("walking home")));
        assert!(engine
            .state
            .events
            .iter()
            .any(|event| event.kind == "RailWorkStarted"));
        assert!(engine
            .state
            .events
            .iter()
            .any(|event| event.kind == "TrainPassengerAlighted"));
        let inspected: serde_json::Value =
            serde_json::from_str(&engine.inspect_json(resident_id)).unwrap();
        assert_eq!(inspected["entityType"], "resident");
        assert_eq!(inspected["transfer"], "home");
        engine.validate().unwrap();
    }

    #[test]
    fn pedestrian_building_connectors_are_continuous_saved_and_face_frontage() {
        let mut engine = Engine::new(46);
        engine.state.development_rate = 0;
        let founder = engine
            .state
            .buildings
            .iter()
            .find(|building| building.anchor)
            .unwrap()
            .clone();
        assert_eq!(founder.rotation, 0.0);
        let home_parcel = engine
            .state
            .parcels
            .iter()
            .find(|parcel| {
                parcel.access_node != engine.state.parcels[0].access_node
                    && engine
                        .state
                        .parcel_frontages
                        .iter()
                        .any(|frontage| frontage.parcel_id == parcel.id)
            })
            .cloned()
            .expect("founding parcels should carry road frontage metadata");
        let home_id = engine.add_building(
            home_parcel.id,
            "home",
            2,
            "open",
            1.0,
            80,
            "Connector Test Home",
            false,
            vec![],
        );
        let route = engine.route(
            engine
                .state
                .parcels
                .iter()
                .find(|parcel| parcel.id == founder.parcel_id)
                .unwrap()
                .access_node,
            home_parcel.access_node,
            false,
        );
        assert!(route.len() >= 2);
        let id = 700_111;
        let mut traveler = Traveler {
            id,
            kind: 0,
            route_lengths: engine.route_lengths(&route),
            route,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 1.5,
            destination: home_parcel.access_node,
            activity: "commute".into(),
            resident: 0,
            name: "Connector Test Walker".into(),
            origin_building: Some(founder.id),
            destination_building: Some(home_id),
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        };
        traveler.origin_access = engine.building_access_path(
            founder.id,
            &traveler.route,
            true,
            traveler.id,
            traveler.kind,
        );
        traveler.destination_access = engine.building_access_path(
            home_id,
            &traveler.route,
            false,
            traveler.id,
            traveler.kind,
        );
        assert!(traveler.origin_access.is_some());
        assert!(traveler.destination_access.is_some());
        let access_snapshot = engine.snapshot();
        let founder_access = access_snapshot
            .building_access
            .iter()
            .find(|access| access.building_id == founder.id)
            .expect("founder access should be exported for its open frontage");
        let founder_path = traveler.origin_access.as_ref().unwrap();
        assert!(
            (founder_access.points[0].x - founder_path.points[0].x)
                .hypot(founder_access.points[0].z - founder_path.points[0].z)
                < 0.01
        );
        assert!(
            (founder_access.points[1].x - founder_path.points[1].x)
                .hypot(founder_access.points[1].z - founder_path.points[1].z)
                < 0.01
        );
        let home_access = access_snapshot
            .building_access
            .iter()
            .find(|access| access.building_id == home_id)
            .expect("home access should be exported for its open frontage");
        let home_path = traveler.destination_access.as_ref().unwrap();
        let home_door = home_path.points.last().unwrap();
        let home_frontage = &home_path.points[home_path.points.len() - 2];
        assert!(
            (home_access.points[0].x - home_door.x).hypot(home_access.points[0].z - home_door.z)
                < 0.01
        );
        assert!(
            (home_access.points[1].x - home_frontage.x)
                .hypot(home_access.points[1].z - home_frontage.z)
                < 0.01
        );

        let frontage_road = home_access.road_id;
        let original_frontage_road = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == frontage_road)
            .unwrap()
            .clone();
        engine
            .state
            .roads
            .iter_mut()
            .find(|road| road.id == frontage_road)
            .unwrap()
            .connected = false;
        assert!(!engine
            .snapshot()
            .building_access
            .iter()
            .any(|access| access.building_id == home_id));
        let road = engine
            .state
            .roads
            .iter_mut()
            .find(|road| road.id == frontage_road)
            .unwrap();
        road.connected = true;
        road.stage = "structure".into();
        assert!(!engine
            .snapshot()
            .building_access
            .iter()
            .any(|access| access.building_id == home_id));
        let road = engine
            .state
            .roads
            .iter_mut()
            .find(|road| road.id == frontage_road)
            .unwrap();
        road.connected = original_frontage_road.connected;
        road.stage = original_frontage_road.stage;
        let door = traveler.origin_access.as_ref().unwrap().points[0];
        let expected_door = WorldPoint {
            x: founder.x + founder.rotation.sin() * (founder.depth * 0.5 + 0.2),
            z: founder.z + founder.rotation.cos() * (founder.depth * 0.5 + 0.2),
        };
        assert!((door.x - expected_door.x).hypot(door.z - expected_door.z) < 0.01);
        traveler.access_phase = 0;
        engine.state.travelers.push(traveler);
        let first = &engine.movement_buffer()[..5];
        assert!((first[1] - door.x).abs() < 0.01 && (first[2] - door.z).abs() < 0.01);
        let save = engine.save_json();
        let mut restored = Engine::restore_json(&save).unwrap();
        for _ in 0..20 {
            engine.state.tick += 1;
            restored.state.tick += 1;
            engine.advance_travelers();
            restored.advance_travelers();
        }
        assert_eq!(engine.save_json(), restored.save_json());
        let mut closest_destination = f32::INFINITY;
        for _ in 0..4_000 {
            let Some(traveler) = engine
                .state
                .travelers
                .iter()
                .find(|traveler| traveler.id == id)
            else {
                break;
            };
            if traveler.access_phase == 2 {
                let door = traveler
                    .destination_access
                    .as_ref()
                    .unwrap()
                    .points
                    .last()
                    .unwrap();
                if let Some(actor) = engine
                    .movement_buffer()
                    .chunks_exact(5)
                    .find(|actor| actor[0] as u32 == id)
                {
                    closest_destination =
                        closest_destination.min((actor[1] - door.x).hypot(actor[2] - door.z));
                }
            }
            engine.state.tick += 1;
            engine.advance_travelers();
        }
        assert!(
            closest_destination < 0.25,
            "walker did not reach the destination door: {closest_destination}"
        );
        assert!(!engine
            .state
            .travelers
            .iter()
            .any(|traveler| traveler.id == id));
        engine.validate().unwrap();
    }

    #[test]
    fn station_queue_positions_are_rail_aligned_reserved_and_continuous() {
        let (mut engine, _station, gateway) = rail_test_engine(48);
        engine.state.development_rate = 0;
        let first_id = add_rail_test_resident(&mut engine, gateway);
        let first_index = engine
            .state
            .residents
            .iter()
            .position(|resident| resident.id == first_id)
            .unwrap();
        engine.schedule_rail_resident(first_index, 0);
        let slot = engine.state.residents[first_index].queue_slot.unwrap();
        let platform = engine.platform_position(true, slot).unwrap();
        let entry = engine
            .state
            .travelers
            .iter()
            .find(|traveler| traveler.id == first_id)
            .unwrap()
            .destination_access
            .clone()
            .expect("station trip needs a platform entry connector");
        assert!((entry.points.last().unwrap().x - platform.x).abs() < 0.01);
        assert!((entry.points.last().unwrap().z - platform.z).abs() < 0.01);
        for _ in 0..4_000 {
            engine.state.tick += 1;
            engine.advance_travelers();
            if engine.state.residents[first_index].transfer == "waiting_station" {
                break;
            }
        }
        assert_eq!(
            engine.state.residents[first_index].transfer,
            "waiting_station"
        );
        let position_for = |engine: &Engine, id: u32| {
            engine
                .movement_buffer()
                .chunks_exact(5)
                .find(|actor| actor[0] as u32 == id)
                .map(|actor| WorldPoint {
                    x: actor[1],
                    z: actor[2],
                })
                .unwrap()
        };
        let waiting_position = position_for(&engine, first_id);
        assert!((waiting_position.x - platform.x).abs() < 0.01);
        assert!((waiting_position.z - platform.z).abs() < 0.01);
        let second_id = add_rail_test_resident(&mut engine, gateway);
        let second_index = engine
            .state
            .residents
            .iter()
            .position(|resident| resident.id == second_id)
            .unwrap();
        engine.schedule_rail_resident(second_index, 0);
        assert_ne!(engine.state.residents[second_index].queue_slot, Some(slot));
        let still_waiting = position_for(&engine, first_id);
        assert!((still_waiting.x - waiting_position.x).abs() < 0.01);
        assert!((still_waiting.z - waiting_position.z).abs() < 0.01);
        engine.validate().unwrap();
    }

    #[test]
    fn legacy_saves_keep_endpoint_walkers_and_all_parcel_paths_obey_the_cap() {
        let mut engine = Engine::new(49);
        let founder = engine
            .state
            .buildings
            .iter()
            .find(|building| building.anchor)
            .unwrap()
            .clone();
        let parcel = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == founder.parcel_id)
            .unwrap();
        let access_node = parcel.access_node;
        let station = engine
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "station")
            .unwrap()
            .id;
        let route = engine.route(parcel.access_node, station, false);
        let mut traveler = Traveler {
            id: 700_112,
            kind: 0,
            route_lengths: engine.route_lengths(&route),
            route,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 1.5,
            destination: station,
            activity: "commute".into(),
            resident: 0,
            name: "Legacy Walker".into(),
            origin_building: Some(founder.id),
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 0,
            access_progress: 0.0,
        };
        traveler.origin_access = engine.building_access_path(
            founder.id,
            &traveler.route,
            true,
            traveler.id,
            traveler.kind,
        );
        engine.state.travelers.push(traveler);
        let mut old_save: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        let state = old_save.as_object_mut().unwrap();
        state.remove("parcel_frontages");
        state.remove("frontage_road_cursor");
        state.remove("frontage_slot_cursor");
        state.remove("road_origin_cursor");
        for traveler in state["travelers"].as_array_mut().unwrap() {
            let traveler = traveler.as_object_mut().unwrap();
            traveler.remove("origin_access");
            traveler.remove("destination_access");
            traveler.remove("access_phase");
            traveler.remove("access_progress");
        }
        for resident in state["residents"].as_array_mut().unwrap() {
            resident.as_object_mut().unwrap().remove("queue_slot");
        }
        let restored = Engine::restore_json(&old_save.to_string()).unwrap();
        let restored_traveler = &restored.state.travelers[0];
        assert_eq!(restored_traveler.access_phase, 1);
        assert!(restored_traveler.origin_access.is_none());
        assert!(restored.state.parcel_frontages.is_empty());
        assert_eq!(restored.state.frontage_road_cursor, 0);
        assert_eq!(restored.state.frontage_slot_cursor, 0);
        assert_eq!(restored.state.road_origin_cursor, 0);
        let movement = restored.movement_buffer();
        assert_eq!(movement[0] as u32, restored_traveler.id);
        let door = engine.state.travelers[0]
            .origin_access
            .as_ref()
            .unwrap()
            .points[0];
        assert!((movement[1] - door.x).hypot(movement[2] - door.z) > 1.0);

        while engine.state.parcels.len() < MAX_PARCELS {
            let _ = engine.add_parcel(0.0, 0.0, 1.0, 1.0, access_node, 0.5, false);
        }
        assert_eq!(
            engine.add_parcel(1.0, 1.0, 1.0, 1.0, access_node, 0.5, false),
            0
        );
        assert_eq!(engine.state.parcels.len(), MAX_PARCELS);
    }

    #[test]
    fn frontage_cursor_accepts_exact_candidate_cap_and_rejects_overflow() {
        let mut engine = Engine::new(17);
        engine.state.frontage_slot_cursor = MAX_FRONTAGE_SITES_PER_ROAD as u16;
        let at_cap = Engine::restore_json(&engine.save_json()).unwrap();
        assert_eq!(
            at_cap.state.frontage_slot_cursor as usize,
            MAX_FRONTAGE_SITES_PER_ROAD
        );

        engine.state.frontage_slot_cursor = MAX_FRONTAGE_SITES_PER_ROAD as u16 + 1;
        assert!(Engine::restore_json(&engine.save_json()).is_err());

        let a = Node {
            id: 10_001,
            x: 200.0,
            z: -160.0,
            kind: "local".into(),
        };
        let b = Node {
            id: 10_002,
            x: 200.0,
            z: 160.0,
            kind: "local".into(),
        };
        let sites = at_cap.frontage_candidate_sites_with_layout(&a, &b, 14.0, 12.0, 18.0);
        // This helper returns only fixed-grid sites. Compact gap candidates
        // are appended separately by progressive infill.
        assert_eq!(sites.len(), MAX_FRONTAGE_GRID_SITES_PER_ROAD);
    }

    #[test]
    fn frontage_gap_default_layout_is_exactly_the_legacy_candidate_sequence() {
        let engine = Engine::new(71);
        let a = Node {
            id: 10,
            x: 0.0,
            z: 0.0,
            kind: "local".into(),
        };
        let b = Node {
            id: 20,
            x: 0.0,
            z: 100.0,
            kind: "local".into(),
        };
        let context = CollisionContext {
            parcel_footprints: Vec::new(),
            open_roads: Vec::new(),
            pending_roads: Vec::new(),
        };
        let save = engine.save_json();
        let actual = engine.frontage_gap_candidates(&a, &b, &context, &[]);
        let legacy_intervals = [
            (9.0_f32, 20.833333_f32),
            (23.033333_f32, 34.866665_f32),
            (37.066666_f32, 48.9_f32),
            (51.1_f32, 62.933334_f32),
            (65.13333_f32, 76.96667_f32),
            (79.16667_f32, 91.0_f32),
        ];
        let mut expected = Vec::new();
        for side in [-1_i8, 1] {
            for (start, end) in legacy_intervals {
                let along_m = (start + end) * 0.5;
                let along = along_m / 100.0;
                expected.push((
                    (
                        -side as f32 * FRONTAGE_GAP_OFFSET,
                        along_m,
                        frontage_access_node(&a, &b, along),
                        along,
                        side,
                    ),
                    end - start,
                ));
            }
        }
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected.iter()) {
            assert!((actual.0 .0 - expected.0 .0).abs() < 1e-4);
            assert!((actual.0 .1 - expected.0 .1).abs() < 1e-4);
            assert_eq!(actual.0 .2, expected.0 .2);
            assert!((actual.0 .3 - expected.0 .3).abs() < 1e-5);
            assert_eq!(actual.0 .4, expected.0 .4);
            assert!((actual.1 - expected.1).abs() < 1e-4);
        }
        assert_eq!(
            engine.save_json(),
            save,
            "candidate generation is read-only"
        );
    }

    #[test]
    fn shallow_frontage_fallback_preserves_existing_prefix_and_bounded_shapes() {
        let engine = Engine::restore_json(include_str!(
            "../../validation/spine-ranking-seed-87231-by-sites-false.json"
        ))
        .expect("mature fixture restores");
        let context = engine.collision_context();
        let towers = engine.available_station_tower_envelopes();
        let road = engine
            .state
            .roads
            .iter()
            .find(|road| road.stage == "open" && road.connected && road.kind == "local")
            .unwrap();
        let (a, b) = (
            engine.node(road.from).unwrap(),
            engine.node(road.to).unwrap(),
        );
        let (grid_width, grid_depth, grid_spacing) = if engine.is_neighborhood_plan_road(road.id) {
            (
                NEIGHBORHOOD_PARCEL_SIZE,
                NEIGHBORHOOD_PARCEL_SIZE,
                NEIGHBORHOOD_FRONTAGE_SPACING,
            )
        } else {
            (14.0, 12.0, FRONTAGE_SLOT_SPACING)
        };
        let mut grid =
            engine.frontage_candidate_sites_with_layout(a, b, grid_width, grid_depth, grid_spacing);
        let tower_candidate = engine.station_tower_candidate_index(road.id, a, b, &grid);
        if let Some(index) = tower_candidate {
            grid.swap(0, index);
        }
        let standard = engine.frontage_gap_candidates(a, b, &context, &towers);
        let shallow = engine.frontage_gap_candidates_with_layout(
            a,
            b,
            &context,
            &towers,
            FRONTAGE_SHALLOW_DEPTH,
            FRONTAGE_SHALLOW_OFFSET,
        );
        assert!(shallow.len() <= 2 * MAX_FRONTAGE_GAP_LOTS_PER_SIDE);
        let (all, dimensions, has_tower_candidate) = engine
            .frontage_candidates_with_shallow_fallback(
                road.id,
                a,
                b,
                &context,
                &towers,
                grid_width,
                grid_depth,
                grid_spacing,
            );
        assert_eq!(has_tower_candidate, tower_candidate.is_some());
        assert!(all.len() <= MAX_FRONTAGE_SITES_PER_ROAD);

        assert_eq!(&all[..grid.len()], grid.as_slice());
        let standard_start = grid.len();
        let old_prefix_len = standard_start + standard.len();
        assert_eq!(
            &all[standard_start..old_prefix_len],
            standard
                .iter()
                .map(|(site, _)| *site)
                .collect::<Vec<_>>()
                .as_slice()
        );
        // The standard grid and gap candidates are an unchanged prefix in
        // production; the shallow recipe only appends after them.
        assert!(old_prefix_len <= all.len());
        for ((_, _, _, _, _), (width, depth)) in all
            .iter()
            .skip(old_prefix_len)
            .zip(dimensions.iter().skip(old_prefix_len))
        {
            assert!((8.0..=14.0).contains(width));
            assert_eq!(*depth, FRONTAGE_SHALLOW_DEPTH);
        }
        let mut shapes = BTreeSet::new();
        for ((x, z, _, _, _), (width, depth)) in all.iter().zip(dimensions.iter()) {
            let key = (
                (x * 10_000.0).round() as i64,
                (z * 10_000.0).round() as i64,
                (width * 10_000.0).round() as i64,
                (depth * 10_000.0).round() as i64,
            );
            assert!(shapes.insert(key), "duplicate candidate shape {key:?}");
        }
        assert!(all.len() <= MAX_FRONTAGE_SITES_PER_ROAD);
    }

    #[test]
    #[ignore = "20k-tick mature-save continuation and construction audit"]
    fn shallow_frontage_mature_continuations_construct_and_replay_pending_lots() {
        use serde_json::json;

        let fixtures = [
            (
                0_u32,
                include_str!("../../validation/spine-ranking-seed-0-by-sites-false.json"),
            ),
            (
                87_231_u32,
                include_str!("../../validation/spine-ranking-seed-87231-by-sites-false.json"),
            ),
        ];
        for (seed, save) in fixtures {
            let mut engine = Engine::restore_json(save).expect("area-policy save restores");
            engine.set_development_rate(16);
            let start_tick = engine.state.tick;
            let initial_roads = engine.state.roads.len();
            let initial_max_parcel_id = engine
                .state
                .parcels
                .iter()
                .map(|parcel| parcel.id)
                .max()
                .unwrap_or(0);
            let mut pending_replay_checked = false;

            for _ in 0..(20_000 / 200) {
                engine.step(200);
                let shallow_ids: BTreeSet<u32> = engine
                    .state
                    .parcels
                    .iter()
                    .filter(|parcel| {
                        parcel.id > initial_max_parcel_id
                            && (parcel.depth - FRONTAGE_SHALLOW_DEPTH).abs() < 1.0e-4
                    })
                    .map(|parcel| parcel.id)
                    .collect();
                if !pending_replay_checked
                    && engine.state.projects.iter().any(|project| {
                        project.kind == "building"
                            && project.progress < 1.0
                            && project
                                .parcel_id
                                .is_some_and(|id| shallow_ids.contains(&id))
                    })
                {
                    let pending_save = engine.save_json();
                    let mut resumed = Engine::restore_json(&pending_save)
                        .expect("pending shallow construction restores");
                    assert_eq!(resumed.save_json(), pending_save);
                    let mut twin = Engine::restore_json(&pending_save).unwrap();
                    resumed.step(400);
                    twin.step(400);
                    assert_eq!(resumed.save_json(), twin.save_json());
                    pending_replay_checked = true;
                }
            }

            assert_eq!(engine.state.tick, start_tick + 20_000);
            let shallow_parcels: Vec<&Parcel> = engine
                .state
                .parcels
                .iter()
                .filter(|parcel| {
                    parcel.id > initial_max_parcel_id
                        && (parcel.depth - FRONTAGE_SHALLOW_DEPTH).abs() < 1.0e-4
                })
                .collect();
            let shallow_ids: BTreeSet<u32> =
                shallow_parcels.iter().map(|parcel| parcel.id).collect();
            let shallow_buildings: Vec<&Building> = engine
                .state
                .buildings
                .iter()
                .filter(|building| shallow_ids.contains(&building.parcel_id))
                .collect();
            let open_shallow = shallow_buildings
                .iter()
                .filter(|building| building.stage == "open")
                .count();
            assert!(
                !shallow_parcels.is_empty(),
                "seed {seed}: no fallback parcel admitted"
            );
            assert!(
                open_shallow > 0,
                "seed {seed}: shallow lots did not construct normally"
            );
            assert!(
                pending_replay_checked,
                "seed {seed}: no pending shallow project was captured"
            );
            assert!(shallow_parcels
                .iter()
                .all(|parcel| engine.node_connected(parcel.access_node)));
            assert!(shallow_buildings
                .iter()
                .all(|building| engine.building_access_segment(building.id).is_some()));
            engine
                .validate()
                .expect("shallow continuation remains valid");

            let summary = json!({
                "seed": seed,
                "start_tick": start_tick,
                "end_tick": engine.state.tick,
                "development_rate": engine.state.development_rate,
                "initial_roads": initial_roads,
                "final_roads": engine.state.roads.len(),
                "road_delta": engine.state.roads.len().saturating_sub(initial_roads),
                "shallow_parcels_added": shallow_parcels.len(),
                "shallow_nominal_envelope_area_m2": shallow_parcels.iter().map(|parcel| (parcel.width * parcel.depth) as f64).sum::<f64>(),
                "shallow_buildings": shallow_buildings.len(),
                "shallow_open_buildings": open_shallow,
                "shallow_construction_projects_pending": engine.state.projects.iter().filter(|project| project.kind == "building" && project.progress < 1.0 && project.parcel_id.is_some_and(|id| shallow_ids.contains(&id))).count(),
                "canonical_pending_project_replay_checked": pending_replay_checked,
                "validation_passed": true,
            });
            let validation_dir =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation");
            std::fs::write(
                validation_dir.join(format!("frontage-shallow-seed-{seed}-summary.json")),
                serde_json::to_vec_pretty(&summary).unwrap(),
            )
            .unwrap();
            std::fs::write(
                validation_dir.join(format!("frontage-shallow-seed-{seed}-native-save.json")),
                engine.save_json(),
            )
            .unwrap();
            std::fs::write(
                validation_dir.join(format!("frontage-shallow-seed-{seed}-snapshot.json")),
                engine.snapshot_json(),
            )
            .unwrap();
            println!("{}", serde_json::to_string(&summary).unwrap());
        }
    }

    #[test]
    #[ignore = "matched mature-world shallow-frontage geometry diagnostic"]
    fn shallow_frontage_gap_diagnostic_on_area_policy_mature_worlds() {
        use serde_json::json;

        let fixtures = [
            (
                0_u32,
                include_str!("../../validation/spine-ranking-seed-0-by-sites-false.json"),
            ),
            (
                87_231_u32,
                include_str!("../../validation/spine-ranking-seed-87231-by-sites-false.json"),
            ),
        ];
        let mut worlds = Vec::new();
        for (seed, save) in fixtures {
            let engine = Engine::restore_json(save).expect("area-policy save restores");
            assert!(engine.state.parcels.len() < MAX_PARCELS);
            assert!(engine.state.buildings.len() < MAX_BUILDINGS);
            let towers = engine.available_station_tower_envelopes();
            let base_context = engine.collision_context();
            let evaluate = |depth: f32, offset: f32| {
                let mut context = base_context.clone();
                let mut generated = 0usize;
                let mut tower_blocked = 0usize;
                let mut geometry_blocked = BTreeMap::<String, usize>::new();
                let mut neighborhood_blocked = 0usize;
                let mut accepted = Vec::new();
                let mut widths = Vec::new();
                let mut roads_with_candidates = BTreeSet::new();
                let mut roads_with_sites = BTreeSet::new();

                for road in &engine.state.roads {
                    if road.stage != "open"
                        || !road.connected
                        || matches!(road.kind.as_str(), "rail" | "bridge")
                    {
                        continue;
                    }
                    let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                        continue;
                    };
                    let proposals = engine.frontage_gap_candidates_with_layout(
                        a, b, &context, &towers, depth, offset,
                    );
                    if !proposals.is_empty() {
                        roads_with_candidates.insert(road.id);
                    }
                    for ((x, z, access_node, along, side), width) in proposals {
                        generated += 1;
                        if width < 8.0 || width > 14.0 || !engine.node_connected(access_node) {
                            *geometry_blocked
                                .entry("invalid_or_disconnected".into())
                                .or_default() += 1;
                            continue;
                        }
                        if Engine::overlaps_station_tower_envelopes(&towers, x, z, width, depth) {
                            tower_blocked += 1;
                            continue;
                        }
                        let rotation =
                            (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
                        let rect = geometry::OrientedRect {
                            x,
                            z,
                            width,
                            depth,
                            rotation,
                        };
                        let (min_x, min_z, max_x, max_z) = rect.bounds();
                        let protected = geometry::OrientedRect {
                            x: -305.0,
                            z: -310.0,
                            width: 90.0,
                            depth: 80.0,
                            rotation: 0.0,
                        };
                        let center_clearance = (width.max(depth) * 0.5 + 4.0).max(9.0);
                        let failed = if !engine.parcel_clear_of_river(x, z, width, depth) {
                            Some("river_or_bank")
                        } else if min_x < MIN + 4.0
                            || min_z < MIN + 4.0
                            || max_x > MAX - 4.0
                            || max_z > MAX - 4.0
                        {
                            Some("world_bounds")
                        } else if geometry::rects_overlap(&rect, &protected, 0.0) {
                            Some("protected_land")
                        } else if context
                            .parcel_footprints
                            .iter()
                            .any(|old| geometry::rects_overlap(&rect, old, 2.0))
                        {
                            Some("existing_parcel_footprint")
                        } else if !engine.interior_plan_allows_parcel(&rect, false) {
                            Some("reserved_interior_access_lot")
                        } else if context.open_roads.iter().any(|road| {
                            geometry::rect_segment_distance(
                                &rect, road.ax, road.az, road.bx, road.bz,
                            ) < 5.0
                        }) {
                            Some("open_road_footprint_clearance")
                        } else if context.pending_roads.iter().any(|road| {
                            geometry::rect_segment_distance(
                                &rect, road.ax, road.az, road.bx, road.bz,
                            ) < 7.0
                        }) {
                            Some("pending_road_footprint_clearance")
                        } else if !engine.neighborhood_corridor_clear_for_parcel(
                            x,
                            z,
                            max_x - min_x,
                            max_z - min_z,
                        ) {
                            Some("reserved_neighborhood_corridor")
                        } else if context.open_roads.iter().any(|road| {
                            point_segment_distance(x, z, road.ax, road.az, road.bx, road.bz)
                                < center_clearance
                        }) {
                            Some("legacy_open_road_center_clearance")
                        } else if context.pending_roads.iter().any(|road| {
                            point_segment_distance(x, z, road.ax, road.az, road.bx, road.bz)
                                < center_clearance + 2.0
                        }) {
                            Some("pending_road_center_clearance")
                        } else {
                            None
                        };
                        let failed = failed.or_else(|| {
                            (!engine.progressive_frontage_geometry_clear(&rect, &context))
                                .then_some("central_progressive_admission_predicate")
                        });
                        if let Some(reason) = failed {
                            *geometry_blocked.entry(reason.into()).or_default() += 1;
                            continue;
                        }
                        let (mut local_open, mut local_active) = (0usize, 0usize);
                        for building in engine.state.buildings.iter().filter(|building| {
                            building.stage == "open"
                                && (building.x - x).hypot(building.z - z) <= 78.0
                        }) {
                            local_open += 1;
                            local_active += usize::from(site_active(building));
                        }
                        if local_open < 4 || local_active < 3 {
                            neighborhood_blocked += 1;
                            continue;
                        }

                        // Mirror sequential admission: accepted compact lots
                        // immediately reserve their exact oriented envelopes
                        // against every later proposal in this diagnostic.
                        context.parcel_footprints.push(rect);
                        roads_with_sites.insert(road.id);
                        widths.push(width);
                        accepted.push(json!({
                            "road_id": road.id,
                            "x": x,
                            "z": z,
                            "width": width,
                            "depth": depth,
                            "rotation": rotation,
                            "access_node": access_node,
                            "along": along,
                            "side": side,
                        }));
                    }
                }
                widths.sort_by(f32::total_cmp);
                json!({
                    "depth": depth,
                    "center_offset": offset,
                    "generated_proposals": generated,
                    "roads_with_proposals": roads_with_candidates.len(),
                    "greedy_admissible_sites": accepted.len(),
                    "roads_with_admissible_sites": roads_with_sites.len(),
                    "nominal_envelope_area_m2": widths.iter().map(|w| (*w * depth) as f64).sum::<f64>(),
                    "width_min_m": widths.first().copied(),
                    "width_median_m": widths.get(widths.len() / 2).copied(),
                    "width_max_m": widths.last().copied(),
                    "blocked_by_tower_reservations": tower_blocked,
                    "blocked_by_geometry_or_access": geometry_blocked.values().sum::<usize>(),
                    "geometry_rejection_reasons": geometry_blocked,
                    "blocked_by_local_building_demand": neighborhood_blocked,
                    "sample_sites": accepted.into_iter().take(12).collect::<Vec<_>>(),
                })
            };
            worlds.push(json!({
                "seed": seed,
                "tick": engine.state.tick,
                "buildings": engine.state.buildings.len(),
                "parcels": engine.state.parcels.len(),
                "roads": engine.state.roads.len(),
                "demand": engine.state.demand,
                "baseline_12m_depth_15m_offset": evaluate(12.0, 15.0),
                "shallow_8m_depth_12m_offset": evaluate(8.0, 12.0),
            }));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"worlds": worlds})).unwrap()
        );
    }

    #[test]
    fn progressive_gap_infill_admits_compact_frontage_with_access_and_replay() {
        let mut engine = Engine::restore_json(include_str!(
            "../../validation/mature-world.cache-candidate.json"
        ))
        .expect("mature fixture restores");
        let context = engine.collision_context();
        let tower_envelopes = engine.available_station_tower_envelopes();
        let mut target = None;

        'roads: for (road_index, road) in engine.state.roads.iter().enumerate() {
            if road.stage != "open"
                || !road.connected
                || matches!(road.kind.as_str(), "rail" | "bridge")
            {
                continue;
            }
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                continue;
            };
            let grid = engine.frontage_candidate_sites_with_layout(
                a,
                b,
                14.0,
                12.0,
                FRONTAGE_SLOT_SPACING,
            );
            let gaps = engine.frontage_gap_candidates(a, b, &context, &tower_envelopes);
            for (gap_index, ((x, z, access_node, along, side), width)) in
                gaps.iter().copied().enumerate()
            {
                if !(8.0..14.0).contains(&width)
                    || Engine::overlaps_station_tower_envelopes(&tower_envelopes, x, z, width, 12.0)
                {
                    continue;
                }
                let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
                let rect = geometry::OrientedRect {
                    x,
                    z,
                    width,
                    depth: 12.0,
                    rotation,
                };
                if !engine.progressive_frontage_geometry_clear(&rect, &context) {
                    continue;
                }
                let (mut local_open, mut local_active) = (0usize, 0usize);
                for building in engine.state.buildings.iter().filter(|building| {
                    building.stage == "open" && (building.x - x).hypot(building.z - z) <= 78.0
                }) {
                    local_open += 1;
                    local_active += usize::from(site_active(building));
                }
                if local_open < 4 || local_active < 3 {
                    continue;
                }
                let road_length = (b.x - a.x).hypot(b.z - a.z);
                let left_flanker = engine
                    .state
                    .parcel_frontages
                    .iter()
                    .filter(|frontage| {
                        frontage.road_id == road.id
                            && frontage.side == side
                            && frontage.along < along
                    })
                    .map(|frontage| along - frontage.along)
                    .min_by(f32::total_cmp);
                let right_flanker = engine
                    .state
                    .parcel_frontages
                    .iter()
                    .filter(|frontage| {
                        frontage.road_id == road.id
                            && frontage.side == side
                            && frontage.along > along
                    })
                    .map(|frontage| frontage.along - along)
                    .min_by(f32::total_cmp);
                if !left_flanker.is_some_and(|distance| distance * road_length <= 32.0)
                    || !right_flanker.is_some_and(|distance| distance * road_length <= 32.0)
                {
                    continue;
                }
                target = Some((
                    road_index,
                    road.id,
                    grid.len() + gap_index,
                    x,
                    z,
                    width,
                    access_node,
                    along,
                    side,
                    rotation,
                ));
                break 'roads;
            }
        }

        let (road_index, road_id, slot, x, z, width, access_node, along, side, rotation) =
            target.expect("mature active frontage has an admissible compact gap lot");
        engine.state.frontage_road_cursor = road_index;
        engine.state.frontage_slot_cursor = slot as u16;
        assert!(slot < MAX_FRONTAGE_SITES_PER_ROAD);
        let old_parcels = engine.state.parcels.len();
        assert!(engine.try_progressive_frontage_infill());
        assert_eq!(engine.state.parcels.len(), old_parcels + 1);
        assert_eq!(engine.state.frontage_road_cursor, road_index);
        assert_eq!(engine.state.frontage_slot_cursor as usize, slot + 1);

        let parcel = engine.state.parcels.last().unwrap().clone();
        assert!((parcel.x - x).hypot(parcel.z - z) < 1e-4);
        assert_eq!(parcel.width, width);
        assert_eq!(parcel.depth, 12.0);
        assert!((8.0..14.0).contains(&parcel.width));
        assert_eq!(parcel.rotation, Some(rotation));
        let frontage = engine.state.parcel_frontages.last().unwrap();
        assert_eq!(frontage.parcel_id, parcel.id);
        assert_eq!(frontage.road_id, road_id);
        assert_eq!(parcel.access_node, access_node);
        assert!((frontage.along - along).abs() < 1e-5);
        assert_eq!(frontage.side, side);
        assert!(engine.node_connected(parcel.access_node));

        let building = engine.add_building(
            parcel.id,
            "home",
            2,
            "open",
            1.0,
            8,
            "Gap frontage home",
            false,
            vec!["Built into an existing street gap".into()],
        );
        assert!(engine.building_access_segment(building).is_some());
        engine.validate().unwrap();

        let save = engine.save_json();
        let mut resumed = Engine::restore_json(&save).unwrap();
        assert_eq!(resumed.save_json(), save);
        engine.step(200);
        resumed.step(200);
        assert_eq!(engine.save_json(), resumed.save_json());
        engine.validate().unwrap();
    }

    #[test]
    fn frontage_scan_skips_unfinished_road_without_opening_it_and_replays() {
        let mut engine = Engine::restore_json(include_str!(
            "../../validation/mature-world.cache-candidate.json"
        ))
        .expect("mature fixture restores");
        let collision_context = engine.collision_context();
        let tower_envelopes = engine.available_station_tower_envelopes();
        let mut target = None;

        for road in &engine.state.roads {
            if road.stage != "open"
                || !road.connected
                || matches!(road.kind.as_str(), "rail" | "bridge")
            {
                continue;
            }
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                continue;
            };
            let mut candidates = engine.frontage_candidate_sites_with_layout(
                a,
                b,
                14.0,
                12.0,
                FRONTAGE_SLOT_SPACING,
            );
            let tower_candidate = engine.station_tower_candidate_index(road.id, a, b, &candidates);
            if let Some(index) = tower_candidate {
                candidates.swap(0, index);
            }
            let Some(&(x, z, access_node, along, side)) = candidates.first() else {
                continue;
            };
            let tower_site = tower_candidate.is_some();
            if tower_site && !engine.state.station_online {
                continue;
            }
            if !tower_site
                && Engine::overlaps_station_tower_envelopes(&tower_envelopes, x, z, 14.0, 12.0)
            {
                continue;
            }
            let (parcel_x, parcel_z, parcel_width, parcel_depth) = if tower_site {
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let length = dx.hypot(dz).max(1.0);
                let nx = -dz / length;
                let nz = dx / length;
                (
                    a.x + dx * along + nx * side as f32 * STATION_TOWER_ROAD_OFFSET,
                    a.z + dz * along + nz * side as f32 * STATION_TOWER_ROAD_OFFSET,
                    STATION_TOWER_PARCEL_WIDTH,
                    STATION_TOWER_PARCEL_DEPTH,
                )
            } else {
                (x, z, 14.0, 12.0)
            };
            let geometry_clear = if tower_site {
                engine.parcel_geometry_clear(parcel_x, parcel_z, parcel_width, parcel_depth)
            } else {
                let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
                engine.progressive_frontage_geometry_clear(
                    &geometry::OrientedRect {
                        x: parcel_x,
                        z: parcel_z,
                        width: parcel_width,
                        depth: parcel_depth,
                        rotation,
                    },
                    &collision_context,
                )
            };
            if !geometry_clear {
                continue;
            }
            let nearby: Vec<_> = engine
                .state
                .buildings
                .iter()
                .filter(|building| {
                    building.stage == "open"
                        && (building.x - parcel_x).hypot(building.z - parcel_z) <= 78.0
                })
                .collect();
            if nearby.len() < 4 || nearby.iter().filter(|b| site_active(b)).count() < 3 {
                continue;
            }
            target = Some((road.id, x, z, access_node, along, side));
            break;
        }
        let (target_road_id, _, _, _, _, _) =
            target.expect("mature city has an eligible later frontage position");

        let blocked_from = engine.add_node(350.0, 350.0, "junction");
        let blocked_to = engine.add_node(370.0, 350.0, "junction");
        let blocked_road_id = engine.add_road(blocked_from, blocked_to, "local", "survey", 0.5);
        assert_eq!(engine.state.roads.last().unwrap().id, blocked_road_id);
        assert!(!engine.state.roads.last().unwrap().connected);
        let blocked_road = engine.state.roads.pop().unwrap();
        let target_index = engine
            .state
            .roads
            .iter()
            .position(|road| road.id == target_road_id)
            .unwrap();
        let target_road = engine.state.roads.remove(target_index);
        engine.state.roads.insert(0, blocked_road);
        engine.state.roads.insert(1, target_road);
        engine.state.frontage_road_cursor = 0;
        engine.state.frontage_slot_cursor = 0;

        let parcels_before = engine.state.parcels.len();
        assert!(engine.try_progressive_frontage_infill());
        assert_eq!(engine.state.parcels.len(), parcels_before + 1);
        assert_eq!(engine.state.frontage_road_cursor, 1);
        assert_eq!(engine.state.frontage_slot_cursor, 1);
        let admitted = engine.state.parcels.last().unwrap();
        let frontage = engine
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == admitted.id)
            .unwrap();
        assert_eq!(frontage.road_id, target_road_id);
        assert!(engine.node_connected(admitted.access_node));

        let blocked = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == blocked_road_id)
            .unwrap();
        assert_eq!(blocked.stage, "survey");
        assert!(!blocked.connected);
        assert!(!engine.node_connected(blocked_from));
        assert!(!engine.node_connected(blocked_to));
        assert!(engine.route(blocked_from, blocked_to, false).is_empty());
        engine.validate().unwrap();

        let save = engine.save_json();
        let mut resumed = Engine::restore_json(&save).unwrap();
        assert_eq!(resumed.save_json(), save);
        engine.step(200);
        resumed.step(200);
        assert_eq!(engine.save_json(), resumed.save_json());
        assert_eq!(
            resumed
                .state
                .roads
                .iter()
                .find(|road| road.id == blocked_road_id)
                .unwrap()
                .stage,
            "survey"
        );
        assert!(!resumed.node_connected(blocked_from));
    }

    #[test]
    fn frontage_access_selection_is_projection_based_and_orientation_stable() {
        let engine = Engine::new(53);
        // This real midpoint geometry exposed a platform-dependent hypot tie:
        // the side offset made one endpoint appear about 2e-5m closer in WASM.
        let a = Node {
            id: 274,
            x: 56.069_992,
            z: 164.448_24,
            kind: "junction".into(),
        };
        let b = Node {
            id: 753,
            x: 117.672_96,
            z: 219.651_73,
            kind: "junction".into(),
        };
        assert_eq!(frontage_access_node(&a, &b, 0.25), a.id);
        assert_eq!(frontage_access_node(&a, &b, 0.75), b.id);
        assert_eq!(frontage_access_node(&a, &b, 0.5), a.id.min(b.id));
        assert_eq!(frontage_access_node(&b, &a, 0.5), a.id.min(b.id));

        let sites = engine.frontage_candidate_sites(&a, &b);
        let midpoint = sites
            .iter()
            .find(|site| site.4 == 1 && site.3 == 0.5)
            .expect("the positive-side midpoint is an eligible frontage site");
        assert_eq!(midpoint.2, a.id.min(b.id));

        let reversed = engine.frontage_candidate_sites(&b, &a);
        let reversed_midpoint = reversed
            .iter()
            .find(|site| site.4 == -1 && site.3 == 0.5)
            .expect("reversing a road preserves the same physical midpoint side");
        assert_eq!(reversed_midpoint.2, a.id.min(b.id));
        assert!((midpoint.0 - reversed_midpoint.0).abs() < 0.0001);
        assert!((midpoint.1 - reversed_midpoint.1).abs() < 0.0001);
    }

    #[test]
    fn legacy_city_save_can_unlock_progressive_frontage() {
        let mut engine = Engine::new(491);
        engine.state.demand = 2.0;
        let first_candidate = engine
            .state
            .roads
            .iter()
            .enumerate()
            .filter(|(_, road)| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .find_map(|(road_index, road)| {
                let a = engine.node(road.from)?;
                let b = engine.node(road.to)?;
                engine
                    .frontage_candidate_sites(a, b)
                    .into_iter()
                    .find(|(x, z, _, along, side)| {
                        (*along - 0.5).abs() >= 0.001
                            && *side == -1
                            && engine.parcel_geometry_clear(*x, *z, 14.0, 12.0)
                    })
                    .map(|site| (road_index, road.id, site))
            })
            .expect("found an available off-center frontage slot");
        let (_, _, (x, z, _, _, _)) = first_candidate;
        let template = engine.state.buildings[0].clone();
        let access_node = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == template.parcel_id)
            .unwrap()
            .access_node;
        for index in 0..7 {
            let mut nearby = template.clone();
            nearby.id = 100_000 + index;
            nearby.x = x + index as f32 * 0.2;
            nearby.z = z + index as f32 * 0.2;
            nearby.occupancy = 1;
            nearby.parcel_id = engine.add_parcel(
                -355.0 + index as f32 * 16.0,
                -340.0,
                8.0,
                8.0,
                access_node,
                0.5,
                false,
            );
            engine.state.buildings.push(nearby);
        }

        let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        let state = legacy.as_object_mut().unwrap();
        state.remove("frontage_road_cursor");
        state.remove("frontage_slot_cursor");
        let mut restored = Engine::restore_json(&legacy.to_string()).unwrap();
        let old_parcels = restored.state.parcels.len();
        assert!(restored.try_progressive_frontage_infill());
        assert_eq!(restored.state.parcels.len(), old_parcels + 1);
        assert!(restored
            .state
            .parcel_frontages
            .iter()
            .any(|frontage| frontage.road_id == first_candidate.1));
        let infill = restored
            .state
            .parcels
            .iter()
            .find(|parcel| !engine.state.parcels.iter().any(|old| old.id == parcel.id))
            .expect("the admitted frontage lot is present");
        let infill_id = infill.id;
        let rotation = infill
            .rotation
            .expect("progressive frontage stores its orientation");
        let facing = restored.frontage_rotation(infill_id);
        assert!((rotation.sin() - facing.sin()).abs() < 1e-5);
        assert!((rotation.cos() - facing.cos()).abs() < 1e-5);
        restored.validate().unwrap();

        let save = restored.save_json();
        let mut resumed = Engine::restore_json(&save).unwrap();
        assert_eq!(resumed.save_json(), save);
        restored.step(200);
        resumed.step(200);
        assert_eq!(restored.save_json(), resumed.save_json());
    }

    #[test]
    fn rotated_progressive_frontage_uses_road_facing_footprint() {
        let mut engine = Engine::new(904);
        let a_id = engine.add_node(200.0, 200.0, "junction");
        let b_id = engine.add_node(280.0, 280.0, "junction");
        let road_id = engine.add_road(a_id, b_id, "local", "open", 1.0);
        engine.state.roads.last_mut().unwrap().connected = true;
        let (a, b) = (
            engine.node(a_id).unwrap().clone(),
            engine.node(b_id).unwrap().clone(),
        );
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz);
        let nx = -dz / length;
        let nz = dx / length;
        let along = 0.3;
        let side = 1_i8;
        let existing_x = a.x + dx * along + nx * side as f32 * 15.0;
        let existing_z = a.z + dz * along + nz * side as f32 * 15.0;
        let rotation = (side as f32 * dz).atan2(-side as f32 * dx);
        let existing_id = engine.add_parcel(existing_x, existing_z, 14.0, 12.0, a_id, 0.8, false);
        engine.state.parcel_frontages.push(ParcelFrontage {
            parcel_id: existing_id,
            road_id,
            along,
            side,
        });
        engine
            .state
            .parcels
            .iter_mut()
            .find(|p| p.id == existing_id)
            .unwrap()
            .rotation = Some(rotation);

        let candidate_along = along + 16.0 / length;
        let candidate_x = a.x + dx * candidate_along + nx * side as f32 * 15.0;
        let candidate_z = a.z + dz * candidate_along + nz * side as f32 * 15.0;
        let oriented = geometry::OrientedRect {
            x: candidate_x,
            z: candidate_z,
            width: 14.0,
            depth: 12.0,
            rotation,
        };
        let context = engine.collision_context();
        assert!(engine.progressive_frontage_geometry_clear(&oriented, &context));

        let legacy_axis = geometry::OrientedRect {
            rotation: 0.0,
            ..oriented
        };
        assert!(
            !engine.progressive_frontage_geometry_clear(&legacy_axis, &context),
            "the same frontage slot must not be admitted with an axis-aligned footprint"
        );

        let overlapping = geometry::OrientedRect {
            x: a.x + dx * (along + 12.0 / length) + nx * side as f32 * 15.0,
            z: a.z + dz * (along + 12.0 / length) + nz * side as f32 * 15.0,
            ..oriented
        };
        assert!(!engine.progressive_frontage_geometry_clear(&overlapping, &context));
    }

    #[test]
    fn frontage_grid_keeps_midpoint_spacing_clear_and_endpoint_bounded() {
        let engine = Engine::new(41);
        let a = Node {
            id: 90_001,
            x: 10.0,
            z: 300.0,
            kind: "junction".into(),
        };
        let b = Node {
            id: 90_002,
            x: 90.0,
            z: 300.0,
            kind: "junction".into(),
        };
        let candidates = engine.frontage_candidate_sites(&a, &b);
        let mut along: Vec<f32> = candidates
            .iter()
            .filter(|candidate| candidate.4 == 1)
            .map(|candidate| candidate.3)
            .collect();
        along.sort_by(f32::total_cmp);
        assert_eq!(along.len(), 3);
        assert!(along.iter().any(|value| (*value - 0.5).abs() < 0.0001));
        for pair in along.windows(2) {
            let spacing = (pair[1] - pair[0]) * 80.0;
            assert!((spacing - FRONTAGE_SLOT_SPACING).abs() < 0.001);
        }
        assert!(along.first().unwrap() * 80.0 >= FRONTAGE_ENDPOINT_CLEARANCE);
        assert!((1.0 - along.last().unwrap()) * 80.0 >= FRONTAGE_ENDPOINT_CLEARANCE);
    }

    fn engine_with_clear_local_chord() -> (Engine, u32, u32) {
        let mut engine = Engine::new(87231);
        engine.state.nodes.clear();
        engine.state.roads.clear();
        engine.state.parcels.clear();
        engine.state.parcel_frontages.clear();
        engine.state.buildings.clear();
        engine.state.projects.clear();
        engine.state.travelers.clear();
        engine.state.residents.clear();
        engine.state.trains.clear();
        engine.state.buses.clear();
        engine.state.bus_line = None;
        engine.state.events.clear();
        engine.state.population = 300;
        engine.state.demand = 2.0;
        engine.state.station_online = false;
        engine.state.station_ready = false;
        engine.state.road_origin_cursor = 0;

        let gateway = engine.add_node(365.0, 360.0, "gateway");
        let hub = engine.add_node(30.0, 400.0, "junction");
        let start = engine.add_node(0.0, 300.0, "junction");
        let target = engine.add_node(60.0, 300.0, "junction");
        engine.add_road(gateway, hub, "local", "open", 1.0);
        engine.add_road(hub, start, "local", "open", 1.0);
        engine.add_road(hub, target, "local", "open", 1.0);
        engine.recompute_connectivity();

        // A compact, occupied block sits south of the clear 60 m chord. These
        // real parcels establish local demand without obstructing the corridor.
        for row in 0..2 {
            for column in 0..6 {
                let x = column as f32 * 12.0;
                let z = 230.0 + row as f32 * 12.0;
                let parcel_id = engine.add_parcel(x, z, 8.0, 8.0, hub, 0.85, false);
                let building_id = engine.add_building(
                    parcel_id,
                    "home",
                    2,
                    "open",
                    1.0,
                    10,
                    "Test Machiya",
                    false,
                    vec!["Occupied beside an active local lane".into()],
                );
                engine
                    .state
                    .buildings
                    .iter_mut()
                    .find(|building| building.id == building_id)
                    .unwrap()
                    .occupancy = 4;
            }
        }
        (engine, start, target)
    }

    fn engine_with_developed_plan_site() -> Engine {
        let mut engine = Engine::new(41);
        engine.state.nodes.clear();
        engine.state.roads.clear();
        engine.state.parcels.clear();
        engine.state.parcel_frontages.clear();
        engine.state.buildings.clear();
        engine.state.projects.clear();
        engine.state.travelers.clear();
        engine.state.residents.clear();
        engine.state.trains.clear();
        engine.state.buses.clear();
        engine.state.bus_line = None;
        engine.state.events.clear();
        engine.state.population = 80;
        engine.state.demand = 2.0;
        engine.state.station_online = false;
        engine.state.station_ready = false;
        engine.state.neighborhood_plan = None;
        engine.state.neighborhood_anchor_cursor = 0;

        let gateway = engine.add_node(365.0, 360.0, "gateway");
        let feeder = engine.add_node(310.0, 315.0, "junction");
        let anchor = engine.add_node(250.0, 250.0, "junction");
        engine.add_road(gateway, feeder, "local", "open", 1.0);
        engine.add_road(feeder, anchor, "local", "open", 1.0);
        engine.recompute_connectivity();

        // Real occupied frontage creates local demand west of the prospective
        // block while leaving the block's street corridors and lots clear.
        for row in 0..3 {
            for column in 0..4 {
                let x = 142.0 + column as f32 * 11.0;
                let z = 188.0 + row as f32 * 11.0;
                let parcel = engine.add_parcel(x, z, 7.0, 7.0, anchor, 0.8, false);
                let building = engine.add_building(
                    parcel,
                    "home",
                    2,
                    "open",
                    1.0,
                    8,
                    "Occupied machiya",
                    false,
                    vec!["A lived-in street edge".into()],
                );
                engine
                    .state
                    .buildings
                    .iter_mut()
                    .find(|candidate| candidate.id == building)
                    .unwrap()
                    .occupancy = 4;
            }
        }
        engine
    }

    fn engine_with_developed_plan_site_without_reusable_edge() -> Engine {
        let mut engine = engine_with_developed_plan_site();
        // Keep the same occupied catchment, but move the feeder junction so
        // both existing roads fall outside the shared-edge length bands.
        let feeder_id = engine.state.roads[1].from;
        let feeder = engine
            .state
            .nodes
            .iter_mut()
            .find(|node| node.id == feeder_id)
            .unwrap();
        feeder.x = 320.0;
        feeder.z = 320.0;
        engine.recompute_connectivity();
        engine
    }

    #[test]
    fn spine_candidate_validation_and_score_include_deep_road_cost() {
        let corners = [
            WorldPoint { x: 250.0, z: 250.0 },
            WorldPoint { x: 332.0, z: 250.0 },
            WorldPoint { x: 332.0, z: 332.0 },
            WorldPoint { x: 250.0, z: 332.0 },
        ];
        let corners_block = corners.map(|point| blocks::BlockPoint {
            x: point.x,
            z: point.z,
        });
        let deep = spine_plan::SpinePlan {
            corners: corners_block,
            recipe_version: spine_plan::RECIPE_VERSION,
            stage: spine_plan::SpineStage::Reserved,
        };
        let lots = Engine::spine_candidate_lots(&deep).expect("82m square has a canonical spine");
        assert!(lots.len() >= 4);
        let deep_candidate = Engine::ranked_neighborhood_candidate(
            1,
            corners,
            [Some(1), None, None, None],
            None,
            None,
            &lots,
            [1, 3, 0],
            Some(deep.clone()),
        );
        assert_eq!(deep_candidate.score.new_road_length_cm, 41_000);
        assert_eq!(deep_candidate.score.new_road_count, 7);

        let shallow_corners = [
            corners[0],
            corners[1],
            WorldPoint { x: 332.0, z: 300.0 },
            WorldPoint { x: 250.0, z: 300.0 },
        ];
        let shallow = blocks::subdivide_rectangular_block(
            shallow_corners.map(|point| blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            8.0,
            11.0,
            2.2,
        );
        let conventional_candidate = Engine::ranked_neighborhood_candidate(
            1,
            shallow_corners,
            [Some(1), None, None, None],
            None,
            None,
            &shallow,
            [1, 1, 0],
            None,
        );
        assert!(development::neighborhood_score_is_better(
            conventional_candidate.score,
            deep_candidate.score,
        ));
        let shallow_as_spine = spine_plan::SpinePlan {
            corners: shallow_corners.map(|point| blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            recipe_version: spine_plan::RECIPE_VERSION,
            stage: spine_plan::SpineStage::Reserved,
        };
        assert!(shallow_as_spine.validate().is_err());

        let mut engine = engine_with_developed_plan_site_without_reusable_edge();
        let planner_anchor = engine.node(engine.state.nodes[2].id).unwrap().clone();
        let collision = engine.collision_context();
        let tower_envelopes = engine.available_station_tower_envelopes();
        let mut feasible_spines = 0;
        let mut feasible_example = None;
        for (ux, uz, vx, vz) in [
            (1.0_f32, 0.0_f32, 0.0_f32, 1.0_f32),
            (1.0, 0.0, 0.0, -1.0),
            (-1.0, 0.0, 0.0, 1.0),
            (-1.0, 0.0, 0.0, -1.0),
            (0.0, 1.0, 1.0, 0.0),
            (0.0, 1.0, -1.0, 0.0),
            (0.0, -1.0, 1.0, 0.0),
            (0.0, -1.0, -1.0, 0.0),
        ] {
            let candidate_corners = [
                WorldPoint {
                    x: planner_anchor.x,
                    z: planner_anchor.z,
                },
                WorldPoint {
                    x: planner_anchor.x + ux * 82.0,
                    z: planner_anchor.z + uz * 82.0,
                },
                WorldPoint {
                    x: planner_anchor.x + ux * 82.0 + vx * 82.0,
                    z: planner_anchor.z + uz * 82.0 + vz * 82.0,
                },
                WorldPoint {
                    x: planner_anchor.x + vx * 82.0,
                    z: planner_anchor.z + vz * 82.0,
                },
            ];
            let plan = spine_plan::SpinePlan {
                corners: candidate_corners.map(|point| blocks::BlockPoint {
                    x: point.x,
                    z: point.z,
                }),
                recipe_version: spine_plan::RECIPE_VERSION,
                stage: spine_plan::SpineStage::Reserved,
            };
            if engine.neighborhood_candidate_clear_with_layout(
                &planner_anchor,
                &candidate_corners,
                &tower_envelopes,
                &collision,
                [Some(planner_anchor.id), None, None, None],
                None,
                None,
                Some(&plan),
            ) {
                feasible_spines += 1;
                feasible_example = Some((candidate_corners, plan));
            }
        }
        assert!(
            feasible_spines > 0,
            "developed catchment has a real feasible deep candidate"
        );
        let (valid_corners, valid_plan) = feasible_example.unwrap();
        let before = engine.save_json();
        let accepts = |engine: &Engine, plan: &spine_plan::SpinePlan| {
            engine.neighborhood_candidate_clear_with_layout(
                &planner_anchor,
                &valid_corners,
                &tower_envelopes,
                &collision,
                [Some(planner_anchor.id), None, None, None],
                None,
                None,
                Some(plan),
            )
        };
        // Establish that every unchanged predicate accepts this exact example.
        assert!(accepts(&engine, &valid_plan));
        let mut mismatched = valid_plan.clone();
        for corner in &mut mismatched.corners {
            corner.x += 1.0;
        }
        assert!(
            mismatched.validate().is_ok(),
            "translation preserves recipe geometry"
        );
        assert!(!accepts(&engine, &mismatched));
        let mut wrong_stage = valid_plan.clone();
        wrong_stage.stage = spine_plan::SpineStage::PerimeterReady {
            host_road_ids: [10, 11],
        };
        assert!(
            wrong_stage.validate().is_ok(),
            "stage IDs are structurally valid"
        );
        assert!(!accepts(&engine, &wrong_stage));
        assert_eq!(
            engine.save_json(),
            before,
            "candidate checks must not mutate the world"
        );
        let prior_next_id = engine.state.next_id;
        engine.state.next_id = u32::MAX - 1;
        let exhausted = engine.save_json();
        assert!(
            !accepts(&engine, &valid_plan),
            "reserve all eventual IDs before admission"
        );
        assert_eq!(engine.save_json(), exhausted);
        engine.state.next_id = prior_next_id;
        assert!(accepts(&engine, &valid_plan));
        assert!(engine.try_create_neighborhood_plan());
        assert!(engine
            .state
            .neighborhood_plan
            .as_ref()
            .unwrap()
            .spine
            .is_none());
    }

    #[test]
    fn neighborhood_plan_reserves_connected_corridors_and_replays_pending_work() {
        let mut engine = engine_with_developed_plan_site_without_reusable_edge();
        assert!(engine.try_create_neighborhood_plan());
        let plan = engine.state.neighborhood_plan.clone().unwrap();
        assert_eq!(plan.segments.len(), 4);
        assert_eq!(plan.next_segment, 0);

        // A lot envelope over a future street is unavailable before any road
        // project opens; this is the reservation that prevents lots sealing
        // the planned block.
        let (from, to) = plan.segments[2];
        let a = engine.node(from).unwrap().clone();
        let b = engine.node(to).unwrap().clone();
        let center = WorldPoint {
            x: (a.x + b.x) * 0.5,
            z: (a.z + b.z) * 0.5,
        };
        assert!(!engine.parcel_geometry_clear(
            center.x,
            center.z,
            NEIGHBORHOOD_PARCEL_SIZE,
            NEIGHBORHOOD_PARCEL_SIZE
        ));

        assert!(engine.plan_next_neighborhood_segment());
        let pending = engine
            .state
            .projects
            .iter()
            .find(|project| project.kind == "road" && project.progress < 1.0)
            .cloned()
            .expect("first plan edge is ordinary road construction");
        assert_eq!(pending.from, Some(plan.segments[0].0));
        assert_eq!(pending.to, Some(plan.segments[0].1));
        assert!(!engine.state.roads.iter().any(|road| {
            road.from == plan.segments[0].0 && road.to == plan.segments[0].1 && road.stage == "open"
        }));
        let (reserved_from, reserved_to) = plan.segments[2];
        let blocked_a = engine.node(reserved_from).unwrap();
        let blocked_b = engine.node(reserved_to).unwrap();
        let crossing = Node {
            id: u32::MAX - 20,
            x: (blocked_a.x + blocked_b.x) * 0.5,
            z: (blocked_a.z + blocked_b.z) * 0.5 - 30.0,
            kind: "candidate".into(),
        };
        let crossing_end = Node {
            id: u32::MAX - 21,
            x: crossing.x,
            z: crossing.z + 60.0,
            kind: "candidate".into(),
        };
        assert!(!engine.neighborhood_corridor_clear_for_road(
            &crossing,
            &crossing_end,
            crossing.id
        ));
        let (active_from, active_to) = plan.segments[0];
        let active_a = engine.node(active_from).unwrap();
        let active_b = engine.node(active_to).unwrap();
        let dx = active_b.x - active_a.x;
        let dz = active_b.z - active_a.z;
        let length = dx.hypot(dz).max(1.0);
        let active_lot = WorldPoint {
            x: (active_a.x + active_b.x) * 0.5 - dz / length * 15.0,
            z: (active_a.z + active_b.z) * 0.5 + dx / length * 15.0,
        };
        assert!(engine.parcel_geometry_clear(
            active_lot.x,
            active_lot.z,
            NEIGHBORHOOD_PARCEL_SIZE,
            NEIGHBORHOOD_PARCEL_SIZE
        ));

        let save = engine.save_json();
        let mut replay = Engine::restore_json(&save).unwrap();
        assert_eq!(engine.save_json(), replay.save_json());
        let pending_id = pending.id;
        engine
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == pending_id)
            .unwrap()
            .progress = 1.0;
        replay
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == pending_id)
            .unwrap()
            .progress = 1.0;
        engine.complete_project(pending_id);
        replay.complete_project(pending_id);
        assert_eq!(engine.save_json(), replay.save_json());
        let opened = engine
            .state
            .roads
            .iter()
            .find(|road| road.from == plan.segments[0].0 && road.to == plan.segments[0].1)
            .unwrap();
        assert_eq!(opened.stage, "open");
        assert!(opened.connected);
        assert!(engine.state.parcels.iter().any(|parcel| {
            engine
                .state
                .parcel_frontages
                .iter()
                .any(|frontage| frontage.parcel_id == parcel.id && frontage.road_id == opened.id)
        }));

        // A road can only be scheduled after its predecessor is connected.
        assert!(engine.plan_next_neighborhood_segment());
        let second = engine
            .state
            .projects
            .iter()
            .find(|project| project.kind == "road" && project.progress < 1.0)
            .cloned()
            .unwrap();
        assert_eq!(second.from, Some(plan.segments[1].0));
        assert!(engine.node_connected(second.from.unwrap()));

        for segment_index in 1..plan.segments.len() {
            let (from, to) = plan.segments[segment_index];
            let project = if segment_index == 1 {
                second.clone()
            } else {
                assert!(engine.plan_next_neighborhood_segment());
                engine
                    .state
                    .projects
                    .iter()
                    .find(|project| {
                        project.kind == "road"
                            && project.progress < 1.0
                            && project.from == Some(from)
                            && project.to == Some(to)
                    })
                    .cloned()
                    .expect("each next perimeter street is built through a project")
            };
            assert_eq!(project.from, Some(from));
            assert_eq!(project.to, Some(to));
            engine
                .state
                .projects
                .iter_mut()
                .find(|candidate| candidate.id == project.id)
                .unwrap()
                .progress = 1.0;
            engine.complete_project(project.id);
            let road = engine
                .state
                .roads
                .iter()
                .find(|road| road.from == from && road.to == to)
                .unwrap();
            assert_eq!(road.stage, "open");
            assert!(road.connected);
        }
        assert!(engine.state.neighborhood_plan.is_none());
        assert_eq!(engine.state.neighborhood_road_ids.len(), 4);
        assert!(plan.segments.iter().all(|(from, to)| {
            engine
                .state
                .roads
                .iter()
                .find(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                })
                .is_some_and(|road| engine.is_neighborhood_plan_road(road.id))
        }));

        // Legacy saves retained the completed plan itself as their only
        // classification source. Restoring that shape migrates its roads to
        // the historic index and retires the completed active plan once.
        let mut legacy_completed: serde_json::Value =
            serde_json::from_str(&engine.save_json()).unwrap();
        let mut completed_plan = plan.clone();
        completed_plan.next_segment = plan.segments.len();
        legacy_completed["neighborhood_plan"] = serde_json::to_value(&completed_plan).unwrap();
        legacy_completed
            .as_object_mut()
            .unwrap()
            .remove("neighborhood_road_ids");
        let migrated = Engine::restore_json(&legacy_completed.to_string()).unwrap();
        assert!(migrated.state.neighborhood_plan.is_none());
        assert_eq!(migrated.state.neighborhood_road_ids.len(), 4);
        assert!(plan.segments.iter().all(|(from, to)| {
            migrated
                .state
                .roads
                .iter()
                .find(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                })
                .is_some_and(|road| migrated.is_neighborhood_plan_road(road.id))
        }));
        let migrated_save = migrated.save_json();
        assert_eq!(
            migrated_save,
            Engine::restore_json(&migrated_save).unwrap().save_json()
        );

        let accessible_lots: Vec<_> = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| {
                engine.node_connected(parcel.access_node)
                    && engine.state.parcel_frontages.iter().any(|frontage| {
                        frontage.parcel_id == parcel.id
                            && engine
                                .state
                                .roads
                                .iter()
                                .any(|road| road.id == frontage.road_id && road.connected)
                    })
            })
            .collect();
        let plan_center = WorldPoint {
            x: plan
                .segments
                .iter()
                .map(|(from, _)| engine.node(*from).unwrap().x)
                .sum::<f32>()
                * 0.25,
            z: plan
                .segments
                .iter()
                .map(|(from, _)| engine.node(*from).unwrap().z)
                .sum::<f32>()
                * 0.25,
        };
        let mut plan_lot_count = 0;
        for (from, to) in &plan.segments {
            let a = engine.node(*from).unwrap();
            let b = engine.node(*to).unwrap();
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let length = dx.hypot(dz).max(1.0);
            let midpoint = WorldPoint {
                x: (a.x + b.x) * 0.5,
                z: (a.z + b.z) * 0.5,
            };
            let interior_side = if (plan_center.x - midpoint.x) * (-dz / length)
                + (plan_center.z - midpoint.z) * (dx / length)
                >= 0.0
            {
                1
            } else {
                -1
            };
            let road = engine
                .state
                .roads
                .iter()
                .find(|road| road.from == *from && road.to == *to)
                .unwrap();
            let road_lots: Vec<_> = engine
                .state
                .parcel_frontages
                .iter()
                .filter(|frontage| frontage.road_id == road.id)
                .collect();
            plan_lot_count += road_lots.len();
            if length >= 70.0 {
                assert!(
                    road_lots
                        .iter()
                        .filter(|frontage| frontage.side == interior_side)
                        .count()
                        >= 2,
                    "long edge has fewer than two clear inside lots"
                );
            }
        }
        assert!(plan_lot_count >= 12, "plan frontage lots: {plan_lot_count}");
        assert!(
            accessible_lots.len() >= 12,
            "lots: {}",
            accessible_lots.len()
        );
        let developed = accessible_lots[0];
        let building_id = engine.add_building(
            developed.id,
            "home",
            3,
            "open",
            1.0,
            12,
            "New block machiya",
            false,
            vec!["Built on a connected block street".into()],
        );
        assert!(engine.building_access_segment(building_id).is_some());
    }

    #[test]
    fn neighborhood_block_is_reserved_before_frontier_and_starts_next_cadence() {
        let mut engine = engine_with_developed_plan_site_without_reusable_edge();
        let gateway = engine
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "gateway")
            .unwrap()
            .id;
        let outer = engine.add_node(390.0, 370.0, "junction");
        engine.add_road(gateway, outer, "local", "open", 1.0);
        engine.recompute_connectivity();
        engine.state.tick = 80;

        assert_ne!(engine.state.tick % 1_600, 0);
        assert_eq!(engine.state.road_infill_due, 0);
        assert!(!engine.neighborhood_block_has_unbuilt_frontage());
        assert!(engine.state.projects.is_empty());
        assert!(engine.state.neighborhood_plan.is_none());

        engine.evaluate_growth();

        let plan = engine
            .state
            .neighborhood_plan
            .as_ref()
            .expect("a feasible neighborhood is reserved at the ordinary road cadence");
        assert_eq!(
            plan.next_segment, 0,
            "reservation does not start work immediately"
        );
        assert!(!engine
            .state
            .projects
            .iter()
            .any(|project| project.kind == "road" && project.progress < 1.0));

        engine.state.tick = 160;
        engine.evaluate_growth();
        let plan = engine.state.neighborhood_plan.as_ref().unwrap();
        assert_eq!(plan.next_segment, 1);
        assert!(engine
            .state
            .projects
            .iter()
            .any(|project| { project.kind == "road" && project.progress < 1.0 }));
        engine.validate().unwrap();
    }

    #[test]
    fn available_connected_infill_precedes_neighborhood_block_planning() {
        let mut engine = engine_with_developed_plan_site();
        let gateway = engine
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "gateway")
            .unwrap()
            .id;
        let outer = engine.add_node(390.0, 370.0, "junction");
        engine.add_road(gateway, outer, "local", "open", 1.0);
        engine.recompute_connectivity();
        let access_node = engine
            .state
            .nodes
            .iter()
            .find(|node| node.x == 250.0 && node.z == 250.0)
            .unwrap()
            .id;
        let parcel_id = engine.add_parcel(187.0, 199.0, 14.0, 12.0, access_node, 0.8, false);
        assert_ne!(
            parcel_id, 0,
            "the infill parcel must pass geometry admission"
        );
        assert!(engine.node_connected(access_node));
        engine.state.tick = 80;

        engine.evaluate_growth();

        let plan = engine
            .state
            .neighborhood_plan
            .as_ref()
            .expect("the block reservation may coexist with usable infill");
        assert_eq!(
            plan.next_segment, 0,
            "existing infill keeps priority over road work"
        );
        assert!(engine.state.projects.iter().any(|project| {
            project.kind == "building"
                && project.progress < 1.0
                && project.parcel_id == Some(parcel_id)
        }));
        assert!(engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == parcel_id)
            .unwrap()
            .reservation
            .is_some());
        assert!(!engine
            .state
            .projects
            .iter()
            .any(|project| project.kind == "road" && project.progress < 1.0));
    }

    #[test]
    fn coordinated_block_reuses_open_edge_and_waits_for_three_new_streets() {
        let mut base = engine_with_developed_plan_site();
        let mut engine = Engine::restore_json(&base.save_json()).unwrap();
        assert!(engine.try_create_neighborhood_plan());
        let plan = engine.state.neighborhood_plan.clone().unwrap();
        assert_eq!(plan.segments.len(), 4);
        assert_eq!(plan.next_segment, 0);
        let (reuse_from, reuse_to) = plan.segments[0];
        let reused = engine
            .state
            .roads
            .iter()
            .find(|road| {
                road.stage == "open"
                    && ((road.from == reuse_from && road.to == reuse_to)
                        || (road.from == reuse_to && road.to == reuse_from))
            })
            .cloned()
            .expect("the first plan edge reuses an existing open street");
        assert!(reused.connected);
        assert_eq!(
            engine
                .state
                .roads
                .iter()
                .filter(|road| {
                    (road.from == reuse_from && road.to == reuse_to)
                        || (road.from == reuse_to && road.to == reuse_from)
                })
                .count(),
            1,
            "reuse must not duplicate the existing road"
        );

        // The candidate validator must reject occupied interior geometry
        // while accepting the exact same rectangle before that lot exists.
        let corners: [WorldPoint; 4] = plan
            .segments
            .iter()
            .map(|(from, _)| {
                let node = engine.node(*from).unwrap();
                WorldPoint {
                    x: node.x,
                    z: node.z,
                }
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let anchor = base.node(plan.anchor_node).unwrap().clone();
        let tower_envelopes = base.available_station_tower_envelopes();
        let context = base.collision_context();
        let available = base.available_coordinated_block_lots(&corners, &context);
        assert_eq!(available.len(), 10);
        assert!(base.neighborhood_candidate_clear_with_edge(
            &anchor,
            &corners,
            &tower_envelopes,
            &context,
            [Some(reuse_from), Some(reuse_to), None, None],
            Some((0, reused.id)),
            Some(&available),
        ));
        let blocked_lot = *available
            .iter()
            .find(|lot| lot.frontage_edge == 0 && (0.4..0.6).contains(&lot.frontage_t))
            .expect("fixture has a center lot away from the new perimeter roads");
        let occupied_probe = base.add_parcel(
            blocked_lot.center.x,
            blocked_lot.center.z,
            2.0,
            2.0,
            plan.anchor_node,
            0.5,
            false,
        );
        assert_ne!(occupied_probe, 0, "the collision probe must be admitted");
        let occupied_context = base.collision_context();
        let filtered = base.available_coordinated_block_lots(&corners, &occupied_context);
        assert_eq!(filtered.len(), 9, "occupied frontage lots remain untouched");
        assert!(!filtered.contains(&blocked_lot));
        assert!(base.neighborhood_candidate_clear_with_edge(
            &anchor,
            &corners,
            &tower_envelopes,
            &occupied_context,
            [Some(reuse_from), Some(reuse_to), None, None],
            Some((0, reused.id)),
            Some(&filtered),
        ));
        let original_parcel_count = base.state.parcels.len();
        let original_frontages = base.state.parcel_frontages.clone();
        let new_corner_c = base.add_node(corners[2].x, corners[2].z, "junction");
        let new_corner_d = base.add_node(corners[3].x, corners[3].z, "junction");
        let selected_plan = NeighborhoodPlan {
            anchor_node: plan.anchor_node,
            segments: vec![
                (reuse_from, reuse_to),
                (reuse_to, new_corner_c),
                (new_corner_c, new_corner_d),
                (new_corner_d, reuse_from),
            ],
            next_segment: 0,
            coordinated_lots: true,
            selected_lots: Some(filtered.clone()),
            spine: None,
        };
        base.state.neighborhood_plan = Some(selected_plan.clone());
        base.state.neighborhood_road_ids.insert(reused.id);
        let selected = selected_plan.selected_lots.as_ref().unwrap();
        assert!(selected.len() >= 4 && selected.len() < 10);
        assert_eq!(base.state.parcels.len(), original_parcel_count);
        assert_eq!(
            serde_json::to_value(&base.state.parcel_frontages).unwrap(),
            serde_json::to_value(&original_frontages).unwrap()
        );
        assert_eq!(base.coordinated_plan_lot_count(), Some(selected.len()));
        let mut partial_capacity = Engine::restore_json(&base.save_json()).unwrap();
        let reserved_partial_lots = partial_capacity.coordinated_plan_lot_count().unwrap();
        while partial_capacity.state.parcels.len() + reserved_partial_lots < MAX_PARCELS {
            assert_ne!(
                partial_capacity.add_parcel(0.0, 0.0, 1.0, 1.0, plan.anchor_node, 0.5, false),
                0
            );
        }
        assert_eq!(
            partial_capacity.state.parcels.len() + reserved_partial_lots,
            MAX_PARCELS
        );
        assert_eq!(
            partial_capacity.add_parcel(0.0, 0.0, 1.0, 1.0, plan.anchor_node, 0.5, false),
            0,
            "the exact selected-lot count stays reserved until the block opens"
        );
        let partial_save = base.save_json();
        assert_eq!(
            Engine::restore_json(&partial_save).unwrap().save_json(),
            partial_save
        );
        let mut duplicate_layout: serde_json::Value = serde_json::from_str(&partial_save).unwrap();
        let layout = duplicate_layout["neighborhood_plan"]["selected_lots"]
            .as_array_mut()
            .unwrap();
        layout[1] = layout[0].clone();
        assert!(Engine::restore_json(&duplicate_layout.to_string()).is_err());
        let mut noncanonical_layout: serde_json::Value =
            serde_json::from_str(&partial_save).unwrap();
        noncanonical_layout["neighborhood_plan"]["selected_lots"][0]["center"]["x"] =
            serde_json::json!(selected[0].center.x + 0.5);
        assert!(Engine::restore_json(&noncanonical_layout.to_string()).is_err());
        let mut partial_replay = Engine::restore_json(&partial_save).unwrap();
        let partial_lot_count = selected.len();
        for edge in 1..4 {
            for world in [&mut base, &mut partial_replay] {
                assert!(world.plan_next_neighborhood_segment());
                let (from, to) = selected_plan.segments[edge];
                let project = world
                    .state
                    .projects
                    .iter_mut()
                    .find(|project| {
                        project.kind == "road"
                            && project.progress < 1.0
                            && project.from == Some(from)
                            && project.to == Some(to)
                    })
                    .expect("each selected shared-edge perimeter extension is scheduled");
                let id = project.id;
                project.progress = 1.0;
                world.complete_project(id);
            }
            assert_eq!(base.save_json(), partial_replay.save_json());
            let admitted_lots = base
                .state
                .parcels
                .iter()
                .filter(|parcel| parcel.rotation.is_some())
                .count();
            if edge < 3 {
                assert_eq!(
                    admitted_lots, 0,
                    "selected lots wait for the full perimeter"
                );
            } else {
                assert_eq!(admitted_lots, partial_lot_count);
                assert!(base.state.parcels.len() >= original_parcel_count + partial_lot_count);
                for lot in selected {
                    assert_eq!(
                        base.state
                            .parcels
                            .iter()
                            .filter(|parcel| {
                                (parcel.x - lot.center.x).abs() < 1.0e-4
                                    && (parcel.z - lot.center.z).abs() < 1.0e-4
                                    && (parcel.width - lot.width).abs() < 1.0e-4
                                    && (parcel.depth - lot.depth).abs() < 1.0e-4
                            })
                            .count(),
                        1,
                        "each selected parcel is inserted exactly once"
                    );
                }
            }
        }
        assert!(base.state.neighborhood_plan.is_none());
        for frontage in &original_frontages {
            assert!(
                base.state.parcel_frontages.iter().any(|current| {
                    current.parcel_id == frontage.parcel_id
                        && current.road_id == frontage.road_id
                        && (current.along - frontage.along).abs() < 1.0e-5
                        && current.side == frontage.side
                }),
                "pre-existing frontage metadata remains intact"
            );
        }
        for lot in selected {
            let parcel_id = base
                .state
                .parcels
                .iter()
                .find(|parcel| {
                    (parcel.x - lot.center.x).abs() < 1.0e-4
                        && (parcel.z - lot.center.z).abs() < 1.0e-4
                        && (parcel.width - lot.width).abs() < 1.0e-4
                        && (parcel.depth - lot.depth).abs() < 1.0e-4
                })
                .unwrap()
                .id;
            assert!(
                base.state
                    .parcel_frontages
                    .iter()
                    .any(|frontage| frontage.parcel_id == parcel_id),
                "every selected lot has frontage metadata"
            );
        }
        assert!(base.state.parcel_frontages.len() >= original_frontages.len() + partial_lot_count);
        base.validate().unwrap();
        let mut replay = Engine::restore_json(&engine.save_json()).unwrap();
        assert_eq!(engine.save_json(), replay.save_json());
        assert!(engine.plan_next_neighborhood_segment());
        assert!(replay.plan_next_neighborhood_segment());
        assert_eq!(engine.save_json(), replay.save_json());
        assert_eq!(
            engine
                .state
                .neighborhood_plan
                .as_ref()
                .unwrap()
                .next_segment,
            2
        );
        assert!(engine
            .state
            .projects
            .iter()
            .all(|project| { project.from != Some(reuse_from) || project.to != Some(reuse_to) }));
        assert!(engine.state.neighborhood_road_ids.contains(&reused.id));
        assert_eq!(
            engine
                .state
                .parcels
                .iter()
                .filter(|parcel| parcel.rotation.is_some())
                .count(),
            0
        );

        for edge in 1..4 {
            for world in [&mut engine, &mut replay] {
                if edge > 1 {
                    assert!(world.plan_next_neighborhood_segment());
                }
                let (from, to) = plan.segments[edge];
                let project = world
                    .state
                    .projects
                    .iter_mut()
                    .find(|project| {
                        project.kind == "road"
                            && project.progress < 1.0
                            && project.from == Some(from)
                            && project.to == Some(to)
                    })
                    .expect("each remaining edge uses normal road construction");
                let id = project.id;
                project.progress = 1.0;
                world.complete_project(id);
            }
            assert_eq!(engine.save_json(), replay.save_json());
            let lots = engine
                .state
                .parcels
                .iter()
                .filter(|parcel| parcel.rotation.is_some())
                .count();
            if edge < 3 {
                assert_eq!(lots, 0, "interior lots wait for the final perimeter edge");
            } else {
                assert_eq!(lots, 10);
            }
        }
        assert!(engine.state.neighborhood_plan.is_none());
        assert_eq!(engine.state.neighborhood_road_ids.len(), 4);
        engine.validate().unwrap();
        let canonical = engine.save_json();
        assert_eq!(
            Engine::restore_json(&canonical).unwrap().save_json(),
            canonical
        );
    }

    #[test]
    fn coordinated_block_reuse_keeps_frontage_valid_when_existing_edge_is_reversed() {
        let mut engine = engine_with_developed_plan_site();
        let road = engine.state.roads[1].id;
        let existing = engine
            .state
            .roads
            .iter_mut()
            .find(|candidate| candidate.id == road)
            .unwrap();
        std::mem::swap(&mut existing.from, &mut existing.to);
        engine.recompute_connectivity();
        assert!(engine.try_create_neighborhood_plan());
        let plan = engine.state.neighborhood_plan.clone().unwrap();
        assert!(engine.state.roads.iter().any(|candidate| {
            candidate.id == road
                && ((candidate.from == plan.segments[0].0 && candidate.to == plan.segments[0].1)
                    || (candidate.from == plan.segments[0].1 && candidate.to == plan.segments[0].0))
        }));

        let mut replay = Engine::restore_json(&engine.save_json()).unwrap();
        for edge in 0..4 {
            for world in [&mut engine, &mut replay] {
                assert!(world.plan_next_neighborhood_segment());
                let segment = plan.segments[edge];
                let pending = world.state.projects.iter_mut().find(|project| {
                    project.kind == "road"
                        && project.progress < 1.0
                        && project.from == Some(segment.0)
                        && project.to == Some(segment.1)
                });
                if let Some(project) = pending {
                    let id = project.id;
                    project.progress = 1.0;
                    world.complete_project(id);
                }
            }
            assert_eq!(engine.save_json(), replay.save_json());
        }
        assert!(engine.state.neighborhood_plan.is_none());
        assert!(engine
            .state
            .parcels
            .iter()
            .any(|parcel| parcel.rotation.is_some()));
        engine.validate().unwrap();
    }

    #[test]
    fn seed_87231_natural_growth_uses_a_saved_partial_shared_edge_layout() {
        let mut engine = Engine::new(87_231);
        engine.set_development_rate(16);
        let mut observed_shared_plan = false;
        for _ in 0..121 {
            engine.step(2_000);
            if let Some(plan) = engine.state.neighborhood_plan.as_ref() {
                if let Some(selected) = plan.selected_lots.as_ref() {
                    assert!(selected.len() >= 4);
                    assert!(selected.len() < 10, "existing frontage should be retained");
                    engine.validate().unwrap();
                    let save = engine.save_json();
                    assert_eq!(Engine::restore_json(&save).unwrap().save_json(), save);
                    observed_shared_plan = true;
                    break;
                }
            }
        }
        assert!(
            observed_shared_plan,
            "seed 87231 should naturally select collision-free lots beside an existing road"
        );
    }

    #[test]
    fn coordinated_block_lots_wait_for_perimeter_and_preserve_access_on_restore() {
        let mut engine = engine_with_developed_plan_site_without_reusable_edge();
        assert!(engine.try_create_neighborhood_plan());
        assert!(
            engine
                .state
                .neighborhood_plan
                .as_ref()
                .unwrap()
                .coordinated_lots
        );
        let mut replay = Engine::restore_json(&engine.save_json()).unwrap();
        for edge in 0..4 {
            for world in [&mut engine, &mut replay] {
                assert!(world.plan_next_neighborhood_segment());
                let project = world
                    .state
                    .projects
                    .iter_mut()
                    .find(|project| project.kind == "road" && project.progress < 1.0)
                    .unwrap();
                let id = project.id;
                project.progress = 1.0;
                world.complete_project(id);
            }
            assert_eq!(engine.save_json(), replay.save_json());
            let count = engine
                .state
                .parcels
                .iter()
                .filter(|parcel| parcel.rotation.is_some())
                .count();
            if edge < 3 {
                assert_eq!(count, 0, "interior parcels must wait for all four streets");
            } else {
                assert_eq!(
                    count, 10,
                    "both coordinated rows must fill the reserved interior"
                );
            }
        }
        assert!(engine.state.neighborhood_plan.is_none());
        let lots: Vec<_> = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| parcel.rotation.is_some())
            .cloned()
            .collect();
        for parcel in &lots {
            let frontage = engine
                .state
                .parcel_frontages
                .iter()
                .find(|frontage| frontage.parcel_id == parcel.id)
                .unwrap();
            let road = engine
                .state
                .roads
                .iter()
                .find(|road| road.id == frontage.road_id)
                .unwrap();
            assert!(road.stage == "open" && road.connected);
            assert!(engine.node_connected(parcel.access_node));
            let theta = parcel.rotation.unwrap();
            let expected = engine.frontage_rotation(parcel.id);
            assert!(
                (theta.sin() - expected.sin()).abs() < 1e-5
                    && (theta.cos() - expected.cos()).abs() < 1e-5
            );
        }
        engine.validate().unwrap();
        let save = engine.save_json();
        let mut resumed = Engine::restore_json(&save).unwrap();
        assert_eq!(resumed.save_json(), save);
        engine.step(2000);
        resumed.step(2000);
        assert_eq!(engine.save_json(), resumed.save_json());
        engine.validate().unwrap();
    }

    #[test]
    fn neighborhood_plan_is_bounded_and_legacy_saves_restore_without_one() {
        let mut engine = engine_with_developed_plan_site_without_reusable_edge();
        assert!(engine.try_create_neighborhood_plan());
        assert!(engine.state.roads.len() + engine.state.projects.len() <= MAX_SEGMENTS);
        assert!(engine.state.parcels.len() <= MAX_PARCELS);
        assert!(engine.state.buildings.len() <= MAX_BUILDINGS);
        let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        legacy.as_object_mut().unwrap().remove("neighborhood_plan");
        legacy
            .as_object_mut()
            .unwrap()
            .remove("neighborhood_anchor_cursor");
        let restored = Engine::restore_json(&legacy.to_string()).unwrap();
        assert!(restored.state.neighborhood_plan.is_none());
        assert_eq!(restored.state.neighborhood_anchor_cursor, 0);

        let mut malformed: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        malformed["neighborhood_plan"] = serde_json::json!({
            "anchor_node": engine.state.neighborhood_plan.as_ref().unwrap().anchor_node,
            "segments": [],
            "next_segment": 0
        });
        assert!(Engine::restore_json(&malformed.to_string()).is_err());
    }

    #[test]
    fn neighborhood_candidate_cap_resumes_at_later_anchor_without_allocating_losers() {
        let mut engine = engine_with_developed_plan_site_without_reusable_edge();
        let first_anchor = engine.state.nodes[2].id;
        let original_node_count = engine.state.nodes.len();

        // Put the first anchor at the start of the scan and give it enough
        // connected spoke edges to consume the shared-edge candidate budget.
        // Its degree makes every block invalid, so this anchor cannot win.
        let first_point = engine.node(first_anchor).unwrap().clone();
        for spoke in 0..16 {
            let angle = -0.9 + 1.8 * spoke as f32 / 15.0;
            let endpoint = engine.add_node(
                first_point.x + angle.cos() * 75.0,
                first_point.z + angle.sin() * 75.0,
                "junction",
            );
            engine.add_road(first_anchor, endpoint, "local", "open", 1.0);
        }

        // Add a separate later catchment in the same connected road system.
        // Its degree and parcels admit an ordinary full-block candidate.
        let later_anchor = engine.add_node(-250.0, -250.0, "junction");
        let far_south = engine.add_node(360.0, -350.0, "junction");
        let west_south = engine.add_node(-160.0, -350.0, "junction");
        let later_feeder = engine.add_node(-190.0, -310.0, "junction");
        let gateway = engine.state.nodes[0].id;
        engine.add_road(gateway, far_south, "local", "open", 1.0);
        engine.add_road(far_south, west_south, "local", "open", 1.0);
        engine.add_road(west_south, later_feeder, "local", "open", 1.0);
        engine.add_road(later_feeder, later_anchor, "local", "open", 1.0);
        engine.recompute_connectivity();
        for row in 0..3 {
            for column in 0..4 {
                let parcel = engine.add_parcel(
                    -185.0 + column as f32 * 8.0,
                    -242.0 + row as f32 * 8.0,
                    6.0,
                    6.0,
                    later_anchor,
                    0.8,
                    false,
                );
                let building = engine.add_building(
                    parcel,
                    "home",
                    2,
                    "open",
                    1.0,
                    8,
                    "Later occupied machiya",
                    false,
                    Vec::new(),
                );
                engine
                    .state
                    .buildings
                    .iter_mut()
                    .find(|candidate| candidate.id == building)
                    .unwrap()
                    .occupancy = 4;
            }
        }

        // Start on the crowded first anchor. It reaches the 32 shared
        // candidate cap, but all candidates fail its degree constraint. The
        // cursor must point past this anchor, and losing probes allocate no
        // corner nodes or plan state.
        engine.state.neighborhood_anchor_cursor = 2;
        let before_first_scan_nodes = engine.state.nodes.len();
        assert!(!engine.try_create_neighborhood_plan());
        assert_eq!(
            engine.state.nodes.len(),
            before_first_scan_nodes,
            "rejected candidates must not allocate corner nodes"
        );
        assert!(engine.state.neighborhood_plan.is_none());
        assert_eq!(engine.state.neighborhood_anchor_cursor, 3);

        // The following planner evaluation starts at that later cursor and
        // finds the independently eligible catchment rather than rescanning
        // or starving it behind the capped anchor.
        assert!(engine.try_create_neighborhood_plan());
        let plan = engine.state.neighborhood_plan.as_ref().unwrap();
        assert_eq!(plan.anchor_node, later_anchor);
        assert!(engine.state.nodes.len() >= original_node_count + 16 + 3);
    }

    #[test]
    fn coordinated_block_reserves_all_parcel_slots_until_retirement() {
        let mut engine = engine_with_developed_plan_site();
        assert!(engine.try_create_neighborhood_plan());
        let reserved = engine.coordinated_plan_lot_count().unwrap();
        assert_eq!(reserved, 10);

        while engine.state.parcels.len() + reserved < MAX_PARCELS {
            let parcel = engine.add_parcel(0.0, 0.0, 1.0, 1.0, 2, 0.5, false);
            assert_ne!(
                parcel, 0,
                "exterior parcel admission should use unreserved capacity"
            );
        }
        assert_eq!(engine.state.parcels.len(), MAX_PARCELS - reserved);
        assert_eq!(engine.add_parcel(1.0, 0.0, 1.0, 1.0, 2, 0.5, false), 0);
        assert_eq!(engine.state.parcels.len() + reserved, MAX_PARCELS);

        // A new coordinated block must fail atomically when the world has
        // fewer parcel slots than its deterministic subdivision requires.
        let mut near_cap = engine_with_developed_plan_site();
        while near_cap.state.parcels.len() < MAX_PARCELS - reserved + 1 {
            assert_ne!(near_cap.add_parcel(0.0, 0.0, 1.0, 1.0, 2, 0.5, false), 0);
        }
        let before_nodes = near_cap.state.nodes.len();
        let before_parcels = near_cap.state.parcels.len();
        assert!(!near_cap.try_create_neighborhood_plan());
        assert_eq!(near_cap.state.nodes.len(), before_nodes);
        assert_eq!(near_cap.state.parcels.len(), before_parcels);

        // Invalid restored state cannot claim more slots than the hard cap.
        let mut invalid = engine_with_developed_plan_site();
        assert!(invalid.try_create_neighborhood_plan());
        while invalid.state.parcels.len() < MAX_PARCELS - reserved + 1 {
            invalid.state.parcels.push(Parcel {
                id: 1_000_000 + invalid.state.parcels.len() as u32,
                x: 0.0,
                z: 0.0,
                width: 1.0,
                depth: 1.0,
                rotation: None,
                access_node: 2,
                suitability: 0.5,
                protected: false,
                reservation: None,
            });
        }
        assert!(invalid
            .validate()
            .unwrap_err()
            .contains("coordinated block parcel reservation"));
    }

    #[test]
    fn coordinated_block_road_clearance_protects_interior_and_allows_perimeter() {
        let mut engine = engine_with_developed_plan_site();
        assert!(engine.try_create_neighborhood_plan());
        let plan = engine.state.neighborhood_plan.as_ref().unwrap();
        let anchor = engine.node(plan.anchor_node).unwrap().clone();
        let interior = engine.coordinated_block_interior(&plan.segments).unwrap();
        let spur = Node {
            id: u32::MAX,
            x: interior.x,
            z: interior.z,
            kind: "candidate".into(),
        };
        assert!(!engine.neighborhood_corridor_clear_for_road(&anchor, &spur, anchor.id));

        let (_, next_id) = plan.segments[0];
        let perimeter_end = engine.node(next_id).unwrap().clone();
        assert!(engine.neighborhood_corridor_clear_for_road(&anchor, &perimeter_end, anchor.id));
    }

    #[test]
    fn oriented_admission_respects_legacy_saved_and_rendered_envelopes() {
        let mut engine = Engine::new(87231);
        let mut building = engine.state.buildings[0].clone();
        engine.state.parcels.clear();
        engine.state.buildings.clear();
        engine.state.roads.clear();
        engine.state.projects.clear();
        engine.state.parcel_frontages.clear();
        engine.state.neighborhood_plan = None;
        let probes =
            [(262.0, 250.0), (250.0, 262.0), (280.0, 280.0)].map(|(x, z)| geometry::OrientedRect {
                x,
                z,
                width: 4.0,
                depth: 4.0,
                rotation: 0.0,
            });
        for probe in &probes {
            assert!(engine.oriented_parcel_geometry_clear(probe));
        }
        let parcel = engine.add_parcel(250.0, 250.0, 30.0, 8.0, 2, 0.7, false);
        building.parcel_id = parcel;
        building.rotation = std::f32::consts::FRAC_PI_2;
        engine.state.buildings.push(building);
        let stored = parcel_envelope(&engine.state.parcels[0]);
        let rendered = geometry::OrientedRect {
            rotation: std::f32::consts::FRAC_PI_2,
            ..stored
        };
        assert!(geometry::rects_overlap(&probes[0], &stored, 0.0));
        assert!(!geometry::rects_overlap(&probes[0], &rendered, 2.0));
        assert!(!geometry::rects_overlap(&probes[1], &stored, 2.0));
        assert!(geometry::rects_overlap(&probes[1], &rendered, 0.0));
        let context = engine.collision_context();
        for (probe, expected) in probes.iter().zip([false, false, true]) {
            assert_eq!(engine.oriented_parcel_geometry_clear(probe), expected);
            assert_eq!(
                engine.oriented_parcel_geometry_clear_with_context(probe, &context),
                expected
            );
        }
    }

    #[test]
    fn cached_neighborhood_collisions_match_reference_after_world_edits() {
        let probes = [
            geometry::OrientedRect {
                x: 145.0,
                z: 190.0,
                width: 18.0,
                depth: 12.0,
                rotation: 0.0,
            },
            geometry::OrientedRect {
                x: 168.0,
                z: 205.0,
                width: 21.0,
                depth: 13.0,
                rotation: 0.63,
            },
            geometry::OrientedRect {
                x: -18.0,
                z: 42.0,
                width: 15.0,
                depth: 10.0,
                rotation: -0.41,
            },
            geometry::OrientedRect {
                x: 0.0,
                z: 300.0,
                width: 16.0,
                depth: 14.0,
                rotation: 1.2,
            },
        ];
        let assert_matches = |engine: &Engine| {
            let cached = engine.collision_context();
            for probe in &probes {
                assert_eq!(
                    engine.oriented_parcel_geometry_clear_with_context(probe, &cached),
                    engine.oriented_parcel_geometry_clear(probe),
                    "cached collision decision changed for probe {probe:?}"
                );
            }
        };

        // Existing legacy parcels inherit the first building's rendered
        // orientation when their optional parcel rotation is absent.
        let mut with_building = engine_with_developed_plan_site();
        let legacy_parcel = with_building.state.parcels[0].id;
        with_building.state.parcels[0].rotation = None;
        with_building.state.buildings[0].parcel_id = legacy_parcel;
        with_building.state.buildings[0].rotation = 0.73;
        assert_matches(&with_building);

        // Empty parcels with saved orientation take the explicit rotation.
        let mut with_empty_rotated_parcel = engine_with_developed_plan_site();
        let empty = with_empty_rotated_parcel.add_parcel(28.0, 112.0, 17.0, 11.0, 2, 0.6, false);
        with_empty_rotated_parcel
            .state
            .parcels
            .iter_mut()
            .find(|parcel| parcel.id == empty)
            .unwrap()
            .rotation = Some(-0.52);
        assert_matches(&with_empty_rotated_parcel);

        // Road removals and pending construction both alter the cached segment
        // vectors while preserving the reference check's outcome.
        let mut with_pending_road = engine_with_developed_plan_site();
        let road = with_pending_road.state.roads[0].clone();
        with_pending_road.state.roads[0].stage = "closed".into();
        let project_id = with_pending_road.id();
        with_pending_road.state.projects.push(Project {
            id: project_id,
            kind: "road".into(),
            from: Some(road.from),
            to: Some(road.to),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.25,
            reason: "Collision context test road".into(),
            duration: 4.0,
            started_tick: 0,
        });
        assert_matches(&with_pending_road);
    }

    #[test]
    fn local_chord_construction_opens_connected_frontage_and_adds_accessible_density() {
        let (engine, start, target) = engine_with_clear_local_chord();
        let save = engine.save_json();
        let mut first = Engine::restore_json(&save).unwrap();
        let mut replay = Engine::restore_json(&save).unwrap();

        first.plan_road();
        replay.plan_road();
        assert_eq!(first.save_json(), replay.save_json());
        let project = first
            .state
            .projects
            .iter()
            .find(|project| project.kind == "road" && project.progress < 1.0)
            .cloned()
            .expect("a clear occupied neighborhood should reserve a local chord");
        assert_eq!(project.from, Some(start));
        assert_eq!(project.to, Some(target));
        assert!(!first.state.roads.iter().any(|road| {
            road.stage == "open"
                && ((road.from == start && road.to == target)
                    || (road.from == target && road.to == start))
        }));

        let (a, b) = (first.node(start).unwrap(), first.node(target).unwrap());
        assert!((45.0..=90.0).contains(&distance(a, b)));
        assert!(first.ordinary_road_clear_of_river(a.x, a.z, b.x, b.z, 4.5));
        let corridor_context = first.collision_context();
        for footprint in &corridor_context.parcel_footprints {
            assert!(geometry::rect_segment_distance(footprint, a.x, a.z, b.x, b.z) >= 5.0);
        }
        for road in first.state.roads.iter().filter(|road| road.stage == "open") {
            if road.from == start || road.to == start || road.from == target || road.to == target {
                continue;
            }
            let (from, to) = (first.node(road.from).unwrap(), first.node(road.to).unwrap());
            assert!(segments_distance(a, b, from, to) > 8.0);
        }

        // The in-flight checkpoint restores the same reserved corridor. It
        // remains unavailable to traffic until the ordinary road project ends.
        let active_save = first.save_json();
        let mut resumed = Engine::restore_json(&active_save).unwrap();
        assert_eq!(resumed.save_json(), active_save);
        let initial_parcels = first.state.parcels.len();
        let initial_local_envelope_area: f32 = first
            .state
            .buildings
            .iter()
            .filter(|building| (building.x - 30.0).hypot(building.z - 300.0) <= 80.0)
            .map(|building| building.width * building.depth)
            .sum();
        for engine in [&mut first, &mut resumed] {
            engine
                .state
                .projects
                .iter_mut()
                .find(|candidate| candidate.id == project.id)
                .unwrap()
                .duration = 2.0;
            engine.advance_projects();
            assert!(!engine.state.roads.iter().any(|road| {
                road.stage == "open"
                    && ((road.from == start && road.to == target)
                        || (road.from == target && road.to == start))
            }));
            engine.advance_projects();
            assert!(engine.state.roads.iter().any(|road| {
                road.stage == "open"
                    && road.connected
                    && ((road.from == start && road.to == target)
                        || (road.from == target && road.to == start))
            }));
            assert!(engine.state.parcels.len() > initial_parcels);
        }
        assert_eq!(first.save_json(), resumed.save_json());

        let new_parcels: BTreeSet<u32> = first
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| {
                first.state.roads.iter().any(|road| {
                    road.id == frontage.road_id
                        && ((road.from == start && road.to == target)
                            || (road.from == target && road.to == start))
                })
            })
            .map(|frontage| frontage.parcel_id)
            .collect();
        assert!(!new_parcels.is_empty());

        first.plan_building((first.state.tick / TICKS_PER_DAY) as u32);
        let building_project = first
            .state
            .projects
            .iter()
            .find(|candidate| {
                candidate.kind == "building"
                    && candidate.progress < 1.0
                    && candidate
                        .parcel_id
                        .is_some_and(|id| new_parcels.contains(&id))
            })
            .cloned()
            .expect("the completed chord should provide a buildable, connected lot");
        for _ in 0..=6 {
            if first.state.buildings.iter().any(|building| {
                building.parcel_id == building_project.parcel_id.unwrap()
                    && building.stage == "open"
            }) {
                break;
            }
            first.advance_projects();
        }
        let new_building = first
            .state
            .buildings
            .iter()
            .find(|building| building.parcel_id == building_project.parcel_id.unwrap())
            .expect("the frontage building opens after construction");
        assert_eq!(new_building.stage, "open");
        let (access_road, points) = first
            .building_access_segment(new_building.id)
            .expect("the new building has a real frontage access segment");
        assert!(first.state.roads.iter().any(|road| road.id == access_road));
        assert!(points[0].x.is_finite() && points[1].x.is_finite());
        let walk = first
            .building_access_path(new_building.id, &[start, target], true, new_building.id, 0)
            .expect("pedestrian route joins the door to the connected street");
        assert!(walk.points.len() >= 2);
        let final_local_envelope_area: f32 = first
            .state
            .buildings
            .iter()
            .filter(|building| (building.x - 30.0).hypot(building.z - 300.0) <= 80.0)
            .map(|building| building.width * building.depth)
            .sum();
        assert!(final_local_envelope_area >= initial_local_envelope_area * 1.2);
        first.validate().unwrap();
    }

    #[test]
    fn movement_buffer_places_traffic_in_left_lane_and_on_sidewalks() {
        let mut engine = Engine::new(81);
        let road = engine
            .state
            .roads
            .iter()
            .find(|road| road.kind != "rail" && road.stage == "open")
            .expect("founding road is open");
        let (from, to) = (road.from, road.to);
        let arterial = road.kind == "arterial";
        let a = engine.node(from).unwrap().clone();
        let b = engine.node(to).unwrap().clone();
        let len = distance(&a, &b);
        let route = vec![from, to];
        for (id, kind) in [(700_001, 1), (700_002, 0)] {
            engine.state.travelers.push(Traveler {
                id,
                kind,
                route: route.clone(),
                route_lengths: vec![len],
                edge_index: 0,
                edge_progress: len * 0.5,
                speed: 1.0,
                destination: to,
                activity: "test".into(),
                resident: 0,
                name: "Test traveler".into(),
                origin_building: None,
                destination_building: None,
                wait_ticks: 0,
                origin_access: None,
                destination_access: None,
                access_phase: 1,
                access_progress: 0.0,
            });
        }
        let movement = engine.movement_buffer();
        let center_x = a.x + (b.x - a.x) * 0.5;
        let center_z = a.z + (b.z - a.z) * 0.5;
        let nx = -(b.z - a.z) / len;
        let nz = (b.x - a.x) / len;
        let car = &movement[1..5];
        let pedestrian = &movement[6..10];
        assert!((car[0] - center_x - nx * 1.65).abs() < 0.001);
        assert!((car[1] - center_z - nz * 1.65).abs() < 0.001);
        let road_width = if arterial { 14.0 } else { 9.0 };
        let sidewalk_offset = road_width * 0.5 + 0.72;
        let sign = if 700_002 % 2 == 0 { 1.0 } else { -1.0 };
        assert!((pedestrian[0] - center_x - nx * sidewalk_offset * sign).abs() < 0.001);
        assert!((pedestrian[1] - center_z - nz * sidewalk_offset * sign).abs() < 0.001);
    }

    #[test]
    fn save_restore_preserves_residents_and_active_trips() {
        let mut engine = Engine::new(91);
        engine.set_development_rate(60);
        for _ in 0..12 {
            engine.step(2_000);
        }
        assert!(!engine.state.residents.is_empty());
        // Capture an active named journey, not only the resident roster and deadlines.
        for _ in 0..120 {
            engine.step(200);
            if !engine.state.travelers.is_empty() {
                break;
            }
        }
        assert!(
            !engine.state.travelers.is_empty(),
            "no scheduled trip became active"
        );
        let active_id = engine.state.travelers[0].id;
        let inspected: serde_json::Value =
            serde_json::from_str(&engine.inspect_json(active_id)).unwrap();
        assert_eq!(inspected["entityType"], "traveler");
        assert!(inspected["name"].as_str().is_some());
        assert!(inspected["activity"].as_str().is_some());
        let save = engine.save_json();
        let mut restored = Engine::restore_json(&save).unwrap();
        assert_eq!(restored.save_json(), save);
        engine.step(2_000);
        restored.step(2_000);
        assert_eq!(restored.save_json(), engine.save_json());
        engine.validate().unwrap();
    }
    #[test]
    fn save_restore_replays_pending_projects() {
        let mut a = Engine::new(8);
        a.step(1_337);
        assert!(a
            .state
            .projects
            .iter()
            .any(|p| p.progress > 0.0 && p.progress < 1.0));
        let save = a.save_json();
        let mut b = Engine::restore_json(&save).unwrap();
        a.step(2_000);
        b.step(2_000);
        assert_eq!(a.save_json(), b.save_json());
    }
    #[test]
    fn bounded_world_keeps_open_buildings_connected() {
        let mut e = Engine::new(19);
        e.set_development_rate(60);
        for _ in 0..12 {
            e.step(2_000);
            e.validate().unwrap();
        }
        assert!(e.state.buildings.len() <= MAX_BUILDINGS);
        assert!(e.state.roads.len() <= MAX_SEGMENTS);
    }
    #[test]
    fn twelve_seed_corpus_stays_bounded_and_grows_as_a_connected_city() {
        use std::time::Instant;
        let started = Instant::now();
        let mut totals = (0usize, 0usize, 0u32, 0usize);
        let mut saw_ordinary_loop = false;
        let mut bus_service_seeds = 0usize;
        for seed in 0..12 {
            let mut engine = Engine::new(seed);
            engine.set_development_rate(60);
            let mut max_travelers = 0usize;
            let mut max_night_travelers = 0usize;
            for _ in 0..36 {
                engine.step(2_000);
                engine
                    .validate()
                    .unwrap_or_else(|err| panic!("seed {seed}: {err}"));
                max_travelers = max_travelers.max(engine.state.travelers.len());
                let minute = engine.snapshot().minute_of_day;
                if minute >= 1230.0 || minute < 390.0 {
                    max_night_travelers = max_night_travelers.max(
                        engine
                            .state
                            .travelers
                            .iter()
                            .filter(|t| t.activity == "night_shift" || t.activity == "return")
                            .count(),
                    );
                }
            }
            let snapshot = engine.snapshot();
            let open_roads = snapshot.roads.iter().filter(|r| r.stage == "open").count();
            assert!(
                snapshot
                    .parcels
                    .iter()
                    .all(|parcel| engine.parcel_clear_of_river(
                        parcel.x,
                        parcel.z,
                        parcel.width,
                        parcel.depth
                    )),
                "seed {seed} developed a parcel inside the water/bank reserve"
            );
            assert!(
                snapshot
                    .roads
                    .iter()
                    .filter(|road| {
                        road.stage == "open" && !matches!(road.kind.as_str(), "rail" | "bridge")
                    })
                    .all(|road| {
                        let from = engine.node(road.from).unwrap();
                        let to = engine.node(road.to).unwrap();
                        let half_width = if road.kind == "arterial" { 7.0 } else { 4.5 };
                        engine.ordinary_road_clear_of_river(from.x, from.z, to.x, to.z, half_width)
                    }),
                "seed {seed} built an ordinary road inside the water/bank reserve"
            );
            for node in engine.state.nodes.iter().filter(|node| node.id >= 7) {
                let incident: Vec<_> = engine
                    .state
                    .roads
                    .iter()
                    .filter(|road| {
                        road.connected
                            && road.stage == "open"
                            && !matches!(road.kind.as_str(), "rail" | "bridge")
                            && (road.from == node.id || road.to == node.id)
                    })
                    .collect();
                assert!(
                    incident.len() <= 4,
                    "seed {seed} generated junction {} with degree {}",
                    node.id,
                    incident.len()
                );
                let headings: Vec<f32> = incident
                    .iter()
                    .filter_map(|road| {
                        let neighbor_id = if road.from == node.id {
                            road.to
                        } else {
                            road.from
                        };
                        engine
                            .node(neighbor_id)
                            .map(|neighbor| (neighbor.z - node.z).atan2(neighbor.x - node.x))
                    })
                    .collect();
                for i in 0..headings.len() {
                    for j in (i + 1)..headings.len() {
                        let delta = (headings[i] - headings[j]).rem_euclid(std::f32::consts::TAU);
                        assert!(
                            delta.min(std::f32::consts::TAU - delta)
                                >= std::f32::consts::PI / 6.0 - 0.04,
                            "seed {seed} generated acute road branches at junction {}",
                            node.id
                        );
                    }
                }
            }
            let ordinary_roads: Vec<_> = engine
                .state
                .roads
                .iter()
                .filter(|road| {
                    road.connected
                        && road.stage == "open"
                        && !matches!(road.kind.as_str(), "rail" | "bridge")
                })
                .collect();
            for road in &ordinary_roads {
                let mut frontier = vec![road.from];
                let mut visited = vec![road.from];
                while let Some(node_id) = frontier.pop() {
                    for adjacent in ordinary_roads.iter().filter(|candidate| {
                        candidate.id != road.id
                            && (candidate.from == node_id || candidate.to == node_id)
                    }) {
                        let next = if adjacent.from == node_id {
                            adjacent.to
                        } else {
                            adjacent.from
                        };
                        if next == road.to {
                            saw_ordinary_loop = true;
                            break;
                        }
                        if !visited.contains(&next) {
                            visited.push(next);
                            frontier.push(next);
                        }
                    }
                    if saw_ordinary_loop {
                        break;
                    }
                }
                if saw_ordinary_loop {
                    break;
                }
            }
            assert!(
                open_roads >= 6,
                "seed {seed} failed to extend its road network"
            );
            assert!(
                snapshot.buildings.len() >= 8,
                "seed {seed} failed to establish its neighborhood"
            );
            assert!(
                engine.state.station_online,
                "seed {seed} did not open rail service"
            );
            let rail_commuters = engine
                .state
                .residents
                .iter()
                .filter(|resident| resident.rail_commuter)
                .count();
            assert!(
                rail_commuters > 0,
                "seed {seed} generated no external rail commuters"
            );
            let rail_boardings = engine
                .state
                .events
                .iter()
                .filter(|event| event.kind == "TrainPassengerBoarded")
                .count();
            assert!(
                rail_boardings > 0,
                "seed {seed} commuters never boarded a train"
            );
            assert!(
                engine
                    .state
                    .events
                    .iter()
                    .any(|event| event.kind == "RailWorkStarted"),
                "seed {seed} has no completed external rail commute"
            );
            assert!(
                !engine.state.trains.is_empty(),
                "seed {seed} has no active train"
            );
            assert!(engine.state.buses.len() <= MAX_BUSES);
            if let Some(line) = &engine.state.bus_line {
                bus_service_seeds += 1;
                assert!(line.active);
                assert!((3..=MAX_BUS_STOPS).contains(&line.stops.len()));
                assert!(line.route.len() <= MAX_BUS_ROUTE_NODES);
                assert!(engine
                    .state
                    .buses
                    .iter()
                    .all(|bus| bus.line_id == line.id && bus.passengers.len() <= BUS_CAPACITY));
                assert!(engine.bus_waiting_count() <= BUS_QUEUE_CAPACITY);
            }
            assert!(
                max_travelers >= 20,
                "seed {seed} has too little visible daytime activity"
            );
            assert!(
                max_night_travelers > 0,
                "seed {seed} has no visible late-night activity"
            );
            assert!(
                matches!(snapshot.stage.as_str(), "TransitCity" | "Metropolis" | "Maturity"),
                "seed {seed} did not establish transit-era growth: stage={} buildings={} open_roads={} population={}",
                snapshot.stage,
                snapshot.buildings.len(),
                open_roads,
                snapshot.stats.population
            );
            assert!(snapshot.stats.active_projects as usize <= MAX_PROJECTS);
            assert!(engine.state.projects.len() <= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY);
            assert!(engine.state.population <= MAX_POPULATION);
            totals.0 += snapshot.buildings.len();
            totals.1 += open_roads;
            totals.2 += snapshot.stats.population;
            totals.3 += snapshot.stats.active_travelers as usize;
            println!(
                "seed={seed} tick={} stage={} buildings={} open_roads={} population={} travelers={} rail_commuters={rail_commuters} rail_boardings={rail_boardings} projects={} parcels={}",
                snapshot.tick,
                snapshot.stage,
                snapshot.stats.buildings,
                open_roads,
                snapshot.stats.population,
                max_travelers,
                snapshot.stats.active_projects,
                snapshot.parcels.len(),
            );
        }
        println!(
            "12-seed profile: elapsed_ms={} mean_buildings={} mean_open_roads={} mean_population={} mean_final_travelers={} bus_service_seeds={}",
            started.elapsed().as_millis(),
            totals.0 / 12,
            totals.1 / 12,
            totals.2 / 12,
            totals.3 / 12,
            bus_service_seeds,
        );
        assert!(
            saw_ordinary_loop,
            "the seeded road corpus formed no street loops"
        );
        assert!(
            bus_service_seeds > 0,
            "no seed produced a viable neighborhood bus line"
        );
    }
    #[test]
    fn active_commerce_does_not_age_into_false_vacancy() {
        let mut engine = Engine::new(71);
        engine.set_development_rate(60);
        for _ in 0..36 {
            engine.step(2_000);
        }
        let commercial_ids: BTreeSet<u32> = engine
            .state
            .buildings
            .iter()
            .filter(|building| is_commercial(building))
            .map(|building| building.id)
            .collect();
        for business in engine
            .state
            .buildings
            .iter()
            .filter(|building| is_commercial(building) && building.stage == "open")
        {
            assert!(business.business_active);
            assert_eq!(business.vacant_days, 0);
        }
        assert!(!engine.state.events.iter().any(|event| {
            event.kind == "RenovationStarted" && commercial_ids.contains(&event.entity_id)
        }));
        engine.validate().unwrap();
        let save = engine.save_json();
        let restored = Engine::restore_json(&save).unwrap();
        assert_eq!(restored.save_json(), save);
    }

    #[test]
    fn legacy_business_activity_migration_preserves_housing_and_pending_work() {
        let mut engine = Engine::new(604);
        let cafe_id = engine.add_building(
            engine.state.parcels[2].id,
            "cafe",
            2,
            "open",
            1.0,
            12,
            "Legacy Cafe",
            false,
            Vec::new(),
        );
        let office_id = engine.add_building(
            engine.state.parcels[3].id,
            "office",
            3,
            "renovation",
            0.4,
            18,
            "Legacy Office",
            false,
            vec!["renovation underway".into()],
        );
        let office_parcel_id = engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == office_id)
            .unwrap()
            .parcel_id;
        engine
            .state
            .parcels
            .iter_mut()
            .find(|parcel| parcel.id == office_parcel_id)
            .unwrap()
            .reservation = Some(office_id);
        let renovation_project_id = engine.id();
        engine.state.projects.push(Project {
            id: renovation_project_id,
            kind: "building".into(),
            from: None,
            to: None,
            parcel_id: Some(office_parcel_id),
            stage: "structure".into(),
            progress: 0.4,
            reason: "Keep the pending tenant renovation".into(),
            duration: 4.0,
            started_tick: 0,
        });
        let home_id = engine.add_building(
            engine.state.parcels[1].id,
            "home",
            2,
            "open",
            1.0,
            6,
            "Legacy Home",
            false,
            Vec::new(),
        );
        for building in &mut engine.state.buildings {
            if building.id == cafe_id {
                building.occupancy = 9;
                building.vacant_days = 23;
            } else if building.id == office_id {
                building.occupancy = 11;
                building.vacant_days = 41;
            } else if building.id == home_id {
                building.occupancy = 3;
                building.vacant_days = 2;
            }
        }
        let expected_rng = engine.state.rng_growth;
        let expected_tick = engine.state.tick;
        let expected_next_id = engine.state.next_id;
        let expected_events = engine.state.events.clone();
        let expected_projects = engine.state.projects.clone();
        let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        let object = legacy.as_object_mut().unwrap();
        object.remove("activity_schema_version");
        for building in object.get_mut("buildings").unwrap().as_array_mut().unwrap() {
            building.as_object_mut().unwrap().remove("business_active");
        }

        let migrated = Engine::restore_json(&legacy.to_string()).unwrap();
        assert_eq!(
            migrated.state.activity_schema_version,
            ACTIVITY_SCHEMA_VERSION
        );
        assert_eq!(migrated.state.rng_growth, expected_rng);
        assert_eq!(migrated.state.tick, expected_tick);
        assert_eq!(migrated.state.next_id, expected_next_id);
        assert_eq!(
            serde_json::to_value(&migrated.state.events).unwrap(),
            serde_json::to_value(&expected_events).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&migrated.state.projects).unwrap(),
            serde_json::to_value(&expected_projects).unwrap()
        );
        let founder = migrated.state.buildings.iter().find(|b| b.anchor).unwrap();
        assert_eq!(founder.occupancy, 0);
        assert!(founder.business_active);
        assert_eq!(founder.vacant_days, 0);
        let cafe = migrated
            .state
            .buildings
            .iter()
            .find(|b| b.id == cafe_id)
            .unwrap();
        assert_eq!(cafe.occupancy, 0);
        assert!(cafe.business_active);
        assert_eq!(cafe.vacant_days, 0);
        let office = migrated
            .state
            .buildings
            .iter()
            .find(|b| b.id == office_id)
            .unwrap();
        assert_eq!(office.occupancy, 0);
        assert!(!office.business_active);
        assert_eq!(office.vacant_days, 0);
        let home = migrated
            .state
            .buildings
            .iter()
            .find(|b| b.id == home_id)
            .unwrap();
        assert_eq!(home.occupancy, 3);
        assert_eq!(home.vacant_days, 2);
        assert!(!home.business_active);
        migrated.validate().unwrap();

        let current_save = migrated.save_json();
        assert_eq!(
            Engine::restore_json(&current_save).unwrap().save_json(),
            current_save
        );
        legacy["activity_schema_version"] = serde_json::json!(ACTIVITY_SCHEMA_VERSION + 1);
        assert!(Engine::restore_json(&legacy.to_string()).is_err());
    }

    #[test]
    fn active_commerce_is_not_vacant_when_residential_occupancy_is_zero() {
        let mut engine = Engine::new(605);
        let founder = &engine.state.buildings[0];
        assert_eq!(founder.occupancy, 0);
        assert!(founder.business_active);
        assert!(site_active(founder));
        assert!(!site_vacant(founder));

        let cafe_id = engine.add_building(
            engine.state.parcels[1].id,
            "cafe",
            2,
            "open",
            1.0,
            10,
            "Active Cafe",
            false,
            Vec::new(),
        );
        let before = engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == cafe_id)
            .unwrap()
            .vacant_days;
        engine.development_day(1);
        let cafe = engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == cafe_id)
            .unwrap();
        assert!(site_active(cafe));
        assert!(!site_vacant(cafe));
        assert_eq!(cafe.vacant_days, before);
    }

    #[test]
    fn seeded_business_turnover_defers_jobs_and_deliveries_then_reopens() {
        let mut engine = Engine::new(606);
        let home_parcel = engine.state.parcels[1].id;
        let cafe_parcel = engine.state.parcels[2].id;
        let other_cafe_parcel = engine.state.parcels[3].id;
        let home_id = engine.add_building(
            home_parcel,
            "home",
            2,
            "open",
            1.0,
            6,
            "Turnover Home",
            false,
            Vec::new(),
        );
        let cafe_id = engine.add_building(
            cafe_parcel,
            "cafe",
            2,
            "open",
            1.0,
            12,
            "Cafe A",
            false,
            Vec::new(),
        );
        let other_cafe_id = engine.add_building(
            other_cafe_parcel,
            "cafe",
            2,
            "open",
            1.0,
            12,
            "Cafe B",
            false,
            Vec::new(),
        );
        for building in &mut engine.state.buildings {
            if building.id == home_id {
                building.occupancy = 1;
            }
            if building.id == cafe_id || building.id == other_cafe_id {
                building.built_day = 0;
            }
        }
        let home_node = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == home_parcel)
            .unwrap()
            .access_node;
        engine.state.residents.push(Resident {
            id: 900_001,
            name: "Assigned worker".into(),
            home_building: home_id,
            work_building: Some(cafe_id),
            home_node,
            work_node: Some(home_node),
            mode: 0,
            routine: "neighbor".into(),
            activity: "home".into(),
            at_work: false,
            departure_tick: 0,
            return_tick: 0,
            last_departure_day: u32::MAX,
            last_return_day: u32::MAX,
            history: Vec::new(),
            rail_commuter: false,
            transfer: "home".into(),
            train_id: None,
            queue_slot: None,
            bus_id: None,
            bus_origin_stop: None,
            bus_destination_stop: None,
            bus_stop_node: None,
        });
        engine.state.travelers.push(Traveler {
            id: 900_002,
            kind: 1,
            route: Vec::new(),
            route_lengths: Vec::new(),
            edge_index: 0,
            edge_progress: 0.0,
            speed: 9.0,
            destination: 0,
            activity: "delivery".into(),
            resident: 0,
            name: "Delivery van".into(),
            origin_building: None,
            destination_building: Some(other_cafe_id),
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        });
        assert!(!engine.try_close_business_for_turnover(30));
        assert!(engine
            .state
            .buildings
            .iter()
            .filter(|building| building.id == cafe_id || building.id == other_cafe_id)
            .all(|building| building.business_active));
        engine.state.residents.clear();
        engine.state.travelers.clear();

        let initial = engine.save_json();
        let mut replay = Engine::restore_json(&initial).unwrap();
        let mut closed_id = None;
        let mut closed_on_day = None;
        for day in (30..=30 + 30 * 63).step_by(30) {
            let closed = engine.try_close_business_for_turnover(day);
            assert_eq!(closed, replay.try_close_business_for_turnover(day));
            if closed {
                closed_on_day = Some(day);
                closed_id = engine
                    .state
                    .buildings
                    .iter()
                    .find(|building| !building.business_active && is_commercial(building))
                    .map(|building| building.id);
                break;
            }
        }
        assert!(
            closed_id.is_some(),
            "seeded turnover selected no eligible tenant"
        );
        assert_eq!(engine.save_json(), replay.save_json());
        let closed_id = closed_id.unwrap();
        let closed = engine
            .state
            .buildings
            .iter_mut()
            .find(|building| building.id == closed_id)
            .unwrap();
        assert_eq!(closed.occupancy, 0);
        assert!(!closed.business_active);
        assert!(closed
            .history
            .iter()
            .any(|entry| entry.starts_with("Business closed:")));
        assert!(engine
            .state
            .events
            .iter()
            .any(|event| { event.kind == "BusinessClosed" && event.entity_id == closed_id }));
        engine.sync_residents();
        assert!(engine
            .state
            .residents
            .iter()
            .all(|resident| resident.work_building != Some(closed_id)));
        engine.state.next_delivery_tick = engine.state.tick;
        engine.schedule_deliveries();
        assert!(engine
            .state
            .travelers
            .iter()
            .all(|traveler| traveler.destination_building != Some(closed_id)));

        let other_cafe_id = engine
            .state
            .buildings
            .iter()
            .find(|building| {
                building.id != closed_id && building.kind == "cafe" && site_active(building)
            })
            .unwrap()
            .id;
        engine
            .state
            .buildings
            .iter_mut()
            .find(|b| b.id == closed_id)
            .unwrap()
            .vacant_days = 6;
        engine.state.last_renewal_day = u32::MAX;
        engine.renewal(closed_on_day.unwrap() + 3);
        assert_eq!(
            engine
                .state
                .buildings
                .iter()
                .find(|b| b.id == closed_id)
                .unwrap()
                .stage,
            "renovation"
        );
        for _ in 0..2 {
            engine.advance_projects();
        }
        let reopened = engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == closed_id)
            .unwrap();
        assert_eq!(reopened.stage, "open");
        assert!(reopened.business_active);
        assert_eq!(reopened.vacant_days, 0);
        assert!(engine
            .state
            .events
            .iter()
            .any(|event| { event.kind == "BusinessReopened" && event.entity_id == closed_id }));
        assert!(site_active(
            engine
                .state
                .buildings
                .iter()
                .find(|building| building.id == other_cafe_id)
                .unwrap()
        ));
        engine.validate().unwrap();
        let save = engine.save_json();
        let mut restored = Engine::restore_json(&save).unwrap();
        engine.step(2_000);
        restored.step(2_000);
        assert_eq!(engine.save_json(), restored.save_json());
    }
    #[test]
    fn mature_neighborhoods_keep_low_rise_core_and_tall_station_cluster() {
        let mut signatures = BTreeSet::new();
        for seed in 0..6 {
            let mut engine = Engine::new(seed);
            engine.set_development_rate(60);
            for _ in 0..36 {
                engine.step(2_000);
            }
            let station = engine
                .state
                .nodes
                .iter()
                .find(|node| node.kind == "station")
                .unwrap();
            let market = engine
                .state
                .nodes
                .iter()
                .find(|node| node.kind == "market")
                .unwrap();
            let open: Vec<&Building> = engine
                .state
                .buildings
                .iter()
                .filter(|building| building.stage == "open")
                .collect();
            let station_core: Vec<_> = open
                .iter()
                .copied()
                .filter(|building| (building.x - station.x).hypot(building.z - station.z) <= 112.0)
                .collect();
            let station_towers = station_core
                .iter()
                .filter(|building| building.floors >= 20)
                .count();
            let historic_core: Vec<_> = open
                .iter()
                .copied()
                .filter(|building| {
                    (building.x - market.x).hypot(building.z - market.z) <= 118.0
                        && (building.x - station.x).hypot(building.z - station.z) > 112.0
                })
                .collect();
            let outer: Vec<_> = open
                .iter()
                .copied()
                .filter(|building| {
                    (building.x - station.x).hypot(building.z - station.z) > 205.0
                        && (building.x - market.x).hypot(building.z - market.z) > 118.0
                })
                .collect();
            let share = |buildings: &[&Building], predicate: &dyn Fn(&Building) -> bool| {
                buildings
                    .iter()
                    .filter(|building| predicate(building))
                    .count() as f32
                    / buildings.len().max(1) as f32
            };
            let mean_floors = |buildings: &[&Building]| {
                buildings
                    .iter()
                    .map(|building| building.floors as f32)
                    .sum::<f32>()
                    / buildings.len().max(1) as f32
            };
            assert!(
                station_core.len() >= 12,
                "seed {seed} lacks a station district"
            );
            assert!(
                historic_core.len() >= 8,
                "seed {seed} lacks a founding core"
            );
            assert!(outer.len() >= 80, "seed {seed} lacks outer neighborhoods");
            assert!(
                share(&station_core, &|building| building.floors >= 5) >= 0.45,
                "seed {seed} station district did not form a taller cluster"
            );
            assert!(
                mean_floors(&station_core) >= mean_floors(&outer) + 1.0,
                "seed {seed} has no height gradient between station and outer neighborhoods"
            );
            assert!(
                share(&historic_core, &|building| building.floors <= 3) >= 0.72,
                "seed {seed} lost its low-rise founding streets"
            );
            assert!(
                historic_core
                    .iter()
                    .any(|building| building.kind == "shop" || building.kind == "cafe"),
                "seed {seed} lost everyday commerce from its founding core"
            );
            assert!(
                share(&outer, &|building| building.floors <= 3) >= 0.68,
                "seed {seed} outer neighborhoods became uniformly mid-rise"
            );
            let kinds: BTreeSet<&str> =
                open.iter().map(|building| building.kind.as_str()).collect();
            assert!(kinds.len() >= 4, "seed {seed} lost mixed neighborhood uses");
            println!(
                "district mix seed={seed} buildings={} historic={} lowrise={:.0}% station={} tall={:.0}% towers={} mean_floors={:.2} outer={} lowrise={:.0}% mean_floors={:.2} uses={:?}",
                open.len(),
                historic_core.len(),
                share(&historic_core, &|building| building.floors <= 3) * 100.0,
                station_core.len(),
                share(&station_core, &|building| building.floors >= 5) * 100.0,
                station_towers,
                mean_floors(&station_core),
                outer.len(),
                share(&outer, &|building| building.floors <= 3) * 100.0,
                mean_floors(&outer),
                kinds,
            );
            signatures.insert((
                station_core.len(),
                historic_core.len(),
                outer.len(),
                open.iter().filter(|building| building.floors >= 5).count(),
            ));
            engine.validate().unwrap();
        }
        assert!(
            signatures.len() > 1,
            "different seeds produced one fixed district mix"
        );
    }

    #[test]
    fn seed_87231_builds_bounded_station_towers_on_connected_large_lots() {
        let mut engine = Engine::new(87_231);
        engine.set_development_rate(16);
        for _ in 0..121 {
            engine.step(2_000);
        }
        engine.validate().unwrap();
        let station = engine
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "station")
            .unwrap();
        let towers: Vec<_> = engine
            .state
            .buildings
            .iter()
            .filter(|building| {
                building.stage == "open"
                    && building.floors >= 20
                    && (building.x - station.x).hypot(building.z - station.z) <= 112.0
            })
            .collect();
        let snapshot = engine.snapshot();
        for tower in &towers {
            let parcel = engine
                .state
                .parcels
                .iter()
                .find(|parcel| parcel.id == tower.parcel_id)
                .unwrap();
            assert!(parcel.width >= STATION_TOWER_PARCEL_WIDTH);
            assert!(parcel.depth >= STATION_TOWER_PARCEL_DEPTH);
            assert!(engine.node_connected(parcel.access_node));
            assert!(snapshot
                .building_access
                .iter()
                .any(|access| access.building_id == tower.id));
            assert!(matches!(tower.kind.as_str(), "office" | "apartment"));
            assert_eq!(tower.width, parcel.width);
            assert_eq!(tower.depth, parcel.depth);
        }
        let founder = engine
            .state
            .buildings
            .iter()
            .find(|building| building.anchor && building.name == "Kissa Aoba")
            .expect("the founding shop remains a protected low-rise anchor");
        assert!(founder.floors <= 3);
        let midrise = engine
            .state
            .buildings
            .iter()
            .filter(|building| {
                let distance = (building.x - station.x).hypot(building.z - station.z);
                building.stage == "open" && (82.0..205.0).contains(&distance)
            })
            .filter(|building| (5..=12).contains(&building.floors))
            .count();
        let mut candidate_sites = 0usize;
        let mut geometry_valid_sites = 0usize;
        let mut demand_ready_sites = 0usize;
        for road in engine.state.roads.iter().filter(|road| {
            road.stage == "open"
                && road.connected
                && !matches!(road.kind.as_str(), "rail" | "bridge")
        }) {
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                continue;
            };
            for (x, z, _, _, _) in engine.frontage_candidate_sites(a, b) {
                candidate_sites += 1;
                if !engine.parcel_geometry_clear(x, z, 14.0, 12.0) {
                    continue;
                }
                geometry_valid_sites += 1;
                let mut local_open = 0usize;
                let mut local_occupied = 0usize;
                for building in engine.state.buildings.iter().filter(|building| {
                    building.stage == "open" && (building.x - x).hypot(building.z - z) <= 78.0
                }) {
                    local_open += 1;
                    local_occupied += usize::from(site_active(building));
                }
                if local_open >= 4 && local_occupied >= 3 && engine.state.demand >= 1.2 {
                    demand_ready_sites += 1;
                }
            }
        }
        let building_footprint: f32 = engine
            .state
            .buildings
            .iter()
            .filter(|building| building.stage != "demolished")
            .map(|building| building.width * building.depth)
            .sum();
        let planned_segment_count = engine
            .state
            .neighborhood_plan
            .as_ref()
            .map(|plan| {
                plan.segments
                    .iter()
                    .filter(|(from, to)| {
                        engine.state.roads.iter().any(|road| {
                            road.stage == "open"
                                && ((road.from == *from && road.to == *to)
                                    || (road.from == *to && road.to == *from))
                        })
                    })
                    .count()
            })
            .unwrap_or(0);
        let mut historic_graph: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for road in engine
            .state
            .roads
            .iter()
            .filter(|road| engine.state.neighborhood_road_ids.contains(&road.id))
        {
            historic_graph.entry(road.from).or_default().insert(road.to);
            historic_graph.entry(road.to).or_default().insert(road.from);
        }
        let mut remaining_nodes: BTreeSet<u32> = historic_graph.keys().copied().collect();
        let mut completed_neighborhood_blocks = 0usize;
        while let Some(root) = remaining_nodes.iter().next().copied() {
            let mut component = BTreeSet::new();
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                if !component.insert(node) {
                    continue;
                }
                remaining_nodes.remove(&node);
                if let Some(neighbors) = historic_graph.get(&node) {
                    stack.extend(neighbors.iter().copied());
                }
            }
            let degree_sum: usize = component
                .iter()
                .filter_map(|node| historic_graph.get(node))
                .map(BTreeSet::len)
                .sum();
            let edges = degree_sum / 2;
            completed_neighborhood_blocks +=
                edges.saturating_add(1).saturating_sub(component.len());
        }
        let mut planned_road_ids = engine.state.neighborhood_road_ids.clone();
        if let Some(plan) = &engine.state.neighborhood_plan {
            for (from, to) in &plan.segments {
                if let Some(road) = engine.state.roads.iter().find(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                }) {
                    planned_road_ids.insert(road.id);
                }
            }
        }
        let planned_parcel_ids: BTreeSet<u32> = engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| planned_road_ids.contains(&frontage.road_id))
            .map(|frontage| frontage.parcel_id)
            .collect();
        let planned_buildings: Vec<_> = engine
            .state
            .buildings
            .iter()
            .filter(|building| planned_parcel_ids.contains(&building.parcel_id))
            .collect();
        let planned_residential = planned_buildings
            .iter()
            .filter(|building| matches!(building.kind.as_str(), "home" | "apartment"))
            .count();
        let planned_residential_occupied: Vec<_> = planned_buildings
            .iter()
            .filter(|building| {
                matches!(building.kind.as_str(), "home" | "apartment")
                    && building.stage == "open"
                    && building.occupancy > 0
            })
            .collect();
        let planned_commercial = planned_buildings
            .iter()
            .filter(|building| !matches!(building.kind.as_str(), "home" | "apartment"))
            .count();
        let planned_parcel_area: f32 = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| planned_parcel_ids.contains(&parcel.id))
            .map(|parcel| parcel.width * parcel.depth)
            .sum();
        let planned_occupied_footprint: f32 = planned_residential_occupied
            .iter()
            .map(|building| building.width * building.depth)
            .sum();
        println!(
            "station skyline seed=87231 tick={} buildings={} parcels={} towers={} midrise_fringe={} footprint_m2={:.0} coverage={:.1}% frontage_candidates={} geometry_valid={} demand_ready={} stage={} completed_blocks={} block_plan={}/4 planned_lots={} plan_parcel_area_m2={:.0} planned_buildings={} plan_residential={} plan_commercial={} plan_occupied_residences={} plan_occupied_footprint_m2={:.0}",
            engine.state.tick,
            engine.state.buildings.len(),
            engine.state.parcels.len(),
            towers.len(),
            midrise,
            building_footprint,
            building_footprint / 640_000.0 * 100.0,
            candidate_sites,
            geometry_valid_sites,
            demand_ready_sites,
            snapshot.stage,
            completed_neighborhood_blocks,
            planned_segment_count,
            planned_parcel_ids.len(),
            planned_parcel_area,
            planned_buildings.len(),
            planned_residential,
            planned_commercial,
            planned_residential_occupied.len(),
            planned_occupied_footprint,
        );
        assert!(
            !planned_residential_occupied.is_empty(),
            "block frontage added no occupied residential stock"
        );
        assert!(
            towers.len() >= 2,
            "expected a compact cluster of real towers"
        );
        assert!(
            completed_neighborhood_blocks >= 2,
            "seed 87231 formed only {completed_neighborhood_blocks} completed compact blocks by tick {}",
            engine.state.tick
        );
        assert!(engine.state.neighborhood_road_ids.iter().all(|id| {
            engine
                .state
                .roads
                .iter()
                .any(|road| road.id == *id && road.stage == "open" && road.connected)
        }));
        assert!(
            midrise >= 8,
            "station fringe lacks 5–12 storey midrise fabric"
        );
        assert_eq!(
            engine.save_json(),
            Engine::restore_json(&engine.save_json())
                .unwrap()
                .save_json()
        );
    }

    #[test]
    fn station_tower_frontages_are_reserved_before_rail_and_small_lots() {
        let mut engine = Engine::new(31415);
        engine.state.nodes.clear();
        engine.state.roads.clear();
        engine.state.parcels.clear();
        engine.state.parcel_frontages.clear();
        engine.state.buildings.clear();
        engine.state.station_online = false;
        let station = engine.add_node(0.0, 200.0, "station");
        let left_a = engine.add_node(-40.0, 150.0, "junction");
        let left_b = engine.add_node(40.0, 150.0, "junction");
        let right_a = engine.add_node(-40.0, 250.0, "junction");
        let right_b = engine.add_node(40.0, 250.0, "junction");
        let lower = engine.add_road(left_a, left_b, "local", "open", 1.0);
        let upper = engine.add_road(right_a, right_b, "local", "open", 1.0);
        let growth_rng = engine.state.rng_growth;

        engine.add_frontage_parcels(lower);
        assert_eq!(engine.station_tower_frontage_reservations(), 1);
        let first_reservation = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.width >= STATION_TOWER_PARCEL_WIDTH)
            .unwrap();
        assert!(engine
            .state
            .parcels
            .iter()
            .filter(|parcel| parcel.width < STATION_TOWER_PARCEL_WIDTH)
            .all(|parcel| {
                (parcel.x - first_reservation.x).abs()
                    >= (parcel.width + first_reservation.width) * 0.5 + 2.0
                    || (parcel.z - first_reservation.z).abs()
                        >= (parcel.depth + first_reservation.depth) * 0.5 + 2.0
            }));
        engine.add_frontage_parcels(upper);

        let reservations: Vec<_> = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| {
                parcel.width >= STATION_TOWER_PARCEL_WIDTH
                    && parcel.depth >= STATION_TOWER_PARCEL_DEPTH
            })
            .collect();
        assert_eq!(reservations.len(), MIN_STATION_TOWER_FRONTAGE_RESERVATIONS);
        assert_eq!(engine.station_tower_frontage_reservations(), 2);
        assert!(engine.state.parcel_frontages.iter().any(|frontage| {
            frontage.road_id == lower
                && reservations
                    .iter()
                    .any(|parcel| parcel.id == frontage.parcel_id)
        }));
        assert!(engine.state.parcel_frontages.iter().any(|frontage| {
            frontage.road_id == upper
                && reservations
                    .iter()
                    .any(|parcel| parcel.id == frontage.parcel_id)
        }));
        assert!(engine.state.buildings.is_empty());
        assert_eq!(engine.state.rng_growth, growth_rng);
        assert_eq!(
            engine.save_json(),
            Engine::restore_json(&engine.save_json())
                .unwrap()
                .save_json()
        );
        assert_eq!(station, engine.state.nodes[0].id);
    }
    #[test]
    fn long_run_keeps_state_collections_bounded_after_expansion() {
        use std::time::Instant;
        let mut engine = Engine::new(1204);
        engine.set_development_rate(60);
        let started = Instant::now();
        let mut max_travelers = 0usize;
        for batch in 0..150 {
            engine.step(2_000);
            max_travelers = max_travelers.max(engine.state.travelers.len());
            if batch % 10 == 0 {
                engine.validate().unwrap();
            }
        }
        engine.validate().unwrap();
        let snapshot = engine.snapshot();
        assert!(engine.state.events.len() <= MAX_EVENTS);
        assert!(engine.state.projects.len() <= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY);
        assert!(engine.state.parcels.len() <= MAX_BUILDINGS * 2);
        assert!(engine.state.buildings.len() <= MAX_BUILDINGS);
        assert!(engine.state.roads.len() <= MAX_SEGMENTS);
        assert!(max_travelers <= MAX_TRAVELERS);
        let ordinary_open_roads: BTreeSet<u32> = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .map(|road| road.id)
            .collect();
        let frontage_lots = engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| ordinary_open_roads.contains(&frontage.road_id))
            .count();
        let frontage_per_road = frontage_lots as f32 / ordinary_open_roads.len().max(1) as f32;
        assert!(
            frontage_per_road > 2.0,
            "mature ordinary streets average only {frontage_per_road:.2} frontage lots"
        );
        println!(
            "long-run profile: tick={} stage={} buildings={} parcels={} open_roads={} ordinary_frontage_lots={} lots_per_ordinary_road={:.2} population={} max_travelers={} projects={} events={} elapsed_ms={}",
            snapshot.tick,
            snapshot.stage,
            engine.state.buildings.len(),
            engine.state.parcels.len(),
            engine.state.roads.iter().filter(|r| r.stage == "open").count(),
            frontage_lots,
            frontage_per_road,
            snapshot.stats.population,
            max_travelers,
            engine.state.projects.len(),
            engine.state.events.len(),
            started.elapsed().as_millis(),
        );
    }
    #[test]
    fn river_crossing_and_founder_are_present() {
        let e = Engine::new(5);
        let s = e.snapshot();
        assert!(s.roads.iter().any(|r| r.kind == "bridge"));
        let founder = s
            .buildings
            .iter()
            .find(|b| b.name == "Kissa Aoba" && b.anchor)
            .expect("the founding shop remains present");
        assert!(e.parcel_clear_of_river(founder.x, founder.z, founder.width, founder.depth));
        assert!(e.distance_to_river(founder.x, founder.z) < 80.0);
        assert!(s.terrain.river.len() >= 4);
        for seed in 0..5 {
            let world = Engine::new(seed);
            let snapshot = world.snapshot();
            let shop = snapshot
                .buildings
                .iter()
                .find(|building| building.name == "Kissa Aoba")
                .unwrap();
            assert!(
                world.parcel_clear_of_river(shop.x, shop.z, shop.width, shop.depth),
                "seed {seed}: founder footprint infringes the water/bank clearance"
            );
            assert!(
                world.distance_to_river(shop.x, shop.z) <= 70.0,
                "seed {seed}: founder is no longer a waterfront building"
            );
            assert!(snapshot
                .parcels
                .iter()
                .all(|parcel| world.parcel_clear_of_river(
                    parcel.x,
                    parcel.z,
                    parcel.width,
                    parcel.depth
                )));
            assert!(
                snapshot
                    .roads
                    .iter()
                    .filter(|road| {
                        road.stage == "open" && !matches!(road.kind.as_str(), "rail" | "bridge")
                    })
                    .all(|road| {
                        let from = world.node(road.from).unwrap();
                        let to = world.node(road.to).unwrap();
                        world.ordinary_road_clear_of_river(from.x, from.z, to.x, to.z, 4.5)
                    }),
                "seed {seed}: an ordinary initial road enters the river reserve"
            );
            world.validate().unwrap();
        }
    }
    #[test]
    fn time_and_rate_are_integer_deterministic() {
        let mut e = Engine::new(2);
        assert_eq!(e.snapshot().minute_of_day, 360.0);
        for _ in 0..10 {
            e.step(2_000);
        }
        let t = e.snapshot();
        assert_eq!(t.day, 0);
        assert!((t.minute_of_day - 120.0).abs() < 0.1);
        e.set_development_rate(0);
        let pop = e.state.population;
        e.step(1000);
        assert_eq!(e.state.population, pop);
    }

    #[test]
    #[ignore = "bounded diagnostic over mature public-WASM save fixtures"]
    fn diagnose_mature_neighborhood_and_road_proposal_rejections() {
        for (name, json) in [
            (
                "seed87231",
                include_str!("../../validation/mature-world.ranked-blocks.json"),
            ),
            (
                "seed0",
                include_str!("../../validation/ranked-blocks-seed0.json"),
            ),
        ] {
            let engine = Engine::restore_json(json).expect("public save should restore natively");
            let audit = audit_mature_neighborhood_proposals(&engine);
            println!(
                "{name}: tick={} seed={} buildings={} roads={} parcels={} active_projects={} demand={} population={} anchor_cursor={} nodes={} neighborhood_audit={audit:?}",
                engine.state.tick,
                engine.state.seed,
                engine.state.buildings.len(),
                engine.state.roads.len(),
                engine.state.parcels.len(),
                engine.state.projects.iter().filter(|project| project.progress < 1.0).count(),
                engine.state.demand,
                engine.state.population,
                engine.state.neighborhood_anchor_cursor,
                engine.state.nodes.len(),
            );

            // Exercise the real bounded planner over multiple cursor windows,
            // without advancing time or modifying the saved fixture.
            let mut planner_probe =
                Engine::restore_json(&engine.save_json()).expect("diagnostic clone restores");
            let mut selected_anchor = None;
            let start_cursor = planner_probe.state.neighborhood_anchor_cursor;
            for _ in 0..=planner_probe.state.nodes.len() {
                if planner_probe.try_create_neighborhood_plan() {
                    selected_anchor = planner_probe
                        .state
                        .neighborhood_plan
                        .as_ref()
                        .map(|plan| plan.anchor_node);
                    break;
                }
                if planner_probe.state.neighborhood_anchor_cursor == start_cursor {
                    break;
                }
            }
            println!("{name}: real_planner_selected_anchor={selected_anchor:?}");

            // Probe the actual road planner through several rotated origin
            // windows. If it finds a street, report its endpoint coordinates.
            let mut road_probe =
                Engine::restore_json(&engine.save_json()).expect("diagnostic clone restores");
            let mut road_proposal = None;
            for _ in 0..32 {
                let before = road_probe.state.projects.len();
                road_probe.plan_road();
                if road_probe.state.projects.len() > before {
                    if let Some(project) = road_probe
                        .state
                        .projects
                        .iter()
                        .rev()
                        .find(|project| project.kind == "road" && project.progress < 1.0)
                    {
                        let from = project.from.and_then(|id| road_probe.node(id));
                        let to = project.to.and_then(|id| road_probe.node(id));
                        road_proposal = Some((
                            project.id,
                            from.map(|node| (node.id, node.x, node.z)),
                            to.map(|node| (node.id, node.x, node.z)),
                            project.reason.clone(),
                        ));
                    }
                    break;
                }
            }
            println!("{name}: real_road_proposal={road_proposal:?}");
        }
    }

    #[test]
    #[ignore = "bounded offline geometry probe for mid-edge street access on mature saves"]
    fn diagnose_mid_edge_access_feasibility_on_mature_saves() {
        for (name, json) in [
            (
                "seed87231",
                include_str!("../../validation/mature-world.ranked-blocks.json"),
            ),
            (
                "seed0",
                include_str!("../../validation/ranked-blocks-seed0.json"),
            ),
        ] {
            let engine = Engine::restore_json(json).expect("public save should restore natively");
            let audit = audit_mid_edge_access_candidates(&engine);
            println!(
                "{name}: hosts={} attempted={} rejected={:?} safe_lots_in_top8={} feasible_top8={:?}",
                audit.hosts,
                audit.attempted,
                audit.rejected,
                audit.safe_lot_count,
                audit.feasible,
            );
        }
    }

    fn split_test_resident(
        id: u32,
        building: u32,
        home_node: u32,
        work_node: u32,
        transfer: &str,
        train_id: Option<u32>,
        bus_id: Option<u32>,
        bus_stops: Option<(u32, u32)>,
    ) -> Resident {
        Resident {
            id,
            name: format!("Split Test Resident {id}"),
            home_building: building,
            work_building: Some(building),
            home_node,
            work_node: Some(work_node),
            mode: 0,
            routine: "worker".into(),
            activity: "in transit".into(),
            at_work: true,
            departure_tick: 0,
            return_tick: 10_000,
            last_departure_day: u32::MAX,
            last_return_day: u32::MAX,
            history: vec!["Riding a connected route".into()],
            rail_commuter: train_id.is_some(),
            transfer: transfer.into(),
            train_id,
            queue_slot: None,
            bus_id,
            bus_origin_stop: bus_stops.map(|stops| stops.0),
            bus_destination_stop: bus_stops.map(|stops| stops.1),
            bus_stop_node: None,
        }
    }

    fn route_position(engine: &Engine, traveler: &Traveler) -> WorldPoint {
        if traveler.edge_index >= traveler.route_lengths.len() {
            let node = engine.node(*traveler.route.last().unwrap()).unwrap();
            return WorldPoint {
                x: node.x,
                z: node.z,
            };
        }
        let a = engine.node(traveler.route[traveler.edge_index]).unwrap();
        let b = engine
            .node(traveler.route[traveler.edge_index + 1])
            .unwrap();
        let length = traveler.route_lengths[traveler.edge_index];
        let fraction = (traveler.edge_progress / length).clamp(0.0, 1.0);
        WorldPoint {
            x: a.x + (b.x - a.x) * fraction,
            z: a.z + (b.z - a.z) * fraction,
        }
    }

    #[test]
    fn road_split_preserves_frontage_travelers_bus_train_and_replay() {
        let mut engine = Engine::new(7);
        let host = engine
            .state
            .roads
            .iter()
            .find(|road| road.kind == "arterial" && road.stage == "open")
            .unwrap()
            .clone();
        engine.state.neighborhood_road_ids.insert(host.id);
        let a = engine.node(host.from).unwrap().clone();
        let b = engine.node(host.to).unwrap().clone();
        let east = engine
            .state
            .roads
            .iter()
            .find(|road| road.id != host.id && (road.from == b.id || road.to == b.id))
            .map(|road| {
                if road.from == b.id {
                    road.to
                } else {
                    road.from
                }
            })
            .unwrap();
        let t = 0.6;
        let host_length = distance(&a, &b);
        let route = vec![a.id, b.id, east, b.id, a.id];
        let route_lengths = engine.route_lengths(&route);

        let parcel_id = engine.add_parcel(250.0, 60.0, 14.0, 12.0, a.id, 0.8, false);
        engine.state.parcel_frontages.push(ParcelFrontage {
            parcel_id,
            road_id: host.id,
            along: 0.35,
            side: -1,
        });
        let building_id = engine.add_building(
            parcel_id,
            "home",
            2,
            "open",
            1.0,
            8,
            "Split Test Home",
            false,
            vec!["Opened beside the arterial".into()],
        );
        let before_access = engine.building_access_segment(building_id).unwrap();

        let second_parcel = engine.add_parcel(120.0, 75.0, 14.0, 12.0, b.id, 0.8, false);
        engine.state.parcel_frontages.push(ParcelFrontage {
            parcel_id: second_parcel,
            road_id: host.id,
            along: 0.85,
            side: -1,
        });
        let second_building_id = engine.add_building(
            second_parcel,
            "home",
            2,
            "open",
            1.0,
            8,
            "Second Split Test Home",
            false,
            vec!["Opened beside the far half".into()],
        );
        let before_second_access = engine.building_access_segment(second_building_id).unwrap();

        let actor_positions = [
            (0, host_length * 0.3, 1),
            (0, host_length * 0.8, 1),
            (1, route_lengths[1] * 0.45, 1),
            (3, host_length * 0.8, 1),
            (4, 0.0, 0),
        ];
        for (index, (edge_index, edge_progress, access_phase)) in
            actor_positions.into_iter().enumerate()
        {
            let id = engine.id();
            let mut traveler = Traveler {
                id,
                kind: 0,
                route: route.clone(),
                route_lengths: route_lengths.clone(),
                edge_index,
                edge_progress,
                speed: 1.2,
                destination: *route.last().unwrap(),
                activity: "split regression".into(),
                resident: 0,
                name: format!("Split walker {index}"),
                origin_building: None,
                destination_building: None,
                wait_ticks: 0,
                origin_access: None,
                destination_access: None,
                access_phase,
                access_progress: if access_phase == 0 { 2.75 } else { 0.0 },
            };
            if access_phase == 0 {
                traveler.origin_access = Some(AccessPath {
                    points: vec![
                        WorldPoint { x: 250.0, z: 66.0 },
                        WorldPoint { x: 250.0, z: 50.0 },
                    ],
                });
            }
            engine.state.travelers.push(traveler);
        }
        let before_positions: Vec<_> = engine
            .state
            .travelers
            .iter()
            .map(|traveler| route_position(&engine, traveler))
            .collect();
        let original_access_progress = engine
            .state
            .travelers
            .iter()
            .map(|traveler| (traveler.access_phase, traveler.access_progress))
            .collect::<Vec<_>>();

        let line_id = engine.id();
        let line_stops = vec![a.id, b.id, east];
        engine.state.bus_line = Some(BusLine {
            id: line_id,
            stops: line_stops.clone(),
            route: vec![a.id, b.id, east],
            active: true,
            graph_revision: 4,
        });
        let bus_id = engine.id();
        let bus_resident_id = engine.id();
        engine.state.buses.push(Bus {
            id: bus_id,
            line_id,
            progress: host_length * 0.72,
            direction: 1,
            dwell_ticks: 0,
            passengers: vec![bus_resident_id],
            next_departure_tick: 500,
            last_stop: Some(a.id),
        });
        engine.state.residents.push(split_test_resident(
            bus_resident_id,
            building_id,
            a.id,
            east,
            "riding_bus",
            None,
            Some(bus_id),
            Some((a.id, east)),
        ));
        let bus_pose_before = engine
            .bus_pose(
                engine.state.bus_line.as_ref().unwrap(),
                &engine.state.buses[0],
            )
            .unwrap();

        let train_id = engine.id();
        let train_resident_id = engine.id();
        engine.state.station_online = true;
        engine.state.trains.push(Train {
            id: train_id,
            route: vec![a.id, b.id],
            distance: host_length * 0.72,
            speed: 8.0,
            direction: 1,
            dwell_ticks: 0,
            passengers: vec![train_resident_id],
        });
        engine.state.residents.push(split_test_resident(
            train_resident_id,
            building_id,
            a.id,
            b.id,
            "riding_outbound",
            Some(train_id),
            None,
            None,
        ));
        let revision_before = engine.state.revision;
        let access_revision_before = engine.state.access_geometry_revision;
        let old_road_count = engine.state.roads.len();

        let outcome = engine.try_split_open_road(host.id, t).unwrap();
        assert!(engine.state.access_geometry_revision > access_revision_before);
        assert_eq!(outcome.retained_road_id, host.id);
        assert_eq!(engine.state.roads.len(), old_road_count + 1);
        let retained = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == outcome.retained_road_id)
            .unwrap();
        let appended = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == outcome.new_road_id)
            .unwrap();
        assert_eq!((retained.from, retained.to), (a.id, outcome.split_node_id));
        assert_eq!((appended.from, appended.to), (outcome.split_node_id, b.id));
        assert!(
            engine
                .state
                .roads
                .iter()
                .any(|road| { road.from == a.id && road.to == b.id && road.id != host.id })
                == false
        );
        assert_eq!(engine.state.revision, revision_before + 1);

        for (index, traveler) in engine.state.travelers.iter().enumerate() {
            let after = route_position(&engine, traveler);
            assert!((before_positions[index].x - after.x).abs() < 1e-3);
            assert!((before_positions[index].z - after.z).abs() < 1e-3);
            assert_eq!(
                (traveler.access_phase, traveler.access_progress),
                original_access_progress[index]
            );
        }
        let after_access = engine.building_access_segment(building_id).unwrap();
        assert_eq!(after_access.0, host.id);
        assert!((before_access.1[0].x - after_access.1[0].x).abs() < 1e-4);
        assert!((before_access.1[0].z - after_access.1[0].z).abs() < 1e-4);
        assert!((before_access.1[1].x - after_access.1[1].x).abs() < 1e-3);
        assert!((before_access.1[1].z - after_access.1[1].z).abs() < 1e-3);
        let after_second_access = engine.building_access_segment(second_building_id).unwrap();
        assert_eq!(after_second_access.0, outcome.new_road_id);
        assert!((before_second_access.1[0].x - after_second_access.1[0].x).abs() < 1e-4);
        assert!((before_second_access.1[0].z - after_second_access.1[0].z).abs() < 1e-4);
        assert!((before_second_access.1[1].x - after_second_access.1[1].x).abs() < 1e-3);
        assert!((before_second_access.1[1].z - after_second_access.1[1].z).abs() < 1e-3);
        assert_eq!(engine.state.bus_line.as_ref().unwrap().stops, line_stops);
        assert_eq!(
            engine.state.bus_line.as_ref().unwrap().route,
            vec![a.id, outcome.split_node_id, b.id, east]
        );
        assert_eq!(engine.state.bus_line.as_ref().unwrap().graph_revision, 5);
        assert_eq!(engine.state.buses[0].progress, host_length * 0.72);
        assert_eq!(engine.state.buses[0].passengers, vec![bus_resident_id]);
        let bus_pose_after = engine
            .bus_pose(
                engine.state.bus_line.as_ref().unwrap(),
                &engine.state.buses[0],
            )
            .unwrap();
        assert!((bus_pose_before.0 - bus_pose_after.0).abs() < 1e-3);
        assert!((bus_pose_before.1 - bus_pose_after.1).abs() < 1e-3);
        assert_eq!(engine.state.trains[0].distance, host_length * 0.72);
        assert_eq!(engine.state.trains[0].passengers, vec![train_resident_id]);
        assert_eq!(engine.state.residents[0].home_node, a.id);
        assert_eq!(engine.state.residents[0].work_node, Some(east));
        assert!(engine.state.neighborhood_road_ids.contains(&host.id));
        assert!(engine
            .state
            .neighborhood_road_ids
            .contains(&outcome.new_road_id));
        engine.validate().unwrap();

        let canonical = engine.save_json();
        let mut resumed = Engine::restore_json(&canonical).unwrap();
        assert_eq!(resumed.save_json(), canonical);
        engine.step(200);
        resumed.step(200);
        assert_eq!(engine.save_json(), resumed.save_json());
    }

    #[test]
    fn road_split_rejections_leave_state_and_ids_unchanged() {
        let mut engine = Engine::new(8);
        let host = engine
            .state
            .roads
            .iter()
            .find(|road| road.kind == "arterial" && road.stage == "open")
            .unwrap()
            .clone();
        let before = engine.save_json();
        let before_access_revision = engine.state.access_geometry_revision;
        assert_eq!(
            engine.try_split_open_road(host.id, 0.001),
            Err(RoadSplitError::SegmentTooShort)
        );
        assert_eq!(engine.save_json(), before);
        assert_eq!(engine.state.access_geometry_revision, before_access_revision);

        let parcel_id = engine.add_parcel(250.0, 60.0, 14.0, 12.0, host.from, 0.8, false);
        engine.state.parcel_frontages.push(ParcelFrontage {
            parcel_id,
            road_id: host.id,
            along: 0.75,
            side: -1,
        });
        let before = engine.save_json();
        assert_eq!(
            engine.try_split_open_road(host.id, 0.5),
            Err(RoadSplitError::UnsafeFrontage)
        );
        assert_eq!(engine.save_json(), before);

        // An otherwise valid host cannot be split when the route expansion
        // would cross the public per-traveler route bound.
        engine.state.parcel_frontages.clear();
        engine.state.parcels.retain(|parcel| parcel.id != parcel_id);
        let mut route = Vec::with_capacity(MAX_ROUTE_NODES);
        for index in 0..MAX_ROUTE_NODES {
            route.push(if index % 2 == 0 { host.from } else { host.to });
        }
        let long_traveler_id = engine.id();
        engine.state.travelers.push(Traveler {
            id: long_traveler_id,
            kind: 1,
            route,
            route_lengths: vec![host.length; MAX_ROUTE_NODES - 1],
            edge_index: 0,
            edge_progress: 0.0,
            speed: 1.0,
            destination: host.to,
            activity: "route cap test".into(),
            resident: 0,
            name: "Long route".into(),
            origin_building: None,
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        });
        let before = engine.save_json();
        assert_eq!(
            engine.try_split_open_road(host.id, 0.5),
            Err(RoadSplitError::Capacity)
        );
        assert_eq!(engine.save_json(), before);

        let mut bus_engine = Engine::new(9);
        let bus_host = bus_engine
            .state
            .roads
            .iter()
            .find(|road| road.kind == "arterial" && road.stage == "open")
            .unwrap()
            .clone();
        let long_bus_route: Vec<_> = (0..MAX_BUS_ROUTE_NODES)
            .map(|index| {
                if index % 2 == 0 {
                    bus_host.from
                } else {
                    bus_host.to
                }
            })
            .collect();
        let bus_line_id = bus_engine.id();
        bus_engine.state.bus_line = Some(BusLine {
            id: bus_line_id,
            stops: vec![bus_host.from, bus_host.to, bus_host.from],
            route: long_bus_route,
            active: true,
            graph_revision: 4,
        });
        let before = bus_engine.save_json();
        assert_eq!(
            bus_engine.try_split_open_road(bus_host.id, 0.5),
            Err(RoadSplitError::Capacity)
        );
        assert_eq!(bus_engine.save_json(), before);

        let mut project_engine = Engine::new(10);
        let project_host = project_engine
            .state
            .roads
            .iter()
            .find(|road| road.kind == "arterial" && road.stage == "open")
            .unwrap()
            .clone();
        let project_id = project_engine.id();
        project_engine.state.projects.push(Project {
            id: project_id,
            kind: "road".into(),
            from: Some(project_host.from),
            to: Some(project_host.to),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.5,
            reason: "Active host construction".into(),
            duration: 8.0,
            started_tick: 0,
        });
        let before = project_engine.save_json();
        assert_eq!(
            project_engine.try_split_open_road(project_host.id, 0.5),
            Err(RoadSplitError::ActiveProject)
        );
        assert_eq!(project_engine.save_json(), before);

        let mut plan_engine = Engine::new(11);
        let plan_host = plan_engine
            .state
            .roads
            .iter()
            .find(|road| road.kind == "arterial" && road.stage == "open")
            .unwrap()
            .clone();
        plan_engine.state.neighborhood_plan = Some(NeighborhoodPlan {
            anchor_node: plan_host.from,
            segments: vec![(plan_host.from, plan_host.to); 4],
            next_segment: 0,
            coordinated_lots: false,
            selected_lots: None,
            spine: None,
        });
        let before = plan_engine.save_json();
        assert_eq!(
            plan_engine.try_split_open_road(plan_host.id, 0.5),
            Err(RoadSplitError::ActivePlan)
        );
        assert_eq!(plan_engine.save_json(), before);
    }

    #[test]
    #[ignore = "bounded integration probe for mid-edge splits on mature public saves"]
    fn mature_fixture_road_splits_preserve_routes_access_and_replay() {
        let cases = [
            (
                "seed87231-host2517",
                include_str!("../../validation/mature-world.ranked-blocks.json"),
                2_517,
                0.5,
            ),
            (
                "seed0-host1739",
                include_str!("../../validation/ranked-blocks-seed0.json"),
                1_739,
                0.75,
            ),
            (
                "seed0-host1729",
                include_str!("../../validation/ranked-blocks-seed0.json"),
                1_729,
                0.75,
            ),
            (
                "seed0-host2970",
                include_str!("../../validation/ranked-blocks-seed0.json"),
                2_970,
                0.75,
            ),
        ];

        for (name, fixture, host_id, t) in cases {
            let mut engine = Engine::restore_json(fixture)
                .unwrap_or_else(|error| panic!("{name}: source fixture did not restore: {error}"));
            let host = engine
                .state
                .roads
                .iter()
                .find(|road| road.id == host_id)
                .unwrap_or_else(|| panic!("{name}: host road is missing"))
                .clone();
            assert!(
                host.stage == "open" && host.connected,
                "{name}: host is not open"
            );
            let before_positions: Vec<_> = engine
                .state
                .travelers
                .iter()
                .map(|traveler| route_position(&engine, traveler))
                .collect();
            let before_access: Vec<_> = engine
                .state
                .parcel_frontages
                .iter()
                .filter(|frontage| frontage.road_id == host_id)
                .filter_map(|frontage| {
                    let building = engine
                        .state
                        .buildings
                        .iter()
                        .find(|building| building.parcel_id == frontage.parcel_id)?;
                    engine
                        .building_access_segment(building.id)
                        .map(|(_, segment)| (building.id, segment))
                })
                .collect();
            let bus_pose = engine.state.bus_line.as_ref().and_then(|line| {
                engine
                    .state
                    .buses
                    .first()
                    .and_then(|bus| engine.bus_pose(line, bus))
            });
            let train_distances: Vec<_> = engine
                .state
                .trains
                .iter()
                .map(|train| train.distance)
                .collect();
            let access_count = before_access.len();

            let outcome = engine
                .try_split_open_road(host_id, t)
                .unwrap_or_else(|error| panic!("{name}: expected safe split, got {error:?}"));
            assert_eq!(outcome.retained_road_id, host_id, "{name}");
            for (index, traveler) in engine.state.travelers.iter().enumerate() {
                let after = route_position(&engine, traveler);
                assert!(
                    (before_positions[index].x - after.x).abs() < 0.002
                        && (before_positions[index].z - after.z).abs() < 0.002,
                    "{name}: traveler {} moved during topology split",
                    traveler.id
                );
            }
            for (building_id, before) in before_access {
                let after = engine
                    .building_access_segment(building_id)
                    .expect("frontage building keeps a door connector");
                assert!(
                    (before[0].x - after.1[0].x).abs() < 0.001
                        && (before[0].z - after.1[0].z).abs() < 0.001
                        && (before[1].x - after.1[1].x).abs() < 0.002
                        && (before[1].z - after.1[1].z).abs() < 0.002,
                    "{name}: building {building_id} connector moved"
                );
            }
            if let Some(before) = bus_pose {
                let after = engine
                    .bus_pose(
                        engine.state.bus_line.as_ref().unwrap(),
                        engine.state.buses.first().unwrap(),
                    )
                    .unwrap();
                assert!(
                    (before.0 - after.0).abs() < 0.002 && (before.1 - after.1).abs() < 0.002,
                    "{name}: bus moved during topology split"
                );
            }
            assert_eq!(
                engine
                    .state
                    .trains
                    .iter()
                    .map(|train| train.distance)
                    .collect::<Vec<_>>(),
                train_distances,
                "{name}: train progress changed"
            );
            engine
                .validate()
                .unwrap_or_else(|error| panic!("{name}: split save failed validation: {error}"));

            let canonical = engine.save_json();
            let mut resumed = Engine::restore_json(&canonical)
                .unwrap_or_else(|error| panic!("{name}: split save did not restore: {error}"));
            assert_eq!(resumed.save_json(), canonical, "{name}: save not canonical");
            engine.step(2_000);
            resumed.step(2_000);
            assert_eq!(
                engine.save_json(),
                resumed.save_json(),
                "{name}: replay diverged"
            );
            println!(
                "{name}: split {} into {}, travelers={}, frontages_preserved={}",
                outcome.retained_road_id,
                outcome.new_road_id,
                before_positions.len(),
                access_count
            );
        }
    }

    #[test]
    fn interior_access_plan_is_visible_reserved_atomic_and_replayable() {
        let mut engine = Engine::restore_json(include_str!(
            "../../validation/mature-world.ranked-blocks.json"
        ))
        .expect("mature public city fixture restores");
        assert!(engine.state.neighborhood_plan.is_none());
        assert!(engine.state.pending_interior_access.is_none());
        engine.state.demand = 1.2;

        let proposal_bound = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
            })
            .count()
            .saturating_mul(interior_access::PROPOSALS_PER_HOST);
        let evaluations = proposal_bound
            .div_ceil(interior_access::MAX_PROPOSALS_PER_EVALUATION)
            .saturating_add(1);
        let mut admitted = false;
        for _ in 0..evaluations {
            if engine.try_plan_interior_access() {
                admitted = true;
                break;
            }
        }
        assert!(admitted, "mature feasible frontage was never admitted");
        let plan = engine.state.pending_interior_access.clone().unwrap();
        assert!((2..=interior_access::MAX_SELECTED_LOTS).contains(&plan.selected_lots.len()));
        let branch = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
            .unwrap();
        assert_eq!(branch.stage, "survey");
        assert!(!branch.connected);
        let project = engine
            .state
            .projects
            .iter()
            .find(|project| project.id == plan.project_id)
            .unwrap();
        assert_eq!(project.kind, "road");
        assert!(project.progress < 1.0);
        assert_eq!(branch.progress, project.progress);
        let visible = engine.snapshot();
        assert!(visible
            .roads
            .iter()
            .any(|road| road.id == plan.branch_road_id && road.stage != "open"));
        let parcels_before = engine.state.parcels.len();
        for lot in &plan.selected_lots {
            let rect = geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            assert!(
                !engine.oriented_parcel_geometry_clear(&rect),
                "reserved lot admitted by another path"
            );
        }
        let first_lot = plan.selected_lots[0];
        let branch = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
            .unwrap();
        let branch_a = engine.node(branch.from).unwrap();
        let branch_b = engine.node(branch.to).unwrap();
        let branch_length = (branch_b.x - branch_a.x).hypot(branch_b.z - branch_a.z);
        let along_x = (branch_b.x - branch_a.x) / branch_length;
        let along_z = (branch_b.z - branch_a.z) / branch_length;
        let probe_a = (first_lot.x - along_x * 20.0, first_lot.z - along_z * 20.0);
        let probe_b = (first_lot.x + along_x * 20.0, first_lot.z + along_z * 20.0);
        assert!(
            point_segment_distance(
                first_lot.x,
                first_lot.z,
                branch_a.x,
                branch_a.z,
                branch_b.x,
                branch_b.z,
            ) >= 8.0
        );
        assert!(
            !engine.interior_plan_lots_clear_of_road(probe_a.0, probe_a.1, probe_b.0, probe_b.1)
        );
        assert_eq!(engine.state.parcels.len(), parcels_before);
        engine.advance_projects();
        let progressed_branch = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
            .unwrap();
        let progressed_project = engine
            .state
            .projects
            .iter()
            .find(|project| project.id == plan.project_id)
            .unwrap();
        assert!(progressed_project.progress > 0.0 && progressed_project.progress < 1.0);
        assert_eq!(progressed_branch.progress, progressed_project.progress);
        assert!(
            !progressed_branch.connected,
            "construction branch must not be traversable"
        );
        engine
            .validate()
            .expect("pending plan satisfies restore invariants");

        let save = engine.save_json();
        let mut resumed = Engine::restore_json(&save).expect("pending plan restores canonically");
        assert_eq!(resumed.save_json(), save);
        let mut invalid = Engine::restore_json(&save).unwrap();
        let corrupt_plan = invalid.state.pending_interior_access.as_mut().unwrap();
        corrupt_plan.project_id = u32::MAX;
        assert!(Engine::restore_json(&invalid.save_json()).is_err());
        let mut invalid_fraction = Engine::restore_json(&save).unwrap();
        let plan_t = invalid_fraction
            .state
            .pending_interior_access
            .as_ref()
            .unwrap()
            .host_t;
        invalid_fraction
            .state
            .pending_interior_access
            .as_mut()
            .unwrap()
            .host_t = if (plan_t - 0.25).abs() < 0.001 {
            0.75
        } else {
            0.25
        };
        assert!(Engine::restore_json(&invalid_fraction.save_json()).is_err());
        let mut invalid_branch = Engine::restore_json(&save).unwrap();
        let endpoint = invalid_branch
            .state
            .pending_interior_access
            .as_ref()
            .unwrap()
            .branch_endpoint_id;
        let endpoint_node = invalid_branch
            .state
            .nodes
            .iter_mut()
            .find(|node| node.id == endpoint)
            .unwrap();
        endpoint_node.x += 1.0;
        assert!(Engine::restore_json(&invalid_branch.save_json()).is_err());
        let mut blocked = Engine::restore_json(&save).unwrap();
        let existing = blocked.state.parcels[0].clone();
        let blocked_plan = blocked.state.pending_interior_access.as_mut().unwrap();
        blocked_plan.selected_lots[0].x = existing.x;
        blocked_plan.selected_lots[0].z = existing.z;
        blocked
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == plan.project_id)
            .unwrap()
            .duration = 1.0;
        let blocked_parcel_count = blocked.state.parcels.len();
        blocked.advance_projects();
        assert!(blocked.state.pending_interior_access.is_some());
        assert_eq!(blocked.state.parcels.len(), blocked_parcel_count);
        assert_ne!(
            blocked
                .state
                .roads
                .iter()
                .find(|road| road.id == plan.branch_road_id)
                .unwrap()
                .stage,
            "open"
        );
        assert!(
            blocked
                .state
                .projects
                .iter()
                .find(|project| project.id == plan.project_id)
                .unwrap()
                .progress
                < 1.0
        );
        for instance in [&mut engine, &mut resumed] {
            let project = instance
                .state
                .projects
                .iter_mut()
                .find(|project| project.id == plan.project_id)
                .unwrap();
            project.duration = 1.0;
            instance.advance_projects();
            instance
                .validate()
                .expect("completed access plan preserves invariants");
        }
        assert_eq!(
            engine.save_json(),
            resumed.save_json(),
            "completion replays identically"
        );
        assert!(engine.state.pending_interior_access.is_none());
        let branch = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
            .unwrap();
        assert_eq!(branch.stage, "open");
        assert!(branch.connected);
        assert_eq!(
            engine.state.parcels.len(),
            parcels_before + plan.selected_lots.len()
        );
        assert_eq!(
            engine
                .state
                .parcel_frontages
                .iter()
                .filter(|frontage| frontage.road_id == plan.branch_road_id)
                .count(),
            plan.selected_lots.len()
        );
    }

    #[test]
    fn interior_access_capacity_preflight_does_not_split_or_allocate() {
        let mut engine = Engine::new(41);
        engine.state.demand = 1.2;
        engine.state.next_id = u32::MAX - 4;
        let before_nodes = engine.state.nodes.clone();
        let before_roads = engine.state.roads.clone();
        let before_projects = engine.state.projects.clone();
        let before_parcels = engine.state.parcels.clone();
        let before_id = engine.state.next_id;
        let before_cursor = engine.state.interior_access_cursor;
        assert!(!engine.try_plan_interior_access());
        assert_eq!(engine.state.nodes.len(), before_nodes.len());
        assert_eq!(engine.state.roads.len(), before_roads.len());
        assert_eq!(engine.state.projects.len(), before_projects.len());
        assert_eq!(engine.state.parcels.len(), before_parcels.len());
        assert_eq!(engine.state.next_id, before_id);
        assert_eq!(engine.state.interior_access_cursor, before_cursor);
        assert!(engine.state.pending_interior_access.is_none());
    }

    #[test]
    fn interior_access_proposal_scan_is_read_only_and_leaves_admission_unchanged() {
        let mut engine = Engine::restore_json(include_str!(
            "../../validation/mature-world.ranked-blocks.json"
        ))
        .expect("mature public city fixture restores");
        engine.state.demand = 1.2;
        let proposal_bound = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
            })
            .count()
            .saturating_mul(interior_access::PROPOSALS_PER_HOST);
        let scan_count = proposal_bound
            .div_ceil(interior_access::MAX_PROPOSALS_PER_EVALUATION)
            .saturating_add(1);
        let mut found_proposal = false;
        for _ in 0..scan_count {
            let before = engine.save_json();
            let before_rng = engine.state.rng_growth;
            let before_id = engine.state.next_id;
            let before_cursor = engine.state.interior_access_cursor;
            let scan = engine.propose_interior_access();
            assert_eq!(
                engine.save_json(),
                before,
                "candidate enumeration mutated serialized state"
            );
            assert_eq!(engine.state.rng_growth, before_rng);
            assert_eq!(engine.state.next_id, before_id);
            assert_eq!(engine.state.interior_access_cursor, before_cursor);
            if !scan.proposals.is_empty() {
                found_proposal = true;
                break;
            }
            engine.state.interior_access_cursor = scan.next_cursor;
        }
        assert!(
            found_proposal,
            "fixture should expose a valid bounded proposal"
        );

        let mut planner = Engine::restore_json(include_str!(
            "../../validation/mature-world.ranked-blocks.json"
        ))
        .expect("mature public city fixture restores");
        planner.state.demand = 1.2;
        let revision_before_admission = planner.state.access_geometry_revision;
        let mut admitted = false;
        for _ in 0..scan_count {
            if planner.try_plan_interior_access() {
                admitted = true;
                break;
            }
        }
        assert!(
            admitted,
            "production planner should admit an enumerated proposal"
        );
        assert!(planner.state.access_geometry_revision > revision_before_admission);
        assert!(planner.state.pending_interior_access.is_some());
        planner
            .validate()
            .expect("admitted proposal remains connected and restorable");
        let saved = planner.save_json();
        let mut restored = Engine::restore_json(&saved).expect("admission state restores");
        planner.step(2_000);
        restored.step(2_000);
        assert_eq!(planner.save_json(), restored.save_json());
    }

    #[test]
    fn frontier_proposal_scan_is_read_only_and_plan_road_replays() {
        let mut engine = Engine::new(0);
        // Keep this fixture about frontier proposal purity. Independent live
        // interior searches can consume the last available frontier before
        // the probe; their interaction/replay is covered in access_live_engine.
        engine.state.next_face_access_tick = u64::MAX;
        engine.set_development_rate(60);
        for _ in 0..36 {
            engine.step(2_000);
        }
        let open_buildings: Vec<Building> = engine
            .state
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .cloned()
            .collect();
        let bridgehead_headings = engine.eligible_bridgehead_headings(&open_buildings);
        let collision_context = engine.collision_context();
        let before_chord = engine.save_json();
        assert!(!engine.try_plan_local_chord(&open_buildings, &collision_context));
        let before = engine.save_json();
        let before_rng = engine.state.rng_growth;
        let before_cursor = engine.state.road_origin_cursor;
        let scan =
            engine.propose_frontier_road(&open_buildings, &bridgehead_headings, &collision_context);
        let repeated =
            engine.propose_frontier_road(&open_buildings, &bridgehead_headings, &collision_context);
        assert_eq!(
            engine.save_json(),
            before,
            "frontier scan mutated serialized state"
        );
        assert_eq!(engine.state.rng_growth, before_rng);
        assert_eq!(engine.state.road_origin_cursor, before_cursor);
        let proposal = scan
            .proposal
            .expect("fixture should contain a viable frontier proposal");
        let repeated_proposal = repeated.proposal.expect("scan result is repeatable");
        assert_eq!(proposal.start_id, repeated_proposal.start_id);
        assert_eq!(proposal.target_id, repeated_proposal.target_id);
        assert_eq!(proposal.end_x, repeated_proposal.end_x);
        assert_eq!(proposal.end_z, repeated_proposal.end_z);
        assert_eq!(proposal.bridgehead, repeated_proposal.bridgehead);
        assert_eq!(scan.next_cursor, repeated.next_cursor);
        assert_eq!(scan.next_rng, repeated.next_rng);
        assert_eq!(scan.allow_access_fallback, repeated.allow_access_fallback);

        let mut exhausted = Engine::restore_json(include_str!(
            "../../validation/mature-world.ranked-blocks.json"
        ))
        .expect("mature public city fixture restores");
        exhausted.state.demand = 1.2;
        let exhausted_buildings: Vec<Building> = exhausted
            .state
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .cloned()
            .collect();
        let exhausted_bridgeheads = exhausted.eligible_bridgehead_headings(&exhausted_buildings);
        let exhausted_context = exhausted.collision_context();
        assert!(!exhausted.try_plan_local_chord(&exhausted_buildings, &exhausted_context));
        let exhausted_before = exhausted.save_json();
        let exhausted_scan = exhausted.propose_frontier_road(
            &exhausted_buildings,
            &exhausted_bridgeheads,
            &exhausted_context,
        );
        assert!(exhausted_scan.proposal.is_none());
        assert!(exhausted_scan.allow_access_fallback);
        assert_eq!(exhausted.save_json(), exhausted_before);

        let mut left = Engine::restore_json(&before_chord).unwrap();
        let mut right = Engine::restore_json(&before_chord).unwrap();
        left.plan_road();
        right.plan_road();
        assert_eq!(
            left.save_json(),
            right.save_json(),
            "planner replay diverged"
        );
    }

    fn synthetic_district_checkpoint(
        count: usize,
        processed: usize,
    ) -> district_density::MeasurementCheckpoint {
        let samples: Vec<district_density::BuildingSample> = (0..count)
            .map(|index| district_density::BuildingSample {
                id: 10_000 + index as u32,
                footprint: geometry::OrientedRect {
                    x: -70.0 + (index % 8) as f32 * 18.0,
                    z: -48.0 + (index / 8) as f32 * 18.0,
                    width: 10.0,
                    depth: 12.0,
                    rotation: 0.0,
                },
                floors: 2,
                use_class: district_density::BuildingUse::Housing,
                open: true,
                resident_occupancy: 2,
                capacity: 5,
                business_active: false,
            })
            .collect();
        let mut accumulator = district_density::MeasurementAccumulator::begin(
            district_controller::DISTRICT_GRID,
            &samples,
            &[],
        )
        .unwrap();
        let mut remaining = processed;
        while remaining > 0 {
            let batch = remaining.min(district_density::MAX_MEASUREMENT_SAMPLES_PER_ADVANCE);
            accumulator.advance(batch).unwrap();
            remaining -= batch;
        }
        accumulator.checkpoint().unwrap()
    }

    fn engine_with_pending_measurement() -> Engine {
        let mut engine = Engine::new(981);
        engine.state.tick = 1;
        engine.state.district_controller.pending = Some(synthetic_district_checkpoint(40, 16));
        engine.state.district_controller.pending_capture_tick = 1;
        engine.district_runtime =
            district_controller::Runtime::restore(&engine.state.district_controller).unwrap();
        engine
    }

    #[test]
    fn district_measurement_is_frozen_bounded_and_save_resumable() {
        let mut original = engine_with_pending_measurement();
        let frozen = original.state.district_controller.pending.clone().unwrap();
        let mut expected_accumulator =
            district_density::MeasurementAccumulator::resume(frozen).unwrap();
        while !expected_accumulator.is_complete() {
            expected_accumulator.advance(16).unwrap();
        }
        let expected_metrics = expected_accumulator.finish().unwrap();

        // Live-world changes after capture do not restart or alter frozen work.
        original.state.demand = 1.17;
        let before_rng = original.state.rng_growth;
        original.step(1);
        assert_eq!(
            original
                .state
                .district_controller
                .pending
                .as_ref()
                .unwrap()
                .processed_samples,
            32
        );
        assert_eq!(original.state.rng_growth, before_rng);
        let saved = original.save_json();
        let mut restored = Engine::restore_json(&saved).unwrap();
        assert_eq!(restored.save_json(), saved);
        assert!(!restored.district_runtime.frontage_metrics_available());

        original.step(1);
        restored.step(1);
        assert_eq!(original.save_json(), restored.save_json());
        assert!(original.state.district_controller.pending.is_none());
        assert_eq!(original.state.district_controller.completed_capture_tick, 1);
        assert_eq!(original.state.district_controller.completed_tick, 3);
        assert_eq!(original.state.district_controller.next_refresh_tick, 203);
        assert_eq!(
            original.district_runtime.metrics.as_ref(),
            Some(&expected_metrics)
        );
        assert_eq!(
            restored.district_runtime.metrics,
            original.district_runtime.metrics
        );
    }

    #[test]
    fn district_measurement_completes_and_reconstructs_cached_metrics() {
        let mut engine = Engine::new(982);
        engine.step(1);
        let controller = &engine.state.district_controller;
        assert!(controller.pending.is_none());
        assert!(controller.completed.is_some());
        assert_eq!(controller.completed_capture_tick, 1);
        assert_eq!(controller.completed_tick, 1);
        assert_eq!(controller.next_refresh_tick, 201);
        let expected = engine.district_runtime.metrics.clone().unwrap();
        let saved = engine.save_json();
        let restored = Engine::restore_json(&saved).unwrap();
        assert_eq!(restored.save_json(), saved);
        assert_eq!(restored.district_runtime.metrics, Some(expected));
        assert!(restored.district_runtime.accumulator.is_none());
        assert!(!restored.district_runtime.frontage_metrics_available());
    }

    #[test]
    fn district_measurement_rejects_malformed_checkpoints_and_accepts_legacy_saves() {
        let mut invalid_cursor = engine_with_pending_measurement();
        let mut checkpoint = invalid_cursor
            .state
            .district_controller
            .pending
            .clone()
            .unwrap();
        checkpoint.processed_samples = checkpoint.buildings.len() + checkpoint.frontages.len() + 1;
        invalid_cursor.state.district_controller.pending = Some(checkpoint);
        assert!(Engine::restore_json(&invalid_cursor.save_json()).is_err());

        let mut duplicate = engine_with_pending_measurement();
        let mut checkpoint = duplicate.state.district_controller.pending.clone().unwrap();
        checkpoint.buildings[1].id = checkpoint.buildings[0].id;
        duplicate.state.district_controller.pending = Some(checkpoint);
        assert!(Engine::restore_json(&duplicate.save_json()).is_err());

        let mut unsupported_frontage = engine_with_pending_measurement();
        let mut checkpoint = unsupported_frontage
            .state
            .district_controller
            .pending
            .clone()
            .unwrap();
        checkpoint.frontages.push(district_density::FrontageSample {
            id: 1,
            start_x: 0.0,
            start_z: 0.0,
            end_x: 10.0,
            end_z: 0.0,
            occupied: false,
        });
        unsupported_frontage.state.district_controller.pending = Some(checkpoint);
        assert!(Engine::restore_json(&unsupported_frontage.save_json()).is_err());

        let mut future = engine_with_pending_measurement();
        future.state.district_controller.next_refresh_tick = u64::MAX;
        assert!(Engine::restore_json(&future.save_json()).is_err());

        let engine = Engine::new(983);
        let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("district_controller");
        let restored = Engine::restore_json(&legacy.to_string()).unwrap();
        assert!(restored.state.district_controller.is_empty());
        let canonical: serde_json::Value = serde_json::from_str(&restored.save_json()).unwrap();
        assert!(canonical.get("district_controller").is_none());
    }

    #[test]
    fn district_measurement_advances_at_most_sixteen_samples_per_fixed_tick() {
        let mut engine = Engine::new(984);
        engine.state.tick = 1;
        engine.state.district_controller.pending = Some(synthetic_district_checkpoint(80, 0));
        engine.state.district_controller.pending_capture_tick = 1;
        engine.district_runtime =
            district_controller::Runtime::restore(&engine.state.district_controller).unwrap();
        let before_growth_rng = engine.state.rng_growth;
        engine.step(1);
        let checkpoint = engine.state.district_controller.pending.as_ref().unwrap();
        assert_eq!(checkpoint.processed_samples, 16);
        assert_eq!(engine.state.rng_growth, before_growth_rng);
        assert_eq!(engine.state.tick, 2);
    }

    #[test]
    fn district_policy_forecast_geometry_is_clipped_and_bounded() {
        assert_eq!(district_policy::MAX_COMPARED_PROJECTS, 49);
        assert_eq!(district_policy::MAX_FORECAST_RECTS, 818);
        let checkpoint = synthetic_district_checkpoint(40, 40);
        let metrics = district_density::MeasurementAccumulator::resume(checkpoint)
            .unwrap()
            .finish()
            .unwrap();
        let engine = Engine::new(985);
        let rects: Vec<_> = (0..district_policy::MAX_FRONTIER_SITES)
            .map(|index| geometry::OrientedRect {
                x: -350.0 + (index % 10) as f32 * 24.0,
                z: -350.0 + (index / 10) as f32 * 24.0,
                width: 14.0,
                depth: 12.0,
                rotation: index as f32 * 0.03,
            })
            .collect();
        let catchment = project_scoring::LiveCatchment::default();
        let candidate = engine
            .make_district_score_candidate(
                7,
                district_density::CandidateKind::Expansion,
                80.0,
                0.5,
                catchment,
                true,
                &rects,
                &metrics,
            )
            .unwrap();
        assert!(candidate.cell_impacts.len() <= district_density::MAX_DISTRICT_CELLS);
        assert!(candidate.cell_impacts.len() <= 64);
        let clipped_forecast: f64 = candidate
            .cell_impacts
            .iter()
            .map(|impact| impact.forecast_envelope_gain_m2)
            .sum();
        let expected: f64 = rects
            .iter()
            .map(|rect| (rect.width * rect.depth) as f64 * district_policy::FORECAST_UTILIZATION)
            .sum();
        assert!(
            (clipped_forecast - expected).abs() < 0.001,
            "clipped {clipped_forecast} != expected {expected}"
        );
        assert!(candidate
            .cell_impacts
            .iter()
            .all(|impact| impact.forecast_housing_capacity_gain == 1.0
                && impact.forecast_commercial_capacity_gain == 1.0));
    }

    #[test]
    fn district_policy_candidate_ids_are_unique_across_maximum_scan_batch() {
        use std::collections::BTreeSet;

        let mut ids = BTreeSet::new();
        // A scan can return at most 48 access proposals. Exercise the full
        // 3 fractions × 2 sides × 4 lengths lattice across two hosts.
        for host_id in [17, 4_999] {
            for fraction_index in 0..interior_access::FRACTIONS.len() as u8 {
                for side_index in 0..interior_access::SIDES.len() as u8 {
                    for length_index in 0..interior_access::LENGTHS.len() as u8 {
                        let proposal = interior_access::AccessProposal {
                            score: interior_access::CandidateScore {
                                site_area: 256,
                                road_length_mm: 24_000,
                                host_id,
                                fraction_index,
                                side_index,
                                length_index,
                            },
                            host_id,
                            host_t: interior_access::FRACTIONS[fraction_index as usize],
                            end_x: 0.0,
                            end_z: 24.0,
                            selected_lots: Vec::new(),
                        };
                        assert!(ids.insert(district_access_candidate_id(&proposal)));
                    }
                }
            }
        }
        assert_eq!(ids.len(), interior_access::MAX_PROPOSALS_PER_EVALUATION);
        assert!(ids.insert(district_frontier_candidate_id(1, Some(2))));
        assert!(ids.insert(district_frontier_candidate_id(4_999, None)));
    }

    #[test]
    fn district_policy_obeys_existing_road_demand_boundary() {
        assert!(!road_demand_is_sufficient(0.0));
        assert!(!road_demand_is_sufficient(0.8));
        assert!(road_demand_is_sufficient(f32::from_bits(
            0.8_f32.to_bits() + 1
        )));
        assert!(road_demand_is_sufficient(1.0));
    }

    #[test]
    #[ignore = "matched-seed district policy experiment; run explicitly for calibration evidence"]
    fn district_policy_experiment_uses_normal_step_for_seed_87231_and_seed_zero() {
        for seed in [87_231, 0] {
            for enabled in [false, true] {
                let started = std::time::Instant::now();
                let mut engine = Engine::new(seed);
                engine.set_development_rate(16);
                engine.district_policy_experiment = enabled;
                for _ in 0..121 {
                    engine.step(2_000);
                }
                assert_eq!(engine.state.tick, 242_000);
                engine.validate().unwrap();
                let open_roads: Vec<_> = engine
                    .state
                    .roads
                    .iter()
                    .filter(|road| road.stage == "open" && road.connected)
                    .collect();
                let envelope_area: f64 = engine
                    .state
                    .buildings
                    .iter()
                    .filter(|building| building.stage == "open")
                    .map(|building| (building.width * building.depth) as f64)
                    .sum();
                let floor_area: f64 = engine
                    .state
                    .buildings
                    .iter()
                    .filter(|building| building.stage == "open")
                    .map(|building| {
                        (building.width * building.depth * building.floors as f32) as f64
                    })
                    .sum();
                let road_length: f64 = open_roads
                    .iter()
                    .filter(|road| !matches!(road.kind.as_str(), "rail" | "bridge"))
                    .map(|road| road.length as f64)
                    .sum();
                eprintln!(
                    "district-policy seed={seed} enabled={enabled} buildings={} connected_open_roads={} road_length={road_length:.1}m envelope_proxy={envelope_area:.1} floor_proxy={floor_area:.1} comparisons={} access_wins={} frontier_wins={} elapsed={:.2}s",
                    engine.state.buildings.iter().filter(|b| b.stage == "open").count(),
                    open_roads.len(),
                    engine.district_policy_comparisons,
                    engine.district_policy_access_wins,
                    engine.district_policy_frontier_wins,
                    started.elapsed().as_secs_f64(),
                );
                if enabled {
                    assert!(engine.district_policy_comparisons > 0);
                    assert!(
                        engine.district_policy_access_wins + engine.district_policy_frontier_wins
                            > 0
                    );
                    assert_eq!(
                        engine.district_policy_diagnostics.comparisons,
                        engine.district_policy_comparisons
                    );
                    let diag = &engine.district_policy_diagnostics;
                    eprintln!(
                        "district-policy diagnostics seed={seed} comparisons={} frontier(proposals={},geometry={},source_ok={},source_reject={},sites={},area_m2={:.1},ranked={},top_mean={:.4},top_max={:.4},wins={}) access(proposals={},geometry={},source_ok={},source_reject={},sites={},area_m2={:.1},ranked={},top_mean={:.4},top_max={:.4},wins={}) paired(ranked={},frontier_score_wins={},access_score_wins={},ties={},mean_frontier_minus_access={:.5},frontier_admissions={},access_admissions={}) no_admission(no_geometry={},no_source_feasible={},no_cell_forecast={},no_metrics={},score_errors={},ranked_rejected={}) representative_comparisons={} samples={}",
                        diag.comparisons,
                        diag.frontier_proposals,
                        diag.frontier_geometry_candidates,
                        diag.frontier_source_feasible_candidates,
                        diag.frontier_source_vacancy_rejections,
                        diag.frontier_sites,
                        diag.frontier_nominal_area_m2,
                        diag.frontier_ranked,
                        diag.frontier_top_score_sum / diag.frontier_top_score_count.max(1) as f64,
                        diag.frontier_top_score_max,
                        diag.frontier_wins,
                        diag.access_proposals,
                        diag.access_geometry_candidates,
                        diag.access_source_feasible_candidates,
                        diag.access_source_vacancy_rejections,
                        diag.access_sites,
                        diag.access_nominal_area_m2,
                        diag.access_ranked,
                        diag.access_top_score_sum / diag.access_top_score_count.max(1) as f64,
                        diag.access_top_score_max,
                        diag.access_wins,
                        diag.paired_ranked_comparisons,
                        diag.paired_frontier_score_wins,
                        diag.paired_access_score_wins,
                        diag.paired_score_ties,
                        diag.paired_score_difference_sum / diag.paired_ranked_comparisons.max(1) as f64,
                        diag.paired_frontier_admissions,
                        diag.paired_access_admissions,
                        diag.no_geometry,
                        diag.no_source_feasible,
                        diag.no_cell_forecast,
                        diag.no_metrics,
                        diag.score_errors,
                        diag.ranked_admission_rejections,
                        diag.representative_comparisons,
                        diag.records.len(),
                    );
                    for sample in &diag.records {
                        eprintln!(
                            "policy sample seed={seed} comparison={} tick={} frontier_sites={} frontier_area={:.0} frontier_vacancy_rejected={} frontier_score={:?} frontier_ranked={} access_proposals={} access_sites={} access_area={:.0} access_vacancy_rejections={} access_score={:?} access_ranked={} winner={} no_admission={}",
                            sample.comparison,
                            sample.tick,
                            sample.frontier_sites,
                            sample.frontier_area_m2,
                            sample.frontier_source_rejected,
                            sample.frontier_top_score,
                            sample.frontier_ranked,
                            sample.access_proposals,
                            sample.access_sites,
                            sample.access_area_m2,
                            sample.access_vacancy_rejections,
                            sample.access_top_score,
                            sample.access_ranked,
                            sample.winner.unwrap_or("none"),
                            sample.no_admission_reason.unwrap_or("none"),
                        );
                    }
                } else {
                    assert_eq!(engine.district_policy_comparisons, 0);
                }

                let saved = engine.save_json();
                let validation_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .join("validation");
                std::fs::write(
                    validation_dir.join(format!(
                        "district-policy-experiment-seed-{seed}-enabled-{enabled}.json"
                    )),
                    &saved,
                )
                .unwrap();
                let mut restored = Engine::restore_json(&saved).unwrap();
                restored.district_policy_experiment = enabled;
                engine.step(2_000);
                restored.step(2_000);
                assert_eq!(engine.save_json(), restored.save_json());
            }
        }
    }

    #[test]
    fn access_geometry_revision_tracks_effective_writers_and_ignores_ticks() {
        let mut engine = Engine::new(31);
        let initial = engine.state.access_geometry_revision;

        // Snapshot/weather activity is outside the frozen access geometry.
        engine.state.tick = engine.state.tick.saturating_add(40);
        engine.state.weather_intensity = 0.4;
        engine.state.revision = engine.state.revision.saturating_add(1);
        engine.recompute_connectivity();
        assert_eq!(engine.state.access_geometry_revision, initial);

        let node = engine.add_node(260.0, 260.0, "junction");
        let after_node = engine.state.access_geometry_revision;
        assert!(after_node > initial);
        let road = engine.add_road(1, node, "local", "open", 1.0);
        let after_road = engine.state.access_geometry_revision;
        assert!(after_road > after_node);
        engine.recompute_connectivity();
        assert!(engine.state.access_geometry_revision > after_road);

        let parcel = engine.add_parcel(255.0, 250.0, 8.0, 8.0, node, 0.7, false);
        let after_parcel = engine.state.access_geometry_revision;
        assert!(after_parcel > after_road);
        let before_frontage = engine.state.access_geometry_revision;
        engine.remember_nearest_frontage(parcel, node);
        assert!(engine.state.access_geometry_revision > before_frontage);
        assert!(engine
            .state
            .parcel_frontages
            .iter()
            .any(|frontage| frontage.parcel_id == parcel));

        // The normal building planner reserves a parcel. That reservation
        // changes which sites a concurrent access proposal may consume.
        let before_building_reservation = engine.state.access_geometry_revision;
        engine.plan_building(1);
        assert!(engine.state.access_geometry_revision > before_building_reservation);

        let building_project = engine
            .state
            .projects
            .iter()
            .rev()
            .find(|project| project.kind == "building" && project.progress < 1.0)
            .unwrap()
            .clone();
        let before_reservation_release = engine.state.access_geometry_revision;
        engine.complete_project(building_project.id);
        assert!(engine.state.access_geometry_revision > before_reservation_release);
    }

    #[test]
    fn access_geometry_revision_tracks_road_opening_and_plan_reservations() {
        let mut engine = Engine::new(33);
        let bridge_project = engine
            .state
            .projects
            .iter()
            .find(|project| project.kind == "road")
            .unwrap()
            .id;
        let before_open = engine.state.access_geometry_revision;
        engine.complete_project(bridge_project);
        assert!(engine.state.access_geometry_revision > before_open);

        let mut planner = engine_with_developed_plan_site_without_reusable_edge();
        let before_plan = planner.state.access_geometry_revision;
        assert!(planner.try_create_neighborhood_plan());
        assert!(planner.state.access_geometry_revision > before_plan);
        planner.validate().unwrap();
    }

    #[test]
    fn access_geometry_revision_is_backward_compatible_and_fails_closed_at_overflow() {
        let engine = Engine::new(32);
        let mut old_save: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        let state = old_save.as_object_mut().unwrap();
        state.remove("access_geometry_revision");
        state.remove("access_geometry_revision_exhausted");
        let restored = Engine::restore_json(&old_save.to_string()).unwrap();
        assert_eq!(restored.state.access_geometry_revision, 0);
        assert!(!restored.state.access_geometry_revision_exhausted);
        assert_eq!(restored.access_geometry_revision_token(), Some(0));

        let mut exhausted = restored;
        exhausted.state.access_geometry_revision = u64::MAX;
        assert_eq!(exhausted.access_geometry_revision_token(), Some(u64::MAX));
        exhausted.add_node(0.0, 0.0, "junction");
        assert!(exhausted.state.access_geometry_revision_exhausted);
        assert_eq!(exhausted.state.access_geometry_revision, u64::MAX);
        assert_eq!(exhausted.access_geometry_revision_token(), None);
        exhausted.add_node(1.0, 1.0, "junction");
        assert_eq!(exhausted.access_geometry_revision_token(), None);
        assert!(exhausted.validate().is_ok());
    }
    #[test]
    fn indexed_connectivity_matches_source_order_scan() {
        for seed in [1, 23, 87231] {
            let mut engine = Engine::new(seed);
            let gateway = engine.state.nodes.iter().find(|n| n.kind == "gateway").unwrap().id;
            let a = engine.add_node(120.0, 120.0, "junction");
            let b = engine.add_node(160.0, 120.0, "junction");
            let c = engine.add_node(200.0, 120.0, "junction");
            // Include disconnected components, cycles and parallel edges;
            // the scan oracle below is the previous connectivity algorithm.
            engine.add_road(a, b, "local", "open", 1.0);
            engine.add_road(b, c, "local", "open", 1.0);
            engine.add_road(c, a, "local", "open", 1.0);
            engine.add_road(a, b, "local", "open", 1.0);
            for connected in [false, true] {
                if connected { engine.add_road(gateway, a, "local", "open", 1.0); }
                let mut seen = BTreeSet::from([gateway]);
                let mut queue = VecDeque::from([gateway]);
                while let Some(node) = queue.pop_front() {
                    for road in &engine.state.roads {
                        if road.stage == "open" && (road.from == node || road.to == node) {
                            let next = if road.from == node { road.to } else { road.from };
                            if seen.insert(next) { queue.push_back(next); }
                        }
                    }
                }
                let expected = engine.state.roads.iter().map(|r| r.stage == "open" && seen.contains(&r.from) && seen.contains(&r.to)).collect::<Vec<_>>();
                engine.recompute_connectivity();
                assert_eq!(engine.state.roads.iter().map(|r|r.connected).collect::<Vec<_>>(),expected);
                let revision = engine.access_geometry_revision_token();
                engine.recompute_connectivity();
                assert_eq!(engine.access_geometry_revision_token(),revision);
            }
        }
    }

}
