use crate::*;
use serde_json::json;

#[test]
#[ignore = "matched-seed early-access scheduling experiment; run explicitly for calibration"]
fn early_access_scheduling_matched_seeds() {
    let mut rows = Vec::new();
    let validation_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("validation");

    for seed in [0_u32, 87_231] {
        for enabled in [false, true] {
            let started = std::time::Instant::now();
            let mut engine = Engine::new_legacy(seed);
            engine.set_development_rate(16);
            engine.district_policy_experiment = false;
            engine.experiments.early_access_experiment = enabled;
            for _ in 0..121 {
                engine.step(2_000);
            }
            assert_eq!(engine.state.tick, 242_000);
            engine
                .validate()
                .expect("experiment result preserves invariants");

            let open_buildings: Vec<_> = engine
                .state
                .buildings
                .iter()
                .filter(|building| building.stage == Stage::Open)
                .collect();
            let housing: Vec<_> = open_buildings
                .iter()
                .copied()
                .filter(|building| is_housing(building))
                .collect();
            let commerce: Vec<_> = open_buildings
                .iter()
                .copied()
                .filter(|building| is_commercial(building))
                .collect();
            let connected_roads: Vec<_> = engine
                .state
                .roads
                .iter()
                .filter(|road| road.stage == Stage::Open && road.connected)
                .collect();
            let ordinary_roads: Vec<_> = connected_roads
                .iter()
                .filter(|road| !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge))
                .collect();
            let envelope_proxy_m2: f64 = open_buildings
                .iter()
                .map(|building| (building.width * building.depth) as f64)
                .sum();
            let floor_area_proxy_m2: f64 = open_buildings
                .iter()
                .map(|building| (building.width * building.depth * building.floors as f32) as f64)
                .sum();
            let road_length_m: f64 = ordinary_roads.iter().map(|road| road.length as f64).sum();
            let summary = json!({
                "seed": seed,
                "development_rate": 16,
                "tick": engine.state.tick,
                "early_access_enabled": enabled,
                "district_policy_enabled": false,
                "buildings_open": open_buildings.len(),
                "buildings_total": engine.state.buildings.len(),
                "housing_open": housing.len(),
                "housing_occupancy": housing.iter().map(|building| building.occupancy as u64).sum::<u64>(),
                "housing_capacity": housing.iter().map(|building| building.capacity as u64).sum::<u64>(),
                "commercial_open": commerce.len(),
                "commercial_active": commerce.iter().filter(|building| building.business_active).count(),
                "commercial_active_capacity": commerce.iter().filter(|building| building.business_active).map(|building| building.capacity as u64).sum::<u64>(),
                "envelope_area_proxy_m2": envelope_proxy_m2,
                "floor_area_proxy_m2": floor_area_proxy_m2,
                "parcels": engine.state.parcels.len(),
                "connected_open_roads": connected_roads.len(),
                "ordinary_road_length_m": road_length_m,
                "ordinary_road_records": ordinary_roads.len(),
                "total_road_records": engine.state.roads.len(),
                "early_access_attempts": engine.experiments.early_access_attempts,
                "early_access_admissions": engine.experiments.early_access_admissions,
                "interior_access_completions": engine.experiments.interior_access_completions,
                "pending_access_lots": engine.state.pending_interior_access.as_ref().map_or(0, |plan| plan.selected_lots.len()),
                "pending_access_project": engine.state.pending_interior_access.as_ref().map(|plan| plan.project_id),
                "unbuilt_parcels": engine.state.parcels.iter().filter(|parcel| !engine.state.buildings.iter().any(|building| building.parcel_id == parcel.id)).count(),
                "demand": engine.state.demand,
                "population": engine.state.population,
                "elapsed_seconds": started.elapsed().as_secs_f64(),
            });
            eprintln!("early-access-experiment {summary}");

            let save = engine.save_json();
            let tag = format!("seed-{seed}-enabled-{enabled}");
            std::fs::write(
                validation_dir.join(format!("early-access-{tag}-save.json")),
                &save,
            )
            .expect("write native save");
            std::fs::write(
                validation_dir.join(format!("early-access-{tag}-snapshot.json")),
                engine.snapshot_json(),
            )
            .expect("write native snapshot");

            // The policy is intentionally non-persisted, so restore and replay
            // both continuations with the same explicit experiment setting.
            let mut resumed = Engine::restore_json(&save).expect("canonical save restores");
            resumed.experiments.early_access_experiment = enabled;
            resumed.district_policy_experiment = false;
            assert_eq!(resumed.save_json(), save);
            engine.step(2_000);
            resumed.step(2_000);
            assert_eq!(engine.save_json(), resumed.save_json());
            rows.push(summary);
        }
    }

    std::fs::write(
        validation_dir.join("early-access-scheduling-experiment.json"),
        serde_json::to_vec_pretty(&json!({
            "experiment": "offer a normal reserved interior-access construction project before progressive frontage infill",
            "horizon_ticks": 242_000,
            "rate": 16,
            "seeds": [0, 87_231],
            "baseline_and_candidate_same_source": true,
            "bridgehead_bootstrap_preserved": true,
            "regular_usable_infill_priority_preserved": true,
            "rows": rows
        }))
        .unwrap(),
    )
    .expect("write paired experiment report");
}
