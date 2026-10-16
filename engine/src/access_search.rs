//! First resumable stage for face-aware access search: project obstacles onto
//! each possible host road and produce the same bounded split samples as the
//! one-shot candidate builder. Heading and endpoint preparation remain later
//! stages; this type deliberately does not claim to produce access proposals.

use crate::access_candidates::{self, BoundaryHost, CandidateNode};
use crate::geometry::OrientedRect;
use crate::road_faces::RoadFace;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const CHECKPOINT_SCHEMA: u8 = 4;
const MAX_HOST_INPUTS: usize = 512;
pub(crate) const MAX_PREPARED_HOSTS: usize = 24;
const MAX_NODES: usize = access_candidates::MAX_PREPARED_INPUT_NODES;
const MAX_PARCELS: usize = access_candidates::MAX_PREPARED_INPUT_PARCELS;
#[cfg(feature = "legacy")]
const NODE_CLEARANCE_M: f32 = 1.0;
#[cfg(feature = "legacy")]
const HOST_END_CLEARANCE_M: f32 = 10.0;
#[cfg(feature = "legacy")]
const PARCEL_HOST_CLEARANCE_M: f32 = 5.0;
#[cfg(feature = "legacy")]
const PARCEL_PROJECTION_MARGIN_M: f32 = 2.0;
const MAX_SPLITS_PER_HOST: usize = access_candidates::MAX_SPLITS_PER_HOST;
const MAX_INTERVAL_SPLITS: usize = access_candidates::MAX_INTERVAL_SPLITS;
const MAX_EDGE_SPLITS: usize = access_candidates::MAX_EDGE_SPLITS;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Interval {
    start: f32,
    end: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct HostSplitSample {
    pub road_id: u32,
    pub along_m: f32,
    pub host_t: f32,
    pub inward_x: f32,
    pub inward_z: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct ProjectionHost {
    pub(crate) road: BoundaryHost,
    pub(crate) inward_x: f32,
    pub(crate) inward_z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchError {
    InvalidFace,
    InputCap,
    NonFiniteInput,
    DuplicateId,
    NonCanonicalCheckpoint,
    InvalidCursor,
    InvalidScratch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Nodes,
    ParcelCorners,
    SortIntervals,
    SubtractIntervals,
    FilterSpans,
    SelectSplits,
    SelectEdges,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    schema: u8,
    hosts: Vec<ProjectionHost>,
    nodes: Vec<CandidateNode>,
    parcels: Vec<OrientedRect>,
    host_index: usize,
    phase: Phase,
    node_index: usize,
    parcel_index: usize,
    parcel_corner_index: usize,
    parcel_bounds: [f32; 4],
    forbidden: Vec<Interval>,
    sort_width: usize,
    sort_run_start: usize,
    sort_left: usize,
    sort_mid: usize,
    sort_right: usize,
    sort_end: usize,
    sort_output: Vec<Interval>,
    subtract_index: usize,
    span_index: usize,
    free_spans: Vec<Interval>,
    next_spans: Vec<Interval>,
    filter_index: usize,
    filtered_spans: Vec<Interval>,
    select_index: usize,
    select_compare: usize,
    selected_spans: Vec<Interval>,
    edge_span_cursor: usize,
    edge_candidate_cursor: usize,
    edge_compare_cursor: usize,
    edge_samples: Vec<f32>,
    samples: Vec<HostSplitSample>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

/// Incremental host-interval preparation. Construction validates and
/// canonicalizes bounded inputs synchronously; `advance` charges every node,
/// rectangle-corner, interval sort/merge, subtraction, and top-four selection
/// operation. Its completed output is split samples only, not descriptors.
#[derive(Clone, Debug)]
pub(crate) struct AccessHostPreparation {
    checkpoint: Checkpoint,
}

impl AccessHostPreparation {
    #[cfg(any(test, feature = "legacy"))]
    pub(crate) fn new(
        face: &RoadFace,
        hosts: &[BoundaryHost],
        nodes: &[CandidateNode],
        parcel_footprints: &[OrientedRect],
    ) -> Result<Self, SearchError> {
        if face.vertex_ids.len() < 3
            || face.vertex_ids.len() != face.polygon.len()
            || face.vertex_ids.len() != face.boundary_edge_ids.len()
            || face.polygon.len() > access_candidates::MAX_FACE_VERTICES_FOR_PREPARED_SEARCH
            || face
                .polygon
                .iter()
                .any(|p| !p.0.is_finite() || !p.1.is_finite())
        {
            return Err(SearchError::InvalidFace);
        }
        if hosts.len() > MAX_HOST_INPUTS
            || nodes.len() > MAX_NODES
            || parcel_footprints.len() > MAX_PARCELS
        {
            return Err(SearchError::InputCap);
        }
        if nodes
            .iter()
            .any(|node| !node.x.is_finite() || !node.z.is_finite())
            || hosts.iter().any(|host| {
                !host.ax.is_finite()
                    || !host.az.is_finite()
                    || !host.bx.is_finite()
                    || !host.bz.is_finite()
            })
            || parcel_footprints
                .iter()
                .any(|parcel| !parcel.finite_positive())
        {
            return Err(SearchError::NonFiniteInput);
        }
        let mut node_ids = nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        node_ids.sort_unstable();
        if node_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SearchError::DuplicateId);
        }
        let mut road_ids = hosts.iter().map(|host| host.road_id).collect::<Vec<_>>();
        road_ids.sort_unstable();
        if road_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SearchError::DuplicateId);
        }

        // This input preparation is deliberately outside the incremental
        // quota. All source arrays are capped above; the ongoing simulation
        // capture/canonicalization stage must itself be budgeted separately.
        let host_by_id = hosts
            .iter()
            .map(|host| (host.road_id, *host))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut ordered_hosts = Vec::new();
        let signed_area = polygon_signed_area(&face.polygon);
        if !signed_area.is_finite() || signed_area <= 0.0 {
            return Err(SearchError::InvalidFace);
        }
        let mut edge_order = (0..face.boundary_edge_ids.len()).collect::<Vec<_>>();
        edge_order.sort_by_key(|index| face.boundary_edge_ids[*index]);
        for edge_index in edge_order.into_iter().take(MAX_PREPARED_HOSTS) {
            let edge_id = face.boundary_edge_ids[edge_index];
            let Some(host) = host_by_id.get(&edge_id).copied() else {
                continue;
            };
            let (ax, az) = face.polygon[edge_index];
            let (bx, bz) = face.polygon[(edge_index + 1) % face.polygon.len()];
            let edge_dx = bx - ax;
            let edge_dz = bz - az;
            let host_dx = host.bx - host.ax;
            let host_dz = host.bz - host.az;
            let edge_len = crate::access_math::length(edge_dx, edge_dz);
            let host_len = crate::access_math::length(host_dx, host_dz);
            if !edge_len.is_finite()
                || !host_len.is_finite()
                || host_len < 2.0 * HOST_END_CLEARANCE_M
                || (edge_dx * host_dx + edge_dz * host_dz).abs() < edge_len * host_len * 0.999
            {
                continue;
            }
            ordered_hosts.push(ProjectionHost {
                road: host,
                inward_x: -edge_dz / edge_len,
                inward_z: edge_dx / edge_len,
            });
        }

        let mut nodes = nodes.to_vec();
        nodes.sort_by_key(|node| node.id);
        let mut parcels = parcel_footprints.to_vec();
        parcels.sort_by(rect_order);
        parcels.dedup_by(|a, b| rect_order(a, b) == Ordering::Equal);
        Ok(Self::from_projected_owned(ordered_hosts, nodes, parcels))
    }

    /// Construct from an already validated and projected host list. The
    /// descriptor pipeline uses this after budgeting edge ordering, host
    /// lookup, and copies of the frozen node/parcel inputs.
    #[cfg(feature = "legacy")]
    pub(crate) fn from_projected_owned(
        hosts: Vec<ProjectionHost>,
        nodes: Vec<CandidateNode>,
        parcels: Vec<OrientedRect>,
    ) -> Self {
        let phase = if hosts.is_empty() {
            Phase::Done
        } else {
            Phase::Nodes
        };
        let checkpoint = Checkpoint {
            schema: CHECKPOINT_SCHEMA,
            hosts,
            nodes,
            parcels,
            host_index: 0,
            phase,
            node_index: 0,
            parcel_index: 0,
            parcel_corner_index: 0,
            parcel_bounds: [0.0; 4],
            forbidden: Vec::new(),
            sort_width: 1,
            sort_run_start: 0,
            sort_left: 0,
            sort_mid: 0,
            sort_right: 0,
            sort_end: 0,
            sort_output: Vec::new(),
            subtract_index: 0,
            span_index: 0,
            free_spans: Vec::new(),
            next_spans: Vec::new(),
            filter_index: 0,
            filtered_spans: Vec::new(),
            select_index: 0,
            select_compare: 0,
            selected_spans: Vec::new(),
            edge_span_cursor: 0,
            edge_candidate_cursor: 0,
            edge_compare_cursor: 0,
            edge_samples: Vec::new(),
            samples: Vec::new(),
        };
        Self { checkpoint }
    }

    pub(crate) fn restore(checkpoint: Checkpoint) -> Result<Self, SearchError> {
        validate_checkpoint(&checkpoint)?;
        Ok(Self { checkpoint })
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        self.checkpoint.clone()
    }

    pub(crate) fn split_samples(&self) -> Option<&[HostSplitSample]> {
        (self.checkpoint.phase == Phase::Done).then_some(&self.checkpoint.samples)
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || self.checkpoint.phase == Phase::Done {
            return AdvanceResult {
                work_used: 0,
                done: self.checkpoint.phase == Phase::Done,
            };
        }
        let mut work_used = 0;
        while work_used < budget && self.checkpoint.phase != Phase::Done {
            if self.transition_if_ready() {
                work_used += 1;
                continue;
            }
            let did_work = match self.checkpoint.phase {
                Phase::Nodes => self.advance_node(),
                Phase::ParcelCorners => self.advance_parcel_corner(),
                Phase::SortIntervals => self.advance_sort(),
                Phase::SubtractIntervals => self.advance_subtract(),
                Phase::FilterSpans => self.advance_filter_spans(),
                Phase::SelectSplits => self.advance_select(),
                Phase::SelectEdges => self.advance_edge_select(),
                Phase::Done => false,
            };
            if did_work {
                work_used += 1;
            }
        }
        AdvanceResult {
            work_used,
            done: self.checkpoint.phase == Phase::Done,
        }
    }

    #[cfg(feature = "legacy")]
    fn transition_if_ready(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        match cp.phase {
            Phase::Nodes if cp.node_index >= cp.nodes.len() => {
                cp.phase = Phase::ParcelCorners;
                true
            }
            Phase::ParcelCorners if cp.parcel_index >= cp.parcels.len() => {
                cp.phase = Phase::SortIntervals;
                cp.sort_width = 1;
                cp.sort_run_start = 0;
                cp.sort_output.clear();
                cp.sort_output.reserve(cp.forbidden.len());
                initialize_merge_run(cp);
                true
            }
            Phase::SortIntervals if cp.forbidden.len() <= 1 => {
                cp.phase = Phase::SubtractIntervals;
                reset_sort_cursors(cp);
                cp.free_spans = vec![Interval {
                    start: HOST_END_CLEARANCE_M,
                    end: host_length(cp.hosts[cp.host_index].road) - HOST_END_CLEARANCE_M,
                }];
                true
            }
            Phase::SubtractIntervals if cp.subtract_index >= cp.forbidden.len() => {
                cp.phase = Phase::FilterSpans;
                cp.filter_index = 0;
                cp.filtered_spans.clear();
                true
            }
            Phase::FilterSpans if cp.filter_index >= cp.free_spans.len() => {
                cp.free_spans = std::mem::take(&mut cp.filtered_spans);
                cp.filter_index = 0;
                cp.phase = Phase::SelectSplits;
                true
            }
            Phase::SelectSplits if cp.select_index >= cp.free_spans.len() => {
                if cp.free_spans.len() > 1 && !cp.selected_spans.is_empty() {
                    cp.phase = Phase::SelectEdges;
                    cp.edge_span_cursor = 0;
                    cp.edge_candidate_cursor = 0;
                    cp.edge_compare_cursor = 0;
                    cp.edge_samples.clear();
                } else {
                    self.finish_host();
                }
                true
            }
            Phase::SelectEdges
                if cp.edge_samples.len() >= MAX_EDGE_SPLITS
                    || cp.edge_span_cursor >= cp.selected_spans.len() =>
            {
                self.finish_host();
                true
            }
            Phase::Nodes
            | Phase::ParcelCorners
            | Phase::SortIntervals
            | Phase::SubtractIntervals
            | Phase::FilterSpans
            | Phase::SelectSplits
            | Phase::SelectEdges
            | Phase::Done => false,
        }
    }

    #[cfg(feature = "legacy")]
    fn advance_node(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        let host = cp.hosts[cp.host_index].road;
        let node = cp.nodes[cp.node_index];
        cp.node_index += 1;
        if node.id != host.from && node.id != host.to {
            let dx = host.bx - host.ax;
            let dz = host.bz - host.az;
            let length = crate::access_math::length(dx, dz);
            let rx = node.x - host.ax;
            let rz = node.z - host.az;
            let projection = (rx * dx + rz * dz) / length;
            let cross = (rx * dz - rz * dx).abs() / length;
            if cross < NODE_CLEARANCE_M {
                cp.forbidden.push(Interval {
                    start: projection - NODE_CLEARANCE_M,
                    end: projection + NODE_CLEARANCE_M,
                });
            }
        }
        true
    }

    #[cfg(feature = "legacy")]
    fn advance_parcel_corner(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        let host = cp.hosts[cp.host_index].road;
        let rect = cp.parcels[cp.parcel_index];
        let (x, z) = rect.corners()[cp.parcel_corner_index];
        let dx = host.bx - host.ax;
        let dz = host.bz - host.az;
        let length = crate::access_math::length(dx, dz);
        let rx = x - host.ax;
        let rz = z - host.az;
        let along = (rx * dx + rz * dz) / length;
        let side = (dx * rz - dz * rx) / length;
        if cp.parcel_corner_index == 0 {
            cp.parcel_bounds = [along, along, side, side];
        } else {
            cp.parcel_bounds[0] = cp.parcel_bounds[0].min(along);
            cp.parcel_bounds[1] = cp.parcel_bounds[1].max(along);
            cp.parcel_bounds[2] = cp.parcel_bounds[2].min(side);
            cp.parcel_bounds[3] = cp.parcel_bounds[3].max(side);
        }
        cp.parcel_corner_index += 1;
        if cp.parcel_corner_index == 4 {
            if cp.parcel_bounds[2] <= PARCEL_HOST_CLEARANCE_M
                && cp.parcel_bounds[3] >= -PARCEL_HOST_CLEARANCE_M
            {
                cp.forbidden.push(Interval {
                    start: cp.parcel_bounds[0] - PARCEL_PROJECTION_MARGIN_M,
                    end: cp.parcel_bounds[1] + PARCEL_PROJECTION_MARGIN_M,
                });
            }
            cp.parcel_index += 1;
            cp.parcel_corner_index = 0;
            cp.parcel_bounds = [0.0; 4];
        }
        true
    }

    #[cfg(feature = "legacy")]
    fn advance_sort(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        let n = cp.forbidden.len();
        if cp.sort_left < cp.sort_mid && cp.sort_right < cp.sort_end {
            let left = cp.forbidden[cp.sort_left];
            let right = cp.forbidden[cp.sort_right];
            if interval_start_order(left, right) != Ordering::Greater {
                cp.sort_output.push(left);
                cp.sort_left += 1;
            } else {
                cp.sort_output.push(right);
                cp.sort_right += 1;
            }
            return true;
        }
        if cp.sort_left < cp.sort_mid {
            cp.sort_output.push(cp.forbidden[cp.sort_left]);
            cp.sort_left += 1;
            return true;
        }
        if cp.sort_right < cp.sort_end {
            cp.sort_output.push(cp.forbidden[cp.sort_right]);
            cp.sort_right += 1;
            return true;
        }
        cp.sort_run_start = cp
            .sort_run_start
            .saturating_add(cp.sort_width.saturating_mul(2));
        if cp.sort_run_start >= n {
            std::mem::swap(&mut cp.forbidden, &mut cp.sort_output);
            cp.sort_output.clear();
            if cp.sort_width >= n {
                cp.phase = Phase::SubtractIntervals;
                reset_sort_cursors(cp);
                cp.free_spans = vec![Interval {
                    start: HOST_END_CLEARANCE_M,
                    end: host_length(cp.hosts[cp.host_index].road) - HOST_END_CLEARANCE_M,
                }];
            } else {
                cp.sort_width = cp.sort_width.saturating_mul(2);
                cp.sort_run_start = 0;
                initialize_merge_run(cp);
            }
        } else {
            initialize_merge_run(cp);
        }
        true
    }

    #[cfg(feature = "legacy")]
    fn advance_subtract(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        if cp.span_index >= cp.free_spans.len() {
            cp.free_spans = std::mem::take(&mut cp.next_spans);
            cp.span_index = 0;
            cp.subtract_index += 1;
            return true;
        }
        let current = cp.free_spans[cp.span_index];
        let exclusion = cp.forbidden[cp.subtract_index];
        cp.span_index += 1;
        if exclusion.end <= current.start || exclusion.start >= current.end {
            cp.next_spans.push(current);
        } else {
            if exclusion.start > current.start {
                cp.next_spans.push(Interval {
                    start: current.start,
                    end: exclusion.start.min(current.end),
                });
            }
            if exclusion.end < current.end {
                cp.next_spans.push(Interval {
                    start: exclusion.end.max(current.start),
                    end: current.end,
                });
            }
        }
        true
    }

    #[cfg(feature = "legacy")]
    fn advance_filter_spans(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        let span = cp.free_spans[cp.filter_index];
        cp.filter_index += 1;
        if span.end - span.start > 1.0e-3 {
            cp.filtered_spans.push(span);
        }
        true
    }

    #[cfg(feature = "legacy")]
    fn advance_select(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        let span = cp.free_spans[cp.select_index];
        if cp.free_spans.len() == 1 && span.end - span.start >= 12.0 {
            let length = span.end - span.start;
            let starts = [
                span.start + length * 0.25,
                span.start + length * 0.5,
                span.end - length * 0.25,
                span.end - length / 6.0,
            ];
            let host_len = host_length(cp.hosts[cp.host_index].road);
            let edges = access_candidates::edge_split_samples(span.start, span.end, host_len);
            for along_m in starts.into_iter().chain(edges) {
                cp.selected_spans.push(Interval {
                    start: along_m,
                    end: along_m,
                });
            }
            cp.select_index = cp.free_spans.len();
            return true;
        }
        if cp.select_compare < cp.selected_spans.len() {
            let old = cp.selected_spans[cp.select_compare];
            if split_order(span, old) == Ordering::Less {
                cp.selected_spans.insert(cp.select_compare, span);
                cp.selected_spans.truncate(MAX_INTERVAL_SPLITS);
                cp.select_index += 1;
                cp.select_compare = 0;
            } else {
                cp.select_compare += 1;
            }
            return true;
        }
        if cp.selected_spans.len() < MAX_INTERVAL_SPLITS {
            cp.selected_spans.push(span);
        }
        cp.select_index += 1;
        cp.select_compare = 0;
        true
    }

    /// Examine at most two edge samples per selected span. Each generated
    /// sample comparison against a midpoint or prior accepted edge consumes
    /// one work unit; cursor transitions are charged by this phase as well.
    #[cfg(feature = "legacy")]
    fn advance_edge_select(&mut self) -> bool {
        let cp = &mut self.checkpoint;
        if cp.edge_samples.len() >= MAX_EDGE_SPLITS {
            return true;
        }
        let span = cp.selected_spans[cp.edge_span_cursor];
        let host_len = host_length(cp.hosts[cp.host_index].road);
        let edges = access_candidates::edge_split_samples(span.start, span.end, host_len);
        if cp.edge_candidate_cursor >= edges.len() {
            cp.edge_span_cursor += 1;
            cp.edge_candidate_cursor = 0;
            cp.edge_compare_cursor = 0;
            return true;
        }
        let edge = edges[cp.edge_candidate_cursor];
        let comparison_count = cp.selected_spans.len() + cp.edge_samples.len();
        if cp.edge_compare_cursor < comparison_count {
            let duplicate = if cp.edge_compare_cursor < cp.selected_spans.len() {
                let midpoint = cp.selected_spans[cp.edge_compare_cursor];
                let midpoint = (midpoint.start + midpoint.end) * 0.5;
                access_candidates::edge_sample_is_duplicate(edge, midpoint)
            } else {
                let old = cp.edge_samples[cp.edge_compare_cursor - cp.selected_spans.len()];
                access_candidates::edge_sample_is_duplicate(edge, old)
            };
            cp.edge_compare_cursor += 1;
            if duplicate {
                cp.edge_candidate_cursor += 1;
                cp.edge_compare_cursor = 0;
            }
            return true;
        }
        cp.edge_samples.push(edge);
        cp.edge_candidate_cursor += 1;
        cp.edge_compare_cursor = 0;
        true
    }

    #[cfg(feature = "legacy")]
    fn finish_host(&mut self) {
        let cp = &mut self.checkpoint;
        let host = cp.hosts[cp.host_index].road;
        let host_len = host_length(host);
        for span in cp.selected_spans.drain(..) {
            let along_m = if (span.end - span.start).abs() <= f32::EPSILON {
                span.start
            } else {
                (span.start + span.end) * 0.5
            };
            let host_t = along_m / host_len;
            if (0.0..1.0).contains(&host_t) {
                let projection_host = cp.hosts[cp.host_index];
                cp.samples.push(HostSplitSample {
                    road_id: host.road_id,
                    along_m,
                    host_t,
                    inward_x: projection_host.inward_x,
                    inward_z: projection_host.inward_z,
                });
            }
        }
        for along_m in cp.edge_samples.drain(..) {
            let host_t = along_m / host_len;
            if (0.0..1.0).contains(&host_t) {
                let projection_host = cp.hosts[cp.host_index];
                cp.samples.push(HostSplitSample {
                    road_id: host.road_id,
                    along_m,
                    host_t,
                    inward_x: projection_host.inward_x,
                    inward_z: projection_host.inward_z,
                });
            }
        }
        cp.host_index += 1;
        cp.forbidden.clear();
        cp.sort_width = 1;
        cp.sort_run_start = 0;
        cp.sort_left = 0;
        cp.sort_mid = 0;
        cp.sort_right = 0;
        cp.sort_end = 0;
        cp.sort_output.clear();
        cp.subtract_index = 0;
        cp.span_index = 0;
        cp.free_spans.clear();
        cp.next_spans.clear();
        cp.filter_index = 0;
        cp.filtered_spans.clear();
        cp.select_index = 0;
        cp.select_compare = 0;
        cp.selected_spans.clear();
        cp.edge_span_cursor = 0;
        cp.edge_candidate_cursor = 0;
        cp.edge_compare_cursor = 0;
        cp.edge_samples.clear();
        cp.node_index = 0;
        cp.parcel_index = 0;
        cp.parcel_corner_index = 0;
        cp.parcel_bounds = [0.0; 4];
        cp.phase = if cp.host_index >= cp.hosts.len() {
            Phase::Done
        } else {
            Phase::Nodes
        };
    }
}

fn rect_order(a: &OrientedRect, b: &OrientedRect) -> Ordering {
    a.x.total_cmp(&b.x)
        .then_with(|| a.z.total_cmp(&b.z))
        .then_with(|| a.width.total_cmp(&b.width))
        .then_with(|| a.depth.total_cmp(&b.depth))
        .then_with(|| a.rotation.total_cmp(&b.rotation))
}

#[cfg(feature = "legacy")]
fn interval_start_order(a: Interval, b: Interval) -> Ordering {
    a.start
        .total_cmp(&b.start)
        .then_with(|| a.end.total_cmp(&b.end))
}

#[cfg(feature = "legacy")]
fn split_order(a: Interval, b: Interval) -> Ordering {
    (b.end - b.start)
        .total_cmp(&(a.end - a.start))
        .then_with(|| a.start.total_cmp(&b.start))
}

#[cfg(feature = "legacy")]
fn initialize_merge_run(cp: &mut Checkpoint) {
    let n = cp.forbidden.len();
    cp.sort_left = cp.sort_run_start;
    cp.sort_mid = cp.sort_run_start.saturating_add(cp.sort_width).min(n);
    cp.sort_right = cp.sort_mid;
    cp.sort_end = cp
        .sort_run_start
        .saturating_add(cp.sort_width.saturating_mul(2))
        .min(n);
}

#[cfg(feature = "legacy")]
fn reset_sort_cursors(cp: &mut Checkpoint) {
    cp.sort_width = 1;
    cp.sort_run_start = 0;
    cp.sort_left = 0;
    cp.sort_mid = 0;
    cp.sort_right = 0;
    cp.sort_end = 0;
    cp.sort_output.clear();
}

fn polygon_signed_area(polygon: &[(f32, f32)]) -> f32 {
    let mut twice_area = 0.0;
    for index in 0..polygon.len() {
        let (x0, z0) = polygon[index];
        let (x1, z1) = polygon[(index + 1) % polygon.len()];
        twice_area += x0 * z1 - x1 * z0;
    }
    twice_area * 0.5
}

fn validate_checkpoint(cp: &Checkpoint) -> Result<(), SearchError> {
    if cp.schema != CHECKPOINT_SCHEMA {
        return Err(SearchError::NonCanonicalCheckpoint);
    }
    if cp.hosts.len() > MAX_PREPARED_HOSTS
        || cp.nodes.len() > MAX_NODES
        || cp.parcels.len() > MAX_PARCELS
    {
        return Err(SearchError::InputCap);
    }
    if cp
        .hosts
        .windows(2)
        .any(|w| w[0].road.road_id >= w[1].road.road_id)
        || cp.nodes.windows(2).any(|w| w[0].id >= w[1].id)
        || cp
            .parcels
            .windows(2)
            .any(|w| rect_order(&w[0], &w[1]) != Ordering::Less)
    {
        return Err(SearchError::NonCanonicalCheckpoint);
    }
    if cp.hosts.iter().any(|h| {
        [
            h.road.ax, h.road.az, h.road.bx, h.road.bz, h.inward_x, h.inward_z,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || !host_length(h.road).is_finite()
            || host_length(h.road) < 20.0
            || (h.inward_x.hypot(h.inward_z) - 1.0).abs() > 1.0e-4
            || ((h.road.bx - h.road.ax) * h.inward_x + (h.road.bz - h.road.az) * h.inward_z).abs()
                > 1.0e-3
    }) || cp
        .nodes
        .iter()
        .any(|n| !n.x.is_finite() || !n.z.is_finite())
        || cp.parcels.iter().any(|p| !p.finite_positive())
    {
        return Err(SearchError::NonFiniteInput);
    }
    if cp.host_index > cp.hosts.len()
        || cp.node_index > cp.nodes.len()
        || cp.parcel_index > cp.parcels.len()
        || cp.parcel_corner_index >= 4
        || cp.forbidden.len() > cp.nodes.len() + cp.parcels.len()
        || cp.sort_output.len() > cp.forbidden.len()
        || cp.sort_width == 0
        || cp.sort_width
            > cp.forbidden
                .len()
                .saturating_add(1)
                .checked_next_power_of_two()
                .unwrap_or(usize::MAX)
        || cp.sort_run_start
            > cp.forbidden
                .len()
                .saturating_add(cp.sort_width.saturating_mul(2))
        || cp.sort_left > cp.sort_mid
        || cp.sort_mid > cp.sort_right
        || cp.sort_right < cp.sort_mid
        || cp.sort_right > cp.sort_end
        || cp.sort_end > cp.forbidden.len()
        || cp.subtract_index > cp.forbidden.len()
        || cp.span_index > cp.free_spans.len()
        || cp.free_spans.len() > cp.forbidden.len() + 1
        || cp.next_spans.len() > cp.forbidden.len() + 1
        || cp.filtered_spans.len() > cp.filter_index
        || cp.filter_index > cp.free_spans.len()
        || cp.filtered_spans.len() > cp.free_spans.len()
        || cp.select_index > cp.free_spans.len()
        || cp.select_compare > cp.selected_spans.len()
        || cp.selected_spans.len() > MAX_SPLITS_PER_HOST
        || cp.edge_span_cursor > cp.selected_spans.len()
        || cp.edge_candidate_cursor > MAX_EDGE_SPLITS
        || cp.edge_compare_cursor > cp.selected_spans.len() + cp.edge_samples.len()
        || cp.edge_samples.len() > MAX_EDGE_SPLITS
        || cp.samples.len() > MAX_PREPARED_HOSTS * MAX_SPLITS_PER_HOST
    {
        return Err(SearchError::InvalidCursor);
    }
    if cp.phase == Phase::SortIntervals
        && cp.sort_output.len()
            != cp
                .sort_left
                .saturating_add(cp.sort_right)
                .saturating_sub(cp.sort_mid)
    {
        return Err(SearchError::InvalidCursor);
    }
    if cp.phase != Phase::SortIntervals
        && (cp.sort_left > cp.sort_mid
            || cp.sort_mid > cp.sort_right
            || cp.sort_right > cp.sort_end)
    {
        return Err(SearchError::InvalidCursor);
    }
    if (cp.phase == Phase::Done) != (cp.host_index == cp.hosts.len())
        || (cp.phase != Phase::Done && cp.hosts.is_empty())
        || (cp.phase == Phase::Nodes && (cp.parcel_index != 0 || cp.parcel_corner_index != 0))
        || (cp.phase == Phase::ParcelCorners && cp.node_index != cp.nodes.len())
        || (cp.phase == Phase::SortIntervals && cp.parcel_index != cp.parcels.len())
        || (cp.phase == Phase::FilterSpans && cp.subtract_index != cp.forbidden.len())
        || (cp.phase == Phase::SelectSplits
            && (cp.subtract_index != cp.forbidden.len()
                || cp.filter_index != 0
                || !cp.filtered_spans.is_empty()))
        || (cp.phase == Phase::SelectEdges
            && (cp.free_spans.len() <= 1
                || cp.select_index != cp.free_spans.len()
                || cp.selected_spans.is_empty()
                || cp.edge_span_cursor > cp.selected_spans.len()
                || (cp.edge_span_cursor == cp.selected_spans.len()
                    && (cp.edge_candidate_cursor != 0 || cp.edge_compare_cursor != 0))))
        || (cp.phase != Phase::SelectEdges
            && (cp.edge_span_cursor != 0
                || cp.edge_candidate_cursor != 0
                || cp.edge_compare_cursor != 0
                || !cp.edge_samples.is_empty()))
    {
        return Err(SearchError::InvalidCursor);
    }
    match cp.phase {
        Phase::Nodes if cp.forbidden.len() > cp.node_index => {
            return Err(SearchError::InvalidCursor)
        }
        Phase::ParcelCorners if cp.forbidden.len() > cp.nodes.len() + cp.parcel_index => {
            return Err(SearchError::InvalidCursor)
        }
        Phase::SubtractIntervals if cp.next_spans.len() > cp.free_spans.len().saturating_add(1) => {
            return Err(SearchError::InvalidCursor)
        }
        Phase::FilterSpans if cp.filtered_spans.len() > cp.filter_index => {
            return Err(SearchError::InvalidCursor)
        }
        Phase::SelectSplits => {
            let fast_path = cp.free_spans.len() == 1
                && cp.free_spans[0].end - cp.free_spans[0].start >= 12.0
                && cp.select_index == cp.free_spans.len();
            if !fast_path
                && cp.selected_spans.len()
                    > cp.select_index.saturating_add(1).min(MAX_INTERVAL_SPLITS)
            {
                return Err(SearchError::InvalidCursor);
            }
        }
        Phase::SelectEdges => {
            if cp.edge_span_cursor < cp.selected_spans.len() {
                let span = cp.selected_spans[cp.edge_span_cursor];
                let edges = access_candidates::edge_split_samples(
                    span.start,
                    span.end,
                    host_length(cp.hosts[cp.host_index].road),
                );
                if cp.edge_candidate_cursor > edges.len() {
                    return Err(SearchError::InvalidCursor);
                }
            }
            if cp
                .edge_samples
                .windows(2)
                .any(|pair| (pair[0] - pair[1]).abs() < 1.0)
                || cp.edge_samples.iter().any(|edge| {
                    cp.selected_spans.iter().any(|selected| {
                        let midpoint = (selected.start + selected.end) * 0.5;
                        (midpoint - edge).abs() < 1.0
                    })
                })
            {
                return Err(SearchError::InvalidCursor);
            }
        }
        _ => {}
    }
    if cp
        .forbidden
        .iter()
        .chain(cp.sort_output.iter())
        .chain(cp.free_spans.iter())
        .chain(cp.next_spans.iter())
        .chain(cp.filtered_spans.iter())
        .chain(cp.selected_spans.iter())
        .any(|i| !i.start.is_finite() || !i.end.is_finite())
        || cp.edge_samples.iter().any(|edge| !edge.is_finite())
        || cp.parcel_bounds.iter().any(|v| !v.is_finite())
        || cp
            .samples
            .iter()
            .any(|s| !s.along_m.is_finite() || !s.host_t.is_finite())
        || cp
            .forbidden
            .iter()
            .chain(cp.sort_output.iter())
            .chain(cp.free_spans.iter())
            .chain(cp.next_spans.iter())
            .chain(cp.filtered_spans.iter())
            .chain(cp.selected_spans.iter())
            .any(|i| i.start > i.end)
    {
        return Err(SearchError::InvalidScratch);
    }
    for sample in &cp.samples {
        let host = cp
            .hosts
            .iter()
            .take(cp.host_index)
            .find(|host| host.road.road_id == sample.road_id)
            .ok_or(SearchError::InvalidScratch)?;
        if !(0.0..1.0).contains(&sample.host_t)
            || (sample.along_m - sample.host_t * host_length(host.road)).abs() > 1.0e-4
            || !sample.inward_x.is_finite()
            || !sample.inward_z.is_finite()
            || (sample.inward_x.hypot(sample.inward_z) - 1.0).abs() > 1.0e-4
            || (sample.inward_x - host.inward_x).abs() > 1.0e-5
            || (sample.inward_z - host.inward_z).abs() > 1.0e-5
        {
            return Err(SearchError::InvalidScratch);
        }
    }
    Ok(())
}

fn host_length(host: BoundaryHost) -> f32 {
    crate::access_math::length(host.bx - host.ax, host.bz - host.az)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::road_faces::RoadFace;

    fn square_face() -> RoadFace {
        RoadFace {
            identity: vec![1, 2, 3, 4],
            vertex_ids: vec![1, 2, 3, 4],
            polygon: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
            area: 10_000.0,
            boundary_edge_ids: vec![10, 11, 12, 13],
            contains_proposed_edge: false,
        }
    }

    fn hosts() -> Vec<BoundaryHost> {
        vec![
            BoundaryHost {
                road_id: 10,
                from: 1,
                to: 2,
                ax: 0.0,
                az: 0.0,
                bx: 100.0,
                bz: 0.0,
            },
            BoundaryHost {
                road_id: 11,
                from: 2,
                to: 3,
                ax: 100.0,
                az: 0.0,
                bx: 100.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 12,
                from: 3,
                to: 4,
                ax: 100.0,
                az: 100.0,
                bx: 0.0,
                bz: 100.0,
            },
            BoundaryHost {
                road_id: 13,
                from: 4,
                to: 1,
                ax: 0.0,
                az: 100.0,
                bx: 0.0,
                bz: 0.0,
            },
        ]
    }

    // Kept local to the test as an independent copy of the old one-shot
    // arithmetic, so sharing a production helper cannot make the parity test
    // tautological.
    fn old_reference_samples(
        host: BoundaryHost,
        nodes: &[CandidateNode],
        parcels: &[OrientedRect],
    ) -> Vec<f32> {
        let dx = host.bx - host.ax;
        let dz = host.bz - host.az;
        let length = crate::access_math::length(dx, dz);
        if length < 20.0 {
            return Vec::new();
        }
        let mut forbidden: Vec<(f32, f32)> = Vec::new();
        for node in nodes
            .iter()
            .filter(|node| node.id != host.from && node.id != host.to)
        {
            let rx = node.x - host.ax;
            let rz = node.z - host.az;
            let along = (rx * dx + rz * dz) / length;
            let cross = (rx * dz - rz * dx).abs() / length;
            if cross < 1.0 {
                forbidden.push((along - 1.0, along + 1.0));
            }
        }
        for rect in parcels {
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            let mut side_lo = f32::INFINITY;
            let mut side_hi = f32::NEG_INFINITY;
            for (x, z) in rect.corners() {
                let rx = x - host.ax;
                let rz = z - host.az;
                let along = (rx * dx + rz * dz) / length;
                let side = (dx * rz - dz * rx) / length;
                lo = lo.min(along);
                hi = hi.max(along);
                side_lo = side_lo.min(side);
                side_hi = side_hi.max(side);
            }
            if side_lo <= 5.0 && side_hi >= -5.0 {
                forbidden.push((lo - 2.0, hi + 2.0));
            }
        }
        forbidden.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
        let mut free = vec![(10.0, length - 10.0)];
        for (a, b) in forbidden {
            let mut next = Vec::new();
            for (lo, hi) in free {
                if b <= lo || a >= hi {
                    next.push((lo, hi));
                } else {
                    if a > lo {
                        next.push((lo, a.min(hi)));
                    }
                    if b < hi {
                        next.push((b.max(lo), hi));
                    }
                }
            }
            free = next
                .into_iter()
                .filter(|(lo, hi)| hi - lo > 1.0e-3)
                .collect();
        }
        if free.len() == 1 && free[0].1 - free[0].0 >= 12.0 {
            let (lo, hi) = free[0];
            let span = hi - lo;
            let mut samples = vec![
                lo + span * 0.25,
                lo + span * 0.5,
                hi - span * 0.25,
                hi - span / 6.0,
            ];
            // Edge samples: 1m inside a host-end bound, 3m inside any other.
            let lo_inset = if lo == 10.0 { 1.0 } else { 3.0 };
            let hi_inset = if hi == length - 10.0 { 1.0 } else { 3.0 };
            if span * 0.25 - lo_inset >= 1.0 {
                samples.push(lo + lo_inset);
            }
            if span * 0.25 - hi_inset >= 1.0 {
                samples.push(hi - hi_inset);
            }
            samples
        } else {
            let fragmented = free.len() > 1;
            free.sort_by(|left, right| {
                (right.1 - right.0)
                    .total_cmp(&(left.1 - left.0))
                    .then_with(|| left.0.total_cmp(&right.0))
            });
            let selected = free.into_iter().take(4).collect::<Vec<_>>();
            let mut samples = selected
                .iter()
                .map(|(lo, hi)| (lo + hi) * 0.5)
                .collect::<Vec<_>>();
            if fragmented {
                let mut added = 0;
                for &(lo, hi) in &selected {
                    let quartile = (hi - lo) * 0.25;
                    let lo_inset = if lo == 10.0 { 1.0 } else { 3.0 };
                    let hi_inset = if hi == length - 10.0 { 1.0 } else { 3.0 };
                    let mut edges = Vec::new();
                    if quartile - lo_inset >= 1.0 {
                        edges.push(lo + lo_inset);
                    }
                    if quartile - hi_inset >= 1.0 {
                        edges.push(hi - hi_inset);
                    }
                    for edge in edges {
                        if added >= 2 {
                            break;
                        }
                        if samples.iter().all(|old| (old - edge).abs() >= 1.0) {
                            samples.push(edge);
                            added += 1;
                        }
                    }
                    if added >= 2 {
                        break;
                    }
                }
            }
            samples
        }
    }

    #[test]
    fn split_projection_matches_one_shot_for_varied_quotas_and_restore() {
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
            CandidateNode {
                id: 3,
                x: 100.0,
                z: 100.0,
            },
            CandidateNode {
                id: 4,
                x: 0.0,
                z: 100.0,
            },
            CandidateNode {
                id: 5,
                x: 52.0,
                z: 0.4,
            },
            CandidateNode {
                id: 6,
                x: 70.0,
                z: 0.1,
            },
        ];
        let parcels = vec![
            OrientedRect {
                x: 35.0,
                z: 8.0,
                width: 12.0,
                depth: 16.0,
                rotation: 0.0,
            },
            OrientedRect {
                x: 72.0,
                z: -3.0,
                width: 14.0,
                depth: 12.0,
                rotation: 0.37,
            },
        ];
        let mut expected = Vec::new();
        let face = square_face();
        let h = hosts();
        let mut ordered = h.clone();
        ordered.sort_by_key(|host| host.road_id);
        for host in ordered {
            if face.boundary_edge_ids.contains(&host.road_id) {
                for along_m in old_reference_samples(host, &nodes, &parcels) {
                    let edge_index = face
                        .boundary_edge_ids
                        .iter()
                        .position(|id| *id == host.road_id)
                        .unwrap();
                    let (ax, az) = face.polygon[edge_index];
                    let (bx, bz) = face.polygon[(edge_index + 1) % face.polygon.len()];
                    let edge_len = crate::access_math::length(bx - ax, bz - az);
                    expected.push(HostSplitSample {
                        road_id: host.road_id,
                        along_m,
                        host_t: along_m / host_length(host),
                        inward_x: -(bz - az) / edge_len,
                        inward_z: (bx - ax) / edge_len,
                    });
                }
            }
        }

        for quotas in [vec![1], vec![7, 2, 19], vec![48], vec![3, 1, 16, 5, 31]] {
            let mut search = AccessHostPreparation::new(&face, &h, &nodes, &parcels).unwrap();
            assert_eq!(
                search.advance(0),
                AdvanceResult {
                    work_used: 0,
                    done: false
                }
            );
            let initial: Checkpoint =
                serde_json::from_slice(&serde_json::to_vec(&search.checkpoint()).unwrap()).unwrap();
            search = AccessHostPreparation::restore(initial).unwrap();
            let mut index = 0;
            while search.checkpoint.phase != Phase::Done {
                let before = search.checkpoint.clone();
                let result = search.advance(quotas[index % quotas.len()]);
                assert!(result.work_used <= quotas[index % quotas.len()]);
                if result.work_used == 0 && !result.done {
                    assert_ne!(
                        before.phase, search.checkpoint.phase,
                        "zero-cost phase transition must make progress"
                    );
                }
                let encoded = serde_json::to_vec(&search.checkpoint()).unwrap();
                let restored: Checkpoint = serde_json::from_slice(&encoded).unwrap();
                search = AccessHostPreparation::restore(restored.clone()).unwrap_or_else(|error| panic!("restore failed at quota {:?}, index {index}, phase {:?}, checkpoint {restored:?}: {error:?}", quotas, restored.phase));
                index += 1;
                assert!(index < 200_000);
            }
            let complete: Checkpoint =
                serde_json::from_slice(&serde_json::to_vec(&search.checkpoint()).unwrap()).unwrap();
            search = AccessHostPreparation::restore(complete).unwrap();
            assert_eq!(search.split_samples().unwrap(), expected.as_slice());
        }
    }

    #[test]
    fn fragmented_edge_samples_match_sync_at_quotas_one_and_seven() {
        let face = square_face();
        let host = hosts()[0];
        let nodes = [CandidateNode {
            id: 5,
            x: 50.0,
            z: 0.0,
        }];
        let expected = access_candidates::host_split_samples(host, &nodes, &[])
            .into_iter()
            .map(|along_m| HostSplitSample {
                road_id: host.road_id,
                along_m,
                host_t: along_m / host_length(host),
                inward_x: 0.0,
                inward_z: 1.0,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            expected
                .iter()
                .map(|sample| sample.along_m)
                .collect::<Vec<_>>(),
            [29.5, 70.5, 11.0, 46.0]
        );

        for quota in [1, 7] {
            let mut search = AccessHostPreparation::new(&face, &[host], &nodes, &[]).unwrap();
            while !search.advance(quota).done {
                let checkpoint: Checkpoint =
                    serde_json::from_slice(&serde_json::to_vec(&search.checkpoint()).unwrap())
                        .unwrap();
                search = AccessHostPreparation::restore(checkpoint).unwrap();
            }
            assert_eq!(search.split_samples().unwrap(), expected.as_slice());
            assert!(search.split_samples().unwrap().len() <= MAX_SPLITS_PER_HOST);
        }
    }

    #[test]
    fn fragmented_short_spans_exhaust_edge_scan_without_extra_samples() {
        let face = square_face();
        let host = hosts()[0];
        let nodes = [20.0, 32.0, 44.0, 56.0, 68.0, 80.0]
            .into_iter()
            .enumerate()
            .map(|(index, x)| CandidateNode {
                id: 20 + index as u32,
                x,
                z: 0.0,
            })
            .collect::<Vec<_>>();
        let expected_along = access_candidates::host_split_samples(host, &nodes, &[]);
        assert_eq!(expected_along.len(), MAX_INTERVAL_SPLITS);
        assert!(expected_along.iter().all(|sample| {
            ![20.0, 32.0, 44.0, 56.0, 68.0, 80.0]
                .iter()
                .any(|node| (sample - node).abs() < 4.0)
        }));
        let mut search = AccessHostPreparation::new(&face, &[host], &nodes, &[]).unwrap();
        let mut saw_edge_phase = false;
        let mut saw_terminal_edge_cursor = false;
        while !search.advance(1).done {
            saw_edge_phase |= search.checkpoint.phase == Phase::SelectEdges;
            saw_terminal_edge_cursor |= search.checkpoint.phase == Phase::SelectEdges
                && search.checkpoint.edge_span_cursor == search.checkpoint.selected_spans.len();
            let checkpoint: Checkpoint =
                serde_json::from_slice(&serde_json::to_vec(&search.checkpoint()).unwrap()).unwrap();
            search = AccessHostPreparation::restore(checkpoint).unwrap();
        }
        assert!(saw_edge_phase);
        assert!(saw_terminal_edge_cursor);
        assert_eq!(
            search
                .split_samples()
                .unwrap()
                .iter()
                .map(|sample| sample.along_m)
                .collect::<Vec<_>>(),
            expected_along
        );
        assert!(search.split_samples().unwrap().len() <= MAX_INTERVAL_SPLITS);

        let mut unfinished = AccessHostPreparation::new(&face, &[host], &nodes, &[]).unwrap();
        while unfinished.checkpoint.phase != Phase::SelectEdges {
            unfinished.advance(1);
        }
        let mut corrupt = unfinished.checkpoint();
        corrupt.edge_span_cursor = usize::MAX;
        assert_eq!(
            AccessHostPreparation::restore(corrupt).unwrap_err(),
            SearchError::InvalidCursor
        );
    }

    #[test]
    fn malformed_and_over_cap_checkpoints_are_rejected() {
        let face = square_face();
        let too_many_nodes = (0..=MAX_NODES)
            .map(|index| CandidateNode {
                id: index as u32,
                x: 0.0,
                z: 0.0,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            AccessHostPreparation::new(&face, &hosts(), &too_many_nodes, &[]).unwrap_err(),
            SearchError::InputCap
        );
        let mut search = AccessHostPreparation::new(&face, &hosts(), &[], &[]).unwrap();
        search.advance(1);
        let mut checkpoint = search.checkpoint();
        checkpoint.host_index = usize::MAX;
        assert_eq!(
            AccessHostPreparation::restore(checkpoint).unwrap_err(),
            SearchError::InvalidCursor
        );
        let mut checkpoint = search.checkpoint();
        checkpoint.schema += 1;
        assert_eq!(
            AccessHostPreparation::restore(checkpoint).unwrap_err(),
            SearchError::NonCanonicalCheckpoint
        );
        let mut checkpoint = search.checkpoint();
        checkpoint.phase = Phase::Done;
        assert_eq!(
            AccessHostPreparation::restore(checkpoint).unwrap_err(),
            SearchError::InvalidCursor
        );
        let mut checkpoint = search.checkpoint();
        checkpoint.parcel_bounds[0] = f32::NAN;
        assert_eq!(
            AccessHostPreparation::restore(checkpoint).unwrap_err(),
            SearchError::InvalidScratch
        );
        let mut search = AccessHostPreparation::new(&face, &hosts(), &[], &[]).unwrap();
        let mut checkpoint = search.checkpoint();
        checkpoint.forbidden.push(Interval {
            start: 30.0,
            end: 32.0,
        });
        assert_eq!(
            AccessHostPreparation::restore(checkpoint).unwrap_err(),
            SearchError::InvalidCursor
        );
        let search = AccessHostPreparation::new(&face, &hosts(), &[], &[]).unwrap();
        let mut search = search;
        let mut steps = 0;
        while search.checkpoint.phase != Phase::FilterSpans && steps < 10_000 {
            let result = search.advance(1);
            assert!(
                !result.done,
                "search finished before reaching span filtering"
            );
            steps += 1;
        }
        assert_eq!(
            search.checkpoint.phase,
            Phase::FilterSpans,
            "bounded search did not reach span filtering"
        );
        let mut checkpoint = search.checkpoint();
        checkpoint.filtered_spans.push(Interval {
            start: 11.0,
            end: 12.0,
        });
        assert_eq!(
            AccessHostPreparation::restore(checkpoint).unwrap_err(),
            SearchError::InvalidCursor
        );
        search.advance(1);
        let mut checkpoint = search.checkpoint();
        checkpoint.hosts[0].road.bx = f32::MAX;
        checkpoint.hosts[0].road.ax = -f32::MAX;
        assert_eq!(
            AccessHostPreparation::restore(checkpoint).unwrap_err(),
            SearchError::NonFiniteInput
        );
    }

    #[test]
    fn sub_millimeter_residual_is_filtered_like_one_shot_helper() {
        let face = square_face();
        let host = hosts()[0];
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
            CandidateNode {
                id: 5,
                x: 40.0,
                z: 0.0,
            },
            CandidateNode {
                id: 6,
                x: 42.0005,
                z: 0.0,
            },
        ];
        let mut search = AccessHostPreparation::new(&face, &[host], &nodes, &[]).unwrap();
        let mut calls = 0;
        while !search.advance(1).done {
            let checkpoint: Checkpoint =
                serde_json::from_slice(&serde_json::to_vec(&search.checkpoint()).unwrap()).unwrap();
            search = AccessHostPreparation::restore(checkpoint.clone()).unwrap_or_else(|error| {
                panic!("tiny-gap restore failed {error:?}: {checkpoint:?}")
            });
            calls += 1;
            assert!(calls < 1000);
        }
        let expected = old_reference_samples(host, &nodes, &[]);
        assert_eq!(
            search
                .split_samples()
                .unwrap()
                .iter()
                .map(|s| s.along_m)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(search
            .split_samples()
            .unwrap()
            .iter()
            .all(|sample| !(41.0..=41.0005).contains(&sample.along_m)));
    }
}
