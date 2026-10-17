//! Resumable composition of graph preflight, bridge detection, half-edge
//! topology, and bounded-face walking.
//!
//! Stage handoffs move their owned buffers. A handoff itself costs one work
//! unit, as does one unit from the active stage. Construction performs only
//! capped input validation synchronously; all graph scans and output-buffer
//! initialization are advanced under the caller's budget. An optional proposed
//! edge has its append capacity reserved during preflight construction/restore,
//! so the handoff moves buffers and appends it without reallocating. Setup and
//! persistence allocations remain outside the logical advance work counter.

use crate::legacy::road_face_bridges::{BridgeError, BridgeSearch};
use crate::legacy::road_face_preflight::{GraphPreflight, GraphPreflightCheckpoint};
use crate::legacy::road_face_topology::{TopologyError, TopologySearch};
use crate::legacy::road_face_walk::{Checkpoint as WalkCheckpoint, FaceWalkSearch};
use crate::legacy::road_faces::{RoadFace, RoadFaceEdge, RoadFaceError, RoadFaceNode};
use serde::{Deserialize, Serialize};

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_NODES: usize = 512;
const MAX_EDGES: usize = 512;
const MAX_FACES: usize = 512;
const MAX_HALF_EDGES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum StageCheckpoint {
    Preflight(GraphPreflightCheckpoint),
    Bridges {
        checkpoint: crate::legacy::road_face_bridges::Checkpoint,
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
    },
    Topology(crate::legacy::road_face_topology::Checkpoint),
    Walk(WalkCheckpoint),
    Done(Vec<RoadFace>),
    Failed(RoadFaceError),
}

/// Serialized DTO. Callers must use `RoadFaceTrace::restore` rather than
/// deserializing the runtime job directly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    schema: u8,
    proposed_edge_id: Option<u32>,
    stage: StageCheckpoint,
}

#[derive(Clone, Debug, PartialEq)]
enum RuntimeStage {
    Preflight(GraphPreflight),
    Bridges {
        search: BridgeSearch,
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
    },
    Topology(TopologySearch),
    Walk(FaceWalkSearch),
    Done(Vec<RoadFace>),
    Failed(RoadFaceError),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RoadFaceTrace {
    proposed_edge_id: Option<u32>,
    stage: RuntimeStage,
}

impl RoadFaceTrace {
    #[cfg(feature = "legacy")]
    pub(crate) fn new(
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
        proposed_edge: Option<RoadFaceEdge>,
    ) -> Result<Self, RoadFaceError> {
        let proposed_edge_id = proposed_edge.map(|edge| edge.id);
        Ok(Self {
            proposed_edge_id,
            stage: RuntimeStage::Preflight(GraphPreflight::new(nodes, edges, proposed_edge)?),
        })
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        let stage = match &self.stage {
            RuntimeStage::Preflight(search) => StageCheckpoint::Preflight(search.checkpoint()),
            RuntimeStage::Bridges {
                search,
                nodes,
                edges,
            } => StageCheckpoint::Bridges {
                checkpoint: search.checkpoint(),
                nodes: nodes.clone(),
                edges: edges.clone(),
            },
            RuntimeStage::Topology(search) => StageCheckpoint::Topology(search.checkpoint()),
            RuntimeStage::Walk(search) => StageCheckpoint::Walk(search.checkpoint()),
            RuntimeStage::Done(faces) => StageCheckpoint::Done(faces.clone()),
            RuntimeStage::Failed(error) => StageCheckpoint::Failed(*error),
        };
        Checkpoint {
            schema: CHECKPOINT_SCHEMA,
            proposed_edge_id: self.proposed_edge_id,
            stage,
        }
    }

    pub(crate) fn restore(checkpoint: Checkpoint) -> Result<Self, RoadFaceError> {
        if checkpoint.schema != CHECKPOINT_SCHEMA {
            return Err(RoadFaceError::InvalidEmbedding);
        }
        let proposed_edge_id = checkpoint.proposed_edge_id;
        let stage = match checkpoint.stage {
            StageCheckpoint::Preflight(stage_checkpoint) => {
                let search = GraphPreflight::restore(stage_checkpoint)?;
                if search.proposed_edge_id() != proposed_edge_id {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
                RuntimeStage::Preflight(search)
            }
            StageCheckpoint::Bridges {
                checkpoint: stage_checkpoint,
                nodes,
                edges,
            } => {
                if nodes.len() > MAX_NODES
                    || edges.len() > MAX_EDGES
                    || nodes
                        .iter()
                        .any(|node| !node.x.is_finite() || !node.z.is_finite())
                {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
                if proposed_edge_id.is_some_and(|id| edges.last().is_none_or(|edge| edge.id != id))
                {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
                let search = BridgeSearch::restore(stage_checkpoint).map_err(map_bridge_error)?;
                if !search.matches_graph(&nodes, &edges) {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
                RuntimeStage::Bridges {
                    search,
                    nodes,
                    edges,
                }
            }
            StageCheckpoint::Topology(stage_checkpoint) => {
                let search =
                    TopologySearch::restore(stage_checkpoint).map_err(map_topology_error)?;
                if proposed_edge_id.is_some_and(|id| search.last_edge_id() != Some(id)) {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
                RuntimeStage::Topology(search)
            }
            StageCheckpoint::Walk(stage_checkpoint) => {
                let search = FaceWalkSearch::restore(stage_checkpoint)?;
                if search.proposed_edge_id() != proposed_edge_id {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
                RuntimeStage::Walk(search)
            }
            StageCheckpoint::Done(faces) => {
                validate_faces(&faces)?;
                RuntimeStage::Done(faces)
            }
            StageCheckpoint::Failed(error) => RuntimeStage::Failed(error),
        };
        Ok(Self {
            proposed_edge_id: checkpoint.proposed_edge_id,
            stage,
        })
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn faces(&self) -> Option<Result<&[RoadFace], RoadFaceError>> {
        match &self.stage {
            RuntimeStage::Done(faces) => Some(Ok(faces)),
            RuntimeStage::Failed(error) => Some(Err(*error)),
            _ => None,
        }
    }

    /// Move completed output into the next job without copying face buffers.
    #[cfg(feature = "legacy")]
    pub(crate) fn into_faces(self) -> Option<Result<Vec<RoadFace>, RoadFaceError>> {
        match self.stage {
            RuntimeStage::Done(faces) => Some(Ok(faces)),
            RuntimeStage::Failed(error) => Some(Err(error)),
            _ => None,
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || matches!(self.stage, RuntimeStage::Done(_) | RuntimeStage::Failed(_)) {
            return AdvanceResult {
                work_used: 0,
                done: self.faces().is_some(),
            };
        }
        let mut work_used = 0;
        while work_used < budget && self.faces().is_none() {
            let handoff_ready = match &self.stage {
                RuntimeStage::Preflight(search) => {
                    search.result().is_some_and(|result| result.is_ok())
                }
                RuntimeStage::Bridges { search, .. } => search.bridges().is_some(),
                RuntimeStage::Topology(search) => search.topology().is_some(),
                RuntimeStage::Walk(search) => search.faces().is_some_and(|result| result.is_ok()),
                RuntimeStage::Done(_) | RuntimeStage::Failed(_) => false,
            };
            if handoff_ready {
                self.handoff();
                work_used += 1;
                continue;
            }
            match &mut self.stage {
                RuntimeStage::Preflight(search) => {
                    let result = search.advance(1);
                    work_used += result.work_used.max(1);
                    if let Some(Err(error)) = search.result() {
                        self.stage = RuntimeStage::Failed(error);
                    }
                }
                RuntimeStage::Bridges { search, .. } => {
                    let result = search.advance(1);
                    work_used += result.work_used.max(1);
                }
                RuntimeStage::Topology(search) => {
                    let result = search.advance(1);
                    work_used += result.work_used.max(1);
                }
                RuntimeStage::Walk(search) => {
                    let result = search.advance(1);
                    work_used += result.work_used.max(1);
                    if let Some(Err(error)) = search.faces() {
                        self.stage = RuntimeStage::Failed(error);
                    }
                }
                RuntimeStage::Done(_) | RuntimeStage::Failed(_) => break,
            }
        }
        AdvanceResult {
            work_used: work_used.min(budget),
            done: self.faces().is_some(),
        }
    }

    #[cfg(feature = "legacy")]
    fn handoff(&mut self) {
        let next = match std::mem::replace(
            &mut self.stage,
            RuntimeStage::Failed(RoadFaceError::InvalidEmbedding),
        ) {
            RuntimeStage::Preflight(search) => match search.into_result() {
                Err(error) => RuntimeStage::Failed(error),
                Ok((nodes, edges, indexed)) => RuntimeStage::Bridges {
                    search: BridgeSearch::new_trusted(nodes.len(), indexed),
                    nodes,
                    edges,
                },
            },
            RuntimeStage::Bridges {
                search,
                nodes,
                edges,
            } => match search.into_result() {
                Err(error) => RuntimeStage::Failed(map_bridge_error(error)),
                Ok((indexed, bridges)) => RuntimeStage::Topology(TopologySearch::new_trusted(
                    nodes, edges, indexed, bridges,
                )),
            },
            RuntimeStage::Topology(search) => match search.into_result() {
                None => RuntimeStage::Failed(RoadFaceError::InvalidEmbedding),
                Some((nodes, edges, topology)) => match FaceWalkSearch::new_trusted(
                    nodes,
                    edges,
                    topology.half_edges,
                    topology.next,
                    self.proposed_edge_id,
                ) {
                    Err(error) => RuntimeStage::Failed(error),
                    Ok(search) => RuntimeStage::Walk(search),
                },
            },
            RuntimeStage::Walk(search) => match search.into_faces() {
                Err(error) => RuntimeStage::Failed(error),
                Ok(faces) => RuntimeStage::Done(faces),
            },
            stage => stage,
        };
        self.stage = next;
    }
}

fn map_bridge_error(error: BridgeError) -> RoadFaceError {
    match error {
        BridgeError::TooManyNodes => RoadFaceError::TooManyNodes,
        BridgeError::TooManyEdges => RoadFaceError::TooManyEdges,
        BridgeError::InvalidEndpoint | BridgeError::InvalidCheckpoint => {
            RoadFaceError::InvalidEmbedding
        }
    }
}

fn map_topology_error(error: TopologyError) -> RoadFaceError {
    match error {
        TopologyError::TooManyNodes => RoadFaceError::TooManyNodes,
        TopologyError::TooManyEdges => RoadFaceError::TooManyEdges,
        TopologyError::InvalidInput | TopologyError::InvalidCheckpoint => {
            RoadFaceError::InvalidEmbedding
        }
    }
}

fn validate_faces(faces: &[RoadFace]) -> Result<(), RoadFaceError> {
    if faces.len() > MAX_FACES
        || faces
            .iter()
            .map(|face| face.vertex_ids.len())
            .sum::<usize>()
            > MAX_HALF_EDGES
        || faces.iter().any(|face| {
            face.identity.len() < 3
                || face.identity.len() > MAX_HALF_EDGES
                || face.vertex_ids.len() != face.identity.len()
                || face.polygon.len() != face.identity.len()
                || face.boundary_edge_ids.len() != face.identity.len()
                || !face.area.is_finite()
                || face
                    .polygon
                    .iter()
                    .any(|(x, z)| !x.is_finite() || !z.is_finite())
        })
    {
        return Err(RoadFaceError::InvalidEmbedding);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::road_faces::trace_faces;

    fn node(id: u32, x: f32, z: f32) -> RoadFaceNode {
        RoadFaceNode { id, x, z }
    }
    fn edge(id: u32, from: u32, to: u32) -> RoadFaceEdge {
        RoadFaceEdge { id, from, to }
    }

    fn run(
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
        proposal: Option<RoadFaceEdge>,
        quotas: &[usize],
    ) -> Result<Vec<RoadFace>, RoadFaceError> {
        let expected = trace_faces(&nodes, &edges, proposal);
        let mut trace = RoadFaceTrace::new(nodes, edges, proposal).unwrap();
        let mut turn = 0;
        while trace.faces().is_none() {
            let quota = quotas[turn % quotas.len()];
            let before = trace.checkpoint();
            if quota == 0 {
                let result = trace.advance(0);
                assert_eq!(result.work_used, 0);
                assert!(!result.done);
                assert_eq!(trace.checkpoint(), before);
            } else {
                let result = trace.advance(quota);
                assert!(result.work_used <= quota);
                assert!(result.done || result.work_used > 0);
            }
            let encoded = serde_json::to_vec(&trace.checkpoint()).unwrap();
            let checkpoint: Checkpoint = serde_json::from_slice(&encoded).unwrap();
            trace = RoadFaceTrace::restore(checkpoint).unwrap_or_else(|error| {
                panic!(
                    "restore failed at turn {turn} stage {:?}: {error:?}",
                    trace.checkpoint().stage
                )
            });
            turn += 1;
            assert!(turn < 3_000_000, "trace made no bounded progress");
        }
        let actual = trace.faces().unwrap().map(|faces| faces.to_vec());
        assert_eq!(actual, expected);
        actual
    }

    #[test]
    fn composed_job_matches_reference_across_all_stage_handoffs_and_json_resumes() {
        let rectangle_nodes = vec![
            node(10, 0., 0.),
            node(20, 8., 0.),
            node(30, 8., 5.),
            node(40, 0., 5.),
        ];
        let rectangle_edges = vec![
            edge(1, 10, 20),
            edge(2, 20, 30),
            edge(3, 30, 40),
            edge(4, 40, 10),
        ];
        let adjacent_nodes = vec![
            node(1, 0., 0.),
            node(2, 5., 0.),
            node(3, 10., 0.),
            node(4, 0., 4.),
            node(5, 5., 4.),
            node(6, 10., 4.),
        ];
        let adjacent_edges = vec![
            edge(10, 1, 2),
            edge(11, 2, 3),
            edge(12, 3, 6),
            edge(13, 6, 5),
            edge(14, 5, 4),
            edge(15, 4, 1),
            edge(16, 2, 5),
        ];
        let nested_nodes = vec![
            node(10, 0., 0.),
            node(20, 10., 0.),
            node(30, 10., 10.),
            node(40, 0., 10.),
            node(50, 2., 2.),
            node(60, 4., 2.),
            node(70, 4., 4.),
            node(80, 2., 4.),
        ];
        let nested_edges = vec![
            edge(1, 10, 20),
            edge(2, 20, 30),
            edge(3, 30, 40),
            edge(4, 40, 10),
            edge(5, 50, 60),
            edge(6, 60, 70),
            edge(7, 70, 80),
            edge(8, 80, 50),
        ];
        let crossing_nodes = vec![
            node(1, 0., 0.),
            node(2, 4., 4.),
            node(3, 0., 4.),
            node(4, 4., 0.),
        ];
        let crossing_edges = vec![edge(1, 1, 2), edge(2, 3, 4)];

        for quotas in [&[1][..], &[7][..], &[48][..], &[3, 0, 1, 9, 2][..]] {
            run(
                rectangle_nodes.clone(),
                rectangle_edges.clone(),
                None,
                quotas,
            );
            run(
                rectangle_nodes.clone(),
                rectangle_edges[..3].to_vec(),
                Some(rectangle_edges[3]),
                quotas,
            );
            run(adjacent_nodes.clone(), adjacent_edges.clone(), None, quotas);
            assert_eq!(
                run(nested_nodes.clone(), nested_edges.clone(), None, quotas),
                Err(RoadFaceError::NestedFacesUnsupported)
            );
            assert_eq!(
                run(crossing_nodes.clone(), crossing_edges.clone(), None, quotas),
                Err(RoadFaceError::NonPlanar)
            );
            run(Vec::new(), Vec::new(), None, quotas);
        }
    }

    #[test]
    fn invalid_composed_checkpoints_are_rejected_at_each_stage() {
        let nodes = vec![
            node(1, 0., 0.),
            node(2, 4., 0.),
            node(3, 4., 4.),
            node(4, 0., 4.),
        ];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3), edge(3, 3, 4), edge(4, 4, 1)];
        let mut trace = RoadFaceTrace::new(nodes, edges, None).unwrap();
        let initial = trace.checkpoint();
        let mut bad = initial.clone();
        bad.schema = CHECKPOINT_SCHEMA + 1;
        assert_eq!(
            RoadFaceTrace::restore(bad).unwrap_err(),
            RoadFaceError::InvalidEmbedding
        );

        // The substage restore constructors provide the detailed cursor
        // validation; here verify a corrupted nested bridge context is also
        // rejected by the composed wrapper.
        let mut reached_bridge = false;
        for _ in 0..50_000 {
            trace.advance(1);
            if let StageCheckpoint::Bridges { checkpoint, .. } = trace.checkpoint().stage {
                let mut value = serde_json::to_value(trace.checkpoint()).unwrap();
                let inner = value["stage"]["Bridges"]["checkpoint"]
                    .as_object_mut()
                    .unwrap();
                // Corrupt the nested state through its wire representation so
                // this test does not widen checkpoint internals for visibility.
                inner.insert("node_count".into(), serde_json::json!(usize::MAX));
                let cp: Checkpoint = serde_json::from_value(value).unwrap();
                assert_eq!(
                    RoadFaceTrace::restore(cp).unwrap_err(),
                    RoadFaceError::InvalidEmbedding
                );
                let _ = checkpoint;
                reached_bridge = true;
                break;
            }
        }
        assert!(reached_bridge);
    }
}
