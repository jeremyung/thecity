use super::*;

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
    let mut engine = Engine::new_legacy(982);
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

    let engine = Engine::new_legacy(983);
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
    let mut engine = Engine::new_legacy(984);
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
    let engine = Engine::new_legacy(985);
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
            let mut engine = Engine::new_legacy(seed);
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
                .filter(|road| road.stage == Stage::Open && road.connected)
                .collect();
            let envelope_area: f64 = engine
                .state
                .buildings
                .iter()
                .filter(|building| building.stage == Stage::Open)
                .map(|building| (building.width * building.depth) as f64)
                .sum();
            let floor_area: f64 = engine
                .state
                .buildings
                .iter()
                .filter(|building| building.stage == Stage::Open)
                .map(|building| (building.width * building.depth * building.floors as f32) as f64)
                .sum();
            let road_length: f64 = open_roads
                .iter()
                .filter(|road| !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge))
                .map(|road| road.length as f64)
                .sum();
            eprintln!(
                "district-policy seed={seed} enabled={enabled} buildings={} connected_open_roads={} road_length={road_length:.1}m envelope_proxy={envelope_area:.1} floor_proxy={floor_area:.1} comparisons={} access_wins={} frontier_wins={} elapsed={:.2}s",
                engine.state.buildings.iter().filter(|b| b.stage == Stage::Open).count(),
                open_roads.len(),
                engine.experiments.district_policy_comparisons,
                engine.experiments.district_policy_access_wins,
                engine.experiments.district_policy_frontier_wins,
                started.elapsed().as_secs_f64(),
            );
            if enabled {
                assert!(engine.experiments.district_policy_comparisons > 0);
                assert!(
                    engine.experiments.district_policy_access_wins
                        + engine.experiments.district_policy_frontier_wins
                        > 0
                );
                assert_eq!(
                    engine.experiments.district_policy_diagnostics.comparisons,
                    engine.experiments.district_policy_comparisons
                );
                let diag = &engine.experiments.district_policy_diagnostics;
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
                assert_eq!(engine.experiments.district_policy_comparisons, 0);
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
