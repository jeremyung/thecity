//! Resumable validation and planarity preflight for road-face tracing.
//!
//! This stage deliberately stops before bridge detection and half-edge face
//! tracing. It preserves the validation and pair-test order of
//! `road_faces::trace_faces` while exposing every scan to a deterministic
//! work budget.

use crate::geometry::segments_distance_coords;
use crate::road_faces::{RoadFaceEdge, RoadFaceError, RoadFaceNode};
use serde::{Deserialize, Serialize};

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_NODES: usize = 512;
const MAX_EDGES: usize = 512;
const MAX_PAIR_TESTS: usize = 131_072;
const COINCIDENT_EPSILON: f32 = 1.0e-5;
const MAX_PREFLIGHT_WORK: usize = 1_180_160;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreflightAdvance {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum PreflightError {
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

impl From<RoadFaceError> for PreflightError {
    fn from(value: RoadFaceError) -> Self {
        match value {
            RoadFaceError::TooManyNodes => Self::TooManyNodes,
            RoadFaceError::TooManyEdges => Self::TooManyEdges,
            RoadFaceError::MissingNode => Self::MissingNode,
            RoadFaceError::DuplicateNode => Self::DuplicateNode,
            RoadFaceError::InvalidCoordinate => Self::InvalidCoordinate,
            RoadFaceError::CoincidentNodes => Self::CoincidentNodes,
            RoadFaceError::DuplicateEdge => Self::DuplicateEdge,
            RoadFaceError::InvalidEdge => Self::InvalidEdge,
            RoadFaceError::NonPlanar => Self::NonPlanar,
            RoadFaceError::WorkLimit => Self::WorkLimit,
            RoadFaceError::InvalidEmbedding => Self::InvalidEmbedding,
            RoadFaceError::NestedFacesUnsupported => Self::NestedFacesUnsupported,
            RoadFaceError::OutputLimit => Self::OutputLimit,
        }
    }
}

impl From<PreflightError> for RoadFaceError {
    fn from(value: PreflightError) -> Self {
        match value {
            PreflightError::TooManyNodes => Self::TooManyNodes,
            PreflightError::TooManyEdges => Self::TooManyEdges,
            PreflightError::MissingNode => Self::MissingNode,
            PreflightError::DuplicateNode => Self::DuplicateNode,
            PreflightError::InvalidCoordinate => Self::InvalidCoordinate,
            PreflightError::CoincidentNodes => Self::CoincidentNodes,
            PreflightError::DuplicateEdge => Self::DuplicateEdge,
            PreflightError::InvalidEdge => Self::InvalidEdge,
            PreflightError::NonPlanar => Self::NonPlanar,
            PreflightError::WorkLimit => Self::WorkLimit,
            PreflightError::InvalidEmbedding => Self::InvalidEmbedding,
            PreflightError::NestedFacesUnsupported => Self::NestedFacesUnsupported,
            PreflightError::OutputLimit => Self::OutputLimit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Stage {
    Nodes,
    CoincidentNodes,
    Edges,
    Planarity,
    Done,
    Failed(PreflightError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum EdgeStage {
    SelfCheck,
    LookupFrom,
    LookupTo,
    DuplicateId,
    DuplicatePair,
    Geometry,
}

type NodeRecord = RoadFaceNode;
type EdgeRecord = RoadFaceEdge;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct GraphPreflightCheckpoint {
    schema: u8,
    nodes: Vec<NodeRecord>,
    edges: Vec<EdgeRecord>,
    proposed_edge: Option<EdgeRecord>,
    stage: Stage,
    node_cursor: usize,
    node_finite_checked: bool,
    node_unique_cursor: usize,
    coincident_left: usize,
    coincident_right: usize,
    edge_cursor: usize,
    edge_stage: EdgeStage,
    lookup_cursor: usize,
    from_index: Option<usize>,
    to_index: Option<usize>,
    unique_cursor: usize,
    indexed_edges: Vec<(usize, usize)>,
    pair_first: usize,
    pair_second: usize,
    pair_tests_done: usize,
    work_completed: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphPreflight {
    checkpoint: GraphPreflightCheckpoint,
}

impl GraphPreflight {
    pub(crate) fn new(
        nodes: Vec<RoadFaceNode>,
        mut edges: Vec<RoadFaceEdge>,
        proposed_edge: Option<RoadFaceEdge>,
    ) -> Result<Self, RoadFaceError> {
        let edge_count = edges.len() + usize::from(proposed_edge.is_some());
        if nodes.len() > MAX_NODES {
            return Err(RoadFaceError::TooManyNodes);
        }
        if edge_count > MAX_EDGES {
            return Err(RoadFaceError::TooManyEdges);
        }
        // Allocate the optional handoff slot during bounded setup, not in
        // advance when the validated graph is transferred to the next stage.
        if proposed_edge.is_some() {
            edges.reserve(1);
        }
        let empty = nodes.is_empty() && edge_count == 0;
        Ok(Self {
            checkpoint: GraphPreflightCheckpoint {
                schema: CHECKPOINT_SCHEMA,
                nodes,
                edges,
                proposed_edge,
                stage: if empty { Stage::Done } else { Stage::Nodes },
                node_cursor: 0,
                node_finite_checked: false,
                node_unique_cursor: 0,
                coincident_left: 0,
                coincident_right: 1,
                edge_cursor: 0,
                edge_stage: EdgeStage::SelfCheck,
                lookup_cursor: 0,
                from_index: None,
                to_index: None,
                unique_cursor: 0,
                indexed_edges: Vec::with_capacity(edge_count),
                pair_first: 0,
                pair_second: 1,
                pair_tests_done: 0,
                work_completed: 0,
            },
        })
    }

    pub(crate) fn checkpoint(&self) -> GraphPreflightCheckpoint {
        self.checkpoint.clone()
    }

    pub(crate) fn restore(mut checkpoint: GraphPreflightCheckpoint) -> Result<Self, RoadFaceError> {
        if let Err(error) = validate_checkpoint(&checkpoint) {
            return Err(error);
        }
        if checkpoint.proposed_edge.is_some() {
            checkpoint.edges.reserve(1);
        }
        Ok(Self { checkpoint })
    }

    pub(crate) fn result(&self) -> Option<Result<&[(usize, usize)], RoadFaceError>> {
        match self.checkpoint.stage {
            Stage::Done => Some(Ok(&self.checkpoint.indexed_edges)),
            Stage::Failed(error) => Some(Err(error.into())),
            _ => None,
        }
    }

    pub(crate) fn proposed_edge_id(&self) -> Option<u32> {
        self.checkpoint.proposed_edge.map(|edge| edge.id)
    }

    /// Consume a completed preflight and move its frozen inputs into the next
    /// stage without cloning the graph or its indexed-edge output.
    pub(crate) fn into_result(
        self,
    ) -> Result<(Vec<RoadFaceNode>, Vec<RoadFaceEdge>, Vec<(usize, usize)>), RoadFaceError> {
        let checkpoint = self.checkpoint;
        match checkpoint.stage {
            Stage::Failed(error) => Err(error.into()),
            Stage::Done => {
                let mut edges = checkpoint.edges;
                if let Some(proposed_edge) = checkpoint.proposed_edge {
                    edges.push(proposed_edge);
                }
                Ok((checkpoint.nodes, edges, checkpoint.indexed_edges))
            }
            _ => Err(RoadFaceError::InvalidEmbedding),
        }
    }

    pub(crate) fn advance(&mut self, budget: usize) -> PreflightAdvance {
        if budget == 0 || matches!(self.checkpoint.stage, Stage::Done | Stage::Failed(_)) {
            return PreflightAdvance {
                work_used: 0,
                done: matches!(self.checkpoint.stage, Stage::Done | Stage::Failed(_)),
            };
        }
        let mut work_used = 0;
        while work_used < budget && !matches!(self.checkpoint.stage, Stage::Done | Stage::Failed(_))
        {
            let consumed = match self.checkpoint.stage {
                Stage::Nodes => self.advance_nodes(),
                Stage::CoincidentNodes => self.advance_coincident_nodes(),
                Stage::Edges => self.advance_edges(),
                Stage::Planarity => self.advance_planarity(),
                Stage::Done | Stage::Failed(_) => false,
            };
            if consumed {
                work_used += 1;
                self.checkpoint.work_completed += 1;
            }
        }
        PreflightAdvance {
            work_used,
            done: matches!(self.checkpoint.stage, Stage::Done | Stage::Failed(_)),
        }
    }

    fn advance_nodes(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        if cp.node_cursor == cp.nodes.len() {
            cp.stage = Stage::CoincidentNodes;
            return false;
        }
        if !cp.node_finite_checked {
            let node = cp.nodes[cp.node_cursor];
            if !node.x.is_finite() || !node.z.is_finite() {
                cp.stage = Stage::Failed(PreflightError::InvalidCoordinate);
            } else {
                cp.node_finite_checked = true;
            }
            return true;
        }
        if cp.node_unique_cursor < cp.node_cursor {
            let duplicate = cp.nodes[cp.node_unique_cursor].id == cp.nodes[cp.node_cursor].id;
            cp.node_unique_cursor += 1;
            if duplicate {
                cp.stage = Stage::Failed(PreflightError::DuplicateNode);
            }
            return true;
        }
        cp.node_cursor += 1;
        cp.node_finite_checked = false;
        cp.node_unique_cursor = 0;
        false
    }

    fn advance_coincident_nodes(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        if cp.coincident_left + 1 >= cp.nodes.len() {
            cp.stage = Stage::Edges;
            return false;
        }
        let left = cp.nodes[cp.coincident_left];
        let right = cp.nodes[cp.coincident_right];
        let coincident = (left.x - right.x).hypot(left.z - right.z) <= COINCIDENT_EPSILON;
        if coincident {
            cp.stage = Stage::Failed(PreflightError::CoincidentNodes);
        } else {
            cp.coincident_right += 1;
            if cp.coincident_right == cp.nodes.len() {
                cp.coincident_left += 1;
                cp.coincident_right = cp.coincident_left + 1;
            }
        }
        true
    }

    fn advance_edges(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        let edge_count = cp.edges.len() + usize::from(cp.proposed_edge.is_some());
        if cp.edge_cursor == edge_count {
            let pair_count = edge_count.saturating_mul(edge_count.saturating_sub(1)) / 2;
            if pair_count > MAX_PAIR_TESTS {
                cp.stage = Stage::Failed(PreflightError::WorkLimit);
            } else if pair_count == 0 {
                cp.stage = Stage::Done;
            } else {
                cp.stage = Stage::Planarity;
            }
            return false;
        }
        let current = edge_at(cp, cp.edge_cursor);
        match cp.edge_stage {
            EdgeStage::SelfCheck => {
                if current.from == current.to {
                    cp.stage = Stage::Failed(PreflightError::InvalidEdge);
                } else {
                    cp.edge_stage = EdgeStage::LookupFrom;
                    cp.lookup_cursor = 0;
                }
                true
            }
            EdgeStage::LookupFrom => {
                if cp.lookup_cursor == cp.nodes.len() {
                    cp.stage = Stage::Failed(PreflightError::MissingNode);
                    return true;
                }
                let matched = cp.nodes[cp.lookup_cursor].id == current.from;
                let index = cp.lookup_cursor;
                cp.lookup_cursor += 1;
                if matched {
                    cp.from_index = Some(index);
                    cp.edge_stage = EdgeStage::LookupTo;
                    cp.lookup_cursor = 0;
                }
                true
            }
            EdgeStage::LookupTo => {
                if cp.lookup_cursor == cp.nodes.len() {
                    cp.stage = Stage::Failed(PreflightError::MissingNode);
                    return true;
                }
                let matched = cp.nodes[cp.lookup_cursor].id == current.to;
                let index = cp.lookup_cursor;
                cp.lookup_cursor += 1;
                if matched {
                    cp.to_index = Some(index);
                    cp.edge_stage = EdgeStage::DuplicateId;
                    cp.lookup_cursor = 0;
                    cp.unique_cursor = 0;
                }
                true
            }
            EdgeStage::DuplicateId => {
                if cp.unique_cursor == cp.edge_cursor {
                    cp.edge_stage = EdgeStage::DuplicatePair;
                    cp.unique_cursor = 0;
                    return false;
                }
                let duplicate = edge_at(cp, cp.unique_cursor).id == current.id;
                cp.unique_cursor += 1;
                if duplicate {
                    cp.stage = Stage::Failed(PreflightError::DuplicateEdge);
                }
                true
            }
            EdgeStage::DuplicatePair => {
                if cp.unique_cursor == cp.edge_cursor {
                    cp.edge_stage = EdgeStage::Geometry;
                    return false;
                }
                let prior = edge_at(cp, cp.unique_cursor);
                let duplicate = (prior.from.min(prior.to), prior.from.max(prior.to))
                    == (current.from.min(current.to), current.from.max(current.to));
                cp.unique_cursor += 1;
                if duplicate {
                    cp.stage = Stage::Failed(PreflightError::DuplicateEdge);
                }
                true
            }
            EdgeStage::Geometry => {
                let from = cp.from_index.expect("validated from endpoint");
                let to = cp.to_index.expect("validated to endpoint");
                let dx = cp.nodes[to].x - cp.nodes[from].x;
                let dz = cp.nodes[to].z - cp.nodes[from].z;
                if !dx.is_finite() || !dz.is_finite() || dx.hypot(dz) <= COINCIDENT_EPSILON {
                    cp.stage = Stage::Failed(PreflightError::InvalidEdge);
                } else {
                    cp.indexed_edges.push((from, to));
                    cp.edge_cursor += 1;
                    cp.edge_stage = EdgeStage::SelfCheck;
                    cp.lookup_cursor = 0;
                    cp.from_index = None;
                    cp.to_index = None;
                    cp.unique_cursor = 0;
                }
                true
            }
        }
    }

    fn advance_planarity(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        if cp.pair_first + 1 >= cp.indexed_edges.len() {
            cp.stage = Stage::Done;
            return false;
        }
        let (a, b) = cp.indexed_edges[cp.pair_first];
        let (c, d) = cp.indexed_edges[cp.pair_second];
        let shared = a == c || a == d || b == c || b == d;
        let nonplanar = if shared {
            let (shared, first_other, second_other) = if a == c {
                (a, b, d)
            } else if a == d {
                (a, b, c)
            } else if b == c {
                (b, a, d)
            } else {
                (b, a, c)
            };
            let first_dx = cp.nodes[first_other].x - cp.nodes[shared].x;
            let first_dz = cp.nodes[first_other].z - cp.nodes[shared].z;
            let second_dx = cp.nodes[second_other].x - cp.nodes[shared].x;
            let second_dz = cp.nodes[second_other].z - cp.nodes[shared].z;
            let cross = first_dx * second_dz - first_dz * second_dx;
            let dot = first_dx * second_dx + first_dz * second_dz;
            let scale = first_dx.hypot(first_dz) * second_dx.hypot(second_dz);
            cross.abs() <= COINCIDENT_EPSILON * scale && dot > COINCIDENT_EPSILON
        } else {
            let (a, b, c, d) = (cp.nodes[a], cp.nodes[b], cp.nodes[c], cp.nodes[d]);
            segments_distance_coords(a.x, a.z, b.x, b.z, c.x, c.z, d.x, d.z) <= COINCIDENT_EPSILON
        };
        cp.pair_tests_done += 1;
        if nonplanar {
            cp.stage = Stage::Failed(PreflightError::NonPlanar);
        } else {
            cp.pair_second += 1;
            if cp.pair_second == cp.indexed_edges.len() {
                cp.pair_first += 1;
                cp.pair_second = cp.pair_first + 1;
            }
        }
        true
    }
}

fn edge_at(cp: &GraphPreflightCheckpoint, index: usize) -> EdgeRecord {
    cp.edges.get(index).copied().unwrap_or_else(|| {
        cp.proposed_edge
            .expect("proposed edge exists at final index")
    })
}

fn validate_checkpoint(cp: &GraphPreflightCheckpoint) -> Result<(), RoadFaceError> {
    let invalid = || RoadFaceError::InvalidEdge;
    let edge_count = cp.edges.len() + usize::from(cp.proposed_edge.is_some());
    if cp.schema != CHECKPOINT_SCHEMA
        || cp.nodes.len() > MAX_NODES
        || edge_count > MAX_EDGES
        || cp.work_completed > MAX_PREFLIGHT_WORK
        || cp.edge_cursor > edge_count
        || cp.indexed_edges.len() > edge_count
        || cp.node_cursor > cp.nodes.len()
        || cp.node_unique_cursor > cp.node_cursor
        || cp.coincident_left > cp.nodes.len()
        || cp.coincident_right > cp.nodes.len()
        || cp.lookup_cursor > cp.nodes.len()
        || cp.unique_cursor > cp.edge_cursor
    {
        return Err(invalid());
    }
    match cp.stage {
        Stage::Nodes => {
            if cp.node_cursor >= cp.nodes.len()
                || (!cp.node_finite_checked && cp.node_unique_cursor != 0)
                || cp.edge_cursor != 0
                || !cp.indexed_edges.is_empty()
            {
                return Err(invalid());
            }
            for node in cp
                .nodes
                .iter()
                .take(cp.node_cursor + usize::from(cp.node_finite_checked))
            {
                if !node.x.is_finite() || !node.z.is_finite() {
                    return Err(invalid());
                }
            }
            for prior in 0..cp.node_unique_cursor {
                if cp.nodes[prior].id == cp.nodes[cp.node_cursor].id {
                    return Err(invalid());
                }
            }
        }
        Stage::CoincidentNodes => {
            if cp.node_cursor != cp.nodes.len()
                || cp.node_finite_checked
                || cp.node_unique_cursor != 0
                || (cp.nodes.len() >= 2
                    && !((cp.coincident_left + 1 < cp.nodes.len()
                        && cp.coincident_right > cp.coincident_left
                        && cp.coincident_right < cp.nodes.len())
                        || (cp.coincident_left + 1 == cp.nodes.len()
                            && cp.coincident_right == cp.nodes.len())))
                || cp.edge_cursor != 0
                || !cp.indexed_edges.is_empty()
            {
                return Err(invalid());
            }
            validate_completed_nodes(&cp.nodes)?;
        }
        Stage::Edges => {
            if cp.node_cursor != cp.nodes.len()
                || cp.indexed_edges.len() != cp.edge_cursor
                || (cp.from_index.is_some_and(|index| index >= cp.nodes.len()))
                || (cp.to_index.is_some_and(|index| index >= cp.nodes.len()))
            {
                return Err(invalid());
            }
            validate_completed_nodes(&cp.nodes)?;
            validate_indexed_edges(cp)?;
            if cp.edge_cursor == edge_count {
                if cp.edge_stage != EdgeStage::SelfCheck
                    || cp.lookup_cursor != 0
                    || cp.from_index.is_some()
                    || cp.to_index.is_some()
                    || cp.unique_cursor != 0
                {
                    return Err(invalid());
                }
                return Ok(());
            }
            let current = edge_at(cp, cp.edge_cursor);
            let active_cursor_valid = match cp.edge_stage {
                EdgeStage::SelfCheck => {
                    cp.lookup_cursor == 0
                        && cp.from_index.is_none()
                        && cp.to_index.is_none()
                        && cp.unique_cursor == 0
                }
                EdgeStage::LookupFrom => {
                    cp.from_index.is_none() && cp.to_index.is_none() && cp.unique_cursor == 0
                }
                EdgeStage::LookupTo => {
                    cp.from_index.is_some() && cp.to_index.is_none() && cp.unique_cursor == 0
                }
                EdgeStage::DuplicateId | EdgeStage::DuplicatePair => {
                    cp.from_index.is_some() && cp.to_index.is_some() && cp.lookup_cursor == 0
                }
                EdgeStage::Geometry => {
                    cp.from_index.is_some()
                        && cp.to_index.is_some()
                        && cp.lookup_cursor == 0
                        && cp.unique_cursor == cp.edge_cursor
                }
            };
            if !active_cursor_valid
                || cp
                    .from_index
                    .is_some_and(|index| cp.nodes[index].id != current.from)
                || cp
                    .to_index
                    .is_some_and(|index| cp.nodes[index].id != current.to)
            {
                return Err(invalid());
            }
        }
        Stage::Planarity => {
            let pairs = edge_count.saturating_mul(edge_count.saturating_sub(1)) / 2;
            if cp.edge_cursor != edge_count
                || cp.indexed_edges.len() != edge_count
                || pairs > MAX_PAIR_TESTS
            {
                return Err(invalid());
            }
            let finished_pairs =
                cp.pair_first == edge_count.saturating_sub(1) && cp.pair_second == edge_count;
            if finished_pairs {
                if cp.pair_tests_done != pairs {
                    return Err(invalid());
                }
            } else if cp.pair_first >= edge_count.saturating_sub(1)
                || cp.pair_second <= cp.pair_first
                || cp.pair_second >= edge_count
            {
                return Err(invalid());
            }
            let expected_done = pair_ordinal(cp.pair_first, cp.pair_second, edge_count);
            if !finished_pairs && cp.pair_tests_done != expected_done {
                return Err(invalid());
            }
            validate_completed_nodes(&cp.nodes)?;
            validate_indexed_edges(cp)?;
        }
        Stage::Done => {
            if cp.indexed_edges.len() != edge_count {
                return Err(invalid());
            }
            validate_completed_nodes(&cp.nodes)?;
            validate_indexed_edges(cp)?;
        }
        Stage::Failed(_) => {}
    }
    Ok(())
}

fn validate_completed_nodes(nodes: &[NodeRecord]) -> Result<(), RoadFaceError> {
    for (index, node) in nodes.iter().enumerate() {
        if !node.x.is_finite()
            || !node.z.is_finite()
            || nodes[..index].iter().any(|prior| prior.id == node.id)
        {
            return Err(RoadFaceError::InvalidEdge);
        }
    }
    Ok(())
}

fn validate_indexed_edges(cp: &GraphPreflightCheckpoint) -> Result<(), RoadFaceError> {
    for (index, &(from, to)) in cp.indexed_edges.iter().enumerate() {
        let edge = edge_at(cp, index);
        if from >= cp.nodes.len()
            || to >= cp.nodes.len()
            || cp.nodes[from].id != edge.from
            || cp.nodes[to].id != edge.to
        {
            return Err(RoadFaceError::InvalidEdge);
        }
    }
    Ok(())
}

fn pair_ordinal(first: usize, second: usize, edge_count: usize) -> usize {
    let preceding_rows =
        first * (edge_count - 1) - first.saturating_mul(first.saturating_sub(1)) / 2;
    preceding_rows + (second - first - 1)
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

    fn reference_error(
        nodes: &[RoadFaceNode],
        edges: &[RoadFaceEdge],
        proposed: Option<RoadFaceEdge>,
    ) -> RoadFaceError {
        crate::road_faces::trace_faces(nodes, edges, proposed).unwrap_err()
    }

    fn drain(
        mut job: GraphPreflight,
        quotas: &[usize],
    ) -> (Result<Vec<(usize, usize)>, RoadFaceError>, usize) {
        let mut work = 0;
        let mut turn = 0;
        while job.result().is_none() {
            let quota = quotas[turn % quotas.len()];
            let advanced = job.advance(quota);
            assert!(advanced.work_used <= quota);
            work += advanced.work_used;
            let json = serde_json::to_vec(&job.checkpoint()).unwrap();
            let cp: GraphPreflightCheckpoint = serde_json::from_slice(&json).unwrap();
            job = GraphPreflight::restore(cp).unwrap();
            turn += 1;
            assert!(turn < 2_000_000);
        }
        (job.result().unwrap().map(<[_]>::to_vec), work)
    }

    #[test]
    fn rectangle_and_shared_endpoint_pairs_pass_preflight() {
        let nodes = vec![
            node(10, 0.0, 0.0),
            node(20, 8.0, 0.0),
            node(30, 8.0, 5.0),
            node(40, 0.0, 5.0),
        ];
        let edges = vec![
            edge(1, 10, 20),
            edge(2, 20, 30),
            edge(3, 30, 40),
            edge(4, 40, 10),
        ];
        let expected = vec![(0, 1), (1, 2), (2, 3), (3, 0)];
        for quota in [1, 7, 48] {
            let (actual, _) = drain(
                GraphPreflight::new(nodes.clone(), edges.clone(), None).unwrap(),
                &[quota],
            );
            assert_eq!(actual.unwrap(), expected);
        }
        let collinear_nodes = vec![node(10, 0.0, 0.0), node(20, 8.0, 0.0), node(30, 16.0, 0.0)];
        let collinear = vec![edge(1, 10, 20), edge(2, 10, 30)];
        assert_eq!(
            drain(
                GraphPreflight::new(collinear_nodes.clone(), collinear.clone(), None).unwrap(),
                &[3]
            )
            .0,
            Err(reference_error(&collinear_nodes, &collinear, None))
        );
    }

    #[test]
    fn malformed_inputs_preserve_preflight_errors() {
        let simple_nodes = vec![node(1, 0.0, 0.0), node(2, 4.0, 0.0), node(3, 0.0, 4.0)];
        for edges in [
            vec![edge(1, 1, 9)],
            vec![edge(1, 1, 2), edge(1, 2, 3)],
            vec![edge(1, 1, 2), edge(2, 2, 1)],
            vec![edge(1, 1, 1)],
        ] {
            assert_eq!(
                drain(
                    GraphPreflight::new(simple_nodes.clone(), edges.clone(), None).unwrap(),
                    &[7]
                )
                .0,
                Err(reference_error(&simple_nodes, &edges, None))
            );
        }
        let coincident = vec![node(1, 0.0, 0.0), node(2, 0.0, 0.0)];
        assert_eq!(
            drain(
                GraphPreflight::new(coincident.clone(), vec![], None).unwrap(),
                &[1]
            )
            .0,
            Err(reference_error(&coincident, &[], None))
        );
        let nonfinite = vec![node(1, f32::NAN, 0.0)];
        let mut job = GraphPreflight::new(nonfinite.clone(), vec![], None).unwrap();
        while job.result().is_none() {
            job.advance(1);
        }
        assert_eq!(
            job.result().unwrap().unwrap_err(),
            reference_error(&nonfinite, &[], None)
        );
    }

    #[test]
    fn crossing_and_caps_fail_closed() {
        let nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 4.0, 0.0),
            node(3, 4.0, 4.0),
            node(4, 0.0, 4.0),
        ];
        let crossing_edges = vec![edge(1, 1, 3), edge(2, 2, 4)];
        assert_eq!(
            drain(
                GraphPreflight::new(nodes.clone(), crossing_edges.clone(), None).unwrap(),
                &[1]
            )
            .0,
            Err(reference_error(&nodes, &crossing_edges, None))
        );
        let too_many_nodes = (0..=MAX_NODES)
            .map(|i| node(i as u32, i as f32, 0.0))
            .collect();
        assert_eq!(
            GraphPreflight::new(too_many_nodes, vec![], None).unwrap_err(),
            RoadFaceError::TooManyNodes
        );
        let many_nodes = (0..MAX_NODES)
            .map(|i| node(i as u32, i as f32 * 2.0, 0.0))
            .collect();
        let many_edges = (0..=MAX_EDGES).map(|i| edge(i as u32, 0, 1)).collect();
        assert_eq!(
            GraphPreflight::new(many_nodes, many_edges, None).unwrap_err(),
            RoadFaceError::TooManyEdges
        );
    }

    #[test]
    fn every_quota_boundary_json_resume_matches_uninterrupted_work() {
        let nodes = vec![
            node(1, 0.0, 0.0),
            node(2, 10.0, 0.0),
            node(3, 10.0, 10.0),
            node(4, 0.0, 10.0),
            node(5, 20.0, 0.0),
        ];
        let edges = vec![
            edge(1, 1, 2),
            edge(2, 2, 3),
            edge(3, 3, 4),
            edge(4, 4, 1),
            edge(5, 2, 5),
        ];
        let mut baseline = GraphPreflight::new(nodes.clone(), edges.clone(), None).unwrap();
        while baseline.result().is_none() {
            baseline.advance(48);
        }
        let expected = baseline.result().unwrap().map(<[_]>::to_vec);
        let expected_work = baseline.checkpoint.work_completed;
        for quota in [1, 7, 48] {
            let (actual, work) = drain(
                GraphPreflight::new(nodes.clone(), edges.clone(), None).unwrap(),
                &[quota],
            );
            assert_eq!(actual, expected);
            assert_eq!(work, expected_work);
        }
        let (actual, work) = drain(
            GraphPreflight::new(nodes, edges, None).unwrap(),
            &[3, 11, 2, 48],
        );
        assert_eq!(actual, expected);
        assert_eq!(work, expected_work);
    }

    #[test]
    fn restore_rejects_out_of_range_and_forged_cursors() {
        let nodes = vec![node(1, 0.0, 0.0), node(2, 10.0, 0.0), node(3, 10.0, 10.0)];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3)];
        let mut job = GraphPreflight::new(nodes, edges, None).unwrap();
        job.advance(5);
        let mut checkpoint = job.checkpoint();
        checkpoint.node_cursor = usize::MAX;
        assert_eq!(
            GraphPreflight::restore(checkpoint).unwrap_err(),
            RoadFaceError::InvalidEdge
        );

        let mut job = GraphPreflight::new(
            vec![node(1, 0.0, 0.0), node(2, 10.0, 0.0), node(3, 10.0, 10.0)],
            vec![edge(1, 1, 2), edge(2, 2, 3), edge(3, 3, 1)],
            None,
        )
        .unwrap();
        for _ in 0..1_000 {
            if job.checkpoint.stage == Stage::Planarity {
                break;
            }
            assert!(
                job.result().is_none(),
                "preflight completed before planarity"
            );
            job.advance(1);
        }
        assert_eq!(job.checkpoint.stage, Stage::Planarity);
        let mut checkpoint = job.checkpoint();
        checkpoint.pair_second = usize::MAX;
        assert_eq!(
            GraphPreflight::restore(checkpoint).unwrap_err(),
            RoadFaceError::InvalidEdge
        );
    }
    #[test]
    fn proposed_edge_handoff_reuses_buffer_after_json_restore() {
        let nodes = vec![
            RoadFaceNode {
                id: 1,
                x: 0.0,
                z: 0.0,
            },
            RoadFaceNode {
                id: 2,
                x: 10.0,
                z: 0.0,
            },
            RoadFaceNode {
                id: 3,
                x: 10.0,
                z: 10.0,
            },
        ];
        let edges = vec![RoadFaceEdge {
            id: 1,
            from: 1,
            to: 2,
        }];
        let mut job = GraphPreflight::new(
            nodes,
            edges,
            Some(RoadFaceEdge {
                id: 2,
                from: 2,
                to: 3,
            }),
        )
        .unwrap();
        while !job.advance(7).done {}
        let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
        let restored = GraphPreflight::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
        assert!(restored.checkpoint.edges.capacity() > restored.checkpoint.edges.len());
        let pointer = restored.checkpoint.edges.as_ptr();
        let (_, edges, indexed) = restored.into_result().unwrap();
        assert_eq!(edges.as_ptr(), pointer);
        assert_eq!(edges.len(), 2);
        assert_eq!(indexed.len(), 2);
    }
}
