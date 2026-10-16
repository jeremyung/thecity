//! Bounded live revalidation around frozen access discovery.
//!
//! Discovery remains frozen at its creation revision. This wrapper visits its
//! sorted proposal pages and requires a fresh proposal validator result before
//! returning any proposal for admission. It does not read or mutate Engine.
//! The caller owns live capture, current scalar gates, stale-revision decisions,
//! and atomic commit. Checkpoints are explicit save boundaries; ordinary
//! advances never clone or serialize the live validation context.

use crate::access_planning_job::{AccessPlanningCheckpoint, AccessPlanningJob, PlanningError};
use crate::access_proposal_validation::{
    ProposalValidationCheckpoint, ProposalValidationContext, ProposalValidationInput,
    ProposalValidator,
};
use crate::interior_access::{AccessGeometryCandidate, AccessProposal};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_STALE_RETRIES: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiveAccessError {
    Planning(PlanningError),
    InvalidState,
    InvalidCheckpoint,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LiveAccessCheckpoint {
    schema: u8,
    planning: AccessPlanningCheckpoint,
    proposal_cursor: usize,
    stale_retries: u8,
    validation_context_input: Option<ProposalValidationInput>,
    validation_revision: Option<u64>,
    validator: Option<ProposalValidationCheckpoint>,
    approved: Option<AccessProposal>,
}

#[derive(Debug)]
pub(crate) struct LiveAccess {
    planning: AccessPlanningJob,
    proposal_cursor: usize,
    stale_retries: u8,
    validation_context: Option<Arc<ProposalValidationContext>>,
    validation_revision: Option<u64>,
    validator: Option<ProposalValidator>,
    approved: Option<AccessProposal>,
}

impl LiveAccess {
    /// Wrap a planning job whose discovery and proposal inputs share a frozen
    /// geometry revision. The wrapper never restarts that discovery.
    pub(crate) fn new(job: AccessPlanningJob) -> Self {
        Self {
            planning: job,
            proposal_cursor: 0,
            stale_retries: 0,
            validation_context: None,
            validation_revision: None,
            validator: None,
            approved: None,
        }
    }

    /// Spend at most `budget` planning work units. Ready pages are consumed
    /// with one charged unit per page transition, including empty pages.
    /// Returns actual underlying work and bounded page-transition charges.
    pub(crate) fn advance(&mut self, budget: usize) -> usize {
        let mut work = 0;
        while work < budget {
            if self.planning.is_exhausted() {
                break;
            }
            if let Some(page) = self.planning.ready_page() {
                if self.proposal_cursor < page.len() {
                    break;
                }
                work += 1;
                self.proposal_cursor = 0;
                self.stale_retries = 0;
                if self.planning.continue_next_page().is_err() {
                    break;
                }
                continue;
            }
            let used = self.planning.advance(budget - work).work_used;
            work += used;
            if used == 0 {
                break;
            }
        }
        work
    }

    /// Return the current frozen proposal as an exact live geometry candidate.
    /// A caller must capture fresh current context and call `begin_validation`
    /// before the proposal can be exposed by `ready_proposal`.
    pub(crate) fn needs_validation(&self) -> Option<AccessGeometryCandidate> {
        if self.validator.is_some() || self.approved.is_some() {
            return None;
        }
        let proposal = self.planning.ready_page()?.get(self.proposal_cursor)?;
        Some(AccessGeometryCandidate {
            host_id: proposal.host_id,
            host_t: proposal.host_t,
            end_x: proposal.end_x,
            end_z: proposal.end_z,
            tie_index: proposal.score.fraction_index,
        })
    }

    /// Start validating the current proposal against one fresh captured
    /// context and its live geometry revision.
    pub(crate) fn begin_validation(
        &mut self,
        input: ProposalValidationInput,
        revision: u64,
    ) -> Result<(), LiveAccessError> {
        let candidate = self
            .needs_validation()
            .ok_or(LiveAccessError::InvalidState)?;
        let context =
            ProposalValidationContext::new(input).map_err(|_| LiveAccessError::InvalidState)?;
        self.validator = Some(ProposalValidator::new(context.clone(), candidate));
        self.validation_context = Some(context);
        self.validation_revision = Some(revision);
        Ok(())
    }

    /// Return the freshly validated proposal and the revision it was checked
    /// against. The caller still checks live scalar gates and performs commit.
    pub(crate) fn ready_proposal(&self) -> Option<(&AccessProposal, u64)> {
        Some((self.approved.as_ref()?, self.validation_revision?))
    }

    /// Revision associated with the current in-progress or completed live
    /// validation, so the caller can discard work as soon as it turns stale.
    pub(crate) fn validation_revision(&self) -> Option<u64> {
        self.validation_revision
    }

    pub(crate) fn selected_face_identity(&self) -> Option<&[u32]> {
        self.planning.selected_face_identity()
    }

    /// Advance live validation by a bounded quota. Rejected proposals are
    /// skipped automatically; accepted results wait for caller disposition.
    pub(crate) fn advance_validation(&mut self, budget: usize) -> usize {
        let Some(validator) = self.validator.as_mut() else {
            return 0;
        };
        let step = validator.advance(budget);
        let outcome = step.done.then(|| validator.outcome()).flatten();
        if step.done {
            match outcome {
                Some(Ok(proposal)) => {
                    self.approved = Some(proposal);
                    self.validator = None;
                }
                Some(Err(_)) | None => self.reject_or_consume(),
            }
        }
        step.work_used
    }

    /// Consume the current proposal after rejection or successful atomic
    /// commit. This is a constant-time cursor change.
    pub(crate) fn reject_or_consume(&mut self) {
        self.validator = None;
        self.approved = None;
        self.validation_context = None;
        self.validation_revision = None;
        self.proposal_cursor = self.proposal_cursor.saturating_add(1);
        self.stale_retries = 0;
    }

    /// Discard a completed/in-progress validation after observing a stale
    /// world revision. Returns true when the same candidate remains eligible
    /// for a fresh capture; after two retries it skips the candidate.
    pub(crate) fn discard_stale_validation(&mut self) -> bool {
        if self.validator.is_none() && self.approved.is_none() {
            return false;
        }
        self.validator = None;
        self.approved = None;
        self.validation_context = None;
        self.validation_revision = None;
        if self.stale_retries < MAX_STALE_RETRIES {
            self.stale_retries += 1;
            true
        } else {
            self.reject_or_consume();
            false
        }
    }

    pub(crate) fn is_exhausted(&self) -> bool {
        self.planning.is_exhausted()
            && self.planning.ready_page().is_none()
            && self.validator.is_none()
            && self.approved.is_none()
    }

    pub(crate) fn failed(&self) -> bool {
        self.planning.error().is_some()
    }

    /// Create a persistence snapshot. This is the only point where the fresh
    /// context DTO is copied; repeated advances retain its Arc-backed arrays.
    pub(crate) fn checkpoint(&self) -> LiveAccessCheckpoint {
        LiveAccessCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            planning: self.planning.checkpoint(),
            proposal_cursor: self.proposal_cursor,
            stale_retries: self.stale_retries,
            validation_context_input: self
                .validation_context
                .as_ref()
                .map(|context| context.checkpoint()),
            validation_revision: self.validation_revision,
            validator: self.validator.as_ref().map(ProposalValidator::checkpoint),
            approved: self.approved.clone(),
        }
    }

    /// Restore a validated checkpoint, rejecting mismatched cursors, context,
    /// validator candidates, and unsupported schema versions.
    pub(crate) fn restore(checkpoint: LiveAccessCheckpoint) -> Result<Self, LiveAccessError> {
        if checkpoint.schema != CHECKPOINT_SCHEMA || checkpoint.stale_retries > MAX_STALE_RETRIES {
            return Err(LiveAccessError::InvalidCheckpoint);
        }
        let planning =
            AccessPlanningJob::restore(checkpoint.planning).map_err(|error| match error {
                PlanningError::InvalidCheckpoint | PlanningError::SnapshotMismatch => {
                    LiveAccessError::InvalidCheckpoint
                }
                other => LiveAccessError::Planning(other),
            })?;
        let page_len = planning.ready_page().map_or(0, <[_]>::len);
        if checkpoint.proposal_cursor > page_len
            || checkpoint.approved.is_some() && checkpoint.validator.is_some()
            || checkpoint.validation_context_input.is_some()
                != checkpoint.validation_revision.is_some()
            || checkpoint.validation_context_input.is_some()
                != (checkpoint.validator.is_some() || checkpoint.approved.is_some())
        {
            return Err(LiveAccessError::InvalidCheckpoint);
        }
        if let Some(proposal) = &checkpoint.approved {
            if !valid_proposal(proposal)
                || planning
                    .ready_page()
                    .and_then(|page| page.get(checkpoint.proposal_cursor))
                    .map_or(true, |source| {
                        source.host_id != proposal.host_id
                            || source.host_t != proposal.host_t
                            || source.end_x != proposal.end_x
                            || source.end_z != proposal.end_z
                    })
            {
                return Err(LiveAccessError::InvalidCheckpoint);
            }
        }
        if let Some(state) = &checkpoint.validator {
            let source = planning
                .ready_page()
                .and_then(|page| page.get(checkpoint.proposal_cursor))
                .ok_or(LiveAccessError::InvalidCheckpoint)?;
            let candidate = AccessGeometryCandidate {
                host_id: source.host_id,
                host_t: source.host_t,
                end_x: source.end_x,
                end_z: source.end_z,
                tie_index: source.score.fraction_index,
            };
            if !state.matches_candidate(candidate) {
                return Err(LiveAccessError::InvalidCheckpoint);
            }
        }
        let validation_context = match checkpoint.validation_context_input {
            Some(input) => {
                let context = ProposalValidationContext::new(input)
                    .map_err(|_| LiveAccessError::InvalidCheckpoint)?;
                Some(context)
            }
            None => None,
        };
        let validator = match (checkpoint.validator, validation_context.as_ref()) {
            (Some(state), Some(context)) => Some(
                ProposalValidator::restore(state, context.clone())
                    .map_err(|_| LiveAccessError::InvalidCheckpoint)?,
            ),
            (None, _) => None,
            (Some(_), None) => return Err(LiveAccessError::InvalidCheckpoint),
        };
        if let (Some(saved), Some(context)) = (&checkpoint.approved, validation_context.as_ref()) {
            let source = planning
                .ready_page()
                .and_then(|page| page.get(checkpoint.proposal_cursor))
                .ok_or(LiveAccessError::InvalidCheckpoint)?;
            let candidate = AccessGeometryCandidate {
                host_id: source.host_id,
                host_t: source.host_t,
                end_x: source.end_x,
                end_z: source.end_z,
                tie_index: source.score.fraction_index,
            };
            let mut verification = ProposalValidator::new(context.clone(), candidate);
            let result = verification.advance(usize::MAX);
            if !result.done
                || !matches!(verification.outcome(), Some(Ok(ref actual)) if same_proposal(actual, saved))
            {
                return Err(LiveAccessError::InvalidCheckpoint);
            }
        }
        Ok(Self {
            planning,
            proposal_cursor: checkpoint.proposal_cursor,
            stale_retries: checkpoint.stale_retries,
            validation_context,
            validation_revision: checkpoint.validation_revision,
            validator,
            approved: checkpoint.approved,
        })
    }
}

fn valid_proposal(proposal: &AccessProposal) -> bool {
    proposal.host_t.is_finite()
        && proposal.end_x.is_finite()
        && proposal.end_z.is_finite()
        && !proposal.selected_lots.is_empty()
        && proposal.selected_lots.len() <= crate::interior_access::MAX_SELECTED_LOTS
        && proposal.selected_lots.iter().all(|lot| {
            lot.x.is_finite()
                && lot.z.is_finite()
                && lot.width.is_finite()
                && lot.depth.is_finite()
                && lot.rotation.is_finite()
                && lot.along.is_finite()
        })
}

fn same_proposal(left: &AccessProposal, right: &AccessProposal) -> bool {
    left.score == right.score
        && left.host_id == right.host_id
        && left.host_t == right.host_t
        && left.end_x == right.end_x
        && left.end_z == right.end_z
        && left.selected_lots == right.selected_lots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_discovery_job::DiscoveryInput;
    use crate::access_proposal_validation::{
        AdmissionGates, ProposalHost, ProposalNode, ProposalProject, ProposalRoad,
        ProposalValidationInput,
    };
    use crate::face_target_viability::{FrozenViabilityInput, Segment};
    use crate::road_faces::{RoadFaceEdge, RoadFaceNode};

    fn empty_live_access() -> LiveAccess {
        let viability = FrozenViabilityInput {
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
        };
        LiveAccess::new(
            AccessPlanningJob::new(
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
                9,
            )
            .unwrap(),
        )
    }

    #[test]
    fn empty_discovery_terminates_with_unit_quotas_and_json_resume() {
        let mut job = empty_live_access();
        for _ in 0..10_000 {
            let used = job.advance(1);
            assert!(used <= 1);
            if job.is_exhausted() {
                let encoded = serde_json::to_vec(&job.checkpoint()).unwrap();
                job = LiveAccess::restore(serde_json::from_slice(&encoded).unwrap()).unwrap();
                assert!(job.is_exhausted());
                return;
            }
            assert!(
                used > 0 || job.planning.ready_page().is_some(),
                "active live access job made no progress"
            );
        }
        panic!("empty discovery exceeded the bounded test loop");
    }

    #[test]
    fn restore_rejects_old_schema_and_corrupt_proposal_cursor() {
        let mut checkpoint = empty_live_access().checkpoint();
        checkpoint.schema = 0;
        assert_eq!(
            LiveAccess::restore(checkpoint).unwrap_err(),
            LiveAccessError::InvalidCheckpoint
        );

        let mut checkpoint = empty_live_access().checkpoint();
        checkpoint.proposal_cursor = 1;
        assert_eq!(
            LiveAccess::restore(checkpoint).unwrap_err(),
            LiveAccessError::InvalidCheckpoint
        );
    }

    fn square_inputs() -> (DiscoveryInput, ProposalValidationInput) {
        let positions = [(-50.0, -50.0), (50.0, -50.0), (50.0, 50.0), (-50.0, 50.0)];
        let mut graph_nodes = Vec::new();
        let mut all_nodes = Vec::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut hosts = Vec::new();
        let mut roads = Vec::new();
        let mut open_roads = Vec::new();
        for (index, &(x, z)) in positions.iter().enumerate() {
            let id = index as u32 + 1;
            graph_nodes.push(RoadFaceNode { id, x, z });
            all_nodes.push(crate::access_candidates::CandidateNode { id, x, z });
            nodes.push(ProposalNode { id, x, z });
        }
        for index in 0..4 {
            let from = index as u32 + 1;
            let to = ((index + 1) % 4) as u32 + 1;
            let (ax, az) = positions[index];
            let (bx, bz) = positions[(index + 1) % 4];
            let id = index as u32 + 11;
            let segment = Segment { ax, az, bx, bz };
            edges.push(RoadFaceEdge { id, from, to });
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
            open_roads.push(segment);
        }
        let viability = FrozenViabilityInput {
            parcel_footprints: Vec::new(),
            reserved_lots: Vec::new(),
            open_roads,
            pending_roads: Vec::new(),
            river_segments: vec![Segment {
                ax: 500.0,
                az: -400.0,
                bx: 500.0,
                bz: 400.0,
            }],
            neighborhood_segments: Vec::new(),
            neighborhood_interior: None,
        };
        (
            DiscoveryInput {
                graph_nodes,
                graph_edges: edges,
                all_nodes,
                viability: viability.clone(),
            },
            ProposalValidationInput {
                hosts,
                nodes,
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

    fn run_offers(quota: usize, resume_interval: usize) -> Vec<(u32, f32, f32, f32)> {
        let (discovery, validation) = square_inputs();
        let mut job =
            LiveAccess::new(AccessPlanningJob::new(discovery, validation.clone(), 77).unwrap());
        let mut offers = Vec::new();
        let mut work_since_save = 0;
        for _ in 0..3_000_000 {
            if let Some(candidate) = job.needs_validation() {
                job.begin_validation(validation.clone(), 77).unwrap();
                while job.validator.is_some() {
                    let used = job.advance_validation(quota.max(1));
                    assert!(used <= quota.max(1));
                    work_since_save += used;
                    if resume_interval > 0 && work_since_save >= resume_interval {
                        let encoded = serde_json::to_vec(&job.checkpoint()).unwrap();
                        job =
                            LiveAccess::restore(serde_json::from_slice(&encoded).unwrap()).unwrap();
                        work_since_save = 0;
                    }
                }
                if let Some((proposal, revision)) = job.ready_proposal() {
                    assert_eq!(revision, 77);
                    assert_eq!(proposal.host_id, candidate.host_id);
                    offers.push((
                        proposal.host_id,
                        proposal.host_t,
                        proposal.end_x,
                        proposal.end_z,
                    ));
                    job.reject_or_consume();
                }
            } else {
                let used = job.advance(quota);
                assert!(used <= quota);
                work_since_save += used;
                if resume_interval > 0 && work_since_save >= resume_interval {
                    let encoded = serde_json::to_vec(&job.checkpoint()).unwrap();
                    job = LiveAccess::restore(serde_json::from_slice(&encoded).unwrap()).unwrap();
                    work_since_save = 0;
                }
                if job.is_exhausted() {
                    return offers;
                }
                assert!(
                    used > 0 || job.planning.ready_page().is_some(),
                    "active live access job made no progress"
                );
            }
        }
        panic!("live access sequence exceeded the bounded test loop")
    }

    #[test]
    fn fresh_revalidation_offer_sequence_is_quota_invariant_and_resumable() {
        let expected = run_offers(48, 0);
        assert!(
            !expected.is_empty(),
            "synthetic square should produce an offer"
        );
        assert_eq!(run_offers(7, 4096), expected);
    }

    #[test]
    fn stale_validation_retries_twice_then_skips_candidate() {
        let (discovery, validation) = square_inputs();
        let mut job =
            LiveAccess::new(AccessPlanningJob::new(discovery, validation.clone(), 77).unwrap());
        for _ in 0..1_000_000 {
            if job.needs_validation().is_some() {
                break;
            }
            let used = job.advance(48);
            assert!(used > 0 || job.planning.ready_page().is_some());
        }
        let original = job
            .needs_validation()
            .expect("synthetic face yields candidates");
        for retry in 1..=2 {
            job.begin_validation(validation.clone(), 77).unwrap();
            assert!(job.discard_stale_validation());
            let encoded = serde_json::to_vec(&job.checkpoint()).unwrap();
            job = LiveAccess::restore(serde_json::from_slice(&encoded).unwrap()).unwrap();
            assert_eq!(job.needs_validation(), Some(original));
            assert_eq!(job.stale_retries, retry);
        }
        job.begin_validation(validation, 78).unwrap();
        assert!(!job.discard_stale_validation());
        assert_eq!(job.stale_retries, 0);
        assert_ne!(job.needs_validation(), Some(original));
    }

    #[test]
    fn restore_rejects_forged_approved_lot_even_when_finite() {
        let (discovery, validation) = square_inputs();
        let mut job =
            LiveAccess::new(AccessPlanningJob::new(discovery, validation.clone(), 77).unwrap());
        for _ in 0..1_000_000 {
            if job.needs_validation().is_some() {
                break;
            }
            job.advance(48);
        }
        let candidate = job
            .needs_validation()
            .expect("synthetic face yields candidates");
        job.begin_validation(validation, 77).unwrap();
        for _ in 0..100_000 {
            job.advance_validation(48);
            if job.ready_proposal().is_some() {
                break;
            }
            if job.needs_validation().is_some() {
                job.begin_validation(square_inputs().1, 77).unwrap();
            }
        }
        let (proposal, _) = job
            .ready_proposal()
            .expect("synthetic fixture has a valid offer");
        assert_eq!(proposal.host_id, candidate.host_id);
        let mut checkpoint = job.checkpoint();
        checkpoint.approved.as_mut().unwrap().selected_lots[0].x += 0.25;
        assert_eq!(
            LiveAccess::restore(checkpoint).unwrap_err(),
            LiveAccessError::InvalidCheckpoint
        );
    }
}
