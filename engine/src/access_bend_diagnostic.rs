//! Offline, bounded witness search for a two-segment interior access street.
//!
//! This is a geometric diagnostic only. It does not construct a runtime access
//! proposal and must not be used as an admission predicate.

use crate::access_discovery_job::DiscoveryJob;
use crate::access_proposal_validation::{
    AdmissionGates, ProposalHost, ProposalRoad, ProposalValidationInput,
};
use crate::face_target_viability::{FrozenViabilityContext, Segment, ViabilityQuery};
use crate::geometry::{self, OrientedRect};
use crate::road_face_trace::RoadFaceTrace;
use crate::road_faces::RoadFace;
use crate::*;
use serde_json::json;
use std::sync::Arc;

const TARGET: &[u32] = &[2656, 2657, 2772, 2773, 3131, 3010, 3011, 2895];
const MAX_DISCOVERY_WORK: usize = 6_000_000;
const MAX_TRACE_WORK: usize = 12_000_000;
const MAX_DESCRIPTORS: usize = 2_500;
const MAX_ROUTE_CANDIDATES: usize = MAX_DESCRIPTORS * 3 * 4;
const BEND_DISTANCES: [f32; 3] = [10.0, 20.0, 30.0];
const PERP_OFFSETS: [f32; 4] = [-20.0, -10.0, 10.0, 20.0];
const DIAGNOSTIC_MAX_LEG_M: f32 = 60.0;
const DIAGNOSTIC_MIN_FINAL_LEG_M: f32 = 24.0;
const PROTECTED_LAND: OrientedRect = OrientedRect {
    x: -305.0,
    z: -310.0,
    width: 90.0,
    depth: 80.0,
    rotation: 0.0,
};

#[derive(Clone, Copy)]
struct BendPath {
    start: (f32, f32),
    bend: (f32, f32),
    end: (f32, f32),
    host_id: u32,
}

#[derive(Default)]
struct Checks {
    reasons: std::collections::BTreeMap<String, usize>,
    descriptor_reasons: std::collections::BTreeMap<String, usize>,
    candidates: usize,
    tested_lots: usize,
    accepted: usize,
}

#[test]
#[ignore = "bounded offline two-segment bend witness search; writes evidence to /tmp"]
fn diagnose_two_segment_bend_witness() {
    let save_path = std::env::var("DENSITY_AUDIT_SAVE")
        .expect("set DENSITY_AUDIT_SAVE to the save file to audit");
    let report_path = std::env::var("BEND_AUDIT_REPORT")
        .unwrap_or_else(|_| "/tmp/thecity-bend-audit.json".into());
    let compact_lots = match std::env::var("BEND_LOT_PROFILE").as_deref() {
        Ok("small") => true,
        Ok("standard") | Err(_) => false,
        _ => panic!("BEND_LOT_PROFILE must be standard or small"),
    };
    let lot_offset: f32 = std::env::var("BEND_LOT_OFFSET_M")
        .unwrap_or_else(|_| if compact_lots { "9.5" } else { "15" }.into())
        .parse()
        .expect("finite lot offset in metres");
    assert!(
        lot_offset.is_finite()
            && ((if compact_lots { 9.25 } else { 11.25 })..=15.0).contains(&lot_offset)
    );
    let save = std::fs::read_to_string(&save_path).expect("read diagnostic save");
    let engine = Engine::restore_json(&save).expect("restore diagnostic save");
    let proposal = access_proposal_capture::capture(&engine).expect("capture viability");
    let discovery_input = access_capture::capture(&engine).expect("capture face graph");
    let mut excluded = engine.state.exhausted_access_faces.clone();
    excluded.retain(|identity| identity.as_slice() != TARGET);
    let mut discovery = DiscoveryJob::new_excluding(discovery_input.clone(), excluded)
        .expect("start target face discovery");
    let mut discovery_work = 0usize;
    while discovery.descriptors().is_none() {
        let step = discovery.advance(4096);
        discovery_work += step.work_used;
        assert!(
            discovery_work <= MAX_DISCOVERY_WORK,
            "discovery work cap exceeded"
        );
        assert!(step.work_used > 0 || step.done, "discovery stalled");
    }
    assert_eq!(discovery.selected_face_identity(), Some(TARGET));
    let candidates = discovery.descriptors().unwrap().unwrap();

    let mut trace = RoadFaceTrace::new(
        discovery_input.graph_nodes.clone(),
        discovery_input.graph_edges.clone(),
        None,
    )
    .expect("trace graph");
    let mut trace_work = 0usize;
    while trace.faces().is_none() {
        let step = trace.advance(4096);
        trace_work += step.work_used;
        assert!(trace_work <= MAX_TRACE_WORK, "face trace work cap exceeded");
        assert!(step.work_used > 0 || step.done, "face tracing stalled");
    }
    let face = trace
        .faces()
        .unwrap()
        .as_ref()
        .unwrap()
        .iter()
        .find(|face| face.identity.as_slice() == TARGET)
        .expect("target face polygon");

    let ctx = FrozenViabilityContext::new(proposal.viability.clone()).expect("frozen context");
    let mut checks = Checks::default();
    let mut samples = Vec::new();
    let mut node_rejection_samples = Vec::new();
    let mut witness = None;
    let mut examined_descriptors = 0usize;
    'search: for candidate in candidates.iter().take(MAX_DESCRIPTORS) {
        examined_descriptors += 1;
        let Some(host) = proposal
            .hosts
            .iter()
            .find(|host| host.id == candidate.host_id)
        else {
            *checks
                .descriptor_reasons
                .entry("missing_host".into())
                .or_default() += 1;
            continue;
        };
        let start = (
            host.segment.ax + (host.segment.bx - host.segment.ax) * candidate.host_t,
            host.segment.az + (host.segment.bz - host.segment.az) * candidate.host_t,
        );
        let ray = (candidate.end_x - start.0, candidate.end_z - start.1);
        let length = ray.0.hypot(ray.1);
        if !length.is_finite() || length <= 1.0 {
            *checks
                .descriptor_reasons
                .entry("invalid_initial_ray".into())
                .or_default() += 1;
            continue;
        }
        let tangent = (ray.0 / length, ray.1 / length);
        let normal = (-tangent.1, tangent.0);
        for distance in BEND_DISTANCES {
            for offset in PERP_OFFSETS {
                if checks.candidates >= MAX_ROUTE_CANDIDATES {
                    break 'search;
                }
                checks.candidates += 1;
                let bend = (
                    start.0 + tangent.0 * distance + normal.0 * offset,
                    start.1 + tangent.1 * distance + normal.1 * offset,
                );
                let path = BendPath {
                    start,
                    bend,
                    end: (candidate.end_x, candidate.end_z),
                    host_id: candidate.host_id,
                };
                let s1 = seg(path.start, path.bend);
                let s2 = seg(path.bend, path.end);
                let first_length = (s1.bx - s1.ax).hypot(s1.bz - s1.az);
                let final_length = (s2.bx - s2.ax).hypot(s2.bz - s2.az);
                if !first_length.is_finite()
                    || !final_length.is_finite()
                    || first_length < 10.0
                    || first_length > DIAGNOSTIC_MAX_LEG_M
                    || final_length < DIAGNOSTIC_MIN_FINAL_LEG_M
                    || final_length > DIAGNOSTIC_MAX_LEG_M
                    || first_length + final_length > 120.0
                {
                    reject(&mut checks, "diagnostic_leg_length_bounds");
                    continue;
                }
                if !segment_inside_face(s1, face) || !segment_inside_face(s2, face) {
                    reject(&mut checks, "outside_target_face");
                    continue;
                }
                if let Some(reason) = road_conflict(s1, s2, host.id, &proposal) {
                    reject(&mut checks, reason);
                    continue;
                }
                if let Some(reason) = segment_land_rejection(s1, &proposal)
                    .or_else(|| segment_land_rejection(s2, &proposal))
                {
                    reject(&mut checks, reason);
                    continue;
                }
                if !valid_split_and_end(path, *host, &proposal) {
                    if node_rejection_samples.len() < 8 {
                        let nearest = proposal
                            .nodes
                            .iter()
                            .min_by(|a, b| {
                                geometry::distance_coords(a.x, a.z, path.bend.0, path.bend.1)
                                    .total_cmp(&geometry::distance_coords(
                                        b.x,
                                        b.z,
                                        path.bend.0,
                                        path.bend.1,
                                    ))
                            })
                            .unwrap();
                        node_rejection_samples.push(json!({"host_id": path.host_id,
                            "start": path.start, "bend": path.bend, "end": path.end,
                            "nearest_bend_node": {"id": nearest.id, "x": nearest.x, "z": nearest.z},
                            "bend_node_distance_m": geometry::distance_coords(nearest.x, nearest.z, path.bend.0, path.bend.1),
                            "required_new_node_clearance_m": 18.0}));
                    }
                    reject(&mut checks, "split_or_endpoint_clearance");
                    continue;
                }
                let lots = final_segment_lots(s2, lot_offset, compact_lots)
                    .into_iter()
                    .filter(|lot| rect_inside_face(lot, face))
                    .collect::<Vec<_>>();
                let valid = validate_lots(&lots, [s1, s2], ctx.clone(), &mut checks);
                if valid.len() >= 2 {
                    checks.accepted += 1;
                    witness = Some(
                        json!({"host_id": path.host_id, "start": path.start, "bend": path.bend, "end": path.end, "segments": [s1, s2], "valid_final_segment_lots": valid}),
                    );
                    break 'search;
                }
                reject(&mut checks, "fewer_than_two_valid_final_lots");
                if samples.len() < 8 {
                    samples.push(json!({"host_id": path.host_id, "bend": bend, "reason": "fewer_than_two_valid_final_lots", "valid_lots": valid.len()}));
                }
            }
        }
    }
    assert_eq!(
        checks.candidates,
        checks.reasons.values().sum::<usize>() + checks.accepted
    );
    let bounds = face.polygon.iter().fold(
        (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ),
        |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)),
    );
    let local_footprints: Vec<_> = proposal
        .viability
        .parcel_footprints
        .iter()
        .enumerate()
        .filter(|(_, rect)| {
            let b = rect.bounds();
            b.2 >= bounds.0 - 10.0
                && b.0 <= bounds.2 + 10.0
                && b.3 >= bounds.1 - 10.0
                && b.1 <= bounds.3 + 10.0
        })
        .map(|(index, rect)| json!({"capture_index": index, "rect": rect}))
        .collect();
    let report = json!({
        "mode": "offline_geometry_witness_only",
        "lot_center_offset_m": lot_offset,
        "lot_depth_m": if compact_lots { 8.0 } else { 12.0 },
        "lot_target_width_m": if compact_lots { 10.0 } else { 14.0 },
        "production_valid": false,
        "tick": engine.state.tick,
        "save_path": save_path,
        "target_face_identity": TARGET,
        "target_face_polygon_vertices": face.polygon.len(),
        "target_face_polygon_xz": face.polygon,
        "local_parcel_footprints": local_footprints,
        "discovery_work": discovery_work,
        "discovery_work_cap": MAX_DISCOVERY_WORK,
        "face_trace_work": trace_work,
        "face_trace_work_cap": MAX_TRACE_WORK,
        "descriptor_count_available": candidates.len(),
        "descriptor_prefix_examined": examined_descriptors,
        "candidate_cap": MAX_ROUTE_CANDIDATES,
        "bend_distances_m": BEND_DISTANCES,
        "perpendicular_offsets_m": PERP_OFFSETS,
        "diagnostic_leg_length_bounds_m": {"first_min": 10.0, "each_leg_max": DIAGNOSTIC_MAX_LEG_M, "final_leg_min": DIAGNOSTIC_MIN_FINAL_LEG_M, "total_max": 120.0},
        "candidate_count": checks.candidates,
        "route_outcome_count": checks.reasons.values().sum::<usize>() + checks.accepted,
        "accepted_route_count": checks.accepted,
        "tested_lots": checks.tested_lots,
        "rejection_counts": checks.reasons,
        "descriptor_rejection_counts": checks.descriptor_reasons,
        "witness": witness,
        "samples": samples,
        "node_rejection_samples": node_rejection_samples,
        "limitations": ["does not invoke live admission or mutate engine state", "candidate generation examines only the first bounded descriptor prefix", "lot frontage intervals are bounded diagnostic 8-14m spans and are not allocated parcel IDs", "no live construction capacity, demand, project, or scheduler gate is asserted"],
        "note": "A witness means two explicitly checked connected road segments stay inside this captured face, pass listed land/road/node checks, and support at least two mutually nonoverlapping viable lots on the final segment. It is geometric evidence only, not a production-valid proposal."
    });
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap())
        .expect("write bend diagnostic");
    println!("{report}");
}

fn reject(checks: &mut Checks, reason: &str) {
    *checks.reasons.entry(reason.to_owned()).or_default() += 1;
}

fn seg(a: (f32, f32), b: (f32, f32)) -> Segment {
    Segment {
        ax: a.0,
        az: a.1,
        bx: b.0,
        bz: b.1,
    }
}

fn road_conflict(
    first: Segment,
    second: Segment,
    host_id: u32,
    input: &ProposalValidationInput,
) -> Option<&'static str> {
    for road in &input.roads {
        let Some(existing) = road.segment else {
            return Some("road_missing_endpoint");
        };
        // The first segment intentionally attaches to the host. Its distance
        // from that one host is exempt; all other roads retain 8m clearance.
        if road.id != host_id
            && geometry::segments_distance_coords(
                first.ax,
                first.az,
                first.bx,
                first.bz,
                existing.ax,
                existing.az,
                existing.bx,
                existing.bz,
            ) < 8.0
        {
            return Some("road_clearance_first_segment");
        }
        if geometry::segments_distance_coords(
            second.ax,
            second.az,
            second.bx,
            second.bz,
            existing.ax,
            existing.az,
            existing.bx,
            existing.bz,
        ) < 8.0
        {
            return Some("road_clearance_final_segment");
        }
    }
    for pending in &input.viability.pending_roads {
        if geometry::segments_distance_coords(
            first.ax, first.az, first.bx, first.bz, pending.ax, pending.az, pending.bx, pending.bz,
        ) < 7.0
            || geometry::segments_distance_coords(
                second.ax, second.az, second.bx, second.bz, pending.ax, pending.az, pending.bx,
                pending.bz,
            ) < 7.0
        {
            return Some("pending_road_clearance");
        }
    }
    if let Some(reserved) = input.gates.pending_access_branch {
        if geometry::segments_distance_coords(
            first.ax,
            first.az,
            first.bx,
            first.bz,
            reserved.ax,
            reserved.az,
            reserved.bx,
            reserved.bz,
        ) < 8.0
            || geometry::segments_distance_coords(
                second.ax,
                second.az,
                second.bx,
                second.bz,
                reserved.ax,
                reserved.az,
                reserved.bx,
                reserved.bz,
            ) < 8.0
        {
            return Some("pending_access_branch_clearance");
        }
    }
    for reserved in &input.viability.neighborhood_segments {
        if geometry::segments_distance_coords(
            first.ax,
            first.az,
            first.bx,
            first.bz,
            reserved.ax,
            reserved.az,
            reserved.bx,
            reserved.bz,
        ) < 8.0
            || geometry::segments_distance_coords(
                second.ax,
                second.az,
                second.bx,
                second.bz,
                reserved.ax,
                reserved.az,
                reserved.bx,
                reserved.bz,
            ) < 8.0
        {
            return Some("neighborhood_corridor_clearance");
        }
    }
    // These legs share the bend endpoint by construction. Reject only a
    // collinear reversal that overlaps for positive length.
    let first_from_bend = (first.ax - first.bx, first.az - first.bz);
    let second_from_bend = (second.bx - second.ax, second.bz - second.az);
    let cross = first_from_bend.0 * second_from_bend.1 - first_from_bend.1 * second_from_bend.0;
    let dot = first_from_bend.0 * second_from_bend.0 + first_from_bend.1 * second_from_bend.1;
    if cross.abs() < 1.0e-4 && dot > 0.0 {
        return Some("segment_self_collision");
    }
    None
}

fn clear_segment_land(segment: Segment, input: &ProposalValidationInput) -> bool {
    segment_land_rejection(segment, input).is_none()
}

fn segment_land_rejection(
    segment: Segment,
    input: &ProposalValidationInput,
) -> Option<&'static str> {
    if input
        .viability
        .parcel_footprints
        .iter()
        .chain(&input.viability.reserved_lots)
        .any(|rect| {
            geometry::rect_segment_distance(rect, segment.ax, segment.az, segment.bx, segment.bz)
                < 5.0
        })
    {
        return Some("parcel_or_reservation_clearance");
    }
    if input.viability.river_segments.iter().any(|river| {
        geometry::segments_distance_coords(
            segment.ax, segment.az, segment.bx, segment.bz, river.ax, river.az, river.bx, river.bz,
        ) < crate::RIVER_HALF_WIDTH + crate::RIVER_BANK_BUFFER + 4.5
    }) {
        return Some("river_clearance");
    }
    if ![segment.ax, segment.bx]
        .into_iter()
        .all(|x| x >= crate::MIN + 24.0 && x <= crate::MAX - 24.0)
        || ![segment.az, segment.bz]
            .into_iter()
            .all(|z| z >= crate::MIN + 24.0 && z <= crate::MAX - 24.0)
    {
        return Some("world_boundary_clearance");
    }
    if segment_hits_protected(segment) {
        return Some("protected_land_clearance");
    }
    if input.viability.neighborhood_interior.is_some_and(|rect| {
        geometry::rect_segment_distance(&rect, segment.ax, segment.az, segment.bx, segment.bz)
            == 0.0
    }) {
        return Some("neighborhood_interior_clearance");
    }
    None
}

fn valid_split_and_end(
    path: BendPath,
    host: ProposalHost,
    input: &ProposalValidationInput,
) -> bool {
    let host_length = geometry::distance_coords(
        host.segment.ax,
        host.segment.az,
        host.segment.bx,
        host.segment.bz,
    );
    if host_length <= 20.0
        || geometry::distance_coords(host.segment.ax, host.segment.az, path.start.0, path.start.1)
            < 10.0
        || geometry::distance_coords(host.segment.bx, host.segment.bz, path.start.0, path.start.1)
            < 10.0
    {
        return false;
    }
    let split_node_clear = input
        .nodes
        .iter()
        .filter(|node| node.id != host.from && node.id != host.to)
        .all(|node| geometry::distance_coords(node.x, node.z, path.start.0, path.start.1) >= 1.0);
    let split_road_clear = input
        .roads
        .iter()
        .filter(|road| road.id != path.host_id)
        .all(|road| {
            road.segment.is_some_and(|s| {
                geometry::point_segment_distance(path.start.0, path.start.1, s.ax, s.az, s.bx, s.bz)
                    >= 0.5
            })
        });
    let bend_node_clear = input
        .nodes
        .iter()
        .all(|node| geometry::distance_coords(node.x, node.z, path.bend.0, path.bend.1) >= 18.0);
    let end_node_clear = input
        .nodes
        .iter()
        .all(|node| geometry::distance_coords(node.x, node.z, path.end.0, path.end.1) >= 18.0);
    split_node_clear && split_road_clear && bend_node_clear && end_node_clear
}

fn segment_hits_protected(segment: Segment) -> bool {
    geometry::rect_segment_distance(
        &PROTECTED_LAND,
        segment.ax,
        segment.az,
        segment.bx,
        segment.bz,
    ) == 0.0
}

fn final_segment_lots(segment: Segment, offset: f32, compact: bool) -> Vec<OrientedRect> {
    let dx = segment.bx - segment.ax;
    let dz = segment.bz - segment.az;
    let length = dx.hypot(dz);
    if length < 24.0 {
        return Vec::new();
    }
    let tangent = (dx / length, dz / length);
    let intervals = crate::frontage::subdivide_frontage(
        length,
        &[],
        crate::FRONTAGE_ENDPOINT_CLEARANCE,
        8.0,
        if compact { 10.0 } else { 14.0 },
        2.2,
        crate::MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
    );
    let mut lots = Vec::new();
    for side in [-1.0_f32, 1.0] {
        for interval in &intervals {
            let along = (interval.start + interval.end) * 0.5;
            lots.push(OrientedRect {
                x: segment.ax + tangent.0 * along - tangent.1 * side * offset,
                z: segment.az + tangent.1 * along + tangent.0 * side * offset,
                width: interval.end - interval.start,
                depth: if compact { 8.0 } else { 12.0 },
                rotation: (side * dz).atan2(-side * dx),
            });
        }
    }
    lots
}

fn validate_lots(
    lots: &[OrientedRect],
    roads: [Segment; 2],
    context: Arc<FrozenViabilityContext>,
    checks: &mut Checks,
) -> Vec<OrientedRect> {
    let mut valid: Vec<OrientedRect> = Vec::new();
    'lot: for lot in lots {
        checks.tested_lots += 1;
        for road in roads {
            let mut query = ViabilityQuery::new_with_extra_road(*lot, context.clone(), road)
                .expect("finite lot and road");
            while query.result().is_none() {
                let step = query.advance(4096);
                assert!(step.work_used > 0 || step.done, "viability query stalled");
            }
            if query.result() != Some(true) {
                continue 'lot;
            }
        }
        if valid
            .iter()
            .any(|other| geometry::rects_overlap(lot, other, 2.0))
        {
            continue;
        }
        valid.push(*lot);
    }
    valid
}

fn segment_inside_face(segment: Segment, face: &RoadFace) -> bool {
    let midpoint = (
        (segment.ax + segment.bx) * 0.5,
        (segment.az + segment.bz) * 0.5,
    );
    point_in_polygon(midpoint, &face.polygon)
        && point_in_polygon((segment.bx, segment.bz), &face.polygon)
        && !face.polygon.iter().enumerate().any(|(i, a)| {
            crosses_after_start(segment, *a, face.polygon[(i + 1) % face.polygon.len()])
        })
}

fn rect_inside_face(rect: &OrientedRect, face: &RoadFace) -> bool {
    let corners = rect.corners();
    corners
        .iter()
        .all(|corner| point_in_polygon(*corner, &face.polygon))
        && (0..4).all(|index| {
            let a = corners[index];
            let b = corners[(index + 1) % 4];
            segment_inside_face(seg(a, b), face)
        })
}

fn crosses_after_start(path: Segment, a: (f32, f32), b: (f32, f32)) -> bool {
    let (rx, rz) = (path.bx - path.ax, path.bz - path.az);
    let (sx, sz) = (b.0 - a.0, b.1 - a.1);
    let cross = |x: f32, z: f32, u: f32, v: f32| x * v - z * u;
    let (qx, qz) = (a.0 - path.ax, a.1 - path.az);
    let denom = cross(rx, rz, sx, sz);
    if denom.abs() < 1.0e-6 {
        if cross(qx, qz, rx, rz).abs() >= 1.0e-4 {
            return false;
        }
        let rr = rx * rx + rz * rz;
        if rr <= 1.0e-8 {
            return false;
        }
        let ta = (qx * rx + qz * rz) / rr;
        let tb = ((b.0 - path.ax) * rx + (b.1 - path.az) * rz) / rr;
        return ta.max(tb).min(1.0) > 1.0e-4 && ta.min(tb).max(0.0) < 1.0 - 1.0e-4;
    }
    let t = cross(qx, qz, sx, sz) / denom;
    let u = cross(qx, qz, rx, rz) / denom;
    t > 1.0e-4 && t <= 1.0 && (0.0..=1.0).contains(&u)
}

fn point_in_polygon(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if (a.1 > point.1) != (b.1 > point.1)
            && point.0 < (b.0 - a.0) * (point.1 - a.1) / (b.1 - a.1) + a.0
        {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face_target_viability::FrozenViabilityInput;

    fn square() -> RoadFace {
        RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10000.0,
            boundary_edge_ids: vec![1, 2, 3, 4],
            contains_proposed_edge: false,
        }
    }

    #[test]
    fn synthetic_bend_stays_in_face_and_supports_final_lots() {
        let path = BendPath {
            start: (0.0, 50.0),
            bend: (20.0, 60.0),
            end: (75.0, 60.0),
            host_id: 1,
        };
        assert!(segment_inside_face(seg(path.start, path.bend), &square()));
        assert!(segment_inside_face(seg(path.bend, path.end), &square()));
        let host = ProposalHost {
            id: 1,
            from: 1,
            to: 2,
            segment: seg((0.0, 0.0), (0.0, 100.0)),
        };
        let input = synthetic_input(host);
        let first = seg(path.start, path.bend);
        let second = seg(path.bend, path.end);
        assert_eq!(road_conflict(first, second, host.id, &input), None);
        assert!(clear_segment_land(first, &input) && clear_segment_land(second, &input));
        assert!(valid_split_and_end(path, host, &input));
        let lots = final_segment_lots(seg(path.bend, path.end), 15.0, false);
        assert!(lots.len() >= 2);
        let context = FrozenViabilityContext::new(input.viability.clone()).unwrap();
        let mut checks = Checks::default();
        let valid = validate_lots(
            &lots,
            [seg(path.start, path.bend), seg(path.bend, path.end)],
            context,
            &mut checks,
        );
        assert!(
            valid.len() >= 2,
            "clear synthetic geometry should provide two viable lots"
        );
    }

    #[test]
    fn synthetic_face_edge_blocks_bend() {
        let face = square();
        assert!(!segment_inside_face(
            seg((20.0, 50.0), (120.0, 50.0)),
            &face
        ));
    }

    #[test]
    fn compact_frontage_preserves_road_clearance_for_rotated_streets() {
        for end in [(60.0, 0.0), (0.0, 60.0), (36.0, 48.0), (-36.0, 48.0)] {
            let road = seg((0.0, 0.0), end);
            for (offset, compact) in [(11.25, false), (9.5, true)] {
                let lots = final_segment_lots(road, offset, compact);
                assert!(lots.len() >= 2);
                for lot in lots {
                    assert!(
                        geometry::rect_segment_distance(&lot, road.ax, road.az, road.bx, road.bz)
                            >= 5.0
                    );
                }
            }
        }
    }

    #[test]
    fn synthetic_obstacles_reject_second_leg_and_host_approach() {
        let host = ProposalHost {
            id: 1,
            from: 1,
            to: 2,
            segment: seg((0.0, 0.0), (0.0, 100.0)),
        };
        let mut input = synthetic_input(host);
        let first = seg((0.0, 50.0), (20.0, 60.0));
        let second = seg((20.0, 60.0), (75.0, 60.0));
        input.viability.parcel_footprints.push(OrientedRect {
            x: 50.0,
            z: 60.0,
            width: 8.0,
            depth: 8.0,
            rotation: 0.0,
        });
        assert_eq!(
            segment_land_rejection(second, &input),
            Some("parcel_or_reservation_clearance")
        );
        assert_eq!(
            road_conflict(first, seg((20.0, 60.0), (0.0, 50.0)), host.id, &input),
            Some("road_clearance_final_segment")
        );
    }

    #[test]
    fn collinear_reversal_is_rejected_and_concave_escape_is_detected() {
        let host = ProposalHost {
            id: 1,
            from: 1,
            to: 2,
            segment: seg((0.0, 0.0), (0.0, 100.0)),
        };
        let input = synthetic_input(host);
        assert_eq!(
            road_conflict(
                seg((0.0, 50.0), (20.0, 50.0)),
                seg((20.0, 50.0), (10.0, 50.0)),
                host.id,
                &input
            ),
            Some("segment_self_collision")
        );
        let concave = RoadFace {
            identity: vec![1, 2, 3, 4, 5, 6, 7, 8],
            vertex_ids: vec![1, 2, 3, 4, 5, 6, 7, 8],
            polygon: vec![
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 10.0),
                (7.0, 10.0),
                (7.0, 3.0),
                (3.0, 3.0),
                (3.0, 10.0),
                (0.0, 10.0),
            ],
            area: 72.0,
            boundary_edge_ids: vec![1, 2, 3, 4, 5, 6, 7, 8],
            contains_proposed_edge: false,
        };
        assert!(!segment_inside_face(seg((1.0, 8.0), (9.0, 8.0)), &concave));
    }

    #[test]
    fn bend_is_a_new_node_and_retains_eighteen_metre_clearance() {
        let host = ProposalHost {
            id: 1,
            from: 1,
            to: 2,
            segment: seg((0.0, 0.0), (0.0, 100.0)),
        };
        let path = BendPath {
            start: (0.0, 50.0),
            bend: (20.0, 60.0),
            end: (75.0, 60.0),
            host_id: 1,
        };
        let mut input = synthetic_input(host);
        assert!(valid_split_and_end(path, host, &input));
        input
            .nodes
            .push(crate::access_proposal_validation::ProposalNode {
                id: 3,
                x: 20.0,
                z: 70.0,
            });
        assert!(!valid_split_and_end(path, host, &input));
    }

    fn synthetic_input(host: ProposalHost) -> ProposalValidationInput {
        ProposalValidationInput {
            hosts: vec![host],
            nodes: vec![
                crate::access_proposal_validation::ProposalNode {
                    id: 1,
                    x: 0.0,
                    z: 0.0,
                },
                crate::access_proposal_validation::ProposalNode {
                    id: 2,
                    x: 0.0,
                    z: 100.0,
                },
            ],
            roads: vec![ProposalRoad {
                id: host.id,
                is_open: true,
                segment: Some(host.segment),
            }],
            projects: vec![],
            viability: FrozenViabilityInput {
                parcel_footprints: vec![],
                reserved_lots: vec![],
                open_roads: vec![host.segment],
                pending_roads: vec![],
                river_segments: vec![],
                neighborhood_segments: vec![],
                neighborhood_interior: None,
            },
            gates: AdmissionGates {
                demand: 1.0,
                has_pending_interior_plan: false,
                has_neighborhood_plan: false,
                road_count: 0,
                project_count: 0,
                parcel_count: 0,
                frontage_count: 0,
                next_id: 1,
                project_admission_available: true,
                pending_access_branch: None,
            },
        }
    }
}
