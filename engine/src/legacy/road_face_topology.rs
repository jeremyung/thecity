//! Resumable construction of the half-edge topology used by face walking.
//!
//! The input has already passed node/edge validation, planarity, and bridge
//! detection. This stage preserves edge order, forward/reverse half-edge order,
//! outgoing-edge sort tie-breaks, and the clockwise-predecessor rule used by
//! `road_faces::trace_faces`.

use crate::legacy::road_faces::{RoadFaceEdge, RoadFaceNode};
use serde::{Deserialize, Serialize};

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_NODES: usize = 512;
const MAX_EDGES: usize = 512;
const MAX_HALF_EDGES: usize = MAX_EDGES * 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TopologyError {
    TooManyNodes,
    TooManyEdges,
    InvalidInput,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct HalfEdgeRecord {
    pub edge_index: usize,
    pub from: usize,
    pub to: usize,
    pub angle: f32,
}

type NodeRecord = RoadFaceNode;
type EdgeRecord = RoadFaceEdge;

#[derive(Clone, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct Topology {
    pub half_edges: Vec<HalfEdgeRecord>,
    /// For every directed half-edge, the next clockwise predecessor of its
    /// reverse edge at the destination vertex.
    pub next: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct TopologyView<'a> {
    pub half_edges: &'a [HalfEdgeRecord],
    pub next: &'a [usize],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Initialize,
    Build,
    SortOutgoing,
    LinkNext,
    Done,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    schema: u8,
    nodes: Vec<NodeRecord>,
    edges: Vec<EdgeRecord>,
    indexed_edges: Vec<(usize, usize)>,
    bridges: Vec<bool>,
    phase: Phase,
    initialize_cursor: usize,
    build_edge: usize,
    build_step: u8,
    half_edges: Vec<HalfEdgeRecord>,
    outgoing: Vec<Vec<usize>>,
    sort_node: usize,
    sort_index: usize,
    sort_compare: usize,
    sort_key: Option<usize>,
    sort_shifts: usize,
    link_edge: usize,
    reverse_scan: usize,
    reverse_position: Option<usize>,
    next: Vec<usize>,
}

/// Runtime topology builder. It intentionally has no `Deserialize` impl;
/// checkpoint input must go through the validating `restore` constructor.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopologySearch {
    state: Checkpoint,
}

impl TopologySearch {
    #[cfg(any(test, feature = "legacy"))]
    pub(crate) fn new(
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
        indexed_edges: Vec<(usize, usize)>,
        bridges: Vec<bool>,
    ) -> Result<Self, TopologyError> {
        if nodes.len() > MAX_NODES {
            return Err(TopologyError::TooManyNodes);
        }
        if edges.len() > MAX_EDGES {
            return Err(TopologyError::TooManyEdges);
        }
        if indexed_edges.len() != edges.len()
            || bridges.len() != edges.len()
            || nodes
                .iter()
                .any(|node| !node.x.is_finite() || !node.z.is_finite())
            || nodes
                .iter()
                .enumerate()
                .any(|(i, node)| nodes[..i].iter().any(|prior| prior.id == node.id))
            || indexed_edges.iter().enumerate().any(|(i, &(from, to))| {
                from >= nodes.len()
                    || to >= nodes.len()
                    || nodes[from].id != edges[i].from
                    || nodes[to].id != edges[i].to
                    || edges[i].from == edges[i].to
            })
            || edges
                .iter()
                .enumerate()
                .any(|(i, edge)| edges[..i].iter().any(|prior| prior.id == edge.id))
        {
            return Err(TopologyError::InvalidInput);
        }
        Ok(Self::new_trusted(nodes, edges, indexed_edges, bridges))
    }

    /// Construct from the preflight/bridge pipeline. Node-sized output buffers
    /// are initialized one node per `advance` work unit.
    #[cfg(feature = "legacy")]
    pub(crate) fn new_trusted(
        nodes: Vec<RoadFaceNode>,
        edges: Vec<RoadFaceEdge>,
        indexed_edges: Vec<(usize, usize)>,
        bridges: Vec<bool>,
    ) -> Self {
        let node_count = nodes.len();
        Self {
            state: Checkpoint {
                schema: CHECKPOINT_SCHEMA,
                nodes,
                edges,
                indexed_edges,
                bridges,
                phase: Phase::Initialize,
                initialize_cursor: 0,
                build_edge: 0,
                build_step: 0,
                half_edges: Vec::with_capacity(MAX_HALF_EDGES),
                outgoing: Vec::with_capacity(node_count),
                sort_node: 0,
                sort_index: 1,
                sort_compare: 0,
                sort_key: None,
                sort_shifts: 0,
                link_edge: 0,
                reverse_scan: 0,
                reverse_position: None,
                next: Vec::with_capacity(MAX_HALF_EDGES),
            },
        }
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        self.state.clone()
    }

    pub(crate) fn restore(checkpoint: Checkpoint) -> Result<Self, TopologyError> {
        validate_checkpoint(&checkpoint)?;
        Ok(Self { state: checkpoint })
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || self.state.phase == Phase::Done {
            return AdvanceResult {
                work_used: 0,
                done: self.state.phase == Phase::Done,
            };
        }
        let mut used = 0;
        while used < budget && self.state.phase != Phase::Done {
            match self.state.phase {
                Phase::Initialize => self.advance_initialize(),
                Phase::Build => self.advance_build(),
                Phase::SortOutgoing => self.advance_sort(),
                Phase::LinkNext => self.advance_link(),
                Phase::Done => break,
            }
            used += 1;
        }
        AdvanceResult {
            work_used: used,
            done: self.state.phase == Phase::Done,
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_initialize(&mut self) {
        let cp = &mut self.state;
        if cp.initialize_cursor >= cp.nodes.len() {
            cp.phase = Phase::Build;
            return;
        }
        cp.outgoing.push(Vec::new());
        cp.initialize_cursor += 1;
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn topology(&self) -> Option<TopologyView<'_>> {
        if self.state.phase != Phase::Done {
            return None;
        }
        Some(TopologyView {
            half_edges: &self.state.half_edges,
            next: &self.state.next,
        })
    }

    pub(crate) fn last_edge_id(&self) -> Option<u32> {
        self.state.edges.last().map(|edge| edge.id)
    }

    /// Consume a completed topology stage and move all graph buffers into the
    /// face walker without cloning the half-edge arrays.
    #[cfg(feature = "legacy")]
    pub(crate) fn into_result(self) -> Option<(Vec<RoadFaceNode>, Vec<RoadFaceEdge>, Topology)> {
        if self.state.phase != Phase::Done {
            return None;
        }
        Some((
            self.state.nodes,
            self.state.edges,
            Topology {
                half_edges: self.state.half_edges,
                next: self.state.next,
            },
        ))
    }

    #[cfg(feature = "legacy")]
    fn advance_build(&mut self) {
        let cp = &mut self.state;
        if cp.build_edge >= cp.edges.len() {
            cp.phase = Phase::SortOutgoing;
            cp.sort_node = 0;
            cp.sort_index = 1;
            cp.sort_compare = 0;
            cp.sort_key = None;
            cp.sort_shifts = 0;
            return;
        }
        if cp.bridges[cp.build_edge] {
            cp.build_edge += 1;
            cp.build_step = 0;
            return;
        }
        let (from, to) = cp.indexed_edges[cp.build_edge];
        let base = cp.half_edges.len();
        match cp.build_step {
            0 => {
                cp.half_edges.push(HalfEdgeRecord {
                    edge_index: cp.build_edge,
                    from,
                    to,
                    angle: (cp.nodes[to].z - cp.nodes[from].z)
                        .atan2(cp.nodes[to].x - cp.nodes[from].x),
                });
                cp.build_step = 1;
            }
            1 => {
                cp.outgoing[from].push(base - 1);
                cp.build_step = 2;
            }
            2 => {
                cp.half_edges.push(HalfEdgeRecord {
                    edge_index: cp.build_edge,
                    from: to,
                    to: from,
                    angle: (cp.nodes[from].z - cp.nodes[to].z)
                        .atan2(cp.nodes[from].x - cp.nodes[to].x),
                });
                cp.build_step = 3;
            }
            _ => {
                cp.outgoing[to].push(base - 1);
                cp.build_edge += 1;
                cp.build_step = 0;
            }
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_sort(&mut self) {
        let cp = &mut self.state;
        if cp.sort_node >= cp.nodes.len() {
            cp.phase = Phase::LinkNext;
            return;
        }
        let len = cp.outgoing[cp.sort_node].len();
        if cp.sort_key.is_none() {
            if cp.sort_index >= len {
                cp.sort_node += 1;
                cp.sort_index = 1;
                cp.sort_compare = 0;
                cp.sort_shifts = 0;
                return;
            }
            cp.sort_key = Some(cp.outgoing[cp.sort_node][cp.sort_index]);
            cp.sort_compare = cp.sort_index;
            cp.sort_shifts = 0;
            return;
        }
        let key = cp.sort_key.expect("sort key set");
        if cp.sort_compare > 0
            && compare_half_edges(cp, cp.outgoing[cp.sort_node][cp.sort_compare - 1], key).is_gt()
        {
            cp.outgoing[cp.sort_node][cp.sort_compare] =
                cp.outgoing[cp.sort_node][cp.sort_compare - 1];
            cp.sort_compare -= 1;
            cp.sort_shifts += 1;
            return;
        }
        cp.outgoing[cp.sort_node][cp.sort_compare] = key;
        cp.sort_index += 1;
        cp.sort_compare = 0;
        cp.sort_key = None;
        cp.sort_shifts = 0;
    }

    #[cfg(feature = "legacy")]
    fn advance_link(&mut self) {
        let cp = &mut self.state;
        if cp.link_edge >= cp.half_edges.len() {
            cp.phase = Phase::Done;
            return;
        }
        let half = &cp.half_edges[cp.link_edge];
        let incident = &cp.outgoing[half.to];
        if cp.reverse_position.is_none() {
            if cp.reverse_scan >= incident.len() {
                // Validated undirected input guarantees a reverse half-edge.
                // Treat a corrupted intermediate topology as a closed failure
                // at restore rather than indexing beyond this stage.
                cp.phase = Phase::Done;
                cp.next.clear();
                return;
            }
            let candidate_index = incident[cp.reverse_scan];
            cp.reverse_scan += 1;
            let candidate = &cp.half_edges[candidate_index];
            if candidate.edge_index == half.edge_index && candidate.to == half.from {
                cp.reverse_position = Some(cp.reverse_scan - 1);
            }
            return;
        }
        let reverse_position = cp.reverse_position.take().expect("reverse position set");
        let next_position = (reverse_position + incident.len() - 1) % incident.len();
        cp.next.push(incident[next_position]);
        cp.link_edge += 1;
        cp.reverse_scan = 0;
    }
}

fn compare_half_edges(cp: &Checkpoint, left: usize, right: usize) -> std::cmp::Ordering {
    let a = &cp.half_edges[left];
    let b = &cp.half_edges[right];
    a.angle
        .total_cmp(&b.angle)
        .then_with(|| cp.nodes[a.to].id.cmp(&cp.nodes[b.to].id))
        .then_with(|| cp.edges[a.edge_index].id.cmp(&cp.edges[b.edge_index].id))
}

fn validate_checkpoint(cp: &Checkpoint) -> Result<(), TopologyError> {
    if cp.schema != CHECKPOINT_SCHEMA
        || cp.nodes.len() > MAX_NODES
        || cp.edges.len() > MAX_EDGES
        || cp.indexed_edges.len() != cp.edges.len()
        || cp.bridges.len() != cp.edges.len()
        || cp.half_edges.len() > MAX_HALF_EDGES
        || cp.next.len() > cp.half_edges.len()
        || cp
            .nodes
            .iter()
            .any(|node| !node.x.is_finite() || !node.z.is_finite())
        || cp
            .nodes
            .iter()
            .enumerate()
            .any(|(index, node)| cp.nodes[..index].iter().any(|prior| prior.id == node.id))
        || cp.edges.iter().enumerate().any(|(index, edge)| {
            edge.from == edge.to || cp.edges[..index].iter().any(|prior| prior.id == edge.id)
        })
        || cp
            .indexed_edges
            .iter()
            .enumerate()
            .any(|(index, &(from, to))| {
                from >= cp.nodes.len()
                    || to >= cp.nodes.len()
                    || cp.nodes[from].id != cp.edges[index].from
                    || cp.nodes[to].id != cp.edges[index].to
            })
    {
        return Err(TopologyError::InvalidCheckpoint);
    }
    if cp.phase == Phase::Initialize {
        if cp.initialize_cursor > cp.nodes.len()
            || cp.outgoing.len() != cp.initialize_cursor
            || cp.outgoing.iter().any(|edges| !edges.is_empty())
            || !cp.half_edges.is_empty()
            || !cp.next.is_empty()
            || cp.build_edge != 0
            || cp.build_step != 0
            || cp.sort_node != 0
            || cp.sort_index != 1
            || cp.sort_compare != 0
            || cp.sort_key.is_some()
            || cp.link_edge != 0
            || cp.reverse_scan != 0
            || cp.reverse_position.is_some()
        {
            return Err(TopologyError::InvalidCheckpoint);
        }
        return Ok(());
    }
    if cp.outgoing.len() != cp.nodes.len()
        || cp.build_edge > cp.edges.len()
        || cp.build_step > 3
        || cp.sort_node > cp.nodes.len()
        || cp.half_edges.iter().any(|half| {
            half.edge_index >= cp.edges.len()
                || half.from >= cp.nodes.len()
                || half.to >= cp.nodes.len()
                || !half.angle.is_finite()
        })
        || cp
            .outgoing
            .iter()
            .flatten()
            .any(|&index| index >= cp.half_edges.len())
        || cp.next.iter().any(|&index| index >= cp.half_edges.len())
    {
        return Err(TopologyError::InvalidCheckpoint);
    }

    match cp.phase {
        Phase::Initialize => unreachable!(),
        Phase::Build => {
            if cp.sort_node != 0
                || cp.sort_index != 1
                || cp.sort_compare != 0
                || cp.sort_key.is_some()
                || cp.link_edge != 0
                || cp.reverse_scan != 0
                || cp.reverse_position.is_some()
                || !cp.next.is_empty()
                || !build_prefix_matches(cp)
            {
                return Err(TopologyError::InvalidCheckpoint);
            }
        }
        Phase::SortOutgoing => {
            if cp.build_edge != cp.edges.len()
                || cp.build_step != 0
                || cp.link_edge != 0
                || cp.reverse_scan != 0
                || cp.reverse_position.is_some()
                || !cp.next.is_empty()
                || !complete_topology_matches(cp)
                || !sort_cursor_valid(cp)
            {
                return Err(TopologyError::InvalidCheckpoint);
            }
        }
        Phase::LinkNext => {
            if cp.build_edge != cp.edges.len()
                || cp.build_step != 0
                || cp.sort_node != cp.nodes.len()
                || cp.sort_key.is_some()
                || cp.link_edge > cp.half_edges.len()
                || cp.next.len() != cp.link_edge
                || (cp.link_edge < cp.half_edges.len() && {
                    let half = &cp.half_edges[cp.link_edge];
                    let incident = &cp.outgoing[half.to];
                    let reverse = incident.iter().position(|&index| {
                        let candidate = &cp.half_edges[index];
                        candidate.edge_index == half.edge_index && candidate.to == half.from
                    });
                    match (reverse, cp.reverse_position) {
                        (Some(position), None) => cp.reverse_scan > position,
                        (Some(position), Some(saved)) => {
                            saved != position || cp.reverse_scan != position + 1
                        }
                        (None, _) => true,
                    }
                })
                || (cp.link_edge == cp.half_edges.len()
                    && (cp.reverse_scan != 0 || cp.reverse_position.is_some()))
                || cp.reverse_position.is_some_and(|position| {
                    position
                        >= cp
                            .outgoing
                            .get(cp.half_edges.get(cp.link_edge).map_or(0, |half| half.to))
                            .map_or(0, Vec::len)
                })
                || !complete_topology_matches(cp)
                || cp.outgoing.iter().enumerate().any(|(node, list)| {
                    !list
                        .windows(2)
                        .all(|pair| compare_half_edges(cp, pair[0], pair[1]).is_le())
                        || list.iter().any(|&half| cp.half_edges[half].from != node)
                })
            {
                return Err(TopologyError::InvalidCheckpoint);
            }
        }
        Phase::Done => {
            if cp.link_edge != cp.half_edges.len()
                || cp.next.len() != cp.half_edges.len()
                || cp.reverse_scan != 0
                || cp.reverse_position.is_some()
                || !complete_topology_matches(cp)
                || cp.outgoing.iter().enumerate().any(|(node, list)| {
                    !list
                        .windows(2)
                        .all(|pair| compare_half_edges(cp, pair[0], pair[1]).is_le())
                        || list.iter().any(|&half| cp.half_edges[half].from != node)
                })
            {
                return Err(TopologyError::InvalidCheckpoint);
            }
        }
    }
    Ok(())
}

fn expected_build(
    cp: &Checkpoint,
    through_edge: usize,
    current_step: u8,
) -> (Vec<HalfEdgeRecord>, Vec<Vec<usize>>) {
    let mut half_edges = Vec::new();
    let mut outgoing = vec![Vec::new(); cp.nodes.len()];
    for edge_index in 0..through_edge.min(cp.edges.len()) {
        if cp.bridges[edge_index] {
            continue;
        }
        append_expected_edge(cp, edge_index, &mut half_edges, &mut outgoing, 4);
    }
    if through_edge < cp.edges.len() && !cp.bridges[through_edge] {
        append_expected_edge(
            cp,
            through_edge,
            &mut half_edges,
            &mut outgoing,
            current_step,
        );
    }
    (half_edges, outgoing)
}

fn append_expected_edge(
    cp: &Checkpoint,
    edge_index: usize,
    half_edges: &mut Vec<HalfEdgeRecord>,
    outgoing: &mut [Vec<usize>],
    steps: u8,
) {
    let (from, to) = cp.indexed_edges[edge_index];
    let base = half_edges.len();
    if steps >= 1 {
        half_edges.push(HalfEdgeRecord {
            edge_index,
            from,
            to,
            angle: (cp.nodes[to].z - cp.nodes[from].z).atan2(cp.nodes[to].x - cp.nodes[from].x),
        });
    }
    if steps >= 2 {
        outgoing[from].push(base);
    }
    if steps >= 3 {
        half_edges.push(HalfEdgeRecord {
            edge_index,
            from: to,
            to: from,
            angle: (cp.nodes[from].z - cp.nodes[to].z).atan2(cp.nodes[from].x - cp.nodes[to].x),
        });
    }
    if steps >= 4 {
        outgoing[to].push(base + 1);
    }
}

fn build_prefix_matches(cp: &Checkpoint) -> bool {
    let (half_edges, outgoing) = expected_build(cp, cp.build_edge, cp.build_step);
    cp.half_edges == half_edges && cp.outgoing == outgoing
}

fn complete_topology_matches(cp: &Checkpoint) -> bool {
    let (half_edges, outgoing) = expected_build(cp, cp.edges.len(), 0);
    cp.half_edges == half_edges
        && cp.outgoing.len() == outgoing.len()
        && cp
            .outgoing
            .iter()
            .zip(outgoing)
            .enumerate()
            .all(|(node, (actual, mut expected))| {
                let mut actual = actual.clone();
                if cp.phase == Phase::SortOutgoing && node == cp.sort_node {
                    if let Some(key) = cp.sort_key {
                        if cp.sort_compare > cp.sort_index || cp.sort_index >= actual.len() {
                            return false;
                        }
                        let mut logical = Vec::with_capacity(actual.len());
                        logical.extend_from_slice(&actual[..cp.sort_compare]);
                        logical.push(key);
                        logical.extend_from_slice(&actual[cp.sort_compare + 1..=cp.sort_index]);
                        logical.extend_from_slice(&actual[cp.sort_index + 1..]);
                        actual = logical;
                    }
                }
                actual.sort_unstable();
                expected.sort_unstable();
                actual == expected
            })
}

fn sort_cursor_valid(cp: &Checkpoint) -> bool {
    if cp.sort_node == cp.nodes.len() {
        return cp.sort_key.is_none() && cp.sort_compare == 0 && cp.sort_shifts == 0;
    }
    let len = cp.outgoing[cp.sort_node].len();
    if cp.sort_compare > cp.sort_index
        || cp.sort_index > len.saturating_add(1)
        || cp.sort_key.is_none() && (cp.sort_compare != 0 || cp.sort_shifts != 0)
    {
        return false;
    }
    if let Some(key) = cp.sort_key {
        if cp.sort_index >= len
            || key >= cp.half_edges.len()
            || cp.sort_shifts != cp.sort_index - cp.sort_compare
        {
            return false;
        }
        let mut logical = cp.outgoing[cp.sort_node].clone();
        logical.splice(
            cp.sort_compare..=cp.sort_index,
            std::iter::once(key).chain(
                cp.outgoing[cp.sort_node][cp.sort_compare + 1..=cp.sort_index]
                    .iter()
                    .copied(),
            ),
        );
        let mut expected = expected_build(cp, cp.edges.len(), 0).1[cp.sort_node].clone();
        logical.sort_unstable();
        expected.sort_unstable();
        logical == expected
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn node(id: u32, x: f32, z: f32) -> RoadFaceNode {
        RoadFaceNode { id, x, z }
    }
    fn edge(id: u32, from: u32, to: u32) -> RoadFaceEdge {
        RoadFaceEdge { id, from, to }
    }

    fn inputs(
        points: &[(f32, f32)],
        edge_pairs: &[(usize, usize)],
        bridge_indices: &[usize],
    ) -> (
        Vec<RoadFaceNode>,
        Vec<RoadFaceEdge>,
        Vec<(usize, usize)>,
        Vec<bool>,
    ) {
        let nodes = points
            .iter()
            .enumerate()
            .map(|(i, &(x, z))| node(i as u32 + 10, x, z))
            .collect::<Vec<_>>();
        let edges = edge_pairs
            .iter()
            .enumerate()
            .map(|(i, &(a, b))| edge(i as u32 + 100, a as u32 + 10, b as u32 + 10))
            .collect::<Vec<_>>();
        let indexed = edge_pairs.to_vec();
        let mut bridges = vec![false; edges.len()];
        for &index in bridge_indices {
            bridges[index] = true;
        }
        (nodes, edges, indexed, bridges)
    }

    fn oracle(
        nodes: &[RoadFaceNode],
        edges: &[RoadFaceEdge],
        indexed: &[(usize, usize)],
        bridges: &[bool],
    ) -> Topology {
        let mut half_edges = Vec::new();
        let mut outgoing = vec![Vec::<usize>::new(); nodes.len()];
        for (edge_index, &(from, to)) in indexed.iter().enumerate() {
            if bridges[edge_index] {
                continue;
            }
            let forward = half_edges.len();
            half_edges.push(HalfEdgeRecord {
                edge_index,
                from,
                to,
                angle: (nodes[to].z - nodes[from].z).atan2(nodes[to].x - nodes[from].x),
            });
            let reverse = half_edges.len();
            half_edges.push(HalfEdgeRecord {
                edge_index,
                from: to,
                to: from,
                angle: (nodes[from].z - nodes[to].z).atan2(nodes[from].x - nodes[to].x),
            });
            outgoing[from].push(forward);
            outgoing[to].push(reverse);
        }
        for list in &mut outgoing {
            list.sort_by(|&left, &right| {
                let a = &half_edges[left];
                let b = &half_edges[right];
                a.angle
                    .total_cmp(&b.angle)
                    .then_with(|| nodes[a.to].id.cmp(&nodes[b.to].id))
                    .then_with(|| edges[a.edge_index].id.cmp(&edges[b.edge_index].id))
            });
        }
        let mut next = vec![0; half_edges.len()];
        for (index, half) in half_edges.iter().enumerate() {
            let incident = &outgoing[half.to];
            let reverse_pos = incident
                .iter()
                .position(|&candidate| {
                    let candidate = &half_edges[candidate];
                    candidate.edge_index == half.edge_index && candidate.to == half.from
                })
                .unwrap();
            next[index] = incident[(reverse_pos + incident.len() - 1) % incident.len()];
        }
        Topology { half_edges, next }
    }

    fn run_case(
        input: (
            Vec<RoadFaceNode>,
            Vec<RoadFaceEdge>,
            Vec<(usize, usize)>,
            Vec<bool>,
        ),
        quotas: &[usize],
    ) -> Topology {
        let expected = oracle(&input.0, &input.1, &input.2, &input.3);
        let mut search = TopologySearch::new(input.0, input.1, input.2, input.3).unwrap();
        let mut turn = 0usize;
        while search.topology().is_none() {
            let quota = quotas[turn % quotas.len()];
            let result = search.advance(quota);
            assert!(result.work_used <= quota);
            assert!(result.done || result.work_used > 0);
            let json = serde_json::to_vec(&search.checkpoint()).unwrap();
            let cp: Checkpoint = serde_json::from_slice(&json).unwrap();
            search = TopologySearch::restore(cp.clone()).unwrap_or_else(|error| {
                panic!(
                    "invalid topology checkpoint turn={turn} phase={:?} state={cp:?}: {error:?}",
                    cp.phase
                )
            });
            turn += 1;
            assert!(turn < 1_000_000);
        }
        let actual = search.topology().unwrap();
        assert_eq!(actual.half_edges, expected.half_edges);
        assert_eq!(actual.next, expected.next);
        expected
    }

    #[test]
    fn topology_matches_reference_order_for_cycles_spurs_and_disconnected_graphs() {
        let rectangle = inputs(
            &[(0., 0.), (10., 0.), (10., 8.), (0., 8.)],
            &[(0, 1), (1, 2), (2, 3), (3, 0)],
            &[],
        );
        let dangling = inputs(
            &[(0., 0.), (10., 0.), (10., 8.), (0., 8.), (14., 8.)],
            &[(0, 1), (1, 2), (2, 3), (3, 0), (2, 4)],
            &[4],
        );
        let disconnected = inputs(
            &[
                (0., 0.),
                (10., 0.),
                (10., 8.),
                (0., 8.),
                (20., 0.),
                (30., 0.),
                (30., 8.),
                (20., 8.),
            ],
            &[
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
            ],
            &[],
        );
        let nonascending = inputs(
            &[(0., 0.), (10., 0.), (10., 8.), (0., 8.)],
            &[(0, 2), (0, 1), (0, 3), (1, 2), (2, 3)],
            &[4],
        );
        let empty = inputs(&[], &[], &[]);
        for case in [rectangle, dangling, disconnected, nonascending, empty] {
            for quotas in [&[1][..], &[7][..], &[48][..], &[3, 1, 9, 2][..]] {
                run_case(case.clone(), quotas);
            }
        }
    }

    #[test]
    fn malformed_input_and_checkpoint_cursors_reject_safely() {
        assert_eq!(
            TopologySearch::new(vec![], vec![], vec![], vec![])
                .unwrap()
                .advance(0)
                .work_used,
            0
        );
        let (nodes, edges, indexed, bridges) = inputs(
            &[(0., 0.), (10., 0.), (10., 8.), (0., 8.)],
            &[(0, 1), (1, 2), (2, 3), (3, 0)],
            &[],
        );
        assert_eq!(
            TopologySearch::new(nodes.clone(), edges.clone(), vec![], bridges.clone()).unwrap_err(),
            TopologyError::InvalidInput
        );
        let mut search = TopologySearch::new(nodes, edges, indexed, bridges).unwrap();
        search.advance(1);
        let mut bad = search.checkpoint();
        bad.build_edge = usize::MAX;
        assert_eq!(
            TopologySearch::restore(bad).unwrap_err(),
            TopologyError::InvalidCheckpoint
        );
        while search.state.phase != Phase::SortOutgoing {
            search.advance(1);
        }
        let mut bad = search.checkpoint();
        bad.sort_node = bad.nodes.len() + 1;
        assert_eq!(
            TopologySearch::restore(bad).unwrap_err(),
            TopologyError::InvalidCheckpoint
        );
        while search.state.phase != Phase::LinkNext {
            search.advance(1);
        }
        let mut bad = search.checkpoint();
        bad.link_edge = usize::MAX;
        assert_eq!(
            TopologySearch::restore(bad).unwrap_err(),
            TopologyError::InvalidCheckpoint
        );
        let mut bad = search.checkpoint();
        bad.reverse_scan = usize::MAX;
        assert_eq!(
            TopologySearch::restore(bad).unwrap_err(),
            TopologyError::InvalidCheckpoint
        );
        let mut bad = search.checkpoint();
        bad.reverse_position = Some(0);
        bad.reverse_scan = 0;
        assert_eq!(
            TopologySearch::restore(bad).unwrap_err(),
            TopologyError::InvalidCheckpoint
        );
    }

    #[test]
    fn initialize_checkpoint_rejects_forged_indexed_endpoints() {
        let (nodes, edges, indexed, bridges) = inputs(
            &[(0., 0.), (10., 0.), (10., 8.), (0., 8.)],
            &[(0, 1), (1, 2), (2, 3), (3, 0)],
            &[],
        );
        let search = TopologySearch::new(nodes, edges, indexed, bridges).unwrap();
        let mut bad = search.checkpoint();
        assert_eq!(bad.phase, Phase::Initialize);
        bad.indexed_edges[0] = (usize::MAX, 1);
        assert_eq!(
            TopologySearch::restore(bad).unwrap_err(),
            TopologyError::InvalidCheckpoint
        );
    }

    #[test]
    fn zero_budget_and_done_are_noops_and_caps_are_enforced() {
        let (nodes, edges, indexed, bridges) = inputs(&[(0., 0.), (1., 0.)], &[(0, 1)], &[0]);
        let mut search = TopologySearch::new(nodes, edges, indexed, bridges).unwrap();
        let before = search.checkpoint();
        assert_eq!(
            search.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        assert_eq!(search.checkpoint(), before);
        while search.topology().is_none() {
            search.advance(7);
        }
        assert_eq!(
            search.advance(48),
            AdvanceResult {
                work_used: 0,
                done: true
            }
        );
        assert_eq!(
            TopologySearch::new(vec![node(0, 0., 0.); MAX_NODES + 1], vec![], vec![], vec![])
                .unwrap_err(),
            TopologyError::TooManyNodes
        );
    }
}
