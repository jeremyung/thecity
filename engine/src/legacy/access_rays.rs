//! Quota-stepped equivalents of the access planner's face ray/segment tests.
//!
//! The query preserves the existing 1 m sampling and edge classification
//! arithmetic. Each polygon edge test and each sample boundary is charged as
//! one work unit; restoring a checkpoint may replay at most one 64-edge point
//! prefix to validate its cursor.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_POLYGON_VERTICES: usize = 64;
const MAX_COORDINATE: f32 = 1_000_000.0;
const RAY_LOOKAHEAD_M: f32 = 62.0;
const SAMPLE_M: f32 = 1.0;
#[cfg(feature = "legacy")]
const SEGMENT_MAX_STEPS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum RayQueryKind {
    Exit {
        start: (f32, f32),
        heading: (f32, f32),
    },
    Segment {
        start: (f32, f32),
        end: (f32, f32),
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum RayResult {
    ExitDistance(f32),
    SegmentInside(bool),
}

/// A polygon whose size, finiteness, and coordinate bounds have been checked.
/// The private shared slice lets many ray queries reuse one validation pass.
#[derive(Clone, Debug)]
pub(crate) struct ValidatedPolygon {
    points: Arc<[(f32, f32)]>,
}

impl ValidatedPolygon {
    pub(crate) fn new(points: Vec<(f32, f32)>) -> Result<Self, RayError> {
        validate_polygon(&points)?;
        Ok(Self {
            points: Arc::from(points),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RayError {
    InvalidPolygon,
    NonFiniteInput,
    UnsafeQuery,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    StartSample,
    ClassifyEdges,
    FinishSample,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RayCheckpoint {
    schema: u8,
    #[serde(with = "arc_slice_serde")]
    polygon: Arc<[(f32, f32)]>,
    kind: RayQueryKind,
    phase: Phase,
    sample_index: usize,
    sample_x: f32,
    sample_z: f32,
    edge_index: usize,
    inside: bool,
    point_result: Option<bool>,
    last_inside: f32,
    result: Option<RayResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct RayAdvance {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct RayQuery {
    checkpoint: RayCheckpoint,
    polygon: ValidatedPolygon,
}

impl RayQuery {
    #[cfg(any(test, feature = "legacy"))]
    pub(crate) fn new(polygon: Vec<(f32, f32)>, kind: RayQueryKind) -> Result<Self, RayError> {
        Self::with_polygon(ValidatedPolygon::new(polygon)?, kind)
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn with_polygon(
        polygon: ValidatedPolygon,
        kind: RayQueryKind,
    ) -> Result<Self, RayError> {
        validate_kind(kind)?;

        let mut checkpoint = RayCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            polygon: Arc::clone(&polygon.points),
            kind,
            phase: Phase::StartSample,
            sample_index: 1,
            sample_x: 0.0,
            sample_z: 0.0,
            edge_index: 0,
            inside: false,
            point_result: None,
            last_inside: 0.0,
            result: None,
        };

        if let RayQueryKind::Segment { start, end } = kind {
            let steps = segment_steps(start, end);
            if steps == 0 || steps > SEGMENT_MAX_STEPS {
                checkpoint.phase = Phase::Done;
                checkpoint.result = Some(RayResult::SegmentInside(false));
            } else if steps == 1 {
                // The reference evaluates the empty range `1..steps`, so a
                // sub-meter segment has no interior samples and is inside.
                checkpoint.phase = Phase::Done;
                checkpoint.result = Some(RayResult::SegmentInside(true));
            }
        }
        Ok(Self {
            checkpoint,
            polygon,
        })
    }

    pub(crate) fn restore(checkpoint: RayCheckpoint) -> Result<Self, RayError> {
        validate_checkpoint(&checkpoint)?;
        let polygon = ValidatedPolygon {
            points: Arc::clone(&checkpoint.polygon),
        };
        Ok(Self {
            checkpoint,
            polygon,
        })
    }

    pub(crate) fn checkpoint(&self) -> RayCheckpoint {
        self.checkpoint.clone()
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn result(&self) -> Option<RayResult> {
        self.checkpoint.result
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn advance(&mut self, budget: usize) -> RayAdvance {
        if budget == 0 || self.checkpoint.phase == Phase::Done {
            return RayAdvance {
                work_used: 0,
                done: self.checkpoint.phase == Phase::Done,
            };
        }

        let mut work_used = 0;
        while work_used < budget && self.checkpoint.phase != Phase::Done {
            match self.checkpoint.phase {
                Phase::StartSample => {
                    self.start_sample();
                    work_used += 1;
                }
                Phase::ClassifyEdges => {
                    self.classify_one_edge();
                    work_used += 1;
                }
                Phase::FinishSample => {
                    self.finish_sample();
                    work_used += 1;
                }
                Phase::Done => break,
            }
        }
        RayAdvance {
            work_used,
            done: self.checkpoint.phase == Phase::Done,
        }
    }

    #[cfg(feature = "legacy")]
    fn start_sample(&mut self) {
        let cp = &mut self.checkpoint;
        let (x, z) = sample_point(cp.kind, cp.sample_index);
        cp.sample_x = x;
        cp.sample_z = z;
        cp.edge_index = 0;
        cp.inside = false;
        cp.point_result = None;
        cp.phase = Phase::ClassifyEdges;
    }

    #[cfg(feature = "legacy")]
    fn classify_one_edge(&mut self) {
        let cp = &mut self.checkpoint;
        let current = cp.edge_index;
        let previous = if current == 0 {
            cp.polygon.len() - 1
        } else {
            current - 1
        };
        let (ax, az) = self.polygon.points[previous];
        let (bx, bz) = self.polygon.points[current];
        let cross = (bx - ax) * (cp.sample_z - az) - (bz - az) * (cp.sample_x - ax);
        let on_boundary = cross.abs() <= 1.0e-4
            && cp.sample_x >= ax.min(bx) - 1.0e-4
            && cp.sample_x <= ax.max(bx) + 1.0e-4
            && cp.sample_z >= az.min(bz) - 1.0e-4
            && cp.sample_z <= az.max(bz) + 1.0e-4;
        cp.edge_index += 1;
        if on_boundary {
            cp.point_result = Some(true);
            cp.phase = Phase::FinishSample;
            return;
        }
        if (az > cp.sample_z) != (bz > cp.sample_z)
            && cp.sample_x < (bx - ax) * (cp.sample_z - az) / (bz - az) + ax
        {
            cp.inside = !cp.inside;
        }
        if cp.edge_index == cp.polygon.len() {
            cp.point_result = Some(cp.inside);
            cp.phase = Phase::FinishSample;
        }
    }

    #[cfg(feature = "legacy")]
    fn finish_sample(&mut self) {
        let cp = &mut self.checkpoint;
        let inside = cp.point_result.unwrap_or(false);
        match cp.kind {
            RayQueryKind::Exit { .. } => {
                if inside {
                    cp.last_inside = cp.sample_index as f32 * SAMPLE_M;
                    cp.sample_index += 1;
                    if cp.sample_index > ray_steps() {
                        clear_point_state(cp);
                        cp.phase = Phase::Done;
                        cp.result = Some(RayResult::ExitDistance(RAY_LOOKAHEAD_M));
                    } else {
                        clear_point_state(cp);
                        cp.phase = Phase::StartSample;
                    }
                } else {
                    let last_inside = cp.last_inside;
                    clear_point_state(cp);
                    cp.phase = Phase::Done;
                    cp.result = Some(RayResult::ExitDistance(last_inside));
                }
            }
            RayQueryKind::Segment { start, end } => {
                if !inside {
                    clear_point_state(cp);
                    cp.phase = Phase::Done;
                    cp.result = Some(RayResult::SegmentInside(false));
                } else {
                    cp.sample_index += 1;
                    if cp.sample_index >= segment_steps(start, end) {
                        clear_point_state(cp);
                        cp.phase = Phase::Done;
                        cp.result = Some(RayResult::SegmentInside(true));
                    } else {
                        clear_point_state(cp);
                        cp.phase = Phase::StartSample;
                    }
                }
            }
        }
    }
}

/// Serialize an Arc slice as the same JSON sequence used by the original
/// Vec-backed checkpoint format, without requiring serde's `rc` feature.
mod arc_slice_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::sync::Arc;

    pub(crate) fn serialize<S>(value: &Arc<[(f32, f32)]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_ref().serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[(f32, f32)]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let points = Vec::<(f32, f32)>::deserialize(deserializer)?;
        Ok(Arc::from(points))
    }
}

#[cfg(feature = "legacy")]
fn clear_point_state(cp: &mut RayCheckpoint) {
    cp.sample_x = 0.0;
    cp.sample_z = 0.0;
    cp.edge_index = 0;
    cp.inside = false;
    cp.point_result = None;
}

fn validate_polygon(polygon: &[(f32, f32)]) -> Result<(), RayError> {
    if polygon.len() < 3 || polygon.len() > MAX_POLYGON_VERTICES {
        return Err(RayError::InvalidPolygon);
    }
    if polygon
        .iter()
        .any(|(x, z)| !x.is_finite() || !z.is_finite())
    {
        return Err(RayError::NonFiniteInput);
    }
    if polygon
        .iter()
        .any(|(x, z)| x.abs() > MAX_COORDINATE || z.abs() > MAX_COORDINATE)
    {
        return Err(RayError::UnsafeQuery);
    }
    Ok(())
}

fn validate_kind(kind: RayQueryKind) -> Result<(), RayError> {
    let safe_point = |(x, z): (f32, f32)| {
        x.is_finite() && z.is_finite() && x.abs() <= MAX_COORDINATE && z.abs() <= MAX_COORDINATE
    };
    match kind {
        RayQueryKind::Exit { start, heading } => {
            if !safe_point(start) || !heading.0.is_finite() || !heading.1.is_finite() {
                return Err(RayError::NonFiniteInput);
            }
            if heading.0.abs() > 1.0001
                || heading.1.abs() > 1.0001
                || (heading.0.hypot(heading.1) - 1.0).abs() > 1.0e-3
                || (start.0 + heading.0 * RAY_LOOKAHEAD_M).abs() > MAX_COORDINATE
                || (start.1 + heading.1 * RAY_LOOKAHEAD_M).abs() > MAX_COORDINATE
            {
                return Err(RayError::UnsafeQuery);
            }
        }
        RayQueryKind::Segment { start, end } => {
            if !safe_point(start) || !safe_point(end) {
                return Err(RayError::NonFiniteInput);
            }
            let length = (end.0 - start.0).hypot(end.1 - start.1);
            if !length.is_finite() {
                return Err(RayError::UnsafeQuery);
            }
        }
    }
    Ok(())
}

fn segment_steps(start: (f32, f32), end: (f32, f32)) -> usize {
    ((end.0 - start.0).hypot(end.1 - start.1) / SAMPLE_M).ceil() as usize
}

fn ray_steps() -> usize {
    (RAY_LOOKAHEAD_M / SAMPLE_M).ceil() as usize
}

fn sample_point(kind: RayQueryKind, index: usize) -> (f32, f32) {
    match kind {
        RayQueryKind::Exit { start, heading } => {
            let distance = index as f32 * SAMPLE_M;
            (
                start.0 + heading.0 * distance,
                start.1 + heading.1 * distance,
            )
        }
        RayQueryKind::Segment { start, end } => {
            let t = index as f32 / segment_steps(start, end) as f32;
            (
                start.0 + (end.0 - start.0) * t,
                start.1 + (end.1 - start.1) * t,
            )
        }
    }
}

fn validate_checkpoint(cp: &RayCheckpoint) -> Result<(), RayError> {
    if cp.schema != CHECKPOINT_SCHEMA {
        return Err(RayError::InvalidCheckpoint);
    }
    validate_polygon(&cp.polygon).map_err(|_| RayError::InvalidCheckpoint)?;
    validate_kind(cp.kind).map_err(|_| RayError::InvalidCheckpoint)?;
    if !cp.sample_x.is_finite()
        || !cp.sample_z.is_finite()
        || !cp.last_inside.is_finite()
        || cp.edge_index > cp.polygon.len()
    {
        return Err(RayError::InvalidCheckpoint);
    }

    let max_sample = match cp.kind {
        RayQueryKind::Exit { .. } => ray_steps(),
        RayQueryKind::Segment { start, end } => segment_steps(start, end).saturating_sub(1),
    };
    if cp.sample_index == 0 || cp.sample_index > max_sample.saturating_add(1) {
        return Err(RayError::InvalidCheckpoint);
    }
    if cp.phase != Phase::Done && cp.sample_index > max_sample {
        return Err(RayError::InvalidCheckpoint);
    }
    match cp.kind {
        RayQueryKind::Exit { .. }
            if cp.phase != Phase::Done && cp.last_inside != (cp.sample_index - 1) as f32 =>
        {
            return Err(RayError::InvalidCheckpoint);
        }
        RayQueryKind::Segment { .. } if cp.last_inside != 0.0 => {
            return Err(RayError::InvalidCheckpoint);
        }
        _ => {}
    }

    match cp.phase {
        Phase::StartSample => {
            if cp.result.is_some()
                || cp.point_result.is_some()
                || cp.edge_index != 0
                || cp.inside
                || cp.sample_x != 0.0
                || cp.sample_z != 0.0
                || (matches!(cp.kind, RayQueryKind::Exit { .. })
                    && cp.sample_index == 1
                    && cp.last_inside != 0.0)
                || (matches!(cp.kind, RayQueryKind::Segment { .. }) && cp.last_inside != 0.0)
                || (matches!(cp.kind, RayQueryKind::Exit { .. })
                    && cp.sample_index > 1
                    && cp.last_inside != (cp.sample_index - 1) as f32)
            {
                return Err(RayError::InvalidCheckpoint);
            }
        }
        Phase::ClassifyEdges | Phase::FinishSample => {
            if cp.result.is_some() {
                return Err(RayError::InvalidCheckpoint);
            }
            let expected_point = sample_point(cp.kind, cp.sample_index);
            if cp.sample_x.to_bits() != expected_point.0.to_bits()
                || cp.sample_z.to_bits() != expected_point.1.to_bits()
            {
                return Err(RayError::InvalidCheckpoint);
            }
            let (expected_inside, expected_result, expected_edge) =
                replay_prefix(&cp.polygon, expected_point, cp.edge_index)?;
            if cp.inside != expected_inside
                || cp.point_result != expected_result
                || cp.edge_index != expected_edge
                || (cp.phase == Phase::ClassifyEdges && cp.point_result.is_some())
                || (cp.phase == Phase::FinishSample && cp.point_result.is_none())
            {
                return Err(RayError::InvalidCheckpoint);
            }
        }
        Phase::Done => {
            if cp.point_result.is_some() || cp.result.is_none() {
                return Err(RayError::InvalidCheckpoint);
            }
            let valid_result = match (cp.kind, cp.result) {
                (RayQueryKind::Exit { .. }, Some(RayResult::ExitDistance(distance))) => {
                    (0.0..=RAY_LOOKAHEAD_M).contains(&distance)
                        && distance % SAMPLE_M == 0.0
                        && cp.last_inside == distance
                        && cp.sample_index
                            == if distance >= RAY_LOOKAHEAD_M {
                                ray_steps() + 1
                            } else {
                                distance as usize + 1
                            }
                }
                (RayQueryKind::Segment { .. }, Some(RayResult::SegmentInside(_))) => true,
                _ => false,
            };
            if !valid_result {
                return Err(RayError::InvalidCheckpoint);
            }
        }
    }
    if cp.last_inside < 0.0 || cp.last_inside > RAY_LOOKAHEAD_M {
        return Err(RayError::InvalidCheckpoint);
    }
    Ok(())
}

/// Replays at most one polygon-edge prefix while validating a persisted
/// in-progress point classifier. It intentionally mirrors point_in_polygon.
fn replay_prefix(
    polygon: &[(f32, f32)],
    point: (f32, f32),
    edge_limit: usize,
) -> Result<(bool, Option<bool>, usize), RayError> {
    if edge_limit > polygon.len() {
        return Err(RayError::InvalidCheckpoint);
    }
    let mut inside = false;
    for current in 0..edge_limit {
        let previous = if current == 0 {
            polygon.len() - 1
        } else {
            current - 1
        };
        let (ax, az) = polygon[previous];
        let (bx, bz) = polygon[current];
        let cross = (bx - ax) * (point.1 - az) - (bz - az) * (point.0 - ax);
        let on_boundary = cross.abs() <= 1.0e-4
            && point.0 >= ax.min(bx) - 1.0e-4
            && point.0 <= ax.max(bx) + 1.0e-4
            && point.1 >= az.min(bz) - 1.0e-4
            && point.1 <= az.max(bz) + 1.0e-4;
        if on_boundary {
            if current + 1 != edge_limit {
                return Err(RayError::InvalidCheckpoint);
            }
            return Ok((inside, Some(true), edge_limit));
        }
        if (az > point.1) != (bz > point.1) && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
        {
            inside = !inside;
        }
    }
    let result = (edge_limit == polygon.len()).then_some(inside);
    Ok((inside, result, edge_limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point_in_polygon(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
        let mut inside = false;
        let mut previous = polygon.len() - 1;
        for current in 0..polygon.len() {
            let (ax, az) = polygon[previous];
            let (bx, bz) = polygon[current];
            let cross = (bx - ax) * (point.1 - az) - (bz - az) * (point.0 - ax);
            if cross.abs() <= 1.0e-4
                && point.0 >= ax.min(bx) - 1.0e-4
                && point.0 <= ax.max(bx) + 1.0e-4
                && point.1 >= az.min(bz) - 1.0e-4
                && point.1 <= az.max(bz) + 1.0e-4
            {
                return true;
            }
            if (az > point.1) != (bz > point.1)
                && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
            {
                inside = !inside;
            }
            previous = current;
        }
        inside
    }

    fn old_ray(start: (f32, f32), heading: (f32, f32), polygon: &[(f32, f32)]) -> f32 {
        let mut last_inside = 0.0;
        for step in 1..=62 {
            let distance = step as f32;
            let point = (
                start.0 + heading.0 * distance,
                start.1 + heading.1 * distance,
            );
            if !point_in_polygon(point, polygon) {
                return last_inside;
            }
            last_inside = distance;
        }
        62.0
    }

    fn old_segment(start: (f32, f32), end: (f32, f32), polygon: &[(f32, f32)]) -> bool {
        let length = (end.0 - start.0).hypot(end.1 - start.1);
        let steps = (length / 1.0).ceil() as usize;
        if steps == 0 || steps > 64 {
            return false;
        }
        (1..steps).all(|index| {
            let t = index as f32 / steps as f32;
            point_in_polygon(
                (
                    start.0 + (end.0 - start.0) * t,
                    start.1 + (end.1 - start.1) * t,
                ),
                polygon,
            )
        })
    }

    fn finish(mut query: RayQuery, quotas: &[usize]) -> (RayResult, usize) {
        let mut calls = 0;
        while query.result().is_none() {
            let budget = quotas[calls % quotas.len()];
            let before = query.checkpoint();
            let result = query.advance(budget);
            assert!(result.work_used <= budget);
            if budget > 0 && result.work_used == 0 && !result.done {
                assert_ne!(before.phase, query.checkpoint().phase);
            }
            let bytes = serde_json::to_vec(&query.checkpoint()).unwrap();
            query = RayQuery::restore(serde_json::from_slice(&bytes).unwrap()).unwrap();
            calls += 1;
            assert!(calls < 100_000);
        }
        (query.result().unwrap(), calls)
    }

    #[test]
    fn ray_exit_matches_reference_for_convex_concave_and_boundary_paths() {
        let cases = [
            (
                vec![(-2.0, -2.0), (40.0, -2.0), (40.0, 2.0), (-2.0, 2.0)],
                (0.0, 0.0),
                (1.0, 0.0),
            ),
            (
                vec![
                    (-2.0, -3.0),
                    (12.0, -3.0),
                    (12.0, -1.0),
                    (4.0, -1.0),
                    (4.0, 3.0),
                    (-2.0, 3.0),
                ],
                (0.0, 0.0),
                (1.0, 0.0),
            ),
            (
                vec![(0.0, 0.0), (9.0, 0.0), (9.0, 6.0), (0.0, 6.0)],
                (0.0, 0.0),
                (1.0, 0.0),
            ),
            (
                vec![(-2.0, -2.0), (100.0, -2.0), (100.0, 2.0), (-2.0, 2.0)],
                (0.0, 0.0),
                (1.0, 0.0),
            ),
        ];
        for (polygon, start, heading) in cases {
            let expected = old_ray(start, heading, &polygon);
            for quotas in [&[1][..], &[7][..], &[48][..], &[3, 11, 2, 48][..]] {
                let (actual, _) = finish(
                    RayQuery::new(polygon.clone(), RayQueryKind::Exit { start, heading }).unwrap(),
                    quotas,
                );
                assert_eq!(actual, RayResult::ExitDistance(expected));
            }
        }
    }

    #[test]
    fn segment_matches_reference_including_empty_and_overlong_sampling() {
        let polygon = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
        for (start, end) in [
            ((1.0, 1.0), (1.5, 1.0)),
            ((1.0, 1.0), (10.0, 10.0)),
            ((1.0, 1.0), (21.0, 1.0)),
            ((1.0, 1.0), (80.0, 1.0)),
            ((1.0, 1.0), (1.0, 1.0)),
        ] {
            let expected = old_segment(start, end, &polygon);
            for quotas in [&[1][..], &[7][..], &[48][..], &[5, 2, 17][..]] {
                let (actual, _) = finish(
                    RayQuery::new(polygon.clone(), RayQueryKind::Segment { start, end }).unwrap(),
                    quotas,
                );
                assert_eq!(actual, RayResult::SegmentInside(expected));
            }
        }
    }

    #[test]
    fn restore_rejects_bad_cursor_and_query_state() {
        let mut query = RayQuery::new(
            vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)],
            RayQueryKind::Exit {
                start: (1.0, 1.0),
                heading: (1.0, 0.0),
            },
        )
        .unwrap();
        query.advance(2);
        let mut cp = query.checkpoint();
        cp.edge_index = usize::MAX;
        assert_eq!(
            RayQuery::restore(cp).unwrap_err(),
            RayError::InvalidCheckpoint
        );

        let mut query = RayQuery::new(
            vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)],
            RayQueryKind::Exit {
                start: (1.0, 1.0),
                heading: (1.0, 0.0),
            },
        )
        .unwrap();
        query.advance(2);
        let cp = query.checkpoint();
        let decoded: RayCheckpoint =
            serde_json::from_slice(&serde_json::to_vec(&cp).unwrap()).unwrap();
        assert!(RayQuery::restore(decoded).is_ok());

        let mut query = RayQuery::new(
            vec![(-2.0, -2.0), (100.0, -2.0), (100.0, 2.0), (-2.0, 2.0)],
            RayQueryKind::Exit {
                start: (0.0, 0.0),
                heading: (1.0, 0.0),
            },
        )
        .unwrap();
        // One complete in-bounds point reaches StartSample at step two.
        query.advance(6);
        let mut cp = query.checkpoint();
        assert_eq!(cp.phase, Phase::StartSample);
        cp.sample_index = ray_steps() + 1;
        cp.last_inside = RAY_LOOKAHEAD_M;
        assert_eq!(
            RayQuery::restore(cp).unwrap_err(),
            RayError::InvalidCheckpoint
        );
    }

    #[test]
    fn constructor_rejects_nonfinite_unsafe_and_oversized_inputs() {
        assert_eq!(
            RayQuery::new(
                vec![(0.0, 0.0), (1.0, 0.0)],
                RayQueryKind::Exit {
                    start: (0.0, 0.0),
                    heading: (1.0, 0.0)
                }
            )
            .unwrap_err(),
            RayError::InvalidPolygon,
        );
        assert_eq!(
            RayQuery::new(
                vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)],
                RayQueryKind::Exit {
                    start: (0.0, 0.0),
                    heading: (f32::NAN, 0.0)
                }
            )
            .unwrap_err(),
            RayError::NonFiniteInput,
        );
    }

    #[test]
    fn validated_polygon_queries_share_storage_and_keep_checkpoint_json_shape() {
        let points = vec![
            (-1.25, -2.5),
            (31.0, -2.5),
            (31.0, 7.75),
            (9.5, 7.75),
            (9.5, 18.0),
            (-1.25, 18.0),
        ];
        let kind = RayQueryKind::Exit {
            start: (0.0, 0.0),
            heading: (1.0, 0.0),
        };
        let shared = ValidatedPolygon::new(points.clone()).unwrap();
        let mut query = RayQuery::with_polygon(shared, kind).unwrap();
        assert!(Arc::ptr_eq(
            &query.polygon.points,
            &query.checkpoint.polygon
        ));

        let json = serde_json::to_value(query.checkpoint()).unwrap();
        assert_eq!(json["polygon"], serde_json::to_value(&points).unwrap());
        assert!(json["polygon"].is_array());

        let mut legacy = RayQuery::new(points, kind).unwrap();
        query.advance(9);
        legacy.advance(9);
        assert_eq!(query.result(), legacy.result());
        assert_eq!(
            serde_json::to_value(query.checkpoint()).unwrap(),
            serde_json::to_value(legacy.checkpoint()).unwrap()
        );

        let restored: RayCheckpoint = serde_json::from_value(json).unwrap();
        let restored = RayQuery::restore(restored).unwrap();
        assert!(Arc::ptr_eq(
            &restored.polygon.points,
            &restored.checkpoint.polygon
        ));
    }
}
