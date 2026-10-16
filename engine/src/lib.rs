use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod neighborhood_demand;
mod neighborhood_errands;

#[cfg(all(test, feature = "parent-passage-candidate"))]
mod parent_passage_tests;

#[cfg(test)]
mod presentation_revision_tests;

#[cfg(test)]
mod access_bend_diagnostic;
mod access_candidates;
mod access_capture;
mod access_connector;
#[cfg(test)]
mod access_density_diagnostic;
#[cfg(test)]
mod access_descriptor_integration_tests;
mod access_descriptor_search;
mod access_discovery_job;
#[cfg(target_arch = "wasm32")]
mod access_discovery_wasm;
#[cfg(test)]
mod access_discovery_work_diagnostic;
mod access_live;
mod access_live_engine;
mod access_math;
mod access_path_validation;
#[cfg(test)]
mod access_path_validation_tests;
mod access_planning_job;
mod access_proposal_capture;
mod access_proposal_validation;
mod access_rays;
#[cfg(test)]
mod access_reference;
mod access_search;
#[cfg(test)]
mod block_subdivision_diagnostic;
mod blocks;
#[cfg(test)]
mod compact_frontage_diagnostic;
#[cfg(test)]
mod density_gap_diagnosis;
mod development;
mod district_controller;
mod district_density;
#[cfg(test)]
mod district_density_integration_tests;
mod district_policy;
mod expressway_profile;
mod expressway_traffic;
#[cfg(test)]
mod face_access_exact_tests;
mod face_target_containment;
mod face_target_ranking;
mod face_target_sampling;
mod face_target_selection;
mod face_target_viability;
mod frontage;
mod frontier;
mod geometry;
mod height_redevelopment;
#[cfg(test)]
mod height_redevelopment_tests;
#[cfg(test)]
mod initial_depth_diagnostic;
mod interior_access;
mod masterplan;
mod masterplan_growth;
#[cfg(test)]
mod masterplan_replay_tests;
mod masterplan_validation;
#[cfg(test)]
mod mature_planner_trace_tests;
mod movement_support;
mod neighborhood_admission;
mod neighborhood_spine;
mod network_split;
mod node_coincidence_index;
#[cfg(test)]
mod partial_block_diagnostic;
mod project_scoring;
mod rail_alignment;
mod rail_platform;
mod rail_runtime;
#[cfg(test)]
mod rear_extension_diagnostic;
mod rear_extensions;
mod river_course;
mod road_curve;
mod road_face_bridges;
mod road_face_preflight;
mod road_face_topology;
mod road_face_trace;
mod road_face_walk;
mod road_faces;
mod road_pair_broadphase;
mod road_split;
#[cfg(test)]
mod road_split_profile;
#[cfg(test)]
mod route_equivalence_tests;
#[cfg(test)]
mod skyline_allocation_diagnostic;
mod spine_blocks;
mod spine_plan;
#[cfg(test)]
mod spine_ranking_experiment_tests;
#[cfg(test)]
mod spine_replay_tests;
#[cfg(test)]
mod transit_access_tests;
mod transit_recovery;

const SCHEMA_VERSION: u32 = 1;
const ACTIVITY_SCHEMA_VERSION: u8 = 5;
const ENGINE_VERSION: &str = "0.1.0";
const MIN: f32 = -400.0;
const MAX: f32 = 400.0;
const TICKS_PER_SECOND: u64 = 20;
const TICKS_PER_DAY: u64 = 24_000;
const DEVELOPMENT_TICKS_PER_DAY: u64 = 800;
const MAX_BUILDINGS: usize = 3_000;
const MAX_SEGMENTS: usize = 5_000;
const MAX_PARCELS: usize = 4_000;
/// Occupied homes within this distance form a pocket park's catchment.
const POCKET_PARK_CATCHMENT: f32 = 70.0;
/// Occupied homes a catchment needs before a vacant lot becomes a park.
const POCKET_PARK_MIN_HOMES: usize = 8;
/// Minimum distance between pocket parks, so each serves its own streets.
const POCKET_PARK_SPACING: f32 = 120.0;
/// A quarter day between pocket park reviews.
const POCKET_PARK_INTERVAL_TICKS: u64 = TICKS_PER_DAY / 4;
/// City-wide ceiling: at most one pocket park per this many occupied homes.
const HOMES_PER_POCKET_PARK: usize = 30;
/// Ordinary neighborhood lots only; station towers and tiny slivers are skipped.
const POCKET_PARK_AREA: std::ops::RangeInclusive<f32> = 80.0..=520.0;
const MAX_FRONTAGE_ROADS_PER_EVALUATION: usize = 24;
const MAX_FRONTAGE_SITES_PER_EVALUATION: usize = 48;
const MAX_FRONTAGE_GRID_SITES_PER_ROAD: usize = 34;
const MAX_FRONTAGE_GAP_LOTS_PER_SIDE: usize = 8;
const FRONTAGE_GAP_DEPTH: f32 = 12.0;
const FRONTAGE_GAP_OFFSET: f32 = 15.0;
const FRONTAGE_SHALLOW_DEPTH: f32 = 8.0;
const FRONTAGE_SHALLOW_OFFSET: f32 = 12.0;
const MAX_FRONTAGE_SITES_PER_ROAD: usize =
    2 * MAX_FRONTAGE_GRID_SITES_PER_ROAD + 4 * MAX_FRONTAGE_GAP_LOTS_PER_SIDE;
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
// Match the renderer presentation bound; ordinary service currently uses one trainset.
const MAX_PRESENTED_TRAINS: usize = 8;
const RAIL_QUEUE_CAPACITY: usize = 96;
const BUS_CAPACITY: usize = 24;
const BUS_QUEUE_CAPACITY: usize = 48;
const MAX_BUSES: usize = 2;
const MAX_BUS_STOPS: usize = 5;
const MAX_BUS_ROUTE_NODES: usize = 128;
const BUS_HEADWAY_TICKS: u64 = 240;
const BUS_DWELL_TICKS: u16 = 80;
const BUS_RETRY_TICKS: u64 = 2_000;
// A four-car 20 m consist (cab, trailer, motor, cab) with 0.6 m gangways spans
// 81.8 m, so the consist centre sits 40.9 m from each terminal. Keep this equal
// to consistHalfLength() in web/src/render/train-kit.ts and the station platform
// length in web/src/render/city.ts.
const TRAIN_HALF_LENGTH: f32 = 40.9;
const TRAIN_STATION_DWELL_TICKS: u16 = 220;
const TRAIN_GATEWAY_DWELL_TICKS: u16 = 180;
const TRAIN_DOOR_PHASE_TICKS: u16 = 20;
const RAIL_PLATFORM_END: f32 = TRAIN_HALF_LENGTH * 2.0 + 1.0;
const RAIL_GATEWAY_STAIR_BOTTOM: f32 = RAIL_PLATFORM_END + 19.0;

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
    /// Open space a protected, unbuilt parcel holds: "park", "shrine" or
    /// "plaza". Absent for ordinary lots and for parks from older saves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_space: Option<String>,
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
    pub points: Vec<Point>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RearExtensionView {
    pub project_id: u32,
    pub building_id: u32,
    /// Center of the added rear strip, excluding the original building.
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub rotation: f32,
    pub floors: u8,
    pub stage: String,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildingOriginalFootprint {
    pub building_id: u32,
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub rotation: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadDeckView {
    pub road_id: u32,
    pub deck_height: f32,
    pub controls: [[f32; 2]; 2],
}

pub use rail_platform::RailPlatformView;
pub use rail_runtime::RailAlignmentView;

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
    /// Exact presentation generation; only meaningful within the worker epoch.
    pub presentation_revision: String,
    pub stage: String,
    pub weather: Weather,
    pub bounds: Bounds,
    pub terrain: Terrain,
    pub nodes: Vec<Node>,
    pub roads: Vec<Road>,
    #[serde(default)]
    pub road_decks: Vec<RoadDeckView>,
    pub expressway_profile: expressway_profile::ExpresswayProfile,
    #[serde(default)]
    pub rail_platforms: Vec<RailPlatformView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rail_alignments: Vec<RailAlignmentView>,
    pub parcels: Vec<Parcel>,
    pub buildings: Vec<Building>,
    #[serde(default)]
    pub building_access: Vec<BuildingAccessView>,
    #[serde(default)]
    pub rear_extensions: Vec<RearExtensionView>,
    #[serde(default)]
    pub building_original_footprints: Vec<BuildingOriginalFootprint>,
    pub projects: Vec<Project>,
    /// Paved ground of developed master-plan blocks (outline in x/z).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ground: Vec<GroundPatch>,
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
pub struct GroundPatch {
    pub block: u32,
    pub points: Vec<[f32; 2]>,
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
    pub door_open_fraction: f32,
    pub door_terminal: Option<u32>,
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

/// Signed offset of the driving lane from the road centre line, measured along
/// (-dz, dx), the counter-clockwise normal of the heading in the x-z plane.
/// The renderer is right-handed with y up, where that normal is the driver's
/// RIGHT, so Tokyo's left-hand traffic needs the negative side.
const KERB_LANE: f32 = -1.65;
/// Bus-stop poles stand on the kerb side, along the same normal.
const BUS_STOP_KERB: f32 = -6.2;

// Movement-buffer kinds for road vehicle bodies (web/src/render/vehicles.ts).
// Kind 1 stays the private compact so older traces and tools keep working.
const VEHICLE_COMPACT: u8 = 1;
const VEHICLE_KEI: u8 = 4;
const VEHICLE_TAXI: u8 = 5;
const VEHICLE_VAN: u8 = 6;
const VEHICLE_TRUCK: u8 = 7;
const VEHICLE_MAIL: u8 = 8;
/// Bumper-to-bumper allowance between queued road vehicles.
const VEHICLE_QUEUE_MARGIN: f32 = 0.5;
/// The historical fixed queue spacing; short bodies never queue tighter.
const VEHICLE_MIN_GAP: f32 = 4.5;

fn vehicle_hash(id: u32) -> u32 {
    id.wrapping_mul(2_654_435_761) >> 16
}

/// Body length in metres; matches web/src/render/vehicle-manifest.json.
fn vehicle_length(body: u8) -> f32 {
    match body {
        VEHICLE_KEI | VEHICLE_MAIL => 3.4,
        VEHICLE_TAXI => 4.4,
        VEHICLE_VAN | VEHICLE_TRUCK => 4.7,
        _ => 4.0,
    }
}

fn vehicle_name(body: u8) -> &'static str {
    match body {
        VEHICLE_KEI => "kei",
        VEHICLE_TAXI => "taxi",
        VEHICLE_VAN => "van",
        VEHICLE_TRUCK => "truck",
        VEHICLE_MAIL => "mail",
        _ => "compact",
    }
}

/// Where an access path meets the road, on a 10 cm grid, as a queue key.
fn driveway_mouth(point: WorldPoint) -> (i32, i32) {
    (
        (point.x * 10.0).round() as i32,
        (point.z * 10.0).round() as i32,
    )
}

/// Park a car that reached the end of its route at the kerb end of its final
/// lane while its driveway mouth is busy. It stays a road traveler there, so
/// it is still drawn and still holds back the cars queued behind it.
fn hold_at_kerb(t: &mut Traveler) {
    if let Some(&last) = t.route_lengths.last() {
        t.edge_index = t.route_lengths.len() - 1;
        t.edge_progress = last.max(1.0);
    }
}

/// Centre-to-centre following distance for two queued road vehicles.
fn vehicle_gap(lead: u8, follower: u8) -> f32 {
    ((vehicle_length(lead) + vehicle_length(follower)) * 0.5 + VEHICLE_QUEUE_MARGIN)
        .max(VEHICLE_MIN_GAP)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct WorldPoint {
    x: f32,
    z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccessPath {
    points: Vec<WorldPoint>,
    /// Per-point absolute walking height. Empty paths use the ground/road policy.
    #[serde(default)]
    surface_y: Vec<f32>,
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

    fn sample_surface(&self, progress: f32) -> Option<(WorldPoint, f32, f32)> {
        let (point, heading) = self.sample(progress)?;
        if self.surface_y.len() != self.points.len() {
            return Some((point, heading, -1.0));
        }
        let mut remaining = progress.max(0.0);
        for index in 0..self.points.len().saturating_sub(1) {
            let a = self.points[index];
            let b = self.points[index + 1];
            let length = (b.x - a.x).hypot(b.z - a.z);
            if remaining <= length || index + 2 == self.points.len() {
                let t = if length <= f32::EPSILON {
                    1.0
                } else {
                    (remaining / length).clamp(0.0, 1.0)
                };
                let from = self.surface_y[index];
                let to = self.surface_y[index + 1];
                let y = if from < 0.0 && t < 1.0 {
                    -1.0
                } else if to < 0.0 && t >= 1.0 {
                    -1.0
                } else if from < 0.0 {
                    to
                } else if to < 0.0 {
                    from
                } else {
                    from + (to - from) * t
                };
                return Some((point, heading, y));
            }
            remaining -= length;
        }
        Some((point, heading, *self.surface_y.last()?))
    }

    fn reversed(&self) -> Self {
        Self {
            points: self.points.iter().rev().copied().collect(),
            surface_y: self.surface_y.iter().rev().copied().collect(),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_alight: Option<transit_recovery::PendingAlight>,
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

fn train_door_state(train: &Train, rail_length: f32) -> (Option<u32>, f32) {
    let at_station = train.distance <= TRAIN_HALF_LENGTH + 0.01;
    let at_gateway = train.distance >= rail_length - TRAIN_HALF_LENGTH - 0.01;
    let terminal = if train.dwell_ticks > 0 && at_station {
        train.route.first().copied()
    } else if train.dwell_ticks > 0 && at_gateway {
        train.route.last().copied()
    } else {
        None
    };
    let total = if at_station {
        TRAIN_STATION_DWELL_TICKS
    } else {
        TRAIN_GATEWAY_DWELL_TICKS
    };
    let fraction = if train.dwell_ticks > 0 && (at_station || at_gateway) {
        ((total.saturating_sub(train.dwell_ticks) as f32 / TRAIN_DOOR_PHASE_TICKS as f32)
            .clamp(0.0, 1.0))
        .min((train.dwell_ticks as f32 / TRAIN_DOOR_PHASE_TICKS as f32).clamp(0.0, 1.0))
    } else {
        0.0
    };
    (terminal, fraction)
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
    neighborhood_errands: neighborhood_errands::ErrandState,
    #[serde(default)]
    expressway_traffic: expressway_traffic::TrafficState,
    #[serde(default)]
    residents: Vec<Resident>,
    trains: Vec<Train>,
    #[serde(default)]
    bus_line: Option<BusLine>,
    #[serde(default)]
    buses: Vec<Bus>,
    #[serde(default)]
    next_bus_evaluation_tick: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pending_alight_ids: Vec<u32>,
    #[serde(default)]
    pending_alight_cursor: usize,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pending_rear_extensions: Vec<rear_extensions::RearExtensionPlan>,
    #[serde(default)]
    rear_extension_cursor: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    building_original_footprints: Vec<BuildingOriginalFootprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height_redevelopment: Option<height_redevelopment::HeightRedevelopment>,
    #[serde(default)]
    height_redevelopment_cursor: usize,
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
    /// Master-plan realization progress. Absent for legacy-planner worlds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    masterplan: Option<masterplan_growth::MasterplanProgress>,
}

/// Authoritative deterministic simulation. Public methods form the coarse WASM boundary.
pub struct Engine {
    state: WorldState,
    /// Regenerated master plan for `state.masterplan`; never serialized.
    plan: Option<std::sync::Arc<masterplan_growth::PlanRuntime>>,
    /// Bounded station-core renewal index derived from the regenerated plan.
    height_redevelopment_lots: Vec<usize>,
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
    /// Test-only admission treatment allowing a canonical clear-lot subset on
    /// otherwise ordinary full-block candidates.
    #[cfg(test)]
    partial_full_block_experiment: bool,
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
/// A protected lot kept as a pocket park. Untagged protected lots from older
/// saves are parks too; shrines and the station plaza are not.
fn is_pocket_park(parcel: &Parcel) -> bool {
    parcel.protected && !matches!(parcel.open_space.as_deref(), Some("shrine" | "plaza"))
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
/// Rendered carriageway width. Tokyo local streets are narrow shared surfaces;
/// only arterials carry raised sidewalks.
fn road_width(kind: &str) -> f32 {
    match kind {
        "arterial" => 16.0,
        "bridge" => 16.0,
        "local" => 6.5,
        "lane" => 4.0,
        _ => 9.0,
    }
}
/// Shared streets carry walkers inside the painted edge; arterials use the
/// raised sidewalk outside the carriageway. All access and movement consumers
/// must use this same offset so their route endpoints meet.
fn pedestrian_road_offset(kind: &str) -> f32 {
    let half_width = road_width(kind) * 0.5;
    if matches!(kind, "local" | "lane") {
        half_width - 0.72
    } else {
        half_width + 0.72
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

    /// Roads/nodes change through the access generation. Platform construction
    /// and service readiness are independent, so include their discrete phases.
    /// Keep u64 generations as text across the JS boundary. On exhaustion the
    /// tick forces a fresh structural snapshot for each movement publication.
    pub fn presentation_revision(&self) -> String {
        let platform_phase = self.station_construction_phase();
        let generation = match self.access_geometry_revision_token() {
            Some(value) => format!("g{value}"),
            None => format!("t{}", self.state.tick),
        };
        format!(
            "{generation}:{}:{}:{}",
            platform_phase, self.state.station_ready as u8, self.state.station_online as u8
        )
    }

    /// New world grown from the seeded master plan.
    pub fn new(seed: u32) -> Self {
        Self::construct(seed, true, None)
    }

    /// New world using the retired incremental frontier/infill planner. Kept
    /// only for the legacy test fixtures until they are removed.
    pub fn new_legacy(seed: u32) -> Self {
        Self::construct(seed, false, None)
    }

    fn construct(
        seed: u32,
        masterplan: bool,
        initial_plan: Option<masterplan::MasterPlan>,
    ) -> Self {
        let seed64 = (seed as u64).wrapping_add(0x9e3779b97f4a7c15).max(1);
        let mut engine = Self {
            plan: None,
            height_redevelopment_lots: Vec::new(),
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
                neighborhood_errands: neighborhood_errands::ErrandState::default(),
                expressway_traffic: expressway_traffic::TrafficState::default(),
                residents: vec![],
                trains: vec![],
                bus_line: None,
                buses: vec![],
                next_bus_evaluation_tick: 0,
                pending_alight_ids: Vec::new(),
                pending_alight_cursor: 0,
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
                pending_rear_extensions: Vec::new(),
                rear_extension_cursor: 0,
                building_original_footprints: Vec::new(),
                height_redevelopment: None,
                height_redevelopment_cursor: 0,
                interior_access_cursor: 0,
                next_face_access_tick: 0,
                exhausted_access_faces: Vec::new(),
                live_access_checkpoint: None,
                masterplan: None,
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
            partial_full_block_experiment: false,
            #[cfg(test)]
            road_opening_access_attempts: 0,
            #[cfg(test)]
            road_opening_access_admissions: 0,
            #[cfg(test)]
            road_opening_access_branch: None,
        };
        if masterplan {
            engine.initialize_masterplan_world(initial_plan);
        } else {
            engine.initialize_world();
        }
        engine.refresh_height_redevelopment_lots();
        engine
    }

    fn id(&mut self) -> u32 {
        let id = self.state.next_id;
        self.state.next_id = self.state.next_id.saturating_add(1);
        id
    }
    /// The body a road trip uses, derived from the trip so saves need no new field.
    /// Deliveries run vans, trucks and mail vans; residents drive kei cars and
    /// compacts, and station trips are more often taxis.
    /// `station` is `self.station_node()`, looked up once per pass by the caller.
    fn vehicle_body(t: &Traveler, station: Option<u32>) -> u8 {
        if t.kind != 1 {
            return t.kind;
        }
        let h = vehicle_hash(t.id);
        if t.activity.starts_with("delivery") {
            return match h % 6 {
                0 | 1 => VEHICLE_TRUCK,
                2 => VEHICLE_MAIL,
                _ => VEHICLE_VAN,
            };
        }
        let station_trip =
            station.is_some_and(|s| t.destination == s || t.route.first() == Some(&s));
        if h % 10 == 0 || (station_trip && h % 3 == 0) {
            VEHICLE_TAXI
        } else if h % 10 < 5 {
            VEHICLE_KEI
        } else {
            VEHICLE_COMPACT
        }
    }

    fn station_node(&self) -> Option<u32> {
        self.state
            .nodes
            .iter()
            .find(|n| n.kind == "station")
            .map(|n| n.id)
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
            open_space: None,
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
        if cfg!(feature = "parent-passage-candidate") {
            if let Some(rotation) = self
                .masterplan_lot_for_parcel(parcel_id)
                .and_then(|lot| lot.passage_rotation())
            {
                return rotation;
            }
        }
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
        let walking_offset = self
            .state
            .roads
            .iter()
            .find(|road| {
                road.stage == "open"
                    && road.kind != "rail"
                    && ((road.from == edge.0 && road.to == edge.1)
                        || (road.from == edge.1 && road.to == edge.0))
            })
            .map(|road| pedestrian_road_offset(&road.kind))
            .unwrap_or_else(|| pedestrian_road_offset("unknown"));
        let lateral = if kind == 1 {
            KERB_LANE
        } else if traveler_id % 2 == 0 {
            walking_offset
        } else {
            -walking_offset
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
        let sidewalk = pedestrian_road_offset(&road.kind);
        let frontage_side = frontage.side as f32;
        let frontage_walk = *segment.last()?;
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
            x: if access_node == road.from { a.x } else { b.x } + travel_nx * KERB_LANE,
            z: if access_node == road.from { a.z } else { b.z } + travel_nz * KERB_LANE,
        };
        let frontage_lane = WorldPoint {
            x: center.x + travel_nx * KERB_LANE,
            z: center.z + travel_nz * KERB_LANE,
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
            access_connector::pedestrian_points(&segment, end_walk, route_position, true)
        } else {
            access_connector::pedestrian_points(&segment, start_walk, route_position, false)
        };
        points.dedup_by(|right, left| (right.x - left.x).hypot(right.z - left.z) < 0.01);
        (points.len() >= 2).then_some(AccessPath {
            points,
            surface_y: Vec::new(),
        })
    }

    fn building_access_segment(&self, building_id: u32) -> Option<(u32, Vec<WorldPoint>)> {
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
        let sidewalk = pedestrian_road_offset(&road.kind);
        let side = frontage.side as f32;
        let frontage_walk = WorldPoint {
            x: center.x + nx * side * sidewalk,
            z: center.z + nz * side * sidewalk,
        };
        let door = WorldPoint {
            x: building.x + building.rotation.sin() * (building.depth * 0.5 + 0.2),
            z: building.z + building.rotation.cos() * (building.depth * 0.5 + 0.2),
        };
        let mut points = vec![door];
        if cfg!(feature = "flag-passage-candidate") || cfg!(feature = "parent-passage-candidate") {
            if let Some(passage) = self
                .masterplan_lot_for_parcel(parcel.id)
                .and_then(|lot| lot.passage)
            {
                points.extend(passage.into_iter().map(|p| WorldPoint { x: p.x, z: p.z }));
            }
        }
        points.push(frontage_walk);
        points.dedup_by(|a, b| (a.x - b.x).hypot(a.z - b.z) < 0.01);
        Some((road.id, points))
    }
    fn rail_axis(&self) -> Option<(u32, u32, f32, f32)> {
        let (station, gateway) = self.rail_terminals()?;
        let (_, tangent) = self.rail_pose(station, gateway, 0.0)?;
        Some((station, gateway, tangent.x, tangent.z))
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
        if self.state.tick % POCKET_PARK_INTERVAL_TICKS == 0 && self.state.development_rate > 0 {
            self.dedicate_pocket_park();
        }
        self.trim_completed_projects();
        self.advance_face_access();
        if self.state.tick % 200 == 0 {
            self.ensure_bus_service();
            self.schedule_residents(day);
            self.schedule_deliveries();
        }
        if self.state.tick % 10 == 0 {
            self.schedule_neighborhood_errands();
        }
        self.advance_travelers();
        self.advance_expressway_traffic();
        self.advance_trains();
        self.advance_buses();
        self.advance_pending_alights();
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
    /// Once enough households live around a still-vacant master-plan lot, the
    /// neighborhood keeps it as protected open space. Protected parcels are
    /// never developed; the renderer draws vacant ones as pocket parks.
    /// Selection is deterministic and draws no random numbers.
    fn dedicate_pocket_park(&mut self) {
        if !self.masterplan_active() {
            return;
        }
        let homes: Vec<(f32, f32)> = self
            .state
            .buildings
            .iter()
            .filter(|b| is_housing(b) && site_active(b))
            .map(|b| (b.x, b.z))
            .collect();
        let built: BTreeSet<u32> = self.state.buildings.iter().map(|b| b.parcel_id).collect();
        // Only pocket parks count toward the quota and spacing; the station
        // plaza and neighborhood shrines are separate open space.
        let parks: Vec<(f32, f32)> = self
            .state
            .parcels
            .iter()
            .filter(|p| is_pocket_park(p) && !built.contains(&p.id))
            .map(|p| (p.x, p.z))
            .collect();
        if homes.len() < POCKET_PARK_MIN_HOMES
            || parks.len().saturating_mul(HOMES_PER_POCKET_PARK) >= homes.len()
        {
            return;
        }
        let mut best: Option<(usize, u32)> = None;
        for parcel in &self.state.parcels {
            if parcel.protected
                || parcel.reservation.is_some()
                || built.contains(&parcel.id)
                || !POCKET_PARK_AREA.contains(&(parcel.width * parcel.depth))
                || parks
                    .iter()
                    .any(|&(x, z)| (x - parcel.x).hypot(z - parcel.z) < POCKET_PARK_SPACING)
                || !self.node_connected(parcel.access_node)
                || !self.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                || self.is_station_tower_lot(parcel)
            {
                continue;
            }
            let nearby = homes
                .iter()
                .filter(|&&(x, z)| (x - parcel.x).hypot(z - parcel.z) <= POCKET_PARK_CATCHMENT)
                .count();
            if nearby >= POCKET_PARK_MIN_HOMES && best.is_none_or(|(count, _)| nearby > count) {
                best = Some((nearby, parcel.id));
            }
        }
        let Some((nearby, parcel_id)) = best else {
            return;
        };
        if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
            parcel.protected = true;
            parcel.open_space = Some("park".into());
        }
        self.record(
            "ParkDedicated",
            parcel_id,
            format!("A vacant lot becomes a pocket park for {nearby} nearby homes"),
        );
        self.state.revision = self.state.revision.saturating_add(1);
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
        self.advance_height_redevelopment();
        self.sync_masterplan_road_projects();
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
        if self.complete_height_redevelopment(id) {
            return;
        }
        if p.kind == "extension" {
            self.complete_rear_extension_project(id);
            return;
        }
        if self.masterplan_complete_project(&p) {
            self.mark_access_geometry_changed();
            self.state.revision = self.state.revision.saturating_add(1);
            self.trim_completed_projects();
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
        if self.masterplan_active() {
            self.masterplan_growth();
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
        if connected_ordinary_roads >= 40
            && self.state.tick % 160 == 0
            && self.state.demand > 1.1
            && self.try_start_rear_extension()
        {
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
        let grid_candidate_count = candidates.len();
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
        // A legal narrow lot may fit at a grid midpoint even when projected
        // gap exclusions shift their centers or the ordinary grid lot is too
        // wide. Append the existing minimum-width shape without changing the
        // old grid/gap/shallow prefix or any admission clearance. The persisted
        // scan still visits at most MAX_FRONTAGE_SITES_PER_EVALUATION sites.
        for index in 0..grid_candidate_count {
            let site = candidates[index];
            let duplicate_shape = candidates.iter().zip(dimensions.iter()).any(|(old, size)| {
                (old.0 - site.0).hypot(old.1 - site.1) <= 1.0e-4
                    && (size.0 - 8.0).abs() <= 1.0e-4
                    && (size.1 - FRONTAGE_GAP_DEPTH).abs() <= 1.0e-4
            });
            if !duplicate_shape {
                candidates.push(site);
                dimensions.push((8.0, FRONTAGE_GAP_DEPTH));
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
        let result = self.neighborhood_candidate_clear_with_layout_reason(
            anchor,
            corners,
            tower_envelopes,
            collision_context,
            corner_ids,
            reused_edge,
            selected_lots,
            spine_plan,
        );
        #[cfg(test)]
        neighborhood_admission::record_outcome(&result, anchor.id, corners);
        result.is_ok()
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
                #[cfg(test)]
                let partial_lots = if self.partial_full_block_experiment {
                    let lots = self.available_coordinated_block_lots(&corners, &collision_context);
                    let canonical_count = blocks::subdivide_rectangular_block(
                        corners.map(|point| blocks::BlockPoint {
                            x: point.x,
                            z: point.z,
                        }),
                        8.0,
                        11.0,
                        2.2,
                    )
                    .len();
                    (lots.len() < canonical_count).then_some(lots)
                } else {
                    None
                };
                #[cfg(test)]
                let candidate_clear = if let Some(lots) = partial_lots.as_deref() {
                    let candidate_result = self.neighborhood_candidate_clear_with_layout_reason(
                        &anchor,
                        &corners,
                        &tower_envelopes,
                        &collision_context,
                        [Some(anchor.id), None, None, None],
                        None,
                        Some(lots),
                        None,
                    );
                    neighborhood_admission::record_outcome(&candidate_result, anchor.id, &corners);
                    {
                        let canonical_count = blocks::subdivide_rectangular_block(
                            corners.map(|point| blocks::BlockPoint {
                                x: point.x,
                                z: point.z,
                            }),
                            8.0,
                            11.0,
                            2.2,
                        )
                        .len();
                        partial_block_diagnostic::record_partial_candidate(
                            anchor.id,
                            &corners,
                            canonical_count,
                            lots,
                            &candidate_result,
                        );
                    }
                    candidate_result.is_ok()
                } else {
                    self.neighborhood_candidate_clear(
                        &anchor,
                        &corners,
                        &tower_envelopes,
                        &collision_context,
                    )
                };
                #[cfg(not(test))]
                let candidate_clear = self.neighborhood_candidate_clear(
                    &anchor,
                    &corners,
                    &tower_envelopes,
                    &collision_context,
                );
                if candidate_clear {
                    #[cfg(test)]
                    let (score_lots, selected_lots) = if let Some(lots) = partial_lots {
                        (lots, None)
                    } else {
                        let canonical_lots = blocks::subdivide_rectangular_block(
                            corners.map(|point| blocks::BlockPoint {
                                x: point.x,
                                z: point.z,
                            }),
                            8.0,
                            11.0,
                            2.2,
                        );
                        (canonical_lots, None)
                    };
                    #[cfg(not(test))]
                    let (score_lots, selected_lots) = (
                        blocks::subdivide_rectangular_block(
                            corners.map(|point| blocks::BlockPoint {
                                x: point.x,
                                z: point.z,
                            }),
                            8.0,
                            11.0,
                            2.2,
                        ),
                        None,
                    );
                    let candidate = Self::ranked_neighborhood_candidate(
                        anchor.id,
                        corners,
                        [Some(anchor.id), None, None, None],
                        None,
                        selected_lots,
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
        if choices.is_empty() && !self.masterplan_active() {
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
        let (kind, capacity, floors) =
            self.masterplan_adjust_profile(parcel.id, kind, capacity, floors);
        // Test-only matched experiment: before reserving a normal building
        // project, try a deeper initial parcel while retaining the original
        // parcel for use/floor selection and every production admission gate.
        #[cfg(test)]
        let parcel = initial_depth_diagnostic::maybe_expand_selected_parcel(self, parcel);
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
        if self.try_start_height_redevelopment() {
            return;
        }
        self.try_close_business_for_turnover(day);
        let candidates: Vec<u32> = self
            .state
            .buildings
            .iter()
            .filter(|b| {
                !b.anchor
                    && !self.errand_references_building(b.id)
                    && site_vacant(b)
                    && b.vacant_days > 5
                    && !self
                        .state
                        .pending_rear_extensions
                        .iter()
                        .any(|plan| plan.building_id == b.id)
            })
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
                    && !self
                        .state
                        .pending_rear_extensions
                        .iter()
                        .any(|plan| plan.building_id == building.id)
                    && day.saturating_sub(building.built_day) >= 30
                    && !referenced.contains(&building.id)
                    && !self.errand_references_building(building.id)
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
            let available =
                occupancy.saturating_sub(self.aggregate_participants_at_home(home_id) as u16);
            for ordinal in current..available {
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
                    pending_alight: None,
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
                let connector = self.bus_route_avoiding(last, candidate, &visited);
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
        self.bus_queue_position_with_edge(node_id, direction, slot)
            .map(|(point, _)| point)
    }

    fn bus_queue_position_with_edge(
        &self,
        node_id: u32,
        direction: i8,
        slot: u16,
    ) -> Option<(WorldPoint, [u32; 2])> {
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
        Some((
            WorldPoint {
                x: node.x + (other.x - node.x) / length * along
                    - (other.z - node.z) / length * BUS_STOP_KERB,
                z: node.z
                    + (other.z - node.z) / length * along
                    + (other.x - node.x) / length * BUS_STOP_KERB,
            },
            [node_id, other_id],
        ))
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
                let curb = BUS_STOP_KERB;
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
            let end = self.route_endpoint_position(&traveler.route, false, id, 0);
            if let Some(start) = end {
                traveler.destination_access = Some(AccessPath {
                    points: vec![start, queue_position],
                    surface_y: Vec::new(),
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
                "walking_to_bus"
                    | "waiting_bus"
                    | "riding_bus"
                    | "walking_from_bus"
                    | "waiting_alight"
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
        self.route_where(start, goal, blocked, |road| {
            (rail && road.kind == "rail") || (!rail && road.kind != "rail")
        })
    }
    fn bus_route_avoiding(&self, start: u32, goal: u32, blocked: &BTreeSet<u32>) -> Vec<u32> {
        // Full-size neighborhood buses stay on collectors and wider ground
        // streets. Residents can still walk through lanes to reach these stops.
        self.route_where(start, goal, blocked, |road| {
            matches!(road.kind.as_str(), "local" | "arterial" | "bridge")
        })
    }
    fn route_where(
        &self,
        start: u32,
        goal: u32,
        blocked: &BTreeSet<u32>,
        eligible: impl Fn(&Road) -> bool,
    ) -> Vec<u32> {
        if start == goal {
            return vec![start];
        }
        // Keep source order, including parallel edges, so equal-cost choices
        // match the full-road scan. This index lives only for this immutable
        // query; road openings and splits cannot leave a stale route cache.
        let mut adjacency = Vec::with_capacity(self.state.roads.len() * 2);
        for (index, road) in self
            .state
            .roads
            .iter()
            .enumerate()
            .filter(|(_, road)| road.connected && road.stage == "open" && eligible(road))
        {
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
            for &(_, index) in adjacency[first..]
                .iter()
                .take_while(|(node, _)| *node == current)
            {
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
    /// Cars queue on driveways the way they do on roads. Access paths share
    /// geometry at both ends: departures leave a shared door and merge where
    /// neighbouring driveways meet the road; arrivals do the reverse. Each car
    /// therefore queues against both ends of its path, ordered by distance from
    /// that end. Returns, per traveler, the furthest access progress it may reach
    /// this tick (one queue gap behind the car ahead; ties go to the lower id),
    /// and the road mouths whose arrival driveway is still occupied.
    fn driveway_queues(&self) -> (Vec<f32>, BTreeSet<(i32, i32)>) {
        let station = self.station_node();
        let travelers = &self.state.travelers;
        let mut limit = vec![f32::INFINITY; travelers.len()];
        // (phase, queues at the path start?, end point) -> (index, rank, path length).
        // Rank grows toward the back of the queue.
        let mut lanes: BTreeMap<(u8, bool, (i32, i32)), Vec<(usize, f32, f32)>> = BTreeMap::new();
        for (index, t) in travelers.iter().enumerate() {
            let path = match (t.kind, t.access_phase) {
                (1, 0) => t.origin_access.as_ref(),
                (1, 2) => t.destination_access.as_ref(),
                _ => None,
            };
            let Some(path) = path else { continue };
            let (Some(start), Some(end)) = (path.points.first(), path.points.last()) else {
                continue;
            };
            let length = path.length();
            // Leaving the start: further along is ahead. Approaching the end: closer is ahead.
            lanes
                .entry((t.access_phase, true, driveway_mouth(*start)))
                .or_default()
                .push((index, -t.access_progress, length));
            lanes
                .entry((t.access_phase, false, driveway_mouth(*end)))
                .or_default()
                .push((index, length - t.access_progress, length));
        }
        let mut entry_busy = BTreeSet::new();
        for ((phase, at_start, point), list) in &mut lanes {
            list.sort_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then(travelers[a.0].id.cmp(&travelers[b.0].id))
            });
            for pair in list.windows(2) {
                let ((lead, lead_rank, _), (follower, _, length)) = (pair[0], pair[1]);
                let gap = vehicle_gap(
                    Self::vehicle_body(&travelers[lead], station),
                    Self::vehicle_body(&travelers[follower], station),
                );
                let allowed = if *at_start {
                    -lead_rank - gap
                } else {
                    length - (lead_rank + gap)
                };
                limit[follower] = limit[follower].min(allowed);
            }
            // Arrival paths start at the road: the mouth is busy until the last car clears it.
            if *phase == 2 && *at_start {
                let last = &travelers[list.last().unwrap().0];
                if last.access_progress
                    < vehicle_length(Self::vehicle_body(last, station)) + VEHICLE_QUEUE_MARGIN
                {
                    entry_busy.insert(*point);
                }
            }
        }
        (limit, entry_busy)
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
        let (driveway_limit, mut driveway_entry_busy) = self.driveway_queues();
        for index in 0..count {
            let traveler = &mut self.state.travelers[index];
            match traveler.access_phase {
                0 => {
                    skip_road[index] = true;
                    if let Some(path) = &traveler.origin_access {
                        let next =
                            traveler.access_progress + traveler.speed / TICKS_PER_SECOND as f32;
                        traveler.access_progress =
                            next.min(driveway_limit[index].max(traveler.access_progress));
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
                        let next =
                            traveler.access_progress + traveler.speed / TICKS_PER_SECOND as f32;
                        traveler.access_progress =
                            next.min(driveway_limit[index].max(traveler.access_progress));
                        if traveler.access_progress >= path.length() {
                            arrivals.push(traveler.clone());
                        }
                    } else {
                        arrivals.push(traveler.clone());
                    }
                }
                _ => {
                    traveler.access_phase = 1;
                    // A car held at the kerb by a busy driveway retries here, before
                    // any signal can stop it short of the mouth it already reached.
                    let at_kerb = traveler.kind == 1
                        && traveler.edge_index + 1 == traveler.route_lengths.len()
                        && traveler
                            .route_lengths
                            .last()
                            .is_some_and(|&len| traveler.edge_progress >= len.max(1.0));
                    if traveler.edge_index >= traveler.route_lengths.len() || at_kerb {
                        // A car waits at the kerb while another is still at the driveway mouth.
                        let mouth = traveler
                            .destination_access
                            .as_ref()
                            .and_then(|path| path.points.first())
                            .map(|p| driveway_mouth(*p));
                        let held = traveler.kind == 1
                            && mouth.is_some_and(|m| driveway_entry_busy.contains(&m));
                        if held {
                            // Still a road traveler: drawn, and queued ahead of its followers.
                            // (Saves from before kerb holding kept the car past its route.)
                            hold_at_kerb(traveler);
                            skip_road[index] = traveler.edge_index >= traveler.route_lengths.len();
                        } else {
                            skip_road[index] = true;
                            if traveler.destination_access.is_some() {
                                traveler.access_phase = 2;
                                traveler.access_progress = 0.0;
                                if traveler.kind == 1 {
                                    driveway_entry_busy.extend(mouth);
                                }
                            } else {
                                arrivals.push(traveler.clone());
                            }
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
        // behind the leader before reaching the intersection approach. Runs every
        // tick: the red-light stop above also does, and a sparser queue let
        // followers creep onto the leader at the stop line between passes.
        {
            let station = self.station_node();
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
                    let (lead_t, follower_t) =
                        (&self.state.travelers[lead], &self.state.travelers[follower]);
                    let gap = if lead_t.kind == 1 && follower_t.kind == 1 {
                        vehicle_gap(
                            Self::vehicle_body(lead_t, station),
                            Self::vehicle_body(follower_t, station),
                        )
                    } else if lead_t.kind == 1 || follower_t.kind == 1 {
                        VEHICLE_MIN_GAP
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
        // Merge gate: a car enters a road lane only when the last car to enter
        // it is a queue gap down the road. Cars are admitted in index order and
        // each admission becomes the lane's new tail, so two cars arriving from
        // different approaches on one tick cannot both enter.
        let station = self.station_node();
        let mut lane_tail: BTreeMap<(u32, u32), (f32, u8)> = BTreeMap::new();
        for (index, t) in self.state.travelers.iter().enumerate() {
            if t.kind != 1 || skip_road[index] {
                continue;
            }
            if let Some(r) = t.route.get(t.edge_index..=t.edge_index + 1) {
                let entry = lane_tail
                    .entry((r[0], r[1]))
                    .or_insert((f32::INFINITY, VEHICLE_COMPACT));
                if t.edge_progress < entry.0 {
                    *entry = (t.edge_progress, Self::vehicle_body(t, station));
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
            let body = Self::vehicle_body(t, station);
            let mut target = planned[index];
            if t.kind == 1 {
                // Stop a queue gap behind the last car into the next lane, not at the node.
                let edge_len = t.route_lengths[t.edge_index].max(1.0);
                let next_lane = t
                    .route
                    .get(t.edge_index + 1..=t.edge_index + 2)
                    .map(|r| (r[0], r[1]));
                if let Some(tail) = next_lane.and_then(|lane| lane_tail.get(&lane)) {
                    let gap = vehicle_gap(tail.1, body);
                    if tail.0 < gap {
                        target = target.min(edge_len - (gap - tail.0)).max(t.edge_progress);
                    }
                }
            }
            let mut remaining = (target - t.edge_progress).max(0.0);
            while remaining > 0.0 && t.edge_index < t.route_lengths.len() {
                let edge_len = t.route_lengths[t.edge_index].max(1.0);
                let available = edge_len - t.edge_progress;
                if remaining + 0.0001 < available {
                    t.edge_progress += remaining;
                    break;
                }
                let next_lane = t
                    .route
                    .get(t.edge_index + 1..=t.edge_index + 2)
                    .map(|r| (r[0], r[1]));
                if t.kind == 1 {
                    if let Some(tail) = next_lane.and_then(|lane| lane_tail.get(&lane)) {
                        if tail.0 < vehicle_gap(tail.1, body) {
                            // Wait at the junction for the lane ahead to clear.
                            t.edge_progress = edge_len;
                            break;
                        }
                    }
                }
                remaining = (remaining - available).max(0.0);
                t.edge_index += 1;
                t.edge_progress = 0.0;
                if t.kind == 1 {
                    if let Some(lane) = next_lane {
                        lane_tail.insert(
                            lane,
                            (
                                remaining
                                    .min(t.route_lengths.get(t.edge_index).copied().unwrap_or(0.0)),
                                body,
                            ),
                        );
                    }
                }
            }
            if t.edge_index >= t.route_lengths.len() {
                let mouth = t
                    .destination_access
                    .as_ref()
                    .and_then(|path| path.points.first())
                    .map(|p| driveway_mouth(*p));
                let held = t.kind == 1 && mouth.is_some_and(|m| driveway_entry_busy.contains(&m));
                // A held car waits at the kerb end of its last lane and retries
                // admission each tick as it tries to advance past the end.
                if held {
                    hold_at_kerb(t);
                } else {
                    if t.destination_access.is_some() {
                        t.access_phase = 2;
                        t.access_progress = 0.0;
                        if t.kind == 1 {
                            driveway_entry_busy.extend(mouth);
                        }
                    } else {
                        arrivals.push(t.clone());
                    }
                }
            }
        }
        let arrived_ids: BTreeSet<u32> = arrivals.iter().map(|t| t.id).collect();
        self.state
            .travelers
            .retain(|t| !arrived_ids.contains(&t.id));
        let mut return_vehicles = Vec::new();
        for arrived in arrivals {
            if self.handle_neighborhood_errand_arrival(&arrived) {
                continue;
            }
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
                returning.origin_access = returning.origin_building.and_then(|building| {
                    self.building_access_path(
                        building,
                        &returning.route,
                        true,
                        returning.id,
                        returning.kind,
                    )
                });
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
        let wait_position = self
            .state
            .buses
            .get(bus_index)
            .and_then(|bus| {
                self.bus_queue_position(stop, bus.direction, resident.queue_slot.unwrap_or(0))
            })
            .or_else(|| {
                self.node(stop).map(|node| WorldPoint {
                    x: node.x,
                    z: node.z,
                })
            });
        let Some(wait_position) = wait_position else {
            return;
        };
        let activity = if resident.at_work {
            "return"
        } else {
            "commute"
        };
        let route_attempted = self.state.travelers.len() < MAX_TRAVELERS;
        let route = if route_attempted {
            self.route(stop, target_node, false)
        } else {
            Vec::new()
        };
        if route.is_empty() || route.len() > MAX_ROUTE_NODES {
            let queued = self.queue_pending_alight(
                resident_id,
                stop,
                target_node,
                destination_building,
                activity,
                "walking_from_bus",
                wait_position,
                route_attempted,
            );
            if !queued {
                let reason = format!("Still aboard at stop {stop}: pedestrian route queue is full");
                let current = &mut self.state.residents[resident_index];
                if current.history.last() != Some(&reason) {
                    push_history(&mut current.history, reason);
                }
                return;
            }
            let bus_id = self.state.buses[bus_index].id;
            self.state.buses[bus_index]
                .passengers
                .retain(|id| *id != resident_id);
            push_history(
                &mut self.state.residents[resident_index].history,
                format!(
                    "Waited at stop {stop} for a legal walking route at tick {}",
                    self.state.tick
                ),
            );
            self.record(
                "BusPassengerWaitingForRoute",
                resident_id,
                format!("A resident alights from bus {bus_id} into the bounded route queue at stop {stop}"),
            );
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
            activity: activity.into(),
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
                    self.route_endpoint_position(&traveler.route, true, resident.id, 0)
                {
                    traveler.origin_access = Some(AccessPath {
                        points: vec![queue, endpoint],
                        surface_y: Vec::new(),
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
                dwell_ticks: TRAIN_STATION_DWELL_TICKS,
                passengers: Vec::new(),
            });
        }
        let mut stops = Vec::new();
        let mut opened_terminals = Vec::new();
        let quiet = self.rail_service_quiet();
        let station_queue_empty = self.rail_waiting_count(true) == 0;
        for train in &mut self.state.trains {
            if train.route != route {
                train.route = route.clone();
                train.distance = TRAIN_HALF_LENGTH;
                train.direction = 1;
                train.dwell_ticks = TRAIN_STATION_DWELL_TICKS;
                train.passengers.clear();
            }
            let at_station = train.direction >= 0 && train.distance <= TRAIN_HALF_LENGTH + 0.01;
            if quiet && at_station && train.passengers.is_empty() && station_queue_empty {
                train.dwell_ticks = TRAIN_STATION_DWELL_TICKS;
                continue;
            }
            if train.dwell_ticks > 0 {
                train.dwell_ticks -= 1;
                let at_station = train.direction >= 0 && train.distance <= TRAIN_HALF_LENGTH + 0.01;
                let dwell_total = if at_station {
                    TRAIN_STATION_DWELL_TICKS
                } else {
                    TRAIN_GATEWAY_DWELL_TICKS
                };
                if train.dwell_ticks == dwell_total - TRAIN_DOOR_PHASE_TICKS {
                    opened_terminals.push((train.id, at_station));
                }
                continue;
            }
            let step = train.speed / TICKS_PER_SECOND as f32;
            if train.direction >= 0 {
                train.distance += step;
                let far_terminal = rail.length - TRAIN_HALF_LENGTH;
                if train.distance >= far_terminal {
                    train.distance = far_terminal;
                    train.direction = -1;
                    train.dwell_ticks = TRAIN_GATEWAY_DWELL_TICKS;
                    stops.push((
                        train.id,
                        "TrainGatewayStop",
                        "The regional shuttle pauses at its outer terminus",
                    ));
                }
            } else {
                train.distance -= step;
                if train.distance <= TRAIN_HALF_LENGTH {
                    train.distance = TRAIN_HALF_LENGTH;
                    train.direction = 1;
                    train.dwell_ticks = TRAIN_STATION_DWELL_TICKS;
                    stops.push((
                        train.id,
                        "TrainStationStop",
                        "The shuttle pauses at the neighborhood platform",
                    ));
                }
            }
        }
        for (id, kind, description) in stops {
            self.record(kind, id, description.into());
        }
        for (id, at_station) in opened_terminals {
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
                let queue_slot = (resident.id % RAIL_QUEUE_CAPACITY as u32) as u16;
                let wait_position = self.platform_position(true, queue_slot);
                let route_attempted =
                    resident.home_node != station && self.state.travelers.len() < MAX_TRAVELERS;
                let route = if resident.home_node == station {
                    vec![station]
                } else if route_attempted {
                    self.route(station, resident.home_node, false)
                } else {
                    Vec::new()
                };
                if route.is_empty() {
                    let queued = wait_position.is_some_and(|wait_position| {
                        self.queue_pending_alight(
                            resident_id,
                            station,
                            resident.home_node,
                            resident.home_building,
                            "walking_home",
                            "walking_home",
                            wait_position,
                            route_attempted,
                        )
                    });
                    if !queued {
                        let reason = format!(
                            "Still aboard at station {station}: pedestrian route queue is full"
                        );
                        let current = &mut self.state.residents[resident_index];
                        if current.history.last() != Some(&reason) {
                            push_history(&mut current.history, reason);
                        }
                        continue;
                    }
                    unloaded.insert(resident_id);
                    push_history(
                        &mut self.state.residents[resident_index].history,
                        format!(
                            "Waited at station {station} for a legal walking route at tick {}",
                            self.state.tick
                        ),
                    );
                    self.record(
                        "TrainPassengerWaitingForRoute",
                        resident_id,
                        format!("A resident alights from train {train_id} into the bounded route queue at station {station}"),
                    );
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
                } else {
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
                    traveler.origin_access =
                        self.platform_exit_path(&traveler.route, true, queue_slot, traveler.id);
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
        let at_station = direction >= 0 && distance <= TRAIN_HALF_LENGTH + 0.01;
        let rail_length = self
            .state
            .roads
            .iter()
            .find(|road| road.kind == "rail" && road.stage == "open" && road.connected)
            .map(|road| road.length)
            .unwrap_or(0.0);
        let at_gateway = direction < 0 && distance >= rail_length - TRAIN_HALF_LENGTH - 0.01;
        let total_dwell = if at_station {
            TRAIN_STATION_DWELL_TICKS
        } else {
            TRAIN_GATEWAY_DWELL_TICKS
        };
        if (!at_station && !at_gateway)
            || dwell_ticks < TRAIN_DOOR_PHASE_TICKS
            || dwell_ticks > total_dwell - TRAIN_DOOR_PHASE_TICKS
            || passenger_count >= RAIL_TRAIN_CAPACITY
        {
            return;
        }
        if self.state.tick % 3 != 0 {
            return;
        }
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
        self.movement_rows(false)
            .into_iter()
            .flat_map(|row| {
                [
                    row.actor_id as f32,
                    row.x,
                    row.z,
                    row.heading,
                    row.kind as f32,
                    row.surface_y,
                ]
            })
            .collect()
    }
    /// Versioned movement poses and their same-traversal infrastructure support.
    pub fn movement_packet(&self) -> Vec<u32> {
        movement_support::Row::encode(&self.movement_rows(true))
    }
    fn movement_rows(&self, include_support: bool) -> Vec<movement_support::Row> {
        let mut out = Vec::with_capacity(
            self.state.travelers.len()
                + self.state.expressway_traffic.trips.len()
                + self.state.trains.len()
                + self.state.buses.len()
                + self.state.residents.len(),
        );
        let station = self.station_node();
        for t in &self.state.travelers {
            if t.access_phase == 0 || t.access_phase == 2 {
                // A car still at the start of its driveway has not pulled out:
                // it waits off-screen behind the car ahead (see driveway_queues).
                if t.kind == 1 && t.access_phase == 0 && t.access_progress <= 0.0 {
                    continue;
                }
                let path = if t.access_phase == 0 {
                    t.origin_access.as_ref()
                } else {
                    t.destination_access.as_ref()
                };
                if let Some((point, heading, surface_y)) =
                    path.and_then(|path| path.sample_surface(t.access_progress))
                {
                    let support = if include_support {
                        path.and_then(|path| self.access_movement_support(t, path))
                    } else {
                        None
                    };
                    let resolved = support.is_some();
                    let (road_ids, platform_node_ids) = support.unwrap_or_default();
                    out.push(movement_support::Row {
                        actor_id: t.id,
                        x: point.x,
                        z: point.z,
                        heading,
                        kind: Self::vehicle_body(t, station) as u32,
                        surface_y,
                        resolved,
                        road_ids,
                        platform_node_ids,
                    });
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
            let heading = (b.z - a.z).atan2(b.x - a.x);
            let road_ids = if include_support {
                self.state
                    .roads
                    .iter()
                    .find(|r| {
                        r.stage == "open"
                            && r.connected
                            && r.kind != "rail"
                            && ((r.from == a.id && r.to == b.id)
                                || (r.from == b.id && r.to == a.id))
                    })
                    .map(|r| vec![r.id])
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            out.push(movement_support::Row {
                actor_id: t.id,
                x,
                z,
                heading,
                kind: Self::vehicle_body(t, station) as u32,
                surface_y: -1.0,
                resolved: road_ids.len() == 1,
                road_ids,
                platform_node_ids: Vec::new(),
            });
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
                    let rail_ids: Vec<u32> = if include_support {
                        self.platform_support_road(node_id).into_iter().collect()
                    } else {
                        Vec::new()
                    };
                    out.push(movement_support::Row {
                        actor_id: resident.id,
                        x: position.x,
                        z: position.z,
                        heading: 0.0,
                        kind: 0,
                        surface_y: 8.0,
                        resolved: rail_ids.len() == 1,
                        road_ids: rail_ids,
                        platform_node_ids: vec![node_id],
                    });
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
            let Some((position, edge)) = self.bus_queue_position_with_edge(stop, direction, slot)
            else {
                continue;
            };
            let road_ids: Vec<u32> = if include_support {
                self.state
                    .roads
                    .iter()
                    .find(|r| {
                        r.stage == "open"
                            && r.connected
                            && r.kind != "rail"
                            && ((r.from == edge[0] && r.to == edge[1])
                                || (r.from == edge[1] && r.to == edge[0]))
                    })
                    .map(|r| r.id)
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            };
            out.push(movement_support::Row {
                actor_id: resident.id,
                x: position.x,
                z: position.z,
                heading: 0.0,
                kind: 0,
                surface_y: -1.0,
                resolved: road_ids.len() == 1,
                road_ids,
                platform_node_ids: Vec::new(),
            });
        }
        let mut pending_alights: Vec<&Resident> = self
            .state
            .residents
            .iter()
            .filter(|resident| resident.transfer == "waiting_alight")
            .collect();
        pending_alights.sort_by_key(|resident| resident.id);
        for resident in pending_alights {
            let Some(pending) = &resident.pending_alight else {
                continue;
            };
            let surface_y = if pending.transfer_after_route == "walking_home"
                && self.rail_terminals().is_some_and(|(station, gateway)| {
                    pending.stop_node == station || pending.stop_node == gateway
                }) {
                8.0
            } else {
                -1.0
            };
            let rail_ids: Vec<u32> = if include_support {
                self.platform_support_road(pending.stop_node)
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            };
            let resolved = surface_y == 8.0 && rail_ids.len() == 1;
            out.push(movement_support::Row {
                actor_id: resident.id,
                x: pending.wait_position.x,
                z: pending.wait_position.z,
                heading: 0.0,
                kind: 0,
                surface_y,
                resolved,
                road_ids: rail_ids,
                platform_node_ids: if resolved {
                    vec![pending.stop_node]
                } else {
                    Vec::new()
                },
            });
        }
        for t in &self.state.trains {
            if t.route.len() < 2 {
                continue;
            }
            let Some((point, tangent)) = self.rail_pose(t.route[0], t.route[1], t.distance) else {
                continue;
            };
            let forward_heading = tangent.z.atan2(tangent.x);
            let heading = if t.direction >= 0 {
                forward_heading
            } else {
                forward_heading + std::f32::consts::PI
            };
            let x = point.x;
            let z = point.z;
            let road_ids: Vec<u32> = if include_support {
                t.route
                    .windows(2)
                    .filter_map(|edge| {
                        self.state
                            .roads
                            .iter()
                            .find(|r| {
                                r.kind == "rail"
                                    && r.stage == "open"
                                    && r.connected
                                    && ((r.from == edge[0] && r.to == edge[1])
                                        || (r.from == edge[1] && r.to == edge[0]))
                            })
                            .map(|r| r.id)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            out.push(movement_support::Row {
                actor_id: t.id,
                x,
                z,
                heading,
                kind: 2,
                surface_y: -1.0,
                resolved: road_ids.len() == t.route.len() - 1,
                road_ids,
                platform_node_ids: Vec::new(),
            });
        }
        if let Some(line) = &self.state.bus_line {
            for bus in &self.state.buses {
                let Some(((x, z, heading), edge)) = self.bus_pose_with_edge(line, bus) else {
                    continue;
                };
                let road_ids: Vec<u32> = if include_support {
                    self.state
                        .roads
                        .iter()
                        .find(|r| {
                            r.stage == "open"
                                && r.connected
                                && r.kind != "rail"
                                && ((r.from == edge[0] && r.to == edge[1])
                                    || (r.from == edge[1] && r.to == edge[0]))
                        })
                        .map(|r| r.id)
                        .into_iter()
                        .collect()
                } else {
                    Vec::new()
                };
                out.push(movement_support::Row {
                    actor_id: bus.id,
                    x,
                    z,
                    heading,
                    kind: 3,
                    surface_y: -1.0,
                    resolved: road_ids.len() == 1,
                    road_ids,
                    platform_node_ids: Vec::new(),
                });
            }
        }
        // Expressway route provenance uses plan-road indices; it is resolved in
        // the dedicated helper where the same sampled pose is emitted.
        self.append_expressway_movement_rows(&mut out, include_support);
        out
    }

    /// Compact authoritative train-door presentation sampled at the same tick
    /// as movement_buffer. Tuples are [train_id, terminal_id or -1, fraction].
    pub fn train_doors(&self) -> Vec<f32> {
        let rail_length = self.rail_length_for_doors();
        let mut out = Vec::with_capacity(self.state.trains.len().min(MAX_PRESENTED_TRAINS) * 3);
        for train in self.state.trains.iter().take(MAX_PRESENTED_TRAINS) {
            let (terminal, fraction) = train_door_state(train, rail_length);
            out.extend_from_slice(&[
                train.id as f32,
                terminal.map(|id| id as f32).unwrap_or(-1.0),
                fraction,
            ]);
        }
        out
    }

    fn rail_length_for_doors(&self) -> f32 {
        self.state
            .roads
            .iter()
            .find(|road| road.kind == "rail" && road.stage == "open" && road.connected)
            .map(|road| road.length)
            .unwrap_or(0.0)
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
        self.bus_pose_with_edge(line, bus).map(|(pose, _)| pose)
    }

    // Return provenance from the exact direction-sensitive edge selection used
    // for the pose, including the tolerance at shared junctions.
    fn bus_pose_with_edge(&self, line: &BusLine, bus: &Bus) -> Option<((f32, f32, f32), [u32; 2])> {
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
                    (
                        a.x + (b.x - a.x) * f + nx * KERB_LANE,
                        a.z + (b.z - a.z) * f + nz * KERB_LANE,
                        heading,
                    ),
                    [a.id, b.id],
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
        let walking_offset = road
            .map(|r| pedestrian_road_offset(&r.kind))
            .unwrap_or_else(|| pedestrian_road_offset("unknown"));
        let lateral = if traveler.kind == 1 {
            KERB_LANE
        } else if traveler.id % 2 == 0 {
            walking_offset
        } else {
            -walking_offset
        };
        let nx = -dz / len;
        let nz = dx / len;
        (nx * lateral, nz * lateral)
    }
    pub fn inspect_json(&self, id: u32) -> String {
        if let Some(value) = self.inspect_expressway_trip(id) {
            return value.to_string();
        }
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
            serde_json::json!({"entityType":"traveler","id":t.id,"name":t.name,"activity":t.activity,"kind":if t.kind == 1 {"car"} else {"pedestrian"},"vehicle":if t.kind == 1 {Some(vehicle_name(Self::vehicle_body(t, self.station_node())))} else {None},"origin":t.origin_building,"destination":t.destination_building,"history":history,"wait_ticks":t.wait_ticks})
        } else if let Some(r) = self.state.residents.iter().find(|r| r.id == id) {
            serde_json::json!({"entityType":"resident","id":r.id,"name":r.name,"activity":r.activity,"kind":if r.train_id.is_some() {"rail passenger"} else if r.bus_id.is_some() {"bus passenger"} else if r.mode == 1 {"car"} else {"pedestrian"},"origin":r.home_building,"destination":r.work_building,"home_building":r.home_building,"work_building":r.work_building,"transfer":r.transfer,"train_id":r.train_id,"bus_id":r.bus_id,"bus_origin_stop":r.bus_origin_stop,"bus_destination_stop":r.bus_destination_stop,"bus_stop_node":r.bus_stop_node,"pending_alight":r.pending_alight,"location_node":self.resident_location_node(r),"history":r.history})
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
            "waiting_alight" => resident
                .pending_alight
                .as_ref()
                .map(|pending| pending.stop_node),
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
                    points: access.iter().map(|p| Point { x: p.x, z: p.z }).collect(),
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
        let rail_length = self.rail_length_for_doors();
        Snapshot {
            schema_version: SCHEMA_VERSION,
            engine_version: ENGINE_VERSION.into(),
            seed: self.state.seed,
            tick: self.state.tick,
            day,
            minute_of_day: minute,
            development_rate: self.state.development_rate,
            revision: self.state.revision,
            presentation_revision: self.presentation_revision(),
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
            road_decks: self.masterplan_road_decks(),
            expressway_profile: expressway_profile::active(),
            rail_platforms: self.rail_platform_views(),
            rail_alignments: self.rail_alignment_views(),
            parcels: self.state.parcels.clone(),
            buildings: self.state.buildings.clone(),
            building_access: self.building_access_views(),
            rear_extensions: self.rear_extension_views(),
            ground: self.masterplan_ground(),
            building_original_footprints: self.state.building_original_footprints.clone(),
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
                    door_open_fraction: train_door_state(t, rail_length).1,
                    door_terminal: train_door_state(t, rail_length).0,
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
                    + self.state.expressway_traffic.trips.len()
                    + self.state.pending_alight_ids.len()
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
    fn river_control_points(&self) -> [Point; river_course::SAMPLES] {
        river_course::sampled(self.state.seed).map(|p| Point { x: p.x, z: p.z })
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
        if state.activity_schema_version != ACTIVITY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported business activity schema {}; start a fresh world",
                state.activity_schema_version
            ));
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
        let live_access = state
            .live_access_checkpoint
            .take()
            .map(access_live::LiveAccess::restore)
            .transpose()
            .map_err(|error| format!("invalid live access checkpoint: {error:?}"))?;
        let mut engine = Self {
            state,
            plan: None,
            height_redevelopment_lots: Vec::new(),
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
            partial_full_block_experiment: false,
            #[cfg(test)]
            road_opening_access_attempts: 0,
            #[cfg(test)]
            road_opening_access_admissions: 0,
            #[cfg(test)]
            road_opening_access_branch: None,
        };
        engine.load_masterplan()?;
        engine.refresh_height_redevelopment_lots();
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
        self.validate_access_route_joins()?;
        self.validate_expressway_traffic()?;
        self.validate_neighborhood_errands()?;
        self.validate_masterplan_progress()?;
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
        if self.state.rear_extension_cursor > self.state.buildings.len() {
            return Err("rear extension scan cursor is outside its bounded range".into());
        }
        self.validate_rear_extensions()?;
        self.validate_building_original_footprints()?;
        self.validate_height_redevelopment()?;
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
        if self.state.pending_alight_ids.len() > transit_recovery::MAX_PENDING_ALIGHT_WAITERS {
            return Err("pending transit alighting queue exceeded its cap".into());
        }
        if (self.state.pending_alight_ids.is_empty() && self.state.pending_alight_cursor != 0)
            || (!self.state.pending_alight_ids.is_empty()
                && self.state.pending_alight_cursor >= self.state.pending_alight_ids.len())
        {
            return Err("pending transit alighting cursor is invalid".into());
        }
        let mut pending_alight_ids = BTreeSet::new();
        for resident_id in &self.state.pending_alight_ids {
            if !pending_alight_ids.insert(*resident_id) {
                return Err(format!("resident {resident_id} is queued to alight twice"));
            }
            if !self.state.residents.iter().any(|resident| {
                resident.id == *resident_id
                    && resident.transfer == "waiting_alight"
                    && resident.pending_alight.is_some()
                    && resident.bus_id.is_none()
                    && resident.train_id.is_none()
            }) {
                return Err(format!(
                    "pending alight {resident_id} has no waiting resident"
                ));
            }
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
            if let Some(pending) = &resident.pending_alight {
                if resident.transfer != "waiting_alight"
                    || resident.bus_id.is_some()
                    || resident.train_id.is_some()
                    || !pending_alight_ids.contains(&resident.id)
                    || !pending.wait_position.x.is_finite()
                    || !pending.wait_position.z.is_finite()
                    || !self
                        .state
                        .nodes
                        .iter()
                        .any(|node| node.id == pending.stop_node)
                    || !self
                        .state
                        .nodes
                        .iter()
                        .any(|node| node.id == pending.destination_node)
                    || !self
                        .state
                        .buildings
                        .iter()
                        .any(|building| building.id == pending.destination_building)
                    || !matches!(
                        pending.activity.as_str(),
                        "commute" | "return" | "walking_home"
                    )
                    || !matches!(
                        pending.transfer_after_route.as_str(),
                        "walking_from_bus" | "walking_home"
                    )
                {
                    return Err(format!(
                        "resident {} has an invalid pending alight",
                        resident.id
                    ));
                }
            } else if resident.transfer == "waiting_alight" {
                return Err(format!(
                    "resident {} is waiting without an alight target",
                    resident.id
                ));
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
                        | "waiting_alight"
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
                let path_requires_rail_surface = (traveler.activity == "walking_to_station"
                    && traveler
                        .destination_access
                        .as_ref()
                        .is_some_and(|rail| std::ptr::eq(rail, path)))
                    || (traveler.activity == "walking_home"
                        && traveler
                            .origin_access
                            .as_ref()
                            .is_some_and(|rail| std::ptr::eq(rail, path))
                        && self.state.residents.iter().any(|resident| {
                            resident.id == traveler.resident && resident.rail_commuter
                        }));
                // Historical legacy-planner fixtures predate explicit rail surfaces.
                // Current master-plan worlds must always carry them; obsolete master plans
                // are rejected by the layout-version digest before this validation.
                if self.state.masterplan.is_some()
                    && path_requires_rail_surface
                    && path.surface_y.len() != path.points.len()
                {
                    return Err(format!(
                        "traveler {} has a rail access path without surface elevations",
                        traveler.id
                    ));
                }
                if path_requires_rail_surface && !path.surface_y.is_empty() {
                    let matches =
                        path.points
                            .first()
                            .zip(path.points.last())
                            .is_some_and(|(&from, &to)| {
                                [true, false].into_iter().any(|at_station| {
                                    let expected = if traveler.activity == "walking_to_station" {
                                        self.platform_entry_path_from_position(
                                            from,
                                            to,
                                            at_station,
                                            traveler.id,
                                        )
                                    } else {
                                        self.platform_exit_path_from_position(from, to, at_station)
                                    };
                                    expected.is_some_and(|expected| {
                                        expected.surface_y == path.surface_y
                                            && expected.points.len() == path.points.len()
                                            && expected
                                                .points
                                                .iter()
                                                .zip(&path.points)
                                                .all(|(a, b)| a.x == b.x && a.z == b.z)
                                    })
                                })
                            });
                    if !matches {
                        return Err(format!("traveler {} has obsolete or invalid rail access geometry; start a fresh world", traveler.id));
                    }
                }
                // Door + three reserved passage points + curb + street-end +
                // route-end. Keep a finite bound for the candidate save format.
                let max_access_points = if path_requires_rail_surface {
                    8
                } else if cfg!(feature = "flag-passage-candidate")
                    || cfg!(feature = "parent-passage-candidate")
                {
                    7
                } else {
                    5
                };
                if !(2..=max_access_points).contains(&path.points.len())
                    || (!path.surface_y.is_empty()
                        && (path.surface_y.len() != path.points.len()
                            || path.surface_y.iter().any(|height| {
                                !height.is_finite()
                                    || (*height < 0.0 && *height != -1.0)
                                    || *height > 16.0
                            })))
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
            // Master-plan lots tile blocks and share lot lines; only real
            // overlap beyond float noise is invalid.
            let party_wall = self.masterplan_active();
            let shrink = |rect: geometry::OrientedRect| {
                if party_wall {
                    geometry::OrientedRect {
                        width: rect.width - 0.1,
                        depth: rect.depth - 0.1,
                        ..rect
                    }
                } else {
                    rect
                }
            };
            for other in self.state.parcels.iter().skip(i + 1) {
                if party_wall && ((other.x - p.x).abs() > 60.0 || (other.z - p.z).abs() > 60.0) {
                    continue;
                }
                if geometry::rects_overlap(&shrink(footprint), &shrink(parcel_envelope(other)), 0.0)
                {
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
            for road in self
                .state
                .roads
                .iter()
                .filter(|r| r.stage == "open" && r.kind != "expressway")
            {
                let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                    continue;
                };
                let nearest_envelope_distance = road_envelopes
                    .iter()
                    .map(|envelope| geometry::rect_segment_distance(envelope, a.x, a.z, b.x, b.z))
                    .fold(f32::INFINITY, f32::min);
                // Lanes are 4 m shared surfaces; master-plan lots start at
                // 2.3 m, retaining a small margin over this runtime envelope.
                let road_clearance = match road.kind.as_str() {
                    "lane" => 2.2,
                    "local" => f32::min(road_clearance, 3.4),
                    _ => road_clearance,
                };
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
                let rear_extension_reserved = self
                    .state
                    .pending_rear_extensions
                    .iter()
                    .any(|plan| plan.building_id == building_id && plan.parcel_id == p.id);
                if building.stage == "open" && !rear_extension_reserved {
                    return Err(format!("parcel {} reserves an open building", p.id));
                }
                if !self.state.projects.iter().any(|project| {
                    project.kind == "building"
                        && project.parcel_id == Some(p.id)
                        && project.progress < 1.0
                }) && !self.state.pending_rear_extensions.iter().any(|plan| {
                    plan.building_id == building_id
                        && plan.parcel_id == p.id
                        && self.state.projects.iter().any(|project| {
                            project.kind == "extension"
                                && project.id == plan.project_id
                                && project.progress < 1.0
                        })
                }) && !self
                    .state
                    .height_redevelopment
                    .as_ref()
                    .is_some_and(|plan| {
                        plan.building_id == building_id
                            && plan.parcel_id == p.id
                            && self.state.projects.iter().any(|project| {
                                project.id == plan.project_id
                                    && project.kind == "redevelopment"
                                    && project.progress < 1.0
                            })
                    })
                {
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
        /// Legacy incremental planner, kept for comparison fixtures only.
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
mod tests;
