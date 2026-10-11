//! Bounded geometry-derived interior-access candidates for diagnostic and
//! controlled-planner work. The ordinary production planner still uses its
//! established fraction/length grid; this module exposes a face-aware source
//! of exact endpoints to the same proposal evaluator.

use crate::interior_access::{AccessGeometryCandidate, MAX_PROPOSALS_PER_EVALUATION};
use crate::road_faces::RoadFace;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct BoundaryHost {
    pub road_id: u32,
    pub from: u32,
    pub to: u32,
    pub ax: f32,
    pub az: f32,
    pub bx: f32,
    pub bz: f32,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct CandidateNode {
    pub id: u32,
    pub x: f32,
    pub z: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CandidateScan {
    pub candidates: Vec<AccessGeometryCandidate>,
    /// Bounded geometry-work units: one per split-point prefilter bundle plus
    /// one per final endpoint validation. A bundle considers at most 12 target
    /// records, 36 direct bearing variants, and 96 obstacle-corner tangents;
    /// at most four headings proceed to endpoint length checks. The host
    /// inventory projection pass is separate: it visits at most 24 hosts and
    /// scans caller node/parcel arrays once per host (bounded by world caps).
    pub work_units: usize,
}

const HOST_END_CLEARANCE_M: f32 = 10.0;
const SPLIT_NODE_CLEARANCE_M: f32 = 1.0;
pub(crate) const MIN_BRANCH_M: f32 = 24.0;
pub(crate) const MAX_BRANCH_M: f32 = 60.0;
pub(crate) const FACE_EDGE_MARGIN_M: f32 = 2.0;
pub(crate) const MAX_HOSTS_PER_FACE: usize = 24;
/// Quartile/midpoint samples per host; edge samples are appended after them.
pub(crate) const MAX_INTERVAL_SPLITS: usize = 4;
pub(crate) const MAX_EDGE_SPLITS: usize = 2;
pub(crate) const MAX_SPLITS_PER_HOST: usize = MAX_INTERVAL_SPLITS + MAX_EDGE_SPLITS;
pub(crate) const MAX_HEADINGS_PER_SPLIT: usize = 4;
pub(crate) const PARCEL_HOST_CLEARANCE_M: f32 = 5.0;
pub(crate) const PARCEL_PROJECTION_MARGIN_M: f32 = 2.0;
pub(crate) const RAY_SAMPLE_M: f32 = 1.0;
pub(crate) const MAX_RAY_LOOKAHEAD_M: f32 = MAX_BRANCH_M + FACE_EDGE_MARGIN_M;
pub(crate) const FRONTAGE_TARGET_MARGIN_M: f32 = 15.0;
pub(crate) const MAX_TARGETS_PER_SPLIT: usize = 12;
/// Offline prepared searches refuse more complex faces so the per-descriptor
/// point-in-polygon work remains predictably bounded.
pub(crate) const MAX_FACE_VERTICES_FOR_PREPARED_SEARCH: usize = 64;
pub(crate) const MAX_PREPARED_INPUT_NODES: usize = 5_000;
pub(crate) const MAX_PREPARED_INPUT_PARCELS: usize = 4_000;
pub(crate) const MAX_PREPARED_DESCRIPTORS: usize =
    MAX_HOSTS_PER_FACE * MAX_SPLITS_PER_HOST * (1 + MAX_TARGETS_PER_SPLIT * 3 + 4 * 4 * 2) * 4;

/// A fully ordered, immutable offline search. Preparation is deliberately
/// separate from paging: the page cursor indexes this descriptor array
/// directly and never rebuilds/discards an earlier prefix.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedAccessSearch {
    descriptors: Vec<AccessGeometryCandidate>,
    /// Generator's existing counter (split bundles + candidate endpoint
    /// checks); it is not a complete CPU-operation count for preparation.
    pub preparation_counted_units: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CandidatePage {
    pub candidates: Vec<AccessGeometryCandidate>,
    /// Returned descriptor count, not total evaluator CPU work (which also
    /// builds and queries a collision context).
    pub work_units: usize,
    pub next_cursor: usize,
    pub exhausted: bool,
}

impl PreparedAccessSearch {
    /// Build the complete finite descriptor set synchronously for an offline
    /// diagnostic. This is not suitable for a simulation tick. Bounds are
    /// 24 hosts × 4 split points × 69 headings × 4 lengths = 26,496 output
    /// descriptors; the nested preparation loops also cap 12 targets, 4
    /// nearest blockers, 4,000 parcel envelopes, 5,000 nodes, and 64 face
    /// vertices. The resulting object can be paged without prefix rescans.
    pub(crate) fn prepare(
        face: &RoadFace,
        hosts: &[BoundaryHost],
        nodes: &[CandidateNode],
        parcel_footprints: &[crate::geometry::OrientedRect],
        unserved_targets: &[(f32, f32)],
    ) -> Self {
        if face.polygon.len() > MAX_FACE_VERTICES_FOR_PREPARED_SEARCH
            || face.boundary_edge_ids.len() > 512
            || hosts.len() > 512
            || nodes.len() > MAX_PREPARED_INPUT_NODES
            || parcel_footprints.len() > MAX_PREPARED_INPUT_PARCELS
            || unserved_targets.len() > MAX_TARGETS_PER_SPLIT
        {
            return Self {
                descriptors: Vec::new(),
                preparation_counted_units: 0,
            };
        }
        let mut hosts = hosts.to_vec();
        hosts.sort_by_key(|host| host.road_id);
        let mut nodes = nodes.to_vec();
        nodes.sort_by_key(|node| node.id);
        let mut parcels = parcel_footprints.to_vec();
        parcels.sort_by(|a, b| {
            a.x.total_cmp(&b.x)
                .then_with(|| a.z.total_cmp(&b.z))
                .then_with(|| a.width.total_cmp(&b.width))
                .then_with(|| a.depth.total_cmp(&b.depth))
                .then_with(|| a.rotation.total_cmp(&b.rotation))
        });
        parcels.dedup_by(|a, b| {
            a.x == b.x
                && a.z == b.z
                && a.width == b.width
                && a.depth == b.depth
                && a.rotation == b.rotation
        });
        let mut targets = unserved_targets.to_vec();
        targets.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
        targets.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        let scan = candidates_for_face_mode(face, &hosts, &nodes, &parcels, &targets, true);
        assert!(scan.candidates.len() <= MAX_PREPARED_DESCRIPTORS);
        Self {
            descriptors: scan.candidates,
            preparation_counted_units: scan.work_units,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.descriptors.len()
    }

    pub(crate) fn page(&self, cursor: usize) -> CandidatePage {
        let start = cursor.min(self.descriptors.len());
        let end = start
            .saturating_add(MAX_PROPOSALS_PER_EVALUATION)
            .min(self.descriptors.len());
        let candidates = self.descriptors[start..end]
            .iter()
            .enumerate()
            .map(|(index, candidate)| AccessGeometryCandidate {
                tie_index: index as u8,
                ..*candidate
            })
            .collect::<Vec<_>>();
        CandidatePage {
            work_units: candidates.len(),
            candidates,
            next_cursor: end,
            exhausted: end == self.descriptors.len(),
        }
    }
}

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
    candidates_for_face_with_work(face, hosts, nodes, parcel_footprints, unserved_targets)
        .candidates
}

pub(crate) fn candidates_for_face_with_work(
    face: &RoadFace,
    hosts: &[BoundaryHost],
    nodes: &[CandidateNode],
    parcel_footprints: &[crate::geometry::OrientedRect],
    unserved_targets: &[(f32, f32)],
) -> CandidateScan {
    candidates_for_face_mode(
        face,
        hosts,
        nodes,
        parcel_footprints,
        unserved_targets,
        false,
    )
}

fn candidates_for_face_mode(
    face: &RoadFace,
    hosts: &[BoundaryHost],
    nodes: &[CandidateNode],
    parcel_footprints: &[crate::geometry::OrientedRect],
    unserved_targets: &[(f32, f32)],
    exhaustive: bool,
) -> CandidateScan {
    if face.vertex_ids.len() < 3
        || face.vertex_ids.len() != face.polygon.len()
        || face.vertex_ids.len() != face.boundary_edge_ids.len()
        || face
            .polygon
            .iter()
            .any(|point| !point.0.is_finite() || !point.1.is_finite())
    {
        return CandidateScan {
            candidates: Vec::new(),
            work_units: 0,
        };
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
        .filter_map(|index| {
            hosts_by_id
                .get(&face.boundary_edge_ids[*index])
                .map(|host| host.road_id)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        .min(MAX_HOSTS_PER_FACE)
        .max(1);
    let per_host_budget = if exhaustive {
        usize::MAX
    } else {
        (MAX_PROPOSALS_PER_EVALUATION / unique_host_count).max(1)
    };
    let heading_limit = if exhaustive {
        1 + MAX_TARGETS_PER_SPLIT * 3 + 4 * 4 * 2
    } else {
        MAX_HEADINGS_PER_SPLIT
    };
    let descriptor_limit = if exhaustive {
        MAX_PREPARED_DESCRIPTORS
    } else {
        MAX_PROPOSALS_PER_EVALUATION
    };
    // Reserve an equal work share for each host so early hosts cannot starve
    // later boundaries. Unused shares are not charged or redistributed.
    // Count both split-origin prefilter bundles and final endpoint checks.
    // A split bundle includes the bounded node/parcel interval work and its
    // target-heading/ray-prefix scans; counting it before those scans prevents
    // an output-only cap from allowing an unbounded candidate search.
    let mut evaluated = 0usize;
    // Work is charged per road, not per boundary edge: a road bounding this
    // face on both sides is traced twice and shares one allowance.
    let mut host_work_by_road = BTreeMap::<u32, usize>::new();
    for edge_index in boundary_edges.into_iter().take(MAX_HOSTS_PER_FACE) {
        let edge_id = face.boundary_edge_ids[edge_index];
        let Some(host) = hosts_by_id.get(&edge_id).copied() else {
            continue;
        };
        let (edge_ax, edge_az) = face.polygon[edge_index];
        let (edge_bx, edge_bz) = face.polygon[(edge_index + 1) % face.polygon.len()];
        let edge_dx = edge_bx - edge_ax;
        let edge_dz = edge_bz - edge_az;
        let edge_len = crate::access_math::length(edge_dx, edge_dz);
        let host_dx = host.bx - host.ax;
        let host_dz = host.bz - host.az;
        let host_len = crate::access_math::length(host_dx, host_dz);
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
        let split_samples = host_split_samples(host, nodes, parcel_footprints);
        let host_work = host_work_by_road.entry(host.road_id).or_default();
        'splits: for along in split_samples.into_iter().take(MAX_SPLITS_PER_HOST) {
            if !exhaustive && *host_work >= per_host_budget {
                break 'splits;
            }
            let host_candidate_count = per_host.entry(host.road_id).or_default().len();
            if host_candidate_count >= per_host_budget {
                continue;
            }
            evaluated += 1;
            *host_work += 1;
            let host_t = along / host_len;
            if !(0.0..1.0).contains(&host_t) {
                continue;
            }
            let start = (host.ax + host_dx * host_t, host.az + host_dz * host_t);
            // A short, face-normal access corridor is a useful geometry-derived
            // alternative when individual destination rays terminate at nearby
            // perimeter junctions. Its endpoint stays inside the traced face.
            let inward_ray = ray_exit_distance(start, inward, &face.polygon);
            let mut headings = Vec::<((f32, f32), f32)>::new();
            // Do not discard a heading merely because a farther part of its
            // ray crosses a parcel. Final endpoint selection finds the longest
            // clear prefix and evaluates lengths within that prefix.
            if inward_ray >= MIN_BRANCH_M {
                headings.push((inward, f32::INFINITY));
            }
            for target in unserved_targets.iter().take(MAX_TARGETS_PER_SPLIT) {
                let delta = (target.0 - start.0, target.1 - start.1);
                let distance = crate::access_math::length(delta.0, delta.1);
                if !distance.is_finite() || distance < 15.0 || dot(inward, delta) <= 0.0 {
                    continue;
                }
                let direct = (delta.0 / distance, delta.1 / distance);
                let sides = crate::access_math::rotate_clearance_sides(
                    delta.0, delta.1, distance, 15.0,
                );
                let candidates = [
                    sides[0],
                    sides[1],
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
                    let target_along = dot(delta, direction);
                    if ray_length - FACE_EDGE_MARGIN_M < MIN_BRANCH_M
                        || target_along < FRONTAGE_TARGET_MARGIN_M
                    {
                        continue;
                    }
                    // A bounded heading slot must reach its target with a clear
                    // corridor. Exhaustive descriptors defer collision checks
                    // to their separate quota-stepped proposal validator.
                    let target_length = target_along + FRONTAGE_TARGET_MARGIN_M;
                    let safe_length = (ray_length - FACE_EDGE_MARGIN_M).min(MAX_BRANCH_M);
                    if !exhaustive && (target_length > safe_length
                        || !parcel_footprints.iter().all(|parcel| {
                            crate::geometry::rect_segment_distance(parcel, start.0, start.1,
                                start.0 + direction.0 * target_length,
                                start.1 + direction.1 * target_length) >= PARCEL_HOST_CLEARANCE_M
                        })) {
                        continue;
                    }
                    headings.push((direction, target_length));
                    if headings.len() == heading_limit {
                        break;
                    }
                }
                if headings.len() == heading_limit {
                    break;
                }
            }
            // If the direct sightline to a clear target is blocked, derive
            // nearby alternatives from the closest blocking parcel's corner
            // tangents. This is obstacle-driven (at most four corners and
            // eight rays per target), rather than a uniform angle sweep.
            if !exhaustive && headings.len() < 2 {
                for target in unserved_targets.iter().take(MAX_TARGETS_PER_SPLIT) {
                    if !exhaustive && *host_work >= per_host_budget {
                        break 'splits;
                    }
                    let Some(blocker) = parcel_footprints
                        .iter()
                        .map(|parcel| {
                            (
                                crate::geometry::rect_segment_distance(
                                    parcel, start.0, start.1, target.0, target.1,
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
                        let corner_distance = crate::access_math::length(dx, dz);
                        if corner_distance <= PARCEL_HOST_CLEARANCE_M + 0.5 {
                            continue;
                        }
                        let directions = crate::access_math::rotate_clearance_sides(
                            dx,
                            dz,
                            corner_distance,
                            PARCEL_HOST_CLEARANCE_M + 0.5,
                        );
                        for direction in directions {
                            if !exhaustive && *host_work >= per_host_budget {
                                break 'splits;
                            }
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
                            if ray_length < MIN_BRANCH_M {
                                continue;
                            }
                            headings.push((direction, f32::INFINITY));
                            if headings.len() == heading_limit {
                                break;
                            }
                        }
                        if headings.len() == heading_limit {
                            break;
                        }
                    }
                    if headings.len() == 2 && !exhaustive {
                        break;
                    }
                }
            }
            if exhaustive {
                // Keep the nearest four distinct blockers across all targets,
                // then test both tangents of each corner. This is a bounded
                // obstacle-derived set (<=32 headings), not a uniform angle grid.
                let mut blockers = Vec::<(f32, crate::geometry::OrientedRect)>::new();
                for parcel in parcel_footprints {
                    let nearest = unserved_targets
                        .iter()
                        .take(MAX_TARGETS_PER_SPLIT)
                        .map(|target| {
                            crate::geometry::rect_segment_distance(
                                parcel, start.0, start.1, target.0, target.1,
                            )
                        })
                        .fold(f32::INFINITY, f32::min);
                    if nearest >= PARCEL_HOST_CLEARANCE_M {
                        continue;
                    }
                    blockers.push((nearest, *parcel));
                    blockers.sort_by(|(da, a), (db, b)| {
                        da.total_cmp(db)
                            .then_with(|| a.x.total_cmp(&b.x))
                            .then_with(|| a.z.total_cmp(&b.z))
                            .then_with(|| a.width.total_cmp(&b.width))
                            .then_with(|| a.depth.total_cmp(&b.depth))
                            .then_with(|| a.rotation.total_cmp(&b.rotation))
                    });
                    blockers.truncate(4);
                }
                for (_, blocker) in blockers {
                    for corner in blocker.corners() {
                        let dx = corner.0 - start.0;
                        let dz = corner.1 - start.1;
                        let corner_distance = crate::access_math::length(dx, dz);
                        if corner_distance <= PARCEL_HOST_CLEARANCE_M + 0.5 {
                            continue;
                        }
                        let directions = crate::access_math::rotate_clearance_sides(
                            dx,
                            dz,
                            corner_distance,
                            PARCEL_HOST_CLEARANCE_M + 0.5,
                        );
                        for direction in directions {
                            if dot(inward, direction) <= 0.0
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
                            if ray_length < MIN_BRANCH_M {
                                continue;
                            }
                            headings.push((direction, f32::INFINITY));
                            if headings.len() == heading_limit {
                                break;
                            }
                        }
                        if headings.len() == heading_limit {
                            break;
                        }
                    }
                    if headings.len() == heading_limit {
                        break;
                    }
                }
            }
            for (heading, target_distance) in headings {
                let ray_length = ray_exit_distance(start, heading, &face.polygon);
                let ray_limit = (ray_length - FACE_EDGE_MARGIN_M).min(MAX_BRANCH_M);
                let safe_length = if exhaustive {
                    ray_limit
                } else {
                    max_clear_prefix(start, heading, ray_limit, parcel_footprints)
                };
                if safe_length < MIN_BRANCH_M {
                    continue;
                }
                let lengths = if target_distance.is_finite() && target_distance <= safe_length {
                    vec![
                        target_distance,
                        (target_distance - FRONTAGE_TARGET_MARGIN_M).max(MIN_BRANCH_M),
                        safe_length,
                    ]
                } else {
                    vec![MIN_BRANCH_M, 36.0, 48.0, safe_length]
                };
                let mut lengths = lengths
                    .into_iter()
                    .filter(|length| *length >= MIN_BRANCH_M && *length <= safe_length)
                    .collect::<Vec<_>>();
                lengths.sort_by(f32::total_cmp);
                lengths.dedup_by(|left, right| (*left - *right).abs() < 1.0e-4);
                for length in lengths {
                    if !exhaustive && *host_work >= per_host_budget {
                        break 'splits;
                    }
                    if per_host.get(&host.road_id).map_or(0, Vec::len) >= per_host_budget {
                        break;
                    }
                    evaluated += 1;
                    *host_work += 1;
                    let end = (start.0 + heading.0 * length, start.1 + heading.1 * length);
                    if segment_inside_polygon(start, end, &face.polygon)
                        && (exhaustive
                            || nodes
                                .iter()
                                .all(|node| (node.x - end.0).hypot(node.z - end.1) >= 18.0))
                        && (exhaustive
                            || parcel_footprints.iter().all(|parcel| {
                                crate::geometry::rect_segment_distance(
                                    parcel, start.0, start.1, end.0, end.1,
                                ) >= PARCEL_HOST_CLEARANCE_M
                            }))
                    {
                        per_host
                            .entry(host.road_id)
                            .or_default()
                            .push((host_t, end.0, end.1, length));
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
            (a.0 - b.0).abs() < 1.0e-5 && (a.1 - b.1).abs() < 1.0e-4 && (a.2 - b.2).abs() < 1.0e-4
        });
    }
    let host_ids = per_host.keys().copied().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut round = 0;
    while output.len() < descriptor_limit {
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
            if output.len() == descriptor_limit {
                break;
            }
        }
        if !added {
            break;
        }
        round += 1;
    }
    CandidateScan {
        candidates: output,
        work_units: evaluated,
    }
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

/// Reference host projection used by both the one-shot candidate builder and
/// the resumable preparation stage. Keep arithmetic and ordering here stable:
/// the access-search golden test compares its yielded split points against
/// this function.
pub(crate) fn host_split_samples(
    host: BoundaryHost,
    nodes: &[CandidateNode],
    parcel_footprints: &[crate::geometry::OrientedRect],
) -> Vec<f32> {
    let host_dx = host.bx - host.ax;
    let host_dz = host.bz - host.az;
    let host_len = crate::access_math::length(host_dx, host_dz);
    if !host_len.is_finite() || host_len < 2.0 * HOST_END_CLEARANCE_M {
        return Vec::new();
    }
    let mut forbidden = Vec::<(f32, f32)>::new();
    for node in nodes
        .iter()
        .filter(|node| node.id != host.from && node.id != host.to)
    {
        let relative_x = node.x - host.ax;
        let relative_z = node.z - host.az;
        let projection = (relative_x * host_dx + relative_z * host_dz) / host_len;
        let cross_distance = (relative_x * host_dz - relative_z * host_dx).abs() / host_len;
        if cross_distance < SPLIT_NODE_CLEARANCE_M {
            forbidden.push((
                projection - SPLIT_NODE_CLEARANCE_M,
                projection + SPLIT_NODE_CLEARANCE_M,
            ));
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
            end - span * 0.25,
            end - span / 6.0,
        ]);
        split_samples.extend(edge_split_samples(start, end, host_len));
    } else {
        free.sort_by(|left, right| {
            (right.1 - right.0)
                .total_cmp(&(left.1 - left.0))
                .then_with(|| left.0.total_cmp(&right.0))
        });
        let fragmented = free.len() > 1;
        let selected = free.into_iter().take(MAX_INTERVAL_SPLITS).collect::<Vec<_>>();
        split_samples.extend(
            selected
                .iter()
                .map(|(start, end)| (start + end) * 0.5),
        );
        // Preserve all historical midpoint candidates and their order. Use
        // the two remaining slots for deterministic obstacle-edge positions,
        // preferring the widest selected spans and falling through when an
        // edge is too close to an existing midpoint or accepted edge sample.
        if fragmented {
            let midpoint_count = split_samples.len();
            let spans = selected.iter().copied().collect::<Vec<_>>();
            let added_edges =
                append_fragmented_edge_samples(&mut split_samples, &spans, host_len);
            debug_assert_eq!(split_samples.len(), midpoint_count + added_edges);
        }
    }
    split_samples.truncate(MAX_SPLITS_PER_HOST);
    split_samples
}

/// Junctions hugging each end of a single free interval, appended after the
/// quartile samples so bounded scans reach them only with spare host budget.
/// An end at the host's own end clearance only needs node clearance; an end
/// at a parcel projection (already widened by its margin) or a node needs the
/// remaining parcel clearance so perpendicular branches can pass the parcel.
/// Samples that would crowd the quartile samples are omitted.
pub(crate) fn edge_split_samples(start: f32, end: f32, host_len: f32) -> Vec<f32> {
    let obstacle_inset = PARCEL_HOST_CLEARANCE_M - PARCEL_PROJECTION_MARGIN_M;
    let start_inset = if start == HOST_END_CLEARANCE_M {
        SPLIT_NODE_CLEARANCE_M
    } else {
        obstacle_inset
    };
    let end_inset = if end == host_len - HOST_END_CLEARANCE_M {
        SPLIT_NODE_CLEARANCE_M
    } else {
        obstacle_inset
    };
    let quartile = (end - start) * 0.25;
    let mut samples = Vec::with_capacity(MAX_EDGE_SPLITS);
    if quartile - start_inset >= 1.0 {
        samples.push(start + start_inset);
    }
    if quartile - end_inset >= 1.0 {
        samples.push(end - end_inset);
    }
    samples
}

/// Whether a fragmented-span edge sample crowds an existing split point.
/// Shared by the synchronous and resumable selectors to keep their decisions
/// identical while the resumable caller charges each comparison separately.
pub(crate) fn edge_sample_is_duplicate(sample: f32, existing: f32) -> bool {
    (sample - existing).abs() < 1.0
}

/// Synchronous reference selection for the at-most-two fragmented edge
/// samples. `samples` already contains the historical midpoint prefix.
pub(crate) fn append_fragmented_edge_samples(
    samples: &mut Vec<f32>,
    selected_spans: &[(f32, f32)],
    host_len: f32,
) -> usize {
    let mut added = 0;
    for &(start, end) in selected_spans {
        for edge in edge_split_samples(start, end, host_len) {
            if added >= MAX_EDGE_SPLITS {
                return added;
            }
            if samples
                .iter()
                .all(|old| !edge_sample_is_duplicate(edge, *old))
            {
                samples.push(edge);
                added += 1;
            }
        }
    }
    added
}

pub(crate) fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

pub(crate) fn junction_angle_ok(host: (f32, f32), branch: (f32, f32)) -> bool {
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
        if (az > point.1) != (bz > point.1) && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
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
            (
                start.0 + (end.0 - start.0) * t,
                start.1 + (end.1 - start.1) * t,
            ),
            polygon,
        )
    })
}

fn ray_exit_distance(start: (f32, f32), heading: (f32, f32), polygon: &[(f32, f32)]) -> f32 {
    let mut last_inside = 0.0;
    // Look one margin beyond the maximum allowed branch. If the face does not
    // end before then, callers can subtract the margin and still use a full
    // MAX_BRANCH_M segment instead of clipping every open ray to 58m.
    let max_steps = (MAX_RAY_LOOKAHEAD_M / RAY_SAMPLE_M).ceil() as usize;
    for step in 1..=max_steps {
        let distance = step as f32 * RAY_SAMPLE_M;
        let point = (
            start.0 + heading.0 * distance,
            start.1 + heading.1 * distance,
        );
        if !point_in_polygon(point, polygon) {
            return last_inside;
        }
        last_inside = distance;
    }
    MAX_RAY_LOOKAHEAD_M
}

/// Maximum branch prefix, capped by the face ray and branch length, that keeps
/// the entire centerline at the required clearance from every parcel. Segment
/// distance is monotone nonincreasing as a prefix grows, so binary search is
/// safe even when the remaining ray eventually meets an obstacle.
fn max_clear_prefix(
    start: (f32, f32),
    heading: (f32, f32),
    ray_limit: f32,
    parcels: &[crate::geometry::OrientedRect],
) -> f32 {
    let upper = ray_limit.min(MAX_BRANCH_M);
    if !upper.is_finite() || upper < MIN_BRANCH_M {
        return 0.0;
    }
    let clear = |length: f32| {
        parcels.iter().all(|parcel| {
            crate::geometry::rect_segment_distance(
                parcel,
                start.0,
                start.1,
                start.0 + heading.0 * length,
                start.1 + heading.1 * length,
            ) >= PARCEL_HOST_CLEARANCE_M
        })
    };
    if clear(upper) {
        return upper;
    }
    if !clear(MIN_BRANCH_M) {
        return 0.0;
    }
    let mut low = MIN_BRANCH_M;
    let mut high = upper;
    for _ in 0..12 {
        let middle = (low + high) * 0.5;
        if clear(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
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
            BoundaryHost {
                road_id: 10,
                from: 1,
                to: 2,
                ax: 0.0,
                az: 0.0,
                bx: 100.0,
                bz: 0.0,
            },
            BoundaryHost {
                road_id: 11,
                from: 2,
                to: 3,
                ax: 100.0,
                az: 0.0,
                bx: 100.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 12,
                from: 3,
                to: 4,
                ax: 100.0,
                az: 100.0,
                bx: 0.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 13,
                from: 4,
                to: 1,
                ax: 0.0,
                az: 100.0,
                bx: 0.0,
                bz: 0.0,
            },
        ];
        let nodes = vec![
            CandidateNode {
                id: 1,
                x: 0.0,
                z: 0.0,
            },
            CandidateNode {
                id: 2,
                x: 100.0,
                z: 0.0,
            },
            CandidateNode {
                id: 3,
                x: 100.0,
                z: 100.0,
            },
            CandidateNode {
                id: 4,
                x: 0.0,
                z: 100.0,
            },
        ];
        let first_scan = candidates_for_face_with_work(&face, &hosts, &nodes, &[], &[]);
        let first = first_scan.candidates.clone();
        assert!(!first.is_empty());
        assert!(first.len() <= MAX_PROPOSALS_PER_EVALUATION);
        assert!(first_scan.work_units <= MAX_PROPOSALS_PER_EVALUATION);
        let mut reversed_hosts = hosts.clone();
        reversed_hosts.reverse();
        let mut reversed_nodes = nodes.clone();
        reversed_nodes.reverse();
        let reordered =
            candidates_for_face_with_work(&face, &reversed_hosts, &reversed_nodes, &[], &[]);
        assert_eq!(
            first_scan, reordered,
            "stable candidate scan despite input ordering"
        );
        assert!(first
            .iter()
            .enumerate()
            .all(|(index, candidate)| candidate.tie_index as usize == index));
        assert!(
            first
                .iter()
                .map(|candidate| candidate.host_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == 4
        );
    }

    #[test]
    fn fourth_split_stays_inside_free_interval_away_from_parcel_end() {
        let host = BoundaryHost { road_id: 1, from: 1, to: 2, ax: 0.0, az: 0.0, bx: 100.0, bz: 0.0 };
        let parcel = crate::geometry::OrientedRect { x: 85.0, z: 0.0, width: 10.0, depth: 10.0, rotation: 0.0 };
        let samples = host_split_samples(host, &[], &[parcel]);
        // Parcel starts at 80m; its projection margin ends the free span at 78m.
        assert_eq!(samples[3], 78.0 - (78.0 - 10.0) / 6.0);
        assert!(crate::geometry::rect_segment_distance(&parcel, samples[3], 0.0, samples[3], 24.0) >= PARCEL_HOST_CLEARANCE_M);
    }

    #[test]
    fn edge_splits_hug_host_end_clearance_and_clear_parcels() {
        let host = BoundaryHost { road_id: 1, from: 1, to: 2, ax: 0.0, az: 0.0, bx: 100.0, bz: 0.0 };
        // Both ends bounded by host end clearance: 1m inside each.
        let open = host_split_samples(host, &[], &[]);
        assert_eq!(open[..MAX_INTERVAL_SPLITS], [30.0, 50.0, 70.0, 90.0 - 80.0 / 6.0]);
        assert_eq!(open[MAX_INTERVAL_SPLITS..], [11.0, 89.0]);
        // Far end bounded by a parcel projection: inset leaves parcel clearance.
        let parcel = crate::geometry::OrientedRect { x: 85.0, z: 0.0, width: 10.0, depth: 10.0, rotation: 0.0 };
        let blocked = host_split_samples(host, &[], &[parcel]);
        assert_eq!(blocked[MAX_INTERVAL_SPLITS..], [11.0, 75.0]);
        assert!(crate::geometry::rect_segment_distance(&parcel, 75.0, 0.0, 75.0, 24.0) >= PARCEL_HOST_CLEARANCE_M);
        // A 12m span keeps the 1m host-end sample but omits the 3m parcel-end
        // sample, which would land on the 3m quartile offset.
        let long_parcel = crate::geometry::OrientedRect { x: 61.0, z: 0.0, width: 74.0, depth: 10.0, rotation: 0.0 };
        let short = host_split_samples(host, &[], &[long_parcel]);
        assert_eq!(short[MAX_INTERVAL_SPLITS..], [11.0]);
    }

    #[test]
    fn fragmented_intervals_keep_midpoint_prefix_then_add_edge_entrances() {
        let host = BoundaryHost {
            road_id: 1,
            from: 1,
            to: 2,
            ax: 0.0,
            az: 0.0,
            bx: 100.0,
            bz: 0.0,
        };
        let nodes = [
            CandidateNode { id: 5, x: 35.0, z: 0.0 },
            CandidateNode { id: 6, x: 60.0, z: 0.0 },
        ];
        let samples = host_split_samples(host, &nodes, &[]);
        assert_eq!(&samples[..3], &[75.5, 22.0, 47.5]);
        assert_eq!(&samples[3..], &[64.0, 89.0]);
        assert_eq!(samples.len(), 5);
        assert!(samples.len() <= MAX_SPLITS_PER_HOST);
        // This entrance is 4m from node 6, within the existing split-node
        // clearance, while the previous midpoint prefix did not sample it.
        assert!(!samples[..3].contains(&64.0));
        assert!((64.0_f32 - nodes[1].x).abs() >= SPLIT_NODE_CLEARANCE_M);
    }

    #[test]
    fn two_fragmented_spans_keep_both_midpoints_before_two_edges() {
        let host = BoundaryHost {
            road_id: 1,
            from: 1,
            to: 2,
            ax: 0.0,
            az: 0.0,
            bx: 100.0,
            bz: 0.0,
        };
        let samples = host_split_samples(
            host,
            &[CandidateNode { id: 5, x: 50.0, z: 0.0 }],
            &[],
        );
        assert_eq!(&samples[..2], &[29.5, 70.5]);
        assert_eq!(&samples[2..], &[11.0, 46.0]);
        assert_eq!(samples.len(), 4);
    }

    #[test]
    fn fragmented_edge_start_has_a_clear_branch_ray_where_midpoints_are_blocked() {
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        };
        let host = BoundaryHost {
            road_id: 10,
            from: 1,
            to: 2,
            ax: 0.0,
            az: 0.0,
            bx: 100.0,
            bz: 0.0,
        };
        let node = CandidateNode {
            id: 5,
            x: 50.0,
            z: 0.0,
        };
        let blockers = [
            crate::geometry::OrientedRect {
                x: 29.5,
                z: 20.0,
                width: 20.0,
                depth: 20.0,
                rotation: 0.0,
            },
            crate::geometry::OrientedRect {
                x: 70.5,
                z: 20.0,
                width: 20.0,
                depth: 20.0,
                rotation: 0.0,
            },
        ];
        let split_points = host_split_samples(host, &[node], &blockers);
        assert_eq!(&split_points[..2], &[29.5, 70.5]);
        assert_eq!(&split_points[2..], &[11.0, 46.0]);
        let midpoint_ray_clear = |x: f32| {
            blockers.iter().all(|rect| {
                crate::geometry::rect_segment_distance(rect, x, 0.0, x, 24.0)
                    >= PARCEL_HOST_CLEARANCE_M
            })
        };
        assert!(!midpoint_ray_clear(split_points[0]));
        assert!(!midpoint_ray_clear(split_points[1]));
        assert!(midpoint_ray_clear(split_points[2]));
        assert!(midpoint_ray_clear(split_points[3]));

        let scan = candidates_for_face_with_work(&face, &[host], &[node], &blockers, &[]);
        assert!(scan.candidates.iter().any(|candidate| {
            (candidate.host_t - split_points[3] / 100.0).abs() < 1.0e-6
        }));
        assert!(!scan.candidates.iter().any(|candidate| {
            (candidate.host_t - split_points[0] / 100.0).abs() < 1.0e-6
                || (candidate.host_t - split_points[1] / 100.0).abs() < 1.0e-6
        }));
    }

    #[test]
    fn fragmented_edge_selection_skips_duplicates_and_falls_through() {
        let selected = [(10.0_f32, 30.0_f32), (10.0, 30.0), (40.0, 70.0)];
        // Synthetic existing prefix deliberately crowds the first two spans'
        // edge points, so the production helper must fall through to span 3.
        let mut samples = vec![11.0, 27.0, 55.0];
        let midpoint_prefix = samples.clone();
        let added = append_fragmented_edge_samples(&mut samples, &selected, 100.0);
        assert_eq!(&samples[..], &[11.0, 27.0, 55.0, 43.0, 67.0]);
        assert_eq!(&samples[..midpoint_prefix.len()], midpoint_prefix.as_slice());
        assert_eq!(added, MAX_EDGE_SPLITS);
        assert_eq!(samples.iter().filter(|sample| (**sample - 11.0).abs() < 1.0).count(), 1);
        assert!(samples.len() <= MAX_SPLITS_PER_HOST);
    }

    #[test]
    fn road_bounding_face_twice_scans_both_sides_within_one_allowance() {
        // Spur road 5 runs from the bottom edge into the face, so the face
        // boundary traverses it once on each side.
        let face = RoadFace {
            identity: vec![10, 11, 12, 13, 14, 15],
            vertex_ids: vec![10, 11, 12, 11, 13, 14, 15],
            polygon: vec![(0.0, 0.0), (50.0, 0.0), (50.0, 40.0), (50.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![1, 5, 5, 6, 2, 3, 4],
            contains_proposed_edge: false,
        };
        let hosts = [BoundaryHost { road_id: 5, from: 11, to: 12, ax: 50.0, az: 0.0, bx: 50.0, bz: 40.0 }];
        let scan = candidates_for_face_with_work(&face, &hosts, &[], &[], &[]);
        assert!(scan.candidates.iter().any(|c| c.end_x < 49.0), "west side: {scan:?}");
        assert!(scan.candidates.iter().any(|c| c.end_x > 51.0), "east side: {scan:?}");
        assert!(scan.work_units <= MAX_PROPOSALS_PER_EVALUATION);
    }

    #[test]
    fn unreachable_targets_do_not_change_bounded_candidate_slots() {
        let face = RoadFace { identity: vec![1,2,3,4], vertex_ids: vec![1,2,3,4], polygon: vec![(0.0,0.0),(100.0,0.0),(100.0,300.0),(0.0,300.0)], area: 30_000.0, boundary_edge_ids: vec![1,2,3,4], contains_proposed_edge: false };
        let hosts = [BoundaryHost { road_id:1, from:1, to:2, ax:0.0, az:0.0, bx:100.0, bz:0.0 }];
        let baseline = candidates_for_face_with_work(&face,&hosts,&[],&[],&[]);
        let distant = candidates_for_face_with_work(&face,&hosts,&[],&[],&[(50.0,200.0)]);
        assert_eq!(baseline,distant);
    }

    #[test]
    fn blocked_target_slots_leave_room_for_clear_corner_tangents() {
        let face = RoadFace { identity: vec![1,2,3,4], vertex_ids: vec![1,2,3,4], polygon: vec![(0.0,0.0),(100.0,0.0),(100.0,100.0),(0.0,100.0)], area: 10_000.0, boundary_edge_ids: vec![1,2,3,4], contains_proposed_edge: false };
        let hosts = [BoundaryHost { road_id:1, from:1, to:2, ax:0.0, az:0.0, bx:100.0, bz:0.0 }];
        let blocker = crate::geometry::OrientedRect { x:30.0,z:18.0,width:18.0,depth:8.0,rotation:0.0 };
        let scan = candidates_for_face_with_work(&face,&hosts,&[],&[blocker],&[(30.0,40.0)]);
        assert!(scan.candidates.iter().any(|c| (c.host_t-0.3).abs()<1e-6 && (c.end_x-30.0).abs()>1.0), "blocked split must retain a tangent: {:?}",scan);
        assert!(scan.work_units<=MAX_PROPOSALS_PER_EVALUATION);
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
        let host = BoundaryHost {
            road_id: 10,
            from: 1,
            to: 2,
            ax: 0.0,
            az: 0.0,
            bx: 100.0,
            bz: 0.0,
        };
        let hosts = vec![host];
        let nodes = vec![
            CandidateNode {
                id: 1,
                x: 0.0,
                z: 0.0,
            },
            CandidateNode {
                id: 2,
                x: 100.0,
                z: 0.0,
            },
            CandidateNode {
                id: 5,
                x: 50.0,
                z: 0.0,
            },
        ];
        let candidates = candidates_for_face(&face, &hosts, &nodes, &[], &[]);
        assert!(!candidates
            .iter()
            .any(|candidate| (candidate.host_t * 100.0 - 50.0).abs() < 1.0));
    }

    #[test]
    fn blocked_full_ray_keeps_longest_clear_prefix() {
        let parcel = crate::geometry::OrientedRect {
            x: 40.0,
            z: 0.0,
            width: 12.0,
            depth: 12.0,
            rotation: 0.0,
        };
        let prefix = max_clear_prefix((0.0, 0.0), (1.0, 0.0), 60.0, &[parcel]);
        assert!(
            prefix >= MIN_BRANCH_M,
            "a legal short prefix exists: {prefix}"
        );
        assert!(
            prefix <= 30.01,
            "prefix stops before parcel clearance: {prefix}"
        );
        assert!(
            crate::geometry::rect_segment_distance(&parcel, 0.0, 0.0, prefix, 0.0)
                >= PARCEL_HOST_CLEARANCE_M
        );
        assert!(
            crate::geometry::rect_segment_distance(&parcel, 0.0, 0.0, 60.0, 0.0)
                < PARCEL_HOST_CLEARANCE_M
        );
    }

    #[test]
    fn ray_limit_preserves_sixty_meters_and_face_margin() {
        let large_face = [
            (-100.0, -100.0),
            (100.0, -100.0),
            (100.0, 100.0),
            (-100.0, 100.0),
        ];
        let open_ray = ray_exit_distance((0.0, 0.0), (1.0, 0.0), &large_face);
        assert_eq!(open_ray, MAX_RAY_LOOKAHEAD_M);
        assert_eq!(
            (open_ray - FACE_EDGE_MARGIN_M).min(MAX_BRANCH_M),
            MAX_BRANCH_M
        );

        let near_face = [
            (-10.0, -100.0),
            (50.0, -100.0),
            (50.0, 100.0),
            (-10.0, 100.0),
        ];
        let boundary_ray = ray_exit_distance((0.0, 0.0), (1.0, 0.0), &near_face);
        assert_eq!(boundary_ray, 50.0, "a boundary sample is considered inside");
        assert_eq!((boundary_ray - FACE_EDGE_MARGIN_M).min(MAX_BRANCH_M), 48.0);
    }

    #[test]
    fn face_candidate_work_counts_prefilters_and_stays_bounded() {
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        };
        let hosts = vec![
            BoundaryHost {
                road_id: 10,
                from: 1,
                to: 2,
                ax: 0.0,
                az: 0.0,
                bx: 100.0,
                bz: 0.0,
            },
            BoundaryHost {
                road_id: 11,
                from: 2,
                to: 3,
                ax: 100.0,
                az: 0.0,
                bx: 100.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 12,
                from: 3,
                to: 4,
                ax: 100.0,
                az: 100.0,
                bx: 0.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 13,
                from: 4,
                to: 1,
                ax: 0.0,
                az: 100.0,
                bx: 0.0,
                bz: 0.0,
            },
        ];
        let nodes = vec![
            CandidateNode {
                id: 1,
                x: 0.0,
                z: 0.0,
            },
            CandidateNode {
                id: 2,
                x: 100.0,
                z: 0.0,
            },
            CandidateNode {
                id: 3,
                x: 100.0,
                z: 100.0,
            },
            CandidateNode {
                id: 4,
                x: 0.0,
                z: 100.0,
            },
        ];
        let scan = candidates_for_face_with_work(&face, &hosts, &nodes, &[], &[]);
        assert!(!scan.candidates.is_empty());
        assert!(scan.work_units <= MAX_PROPOSALS_PER_EVALUATION);
        assert!(scan.candidates.len() <= scan.work_units);
    }

    #[test]
    fn prepared_pages_concatenate_without_gaps_or_repeats_and_ignore_input_order() {
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (120.0, 0.0), (120.0, 100.0), (0.0, 100.0)],
            area: 12_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        };
        let hosts = vec![
            BoundaryHost {
                road_id: 10,
                from: 1,
                to: 2,
                ax: 0.0,
                az: 0.0,
                bx: 120.0,
                bz: 0.0,
            },
            BoundaryHost {
                road_id: 11,
                from: 2,
                to: 3,
                ax: 120.0,
                az: 0.0,
                bx: 120.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 12,
                from: 3,
                to: 4,
                ax: 120.0,
                az: 100.0,
                bx: 0.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 13,
                from: 4,
                to: 1,
                ax: 0.0,
                az: 100.0,
                bx: 0.0,
                bz: 0.0,
            },
        ];
        let nodes = vec![
            CandidateNode {
                id: 1,
                x: 0.0,
                z: 0.0,
            },
            CandidateNode {
                id: 2,
                x: 120.0,
                z: 0.0,
            },
            CandidateNode {
                id: 3,
                x: 120.0,
                z: 100.0,
            },
            CandidateNode {
                id: 4,
                x: 0.0,
                z: 100.0,
            },
        ];
        let parcels = vec![
            crate::geometry::OrientedRect {
                x: -30.0,
                z: -25.0,
                width: 8.0,
                depth: 10.0,
                rotation: 0.3,
            },
            crate::geometry::OrientedRect {
                x: 150.0,
                z: 130.0,
                width: 12.0,
                depth: 8.0,
                rotation: -0.2,
            },
        ];
        let targets = vec![
            (30.0, 30.0),
            (45.0, 35.0),
            (60.0, 42.0),
            (75.0, 48.0),
            (90.0, 55.0),
            (35.0, 70.0),
            (55.0, 75.0),
            (80.0, 80.0),
        ];
        let prepared = PreparedAccessSearch::prepare(&face, &hosts, &nodes, &parcels, &targets);
        assert!(prepared.len() > MAX_PROPOSALS_PER_EVALUATION);
        assert!(prepared.len() <= MAX_PREPARED_DESCRIPTORS);

        let mut page_cursor = 0usize;
        let mut paged = Vec::new();
        loop {
            let page = prepared.page(page_cursor);
            assert!(page.work_units <= MAX_PROPOSALS_PER_EVALUATION);
            assert_eq!(page.work_units, page.candidates.len());
            assert!(page.next_cursor >= page_cursor);
            assert_eq!(page.exhausted, page.next_cursor == prepared.len());
            paged.extend(page.candidates);
            page_cursor = page.next_cursor;
            if page.exhausted {
                break;
            }
        }
        assert_eq!(page_cursor, prepared.len());
        let identity = |candidate: &AccessGeometryCandidate| {
            (
                candidate.host_id,
                candidate.host_t.to_bits(),
                candidate.end_x.to_bits(),
                candidate.end_z.to_bits(),
            )
        };
        assert_eq!(
            paged.iter().map(identity).collect::<Vec<_>>(),
            prepared
                .descriptors
                .iter()
                .map(identity)
                .collect::<Vec<_>>(),
            "paging exactly covers the prepared descriptor sequence",
        );
        let unique = paged
            .iter()
            .map(identity)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            unique.len(),
            paged.len(),
            "prepared descriptors contain no duplicate geometry"
        );
        assert!(paged
            .windows(2)
            .all(|pair| { identity(&pair[0]) != identity(&pair[1]) }));

        let mut reversed_hosts = hosts.clone();
        reversed_hosts.reverse();
        let mut reversed_nodes = nodes.clone();
        reversed_nodes.reverse();
        let mut reversed_targets = targets.clone();
        reversed_targets.reverse();
        let mut reversed_parcels = parcels.clone();
        reversed_parcels.reverse();
        let reordered = PreparedAccessSearch::prepare(
            &face,
            &reversed_hosts,
            &reversed_nodes,
            &reversed_parcels,
            &reversed_targets,
        );
        assert_eq!(
            prepared, reordered,
            "preparation is stable under equivalent input order"
        );
        let mut reordered_paged = Vec::new();
        let mut cursor = 0;
        loop {
            let page = reordered.page(cursor);
            reordered_paged.extend(page.candidates);
            cursor = page.next_cursor;
            if page.exhausted {
                break;
            }
        }
        assert_eq!(
            paged.iter().map(identity).collect::<Vec<_>>(),
            reordered_paged.iter().map(identity).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn prepared_pages_are_directly_indexed_and_out_of_range_is_exhausted() {
        let search = PreparedAccessSearch {
            descriptors: (0..100)
                .map(|index| AccessGeometryCandidate {
                    host_id: 7,
                    host_t: index as f32 / 100.0,
                    end_x: index as f32,
                    end_z: 0.0,
                    tie_index: 0,
                })
                .collect(),
            preparation_counted_units: 123,
        };
        let page = search.page(49);
        assert_eq!(page.candidates.len(), 48);
        assert_eq!(page.candidates[0].end_x, 49.0);
        assert_eq!(page.next_cursor, 97);
        assert!(!page.exhausted);
        let last = search.page(page.next_cursor);
        assert_eq!(
            last.candidates
                .iter()
                .map(|candidate| candidate.end_x)
                .collect::<Vec<_>>(),
            vec![97.0, 98.0, 99.0]
        );
        assert!(last.exhausted);
        let past_end = search.page(1000);
        assert!(past_end.candidates.is_empty());
        assert_eq!(past_end.next_cursor, 100);
        assert!(past_end.exhausted);
    }

    #[test]
    fn prepared_search_rejects_oversized_target_input_without_sorting_it() {
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        };
        let targets = vec![(50.0, 50.0); MAX_TARGETS_PER_SPLIT + 1];
        let search = PreparedAccessSearch::prepare(&face, &[], &[], &[], &targets);
        assert_eq!(search.len(), 0);
        assert_eq!(search.preparation_counted_units, 0);
    }
}
