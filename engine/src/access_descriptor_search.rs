//! Fully resumable composition of host splits, face headings, ray queries,
//! endpoint candidates, and final deterministic ordering. This remains a pure
//! preparation tool; live engine admission is intentionally separate.

use crate::access_candidates::{self, BoundaryHost, CandidateNode, MAX_PREPARED_DESCRIPTORS};
use crate::access_rays::{RayCheckpoint, RayQuery, RayQueryKind, RayResult, ValidatedPolygon};
use crate::access_search::{
    self, AccessHostPreparation, Checkpoint as HostCheckpoint, HostSplitSample,
};
use crate::geometry::{rect_segment_distance, OrientedRect};
use crate::interior_access::AccessGeometryCandidate;
use crate::road_faces::RoadFace;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const SCHEMA: u8 = 3;
const MAX_HOSTS_INPUT: usize = 512;
const MAX_FACE_EDGES_INPUT: usize = 512;
const MAX_RECT_CORNERS: usize = 4;
const MAX_NEAREST_BLOCKERS: usize = 4;
const MAX_HEADING_COUNT: usize = 1 + access_candidates::MAX_TARGETS_PER_SPLIT * 3 + 4 * 4 * 2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FrozenFace {
    identity: Vec<u32>,
    vertex_ids: Vec<u32>,
    polygon: Vec<(f32, f32)>,
    area: f64,
    boundary_edge_ids: Vec<u32>,
    contains_proposed_edge: bool,
}

impl From<&RoadFace> for FrozenFace {
    fn from(face: &RoadFace) -> Self {
        Self {
            identity: face.identity.clone(),
            vertex_ids: face.vertex_ids.clone(),
            polygon: face.polygon.clone(),
            area: face.area,
            boundary_edge_ids: face.boundary_edge_ids.clone(),
            contains_proposed_edge: face.contains_proposed_edge,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Heading {
    x: f32,
    z: f32,
    /// `None` is the legacy positive-infinity sentinel for obstacle-derived
    /// headings; finite target-driven distances stay explicit.
    target_distance: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Blocker {
    nearest_distance: f32,
    rect: OrientedRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct CandidateRecord {
    host_id: u32,
    host_t: f32,
    end_x: f32,
    end_z: f32,
    length: f32,
}

impl CandidateRecord {
    fn candidate(self, tie_index: usize) -> AccessGeometryCandidate {
        AccessGeometryCandidate {
            host_id: self.host_id,
            host_t: self.host_t,
            end_x: self.end_x,
            end_z: self.end_z,
            tie_index: tie_index as u8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchError {
    InvalidInput,
    InputCap,
    DuplicateId,
    InvalidCheckpoint,
    InvalidCursor,
    InvalidScratch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    InitSortHosts,
    InitSortNodes,
    InitSortParcels,
    InitDeduplicateParcels,
    InitSortTargets,
    InitDeduplicateTargets,
    InitBuildEdgeOrder,
    InitSortEdgeOrder,
    InitProjectHosts,
    InitCopyNodes,
    InitCopyParcels,
    HostIntervals,
    CollectHostSplits,
    StartSplit,
    InitialInwardRay,
    Targets,
    DuplicateCheck,
    TargetHeadingRay,
    BlockerScan,
    InsertBlocker,
    Tangents,
    TangentRay,
    FallbackRay,
    StartHeading,
    HeadingRay,
    BuildLengths,
    SortLengths,
    DeduplicateLengths,
    StartEndpoint,
    EndpointSegment,
    SortCandidates,
    DeduplicateCandidates,
    BuildHostGroups,
    RoundRobin,
    Done,
}

impl Phase {
    fn is_initialization(self) -> bool {
        matches!(
            self,
            Self::InitSortHosts
                | Self::InitSortNodes
                | Self::InitSortParcels
                | Self::InitDeduplicateParcels
                | Self::InitSortTargets
                | Self::InitDeduplicateTargets
                | Self::InitBuildEdgeOrder
                | Self::InitSortEdgeOrder
                | Self::InitProjectHosts
                | Self::InitCopyNodes
                | Self::InitCopyParcels
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DuplicateAfter {
    Target,
    Tangent,
    Fallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum RayPurpose {
    InitialInward,
    TargetHeading { target_along: f32 },
    BlockerTangent,
    FallbackInward,
    HeadingLength,
    Endpoint { length: f32, end_x: f32, end_z: f32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DescriptorCheckpoint {
    schema: u8,
    face: FrozenFace,
    hosts: Vec<BoundaryHost>,
    nodes: Vec<CandidateNode>,
    parcels: Vec<OrientedRect>,
    targets: Vec<(f32, f32)>,
    phase: Phase,
    init_index: usize,
    init_compare: usize,
    init_swap_pending: bool,
    init_dedup_index: usize,
    normalized_parcels: Vec<OrientedRect>,
    normalized_targets: Vec<(f32, f32)>,
    edge_order: Vec<usize>,
    edge_order_index: usize,
    projected_hosts: Vec<crate::access_search::ProjectionHost>,
    edge_host_index: usize,
    host_search_low: usize,
    host_search_high: usize,
    host_found: Option<usize>,
    host_prep_nodes: Vec<CandidateNode>,
    host_prep_node_index: usize,
    host_prep_parcels: Vec<OrientedRect>,
    host_prep_parcel_index: usize,
    host_preparation: Option<HostCheckpoint>,
    split_samples: Vec<HostSplitSample>,
    split_index: usize,
    host_sample_copy_index: usize,
    ray: Option<RayCheckpoint>,
    ray_purpose: Option<RayPurpose>,
    headings: Vec<Heading>,
    target_index: usize,
    target_variant: usize,
    pending_heading: Option<Heading>,
    pending_target_along: f32,
    duplicate_after: Option<DuplicateAfter>,
    duplicate_index: usize,
    blocker_parcel_index: usize,
    blocker_target_index: usize,
    blocker_min_distance: Option<f32>,
    blockers: Vec<Blocker>,
    pending_blocker: Option<Blocker>,
    blocker_insert_index: usize,
    tangent_blocker_index: usize,
    tangent_corner_index: usize,
    tangent_side_index: usize,
    heading_index: usize,
    current_heading: Option<Heading>,
    ray_limit: f32,
    lengths: Vec<f32>,
    length_sort_index: usize,
    length_compare_index: usize,
    deduplicated_lengths: Vec<f32>,
    length_index: usize,
    candidates: Vec<CandidateRecord>,
    sort_width: usize,
    sort_run_start: usize,
    sort_left: usize,
    sort_mid: usize,
    sort_right: usize,
    sort_end: usize,
    sort_output: Vec<CandidateRecord>,
    candidate_index: usize,
    deduplicated_candidates: Vec<CandidateRecord>,
    host_groups: Vec<(u32, usize, usize)>,
    building_group: Option<(u32, usize, usize)>,
    group_scan_index: usize,
    round_index: usize,
    round_group_index: usize,
    round_added: bool,
    descriptors: Vec<AccessGeometryCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

/// Runtime wrapper. The serialized checkpoint is only constructed/examined
/// through validated `checkpoint`/`restore`; runtime query objects never
/// deserialize directly.
#[derive(Clone, Debug)]
pub(crate) struct DescriptorSearch {
    checkpoint: DescriptorCheckpoint,
    host_preparation: Option<AccessHostPreparation>,
    ray_query: Option<RayQuery>,
    polygon: ValidatedPolygon,
}

impl DescriptorSearch {
    pub(crate) fn new(
        face: &RoadFace,
        hosts: &[BoundaryHost],
        nodes: &[CandidateNode],
        parcels: &[OrientedRect],
        targets: &[(f32, f32)],
    ) -> Result<Self, SearchError> {
        validate_inputs(face, hosts, nodes, parcels, targets)?;
        let mut hosts = hosts.to_vec();
        hosts.sort_by_key(|host| host.road_id);
        let mut nodes = nodes.to_vec();
        nodes.sort_by_key(|node| node.id);
        let mut parcels = parcels.to_vec();
        parcels.sort_by(rect_order);
        parcels.dedup_by(|a, b| rect_order(a, b) == Ordering::Equal);
        let mut targets = targets.to_vec();
        targets.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
        targets.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        let host_preparation = AccessHostPreparation::new(face, &hosts, &nodes, &parcels)
            .map_err(|_| SearchError::InvalidInput)?;
        let polygon =
            ValidatedPolygon::new(face.polygon.clone()).map_err(|_| SearchError::InvalidInput)?;
        let initially_done = host_preparation.split_samples().is_some();
        let phase = if initially_done {
            Phase::Done
        } else {
            Phase::HostIntervals
        };
        let checkpoint = DescriptorCheckpoint {
            schema: SCHEMA,
            face: FrozenFace::from(face),
            hosts,
            nodes,
            parcels,
            targets,
            phase,
            init_index: 0,
            init_compare: 0,
            init_swap_pending: false,
            init_dedup_index: 0,
            normalized_parcels: Vec::new(),
            normalized_targets: Vec::new(),
            edge_order: Vec::new(),
            edge_order_index: 0,
            projected_hosts: Vec::new(),
            edge_host_index: 0,
            host_search_low: 0,
            host_search_high: 0,
            host_found: None,
            host_prep_nodes: Vec::new(),
            host_prep_node_index: 0,
            host_prep_parcels: Vec::new(),
            host_prep_parcel_index: 0,
            host_preparation: None,
            split_samples: Vec::new(),
            split_index: 0,
            host_sample_copy_index: 0,
            ray: None,
            ray_purpose: None,
            headings: Vec::new(),
            target_index: 0,
            target_variant: 0,
            pending_heading: None,
            pending_target_along: 0.0,
            duplicate_after: None,
            duplicate_index: 0,
            blocker_parcel_index: 0,
            blocker_target_index: 0,
            blocker_min_distance: None,
            blockers: Vec::new(),
            pending_blocker: None,
            blocker_insert_index: 0,
            tangent_blocker_index: 0,
            tangent_corner_index: 0,
            tangent_side_index: 0,
            heading_index: 0,
            current_heading: None,
            ray_limit: 0.0,
            lengths: Vec::new(),
            length_sort_index: 0,
            length_compare_index: 0,
            deduplicated_lengths: Vec::new(),
            length_index: 0,
            candidates: Vec::new(),
            sort_width: 1,
            sort_run_start: 0,
            sort_left: 0,
            sort_mid: 0,
            sort_right: 0,
            sort_end: 0,
            sort_output: Vec::new(),
            candidate_index: 0,
            deduplicated_candidates: Vec::new(),
            host_groups: Vec::new(),
            building_group: None,
            group_scan_index: 0,
            round_index: 0,
            round_group_index: 0,
            round_added: false,
            descriptors: Vec::new(),
        };
        Ok(Self {
            checkpoint,
            host_preparation: (!initially_done).then_some(host_preparation),
            ray_query: None,
            polygon,
        })
    }

    /// Consume frozen inputs and budget input normalization plus host
    /// projection through `advance`. Fixed-cap validation and face validation
    /// (including its bounded face copy) remain synchronous; input-array
    /// ordering, deduplication, host projection, and prep-array copies are
    /// charged to the incremental work quota.
    pub(crate) fn from_owned(
        face: RoadFace,
        hosts: Vec<BoundaryHost>,
        nodes: Vec<CandidateNode>,
        parcels: Vec<OrientedRect>,
        targets: Vec<(f32, f32)>,
    ) -> Result<Self, SearchError> {
        validate_inputs(&face, &hosts, &nodes, &parcels, &targets)?;
        let polygon =
            ValidatedPolygon::new(face.polygon.clone()).map_err(|_| SearchError::InvalidInput)?;
        let mut checkpoint = empty_checkpoint(
            FrozenFace::from(&face),
            hosts,
            nodes,
            parcels,
            targets,
            Phase::InitSortHosts,
        );
        reserve_initialization_storage(&mut checkpoint);
        Ok(Self {
            checkpoint,
            host_preparation: None,
            ray_query: None,
            polygon,
        })
    }

    pub(crate) fn restore(checkpoint: DescriptorCheckpoint) -> Result<Self, SearchError> {
        validate_checkpoint(&checkpoint)?;
        let mut checkpoint = checkpoint;
        if checkpoint.phase.is_initialization() {
            reserve_initialization_storage(&mut checkpoint);
        }
        let host_preparation = checkpoint
            .host_preparation
            .clone()
            .map(AccessHostPreparation::restore)
            .transpose()
            .map_err(|_| SearchError::InvalidCheckpoint)?;
        if checkpoint.phase == Phase::CollectHostSplits {
            let samples = host_preparation
                .as_ref()
                .and_then(AccessHostPreparation::split_samples)
                .ok_or(SearchError::InvalidCursor)?;
            if checkpoint.host_sample_copy_index > samples.len()
                || checkpoint.split_samples.len() != checkpoint.host_sample_copy_index
            {
                return Err(SearchError::InvalidCursor);
            }
        }
        let ray_query = checkpoint
            .ray
            .clone()
            .map(RayQuery::restore)
            .transpose()
            .map_err(|_| SearchError::InvalidCheckpoint)?;
        let polygon = ValidatedPolygon::new(checkpoint.face.polygon.clone())
            .map_err(|_| SearchError::InvalidCheckpoint)?;
        Ok(Self {
            checkpoint,
            host_preparation,
            ray_query,
            polygon,
        })
    }

    pub(crate) fn checkpoint(&self) -> DescriptorCheckpoint {
        let mut checkpoint = self.checkpoint.clone();
        checkpoint.host_preparation = self
            .host_preparation
            .as_ref()
            .map(AccessHostPreparation::checkpoint);
        checkpoint.ray = self.ray_query.as_ref().map(RayQuery::checkpoint);
        checkpoint
    }

    pub(crate) fn descriptors(&self) -> Option<&[AccessGeometryCandidate]> {
        (self.checkpoint.phase == Phase::Done).then_some(&self.checkpoint.descriptors)
    }

    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || self.checkpoint.phase == Phase::Done {
            return AdvanceResult {
                work_used: 0,
                done: self.checkpoint.phase == Phase::Done,
            };
        }
        let mut work_used = 0;
        while work_used < budget && self.checkpoint.phase != Phase::Done {
            if let Some(query) = self.ray_query.as_mut() {
                let result = query.advance(budget - work_used);
                work_used += result.work_used;
                if result.done && work_used < budget {
                    self.consume_ray_result();
                    work_used += 1;
                } else {
                    break;
                }
                continue;
            }
            if self.checkpoint.phase == Phase::HostIntervals {
                let Some(preparation) = self.host_preparation.as_mut() else {
                    break;
                };
                let result = preparation.advance(budget - work_used);
                work_used += result.work_used;
                if result.done {
                    self.checkpoint.host_sample_copy_index = 0;
                    self.checkpoint.phase = Phase::CollectHostSplits;
                }
                if result.work_used == 0 && !result.done {
                    break;
                }
                continue;
            }
            if self.checkpoint.phase.is_initialization() {
                self.advance_initialization();
                work_used += 1;
                continue;
            }
            self.advance_one();
            work_used += 1;
        }
        AdvanceResult {
            work_used,
            done: self.checkpoint.phase == Phase::Done,
        }
    }

    fn advance_one(&mut self) {
        match self.checkpoint.phase {
            Phase::InitSortHosts
            | Phase::InitSortNodes
            | Phase::InitSortParcels
            | Phase::InitDeduplicateParcels
            | Phase::InitSortTargets
            | Phase::InitDeduplicateTargets
            | Phase::InitBuildEdgeOrder
            | Phase::InitSortEdgeOrder
            | Phase::InitProjectHosts
            | Phase::InitCopyNodes
            | Phase::InitCopyParcels => unreachable!("initialization dispatcher"),
            Phase::StartSplit => self.start_split(),
            Phase::CollectHostSplits => self.collect_host_split(),
            Phase::InitialInwardRay => self.begin_initial_ray(),
            Phase::Targets => self.advance_target(),
            Phase::DuplicateCheck => self.advance_duplicate_check(),
            Phase::TargetHeadingRay => self.begin_target_heading_ray(),
            Phase::BlockerScan => self.advance_blocker_scan(),
            Phase::InsertBlocker => self.advance_blocker_insert(),
            Phase::Tangents => self.advance_tangent(),
            Phase::TangentRay => self.begin_tangent_ray(),
            Phase::FallbackRay => self.begin_fallback_ray(),
            Phase::StartHeading => self.start_heading(),
            Phase::HeadingRay => self.begin_heading_ray(),
            Phase::BuildLengths => self.build_lengths(),
            Phase::SortLengths => self.sort_lengths(),
            Phase::DeduplicateLengths => self.deduplicate_lengths(),
            Phase::StartEndpoint => self.start_endpoint(),
            Phase::EndpointSegment => self.begin_endpoint_query(),
            Phase::SortCandidates => self.sort_candidates(),
            Phase::DeduplicateCandidates => self.deduplicate_candidates(),
            Phase::BuildHostGroups => self.build_host_groups(),
            Phase::RoundRobin => self.round_robin(),
            Phase::HostIntervals | Phase::Done => {}
        }
    }

    fn advance_initialization(&mut self) {
        match self.checkpoint.phase {
            Phase::InitSortHosts => self.advance_insertion_sort_hosts(),
            Phase::InitSortNodes => self.advance_insertion_sort_nodes(),
            Phase::InitSortParcels => self.advance_insertion_sort_parcels(),
            Phase::InitDeduplicateParcels => self.advance_deduplicate_parcels(),
            Phase::InitSortTargets => self.advance_insertion_sort_targets(),
            Phase::InitDeduplicateTargets => self.advance_deduplicate_targets(),
            Phase::InitBuildEdgeOrder => self.advance_build_edge_order(),
            Phase::InitSortEdgeOrder => self.advance_sort_edge_order(),
            Phase::InitProjectHosts => self.advance_project_hosts(),
            Phase::InitCopyNodes => self.advance_copy_host_nodes(),
            Phase::InitCopyParcels => self.advance_copy_host_parcels(),
            _ => unreachable!("initialization dispatcher phase"),
        }
    }

    fn advance_insertion_sort_hosts(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.init_index >= cp.hosts.len() {
            cp.phase = Phase::InitSortNodes;
            cp.init_index = 1;
            cp.init_compare = 1;
            return;
        }
        if cp.init_index == 0 {
            cp.init_index = 1;
            cp.init_compare = 1;
            return;
        }
        if cp.init_compare == 0 {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
            return;
        }
        if cp.init_swap_pending {
            cp.hosts.swap(cp.init_compare, cp.init_compare - 1);
            cp.init_compare -= 1;
            cp.init_swap_pending = false;
            return;
        }
        if cp.hosts[cp.init_compare].road_id < cp.hosts[cp.init_compare - 1].road_id {
            cp.init_swap_pending = true;
        } else {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
        }
    }

    fn advance_insertion_sort_nodes(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.init_index >= cp.nodes.len() {
            cp.phase = Phase::InitSortParcels;
            cp.init_index = 1;
            cp.init_compare = 1;
            return;
        }
        if cp.init_index == 0 {
            cp.init_index = 1;
            cp.init_compare = 1;
            return;
        }
        if cp.init_compare == 0 {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
            return;
        }
        if cp.init_swap_pending {
            cp.nodes.swap(cp.init_compare, cp.init_compare - 1);
            cp.init_compare -= 1;
            cp.init_swap_pending = false;
            return;
        }
        if cp.nodes[cp.init_compare].id < cp.nodes[cp.init_compare - 1].id {
            cp.init_swap_pending = true;
        } else {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
        }
    }

    fn advance_insertion_sort_parcels(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.init_index >= cp.parcels.len() {
            cp.phase = Phase::InitDeduplicateParcels;
            cp.init_dedup_index = 0;
            return;
        }
        if cp.init_index == 0 {
            cp.init_index = 1;
            cp.init_compare = 1;
            return;
        }
        if cp.init_compare == 0 {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
            return;
        }
        if cp.init_swap_pending {
            cp.parcels.swap(cp.init_compare, cp.init_compare - 1);
            cp.init_compare -= 1;
            cp.init_swap_pending = false;
            return;
        }
        if rect_order(
            &cp.parcels[cp.init_compare],
            &cp.parcels[cp.init_compare - 1],
        ) == Ordering::Less
        {
            cp.init_swap_pending = true;
        } else {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
        }
    }

    fn advance_deduplicate_parcels(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.init_dedup_index >= cp.parcels.len() {
            cp.parcels = std::mem::take(&mut cp.normalized_parcels);
            cp.phase = Phase::InitSortTargets;
            cp.init_index = 1;
            cp.init_compare = 1;
            return;
        }
        let value = cp.parcels[cp.init_dedup_index];
        if cp
            .normalized_parcels
            .last()
            .is_none_or(|last| rect_order(last, &value) != Ordering::Equal)
        {
            cp.normalized_parcels.push(value);
        }
        cp.init_dedup_index += 1;
    }

    fn advance_insertion_sort_targets(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.init_index >= cp.targets.len() {
            cp.phase = Phase::InitDeduplicateTargets;
            cp.init_dedup_index = 0;
            return;
        }
        if cp.init_index == 0 {
            cp.init_index = 1;
            cp.init_compare = 1;
            return;
        }
        if cp.init_compare == 0 {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
            return;
        }
        if cp.init_swap_pending {
            cp.targets.swap(cp.init_compare, cp.init_compare - 1);
            cp.init_compare -= 1;
            cp.init_swap_pending = false;
            return;
        }
        let order = cp.targets[cp.init_compare - 1]
            .0
            .total_cmp(&cp.targets[cp.init_compare].0)
            .then_with(|| {
                cp.targets[cp.init_compare - 1]
                    .1
                    .total_cmp(&cp.targets[cp.init_compare].1)
            });
        if order == Ordering::Greater {
            cp.init_swap_pending = true;
        } else {
            cp.init_index += 1;
            cp.init_compare = cp.init_index;
        }
    }

    fn advance_deduplicate_targets(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.init_dedup_index >= cp.targets.len() {
            cp.targets = std::mem::take(&mut cp.normalized_targets);
            cp.phase = Phase::InitBuildEdgeOrder;
            cp.init_index = 0;
            return;
        }
        let value = cp.targets[cp.init_dedup_index];
        if cp
            .normalized_targets
            .last()
            .is_none_or(|last| last.0 != value.0 || last.1 != value.1)
        {
            cp.normalized_targets.push(value);
        }
        cp.init_dedup_index += 1;
    }

    fn advance_build_edge_order(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.init_index >= cp.face.boundary_edge_ids.len() {
            cp.phase = Phase::InitSortEdgeOrder;
            cp.edge_order_index = 1;
            cp.init_compare = 1;
            return;
        }
        cp.edge_order.push(cp.init_index);
        cp.init_index += 1;
    }

    fn advance_sort_edge_order(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.edge_order_index >= cp.edge_order.len() {
            cp.phase = Phase::InitProjectHosts;
            cp.edge_host_index = 0;
            cp.host_search_low = 0;
            cp.host_search_high = cp.hosts.len();
            cp.host_found = None;
            return;
        }
        if cp.edge_order_index == 0 {
            cp.edge_order_index = 1;
            cp.init_compare = 1;
            return;
        }
        if cp.init_compare == 0 {
            cp.edge_order_index += 1;
            cp.init_compare = cp.edge_order_index;
            return;
        }
        if cp.init_swap_pending {
            cp.edge_order.swap(cp.init_compare - 1, cp.init_compare);
            cp.init_compare -= 1;
            cp.init_swap_pending = false;
            return;
        }
        let a = cp.edge_order[cp.init_compare - 1];
        let b = cp.edge_order[cp.init_compare];
        if cp.face.boundary_edge_ids[a] > cp.face.boundary_edge_ids[b] {
            cp.init_swap_pending = true;
        } else {
            cp.edge_order_index += 1;
            cp.init_compare = cp.edge_order_index;
        }
    }

    fn advance_project_hosts(&mut self) {
        let cp = &mut self.checkpoint;
        let limit = cp.edge_order.len().min(access_search::MAX_PREPARED_HOSTS);
        if cp.edge_host_index >= limit {
            cp.phase = Phase::InitCopyNodes;
            cp.host_prep_node_index = 0;
            return;
        }
        if let Some(found) = cp.host_found.take() {
            let edge_index = cp.edge_order[cp.edge_host_index];
            let host = cp.hosts[found];
            let (ax, az) = cp.face.polygon[edge_index];
            let (bx, bz) = cp.face.polygon[(edge_index + 1) % cp.face.polygon.len()];
            let edx = bx - ax;
            let edz = bz - az;
            let hdx = host.bx - host.ax;
            let hdz = host.bz - host.az;
            let edge_len = crate::access_math::length(edx, edz);
            let host_len = crate::access_math::length(hdx, hdz);
            if edge_len.is_finite()
                && host_len.is_finite()
                && host_len >= 20.0
                && (edx * hdx + edz * hdz).abs() >= edge_len * host_len * 0.999
            {
                cp.projected_hosts
                    .push(crate::access_search::ProjectionHost {
                        road: host,
                        inward_x: -edz / edge_len,
                        inward_z: edx / edge_len,
                    });
            }
            cp.edge_host_index += 1;
            cp.host_search_low = 0;
            cp.host_search_high = cp.hosts.len();
            return;
        }
        if cp.host_search_low >= cp.host_search_high {
            cp.edge_host_index += 1;
            cp.host_search_low = 0;
            cp.host_search_high = cp.hosts.len();
            return;
        }
        let mid = cp.host_search_low + (cp.host_search_high - cp.host_search_low) / 2;
        let wanted = cp.face.boundary_edge_ids[cp.edge_order[cp.edge_host_index]];
        match cp.hosts[mid].road_id.cmp(&wanted) {
            Ordering::Less => cp.host_search_low = mid + 1,
            Ordering::Greater => cp.host_search_high = mid,
            Ordering::Equal => cp.host_found = Some(mid),
        }
    }

    fn advance_copy_host_nodes(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.host_prep_node_index < cp.nodes.len() {
            cp.host_prep_nodes.push(cp.nodes[cp.host_prep_node_index]);
            cp.host_prep_node_index += 1;
            return;
        }
        cp.phase = Phase::InitCopyParcels;
        cp.host_prep_parcel_index = 0;
    }

    fn advance_copy_host_parcels(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.host_prep_parcel_index < cp.parcels.len() {
            cp.host_prep_parcels
                .push(cp.parcels[cp.host_prep_parcel_index]);
            cp.host_prep_parcel_index += 1;
            return;
        }
        let hosts = std::mem::take(&mut cp.projected_hosts);
        let nodes = std::mem::take(&mut cp.host_prep_nodes);
        let parcels = std::mem::take(&mut cp.host_prep_parcels);
        self.host_preparation = Some(AccessHostPreparation::from_projected_owned(
            hosts, nodes, parcels,
        ));
        cp.phase = Phase::HostIntervals;
    }

    fn current_sample(&self) -> HostSplitSample {
        self.checkpoint.split_samples[self.checkpoint.split_index]
    }

    fn current_host(&self) -> BoundaryHost {
        let id = self.current_sample().road_id;
        let hosts = &self.checkpoint.hosts;
        let index = hosts
            .binary_search_by_key(&id, |host| host.road_id)
            .expect("validated split host");
        hosts[index]
    }

    fn split_start(&self) -> (f32, f32) {
        let host = self.current_host();
        let sample = self.current_sample();
        (
            host.ax + (host.bx - host.ax) * sample.host_t,
            host.az + (host.bz - host.az) * sample.host_t,
        )
    }

    fn host_heading(&self) -> (f32, f32) {
        let host = self.current_host();
        let length = crate::access_math::length(host.bx - host.ax, host.bz - host.az);
        ((host.bx - host.ax) / length, (host.bz - host.az) / length)
    }

    fn start_split(&mut self) {
        if self.checkpoint.split_index >= self.checkpoint.split_samples.len() {
            self.checkpoint.phase = Phase::SortCandidates;
            self.initialize_candidate_sort();
            return;
        }
        let cp = &mut self.checkpoint;
        cp.headings.clear();
        cp.target_index = 0;
        cp.target_variant = 0;
        cp.pending_heading = None;
        cp.pending_target_along = 0.0;
        cp.duplicate_after = None;
        cp.duplicate_index = 0;
        cp.blocker_parcel_index = 0;
        cp.blocker_target_index = 0;
        cp.blocker_min_distance = None;
        cp.blockers.clear();
        cp.pending_blocker = None;
        cp.blocker_insert_index = 0;
        cp.tangent_blocker_index = 0;
        cp.tangent_corner_index = 0;
        cp.tangent_side_index = 0;
        cp.heading_index = 0;
        cp.current_heading = None;
        cp.ray_limit = 0.0;
        cp.lengths.clear();
        cp.length_sort_index = 0;
        cp.length_compare_index = 0;
        cp.deduplicated_lengths.clear();
        cp.length_index = 0;
        cp.phase = Phase::InitialInwardRay;
    }

    fn collect_host_split(&mut self) {
        let Some(preparation) = self.host_preparation.as_ref() else {
            self.checkpoint.phase = Phase::Done;
            return;
        };
        let samples = preparation.split_samples().unwrap_or(&[]);
        if self.checkpoint.host_sample_copy_index < samples.len() {
            self.checkpoint
                .split_samples
                .push(samples[self.checkpoint.host_sample_copy_index]);
            self.checkpoint.host_sample_copy_index += 1;
            return;
        }
        self.host_preparation = None;
        self.checkpoint.host_preparation = None;
        self.checkpoint.phase = if self.checkpoint.split_samples.is_empty() {
            Phase::Done
        } else {
            Phase::StartSplit
        };
    }

    fn begin_initial_ray(&mut self) {
        let start = self.split_start();
        let inward = self.current_sample();
        self.begin_ray(
            RayQueryKind::Exit {
                start,
                heading: (inward.inward_x, inward.inward_z),
            },
            RayPurpose::InitialInward,
        );
    }

    fn advance_target(&mut self) {
        let host_direction = self.host_heading();
        let start = self.split_start();
        let cp = &mut self.checkpoint;
        if cp.headings.len() >= MAX_HEADING_COUNT || cp.target_index >= cp.targets.len() {
            cp.phase = Phase::BlockerScan;
            cp.blocker_parcel_index = 0;
            cp.blocker_target_index = 0;
            cp.blocker_min_distance = None;
            return;
        }
        let target = cp.targets[cp.target_index];
        let delta = (target.0 - start.0, target.1 - start.1);
        let distance = crate::access_math::length(delta.0, delta.1);
        let inward = {
            let sample = cp.split_samples[cp.split_index];
            (sample.inward_x, sample.inward_z)
        };
        if !distance.is_finite()
            || distance < access_candidates::FRONTAGE_TARGET_MARGIN_M
            || access_candidates::dot(inward, delta) <= 0.0
        {
            cp.target_index += 1;
            cp.target_variant = 0;
            return;
        }
        if cp.target_variant >= 3 {
            cp.target_index += 1;
            cp.target_variant = 0;
            return;
        }
        let direct = (delta.0 / distance, delta.1 / distance);
        let sides = crate::access_math::rotate_clearance_sides(
            delta.0,
            delta.1,
            distance,
            access_candidates::FRONTAGE_TARGET_MARGIN_M,
        );
        let direction = match cp.target_variant {
            0 => sides[0],
            1 => sides[1],
            _ => direct,
        };
        cp.target_variant += 1;
        if access_candidates::dot(inward, direction) <= 0.0
            || !access_candidates::junction_angle_ok(host_direction, direction)
        {
            return;
        }
        cp.pending_heading = Some(Heading {
            x: direction.0,
            z: direction.1,
            target_distance: Some(
                access_candidates::dot(delta, direction)
                    + access_candidates::FRONTAGE_TARGET_MARGIN_M,
            ),
        });
        cp.pending_target_along = access_candidates::dot(delta, direction);
        cp.duplicate_index = 0;
        cp.duplicate_after = Some(DuplicateAfter::Target);
        cp.phase = Phase::DuplicateCheck;
    }

    fn advance_duplicate_check(&mut self) {
        let cp = &mut self.checkpoint;
        let pending = cp.pending_heading.expect("duplicate check has candidate");
        if cp.duplicate_index >= cp.headings.len() {
            cp.phase = match cp.duplicate_after.take().unwrap() {
                DuplicateAfter::Target => Phase::TargetHeadingRay,
                DuplicateAfter::Tangent => Phase::TangentRay,
                DuplicateAfter::Fallback => Phase::FallbackRay,
            };
            return;
        }
        let old = cp.headings[cp.duplicate_index];
        cp.duplicate_index += 1;
        if access_candidates::dot((old.x, old.z), (pending.x, pending.z)) > 0.9999 {
            cp.pending_heading = None;
            cp.phase = match cp.duplicate_after.take().unwrap() {
                DuplicateAfter::Target => Phase::Targets,
                DuplicateAfter::Tangent => Phase::Tangents,
                DuplicateAfter::Fallback => Phase::StartHeading,
            };
        }
    }

    fn begin_target_heading_ray(&mut self) {
        let start = self.split_start();
        let heading = self.checkpoint.pending_heading.unwrap();
        self.begin_ray(
            RayQueryKind::Exit {
                start,
                heading: (heading.x, heading.z),
            },
            RayPurpose::TargetHeading {
                target_along: self.checkpoint.pending_target_along,
            },
        );
    }

    fn advance_blocker_scan(&mut self) {
        let start = self.split_start();
        let cp = &mut self.checkpoint;
        if cp.blocker_parcel_index >= cp.parcels.len() || cp.targets.is_empty() {
            cp.tangent_blocker_index = 0;
            cp.tangent_corner_index = 0;
            cp.tangent_side_index = 0;
            cp.phase = Phase::Tangents;
            return;
        }
        if cp.blocker_target_index >= cp.targets.len() {
            if cp.blocker_min_distance.unwrap_or(f32::INFINITY)
                < access_candidates::PARCEL_HOST_CLEARANCE_M
            {
                cp.pending_blocker = Some(Blocker {
                    nearest_distance: cp.blocker_min_distance.unwrap(),
                    rect: cp.parcels[cp.blocker_parcel_index],
                });
                cp.blocker_insert_index = 0;
                cp.phase = Phase::InsertBlocker;
            } else {
                cp.blocker_parcel_index += 1;
                cp.blocker_target_index = 0;
                cp.blocker_min_distance = None;
                cp.phase = Phase::BlockerScan;
            }
            return;
        }
        let target = cp.targets[cp.blocker_target_index];
        let parcel = cp.parcels[cp.blocker_parcel_index];
        let distance = rect_segment_distance(&parcel, start.0, start.1, target.0, target.1);
        cp.blocker_min_distance = Some(
            cp.blocker_min_distance
                .map_or(distance, |old| old.min(distance)),
        );
        cp.blocker_target_index += 1;
    }

    fn advance_blocker_insert(&mut self) {
        let cp = &mut self.checkpoint;
        let candidate = cp.pending_blocker.expect("pending blocker");
        if cp.blocker_insert_index < cp.blockers.len() {
            let index = cp.blocker_insert_index;
            cp.blocker_insert_index += 1;
            if blocker_order(&candidate, &cp.blockers[index]) == Ordering::Less {
                cp.blockers.insert(index, candidate);
                cp.blockers.truncate(MAX_NEAREST_BLOCKERS);
                cp.pending_blocker = None;
                cp.blocker_parcel_index += 1;
                cp.blocker_target_index = 0;
                cp.blocker_min_distance = None;
                cp.phase = Phase::BlockerScan;
            }
            return;
        }
        if cp.blockers.len() < MAX_NEAREST_BLOCKERS {
            cp.blockers.push(candidate);
        }
        cp.pending_blocker = None;
        cp.blocker_parcel_index += 1;
        cp.blocker_target_index = 0;
        cp.blocker_min_distance = None;
        cp.phase = Phase::BlockerScan;
    }

    fn advance_tangent(&mut self) {
        let host_direction = self.host_heading();
        let start = self.split_start();
        let cp = &mut self.checkpoint;
        if cp.headings.len() >= MAX_HEADING_COUNT || cp.tangent_blocker_index >= cp.blockers.len() {
            if cp.headings.len() < 2 {
                let inward = cp.split_samples[cp.split_index];
                cp.pending_heading = Some(Heading {
                    x: inward.inward_x,
                    z: inward.inward_z,
                    target_distance: None,
                });
                cp.duplicate_index = 0;
                cp.duplicate_after = Some(DuplicateAfter::Fallback);
                cp.phase = Phase::DuplicateCheck;
            } else {
                cp.phase = Phase::StartHeading;
            }
            return;
        }
        let blocker = cp.blockers[cp.tangent_blocker_index];
        let corners = blocker.rect.corners();
        if cp.tangent_corner_index >= corners.len() {
            cp.tangent_blocker_index += 1;
            cp.tangent_corner_index = 0;
            cp.tangent_side_index = 0;
            return;
        }
        let corner = corners[cp.tangent_corner_index];
        let dx = corner.0 - start.0;
        let dz = corner.1 - start.1;
        let distance = crate::access_math::length(dx, dz);
        if distance <= access_candidates::PARCEL_HOST_CLEARANCE_M + 0.5 {
            cp.tangent_corner_index += 1;
            cp.tangent_side_index = 0;
            return;
        }
        let tangent_directions = crate::access_math::rotate_clearance_sides(
            dx,
            dz,
            distance,
            access_candidates::PARCEL_HOST_CLEARANCE_M + 0.5,
        );
        let direction = tangent_directions[cp.tangent_side_index as usize];
        cp.tangent_side_index += 1;
        if cp.tangent_side_index >= 2 {
            cp.tangent_side_index = 0;
            cp.tangent_corner_index += 1;
        }
        let sample = cp.split_samples[cp.split_index];
        let inward = (sample.inward_x, sample.inward_z);
        if access_candidates::dot(inward, direction) <= 0.0
            || !access_candidates::junction_angle_ok(host_direction, direction)
        {
            return;
        }
        cp.pending_heading = Some(Heading {
            x: direction.0,
            z: direction.1,
            target_distance: None,
        });
        cp.duplicate_index = 0;
        cp.duplicate_after = Some(DuplicateAfter::Tangent);
        cp.phase = Phase::DuplicateCheck;
    }

    fn begin_tangent_ray(&mut self) {
        let start = self.split_start();
        let heading = self.checkpoint.pending_heading.unwrap();
        self.begin_ray(
            RayQueryKind::Exit {
                start,
                heading: (heading.x, heading.z),
            },
            RayPurpose::BlockerTangent,
        );
    }

    fn begin_fallback_ray(&mut self) {
        let start = self.split_start();
        let heading = self.checkpoint.pending_heading.unwrap();
        self.begin_ray(
            RayQueryKind::Exit {
                start,
                heading: (heading.x, heading.z),
            },
            RayPurpose::FallbackInward,
        );
    }

    fn start_heading(&mut self) {
        if self.checkpoint.heading_index >= self.checkpoint.headings.len() {
            self.checkpoint.split_index += 1;
            self.checkpoint.phase = Phase::StartSplit;
            return;
        }
        self.checkpoint.current_heading =
            Some(self.checkpoint.headings[self.checkpoint.heading_index]);
        self.checkpoint.phase = Phase::HeadingRay;
    }

    fn begin_heading_ray(&mut self) {
        let start = self.split_start();
        let heading = self.checkpoint.current_heading.unwrap();
        self.begin_ray(
            RayQueryKind::Exit {
                start,
                heading: (heading.x, heading.z),
            },
            RayPurpose::HeadingLength,
        );
    }

    fn build_lengths(&mut self) {
        let cp = &mut self.checkpoint;
        let heading = cp.current_heading.unwrap();
        cp.lengths.clear();
        if let Some(target_distance) = heading
            .target_distance
            .filter(|distance| *distance <= cp.ray_limit)
        {
            cp.lengths.push(target_distance);
            cp.lengths.push(
                (target_distance - access_candidates::FRONTAGE_TARGET_MARGIN_M)
                    .max(access_candidates::MIN_BRANCH_M),
            );
            cp.lengths.push(cp.ray_limit);
        } else {
            cp.lengths
                .extend([access_candidates::MIN_BRANCH_M, 36.0, 48.0, cp.ray_limit]);
        }
        cp.lengths
            .retain(|length| *length >= access_candidates::MIN_BRANCH_M && *length <= cp.ray_limit);
        cp.length_sort_index = 1;
        cp.length_compare_index = 1;
        cp.phase = Phase::SortLengths;
    }

    fn sort_lengths(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.length_sort_index >= cp.lengths.len() {
            cp.deduplicated_lengths.clear();
            cp.length_index = 0;
            cp.phase = Phase::DeduplicateLengths;
            return;
        }
        if cp.length_compare_index > 0
            && cp.lengths[cp.length_compare_index]
                .total_cmp(&cp.lengths[cp.length_compare_index - 1])
                == Ordering::Less
        {
            let index = cp.length_compare_index;
            cp.lengths.swap(index, index - 1);
            cp.length_compare_index -= 1;
        } else {
            cp.length_sort_index += 1;
            cp.length_compare_index = cp.length_sort_index;
        }
    }

    fn deduplicate_lengths(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.length_index >= cp.lengths.len() {
            cp.length_index = 0;
            cp.phase = Phase::StartEndpoint;
            return;
        }
        let value = cp.lengths[cp.length_index];
        cp.length_index += 1;
        if cp
            .deduplicated_lengths
            .last()
            .is_none_or(|last| (*last - value).abs() >= 1.0e-4)
        {
            cp.deduplicated_lengths.push(value);
        }
    }

    fn start_endpoint(&mut self) {
        if self.checkpoint.length_index >= self.checkpoint.deduplicated_lengths.len() {
            self.checkpoint.heading_index += 1;
            self.checkpoint.current_heading = None;
            self.checkpoint.phase = Phase::StartHeading;
        } else {
            self.checkpoint.phase = Phase::EndpointSegment;
        }
    }

    fn begin_endpoint_query(&mut self) {
        let start = self.split_start();
        let heading = self.checkpoint.current_heading.unwrap();
        let length = self.checkpoint.deduplicated_lengths[self.checkpoint.length_index];
        let end_x = start.0 + heading.x * length;
        let end_z = start.1 + heading.z * length;
        self.begin_ray(
            RayQueryKind::Segment {
                start,
                end: (end_x, end_z),
            },
            RayPurpose::Endpoint {
                length,
                end_x,
                end_z,
            },
        );
    }

    fn initialize_candidate_sort(&mut self) {
        let cp = &mut self.checkpoint;
        cp.sort_width = 1;
        cp.sort_run_start = 0;
        cp.sort_output.clear();
        cp.sort_output.reserve(cp.candidates.len());
        initialize_candidate_merge_run(cp);
    }

    fn sort_candidates(&mut self) {
        let cp = &mut self.checkpoint;
        let len = cp.candidates.len();
        if len <= 1 {
            reset_candidate_sort(cp);
            cp.phase = Phase::DeduplicateCandidates;
            cp.candidate_index = 0;
            return;
        }
        if cp.sort_left < cp.sort_mid && cp.sort_right < cp.sort_end {
            let left = cp.candidates[cp.sort_left];
            let right = cp.candidates[cp.sort_right];
            if candidate_order(&left, &right) != Ordering::Greater {
                cp.sort_output.push(left);
                cp.sort_left += 1;
            } else {
                cp.sort_output.push(right);
                cp.sort_right += 1;
            }
            return;
        }
        if cp.sort_left < cp.sort_mid {
            cp.sort_output.push(cp.candidates[cp.sort_left]);
            cp.sort_left += 1;
            return;
        }
        if cp.sort_right < cp.sort_end {
            cp.sort_output.push(cp.candidates[cp.sort_right]);
            cp.sort_right += 1;
            return;
        }
        cp.sort_run_start = cp
            .sort_run_start
            .saturating_add(cp.sort_width.saturating_mul(2));
        if cp.sort_run_start >= len {
            std::mem::swap(&mut cp.candidates, &mut cp.sort_output);
            cp.sort_output.clear();
            if cp.sort_width >= len {
                reset_candidate_sort(cp);
                cp.phase = Phase::DeduplicateCandidates;
                cp.candidate_index = 0;
            } else {
                cp.sort_width = cp.sort_width.saturating_mul(2);
                cp.sort_run_start = 0;
                initialize_candidate_merge_run(cp);
            }
        } else {
            initialize_candidate_merge_run(cp);
        }
    }

    fn deduplicate_candidates(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.candidate_index >= cp.candidates.len() {
            cp.candidates = std::mem::take(&mut cp.deduplicated_candidates);
            cp.candidate_index = 0;
            reset_candidate_sort(cp);
            cp.phase = Phase::BuildHostGroups;
            cp.group_scan_index = 0;
            cp.building_group = None;
            return;
        }
        let candidate = cp.candidates[cp.candidate_index];
        cp.candidate_index += 1;
        let duplicate = cp.deduplicated_candidates.last().is_some_and(|old| {
            old.host_id == candidate.host_id
                && (old.host_t - candidate.host_t).abs() < 1.0e-5
                && (old.end_x - candidate.end_x).abs() < 1.0e-4
                && (old.end_z - candidate.end_z).abs() < 1.0e-4
        });
        if !duplicate {
            cp.deduplicated_candidates.push(candidate);
        }
    }

    fn build_host_groups(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.group_scan_index >= cp.candidates.len() {
            if let Some(group) = cp.building_group.take() {
                cp.host_groups.push(group);
            }
            cp.round_index = 0;
            cp.round_group_index = 0;
            cp.round_added = false;
            cp.phase = Phase::RoundRobin;
            return;
        }
        let index = cp.group_scan_index;
        let candidate = cp.candidates[index];
        cp.group_scan_index += 1;
        match cp.building_group {
            Some((host_id, start, _)) if host_id == candidate.host_id => {
                cp.building_group = Some((host_id, start, cp.group_scan_index));
            }
            Some(group) => {
                cp.host_groups.push(group);
                cp.building_group = Some((candidate.host_id, index, index + 1));
            }
            None => cp.building_group = Some((candidate.host_id, index, index + 1)),
        }
    }

    fn round_robin(&mut self) {
        let cp = &mut self.checkpoint;
        if cp.round_group_index >= cp.host_groups.len() {
            if cp.round_added {
                cp.round_index += 1;
                cp.round_group_index = 0;
                cp.round_added = false;
            } else {
                cp.phase = Phase::Done;
            }
            return;
        }
        let (_, start, end) = cp.host_groups[cp.round_group_index];
        let candidate_index = start.saturating_add(cp.round_index);
        if candidate_index < end {
            if cp.descriptors.len() < MAX_PREPARED_DESCRIPTORS {
                cp.descriptors
                    .push(cp.candidates[candidate_index].candidate(cp.descriptors.len()));
            }
            cp.round_added = true;
        }
        cp.round_group_index += 1;
    }

    fn begin_ray(&mut self, kind: RayQueryKind, purpose: RayPurpose) {
        match RayQuery::with_polygon(self.polygon.clone(), kind) {
            Ok(query) => {
                self.checkpoint.ray = Some(query.checkpoint());
                self.checkpoint.ray_purpose = Some(purpose);
                self.ray_query = Some(query);
            }
            Err(_) => {
                // Input validation and endpoint bounds make this unreachable
                // for accepted jobs. Fail closed if invariants ever drift.
                self.checkpoint.phase = Phase::Done;
                self.checkpoint.descriptors.clear();
            }
        }
    }

    fn endpoint_result(&mut self, inside: bool, length: f32, end_x: f32, end_z: f32) {
        if inside {
            let sample = self.current_sample();
            self.checkpoint.candidates.push(CandidateRecord {
                host_id: sample.road_id,
                host_t: sample.host_t,
                end_x,
                end_z,
                length,
            });
        }
        self.checkpoint.length_index += 1;
        self.checkpoint.phase = Phase::StartEndpoint;
    }

    fn consume_ray_result(&mut self) {
        let result = self.ray_query.take().and_then(|query| query.result());
        self.checkpoint.ray = None;
        let purpose = self
            .checkpoint
            .ray_purpose
            .take()
            .expect("ray result purpose");
        let Some(result) = result else {
            self.checkpoint.phase = Phase::Done;
            self.checkpoint.descriptors.clear();
            return;
        };
        if let RayPurpose::Endpoint {
            length,
            end_x,
            end_z,
        } = purpose
        {
            if let RayResult::SegmentInside(inside) = result {
                self.endpoint_result(inside, length, end_x, end_z);
            } else {
                self.checkpoint.phase = Phase::Done;
            }
            return;
        }
        let RayResult::ExitDistance(distance) = result else {
            self.checkpoint.phase = Phase::Done;
            return;
        };
        match purpose {
            RayPurpose::InitialInward => {
                if distance >= access_candidates::MIN_BRANCH_M {
                    let sample = self.current_sample();
                    self.checkpoint.headings.push(Heading {
                        x: sample.inward_x,
                        z: sample.inward_z,
                        target_distance: None,
                    });
                }
                self.checkpoint.phase = Phase::Targets;
            }
            RayPurpose::TargetHeading { target_along } => {
                if distance - access_candidates::FACE_EDGE_MARGIN_M
                    >= access_candidates::MIN_BRANCH_M
                    && target_along >= access_candidates::FRONTAGE_TARGET_MARGIN_M
                {
                    let mut heading = self.checkpoint.pending_heading.take().unwrap();
                    heading.target_distance =
                        Some(target_along + access_candidates::FRONTAGE_TARGET_MARGIN_M);
                    self.checkpoint.headings.push(heading);
                } else {
                    self.checkpoint.pending_heading = None;
                }
                self.checkpoint.phase = Phase::Targets;
            }
            RayPurpose::BlockerTangent => {
                if distance >= access_candidates::MIN_BRANCH_M {
                    self.checkpoint
                        .headings
                        .push(self.checkpoint.pending_heading.take().unwrap());
                } else {
                    self.checkpoint.pending_heading = None;
                }
                self.checkpoint.phase = Phase::Tangents;
            }
            RayPurpose::FallbackInward => {
                if distance >= access_candidates::MIN_BRANCH_M {
                    self.checkpoint
                        .headings
                        .push(self.checkpoint.pending_heading.take().unwrap());
                } else {
                    self.checkpoint.pending_heading = None;
                }
                self.checkpoint.phase = Phase::StartHeading;
            }
            RayPurpose::HeadingLength => {
                let limit = (distance - access_candidates::FACE_EDGE_MARGIN_M)
                    .min(access_candidates::MAX_BRANCH_M);
                self.checkpoint.ray_limit = limit;
                if limit < access_candidates::MIN_BRANCH_M {
                    self.checkpoint.heading_index += 1;
                    self.checkpoint.current_heading = None;
                    self.checkpoint.phase = Phase::StartHeading;
                } else {
                    self.checkpoint.phase = Phase::BuildLengths;
                }
            }
            RayPurpose::Endpoint { .. } => unreachable!(),
        }
    }
}

fn empty_checkpoint(
    face: FrozenFace,
    hosts: Vec<BoundaryHost>,
    nodes: Vec<CandidateNode>,
    parcels: Vec<OrientedRect>,
    targets: Vec<(f32, f32)>,
    phase: Phase,
) -> DescriptorCheckpoint {
    DescriptorCheckpoint {
        schema: SCHEMA,
        face,
        hosts,
        nodes,
        parcels,
        targets,
        phase,
        init_index: 0,
        init_compare: 0,
        init_swap_pending: false,
        init_dedup_index: 0,
        normalized_parcels: Vec::new(),
        normalized_targets: Vec::new(),
        edge_order: Vec::new(),
        edge_order_index: 0,
        projected_hosts: Vec::new(),
        edge_host_index: 0,
        host_search_low: 0,
        host_search_high: 0,
        host_found: None,
        host_prep_nodes: Vec::new(),
        host_prep_node_index: 0,
        host_prep_parcels: Vec::new(),
        host_prep_parcel_index: 0,
        host_preparation: None,
        split_samples: Vec::new(),
        split_index: 0,
        host_sample_copy_index: 0,
        ray: None,
        ray_purpose: None,
        headings: Vec::new(),
        target_index: 0,
        target_variant: 0,
        pending_heading: None,
        pending_target_along: 0.0,
        duplicate_after: None,
        duplicate_index: 0,
        blocker_parcel_index: 0,
        blocker_target_index: 0,
        blocker_min_distance: None,
        blockers: Vec::new(),
        pending_blocker: None,
        blocker_insert_index: 0,
        tangent_blocker_index: 0,
        tangent_corner_index: 0,
        tangent_side_index: 0,
        heading_index: 0,
        current_heading: None,
        ray_limit: 0.0,
        lengths: Vec::new(),
        length_sort_index: 0,
        length_compare_index: 0,
        deduplicated_lengths: Vec::new(),
        length_index: 0,
        candidates: Vec::new(),
        sort_width: 1,
        sort_run_start: 0,
        sort_left: 0,
        sort_mid: 0,
        sort_right: 0,
        sort_end: 0,
        sort_output: Vec::new(),
        candidate_index: 0,
        deduplicated_candidates: Vec::new(),
        host_groups: Vec::new(),
        building_group: None,
        group_scan_index: 0,
        round_index: 0,
        round_group_index: 0,
        round_added: false,
        descriptors: Vec::new(),
    }
}

fn reserve_initialization_storage(cp: &mut DescriptorCheckpoint) {
    cp.normalized_parcels.reserve_exact(cp.parcels.len());
    cp.normalized_targets.reserve_exact(cp.targets.len());
    cp.edge_order.reserve_exact(cp.face.boundary_edge_ids.len());
    cp.projected_hosts
        .reserve_exact(access_search::MAX_PREPARED_HOSTS);
    cp.host_prep_nodes.reserve_exact(cp.nodes.len());
    cp.host_prep_parcels.reserve_exact(cp.parcels.len());
}

fn initialize_candidate_merge_run(cp: &mut DescriptorCheckpoint) {
    let len = cp.candidates.len();
    cp.sort_left = cp.sort_run_start;
    cp.sort_mid = cp.sort_run_start.saturating_add(cp.sort_width).min(len);
    cp.sort_right = cp.sort_mid;
    cp.sort_end = cp
        .sort_run_start
        .saturating_add(cp.sort_width.saturating_mul(2))
        .min(len);
}

fn reset_candidate_sort(cp: &mut DescriptorCheckpoint) {
    cp.sort_width = 1;
    cp.sort_run_start = 0;
    cp.sort_left = 0;
    cp.sort_mid = 0;
    cp.sort_right = 0;
    cp.sort_end = 0;
    cp.sort_output.clear();
}

fn validate_inputs(
    face: &RoadFace,
    hosts: &[BoundaryHost],
    nodes: &[CandidateNode],
    parcels: &[OrientedRect],
    targets: &[(f32, f32)],
) -> Result<(), SearchError> {
    validate_inputs_frozen(&FrozenFace::from(face), hosts, nodes, parcels, targets)
}

fn polygon_area(points: &[(f32, f32)]) -> f64 {
    let mut area = 0.0f64;
    for i in 0..points.len() {
        let (x0, z0) = points[i];
        let (x1, z1) = points[(i + 1) % points.len()];
        area += x0 as f64 * z1 as f64 - x1 as f64 * z0 as f64;
    }
    area * 0.5
}

fn validate_checkpoint(cp: &DescriptorCheckpoint) -> Result<(), SearchError> {
    if cp.schema != SCHEMA {
        return Err(SearchError::InvalidCheckpoint);
    }
    validate_inputs_frozen(&cp.face, &cp.hosts, &cp.nodes, &cp.parcels, &cp.targets)?;
    if cp.phase.is_initialization() {
        return validate_initialization_checkpoint(cp);
    }
    if cp
        .hosts
        .windows(2)
        .any(|pair| pair[0].road_id >= pair[1].road_id)
        || cp.nodes.windows(2).any(|pair| pair[0].id >= pair[1].id)
        || cp
            .parcels
            .windows(2)
            .any(|pair| rect_order(&pair[0], &pair[1]) != Ordering::Less)
        || cp.targets.windows(2).any(|pair| {
            pair[0]
                .0
                .total_cmp(&pair[1].0)
                .then_with(|| pair[0].1.total_cmp(&pair[1].1))
                != Ordering::Less
        })
    {
        return Err(SearchError::InvalidCheckpoint);
    }
    let host_count = cp.hosts.len().min(MAX_HOSTS_INPUT);
    let max_splits = host_count.saturating_mul(access_candidates::MAX_SPLITS_PER_HOST);
    if cp.split_samples.len() > max_splits
        || cp.split_index > cp.split_samples.len()
        || cp.host_sample_copy_index > max_splits
        || cp.headings.len() > MAX_HEADING_COUNT
        || cp.blockers.len() > MAX_NEAREST_BLOCKERS
        || cp.blocker_parcel_index > cp.parcels.len()
        || cp.blocker_target_index > cp.targets.len()
        || cp.target_index > cp.targets.len()
        || cp.target_variant > 3
        || cp.duplicate_index > cp.headings.len()
        || cp.tangent_blocker_index > cp.blockers.len()
        || cp.tangent_corner_index > MAX_RECT_CORNERS
        || cp.tangent_side_index > 1
        || cp.heading_index > cp.headings.len()
        || cp.lengths.len() > 4
        || cp.deduplicated_lengths.len() > 4
        || (cp.phase == Phase::DeduplicateLengths && cp.length_index > cp.lengths.len())
        || (cp.phase != Phase::DeduplicateLengths
            && cp.length_index > cp.deduplicated_lengths.len())
        || (cp.phase == Phase::SortLengths
            && (cp.length_sort_index > cp.lengths.len()
                || cp.length_compare_index > cp.length_sort_index
                || cp.length_compare_index > cp.lengths.len()
                || (cp.length_sort_index < cp.lengths.len()
                    && cp.length_compare_index >= cp.lengths.len())))
        || cp.candidates.len() > access_candidates::MAX_PREPARED_DESCRIPTORS
        || cp.sort_output.len() > cp.candidates.len()
        || cp.sort_run_start > cp.candidates.len()
        || cp.sort_left > cp.sort_mid
        || cp.sort_mid > cp.sort_right
        || cp.sort_right > cp.sort_end
        || cp.sort_end > cp.candidates.len()
        || cp.candidate_index > cp.candidates.len()
        || cp.deduplicated_candidates.len() > cp.candidates.len()
        || cp.host_groups.len() > MAX_HOSTS_INPUT
        || cp.descriptors.len() > access_candidates::MAX_PREPARED_DESCRIPTORS
        || cp.round_group_index > cp.host_groups.len()
    {
        return Err(SearchError::InvalidCheckpoint);
    }
    if cp.split_samples.iter().any(|s| {
        !s.along_m.is_finite()
            || !s.host_t.is_finite()
            || !(0.0..1.0).contains(&s.host_t)
            || !s.inward_x.is_finite()
            || !s.inward_z.is_finite()
            || (s.inward_x.hypot(s.inward_z) - 1.0).abs() > 1.0e-3
            || !cp.hosts.iter().any(|h| h.road_id == s.road_id)
    }) || cp.headings.iter().any(|h| {
        !h.x.is_finite()
            || !h.z.is_finite()
            || (h.x.hypot(h.z) - 1.0).abs() > 1.0e-3
            || h.target_distance.is_some_and(|d| !d.is_finite() || d < 0.0)
    }) || cp
        .blockers
        .iter()
        .any(|b| !b.nearest_distance.is_finite() || !b.rect.finite_positive())
        || cp
            .blocker_min_distance
            .is_some_and(|d| !d.is_finite() || d < 0.0)
        || cp
            .pending_blocker
            .is_some_and(|b| !b.nearest_distance.is_finite() || !b.rect.finite_positive())
        || cp.pending_heading.is_some_and(|h| {
            !h.x.is_finite()
                || !h.z.is_finite()
                || h.target_distance.is_some_and(|d| !d.is_finite())
        })
        || !cp.pending_target_along.is_finite()
        || !cp.ray_limit.is_finite()
        || cp
            .lengths
            .iter()
            .chain(cp.deduplicated_lengths.iter())
            .any(|v| !v.is_finite() || *v < 0.0)
        || cp
            .candidates
            .iter()
            .chain(cp.deduplicated_candidates.iter())
            .any(|c| !candidate_finite(*c))
        || cp
            .descriptors
            .iter()
            .any(|d| !d.host_t.is_finite() || !d.end_x.is_finite() || !d.end_z.is_finite())
    {
        return Err(SearchError::InvalidScratch);
    }

    let host_done = cp.host_preparation.is_none();
    if host_done && cp.split_samples.is_empty() && cp.phase != Phase::Done {
        return Err(SearchError::InvalidCursor);
    }
    if !host_done && !matches!(cp.phase, Phase::HostIntervals | Phase::CollectHostSplits) {
        return Err(SearchError::InvalidCursor);
    }
    if cp.phase == Phase::HostIntervals
        && (cp.host_preparation.is_none() || !cp.split_samples.is_empty())
    {
        return Err(SearchError::InvalidCursor);
    }
    if cp.phase == Phase::CollectHostSplits && cp.host_preparation.is_none() {
        return Err(SearchError::InvalidCursor);
    }
    if cp.phase != Phase::HostIntervals
        && cp.phase != Phase::CollectHostSplits
        && cp.split_index < cp.split_samples.len()
    {
        // All later phases refer to the current split sample.
    } else if !matches!(
        cp.phase,
        Phase::HostIntervals
            | Phase::CollectHostSplits
            | Phase::StartSplit
            | Phase::SortCandidates
            | Phase::DeduplicateCandidates
            | Phase::BuildHostGroups
            | Phase::RoundRobin
            | Phase::Done
    ) {
        return Err(SearchError::InvalidCursor);
    }
    if cp.ray.is_some() != cp.ray_purpose.is_some() {
        return Err(SearchError::InvalidCursor);
    }
    if cp.ray.is_some()
        && !matches!(
            cp.phase,
            Phase::InitialInwardRay
                | Phase::TargetHeadingRay
                | Phase::TangentRay
                | Phase::FallbackRay
                | Phase::HeadingRay
                | Phase::EndpointSegment
        )
    {
        return Err(SearchError::InvalidCursor);
    }
    let needs_heading = matches!(
        cp.phase,
        Phase::DuplicateCheck | Phase::TargetHeadingRay | Phase::TangentRay | Phase::FallbackRay
    );
    let needs_current_heading = matches!(
        cp.phase,
        Phase::HeadingRay
            | Phase::BuildLengths
            | Phase::SortLengths
            | Phase::DeduplicateLengths
            | Phase::StartEndpoint
            | Phase::EndpointSegment
    );
    if (needs_heading && cp.pending_heading.is_none())
        || (cp.phase == Phase::DuplicateCheck && cp.duplicate_after.is_none())
        || (!matches!(cp.phase, Phase::DuplicateCheck) && cp.duplicate_after.is_some())
        || (cp.phase == Phase::InsertBlocker && cp.pending_blocker.is_none())
        || (cp.phase != Phase::InsertBlocker && cp.pending_blocker.is_some())
        || (needs_current_heading && cp.current_heading.is_none())
        || (!needs_current_heading && cp.current_heading.is_some())
    {
        return Err(SearchError::InvalidCursor);
    }
    if let Some(purpose) = cp.ray_purpose {
        let matches_phase = matches!(
            (cp.phase, purpose),
            (Phase::InitialInwardRay, RayPurpose::InitialInward)
                | (Phase::TargetHeadingRay, RayPurpose::TargetHeading { .. })
                | (Phase::TangentRay, RayPurpose::BlockerTangent)
                | (Phase::FallbackRay, RayPurpose::FallbackInward)
                | (Phase::HeadingRay, RayPurpose::HeadingLength)
                | (Phase::EndpointSegment, RayPurpose::Endpoint { .. })
        );
        if !matches_phase {
            return Err(SearchError::InvalidCursor);
        }
    }
    if cp.phase == Phase::Done && cp.descriptors.len() > access_candidates::MAX_PREPARED_DESCRIPTORS
    {
        return Err(SearchError::InvalidCursor);
    }
    if !matches!(cp.phase, Phase::RoundRobin | Phase::Done) && !cp.descriptors.is_empty() {
        return Err(SearchError::InvalidCursor);
    }
    Ok(())
}

fn validate_initialization_checkpoint(cp: &DescriptorCheckpoint) -> Result<(), SearchError> {
    if cp.host_preparation.is_some()
        || !cp.split_samples.is_empty()
        || cp.split_index != 0
        || cp.ray.is_some()
        || cp.ray_purpose.is_some()
        || !cp.descriptors.is_empty()
        || cp.normalized_parcels.len() > cp.parcels.len()
        || cp.normalized_targets.len() > cp.targets.len()
        || cp.edge_order.len() > cp.face.boundary_edge_ids.len()
        || cp.projected_hosts.len() > access_search::MAX_PREPARED_HOSTS
        || cp.host_prep_nodes.len() > cp.nodes.len()
        || cp.host_prep_parcels.len() > cp.parcels.len()
        || cp.edge_host_index > cp.edge_order.len().min(access_search::MAX_PREPARED_HOSTS)
        || cp.host_search_low > cp.hosts.len()
        || cp.host_search_high > cp.hosts.len()
        || cp.host_search_low > cp.host_search_high
        || cp.host_found.is_some_and(|i| i >= cp.hosts.len())
    {
        return Err(SearchError::InvalidCheckpoint);
    }
    let sorting_len = match cp.phase {
        Phase::InitSortHosts => cp.hosts.len(),
        Phase::InitSortNodes => cp.nodes.len(),
        Phase::InitSortParcels => cp.parcels.len(),
        Phase::InitSortTargets => cp.targets.len(),
        _ => 0,
    };
    if sorting_len > 0
        && (cp.init_index > sorting_len
            || cp.init_compare > cp.init_index
            || (cp.init_index > 0 && cp.init_compare > sorting_len))
    {
        return Err(SearchError::InvalidCursor);
    }
    if cp.init_swap_pending
        && (!matches!(
            cp.phase,
            Phase::InitSortHosts
                | Phase::InitSortNodes
                | Phase::InitSortParcels
                | Phase::InitSortTargets
                | Phase::InitSortEdgeOrder
        ) || cp.init_compare == 0
            || cp.init_index >= sorting_len.max(cp.edge_order.len()))
    {
        return Err(SearchError::InvalidCursor);
    }
    if (cp.phase == Phase::InitDeduplicateParcels && cp.init_dedup_index > cp.parcels.len())
        || (cp.phase == Phase::InitDeduplicateTargets && cp.init_dedup_index > cp.targets.len())
        || cp.edge_order_index > cp.edge_order.len()
        || cp.init_index
            > cp.face
                .boundary_edge_ids
                .len()
                .max(cp.hosts.len())
                .max(cp.nodes.len())
                .max(cp.parcels.len())
                .max(cp.targets.len())
        || cp.host_prep_node_index > cp.nodes.len()
        || cp.host_prep_parcel_index > cp.parcels.len()
        || cp
            .edge_order
            .iter()
            .any(|&i| i >= cp.face.boundary_edge_ids.len())
        || cp
            .edge_order
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != cp.edge_order.len()
        || cp
            .projected_hosts
            .iter()
            .any(|h| !cp.hosts.iter().any(|host| host.road_id == h.road.road_id))
    {
        return Err(SearchError::InvalidCursor);
    }
    if cp.phase == Phase::InitBuildEdgeOrder && cp.init_index > cp.face.boundary_edge_ids.len()
        || cp.phase == Phase::InitSortEdgeOrder
            && (cp.edge_order_index > cp.edge_order.len() || cp.init_compare > cp.edge_order_index)
    {
        return Err(SearchError::InvalidCursor);
    }
    if cp.phase == Phase::InitSortEdgeOrder {
        let prefix = cp.edge_order_index.min(cp.edge_order.len());
        if cp.edge_order[..prefix]
            .windows(2)
            .any(|pair| cp.face.boundary_edge_ids[pair[0]] > cp.face.boundary_edge_ids[pair[1]])
        {
            return Err(SearchError::InvalidCheckpoint);
        }
    }
    if !cp.normalized_parcels.iter().all(|r| r.finite_positive())
        || !cp.host_prep_parcels.iter().all(|r| r.finite_positive())
        || cp
            .normalized_targets
            .iter()
            .any(|(x, z)| !x.is_finite() || !z.is_finite())
        || cp.projected_hosts.iter().any(|h| {
            !h.inward_x.is_finite()
                || !h.inward_z.is_finite()
                || (h.inward_x.hypot(h.inward_z) - 1.0).abs() > 1.0e-3
        })
    {
        return Err(SearchError::InvalidScratch);
    }
    Ok(())
}

fn validate_inputs_frozen(
    face: &FrozenFace,
    hosts: &[BoundaryHost],
    nodes: &[CandidateNode],
    parcels: &[OrientedRect],
    targets: &[(f32, f32)],
) -> Result<(), SearchError> {
    if face.vertex_ids.len() < 3
        || face.vertex_ids.len() > access_candidates::MAX_FACE_VERTICES_FOR_PREPARED_SEARCH
        || face.vertex_ids.len() != face.polygon.len()
        || face.vertex_ids.len() != face.boundary_edge_ids.len()
        || !face.area.is_finite()
        || face.area <= 0.0
        || hosts.len() > MAX_HOSTS_INPUT
        || face.boundary_edge_ids.len() > MAX_FACE_EDGES_INPUT
        || nodes.len() > access_candidates::MAX_PREPARED_INPUT_NODES
        || parcels.len() > access_candidates::MAX_PREPARED_INPUT_PARCELS
        || targets.len() > access_candidates::MAX_TARGETS_PER_SPLIT
    {
        return Err(SearchError::InputCap);
    }
    // RayQuery permits coordinates within one million metres and has a 62m
    // lookahead. Leave a fixed margin so every generated endpoint stays safe.
    let coord_ok = |v: f32| v.is_finite() && v.abs() <= 999_900.0;
    if face
        .polygon
        .iter()
        .any(|(x, z)| !coord_ok(*x) || !coord_ok(*z))
        || hosts
            .iter()
            .any(|h| ![h.ax, h.az, h.bx, h.bz].iter().all(|v| coord_ok(*v)))
        || nodes.iter().any(|n| !coord_ok(n.x) || !coord_ok(n.z))
        || parcels.iter().any(|p| {
            !p.finite_positive()
                || !coord_ok(p.x)
                || !coord_ok(p.z)
                || p.width > 100_000.0
                || p.depth > 100_000.0
                || !p.rotation.is_finite()
        })
        || targets.iter().any(|(x, z)| !coord_ok(*x) || !coord_ok(*z))
        || !face.area.is_finite()
        || polygon_area(&face.polygon) <= 0.0
    {
        return Err(SearchError::InvalidInput);
    }
    let mut host_ids = hosts.iter().map(|h| h.road_id).collect::<Vec<_>>();
    host_ids.sort_unstable();
    if host_ids.windows(2).any(|p| p[0] == p[1]) {
        return Err(SearchError::DuplicateId);
    }
    let mut node_ids = nodes.iter().map(|n| n.id).collect::<Vec<_>>();
    node_ids.sort_unstable();
    if node_ids.windows(2).any(|p| p[0] == p[1]) {
        return Err(SearchError::DuplicateId);
    }
    let mut face_ids = face.vertex_ids.clone();
    face_ids.sort_unstable();
    if face_ids.windows(2).any(|p| p[0] == p[1]) {
        return Err(SearchError::DuplicateId);
    }
    Ok(())
}

fn candidate_finite(c: CandidateRecord) -> bool {
    c.host_t.is_finite()
        && (0.0..1.0).contains(&c.host_t)
        && c.end_x.is_finite()
        && c.end_z.is_finite()
        && c.length.is_finite()
        && c.length >= 0.0
}

fn rect_order(a: &OrientedRect, b: &OrientedRect) -> Ordering {
    a.x.total_cmp(&b.x)
        .then_with(|| a.z.total_cmp(&b.z))
        .then_with(|| a.width.total_cmp(&b.width))
        .then_with(|| a.depth.total_cmp(&b.depth))
        .then_with(|| a.rotation.total_cmp(&b.rotation))
}

fn candidate_order(a: &CandidateRecord, b: &CandidateRecord) -> Ordering {
    a.host_id
        .cmp(&b.host_id)
        .then_with(|| a.host_t.total_cmp(&b.host_t))
        .then_with(|| a.length.total_cmp(&b.length))
        .then_with(|| a.end_x.total_cmp(&b.end_x))
        .then_with(|| a.end_z.total_cmp(&b.end_z))
}

fn blocker_order(a: &Blocker, b: &Blocker) -> Ordering {
    a.nearest_distance
        .total_cmp(&b.nearest_distance)
        .then_with(|| rect_order(&a.rect, &b.rect))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (RoadFace, Vec<BoundaryHost>, Vec<CandidateNode>) {
        let face = RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        };
        let hosts = vec![BoundaryHost {
            road_id: 10,
            from: 1,
            to: 2,
            ax: 0.0,
            az: 0.0,
            bx: 100.0,
            bz: 0.0,
        }];
        let nodes = vec![
            CandidateNode {
                id: 1,
                x: 0.0,
                z: 0.0,
            },
            CandidateNode {
                id: 2,
                x: 100.0,
                z: 0.0,
            },
        ];
        (face, hosts, nodes)
    }

    #[test]
    fn small_search_makes_bounded_progress_and_resumes_from_json() {
        let (face, hosts, nodes) = fixture();
        let mut search = DescriptorSearch::new(&face, &hosts, &nodes, &[], &[]).unwrap();
        assert_eq!(
            search.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        assert!(search.descriptors().is_none());
        let mut total = 0;
        let mut calls = 0;
        while search.descriptors().is_none() {
            let result = search.advance(7);
            assert!(result.work_used <= 7);
            assert!(result.work_used > 0 || result.done);
            total += result.work_used;
            calls += 1;
            assert!(calls < 100_000);
            if calls % 5 == 0 {
                let bytes = serde_json::to_vec(&search.checkpoint()).unwrap();
                search =
                    DescriptorSearch::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
            }
        }
        let descriptors = search.descriptors().unwrap();
        assert!(!descriptors.is_empty());
        assert!(total > 0);
        assert_eq!(search.advance(100).work_used, 0);
    }

    #[test]
    fn owned_initialization_matches_borrowed_output_and_resumes_every_phase() {
        let (face, mut hosts, mut nodes) = fixture();
        hosts.reverse();
        nodes.reverse();
        let parcels = vec![
            OrientedRect {
                x: 61.0,
                z: 44.0,
                width: 8.0,
                depth: 11.0,
                rotation: 0.3,
            },
            OrientedRect {
                x: 31.0,
                z: 52.0,
                width: 9.0,
                depth: 10.0,
                rotation: -0.2,
            },
            OrientedRect {
                x: 61.0,
                z: 44.0,
                width: 8.0,
                depth: 11.0,
                rotation: 0.3,
            },
        ];
        let targets = vec![(70.0, 75.0), (24.0, 66.0), (70.0, 75.0)];
        let input_items = hosts.len() + nodes.len() + parcels.len();
        let mut borrowed =
            DescriptorSearch::new(&face, &hosts, &nodes, &parcels, &targets).unwrap();
        let mut owned =
            DescriptorSearch::from_owned(face.clone(), hosts, nodes, parcels, targets).unwrap();
        let mut steps = 0usize;
        while owned.descriptors().is_none() {
            let result = owned.advance(1);
            assert_eq!(result.work_used, 1);
            let bytes = serde_json::to_vec(&owned.checkpoint()).unwrap();
            owned = DescriptorSearch::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
            steps += 1;
            assert!(
                steps < 200_000,
                "owned initialization/search did not finish"
            );
        }
        while borrowed.descriptors().is_none() {
            let result = borrowed.advance(48);
            assert!(result.work_used <= 48);
            assert!(result.work_used > 0 || result.done);
        }
        assert!(steps > input_items);
        assert_eq!(owned.descriptors(), borrowed.descriptors());
    }

    #[test]
    fn owned_initializer_rejects_edge_sort_cursor_out_of_bounds() {
        let (face, hosts, nodes) = fixture();
        let mut search = DescriptorSearch::from_owned(face, hosts, nodes, vec![], vec![]).unwrap();
        let mut steps = 0;
        while search.checkpoint.phase != Phase::InitSortEdgeOrder {
            let step = search.advance(1);
            assert_eq!(step.work_used, 1);
            steps += 1;
            assert!(steps < 1000);
        }
        let mut valid = search.checkpoint();
        assert!(DescriptorSearch::restore(valid.clone()).is_ok());
        valid.init_compare = usize::MAX;
        assert_eq!(
            DescriptorSearch::restore(valid).err(),
            Some(SearchError::InvalidCursor)
        );
    }

    #[test]
    #[ignore = "mature golden parity through the owned incremental initializer"]
    fn owned_initialization_matches_frozen_mature_descriptors() {
        let reference = crate::access_reference::load();
        let mut search = DescriptorSearch::from_owned(
            reference.face.clone(),
            reference.hosts.clone(),
            reference.nodes.clone(),
            reference.parcels.clone(),
            reference.targets.clone(),
        )
        .unwrap();
        let mut work = 0usize;
        let mut next_save = 16_381usize;
        while search.descriptors().is_none() {
            let result = search.advance(48);
            assert!(result.work_used <= 48);
            assert!(result.work_used > 0 || result.done);
            work += result.work_used;
            assert!(work < 20_000_000);
            if work >= next_save && !result.done {
                let bytes = serde_json::to_vec(&search.checkpoint()).unwrap();
                search =
                    DescriptorSearch::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
                next_save += 16_381;
            }
        }
        let bits = search
            .descriptors()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                assert_eq!(c.tie_index, i as u8);
                [
                    c.host_id,
                    c.host_t.to_bits(),
                    c.end_x.to_bits(),
                    c.end_z.to_bits(),
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(bits, reference.ordered_bits);
    }

    #[test]
    fn malformed_phase_cursor_and_nonfinite_input_are_rejected() {
        let (face, hosts, nodes) = fixture();
        let search = DescriptorSearch::new(&face, &hosts, &nodes, &[], &[]).unwrap();
        let mut cp = search.checkpoint();
        cp.host_sample_copy_index = usize::MAX;
        assert_eq!(
            DescriptorSearch::restore(cp).unwrap_err(),
            SearchError::InvalidCheckpoint
        );

        let mut bad = face.clone();
        bad.polygon[0].0 = f32::NAN;
        assert!(matches!(
            DescriptorSearch::new(&bad, &hosts, &nodes, &[], &[]),
            Err(SearchError::InvalidInput)
        ));
    }

    #[test]
    fn every_work_boundary_restores_and_matches_reference() {
        let (face, hosts, nodes) = fixture();
        let parcels = [OrientedRect {
            x: 52.0,
            z: 42.0,
            width: 12.0,
            depth: 9.0,
            rotation: 0.2,
        }];
        let targets = [(72.0, 72.0), (27.0, 64.0)];
        let expected = crate::access_candidates::PreparedAccessSearch::prepare(
            &face, &hosts, &nodes, &parcels, &targets,
        );
        let mut expected_bits = Vec::new();
        let mut cursor = 0;
        loop {
            let page = expected.page(cursor);
            expected_bits.extend(page.candidates.iter().map(|d| {
                (
                    d.host_id,
                    d.host_t.to_bits(),
                    d.end_x.to_bits(),
                    d.end_z.to_bits(),
                )
            }));
            cursor = page.next_cursor;
            if page.exhausted {
                break;
            }
        }
        let mut search = DescriptorSearch::new(&face, &hosts, &nodes, &parcels, &targets).unwrap();
        let mut steps = 0usize;
        let mut saw_blocker = false;
        let mut saw_sort = false;
        let mut seen_phases = Vec::new();
        loop {
            let result = search.advance(1);
            assert!(result.work_used <= 1);
            steps += result.work_used;
            assert!(steps < 1_000_000);
            let valid = search.checkpoint();
            let phase = valid.phase;
            if !seen_phases.contains(&phase) {
                seen_phases.push(phase);
            }
            if phase == Phase::InsertBlocker {
                saw_blocker = true;
                let mut bad = valid.clone();
                bad.pending_blocker = None;
                assert!(DescriptorSearch::restore(bad).is_err());
            }
            if phase == Phase::DuplicateCheck {
                let mut bad = valid.clone();
                bad.pending_heading = None;
                assert!(DescriptorSearch::restore(bad).is_err());
                let mut bad = valid.clone();
                bad.duplicate_after = None;
                assert!(DescriptorSearch::restore(bad).is_err());
            }
            if matches!(
                phase,
                Phase::TargetHeadingRay | Phase::TangentRay | Phase::FallbackRay
            ) {
                let mut bad = valid.clone();
                bad.pending_heading = None;
                assert!(DescriptorSearch::restore(bad).is_err());
            }
            if matches!(
                phase,
                Phase::HeadingRay
                    | Phase::BuildLengths
                    | Phase::SortLengths
                    | Phase::DeduplicateLengths
                    | Phase::StartEndpoint
                    | Phase::EndpointSegment
            ) {
                let mut bad = valid.clone();
                bad.current_heading = None;
                assert!(DescriptorSearch::restore(bad).is_err());
            }
            if phase == Phase::SortLengths && valid.length_sort_index < valid.lengths.len() {
                saw_sort = true;
                let mut bad = valid.clone();
                bad.length_compare_index = bad.lengths.len();
                assert!(DescriptorSearch::restore(bad).is_err());
            }
            if phase == Phase::SortCandidates {
                saw_sort = true;
                let mut bad = valid.clone();
                bad.sort_left = usize::MAX;
                assert_eq!(
                    DescriptorSearch::restore(bad).unwrap_err(),
                    SearchError::InvalidCheckpoint
                );
            }
            let bytes = serde_json::to_vec(&search.checkpoint()).unwrap();
            search = DescriptorSearch::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
            if result.done {
                break;
            }
        }
        assert!(saw_blocker);
        assert!(saw_sort);
        for required in [
            Phase::DuplicateCheck,
            Phase::TargetHeadingRay,
            Phase::BlockerScan,
            Phase::InsertBlocker,
            Phase::Tangents,
            Phase::TangentRay,
            Phase::HeadingRay,
            Phase::SortLengths,
            Phase::EndpointSegment,
        ] {
            assert!(
                seen_phases.contains(&required),
                "fixture must cover {required:?}"
            );
        }
        let actual = search
            .descriptors()
            .unwrap()
            .iter()
            .map(|d| {
                (
                    d.host_id,
                    d.host_t.to_bits(),
                    d.end_x.to_bits(),
                    d.end_z.to_bits(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected_bits);
    }
}
