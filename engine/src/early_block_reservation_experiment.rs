use crate::*;
use serde_json::json;

fn developed_site_with_one_open_lot() -> Engine {
    let mut engine = Engine::new_legacy(41);
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
    let feeder = engine.add_node(320.0, 320.0, "junction");
    let anchor = engine.add_node(250.0, 250.0, "junction");
    let branch = engine.add_node(390.0, 350.0, "junction");
    engine.add_road(gateway, feeder, "local", "open", 1.0);
    engine.add_road(feeder, anchor, "local", "open", 1.0);
    engine.add_road(gateway, branch, "local", "open", 1.0);
    engine.recompute_connectivity();

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
    // This existing usable parcel lets the normal infill/building path proceed
    // after the experiment reserves a block.
    engine.add_parcel(100.0, 150.0, 8.0, 8.0, anchor, 0.8, false);
    engine
}

#[test]
fn early_reservation_keeps_block_safe_while_existing_building_finishes() {
    let mut engine = developed_site_with_one_open_lot();
    engine.state.tick = 79;
    engine.set_development_rate(60);
    let existing_building_parcel = engine.state.parcels.last().unwrap().id;

    engine.step(1);
    assert_eq!(engine.state.tick, 80);
    assert_eq!(engine.early_block_reservation_attempts, 1);
    assert_eq!(engine.early_block_reservation_admissions, 1);
    assert_eq!(engine.early_block_reservation_noops, 0);
    let plan = engine.state.neighborhood_plan.as_ref().unwrap();
    assert_eq!(
        plan.next_segment, 0,
        "reservation must not start road work immediately"
    );
    assert!(engine
        .state
        .projects
        .iter()
        .any(|project| project.kind == "building"));
    assert!(!engine
        .state
        .projects
        .iter()
        .any(|project| project.kind == "road"));
    engine.validate().unwrap();

    // Save at the actual reservation tick. Restoration keeps the pending
    // plan, and the next normal evaluation follows the same 80-tick cadence.
    let save = engine.save_json();
    let mut resumed = Engine::restore_json(&save).expect("reserved plan restores canonically");
    assert_eq!(resumed.save_json(), save);
    engine.step(2_000);
    resumed.step(2_000);
    assert_eq!(engine.save_json(), resumed.save_json());
    assert!(engine.early_block_reservation_duration_ticks > 0);
    assert!(engine.state.neighborhood_plan.is_some());
    assert!(
        engine.state.buildings.iter().any(|building| {
            building.parcel_id == existing_building_parcel && building.stage == "open"
        }),
        "the pre-existing building project should finish during reservation"
    );
    engine.validate().unwrap();
}

#[test]
fn early_reservation_records_noop_when_existing_plan_guards_reject_candidate() {
    let mut engine = developed_site_with_one_open_lot();
    engine.state.tick = 79;
    engine.state.demand = 0.4;
    engine.step(1);

    assert_eq!(engine.state.tick, 80);
    assert_eq!(engine.early_block_reservation_attempts, 1);
    assert_eq!(engine.early_block_reservation_admissions, 0);
    assert_eq!(engine.early_block_reservation_noops, 1);
    assert!(engine.state.neighborhood_plan.is_none());
    assert_eq!(
        engine
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "road")
            .count(),
        0
    );
}

fn run_matched_seeds(seeds: &[u32], report_file: &str) {
    let mut rows = Vec::new();
    let validation_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("validation");

    for &seed in seeds {
        for enabled in [false, true] {
            let started = std::time::Instant::now();
            let mut engine = Engine::new_legacy(seed);
            engine.set_development_rate(16);
            engine.district_policy_experiment = false;
            engine.early_block_reservation_disable_override = !enabled;
            for _ in 0..121 {
                engine.step(2_000);
            }
            assert_eq!(engine.state.tick, 242_000);
            engine
                .validate()
                .expect("experiment result preserves invariants");
            assert!(engine.state.nodes.iter().all(|node| {
                node.x.is_finite()
                    && node.z.is_finite()
                    && (MIN..=MAX).contains(&node.x)
                    && (MIN..=MAX).contains(&node.z)
            }));
            assert!(engine.state.buildings.len() <= MAX_BUILDINGS);
            assert!(engine.state.parcels.len() <= MAX_PARCELS);
            assert!(engine.state.roads.len() + engine.state.projects.len() <= MAX_SEGMENTS);
            assert!(
                engine
                    .state
                    .projects
                    .iter()
                    .filter(|project| project.progress < 1.0)
                    .count()
                    <= MAX_PROJECTS
            );
            let mut recomputed = Engine::restore_json(&engine.save_json()).unwrap();
            recomputed.recompute_connectivity();
            assert_eq!(
                engine
                    .state
                    .roads
                    .iter()
                    .map(|road| (road.id, road.connected))
                    .collect::<Vec<_>>(),
                recomputed
                    .state
                    .roads
                    .iter()
                    .map(|road| (road.id, road.connected))
                    .collect::<Vec<_>>(),
                "saved road connectivity matches the authoritative graph"
            );

            let open: Vec<_> = engine
                .state
                .buildings
                .iter()
                .filter(|building| building.stage == "open")
                .collect();
            let housing: Vec<_> = open.iter().copied().filter(|b| is_housing(b)).collect();
            let commerce: Vec<_> = open.iter().copied().filter(|b| is_commercial(b)).collect();
            let ordinary_roads: Vec<_> = engine
                .state
                .roads
                .iter()
                .filter(|road| {
                    road.stage == "open"
                        && road.connected
                        && !matches!(road.kind.as_str(), "rail" | "bridge")
                })
                .collect();
            let summary = json!({
                "seed": seed,
                "enabled": enabled,
                "tick": engine.state.tick,
                "development_rate": 16,
                "buildings_open": open.len(),
                "buildings_total": engine.state.buildings.len(),
                "housing_occupancy": housing.iter().map(|b| b.occupancy as u64).sum::<u64>(),
                "housing_capacity": housing.iter().map(|b| b.capacity as u64).sum::<u64>(),
                "commercial_open": commerce.len(),
                "commercial_active": commerce.iter().filter(|b| b.business_active).count(),
                "commercial_active_capacity": commerce.iter().filter(|b| b.business_active).map(|b| b.capacity as u64).sum::<u64>(),
                "nominal_building_envelope_m2": open.iter().map(|b| (b.width * b.depth) as f64).sum::<f64>(),
                "floor_area_proxy_m2": open.iter().map(|b| (b.width * b.depth * b.floors as f32) as f64).sum::<f64>(),
                "ordinary_road_records": ordinary_roads.len(),
                "ordinary_road_length_m": ordinary_roads.iter().map(|r| r.length as f64).sum::<f64>(),
                "parcels": engine.state.parcels.len(),
                "reservation_attempts": engine.early_block_reservation_attempts,
                "reservation_admissions": engine.early_block_reservation_admissions,
                "reservation_noops": engine.early_block_reservation_noops,
                "reservation_duration_ticks": engine.early_block_reservation_duration_ticks,
                "planner_work_cap": {
                    "evaluation_cadence_ticks": 80,
                    "anchors": MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION,
                    "reused_edges": MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION,
                    "shared_candidates": development::MAX_SHARED_BLOCK_CANDIDATES_PER_EVALUATION,
                    "full_candidates": development::MAX_FULL_BLOCK_CANDIDATES_PER_EVALUATION,
                    "spine_candidates": development::MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION
                },
                "plan_pending_at_capture": engine.state.neighborhood_plan.is_some(),
                "pending_plan_next_segment": engine.state.neighborhood_plan.as_ref().map(|p| p.next_segment),
                "elapsed_seconds": started.elapsed().as_secs_f64()
            });
            eprintln!("early-block-reservation {summary}");

            let tag = format!("seed-{seed}-enabled-{enabled}");
            let save = engine.save_json();
            std::fs::write(
                validation_dir.join(format!("early-block-reservation-{tag}-save.json")),
                &save,
            )
            .expect("write native save");
            std::fs::write(
                validation_dir.join(format!("early-block-reservation-{tag}-snapshot.json")),
                engine.snapshot_json(),
            )
            .expect("write native snapshot");

            let mut resumed = Engine::restore_json(&save).expect("canonical save restores");
            resumed.early_block_reservation_disable_override = !enabled;
            resumed.district_policy_experiment = false;
            assert_eq!(resumed.save_json(), save);
            engine.step(2_000);
            resumed.step(2_000);
            assert_eq!(engine.save_json(), resumed.save_json());
            rows.push(summary);
        }
    }

    std::fs::write(
        validation_dir.join(report_file),
        serde_json::to_vec_pretty(&json!({
            "experiment": "reserve normal coordinated-block candidate before global usable-infill/build-project return; construction still follows normal scheduler",
            "tick": 242_000,
            "rate": 16,
            "seeds": seeds,
            "baseline_and_candidate_same_source": true,
            "production_default_unchanged": true,
            "rows": rows
        }))
        .unwrap(),
    )
    .expect("write paired experiment report");
}

#[test]
#[ignore = "matched-seed early coordinated-block reservation experiment"]
fn early_block_reservation_matched_seeds() {
    run_matched_seeds(&[0, 87_231], "early-block-reservation-experiment.json");
}

#[test]
#[ignore = "additional matched seeds for early coordinated-block reservation"]
fn early_block_reservation_additional_seeds() {
    run_matched_seeds(
        &[7, 41, 2026],
        "early-block-reservation-additional-seeds.json",
    );
}
