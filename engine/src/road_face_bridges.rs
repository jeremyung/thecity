//! Incremental Tarjan bridge detection for bounded road-face graphs.
//!
//! This is a pure preparation job. `new` validates and allocates only inputs
//! within the fixed node/edge caps; all adjacency construction, ordering, and
//! DFS work is charged to `advance`. The runtime job cannot be deserialized
//! directly: callers must use `restore` to validate a checkpoint.

use serde::{Deserialize, Serialize};

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_NODES: usize = 512;
const MAX_EDGES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BridgeError {
    TooManyNodes,
    TooManyEdges,
    InvalidEndpoint,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Initialize,
    BuildAdjacency,
    SortAdjacency,
    FindBridges,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct DfsFrame {
    node: usize,
    parent_edge: Option<usize>,
    next_neighbor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    schema: u8,
    node_count: usize,
    edges: Vec<(usize, usize)>,
    phase: Phase,
    initialize_cursor: usize,
    build_edge: usize,
    build_endpoint: u8,
    adjacency: Vec<Vec<(usize, usize)>>,
    sort_node: usize,
    sort_index: usize,
    sort_compare: usize,
    sort_key: Option<(usize, usize)>,
    sort_shifts: usize,
    entered: Vec<usize>,
    low: Vec<usize>,
    bridges: Vec<bool>,
    dfs_root: usize,
    pending_enter: Option<(usize, Option<usize>)>,
    stack: Vec<DfsFrame>,
    time: usize,
}

/// Runtime state for bounded, resumable Tarjan bridge detection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BridgeSearch {
    state: Checkpoint,
}

impl BridgeSearch {
    pub(crate) fn new(node_count: usize, edges: Vec<(usize, usize)>) -> Result<Self, BridgeError> {
        if node_count > MAX_NODES {
            return Err(BridgeError::TooManyNodes);
        }
        if edges.len() > MAX_EDGES {
            return Err(BridgeError::TooManyEdges);
        }
        if edges
            .iter()
            .any(|&(from, to)| from >= node_count || to >= node_count)
        {
            return Err(BridgeError::InvalidEndpoint);
        }
        let edge_count = edges.len();
        Ok(Self::new_trusted(node_count, edges))
    }

    /// Construct from endpoint indices already validated by graph preflight.
    /// All node/edge-sized initialization is performed incrementally by
    /// `advance`, so this handoff itself is constant-work.
    pub(crate) fn new_trusted(node_count: usize, edges: Vec<(usize, usize)>) -> Self {
        let edge_count = edges.len();
        Self {
            state: Checkpoint {
                schema: CHECKPOINT_SCHEMA,
                node_count,
                edges,
                phase: Phase::Initialize,
                initialize_cursor: 0,
                build_edge: 0,
                build_endpoint: 0,
                adjacency: Vec::with_capacity(node_count),
                sort_node: 0,
                sort_index: 1,
                sort_compare: 0,
                sort_key: None,
                sort_shifts: 0,
                entered: Vec::with_capacity(node_count),
                low: Vec::with_capacity(node_count),
                bridges: Vec::with_capacity(edge_count),
                dfs_root: 0,
                pending_enter: None,
                stack: Vec::with_capacity(node_count),
                time: 0,
            },
        }
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        self.state.clone()
    }

    pub(crate) fn restore(checkpoint: Checkpoint) -> Result<Self, BridgeError> {
        validate_checkpoint(&checkpoint)?;
        Ok(Self { state: checkpoint })
    }

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
                Phase::BuildAdjacency => self.advance_adjacency(),
                Phase::SortAdjacency => self.advance_sort(),
                Phase::FindBridges => self.advance_dfs(),
                Phase::Done => break,
            }
            used += 1;
        }
        AdvanceResult {
            work_used: used,
            done: self.state.phase == Phase::Done,
        }
    }

    pub(crate) fn bridges(&self) -> Option<&[bool]> {
        (self.state.phase == Phase::Done).then_some(self.state.bridges.as_slice())
    }

    /// Move the completed indexed graph and bridge flags into topology setup.
    pub(crate) fn into_result(self) -> Result<(Vec<(usize, usize)>, Vec<bool>), BridgeError> {
        if self.state.phase != Phase::Done {
            return Err(BridgeError::InvalidCheckpoint);
        }
        Ok((self.state.edges, self.state.bridges))
    }

    pub(crate) fn matches_graph(
        &self,
        nodes: &[crate::road_faces::RoadFaceNode],
        edges: &[crate::road_faces::RoadFaceEdge],
    ) -> bool {
        self.state.node_count == nodes.len()
            && self.state.edges.len() == edges.len()
            && self
                .state
                .edges
                .iter()
                .enumerate()
                .all(|(index, &(from, to))| {
                    nodes
                        .get(from)
                        .is_some_and(|node| node.id == edges[index].from)
                        && nodes.get(to).is_some_and(|node| node.id == edges[index].to)
                })
    }

    fn advance_initialize(&mut self) {
        let cp = &mut self.state;
        let total = cp.node_count.max(cp.edges.len());
        if cp.initialize_cursor >= total {
            cp.phase = Phase::BuildAdjacency;
            return;
        }
        let index = cp.initialize_cursor;
        if index < cp.node_count {
            cp.adjacency.push(Vec::new());
            cp.entered.push(0);
            cp.low.push(0);
        }
        if index < cp.edges.len() {
            cp.bridges.push(false);
        }
        cp.initialize_cursor += 1;
    }

    fn advance_adjacency(&mut self) {
        let cp = &mut self.state;
        if cp.build_edge >= cp.edges.len() {
            cp.phase = Phase::SortAdjacency;
            cp.sort_node = 0;
            cp.sort_index = 1;
            cp.sort_compare = 0;
            cp.sort_key = None;
            cp.sort_shifts = 0;
            return;
        }
        let (from, to) = cp.edges[cp.build_edge];
        if cp.build_endpoint == 0 {
            cp.adjacency[from].push((to, cp.build_edge));
            cp.build_endpoint = 1;
        } else {
            cp.adjacency[to].push((from, cp.build_edge));
            cp.build_endpoint = 0;
            cp.build_edge += 1;
        }
    }

    fn advance_sort(&mut self) {
        let cp = &mut self.state;
        if cp.sort_node >= cp.node_count {
            cp.phase = Phase::FindBridges;
            return;
        }
        let len = cp.adjacency[cp.sort_node].len();
        if cp.sort_key.is_none() {
            if cp.sort_index >= len {
                cp.sort_node += 1;
                cp.sort_index = 1;
                cp.sort_compare = 0;
                cp.sort_shifts = 0;
                return;
            }
            cp.sort_key = Some(cp.adjacency[cp.sort_node][cp.sort_index]);
            cp.sort_compare = cp.sort_index;
            cp.sort_shifts = 0;
            return;
        }

        let key = cp.sort_key.expect("sort key set");
        if cp.sort_compare > 0 && cp.adjacency[cp.sort_node][cp.sort_compare - 1] > key {
            cp.adjacency[cp.sort_node][cp.sort_compare] =
                cp.adjacency[cp.sort_node][cp.sort_compare - 1];
            cp.sort_compare -= 1;
            cp.sort_shifts += 1;
            return;
        }
        cp.adjacency[cp.sort_node][cp.sort_compare] = key;
        cp.sort_index += 1;
        cp.sort_compare = 0;
        cp.sort_key = None;
        cp.sort_shifts = 0;
    }

    fn advance_dfs(&mut self) {
        let cp = &mut self.state;
        if let Some((node, parent_edge)) = cp.pending_enter.take() {
            cp.time += 1;
            cp.entered[node] = cp.time;
            cp.low[node] = cp.time;
            cp.stack.push(DfsFrame {
                node,
                parent_edge,
                next_neighbor: 0,
            });
            return;
        }

        if cp.stack.is_empty() {
            if cp.dfs_root >= cp.node_count {
                cp.phase = Phase::Done;
                return;
            }
            let root = cp.dfs_root;
            cp.dfs_root += 1;
            if cp.entered[root] == 0 {
                cp.pending_enter = Some((root, None));
            }
            return;
        }

        let top = cp.stack.len() - 1;
        let node = cp.stack[top].node;
        let neighbor_index = cp.stack[top].next_neighbor;
        if neighbor_index < cp.adjacency[node].len() {
            let (next, edge) = cp.adjacency[node][neighbor_index];
            cp.stack[top].next_neighbor += 1;
            if Some(edge) == cp.stack[top].parent_edge {
                return;
            }
            if cp.entered[next] == 0 {
                cp.pending_enter = Some((next, Some(edge)));
            } else {
                cp.low[node] = cp.low[node].min(cp.entered[next]);
            }
            return;
        }

        let finished = cp.stack.pop().expect("stack nonempty");
        if let Some(parent) = cp.stack.last_mut() {
            cp.low[parent.node] = cp.low[parent.node].min(cp.low[finished.node]);
            if cp.low[finished.node] > cp.entered[parent.node] {
                let parent_edge = finished.parent_edge.expect("nonroot frame has parent edge");
                cp.bridges[parent_edge] = true;
            }
        }
    }
}

fn validate_checkpoint(cp: &Checkpoint) -> Result<(), BridgeError> {
    if cp.schema != CHECKPOINT_SCHEMA
        || cp.node_count > MAX_NODES
        || cp.edges.len() > MAX_EDGES
        || cp
            .edges
            .iter()
            .any(|&(from, to)| from >= cp.node_count || to >= cp.node_count)
    {
        return Err(BridgeError::InvalidCheckpoint);
    }
    if cp.phase == Phase::Initialize {
        let limit = cp.node_count.max(cp.edges.len());
        let initialized_nodes = cp.initialize_cursor.min(cp.node_count);
        let initialized_edges = cp.initialize_cursor.min(cp.edges.len());
        if cp.initialize_cursor > limit
            || cp.adjacency.len() != initialized_nodes
            || cp.entered.len() != initialized_nodes
            || cp.low.len() != initialized_nodes
            || cp.bridges.len() != initialized_edges
            || cp.adjacency.iter().any(|neighbors| !neighbors.is_empty())
            || cp.entered.iter().any(|&value| value != 0)
            || cp.low.iter().any(|&value| value != 0)
            || cp.bridges.iter().any(|&is_bridge| is_bridge)
            || cp.build_edge != 0
            || cp.build_endpoint != 0
            || cp.sort_node != 0
            || cp.sort_index != 1
            || cp.sort_compare != 0
            || cp.sort_key.is_some()
            || !cp.stack.is_empty()
            || cp.time != 0
            || cp.dfs_root != 0
            || cp.pending_enter.is_some()
        {
            return Err(BridgeError::InvalidCheckpoint);
        }
        return Ok(());
    }
    if cp.adjacency.len() != cp.node_count
        || cp.entered.len() != cp.node_count
        || cp.low.len() != cp.node_count
        || cp.bridges.len() != cp.edges.len()
        || cp
            .adjacency
            .iter()
            .flatten()
            .any(|&(neighbor, edge)| neighbor >= cp.node_count || edge >= cp.edges.len())
        || cp.stack.len() > cp.node_count
        || cp.time > cp.node_count
        || cp.dfs_root > cp.node_count
        || cp.build_edge > cp.edges.len()
        || cp.build_endpoint > 1
        || cp.sort_node > cp.node_count
        || cp.phase == Phase::SortAdjacency
            && (cp.sort_node == cp.node_count && cp.sort_key.is_some()
                || cp.sort_node < cp.node_count
                    && (cp.sort_index > cp.adjacency[cp.sort_node].len().saturating_add(1)
                        || cp.sort_compare > cp.sort_index
                        || cp.sort_key.is_some()
                            && (cp.sort_index >= cp.adjacency[cp.sort_node].len()
                                || cp.sort_compare > cp.sort_index)))
    {
        return Err(BridgeError::InvalidCheckpoint);
    }
    match cp.phase {
        Phase::Initialize => unreachable!(),
        Phase::BuildAdjacency => {
            let mut expected = vec![Vec::new(); cp.node_count];
            for (edge_index, &(from, to)) in cp.edges.iter().take(cp.build_edge).enumerate() {
                expected[from].push((to, edge_index));
                expected[to].push((from, edge_index));
            }
            if cp.build_endpoint == 1 {
                if let Some(&(from, to)) = cp.edges.get(cp.build_edge) {
                    expected[from].push((to, cp.build_edge));
                }
            }
            if cp.adjacency != expected
                || cp.build_edge >= cp.edges.len() && cp.build_endpoint != 0
                || cp.sort_key.is_some()
                || !cp.stack.is_empty()
            {
                return Err(BridgeError::InvalidCheckpoint);
            }
        }
        Phase::SortAdjacency => {
            let expected_entries = cp.edges.len().saturating_mul(2);
            let mut expected_adjacency = vec![Vec::new(); cp.node_count];
            for (edge_index, &(from, to)) in cp.edges.iter().enumerate() {
                expected_adjacency[from].push((to, edge_index));
                expected_adjacency[to].push((from, edge_index));
            }
            let adjacency_matches = cp
                .adjacency
                .iter()
                .zip(&expected_adjacency)
                .enumerate()
                .all(|(node, (actual, expected))| {
                    let mut logical = actual.clone();
                    if node == cp.sort_node {
                        if let Some(key) = cp.sort_key {
                            let mut restored = Vec::with_capacity(logical.len());
                            restored.extend_from_slice(&logical[..cp.sort_compare]);
                            restored.push(key);
                            restored.extend_from_slice(
                                &logical[cp.sort_compare + 1..cp.sort_index + 1],
                            );
                            restored.extend_from_slice(&logical[cp.sort_index + 1..]);
                            logical = restored;
                        }
                    }
                    let mut expected = expected.clone();
                    logical.sort_unstable();
                    expected.sort_unstable();
                    logical == expected
                });
            if cp.adjacency.iter().map(Vec::len).sum::<usize>() != expected_entries
                || !adjacency_matches
                || cp.build_edge != cp.edges.len()
                || cp.build_endpoint != 0
                || cp.sort_node > cp.node_count
                || cp.sort_index
                    > cp.adjacency
                        .get(cp.sort_node)
                        .map_or(0, Vec::len)
                        .saturating_add(1)
                || cp.sort_compare > cp.sort_index
                || cp.sort_key.is_some_and(|_| {
                    cp.sort_node >= cp.node_count
                        || cp.sort_index >= cp.adjacency[cp.sort_node].len()
                        || cp.sort_compare > cp.sort_index
                        || cp.sort_shifts != cp.sort_index - cp.sort_compare
                })
                || cp.sort_key.is_none() && (cp.sort_compare != 0 || cp.sort_shifts != 0)
                || !cp.stack.is_empty()
            {
                return Err(BridgeError::InvalidCheckpoint);
            }
        }
        Phase::FindBridges | Phase::Done => {
            let frames_in_bounds = cp.stack.iter().all(|frame| {
                frame.node < cp.node_count
                    && frame.next_neighbor <= cp.adjacency[frame.node].len()
                    && frame.parent_edge.is_none_or(|edge| edge < cp.edges.len())
            });
            if !frames_in_bounds {
                return Err(BridgeError::InvalidCheckpoint);
            }
            let mut entered_times = cp
                .entered
                .iter()
                .copied()
                .filter(|&time| time != 0)
                .collect::<Vec<_>>();
            entered_times.sort_unstable();
            let time_sequence_valid = entered_times.len() == cp.time
                && entered_times
                    .iter()
                    .enumerate()
                    .all(|(index, &time)| time == index + 1);
            let stack_valid = cp.stack.iter().enumerate().all(|(index, frame)| {
                if index == 0 {
                    frame.parent_edge.is_none()
                } else {
                    let parent = cp.stack[index - 1].node;
                    frame.parent_edge.is_some_and(|edge| {
                        cp.edges.get(edge).is_some_and(|&(from, to)| {
                            (from == parent && to == frame.node)
                                || (to == parent && from == frame.node)
                        }) && cp.entered[parent] < cp.entered[frame.node]
                    })
                }
            }) && !cp.stack.iter().enumerate().any(|(index, frame)| {
                cp.stack[..index]
                    .iter()
                    .any(|prior| prior.node == frame.node)
            });
            if cp
                .adjacency
                .iter()
                .any(|neighbors| !neighbors.windows(2).all(|pair| pair[0] <= pair[1]))
                || cp.build_edge != cp.edges.len()
                || cp.build_endpoint != 0
                || cp.sort_node != cp.node_count
                || cp.sort_key.is_some()
                || cp.dfs_root > cp.node_count
                || cp.pending_enter.is_some_and(|(node, edge)| {
                    node >= cp.node_count || edge.is_some_and(|e| e >= cp.edges.len())
                })
                || cp.stack.iter().enumerate().any(|(index, frame)| {
                    cp.entered[frame.node] == 0
                        || (index == 0 && frame.parent_edge.is_some())
                        || frame.parent_edge.is_some_and(|edge| edge >= cp.edges.len())
                })
                || cp.entered.iter().any(|&time| time > cp.time)
                || cp.low.iter().zip(&cp.entered).any(|(&low, &entered)| {
                    low > entered || entered == 0 && low != 0 || entered != 0 && low == 0
                })
                || !time_sequence_valid
                || !stack_valid
                || cp.pending_enter.is_some_and(|(node, parent_edge)| {
                    cp.entered[node] != 0
                        || match (cp.stack.last(), parent_edge) {
                            (None, None) => false,
                            (Some(parent), Some(edge)) => {
                                !cp.edges.get(edge).is_some_and(|&(from, to)| {
                                    (from == parent.node && to == node)
                                        || (to == parent.node && from == node)
                                })
                            }
                            _ => true,
                        }
                })
                || cp.phase == Phase::Done
                    && (!cp.stack.is_empty()
                        || cp.pending_enter.is_some()
                        || cp.dfs_root != cp.node_count
                        || cp.time != cp.node_count)
            {
                return Err(BridgeError::InvalidCheckpoint);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn drain(node_count: usize, edges: Vec<(usize, usize)>, quotas: &[usize]) -> Vec<bool> {
        let expected_set = crate::road_faces::graph_bridge_edges(node_count, &edges);
        let expected = (0..edges.len())
            .map(|index| expected_set.contains(&index))
            .collect::<Vec<_>>();
        let mut search = BridgeSearch::new(node_count, edges).unwrap();
        let mut turn = 0;
        while search.bridges().is_none() {
            let quota = quotas[turn % quotas.len()];
            let before = search.state.clone();
            let result = search.advance(quota);
            assert!(result.work_used <= quota);
            if result.work_used == 0 && !result.done {
                panic!("search made no progress: {:?}", before.phase);
            }
            let saved = serde_json::to_vec(&search.checkpoint()).unwrap();
            let checkpoint: Checkpoint = serde_json::from_slice(&saved).unwrap();
            search = BridgeSearch::restore(checkpoint.clone()).unwrap_or_else(|error| {
                panic!("invalid checkpoint after turn {turn}: {error:?}; state={checkpoint:?}")
            });
            turn += 1;
            assert!(turn < 2_000_000);
        }
        assert_eq!(search.bridges().unwrap(), expected);
        expected
    }

    #[test]
    fn tarjan_matches_reference_graph_shapes_and_parallel_edges() {
        let cases = [
            (0, vec![]),
            (4, vec![(0, 1), (1, 2), (2, 3)]),
            (4, vec![(0, 1), (1, 2), (2, 0), (2, 3)]),
            (
                7,
                vec![(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (5, 6)],
            ),
            (3, vec![(0, 1), (0, 1), (1, 2)]),
            (3, vec![(0, 0), (0, 1)]),
        ];
        for (nodes, edges) in cases {
            for quotas in [&[1][..], &[7][..], &[48][..], &[3, 1, 11, 2][..]] {
                drain(nodes, edges.clone(), quotas);
            }
        }
    }

    #[test]
    fn malformed_inputs_and_checkpoints_fail_closed() {
        assert_eq!(
            BridgeSearch::new(MAX_NODES + 1, vec![]).unwrap_err(),
            BridgeError::TooManyNodes
        );
        assert_eq!(
            BridgeSearch::new(1, vec![(0, 0); MAX_EDGES + 1]).unwrap_err(),
            BridgeError::TooManyEdges
        );
        assert_eq!(
            BridgeSearch::new(2, vec![(0, 2)]).unwrap_err(),
            BridgeError::InvalidEndpoint
        );

        let mut search = BridgeSearch::new(4, vec![(0, 1), (1, 2), (2, 0), (2, 3)]).unwrap();
        while search.state.phase != Phase::SortAdjacency {
            search.advance(1);
        }
        let mut bad = search.checkpoint();
        bad.sort_node = bad.node_count + 1;
        assert_eq!(
            BridgeSearch::restore(bad).unwrap_err(),
            BridgeError::InvalidCheckpoint
        );
        let mut bad = search.checkpoint();
        bad.adjacency[0][0].1 = bad.edges.len();
        assert_eq!(
            BridgeSearch::restore(bad).unwrap_err(),
            BridgeError::InvalidCheckpoint
        );
        let mut bad = search.checkpoint();
        bad.sort_index = usize::MAX;
        assert_eq!(
            BridgeSearch::restore(bad).unwrap_err(),
            BridgeError::InvalidCheckpoint
        );

        while search.state.phase != Phase::FindBridges {
            search.advance(1);
        }
        let mut bad = search.checkpoint();
        bad.pending_enter = Some((bad.node_count, None));
        assert_eq!(
            BridgeSearch::restore(bad).unwrap_err(),
            BridgeError::InvalidCheckpoint
        );
        let mut bad = search.checkpoint();
        bad.stack.push(DfsFrame {
            node: 0,
            parent_edge: None,
            next_neighbor: 0,
        });
        assert_eq!(
            BridgeSearch::restore(bad).unwrap_err(),
            BridgeError::InvalidCheckpoint
        );
        while search.state.stack.len() < 2 {
            search.advance(1);
        }
        let mut bad = search.checkpoint();
        bad.stack[1].parent_edge = Some(bad.edges.len());
        assert_eq!(
            BridgeSearch::restore(bad).unwrap_err(),
            BridgeError::InvalidCheckpoint
        );
        let mut bad = search.checkpoint();
        bad.stack[1].node = usize::MAX;
        assert_eq!(
            BridgeSearch::restore(bad).unwrap_err(),
            BridgeError::InvalidCheckpoint
        );
    }

    #[test]
    fn zero_budget_and_finished_calls_are_noops() {
        let mut search = BridgeSearch::new(3, vec![(0, 1), (1, 2)]).unwrap();
        let checkpoint = search.checkpoint();
        assert_eq!(
            search.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        assert_eq!(search.checkpoint(), checkpoint);
        while search.bridges().is_none() {
            search.advance(7);
        }
        assert_eq!(
            search.advance(48),
            AdvanceResult {
                work_used: 0,
                done: true
            }
        );
    }

    #[test]
    fn capped_chain_drains_without_recursion_or_quota_overrun() {
        let edges = (0..MAX_NODES - 1)
            .map(|node| (node, node + 1))
            .collect::<Vec<_>>();
        let bridges = drain(MAX_NODES, edges, &[48, 1, 7]);
        assert_eq!(bridges.len(), MAX_NODES - 1);
        assert!(bridges.into_iter().all(|bridge| bridge));
    }
}
