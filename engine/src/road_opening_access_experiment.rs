use crate::*;
use serde_json::json;

#[test]
fn face_filter_accepts_inside_plan_and_rejects_external_branch_or_lot() {
    let polygon = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    let host = Road {
        id: 1,
        from: 10,
        to: 11,
        kind: "local".into(),
        stage: "open".into(),
        progress: 1.0,
        connected: true,
        length: 100.0,
        lanes: 2,
    };
    let from = Node {
        id: 10,
        x: 0.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let to = Node {
        id: 11,
        x: 100.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let proposal = interior_access::AccessProposal {
        score: interior_access::CandidateScore {
            site_area: 2_000_000,
            road_length_mm: 48_000,
            host_id: 1,
            fraction_index: 1,
            side_index: 1,
            length_index: 1,
        },
        host_id: 1,
        host_t: 0.5,
        end_x: 50.0,
        end_z: 80.0,
        selected_lots: vec![
            interior_access::InteriorAccessLot {
                x: 30.0,
                z: 50.0,
                width: 12.0,
                depth: 12.0,
                rotation: 0.0,
                along: 0.25,
                side: -1,
            },
            interior_access::InteriorAccessLot {
                x: 70.0,
                z: 50.0,
                width: 12.0,
                depth: 12.0,
                rotation: 0.0,
                along: 0.75,
                side: 1,
            },
        ],
    };
    assert!(proposal_fits_face(&proposal, &host, &from, &to, &polygon));

    let mut external_branch = proposal.clone();
    external_branch.end_z = 120.0;
    assert!(!proposal_fits_face(
        &external_branch,
        &host,
        &from,
        &to,
        &polygon
    ));

    let mut external_lot = proposal;
    external_lot.selected_lots[0].x = 110.0;
    assert!(!proposal_fits_face(
        &external_lot,
        &host,
        &from,
        &to,
        &polygon
    ));
}

#[test]
fn face_filter_keeps_only_contained_lots_and_requires_two_sites() {
    let polygon = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    let host = Road {
        id: 1,
        from: 10,
        to: 11,
        kind: "local".into(),
        stage: "open".into(),
        progress: 1.0,
        connected: true,
        length: 100.0,
        lanes: 2,
    };
    let from = Node {
        id: 10,
        x: 0.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let to = Node {
        id: 11,
        x: 100.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let lot = |x, z| interior_access::InteriorAccessLot {
        x,
        z,
        width: 10.0,
        depth: 10.0,
        rotation: 0.0,
        along: 0.5,
        side: 1,
    };
    let proposal = interior_access::AccessProposal {
        score: interior_access::CandidateScore {
            site_area: 3_000_000,
            road_length_mm: 48_000,
            host_id: 1,
            fraction_index: 1,
            side_index: 1,
            length_index: 1,
        },
        host_id: 1,
        host_t: 0.5,
        end_x: 50.0,
        end_z: 80.0,
        selected_lots: vec![lot(25.0, 50.0), lot(70.0, 50.0), lot(110.0, 50.0)],
    };
    let retained = retain_face_lots(&proposal, &host, &from, &to, &polygon).unwrap();
    assert_eq!(retained.selected_lots.len(), 2);
    assert_eq!(retained.score.site_area, 2_000_000);
    assert!(retained.selected_lots.iter().all(|lot| {
        rect_within_face(
            geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            },
            &polygon,
        )
    }));

    let one_site = interior_access::AccessProposal {
        selected_lots: vec![lot(25.0, 50.0), lot(110.0, 50.0)],
        ..proposal
    };
    assert!(retain_face_lots(&one_site, &host, &from, &to, &polygon).is_none());
}

#[test]
fn blocked_closing_edge_yields_to_bounded_alternate_face_boundary_host() {
    let polygon = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    let face = crate::road_faces::RoadFace {
        identity: vec![10, 20, 30, 40],
        vertex_ids: vec![10, 20, 30, 40],
        polygon: polygon.clone(),
        area: 10_000.0,
        boundary_edge_ids: vec![1, 2, 3, 4],
        contains_proposed_edge: false,
    };
    let eligible = [1, 2, 3, 4].into_iter().collect();
    let order = ordered_face_boundary_hosts(1, std::slice::from_ref(&face), &eligible);
    assert_eq!(order, vec![1, 2, 3, 4]);
    let bounded = order.iter().take(2).copied().collect::<Vec<_>>();
    assert_eq!(bounded, vec![1, 2]);
    assert_eq!(bounded, vec![1, 2]);
    assert_eq!(
        ordered_face_boundary_hosts(1, std::slice::from_ref(&face), &eligible),
        order,
        "host order is stable across repeated evaluations"
    );

    // The newly opened bottom edge's proposal heads out of the bounded face.
    let bottom = Road {
        id: 1,
        from: 10,
        to: 20,
        kind: "local".into(),
        stage: "open".into(),
        progress: 1.0,
        connected: true,
        length: 100.0,
        lanes: 2,
    };
    let bottom_a = Node {
        id: 10,
        x: 0.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let bottom_b = Node {
        id: 20,
        x: 100.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let blocked = interior_access::AccessProposal {
        score: interior_access::CandidateScore {
            site_area: 2_000_000,
            road_length_mm: 48_000,
            host_id: 1,
            fraction_index: 1,
            side_index: 1,
            length_index: 1,
        },
        host_id: 1,
        host_t: 0.5,
        end_x: 50.0,
        end_z: -80.0,
        selected_lots: vec![
            interior_access::InteriorAccessLot {
                x: 30.0,
                z: -40.0,
                width: 12.0,
                depth: 12.0,
                rotation: 0.0,
                along: 0.25,
                side: -1,
            },
            interior_access::InteriorAccessLot {
                x: 70.0,
                z: -40.0,
                width: 12.0,
                depth: 12.0,
                rotation: 0.0,
                along: 0.75,
                side: 1,
            },
        ],
    };
    assert!(retain_face_lots(&blocked, &bottom, &bottom_a, &bottom_b, &polygon).is_none());

    // The next boundary edge is a viable inward-facing frontage for the same
    // newly bounded face. This is the production scan's alternate-host path.
    let right = Road {
        id: 2,
        from: 20,
        to: 30,
        kind: "local".into(),
        stage: "open".into(),
        progress: 1.0,
        connected: true,
        length: 100.0,
        lanes: 2,
    };
    let right_a = Node {
        id: 20,
        x: 100.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let right_b = Node {
        id: 30,
        x: 100.0,
        z: 100.0,
        kind: "junction".into(),
    };
    let usable = interior_access::AccessProposal {
        score: interior_access::CandidateScore {
            site_area: 2_000_000,
            road_length_mm: 48_000,
            host_id: 2,
            fraction_index: 1,
            side_index: 1,
            length_index: 1,
        },
        host_id: 2,
        host_t: 0.5,
        end_x: 20.0,
        end_z: 50.0,
        selected_lots: vec![
            interior_access::InteriorAccessLot {
                x: 70.0,
                z: 35.0,
                width: 12.0,
                depth: 12.0,
                rotation: std::f32::consts::FRAC_PI_2,
                along: 0.25,
                side: 1,
            },
            interior_access::InteriorAccessLot {
                x: 70.0,
                z: 65.0,
                width: 12.0,
                depth: 12.0,
                rotation: std::f32::consts::FRAC_PI_2,
                along: 0.75,
                side: 1,
            },
        ],
    };
    let selected = retain_face_lots(&usable, &right, &right_a, &right_b, &polygon)
        .expect("alternate boundary proposal fits face");
    assert_eq!(selected.host_id, 2);
    assert_eq!(selected.selected_lots.len(), 2);
}

#[test]
fn face_candidate_window_round_robins_over_boundary_with_global_budget() {
    let host_sides = (100..124)
        .map(|id| (id, if id % 2 == 0 { 1 } else { -1 }))
        .collect::<Vec<_>>();
    let first = round_robin_face_candidates(&host_sides);
    let second = round_robin_face_candidates(&host_sides);
    assert_eq!(first, second, "candidate allocation is deterministic");
    assert_eq!(first.len(), FACE_ACCESS_MAX_PROPOSALS);
    let mut counts = std::collections::BTreeMap::<u32, usize>::new();
    for candidate in &first {
        *counts.entry(candidate.host_id).or_default() += 1;
    }
    assert_eq!(counts.len(), 24, "every eligible boundary side is sampled");
    assert!(counts.values().all(|count| *count == 2));
    assert!(first.iter().all(|candidate| {
        candidate.fraction_index < interior_access::FRACTIONS.len() as u8
            && candidate.length_index < interior_access::LENGTHS.len() as u8
            && candidate.side.abs() == 1
    }));

    let smaller = round_robin_face_candidates(&host_sides[..3]);
    assert_eq!(smaller.len(), 36);
    let mut smaller_counts = std::collections::BTreeMap::<u32, usize>::new();
    for candidate in &smaller {
        *smaller_counts.entry(candidate.host_id).or_default() += 1;
    }
    assert!(smaller_counts.values().all(|count| *count == 12));
}

#[test]
fn explicit_host_window_matches_legacy_wrapper_geometry_and_order() {
    let mut engine = Engine::restore_json(include_str!(
        "../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"
    ))
    .expect("mature save restores");
    engine.state.demand = 4.0;
    engine
        .state
        .projects
        .retain(|project| project.progress >= 1.0);
    let host_id = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
        })
        .map(|road| road.id)
        .find(|host_id| {
            !engine
                .propose_interior_access_for_host(*host_id)
                .proposals
                .is_empty()
        })
        .expect("fixture has a feasible access host");

    let legacy = engine.propose_interior_access_for_host(host_id);
    let mut explicit = Vec::with_capacity(interior_access::PROPOSALS_PER_HOST);
    for fraction_index in 0..interior_access::FRACTIONS.len() {
        for side in interior_access::SIDES {
            for length_index in 0..interior_access::LENGTHS.len() {
                explicit.push(interior_access::AccessCandidate {
                    host_id,
                    fraction_index: fraction_index as u8,
                    side,
                    length_index: length_index as u8,
                });
            }
        }
    }
    let candidate_scan = engine.propose_interior_access_from_candidates(None, 0, Some(&explicit));
    assert_eq!(candidate_scan.proposals, legacy.proposals);
    assert_eq!(candidate_scan.next_cursor, legacy.next_cursor);
}

#[test]
fn face_containment_rejects_segment_and_lot_crossing_concave_notch() {
    let concave = vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 40.0),
        (40.0, 40.0),
        (40.0, 100.0),
        (0.0, 100.0),
    ];
    let start = WorldPoint { x: 10.0, z: 90.0 };
    let end = WorldPoint { x: 90.0, z: 10.0 };
    assert!(point_in_or_on_polygon(start, &concave));
    assert!(point_in_or_on_polygon(end, &concave));
    assert!(!segment_within_face(start, end, &concave));

    let crossing_rect = geometry::OrientedRect {
        x: 40.0,
        z: 50.0,
        width: 24.0,
        depth: 24.0,
        rotation: 0.0,
    };
    assert!(!rect_within_face(crossing_rect, &concave));
}

#[test]
fn shallow_or_fully_served_faces_do_not_qualify_for_access() {
    let thin_face = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 20.0), (0.0, 20.0)];
    let bottom_a = Node {
        id: 1,
        x: 0.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let bottom_b = Node {
        id: 2,
        x: 100.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let top_a = Node {
        id: 3,
        x: 0.0,
        z: 20.0,
        kind: "junction".into(),
    };
    let top_b = Node {
        id: 4,
        x: 100.0,
        z: 20.0,
        kind: "junction".into(),
    };
    let bottom = Road {
        id: 1,
        from: 1,
        to: 2,
        kind: "local".into(),
        stage: "open".into(),
        progress: 1.0,
        connected: true,
        length: 100.0,
        lanes: 2,
    };
    let top = Road {
        id: 2,
        from: 4,
        to: 3,
        ..bottom.clone()
    };
    let roads = vec![(&bottom, &bottom_a, &bottom_b), (&top, &top_b, &top_a)];
    assert!(!face_has_unserved_interior(&thin_face, &roads, &[]));

    let roomy = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    let left_a = Node {
        id: 5,
        x: 0.0,
        z: 100.0,
        kind: "junction".into(),
    };
    let left_b = Node {
        id: 6,
        x: 0.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let right_a = Node {
        id: 7,
        x: 100.0,
        z: 0.0,
        kind: "junction".into(),
    };
    let right_b = Node {
        id: 8,
        x: 100.0,
        z: 100.0,
        kind: "junction".into(),
    };
    let left = Road {
        id: 3,
        from: 5,
        to: 6,
        ..bottom.clone()
    };
    let right = Road {
        id: 4,
        from: 7,
        to: 8,
        ..bottom.clone()
    };
    let perimeter = vec![
        (&bottom, &bottom_a, &bottom_b),
        (&top, &top_b, &top_a),
        (&left, &left_a, &left_b),
        (&right, &right_a, &right_b),
    ];
    let covering_parcel = geometry::OrientedRect {
        x: 50.0,
        z: 50.0,
        width: 96.0,
        depth: 96.0,
        rotation: 0.0,
    };
    assert!(!face_has_unserved_interior(
        &roomy,
        &perimeter,
        &[covering_parcel]
    ));
}

const FACE_SAMPLE_GRID_SIDE: usize = 16;
const FACE_MIN_UNSERVED_SAMPLES: usize = 4;
const FACE_MIN_ROAD_EDGE_CLEARANCE: f32 = 12.0;
const FACE_MIN_PARCEL_CLEARANCE: f32 = 2.2;
const FACE_POINT_EPSILON: f32 = 1.0e-4;
const FACE_ACCESS_MAX_PROPOSALS: usize = 48;
const FACE_ACCESS_MAX_HOST_SIDE_PAIRS: usize = 24;

fn point_in_or_on_polygon(point: WorldPoint, polygon: &[(f32, f32)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    for index in 0..polygon.len() {
        let (ax, az) = polygon[index];
        let (bx, bz) = polygon[(index + 1) % polygon.len()];
        if geometry::point_segment_distance(point.x, point.z, ax, az, bx, bz) <= FACE_POINT_EPSILON
        {
            return true;
        }
        if (az > point.z) != (bz > point.z) && point.x < (bx - ax) * (point.z - az) / (bz - az) + ax
        {
            inside = !inside;
        }
    }
    inside
}

/// Checks a whole segment, including boundary intersections. Only contact at
/// its initial point is allowed because an access branch begins on its host
/// face edge. Sampling catches boundary-collinear departure; intersection
/// checks catch narrow concave excursions that a sample grid could miss.
fn segment_within_face(start: WorldPoint, end: WorldPoint, polygon: &[(f32, f32)]) -> bool {
    if !point_in_or_on_polygon(start, polygon) || !point_in_or_on_polygon(end, polygon) {
        return false;
    }
    for sample in 1..=32 {
        let t = sample as f32 / 32.0;
        let point = WorldPoint {
            x: start.x + (end.x - start.x) * t,
            z: start.z + (end.z - start.z) * t,
        };
        if !point_in_or_on_polygon(point, polygon) {
            return false;
        }
    }
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        if geometry::segments_distance_coords(start.x, start.z, end.x, end.z, a.0, a.1, b.0, b.1)
            <= FACE_POINT_EPSILON
        {
            let only_start = geometry::point_segment_distance(start.x, start.z, a.0, a.1, b.0, b.1)
                <= FACE_POINT_EPSILON;
            let dx1 = end.x - start.x;
            let dz1 = end.z - start.z;
            let dx2 = b.0 - a.0;
            let dz2 = b.1 - a.1;
            let collinear = (dx1 * dz2 - dz1 * dx2).abs()
                <= FACE_POINT_EPSILON * dx1.hypot(dz1) * dx2.hypot(dz2);
            if !only_start || collinear {
                return false;
            }
        }
    }
    true
}

fn rect_within_face(rect: geometry::OrientedRect, polygon: &[(f32, f32)]) -> bool {
    if !rect.finite_positive() {
        return false;
    }
    let corners = rect.corners();
    if corners
        .iter()
        .any(|&(x, z)| !point_in_or_on_polygon(WorldPoint { x, z }, polygon))
    {
        return false;
    }
    for index in 0..4 {
        let a = corners[index];
        let b = corners[(index + 1) % 4];
        if !segment_within_face(
            WorldPoint { x: a.0, z: a.1 },
            WorldPoint { x: b.0, z: b.1 },
            polygon,
        ) {
            return false;
        }
    }
    true
}

fn proposal_fits_face(
    proposal: &interior_access::AccessProposal,
    host: &Road,
    host_from: &Node,
    host_to: &Node,
    polygon: &[(f32, f32)],
) -> bool {
    let start = WorldPoint {
        x: host_from.x + (host_to.x - host_from.x) * proposal.host_t,
        z: host_from.z + (host_to.z - host_from.z) * proposal.host_t,
    };
    if proposal.host_id != host.id
        || !segment_within_face(
            start,
            WorldPoint {
                x: proposal.end_x,
                z: proposal.end_z,
            },
            polygon,
        )
    {
        return false;
    }
    proposal.selected_lots.iter().all(|lot| {
        rect_within_face(
            geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            },
            polygon,
        )
    })
}

fn retain_face_lots(
    proposal: &interior_access::AccessProposal,
    host: &Road,
    host_from: &Node,
    host_to: &Node,
    polygon: &[(f32, f32)],
) -> Option<interior_access::AccessProposal> {
    let start = WorldPoint {
        x: host_from.x + (host_to.x - host_from.x) * proposal.host_t,
        z: host_from.z + (host_to.z - host_from.z) * proposal.host_t,
    };
    if proposal.host_id != host.id
        || !segment_within_face(
            start,
            WorldPoint {
                x: proposal.end_x,
                z: proposal.end_z,
            },
            polygon,
        )
    {
        return None;
    }
    let selected_lots = proposal
        .selected_lots
        .iter()
        .copied()
        .filter(|lot| {
            rect_within_face(
                geometry::OrientedRect {
                    x: lot.x,
                    z: lot.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                },
                polygon,
            )
        })
        .collect::<Vec<_>>();
    if selected_lots.len() < 2 {
        return None;
    }
    let mut retained = proposal.clone();
    retained.score.site_area = selected_lots
        .iter()
        .map(|lot| (lot.width * lot.depth * 10_000.0).round() as u32)
        .sum();
    retained.selected_lots = selected_lots;
    Some(retained)
}

fn ordered_face_boundary_hosts(
    trigger_id: u32,
    faces: &[crate::road_faces::RoadFace],
    eligible_ids: &std::collections::BTreeSet<u32>,
) -> Vec<u32> {
    let mut ordered = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for face in faces {
        let mut boundary = face
            .boundary_edge_ids
            .iter()
            .copied()
            .filter(|id| eligible_ids.contains(id))
            .collect::<Vec<_>>();
        let mut local_seen = std::collections::BTreeSet::new();
        boundary.retain(|id| local_seen.insert(*id));
        if boundary.contains(&trigger_id) && seen.insert(trigger_id) {
            ordered.push(trigger_id);
        }
        let Some(trigger_index) = boundary.iter().position(|id| *id == trigger_id) else {
            continue;
        };
        for offset in 1..boundary.len() {
            let id = boundary[(trigger_index + offset) % boundary.len()];
            if seen.insert(id) {
                ordered.push(id);
            }
        }
    }
    ordered
}

fn inward_sides_for_face(host_from: &Node, host_to: &Node, polygon: &[(f32, f32)]) -> Vec<i8> {
    let dx = host_to.x - host_from.x;
    let dz = host_to.z - host_from.z;
    let length = dx.hypot(dz);
    if !length.is_finite() || length <= 1.0e-4 {
        return Vec::new();
    }
    let nx = -dz / length;
    let nz = dx / length;
    let mut scores = [0usize; 2];
    for fraction in [0.25_f32, 0.5, 0.75] {
        let x = host_from.x + dx * fraction;
        let z = host_from.z + dz * fraction;
        for (index, side) in [-1_i8, 1].into_iter().enumerate() {
            // Two small probes avoid misclassifying a concave boundary from a
            // single midpoint while keeping boundary-side work constant.
            for offset in [1.0_f32, 6.0] {
                if point_in_or_on_polygon(
                    WorldPoint {
                        x: x + nx * side as f32 * offset,
                        z: z + nz * side as f32 * offset,
                    },
                    polygon,
                ) {
                    scores[index] += 1;
                }
            }
        }
    }
    match scores {
        [0, 0] => Vec::new(),
        [left, right] if left == right => vec![-1, 1],
        [left, right] if left > right => vec![-1],
        _ => vec![1],
    }
}

fn round_robin_face_candidates(host_sides: &[(u32, i8)]) -> Vec<interior_access::AccessCandidate> {
    let mut unique = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for &(host_id, side) in host_sides {
        if (side == -1 || side == 1) && seen.insert((host_id, side)) {
            unique.push((host_id, side));
        }
    }
    unique.truncate(FACE_ACCESS_MAX_HOST_SIDE_PAIRS);
    if unique.is_empty() {
        return Vec::new();
    }
    let per_pair = FACE_ACCESS_MAX_PROPOSALS / unique.len();
    let extra = FACE_ACCESS_MAX_PROPOSALS % unique.len();
    let quotas = (0..unique.len())
        .map(|index| {
            (per_pair + usize::from(index < extra))
                .min(interior_access::FRACTIONS.len() * interior_access::LENGTHS.len())
        })
        .collect::<Vec<_>>();
    let max_rounds = quotas.iter().copied().max().unwrap_or(0);
    let mut candidates = Vec::with_capacity(FACE_ACCESS_MAX_PROPOSALS.min(unique.len() * 12));
    for round in 0..max_rounds {
        for (pair_index, &(host_id, side)) in unique.iter().enumerate() {
            let quota = quotas[pair_index];
            if round >= quota {
                continue;
            }
            // Spread a partial allocation over the complete 3×4 fraction /
            // length grid instead of spending every host's share at the first
            // short length and first quarter point.
            let local =
                round * interior_access::FRACTIONS.len() * interior_access::LENGTHS.len() / quota;
            candidates.push(interior_access::AccessCandidate {
                host_id,
                fraction_index: (local / interior_access::LENGTHS.len()) as u8,
                side,
                length_index: (local % interior_access::LENGTHS.len()) as u8,
            });
        }
    }
    candidates
}

fn point_to_rect_distance(point: WorldPoint, rect: geometry::OrientedRect) -> f32 {
    // Rust returns (sin, cos); use the inverse of OrientedRect's local-to-world
    // Three.js Y transform (local X -> (cos,-sin), local Z -> (sin,cos)).
    let cos = rect.rotation.cos();
    let sin = rect.rotation.sin();
    let dx = point.x - rect.x;
    let dz = point.z - rect.z;
    let local_x = dx * cos - dz * sin;
    let local_z = dx * sin + dz * cos;
    let outside_x = (local_x.abs() - rect.width * 0.5).max(0.0);
    let outside_z = (local_z.abs() - rect.depth * 0.5).max(0.0);
    outside_x.hypot(outside_z)
}

#[test]
fn point_to_oriented_rect_distance_matches_local_axes_at_zero_right_and_oblique_angles() {
    let rect = geometry::OrientedRect {
        x: 0.0,
        z: 0.0,
        width: 20.0,
        depth: 4.0,
        rotation: 0.0,
    };
    assert_eq!(
        point_to_rect_distance(WorldPoint { x: 8.0, z: 0.0 }, rect),
        0.0
    );
    assert_eq!(
        point_to_rect_distance(WorldPoint { x: 0.0, z: 4.0 }, rect),
        2.0
    );

    let right_angle = geometry::OrientedRect {
        rotation: std::f32::consts::FRAC_PI_2,
        ..rect
    };
    assert_eq!(
        point_to_rect_distance(WorldPoint { x: 0.0, z: -8.0 }, right_angle),
        0.0
    );
    assert_eq!(
        point_to_rect_distance(WorldPoint { x: 4.0, z: 0.0 }, right_angle),
        2.0
    );

    let angle = 0.71_f32;
    let oblique = geometry::OrientedRect {
        rotation: angle,
        ..rect
    };
    let world_x = angle.cos() * 13.0;
    let world_z = -angle.sin() * 13.0;
    assert!(
        (point_to_rect_distance(
            WorldPoint {
                x: world_x,
                z: world_z
            },
            oblique
        ) - 3.0)
            .abs()
            < 1e-5
    );
}

fn face_has_unserved_interior(
    polygon: &[(f32, f32)],
    roads: &[(&Road, &Node, &Node)],
    parcels: &[geometry::OrientedRect],
) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let min_x = polygon
        .iter()
        .map(|point| point.0)
        .fold(f32::INFINITY, f32::min);
    let max_x = polygon
        .iter()
        .map(|point| point.0)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_z = polygon
        .iter()
        .map(|point| point.1)
        .fold(f32::INFINITY, f32::min);
    let max_z = polygon
        .iter()
        .map(|point| point.1)
        .fold(f32::NEG_INFINITY, f32::max);
    if ![min_x, max_x, min_z, max_z].into_iter().all(f32::is_finite)
        || max_x <= min_x
        || max_z <= min_z
    {
        return false;
    }
    let mut unserved = 0usize;
    for iz in 0..FACE_SAMPLE_GRID_SIDE {
        for ix in 0..FACE_SAMPLE_GRID_SIDE {
            let point = WorldPoint {
                x: min_x + (ix as f32 + 0.5) * (max_x - min_x) / FACE_SAMPLE_GRID_SIDE as f32,
                z: min_z + (iz as f32 + 0.5) * (max_z - min_z) / FACE_SAMPLE_GRID_SIDE as f32,
            };
            if !point_in_or_on_polygon(point, polygon) {
                continue;
            }
            let away_from_roads = roads.iter().all(|(road, start, end)| {
                let half_width = if road.kind == "arterial" { 7.0 } else { 4.5 };
                geometry::point_segment_distance(point.x, point.z, start.x, start.z, end.x, end.z)
                    - half_width
                    >= FACE_MIN_ROAD_EDGE_CLEARANCE
            });
            if !away_from_roads {
                continue;
            }
            let away_from_parcels = parcels
                .iter()
                .all(|rect| point_to_rect_distance(point, *rect) >= FACE_MIN_PARCEL_CLEARANCE);
            if away_from_parcels {
                unserved += 1;
                if unserved >= FACE_MIN_UNSERVED_SAMPLES {
                    return true;
                }
            }
        }
    }
    false
}

impl Engine {
    pub(super) fn face_conditioned_access_proposals(
        &self,
        road_id: u32,
    ) -> Option<Vec<interior_access::AccessProposal>> {
        let eligible_roads = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
            })
            .collect::<Vec<_>>();
        let participating = eligible_roads
            .iter()
            .flat_map(|road| [road.from, road.to])
            .collect::<std::collections::BTreeSet<_>>();
        let nodes = self
            .state
            .nodes
            .iter()
            .filter(|node| participating.contains(&node.id))
            .map(|node| crate::road_faces::RoadFaceNode {
                id: node.id,
                x: node.x,
                z: node.z,
            })
            .collect::<Vec<_>>();
        let edges = eligible_roads
            .iter()
            .map(|road| crate::road_faces::RoadFaceEdge {
                id: road.id,
                from: road.from,
                to: road.to,
            })
            .collect::<Vec<_>>();
        let faces = crate::road_faces::trace_faces(&nodes, &edges, None).ok()?;
        let bounded_count = faces.len();
        eligible_roads.iter().find(|road| road.id == road_id)?;
        let face_roads = eligible_roads
            .iter()
            .filter_map(|road| Some((*road, self.node(road.from)?, self.node(road.to)?)))
            .collect::<Vec<_>>();
        let parcel_rects = self
            .state
            .parcels
            .iter()
            .map(|parcel| geometry::OrientedRect {
                x: parcel.x,
                z: parcel.z,
                width: parcel.width,
                depth: parcel.depth,
                rotation: parcel
                    .rotation
                    .unwrap_or_else(|| self.frontage_rotation(parcel.id)),
            })
            .collect::<Vec<_>>();
        let incident_faces = faces
            .into_iter()
            .filter(|face| face.boundary_edge_ids.contains(&road_id))
            .collect::<Vec<_>>();
        let face_count = incident_faces.len();
        let faces = incident_faces
            .into_iter()
            .filter(|face| face_has_unserved_interior(&face.polygon, &face_roads, &parcel_rects))
            .collect::<Vec<_>>();
        if std::env::var_os("FACE_ACCESS_DEBUG").is_some() {
            eprintln!(
                "face access host={road_id} graph_nodes={} graph_roads={} bounded_faces={bounded_count} incident_faces={face_count} unserved_faces={}",
                nodes.len(), edges.len(), faces.len()
            );
        }
        if faces.is_empty() {
            return Some(Vec::new());
        }

        let host_by_id = eligible_roads
            .iter()
            .map(|road| (road.id, *road))
            .collect::<std::collections::BTreeMap<_, _>>();
        let eligible_ids = host_by_id
            .keys()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut host_sides = Vec::new();
        for face in &faces {
            for host_id in
                ordered_face_boundary_hosts(road_id, std::slice::from_ref(face), &eligible_ids)
            {
                let Some(host) = host_by_id.get(&host_id).copied() else {
                    continue;
                };
                let (Some(host_from), Some(host_to)) = (self.node(host.from), self.node(host.to))
                else {
                    continue;
                };
                for side in inward_sides_for_face(host_from, host_to, &face.polygon) {
                    host_sides.push((host_id, side));
                }
            }
        }
        let candidate_window = round_robin_face_candidates(&host_sides);
        let scan = self.propose_interior_access_from_candidates(None, 0, Some(&candidate_window));
        let mut proposals = Vec::new();
        let mut host_diagnostics = Vec::new();
        let scanned_by_host = candidate_window.iter().fold(
            std::collections::BTreeMap::<u32, usize>::new(),
            |mut counts, candidate| {
                *counts.entry(candidate.host_id).or_default() += 1;
                counts
            },
        );
        let geometry_valid_by_host = scan.proposals.iter().fold(
            std::collections::BTreeMap::<u32, usize>::new(),
            |mut counts, proposal| {
                *counts.entry(proposal.host_id).or_default() += 1;
                counts
            },
        );
        for (&host_id, &candidate_count) in &scanned_by_host {
            let Some(host) = host_by_id.get(&host_id).copied() else {
                continue;
            };
            let (Some(host_from), Some(host_to)) = (self.node(host.from), self.node(host.to))
            else {
                continue;
            };
            let mut retained_count = 0usize;
            for proposal in scan
                .proposals
                .iter()
                .filter(|proposal| proposal.host_id == host_id)
            {
                let retained = faces.iter().find_map(|face| {
                    retain_face_lots(proposal, host, host_from, host_to, &face.polygon)
                });
                if let Some(retained) = retained {
                    retained_count += 1;
                    proposals.push(retained);
                }
            }
            host_diagnostics.push((
                host_id,
                candidate_count,
                geometry_valid_by_host.get(&host_id).copied().unwrap_or(0),
                retained_count,
            ));
        }
        if std::env::var_os("FACE_ACCESS_DEBUG").is_some() {
            for face in &faces {
                let min_x = face
                    .polygon
                    .iter()
                    .map(|p| p.0)
                    .fold(f32::INFINITY, f32::min);
                let max_x = face
                    .polygon
                    .iter()
                    .map(|p| p.0)
                    .fold(f32::NEG_INFINITY, f32::max);
                let min_z = face
                    .polygon
                    .iter()
                    .map(|p| p.1)
                    .fold(f32::INFINITY, f32::min);
                let max_z = face
                    .polygon
                    .iter()
                    .map(|p| p.1)
                    .fold(f32::NEG_INFINITY, f32::max);
                eprintln!("face trigger={road_id} bbox=({min_x:.1},{min_z:.1})-({max_x:.1},{max_z:.1}) vertices={} area={:.1}", face.polygon.len(), face.area);
            }
            eprintln!("face access trigger={road_id} host(candidate,geometry_valid,face_valid)={host_diagnostics:?} total_candidates={} feasible_proposals={}", candidate_window.len(), proposals.len());
        }
        Some(proposals)
    }
}

#[test]
fn road_opening_access_noop_keeps_global_planner_state_unchanged() {
    let mut engine = Engine::restore_json(include_str!(
        "../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"
    ))
    .expect("mature save restores");
    engine.state.demand = 4.0;
    engine.road_opening_access_experiment = true;
    let host_ids = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
                && !engine.state.neighborhood_road_ids.contains(&road.id)
        })
        .map(|road| road.id)
        .collect::<Vec<_>>();
    let host = host_ids
        .into_iter()
        .find(|host| {
            engine
                .propose_interior_access_for_host(*host)
                .proposals
                .is_empty()
        })
        .expect("mature fixture has an ordinary host with no feasible access geometry");
    let before = engine.save_json();
    let cursor = engine.state.interior_access_cursor;
    assert!(!engine.try_plan_access_at_road_opening(host));
    assert_eq!(engine.state.interior_access_cursor, cursor);
    assert_eq!(engine.save_json(), before);
    assert_eq!(engine.road_opening_access_attempts, 1);
    assert_eq!(engine.road_opening_access_admissions, 0);
}

#[test]
fn opening_host_frontage_uses_rendered_rotation_against_reserved_lots() {
    let mut engine = Engine::new_legacy(7);
    let reserved = interior_access::InteriorAccessLot {
        x: -343.22574,
        z: 11.176857,
        width: 13.9,
        depth: 12.0,
        rotation: 0.010097436,
        along: 0.33229166,
        side: 1,
    };
    engine.state.pending_interior_access = Some(interior_access::InteriorAccessPlan {
        host_road_id: 1,
        host_second_half_road_id: 2,
        host_t: 0.5,
        split_node_id: 3,
        branch_endpoint_id: 4,
        branch_road_id: 5,
        project_id: 6,
        selected_lots: vec![reserved],
    });
    let (a, b) = (
        Node {
            id: 7,
            x: 0.0,
            z: 0.0,
            kind: "junction".into(),
        },
        Node {
            id: 8,
            x: 0.0,
            z: -100.0,
            kind: "junction".into(),
        },
    );
    let x = -342.42465;
    let z = -3.5753856;
    let legacy_axis = geometry::OrientedRect {
        x,
        z,
        width: 14.0,
        depth: 12.0,
        rotation: 0.0,
    };
    let rendered = geometry::OrientedRect {
        rotation: std::f32::consts::FRAC_PI_2,
        ..legacy_axis
    };
    assert!(engine.interior_plan_allows_parcel(&legacy_axis, false));
    assert!(!engine.interior_plan_allows_parcel(&rendered, false));
    assert!(!engine.interior_plan_allows_frontage_candidate(&a, &b, x, z, 14.0, 12.0, -1));
}

// Rejected opening-time treatment; see validation/early-reservation-gap-diagnosis.md.
// Preserve this failing historical probe for explicit investigation, not as a
// current default-planner acceptance requirement. Safety unit tests stay active.
#[test]
#[ignore = "rejected test-only road-opening treatment; historical seed no longer admits; run explicitly for diagnosis"]
fn road_opening_access_reserves_before_frontage_and_completes_after_restore() {
    let mut engine = Engine::new_legacy(87_231);
    engine.set_development_rate(16);
    engine.road_opening_access_experiment = true;
    engine.early_access_experiment = false;
    engine.district_policy_experiment = false;

    for _ in 0..242_000 {
        engine.step(1);
        if engine.road_opening_access_admissions > 0 {
            break;
        }
    }
    assert!(
        engine.road_opening_access_admissions > 0,
        "seed 87231 did not produce a valid access proposal while a new road opened (attempts={})",
        engine.road_opening_access_attempts
    );

    let plan = engine.state.pending_interior_access.clone().unwrap();
    assert_eq!(engine.road_opening_access_branch, Some(plan.branch_road_id));
    let completions_before = engine.interior_access_completions;
    assert!(!engine
        .state
        .neighborhood_road_ids
        .contains(&plan.host_road_id));
    let branch = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == plan.branch_road_id)
        .unwrap();
    assert_eq!(branch.stage, "survey");
    assert!(!branch.connected);
    let project = engine
        .state
        .projects
        .iter()
        .find(|project| project.id == plan.project_id)
        .unwrap();
    assert_eq!(project.kind, "road");
    assert!(project.progress < 1.0);

    // Frontage admission runs after the split while this pending project is
    // present. Its centerline must be part of the collision context, in
    // addition to the explicit selected-lot reservation.
    let context = engine.collision_context();
    let (Some(branch_a), Some(branch_b)) = (engine.node(branch.from), engine.node(branch.to))
    else {
        panic!("pending branch endpoints exist");
    };
    assert!(context.pending_roads.iter().any(|road| {
        (road.ax == branch_a.x
            && road.az == branch_a.z
            && road.bx == branch_b.x
            && road.bz == branch_b.z)
            || (road.bx == branch_a.x
                && road.bz == branch_a.z
                && road.ax == branch_b.x
                && road.az == branch_b.z)
    }));
    let branch_midpoint = geometry::OrientedRect {
        x: (branch_a.x + branch_b.x) * 0.5,
        z: (branch_a.z + branch_b.z) * 0.5,
        width: 10.0,
        depth: 10.0,
        rotation: 0.0,
    };
    assert!(
        !engine.progressive_frontage_geometry_clear(&branch_midpoint, &context),
        "ordinary frontage checks must see the reserved pending-road corridor"
    );

    let host_ids = [plan.host_road_id, plan.host_second_half_road_id];
    let frontages = engine
        .state
        .parcel_frontages
        .iter()
        .filter(|frontage| host_ids.contains(&frontage.road_id))
        .collect::<Vec<_>>();
    for frontage in frontages {
        let parcel = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == frontage.parcel_id)
            .unwrap();
        let rendered_rotation = parcel
            .rotation
            .unwrap_or_else(|| engine.frontage_rotation(parcel.id));
        let rect = geometry::OrientedRect {
            x: parcel.x,
            z: parcel.z,
            width: parcel.width,
            depth: parcel.depth,
            rotation: rendered_rotation,
        };
        assert!(
            geometry::rect_segment_distance(&rect, branch_a.x, branch_a.z, branch_b.x, branch_b.z)
                >= 5.0
        );
        for lot in &plan.selected_lots {
            let reserved = geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            assert!(!geometry::rects_overlap(&rect, &reserved, 2.0));
        }
    }

    engine
        .validate()
        .expect("pending opening-time plan is valid");
    let save = engine.save_json();
    let mut resumed = Engine::restore_json(&save).expect("pending opening-time plan restores");
    resumed.road_opening_access_experiment = true;
    resumed.early_access_experiment = false;
    resumed.district_policy_experiment = false;
    assert_eq!(resumed.save_json(), save);
    for _ in 0..24_000 {
        let opened = engine
            .state
            .roads
            .iter()
            .any(|road| road.id == plan.branch_road_id && road.stage == "open");
        if opened {
            break;
        }
        engine.step(1);
        resumed.step(1);
        assert_eq!(engine.save_json(), resumed.save_json());
        if engine
            .state
            .roads
            .iter()
            .any(|road| road.id == plan.branch_road_id && road.stage == "open")
        {
            // Keep this lifecycle assertion scoped to the admitted child. A
            // later, unrelated road opening belongs in the matched run.
            engine.road_opening_access_experiment = false;
            resumed.road_opening_access_experiment = false;
            break;
        }
    }
    assert!(engine
        .state
        .roads
        .iter()
        .any(|road| road.id == plan.branch_road_id && road.stage == "open"));
    assert_eq!(engine.save_json(), resumed.save_json());
    engine
        .validate()
        .expect("completed child road and lots preserve invariants");
    assert_eq!(engine.interior_access_completions, completions_before + 1);
    let opened_branch = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == plan.branch_road_id)
        .unwrap();
    assert_eq!(opened_branch.stage, "open");
    assert!(opened_branch.connected);
    assert_eq!(
        engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| frontage.road_id == plan.branch_road_id)
            .count(),
        plan.selected_lots.len()
    );
}

#[test]
#[ignore = "matched-seed road-opening access experiment; run explicitly for calibration"]
fn road_opening_access_matched_seeds() {
    let mut rows = Vec::new();
    let validation_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("validation");

    let cases = if std::env::var_os("ROAD_OPENING_BASELINE_DIAGNOSTIC").is_some() {
        vec![(87_231_u32, false)]
    } else if std::env::var_os("ROAD_OPENING_TREATMENT_DIAGNOSTIC").is_some() {
        vec![(87_231_u32, true)]
    } else {
        [0_u32, 87_231]
            .into_iter()
            .flat_map(|seed| [(seed, false), (seed, true)])
            .collect()
    };
    for (seed, enabled) in cases {
        let started = std::time::Instant::now();
        let mut engine = Engine::new_legacy(seed);
        engine.set_development_rate(16);
        engine.district_policy_experiment = false;
        engine.early_access_experiment = false;
        engine.road_opening_access_experiment = enabled;
        for _ in 0..121 {
            engine.step(2_000);
        }
        assert_eq!(engine.state.tick, 242_000);
        if let Err(error) = engine.validate() {
            if let Some(plan) = &engine.state.pending_interior_access {
                let context = engine.collision_context();
                eprintln!(
                    "invalid pending plan at tick {}: {error}; plan={plan:?}",
                    engine.state.tick
                );
                for (index, lot) in plan.selected_lots.iter().enumerate() {
                    let rect = geometry::OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    };
                    eprintln!("lot {index}: ({:.3},{:.3}) {}x{} rot {:.3}, geometric_clear={}, overlaps_prior={}",
                            lot.x, lot.z, lot.width, lot.depth, lot.rotation,
                            engine.oriented_parcel_geometry_clear_with_context_mode(&rect, &context, true),
                            plan.selected_lots[..index].iter().any(|other| geometry::rects_overlap(&rect, &geometry::OrientedRect { x: other.x, z: other.z, width: other.width, depth: other.depth, rotation: other.rotation }, 2.0)));
                    if !engine
                        .oriented_parcel_geometry_clear_with_context_mode(&rect, &context, true)
                    {
                        let (min_x, min_z, max_x, max_z) = rect.bounds();
                        let parcel_hits = context
                            .parcel_footprints
                            .iter()
                            .enumerate()
                            .filter(|(_, other)| geometry::rects_overlap(&rect, other, 2.0))
                            .map(|(i, other)| {
                                (
                                    i,
                                    other.x,
                                    other.z,
                                    other.width,
                                    other.depth,
                                    other.rotation,
                                )
                            })
                            .collect::<Vec<_>>();
                        let open_hits = context
                            .open_roads
                            .iter()
                            .enumerate()
                            .filter(|(_, road)| {
                                geometry::rect_segment_distance(
                                    &rect, road.ax, road.az, road.bx, road.bz,
                                ) < 5.0
                            })
                            .map(|(i, road)| {
                                (
                                    i,
                                    road.ax,
                                    road.az,
                                    road.bx,
                                    road.bz,
                                    geometry::rect_segment_distance(
                                        &rect, road.ax, road.az, road.bx, road.bz,
                                    ),
                                )
                            })
                            .collect::<Vec<_>>();
                        let pending_hits = context
                            .pending_roads
                            .iter()
                            .enumerate()
                            .filter(|(_, road)| {
                                geometry::rect_segment_distance(
                                    &rect, road.ax, road.az, road.bx, road.bz,
                                ) < 7.0
                            })
                            .map(|(i, road)| {
                                (
                                    i,
                                    road.ax,
                                    road.az,
                                    road.bx,
                                    road.bz,
                                    geometry::rect_segment_distance(
                                        &rect, road.ax, road.az, road.bx, road.bz,
                                    ),
                                )
                            })
                            .collect::<Vec<_>>();
                        eprintln!("lot {index} reject detail: bounds=({min_x},{min_z},{max_x},{max_z}) river_ok={} parcels={parcel_hits:?} open_roads={open_hits:?} pending_roads={pending_hits:?} neighborhood_clear={}",
                                engine.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth),
                                engine.neighborhood_corridor_clear_for_parcel(rect.x, rect.z, max_x - min_x, max_z - min_z));
                    }
                }
            }
            let tag = format!("seed-{seed}-enabled-{enabled}-invalid");
            std::fs::write(
                validation_dir.join(format!("road-opening-access-{tag}-save.json")),
                engine.save_json(),
            )
            .unwrap();
            panic!("experiment result preserves invariants: {error}");
        }

        let open_buildings: Vec<_> = engine
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .collect();
        let housing: Vec<_> = open_buildings
            .iter()
            .copied()
            .filter(|b| is_housing(b))
            .collect();
        let commerce: Vec<_> = open_buildings
            .iter()
            .copied()
            .filter(|b| is_commercial(b))
            .collect();
        let connected_roads: Vec<_> = engine
            .state
            .roads
            .iter()
            .filter(|road| road.stage == "open" && road.connected)
            .collect();
        let ordinary_roads: Vec<_> = connected_roads
            .iter()
            .filter(|road| !matches!(road.kind.as_str(), "rail" | "bridge"))
            .collect();
        let summary = json!({
            "seed": seed,
            "development_rate": 16,
            "tick": engine.state.tick,
            "road_opening_access_enabled": enabled,
            "early_access_scheduler_enabled": false,
            "district_policy_enabled": false,
            "buildings_open": open_buildings.len(),
            "housing_open": housing.len(),
            "housing_occupancy": housing.iter().map(|b| b.occupancy as u64).sum::<u64>(),
            "housing_capacity": housing.iter().map(|b| b.capacity as u64).sum::<u64>(),
            "commercial_open": commerce.len(),
            "commercial_active": commerce.iter().filter(|b| b.business_active).count(),
            "commercial_active_capacity": commerce.iter().filter(|b| b.business_active).map(|b| b.capacity as u64).sum::<u64>(),
            "envelope_area_proxy_m2": open_buildings.iter().map(|b| (b.width * b.depth) as f64).sum::<f64>(),
            "floor_area_proxy_m2": open_buildings.iter().map(|b| (b.width * b.depth * b.floors as f32) as f64).sum::<f64>(),
            "parcel_count": engine.state.parcels.len(),
            "connected_open_roads": connected_roads.len(),
            "ordinary_road_records": ordinary_roads.len(),
            "ordinary_road_length_m": ordinary_roads.iter().map(|r| r.length as f64).sum::<f64>(),
            "access_attempts": engine.road_opening_access_attempts,
            "access_admissions": engine.road_opening_access_admissions,
            "access_completions": engine.interior_access_completions,
            "pending_access_lots": engine.state.pending_interior_access.as_ref().map_or(0, |p| p.selected_lots.len()),
            "demand": engine.state.demand,
            "population": engine.state.population,
            "elapsed_seconds": started.elapsed().as_secs_f64(),
        });
        eprintln!("road-opening-access-experiment {summary}");
        let save = engine.save_json();
        let tag = format!("seed-{seed}-enabled-{enabled}");
        std::fs::write(
            validation_dir.join(format!("road-opening-access-{tag}-save.json")),
            &save,
        )
        .expect("write native save");
        std::fs::write(
            validation_dir.join(format!("road-opening-access-{tag}-snapshot.json")),
            engine.snapshot_json(),
        )
        .expect("write native snapshot");
        let mut resumed = Engine::restore_json(&save).expect("canonical save restores");
        resumed.road_opening_access_experiment = enabled;
        resumed.early_access_experiment = false;
        resumed.district_policy_experiment = false;
        assert_eq!(resumed.save_json(), save);
        engine.step(2_000);
        resumed.step(2_000);
        assert_eq!(engine.save_json(), resumed.save_json());
        rows.push(summary);
    }
    std::fs::write(
        validation_dir.join("road-opening-access-experiment.json"),
        serde_json::to_vec_pretty(&json!({
            "experiment": "on connected ordinary road completion, try a bounded real access plan on that road before adding its frontage parcels",
            "horizon_ticks": 242_000,
            "rate": 16,
            "seeds": [0, 87_231],
            "baseline_and_candidate_same_source": true,
            "prior_early_access_scheduler": false,
            "district_policy": false,
            "frontage_collision_context_includes_pending_branch": true,
            "rows": rows
        })).unwrap(),
    ).expect("write paired experiment report");
}

#[test]
#[ignore = "matched face-conditioned road-opening access experiment; run explicitly for calibration"]
fn face_conditioned_road_opening_access_matched_seeds() {
    let mut rows = Vec::new();
    let validation_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("validation");
    let diagnostic_seed = std::env::var("FACE_ACCESS_DIAGNOSTIC_SEED")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .or_else(|| std::env::var_os("FACE_ACCESS_DIAGNOSTIC_SEED_87231").map(|_| 87_231));
    let matched_seed = std::env::var("FACE_ACCESS_MATCHED_SEED")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    let cases = if let Some(seed) = diagnostic_seed {
        vec![(seed, true)]
    } else if let Some(seed) = matched_seed {
        vec![(seed, false), (seed, true)]
    } else {
        [0_u32, 87_231]
            .into_iter()
            .flat_map(|seed| [(seed, false), (seed, true)])
            .collect::<Vec<_>>()
    };
    for (seed, enabled) in cases {
        let started = std::time::Instant::now();
        let mut engine = Engine::new_legacy(seed);
        engine.set_development_rate(16);
        engine.district_policy_experiment = false;
        engine.early_access_experiment = false;
        engine.road_opening_access_experiment = enabled;
        engine.face_conditioned_access_experiment = enabled;
        for _ in 0..121 {
            engine.step(2_000);
        }
        assert_eq!(engine.tick(), 242_000);
        engine
            .validate()
            .expect("face-conditioned treatment preserves invariants");

        let open_buildings = engine
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .collect::<Vec<_>>();
        let housing = open_buildings
            .iter()
            .filter(|building| is_housing(building))
            .collect::<Vec<_>>();
        let commerce = open_buildings
            .iter()
            .filter(|building| is_commercial(building))
            .collect::<Vec<_>>();
        let connected_roads = engine
            .state
            .roads
            .iter()
            .filter(|road| road.stage == "open" && road.connected)
            .collect::<Vec<_>>();
        let ordinary_roads = connected_roads
            .iter()
            .filter(|road| matches!(road.kind.as_str(), "local" | "arterial"))
            .collect::<Vec<_>>();
        let summary = json!({
            "seed": seed,
            "development_rate": 16,
            "tick": engine.tick(),
            "face_conditioned_access_enabled": enabled,
            "district_policy_enabled": false,
            "early_access_scheduler_enabled": false,
            "open_buildings": open_buildings.len(),
            "open_housing_buildings": housing.len(),
            "housing_occupancy": housing.iter().map(|b| b.occupancy as u64).sum::<u64>(),
            "housing_capacity": housing.iter().map(|b| b.capacity as u64).sum::<u64>(),
            "open_commercial_buildings": commerce.len(),
            "active_commercial_buildings": commerce.iter().filter(|b| b.business_active).count(),
            "active_commercial_capacity": commerce.iter().filter(|b| b.business_active).map(|b| b.capacity as u64).sum::<u64>(),
            "nominal_envelope_area_m2": open_buildings.iter().map(|b| (b.width * b.depth) as f64).sum::<f64>(),
            "floor_area_proxy_m2": open_buildings.iter().map(|b| (b.width * b.depth * b.floors as f32) as f64).sum::<f64>(),
            "parcels": engine.state.parcels.len(),
            "connected_open_roads": connected_roads.len(),
            "ordinary_open_roads": ordinary_roads.len(),
            "ordinary_road_length_m": ordinary_roads.iter().map(|road| road.length as f64).sum::<f64>(),
            "attempts": engine.road_opening_access_attempts,
            "admissions": engine.road_opening_access_admissions,
            "completions": engine.interior_access_completions,
            "pending_selected_lots": engine.state.pending_interior_access.as_ref().map_or(0, |plan| plan.selected_lots.len()),
            "pending_branch_id": engine.state.pending_interior_access.as_ref().map(|plan| plan.branch_road_id),
            "demand": engine.state.demand,
            "population": engine.state.population,
            "elapsed_seconds": started.elapsed().as_secs_f64(),
        });
        eprintln!("face-conditioned-access-experiment {summary}");
        let tag = format!("seed-{seed}-enabled-{enabled}-round-robin-boundary");
        let save = engine.save_json();
        std::fs::write(
            validation_dir.join(format!("face-access-{tag}-save.json")),
            &save,
        )
        .expect("write native save");
        std::fs::write(
            validation_dir.join(format!("face-access-{tag}-snapshot.json")),
            engine.snapshot_json(),
        )
        .expect("write native snapshot");
        let mut resumed = Engine::restore_json(&save).expect("canonical save restores");
        resumed.road_opening_access_experiment = enabled;
        resumed.face_conditioned_access_experiment = enabled;
        resumed.early_access_experiment = false;
        resumed.district_policy_experiment = false;
        assert_eq!(resumed.save_json(), save);
        engine.step(2_000);
        resumed.step(2_000);
        assert_eq!(engine.save_json(), resumed.save_json());
        rows.push(summary);
    }
    let report_name = if let Some(seed) = diagnostic_seed {
        format!("face-conditioned-access-seed-{seed}-round-robin-diagnostic.json")
    } else if let Some(seed) = matched_seed {
        format!("face-conditioned-access-seed-{seed}-round-robin-matched.json")
    } else {
        "face-conditioned-access-round-robin-experiment.json".to_string()
    };
    std::fs::write(
        validation_dir.join(report_name),
        serde_json::to_vec_pretty(&json!({
            "experiment": "when an ordinary road opens, propose bounded access only inside a newly bounded face with sampled unserved interior and wholly contained branch/lots",
            "sample_grid_side": FACE_SAMPLE_GRID_SIDE,
            "minimum_unserved_sample_cells": FACE_MIN_UNSERVED_SAMPLES,
            "road_edge_clearance_m": FACE_MIN_ROAD_EDGE_CLEARANCE,
            "parcel_clearance_m": FACE_MIN_PARCEL_CLEARANCE,
            "thresholds_are_calibration_constants_not_city_targets": true,
            "horizon_ticks": 242_000,
            "development_rate": 16,
            "seeds": [0, 87_231],
            "save_resume_replay_ticks": 2_000,
            "rows": rows
        })).unwrap(),
    )
    .expect("write paired face-conditioned experiment report");
}
