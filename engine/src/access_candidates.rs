//! Bounded geometry-derived interior-access candidates for diagnostic and
//! controlled-planner work. The ordinary production planner still uses its
//! established fraction/length grid; this module exposes a face-aware source
//! of exact endpoints to the same proposal evaluator.

use crate::interior_access::{AccessGeometryCandidate, MAX_PROPOSALS_PER_EVALUATION};
use crate::road_faces::RoadFace;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BoundaryHost {
    pub road_id: u32,
    pub from: u32,
    pub to: u32,
    pub ax: f32,
    pub az: f32,
    pub bx: f32,
    pub bz: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CandidateNode {
    pub id: u32,
    pub x: f32,
    pub z: f32,
}

const HOST_END_CLEARANCE_M: f32 = 10.0;
const SPLIT_NODE_CLEARANCE_M: f32 = 1.0;
const MIN_BRANCH_M: f32 = 24.0;
const MAX_BRANCH_M: f32 = 60.0;
const FACE_EDGE_MARGIN_M: f32 = 2.0;
const MAX_HOSTS_PER_FACE: usize = 24;
const MAX_SPLITS_PER_HOST: usize = 4;
const PARCEL_HOST_CLEARANCE_M: f32 = 5.0;
const PARCEL_PROJECTION_MARGIN_M: f32 = 2.0;
const RAY_SAMPLE_M: f32 = 1.0;
const FRONTAGE_TARGET_MARGIN_M: f32 = 8.0;

/// Build a bounded set from one already-closed planar face. Candidate split
/// points come from free host intervals; headings use the inward face normal
/// and the face's sampled interior centroid; lengths are clipped to the face
/// ray. Exact world, road, parcel, project, and capacity checks remain in the
/// engine proposal evaluator.
pub(crate) fn candidates_for_face(
    face: &RoadFace,
    hosts: &[BoundaryHost],
    nodes: &[CandidateNode],
    parcel_footprints: &[crate::geometry::OrientedRect],
    unserved_targets: &[(f32, f32)],
) -> Vec<AccessGeometryCandidate> {
    if face.vertex_ids.len() < 3
        || face.vertex_ids.len() != face.polygon.len()
        || face.vertex_ids.len() != face.boundary_edge_ids.len()
        || face
            .polygon
            .iter()
            .any(|point| !point.0.is_finite() || !point.1.is_finite())
    {
        return Vec::new();
    }

    let hosts_by_id: BTreeMap<u32, BoundaryHost> = hosts
        .iter()
        .copied()
        .map(|host| (host.road_id, host))
        .collect();
    let mut per_host: BTreeMap<u32, Vec<(f32, f32, f32, f32)>> = BTreeMap::new();

    let mut boundary_edges = (0..face.vertex_ids.len()).collect::<Vec<_>>();
    boundary_edges.sort_by_key(|index| face.boundary_edge_ids[*index]);
    let unique_host_count = boundary_edges
        .iter()
        .filter_map(|index| hosts_by_id.get(&face.boundary_edge_ids[*index]).map(|host| host.road_id))
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        .min(MAX_HOSTS_PER_FACE)
        .max(1);
    let per_host_budget = (MAX_PROPOSALS_PER_EVALUATION / unique_host_count).max(1);
    for edge_index in boundary_edges.into_iter().take(MAX_HOSTS_PER_FACE) {
        let edge_id = face.boundary_edge_ids[edge_index];
        let Some(host) = hosts_by_id.get(&edge_id).copied() else {
            continue;
        };
        let (edge_ax, edge_az) = face.polygon[edge_index];
        let (edge_bx, edge_bz) = face.polygon[(edge_index + 1) % face.polygon.len()];
        let edge_dx = edge_bx - edge_ax;
        let edge_dz = edge_bz - edge_az;
        let edge_len = edge_dx.hypot(edge_dz);
        let host_dx = host.bx - host.ax;
        let host_dz = host.bz - host.az;
        let host_len = host_dx.hypot(host_dz);
        if !edge_len.is_finite() || !host_len.is_finite() || host_len < 2.0 * HOST_END_CLEARANCE_M {
            continue;
        }

        // Face polygons are counter-clockwise, so their inward normal is left
        // of each directed boundary edge. Translate the edge-relative t into
        // the host road's persisted direction.
        let alignment = edge_dx * host_dx + edge_dz * host_dz;
        if alignment.abs() < edge_len * host_len * 0.999 {
            continue;
        }
        let inward = (-edge_dz / edge_len, edge_dx / edge_len);
        let mut forbidden = Vec::<(f32, f32)>::new();
        for node in nodes.iter().filter(|node| node.id != host.from && node.id != host.to) {
            let relative_x = node.x - host.ax;
            let relative_z = node.z - host.az;
            let projection = (relative_x * host_dx + relative_z * host_dz) / host_len;
            let cross_distance = (relative_x * host_dz - relative_z * host_dx).abs() / host_len;
            if cross_distance < SPLIT_NODE_CLEARANCE_M {
                forbidden.push((projection - SPLIT_NODE_CLEARANCE_M, projection + SPLIT_NODE_CLEARANCE_M));
            }
        }
        for rect in parcel_footprints {
            let mut min_along = f32::INFINITY;
            let mut max_along = f32::NEG_INFINITY;
            let mut min_side = f32::INFINITY;
            let mut max_side = f32::NEG_INFINITY;
            for (x, z) in rect.corners() {
                let rx = x - host.ax;
                let rz = z - host.az;
                let along = (rx * host_dx + rz * host_dz) / host_len;
                let side = (host_dx * rz - host_dz * rx) / host_len;
                min_along = min_along.min(along);
                max_along = max_along.max(along);
                min_side = min_side.min(side);
                max_side = max_side.max(side);
            }
            if min_side <= PARCEL_HOST_CLEARANCE_M && max_side >= -PARCEL_HOST_CLEARANCE_M {
                forbidden.push((
                    min_along - PARCEL_PROJECTION_MARGIN_M,
                    max_along + PARCEL_PROJECTION_MARGIN_M,
                ));
            }
        }
        forbidden.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
        let mut free = subtract_intervals(
            (HOST_END_CLEARANCE_M, host_len - HOST_END_CLEARANCE_M),
            &forbidden,
        );
        let mut split_samples = Vec::new();
        if free.len() == 1 && free[0].1 - free[0].0 >= 12.0 {
            let (start, end) = free[0];
            let span = end - start;
            split_samples.extend([
                start + span * 0.25,
                start + span * 0.5,
                start + span * 0.75,
                end - span / 6.0,
            ]);
        } else {
            free.sort_by(|left, right| {
                (right.1 - right.0)
                    .total_cmp(&(left.1 - left.0))
                    .then_with(|| left.0.total_cmp(&right.0))
            });
            split_samples.extend(
                free.into_iter()
                    .take(MAX_SPLITS_PER_HOST)
                    .map(|(start, end)| (start + end) * 0.5),
            );
        }
        for along in split_samples.into_iter().take(MAX_SPLITS_PER_HOST) {
            let host_candidate_count = per_host.entry(host.road_id).or_default().len();
            if host_candidate_count >= per_host_budget {
                continue;
            }
            let host_t = along / host_len;
            if !(0.0..1.0).contains(&host_t) {
                continue;
            }
            let start = (host.ax + host_dx * host_t, host.az + host_dz * host_t);
            // A short, face-normal access corridor is a useful geometry-derived
            // alternative when individual destination rays terminate at nearby
            // perimeter junctions. Its endpoint stays inside the traced face.
            let inward_ray = ray_exit_distance(start, inward, &face.polygon);
            let inward_safe = (inward_ray - FACE_EDGE_MARGIN_M).min(MAX_BRANCH_M);
            for length in [30.0_f32, 36.0, 42.0] {
                if per_host.get(&host.road_id).map_or(0, Vec::len) >= per_host_budget {
                    break;
                }
                if length < MIN_BRANCH_M || length > inward_safe {
                    continue;
                }
                let end = (start.0 + inward.0 * length, start.1 + inward.1 * length);
                if nodes.iter().any(|node| (node.x - end.0).hypot(node.z - end.1) < 18.0)
                    || !segment_inside_polygon(start, end, &face.polygon)
                    || !parcel_footprints.iter().all(|parcel| {
                        crate::geometry::rect_segment_distance(
                            parcel, start.0, start.1, end.0, end.1,
                        ) >= PARCEL_HOST_CLEARANCE_M
                    })
                {
                    continue;
                }
                per_host.entry(host.road_id).or_default().push((host_t, end.0, end.1, length));
            }
            let mut headings = Vec::<((f32, f32), f32)>::new();
            for target in unserved_targets.iter().take(12) {
                let delta = (target.0 - start.0, target.1 - start.1);
                let distance = delta.0.hypot(delta.1);
                if !distance.is_finite() || distance < 15.0 || dot(inward, delta) <= 0.0 {
                    continue;
                }
                let bearing = delta.1.atan2(delta.0);
                let offset = (15.0_f32 / distance).clamp(-1.0, 1.0).asin();
                let direct = (delta.0 / distance, delta.1 / distance);
                let candidates = [
                    ((bearing - offset).cos(), (bearing - offset).sin()),
                    ((bearing + offset).cos(), (bearing + offset).sin()),
                    direct,
                ];
                for direction in candidates {
                    if dot(inward, direction) <= 0.0
                        || !junction_angle_ok((host_dx / host_len, host_dz / host_len), direction)
                        || headings
                            .iter()
                            .any(|(old, _)| dot(*old, direction) > 0.9999)
                    {
                        continue;
                    }
                    let ray_length = ray_exit_distance(start, direction, &face.polygon);
                    let safe_length = (ray_length - FACE_EDGE_MARGIN_M).min(MAX_BRANCH_M);
                    let target_along = dot(delta, direction);
                    if safe_length < MIN_BRANCH_M
                        || target_along < FRONTAGE_TARGET_MARGIN_M
                        || target_along > safe_length - FRONTAGE_TARGET_MARGIN_M
                    {
                        continue;
                    }
                    let endpoint = (
                        start.0 + direction.0 * safe_length,
                        start.1 + direction.1 * safe_length,
                    );
                    if !segment_inside_polygon(start, endpoint, &face.polygon)
                        || !parcel_footprints.iter().all(|parcel| {
                            crate::geometry::rect_segment_distance(
                                parcel, start.0, start.1, endpoint.0, endpoint.1,
                            ) >= PARCEL_HOST_CLEARANCE_M
                        })
                    {
                        continue;
                    }
                    headings.push((direction, target_along + FRONTAGE_TARGET_MARGIN_M));
                    if headings.len() == 2 {
                        break;
                    }
                }
                if headings.len() == 2 {
                    break;
                }
            }
            // If the direct sightline to a clear target is blocked, derive
            // nearby alternatives from the closest blocking parcel's corner
            // tangents. This is obstacle-driven (at most four corners and
            // eight rays per target), rather than a uniform angle sweep.
            if headings.len() < 2 {
                for target in unserved_targets.iter().take(12) {
                    let Some(blocker) = parcel_footprints
                        .iter()
                        .map(|parcel| {
                            (
                                crate::geometry::rect_segment_distance(
                                    parcel,
                                    start.0,
                                    start.1,
                                    target.0,
                                    target.1,
                                ),
                                parcel,
                            )
                        })
                        .filter(|(distance, _)| *distance < PARCEL_HOST_CLEARANCE_M)
                        .min_by(|left, right| left.0.total_cmp(&right.0))
                        .map(|(_, parcel)| parcel)
                    else {
                        continue;
                    };
                    for corner in blocker.corners() {
                        let dx = corner.0 - start.0;
                        let dz = corner.1 - start.1;
                        let corner_distance = dx.hypot(dz);
                        if corner_distance <= PARCEL_HOST_CLEARANCE_M + 0.5 {
                            continue;
                        }
                        let bearing = dz.atan2(dx);
                        let tangent = ((PARCEL_HOST_CLEARANCE_M + 0.5) / corner_distance)
                            .clamp(-1.0, 1.0)
                            .asin();
                        for angle in [bearing - tangent, bearing + tangent] {
                            let direction = (angle.cos(), angle.sin());
                            if dot(inward, direction) <= 0.0
                                || dot(direction, (target.0 - start.0, target.1 - start.1)) <= 0.0
                                || !junction_angle_ok(
                                    (host_dx / host_len, host_dz / host_len),
                                    direction,
                                )
                                || headings
                                    .iter()
                                    .any(|(old, _)| dot(*old, direction) > 0.9999)
                            {
                                continue;
                            }
                            let ray_length = ray_exit_distance(start, direction, &face.polygon);
                            if ray_length < MIN_BRANCH_M
                                || !parcel_footprints.iter().all(|parcel| {
                                    crate::geometry::rect_segment_distance(
                                        parcel,
                                        start.0,
                                        start.1,
                                        start.0 + direction.0 * ray_length,
                                        start.1 + direction.1 * ray_length,
                                    ) >= PARCEL_HOST_CLEARANCE_M
                                })
                            {
                                continue;
                            }
                            headings.push((direction, f32::INFINITY));
                            if headings.len() == 2 {
                                break;
                            }
                        }
                        if headings.len() == 2 {
                            break;
                        }
                    }
                    if headings.len() == 2 {
                        break;
                    }
                }
            }
            if headings.len() < 2 {
                let ray_length = ray_exit_distance(start, inward, &face.polygon);
                if ray_length >= MIN_BRANCH_M
                    && parcel_footprints.iter().all(|parcel| {
                        crate::geometry::rect_segment_distance(
                            parcel,
                            start.0,
                            start.1,
                            start.0 + inward.0 * ray_length,
                            start.1 + inward.1 * ray_length,
                        ) >= PARCEL_HOST_CLEARANCE_M
                    })
                {
                    if !headings
                        .iter()
                        .any(|(old, _)| dot(*old, inward) > 0.9999)
                    {
                        headings.push((inward, f32::INFINITY));
                    }
                }
            }
            for (heading, target_distance) in headings {
                let ray_length = ray_exit_distance(start, heading, &face.polygon);
                let safe_length = (ray_length - FACE_EDGE_MARGIN_M).min(MAX_BRANCH_M);
                if safe_length < MIN_BRANCH_M {
                    continue;
                }
                let lengths = if target_distance.is_finite() && target_distance <= safe_length {
                    [target_distance, (target_distance - FRONTAGE_TARGET_MARGIN_M).max(MIN_BRANCH_M), 0.0, 0.0]
                } else {
                    [MIN_BRANCH_M, 36.0, 48.0, safe_length]
                };
                for length in lengths.into_iter().filter(|length| *length >= MIN_BRANCH_M && *length <= safe_length) {
                    if per_host.get(&host.road_id).map_or(0, Vec::len) >= per_host_budget {
                        break;
                    }
                    let end = (start.0 + heading.0 * length, start.1 + heading.1 * length);
                    if segment_inside_polygon(start, end, &face.polygon)
                        && nodes
                            .iter()
                            .all(|node| (node.x - end.0).hypot(node.z - end.1) >= 18.0)
                        && parcel_footprints.iter().all(|parcel| {
                            crate::geometry::rect_segment_distance(
                                parcel, start.0, start.1, end.0, end.1,
                            ) >= PARCEL_HOST_CLEARANCE_M
                        })
                    {
                        per_host.entry(host.road_id).or_default().push((host_t, end.0, end.1, length));
                    }
                }
            }
        }
    }

    // Round-robin hosts so one long boundary cannot consume the full global
    // proposal budget before other face edges are considered.
    for candidates in per_host.values_mut() {
        candidates.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then_with(|| a.3.total_cmp(&b.3))
                .then_with(|| a.1.total_cmp(&b.1))
                .then_with(|| a.2.total_cmp(&b.2))
        });
        candidates.dedup_by(|a, b| {
            (a.0 - b.0).abs() < 1.0e-5
                && (a.1 - b.1).abs() < 1.0e-4
                && (a.2 - b.2).abs() < 1.0e-4
        });
    }
    let host_ids = per_host.keys().copied().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut round = 0;
    while output.len() < MAX_PROPOSALS_PER_EVALUATION {
        let mut added = false;
        for host_id in &host_ids {
            let Some(candidate) = per_host.get(host_id).and_then(|items| items.get(round)) else {
                continue;
            };
            output.push(AccessGeometryCandidate {
                host_id: *host_id,
                host_t: candidate.0,
                end_x: candidate.1,
                end_z: candidate.2,
                tie_index: output.len() as u8,
            });
            added = true;
            if output.len() == MAX_PROPOSALS_PER_EVALUATION {
                break;
            }
        }
        if !added {
            break;
        }
        round += 1;
    }
    output
}

fn subtract_intervals(base: (f32, f32), exclusions: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut spans = vec![base];
    for &(exclude_start, exclude_end) in exclusions {
        let mut next = Vec::new();
        for (start, end) in spans {
            if exclude_end <= start || exclude_start >= end {
                next.push((start, end));
            } else {
                if exclude_start > start {
                    next.push((start, exclude_start.min(end)));
                }
                if exclude_end < end {
                    next.push((exclude_end.max(start), end));
                }
            }
        }
        spans = next;
    }
    spans
        .into_iter()
        .filter(|(start, end)| end - start > 1.0e-3)
        .collect()
}

fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

fn junction_angle_ok(host: (f32, f32), branch: (f32, f32)) -> bool {
    let cross = (host.0 * branch.1 - host.1 * branch.0).abs();
    let cosine = dot(host, branch).abs();
    cross >= 0.5 && cosine <= 0.866_026
}

fn point_in_polygon(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let (ax, az) = polygon[previous];
        let (bx, bz) = polygon[current];
        let cross = (bx - ax) * (point.1 - az) - (bz - az) * (point.0 - ax);
        if cross.abs() <= 1.0e-4
            && point.0 >= ax.min(bx) - 1.0e-4
            && point.0 <= ax.max(bx) + 1.0e-4
            && point.1 >= az.min(bz) - 1.0e-4
            && point.1 <= az.max(bz) + 1.0e-4
        {
            return true;
        }
        if (az > point.1) != (bz > point.1)
            && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn segment_inside_polygon(start: (f32, f32), end: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let length = (end.0 - start.0).hypot(end.1 - start.1);
    let steps = (length / RAY_SAMPLE_M).ceil() as usize;
    if steps == 0 || steps > 64 {
        return false;
    }
    (1..steps).all(|index| {
        let t = index as f32 / steps as f32;
        point_in_polygon(
            (start.0 + (end.0 - start.0) * t, start.1 + (end.1 - start.1) * t),
            polygon,
        )
    })
}

fn ray_exit_distance(start: (f32, f32), heading: (f32, f32), polygon: &[(f32, f32)]) -> f32 {
    let mut last_inside = 0.0;
    let max_steps = (MAX_BRANCH_M / RAY_SAMPLE_M).ceil() as usize;
    for step in 1..=max_steps {
        let distance = step as f32 * RAY_SAMPLE_M;
        let point = (start.0 + heading.0 * distance, start.1 + heading.1 * distance);
        if !point_in_polygon(point, polygon) {
            return last_inside;
        }
        last_inside = distance;
    }
    MAX_BRANCH_M
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_candidate_budget_and_order_are_stable() {
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        };
        let hosts = vec![
            BoundaryHost { road_id: 10, from: 1, to: 2, ax: 0.0, az: 0.0, bx: 100.0, bz: 0.0 },
            BoundaryHost { road_id: 11, from: 2, to: 3, ax: 100.0, az: 0.0, bx: 100.0, bz: 100.0 },
            BoundaryHost { road_id: 12, from: 3, to: 4, ax: 100.0, az: 100.0, bx: 0.0, bz: 100.0 },
            BoundaryHost { road_id: 13, from: 4, to: 1, ax: 0.0, az: 100.0, bx: 0.0, bz: 0.0 },
        ];
        let nodes = vec![
            CandidateNode { id: 1, x: 0.0, z: 0.0 },
            CandidateNode { id: 2, x: 100.0, z: 0.0 },
            CandidateNode { id: 3, x: 100.0, z: 100.0 },
            CandidateNode { id: 4, x: 0.0, z: 100.0 },
        ];
        let first = candidates_for_face(&face, &hosts, &nodes, &[], &[]);
        assert!(!first.is_empty());
        assert!(first.len() <= MAX_PROPOSALS_PER_EVALUATION);
        assert_eq!(first, candidates_for_face(&face, &hosts, &nodes, &[], &[]));
        assert!(first.iter().enumerate().all(|(index, candidate)| candidate.tie_index as usize == index));
        assert!(first.iter().map(|candidate| candidate.host_id).collect::<std::collections::BTreeSet<_>>().len() > 1);
    }

    #[test]
    fn safe_host_intervals_skip_existing_split_nodes() {
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        };
        let host = BoundaryHost { road_id: 10, from: 1, to: 2, ax: 0.0, az: 0.0, bx: 100.0, bz: 0.0 };
        let hosts = vec![host];
        let nodes = vec![
            CandidateNode { id: 1, x: 0.0, z: 0.0 },
            CandidateNode { id: 2, x: 100.0, z: 0.0 },
            CandidateNode { id: 5, x: 50.0, z: 0.0 },
        ];
        let candidates = candidates_for_face(&face, &hosts, &nodes, &[], &[]);
        assert!(!candidates.iter().any(|candidate| (candidate.host_t * 100.0 - 50.0).abs() < 1.0));
    }
}
