//! Incremental directed-face walking, canonicalization, sorting, and nesting
//! rejection for an already-built half-edge topology.
//!
//! This stage owns no graph mutation or planning policy. Every walk visit,
//! output copy, polygon-edge test, identity comparison, and sort move is a
//! separate deterministic work unit. Checkpoints are restored only through
//! `FaceWalkSearch::restore`.

use crate::legacy::road_face_topology::HalfEdgeRecord;
use crate::legacy::road_faces::{RoadFace, RoadFaceEdge, RoadFaceError, RoadFaceNode};
use crate::world::geometry::segments_distance_coords;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_NODES: usize = 512;
const MAX_EDGES: usize = 512;
const MAX_HALF_EDGES: usize = 1024;
const MAX_HALF_EDGE_STEPS: usize = 1024;
const MAX_OUTPUT_FACES: usize = 512;
#[cfg(feature = "legacy")]
const COINCIDENT_EPSILON: f32 = 1.0e-5;
#[cfg(feature = "legacy")]
const MIN_FACE_AREA: f64 = 1.0e-3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedOrdering {
    Less,
    Equal,
    Greater,
}

impl From<Ordering> for SavedOrdering {
    fn from(value: Ordering) -> Self {
        match value {
            Ordering::Less => Self::Less,
            Ordering::Equal => Self::Equal,
            Ordering::Greater => Self::Greater,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Initialize,
    ScanStart,
    WalkVisit,
    AppendVertex,
    AppendEdge,
    MoveNext,
    RepeatVertices,
    Area,
    FindMinimum,
    BuildFaceArrays,
    CanonicalBuild,
    CanonicalCompare,
    CanonicalDecide,
    ContainsProposed,
    EmitFace,
    SortFaces,
    Nested,
    Done,
    Failed(RoadFaceError),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    schema: u8,
    nodes: Vec<RoadFaceNode>,
    edges: Vec<RoadFaceEdge>,
    half_edges: Vec<HalfEdgeRecord>,
    next: Vec<usize>,
    proposed_edge_id: Option<u32>,
    phase: Phase,
    initialize_cursor: usize,

    visited: Vec<bool>,
    start_cursor: usize,
    walk_start: usize,
    current: usize,
    walk_vertices: Vec<usize>,
    walk_edges: Vec<usize>,
    walk_steps: usize,
    seen_marks: Vec<u32>,
    walk_epoch: u32,
    repeat_index: usize,
    repeated: bool,

    area_index: usize,
    area_twice: f64,
    area: f64,
    minimum_index: usize,
    minimum_cursor: usize,

    output_vertex_ids: Vec<u32>,
    output_polygon: Vec<(f32, f32)>,
    output_edge_ids: Vec<u32>,
    output_cursor: usize,
    output_component: u8,

    canonical_sequence: u8,
    canonical_offset: usize,
    canonical_build_cursor: usize,
    canonical_candidate: Vec<u32>,
    canonical_best: Vec<u32>,
    canonical_compare_cursor: usize,
    canonical_order: Option<SavedOrdering>,

    proposed_cursor: usize,
    contains_proposed_edge: bool,
    faces: Vec<RoadFace>,

    sort_index: usize,
    sort_compare: usize,
    sort_compare_cursor: usize,
    sort_order: Option<SavedOrdering>,
    sort_key: Option<RoadFace>,

    nested_first: usize,
    nested_second: usize,
    nested_direction: u8,
    nested_edge: usize,
    nested_inside: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FaceWalkSearch {
    state: Checkpoint,
}

impl FaceWalkSearch {
    pub(crate) fn new(
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
        half_edges: Vec<HalfEdgeRecord>,
        next: Vec<usize>,
        proposed_edge_id: Option<u32>,
    ) -> Result<Self, RoadFaceError> {
        validate_inputs(&nodes, &edges, &half_edges, &next)?;
        Ok(Self {
            state: Checkpoint {
                schema: CHECKPOINT_SCHEMA,
                visited: vec![false; half_edges.len()],
                seen_marks: vec![0; nodes.len()],
                nodes,
                edges,
                half_edges,
                next,
                proposed_edge_id,
                phase: Phase::ScanStart,
                initialize_cursor: 0,
                start_cursor: 0,
                walk_start: 0,
                current: 0,
                walk_vertices: Vec::new(),
                walk_edges: Vec::new(),
                walk_steps: 0,
                walk_epoch: 0,
                repeat_index: 0,
                repeated: false,
                area_index: 0,
                area_twice: 0.0,
                area: 0.0,
                minimum_index: 0,
                minimum_cursor: 0,
                output_vertex_ids: Vec::new(),
                output_polygon: Vec::new(),
                output_edge_ids: Vec::new(),
                output_cursor: 0,
                output_component: 0,
                canonical_sequence: 0,
                canonical_offset: 0,
                canonical_build_cursor: 0,
                canonical_candidate: Vec::new(),
                canonical_best: Vec::new(),
                canonical_compare_cursor: 0,
                canonical_order: None,
                proposed_cursor: 0,
                contains_proposed_edge: false,
                faces: Vec::new(),
                sort_index: 1,
                sort_compare: 0,
                sort_compare_cursor: 0,
                sort_order: None,
                sort_key: None,
                nested_first: 0,
                nested_second: 1,
                nested_direction: 0,
                nested_edge: 0,
                nested_inside: false,
            },
        })
    }

    /// Construct from graph stages that have already validated every input.
    /// Per-node and per-half-edge marks are allocated incrementally by
    /// `advance` rather than initialized during a stage handoff.
    #[cfg(feature = "legacy")]
    pub(crate) fn new_trusted(
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
        half_edges: Vec<HalfEdgeRecord>,
        next: Vec<usize>,
        proposed_edge_id: Option<u32>,
    ) -> Result<Self, RoadFaceError> {
        if nodes.len() > MAX_NODES
            || edges.len() > MAX_EDGES
            || half_edges.len() > MAX_HALF_EDGES
            || next.len() != half_edges.len()
        {
            return Err(RoadFaceError::InvalidEmbedding);
        }
        Ok(Self {
            state: Checkpoint {
                schema: CHECKPOINT_SCHEMA,
                nodes,
                edges,
                half_edges,
                next,
                proposed_edge_id,
                phase: Phase::Initialize,
                initialize_cursor: 0,
                visited: Vec::with_capacity(MAX_HALF_EDGES),
                start_cursor: 0,
                walk_start: 0,
                current: 0,
                walk_vertices: Vec::new(),
                walk_edges: Vec::new(),
                walk_steps: 0,
                seen_marks: Vec::with_capacity(MAX_NODES),
                walk_epoch: 0,
                repeat_index: 0,
                repeated: false,
                area_index: 0,
                area_twice: 0.0,
                area: 0.0,
                minimum_index: 0,
                minimum_cursor: 0,
                output_vertex_ids: Vec::new(),
                output_polygon: Vec::new(),
                output_edge_ids: Vec::new(),
                output_cursor: 0,
                output_component: 0,
                canonical_sequence: 0,
                canonical_offset: 0,
                canonical_build_cursor: 0,
                canonical_candidate: Vec::new(),
                canonical_best: Vec::new(),
                canonical_compare_cursor: 0,
                canonical_order: None,
                proposed_cursor: 0,
                contains_proposed_edge: false,
                faces: Vec::new(),
                sort_index: 1,
                sort_compare: 0,
                sort_compare_cursor: 0,
                sort_order: None,
                sort_key: None,
                nested_first: 0,
                nested_second: 1,
                nested_direction: 0,
                nested_edge: 0,
                nested_inside: false,
            },
        })
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        self.state.clone()
    }

    pub(crate) fn restore(checkpoint: Checkpoint) -> Result<Self, RoadFaceError> {
        validate_checkpoint(&checkpoint)?;
        Ok(Self { state: checkpoint })
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn faces(&self) -> Option<Result<&[RoadFace], RoadFaceError>> {
        match self.state.phase {
            Phase::Done => Some(Ok(&self.state.faces)),
            Phase::Failed(error) => Some(Err(error)),
            _ => None,
        }
    }

    pub(crate) fn proposed_edge_id(&self) -> Option<u32> {
        self.state.proposed_edge_id
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn into_faces(self) -> Result<Vec<RoadFace>, RoadFaceError> {
        match self.state.phase {
            Phase::Done => Ok(self.state.faces),
            Phase::Failed(error) => Err(error),
            _ => Err(RoadFaceError::InvalidEmbedding),
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || matches!(self.state.phase, Phase::Done | Phase::Failed(_)) {
            return AdvanceResult {
                work_used: 0,
                done: matches!(self.state.phase, Phase::Done | Phase::Failed(_)),
            };
        }
        let mut used = 0;
        while used < budget && !matches!(self.state.phase, Phase::Done | Phase::Failed(_)) {
            match self.state.phase {
                Phase::Initialize => self.advance_initialize(),
                Phase::ScanStart => self.advance_scan_start(),
                Phase::WalkVisit => self.advance_walk_visit(),
                Phase::AppendVertex => self.advance_append_vertex(),
                Phase::AppendEdge => self.advance_append_edge(),
                Phase::MoveNext => self.advance_move_next(),
                Phase::RepeatVertices => self.advance_repeat(),
                Phase::Area => self.advance_area(),
                Phase::FindMinimum => self.advance_minimum(),
                Phase::BuildFaceArrays => self.advance_face_arrays(),
                Phase::CanonicalBuild => self.advance_canonical_build(),
                Phase::CanonicalCompare => self.advance_canonical_compare(),
                Phase::CanonicalDecide => self.advance_canonical_decide(),
                Phase::ContainsProposed => self.advance_contains_proposed(),
                Phase::EmitFace => self.advance_emit_face(),
                Phase::SortFaces => self.advance_sort_faces(),
                Phase::Nested => self.advance_nested(),
                Phase::Done | Phase::Failed(_) => break,
            }
            used += 1;
        }
        AdvanceResult {
            work_used: used,
            done: matches!(self.state.phase, Phase::Done | Phase::Failed(_)),
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_initialize(&mut self) {
        let cp = &mut self.state;
        let total = cp.nodes.len().max(cp.half_edges.len());
        if cp.initialize_cursor >= total {
            cp.phase = Phase::ScanStart;
            return;
        }
        let index = cp.initialize_cursor;
        if index < cp.nodes.len() {
            cp.seen_marks.push(0);
        }
        if index < cp.half_edges.len() {
            cp.visited.push(false);
        }
        cp.initialize_cursor += 1;
    }

    #[cfg(feature = "legacy")]
    fn advance_scan_start(&mut self) {
        if self.state.start_cursor >= self.state.half_edges.len() {
            self.state.phase = Phase::SortFaces;
            self.state.sort_index = 1;
            return;
        }
        let start = self.state.start_cursor;
        self.state.start_cursor += 1;
        if self.state.visited[start] {
            return;
        }
        self.state.walk_start = start;
        self.state.current = start;
        self.state.walk_vertices.clear();
        self.state.walk_edges.clear();
        self.state.phase = Phase::WalkVisit;
    }

    #[cfg(feature = "legacy")]
    fn advance_walk_visit(&mut self) {
        let current = self.state.current;
        if self.state.visited[current] {
            if current != self.state.walk_start {
                self.fail(RoadFaceError::InvalidEmbedding);
            } else {
                self.finish_walk();
            }
            return;
        }
        if self.state.walk_steps >= MAX_HALF_EDGE_STEPS {
            self.fail(RoadFaceError::WorkLimit);
            return;
        }
        self.state.visited[current] = true;
        self.state.walk_steps += 1;
        self.state.phase = Phase::AppendVertex;
    }

    #[cfg(feature = "legacy")]
    fn advance_append_vertex(&mut self) {
        let half = &self.state.half_edges[self.state.current];
        self.state.walk_vertices.push(half.from);
        self.state.phase = Phase::AppendEdge;
    }

    #[cfg(feature = "legacy")]
    fn advance_append_edge(&mut self) {
        let half = &self.state.half_edges[self.state.current];
        self.state.walk_edges.push(half.edge_index);
        self.state.phase = Phase::MoveNext;
    }

    #[cfg(feature = "legacy")]
    fn advance_move_next(&mut self) {
        self.state.current = self.state.next[self.state.current];
        self.state.phase = Phase::WalkVisit;
    }

    #[cfg(feature = "legacy")]
    fn finish_walk(&mut self) {
        let count = self.state.walk_vertices.len();
        if count < 3 {
            self.next_walk();
            return;
        }
        self.state.walk_epoch = self.state.walk_epoch.wrapping_add(1);
        if self.state.walk_epoch == 0 {
            self.state.seen_marks.fill(0);
            self.state.walk_epoch = 1;
        }
        self.state.repeat_index = 0;
        self.state.repeated = false;
        self.state.phase = Phase::RepeatVertices;
    }

    #[cfg(feature = "legacy")]
    fn advance_repeat(&mut self) {
        if self.state.repeat_index >= self.state.walk_vertices.len() {
            if self.state.repeated {
                self.next_walk();
            } else {
                self.state.area_index = 0;
                self.state.area_twice = 0.0;
                self.state.phase = Phase::Area;
            }
            return;
        }
        let vertex = self.state.walk_vertices[self.state.repeat_index];
        if self.state.seen_marks[vertex] == self.state.walk_epoch {
            self.state.repeated = true;
        } else {
            self.state.seen_marks[vertex] = self.state.walk_epoch;
        }
        self.state.repeat_index += 1;
    }

    #[cfg(feature = "legacy")]
    fn advance_area(&mut self) {
        if self.state.area_index >= self.state.walk_vertices.len() {
            self.state.area = self.state.area_twice * 0.5;
            if !self.state.area.is_finite() || self.state.area <= MIN_FACE_AREA {
                self.next_walk();
            } else if self.state.faces.len() >= MAX_OUTPUT_FACES {
                self.fail(RoadFaceError::OutputLimit);
            } else {
                self.state.minimum_index = 0;
                self.state.minimum_cursor = 1;
                self.state.phase = Phase::FindMinimum;
            }
            return;
        }
        let index = self.state.area_index;
        let current = self.state.nodes[self.state.walk_vertices[index]];
        let next = self.state.nodes
            [self.state.walk_vertices[(index + 1) % self.state.walk_vertices.len()]];
        self.state.area_twice +=
            current.x as f64 * next.z as f64 - next.x as f64 * current.z as f64;
        self.state.area_index += 1;
    }

    #[cfg(feature = "legacy")]
    fn advance_minimum(&mut self) {
        if self.state.minimum_cursor >= self.state.walk_vertices.len() {
            self.state.output_vertex_ids.clear();
            self.state.output_polygon.clear();
            self.state.output_edge_ids.clear();
            self.state.output_cursor = 0;
            self.state.output_component = 0;
            self.state.phase = Phase::BuildFaceArrays;
            return;
        }
        let cursor = self.state.minimum_cursor;
        let candidate_id = self.state.nodes[self.state.walk_vertices[cursor]].id;
        let best_id = self.state.nodes[self.state.walk_vertices[self.state.minimum_index]].id;
        if candidate_id < best_id {
            self.state.minimum_index = cursor;
        }
        self.state.minimum_cursor += 1;
    }

    #[cfg(feature = "legacy")]
    fn advance_face_arrays(&mut self) {
        if self.state.output_cursor >= self.state.walk_vertices.len() {
            self.state.canonical_sequence = 0;
            self.state.canonical_offset = 0;
            self.state.canonical_build_cursor = 0;
            self.state.canonical_candidate.clear();
            self.state.canonical_best.clear();
            self.state.canonical_compare_cursor = 0;
            self.state.canonical_order = None;
            self.state.phase = Phase::CanonicalBuild;
            return;
        }
        let source_index =
            (self.state.minimum_index + self.state.output_cursor) % self.state.walk_vertices.len();
        let node = self.state.nodes[self.state.walk_vertices[source_index]];
        match self.state.output_component {
            0 => {
                self.state.output_vertex_ids.push(node.id);
                self.state.output_component = 1;
            }
            1 => {
                self.state.output_polygon.push((node.x, node.z));
                self.state.output_component = 2;
            }
            _ => {
                self.state
                    .output_edge_ids
                    .push(self.state.edges[self.state.walk_edges[source_index]].id);
                self.state.output_cursor += 1;
                self.state.output_component = 0;
            }
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_canonical_build(&mut self) {
        let count = self.state.output_vertex_ids.len();
        if self.state.canonical_build_cursor >= count {
            self.state.canonical_compare_cursor = 0;
            self.state.canonical_order = None;
            self.state.phase = Phase::CanonicalCompare;
            return;
        }
        let rotated = (self.state.canonical_offset + self.state.canonical_build_cursor) % count;
        let source = if self.state.canonical_sequence == 0 {
            rotated
        } else {
            count - 1 - rotated
        };
        self.state
            .canonical_candidate
            .push(self.state.output_vertex_ids[source]);
        self.state.canonical_build_cursor += 1;
    }

    #[cfg(feature = "legacy")]
    fn advance_canonical_compare(&mut self) {
        if self.state.canonical_best.is_empty() {
            self.state.phase = Phase::CanonicalDecide;
            self.state.canonical_order = Some(SavedOrdering::Less);
            return;
        }
        let cursor = self.state.canonical_compare_cursor;
        let compared_len = self
            .state
            .canonical_candidate
            .len()
            .min(self.state.canonical_best.len());
        if cursor >= compared_len {
            self.state.canonical_order = Some(
                self.state
                    .canonical_candidate
                    .len()
                    .cmp(&self.state.canonical_best.len())
                    .into(),
            );
            self.state.phase = Phase::CanonicalDecide;
            return;
        }
        let order = self.state.canonical_candidate[cursor].cmp(&self.state.canonical_best[cursor]);
        self.state.canonical_compare_cursor += 1;
        if order != Ordering::Equal {
            self.state.canonical_order = Some(order.into());
            self.state.phase = Phase::CanonicalDecide;
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_canonical_decide(&mut self) {
        if self.state.canonical_order == Some(SavedOrdering::Less) {
            self.state.canonical_best = std::mem::take(&mut self.state.canonical_candidate);
        } else {
            self.state.canonical_candidate.clear();
        }
        self.state.canonical_offset += 1;
        self.state.canonical_build_cursor = 0;
        self.state.canonical_compare_cursor = 0;
        self.state.canonical_order = None;
        let count = self.state.output_vertex_ids.len();
        if self.state.canonical_offset >= count {
            self.state.canonical_offset = 0;
            self.state.canonical_sequence += 1;
            if self.state.canonical_sequence >= 2 {
                self.state.proposed_cursor = 0;
                self.state.contains_proposed_edge = false;
                self.state.phase = Phase::ContainsProposed;
                return;
            }
        }
        self.state.phase = Phase::CanonicalBuild;
    }

    #[cfg(feature = "legacy")]
    fn advance_contains_proposed(&mut self) {
        let Some(proposed_id) = self.state.proposed_edge_id else {
            self.state.phase = Phase::EmitFace;
            return;
        };
        if self.state.proposed_cursor >= self.state.output_edge_ids.len() {
            self.state.phase = Phase::EmitFace;
            return;
        }
        if self.state.output_edge_ids[self.state.proposed_cursor] == proposed_id {
            self.state.contains_proposed_edge = true;
            self.state.phase = Phase::EmitFace;
        } else {
            self.state.proposed_cursor += 1;
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_emit_face(&mut self) {
        self.state.faces.push(RoadFace {
            identity: std::mem::take(&mut self.state.canonical_best),
            vertex_ids: std::mem::take(&mut self.state.output_vertex_ids),
            polygon: std::mem::take(&mut self.state.output_polygon),
            area: self.state.area,
            boundary_edge_ids: std::mem::take(&mut self.state.output_edge_ids),
            contains_proposed_edge: self.state.contains_proposed_edge,
        });
        self.next_walk();
    }

    #[cfg(feature = "legacy")]
    fn next_walk(&mut self) {
        self.state.walk_vertices.clear();
        self.state.walk_edges.clear();
        self.state.walk_steps = 0;
        self.state.repeat_index = 0;
        self.state.repeated = false;
        self.state.area_index = 0;
        self.state.area_twice = 0.0;
        self.state.area = 0.0;
        self.state.minimum_index = 0;
        self.state.minimum_cursor = 0;
        self.state.output_vertex_ids.clear();
        self.state.output_polygon.clear();
        self.state.output_edge_ids.clear();
        self.state.output_cursor = 0;
        self.state.output_component = 0;
        self.state.canonical_sequence = 0;
        self.state.canonical_offset = 0;
        self.state.canonical_build_cursor = 0;
        self.state.canonical_candidate.clear();
        self.state.canonical_best.clear();
        self.state.canonical_compare_cursor = 0;
        self.state.canonical_order = None;
        self.state.proposed_cursor = 0;
        self.state.contains_proposed_edge = false;
        self.state.phase = Phase::ScanStart;
    }

    #[cfg(feature = "legacy")]
    fn advance_sort_faces(&mut self) {
        if self.state.sort_key.is_none() {
            if self.state.sort_index >= self.state.faces.len() {
                self.state.phase = if self.state.faces.len() < 2 {
                    Phase::Done
                } else {
                    Phase::Nested
                };
                self.state.nested_first = 0;
                self.state.nested_second = 1;
                self.state.nested_direction = 0;
                self.state.nested_edge = 0;
                self.state.nested_inside = false;
                return;
            }
            let placeholder = empty_face();
            self.state.sort_key = Some(std::mem::replace(
                &mut self.state.faces[self.state.sort_index],
                placeholder,
            ));
            self.state.sort_compare = self.state.sort_index;
            self.state.sort_compare_cursor = 0;
            self.state.sort_order = None;
            return;
        }
        let key = self.state.sort_key.as_ref().expect("face sort key set");
        if self.state.sort_compare == 0 {
            self.state.faces[0] = self.state.sort_key.take().expect("key remains available");
            self.state.sort_index += 1;
            self.state.sort_compare_cursor = 0;
            self.state.sort_order = None;
            return;
        }
        let previous = &self.state.faces[self.state.sort_compare - 1].identity;
        if self.state.sort_order.is_none() {
            let cursor = self.state.sort_compare_cursor;
            let common = previous.len().min(key.identity.len());
            if cursor < common {
                let order = previous[cursor].cmp(&key.identity[cursor]);
                self.state.sort_compare_cursor += 1;
                if order != Ordering::Equal {
                    self.state.sort_order = Some(order.into());
                }
                return;
            }
            self.state.sort_order = Some(previous.len().cmp(&key.identity.len()).into());
            return;
        }
        if self.state.sort_order == Some(SavedOrdering::Greater) {
            let source = self.state.sort_compare - 1;
            self.state.faces[self.state.sort_compare] =
                std::mem::replace(&mut self.state.faces[source], empty_face());
            self.state.sort_compare -= 1;
            self.state.sort_compare_cursor = 0;
            self.state.sort_order = None;
        } else {
            self.state.faces[self.state.sort_compare] =
                self.state.sort_key.take().expect("key remains available");
            self.state.sort_index += 1;
            self.state.sort_compare_cursor = 0;
            self.state.sort_order = None;
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_nested(&mut self) {
        if self.state.faces.len() < 2 || self.state.nested_first >= self.state.faces.len() - 1 {
            self.state.phase = Phase::Done;
            return;
        }
        if self.state.nested_second >= self.state.faces.len() {
            self.state.nested_first += 1;
            self.state.nested_second = self.state.nested_first + 1;
            self.state.nested_direction = 0;
            self.state.nested_edge = 0;
            self.state.nested_inside = false;
            return;
        }
        let (point, polygon) = if self.state.nested_direction == 0 {
            (
                self.state.faces[self.state.nested_first].polygon[0],
                &self.state.faces[self.state.nested_second].polygon,
            )
        } else {
            (
                self.state.faces[self.state.nested_second].polygon[0],
                &self.state.faces[self.state.nested_first].polygon,
            )
        };
        if self.state.nested_edge >= polygon.len() {
            let inside = self.state.nested_inside;
            if inside {
                self.fail(RoadFaceError::NestedFacesUnsupported);
            } else if self.state.nested_direction == 0 {
                self.state.nested_direction = 1;
                self.state.nested_edge = 0;
                self.state.nested_inside = false;
            } else {
                self.state.nested_second += 1;
                self.state.nested_direction = 0;
                self.state.nested_edge = 0;
                self.state.nested_inside = false;
            }
            return;
        }
        let index = self.state.nested_edge;
        let (ax, az) = polygon[index];
        let (bx, bz) = polygon[(index + 1) % polygon.len()];
        if segments_distance_coords(point.0, point.1, point.0, point.1, ax, az, bx, bz)
            <= COINCIDENT_EPSILON
        {
            self.state.nested_inside = false;
            self.state.nested_edge = polygon.len();
            return;
        }
        if (az > point.1) != (bz > point.1) && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
        {
            self.state.nested_inside = !self.state.nested_inside;
        }
        self.state.nested_edge += 1;
    }

    #[cfg(feature = "legacy")]
    fn fail(&mut self, error: RoadFaceError) {
        self.state.phase = Phase::Failed(error);
    }
}

#[cfg(feature = "legacy")]
fn empty_face() -> RoadFace {
    RoadFace {
        identity: Vec::new(),
        vertex_ids: Vec::new(),
        polygon: Vec::new(),
        area: 0.0,
        boundary_edge_ids: Vec::new(),
        contains_proposed_edge: false,
    }
}

fn validate_inputs(
    nodes: &[RoadFaceNode],
    edges: &[RoadFaceEdge],
    half_edges: &[HalfEdgeRecord],
    next: &[usize],
) -> Result<(), RoadFaceError> {
    if nodes.len() > MAX_NODES {
        return Err(RoadFaceError::TooManyNodes);
    }
    if edges.len() > MAX_EDGES {
        return Err(RoadFaceError::TooManyEdges);
    }
    if half_edges.len() > MAX_HALF_EDGES
        || next.len() != half_edges.len()
        || nodes
            .iter()
            .any(|node| !node.x.is_finite() || !node.z.is_finite())
        || half_edges.iter().any(|half| {
            half.edge_index >= edges.len()
                || half.from >= nodes.len()
                || half.to >= nodes.len()
                || !half.angle.is_finite()
        })
        || next.iter().any(|&index| index >= half_edges.len())
    {
        return Err(RoadFaceError::InvalidEmbedding);
    }
    Ok(())
}

fn validate_checkpoint(cp: &Checkpoint) -> Result<(), RoadFaceError> {
    if cp.schema != CHECKPOINT_SCHEMA
        || cp.nodes.len() > MAX_NODES
        || cp.edges.len() > MAX_EDGES
        || cp.half_edges.len() > MAX_HALF_EDGES
        || cp.next.len() != cp.half_edges.len()
        || cp
            .nodes
            .iter()
            .any(|node| !node.x.is_finite() || !node.z.is_finite())
        || cp.half_edges.iter().any(|half| {
            half.edge_index >= cp.edges.len()
                || half.from >= cp.nodes.len()
                || half.to >= cp.nodes.len()
                || !half.angle.is_finite()
        })
        || cp.next.iter().any(|&index| index >= cp.half_edges.len())
    {
        return Err(RoadFaceError::InvalidEmbedding);
    }
    if cp.phase == Phase::Initialize {
        let total = cp.nodes.len().max(cp.half_edges.len());
        let initialized_nodes = cp.initialize_cursor.min(cp.nodes.len());
        let initialized_half_edges = cp.initialize_cursor.min(cp.half_edges.len());
        if cp.initialize_cursor > total
            || cp.visited.len() != initialized_half_edges
            || cp.seen_marks.len() != initialized_nodes
            || cp.visited.iter().any(|&visited| visited)
            || cp.seen_marks.iter().any(|&mark| mark != 0)
            || cp.start_cursor != 0
            || cp.walk_start != 0
            || cp.current != 0
            || cp.walk_steps != 0
            || cp.walk_epoch != 0
            || cp.repeat_index != 0
            || cp.repeated
            || cp.area_index != 0
            || cp.area_twice != 0.0
            || cp.area != 0.0
            || cp.minimum_index != 0
            || cp.minimum_cursor != 0
            || !cp.walk_vertices.is_empty()
            || !cp.walk_edges.is_empty()
            || !cp.output_vertex_ids.is_empty()
            || !cp.output_polygon.is_empty()
            || !cp.output_edge_ids.is_empty()
            || cp.output_cursor != 0
            || cp.output_component != 0
            || cp.canonical_sequence != 0
            || cp.canonical_offset != 0
            || cp.canonical_build_cursor != 0
            || !cp.canonical_candidate.is_empty()
            || !cp.canonical_best.is_empty()
            || cp.canonical_compare_cursor != 0
            || cp.canonical_order.is_some()
            || cp.proposed_cursor != 0
            || cp.contains_proposed_edge
            || !cp.faces.is_empty()
            || cp.sort_index != 1
            || cp.sort_compare != 0
            || cp.sort_compare_cursor != 0
            || cp.sort_order.is_some()
            || cp.sort_key.is_some()
            || cp.nested_first != 0
            || cp.nested_second != 1
            || cp.nested_direction != 0
            || cp.nested_edge != 0
            || cp.nested_inside
        {
            return Err(RoadFaceError::InvalidEmbedding);
        }
        return Ok(());
    }
    let face_perimeter = cp.faces.iter().try_fold(0usize, |total, face| {
        total.checked_add(face.vertex_ids.len())
    });
    let is_active_walk = matches!(
        cp.phase,
        Phase::WalkVisit | Phase::AppendVertex | Phase::AppendEdge | Phase::MoveNext
    );
    if cp.schema != CHECKPOINT_SCHEMA
        || cp.visited.len() != cp.half_edges.len()
        || cp.seen_marks.len() != cp.nodes.len()
        || (cp.phase != Phase::AppendEdge && cp.walk_vertices.len() != cp.walk_edges.len())
        || cp.walk_vertices.len() > MAX_HALF_EDGE_STEPS
        || cp.walk_edges.len() > MAX_HALF_EDGE_STEPS
        || face_perimeter.is_none_or(|total| total > MAX_HALF_EDGES)
        || cp
            .walk_vertices
            .iter()
            .any(|&index| index >= cp.nodes.len())
        || cp.walk_edges.iter().any(|&index| index >= cp.edges.len())
        || cp.faces.len() > MAX_OUTPUT_FACES
        || cp.start_cursor > cp.half_edges.len()
        || (is_active_walk
            && (cp.current >= cp.half_edges.len() || cp.walk_start >= cp.half_edges.len()))
        || cp.walk_steps > MAX_HALF_EDGE_STEPS
        || cp.walk_epoch as usize > cp.half_edges.len()
        || cp.repeat_index > cp.walk_vertices.len()
        || cp.area_index > cp.walk_vertices.len()
        || cp.minimum_cursor > cp.walk_vertices.len()
        || cp.output_cursor > cp.walk_vertices.len()
        || cp.output_component > 2
        || cp.output_vertex_ids.len() > cp.walk_vertices.len()
        || cp.output_polygon.len() > cp.walk_vertices.len()
        || cp.output_edge_ids.len() > cp.walk_edges.len()
        || cp.canonical_sequence > 2
        || cp.canonical_offset > cp.output_vertex_ids.len()
        || cp.canonical_build_cursor > cp.output_vertex_ids.len()
        || cp.canonical_candidate.len() > cp.output_vertex_ids.len()
        || cp.canonical_best.len() > cp.output_vertex_ids.len()
        || cp.canonical_compare_cursor > cp.output_vertex_ids.len()
        || cp.proposed_cursor > cp.output_edge_ids.len()
        || cp.faces.iter().any(|face| {
            face.identity.len() > MAX_HALF_EDGES
                || face.vertex_ids.len() > MAX_HALF_EDGES
                || face.polygon.len() > MAX_HALF_EDGES
                || face.boundary_edge_ids.len() > MAX_HALF_EDGES
                || !face.area.is_finite()
                || face.identity.len() != face.vertex_ids.len()
                || face.vertex_ids.len() != face.polygon.len()
                || face.vertex_ids.len() != face.boundary_edge_ids.len()
                || face
                    .polygon
                    .iter()
                    .any(|(x, z)| !x.is_finite() || !z.is_finite())
        })
    {
        return Err(RoadFaceError::InvalidEmbedding);
    }
    if matches!(
        cp.phase,
        Phase::FindMinimum
            | Phase::BuildFaceArrays
            | Phase::CanonicalBuild
            | Phase::CanonicalCompare
            | Phase::CanonicalDecide
            | Phase::ContainsProposed
            | Phase::EmitFace
    ) && (cp.walk_vertices.is_empty() || cp.minimum_index >= cp.walk_vertices.len())
    {
        return Err(RoadFaceError::InvalidEmbedding);
    }
    if cp.phase == Phase::WalkVisit && cp.half_edges.is_empty() {
        return Err(RoadFaceError::InvalidEmbedding);
    }
    if cp.phase == Phase::SortFaces {
        if cp.sort_index > cp.faces.len().max(1)
            || cp.sort_compare > cp.sort_index
            || cp.sort_compare_cursor > MAX_HALF_EDGES
        {
            return Err(RoadFaceError::InvalidEmbedding);
        }
    }
    if let Some(key) = &cp.sort_key {
        if cp.phase != Phase::SortFaces
            || cp.sort_index >= cp.faces.len()
            || key.identity.len() > MAX_HALF_EDGES
            || key.vertex_ids.len() != key.identity.len()
            || key.polygon.len() != key.identity.len()
            || key.boundary_edge_ids.len() != key.identity.len()
        {
            return Err(RoadFaceError::InvalidEmbedding);
        }
        // Insertion sort holds the key outside the array and exactly one
        // empty slot moves left as preceding faces shift right.
        if cp.faces.iter().filter(|face| is_empty_face(face)).count() != 1
            || cp.sort_compare >= cp.faces.len()
            || !is_empty_face(&cp.faces[cp.sort_compare])
        {
            return Err(RoadFaceError::InvalidEmbedding);
        }
    } else if cp.faces.iter().any(is_empty_face) {
        return Err(RoadFaceError::InvalidEmbedding);
    }
    if cp.phase == Phase::Nested {
        if cp.nested_direction > 1
            || cp.nested_first > cp.faces.len()
            || cp.nested_second > cp.faces.len()
        {
            return Err(RoadFaceError::InvalidEmbedding);
        }
        if cp.nested_first < cp.faces.len().saturating_sub(1) {
            if cp.nested_second <= cp.nested_first {
                return Err(RoadFaceError::InvalidEmbedding);
            }
            if cp.nested_second > cp.faces.len() {
                return Err(RoadFaceError::InvalidEmbedding);
            }
            if cp.nested_second < cp.faces.len() {
                if cp.faces[cp.nested_first].polygon.len() < 3
                    || cp.faces[cp.nested_second].polygon.len() < 3
                    || cp.nested_edge
                        > cp.faces[if cp.nested_direction == 0 {
                            cp.nested_second
                        } else {
                            cp.nested_first
                        }]
                        .polygon
                        .len()
                {
                    return Err(RoadFaceError::InvalidEmbedding);
                }
            }
        }
    }
    Ok(())
}

fn is_empty_face(face: &RoadFace) -> bool {
    face.identity.is_empty()
        && face.vertex_ids.is_empty()
        && face.polygon.is_empty()
        && face.boundary_edge_ids.is_empty()
        && face.area == 0.0
        && !face.contains_proposed_edge
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::road_face_bridges::BridgeSearch;
    use crate::legacy::road_face_topology::TopologySearch;
    use crate::legacy::road_faces::trace_faces;
    use serde_json;

    fn node(id: u32, x: f32, z: f32) -> RoadFaceNode {
        RoadFaceNode { id, x, z }
    }
    fn edge(id: u32, from: u32, to: u32) -> RoadFaceEdge {
        RoadFaceEdge { id, from, to }
    }

    fn rectangle_search() -> FaceWalkSearch {
        let nodes = vec![
            node(1, 0., 0.),
            node(2, 4., 0.),
            node(3, 4., 4.),
            node(4, 0., 4.),
        ];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3), edge(3, 3, 4), edge(4, 4, 1)];
        let half_edges = vec![
            HalfEdgeRecord {
                edge_index: 0,
                from: 0,
                to: 1,
                angle: 0.,
            },
            HalfEdgeRecord {
                edge_index: 0,
                from: 1,
                to: 0,
                angle: std::f32::consts::PI,
            },
            HalfEdgeRecord {
                edge_index: 1,
                from: 1,
                to: 2,
                angle: std::f32::consts::FRAC_PI_2,
            },
            HalfEdgeRecord {
                edge_index: 1,
                from: 2,
                to: 1,
                angle: -std::f32::consts::FRAC_PI_2,
            },
            HalfEdgeRecord {
                edge_index: 2,
                from: 2,
                to: 3,
                angle: std::f32::consts::PI,
            },
            HalfEdgeRecord {
                edge_index: 2,
                from: 3,
                to: 2,
                angle: 0.,
            },
            HalfEdgeRecord {
                edge_index: 3,
                from: 3,
                to: 0,
                angle: -std::f32::consts::FRAC_PI_2,
            },
            HalfEdgeRecord {
                edge_index: 3,
                from: 0,
                to: 3,
                angle: std::f32::consts::FRAC_PI_2,
            },
        ];
        FaceWalkSearch::new(nodes, edges, half_edges, vec![2, 7, 4, 1, 6, 3, 0, 5], None).unwrap()
    }

    fn advance_to(search: &mut FaceWalkSearch, target: Phase) {
        for _ in 0..1000 {
            if search.state.phase == target {
                return;
            }
            assert!(!matches!(
                search.state.phase,
                Phase::Done | Phase::Failed(_)
            ));
            search.advance(1);
        }
        panic!("phase {target:?} was not reached");
    }

    fn run(
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
        proposed: Option<RoadFaceEdge>,
        quotas: &[usize],
    ) -> Result<Vec<RoadFace>, RoadFaceError> {
        let expected = trace_faces(&nodes, &edges, proposed);
        let mut all_edges = edges.clone();
        if let Some(edge) = proposed {
            all_edges.push(edge);
        }
        let id_to_index = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id, i))
            .collect::<std::collections::BTreeMap<_, _>>();
        let indexed = all_edges
            .iter()
            .map(|edge| (id_to_index[&edge.from], id_to_index[&edge.to]))
            .collect::<Vec<_>>();
        let bridge_set = crate::legacy::road_faces::graph_bridge_edges(nodes.len(), &indexed);
        let bridges = (0..all_edges.len())
            .map(|i| bridge_set.contains(&i))
            .collect::<Vec<_>>();
        let mut bridge_search = BridgeSearch::new(nodes.len(), indexed.clone()).unwrap();
        while bridge_search.bridges().is_none() {
            bridge_search.advance(48);
        }
        let mut topology_search =
            TopologySearch::new(nodes.clone(), all_edges.clone(), indexed, bridges).unwrap();
        while topology_search.topology().is_none() {
            topology_search.advance(48);
        }
        let topology = topology_search.topology().unwrap();
        let mut search = FaceWalkSearch::new(
            nodes,
            all_edges,
            topology.half_edges.to_vec(),
            topology.next.to_vec(),
            proposed.map(|edge| edge.id),
        )
        .unwrap();
        let mut turn = 0;
        while search.faces().is_none() {
            let budget = quotas[turn % quotas.len()];
            let result = search.advance(budget);
            assert!(result.work_used <= budget);
            assert!(result.done || result.work_used > 0);
            let bytes = serde_json::to_vec(&search.checkpoint()).unwrap();
            let cp: Checkpoint = serde_json::from_slice(&bytes).unwrap();
            let checkpoint_phase = cp.phase;
            search = FaceWalkSearch::restore(cp).unwrap_or_else(|error| {
                panic!("restore failed in {checkpoint_phase:?}: {error:?}")
            });
            turn += 1;
            assert!(turn < 2_000_000);
        }
        let actual = search.faces().unwrap().map(|faces| faces.to_vec());
        assert_eq!(actual, expected);
        actual
    }

    #[test]
    fn results_match_reference_for_faces_spurs_and_proposed_marking() {
        let rect_nodes = vec![
            node(10, 0., 0.),
            node(20, 8., 0.),
            node(30, 8., 5.),
            node(40, 0., 5.),
        ];
        let rect_edges = vec![
            edge(1, 10, 20),
            edge(2, 20, 30),
            edge(3, 30, 40),
            edge(4, 40, 10),
        ];
        let proposed_nodes = rect_nodes.clone();
        let mut proposed_edges = rect_edges.clone();
        proposed_edges.pop();
        let proposed = edge(4, 40, 10);
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
        let dangling_nodes = vec![
            node(1, 0., 0.),
            node(2, 4., 0.),
            node(3, 4., 4.),
            node(4, 0., 4.),
            node(5, 2., 2.),
            node(6, 20., 0.),
            node(7, 22., 0.),
        ];
        let dangling_edges = vec![
            edge(1, 1, 2),
            edge(2, 2, 3),
            edge(3, 3, 4),
            edge(4, 4, 1),
            edge(5, 1, 5),
            edge(6, 6, 7),
        ];
        for (nodes, edges, proposal) in [
            (rect_nodes.clone(), rect_edges.clone(), None),
            (proposed_nodes, proposed_edges, Some(proposed)),
            (adjacent_nodes, adjacent_edges, None),
            (dangling_nodes, dangling_edges, None),
            (Vec::new(), Vec::new(), None),
        ] {
            for quotas in [&[1][..], &[7][..], &[48][..], &[3, 1, 9, 2][..]] {
                run(nodes.clone(), edges.clone(), proposal, quotas).unwrap();
            }
        }
    }

    #[test]
    fn nested_disconnected_cycles_match_reference_rejection() {
        let nodes = vec![
            node(10, 0., 0.),
            node(20, 10., 0.),
            node(30, 10., 10.),
            node(40, 0., 10.),
            node(50, 2., 2.),
            node(60, 4., 2.),
            node(70, 4., 4.),
            node(80, 2., 4.),
        ];
        let edges = vec![
            edge(1, 10, 20),
            edge(2, 20, 30),
            edge(3, 30, 40),
            edge(4, 40, 10),
            edge(5, 50, 60),
            edge(6, 60, 70),
            edge(7, 70, 80),
            edge(8, 80, 50),
        ];
        for quotas in [&[1][..], &[7][..], &[3, 1, 9, 2][..]] {
            assert_eq!(
                run(nodes.clone(), edges.clone(), None, quotas),
                Err(RoadFaceError::NestedFacesUnsupported)
            );
        }
    }

    #[test]
    fn malformed_active_cursors_are_rejected_without_indexing() {
        let nodes = vec![
            node(1, 0., 0.),
            node(2, 4., 0.),
            node(3, 4., 4.),
            node(4, 0., 4.),
        ];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3), edge(3, 3, 4), edge(4, 4, 1)];
        let indexed = vec![(0, 1), (1, 2), (2, 3), (3, 0)];
        let bridges = vec![false; 4];
        let mut topo = TopologySearch::new(nodes.clone(), edges.clone(), indexed, bridges).unwrap();
        while topo.topology().is_none() {
            topo.advance(48);
        }
        let topology = topo.topology().unwrap();
        let mut walk = FaceWalkSearch::new(
            nodes,
            edges,
            topology.half_edges.to_vec(),
            topology.next.to_vec(),
            None,
        )
        .unwrap();
        walk.advance(1);
        let mut bad = walk.checkpoint();
        bad.current = usize::MAX;
        assert_eq!(
            FaceWalkSearch::restore(bad).unwrap_err(),
            RoadFaceError::InvalidEmbedding
        );
        let mut bad = walk.checkpoint();
        bad.start_cursor = usize::MAX;
        assert_eq!(
            FaceWalkSearch::restore(bad).unwrap_err(),
            RoadFaceError::InvalidEmbedding
        );
    }

    #[test]
    fn restore_rejects_empty_active_walks_bad_minimum_indices_and_empty_nested_polygons() {
        let mut empty = FaceWalkSearch::new(Vec::new(), Vec::new(), Vec::new(), Vec::new(), None)
            .unwrap()
            .checkpoint();
        for phase in [
            Phase::WalkVisit,
            Phase::AppendVertex,
            Phase::AppendEdge,
            Phase::MoveNext,
        ] {
            empty.phase = phase;
            assert_eq!(
                FaceWalkSearch::restore(empty.clone()).unwrap_err(),
                RoadFaceError::InvalidEmbedding
            );
        }

        for phase in [Phase::FindMinimum, Phase::BuildFaceArrays] {
            let mut search = rectangle_search();
            advance_to(&mut search, phase);
            let mut bad = search.checkpoint();
            bad.minimum_index = usize::MAX;
            assert_eq!(
                FaceWalkSearch::restore(bad).unwrap_err(),
                RoadFaceError::InvalidEmbedding
            );
        }

        let nodes = vec![
            node(1, 0., 0.),
            node(2, 5., 0.),
            node(3, 10., 0.),
            node(4, 0., 4.),
            node(5, 5., 4.),
            node(6, 10., 4.),
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
        let indexed = vec![(0, 1), (1, 2), (2, 5), (5, 4), (4, 3), (3, 0), (1, 4)];
        let bridges = vec![false; edges.len()];
        let mut topo = TopologySearch::new(nodes.clone(), edges.clone(), indexed, bridges).unwrap();
        while topo.topology().is_none() {
            topo.advance(48);
        }
        let topology = topo.topology().unwrap();
        let mut search = FaceWalkSearch::new(
            nodes,
            edges,
            topology.half_edges.to_vec(),
            topology.next.to_vec(),
            None,
        )
        .unwrap();
        advance_to(&mut search, Phase::Nested);
        assert_eq!(search.state.faces.len(), 2);
        let mut bad = search.checkpoint();
        bad.faces[1].polygon.clear();
        assert_eq!(
            FaceWalkSearch::restore(bad).unwrap_err(),
            RoadFaceError::InvalidEmbedding
        );
    }

    #[test]
    fn initialize_checkpoint_rejects_scratch_that_would_survive_initialization() {
        let nodes = vec![
            node(1, 0., 0.),
            node(2, 4., 0.),
            node(3, 4., 4.),
            node(4, 0., 4.),
        ];
        let edges = vec![edge(1, 1, 2), edge(2, 2, 3), edge(3, 3, 4), edge(4, 4, 1)];
        let mut initial = FaceWalkSearch::new(
            nodes,
            edges,
            vec![
                HalfEdgeRecord {
                    edge_index: 0,
                    from: 0,
                    to: 1,
                    angle: 0.,
                },
                HalfEdgeRecord {
                    edge_index: 0,
                    from: 1,
                    to: 0,
                    angle: std::f32::consts::PI,
                },
            ],
            vec![1, 0],
            None,
        )
        .unwrap()
        .checkpoint();
        initial.phase = Phase::Initialize;
        initial.initialize_cursor = 0;
        initial.visited.clear();
        initial.seen_marks.clear();

        let mut bad = initial.clone();
        bad.walk_epoch = 1;
        assert_eq!(
            FaceWalkSearch::restore(bad).unwrap_err(),
            RoadFaceError::InvalidEmbedding
        );
        let mut bad = initial.clone();
        bad.current = 1;
        assert_eq!(
            FaceWalkSearch::restore(bad).unwrap_err(),
            RoadFaceError::InvalidEmbedding
        );
        let mut bad = initial;
        bad.minimum_index = 1;
        assert_eq!(
            FaceWalkSearch::restore(bad).unwrap_err(),
            RoadFaceError::InvalidEmbedding
        );
    }
}
