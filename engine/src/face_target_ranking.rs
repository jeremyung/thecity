//! Bounded, resumable ranking and spacing of face-center target candidates.
//!
//! The stage mirrors the mature exact oracle: score every input center against
//! parcel centers (minus half-diagonal) and road segments, stably sort by score
//! descending then X/Z ascending, then accept candidates at least 20m from any
//! node and 18m from previously selected targets. Inputs are frozen in the
//! checkpoint; callers should use `restore`, never deserialize runtime state.

use crate::face_target_viability::Segment;
use crate::geometry::{self, OrientedRect};
use serde::{Deserialize, Serialize};

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_CENTERS: usize = 576;
const MAX_PARCELS: usize = crate::MAX_PARCELS * 2;
const MAX_ROADS: usize = crate::MAX_SEGMENTS;
const MAX_NODES: usize = crate::MAX_SEGMENTS * 2;
const MAX_COORDINATE: f32 = 100_000.0;
const TARGET_LIMIT: usize = 12;
// Worst case is bounded by score pairs (576 * (8,000 + 5,000)),
// insertion-sort comparisons, and node/selected-target spacing checks
// (576 * 10,000 + 576 * 12), plus at most a few constant transitions per
// item. No advance call can exceed the caller's explicit quota.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RankingError {
    InputTooLarge,
    InvalidInput,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    ScoreParcels,
    ScoreRoads,
    RecordScore,
    SortStart,
    SortCompare,
    SelectStart,
    SelectNodes,
    SelectTargets,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct RankedCenter {
    /// None denotes +infinity, which occurs when no parcels or roads exist.
    score: Option<f32>,
    x: f32,
    z: f32,
    input_index: usize,
}

/// Versioned serialized data transfer object. Runtime `TargetRanking` does
/// not derive Deserialize, so restoration always validates cursors and inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    schema: u8,
    centers: Vec<(f32, f32)>,
    parcels: Vec<OrientedRect>,
    roads: Vec<Segment>,
    nodes: Vec<(f32, f32)>,
    phase: Phase,

    center_index: usize,
    parcel_index: usize,
    road_index: usize,
    current_score: Option<f32>,

    scored: Vec<RankedCenter>,
    sort_index: usize,
    sort_compare: usize,
    sort_key: Option<RankedCenter>,

    candidate_index: usize,
    node_index: usize,
    selected_index: usize,
    candidate_near_node: bool,
    candidate_near_target: bool,
    selected: Vec<(f32, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TargetRanking {
    state: Checkpoint,
}

impl TargetRanking {
    pub(crate) fn new(
        centers: Vec<(f32, f32)>,
        parcels: Vec<OrientedRect>,
        roads: Vec<Segment>,
        nodes: Vec<(f32, f32)>,
    ) -> Result<Self, RankingError> {
        validate_inputs(&centers, &parcels, &roads, &nodes)?;
        Ok(Self {
            state: Checkpoint {
                schema: CHECKPOINT_SCHEMA,
                centers,
                parcels,
                roads,
                nodes,
                phase: Phase::ScoreParcels,
                center_index: 0,
                parcel_index: 0,
                road_index: 0,
                current_score: None,
                scored: Vec::with_capacity(MAX_CENTERS),
                sort_index: 1,
                sort_compare: 0,
                sort_key: None,
                candidate_index: 0,
                node_index: 0,
                selected_index: 0,
                candidate_near_node: false,
                candidate_near_target: false,
                selected: Vec::with_capacity(TARGET_LIMIT),
            },
        })
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        self.state.clone()
    }

    pub(crate) fn restore(state: Checkpoint) -> Result<Self, RankingError> {
        validate_checkpoint(&state)?;
        Ok(Self { state })
    }

    pub(crate) fn targets(&self) -> Option<&[(f32, f32)]> {
        (self.state.phase == Phase::Done).then_some(self.state.selected.as_slice())
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
            self.advance_one();
            used += 1;
        }
        AdvanceResult {
            work_used: used,
            done: self.state.phase == Phase::Done,
        }
    }

    fn advance_one(&mut self) {
        match self.state.phase {
            Phase::ScoreParcels => self.score_parcel(),
            Phase::ScoreRoads => self.score_road(),
            Phase::RecordScore => self.record_score(),
            Phase::SortStart => self.start_sort(),
            Phase::SortCompare => self.sort_compare(),
            Phase::SelectStart => self.start_candidate(),
            Phase::SelectNodes => self.scan_node(),
            Phase::SelectTargets => self.scan_target(),
            Phase::Done => {}
        }
    }

    fn score_parcel(&mut self) {
        let state = &mut self.state;
        if state.centers.is_empty() {
            state.phase = Phase::SortStart;
            return;
        }
        if state.center_index >= state.centers.len() {
            state.sort_index = 1;
            state.phase = if state.scored.len() > 1 {
                Phase::SortStart
            } else {
                Phase::SelectStart
            };
            return;
        }
        if state.parcels.is_empty() {
            state.phase = if state.roads.is_empty() {
                Phase::RecordScore
            } else {
                Phase::ScoreRoads
            };
            return;
        }
        let (x, z) = state.centers[state.center_index];
        let parcel = state.parcels[state.parcel_index];
        let clearance =
            ((x - parcel.x).hypot(z - parcel.z) - 0.5 * parcel.width.hypot(parcel.depth)).max(0.0);
        state.current_score = Some(
            state
                .current_score
                .map_or(clearance, |score| score.min(clearance)),
        );
        state.parcel_index += 1;
        if state.parcel_index == state.parcels.len() {
            state.parcel_index = 0;
            state.phase = if state.roads.is_empty() {
                Phase::RecordScore
            } else {
                Phase::ScoreRoads
            };
        }
    }

    fn score_road(&mut self) {
        let state = &mut self.state;
        if state.roads.is_empty() {
            state.phase = Phase::RecordScore;
            return;
        }
        let (x, z) = state.centers[state.center_index];
        let road = state.roads[state.road_index];
        let clearance = geometry::point_segment_distance(x, z, road.ax, road.az, road.bx, road.bz);
        state.current_score = Some(
            state
                .current_score
                .map_or(clearance, |score| score.min(clearance)),
        );
        state.road_index += 1;
        if state.road_index == state.roads.len() {
            state.road_index = 0;
            state.phase = Phase::RecordScore;
        }
    }

    fn record_score(&mut self) {
        let state = &mut self.state;
        let (x, z) = state.centers[state.center_index];
        state.scored.push(RankedCenter {
            score: state.current_score,
            x,
            z,
            input_index: state.center_index,
        });
        state.current_score = None;
        state.center_index += 1;
        if state.center_index == state.centers.len() {
            state.sort_index = 1;
            state.phase = if state.scored.len() > 1 {
                Phase::SortStart
            } else {
                Phase::SelectStart
            };
        } else {
            state.phase = Phase::ScoreParcels;
        }
    }

    fn start_sort(&mut self) {
        if self.state.sort_index >= self.state.scored.len() {
            self.state.phase = Phase::SelectStart;
            self.state.candidate_index = 0;
            return;
        }
        self.state.sort_key = Some(self.state.scored[self.state.sort_index]);
        self.state.sort_compare = self.state.sort_index;
        self.state.phase = Phase::SortCompare;
    }

    fn sort_compare(&mut self) {
        let key = self.state.sort_key.expect("sort key initialized");
        if self.state.sort_compare == 0 {
            self.state.scored[0] = key;
            self.finish_sort_item();
            return;
        }
        let previous_index = self.state.sort_compare - 1;
        let previous = self.state.scored[previous_index];
        if rank_order(&key, &previous).is_lt() {
            self.state.scored[self.state.sort_compare] = previous;
            self.state.sort_compare = previous_index;
        } else {
            self.state.scored[self.state.sort_compare] = key;
            self.finish_sort_item();
        }
    }

    fn finish_sort_item(&mut self) {
        self.state.sort_index += 1;
        self.state.sort_key = None;
        self.state.phase = if self.state.sort_index >= self.state.scored.len() {
            self.state.candidate_index = 0;
            Phase::SelectStart
        } else {
            Phase::SortStart
        };
    }

    fn start_candidate(&mut self) {
        if self.state.candidate_index >= self.state.scored.len()
            || self.state.selected.len() == TARGET_LIMIT
        {
            self.state.phase = Phase::Done;
            return;
        }
        self.state.node_index = 0;
        self.state.selected_index = 0;
        self.state.candidate_near_node = false;
        self.state.candidate_near_target = false;
        self.state.phase = if self.state.nodes.is_empty() {
            Phase::SelectTargets
        } else {
            Phase::SelectNodes
        };
    }

    fn scan_node(&mut self) {
        let candidate = self.state.scored[self.state.candidate_index];
        if self.state.node_index >= self.state.nodes.len() {
            self.state.phase = if self.state.candidate_near_node {
                self.next_candidate_phase()
            } else {
                Phase::SelectTargets
            };
            return;
        }
        let (x, z) = self.state.nodes[self.state.node_index];
        self.state.node_index += 1;
        if (x - candidate.x).hypot(z - candidate.z) < 20.0 {
            self.state.candidate_near_node = true;
            self.state.phase = self.next_candidate_phase();
        }
    }

    fn scan_target(&mut self) {
        let candidate = self.state.scored[self.state.candidate_index];
        if self.state.selected_index >= self.state.selected.len() {
            if !self.state.candidate_near_target {
                self.state.selected.push((candidate.x, candidate.z));
            }
            self.state.phase = self.next_candidate_phase();
            return;
        }
        let (x, z) = self.state.selected[self.state.selected_index];
        self.state.selected_index += 1;
        if (x - candidate.x).hypot(z - candidate.z) < 18.0 {
            self.state.candidate_near_target = true;
            self.state.phase = self.next_candidate_phase();
        }
    }

    fn next_candidate_phase(&mut self) -> Phase {
        self.state.candidate_index += 1;
        self.state.node_index = 0;
        self.state.selected_index = 0;
        self.state.candidate_near_node = false;
        self.state.candidate_near_target = false;
        if self.state.candidate_index >= self.state.scored.len()
            || self.state.selected.len() == TARGET_LIMIT
        {
            Phase::Done
        } else {
            Phase::SelectStart
        }
    }
}

fn rank_order(left: &RankedCenter, right: &RankedCenter) -> std::cmp::Ordering {
    let score_order = match (left.score, right.score) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(left), Some(right)) => right.total_cmp(&left),
    };
    score_order
        .then_with(|| left.x.total_cmp(&right.x))
        .then_with(|| left.z.total_cmp(&right.z))
        .then_with(|| left.input_index.cmp(&right.input_index))
}

fn validate_inputs(
    centers: &[(f32, f32)],
    parcels: &[OrientedRect],
    roads: &[Segment],
    nodes: &[(f32, f32)],
) -> Result<(), RankingError> {
    if centers.len() > MAX_CENTERS
        || parcels.len() > MAX_PARCELS
        || roads.len() > MAX_ROADS
        || nodes.len() > MAX_NODES
    {
        return Err(RankingError::InputTooLarge);
    }
    if centers
        .iter()
        .chain(nodes)
        .any(|&(x, z)| !finite_coordinate(x) || !finite_coordinate(z))
        || parcels.iter().any(|rect| {
            !rect.finite_positive()
                || !finite_coordinate(rect.x)
                || !finite_coordinate(rect.z)
                || [
                    rect.bounds().0,
                    rect.bounds().1,
                    rect.bounds().2,
                    rect.bounds().3,
                ]
                .iter()
                .any(|value| !value.is_finite())
        })
        || roads.iter().any(|road| {
            !finite_coordinate(road.ax)
                || !finite_coordinate(road.az)
                || !finite_coordinate(road.bx)
                || !finite_coordinate(road.bz)
        })
    {
        return Err(RankingError::InvalidInput);
    }
    Ok(())
}

fn finite_coordinate(value: f32) -> bool {
    value.is_finite() && value.abs() <= MAX_COORDINATE
}

fn validate_checkpoint(state: &Checkpoint) -> Result<(), RankingError> {
    validate_inputs(&state.centers, &state.parcels, &state.roads, &state.nodes)
        .map_err(|_| RankingError::InvalidCheckpoint)?;
    if state.schema != CHECKPOINT_SCHEMA
        || state.scored.len() > state.centers.len()
        || state.selected.len() > TARGET_LIMIT
        || state.scored.iter().any(|item| {
            item.input_index >= state.centers.len()
                || !finite_coordinate(item.x)
                || !finite_coordinate(item.z)
                || item
                    .score
                    .is_some_and(|score| !score.is_finite() || score < 0.0)
                || state.centers.get(item.input_index) != Some(&(item.x, item.z))
        })
        || state
            .selected
            .iter()
            .any(|&(x, z)| !finite_coordinate(x) || !finite_coordinate(z))
        || state.center_index > state.centers.len()
        || state.parcel_index > state.parcels.len()
        || state.road_index > state.roads.len()
        || state
            .current_score
            .is_some_and(|score| !score.is_finite() || score < 0.0)
        || state.sort_compare > state.sort_index
        || state.sort_key.is_some_and(|key| {
            key.input_index >= state.centers.len()
                || state.centers.get(key.input_index) != Some(&(key.x, key.z))
                || key
                    .score
                    .is_some_and(|score| !score.is_finite() || score < 0.0)
        })
        || state.candidate_index > state.scored.len()
        || state.node_index > state.nodes.len()
        || state.selected_index > state.selected.len()
        || state.phase == Phase::Done
            && (state.candidate_index < state.scored.len() && state.selected.len() < TARGET_LIMIT)
        || state.candidate_near_node
        || state.candidate_near_target
    {
        return Err(RankingError::InvalidCheckpoint);
    }
    let mut indices = state
        .scored
        .iter()
        .map(|item| item.input_index)
        .collect::<Vec<_>>();
    indices.sort_unstable();
    let duplicate_indices = indices.windows(2).filter(|pair| pair[0] == pair[1]).count();
    let duplicate_shape_valid = if state.phase != Phase::SortCompare {
        duplicate_indices == 0
    } else if state.sort_compare == state.sort_index {
        duplicate_indices == 0
            && state.sort_key.is_some_and(|key| {
                state
                    .scored
                    .get(state.sort_index)
                    .is_some_and(|item| item.input_index == key.input_index)
            })
    } else {
        let hole_id = state.sort_key.map(|key| key.input_index);
        duplicate_indices == 1
            && state.sort_compare < state.sort_index
            && state
                .scored
                .get(state.sort_compare)
                .zip(state.scored.get(state.sort_compare + 1))
                .is_some_and(|(left, right)| left.input_index == right.input_index)
            && hole_id.is_some_and(|id| indices.binary_search(&id).is_err())
    };
    if !duplicate_shape_valid {
        return Err(RankingError::InvalidCheckpoint);
    }
    let score_complete = state.scored.len() == state.centers.len();
    let cursors_valid_for_phase = match state.phase {
        Phase::ScoreParcels => {
            (state.centers.is_empty() || state.center_index < state.centers.len())
                && (state.parcels.is_empty() && state.parcel_index == 0
                    || !state.parcels.is_empty() && state.parcel_index < state.parcels.len())
                && state.scored.len() == state.center_index
                && state.road_index == 0
                && state.sort_key.is_none()
        }
        Phase::ScoreRoads => {
            state.center_index < state.centers.len()
                && !state.roads.is_empty()
                && state.road_index < state.roads.len()
                && state.parcel_index == 0
                && state.scored.len() == state.center_index
                && state.sort_key.is_none()
        }
        Phase::RecordScore => {
            state.center_index < state.centers.len()
                && state.scored.len() == state.center_index
                && state.parcel_index == 0
                && state.road_index == 0
                && state.sort_key.is_none()
        }
        Phase::SortStart => {
            score_complete
                && state.sort_key.is_none()
                && (state.scored.len() < 2 && state.sort_index == 1
                    || state.scored.len() >= 2
                        && state.sort_index >= 1
                        && state.sort_index < state.scored.len())
        }
        Phase::SortCompare => {
            score_complete
                && state.sort_index >= 1
                && state.sort_index < state.scored.len()
                && state.sort_compare <= state.sort_index
                && state
                    .sort_key
                    .is_some_and(|key| key.input_index == state.sort_index)
        }
        Phase::SelectStart => {
            score_complete
                && state.sort_key.is_none()
                && state.candidate_index <= state.scored.len()
                && state.selected.len() < TARGET_LIMIT
        }
        Phase::SelectNodes => {
            state.candidate_index < state.scored.len()
                && !state.nodes.is_empty()
                && state.node_index <= state.nodes.len()
                && state.selected_index == 0
        }
        Phase::SelectTargets => {
            state.candidate_index < state.scored.len()
                && state.node_index <= state.nodes.len()
                && state.selected_index <= state.selected.len()
        }
        Phase::Done => {
            state.candidate_index == state.scored.len() || state.selected.len() == TARGET_LIMIT
        }
    };
    if !cursors_valid_for_phase {
        return Err(RankingError::InvalidCheckpoint);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, z: f32, width: f32, depth: f32, rotation: f32) -> OrientedRect {
        OrientedRect {
            x,
            z,
            width,
            depth,
            rotation,
        }
    }

    fn reference(
        centers: &[(f32, f32)],
        parcels: &[OrientedRect],
        roads: &[Segment],
        nodes: &[(f32, f32)],
    ) -> Vec<(f32, f32)> {
        let mut ranked = centers
            .iter()
            .copied()
            .enumerate()
            .map(|(index, (x, z))| {
                let clearance = parcels
                    .iter()
                    .map(|parcel| {
                        ((x - parcel.x).hypot(z - parcel.z)
                            - 0.5 * parcel.width.hypot(parcel.depth))
                        .max(0.0)
                    })
                    .chain(roads.iter().map(|road| {
                        geometry::point_segment_distance(x, z, road.ax, road.az, road.bx, road.bz)
                    }))
                    .fold(f32::INFINITY, f32::min);
                (clearance, x, z, index)
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.3.cmp(&right.3))
        });
        let mut selected: Vec<(f32, f32)> = Vec::new();
        for (_, x, z, _) in ranked {
            let far_from_nodes = nodes.iter().all(|&(nx, nz)| (nx - x).hypot(nz - z) >= 20.0);
            let far_from_targets = selected
                .iter()
                .all(|&(tx, tz)| (tx - x).hypot(tz - z) >= 18.0);
            if far_from_nodes && far_from_targets {
                selected.push((x, z));
                if selected.len() == TARGET_LIMIT {
                    break;
                }
            }
        }
        selected
    }

    fn run(
        centers: Vec<(f32, f32)>,
        parcels: Vec<OrientedRect>,
        roads: Vec<Segment>,
        nodes: Vec<(f32, f32)>,
        quotas: &[usize],
        resume_every_advance: bool,
    ) -> Vec<(f32, f32)> {
        let expected = reference(&centers, &parcels, &roads, &nodes);
        let mut search = TargetRanking::new(centers, parcels, roads, nodes).unwrap();
        let before = search.checkpoint();
        assert_eq!(
            search.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        assert_eq!(search.checkpoint(), before);
        let mut turn = 0usize;
        while search.targets().is_none() {
            let quota = quotas[turn % quotas.len()];
            let result = search.advance(quota);
            assert!(result.work_used <= quota);
            assert!(result.done || quota == 0 || result.work_used > 0);
            if resume_every_advance && quota != 0 {
                let bytes = serde_json::to_vec(&search.checkpoint()).unwrap();
                let checkpoint: Checkpoint = serde_json::from_slice(&bytes).unwrap();
                search = TargetRanking::restore(checkpoint.clone()).unwrap_or_else(|error| {
                    panic!(
                        "restore failed turn={turn} phase={:?} state={checkpoint:?}: {error:?}",
                        checkpoint.phase
                    )
                });
            }
            turn += 1;
            assert!(turn < 2_000_000, "ranking did not terminate");
        }
        let actual = search.targets().unwrap().to_vec();
        assert_eq!(actual, expected);
        actual
    }

    #[test]
    fn ranking_matches_exact_oracle_across_quotas_and_every_boundary_resume() {
        let centers = vec![
            (-22., 0.),
            (-10., 0.),
            (0., 0.),
            (10., 0.),
            (22., 0.),
            (0., 24.),
            (0., -24.),
            (38., 8.),
        ];
        let parcels = vec![rect(0., 0., 12., 8., 0.31), rect(25., 4., 14., 10., -0.8)];
        let roads = vec![
            Segment {
                ax: -40.,
                az: -16.,
                bx: 42.,
                bz: 18.,
            },
            Segment {
                ax: 16.,
                az: -32.,
                bx: 16.,
                bz: 32.,
            },
        ];
        let nodes = vec![(-40., 0.), (42., 40.)];
        let baseline = reference(&centers, &parcels, &roads, &nodes);
        for quotas in [&[1][..], &[7][..], &[48][..], &[3, 0, 1, 11, 2][..]] {
            assert_eq!(
                run(
                    centers.clone(),
                    parcels.clone(),
                    roads.clone(),
                    nodes.clone(),
                    quotas,
                    true
                ),
                baseline
            );
        }
    }

    #[test]
    fn empty_score_inputs_use_infinity_and_spacing_searches_past_first_twelve() {
        let centers = (0..14)
            .map(|index| (index as f32, 0.))
            .chain([(100., 0.), (130., 0.)])
            .collect::<Vec<_>>();
        let nodes = (0..14).map(|index| (index as f32, 0.)).collect::<Vec<_>>();
        let expected = vec![(100., 0.), (130., 0.)];
        assert_eq!(reference(&centers, &[], &[], &nodes), expected);
        for quotas in [&[1][..], &[7][..], &[48][..], &[4, 1, 3][..]] {
            assert_eq!(
                run(centers.clone(), vec![], vec![], nodes.clone(), quotas, true),
                expected
            );
        }
        assert_eq!(
            run(vec![], vec![], vec![], vec![], &[1], true),
            Vec::<(f32, f32)>::new()
        );
    }

    #[test]
    fn checkpoint_rejects_malformed_cursors_and_inputs() {
        let mut search =
            TargetRanking::new(vec![(0., 0.), (30., 0.)], vec![], vec![], vec![]).unwrap();
        let mut bad = search.checkpoint();
        bad.center_index = usize::MAX;
        assert_eq!(
            TargetRanking::restore(bad).unwrap_err(),
            RankingError::InvalidCheckpoint
        );
        let mut bad = search.checkpoint();
        bad.parcels.push(rect(0., 0., f32::NAN, 3., 0.));
        assert_eq!(
            TargetRanking::restore(bad).unwrap_err(),
            RankingError::InvalidCheckpoint
        );
        let with_parcel = TargetRanking::new(
            vec![(0., 0.)],
            vec![rect(0., 0., 10., 10., 0.)],
            vec![],
            vec![],
        )
        .unwrap();
        let mut bad = with_parcel.checkpoint();
        bad.parcel_index = bad.parcels.len();
        assert_eq!(
            TargetRanking::restore(bad).unwrap_err(),
            RankingError::InvalidCheckpoint
        );
        assert_eq!(
            TargetRanking::new(vec![(f32::INFINITY, 0.)], vec![], vec![], vec![]).unwrap_err(),
            RankingError::InvalidInput
        );
        let _ = search.advance(3);
    }
}
