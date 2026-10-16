use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn signature(plan: &NeighborhoodPlan) -> Option<[u32; 8]> {
    plan.spine.as_ref()?;
    Some(std::array::from_fn(|index| {
        let point = plan.spine.as_ref().unwrap().corners[index / 2];
        if index % 2 == 0 {
            point.x.to_bits()
        } else {
            point.z.to_bits()
        }
    }))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    })
}

#[test]
fn sites_per_road_experiment_comparator_has_area_and_stable_tiebreaks() {
    let make = |sites, area, road, key| development::NeighborhoodBenefitScore {
        usable_sites: sites,
        nominal_area_cm2: area,
        new_road_length_cm: road,
        new_road_count: 4,
        stable_key: key,
    };
    let more_sites_per_road = make(8, 100, 200, [9, 0, 0]);
    let less_sites_per_road = make(6, 10_000, 200, [0, 0, 0]);
    assert!(development::neighborhood_sites_per_road_is_better(
        more_sites_per_road,
        less_sites_per_road,
    ));

    let larger_area_tie = make(4, 1_000, 200, [9, 0, 0]);
    let smaller_area_tie = make(2, 250, 100, [0, 0, 0]);
    assert!(development::neighborhood_sites_per_road_is_better(
        larger_area_tie,
        smaller_area_tie,
    ));
    let stable_first = make(6, 1_200, 200, [1, 0, 0]);
    let stable_later = make(6, 1_200, 200, [2, 0, 0]);
    assert!(development::neighborhood_sites_per_road_is_better(
        stable_first,
        stable_later,
    ));
}

#[test]
fn ranking_selector_is_scoped_and_defaults_to_nominal_area() {
    let high_site_count = development::NeighborhoodBenefitScore {
        usable_sites: 8,
        nominal_area_cm2: 1_000,
        new_road_length_cm: 200,
        new_road_count: 4,
        stable_key: [1, 0, 0],
    };
    let high_area = development::NeighborhoodBenefitScore {
        usable_sites: 4,
        nominal_area_cm2: 4_000,
        new_road_length_cm: 200,
        new_road_count: 4,
        stable_key: [2, 0, 0],
    };
    assert!(neighborhood_candidate_is_better(high_area, high_site_count));
    {
        let _selector = SpineRankingExperimentGuard::by_sites_per_road();
        assert!(neighborhood_candidate_is_better(high_site_count, high_area));
    }
    assert!(neighborhood_candidate_is_better(high_area, high_site_count));
}

/// Matched-run calibration harness. The selector is thread-local and RAII
/// scoped, so production default behavior and serialized state stay unchanged.
#[test]
#[ignore = "controlled matched-seed comparison of two neighborhood ranking objectives"]
fn compare_spine_ranking_by_area_vs_sites_per_road() {
    let validation_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("validation");
    let mut paired_results = Vec::new();
    for seed in [0_u32, 87_231] {
        for by_sites in [false, true] {
            let started = std::time::Instant::now();
            let _policy = if by_sites {
                Some(SpineRankingExperimentGuard::by_sites_per_road())
            } else {
                None
            };
            let mut engine = Engine::new_legacy(seed);
            engine.set_development_rate(16);
            let mut selected_spines = BTreeSet::new();
            let mut seen_spines = BTreeMap::<[u32; 8], Option<u32>>::new();
            for _ in 0..12_100 {
                engine.step(20);
                if let Some(plan) = engine.state.neighborhood_plan.as_ref() {
                    if let Some(key) = signature(plan) {
                        selected_spines.insert(key);
                        let spine_road_id = match plan.spine.as_ref().unwrap().stage {
                            spine_plan::SpineStage::UnderConstruction { spine_road_id, .. } => {
                                Some(spine_road_id)
                            }
                            _ => None,
                        };
                        seen_spines.entry(key).or_insert(spine_road_id);
                        if let Some(id) = spine_road_id {
                            seen_spines.insert(key, Some(id));
                        }
                    }
                }
            }
            assert_eq!(engine.state.tick, 242_000);
            engine.validate().unwrap();
            assert!(engine.state.roads.len() <= MAX_SEGMENTS);
            assert!(engine.state.parcels.len() <= MAX_PARCELS);
            assert!(engine.state.buildings.len() <= MAX_BUILDINGS);

            let open: Vec<_> = engine
                .state
                .buildings
                .iter()
                .filter(|building| building.stage == "open")
                .collect();
            let housing: Vec<_> = open.iter().copied().filter(|b| is_housing(b)).collect();
            let commerce: Vec<_> = open.iter().copied().filter(|b| is_commercial(b)).collect();
            let envelope_area_m2: f64 = open.iter().map(|b| (b.width * b.depth) as f64).sum();
            let floor_area_m2: f64 = open
                .iter()
                .map(|b| (b.width * b.depth * b.floors as f32) as f64)
                .sum();
            let road_length_m: f64 = engine
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open" && road.connected)
                .map(|road| road.length as f64)
                .sum();
            let ordinary_road_length_m: f64 = engine
                .state
                .roads
                .iter()
                .filter(|road| {
                    road.stage == "open"
                        && road.connected
                        && !matches!(road.kind.as_str(), "rail" | "bridge")
                })
                .map(|road| road.length as f64)
                .sum();
            let rail_length_m: f64 = engine
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open" && road.connected && road.kind == "rail")
                .map(|road| road.length as f64)
                .sum();
            let bridge_length_m: f64 = engine
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open" && road.connected && road.kind == "bridge")
                .map(|road| road.length as f64)
                .sum();
            let housing_capacity: u64 = housing.iter().map(|b| b.capacity as u64).sum();
            let housing_occupied: u64 = housing.iter().map(|b| b.occupancy as u64).sum();
            let active_commerce = commerce.iter().filter(|b| b.business_active).count();
            let commercial_capacity: u64 = commerce.iter().map(|b| b.capacity as u64).sum();
            let commercial_occupancy: u64 = commerce.iter().map(|b| b.occupancy as u64).sum();
            let active_commercial_capacity: u64 = commerce
                .iter()
                .filter(|b| b.business_active)
                .map(|b| b.capacity as u64)
                .sum();
            let active_spine_plan_at_capture = engine
                .state
                .neighborhood_plan
                .as_ref()
                .is_some_and(|p| p.spine.is_some());
            let spine_road_ids: BTreeSet<u32> = seen_spines.values().flatten().copied().collect();
            let completed_spine_road_ids: BTreeSet<u32> = spine_road_ids
                .into_iter()
                .filter(|id| {
                    engine
                        .state
                        .roads
                        .iter()
                        .any(|road| road.id == *id && road.stage == "open" && road.connected)
                })
                .collect();
            let completed_spine_count = completed_spine_road_ids.len();
            assert!(completed_spine_count <= selected_spines.len());
            let open_building_count = open.len();
            let housing_building_count = housing.len();
            let commercial_building_count = commerce.len();
            drop(housing);
            drop(commerce);
            drop(open);
            let saved = engine.save_json();
            let save_hash = fnv1a64(saved.as_bytes());
            let snapshot = engine.snapshot_json();
            let mut restored = Engine::restore_json(&saved).unwrap();
            engine.step(2_000);
            restored.step(2_000);
            assert_eq!(engine.save_json(), restored.save_json());
            let replay_hash = fnv1a64(engine.save_json().as_bytes());
            std::fs::write(
                validation_dir.join(format!(
                    "spine-ranking-seed-{seed}-by-sites-{by_sites}.json"
                )),
                &saved,
            )
            .unwrap();
            std::fs::write(
                validation_dir.join(format!(
                    "spine-ranking-seed-{seed}-by-sites-{by_sites}-snapshot.json"
                )),
                snapshot,
            )
            .unwrap();

            let summary = serde_json::json!({
                "seed": seed,
                "development_rate": 16,
                "tick": 242_000,
                "ranking": if by_sites { "usable_sites_per_new_road_m" } else { "nominal_envelope_area_per_new_road_m" },
                "open_buildings": open_building_count,
                "housing": { "open_buildings": housing_building_count, "occupied": housing_occupied, "capacity": housing_capacity },
                "commercial": { "open_buildings": commercial_building_count, "active_businesses": active_commerce, "occupancy_sum": commercial_occupancy, "capacity": commercial_capacity, "active_capacity": active_commercial_capacity },
                "nominal_building_envelope_area_m2": envelope_area_m2,
                "nominal_building_floor_area_m2": floor_area_m2,
                "connected_open_road_length_m": road_length_m,
                "connected_open_ordinary_length_m": ordinary_road_length_m,
                "connected_open_rail_length_m": rail_length_m,
                "connected_open_bridge_length_m": bridge_length_m,
                "spine_sampling_interval_ticks": 20,
                "spine_plans_selected_observed_lower_bound": selected_spines.len(),
                "spine_roads_observed_under_construction_lower_bound": seen_spines.values().filter(|id| id.is_some()).count(),
                "spine_roads_verified_open_connected_lower_bound": completed_spine_count,
                "active_spine_plan_at_capture": active_spine_plan_at_capture,
                "save_fnv1a64": format!("{save_hash:016x}"),
                "replay_fnv1a64": format!("{replay_hash:016x}"),
                "save_resume_equivalent": true,
                "elapsed_seconds": started.elapsed().as_secs_f64(),
            });
            paired_results.push(summary.clone());
            std::fs::write(
                validation_dir.join(format!(
                    "spine-ranking-seed-{seed}-by-sites-{by_sites}.summary.json"
                )),
                serde_json::to_vec_pretty(&summary).unwrap(),
            )
            .unwrap();
            eprintln!("spine ranking result: {summary}");
        }
    }
    std::fs::write(
        validation_dir.join("spine-ranking-experiment.json"),
        serde_json::to_vec_pretty(&paired_results).unwrap(),
    )
    .unwrap();
}
