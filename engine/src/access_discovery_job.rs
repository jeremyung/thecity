//! Persisted, quota-stepped composition of frozen street-face discovery.
//!
//! This job deliberately stops at ordered access descriptors. Capturing a live
//! engine snapshot, evaluating/admitting a proposal, and changing simulation
//! state belong to the caller. Constructor/restore validation is synchronous
//! but strictly capped; all cross-stage variable-size transfers and stage work
//! are charged by `advance`.

use crate::access_candidates::{
    BoundaryHost, CandidateNode, MAX_PREPARED_INPUT_NODES, MAX_PREPARED_INPUT_PARCELS,
};
use crate::access_descriptor_search::{DescriptorCheckpoint, DescriptorSearch};
use crate::face_target_ranking::{Checkpoint as RankingCheckpoint, TargetRanking};
use crate::face_target_selection::{FaceSelection, FaceSelectionCheckpoint};
use crate::face_target_viability::{
    FrozenViabilityContext, FrozenViabilityInput, Segment, ViabilityError,
};
use crate::geometry::OrientedRect;
use crate::interior_access::AccessGeometryCandidate;
use crate::road_face_trace::{self, RoadFaceTrace};
use crate::road_faces::{RoadFace, RoadFaceEdge, RoadFaceError, RoadFaceNode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 2;
const MAX_GRAPH_NODES: usize = 512;
const MAX_GRAPH_EDGES: usize = 512;
const MAX_CANDIDATE_NODES: usize = MAX_PREPARED_INPUT_NODES;
const MAX_DESCRIPTORS: usize = crate::access_candidates::MAX_HOSTS_PER_FACE
    * crate::access_candidates::MAX_SPLITS_PER_HOST
    * (1 + crate::access_candidates::MAX_TARGETS_PER_SPLIT * 3 + 4 * 4 * 2)
    * 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DiscoveryInput {
    pub graph_nodes: Vec<RoadFaceNode>,
    pub graph_edges: Vec<RoadFaceEdge>,
    pub all_nodes: Vec<CandidateNode>,
    pub viability: FrozenViabilityInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiscoveryError {
    InputTooLarge,
    InvalidInput,
    Trace(RoadFaceError),
    Selection,
    Ranking,
    Descriptor,
    InvalidCheckpoint,
}

impl From<ViabilityError> for DiscoveryError {
    fn from(value: ViabilityError) -> Self {
        match value {
            ViabilityError::InputTooLarge => Self::InputTooLarge,
            ViabilityError::NonFiniteInput => Self::InvalidInput,
            ViabilityError::InvalidCheckpoint => Self::InvalidCheckpoint,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DiscoveryAdvance {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum CopyKind {
    Parcels,
    Roads,
    Nodes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum HostJoinPhase {
    Edge,
    FromNode,
    ToNode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum StageCheckpoint {
    CopyGraph {
        graph_nodes: Vec<RoadFaceNode>,
        graph_edges: Vec<RoadFaceEdge>,
        cursor: usize,
        retained_edges: Vec<RoadFaceEdge>,
    },
    Trace {
        trace: road_face_trace::Checkpoint,
        retained_edges: Vec<RoadFaceEdge>,
    },
    Selection {
        selection: FaceSelectionCheckpoint,
        retained_edges: Vec<RoadFaceEdge>,
    },
    CopyRankInputs {
        face: RoadFace,
        centers: Vec<(f32, f32)>,
        kind: CopyKind,
        cursor: usize,
        parcels: Vec<OrientedRect>,
        roads: Vec<Segment>,
        nodes: Vec<(f32, f32)>,
        retained_edges: Vec<RoadFaceEdge>,
    },
    Ranking {
        face: RoadFace,
        ranking: RankingCheckpoint,
        retained_edges: Vec<RoadFaceEdge>,
    },
    BuildDescriptors {
        face: RoadFace,
        ranking: RankingCheckpoint,
        retained_edges: Vec<RoadFaceEdge>,
        hosts: Vec<BoundaryHost>,
        nodes: Vec<CandidateNode>,
        parcels: Vec<OrientedRect>,
        targets: Vec<(f32, f32)>,
        copy_kind: CopyKind,
        cursor: usize,
        boundary_index: usize,
        host_phase: HostJoinPhase,
        edge_scan: usize,
        node_scan: usize,
        matched_edge: Option<RoadFaceEdge>,
        from_node: Option<CandidateNode>,
    },
    Descriptors(DescriptorCheckpoint),
    Done(Vec<AccessGeometryCandidate>),
    Failed(DiscoveryError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DiscoveryCheckpoint {
    schema: u8,
    all_nodes: Vec<CandidateNode>,
    viability: FrozenViabilityInput,
    stage: StageCheckpoint,
}

impl DiscoveryCheckpoint {
    pub(crate) fn matches_viability(&self, input: &FrozenViabilityInput) -> bool {
        &self.viability == input
    }
}

#[derive(Debug)]
enum Stage {
    CopyGraph {
        graph_nodes: Vec<RoadFaceNode>,
        graph_edges: Vec<RoadFaceEdge>,
        cursor: usize,
        retained_edges: Vec<RoadFaceEdge>,
    },
    Trace {
        trace: RoadFaceTrace,
        retained_edges: Vec<RoadFaceEdge>,
    },
    Selection {
        selection: FaceSelection,
        retained_edges: Vec<RoadFaceEdge>,
    },
    CopyRankInputs {
        face: RoadFace,
        centers: Vec<(f32, f32)>,
        kind: CopyKind,
        cursor: usize,
        parcels: Vec<OrientedRect>,
        roads: Vec<Segment>,
        nodes: Vec<(f32, f32)>,
        retained_edges: Vec<RoadFaceEdge>,
    },
    Ranking {
        face: RoadFace,
        ranking: TargetRanking,
        retained_edges: Vec<RoadFaceEdge>,
    },
    BuildDescriptors {
        face: RoadFace,
        ranking: TargetRanking,
        retained_edges: Vec<RoadFaceEdge>,
        hosts: Vec<BoundaryHost>,
        nodes: Vec<CandidateNode>,
        parcels: Vec<OrientedRect>,
        targets: Vec<(f32, f32)>,
        copy_kind: CopyKind,
        cursor: usize,
        boundary_index: usize,
        host_phase: HostJoinPhase,
        edge_scan: usize,
        node_scan: usize,
        matched_edge: Option<RoadFaceEdge>,
        from_node: Option<CandidateNode>,
    },
    Descriptors(DescriptorSearch),
    Done(Vec<AccessGeometryCandidate>),
    Failed(DiscoveryError),
}

/// Runtime job. Deep checkpoint copies are made only when explicitly asked;
/// `advance` never serializes or recursively checkpoints a nested stage.
#[derive(Debug)]
pub(crate) struct DiscoveryJob {
    all_nodes: Vec<CandidateNode>,
    viability: Arc<FrozenViabilityContext>,
    stage: Stage,
}

impl DiscoveryJob {
    /// Fixed-size/count validation is synchronous and bounded. The graph and
    /// candidate caps match the constituent trace/descriptor stages.
    pub(crate) fn new(input: DiscoveryInput) -> Result<Self, DiscoveryError> {
        validate_nodes(&input.all_nodes)?;
        if input.graph_nodes.len() > MAX_GRAPH_NODES
            || input.graph_edges.len() > MAX_GRAPH_EDGES
            || input.viability.parcel_footprints.len() > MAX_PREPARED_INPUT_PARCELS
        {
            return Err(DiscoveryError::InputTooLarge);
        }
        if !valid_graph(&input.graph_nodes, &input.graph_edges)
            || !valid_retained_edges(&input.graph_edges, &input.all_nodes)
        {
            return Err(DiscoveryError::InvalidInput);
        }
        let viability = FrozenViabilityContext::new(input.viability)?;
        let edge_capacity = input.graph_edges.len();
        Ok(Self {
            all_nodes: input.all_nodes,
            viability,
            stage: Stage::CopyGraph {
                graph_nodes: input.graph_nodes,
                graph_edges: input.graph_edges,
                cursor: 0,
                retained_edges: Vec::with_capacity(edge_capacity),
            },
        })
    }

    pub(crate) fn restore(checkpoint: DiscoveryCheckpoint) -> Result<Self, DiscoveryError> {
        if checkpoint.schema != CHECKPOINT_SCHEMA {
            return Err(DiscoveryError::InvalidCheckpoint);
        }
        validate_nodes(&checkpoint.all_nodes).map_err(|_| DiscoveryError::InvalidCheckpoint)?;
        if checkpoint.viability.parcel_footprints.len() > MAX_PREPARED_INPUT_PARCELS {
            return Err(DiscoveryError::InvalidCheckpoint);
        }
        let viability = FrozenViabilityContext::restore(checkpoint.viability)?;
        let mut stage = match checkpoint.stage {
            StageCheckpoint::CopyGraph {
                graph_nodes,
                graph_edges,
                cursor,
                retained_edges,
            } => {
                if graph_nodes.len() > MAX_GRAPH_NODES
                    || graph_edges.len() > MAX_GRAPH_EDGES
                    || cursor > graph_edges.len()
                    || retained_edges.len() != cursor
                    || retained_edges.as_slice() != &graph_edges[..cursor]
                    || !valid_graph(&graph_nodes, &graph_edges)
                    || !valid_retained_edges(&graph_edges, &checkpoint.all_nodes)
                {
                    return Err(DiscoveryError::InvalidCheckpoint);
                }
                Stage::CopyGraph {
                    graph_nodes,
                    graph_edges,
                    cursor,
                    retained_edges,
                }
            }
            StageCheckpoint::Trace {
                trace,
                retained_edges,
            } => {
                if !valid_retained_edges(&retained_edges, &checkpoint.all_nodes) {
                    return Err(DiscoveryError::InvalidCheckpoint);
                }
                Stage::Trace {
                    trace: RoadFaceTrace::restore(trace)
                        .map_err(|_| DiscoveryError::InvalidCheckpoint)?,
                    retained_edges,
                }
            }
            StageCheckpoint::Selection {
                selection,
                retained_edges,
            } => {
                if !valid_retained_edges(&retained_edges, &checkpoint.all_nodes) {
                    return Err(DiscoveryError::InvalidCheckpoint);
                }
                Stage::Selection {
                    selection: FaceSelection::restore(selection, viability.clone())
                        .map_err(|_| DiscoveryError::InvalidCheckpoint)?,
                    retained_edges,
                }
            }
            StageCheckpoint::CopyRankInputs {
                face,
                centers,
                kind,
                cursor,
                parcels,
                roads,
                nodes,
                retained_edges,
            } => {
                validate_copy_rank(
                    &face,
                    &centers,
                    kind,
                    cursor,
                    &parcels,
                    &roads,
                    &nodes,
                    &checkpoint.all_nodes,
                    &viability,
                    &retained_edges,
                )?;
                Stage::CopyRankInputs {
                    face,
                    centers,
                    kind,
                    cursor,
                    parcels,
                    roads,
                    nodes,
                    retained_edges,
                }
            }
            StageCheckpoint::Ranking {
                face,
                ranking,
                retained_edges,
            } => {
                if !valid_retained_edges(&retained_edges, &checkpoint.all_nodes)
                    || !valid_face(&face)
                {
                    return Err(DiscoveryError::InvalidCheckpoint);
                }
                Stage::Ranking {
                    face,
                    ranking: TargetRanking::restore(ranking)
                        .map_err(|_| DiscoveryError::InvalidCheckpoint)?,
                    retained_edges,
                }
            }
            StageCheckpoint::BuildDescriptors {
                face,
                ranking,
                retained_edges,
                hosts,
                nodes,
                parcels,
                targets,
                copy_kind,
                cursor,
                boundary_index,
                host_phase,
                edge_scan,
                node_scan,
                matched_edge,
                from_node,
            } => {
                let ranking = TargetRanking::restore(ranking)
                    .map_err(|_| DiscoveryError::InvalidCheckpoint)?;
                validate_descriptor_build(
                    &face,
                    &hosts,
                    &nodes,
                    &parcels,
                    &targets,
                    copy_kind,
                    cursor,
                    boundary_index,
                    host_phase,
                    edge_scan,
                    node_scan,
                    matched_edge,
                    from_node,
                    &ranking,
                    &checkpoint.all_nodes,
                    &viability,
                    &retained_edges,
                )?;
                Stage::BuildDescriptors {
                    face,
                    ranking,
                    retained_edges,
                    hosts,
                    nodes,
                    parcels,
                    targets,
                    copy_kind,
                    cursor,
                    boundary_index,
                    host_phase,
                    edge_scan,
                    node_scan,
                    matched_edge,
                    from_node,
                }
            }
            StageCheckpoint::Descriptors(search) => Stage::Descriptors(
                DescriptorSearch::restore(search).map_err(|_| DiscoveryError::InvalidCheckpoint)?,
            ),
            StageCheckpoint::Done(output) => {
                if output.len() > MAX_DESCRIPTORS
                    || output.iter().any(|item| {
                        !item.host_t.is_finite()
                            || !(0.0..=1.0).contains(&item.host_t)
                            || !item.end_x.is_finite()
                            || !item.end_z.is_finite()
                    })
                {
                    return Err(DiscoveryError::InvalidCheckpoint);
                }
                Stage::Done(output)
            }
            StageCheckpoint::Failed(error) => Stage::Failed(error),
        };
        reserve_stage(&mut stage, &checkpoint.all_nodes, viability.input());
        Ok(Self {
            all_nodes: checkpoint.all_nodes,
            viability,
            stage,
        })
    }

    pub(crate) fn checkpoint(&self) -> DiscoveryCheckpoint {
        let stage = match &self.stage {
            Stage::CopyGraph {
                graph_nodes,
                graph_edges,
                cursor,
                retained_edges,
            } => StageCheckpoint::CopyGraph {
                graph_nodes: graph_nodes.clone(),
                graph_edges: graph_edges.clone(),
                cursor: *cursor,
                retained_edges: retained_edges.clone(),
            },
            Stage::Trace {
                trace,
                retained_edges,
            } => StageCheckpoint::Trace {
                trace: trace.checkpoint(),
                retained_edges: retained_edges.clone(),
            },
            Stage::Selection {
                selection,
                retained_edges,
            } => StageCheckpoint::Selection {
                selection: selection.checkpoint(),
                retained_edges: retained_edges.clone(),
            },
            Stage::CopyRankInputs {
                face,
                centers,
                kind,
                cursor,
                parcels,
                roads,
                nodes,
                retained_edges,
            } => StageCheckpoint::CopyRankInputs {
                face: face.clone(),
                centers: centers.clone(),
                kind: *kind,
                cursor: *cursor,
                parcels: parcels.clone(),
                roads: roads.clone(),
                nodes: nodes.clone(),
                retained_edges: retained_edges.clone(),
            },
            Stage::Ranking {
                face,
                ranking,
                retained_edges,
            } => StageCheckpoint::Ranking {
                face: face.clone(),
                ranking: ranking.checkpoint(),
                retained_edges: retained_edges.clone(),
            },
            Stage::BuildDescriptors {
                face,
                ranking,
                retained_edges,
                hosts,
                nodes,
                parcels,
                targets,
                copy_kind,
                cursor,
                boundary_index,
                host_phase,
                edge_scan,
                node_scan,
                matched_edge,
                from_node,
            } => StageCheckpoint::BuildDescriptors {
                face: face.clone(),
                ranking: ranking.checkpoint(),
                retained_edges: retained_edges.clone(),
                hosts: hosts.clone(),
                nodes: nodes.clone(),
                parcels: parcels.clone(),
                targets: targets.clone(),
                copy_kind: *copy_kind,
                cursor: *cursor,
                boundary_index: *boundary_index,
                host_phase: *host_phase,
                edge_scan: *edge_scan,
                node_scan: *node_scan,
                matched_edge: *matched_edge,
                from_node: *from_node,
            },
            Stage::Descriptors(search) => StageCheckpoint::Descriptors(search.checkpoint()),
            Stage::Done(output) => StageCheckpoint::Done(output.clone()),
            Stage::Failed(error) => StageCheckpoint::Failed(*error),
        };
        DiscoveryCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            all_nodes: self.all_nodes.clone(),
            viability: self.viability.checkpoint(),
            stage,
        }
    }

    pub(crate) fn descriptors(&self) -> Option<Result<&[AccessGeometryCandidate], DiscoveryError>> {
        match &self.stage {
            Stage::Done(output) => Some(Ok(output)),
            Stage::Failed(error) => Some(Err(*error)),
            Stage::Descriptors(search) => search.descriptors().map(Ok),
            _ => None,
        }
    }

    pub(crate) fn advance(&mut self, budget: usize) -> DiscoveryAdvance {
        if budget == 0 || self.descriptors().is_some() {
            return DiscoveryAdvance {
                work_used: 0,
                done: self.descriptors().is_some(),
            };
        }
        let mut used = 0;
        while used < budget && self.descriptors().is_none() {
            match &mut self.stage {
                Stage::CopyGraph {
                    graph_nodes,
                    graph_edges,
                    cursor,
                    retained_edges,
                } => {
                    if *cursor < graph_edges.len() {
                        retained_edges.push(graph_edges[*cursor]);
                        *cursor += 1;
                        used += 1;
                    } else {
                        let nodes = std::mem::take(graph_nodes);
                        let edges = std::mem::take(graph_edges);
                        let retained = std::mem::take(retained_edges);
                        match RoadFaceTrace::new(nodes, edges, None) {
                            Ok(trace) => {
                                self.stage = Stage::Trace {
                                    trace,
                                    retained_edges: retained,
                                }
                            }
                            Err(error) => self.stage = Stage::Failed(DiscoveryError::Trace(error)),
                        }
                        used += 1;
                    }
                }
                Stage::Trace { trace, .. } => {
                    if trace.faces().is_some() {
                        let old = std::mem::replace(
                            &mut self.stage,
                            Stage::Failed(DiscoveryError::InvalidInput),
                        );
                        if let Stage::Trace {
                            trace,
                            retained_edges,
                        } = old
                        {
                            match trace.into_faces().expect("trace completion was checked") {
                                Err(error) => {
                                    self.stage = Stage::Failed(DiscoveryError::Trace(error))
                                }
                                Ok(faces) => {
                                    match FaceSelection::new(faces, self.viability.clone()) {
                                        Ok(selection) => {
                                            self.stage = Stage::Selection {
                                                selection,
                                                retained_edges,
                                            }
                                        }
                                        Err(_) => {
                                            self.stage = Stage::Failed(DiscoveryError::Selection)
                                        }
                                    }
                                }
                            }
                        }
                        used += 1;
                    } else {
                        let step = trace.advance(budget - used);
                        used += step.work_used;
                    }
                }
                Stage::Selection { selection, .. } => {
                    if selection.error().is_some() {
                        self.stage = Stage::Failed(DiscoveryError::Selection);
                        used += 1;
                    } else if selection.selection().is_some() {
                        let old = std::mem::replace(
                            &mut self.stage,
                            Stage::Failed(DiscoveryError::InvalidInput),
                        );
                        if let Stage::Selection {
                            selection,
                            retained_edges,
                        } = old
                        {
                            match selection.into_selected() {
                                Err(_) => self.stage = Stage::Failed(DiscoveryError::Selection),
                                Ok(None) => self.stage = Stage::Done(Vec::new()),
                                Ok(Some((face, centers))) => {
                                    self.stage = Stage::CopyRankInputs {
                                        face,
                                        centers,
                                        kind: CopyKind::Parcels,
                                        cursor: 0,
                                        parcels: Vec::with_capacity(
                                            self.viability.input().parcel_footprints.len(),
                                        ),
                                        roads: Vec::with_capacity(
                                            self.viability.input().open_roads.len(),
                                        ),
                                        nodes: Vec::with_capacity(self.all_nodes.len()),
                                        retained_edges,
                                    }
                                }
                            }
                        }
                        used += 1;
                    } else {
                        let step = selection.advance(budget - used);
                        used += step.work_used;
                    }
                }
                Stage::CopyRankInputs {
                    kind,
                    cursor,
                    parcels,
                    roads,
                    nodes,
                    ..
                } => {
                    let source = self.viability.input();
                    match kind {
                        CopyKind::Parcels if *cursor < source.parcel_footprints.len() => {
                            parcels.push(source.parcel_footprints[*cursor]);
                            *cursor += 1;
                            used += 1;
                        }
                        CopyKind::Parcels => {
                            *kind = CopyKind::Roads;
                            *cursor = 0;
                            used += 1;
                        }
                        CopyKind::Roads if *cursor < source.open_roads.len() => {
                            roads.push(source.open_roads[*cursor]);
                            *cursor += 1;
                            used += 1;
                        }
                        CopyKind::Roads => {
                            *kind = CopyKind::Nodes;
                            *cursor = 0;
                            used += 1;
                        }
                        CopyKind::Nodes if *cursor < self.all_nodes.len() => {
                            let n = self.all_nodes[*cursor];
                            nodes.push((n.x, n.z));
                            *cursor += 1;
                            used += 1;
                        }
                        CopyKind::Nodes => {
                            let old = std::mem::replace(
                                &mut self.stage,
                                Stage::Failed(DiscoveryError::InvalidInput),
                            );
                            if let Stage::CopyRankInputs {
                                face,
                                centers,
                                parcels,
                                roads,
                                nodes,
                                retained_edges,
                                ..
                            } = old
                            {
                                match TargetRanking::new(centers, parcels, roads, nodes) {
                                    Ok(ranking) => {
                                        self.stage = Stage::Ranking {
                                            face,
                                            ranking,
                                            retained_edges,
                                        }
                                    }
                                    Err(_) => self.stage = Stage::Failed(DiscoveryError::Ranking),
                                }
                            }
                            used += 1;
                        }
                    }
                }
                Stage::Ranking { ranking, .. } => {
                    if ranking.targets().is_some() {
                        let old = std::mem::replace(
                            &mut self.stage,
                            Stage::Failed(DiscoveryError::InvalidInput),
                        );
                        if let Stage::Ranking {
                            face,
                            ranking,
                            retained_edges,
                        } = old
                        {
                            let target_count = ranking.targets().map_or(0, |targets| targets.len());
                            let host_count = face.boundary_edge_ids.len();
                            self.stage = Stage::BuildDescriptors {
                                face,
                                ranking,
                                retained_edges,
                                hosts: Vec::with_capacity(host_count),
                                nodes: Vec::with_capacity(self.all_nodes.len()),
                                parcels: Vec::with_capacity(
                                    self.viability.input().parcel_footprints.len(),
                                ),
                                targets: Vec::with_capacity(target_count),
                                copy_kind: CopyKind::Parcels,
                                cursor: 0,
                                boundary_index: 0,
                                host_phase: HostJoinPhase::Edge,
                                edge_scan: 0,
                                node_scan: 0,
                                matched_edge: None,
                                from_node: None,
                            };
                        }
                        used += 1;
                    } else {
                        let step = ranking.advance(budget - used);
                        used += step.work_used;
                    }
                }
                Stage::BuildDescriptors {
                    face,
                    ranking,
                    retained_edges,
                    hosts,
                    nodes,
                    parcels,
                    targets,
                    copy_kind,
                    cursor,
                    boundary_index,
                    host_phase,
                    edge_scan,
                    node_scan,
                    matched_edge,
                    from_node,
                } => {
                    if *boundary_index < face.boundary_edge_ids.len() {
                        match host_phase {
                            HostJoinPhase::Edge => {
                                if *edge_scan >= retained_edges.len() {
                                    self.stage = Stage::Failed(DiscoveryError::InvalidInput);
                                    used += 1;
                                } else {
                                    let edge = retained_edges[*edge_scan];
                                    *edge_scan += 1;
                                    used += 1;
                                    if edge.id == face.boundary_edge_ids[*boundary_index] {
                                        *matched_edge = Some(edge);
                                        *host_phase = HostJoinPhase::FromNode;
                                        *node_scan = 0;
                                    }
                                }
                            }
                            HostJoinPhase::FromNode => {
                                let edge =
                                    matched_edge.expect("edge matched before endpoint lookup");
                                if *node_scan >= self.all_nodes.len() {
                                    self.stage = Stage::Failed(DiscoveryError::InvalidInput);
                                    used += 1;
                                } else {
                                    let node = self.all_nodes[*node_scan];
                                    *node_scan += 1;
                                    used += 1;
                                    if node.id == edge.from {
                                        *from_node = Some(node);
                                        *host_phase = HostJoinPhase::ToNode;
                                        *node_scan = 0;
                                    }
                                }
                            }
                            HostJoinPhase::ToNode => {
                                let edge =
                                    matched_edge.expect("edge matched before endpoint lookup");
                                if *node_scan >= self.all_nodes.len() {
                                    self.stage = Stage::Failed(DiscoveryError::InvalidInput);
                                    used += 1;
                                } else {
                                    let node = self.all_nodes[*node_scan];
                                    *node_scan += 1;
                                    used += 1;
                                    if node.id == edge.to {
                                        let a = from_node.expect("from node matched");
                                        hosts.push(BoundaryHost {
                                            road_id: edge.id,
                                            from: edge.from,
                                            to: edge.to,
                                            ax: a.x,
                                            az: a.z,
                                            bx: node.x,
                                            bz: node.z,
                                        });
                                        *boundary_index += 1;
                                        *edge_scan = 0;
                                        *node_scan = 0;
                                        *host_phase = HostJoinPhase::Edge;
                                        *matched_edge = None;
                                        *from_node = None;
                                    }
                                }
                            }
                        }
                    } else {
                        let viability = self.viability.input();
                        match copy_kind {
                            CopyKind::Parcels if *cursor < viability.parcel_footprints.len() => {
                                parcels.push(viability.parcel_footprints[*cursor]);
                                *cursor += 1;
                                used += 1;
                            }
                            CopyKind::Parcels => {
                                *copy_kind = CopyKind::Roads;
                                *cursor = 0;
                                used += 1;
                            }
                            CopyKind::Roads if *cursor < self.all_nodes.len() => {
                                let n = self.all_nodes[*cursor];
                                nodes.push(n);
                                *cursor += 1;
                                used += 1;
                            }
                            CopyKind::Roads => {
                                *copy_kind = CopyKind::Nodes;
                                *cursor = 0;
                                used += 1;
                            }
                            CopyKind::Nodes if *cursor < ranking.targets().unwrap().len() => {
                                targets.push(ranking.targets().unwrap()[*cursor]);
                                *cursor += 1;
                                used += 1;
                            }
                            CopyKind::Nodes => {
                                let old = std::mem::replace(
                                    &mut self.stage,
                                    Stage::Failed(DiscoveryError::InvalidInput),
                                );
                                if let Stage::BuildDescriptors {
                                    face,
                                    hosts,
                                    nodes,
                                    parcels,
                                    targets,
                                    ..
                                } = old
                                {
                                    match DescriptorSearch::from_owned(
                                        face, hosts, nodes, parcels, targets,
                                    ) {
                                        Ok(search) => self.stage = Stage::Descriptors(search),
                                        Err(_) => {
                                            self.stage = Stage::Failed(DiscoveryError::Descriptor)
                                        }
                                    }
                                }
                                used += 1;
                            }
                        }
                    }
                }
                Stage::Descriptors(search) => {
                    let step = search.advance(budget - used);
                    used += step.work_used;
                }
                Stage::Done(_) | Stage::Failed(_) => break,
            }
        }
        assert!(used <= budget, "discovery stage exceeded caller work quota");
        DiscoveryAdvance {
            work_used: used,
            done: self.descriptors().is_some(),
        }
    }
}

fn validate_nodes(nodes: &[CandidateNode]) -> Result<(), DiscoveryError> {
    if nodes.len() > MAX_CANDIDATE_NODES {
        return Err(DiscoveryError::InputTooLarge);
    }
    if nodes.iter().any(|n| {
        !n.x.is_finite() || !n.z.is_finite() || n.x.abs() > 100_000.0 || n.z.abs() > 100_000.0
    }) {
        return Err(DiscoveryError::InvalidInput);
    }
    let mut ids = std::collections::BTreeSet::new();
    if nodes.iter().any(|node| !ids.insert(node.id)) {
        return Err(DiscoveryError::InvalidInput);
    }
    Ok(())
}

fn valid_graph(nodes: &[RoadFaceNode], edges: &[RoadFaceEdge]) -> bool {
    if nodes.len() > MAX_GRAPH_NODES
        || edges.len() > MAX_GRAPH_EDGES
        || nodes.iter().any(|n| !n.x.is_finite() || !n.z.is_finite())
    {
        return false;
    }
    let mut node_ids = std::collections::BTreeSet::new();
    let mut edge_ids = std::collections::BTreeSet::new();
    nodes.iter().all(|n| node_ids.insert(n.id))
        && edges.iter().all(|e| {
            edge_ids.insert(e.id) && node_ids.contains(&e.from) && node_ids.contains(&e.to)
        })
}

fn valid_retained_edges(edges: &[RoadFaceEdge], all_nodes: &[CandidateNode]) -> bool {
    if edges.len() > MAX_GRAPH_EDGES {
        return false;
    }
    let node_ids = all_nodes
        .iter()
        .map(|node| node.id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut edge_ids = std::collections::BTreeSet::new();
    edges.iter().all(|edge| {
        edge_ids.insert(edge.id) && node_ids.contains(&edge.from) && node_ids.contains(&edge.to)
    })
}

fn valid_face(face: &RoadFace) -> bool {
    face.vertex_ids.len() >= 3
        && face.vertex_ids.len() <= 1024
        && face.identity.len() >= 3
        && face.identity.len() <= 1024
        && face.boundary_edge_ids.len() == face.vertex_ids.len()
        && face.polygon.len() == face.vertex_ids.len()
        && face
            .polygon
            .iter()
            .all(|(x, z)| x.is_finite() && z.is_finite())
        && face.area.is_finite()
        && face.area > 0.0
}

/// Restored vectors arrive with capacity equal to their serialized length.
/// Reserve only within source caps here so subsequent advance pushes never
/// relocate a previously copied prefix outside the logical work budget.
fn reserve_stage(stage: &mut Stage, all_nodes: &[CandidateNode], viability: &FrozenViabilityInput) {
    match stage {
        Stage::CopyGraph {
            graph_edges,
            retained_edges,
            ..
        } => retained_edges.reserve_exact(graph_edges.len().saturating_sub(retained_edges.len())),
        Stage::CopyRankInputs {
            parcels,
            roads,
            nodes,
            ..
        } => {
            parcels.reserve_exact(
                viability
                    .parcel_footprints
                    .len()
                    .saturating_sub(parcels.len()),
            );
            roads.reserve_exact(viability.open_roads.len().saturating_sub(roads.len()));
            nodes.reserve_exact(all_nodes.len().saturating_sub(nodes.len()));
        }
        Stage::BuildDescriptors {
            face,
            hosts,
            nodes,
            parcels,
            targets,
            ranking,
            ..
        } => {
            hosts.reserve_exact(face.boundary_edge_ids.len().saturating_sub(hosts.len()));
            nodes.reserve_exact(all_nodes.len().saturating_sub(nodes.len()));
            parcels.reserve_exact(
                viability
                    .parcel_footprints
                    .len()
                    .saturating_sub(parcels.len()),
            );
            if let Some(all_targets) = ranking.targets() {
                targets.reserve_exact(all_targets.len().saturating_sub(targets.len()));
            }
        }
        _ => {}
    }
}

fn validate_copy_rank(
    face: &RoadFace,
    centers: &[(f32, f32)],
    kind: CopyKind,
    cursor: usize,
    parcels: &[OrientedRect],
    roads: &[Segment],
    nodes: &[(f32, f32)],
    all_nodes: &[CandidateNode],
    viability: &FrozenViabilityContext,
    edges: &[RoadFaceEdge],
) -> Result<(), DiscoveryError> {
    let source = viability.input();
    let cursor_matches = match kind {
        CopyKind::Parcels => {
            cursor == parcels.len()
                && roads.is_empty()
                && nodes.is_empty()
                && source.parcel_footprints.get(..parcels.len()) == Some(parcels)
        }
        CopyKind::Roads => {
            parcels.len() == source.parcel_footprints.len()
                && cursor == roads.len()
                && nodes.is_empty()
                && parcels == source.parcel_footprints
                && source.open_roads.get(..roads.len()) == Some(roads)
        }
        CopyKind::Nodes => {
            parcels.len() == source.parcel_footprints.len()
                && roads.len() == source.open_roads.len()
                && cursor == nodes.len()
                && parcels == source.parcel_footprints
                && roads == source.open_roads
                && nodes.len() <= all_nodes.len()
                && nodes.iter().zip(all_nodes).all(|(a, b)| *a == (b.x, b.z))
        }
    };
    if !valid_face(face)
        || edges.len() > MAX_GRAPH_EDGES
        || face.boundary_edge_ids.len() > MAX_GRAPH_EDGES
        || centers.len() > 576
        || centers
            .iter()
            .any(|(x, z)| !x.is_finite() || !z.is_finite())
        || cursor
            > match kind {
                CopyKind::Parcels => source.parcel_footprints.len(),
                CopyKind::Roads => source.open_roads.len(),
                CopyKind::Nodes => all_nodes.len(),
            }
        || parcels.len() > source.parcel_footprints.len()
        || roads.len() > source.open_roads.len()
        || nodes.len() > all_nodes.len()
        || !cursor_matches
    {
        return Err(DiscoveryError::InvalidCheckpoint);
    }
    Ok(())
}

fn validate_descriptor_build(
    face: &RoadFace,
    hosts: &[BoundaryHost],
    nodes: &[CandidateNode],
    parcels: &[OrientedRect],
    targets: &[(f32, f32)],
    kind: CopyKind,
    cursor: usize,
    boundary: usize,
    host_phase: HostJoinPhase,
    edge_scan: usize,
    node_scan: usize,
    matched: Option<RoadFaceEdge>,
    from: Option<CandidateNode>,
    ranking: &TargetRanking,
    all_nodes: &[CandidateNode],
    viability: &FrozenViabilityContext,
    edges: &[RoadFaceEdge],
) -> Result<(), DiscoveryError> {
    let source_len = match kind {
        CopyKind::Parcels => viability.input().parcel_footprints.len(),
        CopyKind::Roads => all_nodes.len(),
        CopyKind::Nodes => ranking
            .targets()
            .ok_or(DiscoveryError::InvalidCheckpoint)?
            .len(),
    };
    let (expected_parcels, expected_nodes, expected_targets) = match kind {
        CopyKind::Parcels => (cursor, 0, 0),
        CopyKind::Roads => (viability.input().parcel_footprints.len(), cursor, 0),
        CopyKind::Nodes => (
            viability.input().parcel_footprints.len(),
            all_nodes.len(),
            cursor,
        ),
    };
    let joining = boundary < face.boundary_edge_ids.len();
    let join_state_valid = if joining {
        hosts.len() == boundary
            && face
                .boundary_edge_ids
                .get(..boundary)
                .is_some_and(|ids| hosts_match_boundary(hosts, ids, edges, all_nodes))
            && nodes.is_empty()
            && parcels.is_empty()
            && targets.is_empty()
            && cursor == 0
            && match host_phase {
                HostJoinPhase::Edge => {
                    matched.is_none()
                        && from.is_none()
                        && node_scan == 0
                        && edge_scan < edges.len()
                        && edges.get(..edge_scan).is_some_and(|prefix| {
                            !prefix
                                .iter()
                                .any(|edge| edge.id == face.boundary_edge_ids[boundary])
                        })
                }
                HostJoinPhase::FromNode => {
                    matched.is_some_and(|e| {
                        e.id == face.boundary_edge_ids[boundary]
                            && edge_scan > 0
                            && edges.get(edge_scan - 1) == Some(&e)
                    }) && from.is_none()
                        && node_scan <= all_nodes.len()
                        && all_nodes.get(..node_scan).is_some_and(|prefix| {
                            !prefix.iter().any(|node| node.id == matched.unwrap().from)
                        })
                }
                HostJoinPhase::ToNode => {
                    matched.is_some_and(|e| {
                        e.id == face.boundary_edge_ids[boundary]
                            && edge_scan > 0
                            && edges.get(edge_scan - 1) == Some(&e)
                    }) && from.is_some_and(|n| {
                        all_nodes.iter().any(|candidate| {
                            candidate.id == n.id && candidate.x == n.x && candidate.z == n.z
                        }) && n.id == matched.unwrap().from
                    }) && node_scan <= all_nodes.len()
                        && all_nodes.get(..node_scan).is_some_and(|prefix| {
                            !prefix.iter().any(|node| node.id == matched.unwrap().to)
                        })
                }
            }
    } else {
        matched.is_none()
            && from.is_none()
            && host_phase == HostJoinPhase::Edge
            && edge_scan == 0
            && node_scan == 0
            && hosts.len() == boundary
            && face
                .boundary_edge_ids
                .get(..boundary)
                .is_some_and(|ids| hosts_match_boundary(hosts, ids, edges, all_nodes))
            && parcels.len() == expected_parcels
            && nodes.len() == expected_nodes
            && targets.len() == expected_targets
            && viability.input().parcel_footprints.get(..parcels.len()) == Some(parcels)
            && nodes.len() <= all_nodes.len()
            && nodes
                .iter()
                .zip(all_nodes)
                .all(|(a, b)| a.id == b.id && a.x == b.x && a.z == b.z)
            && ranking
                .targets()
                .is_some_and(|source| source.get(..targets.len()) == Some(targets))
    };
    if !valid_face(face)
        || edges.len() > MAX_GRAPH_EDGES
        || hosts.len() > face.boundary_edge_ids.len()
        || boundary > face.boundary_edge_ids.len()
        || edge_scan > edges.len()
        || node_scan > all_nodes.len()
        || cursor > source_len
        || nodes.len() > all_nodes.len()
        || parcels.len() > viability.input().parcel_footprints.len()
        || targets.len()
            > ranking
                .targets()
                .ok_or(DiscoveryError::InvalidCheckpoint)?
                .len()
        || !join_state_valid
    {
        return Err(DiscoveryError::InvalidCheckpoint);
    }
    Ok(())
}

fn hosts_match_boundary(
    hosts: &[BoundaryHost],
    edge_ids: &[u32],
    edges: &[RoadFaceEdge],
    all_nodes: &[CandidateNode],
) -> bool {
    hosts.len() == edge_ids.len()
        && hosts.iter().zip(edge_ids).all(|(host, edge_id)| {
            let Some(edge) = edges.iter().find(|edge| edge.id == *edge_id) else {
                return false;
            };
            let Some(from) = all_nodes.iter().find(|node| node.id == edge.from) else {
                return false;
            };
            let Some(to) = all_nodes.iter().find(|node| node.id == edge.to) else {
                return false;
            };
            host.road_id == edge.id
                && host.from == edge.from
                && host.to == edge.to
                && host.ax == from.x
                && host.az == from.z
                && host.bx == to.x
                && host.bz == to.z
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(square: bool) -> DiscoveryInput {
        let graph_nodes = if square {
            vec![
                RoadFaceNode {
                    id: 1,
                    x: 0.0,
                    z: 0.0,
                },
                RoadFaceNode {
                    id: 2,
                    x: 100.0,
                    z: 0.0,
                },
                RoadFaceNode {
                    id: 3,
                    x: 100.0,
                    z: 100.0,
                },
                RoadFaceNode {
                    id: 4,
                    x: 0.0,
                    z: 100.0,
                },
            ]
        } else {
            Vec::new()
        };
        let graph_edges = if square {
            vec![
                RoadFaceEdge {
                    id: 11,
                    from: 1,
                    to: 2,
                },
                RoadFaceEdge {
                    id: 12,
                    from: 2,
                    to: 3,
                },
                RoadFaceEdge {
                    id: 13,
                    from: 3,
                    to: 4,
                },
                RoadFaceEdge {
                    id: 14,
                    from: 4,
                    to: 1,
                },
            ]
        } else {
            Vec::new()
        };
        let all_nodes = graph_nodes
            .iter()
            .map(|n| CandidateNode {
                id: n.id,
                x: n.x,
                z: n.z,
            })
            .collect();
        DiscoveryInput {
            graph_nodes,
            graph_edges,
            all_nodes,
            viability: FrozenViabilityInput {
                parcel_footprints: Vec::new(),
                reserved_lots: Vec::new(),
                open_roads: Vec::new(),
                pending_roads: Vec::new(),
                river_segments: Vec::new(),
                neighborhood_segments: Vec::new(),
                neighborhood_interior: None,
            },
        }
    }

    fn run(
        quota: usize,
        checkpoint_each_call: bool,
        input: DiscoveryInput,
    ) -> (Vec<AccessGeometryCandidate>, usize) {
        let mut job = DiscoveryJob::new(input).unwrap();
        let mut total = 0;
        let mut calls = 0usize;
        let mut prior_stage = stage_tag(&job.stage);
        for _ in 0..10_000_000 {
            let step = job.advance(quota);
            calls += 1;
            assert!(step.work_used <= quota, "wrapper exceeded quota");
            total += step.work_used;
            let current_stage = stage_tag(&job.stage);
            if checkpoint_each_call
                && !step.done
                && (current_stage != prior_stage || calls % 32 == 0)
            {
                let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                let cp: DiscoveryCheckpoint = serde_json::from_slice(&bytes).unwrap();
                job = DiscoveryJob::restore(cp).unwrap();
            }
            prior_stage = current_stage;
            if step.done {
                return (job.descriptors().unwrap().unwrap().to_vec(), total);
            }
            assert!(step.work_used > 0, "unfinished stage must make progress");
        }
        panic!("discovery did not terminate");
    }

    #[test]
    fn single_item_and_large_quotas_have_same_ordered_output_and_work() {
        let (one, work_one) = run(1, false, input(true));
        let (small, work_small) = run(7, false, input(true));
        let (large, work_large) = run(4096, false, input(true));
        assert_eq!(one, small);
        assert_eq!(one, large);
        assert_eq!(work_one, work_small);
        assert_eq!(work_one, work_large);
    }

    #[test]
    fn json_checkpoint_restores_at_each_quota_boundary() {
        let (plain, work) = run(1024, false, input(true));
        let (restored, restored_work) = run(1024, true, input(true));
        assert_eq!(plain, restored);
        assert_eq!(work, restored_work);
    }

    fn stage_tag(stage: &Stage) -> u8 {
        match stage {
            Stage::CopyGraph { .. } => 0,
            Stage::Trace { .. } => 1,
            Stage::Selection { .. } => 2,
            Stage::CopyRankInputs { .. } => 3,
            Stage::Ranking { .. } => 4,
            Stage::BuildDescriptors { .. } => 5,
            Stage::Descriptors(_) => 6,
            Stage::Done(_) => 7,
            Stage::Failed(_) => 8,
        }
    }

    #[test]
    fn no_bounded_face_is_successful_empty_discovery() {
        let (descriptors, _) = run(3, true, input(false));
        assert!(descriptors.is_empty());
    }

    #[test]
    fn duplicate_snapshot_node_ids_reject_before_work() {
        let mut invalid = input(false);
        invalid.all_nodes = vec![
            CandidateNode {
                id: 9,
                x: 0.0,
                z: 0.0,
            },
            CandidateNode {
                id: 9,
                x: 1.0,
                z: 1.0,
            },
        ];
        assert_eq!(
            DiscoveryJob::new(invalid).err(),
            Some(DiscoveryError::InvalidInput)
        );
    }

    #[test]
    fn invalid_graphs_reject_at_construction_and_valid_copy_checkpoints_resume() {
        for kind in 0..4 {
            let mut invalid = input(true);
            match kind {
                0 => invalid.graph_edges[0].to = u32::MAX,
                1 => invalid.graph_edges.push(invalid.graph_edges[0]),
                2 => invalid.graph_nodes.push(invalid.graph_nodes[0]),
                _ => { invalid.all_nodes.pop(); },
            }
            assert_eq!(DiscoveryJob::new(invalid).err(), Some(DiscoveryError::InvalidInput));
        }
        let mut job = DiscoveryJob::new(input(true)).unwrap();
        for _ in 0..4 {
            let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
            job = DiscoveryJob::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
            job.advance(1);
        }
    }

    #[test]
    fn corrupted_copy_cursor_is_rejected() {
        let mut job = DiscoveryJob::new(input(true)).unwrap();
        let _ = job.advance(1);
        let mut cp = job.checkpoint();
        if let StageCheckpoint::CopyGraph {
            cursor,
            graph_edges,
            ..
        } = &mut cp.stage
        {
            *cursor = graph_edges.len() + 1;
        } else {
            panic!("expected graph copy stage");
        }
        assert_eq!(
            DiscoveryJob::restore(cp).err(),
            Some(DiscoveryError::InvalidCheckpoint)
        );
    }

    #[test]
    fn forged_host_phase_without_matching_edge_is_rejected() {
        let input = input(true);
        let context = FrozenViabilityContext::new(input.viability.clone()).unwrap();
        let mut ranking =
            TargetRanking::new(Vec::new(), Vec::new(), Vec::new(), Vec::new()).unwrap();
        while !ranking.advance(1).done {}
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![11, 12, 13, 14],
            contains_proposed_edge: false,
        };
        let cp = DiscoveryCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            all_nodes: input.all_nodes,
            viability: context.checkpoint(),
            stage: StageCheckpoint::BuildDescriptors {
                face,
                ranking: ranking.checkpoint(),
                retained_edges: vec![RoadFaceEdge {
                    id: 11,
                    from: 1,
                    to: 2,
                }],
                hosts: Vec::new(),
                nodes: Vec::new(),
                parcels: Vec::new(),
                targets: Vec::new(),
                copy_kind: CopyKind::Parcels,
                cursor: 0,
                boundary_index: 0,
                host_phase: HostJoinPhase::FromNode,
                edge_scan: 1,
                node_scan: 0,
                matched_edge: None,
                from_node: None,
            },
        };
        assert_eq!(
            DiscoveryJob::restore(cp).err(),
            Some(DiscoveryError::InvalidCheckpoint)
        );
    }

    #[test]
    fn non_planar_trace_is_a_terminal_error_without_partial_descriptors() {
        let mut invalid = input(true);
        invalid.graph_edges.extend([
            RoadFaceEdge {
                id: 15,
                from: 1,
                to: 3,
            },
            RoadFaceEdge {
                id: 16,
                from: 2,
                to: 4,
            },
        ]);
        let mut job = DiscoveryJob::new(invalid).unwrap();
        for _ in 0..100_000 {
            let step = job.advance(128);
            assert!(step.work_used <= 128);
            if step.done {
                break;
            }
        }
        assert_eq!(
            job.descriptors(),
            Some(Err(DiscoveryError::Trace(RoadFaceError::NonPlanar)))
        );
    }
}
