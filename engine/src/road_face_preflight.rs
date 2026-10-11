//! Resumable validation and planarity preflight for road-face tracing.
//!
//! This stage deliberately stops before bridge detection and half-edge face
//! tracing. It preserves the validation and pair-test order of
//! `road_faces::trace_faces` while exposing every scan to a deterministic
//! work budget.

use crate::geometry::segments_distance_coords;
use crate::node_coincidence_index::NodeCoincidenceIndex;
use crate::road_faces::{RoadFaceEdge, RoadFaceError, RoadFaceNode};
use crate::road_pair_broadphase::{Bounds, BroadPhase, Checkpoint as BroadPhaseCheckpoint};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const CHECKPOINT_SCHEMA: u8 = 3;
const MAX_NODES: usize = 512;
const MAX_EDGES: usize = 512;
const MAX_PAIR_TESTS: usize = 131_072;
const COINCIDENT_EPSILON: f32 = 1.0e-5;
const MAX_PREFLIGHT_WORK: usize = 1_700_000;

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
    FilterBuild,
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
    #[serde(default)]
    filter_work: usize,
    #[serde(default)]
    filter_build_cursor: usize,
    #[serde(default)]
    filter_fallback: bool,
    #[serde(default)]
    filter_checkpoint: Option<BroadPhaseCheckpoint>,
    #[serde(default)]
    pending_pair: Option<(usize, usize)>,
    #[serde(default)]
    last_checked_pair: Option<(usize, usize)>,
    work_completed: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphPreflight {
    checkpoint: GraphPreflightCheckpoint,
    filter: Option<BroadPhase>,
    node_ids: BTreeMap<u32, usize>,
    node_cells: NodeCoincidenceIndex,
    edge_ids: BTreeSet<u32>,
    edge_pairs: BTreeSet<(u32, u32)>,
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
        let no_nodes = nodes.is_empty();
        let empty = no_nodes && edge_count == 0;
        let filter = BroadPhase::new(Vec::new()).ok();
        Ok(Self {
            checkpoint: GraphPreflightCheckpoint {
                schema: CHECKPOINT_SCHEMA,
                nodes,
                edges,
                proposed_edge,
                stage: if empty {
                    Stage::Done
                } else if no_nodes {
                    Stage::Edges
                } else {
                    Stage::Nodes
                },
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
                filter_work: 0,
                filter_build_cursor: 0,
                filter_fallback: false,
                filter_checkpoint: None,
                pending_pair: None,
                last_checked_pair: None,
                work_completed: 0,
            },
            filter,
            node_ids: BTreeMap::new(),
            node_cells: NodeCoincidenceIndex::default(),
            edge_ids: BTreeSet::new(),
            edge_pairs: BTreeSet::new(),
        })
    }

    pub(crate) fn checkpoint(&self) -> GraphPreflightCheckpoint {
        let mut checkpoint = self.checkpoint.clone();
        checkpoint.filter_checkpoint = self.filter.as_ref().map(BroadPhase::checkpoint);
        checkpoint
    }

    pub(crate) fn restore(mut checkpoint: GraphPreflightCheckpoint) -> Result<Self, RoadFaceError> {
        if checkpoint.schema == 1 {
            // Schema 1 predates the spatial index. Preserve its frozen graph,
            // but restart derived validation work instead of interpreting old
            // pair cursors as spatial-index progress. Admission still uses the
            // surrounding live job's fresh geometry validation.
            if checkpoint.filter_checkpoint.is_some()
                || checkpoint.filter_work != 0
                || checkpoint.filter_build_cursor != 0
                || checkpoint.filter_fallback
                || checkpoint.pending_pair.is_some()
                || checkpoint.last_checked_pair.is_some()
                || checkpoint.stage == Stage::FilterBuild
            {
                return Err(RoadFaceError::InvalidEdge);
            }
            return Self::new(checkpoint.nodes, checkpoint.edges, checkpoint.proposed_edge);
        }
        if checkpoint.schema == 2 {
            return Self::new(checkpoint.nodes, checkpoint.edges, checkpoint.proposed_edge);
        }
        if let Err(error) = validate_checkpoint(&checkpoint) {
            return Err(error);
        }
        if checkpoint.proposed_edge.is_some() {
            checkpoint.edges.reserve(1);
        }
        let filter = checkpoint
            .filter_checkpoint
            .take()
            .map(BroadPhase::restore)
            .transpose()
            .map_err(|_| RoadFaceError::InvalidEdge)?;
        let (node_ids, edge_ids, edge_pairs, node_cells) = rebuild_indexes(&checkpoint)?;
        Ok(Self {
            checkpoint,
            filter,
            node_ids,
            node_cells,
            edge_ids,
            edge_pairs,
        })
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
        if self.checkpoint.work_completed >= MAX_PREFLIGHT_WORK {
            self.checkpoint.stage = Stage::Failed(PreflightError::WorkLimit);
            return PreflightAdvance {
                work_used: 0,
                done: true,
            };
        }
        let budget = budget.min(MAX_PREFLIGHT_WORK - self.checkpoint.work_completed);
        while work_used < budget && !matches!(self.checkpoint.stage, Stage::Done | Stage::Failed(_))
        {
            if self.checkpoint.stage == Stage::Planarity {
                let consumed = self.advance_planarity(budget - work_used);
                work_used += consumed;
                self.checkpoint.work_completed += consumed;
                continue;
            }
            let consumed = match self.checkpoint.stage {
                Stage::Nodes => self.advance_nodes(),
                Stage::CoincidentNodes => self.advance_coincident_nodes(),
                Stage::Edges => self.advance_edges(),
                Stage::FilterBuild => self.advance_filter_build(),
                Stage::Planarity => false,
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
        if cp.node_unique_cursor == 0 {
            let id = cp.nodes[cp.node_cursor].id;
            let duplicate = self.node_ids.contains_key(&id);
            cp.node_unique_cursor = 1;
            if duplicate {
                cp.stage = Stage::Failed(PreflightError::DuplicateNode);
            } else {
                self.node_ids.insert(id, cp.node_cursor);
                let node = cp.nodes[cp.node_cursor];
                self.node_cells.insert(cp.node_cursor, node.x, node.z);
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
        // Fixed nine-cell query: advance source-index cursors exactly as the
        // old scan did, skipping only pairs outside the conservative cells.
        match self
            .node_cells
            .next_candidate(left.x, left.z, cp.coincident_right)
        {
            Ok(Some(index)) => cp.coincident_right = index,
            Ok(None) => {
                cp.coincident_left += 1;
                cp.coincident_right = cp.coincident_left + 1;
                return true;
            }
            Err(()) => {} // Unsupported coordinates retain the exact scan.
        }
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
                cp.stage = Stage::FilterBuild;
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
                if let Some(&index) = self.node_ids.get(&current.from) {
                    cp.lookup_cursor = 0;
                    cp.from_index = Some(index);
                    cp.edge_stage = EdgeStage::LookupTo;
                } else {
                    cp.lookup_cursor = 0;
                    cp.stage = Stage::Failed(PreflightError::MissingNode);
                }
                true
            }
            EdgeStage::LookupTo => {
                if let Some(&index) = self.node_ids.get(&current.to) {
                    cp.lookup_cursor = 0;
                    cp.to_index = Some(index);
                    cp.edge_stage = EdgeStage::DuplicateId;
                    cp.unique_cursor = 0;
                } else {
                    cp.lookup_cursor = 0;
                    cp.stage = Stage::Failed(PreflightError::MissingNode);
                }
                true
            }
            EdgeStage::DuplicateId => {
                let duplicate = self.edge_ids.contains(&current.id);
                cp.unique_cursor = cp.edge_cursor;
                if duplicate {
                    cp.stage = Stage::Failed(PreflightError::DuplicateEdge);
                } else {
                    cp.edge_stage = EdgeStage::DuplicatePair;
                    cp.unique_cursor = 0;
                }
                true
            }
            EdgeStage::DuplicatePair => {
                let pair = (current.from.min(current.to), current.from.max(current.to));
                let duplicate = self.edge_pairs.contains(&pair);
                cp.unique_cursor = cp.edge_cursor;
                if duplicate {
                    cp.stage = Stage::Failed(PreflightError::DuplicateEdge);
                } else {
                    cp.edge_stage = EdgeStage::Geometry;
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
                    self.edge_ids.insert(current.id);
                    self.edge_pairs
                        .insert((current.from.min(current.to), current.from.max(current.to)));
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

    fn advance_filter_build(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        if cp.filter_fallback || self.filter.is_none() {
            cp.filter_fallback = true;
            cp.stage = Stage::Planarity;
            return false;
        }
        if cp.filter_build_cursor == cp.indexed_edges.len() {
            cp.stage = Stage::Planarity;
            return false;
        }
        let (from, to) = cp.indexed_edges[cp.filter_build_cursor];
        let a = cp.nodes[from];
        let b = cp.nodes[to];
        let bounds = Bounds::from_segment_f32(a.x, a.z, b.x, b.z);
        cp.filter_work += 1;
        match bounds {
            Ok(bounds) => {
                if self.filter.as_mut().unwrap().push_bounds(bounds).is_err() {
                    cp.filter_fallback = true;
                    self.filter = None;
                } else {
                    cp.filter_build_cursor += 1;
                }
            }
            Err(_) => {
                cp.filter_fallback = true;
                self.filter = None;
            }
        }
        true
    }

    fn advance_planarity(&mut self, budget: usize) -> usize {
        let cp = &mut self.checkpoint;
        if budget == 0 {
            return 0;
        }
        if cp.pending_pair.is_none() {
            if cp.filter_fallback {
                if cp.pair_first + 1 >= cp.indexed_edges.len() {
                    cp.stage = Stage::Done;
                    return 0;
                }
                cp.pending_pair = Some((cp.pair_first, cp.pair_second));
            } else {
                let limit = pair_limit(cp.indexed_edges.len());
                if cp.filter_work.saturating_sub(cp.filter_build_cursor) >= limit {
                    cp.filter_fallback = true;
                    self.filter = None;
                    cp.pair_first = 0;
                    cp.pair_second = 1;
                    cp.pair_tests_done = 0;
                    return 0;
                }
                let step = self.filter.as_mut().unwrap().advance(
                    budget.min(limit - cp.filter_work.saturating_sub(cp.filter_build_cursor)),
                );
                cp.filter_work = cp.filter_work.saturating_add(step.work_used);
                if let Some(pair) = step.pair {
                    cp.pending_pair = Some(pair);
                } else if step.done {
                    cp.stage = Stage::Done;
                    return step.work_used;
                }
                if cp.pending_pair.is_none() {
                    if cp.filter_work.saturating_sub(cp.filter_build_cursor) >= limit {
                        cp.filter_fallback = true;
                        self.filter = None;
                        cp.pair_first = 0;
                        cp.pair_second = 1;
                        cp.pair_tests_done = 0;
                        cp.pending_pair = None;
                        return step.work_used;
                    }
                    return step.work_used;
                }
                if cp.filter_work.saturating_sub(cp.filter_build_cursor) >= limit {
                    cp.filter_fallback = true;
                    self.filter = None;
                    cp.pair_first = 0;
                    cp.pair_second = 1;
                    cp.pair_tests_done = 0;
                    cp.pending_pair = None;
                    return step.work_used;
                }
                // Candidate discovery has its own charge. Defer its exact
                // test if this call exhausted the caller's quota.
                if step.work_used >= budget {
                    return step.work_used;
                }
                let exact = self.test_pending_pair();
                return step.work_used + exact;
            }
        }
        self.test_pending_pair()
    }

    fn test_pending_pair(&mut self) -> usize {
        let cp = &mut self.checkpoint;
        let (first, second) = cp.pending_pair.take().unwrap();
        cp.last_checked_pair = Some((first, second));
        let (a, b) = cp.indexed_edges[first];
        let (c, d) = cp.indexed_edges[second];
        if pair_nonplanar(&cp.nodes, a, b, c, d) {
            cp.stage = Stage::Failed(PreflightError::NonPlanar);
        } else if cp.filter_fallback {
            cp.pair_second += 1;
            if cp.pair_second == cp.indexed_edges.len() {
                cp.pair_first += 1;
                cp.pair_second = cp.pair_first + 1;
            }
        }
        cp.pair_tests_done += 1;
        1
    }
}

// Shared by the live validator and broad-phase parity tests so the oracle
// includes the actual f32 arithmetic and shared-node collinearity rule.
fn pair_nonplanar(nodes: &[RoadFaceNode], a: usize, b: usize, c: usize, d: usize) -> bool {
    let shared = a == c || a == d || b == c || b == d;
    if shared {
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
        cross.abs() <= COINCIDENT_EPSILON * scale && dot > COINCIDENT_EPSILON
    } else {
        let (a, b, c, d) = (nodes[a], nodes[b], nodes[c], nodes[d]);
        segments_distance_coords(a.x, a.z, b.x, b.z, c.x, c.z, d.x, d.z) <= COINCIDENT_EPSILON
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
        || cp.filter_work > MAX_PAIR_TESTS + MAX_EDGES
        || cp.edge_cursor > edge_count
        || cp.indexed_edges.len() > edge_count
        || cp.node_cursor > cp.nodes.len()
        || cp.node_unique_cursor > 1
        || cp.coincident_left > cp.nodes.len()
        || cp.coincident_right > cp.nodes.len().max(1)
        || cp.lookup_cursor > cp.nodes.len()
        || cp.unique_cursor > cp.edge_cursor
    {
        return Err(invalid());
    }
    validate_filter_state(cp)?;
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
            if cp.node_unique_cursor == 1
                && cp.node_cursor < cp.nodes.len()
                && cp.nodes[..cp.node_cursor]
                    .iter()
                    .any(|prior| prior.id == cp.nodes[cp.node_cursor].id)
            {
                return Err(invalid());
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
                    cp.from_index.is_none()
                        && cp.to_index.is_none()
                        && cp.lookup_cursor == 0
                        && cp.unique_cursor == 0
                }
                EdgeStage::LookupTo => {
                    cp.from_index.is_some()
                        && cp.to_index.is_none()
                        && cp.lookup_cursor == 0
                        && cp.unique_cursor == 0
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
        Stage::FilterBuild => {
            if cp.edge_cursor != edge_count || cp.indexed_edges.len() != edge_count {
                return Err(invalid());
            }
            validate_completed_nodes(&cp.nodes)?;
            validate_indexed_edges(cp)?;
        }
        Stage::Planarity => {
            let pairs = edge_count.saturating_mul(edge_count.saturating_sub(1)) / 2;
            if cp.edge_cursor != edge_count
                || cp.indexed_edges.len() != edge_count
                || pairs > MAX_PAIR_TESTS
            {
                return Err(invalid());
            }
            if cp.filter_fallback {
                let finished =
                    cp.pair_first == edge_count.saturating_sub(1) && cp.pair_second == edge_count;
                if (!finished
                    && (cp.pair_first >= edge_count.saturating_sub(1)
                        || cp.pair_second <= cp.pair_first
                        || cp.pair_second >= edge_count))
                    || cp.pair_tests_done
                        != if finished {
                            pairs
                        } else {
                            pair_ordinal(cp.pair_first, cp.pair_second, edge_count)
                        }
                    || cp
                        .pending_pair
                        .is_some_and(|pair| pair != (cp.pair_first, cp.pair_second))
                {
                    return Err(invalid());
                }
            } else if cp.pair_first != 0
                || cp.pair_second != 1
                || cp.pair_tests_done > pairs
                || cp.filter_build_cursor != cp.indexed_edges.len()
                || cp.filter_work.saturating_sub(cp.filter_build_cursor) > pairs
                || cp.pending_pair.is_some_and(|(i, j)| {
                    i >= edge_count.saturating_sub(1)
                        || j <= i
                        || j >= edge_count
                        || !bounds_overlap(cp, i, j)
                })
            {
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
        Stage::Failed(_) => {
            // Failed checkpoints retain the exact failure cursor. Validate all
            // completed prefixes before rebuilding their transient indexes.
            if cp.indexed_edges.len() != cp.edge_cursor || cp.node_cursor > cp.nodes.len() {
                return Err(invalid());
            }
            match cp.stage {
                Stage::Failed(
                    PreflightError::InvalidCoordinate | PreflightError::DuplicateNode,
                ) => {
                    if cp.node_cursor >= cp.nodes.len() || cp.edge_cursor != 0 {
                        return Err(invalid());
                    }
                    if cp.stage == Stage::Failed(PreflightError::DuplicateNode)
                        && (!cp.nodes[cp.node_cursor].x.is_finite()
                            || !cp.nodes[cp.node_cursor].z.is_finite()
                            || cp.node_unique_cursor != 1)
                    {
                        return Err(invalid());
                    }
                    if cp.stage == Stage::Failed(PreflightError::DuplicateNode)
                        && !cp.nodes[..cp.node_cursor]
                            .iter()
                            .any(|prior| prior.id == cp.nodes[cp.node_cursor].id)
                    {
                        return Err(invalid());
                    }
                }
                Stage::Failed(PreflightError::CoincidentNodes) => {
                    validate_completed_nodes(&cp.nodes)?;
                    if cp.coincident_left + 1 >= cp.nodes.len()
                        || cp.coincident_right <= cp.coincident_left
                        || cp.coincident_right >= cp.nodes.len()
                        || (cp.nodes[cp.coincident_left].x - cp.nodes[cp.coincident_right].x)
                            .hypot(cp.nodes[cp.coincident_left].z - cp.nodes[cp.coincident_right].z)
                            > COINCIDENT_EPSILON
                    {
                        return Err(invalid());
                    }
                }
                _ => {}
            }
            if cp.stage == Stage::Failed(PreflightError::InvalidCoordinate)
                && cp
                    .nodes
                    .get(cp.node_cursor)
                    .is_none_or(|node| node.x.is_finite() && node.z.is_finite())
            {
                return Err(invalid());
            }
            if matches!(
                cp.stage,
                Stage::Failed(
                    PreflightError::MissingNode
                        | PreflightError::DuplicateEdge
                        | PreflightError::InvalidEdge
                        | PreflightError::NonPlanar
                )
            ) {
                validate_completed_nodes(&cp.nodes)?;
                validate_indexed_edges(cp)?;
            }
            if cp.stage == Stage::Failed(PreflightError::DuplicateEdge)
                && cp.edge_cursor >= edge_count
            {
                return Err(invalid());
            }
            if cp.stage == Stage::Failed(PreflightError::MissingNode)
                && cp.edge_cursor >= edge_count
            {
                return Err(invalid());
            }
            if cp.stage == Stage::Failed(PreflightError::NonPlanar) && cp.edge_cursor != edge_count
            {
                return Err(invalid());
            }
            if cp.stage == Stage::Failed(PreflightError::InvalidEdge)
                && cp.edge_cursor >= edge_count
            {
                return Err(invalid());
            }
            if matches!(
                cp.stage,
                Stage::Failed(PreflightError::TooManyNodes)
                    | Stage::Failed(PreflightError::TooManyEdges)
                    | Stage::Failed(PreflightError::InvalidEmbedding)
                    | Stage::Failed(PreflightError::NestedFacesUnsupported)
                    | Stage::Failed(PreflightError::OutputLimit)
            ) {
                // These errors are not produced by the resumable graph scan.
                return Err(invalid());
            }
            if cp.stage == Stage::Failed(PreflightError::WorkLimit) {
                validate_completed_node_prefix(cp)?;
                validate_indexed_edges(cp)?;
            }
        }
    }
    Ok(())
}

fn validate_filter_state(cp: &GraphPreflightCheckpoint) -> Result<(), RoadFaceError> {
    let invalid = || RoadFaceError::InvalidEdge;
    let early = matches!(
        cp.stage,
        Stage::Nodes | Stage::CoincidentNodes | Stage::Edges
    );
    if early {
        if cp.filter_fallback
            || cp.filter_build_cursor != 0
            || cp.filter_work != 0
            || cp.pair_tests_done != 0
            || cp.pending_pair.is_some()
            || cp.last_checked_pair.is_some()
            || cp.pair_first != 0
            || cp.pair_second != 1
        {
            return Err(invalid());
        }
    }
    if cp.filter_fallback {
        if cp.filter_checkpoint.is_some() || cp.filter_build_cursor > cp.indexed_edges.len() {
            return Err(invalid());
        }
        if cp.stage == Stage::Done && cp.indexed_edges.len() >= 2 {
            let edge_count = cp.indexed_edges.len();
            if cp.pair_first != edge_count - 1
                || cp.pair_second != edge_count
                || cp.pair_tests_done != pair_limit(edge_count)
                || cp.pending_pair.is_some()
            {
                return Err(invalid());
            }
        }
        return Ok(());
    }
    let saved = cp.filter_checkpoint.clone().ok_or_else(invalid)?;
    let filter = BroadPhase::restore(saved).map_err(|_| RoadFaceError::InvalidEdge)?;
    if cp.filter_build_cursor != filter.input_bounds().len()
        || cp.filter_build_cursor > cp.indexed_edges.len()
    {
        return Err(invalid());
    }
    if early || cp.stage == Stage::FilterBuild {
        let pristine = BroadPhase::new(filter.input_bounds().to_vec())
            .map_err(|_| RoadFaceError::InvalidEdge)?;
        if filter != pristine || cp.pending_pair.is_some() || cp.last_checked_pair.is_some() {
            return Err(invalid());
        }
    }
    if cp.stage == Stage::Planarity || cp.stage == Stage::Done {
        if cp.indexed_edges.len() >= 2 && cp.filter_build_cursor != cp.indexed_edges.len() {
            return Err(invalid());
        }
        if cp.stage == Stage::Done && cp.indexed_edges.len() >= 2 {
            let mut complete = filter.clone();
            if !complete.advance(0).done {
                return Err(invalid());
            }
        }
        if let Some(emitted) = filter.last_emitted_pair() {
            if Some(emitted) != cp.pending_pair.or(cp.last_checked_pair) {
                return Err(invalid());
            }
        } else if cp.pending_pair.is_some() {
            return Err(invalid());
        }
        if cp.pending_pair.is_some() && cp.pending_pair == cp.last_checked_pair {
            return Err(invalid());
        }
    }
    if cp
        .pending_pair
        .is_some_and(|pair| filter.last_emitted_pair() != Some(pair))
    {
        return Err(invalid());
    }
    for (index, &(from, to)) in cp
        .indexed_edges
        .iter()
        .take(cp.filter_build_cursor)
        .enumerate()
    {
        let a = *cp.nodes.get(from).ok_or_else(invalid)?;
        let b = *cp.nodes.get(to).ok_or_else(invalid)?;
        let expected =
            Bounds::from_segment_f32(a.x, a.z, b.x, b.z).map_err(|_| RoadFaceError::InvalidEdge)?;
        if filter.input_bounds()[index] != expected {
            return Err(invalid());
        }
    }
    Ok(())
}

fn bounds_overlap(cp: &GraphPreflightCheckpoint, i: usize, j: usize) -> bool {
    let segment_bounds = |index: usize| {
        let (a, b) = *cp.indexed_edges.get(index)?;
        let a = cp.nodes.get(a)?;
        let b = cp.nodes.get(b)?;
        Bounds::from_segment_f32(a.x, a.z, b.x, b.z).ok()
    };
    match (segment_bounds(i), segment_bounds(j)) {
        (Some(a), Some(b)) => {
            a.min_x <= b.max_x + 1e-5
                && b.min_x <= a.max_x + 1e-5
                && a.min_z <= b.max_z + 1e-5
                && b.min_z <= a.max_z + 1e-5
        }
        _ => false,
    }
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

fn rebuild_indexes(
    cp: &GraphPreflightCheckpoint,
) -> Result<
    (
        BTreeMap<u32, usize>,
        BTreeSet<u32>,
        BTreeSet<(u32, u32)>,
        NodeCoincidenceIndex,
    ),
    RoadFaceError,
> {
    let mut node_ids = BTreeMap::new();
    let mut node_cells = NodeCoincidenceIndex::default();
    let node_prefix = match cp.stage {
        Stage::Nodes => {
            cp.node_cursor + usize::from(cp.node_finite_checked && cp.node_unique_cursor == 1)
        }
        Stage::Failed(PreflightError::InvalidCoordinate | PreflightError::DuplicateNode) => {
            cp.node_cursor
        }
        Stage::Failed(PreflightError::WorkLimit) => {
            cp.node_cursor + usize::from(cp.node_finite_checked && cp.node_unique_cursor == 1)
        }
        _ => cp.nodes.len(),
    };
    for (index, node) in cp.nodes.iter().take(node_prefix).enumerate() {
        if !node.x.is_finite() || !node.z.is_finite() || node_ids.insert(node.id, index).is_some() {
            return Err(RoadFaceError::InvalidEdge);
        }
        node_cells.insert(index, node.x, node.z);
    }
    let mut edge_ids = BTreeSet::new();
    let mut edge_pairs = BTreeSet::new();
    for index in 0..cp.edge_cursor {
        let edge = edge_at(cp, index);
        if !edge_ids.insert(edge.id)
            || !edge_pairs.insert((edge.from.min(edge.to), edge.from.max(edge.to)))
        {
            return Err(RoadFaceError::InvalidEdge);
        }
    }
    Ok((node_ids, edge_ids, edge_pairs, node_cells))
}

fn validate_completed_node_prefix(cp: &GraphPreflightCheckpoint) -> Result<(), RoadFaceError> {
    let prefix_len = match cp.stage {
        Stage::Failed(PreflightError::WorkLimit) => {
            cp.node_cursor + usize::from(cp.node_finite_checked && cp.node_unique_cursor == 1)
        }
        _ => cp.nodes.len(),
    };
    let mut ids = BTreeSet::new();
    for node in cp.nodes.iter().take(prefix_len) {
        if !node.x.is_finite() || !node.z.is_finite() || !ids.insert(node.id) {
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

fn pair_limit(edge_count: usize) -> usize {
    edge_count.saturating_mul(edge_count.saturating_sub(1)) / 2
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
    fn spatial_candidates_preserve_actual_f32_planarity_rejections() {
        use crate::road_pair_broadphase::{Bounds, BroadPhase};
        let mut seed = 0x91e2_u64;
        let mut random = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) as u32 as f64 / u32::MAX as f64 * 200_000.0 - 100_000.0) as f32
        };
        for case in 0..24 {
            let mut nodes = vec![
                node(0, -100_000.0, 0.0),
                node(1, -0.001, 0.0),
                node(2, 0.0, 0.0),
                node(3, 0.0, 1.0),
            ];
            for id in 4..68 {
                nodes.push(node(id, random(), random()));
            }
            let mut edges = vec![(0, 1), (2, 3)];
            for j in 0..64 {
                edges.push((j, (j + 1 + case) % nodes.len()));
            }
            let bounds: Vec<_> = edges
                .iter()
                .map(|&(a, b)| {
                    Bounds::from_segment_f32(nodes[a].x, nodes[a].z, nodes[b].x, nodes[b].z)
                        .unwrap()
                })
                .collect();
            let mut expected = Vec::new();
            for i in 0..edges.len() {
                for j in i + 1..edges.len() {
                    let (a, b) = edges[i];
                    let (c, d) = edges[j];
                    if pair_nonplanar(&nodes, a, b, c, d) {
                        expected.push((i, j));
                    }
                }
            }
            assert!(
                expected.contains(&(0, 1)),
                "f32 endpoint cancellation witness"
            );
            for quota in [1, 7, 48] {
                let mut bp = BroadPhase::new(bounds.clone()).unwrap();
                let mut actual = Vec::new();
                let mut calls = 0;
                loop {
                    let step = bp.advance(quota);
                    assert!(step.work_used <= quota);
                    if let Some((i, j)) = step.pair {
                        let (a, b) = edges[i];
                        let (c, d) = edges[j];
                        if pair_nonplanar(&nodes, a, b, c, d) {
                            actual.push((i, j));
                        }
                    }
                    if step.done {
                        break;
                    }
                    calls += 1;
                    if calls % 97 == 0 {
                        let json = serde_json::to_vec(&bp.checkpoint()).unwrap();
                        bp = BroadPhase::restore(serde_json::from_slice(&json).unwrap()).unwrap();
                    }
                }
                assert_eq!(actual, expected, "case {case}, quota {quota}");
            }
        }
    }

    #[test]
    fn raw_segment_aabb_can_miss_f32_endpoint_projection() {
        use crate::road_pair_broadphase::{Bounds, BroadPhase};
        let a = node(0, -100_000.0, 0.0);
        let b = node(1, -0.001, 0.0);
        let c = node(2, 0.0, 0.0);
        let d = node(3, 0.0, 1.0);
        assert_eq!(a.x + (b.x - a.x), 0.0);
        assert!(pair_nonplanar(&[a, b, c, d], 0, 1, 2, 3));
        let mut raw = BroadPhase::new(vec![
            Bounds {
                min_x: a.x as f64,
                max_x: b.x as f64,
                min_z: 0.0,
                max_z: 0.0,
            },
            Bounds {
                min_x: 0.0,
                max_x: 0.0,
                min_z: 0.0,
                max_z: 1.0,
            },
        ])
        .unwrap();
        loop {
            let step = raw.advance(48);
            assert!(step.pair.is_none());
            if step.done {
                break;
            }
        }
        for (ax, az, bx, bz) in [(f32::NAN, 0., 1., 1.), (100_001., 0., 1., 1.)] {
            assert!(Bounds::from_segment_f32(ax, az, bx, bz).is_err());
        }
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
    fn empty_node_index_missing_endpoint_failure_restores() {
        for quota in [1, 7, 48] {
            let job = GraphPreflight::new(Vec::new(), vec![edge(1, 10, 20)], None).unwrap();
            let job = GraphPreflight::restore(job.checkpoint()).unwrap();
            assert_eq!(drain(job, &[quota]).0, Err(RoadFaceError::MissingNode));
        }
    }

    #[test]
    fn coincidence_index_preserves_first_pair_and_source_cursor_after_resume() {
        for outside_index_domain in [false, true] {
            let mut nodes = vec![
                node(1, 10.0, 0.0),
                node(2, -2.0e-5, 0.0),
                node(3, 30.0, 0.0),
                node(4, -1.1e-5, 0.0),
                node(5, -1.9e-5, 0.0),
            ];
            if outside_index_domain {
                nodes[0].x = 200_000.0;
            }
            // A schema-3 checkpoint from the former all-pairs scan can point
            // at a distant pair that the new query would skip. Its cursor
            // still denotes the next eligible source index, not a cell slot.
            let mut old_cursor = GraphPreflight::new(nodes.clone(), vec![], None).unwrap();
            while old_cursor.checkpoint.stage == Stage::Nodes {
                old_cursor.advance(1);
            }
            let mut cp = old_cursor.checkpoint();
            cp.coincident_left = 1;
            cp.coincident_right = 2;
            let mut resumed = GraphPreflight::restore(cp).unwrap();
            while resumed.result().is_none() {
                resumed.advance(1);
            }
            assert_eq!(
                (
                    resumed.checkpoint.coincident_left,
                    resumed.checkpoint.coincident_right
                ),
                (1, 3)
            );
            for quota in [1, 7, 48] {
                let mut job = GraphPreflight::new(nodes.clone(), vec![], None).unwrap();
                while job.result().is_none() {
                    assert!(job.advance(quota).work_used <= quota);
                    let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                    let restored =
                        GraphPreflight::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
                    assert_eq!(restored, job);
                    job = restored;
                }
                assert_eq!(
                    job.result().unwrap().unwrap_err(),
                    reference_error(&nodes, &[], None)
                );
                assert_eq!(
                    (
                        job.checkpoint.coincident_left,
                        job.checkpoint.coincident_right
                    ),
                    (1, 3)
                );
            }
        }
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
        let failed = GraphPreflight::restore(job.checkpoint()).unwrap();
        assert_eq!(
            failed.result().unwrap().unwrap_err(),
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
            vec![node(1, 0.0, 0.0), node(2, 10.0, 0.0)],
            vec![edge(1, 1, 2)],
            None,
        )
        .unwrap();
        while job.checkpoint.stage != Stage::Edges
            || job.checkpoint.edge_stage != EdgeStage::LookupFrom
        {
            job.advance(1);
        }
        let mut checkpoint = job.checkpoint();
        checkpoint.lookup_cursor = 1;
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
    fn zero_quota_is_stable_and_unsupported_coordinates_use_exact_fallback() {
        let nodes = vec![
            node(1, 200_000.0, 0.0),
            node(2, 200_010.0, 10.0),
            node(3, 200_010.0, 0.0),
            node(4, 200_000.0, 10.0),
        ];
        let edges = vec![edge(1, 1, 2), edge(2, 3, 4)];
        let mut job = GraphPreflight::new(nodes.clone(), edges.clone(), None).unwrap();
        let before = serde_json::to_vec(&job.checkpoint()).unwrap();
        assert_eq!(job.advance(0).work_used, 0);
        assert_eq!(serde_json::to_vec(&job.checkpoint()).unwrap(), before);
        while job.result().is_none() {
            let step = job.advance(1);
            assert!(step.work_used <= 1);
            let checkpoint: GraphPreflightCheckpoint =
                serde_json::from_slice(&serde_json::to_vec(&job.checkpoint()).unwrap()).unwrap();
            job = GraphPreflight::restore(checkpoint).unwrap();
        }
        assert_eq!(job.result().unwrap().unwrap_err(), RoadFaceError::NonPlanar);
        assert!(job.checkpoint.filter_fallback);
    }

    #[test]
    fn mixed_schema_downgrade_is_rejected_and_pending_candidate_cannot_be_forged() {
        let edge_count = 256usize;
        let mut nodes = Vec::with_capacity(edge_count * 2);
        let mut edges = Vec::new();
        for index in 0..edge_count {
            let (x1, z1, x2, z2) = match index {
                0 => (0.0, 0.0, 1.0, 0.0),
                1 => (0.5, -1.0, 0.5, 1.0),
                _ => {
                    let x = 10.0 + index as f32 * 3.0;
                    (x, 0.0, x + 1.0, 0.0)
                }
            };
            let from = index as u32 * 2;
            nodes.push(node(from, x1, z1));
            nodes.push(node(from + 1, x2, z2));
            edges.push(edge(index as u32, from, from + 1));
        }
        let mut job = GraphPreflight::new(nodes, edges, None).unwrap();
        let mut legacy = job.checkpoint();
        legacy.schema = 1;
        assert_eq!(
            GraphPreflight::restore(legacy).unwrap_err(),
            RoadFaceError::InvalidEdge
        );
        while job.checkpoint.stage != Stage::Planarity {
            job.advance(48);
        }
        let mut turns = 0;
        while job.checkpoint.pending_pair.is_none() {
            job.advance(1);
            turns += 1;
            assert!(
                job.result().is_none(),
                "completed before a candidate paused"
            );
            assert!(turns < 100_000);
        }
        let mut forged = job.checkpoint();
        forged.stage = Stage::Done;
        assert_eq!(
            GraphPreflight::restore(forged.clone()).unwrap_err(),
            RoadFaceError::InvalidEdge
        );
        forged.pending_pair = None;
        assert_eq!(
            GraphPreflight::restore(forged).unwrap_err(),
            RoadFaceError::InvalidEdge
        );
    }

    #[test]
    fn dense_filter_falls_back_once_and_resumes_exact_scan() {
        let count = 401usize;
        let mut nodes = vec![node(0, 0.0, 0.0)];
        let mut edges = Vec::new();
        for index in 0..count - 1 {
            let angle = std::f32::consts::TAU * index as f32 / (count - 1) as f32;
            nodes.push(node(index as u32 + 1, angle.cos(), angle.sin()));
            edges.push(edge(index as u32, 0, index as u32 + 1));
        }
        let mut job = GraphPreflight::new(nodes, edges, None).unwrap();
        let mut turned_on_fallback = false;
        while job.result().is_none() {
            job.advance(48);
            if job.checkpoint.filter_fallback {
                let cp: GraphPreflightCheckpoint =
                    serde_json::from_slice(&serde_json::to_vec(&job.checkpoint()).unwrap())
                        .unwrap();
                let mut premature_done = cp.clone();
                premature_done.stage = Stage::Done;
                assert_eq!(
                    GraphPreflight::restore(premature_done).unwrap_err(),
                    RoadFaceError::InvalidEdge
                );
                job = GraphPreflight::restore(cp).unwrap();
                turned_on_fallback = true;
                break;
            }
        }
        assert!(
            turned_on_fallback,
            "dense candidates should hit the scan budget"
        );
        while job.result().is_none() {
            job.advance(48);
        }
        assert_eq!(job.result().unwrap().unwrap().len(), count - 1);
    }
    #[test]
    fn legacy_preflight_restarts_frozen_graph_and_preserves_result() {
        let nodes = vec![
            node(1, 0., 0.),
            node(2, 8., 0.),
            node(3, 8., 8.),
            node(4, 0., 8.),
        ];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3), edge(3, 3, 4)];
        let original = GraphPreflight::new(nodes, edges, Some(edge(4, 4, 1))).unwrap();
        let mut value = serde_json::to_value(original.checkpoint()).unwrap();
        value["schema"] = 1.into();
        for field in [
            "filter_work",
            "filter_build_cursor",
            "filter_fallback",
            "filter_checkpoint",
            "pending_pair",
            "last_checked_pair",
        ] {
            value.as_object_mut().unwrap().remove(field);
        }
        // Legacy progress is obsolete computation, never authoritative world state.
        value["node_cursor"] = 2.into();
        value["work_completed"] = 5.into();
        let legacy: GraphPreflightCheckpoint = serde_json::from_value(value.clone()).unwrap();
        let migrated = GraphPreflight::restore(legacy).unwrap();
        assert_eq!(migrated, original);
        assert_eq!(drain(migrated, &[1, 7, 48]), drain(original.clone(), &[48]));

        let mut schema_two = original.checkpoint();
        schema_two.schema = 2;
        schema_two.node_cursor = 2;
        schema_two.work_completed = 5;
        let migrated_two = GraphPreflight::restore(schema_two).unwrap();
        assert_eq!(migrated_two, original);
        assert_eq!(drain(migrated_two, &[1, 7, 48]), drain(original, &[48]));

        value["schema"] = 4.into();
        assert!(GraphPreflight::restore(serde_json::from_value(value).unwrap()).is_err());
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
