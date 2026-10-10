//! Quota-stepped equivalent of the live interior-access proposal evaluator.
//!
//! This module consumes an immutable, source-ordered projection. It validates
//! one explicit geometry candidate at a time and never consults Engine state.
//! Final live admission remains the caller's responsibility.

use crate::access_candidates::MAX_PREPARED_INPUT_NODES;
use crate::face_target_viability::{
    FrozenViabilityContext, FrozenViabilityInput, Segment, ViabilityCheckpoint, ViabilityError,
    ViabilityQuery,
};
use crate::geometry::{self, OrientedRect};
use crate::interior_access::{
    AccessGeometryCandidate, AccessProposal, CandidateScore, InteriorAccessLot,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_HOSTS: usize = crate::MAX_SEGMENTS;
const MAX_ROADS: usize = crate::MAX_SEGMENTS;
const MAX_PROJECT_RECORDS: usize = crate::MAX_PROJECTS + crate::COMPLETED_PROJECT_HISTORY;
const MAX_FOOTPRINTS: usize = crate::MAX_PARCELS.saturating_mul(2);
const MAX_LOTS_PER_SIDE: usize = crate::MAX_FRONTAGE_GAP_LOTS_PER_SIDE;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProposalHost {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub segment: Segment,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProposalNode {
    pub id: u32,
    pub x: f32,
    pub z: f32,
}

/// Every road is retained in source order. `segment == None` preserves the
/// live evaluator's conservative behavior for a road whose endpoint node is
/// missing during split/clearance checks.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProposalRoad {
    pub id: u32,
    pub is_open: bool,
    pub segment: Option<Segment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum ProjectEndpoints {
    NoIds,
    MissingNode,
    Segment(Segment),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProposalProject {
    /// True exactly when `kind == "road" && progress < 1.0`.
    pub active_road: bool,
    pub endpoints: ProjectEndpoints,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct AdmissionGates {
    pub demand: f32,
    pub has_pending_interior_plan: bool,
    pub has_neighborhood_plan: bool,
    pub road_count: usize,
    pub project_count: usize,
    pub parcel_count: usize,
    pub frontage_count: usize,
    pub next_id: u32,
    pub project_admission_available: bool,
    /// The currently pending access branch, if any. This is normally absent
    /// because the gate above rejects a pending plan, but is retained so the
    /// road-corridor predicate has the same explicit input as live admission.
    pub pending_access_branch: Option<Segment>,
}

/// Serializable capture DTO. Hosts are in strictly ascending road-id order;
/// all other vectors retain their source order from Engine state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProposalValidationInput {
    pub hosts: Vec<ProposalHost>,
    pub nodes: Vec<ProposalNode>,
    pub roads: Vec<ProposalRoad>,
    pub projects: Vec<ProposalProject>,
    pub viability: FrozenViabilityInput,
    pub gates: AdmissionGates,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputError {
    TooManyHosts,
    TooManyNodes,
    TooManyRoads,
    TooManyProjects,
    TooManyFootprints,
    InvalidOrder,
    NonFiniteInput,
    InvalidCapacity,
    Viability(ViabilityError),
}

/// Validated immutable projection shared by candidate validators. The
/// ViabilityContext owns the collision arrays once; each candidate adds its
/// proposed branch through `ViabilityQuery::new_with_extra_road` without
/// cloning those arrays.
#[derive(Clone, Debug)]
pub(crate) struct ProposalValidationContext {
    hosts: Vec<ProposalHost>,
    nodes: Vec<ProposalNode>,
    roads: Vec<ProposalRoad>,
    projects: Vec<ProposalProject>,
    viability: Arc<FrozenViabilityContext>,
    gates: AdmissionGates,
    fingerprint: u64,
}

impl ProposalValidationContext {
    pub(crate) fn new(input: ProposalValidationInput) -> Result<Arc<Self>, InputError> {
        let viability = validate_input(&input)?;
        let fingerprint = fingerprint(&input);
        let ProposalValidationInput {
            hosts,
            nodes,
            roads,
            projects,
            viability: _,
            gates,
        } = input;
        Ok(Arc::new(Self {
            hosts,
            nodes,
            roads,
            projects,
            viability,
            gates,
            fingerprint,
        }))
    }

    /// Explicit save boundary; ordinary candidate steps only borrow the
    /// shared arrays and never clone the frozen geometry.
    pub(crate) fn checkpoint(&self) -> ProposalValidationInput {
        ProposalValidationInput {
            hosts: self.hosts.clone(),
            nodes: self.nodes.clone(),
            roads: self.roads.clone(),
            projects: self.projects.clone(),
            viability: self.viability.checkpoint(),
            gates: self.gates,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Gates,
    ActiveProjects,
    FindHost,
    DeriveBranch,
    River,
    ProtectedSamples,
    ReservedBranch,
    ReservedLots,
    ParcelCorridor,
    SplitNodes,
    SplitRoads,
    EndpointNodes,
    OpenRoads,
    Projects,
    BuildIntervals,
    BuildLot,
    CheckLot,
    LotCapacity,
    SumArea,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct DerivedBranch {
    host_from: u32,
    host_to: u32,
    start_x: f32,
    start_z: f32,
    end_x: f32,
    end_z: f32,
    length: f32,
    normal_x: f32,
    normal_z: f32,
    tangent_x: f32,
    tangent_z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct LotInterval {
    start: f32,
    end: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RejectReason {
    AdmissionGate,
    ActiveRoadProject,
    HostNotFound,
    InvalidCandidate,
    InvalidHost,
    BranchRiverOrProtected,
    BranchReservedOrParcelConflict,
    SplitCollision,
    EndpointCollision,
    BranchRoadConflict,
    LotGeometry,
    InsufficientLots,
    Capacity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum SavedOutcome {
    Rejected(RejectReason),
    Accepted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ProposalValidationCheckpoint {
    schema: u8,
    input_fingerprint: u64,
    candidate: AccessGeometryCandidate,
    phase: Phase,
    cursor: usize,
    host_index: Option<usize>,
    branch: Option<DerivedBranch>,
    intervals: Vec<LotInterval>,
    lot_cursor: usize,
    current_lot: Option<OrientedRect>,
    current_lot_query: Option<ViabilityCheckpoint>,
    lots: Vec<InteriorAccessLot>,
    area_cursor: usize,
    area_cm2: u32,
    outcome: Option<SavedOutcome>,
}

impl ProposalValidationCheckpoint {
    pub(crate) fn matches_candidate(&self, candidate: AccessGeometryCandidate) -> bool {
        self.candidate == candidate
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ValidationAdvance {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ProposalValidator {
    input: Arc<ProposalValidationContext>,
    state: ProposalValidationCheckpoint,
    lot_query: Option<ViabilityQuery>,
}

impl ProposalValidator {
    pub(crate) fn new(
        input: Arc<ProposalValidationContext>,
        candidate: AccessGeometryCandidate,
    ) -> Self {
        Self {
            state: ProposalValidationCheckpoint {
                schema: CHECKPOINT_SCHEMA,
                input_fingerprint: input.fingerprint,
                candidate,
                phase: Phase::Gates,
                cursor: 0,
                host_index: None,
                branch: None,
                intervals: Vec::new(),
                lot_cursor: 0,
                current_lot: None,
                current_lot_query: None,
                lots: Vec::new(),
                area_cursor: 0,
                area_cm2: 0,
                outcome: None,
            },
            input,
            lot_query: None,
        }
    }

    pub(crate) fn restore(
        state: ProposalValidationCheckpoint,
        input: Arc<ProposalValidationContext>,
    ) -> Result<Self, InputError> {
        validate_checkpoint(&state, &input)?;
        let lot_query = state
            .current_lot_query
            .clone()
            .map(|query| ViabilityQuery::restore(query, input.viability.clone()))
            .transpose()
            .map_err(InputError::Viability)?;
        Ok(Self {
            input,
            state,
            lot_query,
        })
    }

    pub(crate) fn checkpoint(&self) -> ProposalValidationCheckpoint {
        let mut state = self.state.clone();
        state.current_lot_query = self.lot_query.as_ref().map(ViabilityQuery::checkpoint);
        state
    }

    pub(crate) fn outcome(&self) -> Option<Result<AccessProposal, RejectReason>> {
        match self.state.outcome? {
            SavedOutcome::Rejected(reason) => Some(Err(reason)),
            SavedOutcome::Accepted => {
                let branch = self.state.branch?;
                Some(Ok(AccessProposal {
                    score: CandidateScore {
                        site_area: self.state.area_cm2,
                        road_length_mm: (branch.length * 1_000.0).round() as u32,
                        host_id: self.state.candidate.host_id,
                        fraction_index: self.state.candidate.tie_index,
                        side_index: 1,
                        length_index: 0,
                    },
                    host_id: self.state.candidate.host_id,
                    host_t: self.state.candidate.host_t,
                    end_x: self.state.candidate.end_x,
                    end_z: self.state.candidate.end_z,
                    selected_lots: self.state.lots.clone(),
                }))
            }
        }
    }

    pub(crate) fn advance(&mut self, budget: usize) -> ValidationAdvance {
        if budget == 0 || self.state.outcome.is_some() {
            return ValidationAdvance {
                work_used: 0,
                done: self.state.outcome.is_some(),
            };
        }
        let mut work = 0;
        while work < budget && self.state.outcome.is_none() {
            match self.state.phase {
                Phase::Gates => {
                    work += 1;
                    let gates = self.input.gates;
                    if gates.has_pending_interior_plan
                        || gates.has_neighborhood_plan
                        || gates.demand <= 0.8
                        || !gates.project_admission_available
                        || gates
                            .road_count
                            .saturating_add(gates.project_count)
                            .saturating_add(3)
                            > crate::MAX_SEGMENTS
                        || gates.parcel_count.saturating_add(2) > crate::MAX_PARCELS
                        || gates.next_id.checked_add(5).is_none()
                    {
                        self.reject(RejectReason::AdmissionGate);
                    } else {
                        self.state.phase = Phase::ActiveProjects;
                        self.state.cursor = 0;
                    }
                }
                Phase::ActiveProjects => {
                    if self.state.cursor == self.input.projects.len() {
                        self.state.phase = Phase::FindHost;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    if self.input.projects[self.state.cursor].active_road {
                        self.reject(RejectReason::ActiveRoadProject);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::FindHost => {
                    if self.state.cursor == self.input.hosts.len() {
                        self.reject(RejectReason::HostNotFound);
                        continue;
                    }
                    work += 1;
                    let index = self.state.cursor;
                    if self.input.hosts[index].id == self.state.candidate.host_id {
                        self.state.host_index = Some(index);
                        self.state.phase = Phase::DeriveBranch;
                        self.state.cursor = 0;
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::DeriveBranch => {
                    work += 1;
                    if !self.derive_branch() {
                        self.reject(RejectReason::InvalidCandidate);
                    } else {
                        self.state.phase = Phase::River;
                        self.state.cursor = 0;
                    }
                }
                Phase::River => {
                    if self.state.cursor == self.input.viability.input().river_segments.len() {
                        self.state.phase = Phase::ProtectedSamples;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let branch = self.state.branch.unwrap();
                    let river = self.input.viability.input().river_segments[self.state.cursor];
                    let required = crate::RIVER_HALF_WIDTH + crate::RIVER_BANK_BUFFER + 4.5;
                    if geometry::segments_distance_coords(
                        branch.start_x,
                        branch.start_z,
                        branch.end_x,
                        branch.end_z,
                        river.ax,
                        river.az,
                        river.bx,
                        river.bz,
                    ) < required
                    {
                        self.reject(RejectReason::BranchRiverOrProtected);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::ProtectedSamples => {
                    if self.state.cursor == 17 {
                        self.state.phase = Phase::ReservedBranch;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let branch = self.state.branch.unwrap();
                    let t = self.state.cursor as f32 / 16.0;
                    let x = branch.start_x + (branch.end_x - branch.start_x) * t;
                    let z = branch.start_z + (branch.end_z - branch.start_z) * t;
                    if crate::in_protected_land(x, z) {
                        self.reject(RejectReason::BranchRiverOrProtected);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::ReservedBranch => {
                    work += 1;
                    let branch = self.state.branch.unwrap();
                    if self
                        .input
                        .gates
                        .pending_access_branch
                        .is_some_and(|reserved| {
                            geometry::segments_distance_coords(
                                branch.start_x,
                                branch.start_z,
                                branch.end_x,
                                branch.end_z,
                                reserved.ax,
                                reserved.az,
                                reserved.bx,
                                reserved.bz,
                            ) < 8.0
                        })
                    {
                        self.reject(RejectReason::BranchReservedOrParcelConflict);
                    } else {
                        self.state.phase = Phase::ReservedLots;
                        self.state.cursor = 0;
                    }
                }
                Phase::ReservedLots => {
                    if self.state.cursor == self.input.viability.input().reserved_lots.len() {
                        self.state.phase = Phase::ParcelCorridor;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let lot = self.input.viability.input().reserved_lots[self.state.cursor];
                    let branch = self.state.branch.unwrap();
                    if geometry::rect_segment_distance(
                        &lot,
                        branch.start_x,
                        branch.start_z,
                        branch.end_x,
                        branch.end_z,
                    ) < 5.0
                    {
                        self.reject(RejectReason::BranchReservedOrParcelConflict);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::ParcelCorridor => {
                    if self.state.cursor == self.input.viability.input().parcel_footprints.len() {
                        self.state.phase = Phase::SplitNodes;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let footprint =
                        &self.input.viability.input().parcel_footprints[self.state.cursor];
                    let branch = self.state.branch.unwrap();
                    if geometry::rect_segment_distance(
                        footprint,
                        branch.start_x,
                        branch.start_z,
                        branch.end_x,
                        branch.end_z,
                    ) < 5.0
                    {
                        self.reject(RejectReason::BranchReservedOrParcelConflict);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::SplitNodes => {
                    if self.state.cursor == self.input.nodes.len() {
                        self.state.phase = Phase::SplitRoads;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let node = self.input.nodes[self.state.cursor];
                    let branch = self.state.branch.unwrap();
                    if node.id != branch.host_from
                        && node.id != branch.host_to
                        && (node.x - branch.start_x).hypot(node.z - branch.start_z) < 1.0
                    {
                        self.reject(RejectReason::SplitCollision);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::SplitRoads => {
                    if self.state.cursor == self.input.roads.len() {
                        self.state.phase = Phase::EndpointNodes;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let road = self.input.roads[self.state.cursor];
                    let branch = self.state.branch.unwrap();
                    if road.id == self.state.candidate.host_id {
                        self.state.cursor += 1;
                        continue;
                    }
                    let Some(segment) = road.segment else {
                        self.reject(RejectReason::SplitCollision);
                        continue;
                    };
                    if geometry::point_segment_distance(
                        branch.start_x,
                        branch.start_z,
                        segment.ax,
                        segment.az,
                        segment.bx,
                        segment.bz,
                    ) < 0.5
                    {
                        self.reject(RejectReason::SplitCollision);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::EndpointNodes => {
                    if self.state.cursor == self.input.nodes.len() {
                        self.state.phase = Phase::OpenRoads;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let node = self.input.nodes[self.state.cursor];
                    let branch = self.state.branch.unwrap();
                    if (node.x - branch.end_x).hypot(node.z - branch.end_z) < 18.0 {
                        self.reject(RejectReason::EndpointCollision);
                    } else {
                        self.state.cursor += 1;
                    }
                }
                Phase::OpenRoads => {
                    if self.state.cursor == self.input.roads.len() {
                        self.state.phase = Phase::Projects;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let road = self.input.roads[self.state.cursor];
                    self.state.cursor += 1;
                    if !road.is_open || road.id == self.state.candidate.host_id {
                        continue;
                    }
                    let Some(segment) = road.segment else {
                        self.reject(RejectReason::BranchRoadConflict);
                        continue;
                    };
                    let branch = self.state.branch.unwrap();
                    if geometry::segments_distance_coords(
                        branch.start_x,
                        branch.start_z,
                        branch.end_x,
                        branch.end_z,
                        segment.ax,
                        segment.az,
                        segment.bx,
                        segment.bz,
                    ) < 8.0
                    {
                        self.reject(RejectReason::BranchRoadConflict);
                    }
                }
                Phase::Projects => {
                    if self.state.cursor == self.input.projects.len() {
                        self.state.phase = Phase::BuildIntervals;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let project = self.input.projects[self.state.cursor];
                    self.state.cursor += 1;
                    if !project.active_road {
                        continue;
                    }
                    let segment = match project.endpoints {
                        ProjectEndpoints::NoIds => continue,
                        ProjectEndpoints::MissingNode => {
                            self.reject(RejectReason::BranchRoadConflict);
                            continue;
                        }
                        ProjectEndpoints::Segment(segment) => segment,
                    };
                    let branch = self.state.branch.unwrap();
                    if geometry::segments_distance_coords(
                        branch.start_x,
                        branch.start_z,
                        branch.end_x,
                        branch.end_z,
                        segment.ax,
                        segment.az,
                        segment.bx,
                        segment.bz,
                    ) < 8.0
                    {
                        self.reject(RejectReason::BranchRoadConflict);
                    }
                }
                Phase::BuildIntervals => {
                    work += 1;
                    let branch = self.state.branch.unwrap();
                    let intervals = crate::frontage::subdivide_frontage(
                        branch.length,
                        &[],
                        crate::FRONTAGE_ENDPOINT_CLEARANCE,
                        8.0,
                        14.0,
                        2.2,
                        MAX_LOTS_PER_SIDE,
                    );
                    self.state.intervals = intervals
                        .into_iter()
                        .map(|item| LotInterval {
                            start: item.start,
                            end: item.end,
                        })
                        .collect();
                    self.state.phase = Phase::BuildLot;
                    self.state.lot_cursor = 0;
                }
                Phase::BuildLot => {
                    let total_lots = self.state.intervals.len().saturating_mul(2);
                    if self.state.lot_cursor == total_lots {
                        self.state.phase = Phase::LotCapacity;
                        self.state.cursor = 0;
                        continue;
                    }
                    work += 1;
                    let branch = self.state.branch.unwrap();
                    let interval_index = self.state.lot_cursor % self.state.intervals.len();
                    let side = if self.state.lot_cursor < self.state.intervals.len() {
                        -1_i8
                    } else {
                        1_i8
                    };
                    let interval = self.state.intervals[interval_index];
                    let along_meters = (interval.start + interval.end) * 0.5;
                    let x = branch.start_x
                        + (branch.end_x - branch.start_x) * (along_meters / branch.length)
                        - branch.tangent_z * side as f32 * 15.0;
                    let z = branch.start_z
                        + (branch.end_z - branch.start_z) * (along_meters / branch.length)
                        + branch.tangent_x * side as f32 * 15.0;
                    let lot = InteriorAccessLot {
                        x,
                        z,
                        width: interval.end - interval.start,
                        depth: 12.0,
                        rotation: (side as f32 * (branch.end_z - branch.start_z))
                            .atan2(-side as f32 * (branch.end_x - branch.start_x)),
                        along: along_meters / branch.length,
                        side,
                    };
                    self.state.current_lot = Some(OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    });
                    self.state.phase = Phase::CheckLot;
                    let extra_road = Segment {
                        ax: branch.start_x,
                        az: branch.start_z,
                        bx: branch.end_x,
                        bz: branch.end_z,
                    };
                    self.lot_query = Some(
                        ViabilityQuery::new_with_extra_road(
                            self.state.current_lot.unwrap(),
                            self.input.viability.clone(),
                            extra_road,
                        )
                        .expect("validated input and finite candidate create a query"),
                    );
                    // Persist the candidate lot in a bounded slot until its
                    // geometry query finishes. It is not yet selected.
                    self.state.lots.push(lot);
                }
                Phase::CheckLot => {
                    let Some(query) = self.lot_query.as_mut() else {
                        self.reject(RejectReason::LotGeometry);
                        continue;
                    };
                    let step = query.advance(budget - work);
                    work += step.work_used;
                    if let Some(clear) = query.result() {
                        let lot = self.state.lots.pop().expect("built lot is pending");
                        self.lot_query = None;
                        self.state.current_lot_query = None;
                        self.state.current_lot = None;
                        if clear
                            && self.state.lots.len() < crate::interior_access::MAX_SELECTED_LOTS
                        {
                            self.state.lots.push(lot);
                        }
                        self.state.lot_cursor += 1;
                        self.state.phase = Phase::BuildLot;
                    } else if step.work_used == 0 {
                        // Empty arrays are normally settled by ViabilityQuery;
                        // guard against a malformed/non-progressing child.
                        self.reject(RejectReason::LotGeometry);
                    }
                }
                Phase::LotCapacity => {
                    work += 1;
                    let lots = self.state.lots.len();
                    if lots < 2
                        || self.input.gates.parcel_count.saturating_add(lots) > crate::MAX_PARCELS
                        || self.input.gates.frontage_count.saturating_add(lots) > crate::MAX_PARCELS
                    {
                        self.reject(RejectReason::InsufficientLots);
                    } else {
                        self.state.phase = Phase::SumArea;
                        self.state.area_cursor = 0;
                        self.state.area_cm2 = 0;
                    }
                }
                Phase::SumArea => {
                    if self.state.area_cursor == self.state.lots.len() {
                        self.state.phase = Phase::Done;
                        self.state.outcome = Some(SavedOutcome::Accepted);
                        continue;
                    }
                    work += 1;
                    let lot = self.state.lots[self.state.area_cursor];
                    self.state.area_cm2 += (lot.width * lot.depth * 10_000.0).round() as u32;
                    self.state.area_cursor += 1;
                }
                Phase::Done => break,
            }
        }
        ValidationAdvance {
            work_used: work,
            done: self.state.outcome.is_some(),
        }
    }

    fn derive_branch(&mut self) -> bool {
        let Some(host_index) = self.state.host_index else {
            return false;
        };
        let host = self.input.hosts[host_index];
        let candidate = self.state.candidate;
        let host_t = candidate.host_t;
        if !host_t.is_finite() {
            return false;
        }
        let dx = host.segment.bx - host.segment.ax;
        let dz = host.segment.bz - host.segment.az;
        let host_length = dx.hypot(dz);
        if !host_length.is_finite()
            || host_length <= 0.0
            || host_length * host_t < 10.0
            || host_length * (1.0 - host_t) < 10.0
        {
            return false;
        }
        let ux = dx / host_length;
        let uz = dz / host_length;
        let side = 1_i8;
        let normal_x = -uz * side as f32;
        let normal_z = ux * side as f32;
        let start_x = host.segment.ax + dx * host_t;
        let start_z = host.segment.az + dz * host_t;
        let end_x = candidate.end_x;
        let end_z = candidate.end_z;
        let length = (end_x - start_x).hypot(end_z - start_z);
        if !length.is_finite() || length < 24.0 - 0.01 || length > 60.0 + 0.01 {
            return false;
        }
        if [start_x, start_z, end_x, end_z].iter().any(|value| {
            !value.is_finite() || *value < crate::MIN + 24.0 || *value > crate::MAX - 24.0
        }) {
            return false;
        }
        let tangent_x = (end_x - start_x) / length;
        let tangent_z = (end_z - start_z) / length;
        self.state.branch = Some(DerivedBranch {
            host_from: host.from,
            host_to: host.to,
            start_x,
            start_z,
            end_x,
            end_z,
            length,
            normal_x,
            normal_z,
            tangent_x,
            tangent_z,
        });
        true
    }

    fn reject(&mut self, reason: RejectReason) {
        self.state.phase = Phase::Done;
        self.state.cursor = 0;
        self.state.current_lot = None;
        self.state.current_lot_query = None;
        self.state.intervals.clear();
        self.state.lot_cursor = 0;
        self.state.lots.clear();
        self.state.area_cursor = 0;
        self.state.area_cm2 = 0;
        self.lot_query = None;
        self.state.outcome = Some(SavedOutcome::Rejected(reason));
    }
}

fn validate_input(
    input: &ProposalValidationInput,
) -> Result<Arc<FrozenViabilityContext>, InputError> {
    if input.hosts.len() > MAX_HOSTS {
        return Err(InputError::TooManyHosts);
    }
    if input.nodes.len() > MAX_PREPARED_INPUT_NODES {
        return Err(InputError::TooManyNodes);
    }
    if input.roads.len() > MAX_ROADS {
        return Err(InputError::TooManyRoads);
    }
    if input.projects.len() > MAX_PROJECT_RECORDS {
        return Err(InputError::TooManyProjects);
    }
    if input.viability.parcel_footprints.len() > MAX_FOOTPRINTS {
        return Err(InputError::TooManyFootprints);
    }
    if input.hosts.windows(2).any(|pair| pair[0].id >= pair[1].id) {
        return Err(InputError::InvalidOrder);
    }
    if !input.gates.demand.is_finite()
        || input.gates.road_count > crate::MAX_SEGMENTS
        || input.gates.project_count > MAX_PROJECT_RECORDS
        || input.gates.parcel_count > crate::MAX_PARCELS
        || input.gates.frontage_count > crate::MAX_PARCELS
    {
        return Err(InputError::InvalidCapacity);
    }
    let finite_segment = |segment: Segment| {
        [segment.ax, segment.az, segment.bx, segment.bz]
            .iter()
            .all(|value| value.is_finite())
    };
    if input.hosts.iter().any(|host| !finite_segment(host.segment))
        || input
            .nodes
            .iter()
            .any(|node| !node.x.is_finite() || !node.z.is_finite())
        || input.roads.iter().any(|road| road.segment.is_some_and(|segment| !finite_segment(segment)))
        || input.projects.iter().any(|project| {
            matches!(project.endpoints, ProjectEndpoints::Segment(segment) if !finite_segment(segment))
        })
        || input.gates.pending_access_branch.is_some_and(|segment| !finite_segment(segment))
    {
        return Err(InputError::NonFiniteInput);
    }
    FrozenViabilityContext::new(input.viability.clone()).map_err(InputError::Viability)
}

fn fingerprint(input: &ProposalValidationInput) -> u64 {
    // Explicit checkpoint boundary only; the bounded DTO is at most a few
    // thousand records, and no input arrays are copied during `advance`.
    let bytes = serde_json::to_vec(input).expect("validated input serializes");
    bytes
        .into_iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(0x100000001b3)
        })
}

fn validate_checkpoint(
    state: &ProposalValidationCheckpoint,
    input: &ProposalValidationContext,
) -> Result<(), InputError> {
    if state.schema != CHECKPOINT_SCHEMA
        || state.input_fingerprint != input.fingerprint
        || !state.candidate.host_t.is_finite()
        || !state.candidate.end_x.is_finite()
        || !state.candidate.end_z.is_finite()
        || state.cursor
            > input
                .nodes
                .len()
                .max(input.roads.len())
                .max(input.projects.len())
                .max(input.viability.input().parcel_footprints.len())
                .max(input.viability.input().reserved_lots.len())
                .max(input.viability.input().open_roads.len())
                .max(input.viability.input().pending_roads.len())
                .max(input.viability.input().river_segments.len())
                .max(input.viability.input().neighborhood_segments.len())
                .max(17)
        || state.intervals.len() > MAX_LOTS_PER_SIDE
        || state
            .intervals
            .iter()
            .any(|item| !item.start.is_finite() || !item.end.is_finite() || item.start > item.end)
        || state.lots.len() > crate::interior_access::MAX_SELECTED_LOTS
        || state.lots.iter().any(|lot| {
            ![lot.x, lot.z, lot.width, lot.depth, lot.rotation, lot.along]
                .iter()
                .all(|value| value.is_finite())
                || lot.width <= 0.0
                || lot.depth <= 0.0
                || ![-1, 1].contains(&lot.side)
        })
        || state.area_cursor > state.lots.len()
        || state
            .host_index
            .is_some_and(|index| index >= input.hosts.len())
        || state.branch.is_some_and(|branch| {
            ![
                branch.start_x,
                branch.start_z,
                branch.end_x,
                branch.end_z,
                branch.length,
                branch.normal_x,
                branch.normal_z,
                branch.tangent_x,
                branch.tangent_z,
            ]
            .iter()
            .all(|value| value.is_finite())
                || branch.length <= 0.0
        })
        || state
            .current_lot
            .is_some_and(|rect| !rect.finite_positive())
        || state.lot_cursor > state.intervals.len().saturating_mul(2)
    {
        return Err(InputError::InvalidCapacity);
    }

    let expects_lot_query = state.phase == Phase::CheckLot;
    if state.current_lot_query.is_some() != expects_lot_query
        || state.current_lot.is_some() != expects_lot_query
        || state.outcome.is_some() != (state.phase == Phase::Done)
    {
        return Err(InputError::InvalidCapacity);
    }
    if state.phase == Phase::CheckLot {
        let Some(pending) = state.lots.last() else {
            return Err(InputError::InvalidCapacity);
        };
        let Some(rect) = state.current_lot else {
            return Err(InputError::InvalidCapacity);
        };
        let Some(branch) = state.branch else {
            return Err(InputError::InvalidCapacity);
        };
        let extra_road = Segment {
            ax: branch.start_x,
            az: branch.start_z,
            bx: branch.end_x,
            bz: branch.end_z,
        };
        if !lot_matches_rect(pending, rect)
            || !state
                .current_lot_query
                .as_ref()
                .is_some_and(|query| query.matches_query(rect, Some(extra_road)))
        {
            return Err(InputError::InvalidCapacity);
        }
    }
    let host_required = matches!(
        state.phase,
        Phase::DeriveBranch
            | Phase::River
            | Phase::ProtectedSamples
            | Phase::ReservedBranch
            | Phase::ReservedLots
            | Phase::ParcelCorridor
            | Phase::SplitNodes
            | Phase::SplitRoads
            | Phase::EndpointNodes
            | Phase::OpenRoads
            | Phase::Projects
            | Phase::BuildIntervals
            | Phase::BuildLot
            | Phase::CheckLot
            | Phase::LotCapacity
            | Phase::SumArea
    );
    let branch_required = matches!(
        state.phase,
        Phase::River
            | Phase::ProtectedSamples
            | Phase::ReservedBranch
            | Phase::ReservedLots
            | Phase::ParcelCorridor
            | Phase::SplitNodes
            | Phase::SplitRoads
            | Phase::EndpointNodes
            | Phase::OpenRoads
            | Phase::Projects
            | Phase::BuildIntervals
            | Phase::BuildLot
            | Phase::CheckLot
            | Phase::LotCapacity
            | Phase::SumArea
    );
    if (host_required && state.host_index.is_none())
        || (branch_required && state.branch.is_none())
        || state.host_index.is_some()
            && state.branch.is_some()
            && !host_required
            && state.phase != Phase::Done
        || state.branch.is_some() && !branch_required && state.phase != Phase::Done
    {
        return Err(InputError::InvalidCapacity);
    }
    if let Some(SavedOutcome::Accepted) = state.outcome {
        if state.host_index.is_none() || state.branch.is_none() || state.lots.len() < 2 {
            return Err(InputError::InvalidCapacity);
        }
    }
    let cursor_limit = match state.phase {
        Phase::Gates
        | Phase::DeriveBranch
        | Phase::ReservedBranch
        | Phase::BuildIntervals
        | Phase::BuildLot
        | Phase::CheckLot
        | Phase::LotCapacity
        | Phase::SumArea
        | Phase::Done => 0,
        Phase::ActiveProjects | Phase::Projects => input.projects.len(),
        Phase::FindHost => input.hosts.len(),
        Phase::River => input.viability.input().river_segments.len(),
        Phase::ProtectedSamples => 17,
        Phase::ReservedLots => input.viability.input().reserved_lots.len(),
        Phase::ParcelCorridor => input.viability.input().parcel_footprints.len(),
        Phase::SplitNodes | Phase::EndpointNodes => input.nodes.len(),
        Phase::SplitRoads | Phase::OpenRoads => input.roads.len(),
    };
    if state.cursor > cursor_limit {
        return Err(InputError::InvalidCapacity);
    }
    if matches!(
        state.phase,
        Phase::Gates
            | Phase::DeriveBranch
            | Phase::ReservedBranch
            | Phase::BuildIntervals
            | Phase::BuildLot
            | Phase::CheckLot
            | Phase::LotCapacity
            | Phase::SumArea
            | Phase::Done
    ) && state.cursor != 0
    {
        return Err(InputError::InvalidCapacity);
    }
    if (state.phase == Phase::BuildLot
        && state.lot_cursor > state.intervals.len().saturating_mul(2))
        || (state.phase == Phase::CheckLot
            && state.lot_cursor >= state.intervals.len().saturating_mul(2))
    {
        return Err(InputError::InvalidCapacity);
    }
    if let Some(query) = &state.current_lot_query {
        ViabilityQuery::restore(query.clone(), input.viability.clone())
            .map_err(InputError::Viability)?;
    }
    let completed_area = if matches!(state.outcome, Some(SavedOutcome::Rejected(_))) {
        0
    } else {
        state.area_cursor
    };
    let mut expected_area_cm2 = 0_u32;
    for lot in state.lots.iter().take(completed_area) {
        let area_cm2 = (lot.width * lot.depth * 10_000.0).round() as u32;
        expected_area_cm2 = expected_area_cm2
            .checked_add(area_cm2)
            .ok_or(InputError::InvalidCapacity)?;
    }
    if expected_area_cm2 != state.area_cm2
        || (state.phase != Phase::SumArea && state.phase != Phase::Done && state.area_cursor != 0)
    {
        return Err(InputError::InvalidCapacity);
    }
    Ok(())
}

fn lot_matches_rect(lot: &InteriorAccessLot, rect: OrientedRect) -> bool {
    lot.x == rect.x
        && lot.z == rect.z
        && lot.width == rect.width
        && lot.depth == rect.depth
        && lot.rotation == rect.rotation
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_input() -> ProposalValidationInput {
        ProposalValidationInput {
            hosts: vec![ProposalHost {
                id: 1,
                from: 1,
                to: 2,
                segment: Segment {
                    ax: 0.0,
                    az: 0.0,
                    bx: 80.0,
                    bz: 0.0,
                },
            }],
            nodes: vec![
                ProposalNode {
                    id: 1,
                    x: 0.0,
                    z: 0.0,
                },
                ProposalNode {
                    id: 2,
                    x: 80.0,
                    z: 0.0,
                },
            ],
            roads: vec![ProposalRoad {
                id: 1,
                is_open: true,
                segment: Some(Segment {
                    ax: 0.0,
                    az: 0.0,
                    bx: 80.0,
                    bz: 0.0,
                }),
            }],
            projects: Vec::new(),
            viability: FrozenViabilityInput {
                parcel_footprints: Vec::new(),
                reserved_lots: Vec::new(),
                open_roads: vec![Segment {
                    ax: 0.0,
                    az: 0.0,
                    bx: 80.0,
                    bz: 0.0,
                }],
                pending_roads: Vec::new(),
                river_segments: vec![Segment {
                    ax: 1000.0,
                    az: 0.0,
                    bx: 1000.0,
                    bz: 10.0,
                }],
                neighborhood_segments: Vec::new(),
                neighborhood_interior: None,
            },
            gates: AdmissionGates {
                demand: 1.0,
                has_pending_interior_plan: false,
                has_neighborhood_plan: false,
                road_count: 1,
                project_count: 0,
                parcel_count: 0,
                frontage_count: 0,
                next_id: 10,
                project_admission_available: true,
                pending_access_branch: None,
            },
        }
    }

    fn candidate() -> AccessGeometryCandidate {
        AccessGeometryCandidate {
            host_id: 1,
            host_t: 0.5,
            end_x: 40.0,
            end_z: 40.0,
            tie_index: 2,
        }
    }

    fn run(quota: usize, resume_every: usize) -> (Result<AccessProposal, RejectReason>, usize) {
        let context = ProposalValidationContext::new(simple_input()).unwrap();
        let mut validator = ProposalValidator::new(context.clone(), candidate());
        let mut total = 0;
        let mut since_save = 0;
        loop {
            let step = validator.advance(quota);
            assert!(step.work_used <= quota);
            total += step.work_used;
            since_save += step.work_used;
            if resume_every > 0 && since_save >= resume_every && !step.done {
                let bytes = serde_json::to_vec(&validator.checkpoint()).unwrap();
                validator = ProposalValidator::restore(
                    serde_json::from_slice(&bytes).unwrap(),
                    context.clone(),
                )
                .unwrap();
                since_save = 0;
            }
            if step.done {
                break;
            }
            assert!(step.work_used > 0, "active validator made no progress");
        }
        (validator.outcome().unwrap(), total)
    }

    #[test]
    fn explicit_candidate_is_quota_invariant_and_resumes_through_json() {
        let (expected, expected_work) = run(4096, 0);
        assert!(expected.is_ok(), "fixture candidate should produce lots");
        for quota in [1, 7, 48] {
            let (actual, work) = run(quota, 5);
            assert_eq!(actual, expected);
            assert_eq!(work, expected_work);
        }
        let proposal = expected.unwrap();
        assert!(proposal.selected_lots.len() >= 2);
        assert_eq!(proposal.score.host_id, 1);
        assert_eq!(proposal.score.fraction_index, 2);
        assert_eq!(proposal.score.side_index, 1);
        assert_eq!(proposal.score.length_index, 0);
    }

    #[test]
    fn zero_budget_and_terminal_advance_are_noops() {
        let context = ProposalValidationContext::new(simple_input()).unwrap();
        let mut validator = ProposalValidator::new(context, candidate());
        let before = serde_json::to_vec(&validator.checkpoint()).unwrap();
        assert_eq!(
            validator.advance(0),
            ValidationAdvance {
                work_used: 0,
                done: false
            }
        );
        assert_eq!(serde_json::to_vec(&validator.checkpoint()).unwrap(), before);
        loop {
            if validator.advance(48).done {
                break;
            }
        }
        assert_eq!(
            validator.advance(48),
            ValidationAdvance {
                work_used: 0,
                done: true
            }
        );
    }

    #[test]
    fn checkpoint_rejects_cursor_outside_its_phase() {
        let context = ProposalValidationContext::new(simple_input()).unwrap();
        let mut validator = ProposalValidator::new(context.clone(), candidate());
        assert_eq!(validator.advance(1).work_used, 1);
        let mut checkpoint = validator.checkpoint();
        checkpoint.phase = Phase::SplitNodes;
        checkpoint.cursor = context.nodes.len() + 1;
        assert_eq!(
            ProposalValidator::restore(checkpoint, context).unwrap_err(),
            InputError::InvalidCapacity
        );
    }

    #[test]
    fn checkpoint_rejects_missing_or_mismatched_pending_lot() {
        let context = ProposalValidationContext::new(simple_input()).unwrap();
        let mut validator = ProposalValidator::new(context.clone(), candidate());
        while validator.state.phase != Phase::CheckLot {
            let step = validator.advance(1);
            assert!(!step.done);
            assert!(step.work_used > 0);
        }
        let checkpoint = validator.checkpoint();
        let mut missing = checkpoint.clone();
        missing.lots.clear();
        assert_eq!(
            ProposalValidator::restore(missing, context.clone()).unwrap_err(),
            InputError::InvalidCapacity
        );

        let mut mismatched = checkpoint;
        mismatched.lots.last_mut().unwrap().x += 1.0;
        assert_eq!(
            ProposalValidator::restore(mismatched, context).unwrap_err(),
            InputError::InvalidCapacity
        );
    }

    #[test]
    fn checkpoint_rejects_forged_area_accumulator() {
        let context = ProposalValidationContext::new(simple_input()).unwrap();
        let mut validator = ProposalValidator::new(context.clone(), candidate());
        loop {
            if validator.state.phase == Phase::SumArea && validator.state.area_cursor == 0 {
                break;
            }
            let step = validator.advance(1);
            assert!(!step.done);
            assert!(step.work_used > 0);
        }
        validator.advance(1);
        let mut checkpoint = validator.checkpoint();
        checkpoint.area_cm2 = checkpoint.area_cm2.wrapping_add(1);
        assert_eq!(
            ProposalValidator::restore(checkpoint, context).unwrap_err(),
            InputError::InvalidCapacity
        );
    }

    #[test]
    fn input_rejects_nonfinite_geometry_and_unsorted_hosts() {
        let mut invalid = simple_input();
        invalid.hosts[0].segment.ax = f32::NAN;
        assert_eq!(
            ProposalValidationContext::new(invalid).unwrap_err(),
            InputError::NonFiniteInput
        );

        let mut invalid = simple_input();
        invalid.hosts = vec![
            ProposalHost {
                id: 2,
                ..invalid.hosts[0]
            },
            ProposalHost {
                id: 1,
                ..invalid.hosts[0]
            },
        ];
        assert_eq!(
            ProposalValidationContext::new(invalid).unwrap_err(),
            InputError::InvalidOrder
        );
    }

    #[test]
    fn input_accepts_full_engine_host_and_legacy_footprint_caps() {
        let mut input = simple_input();
        input.hosts = (0..MAX_HOSTS)
            .map(|id| ProposalHost {
                id: id as u32,
                from: id as u32,
                to: id as u32 + 1,
                segment: Segment {
                    ax: 0.0,
                    az: 0.0,
                    bx: 1.0,
                    bz: 0.0,
                },
            })
            .collect();
        input.viability.parcel_footprints = vec![
            OrientedRect {
                x: 0.0,
                z: 0.0,
                width: 1.0,
                depth: 1.0,
                rotation: 0.0,
            };
            MAX_FOOTPRINTS
        ];
        assert!(ProposalValidationContext::new(input).is_ok());
    }

    #[test]
    fn input_rejects_over_cap_hosts_and_footprints() {
        let mut hosts = simple_input();
        hosts.hosts = (0..=MAX_HOSTS)
            .map(|id| ProposalHost {
                id: id as u32,
                from: id as u32,
                to: id as u32 + 1,
                segment: Segment {
                    ax: 0.0,
                    az: 0.0,
                    bx: 1.0,
                    bz: 0.0,
                },
            })
            .collect();
        assert_eq!(
            ProposalValidationContext::new(hosts).unwrap_err(),
            InputError::TooManyHosts
        );

        let mut footprints = simple_input();
        footprints.viability.parcel_footprints = vec![
            OrientedRect {
                x: 0.0,
                z: 0.0,
                width: 1.0,
                depth: 1.0,
                rotation: 0.0,
            };
            MAX_FOOTPRINTS + 1
        ];
        assert_eq!(
            ProposalValidationContext::new(footprints).unwrap_err(),
            InputError::TooManyFootprints
        );
    }
}
