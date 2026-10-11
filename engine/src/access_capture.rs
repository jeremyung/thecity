//! Atomic, read-only projection from the live engine into the immutable input
//! consumed by `DiscoveryJob`.
//!
//! Capture is intentionally synchronous at a single tick boundary. The job
//! that consumes this value is resumable; this adapter does not pretend that
//! walking mutable engine vectors over multiple ticks would be a consistent
//! snapshot.

use crate::access_candidates::{
    CandidateNode, MAX_PREPARED_INPUT_NODES, MAX_PREPARED_INPUT_PARCELS,
};
use crate::access_discovery_job::DiscoveryInput;
use crate::face_target_viability::{FrozenViabilityInput, Segment};
use crate::road_faces::{RoadFaceEdge, RoadFaceNode};
use crate::{Engine, MAX_PROJECTS, MAX_SEGMENTS};
use std::collections::BTreeSet;

const MAX_FACE_GRAPH_NODES: usize = 512;
const MAX_FACE_GRAPH_EDGES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureError {
    TooManyGraphNodes,
    TooManyGraphEdges,
    TooManyCandidateNodes,
    TooManyParcelFootprints,
    TooManyOpenRoads,
    TooManyPendingRoads,
}

/// Capture the complete current geometry projection in source order.
///
/// The graph includes every open, connected local or arterial road and every
/// node participating in those roads. It fails closed above the exact face
/// tracer's limits instead of truncating the city graph. The remaining
/// candidate-node and collision arrays retain their engine source order to
/// match the existing frozen diagnostic oracle.
pub(crate) fn capture(engine: &Engine) -> Result<DiscoveryInput, CaptureError> {
    if engine.state.nodes.len() > MAX_PREPARED_INPUT_NODES {
        return Err(CaptureError::TooManyCandidateNodes);
    }

    let ordinary_road_count = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
        })
        .count();
    if ordinary_road_count > MAX_FACE_GRAPH_EDGES {
        return Err(CaptureError::TooManyGraphEdges);
    }

    let projected_footprint_count = engine
        .state
        .parcels
        .iter()
        .map(|parcel| 1 + usize::from(parcel.rotation.is_none()))
        .sum::<usize>();
    if projected_footprint_count > MAX_PREPARED_INPUT_PARCELS {
        return Err(CaptureError::TooManyParcelFootprints);
    }
    let open_road_count = engine
        .state
        .roads
        .iter()
        .filter(|road| road.stage == "open")
        .count();
    if open_road_count > MAX_SEGMENTS {
        return Err(CaptureError::TooManyOpenRoads);
    }
    let pending_road_count = engine
        .state
        .projects
        .iter()
        .filter(|project| project.kind == "road" && project.progress < 1.0)
        .count();
    if pending_road_count > MAX_PROJECTS {
        return Err(CaptureError::TooManyPendingRoads);
    }

    let ordinary_roads = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
        })
        .collect::<Vec<_>>();

    // The world cap allows more roads than the face tracer. Never feed a
    // partial embedding to face discovery: it could invent an interior face.
    let used_node_ids = ordinary_roads
        .iter()
        .flat_map(|road| [road.from, road.to])
        .collect::<BTreeSet<_>>();
    let graph_nodes = engine
        .state
        .nodes
        .iter()
        .filter(|node| used_node_ids.contains(&node.id))
        .map(|node| RoadFaceNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();
    if graph_nodes.len() > MAX_FACE_GRAPH_NODES {
        return Err(CaptureError::TooManyGraphNodes);
    }

    let graph_edges = ordinary_roads
        .iter()
        .map(|road| RoadFaceEdge {
            id: road.id,
            from: road.from,
            to: road.to,
        })
        .collect::<Vec<_>>();

    let all_nodes = engine
        .state
        .nodes
        .iter()
        .map(|node| CandidateNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();

    // This is the same collision projection used by live admission. In
    // particular it retains both legacy and rendered envelopes for parcels
    // with an absent saved rotation.
    let collision = engine.collision_context();

    let reserved_lots = engine
        .state
        .pending_interior_access
        .as_ref()
        .map(|plan| {
            plan.selected_lots
                .iter()
                .map(|lot| crate::geometry::OrientedRect {
                    x: lot.x,
                    z: lot.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                })
                .collect()
        })
        .unwrap_or_default();

    let neighborhood_segments = engine
        .state
        .neighborhood_plan
        .as_ref()
        .map(|plan| {
            plan.segments
                .iter()
                .skip(plan.next_segment)
                .filter_map(|&(from, to)| {
                    let (a, b) = (engine.node(from)?, engine.node(to)?);
                    Some(Segment {
                        ax: a.x,
                        az: a.z,
                        bx: b.x,
                        bz: b.z,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let neighborhood_interior = engine.state.neighborhood_plan.as_ref().and_then(|plan| {
        if plan.coordinated_lots && plan.segments.len() == 4 {
            engine.coordinated_block_interior(&plan.segments)
        } else {
            None
        }
    });

    let river = engine.river_control_points();
    let viability = FrozenViabilityInput {
        parcel_footprints: collision.parcel_footprints,
        reserved_lots,
        open_roads: collision
            .open_roads
            .into_iter()
            .map(|segment| Segment {
                ax: segment.ax,
                az: segment.az,
                bx: segment.bx,
                bz: segment.bz,
            })
            .collect(),
        pending_roads: collision
            .pending_roads
            .into_iter()
            .map(|segment| Segment {
                ax: segment.ax,
                az: segment.az,
                bx: segment.bx,
                bz: segment.bz,
            })
            .collect(),
        river_segments: river
            .windows(2)
            .map(|points| Segment {
                ax: points[0].x,
                az: points[0].z,
                bx: points[1].x,
                bz: points[1].z,
            })
            .collect(),
        neighborhood_segments,
        neighborhood_interior,
    };

    // Protect the descriptor stage's limits here too, so an input cannot be
    // captured successfully and only fail after a long persisted search.
    Ok(DiscoveryInput {
        graph_nodes,
        graph_edges,
        all_nodes,
        viability,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face_access_exact_tests::frozen_target_viability_input;

    #[test]
    fn capture_matches_mature_frozen_projection_oracle() {
        let engine = Engine::restore_json(include_str!(
            "../../validation/early-reservation-wasm-87231-save.json"
        ))
        .expect("mature save restores");
        let captured = capture(&engine).expect("mature graph is within the exact tracer cap");

        let ordinary = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
            })
            .collect::<Vec<_>>();
        let used = ordinary
            .iter()
            .flat_map(|road| [road.from, road.to])
            .collect::<BTreeSet<_>>();
        let graph_nodes = engine
            .state
            .nodes
            .iter()
            .filter(|node| used.contains(&node.id))
            .map(|node| RoadFaceNode {
                id: node.id,
                x: node.x,
                z: node.z,
            })
            .collect::<Vec<_>>();
        let graph_edges = ordinary
            .iter()
            .map(|road| RoadFaceEdge {
                id: road.id,
                from: road.from,
                to: road.to,
            })
            .collect::<Vec<_>>();
        let all_nodes = engine
            .state
            .nodes
            .iter()
            .map(|node| CandidateNode {
                id: node.id,
                x: node.x,
                z: node.z,
            })
            .collect::<Vec<_>>();
        let expected = DiscoveryInput {
            graph_nodes,
            graph_edges,
            all_nodes,
            viability: frozen_target_viability_input(&engine),
        };
        assert_eq!(
            serde_json::to_vec(&captured).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
    }

    #[test]
    fn capture_is_read_only_and_initial_world_counts_are_bounded() {
        let engine = Engine::new(41);
        let before = engine.save_json();
        let captured = capture(&engine).expect("initial city fits all caps");
        assert!(captured.graph_nodes.len() <= MAX_FACE_GRAPH_NODES);
        assert!(captured.graph_edges.len() <= MAX_FACE_GRAPH_EDGES);
        assert!(captured.all_nodes.len() <= MAX_PREPARED_INPUT_NODES);
        assert!(captured.viability.parcel_footprints.len() <= MAX_PREPARED_INPUT_PARCELS);
        assert_eq!(engine.save_json(), before);
    }

    #[test]
    fn over_cap_graph_fails_without_truncating_or_mutating_world() {
        let mut engine = Engine::new(42);
        let road = engine
            .state
            .roads
            .iter()
            .find(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
            })
            .expect("founding city has an ordinary open road")
            .clone();
        let original = engine.state.roads.clone();
        for _ in 0..=MAX_FACE_GRAPH_EDGES {
            engine.state.roads.push(road.clone());
        }
        let before_capture = engine.save_json();
        let result = capture(&engine);
        assert!(matches!(result, Err(CaptureError::TooManyGraphEdges)));
        assert_eq!(engine.save_json(), before_capture);
        assert_eq!(
            engine.state.roads.len(),
            original.len() + MAX_FACE_GRAPH_EDGES + 1
        );
    }
}
