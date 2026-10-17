use crate::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn point_in_polygon(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let (x, z) = point;
    let mut previous = polygon.len().saturating_sub(1);
    for current in 0..polygon.len() {
        let (ax, az) = polygon[previous];
        let (bx, bz) = polygon[current];
        let cross = (bx - ax) * (z - az) - (bz - az) * (x - ax);
        let on_segment = cross.abs() <= 1.0e-4
            && x >= ax.min(bx) - 1.0e-4
            && x <= ax.max(bx) + 1.0e-4
            && z >= az.min(bz) - 1.0e-4
            && z <= az.max(bz) + 1.0e-4;
        if on_segment {
            return true;
        }
        if (az > z) != (bz > z) && x < (bx - ax) * (z - az) / (bz - az) + ax {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn point_in_bounds(point: (f32, f32), bounds: &[f32]) -> bool {
    bounds.len() == 4
        && point.0 >= bounds[0]
        && point.0 <= bounds[2]
        && point.1 >= bounds[1]
        && point.1 <= bounds[3]
}

fn distance_to_polygon_boundary(point: (f32, f32), polygon: &[(f32, f32)]) -> f32 {
    let mut distance = f32::INFINITY;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        distance = distance.min(point_segment_distance(point.0, point.1, a.0, a.1, b.0, b.1));
    }
    distance
}

fn classify_candidate(
    engine: &Engine,
    context: &CollisionContext,
    rect: &geometry::OrientedRect,
    access_node: u32,
) -> &'static str {
    if !engine.node_connected(access_node) {
        return "disconnected_access";
    }
    if !engine.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth) {
        return "river_or_bank";
    }
    let (min_x, min_z, max_x, max_z) = rect.bounds();
    if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
        return "world_bounds";
    }
    let protected = geometry::OrientedRect {
        x: -305.0,
        z: -310.0,
        width: 90.0,
        depth: 80.0,
        rotation: 0.0,
    };
    if geometry::rects_overlap(rect, &protected, 0.0) {
        return "protected_land";
    }
    if Engine::overlaps_station_tower_envelopes(
        &engine.available_station_tower_envelopes(),
        rect.x,
        rect.z,
        rect.width,
        rect.depth,
    ) {
        return "station_tower_reservation";
    }
    if !engine.interior_plan_allows_parcel(rect, false) {
        return "interior_access_reservation";
    }
    if !engine.neighborhood_corridor_clear_for_parcel(rect.x, rect.z, max_x - min_x, max_z - min_z)
    {
        return "coordinated_block_reservation";
    }
    if context
        .parcel_footprints
        .iter()
        .any(|old| geometry::rects_overlap(rect, old, 2.0))
    {
        return "existing_parcel_envelope";
    }
    if context
        .open_roads
        .iter()
        .any(|road| geometry::rect_segment_distance(rect, road.ax, road.az, road.bx, road.bz) < 5.0)
    {
        return "open_road_footprint_clearance";
    }
    if context
        .pending_roads
        .iter()
        .any(|road| geometry::rect_segment_distance(rect, road.ax, road.az, road.bx, road.bz) < 7.0)
    {
        return "pending_road_footprint_clearance";
    }
    if !engine.oriented_parcel_geometry_clear_with_context(rect, context) {
        return "other_oriented_geometry_rule";
    }
    if !engine.progressive_frontage_geometry_clear(rect, context) {
        return "legacy_centerline_spacing";
    }
    let mut local_open = 0;
    let mut local_active = 0;
    for building in engine
        .state
        .construction
        .buildings
        .iter()
        .filter(|building| {
            building.stage == Stage::Open
                && (building.x - rect.x).hypot(building.z - rect.z) <= 78.0
        })
    {
        local_open += 1;
        local_active += usize::from(site_active(building));
    }
    if local_open < 4 || local_active < 3 {
        return "local_demand_support";
    }
    "feasible_under_current_infill_rules"
}

fn overlapping_existing_parcel(engine: &Engine, rect: &geometry::OrientedRect) -> Option<u32> {
    engine.state.land.parcels.iter().find_map(|parcel| {
        let mut envelope = parcel_envelope(parcel);
        if geometry::rects_overlap(rect, &envelope, 2.0) {
            return Some(parcel.id);
        }
        if parcel.rotation.is_none() {
            envelope.rotation = engine
                .state
                .construction
                .buildings
                .iter()
                .find(|building| building.parcel_id == parcel.id)
                .map(|building| building.rotation)
                .unwrap_or_else(|| engine.frontage_rotation(parcel.id));
            if geometry::rects_overlap(rect, &envelope, 2.0) {
                return Some(parcel.id);
            }
        }
        None
    })
}

#[test]
#[ignore = "read-only exact-predicate diagnosis of the three largest mature-world voids"]
fn early_reservation_mature_gaps_use_current_admission_predicates() {
    let validation = Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation");
    let save_path = validation.join("early-reservation-wasm-87231-save.json");
    let checkpoint: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&save_path).expect("read reviewed public-WASM checkpoint"),
    )
    .expect("parse public checkpoint");
    let engine =
        Engine::restore_json(&std::fs::read_to_string(&save_path).expect("read public save"))
            .expect("public save canonically restores");
    let void_audit: Value = serde_json::from_str(
        &std::fs::read_to_string(validation.join("density-void-audit.json"))
            .expect("read UI void geometry audit"),
    )
    .expect("parse void audit");
    let context = engine.collision_context();
    let towers = engine.available_station_tower_envelopes();
    let access_scan = engine.propose_interior_access();
    let access_host_count = engine
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .count();
    let access_candidate_count = access_host_count * interior_access::PROPOSALS_PER_HOST;
    let mut access_cycle_cursor = engine.state.legacy.interior_access_cursor;
    let mut access_cycle_examined = 0usize;
    let mut access_cycle_proposals = Vec::new();
    while access_cycle_examined < access_candidate_count {
        let scan = engine.propose_interior_access_from(None, access_cycle_cursor);
        let batch_size = (access_candidate_count - access_cycle_examined)
            .min(interior_access::MAX_PROPOSALS_PER_EVALUATION);
        access_cycle_examined += batch_size;
        access_cycle_cursor = scan.next_cursor;
        access_cycle_proposals.extend(scan.proposals);
    }
    let mut region_reports = Vec::new();

    for region in void_audit["top3VoidComponents"]
        .as_array()
        .expect("3 audited gaps")
    {
        let polygon = region["road_face_polygon_xz"]
            .as_array()
            .expect("face polygon")
            .iter()
            .map(|point| {
                (
                    point[0].as_f64().unwrap() as f32,
                    point[1].as_f64().unwrap() as f32,
                )
            })
            .collect::<Vec<_>>();
        let bounds = region["bbox"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_f64().unwrap() as f32)
            .collect::<Vec<_>>();
        let mut counts = BTreeMap::<String, usize>::new();
        let mut site_keys = BTreeSet::new();
        let mut examples = Vec::new();
        let mut candidate_count = 0usize;
        let mut face_count = 0usize;
        let mut geometry_feasible = 0usize;
        let mut centers_in_bbox = 0usize;
        let mut access_options = Vec::new();
        let mut fully_inside_face_count = 0usize;
        let mut boundary_distances = Vec::new();

        for road in &engine.state.network.roads {
            if road.stage != Stage::Open
                || !road.connected
                || !matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
            {
                continue;
            }
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                continue;
            };
            let (grid_width, grid_depth, spacing) = if engine.is_neighborhood_plan_road(road.id) {
                (
                    NEIGHBORHOOD_PARCEL_SIZE,
                    NEIGHBORHOOD_PARCEL_SIZE,
                    NEIGHBORHOOD_FRONTAGE_SPACING,
                )
            } else {
                (14.0, 12.0, FRONTAGE_SLOT_SPACING)
            };
            let (sites, dimensions, has_tower) = engine.frontage_candidates_with_shallow_fallback(
                road.id, a, b, &context, &towers, grid_width, grid_depth, spacing,
            );
            for (slot, ((x, z, access_node, _along, side), (width, depth))) in
                sites.into_iter().zip(dimensions).enumerate()
            {
                let point = (x, z);
                if !point_in_bounds(point, &bounds) {
                    continue;
                }
                centers_in_bbox += 1;
                if !point_in_polygon(point, &polygon) {
                    continue;
                }
                face_count += 1;
                let rect = geometry::OrientedRect {
                    x,
                    z,
                    width,
                    depth,
                    rotation: (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x)),
                };
                let corners_inside = rect
                    .corners()
                    .iter()
                    .all(|corner| point_in_polygon(*corner, &polygon));
                fully_inside_face_count += usize::from(corners_inside);
                boundary_distances.push(distance_to_polygon_boundary(point, &polygon));
                let key = (
                    (x * 10_000.0).round() as i64,
                    (z * 10_000.0).round() as i64,
                    (width * 10_000.0).round() as i64,
                    (depth * 10_000.0).round() as i64,
                );
                if !site_keys.insert(key) {
                    *counts
                        .entry("duplicate_candidate_center".into())
                        .or_default() += 1;
                    continue;
                }
                candidate_count += 1;
                let reason = if slot == 0 && has_tower && !engine.state.transit.station_online {
                    "station_not_online"
                } else {
                    classify_candidate(&engine, &context, &rect, access_node)
                };
                *counts.entry(reason.into()).or_default() += 1;
                if reason == "feasible_under_current_infill_rules" {
                    geometry_feasible += 1;
                }
                if examples.len() < 16 {
                    let blocker = (reason == "existing_parcel_envelope")
                        .then(|| overlapping_existing_parcel(&engine, &rect))
                        .flatten();
                    let blocker_building = blocker.and_then(|parcel_id| {
                        engine
                            .state
                            .construction
                            .buildings
                            .iter()
                            .find(|building| building.parcel_id == parcel_id)
                    });
                    examples.push(json!({
                        "road_id": road.id,
                        "center": [x,z],
                        "size": [width,depth],
                        "rotation": rect.rotation,
                        "candidate_corners_all_inside_approx_face": corners_inside,
                        "center_to_approx_face_boundary_m": distance_to_polygon_boundary(point, &polygon),
                        "reason": reason,
                        "blocking_parcel_id": blocker,
                        "blocking_building_kind": blocker_building.map(|building| building.kind.as_str()),
                        "blocking_building_stage": blocker_building.map(|building| building.stage.as_str())
                    }));
                }
            }
        }
        let mut access_cycle_options = Vec::new();
        for proposal in &access_cycle_proposals {
            let Some(host) = engine
                .state
                .network
                .roads
                .iter()
                .find(|road| road.id == proposal.host_id)
            else {
                continue;
            };
            let (Some(host_a), Some(host_b)) = (engine.node(host.from), engine.node(host.to))
            else {
                continue;
            };
            let split = (
                host_a.x + (host_b.x - host_a.x) * proposal.host_t,
                host_a.z + (host_b.z - host_a.z) * proposal.host_t,
            );
            let lots_inside = proposal
                .selected_lots
                .iter()
                .filter(|lot| point_in_polygon((lot.x, lot.z), &polygon))
                .count();
            if point_in_polygon((proposal.end_x, proposal.end_z), &polygon) || lots_inside > 0 {
                let option = json!({
                    "host_road_id": proposal.host_id,
                    "split_point": [split.0, split.1],
                    "endpoint": [proposal.end_x, proposal.end_z],
                    "selected_lots": proposal.selected_lots.len(),
                    "lot_centers_inside_face": lots_inside,
                    "score_site_area_m2": proposal.score.site_area as f64 / 1_000_000.0,
                    "branch_length_m": proposal.score.road_length_mm as f64 / 1_000.0,
                });
                access_cycle_options.push(option.clone());
                if access_scan.proposals.iter().any(|current| {
                    current.host_id == proposal.host_id
                        && (current.host_t - proposal.host_t).abs() < 1.0e-6
                        && (current.end_x - proposal.end_x).abs() < 1.0e-4
                        && (current.end_z - proposal.end_z).abs() < 1.0e-4
                }) {
                    access_options.push(option);
                }
            }
        }
        boundary_distances.sort_by(f32::total_cmp);
        let boundary_distance_summary = if boundary_distances.is_empty() {
            json!(null)
        } else {
            json!({
                "min_m": boundary_distances[0],
                "median_m": boundary_distances[boundary_distances.len()/2],
                "max_m": boundary_distances[boundary_distances.len()-1]
            })
        };
        region_reports.push(json!({
            "bbox": bounds,
            "centroid": region["centroid"],
            "face_boundary_road_ids": region["boundary_road_ids"],
            "ui_mask_cells_2m": region["mask_cell_count"],
            "all_road_candidates_with_centers_in_bbox": centers_in_bbox,
            "production_frontage_candidates_center_in_face": face_count,
            "unique_candidates_classified": candidate_count,
            "candidates_whose_four_corners_fit_approx_face": fully_inside_face_count,
            "candidate_center_distance_to_approx_face_boundary": boundary_distance_summary,
            "unique_candidates_feasible_under_current_infill_rules": geometry_feasible,
            "bounded_interior_access_options_touching_face": access_options,
            "all_cycle_interior_access_options_touching_face": access_cycle_options,
            "rejection_counts": counts,
            "examples_first16": examples,
            "classification_note": "Candidate locations come from the engine's production grid, 12m gap and 8m shallow assemblers on every open connected ordinary road. Polygon membership associates a center with the approximate face; all parcel admission checks below call the engine's exact production geometry predicates."
        }));
    }

    let active_housing = engine
        .state
        .construction
        .buildings
        .iter()
        .filter(|building| building.stage == Stage::Open && is_housing(building))
        .collect::<Vec<_>>();
    let active_commerce = engine
        .state
        .construction
        .buildings
        .iter()
        .filter(|building| building.stage == Stage::Open && is_commercial(building))
        .collect::<Vec<_>>();
    let ordinary_open = engine
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .collect::<Vec<_>>();
    let report = json!({
        "checkpoint": "tests/fixtures/validation/early-reservation-wasm-87231-save.json",
        "checkpoint_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
        "seed": engine.state.seed,
        "tick": engine.state.tick,
        "demand": engine.state.people.demand,
        "population": engine.state.people.population,
        "buildings": engine.state.construction.buildings.len(),
        "parcels": engine.state.land.parcels.len(),
        "unbuilt_parcels": engine.state.land.parcels.iter().filter(|parcel| !engine.state.construction.buildings.iter().any(|building| building.parcel_id == parcel.id)).count(),
        "roads": engine.state.network.roads.len(),
        "open_connected_ordinary_roads": ordinary_open.len(),
        "ordinary_road_cap": MAX_SEGMENTS,
        "building_cap": MAX_BUILDINGS,
        "parcel_cap": MAX_PARCELS,
        "active_projects": engine.state.construction.projects.iter().filter(|project| project.progress < 1.0).count(),
        "housing_occupancy": active_housing.iter().map(|building| building.occupancy as u64).sum::<u64>(),
        "housing_capacity": active_housing.iter().map(|building| building.capacity as u64).sum::<u64>(),
        "commercial_active_count": active_commerce.iter().filter(|building| building.business_active).count(),
        "commercial_open_count": active_commerce.len(),
        "commercial_active_capacity": active_commerce.iter().filter(|building| building.business_active).map(|building| building.capacity as u64).sum::<u64>(),
        "neighborhood_plan": engine.state.legacy.neighborhood_plan.as_ref().map(|plan| json!({"next_segment":plan.next_segment,"segments":plan.segments.len()})),
        "pending_interior_access": engine.state.legacy.pending_interior_access.is_some(),
        "bounded_interior_access_proposal_count": access_scan.proposals.len(),
        "bounded_interior_access_next_cursor": access_scan.next_cursor,
        "bounded_interior_access_start_cursor": engine.state.legacy.interior_access_cursor,
        "bounded_interior_access_candidates_examined": (interior_access::MAX_PROPOSALS_PER_EVALUATION)
            .min(ordinary_open.len() * interior_access::PROPOSALS_PER_HOST),
        "full_interior_access_cycle_candidates": access_candidate_count,
        "full_interior_access_cycle_examined": access_cycle_examined,
        "full_interior_access_cycle_proposal_count": access_cycle_proposals.len(),
        "growth_cursors": {
            "frontage_road_cursor": engine.state.legacy.frontage_road_cursor,
            "frontage_slot_cursor": engine.state.legacy.frontage_slot_cursor,
            "road_origin_cursor": engine.state.legacy.road_origin_cursor,
            "neighborhood_anchor_cursor": engine.state.legacy.neighborhood_anchor_cursor
        },
        "voids": region_reports,
        "methodology": "Exact saved-WASM world restored natively without advancing it. The UI-provided polygons are approximate face masks used only to associate candidate centers with the three sampled gaps. Candidate generation uses the engine's production functions. Classification invokes the production oriented geometry and progressive-frontage predicates; only the 4-neighbor/3-active local support is applied as the next gate. This samples generated candidates, not every possible parcel center or unproposed alley."
    });
    let output_path = validation.join("early-reservation-gap-diagnosis.json");
    std::fs::write(&output_path, serde_json::to_vec_pretty(&report).unwrap())
        .expect("write structured diagnosis");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert_eq!(checkpoint["tick"].as_u64(), Some(engine.state.tick));
}
