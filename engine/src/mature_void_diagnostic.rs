use crate::*;
use crate::geometry::{distance_coords, point_segment_distance, OrientedRect};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
struct VoidSample {
    label: &'static str,
    bbox: [f32; 4],
    point: WorldPoint,
}

fn inside_bbox(point: WorldPoint, bbox: [f32; 4]) -> bool {
    point.x >= bbox[0] && point.x <= bbox[1] && point.z >= bbox[2] && point.z <= bbox[3]
}

fn point_bbox_distance(point: WorldPoint, bbox: [f32; 4]) -> f32 {
    let dx = (bbox[0] - point.x).max(0.0).max(point.x - bbox[1]);
    let dz = (bbox[2] - point.z).max(0.0).max(point.z - bbox[3]);
    dx.hypot(dz)
}

fn rle_cells(region: &serde_json::Value) -> BTreeSet<(i32, i32)> {
    let mut cells = BTreeSet::new();
    let rows = &region["void_mask_2m_rle"]["x_cell_index_runs_by_z_index"];
    for (z_text, runs) in rows.as_object().expect("RLE rows") {
        let z = z_text.parse::<i32>().expect("integer z cell");
        for run in runs.as_array().expect("x runs") {
            let start = run[0].as_i64().unwrap() as i32;
            let end = run[1].as_i64().unwrap() as i32;
            for x in start..=end {
                cells.insert((x, z));
            }
        }
    }
    cells
}

fn cell_for(point: WorldPoint) -> (i32, i32) {
    ((point.x / 2.0).floor() as i32, (point.z / 2.0).floor() as i32)
}

fn segment_hits_mask(start: WorldPoint, end: WorldPoint, cells: &BTreeSet<(i32, i32)>) -> bool {
    let length = distance_coords(start.x, start.z, end.x, end.z);
    let samples = (length * 2.0).ceil() as usize;
    (0..=samples).any(|index| {
        let t = if samples == 0 { 0.0 } else { index as f32 / samples as f32 };
        cells.contains(&cell_for(WorldPoint {
            x: start.x + (end.x - start.x) * t,
            z: start.z + (end.z - start.z) * t,
        }))
    })
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum Reject {
    HostMargin,
    BoundsOrRiver,
    Protected,
    ParcelCorridor,
    SplitJunction,
    EndpointSpacing,
    ExistingRoadOrProject,
    TooFewLots,
    Capacity,
    Accepted,
}

/// Mirrors the production proposal's ordered gates, calling the same engine
/// validators and lot-clearance predicate. This is diagnostic-only: it never
/// allocates IDs or mutates the engine.
fn evaluate_exact_candidate(
    engine: &Engine,
    host: &Road,
    host_t: f32,
    side: i8,
    length: f32,
    context: &CollisionContext,
) -> (Reject, Option<(WorldPoint, WorldPoint, Vec<interior_access::InteriorAccessLot>)>) {
    let (Some(host_a), Some(host_b)) = (engine.node(host.from), engine.node(host.to)) else {
        return (Reject::HostMargin, None);
    };
    let dx = host_b.x - host_a.x;
    let dz = host_b.z - host_a.z;
    let host_length = dx.hypot(dz);
    if !host_length.is_finite()
        || host_length * host_t < 10.0
        || host_length * (1.0 - host_t) < 10.0
    {
        return (Reject::HostMargin, None);
    }
    let ux = dx / host_length;
    let uz = dz / host_length;
    let nx = -uz * side as f32;
    let nz = ux * side as f32;
    let start = WorldPoint {
        x: host_a.x + dx * host_t,
        z: host_a.z + dz * host_t,
    };
    let end = WorldPoint {
        x: start.x + nx * length,
        z: start.z + nz * length,
    };
    if [start.x, start.z, end.x, end.z]
        .iter()
        .any(|v| !v.is_finite() || *v < MIN + 24.0 || *v > MAX - 24.0)
        || !engine.ordinary_road_clear_of_river(start.x, start.z, end.x, end.z, 4.5)
    {
        return (Reject::BoundsOrRiver, None);
    }
    if corridor_crosses_protected(start.x, start.z, end.x, end.z) {
        return (Reject::Protected, None);
    }
    if !engine.road_corridor_clear_of_parcels_with_context(
        start.x, start.z, end.x, end.z, context,
    ) {
        return (Reject::ParcelCorridor, None);
    }
    let split_x = start.x;
    let split_z = start.z;
    if engine.state.nodes.iter().any(|node| {
        node.id != host.from
            && node.id != host.to
            && (node.x - split_x).hypot(node.z - split_z) < 1.0
    }) || engine
        .state
        .roads
        .iter()
        .filter(|road| road.id != host.id)
        .any(|road| {
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                return true;
            };
            point_segment_distance(split_x, split_z, a.x, a.z, b.x, b.z) < 0.5
        })
    {
        return (Reject::SplitJunction, None);
    }
    if engine
        .state
        .nodes
        .iter()
        .any(|node| (node.x - end.x).hypot(node.z - end.z) < 18.0)
    {
        return (Reject::EndpointSpacing, None);
    }
    let road_clear = engine
        .state
        .roads
        .iter()
        .filter(|road| road.stage == "open")
        .all(|road| {
            if road.id == host.id {
                return true;
            }
            let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                return false;
            };
            geometry::segments_distance_coords(
                start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z,
            ) >= 8.0
        });
    let projects_clear = engine
        .state
        .projects
        .iter()
        .filter(|project| project.kind == "road" && project.progress < 1.0)
        .all(|project| {
            let Some((from, to)) = project.from.zip(project.to) else {
                return true;
            };
            let (Some(a), Some(b)) = (engine.node(from), engine.node(to)) else {
                return false;
            };
            geometry::segments_distance_coords(
                start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z,
            ) >= 8.0
        });
    if !road_clear || !projects_clear {
        return (Reject::ExistingRoadOrProject, None);
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
    let mut lots = Vec::new();
    for lot_side in [-1_i8, 1] {
        for interval in &intervals {
            let along_meters = (interval.start + interval.end) * 0.5;
            let x = start.x + (end.x - start.x) * (along_meters / length)
                - nz * lot_side as f32 * FRONTAGE_GAP_OFFSET;
            let z = start.z
                + (end.z - start.z) * (along_meters / length)
                + nx * lot_side as f32 * FRONTAGE_GAP_OFFSET;
            let lot = interior_access::InteriorAccessLot {
                x,
                z,
                width: interval.end - interval.start,
                depth: FRONTAGE_GAP_DEPTH,
                rotation: (lot_side as f32 * (end.z - start.z))
                    .atan2(-lot_side as f32 * (end.x - start.x)),
                along: along_meters / length,
                side: lot_side,
            };
            let rect = OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if engine.progressive_frontage_geometry_clear(&rect, &proposed_context)
                && lots.len() < interior_access::MAX_SELECTED_LOTS
            {
                lots.push(lot);
            }
        }
    }
    if lots.len() < 2 {
        return (Reject::TooFewLots, Some((start, end, lots)));
    }
    if engine.state.parcels.len().saturating_add(lots.len()) > MAX_PARCELS
        || engine.state.parcel_frontages.len().saturating_add(lots.len()) > MAX_PARCELS
    {
        return (Reject::Capacity, Some((start, end, lots)));
    }
    (Reject::Accepted, Some((start, end, lots)))
}

#[test]
#[ignore = "bounded exact proposal scan for mature browser-review save"]
fn diagnose_access_proposals_for_mature_browser_voids() {
    let source = include_str!("../../tests/fixtures/access-joins/spine-experiment-browser-review-save.json");
    let mut engine = Engine::restore_json(source).expect("mature browser save restores");
    let start_save = engine.save_json();
    let initial_cursor = engine.state.interior_access_cursor;
    let initial_id = engine.state.next_id;
    let audit: serde_json::Value = serde_json::from_str(include_str!(
        "../../validation/density-void-audit.json"
    ))
    .expect("UI void mask audit");
    let regions = audit["top3VoidComponents"].as_array().unwrap();
    let samples = [
        VoidSample {
            label: "east-south",
            bbox: [91.0, 297.0, -351.0, -103.0],
            point: WorldPoint { x: 175.0, z: -223.0 },
        },
        VoidSample {
            label: "west-north",
            bbox: [-369.0, -249.0, 133.0, 305.0],
            point: WorldPoint { x: -314.85, z: 201.27 },
        },
        VoidSample {
            label: "west-center",
            bbox: [-323.0, -187.0, -35.0, 43.0],
            point: WorldPoint { x: -253.33, z: 4.54 },
        },
    ];

    let mut hosts: Vec<(u32, f32)> = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
        })
        .filter_map(|road| {
            let (a, b) = (engine.node(road.from)?, engine.node(road.to)?);
            Some((road.id, distance(a, b)))
        })
        .collect();
    hosts.sort_by_key(|(id, _)| *id);
    let candidate_count = hosts.len() * interior_access::PROPOSALS_PER_HOST;
    let start_cursor = initial_cursor % candidate_count.max(1);
    let mut cursor = start_cursor;
    let mut calls = 0usize;
    let mut proposals = Vec::new();
    loop {
        engine.state.interior_access_cursor = cursor;
        let scan = engine.propose_interior_access();
        proposals.extend(scan.proposals);
        cursor = scan.next_cursor;
        calls += 1;
        if cursor == start_cursor || calls > candidate_count.div_ceil(interior_access::MAX_PROPOSALS_PER_EVALUATION) + 1 {
            break;
        }
    }

    // Re-run every fixed candidate through the current production geometry
    // gates to identify the earliest limiting check, not just the final zero.
    let context = engine.collision_context();
    let mut fixed_rejections = BTreeMap::<String, usize>::new();
    let mut fixed_accepted = Vec::new();
    for host in engine.state.roads.iter().filter(|road| {
        road.stage == "open"
            && road.connected
            && matches!(road.kind.as_str(), "local" | "arterial")
    }) {
        for fraction in interior_access::FRACTIONS {
            for side in interior_access::SIDES {
                for length in interior_access::LENGTHS {
                    let (reason, candidate) = evaluate_exact_candidate(
                        &engine, host, fraction, side, length, &context,
                    );
                    *fixed_rejections.entry(format!("{reason:?}")).or_default() += 1;
                    if let Some((start, end, lots)) = candidate {
                        if reason == Reject::Accepted {
                            fixed_accepted.push((host.id, fraction, start, end, lots));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(fixed_accepted.len(), proposals.len());

    // Diagnostic-only finer t sweep. The production planner still samples
    // only [.25, .5, .75]; here we look at 0.025 increments and invoke the
    // same exact validators only when the proposed branch crosses a measured
    // void mask.
    let mut adaptive_regions = Vec::new();
    for (region, mask_source) in samples.into_iter().zip(regions) {
        let mask = rle_cells(mask_source);
        let mut counts = BTreeMap::<String, usize>::new();
        let mut nearby_valid = Vec::new();
        let mut rejected_examples = Vec::new();
        let mut targeted = 0usize;
        for host in engine.state.roads.iter().filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
        }) {
            let (Some(a), Some(b)) = (engine.node(host.from), engine.node(host.to)) else {
                continue;
            };
            let host_length = distance(a, b);
            if host_length < 20.0 {
                continue;
            }
            for step in 1..40 {
                let t = step as f32 * 0.025;
                if host_length * t < 10.0 || host_length * (1.0 - t) < 10.0 {
                    continue;
                }
                for side in interior_access::SIDES {
                    for length in interior_access::LENGTHS {
                        let dx = b.x - a.x;
                        let dz = b.z - a.z;
                        let nx = -dz / host_length * side as f32;
                        let nz = dx / host_length * side as f32;
                        let start = WorldPoint { x: a.x + dx * t, z: a.z + dz * t };
                        let end = WorldPoint { x: start.x + nx * length, z: start.z + nz * length };
                        if !segment_hits_mask(start, end, &mask) {
                            continue;
                        }
                        targeted += 1;
                        let (reason, candidate) = evaluate_exact_candidate(
                            &engine, host, t, side, length, &context,
                        );
                        *counts.entry(format!("{reason:?}")).or_default() += 1;
                        if reason != Reject::Accepted {
                            rejected_examples.push((
                                distance_coords(
                                    region.point.x,
                                    region.point.z,
                                    start.x + (end.x - start.x) * 0.5,
                                    start.z + (end.z - start.z) * 0.5,
                                ),
                                json!({
                                    "reason": format!("{reason:?}"),
                                    "host_id": host.id,
                                    "host_t": t,
                                    "side": side,
                                    "length_m": length,
                                    "start": [start.x, start.z],
                                    "end": [end.x, end.z],
                                    "centerline_distance_to_void_sample_m": point_segment_distance(
                                        region.point.x, region.point.z, start.x, start.z, end.x, end.z
                                    ),
                                }),
                            ));
                        }
                        if reason == Reject::Accepted {
                            let lots = candidate.map(|entry| entry.2).unwrap_or_default();
                            nearby_valid.push(json!({
                                "host_id": host.id,
                                "host_t": t,
                                "side": side,
                                "length_m": length,
                                "start": [start.x, start.z],
                                "end": [end.x, end.z],
                                "selected_lots": lots.iter().map(|lot| json!({
                                    "x": lot.x, "z": lot.z,
                                    "width": lot.width, "depth": lot.depth,
                                    "side": lot.side,
                                })).collect::<Vec<_>>(),
                                "lots_in_mask": lots.iter().filter(|lot| mask.contains(&cell_for(WorldPoint { x: lot.x, z: lot.z }))).count(),
                            }));
                        }
                    }
                }
            }
        }
        rejected_examples.sort_by(|a, b| a.0.total_cmp(&b.0));
        adaptive_regions.push(json!({
            "region": region.label,
            "targeted_finer_candidates": targeted,
            "exact_gate_rejections": counts,
            "feasible_candidates": nearby_valid.len(),
            "representative_feasible": nearby_valid.into_iter().take(8).collect::<Vec<_>>(),
            "nearest_rejected_candidates": rejected_examples.into_iter().take(8).map(|(_, record)| record).collect::<Vec<_>>(),
        }));
    }

    let mut region_reports = Vec::new();
    for (region, mask_source) in samples.into_iter().zip(regions) {
        let mask = rle_cells(mask_source);
        let mut nearby = proposals
            .iter()
            .map(|proposal| {
                let road = engine
                    .state
                    .roads
                    .iter()
                    .find(|road| road.id == proposal.host_id)
                    .expect("proposal host exists");
                let a = engine.node(road.from).unwrap();
                let b = engine.node(road.to).unwrap();
                let start = WorldPoint {
                    x: a.x + (b.x - a.x) * proposal.host_t,
                    z: a.z + (b.z - a.z) * proposal.host_t,
                };
                let end = WorldPoint { x: proposal.end_x, z: proposal.end_z };
                let corridor_distance = point_segment_distance(
                    region.point.x, region.point.z, start.x, start.z, end.x, end.z,
                );
                let closest_lot = proposal
                    .selected_lots
                    .iter()
                    .map(|lot| WorldPoint { x: lot.x, z: lot.z })
                    .min_by(|left, right| {
                        distance_coords(left.x, left.z, region.point.x, region.point.z)
                            .total_cmp(&distance_coords(right.x, right.z, region.point.x, region.point.z))
                    });
                let nearest_lot_distance = closest_lot.map_or(f32::INFINITY, |lot| {
                    distance_coords(lot.x, lot.z, region.point.x, region.point.z)
                });
                let lot_centers_in_bbox = proposal
                    .selected_lots
                    .iter()
                    .filter(|lot| inside_bbox(WorldPoint { x: lot.x, z: lot.z }, region.bbox))
                    .count();
                let lot_centers_in_mask = proposal
                    .selected_lots
                    .iter()
                    .filter(|lot| mask.contains(&cell_for(WorldPoint { x: lot.x, z: lot.z })))
                    .count();
                json!({
                    "host_id": proposal.host_id,
                    "host_t": proposal.host_t,
                    "start": [start.x, start.z],
                    "end": [end.x, end.z],
                    "length_m": proposal.score.road_length_mm as f32 / 1000.0,
                    "lot_count": proposal.selected_lots.len(),
                    "area_m2": proposal.score.site_area as f32 / 10_000.0,
                    "corridor_distance_to_sample_m": corridor_distance,
                    "nearest_lot_distance_to_sample_m": nearest_lot_distance,
                    "nearest_lot": closest_lot.map(|p| [p.x, p.z]),
                    "lot_centers_in_bbox": lot_centers_in_bbox,
                    "lot_centers_in_void_mask": lot_centers_in_mask,
                    "branch_intersects_void_mask": segment_hits_mask(start, end, &mask),
                    "bbox_distance_to_sample_m": point_bbox_distance(region.point, region.bbox),
                    "score": [proposal.score.site_area, proposal.score.road_length_mm]
                })
            })
            .collect::<Vec<_>>();
        nearby.sort_by(|left, right| {
            left["nearest_lot_distance_to_sample_m"]
                .as_f64()
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right["nearest_lot_distance_to_sample_m"].as_f64().unwrap_or(f64::INFINITY))
        });
        let bbox_proposals = proposals
            .iter()
            .filter(|proposal| {
                proposal.selected_lots.iter().any(|lot| {
                    inside_bbox(WorldPoint { x: lot.x, z: lot.z }, region.bbox)
                })
            })
            .count();
        let masked_lot_proposals = proposals
            .iter()
            .filter(|proposal| {
                proposal.selected_lots.iter().any(|lot| {
                    mask.contains(&cell_for(WorldPoint { x: lot.x, z: lot.z }))
                })
            })
            .count();
        let corridor_near_sample = proposals
            .iter()
            .filter(|proposal| {
                let road = engine.state.roads.iter().find(|road| road.id == proposal.host_id).unwrap();
                let a = engine.node(road.from).unwrap();
                let b = engine.node(road.to).unwrap();
                let start = WorldPoint { x: a.x + (b.x - a.x) * proposal.host_t, z: a.z + (b.z - a.z) * proposal.host_t };
                point_segment_distance(region.point.x, region.point.z, start.x, start.z, proposal.end_x, proposal.end_z) <= 20.0
            })
            .count();
        region_reports.push(json!({
            "region": region.label,
            "bbox": region.bbox,
            "sample": [region.point.x, region.point.z],
                "feasible_proposals_with_lot_center_in_bbox": bbox_proposals,
            "feasible_proposals_with_lot_center_in_void_mask": masked_lot_proposals,
            "void_mask_cells": mask.len(),
            "feasible_branch_corridors_within20m_sample": corridor_near_sample,
            "nearest_feasible_lot_options": nearby.into_iter().take(5).collect::<Vec<_>>()
        }));
    }

    let best = proposals
        .iter()
        .max_by(|a, b| a.score.compare(b.score))
        .map(|p| json!({
            "host_id": p.host_id,
            "host_t": p.host_t,
            "length_m": p.score.road_length_mm as f32 / 1000.0,
            "lot_count": p.selected_lots.len(),
            "area_m2": p.score.site_area as f32 / 10_000.0,
            "score": [p.score.site_area, p.score.road_length_mm]
        }));
    engine.state.interior_access_cursor = initial_cursor;
    assert_eq!(engine.save_json(), start_save, "read-only proposal scan changed world state");
    let report = json!({
        "input": "validation/spine-experiment-browser-review-save.json",
        "tick": engine.state.tick,
        "initial_cursor": initial_cursor,
        "initial_next_id": initial_id,
        "open_connected_ordinary_hosts": hosts.len(),
        "candidate_positions_in_full_cycle": candidate_count,
        "proposal_windows_scanned": calls,
        "window_budget": interior_access::MAX_PROPOSALS_PER_EVALUATION,
        "feasible_proposals_returned": proposals.len(),
        "fixed_candidate_exact_gate_counts": fixed_rejections,
        "fixed_candidate_exact_successes": fixed_accepted.len(),
        "adaptive_fraction_sweep": {
            "fractions": "0.025 through 0.975 in 0.025 increments, subject to 10m endpoint margins",
            "only_candidates_whose_branch_intersects_mask_are_evaluated": true,
            "regions": adaptive_regions
        },
        "proposal_count_is_bounded_by_scan_windows": proposals.len() <= calls * interior_access::MAX_PROPOSALS_PER_EVALUATION,
        "highest_area_per_road_score": best,
        "regions": region_reports,
        "save_unchanged_after_restoring_cursor": engine.save_json() == start_save
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("validation/mature-void-access-proposals.json");
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
