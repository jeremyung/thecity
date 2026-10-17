//! Read-only compact frontage witness on existing connected roads.
//!
//! The ignored integration diagnostic compares an ordinary 14x12 fixed-grid
//! recipe with 7x12 and 7x8 alternatives. It does not add parcels or
//! alter any growth policy. Each accepted candidate must pass the existing
//! frontage geometry, tower-reservation, local-occupancy, and connected-access
//! gates, and is added to a private collision context before later candidates.

use crate::{geometry, site_active, Engine, Road};
use crate::{ProjectKind, Stage};
use serde::Serialize;
use std::collections::BTreeMap;

const MATURE_SAVE: &str =
    include_str!("../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
const CROP: [f32; 4] = [50.0, 300.0, -350.0, -100.0];
const ENDPOINT_CLEARANCE: f32 = 9.0;
const MAX_STEPS: usize = 8;
const MAX_GRID_SITES_PER_ROAD: usize = 34;

#[derive(Clone, Copy)]
struct Recipe {
    name: &'static str,
    width: f32,
    depth: f32,
    spacing: f32,
    offset: f32,
}

const RECIPES: [Recipe; 3] = [
    Recipe {
        name: "ordinary-grid-14x12",
        width: 14.0,
        depth: 12.0,
        spacing: 16.0,
        offset: 15.0,
    },
    Recipe {
        name: "compact-7x12",
        width: 7.0,
        depth: 12.0,
        spacing: 9.2,
        offset: 15.0,
    },
    Recipe {
        name: "compact-shallow-7x8",
        width: 7.0,
        depth: 8.0,
        spacing: 9.2,
        offset: 12.0,
    },
];

#[derive(Serialize)]
struct Report {
    seed: u32,
    tick: u64,
    demand: f32,
    crop_bounds: [f32; 4],
    ordinary_connected_roads_in_crop_scope: usize,
    full_world_ordinary_connected_roads: usize,
    candidate_method: &'static str,
    max_sampled_steps_from_midpoint: usize,
    max_candidate_sites_per_road: usize,
    includes_gap_and_shallow_fallbacks: bool,
    source_unchanged: bool,
    scopes: Vec<ScopeReport>,
}

#[derive(Serialize)]
struct ScopeReport {
    name: &'static str,
    bounds: Option<[f32; 4]>,
    road_count: usize,
    recipes: Vec<RecipeReport>,
}

#[derive(Serialize)]
struct RecipeReport {
    recipe: &'static str,
    width_m: f32,
    depth_m: f32,
    along_spacing_m: f32,
    center_offset_m: f32,
    generated_candidates: usize,
    globally_eligible: bool,
    rejections: BTreeMap<&'static str, usize>,
    geometry_access_witness_found: bool,
    geometry_access_pass_count: usize,
    geometry_access_candidates: Vec<AcceptedCandidate>,
    production_gate_pass_count: usize,
    production_gate_candidates: Vec<AcceptedCandidate>,
    two_lot_same_host_geometry_access_witness: bool,
    two_lot_same_host_production_gate_witness: bool,
    geometry_access_area_sum_m2_no_overlap_claim: f64,
    accepted_nominal_area_sum_m2_no_overlap_claim: f64,
}

#[derive(Clone, Copy, Serialize)]
struct AcceptedCandidate {
    host_road_id: u32,
    access_node_id: u32,
    along: f32,
    side: i8,
    center: (f32, f32),
    rotation: f32,
    dimensions: (f32, f32),
    nearby_open: usize,
    nearby_active: usize,
}

#[derive(Clone, Copy)]
struct Candidate {
    x: f32,
    z: f32,
    along: f32,
    side: i8,
    access_node: u32,
}

fn ordinary_connected(road: &Road) -> bool {
    road.stage == Stage::Open
        && road.connected
        && !matches!(road.kind, crate::RoadKind::Rail | crate::RoadKind::Bridge)
}

fn road_intersects_expanded_crop(engine: &Engine, road: &Road) -> bool {
    let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
        return false;
    };
    // This conservative padding covers the largest lot half-width, depth,
    // offset and road endpoint margin among the compared recipes.
    let pad = 32.0;
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_z = a.z.min(b.z);
    let max_z = a.z.max(b.z);
    max_x >= CROP[0] - pad
        && min_x <= CROP[1] + pad
        && max_z >= CROP[2] - pad
        && min_z <= CROP[3] + pad
}

fn candidates_for_road(engine: &Engine, road: &Road, recipe: Recipe) -> Vec<Candidate> {
    let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
        return Vec::new();
    };
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    let length = dx.hypot(dz);
    if !length.is_finite() || length <= 0.0 || !recipe.spacing.is_finite() || recipe.spacing <= 0.0
    {
        return Vec::new();
    }
    let nx = -dz / length;
    let nz = dx / length;
    let steps =
        (((length * 0.5 - ENDPOINT_CLEARANCE) / recipe.spacing).floor() as usize).min(MAX_STEPS);
    let mut candidates = Vec::with_capacity((steps * 2 + 1) * 2);
    for step in -(steps as isize)..=(steps as isize) {
        let along = (0.5 + step as f32 * recipe.spacing / length).clamp(0.0, 1.0);
        for side in [-1_i8, 1] {
            candidates.push(Candidate {
                x: a.x + dx * along + nx * side as f32 * recipe.offset,
                z: a.z + dz * along + nz * side as f32 * recipe.offset,
                along,
                side,
                access_node: crate::frontage_access_node(a, b, along),
            });
        }
    }
    debug_assert!(candidates.len() <= MAX_GRID_SITES_PER_ROAD);
    candidates
}

fn rect_inside_crop(rect: &geometry::OrientedRect) -> bool {
    rect.corners()
        .iter()
        .all(|(x, z)| *x >= CROP[0] && *x <= CROP[1] && *z >= CROP[2] && *z <= CROP[3])
}

fn nearby_building_counts(engine: &Engine, x: f32, z: f32) -> (usize, usize) {
    engine
        .state
        .construction
        .buildings
        .iter()
        .filter(|building| {
            building.stage == Stage::Open && (building.x - x).hypot(building.z - z) <= 78.0
        })
        .fold((0, 0), |(open, active), building| {
            (open + 1, active + usize::from(site_active(building)))
        })
}

fn global_frontage_gate(engine: &Engine) -> bool {
    engine.state.land.parcels.len() < crate::MAX_PARCELS
        && engine.state.construction.buildings.len() < crate::MAX_BUILDINGS
        && engine.state.people.demand >= 1.2
        && engine
            .state
            .construction
            .buildings
            .iter()
            .filter(|b| site_active(b))
            .count()
            >= 8
}

fn analyze_recipe(
    engine: &Engine,
    roads: &[Road],
    recipe: Recipe,
    crop_only: bool,
) -> RecipeReport {
    let globally_eligible = global_frontage_gate(engine);
    let mut collision_context = engine.collision_context();
    let tower_envelopes = engine.available_station_tower_envelopes();
    let mut rejections = BTreeMap::new();
    let mut geometry_access_candidates = Vec::new();
    let mut production_gate_candidates = Vec::new();
    let mut generated_candidates = 0;
    let mut geometry_access_area_sum_m2_no_overlap_claim = 0.0;
    let mut accepted_nominal_area_sum_m2_no_overlap_claim = 0.0;

    let mut reject = |reason: &'static str| {
        *rejections.entry(reason).or_insert(0) += 1;
    };

    for road in roads {
        for candidate in candidates_for_road(engine, road, recipe) {
            generated_candidates += 1;
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                reject("missing_host_endpoint");
                continue;
            };
            let rotation =
                (candidate.side as f32 * (b.z - a.z)).atan2(-candidate.side as f32 * (b.x - a.x));
            let rect = geometry::OrientedRect {
                x: candidate.x,
                z: candidate.z,
                width: recipe.width,
                depth: recipe.depth,
                rotation,
            };
            if crop_only && !rect_inside_crop(&rect) {
                reject("outside_crop");
                continue;
            }
            if !engine.node_connected(candidate.access_node) {
                reject("access_node_not_connected");
                continue;
            }
            if !engine.parcel_clear_of_river(candidate.x, candidate.z, recipe.width, recipe.depth) {
                reject("river_site_prefilter");
                continue;
            }
            if Engine::overlaps_station_tower_envelopes(
                &tower_envelopes,
                candidate.x,
                candidate.z,
                recipe.width,
                recipe.depth,
            ) {
                reject("station_tower_reservation");
                continue;
            }
            if !engine.progressive_frontage_geometry_clear(&rect, &collision_context) {
                reject("exact_geometry_or_clearance");
                continue;
            }
            let (nearby_open, nearby_active) =
                nearby_building_counts(engine, candidate.x, candidate.z);
            let accepted_candidate = AcceptedCandidate {
                host_road_id: road.id,
                access_node_id: candidate.access_node,
                along: candidate.along,
                side: candidate.side,
                center: (candidate.x, candidate.z),
                rotation,
                dimensions: (recipe.width, recipe.depth),
                nearby_open,
                nearby_active,
            };
            geometry_access_candidates.push(accepted_candidate);
            geometry_access_area_sum_m2_no_overlap_claim += (recipe.width * recipe.depth) as f64;
            // Reserve each geometric/access witness locally so subsequent
            // candidates must also clear the same 2m parcel separation used
            // by production admission.
            collision_context.parcel_footprints.push(rect);

            if !globally_eligible {
                reject("global_demand_or_capacity_gate");
                continue;
            }
            if nearby_open < 4 || nearby_active < 3 {
                reject("local_occupancy_gate");
                continue;
            }

            accepted_nominal_area_sum_m2_no_overlap_claim += (recipe.width * recipe.depth) as f64;
            production_gate_candidates.push(accepted_candidate);
        }
    }

    let has_two_on_same_host = |candidates: &[AcceptedCandidate]| {
        let mut counts = BTreeMap::<u32, usize>::new();
        candidates.iter().any(|candidate| {
            let count = counts.entry(candidate.host_road_id).or_default();
            *count += 1;
            *count >= 2
        })
    };
    let geometry_access_witness_found = has_two_on_same_host(&geometry_access_candidates);
    let two_lot_same_host_production_gate_witness =
        has_two_on_same_host(&production_gate_candidates);

    RecipeReport {
        recipe: recipe.name,
        width_m: recipe.width,
        depth_m: recipe.depth,
        along_spacing_m: recipe.spacing,
        center_offset_m: recipe.offset,
        generated_candidates,
        globally_eligible,
        rejections,
        geometry_access_witness_found,
        geometry_access_pass_count: geometry_access_candidates.len(),
        geometry_access_candidates,
        production_gate_pass_count: production_gate_candidates.len(),
        production_gate_candidates,
        two_lot_same_host_geometry_access_witness: geometry_access_witness_found,
        two_lot_same_host_production_gate_witness,
        geometry_access_area_sum_m2_no_overlap_claim,
        accepted_nominal_area_sum_m2_no_overlap_claim,
    }
}

fn analyze_scope(
    engine: &Engine,
    name: &'static str,
    roads: Vec<Road>,
    crop_only: bool,
) -> ScopeReport {
    let recipes = RECIPES
        .iter()
        .copied()
        .map(|recipe| analyze_recipe(engine, &roads, recipe, crop_only))
        .collect();
    ScopeReport {
        name,
        bounds: crop_only.then_some(CROP),
        road_count: roads.len(),
        recipes,
    }
}

fn build_report() -> Report {
    let engine = Engine::restore_json(MATURE_SAVE).expect("mature diagnostic save restores");
    let source_before = engine.save_json();
    let roads: Vec<Road> = engine
        .state
        .network
        .roads
        .iter()
        .filter(|road| ordinary_connected(road))
        .cloned()
        .collect();
    let crop_roads: Vec<Road> = roads
        .iter()
        .filter(|road| road_intersects_expanded_crop(&engine, road))
        .cloned()
        .collect();

    let scopes = vec![
        analyze_scope(&engine, "compact-crop", crop_roads.clone(), true),
        analyze_scope(&engine, "full-world", roads.clone(), false),
    ];
    let source_unchanged = engine.save_json() == source_before;
    assert!(
        source_unchanged,
        "diagnostic mutated the mature source world"
    );
    Report {
        seed: engine.state.seed,
        tick: engine.state.tick,
        demand: engine.state.people.demand,
        crop_bounds: CROP,
        ordinary_connected_roads_in_crop_scope: crop_roads.len(),
        full_world_ordinary_connected_roads: roads.len(),
        candidate_method: "fixed midpoint grid sampled at the recipe spacing on both sides; crop scope retains roads whose bounding boxes intersect the crop expanded by 32m",
        max_sampled_steps_from_midpoint: MAX_STEPS,
        max_candidate_sites_per_road: MAX_GRID_SITES_PER_ROAD,
        includes_gap_and_shallow_fallbacks: false,
        source_unchanged,
        scopes,
    }
}

#[test]
#[ignore = "bounded read-only mature-crop geometry/access diagnostic"]
fn compact_frontage_existing_road() {
    let report = build_report();
    let json = serde_json::to_string_pretty(&report).expect("diagnostic report serializes");
    if let Ok(path) = std::env::var("COMPACT_FRONTAGE_REPORT") {
        std::fs::write(path, &json).expect("write compact frontage report");
    }
    println!("{json}");
}

#[test]
fn compact_frontage_width_comparison_at_crop_host() {
    let engine = Engine::restore_json(MATURE_SAVE).expect("mature diagnostic save restores");
    let source_before = engine.save_json();
    let road = engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.id == 2985 && ordinary_connected(road))
        .expect("known crop host road is open, connected, and ordinary");
    let a = engine.node(road.from).expect("host start node exists");
    let b = engine.node(road.to).expect("host end node exists");
    let along = 0.5;
    let side = -1_i8;
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    let length = dx.hypot(dz);
    assert!(length > 0.0);
    let nx = -dz / length;
    let nz = dx / length;
    let center = (
        a.x + dx * along + nx * side as f32 * 15.0,
        a.z + dz * along + nz * side as f32 * 15.0,
    );
    let access_node = crate::frontage_access_node(a, b, along);
    let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
    let global_gates_pass = global_frontage_gate(&engine);
    let (nearby_open, nearby_active) = nearby_building_counts(&engine, center.0, center.1);
    assert!(
        global_gates_pass,
        "the source meets the global demand/capacity gates"
    );
    assert!(
        nearby_open >= 4 && nearby_active >= 3,
        "the source meets local occupancy gates"
    );
    assert!(
        engine.node_connected(access_node),
        "planned frontage access node is connected"
    );

    let evaluate_width = |width: f32| {
        let rect = geometry::OrientedRect {
            x: center.0,
            z: center.1,
            width,
            depth: 12.0,
            rotation,
        };
        assert!(
            rect_inside_crop(&rect),
            "the comparison footprint is fully within the crop"
        );
        let clear_of_towers = !Engine::overlaps_station_tower_envelopes(
            &engine.available_station_tower_envelopes(),
            center.0,
            center.1,
            width,
            12.0,
        );
        let clear_of_river = engine.parcel_clear_of_river(center.0, center.1, width, 12.0);
        let clear_of_geometry =
            engine.progressive_frontage_geometry_clear(&rect, &engine.collision_context());
        (clear_of_towers, clear_of_river, clear_of_geometry)
    };

    let compact_7m = evaluate_width(7.0);
    let ordinary_8m = evaluate_width(8.0);
    assert!(
        compact_7m.0 && compact_7m.1 && compact_7m.2,
        "the 7m crop witness passes unchanged geometry gates"
    );

    let (grid_width, grid_depth, spacing) = if engine.is_neighborhood_plan_road(road.id) {
        (
            crate::NEIGHBORHOOD_PARCEL_SIZE,
            crate::NEIGHBORHOOD_PARCEL_SIZE,
            crate::NEIGHBORHOOD_FRONTAGE_SPACING,
        )
    } else {
        (14.0, 12.0, crate::FRONTAGE_SLOT_SPACING)
    };
    let context = engine.collision_context();
    let towers = engine.available_station_tower_envelopes();
    let (actual_sites, actual_dimensions, _) = engine.frontage_candidates_with_shallow_fallback(
        road.id, a, b, &context, &towers, grid_width, grid_depth, spacing,
    );
    let mut actual_candidates = actual_sites
        .iter()
        .zip(actual_dimensions.iter())
        .map(
            |(&(x, z, access, site_along, site_side), &(width, depth))| {
                let rect = geometry::OrientedRect {
                    x,
                    z,
                    width,
                    depth,
                    rotation: (site_side as f32 * (b.z - a.z))
                        .atan2(-site_side as f32 * (b.x - a.x)),
                };
                let access_pass = engine.node_connected(access);
                let geometry_pass =
                    !Engine::overlaps_station_tower_envelopes(&towers, x, z, width, depth)
                        && engine.parcel_clear_of_river(x, z, width, depth)
                        && engine.progressive_frontage_geometry_clear(&rect, &context);
                let distance = (x - center.0).hypot(z - center.1);
                (
                    distance,
                    x,
                    z,
                    access,
                    site_along,
                    site_side,
                    width,
                    depth,
                    access_pass,
                    geometry_pass,
                )
            },
        )
        .collect::<Vec<_>>();
    actual_candidates.sort_by(|left, right| left.0.total_cmp(&right.0));
    let actual_geometry_access_passes = actual_candidates
        .iter()
        .filter(|candidate| candidate.8 && candidate.9)
        .count();
    let actual_8x12_midpoint_present = actual_candidates.iter().any(|candidate| {
        candidate.0 <= 1.0e-4
            && (candidate.6 - 8.0).abs() <= 1.0e-4
            && (candidate.7 - 12.0).abs() <= 1.0e-4
    });
    let actual_8x12_midpoint_slot =
        actual_sites
            .iter()
            .zip(actual_dimensions.iter())
            .position(|(site, dimensions)| {
                (site.0 - center.0).hypot(site.1 - center.1) <= 1.0e-4
                    && (dimensions.0 - 8.0).abs() <= 1.0e-4
                    && (dimensions.1 - 12.0).abs() <= 1.0e-4
            });
    assert!(
        actual_8x12_midpoint_present,
        "production generator now includes the 8x12 midpoint witness"
    );
    assert!(
        actual_8x12_midpoint_slot.is_some_and(|slot| {
            actual_candidates.iter().any(|candidate| {
                candidate.0 <= 1.0e-4
                    && candidate.6 == 8.0
                    && candidate.7 == 12.0
                    && candidate.8
                    && candidate.9
            }) && slot < actual_sites.len()
        }),
        "the actual 8x12 midpoint candidate passes unchanged geometry and access gates"
    );
    println!(
        "actual production generator for host {}: neighborhood_plan_road={} inputs=({grid_width}x{grid_depth}, spacing={spacing}); generated_count={}; geometry+access_pass_count={actual_geometry_access_passes}; exact 8x12 midpoint present={actual_8x12_midpoint_present}; closest candidates (distance,x,z,access,along,side,width,depth,access_pass,geometry_pass)={:?}",
        road.id,
        engine.is_neighborhood_plan_road(road.id),
        actual_candidates.len(),
        actual_candidates.iter().take(8).collect::<Vec<_>>()
    );
    for candidate in actual_candidates.iter().take(3) {
        let rect = geometry::OrientedRect {
            x: candidate.1,
            z: candidate.2,
            width: candidate.6,
            depth: candidate.7,
            rotation: (candidate.5 as f32 * (b.z - a.z)).atan2(-candidate.5 as f32 * (b.x - a.x)),
        };
        let overlaps_existing_parcel = context
            .parcel_footprints
            .iter()
            .any(|existing| geometry::rects_overlap(&rect, existing, 2.0));
        let interior_plan_allows = engine.interior_plan_allows_parcel(&rect, false);
        let neighborhood_corridor_clear =
            engine.neighborhood_corridor_clear_for_parcel(rect.x, rect.z, rect.width, rect.depth);
        let open_road_footprint_clear = context.open_roads.iter().all(|segment| {
            geometry::rect_segment_distance(&rect, segment.ax, segment.az, segment.bx, segment.bz)
                >= 5.0
        });
        let open_road_center_clear = context.open_roads.iter().all(|segment| {
            crate::point_segment_distance(
                rect.x, rect.z, segment.ax, segment.az, segment.bx, segment.bz,
            ) >= (rect.width.max(rect.depth) * 0.5 + 4.0).max(9.0)
        });
        println!(
            "closest actual candidate obstruction breakdown at ({:.5},{:.5}) {}x{}: parcel_overlap={overlaps_existing_parcel}, interior_plan_allows={interior_plan_allows}, neighborhood_corridor_clear={neighborhood_corridor_clear}, open_road_footprint_clear={open_road_footprint_clear}, open_road_center_clear={open_road_center_clear}",
            candidate.1, candidate.2, candidate.6, candidate.7
        );
    }

    // Exercise the unchanged production admission path from the exact
    // persisted road/slot cursor that would visit this new candidate.
    let mut admission =
        Engine::restore_json(MATURE_SAVE).expect("mature save restores for admission");
    let host_road_index = admission
        .state
        .network
        .roads
        .iter()
        .position(|host| host.id == road.id)
        .expect("host road index exists");
    let admission_context = admission.collision_context();
    let admission_towers = admission.available_station_tower_envelopes();
    let admission_road = admission.state.network.roads[host_road_index].clone();
    let admission_a = admission
        .node(admission_road.from)
        .cloned()
        .expect("admission host start exists");
    let admission_b = admission
        .node(admission_road.to)
        .cloned()
        .expect("admission host end exists");
    let admission_candidates = admission.frontage_candidates_with_shallow_fallback(
        admission_road.id,
        &admission_a,
        &admission_b,
        &admission_context,
        &admission_towers,
        grid_width,
        grid_depth,
        spacing,
    );
    let admission_slot = admission_candidates
        .0
        .iter()
        .zip(admission_candidates.1.iter())
        .position(|(site, dimensions)| {
            (site.0 - center.0).hypot(site.1 - center.1) <= 1.0e-4
                && (dimensions.0 - 8.0).abs() <= 1.0e-4
                && (dimensions.1 - 12.0).abs() <= 1.0e-4
        })
        .expect("production admission generator includes exact narrow witness");
    let (target_x, target_z, target_access, target_along, target_side) =
        admission_candidates.0[admission_slot];
    admission.state.legacy.frontage_road_cursor = host_road_index;
    admission.state.legacy.frontage_slot_cursor = admission_slot as u16;
    let before_parcels = admission.state.land.parcels.len();
    assert!(admission.try_progressive_frontage_infill());
    assert_eq!(admission.state.land.parcels.len(), before_parcels + 1);
    let admitted = admission
        .state
        .land
        .parcels
        .last()
        .expect("admitted parcel exists")
        .clone();
    assert!((admitted.x - target_x).hypot(admitted.z - target_z) <= 1.0e-4);
    assert_eq!((admitted.width, admitted.depth), (8.0, 12.0));
    assert!(admission.node_connected(admitted.access_node));
    let frontage = admission
        .state
        .land
        .parcel_frontages
        .iter()
        .find(|frontage| frontage.parcel_id == admitted.id)
        .expect("production admission records frontage metadata")
        .clone();
    assert_eq!(frontage.road_id, road.id);
    assert_eq!(frontage.side, target_side);
    assert!((frontage.along - target_along).abs() <= 1.0e-5);
    assert_eq!(admitted.access_node, target_access);
    admission.validate().expect("admitted world validates");

    let admitted_save = admission.save_json();
    let mut resumed = Engine::restore_json(&admitted_save).expect("admitted world resumes");
    assert_eq!(resumed.save_json(), admitted_save);
    admission.step(80);
    resumed.step(80);
    assert_eq!(admission.save_json(), resumed.save_json());
    let construction_started = admission.state.construction.projects.iter().any(|project| {
        project.kind == ProjectKind::Building
            && project.progress < 1.0
            && project.parcel_id == Some(admitted.id)
    });
    assert!(
        construction_started,
        "admitted narrow parcel starts ordinary construction"
    );
    println!(
        "normal admission: parcel={} at ({:.5},{:.5}) size={}x{} host={} along={:.5} side={} access={}; save/resume matches after step(80); building_construction_started_on_witness={construction_started}",
        admitted.id,
        admitted.x,
        admitted.z,
        admitted.width,
        admitted.depth,
        frontage.road_id,
        frontage.along,
        frontage.side,
        admitted.access_node
    );
    admission.validate().expect("stepped world validates");
    assert!(
        engine.save_json() == source_before,
        "the witness comparison leaves the source unchanged"
    );
    println!(
        "crop host 2985, access node {access_node}, along {along}, side {side}, center ({:.5}, {:.5}): gates ordered [tower, river, exact_geometry_or_clearance]; 7m={compact_7m:?}; 8m={ordinary_8m:?}; global_gates={global_gates_pass}; nearby_open={nearby_open}; nearby_active={nearby_active}; source_unchanged=true",
        center.0, center.1
    );
}
