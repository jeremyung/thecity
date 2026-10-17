//! Bounded deterministic tracing of bounded faces in a planar street graph.
//!
//! The caller supplies only open, connected ordinary-road edges that belong in
//! the street-plane graph. Rail and elevated bridge roads must be filtered by
//! the caller. Disconnected graph components are supported. Inputs with a
//! crossing, duplicate/coincident geometry, dangling malformed references, or
//! exhausted work limits fail closed rather than returning partial faces.

use crate::world::geometry::segments_distance_coords;
use crate::{NodeKind, RoadKind, Stage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 512;
const MAX_EDGES: usize = 512;
const MAX_HALF_EDGE_STEPS: usize = MAX_EDGES * 2;
const MAX_PAIR_TESTS: usize = 131_072;
const MAX_ADJACENCY_SEARCH_STEPS: usize = MAX_HALF_EDGE_STEPS * MAX_NODES;
const MAX_OUTPUT_FACES: usize = MAX_EDGES;
const MIN_FACE_AREA: f64 = 1.0e-3;
const COINCIDENT_EPSILON: f32 = 1.0e-5;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct RoadFaceNode {
    pub id: u32,
    pub x: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RoadFaceEdge {
    pub id: u32,
    pub from: u32,
    pub to: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct RoadFace {
    /// Canonical identity: the lexicographically smallest cyclic sequence
    /// across both directions. It is independent of input ordering/winding.
    pub identity: Vec<u32>,
    /// Boundary vertices in counter-clockwise order in X/Z coordinates.
    pub vertex_ids: Vec<u32>,
    pub polygon: Vec<(f32, f32)>,
    /// Positive area of the bounded polygon, in square world units.
    pub area: f64,
    /// Edge IDs aligned so boundary_edge_ids[i] joins vertex_ids[i] to the
    /// next vertex (wrapping at the end).
    pub boundary_edge_ids: Vec<u32>,
    pub contains_proposed_edge: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RoadFaceError {
    TooManyNodes,
    TooManyEdges,
    MissingNode,
    DuplicateNode,
    InvalidCoordinate,
    CoincidentNodes,
    DuplicateEdge,
    InvalidEdge,
    NonPlanar,
    WorkLimit,
    InvalidEmbedding,
    NestedFacesUnsupported,
    OutputLimit,
}

#[derive(Clone, Debug)]
struct HalfEdge {
    edge_index: usize,
    from: usize,
    to: usize,
    angle: f32,
}

/// Trace the bounded faces of a straight-line planar graph.
///
/// The optional edge is added to a temporary copy of the input graph and is
/// marked in any bounded face whose perimeter uses it. This function has no
/// simulation-state, RNG, or allocation side effects outside its bounded
/// temporary buffers.
pub(crate) fn trace_faces(
    nodes: &[RoadFaceNode],
    edges: &[RoadFaceEdge],
    proposed_edge: Option<RoadFaceEdge>,
) -> Result<Vec<RoadFace>, RoadFaceError> {
    let edge_count = edges.len() + usize::from(proposed_edge.is_some());
    if nodes.len() > MAX_NODES {
        return Err(RoadFaceError::TooManyNodes);
    }
    if edge_count > MAX_EDGES {
        return Err(RoadFaceError::TooManyEdges);
    }
    if nodes.is_empty() && edge_count == 0 {
        return Ok(Vec::new());
    }

    let mut node_index = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.z.is_finite() {
            return Err(RoadFaceError::InvalidCoordinate);
        }
        if node_index.insert(node.id, index).is_some() {
            return Err(RoadFaceError::DuplicateNode);
        }
    }
    for left in 0..nodes.len() {
        for right in left + 1..nodes.len() {
            if (nodes[left].x - nodes[right].x).hypot(nodes[left].z - nodes[right].z)
                <= COINCIDENT_EPSILON
            {
                return Err(RoadFaceError::CoincidentNodes);
            }
        }
    }

    let mut all_edges = Vec::with_capacity(edge_count);
    all_edges.extend_from_slice(edges);
    if let Some(edge) = proposed_edge {
        all_edges.push(edge);
    }
    let mut edge_ids = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    let mut indexed_edges = Vec::with_capacity(all_edges.len());
    for edge in &all_edges {
        if edge.from == edge.to {
            return Err(RoadFaceError::InvalidEdge);
        }
        let Some(&from) = node_index.get(&edge.from) else {
            return Err(RoadFaceError::MissingNode);
        };
        let Some(&to) = node_index.get(&edge.to) else {
            return Err(RoadFaceError::MissingNode);
        };
        if !edge_ids.insert(edge.id) {
            return Err(RoadFaceError::DuplicateEdge);
        }
        let key = (edge.from.min(edge.to), edge.from.max(edge.to));
        if !pairs.insert(key) {
            return Err(RoadFaceError::DuplicateEdge);
        }
        let dx = nodes[to].x - nodes[from].x;
        let dz = nodes[to].z - nodes[from].z;
        if !dx.is_finite() || !dz.is_finite() || dx.hypot(dz) <= COINCIDENT_EPSILON {
            return Err(RoadFaceError::InvalidEdge);
        }
        indexed_edges.push((from, to));
    }

    // A strict planar embedding requires every geometric crossing to already
    // be represented by a shared node. Cap pairwise checks explicitly.
    let pair_count = all_edges
        .len()
        .saturating_mul(all_edges.len().saturating_sub(1))
        / 2;
    if pair_count > MAX_PAIR_TESTS {
        return Err(RoadFaceError::WorkLimit);
    }
    let mut adjacency_search_steps = 0usize;
    for first in 0..all_edges.len() {
        for second in first + 1..all_edges.len() {
            let (a, b) = indexed_edges[first];
            let (c, d) = indexed_edges[second];
            if a == c || a == d || b == c || b == d {
                let (shared, first_other, second_other) = if a == c {
                    (a, b, d)
                } else if a == d {
                    (a, b, c)
                } else if b == c {
                    (b, a, d)
                } else {
                    (b, a, c)
                };
                let first_dx = nodes[first_other].x - nodes[shared].x;
                let first_dz = nodes[first_other].z - nodes[shared].z;
                let second_dx = nodes[second_other].x - nodes[shared].x;
                let second_dz = nodes[second_other].z - nodes[shared].z;
                let cross = first_dx * second_dz - first_dz * second_dx;
                let dot = first_dx * second_dx + first_dz * second_dz;
                let scale = first_dx.hypot(first_dz) * second_dx.hypot(second_dz);
                if cross.abs() <= COINCIDENT_EPSILON * scale && dot > COINCIDENT_EPSILON {
                    // Two edges leave the same vertex along the same ray and
                    // overlap beyond their shared endpoint.
                    return Err(RoadFaceError::NonPlanar);
                }
                continue;
            }
            let a = nodes[a];
            let b = nodes[b];
            let c = nodes[c];
            let d = nodes[d];
            if segments_distance_coords(a.x, a.z, b.x, b.z, c.x, c.z, d.x, d.z)
                <= COINCIDENT_EPSILON
            {
                return Err(RoadFaceError::NonPlanar);
            }
        }
    }

    // Graph bridges cannot bound a face. Removing them before face walking
    // keeps a dangling local street from appearing as a repeated spur in the
    // boundary of a genuine block.
    let graph_bridges = graph_bridge_edges(nodes.len(), &indexed_edges);
    let mut outgoing = vec![Vec::<HalfEdge>::new(); nodes.len()];
    let mut half_edges = Vec::with_capacity(all_edges.len() * 2);
    let mut half_edge_indices = vec![None; all_edges.len()];
    for (edge_index, &(from, to)) in indexed_edges
        .iter()
        .enumerate()
        .filter(|(index, _)| !graph_bridges.contains(index))
    {
        let forward_index = half_edges.len();
        let forward = HalfEdge {
            edge_index,
            from,
            to,
            angle: (nodes[to].z - nodes[from].z).atan2(nodes[to].x - nodes[from].x),
        };
        let reverse = HalfEdge {
            edge_index,
            from: to,
            to: from,
            angle: (nodes[from].z - nodes[to].z).atan2(nodes[from].x - nodes[to].x),
        };
        outgoing[from].push(forward.clone());
        outgoing[to].push(reverse.clone());
        half_edges.push(forward);
        half_edges.push(reverse);
        half_edge_indices[edge_index] = Some([forward_index, forward_index + 1]);
        debug_assert_eq!(half_edges.len(), forward_index + 2);
    }
    for incident in &mut outgoing {
        incident.sort_by(|a, b| {
            a.angle
                .total_cmp(&b.angle)
                .then_with(|| nodes[a.to].id.cmp(&nodes[b.to].id))
                .then_with(|| all_edges[a.edge_index].id.cmp(&all_edges[b.edge_index].id))
        });
    }

    let mut visited = vec![false; half_edges.len()];
    let mut faces = Vec::new();
    let mut work = 0usize;
    for start in 0..half_edges.len() {
        if visited[start] {
            continue;
        }
        let mut walk_vertices = Vec::<usize>::new();
        let mut walk_edges = Vec::<usize>::new();
        let mut current = start;
        loop {
            if visited[current] {
                if current != start {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
                break;
            }
            if work >= MAX_HALF_EDGE_STEPS {
                return Err(RoadFaceError::WorkLimit);
            }
            work += 1;
            visited[current] = true;
            let half = &half_edges[current];
            walk_vertices.push(half.from);
            walk_edges.push(half.edge_index);

            let incident = &outgoing[half.to];
            let mut reverse_position = None;
            for (position, candidate) in incident.iter().enumerate() {
                adjacency_search_steps += 1;
                if adjacency_search_steps > MAX_ADJACENCY_SEARCH_STEPS {
                    return Err(RoadFaceError::WorkLimit);
                }
                if candidate.edge_index == half.edge_index && candidate.to == half.from {
                    reverse_position = Some(position);
                    break;
                }
            }
            let Some(reverse_position) = reverse_position else {
                return Err(RoadFaceError::InvalidEmbedding);
            };
            // The immediately clockwise edge from the reverse half-edge keeps
            // the traversed face on the left of this directed boundary.
            let next = &incident[(reverse_position + incident.len() - 1) % incident.len()];
            current = half_edge_index(
                &half_edges,
                &half_edge_indices,
                next.edge_index,
                next.from,
                next.to,
            )
            .ok_or(RoadFaceError::InvalidEmbedding)?;
        }

        if walk_vertices.len() < 3 || has_repeated_vertex(&walk_vertices) {
            continue;
        }
        let area = signed_area(&walk_vertices, nodes);
        if !area.is_finite() || area <= MIN_FACE_AREA {
            // Negative loops are the exterior face; zero-area loops are
            // dangling bridges, collinear chains, or degenerate cycles.
            continue;
        }
        if faces.len() >= MAX_OUTPUT_FACES {
            return Err(RoadFaceError::OutputLimit);
        }
        let mut vertex_ids = walk_vertices
            .iter()
            .map(|&i| nodes[i].id)
            .collect::<Vec<_>>();
        let mut polygon = walk_vertices
            .iter()
            .map(|&i| (nodes[i].x, nodes[i].z))
            .collect::<Vec<_>>();
        let mut boundary_edge_ids = walk_edges
            .iter()
            .map(|&i| all_edges[i].id)
            .collect::<Vec<_>>();
        let start = vertex_ids
            .iter()
            .enumerate()
            .min_by_key(|(_, id)| *id)
            .map(|(index, _)| index)
            .unwrap_or(0);
        vertex_ids.rotate_left(start);
        polygon.rotate_left(start);
        boundary_edge_ids.rotate_left(start);
        let identity = canonical_cycle(&vertex_ids);
        faces.push(RoadFace {
            identity,
            vertex_ids,
            polygon,
            area,
            contains_proposed_edge: proposed_edge
                .is_some_and(|edge| boundary_edge_ids.contains(&edge.id)),
            boundary_edge_ids,
        });
    }
    faces.sort_by(|a, b| a.identity.cmp(&b.identity));
    for first in 0..faces.len() {
        for second in first + 1..faces.len() {
            if point_strictly_inside(faces[first].polygon[0], &faces[second].polygon)
                || point_strictly_inside(faces[second].polygon[0], &faces[first].polygon)
            {
                // Nested disconnected cycles describe a face with a hole.
                // This API intentionally returns simple polygons only.
                return Err(RoadFaceError::NestedFacesUnsupported);
            }
        }
    }
    Ok(faces)
}

fn half_edge_index(
    edges: &[HalfEdge],
    indices_by_edge: &[Option<[usize; 2]>],
    edge_index: usize,
    from: usize,
    to: usize,
) -> Option<usize> {
    let [forward, reverse] = *indices_by_edge.get(edge_index)?.as_ref()?;
    let first = edges.get(forward)?;
    if first.from == from && first.to == to {
        return Some(forward);
    }
    let second = edges.get(reverse)?;
    (second.from == from && second.to == to).then_some(reverse)
}

fn has_repeated_vertex(vertices: &[usize]) -> bool {
    let mut seen = BTreeSet::new();
    vertices.iter().any(|vertex| !seen.insert(*vertex))
}

pub(crate) fn graph_bridge_edges(node_count: usize, edges: &[(usize, usize)]) -> BTreeSet<usize> {
    let mut adjacency = vec![Vec::<(usize, usize)>::new(); node_count];
    for (edge_index, &(from, to)) in edges.iter().enumerate() {
        adjacency[from].push((to, edge_index));
        adjacency[to].push((from, edge_index));
    }
    for neighbors in &mut adjacency {
        neighbors.sort_unstable();
    }
    fn visit(
        node: usize,
        parent_edge: Option<usize>,
        adjacency: &[Vec<(usize, usize)>],
        time: &mut usize,
        entered: &mut [usize],
        low: &mut [usize],
        bridges: &mut BTreeSet<usize>,
    ) {
        *time += 1;
        entered[node] = *time;
        low[node] = *time;
        for &(next, edge) in &adjacency[node] {
            if Some(edge) == parent_edge {
                continue;
            }
            if entered[next] == 0 {
                visit(next, Some(edge), adjacency, time, entered, low, bridges);
                low[node] = low[node].min(low[next]);
                if low[next] > entered[node] {
                    bridges.insert(edge);
                }
            } else {
                low[node] = low[node].min(entered[next]);
            }
        }
    }
    let mut time = 0;
    let mut entered = vec![0; node_count];
    let mut low = vec![0; node_count];
    let mut bridges = BTreeSet::new();
    for node in 0..node_count {
        if entered[node] == 0 {
            visit(
                node,
                None,
                &adjacency,
                &mut time,
                &mut entered,
                &mut low,
                &mut bridges,
            );
        }
    }
    bridges
}

fn signed_area(vertices: &[usize], nodes: &[RoadFaceNode]) -> f64 {
    let mut twice_area = 0.0f64;
    for index in 0..vertices.len() {
        let current = nodes[vertices[index]];
        let next = nodes[vertices[(index + 1) % vertices.len()]];
        twice_area += current.x as f64 * next.z as f64 - next.x as f64 * current.z as f64;
    }
    twice_area * 0.5
}

fn canonical_cycle(ids: &[u32]) -> Vec<u32> {
    let mut best: Option<Vec<u32>> = None;
    for sequence in [ids.to_vec(), ids.iter().rev().copied().collect()] {
        for offset in 0..sequence.len() {
            let candidate = sequence[offset..]
                .iter()
                .chain(sequence[..offset].iter())
                .copied()
                .collect::<Vec<_>>();
            if best.as_ref().is_none_or(|known| candidate < *known) {
                best = Some(candidate);
            }
        }
    }
    best.unwrap_or_default()
}

fn point_strictly_inside(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    for index in 0..polygon.len() {
        let (ax, az) = polygon[index];
        let (bx, bz) = polygon[(index + 1) % polygon.len()];
        if segments_distance_coords(point.0, point.1, point.0, point.1, ax, az, bx, bz)
            <= COINCIDENT_EPSILON
        {
            return false;
        }
        if (az > point.1) != (bz > point.1) && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
        {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u32, x: f32, z: f32) -> RoadFaceNode {
        RoadFaceNode { id, x, z }
    }

    fn edge(id: u32, from: u32, to: u32) -> RoadFaceEdge {
        RoadFaceEdge { id, from, to }
    }

    fn rectangle() -> (Vec<RoadFaceNode>, Vec<RoadFaceEdge>) {
        (
            vec![
                node(10, 0.0, 0.0),
                node(20, 8.0, 0.0),
                node(30, 8.0, 5.0),
                node(40, 0.0, 5.0),
            ],
            vec![
                edge(1, 10, 20),
                edge(2, 20, 30),
                edge(3, 30, 40),
                edge(4, 40, 10),
            ],
        )
    }

    #[test]
    fn rectangle_is_one_ccw_bounded_face_and_marks_proposed_edge() {
        let (nodes, edges) = rectangle();
        let faces = trace_faces(&nodes, &edges, None).unwrap();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].area, 40.0);
        assert_eq!(faces[0].vertex_ids.len(), 4);
        assert_eq!(faces[0].boundary_edge_ids.len(), 4);
        assert!(!faces[0].contains_proposed_edge);

        let (nodes, mut edges) = rectangle();
        let proposed = edge(5, 10, 20);
        // Duplicate proposals are malformed and fail closed.
        assert_eq!(
            trace_faces(&nodes, &edges, Some(proposed)),
            Err(RoadFaceError::DuplicateEdge)
        );
        edges.pop();
        let faces = trace_faces(&nodes, &edges, Some(edge(5, 40, 10))).unwrap();
        assert_eq!(faces.len(), 1);
        assert!(faces[0].contains_proposed_edge);
    }

    #[test]
    fn rotated_irregular_face_is_stable_under_reordered_reversed_inputs() {
        let points = [(2.0f32, 1.0f32), (8.0, 4.0), (7.0, 10.0), (1.0, 8.0)];
        let angle: f32 = 0.71;
        let (sin, cos) = angle.sin_cos();
        let nodes = points
            .iter()
            .enumerate()
            .map(|(i, &(x, z))| node(101 + i as u32 * 7, x * cos - z * sin, x * sin + z * cos))
            .collect::<Vec<_>>();
        let edges = vec![
            edge(8, 101, 108),
            edge(2, 108, 115),
            edge(9, 115, 122),
            edge(4, 122, 101),
        ];
        let first = trace_faces(&nodes, &edges, None).unwrap();
        let mut shuffled_nodes = nodes.clone();
        shuffled_nodes.reverse();
        let shuffled_edges = vec![
            edge(4, 101, 122),
            edge(9, 122, 115),
            edge(2, 115, 108),
            edge(8, 108, 101),
        ];
        let second = trace_faces(&shuffled_nodes, &shuffled_edges, None).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert!(first[0].area > 30.0);
    }

    #[test]
    fn adjacent_rectangles_trace_two_faces_and_proposed_diagonal_is_marked() {
        let nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 5.0, 0.0),
            node(3, 10.0, 0.0),
            node(4, 0.0, 4.0),
            node(5, 5.0, 4.0),
            node(6, 10.0, 4.0),
        ];
        let edges = vec![
            edge(10, 1, 2),
            edge(11, 2, 3),
            edge(12, 3, 6),
            edge(13, 6, 5),
            edge(14, 5, 4),
            edge(15, 4, 1),
            edge(16, 2, 5),
        ];
        let faces = trace_faces(&nodes, &edges, None).unwrap();
        assert_eq!(faces.len(), 2);
        assert!(faces.iter().all(|face| (face.area - 20.0).abs() < 1.0e-6));
        assert!(faces
            .iter()
            .all(|face| face.boundary_edge_ids.contains(&16)));
    }

    #[test]
    fn dangling_street_and_disconnected_components_add_no_false_faces() {
        let nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 4.0, 0.0),
            node(3, 4.0, 4.0),
            node(4, 0.0, 4.0),
            node(5, 2.0, 2.0),
            node(6, 20.0, 0.0),
            node(7, 22.0, 0.0),
        ];
        let edges = vec![
            edge(1, 1, 2),
            edge(2, 2, 3),
            edge(3, 3, 4),
            edge(4, 4, 1),
            edge(5, 1, 5),
            edge(6, 6, 7),
        ];
        let faces = trace_faces(&nodes, &edges, None).unwrap();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].area, 16.0);
        assert!(!faces[0].boundary_edge_ids.contains(&5));
        assert!(!faces[0].boundary_edge_ids.contains(&6));
    }

    #[test]
    fn interior_dangling_branch_is_removed_without_losing_its_block_face() {
        let nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 8.0, 0.0),
            node(3, 8.0, 8.0),
            node(4, 0.0, 8.0),
            node(5, 4.0, 4.0),
            node(6, 4.0, 6.0),
        ];
        let edges = vec![
            edge(6, 6, 5),
            edge(2, 3, 2),
            edge(5, 5, 1),
            edge(4, 1, 4),
            edge(1, 2, 1),
            edge(3, 4, 3),
        ];
        let faces = trace_faces(&nodes, &edges, None).unwrap();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].area, 64.0);
        assert_eq!(faces[0].boundary_edge_ids, vec![1, 2, 3, 4]);
    }

    #[test]
    fn nested_disconnected_cycles_are_rejected_instead_of_double_counted() {
        let nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 10.0, 0.0),
            node(3, 10.0, 10.0),
            node(4, 0.0, 10.0),
            node(5, 3.0, 3.0),
            node(6, 7.0, 3.0),
            node(7, 7.0, 7.0),
            node(8, 3.0, 7.0),
        ];
        let edges = vec![
            edge(1, 1, 2),
            edge(2, 2, 3),
            edge(3, 3, 4),
            edge(4, 4, 1),
            edge(5, 5, 6),
            edge(6, 6, 7),
            edge(7, 7, 8),
            edge(8, 8, 5),
        ];
        assert_eq!(
            trace_faces(&nodes, &edges, None),
            Err(RoadFaceError::NestedFacesUnsupported)
        );
    }

    #[test]
    fn graph_bridge_and_crossing_edges_fail_or_yield_no_bounded_face_safely() {
        let nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 4.0, 0.0),
            node(3, 2.0, 3.0),
            node(4, 2.0, -2.0),
        ];
        let bridge = vec![edge(1, 1, 2), edge(2, 2, 3), edge(3, 3, 1), edge(4, 2, 4)];
        assert_eq!(trace_faces(&nodes, &bridge, None).unwrap().len(), 1);
        let crossing_nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 4.0, 0.0),
            node(3, 4.0, 4.0),
            node(4, 0.0, 4.0),
        ];
        let crossing = vec![edge(1, 1, 3), edge(2, 2, 4)];
        assert_eq!(
            trace_faces(&crossing_nodes, &crossing, None),
            Err(RoadFaceError::NonPlanar)
        );
    }

    #[test]
    fn malformed_and_nonfinite_inputs_fail_closed() {
        let (nodes, edges) = rectangle();
        assert_eq!(
            trace_faces(&nodes, &[edge(1, 10, 20), edge(2, 20, 10)], None),
            Err(RoadFaceError::DuplicateEdge)
        );
        assert_eq!(
            trace_faces(&nodes, &[edge(1, 10, 999)], None),
            Err(RoadFaceError::MissingNode)
        );
        assert_eq!(
            trace_faces(&nodes, &[edge(1, 10, 10)], None),
            Err(RoadFaceError::InvalidEdge)
        );
        let bad_nodes = vec![node(1, f32::NAN, 0.0)];
        assert_eq!(
            trace_faces(&bad_nodes, &[], None),
            Err(RoadFaceError::InvalidCoordinate)
        );
        let duplicate_nodes = vec![node(1, 0.0, 0.0), node(1, 1.0, 0.0)];
        assert_eq!(
            trace_faces(&duplicate_nodes, &[], None),
            Err(RoadFaceError::DuplicateNode)
        );
        let coincident_nodes = vec![node(1, 0.0, 0.0), node(2, 0.0, 0.0)];
        assert_eq!(
            trace_faces(&coincident_nodes, &[], None),
            Err(RoadFaceError::CoincidentNodes)
        );
        assert!(trace_faces(&nodes, &edges, Some(edge(9, 10, 30))).is_ok());
    }

    #[test]
    fn explicit_input_and_work_caps_are_enforced() {
        let nodes = (0..=MAX_NODES)
            .map(|id| node(id as u32, id as f32 * 2.0, 0.0))
            .collect::<Vec<_>>();
        assert_eq!(
            trace_faces(&nodes, &[], None),
            Err(RoadFaceError::TooManyNodes)
        );
        let nodes = (0..MAX_NODES)
            .map(|id| {
                let angle = id as f32 * std::f32::consts::TAU / MAX_NODES as f32;
                node(id as u32, angle.cos() * 100.0, angle.sin() * 100.0)
            })
            .collect::<Vec<_>>();
        let edges = (0..MAX_EDGES)
            .map(|id| {
                edge(
                    id as u32,
                    id as u32 % MAX_NODES as u32,
                    (id as u32 + 1) % MAX_NODES as u32,
                )
            })
            .collect::<Vec<_>>();
        let traced = trace_faces(&nodes, &edges, None).unwrap();
        assert_eq!(traced.len(), 1);
        assert_eq!(
            trace_faces(&nodes, &edges, Some(edge(MAX_EDGES as u32, 0, 1))),
            Err(RoadFaceError::TooManyEdges)
        );
        assert!(MAX_HALF_EDGE_STEPS <= 1024);
        assert!(MAX_PAIR_TESTS <= 131_072);
        assert_eq!(MAX_ADJACENCY_SEARCH_STEPS, 524_288);
    }

    #[test]
    fn mature_connected_ordinary_road_graph_reports_unsupported_crossing() {
        let engine = crate::Engine::restore_json(include_str!(
            "../../../tests/fixtures/access-joins/spine-experiment-browser-review-save.json"
        ))
        .expect("saved mature world restores");
        let nodes = engine
            .state
            .network
            .nodes
            .iter()
            .map(|node| RoadFaceNode {
                id: node.id,
                x: node.x,
                z: node.z,
            })
            .collect::<Vec<_>>();
        let edges = engine
            .state
            .network
            .roads
            .iter()
            .filter(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
            })
            .map(|road| RoadFaceEdge {
                id: road.id,
                from: road.from,
                to: road.to,
            })
            .collect::<Vec<_>>();
        assert!(nodes.len() <= MAX_NODES);
        assert!(edges.len() <= MAX_EDGES);
        // This save has 215 ordinary roads, but the whole graph is not a valid
        // straight-line planar embedding. Record the first exact collision so
        // integration can distinguish a missing junction from a real crossing
        // or coincident corridor. Do not crop edges by endpoint-only windows:
        // that could omit a road crossing the window from outside.
        assert_eq!(
            trace_faces(&nodes, &edges, None),
            Err(RoadFaceError::NonPlanar)
        );
        let by_id = nodes
            .iter()
            .map(|node| (node.id, *node))
            .collect::<BTreeMap<_, _>>();
        let mut found = false;
        'pairs: for first in 0..edges.len() {
            for second in first + 1..edges.len() {
                let a_edge = edges[first];
                let b_edge = edges[second];
                if [a_edge.from, a_edge.to]
                    .iter()
                    .any(|node| *node == b_edge.from || *node == b_edge.to)
                {
                    continue;
                }
                let (a, b, c, d) = (
                    by_id[&a_edge.from],
                    by_id[&a_edge.to],
                    by_id[&b_edge.from],
                    by_id[&b_edge.to],
                );
                let distance = segments_distance_coords(a.x, a.z, b.x, b.z, c.x, c.z, d.x, d.z);
                if distance <= COINCIDENT_EPSILON {
                    let a_kind = engine
                        .state
                        .network
                        .roads
                        .iter()
                        .find(|road| road.id == a_edge.id)
                        .unwrap()
                        .kind
                        .as_str();
                    let b_kind = engine
                        .state
                        .network
                        .roads
                        .iter()
                        .find(|road| road.id == b_edge.id)
                        .unwrap()
                        .kind
                        .as_str();
                    let a_len = (b.x - a.x).hypot(b.z - a.z);
                    let b_len = (d.x - c.x).hypot(d.z - c.z);
                    let cross = (b.x - a.x) * (d.z - c.z) - (b.z - a.z) * (d.x - c.x);
                    let projects_to_interior =
                        |p: RoadFaceNode, start: RoadFaceNode, end: RoadFaceNode| {
                            let dx = end.x - start.x;
                            let dz = end.z - start.z;
                            let t =
                                ((p.x - start.x) * dx + (p.z - start.z) * dz) / (dx * dx + dz * dz);
                            t > COINCIDENT_EPSILON
                                && t < 1.0 - COINCIDENT_EPSILON
                                && crate::world::geometry::point_segment_distance(
                                    p.x, p.z, start.x, start.z, end.x, end.z,
                                ) <= COINCIDENT_EPSILON
                        };
                    let has_t_junction = projects_to_interior(a, c, d)
                        || projects_to_interior(b, c, d)
                        || projects_to_interior(c, a, b)
                        || projects_to_interior(d, a, b);
                    let classification = if cross.abs() <= COINCIDENT_EPSILON * a_len * b_len {
                        "collinear overlap"
                    } else if has_t_junction {
                        "unsplit T-junction"
                    } else {
                        "proper at-grade crossing"
                    };
                    eprintln!(
                        "mature road-face nonplanarity: {classification}; road {} {:?}->{:?} ({a_kind}) intersects road {} {:?}->{:?} ({b_kind}); distance={distance:.8}",
                        a_edge.id, (a.x, a.z), (b.x, b.z), b_edge.id, (c.x, c.z), (d.x, d.z)
                    );
                    found = true;
                    break 'pairs;
                }
            }
        }
        assert!(
            found,
            "nonplanar classification should identify an offending pair"
        );
    }

    #[test]
    fn new_founder_network_is_planar_connected_and_keeps_nonincident_parcels_clear() {
        for seed in [0, 1, 5, 87_231, u32::MAX] {
            let engine = crate::Engine::new_legacy(seed);
            let market = engine
                .state
                .network
                .nodes
                .iter()
                .find(|node| node.kind == NodeKind::Market)
                .unwrap();
            let station = engine
                .state
                .network
                .nodes
                .iter()
                .find(|node| node.kind == NodeKind::Station)
                .unwrap();
            let gateway = engine
                .state
                .network
                .nodes
                .iter()
                .find(|node| node.kind == NodeKind::Gateway)
                .unwrap();
            let center = engine
                .state
                .network
                .nodes
                .iter()
                .filter(|node| node.kind == NodeKind::Junction)
                .min_by(|a, b| {
                    (a.x - 72.0)
                        .hypot(a.z - 42.0)
                        .total_cmp(&(b.x - 72.0).hypot(b.z - 42.0))
                })
                .unwrap();

            let ordinary = engine
                .state
                .network
                .roads
                .iter()
                .filter(|road| {
                    road.stage == Stage::Open
                        && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
                })
                .collect::<Vec<_>>();
            let participating_ids = ordinary
                .iter()
                .flat_map(|road| [road.from, road.to])
                .collect::<BTreeSet<_>>();
            let nodes = engine
                .state
                .network
                .nodes
                .iter()
                .filter(|node| participating_ids.contains(&node.id))
                .map(|node| RoadFaceNode {
                    id: node.id,
                    x: node.x,
                    z: node.z,
                })
                .collect::<Vec<_>>();
            let edges = ordinary
                .iter()
                .map(|road| RoadFaceEdge {
                    id: road.id,
                    from: road.from,
                    to: road.to,
                })
                .collect::<Vec<_>>();
            let faces = trace_faces(&nodes, &edges, None)
                .expect("new founder ordinary network must have a planar embedding");
            assert!(
                faces.is_empty(),
                "founder roads form a connected tree at seed {seed}"
            );
            assert!(ordinary.iter().all(|road| road.connected));

            let adjacency =
                ordinary
                    .iter()
                    .fold(BTreeMap::<u32, Vec<u32>>::new(), |mut graph, road| {
                        graph.entry(road.from).or_default().push(road.to);
                        graph.entry(road.to).or_default().push(road.from);
                        graph
                    });
            let reaches = |start: u32, end: u32| {
                let mut todo = std::collections::VecDeque::from([start]);
                let mut seen = BTreeSet::from([start]);
                while let Some(current) = todo.pop_front() {
                    if current == end {
                        return true;
                    }
                    for next in adjacency.get(&current).into_iter().flatten() {
                        if seen.insert(*next) {
                            todo.push_back(*next);
                        }
                    }
                }
                false
            };
            assert!(reaches(gateway.id, market.id));
            assert!(reaches(market.id, station.id));
            assert!(reaches(center.id, station.id));
            assert!(!ordinary.iter().any(|road| {
                (road.from == market.id && road.to == station.id)
                    || (road.from == station.id && road.to == market.id)
            }));

            let connector = ordinary
                .iter()
                .find(|road| {
                    (road.from == center.id && road.to == station.id)
                        || (road.from == station.id && road.to == center.id)
                })
                .unwrap();
            let (a, b) = (
                engine.node(connector.from).unwrap(),
                engine.node(connector.to).unwrap(),
            );
            for parcel in &engine.state.land.parcels {
                let fronts_this_connector =
                    engine.state.land.parcel_frontages.iter().any(|frontage| {
                        frontage.parcel_id == parcel.id && frontage.road_id == connector.id
                    });
                if fronts_this_connector {
                    continue;
                }
                let rect = crate::world::geometry::OrientedRect {
                    x: parcel.x,
                    z: parcel.z,
                    width: parcel.width,
                    depth: parcel.depth,
                    rotation: parcel
                        .rotation
                        .unwrap_or_else(|| engine.frontage_rotation(parcel.id)),
                };
                assert!(
                    crate::world::geometry::rect_segment_distance(&rect, a.x, a.z, b.x, b.z) >= 5.0,
                    "seed {seed}: connector intersects nonincident parcel {}",
                    parcel.id
                );
            }
        }
    }

    #[test]
    #[ignore = "bounded fresh-world topology diagnostic for mature growth; run explicitly with --ignored --nocapture"]
    fn fresh_mature_world_ordinary_graph_topology_diagnostic() {
        let mut worlds = Vec::new();
        for seed in [0, 87_231] {
            let mut engine = crate::Engine::new_legacy(seed);
            engine.set_development_rate(16);
            for _ in 0..121 {
                engine.step(2_000);
            }
            assert_eq!(
                engine.tick(),
                242_000,
                "seed {seed} must reach the requested horizon"
            );
            worlds.push((format!("native seed={seed}"), engine));
        }
        let wasm_world = crate::Engine::restore_json(include_str!(
            "../../../tests/fixtures/access-joins/founder-topology-wasm-87231-save.json"
        ))
        .expect("public WASM checkpoint restores for read-only topology inspection");
        assert_eq!(wasm_world.tick(), 242_000);
        worlds.push(("public WASM seed=87231 checkpoint".to_owned(), wasm_world));

        for (label, engine) in worlds {
            let roads = engine
                .state
                .network
                .roads
                .iter()
                .filter(|road| {
                    road.stage == Stage::Open
                        && road.connected
                        && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
                })
                .map(|road| RoadFaceEdge {
                    id: road.id,
                    from: road.from,
                    to: road.to,
                })
                .collect::<Vec<_>>();
            let participating = roads
                .iter()
                .flat_map(|road| [road.from, road.to])
                .collect::<BTreeSet<_>>();
            let nodes = engine
                .state
                .network
                .nodes
                .iter()
                .filter(|node| participating.contains(&node.id))
                .map(|node| RoadFaceNode {
                    id: node.id,
                    x: node.x,
                    z: node.z,
                })
                .collect::<Vec<_>>();

            // Never interpret a capped/partial graph as planar. These seeds
            // are expected to stay inside the tracer's explicit local-graph cap.
            assert!(
                nodes.len() <= MAX_NODES,
                "{label}: node cap exceeded ({})",
                nodes.len()
            );
            assert!(
                roads.len() <= MAX_EDGES,
                "{label}: edge cap exceeded ({})",
                roads.len()
            );
            let traced = trace_faces(&nodes, &roads, None);
            match &traced {
                Ok(faces) => eprintln!(
                    "fresh topology {label} tick={} nodes={} ordinary_open_connected_roads={} result=planar bounded_faces={}",
                    engine.tick(), nodes.len(), roads.len(), faces.len()
                ),
                Err(error) => {
                    let by_id = nodes.iter().map(|node| (node.id, *node)).collect::<BTreeMap<_, _>>();
                    let mut offending = None;
                    'pairs: for first in 0..roads.len() {
                        for second in first + 1..roads.len() {
                            let left = roads[first];
                            let right = roads[second];
                            if [left.from, left.to]
                                .iter()
                                .any(|id| *id == right.from || *id == right.to)
                            {
                                continue;
                            }
                            let a = by_id[&left.from];
                            let b = by_id[&left.to];
                            let c = by_id[&right.from];
                            let d = by_id[&right.to];
                            let distance = segments_distance_coords(a.x, a.z, b.x, b.z, c.x, c.z, d.x, d.z);
                            if distance <= COINCIDENT_EPSILON {
                                let cross = (b.x - a.x) * (d.z - c.z) - (b.z - a.z) * (d.x - c.x);
                                let first_length = (b.x - a.x).hypot(b.z - a.z);
                                let second_length = (d.x - c.x).hypot(d.z - c.z);
                                let projected_interior = |point: RoadFaceNode, start: RoadFaceNode, end: RoadFaceNode| {
                                    let dx = end.x - start.x;
                                    let dz = end.z - start.z;
                                    let t = ((point.x - start.x) * dx + (point.z - start.z) * dz)
                                        / (dx * dx + dz * dz);
                                    t > COINCIDENT_EPSILON
                                        && t < 1.0 - COINCIDENT_EPSILON
                                        && crate::world::geometry::point_segment_distance(
                                            point.x, point.z, start.x, start.z, end.x, end.z,
                                        ) <= COINCIDENT_EPSILON
                                };
                                let has_t_junction = projected_interior(a, c, d)
                                    || projected_interior(b, c, d)
                                    || projected_interior(c, a, b)
                                    || projected_interior(d, a, b);
                                let kind = if cross.abs() <= COINCIDENT_EPSILON * first_length * second_length {
                                    "collinear overlap"
                                } else if has_t_junction {
                                    "unsplit T-junction"
                                } else {
                                    "proper at-grade crossing"
                                };
                                offending = Some((left.id, (a.x, a.z), (b.x, b.z), right.id, (c.x, c.z), (d.x, d.z), distance, kind));
                                break 'pairs;
                            }
                        }
                    }
                    eprintln!(
                        "fresh topology {label} tick={} nodes={} ordinary_open_connected_roads={} result={error:?} offending_pair={offending:?}",
                        engine.tick(), nodes.len(), roads.len()
                    );
                }
            }
            assert!(
                traced.is_ok(),
                "fresh mature ordinary-road graph regressed from a valid planar embedding: {label}"
            );
        }
    }

    #[test]
    #[ignore = "audits the exact public-WASM early-reservation checkpoint without advancing it"]
    fn early_reservation_wasm_checkpoint_has_planar_ordinary_network() {
        use serde_json::json;
        use std::path::Path;

        const SAVE_PATH: &str =
            "../tests/fixtures/validation/early-reservation-wasm-87231-save.json";
        const REPORT_PATH: &str = "../validation/early-reservation-wasm-87231-report.json";
        const OUTPUT_PATH: &str = "../validation/early-reservation-wasm-87231-topology.json";

        let save = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(SAVE_PATH))
            .expect("read exact public WASM save");
        let provenance: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(REPORT_PATH))
                .expect("read checkpoint provenance"),
        )
        .expect("parse checkpoint provenance");
        let save_sha256 = provenance["saveSha256"]
            .as_str()
            .expect("checkpoint report records save SHA-256");
        assert_eq!(
            save_sha256, "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
            "checkpoint provenance must refer to the reviewed public WASM save"
        );

        let engine = crate::Engine::restore_json(&save)
            .expect("exact public WASM checkpoint restores without mutation");
        assert_eq!(engine.tick(), 242_000);
        assert_eq!(engine.state.seed, 87_231);

        let mut excluded = BTreeMap::<String, usize>::new();
        let mut selected = Vec::new();
        for road in &engine.state.network.roads {
            let reason = if road.stage != Stage::Open {
                Some("not_open")
            } else if !road.connected {
                Some("disconnected")
            } else if road.kind == RoadKind::Rail {
                Some("rail")
            } else if road.kind == RoadKind::Bridge {
                Some("bridge")
            } else if !matches!(road.kind, RoadKind::Local | RoadKind::Arterial) {
                Some("other_kind")
            } else {
                None
            };
            if let Some(reason) = reason {
                *excluded.entry(reason.to_owned()).or_default() += 1;
            } else {
                selected.push(RoadFaceEdge {
                    id: road.id,
                    from: road.from,
                    to: road.to,
                });
            }
        }
        let participating = selected
            .iter()
            .flat_map(|road| [road.from, road.to])
            .collect::<BTreeSet<_>>();
        let nodes = engine
            .state
            .network
            .nodes
            .iter()
            .filter(|node| participating.contains(&node.id))
            .map(|node| RoadFaceNode {
                id: node.id,
                x: node.x,
                z: node.z,
            })
            .collect::<Vec<_>>();
        assert!(nodes.len() <= MAX_NODES, "do not interpret a capped graph");
        assert!(
            selected.len() <= MAX_EDGES,
            "do not interpret a capped graph"
        );
        let faces = trace_faces(&nodes, &selected, None)
            .expect("ordinary connected road network must have no planarity errors");
        assert!(
            !faces.is_empty(),
            "checkpoint should contain bounded blocks"
        );

        let audit = json!({
            "checkpoint": "tests/fixtures/validation/early-reservation-wasm-87231-save.json",
            "checkpoint_sha256": save_sha256,
            "source": "public WASM at seed 87231, rate 16, tick 242000; restored natively for read-only graph audit",
            "tick": engine.tick(),
            "seed": engine.state.seed,
            "all_road_records": engine.state.network.roads.len(),
            "ordinary_open_connected_edges": selected.len(),
            "participating_nodes": nodes.len(),
            "bounded_faces": faces.len(),
            "excluded_layers": excluded,
            "planarity_error": null,
            "face_identity_sample": faces.iter().take(8).map(|face| &face.identity).collect::<Vec<_>>(),
            "limits": { "max_nodes": MAX_NODES, "max_edges": MAX_EDGES },
            "classification": "road-face audit only; rail and bridge roads are excluded because they use separate or elevated layers"
        });
        std::fs::write(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(OUTPUT_PATH),
            serde_json::to_vec_pretty(&audit).expect("serialize topology audit"),
        )
        .expect("write checkpoint topology report");
        eprintln!("early-reservation WASM topology audit: {audit}");
    }
}
