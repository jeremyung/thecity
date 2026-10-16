//! Bounded, test-only trace of mature neighborhood planner feasibility.
//!
//! This diagnostic never changes the fixture. It separately advances a
//! restored planner clone through the production anchor scan and a second
//! restored clone through ordinary simulation ticks.

use crate::{site_active, Engine, NeighborhoodPlan, Road};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const TRACE_TICKS: u32 = 2_000;

#[derive(Serialize)]
struct GateInputs {
    plan_absent: bool,
    pending_interior_access_absent: bool,
    demand: f32,
    demand_pass: bool,
    active_buildings: usize,
    active_buildings_pass: bool,
    population: u32,
    population_pass: bool,
    roads_plus_projects: usize,
    segment_capacity_pass: bool,
    active_projects: usize,
    active_road_projects: usize,
    active_building_projects: usize,
    node_count: usize,
    connected_ordinary_roads: usize,
    local_qualified_anchors: usize,
}

#[derive(Serialize)]
struct PlannerCall {
    call: usize,
    cursor_start: usize,
    cursor_end: usize,
    cursor_anchors_examined: usize,
    production_method_accepted: bool,
    accepted_plan: Option<PlanSummary>,
    geometry_rejection_capture: crate::neighborhood_admission::RejectionCapture,
}

#[derive(Serialize)]
struct PlanSummary {
    anchor_node: u32,
    segment_count: usize,
    next_segment: usize,
    coordinated_lots: bool,
    selected_lot_count: usize,
    has_spine_recipe: bool,
}

#[derive(Serialize)]
struct NormalTickTrace {
    requested_ticks: u32,
    start_tick: u64,
    end_tick: u64,
    start_buildings: usize,
    end_buildings: usize,
    start_parcels: usize,
    end_parcels: usize,
    start_roads: usize,
    end_roads: usize,
    start_projects: usize,
    end_projects: usize,
    start_anchor_cursor: usize,
    end_anchor_cursor: usize,
    start_early_access_attempts: u32,
    end_early_access_attempts: u32,
    start_early_block_reservation_attempts: u32,
    end_early_block_reservation_attempts: u32,
    samples_at_80_tick_boundaries: Vec<NormalTickSample>,
    final_plan: Option<PlanSummary>,
    planner_frontdoor_eligible_at_end: bool,
    planner_frontdoor_blockers_at_end: Vec<&'static str>,
}

#[derive(Serialize)]
struct NormalTickSample {
    tick: u64,
    anchor_cursor: usize,
    anchor_cursor_changed: bool,
    early_access_attempts: u32,
    early_block_reservation_attempts: u32,
    plan: Option<PlanSummary>,
    buildings: usize,
    parcels: usize,
    roads: usize,
    active_projects: usize,
    active_road_projects: usize,
    active_building_projects: usize,
    planner_frontdoor_eligible: bool,
    planner_frontdoor_blockers: Vec<&'static str>,
}

#[derive(Serialize)]
struct Report {
    input_path: String,
    seed: u32,
    source_tick: u64,
    source_demand: f32,
    source_save_unchanged: bool,
    gate_inputs: GateInputs,
    anchor_attempt_cap: usize,
    shared_candidate_cap: usize,
    full_candidate_cap: usize,
    spine_candidate_cap: usize,
    reuse_edge_cap: usize,
    planner_calls: Vec<PlannerCall>,
    rejection_histogram: BTreeMap<&'static str, usize>,
    rejection_witnesses: Vec<crate::neighborhood_admission::RejectionWitness>,
    scanned_anchor_count: usize,
    full_anchor_cursor_coverage: bool,
    production_plan_found: bool,
    planner_outcome: &'static str,
    normal_ticks: NormalTickTrace,
    limitations: &'static str,
}

fn fixture_path() -> PathBuf {
    if let Some(path) = std::env::var_os("CITY_TRACE_SAVE") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine crate has a repository parent")
        .join(".tools/narrow-frontage-review/final-save.json")
}

fn plan_summary(plan: Option<&NeighborhoodPlan>) -> Option<PlanSummary> {
    plan.map(|plan| PlanSummary {
        anchor_node: plan.anchor_node,
        segment_count: plan.segments.len(),
        next_segment: plan.next_segment,
        coordinated_lots: plan.coordinated_lots,
        selected_lot_count: plan.selected_lots.as_ref().map_or(0, Vec::len),
        has_spine_recipe: plan.spine.is_some(),
    })
}

fn ordinary_connected_roads(engine: &Engine) -> Vec<Road> {
    engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && !matches!(road.kind.as_str(), "rail" | "bridge")
        })
        .cloned()
        .collect()
}

fn gate_inputs(engine: &Engine) -> GateInputs {
    let active_buildings = engine
        .state
        .buildings
        .iter()
        .filter(|building| site_active(building))
        .count();
    let roads_plus_projects = engine.state.roads.len() + engine.state.projects.len() + 3;
    let active_projects = active_projects(engine);
    let active_road_projects = engine
        .state
        .projects
        .iter()
        .filter(|project| project.kind == "road" && project.progress < 1.0)
        .count();
    let active_building_projects = engine
        .state
        .projects
        .iter()
        .filter(|project| project.kind == "building" && project.progress < 1.0)
        .count();
    let ordinary_roads = ordinary_connected_roads(engine);
    let connected_nodes: BTreeSet<u32> = ordinary_roads
        .iter()
        .flat_map(|road| [road.from, road.to])
        .collect();
    let local_qualified_anchors = engine
        .state
        .nodes
        .iter()
        .filter(|anchor| {
            connected_nodes.contains(&anchor.id)
                && engine.ordinary_road_degree(anchor.id) > 0
                && engine
                    .state
                    .buildings
                    .iter()
                    .filter(|building| {
                        site_active(building)
                            && (building.x - anchor.x).hypot(building.z - anchor.z) <= 112.0
                    })
                    .count()
                    >= 8
                && engine
                    .state
                    .buildings
                    .iter()
                    .filter(|building| {
                        site_active(building)
                            && (building.x - anchor.x).hypot(building.z - anchor.z) <= 112.0
                    })
                    .filter(|building| site_active(building))
                    .count()
                    >= 4
        })
        .count();
    GateInputs {
        plan_absent: engine.state.neighborhood_plan.is_none(),
        pending_interior_access_absent: engine.state.pending_interior_access.is_none(),
        demand: engine.state.demand,
        demand_pass: engine.state.demand >= 1.1,
        active_buildings,
        active_buildings_pass: active_buildings >= 10,
        population: engine.state.population,
        population_pass: engine.state.population >= 24,
        roads_plus_projects,
        segment_capacity_pass: roads_plus_projects <= crate::MAX_SEGMENTS,
        active_projects,
        active_road_projects,
        active_building_projects,
        node_count: engine.state.nodes.len(),
        connected_ordinary_roads: ordinary_roads.len(),
        local_qualified_anchors,
    }
}

fn active_projects(engine: &Engine) -> usize {
    engine
        .state
        .projects
        .iter()
        .filter(|project| project.progress < 1.0)
        .count()
}

fn usable_infill(engine: &Engine) -> bool {
    engine.state.parcels.iter().any(|parcel| {
        !parcel.protected
            && parcel.reservation.is_none()
            && (engine.state.station_online || !engine.is_station_tower_lot(parcel))
            && engine.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
            && !engine
                .state
                .buildings
                .iter()
                .any(|building| building.parcel_id == parcel.id)
            && engine.node_connected(parcel.access_node)
    })
}

fn planner_frontdoor_blockers(engine: &Engine) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if engine.state.neighborhood_plan.is_some() {
        blockers.push("neighborhood_plan_already_active");
    }
    if engine.state.pending_interior_access.is_some() {
        blockers.push("pending_interior_access_active");
    }
    if engine.state.demand < 1.1 {
        blockers.push("demand_below_neighborhood_threshold");
    }
    if engine
        .state
        .buildings
        .iter()
        .filter(|building| site_active(building))
        .count()
        < 10
    {
        blockers.push("active_buildings_below_neighborhood_threshold");
    }
    if engine.state.population < 24 {
        blockers.push("population_below_neighborhood_threshold");
    }
    if engine.state.roads.len() + engine.state.projects.len() + 3 > crate::MAX_SEGMENTS {
        blockers.push("road_and_project_capacity_exhausted");
    }
    if engine.state.road_infill_due != 0 {
        blockers.push("road_infill_quota_nonzero");
    }
    if engine
        .state
        .projects
        .iter()
        .any(|project| project.kind == "road" && project.progress < 1.0)
    {
        blockers.push("active_road_project_exists");
    }
    if engine
        .state
        .projects
        .iter()
        .any(|project| project.kind == "building" && project.progress < 1.0)
    {
        blockers.push("active_building_project_exists");
    }
    if engine.neighborhood_block_has_unbuilt_frontage() {
        blockers.push("neighborhood_frontage_unbuilt");
    }
    if usable_infill(engine) {
        blockers.push("usable_infill_exists");
    }
    if engine.state.tick % 80 != 0 {
        blockers.push("tick_not_on_80_tick_planner_phase");
    }
    blockers
}

fn normal_trace(source: &Engine) -> NormalTickTrace {
    let mut engine =
        Engine::restore_json(&source.save_json()).expect("normal-trace clone restores");
    let start_tick = engine.state.tick;
    let start_buildings = engine.state.buildings.len();
    let start_parcels = engine.state.parcels.len();
    let start_roads = engine.state.roads.len();
    let start_projects = engine.state.projects.len();
    let start_anchor_cursor = engine.state.neighborhood_anchor_cursor;
    let start_early_access_attempts = engine.experiments.early_access_attempts;
    let start_early_block_attempts = engine.experiments.early_block_reservation_attempts;
    let mut samples = Vec::new();
    for _ in 0..(TRACE_TICKS / 80) {
        let cursor_before = engine.state.neighborhood_anchor_cursor;
        engine.step(80);
        let sample_blockers = planner_frontdoor_blockers(&engine);
        samples.push(NormalTickSample {
            tick: engine.state.tick,
            anchor_cursor: engine.state.neighborhood_anchor_cursor,
            anchor_cursor_changed: cursor_before != engine.state.neighborhood_anchor_cursor,
            early_access_attempts: engine.experiments.early_access_attempts,
            early_block_reservation_attempts: engine.experiments.early_block_reservation_attempts,
            plan: plan_summary(engine.state.neighborhood_plan.as_ref()),
            buildings: engine.state.buildings.len(),
            parcels: engine.state.parcels.len(),
            roads: engine.state.roads.len(),
            active_projects: active_projects(&engine),
            active_road_projects: engine
                .state
                .projects
                .iter()
                .filter(|project| project.kind == "road" && project.progress < 1.0)
                .count(),
            active_building_projects: engine
                .state
                .projects
                .iter()
                .filter(|project| project.kind == "building" && project.progress < 1.0)
                .count(),
            planner_frontdoor_eligible: sample_blockers.is_empty(),
            planner_frontdoor_blockers: sample_blockers,
        });
    }
    let blockers = planner_frontdoor_blockers(&engine);
    NormalTickTrace {
        requested_ticks: TRACE_TICKS,
        start_tick,
        end_tick: engine.state.tick,
        start_buildings,
        end_buildings: engine.state.buildings.len(),
        start_parcels,
        end_parcels: engine.state.parcels.len(),
        start_roads,
        end_roads: engine.state.roads.len(),
        start_projects,
        end_projects: engine.state.projects.len(),
        start_anchor_cursor,
        end_anchor_cursor: engine.state.neighborhood_anchor_cursor,
        start_early_access_attempts,
        end_early_access_attempts: engine.experiments.early_access_attempts,
        start_early_block_reservation_attempts: start_early_block_attempts,
        end_early_block_reservation_attempts: engine.experiments.early_block_reservation_attempts,
        samples_at_80_tick_boundaries: samples,
        final_plan: plan_summary(engine.state.neighborhood_plan.as_ref()),
        planner_frontdoor_eligible_at_end: blockers.is_empty(),
        planner_frontdoor_blockers_at_end: blockers,
    }
}

fn build_report() -> Report {
    let input_path = fixture_path();
    let raw = std::fs::read_to_string(&input_path).expect("mature final save exists");
    let source_engine = Engine::restore_json(&raw).expect("mature final save restores");
    let source_before = source_engine.save_json();
    let mut engine = Engine::restore_json(&source_before).expect("planner clone restores");
    let source_tick = engine.state.tick;
    let source_demand = engine.state.demand;
    let gates = gate_inputs(&engine);
    let node_count = engine.state.nodes.len();
    // A call can stop early when a per-type candidate budget fills, so use a
    // hard node-count bound rather than assuming each call scans 24 anchors.
    let max_calls = node_count.saturating_add(1);
    let mut calls = Vec::new();
    let mut covered = BTreeSet::new();
    let mut rejection_histogram = BTreeMap::<&'static str, usize>::new();
    let mut rejection_witness_counts = BTreeMap::<&'static str, usize>::new();
    let mut rejection_witnesses = Vec::new();
    let mut accepted = false;

    if gates.plan_absent
        && gates.pending_interior_access_absent
        && gates.demand_pass
        && gates.active_buildings_pass
        && gates.population_pass
        && gates.segment_capacity_pass
        && node_count > 0
    {
        for call_index in 0..max_calls {
            let cursor_start = engine.state.neighborhood_anchor_cursor % node_count;
            let (accepted_this_call, mut geometry_rejection_capture) =
                crate::neighborhood_admission::capture_rejections(|| {
                    engine.try_create_neighborhood_plan()
                });
            for (reason, count) in &geometry_rejection_capture.reason_counts {
                *rejection_histogram.entry(*reason).or_default() += count;
            }
            for witness in &geometry_rejection_capture.witnesses {
                let count = rejection_witness_counts.entry(witness.reason).or_default();
                if *count < 8 {
                    rejection_witnesses.push(witness.clone());
                    *count += 1;
                }
            }
            // Keep per-window reason counts, while storing only the first
            // eight geometric witnesses per reason for the whole trace.
            geometry_rejection_capture.witnesses.clear();
            let cursor_end = engine.state.neighborhood_anchor_cursor % node_count;
            let cursor_delta = (cursor_end + node_count - cursor_start) % node_count;
            let examined = if cursor_delta == 0 && !accepted_this_call {
                node_count
            } else {
                cursor_delta
            };
            for offset in 0..examined {
                covered.insert((cursor_start + offset) % node_count);
            }
            let accepted_plan = plan_summary(engine.state.neighborhood_plan.as_ref());
            calls.push(PlannerCall {
                call: call_index + 1,
                cursor_start,
                cursor_end,
                cursor_anchors_examined: examined,
                production_method_accepted: accepted_this_call,
                accepted_plan,
                geometry_rejection_capture,
            });
            if accepted_this_call {
                accepted = true;
                engine
                    .validate()
                    .expect("accepted neighborhood plan validates");
                break;
            }
            if cursor_end == cursor_start || covered.len() == node_count {
                break;
            }
        }
    }

    let source_save_unchanged = source_demand == engine.state.demand
        && source_engine.save_json() == source_before
        && std::fs::read_to_string(&input_path).expect("fixture remains readable") == raw;
    assert!(
        source_save_unchanged,
        "trace must not mutate original fixture inputs"
    );
    assert_eq!(
        engine.state.demand, source_demand,
        "planner probes preserve demand"
    );
    assert!(calls.len() <= max_calls, "planner trace remains bounded");

    let planner_outcome = if !gates.plan_absent
        || !gates.pending_interior_access_absent
        || !gates.demand_pass
        || !gates.active_buildings_pass
        || !gates.population_pass
        || !gates.segment_capacity_pass
    {
        "blocked_by_top_level_planner_gate"
    } else if accepted {
        "production_geometry_candidate_accepted"
    } else if covered.len() == node_count {
        "all_anchor_cursor_windows_exhausted_without_feasible_candidate"
    } else {
        "bounded_scan_incomplete"
    };
    let trace = normal_trace(&Engine::restore_json(&source_before).expect("tick trace restores"));
    Report {
        input_path: input_path.display().to_string(),
        seed: engine.state.seed,
        source_tick,
        source_demand,
        source_save_unchanged,
        gate_inputs: gates,
        anchor_attempt_cap: crate::MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION,
        shared_candidate_cap: crate::development::MAX_SHARED_BLOCK_CANDIDATES_PER_EVALUATION,
        full_candidate_cap: crate::development::MAX_FULL_BLOCK_CANDIDATES_PER_EVALUATION,
        spine_candidate_cap: crate::development::MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION,
        reuse_edge_cap: crate::MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION,
        planner_calls: calls,
        rejection_histogram,
        rejection_witnesses,
        scanned_anchor_count: covered.len(),
        full_anchor_cursor_coverage: covered.len() == node_count,
        production_plan_found: accepted,
        planner_outcome,
        normal_ticks: trace,
        limitations: "The planner method is run against its unchanged production gates on a restored clone. A full no-plan sweep shows no candidate survived the production method's current bounded geometry checks for the sampled anchor set; it does not identify a specific primitive geometry rejection or prove all possible layouts infeasible.",
    }
}

#[test]
#[ignore = "bounded mature planner feasibility trace; no production mutation"]
fn mature_neighborhood_planner_trace() {
    let report = build_report();
    let json = serde_json::to_string_pretty(&report).expect("trace serializes");
    if let Ok(path) = std::env::var("MATURE_PLANNER_TRACE_REPORT") {
        std::fs::write(path, &json).expect("write mature planner trace report");
    }
    println!("{json}");
}
