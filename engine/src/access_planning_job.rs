//! Persisted composition of frozen access discovery and proposal validation.
//!
//! The caller captures both inputs at one geometry revision. This job never
//! reads or mutates Engine state; after a page is reviewed, the caller may
//! advance to the next one and must revalidate world and actor state before
//! live admission.

use crate::access_discovery_job::{
    DiscoveryCheckpoint, DiscoveryError, DiscoveryInput, DiscoveryJob,
};
use crate::access_proposal_validation::{
    InputError, ProposalValidationCheckpoint, ProposalValidationContext, ProposalValidationInput,
    ProposalValidator,
};
use crate::interior_access::{AccessProposal, MAX_PROPOSALS_PER_EVALUATION};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 3;
const PAGE_SIZE: usize = MAX_PROPOSALS_PER_EVALUATION;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Discover,
    Validate,
    Sort,
    Ready,
    Exhausted,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlanningError {
    Discovery(DiscoveryError),
    ProposalInput(InputError),
    SnapshotMismatch,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlanningAdvance {
    pub work_used: usize,
    pub page_ready: bool,
    pub exhausted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AccessPlanningCheckpoint {
    schema: u8,
    geometry_revision: u64,
    discovery: DiscoveryCheckpoint,
    proposal_input: ProposalValidationInput,
    phase: Phase,
    page_start: usize,
    page_end: usize,
    candidate_cursor: usize,
    validator: Option<ProposalValidationCheckpoint>,
    proposals: Vec<AccessProposal>,
    sort_i: usize,
    sort_j: usize,
    failure: Option<DiscoveryError>,
}

#[derive(Debug)]
pub(crate) struct AccessPlanningJob {
    geometry_revision: u64,
    discovery: DiscoveryJob,
    proposal_input: Arc<ProposalValidationContext>,
    phase: Phase,
    page_start: usize,
    page_end: usize,
    candidate_cursor: usize,
    validator: Option<ProposalValidator>,
    proposals: Vec<AccessProposal>,
    sort_i: usize,
    sort_j: usize,
    failure: Option<DiscoveryError>,
}

impl AccessPlanningJob {
    /// Build a job from the two projections captured at the same simulation
    /// boundary. Their full viability arrays must agree before either is used.
    pub(crate) fn new(
        discovery_input: DiscoveryInput,
        proposal_input: ProposalValidationInput,
        geometry_revision: u64,
    ) -> Result<Self, PlanningError> {
        Self::new_excluding(discovery_input, proposal_input, geometry_revision, Vec::new())
    }

    pub(crate) fn new_excluding(
        discovery_input: DiscoveryInput,
        proposal_input: ProposalValidationInput,
        geometry_revision: u64,
        excluded_faces: Vec<Vec<u32>>,
    ) -> Result<Self, PlanningError> {
        if discovery_input.viability != proposal_input.viability {
            return Err(PlanningError::SnapshotMismatch);
        }
        let discovery = DiscoveryJob::new_excluding(discovery_input, excluded_faces).map_err(PlanningError::Discovery)?;
        let proposal_input =
            ProposalValidationContext::new(proposal_input).map_err(PlanningError::ProposalInput)?;
        Ok(Self {
            geometry_revision,
            discovery,
            proposal_input,
            phase: Phase::Discover,
            page_start: 0,
            page_end: 0,
            candidate_cursor: 0,
            validator: None,
            proposals: Vec::with_capacity(PAGE_SIZE),
            sort_i: 0,
            sort_j: 0,
            failure: None,
        })
    }

    pub(crate) fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }

    pub(crate) fn selected_face_identity(&self) -> Option<&[u32]> {
        self.discovery.selected_face_identity()
    }

    pub(crate) fn ready_page(&self) -> Option<&[AccessProposal]> {
        (self.phase == Phase::Ready).then_some(self.proposals.as_slice())
    }

    pub(crate) fn is_exhausted(&self) -> bool {
        self.phase == Phase::Exhausted || self.phase == Phase::Failed
    }

    pub(crate) fn error(&self) -> Option<PlanningError> {
        self.failure.map(PlanningError::Discovery)
    }

    /// Mark the ready page consumed/rejected. Empty ready pages are valid and
    /// must also be continued so an all-rejected descriptor page cannot stall.
    /// Returns true when another descriptor page remains.
    pub(crate) fn continue_next_page(&mut self) -> Result<bool, PlanningError> {
        if self.phase != Phase::Ready {
            return Err(PlanningError::InvalidCheckpoint);
        }
        self.proposals.clear();
        self.sort_i = 0;
        self.sort_j = 0;
        self.validator = None;
        if self.page_end >= self.descriptor_count()? {
            self.page_start = self.page_end;
            self.candidate_cursor = self.page_end;
            self.phase = Phase::Exhausted;
            return Ok(false);
        }
        self.page_start = self.page_end;
        self.candidate_cursor = self.page_start;
        self.page_end = self
            .page_start
            .saturating_add(PAGE_SIZE)
            .min(self.descriptor_count()?);
        self.phase = Phase::Validate;
        Ok(true)
    }

    pub(crate) fn checkpoint(&self) -> AccessPlanningCheckpoint {
        AccessPlanningCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            geometry_revision: self.geometry_revision,
            discovery: self.discovery.checkpoint(),
            proposal_input: self.proposal_input.checkpoint(),
            phase: self.phase,
            page_start: self.page_start,
            page_end: self.page_end,
            candidate_cursor: self.candidate_cursor,
            validator: self.validator.as_ref().map(ProposalValidator::checkpoint),
            proposals: self.proposals.clone(),
            sort_i: self.sort_i,
            sort_j: self.sort_j,
            failure: self.failure,
        }
    }

    pub(crate) fn restore(checkpoint: AccessPlanningCheckpoint) -> Result<Self, PlanningError> {
        if checkpoint.schema != CHECKPOINT_SCHEMA
            || checkpoint.proposals.len() > PAGE_SIZE
            || checkpoint
                .proposals
                .iter()
                .any(|proposal| !valid_proposal(proposal))
        {
            return Err(PlanningError::InvalidCheckpoint);
        }
        let discovery = DiscoveryJob::restore(checkpoint.discovery)
            .map_err(|_| PlanningError::InvalidCheckpoint)?;
        let proposal_input = ProposalValidationContext::new(checkpoint.proposal_input)
            .map_err(|_| PlanningError::InvalidCheckpoint)?;
        let descriptors_status = discovery.descriptors();
        let descriptor_count = match &descriptors_status {
            Some(Ok(descriptors)) => Some(descriptors.len()),
            _ => None,
        };
        if let Some(count) = descriptor_count {
            if checkpoint.page_start > count
                || checkpoint.page_end > count
                || checkpoint.page_start > checkpoint.page_end
                || checkpoint.page_end.saturating_sub(checkpoint.page_start) > PAGE_SIZE
                || checkpoint.candidate_cursor < checkpoint.page_start
                || checkpoint.candidate_cursor > checkpoint.page_end
            {
                return Err(PlanningError::InvalidCheckpoint);
            }
        }
        match checkpoint.phase {
            Phase::Discover => {
                if !matches!(descriptors_status, None)
                    || checkpoint.page_start != 0
                    || checkpoint.page_end != 0
                    || checkpoint.candidate_cursor != 0
                    || checkpoint.validator.is_some()
                    || !checkpoint.proposals.is_empty()
                    || checkpoint.sort_i != 0
                    || checkpoint.sort_j != 0
                    || checkpoint.failure.is_some()
                {
                    return Err(PlanningError::InvalidCheckpoint);
                }
            }
            Phase::Validate => {
                let count = descriptor_count.ok_or(PlanningError::InvalidCheckpoint)?;
                if checkpoint.failure.is_some()
                    || checkpoint.page_start >= count
                    || checkpoint.page_end <= checkpoint.page_start
                    || checkpoint.page_end
                        != checkpoint.page_start.saturating_add(PAGE_SIZE).min(count)
                    || checkpoint.candidate_cursor > checkpoint.page_end
                    || checkpoint.validator.is_some()
                        && checkpoint.candidate_cursor >= checkpoint.page_end
                    || checkpoint.sort_i != 0
                    || checkpoint.sort_j != 0
                {
                    return Err(PlanningError::InvalidCheckpoint);
                }
            }
            Phase::Sort => {
                let count = descriptor_count.ok_or(PlanningError::InvalidCheckpoint)?;
                if checkpoint.failure.is_some()
                    || checkpoint.candidate_cursor != checkpoint.page_end
                    || checkpoint.page_start >= checkpoint.page_end
                    || checkpoint.proposals.len() > checkpoint.page_end - checkpoint.page_start
                    || checkpoint.validator.is_some()
                    || !valid_sort_cursor(
                        checkpoint.sort_i,
                        checkpoint.sort_j,
                        checkpoint.proposals.len(),
                    )
                    || count < checkpoint.page_end
                {
                    return Err(PlanningError::InvalidCheckpoint);
                }
            }
            Phase::Ready => {
                let count = descriptor_count.ok_or(PlanningError::InvalidCheckpoint)?;
                if checkpoint.failure.is_some()
                    || checkpoint.candidate_cursor != checkpoint.page_end
                    || checkpoint.page_start >= checkpoint.page_end
                    || checkpoint.page_end
                        != checkpoint.page_start.saturating_add(PAGE_SIZE).min(count)
                    || checkpoint.proposals.len() > checkpoint.page_end - checkpoint.page_start
                    || checkpoint.validator.is_some()
                    || checkpoint.sort_i != checkpoint.proposals.len()
                    || checkpoint.sort_j != 0
                    || !sorted_descending(&checkpoint.proposals)
                {
                    return Err(PlanningError::InvalidCheckpoint);
                }
            }
            Phase::Exhausted => {
                let count = descriptor_count.ok_or(PlanningError::InvalidCheckpoint)?;
                if checkpoint.failure.is_some()
                    || count != checkpoint.page_end
                    || checkpoint.page_start != checkpoint.page_end
                    || checkpoint.candidate_cursor != checkpoint.page_end
                    || checkpoint.validator.is_some()
                    || !checkpoint.proposals.is_empty()
                    || checkpoint.sort_i != 0
                    || checkpoint.sort_j != 0
                {
                    return Err(PlanningError::InvalidCheckpoint);
                }
            }
            Phase::Failed => {
                if checkpoint.failure.is_none()
                    || !matches!(descriptors_status, Some(Err(error)) if Some(error) == checkpoint.failure)
                    || checkpoint.validator.is_some()
                    || !checkpoint.proposals.is_empty()
                {
                    return Err(PlanningError::InvalidCheckpoint);
                }
            }
        }

        if let Some(validator_state) = &checkpoint.validator {
            let descriptors = discovery
                .descriptors()
                .and_then(Result::ok)
                .ok_or(PlanningError::InvalidCheckpoint)?;
            let mut candidate = *descriptors
                .get(checkpoint.candidate_cursor)
                .ok_or(PlanningError::InvalidCheckpoint)?;
            candidate.tie_index =
                ((checkpoint.candidate_cursor - checkpoint.page_start) % PAGE_SIZE) as u8;
            if !validator_state.matches_candidate(candidate) {
                return Err(PlanningError::InvalidCheckpoint);
            }
        }
        let saved_proposal_input = proposal_input.checkpoint();
        if !discovery
            .checkpoint()
            .matches_viability(&saved_proposal_input.viability)
        {
            return Err(PlanningError::SnapshotMismatch);
        }
        let validator = checkpoint
            .validator
            .map(|state| ProposalValidator::restore(state, proposal_input.clone()))
            .transpose()
            .map_err(|_| PlanningError::InvalidCheckpoint)?;
        Ok(Self {
            geometry_revision: checkpoint.geometry_revision,
            discovery,
            proposal_input,
            phase: checkpoint.phase,
            page_start: checkpoint.page_start,
            page_end: checkpoint.page_end,
            candidate_cursor: checkpoint.candidate_cursor,
            validator,
            proposals: checkpoint.proposals,
            sort_i: checkpoint.sort_i,
            sort_j: checkpoint.sort_j,
            failure: checkpoint.failure,
        })
    }

    pub(crate) fn advance(&mut self, budget: usize) -> PlanningAdvance {
        if budget == 0 || matches!(self.phase, Phase::Ready | Phase::Exhausted | Phase::Failed) {
            return self.advance_result(0);
        }
        let mut work = 0;
        while work < budget {
            match self.phase {
                Phase::Discover => {
                    let step = self.discovery.advance(budget - work);
                    work += step.work_used;
                    if step.done {
                        match self.discovery.descriptors() {
                            Some(Ok(descriptors)) if !descriptors.is_empty() => {
                                self.page_start = 0;
                                self.candidate_cursor = 0;
                                self.page_end = descriptors.len().min(PAGE_SIZE);
                                self.phase = Phase::Validate;
                            }
                            Some(Ok(_)) => self.phase = Phase::Exhausted,
                            Some(Err(error)) => {
                                self.failure = Some(error);
                                self.phase = Phase::Failed;
                            }
                            None => {
                                self.failure = Some(DiscoveryError::InvalidInput);
                                self.phase = Phase::Failed;
                            }
                        }
                    } else if step.work_used == 0 {
                        self.failure = Some(DiscoveryError::InvalidInput);
                        self.phase = Phase::Failed;
                    }
                }
                Phase::Validate => {
                    if self.candidate_cursor >= self.page_end {
                        self.phase = Phase::Sort;
                        self.sort_i = 1;
                        self.sort_j = 1;
                        continue;
                    }
                    if self.validator.is_none() {
                        let Some(Ok(descriptors)) = self.discovery.descriptors() else {
                            self.failure = Some(DiscoveryError::InvalidInput);
                            self.phase = Phase::Failed;
                            continue;
                        };
                        let Some(mut candidate) = descriptors.get(self.candidate_cursor).copied()
                        else {
                            self.failure = Some(DiscoveryError::InvalidCheckpoint);
                            self.phase = Phase::Failed;
                            continue;
                        };
                        candidate.tie_index =
                            ((self.candidate_cursor - self.page_start) % PAGE_SIZE) as u8;
                        self.validator = Some(ProposalValidator::new(
                            self.proposal_input.clone(),
                            candidate,
                        ));
                        work += 1; // Descriptor transfer and validator start.
                        if work == budget {
                            break;
                        }
                    }
                    let validator = self.validator.as_mut().expect("just initialized");
                    let step = validator.advance(budget - work);
                    work += step.work_used;
                    if step.done {
                        if let Some(Ok(proposal)) = validator.outcome() {
                            self.proposals.push(proposal);
                        }
                        self.validator = None;
                        self.candidate_cursor += 1;
                    } else if step.work_used == 0 {
                        self.failure = Some(DiscoveryError::InvalidInput);
                        self.phase = Phase::Failed;
                    }
                }
                Phase::Sort => {
                    if self.proposals.len() < 2 || self.sort_i >= self.proposals.len() {
                        self.sort_i = self.proposals.len();
                        self.sort_j = 0;
                        self.phase = Phase::Ready;
                        continue;
                    }
                    if self.sort_j == 0 {
                        self.sort_i += 1;
                        self.sort_j = self.sort_i;
                        continue;
                    }
                    work += 1;
                    let current = self.proposals[self.sort_j].score;
                    let prior = self.proposals[self.sort_j - 1].score;
                    if current.compare(prior) == Ordering::Greater {
                        self.proposals.swap(self.sort_j, self.sort_j - 1);
                        self.sort_j -= 1;
                    } else {
                        self.sort_i += 1;
                        self.sort_j = self.sort_i;
                    }
                }
                Phase::Ready | Phase::Exhausted | Phase::Failed => break,
            }
        }
        self.advance_result(work)
    }

    fn descriptor_count(&self) -> Result<usize, PlanningError> {
        match self.discovery.descriptors() {
            Some(Ok(descriptors)) => Ok(descriptors.len()),
            Some(Err(error)) => Err(PlanningError::Discovery(error)),
            None => Err(PlanningError::InvalidCheckpoint),
        }
    }

    fn advance_result(&self, work_used: usize) -> PlanningAdvance {
        PlanningAdvance {
            work_used,
            page_ready: self.phase == Phase::Ready,
            exhausted: self.is_exhausted(),
        }
    }
}

fn valid_proposal(proposal: &AccessProposal) -> bool {
    proposal.host_t.is_finite()
        && proposal.end_x.is_finite()
        && proposal.end_z.is_finite()
        && (2..=crate::interior_access::MAX_SELECTED_LOTS).contains(&proposal.selected_lots.len())
        && proposal.selected_lots.iter().all(|lot| {
            [lot.x, lot.z, lot.width, lot.depth, lot.rotation, lot.along]
                .iter()
                .all(|value| value.is_finite())
                && lot.width > 0.0
                && lot.depth > 0.0
                && [-1, 1].contains(&lot.side)
        })
}

fn valid_sort_cursor(i: usize, j: usize, len: usize) -> bool {
    if len < 2 {
        return i == 0 || i == len && j == 0;
    }
    i >= 1 && i <= len && j <= i && (j == 0 || j >= 1)
}

fn sorted_descending(proposals: &[AccessProposal]) -> bool {
    proposals
        .windows(2)
        .all(|pair| pair[0].score.compare(pair[1].score) != Ordering::Less)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_proposal_validation::{
        AdmissionGates, ProposalHost, ProposalNode, ProposalProject, ProposalRoad,
    };
    use crate::face_target_viability::{FrozenViabilityInput, Segment};
    use crate::road_faces::{RoadFaceEdge, RoadFaceNode};

    fn empty_viability() -> FrozenViabilityInput {
        FrozenViabilityInput {
            parcel_footprints: Vec::new(),
            reserved_lots: Vec::new(),
            open_roads: Vec::new(),
            pending_roads: Vec::new(),
            river_segments: vec![Segment {
                ax: 500.0,
                az: -400.0,
                bx: 500.0,
                bz: 400.0,
            }],
            neighborhood_segments: Vec::new(),
            neighborhood_interior: None,
        }
    }

    fn empty_inputs() -> (DiscoveryInput, ProposalValidationInput) {
        let viability = empty_viability();
        (
            DiscoveryInput {
                graph_nodes: Vec::new(),
                graph_edges: Vec::new(),
                all_nodes: Vec::new(),
                viability: viability.clone(),
            },
            ProposalValidationInput {
                hosts: Vec::new(),
                nodes: Vec::new(),
                roads: Vec::new(),
                projects: Vec::new(),
                viability,
                gates: AdmissionGates {
                    demand: 1.0,
                    has_pending_interior_plan: false,
                    has_neighborhood_plan: false,
                    road_count: 0,
                    project_count: 0,
                    parcel_count: 0,
                    frontage_count: 0,
                    next_id: 1,
                    project_admission_available: true,
                    pending_access_branch: None,
                },
            },
        )
    }

    fn square_inputs() -> (DiscoveryInput, ProposalValidationInput) {
        let positions = [(-50.0, -50.0), (50.0, -50.0), (50.0, 50.0), (-50.0, 50.0)];
        let mut roads = Vec::new();
        let mut hosts = Vec::new();
        let mut graph_nodes = Vec::new();
        let mut all_nodes = Vec::new();
        let mut proposal_nodes = Vec::new();
        for (index, &(x, z)) in positions.iter().enumerate() {
            let id = index as u32 + 1;
            graph_nodes.push(RoadFaceNode { id, x, z });
            all_nodes.push(crate::access_candidates::CandidateNode { id, x, z });
            proposal_nodes.push(ProposalNode { id, x, z });
        }
        let mut edges = Vec::new();
        let mut open_roads = Vec::new();
        for index in 0..4 {
            let from = index as u32 + 1;
            let to = ((index + 1) % 4) as u32 + 1;
            let (ax, az) = positions[index];
            let (bx, bz) = positions[(index + 1) % 4];
            let id = index as u32 + 11;
            let segment = Segment { ax, az, bx, bz };
            edges.push(RoadFaceEdge { id, from, to });
            open_roads.push(segment);
            hosts.push(ProposalHost {
                id,
                from,
                to,
                segment,
            });
            roads.push(ProposalRoad {
                id,
                is_open: true,
                segment: Some(segment),
            });
        }
        let mut viability = empty_viability();
        viability.open_roads = open_roads;
        (
            DiscoveryInput {
                graph_nodes,
                graph_edges: edges,
                all_nodes,
                viability: viability.clone(),
            },
            ProposalValidationInput {
                hosts,
                nodes: proposal_nodes,
                roads,
                projects: Vec::<ProposalProject>::new(),
                viability,
                gates: AdmissionGates {
                    demand: 1.0,
                    has_pending_interior_plan: false,
                    has_neighborhood_plan: false,
                    road_count: 4,
                    project_count: 0,
                    parcel_count: 0,
                    frontage_count: 0,
                    next_id: 100,
                    project_admission_available: true,
                    pending_access_branch: None,
                },
            },
        )
    }

    fn run(
        inputs: (DiscoveryInput, ProposalValidationInput),
        quota: usize,
        resume_interval: usize,
    ) -> (Vec<Vec<AccessProposal>>, usize) {
        let mut job = AccessPlanningJob::new(inputs.0, inputs.1, 77).unwrap();
        assert_eq!(job.geometry_revision(), 77);
        let mut work = 0;
        let mut work_since_checkpoint = 0;
        let mut pages = Vec::new();
        let mut finished = false;
        for _ in 0..2_000_000 {
            let step = job.advance(quota);
            assert!(step.work_used <= quota);
            work += step.work_used;
            work_since_checkpoint += step.work_used;
            if resume_interval > 0 && work_since_checkpoint >= resume_interval {
                let encoded = serde_json::to_vec(&job.checkpoint()).unwrap();
                job =
                    AccessPlanningJob::restore(serde_json::from_slice(&encoded).unwrap()).unwrap();
                work_since_checkpoint = 0;
            }
            if step.page_ready {
                let page = job.ready_page().expect("ready page exists");
                assert!(page.len() <= PAGE_SIZE);
                assert!(sorted_descending(page));
                pages.push(page.to_vec());
                if !job.continue_next_page().unwrap() {
                    finished = true;
                    break;
                }
            } else if step.exhausted {
                finished = true;
                break;
            } else {
                assert!(step.work_used > 0, "active planning job made no progress");
            }
        }
        assert!(finished, "planning job exceeded the bounded test loop");
        assert!(job.is_exhausted());
        (pages, work)
    }

    #[test]
    fn empty_no_face_input_exhausts_cleanly_and_restores() {
        let (pages, work) = run(empty_inputs(), 7, 1);
        assert!(pages.is_empty());
        assert!(work > 0);
    }

    #[test]
    fn synthetic_face_is_quota_invariant_and_page_sort_is_stable() {
        let baseline = run(square_inputs(), 48, 0);
        for quota in [1, 7] {
            let resumed = run(square_inputs(), quota, 4096);
            assert_eq!(resumed, baseline);
        }
    }

    #[test]
    fn constructor_rejects_different_frozen_world_projections() {
        let (discovery, mut proposal) = empty_inputs();
        proposal.viability.river_segments.clear();
        assert_eq!(
            AccessPlanningJob::new(discovery, proposal, 0).unwrap_err(),
            PlanningError::SnapshotMismatch
        );
    }
}
