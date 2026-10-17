//! Test-only counterfactual for parcel packing inside live coordinated blocks.
//!
//! This diagnostic never changes the simulated engine. For each first-seen
//! active coordinated, non-spine plan, it restores a disposable copy, removes
//! only that plan's active perimeter reservation, and tests alternative
//! subdivisions against all extant parcels, roads, pending projects, river,
//! protected land, bounds, and other reservations. It is an opportunity
//! measurement, not a legal retrofit or a demand/construction forecast.

use crate::{blocks, geometry, CollisionSegment, Engine, NeighborhoodPlan};
use serde::Serialize;
use std::collections::BTreeSet;

const CAPTURE_TICK: u64 = 242_000;
const SAMPLE_TICKS: u32 = 200;
const DEVELOPMENT_RATE: u32 = 16;
const MAX_UNIQUE_PLANS_PER_SEED: usize = 64;
const TARGET_FRONTAGES: [(f32, f32); 5] = [
    (11.0, 11.0),
    (14.0, 14.0),
    (18.0, 18.0),
    (11.0, 14.0),
    (11.0, 18.0),
];

#[derive(Serialize)]
struct SeedReport {
    seed: u32,
    capture_tick: u64,
    plans: Vec<PlanReport>,
    plan_cap_reached: bool,
    source_unchanged_at_each_observation: bool,
}

#[derive(Serialize)]
struct PlanReport {
    first_observed_tick: u64,
    anchor_node: u32,
    segments: Vec<(u32, u32)>,
    corners: [(f32, f32); 4],
    road_ids_present_at_observation: Vec<u32>,
    next_segment: usize,
    planned_perimeter_segments_checked: usize,
    excluded_parcel_count: usize,
    preserved_parcel_count: usize,
    preserved_collision_footprint_count: usize,
    variants: Vec<VariantReport>,
}

#[derive(Serialize)]
struct VariantReport {
    target_frontage_m: f32,
    opposite_target_frontage_m: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipe: Option<&'static str>,
    generated_lots: usize,
    accepted_lots: usize,
    rejected_lots: usize,
    accepted_nominal_area_m2: f64,
}

fn matching_plan_road_ids(engine: &Engine, plan: &NeighborhoodPlan) -> Vec<u32> {
    let mut ids: Vec<u32> = plan
        .segments
        .iter()
        .filter_map(|(from, to)| {
            engine
                .state
                .network
                .roads
                .iter()
                .find(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                })
                .map(|road| road.id)
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn evaluate_plan(engine: &Engine, observed_tick: u64, plan: &NeighborhoodPlan) -> PlanReport {
    let before = engine.save_json();
    let mut counterfactual = Engine::restore_json(&before)
        .expect("diagnostic copy of valid simulated engine should restore");
    let road_ids = matching_plan_road_ids(engine, plan);

    // A still-active plan has not published its coordinated lots. Do not
    // remove frontage parcels: a reused shared edge may contain pre-existing
    // lots that remain genuine obstacles. Clearing this plan alone exposes
    // its proposed block interior to the normal geometry helper.
    let same_plan = counterfactual
        .state
        .legacy
        .neighborhood_plan
        .as_ref()
        .is_some_and(|current| current.segments == plan.segments);
    assert!(
        same_plan,
        "restored diagnostic copy must retain the observed plan"
    );
    counterfactual.state.legacy.neighborhood_plan = None;
    let excluded_parcel_count = 0;

    let mut context = counterfactual.collision_context();
    // The remaining perimeter projects may not yet exist in the source save.
    // Include every proposed edge as pending so all candidate layouts face
    // the ordinary 7m construction-clearance check for the complete block.
    for (from, to) in &plan.segments {
        let (Some(a), Some(b)) = (counterfactual.node(*from), counterfactual.node(*to)) else {
            panic!("active neighborhood plan references existing corner nodes");
        };
        context.pending_roads.push(CollisionSegment {
            ax: a.x,
            az: a.z,
            bx: b.x,
            bz: b.z,
        });
    }
    let preserved_collision_footprint_count = context.parcel_footprints.len();
    let corners_vec = plan
        .segments
        .iter()
        .map(|(from, _)| {
            let node = counterfactual
                .node(*from)
                .expect("active neighborhood plan references existing corner nodes");
            blocks::BlockPoint {
                x: node.x,
                z: node.z,
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(corners_vec.len(), 4);
    let corners: [blocks::BlockPoint; 4] = corners_vec
        .try_into()
        .expect("coordinated plan has four ordered corners");

    let variants = TARGET_FRONTAGES
        .iter()
        .copied()
        .map(|(a, b)| (a, b, None))
        .chain([
            (7.0, 7.0, Some("uniform-small")),
            (7.0, 7.0, Some("compact-varied")),
        ])
        .map(|(target_frontage_m, opposite_target_frontage_m, recipe)| {
            let lots = if recipe == Some("compact-varied") {
                blocks::subdivide_compact_rectangular_block(corners, 8.0, 2.2)
            } else {
                blocks::subdivide_mixed_rectangular_block(
                    corners,
                    8.0,
                    target_frontage_m,
                    opposite_target_frontage_m,
                    2.2,
                )
            };
            let generated_lots = lots.len();
            let mut context = context.clone();
            let mut accepted_lots = 0;
            let mut accepted_nominal_area_m2 = 0.0;
            for lot in lots {
                let rect = geometry::OrientedRect {
                    x: lot.center.x,
                    z: lot.center.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                };
                if counterfactual.oriented_parcel_geometry_clear_with_context(&rect, &context) {
                    accepted_lots += 1;
                    accepted_nominal_area_m2 += (lot.width * lot.depth) as f64;
                    // Apply the production parcel-separation distance to later
                    // lots in the same proposed layout.
                    context.parcel_footprints.push(rect);
                }
            }
            VariantReport {
                target_frontage_m,
                opposite_target_frontage_m,
                recipe,
                generated_lots,
                accepted_lots,
                rejected_lots: generated_lots.saturating_sub(accepted_lots),
                accepted_nominal_area_m2,
            }
        })
        .collect();

    assert_eq!(
        engine.save_json(),
        before,
        "diagnostic mutated source engine"
    );
    PlanReport {
        first_observed_tick: observed_tick,
        anchor_node: plan.anchor_node,
        segments: plan.segments.clone(),
        corners: corners.map(|point| (point.x, point.z)),
        road_ids_present_at_observation: road_ids,
        next_segment: plan.next_segment,
        planned_perimeter_segments_checked: 4,
        excluded_parcel_count,
        preserved_parcel_count: engine.state.land.parcels.len(),
        preserved_collision_footprint_count,
        variants,
    }
}

fn run_seed(seed: u32) -> SeedReport {
    let mut engine = Engine::new_legacy(seed);
    engine.set_development_rate(DEVELOPMENT_RATE);
    let mut observed_segments = BTreeSet::new();
    let mut plans = Vec::new();
    let mut plan_cap_reached = false;

    while engine.state.tick < CAPTURE_TICK {
        engine.step(SAMPLE_TICKS);
        let Some(plan) = engine.state.legacy.neighborhood_plan.as_ref() else {
            continue;
        };
        if !plan.coordinated_lots || plan.spine.is_some() || plan.segments.len() != 4 {
            continue;
        }
        // Capture while the plan is still active, before completion releases
        // its reservation and publishes its selected parcel set.
        if plan.next_segment >= plan.segments.len() {
            continue;
        }
        let identity = plan.segments.clone();
        if !observed_segments.insert(identity) {
            continue;
        }
        if plans.len() >= MAX_UNIQUE_PLANS_PER_SEED {
            plan_cap_reached = true;
            break;
        }
        plans.push(evaluate_plan(&engine, engine.state.tick, plan));
    }

    SeedReport {
        seed,
        capture_tick: engine.state.tick,
        plans,
        plan_cap_reached,
        source_unchanged_at_each_observation: true,
    }
}

#[test]
fn subdivision_geometry_diagnostic_keeps_source_and_respects_parcel_blocker() {
    let engine = Engine::new_legacy(0);
    let source_before = engine.save_json();
    let mut witness = None;

    'search: for x in [-250.0_f32, -150.0, -50.0, 50.0, 150.0, 250.0] {
        for z in [-250.0_f32, -150.0, -50.0, 50.0, 150.0, 250.0] {
            let corners = [
                blocks::BlockPoint { x, z },
                blocks::BlockPoint { x: x + 82.0, z },
                blocks::BlockPoint {
                    x: x + 82.0,
                    z: z + 50.0,
                },
                blocks::BlockPoint { x, z: z + 50.0 },
            ];
            for lot in blocks::subdivide_rectangular_block(corners, 8.0, 11.0, 2.2) {
                let rect = geometry::OrientedRect {
                    x: lot.center.x,
                    z: lot.center.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                };
                let context = engine.collision_context();
                if engine.oriented_parcel_geometry_clear_with_context(&rect, &context) {
                    witness = Some(rect);
                    break 'search;
                }
            }
        }
    }

    let rect = witness.expect("find one legal baseline lot in the synthetic search");
    let mut blocked_context = engine.collision_context();
    assert!(engine.oriented_parcel_geometry_clear_with_context(&rect, &blocked_context));
    blocked_context.parcel_footprints.push(rect);
    assert!(!engine.oriented_parcel_geometry_clear_with_context(&rect, &blocked_context));
    assert_eq!(engine.save_json(), source_before);
}

#[test]
#[ignore = "bounded seed growth diagnostic; writes a report only when BLOCK_SUBDIVISION_REPORT is set"]
fn compare_live_block_subdivision_geometry() {
    let reports = [run_seed(0), run_seed(87_231)];
    let json = serde_json::to_string_pretty(&reports).expect("diagnostic JSON serializes");
    if let Ok(path) = std::env::var("BLOCK_SUBDIVISION_REPORT") {
        std::fs::write(path, &json).expect("write subdivision diagnostic report");
    }
    println!("{json}");
}
