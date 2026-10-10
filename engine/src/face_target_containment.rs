//! Resumable, boundary-inclusive point-in-face classification used by target
//! sampling. The arithmetic/order mirrors the mature diagnostic oracle.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_POLYGON_VERTICES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentError {
    InvalidPolygon,
    NonFiniteInput,
    InvalidCheckpoint,
}

/// A polygon validated once and shared by all point queries against one face.
#[derive(Clone, Debug)]
pub(crate) struct ValidatedPolygon {
    points: Arc<[(f32, f32)]>,
    fingerprint: u64,
}

impl ValidatedPolygon {
    pub(crate) fn new(points: Vec<(f32, f32)>) -> Result<Self, ContainmentError> {
        if points.len() < 3 || points.len() > MAX_POLYGON_VERTICES {
            return Err(ContainmentError::InvalidPolygon);
        }
        let mut fingerprint = polygon_fingerprint_start(points.len());
        for &point in &points {
            if !point.0.is_finite() || !point.1.is_finite() {
                return Err(ContainmentError::NonFiniteInput);
            }
            fingerprint = polygon_fingerprint_add(fingerprint, point);
        }
        Ok(Self {
            points: Arc::from(points),
            fingerprint,
        })
    }

    /// Construct from a polygon whose exact contents were already verified
    /// while being copied in a quota-stepped preparation stage.
    pub(crate) fn from_validated(points: Vec<(f32, f32)>, fingerprint: u64) -> Self {
        Self {
            points: Arc::from(points),
            fingerprint,
        }
    }

    pub(crate) fn points(&self) -> &[(f32, f32)] {
        &self.points
    }

    pub(crate) fn checkpoint_points(&self) -> Vec<(f32, f32)> {
        self.points.to_vec()
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PointContainmentCheckpoint {
    schema: u8,
    #[serde(with = "arc_slice_serde")]
    polygon: Arc<[(f32, f32)]>,
    point: (f32, f32),
    edge_index: usize,
    inside: bool,
    result: Option<bool>,
}

/// Compact query state bound to a separately persisted frozen polygon.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PointContainmentState {
    schema: u8,
    polygon_fingerprint: u64,
    point: (f32, f32),
    edge_index: usize,
    inside: bool,
    result: Option<bool>,
}

impl PointContainmentState {
    pub(crate) fn result(&self) -> Option<bool> {
        self.result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PointContainment {
    state: PointContainmentCheckpoint,
    polygon: ValidatedPolygon,
}

impl PointContainment {
    pub(crate) fn new(
        point: (f32, f32),
        polygon: ValidatedPolygon,
    ) -> Result<Self, ContainmentError> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return Err(ContainmentError::NonFiniteInput);
        }
        let state = PointContainmentCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            polygon: Arc::clone(&polygon.points),
            point,
            edge_index: 0,
            inside: false,
            result: None,
        };
        Ok(Self { state, polygon })
    }

    pub(crate) fn restore(state: PointContainmentCheckpoint) -> Result<Self, ContainmentError> {
        validate_checkpoint(&state)?;
        let polygon = ValidatedPolygon {
            points: Arc::clone(&state.polygon),
            fingerprint: polygon_fingerprint(&state.polygon),
        };
        Ok(Self { state, polygon })
    }

    /// Checkpoints share polygon memory during ordinary execution. Serde only
    /// materializes the polygon when the caller explicitly persists a job.
    pub(crate) fn checkpoint(&self) -> PointContainmentCheckpoint {
        self.state.clone()
    }

    pub(crate) fn checkpoint_state(&self) -> PointContainmentState {
        PointContainmentState {
            schema: self.state.schema,
            polygon_fingerprint: self.polygon.fingerprint,
            point: self.state.point,
            edge_index: self.state.edge_index,
            inside: self.state.inside,
            result: self.state.result,
        }
    }

    pub(crate) fn restore_state(
        state: PointContainmentState,
        polygon: ValidatedPolygon,
    ) -> Result<Self, ContainmentError> {
        if state.schema != CHECKPOINT_SCHEMA
            || state.polygon_fingerprint != polygon.fingerprint
            || !state.point.0.is_finite()
            || !state.point.1.is_finite()
            || state.edge_index > polygon.points.len()
        {
            return Err(ContainmentError::InvalidCheckpoint);
        }
        match state.result {
            None if state.edge_index < polygon.points.len() => {}
            Some(false) if state.edge_index == polygon.points.len() && !state.inside => {}
            Some(true) if state.edge_index > 0 => {}
            _ => return Err(ContainmentError::InvalidCheckpoint),
        }
        let checkpoint = PointContainmentCheckpoint {
            schema: state.schema,
            polygon: Arc::clone(&polygon.points),
            point: state.point,
            edge_index: state.edge_index,
            inside: state.inside,
            result: state.result,
        };
        Ok(Self {
            state: checkpoint,
            polygon,
        })
    }

    pub(crate) fn result(&self) -> Option<bool> {
        self.state.result
    }

    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || self.state.result.is_some() {
            return AdvanceResult {
                work_used: 0,
                done: self.state.result.is_some(),
            };
        }

        let mut work_used = 0;
        while work_used < budget && self.state.result.is_none() {
            let current = self.state.edge_index;
            let previous = if current == 0 {
                self.polygon.points.len() - 1
            } else {
                current - 1
            };
            let (ax, az) = self.polygon.points[previous];
            let (bx, bz) = self.polygon.points[current];
            let (px, pz) = self.state.point;

            // Keep this expression and the boundary checks in the same order
            // as diagnostic_point_in_face in face_access_exact_tests.rs.
            let cross = (bx - ax) * (pz - az) - (bz - az) * (px - ax);
            let on_boundary = cross.abs() <= 1.0e-4
                && px >= ax.min(bx) - 1.0e-4
                && px <= ax.max(bx) + 1.0e-4
                && pz >= az.min(bz) - 1.0e-4
                && pz <= az.max(bz) + 1.0e-4;
            self.state.edge_index += 1;
            if on_boundary {
                self.state.result = Some(true);
            } else if (az > pz) != (bz > pz) && px < (bx - ax) * (pz - az) / (bz - az) + ax {
                self.state.inside = !self.state.inside;
            }

            if self.state.result.is_none() && self.state.edge_index == self.polygon.points.len() {
                self.state.result = Some(self.state.inside);
            }
            work_used += 1;
        }

        AdvanceResult {
            work_used,
            done: self.state.result.is_some(),
        }
    }
}

fn validate_polygon(points: &[(f32, f32)]) -> Result<(), ContainmentError> {
    if points.len() < 3 || points.len() > MAX_POLYGON_VERTICES {
        return Err(ContainmentError::InvalidPolygon);
    }
    if points.iter().any(|(x, z)| !x.is_finite() || !z.is_finite()) {
        return Err(ContainmentError::NonFiniteInput);
    }
    Ok(())
}

fn polygon_fingerprint(points: &[(f32, f32)]) -> u64 {
    points
        .iter()
        .fold(polygon_fingerprint_start(points.len()), |hash, &point| {
            polygon_fingerprint_add(hash, point)
        })
}

pub(crate) fn polygon_fingerprint_start(len: usize) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in (len as u64).to_le_bytes() {
        hash ^= u64::from(value);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

pub(crate) fn polygon_fingerprint_add(mut hash: u64, point: (f32, f32)) -> u64 {
    for value in point
        .0
        .to_bits()
        .to_le_bytes()
        .into_iter()
        .chain(point.1.to_bits().to_le_bytes())
    {
        hash ^= u64::from(value);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

fn validate_checkpoint(state: &PointContainmentCheckpoint) -> Result<(), ContainmentError> {
    if state.schema != CHECKPOINT_SCHEMA
        || !state.point.0.is_finite()
        || !state.point.1.is_finite()
        || validate_polygon(&state.polygon).is_err()
        || state.edge_index > state.polygon.len()
    {
        return Err(ContainmentError::InvalidCheckpoint);
    }
    match state.result {
        None if state.edge_index < state.polygon.len() => Ok(()),
        None => Err(ContainmentError::InvalidCheckpoint),
        Some(false) if state.edge_index == state.polygon.len() && !state.inside => Ok(()),
        Some(true) if state.edge_index > 0 => Ok(()),
        _ => Err(ContainmentError::InvalidCheckpoint),
    }
}

mod arc_slice_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::sync::Arc;

    pub(super) fn serialize<S>(value: &Arc<[(f32, f32)]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_ref().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[(f32, f32)]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let points = Vec::<(f32, f32)>::deserialize(deserializer)?;
        Ok(Arc::from(points))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oracle(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
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

    fn run_with_quota(
        point: (f32, f32),
        points: &[(f32, f32)],
        quota: usize,
        checkpoint_every_step: bool,
    ) -> bool {
        let polygon = ValidatedPolygon::new(points.to_vec()).unwrap();
        let mut query = PointContainment::new(point, polygon).unwrap();
        if quota == 0 {
            let result = query.advance(0);
            assert_eq!(result.work_used, 0);
            assert!(!result.done);
        }
        for _ in 0..(points.len() + 1) {
            let result = query.advance(quota);
            assert!(result.work_used <= quota);
            if result.done {
                break;
            }
            assert!(result.work_used > 0);
            if checkpoint_every_step {
                let bytes = serde_json::to_vec(&query.checkpoint()).unwrap();
                let state = serde_json::from_slice(&bytes).unwrap();
                query = PointContainment::restore(state).unwrap();
            }
        }
        query.result().expect("query completes")
    }

    #[test]
    fn concave_rotated_and_boundary_points_match_frozen_oracle() {
        let rotated = [
            (4.0, 1.0),
            (7.0, 4.0),
            (5.0, 7.0),
            (3.4, 5.4),
            (2.0, 7.0),
            (1.0, 3.0),
        ];
        let cases = [
            ((4.0, 4.0), rotated.as_slice()),
            ((6.8, 6.2), rotated.as_slice()),
            ((4.0, 1.0), rotated.as_slice()),
            ((3.0, 5.0), rotated.as_slice()),
        ];
        for (point, polygon) in cases {
            let expected = oracle(point, polygon);
            for quota in [1, 7, 48] {
                assert_eq!(run_with_quota(point, polygon, quota, true), expected);
            }
        }
    }

    #[test]
    fn quota_zero_and_terminal_calls_do_no_work() {
        let points = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        let polygon = ValidatedPolygon::new(points.to_vec()).unwrap();
        let mut query = PointContainment::new((2.0, 2.0), polygon).unwrap();
        assert_eq!(
            query.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        while !query.advance(2).done {}
        assert_eq!(query.result(), Some(true));
        assert_eq!(
            query.advance(7),
            AdvanceResult {
                work_used: 0,
                done: true
            }
        );
    }

    #[test]
    fn validation_and_forged_cursor_fail_closed() {
        assert_eq!(
            ValidatedPolygon::new(vec![]).err(),
            Some(ContainmentError::InvalidPolygon)
        );
        assert_eq!(
            ValidatedPolygon::new(vec![(0.0, 0.0), (1.0, 0.0), (f32::NAN, 1.0)]).err(),
            Some(ContainmentError::NonFiniteInput)
        );
        let polygon = ValidatedPolygon::new(vec![(0.0, 0.0), (2.0, 0.0), (0.0, 2.0)]).unwrap();
        assert!(matches!(
            PointContainment::new((f32::INFINITY, 0.0), polygon),
            Err(ContainmentError::NonFiniteInput)
        ));

        let polygon = ValidatedPolygon::new(vec![(0.0, 0.0), (2.0, 0.0), (0.0, 2.0)]).unwrap();
        let query = PointContainment::new((0.5, 0.5), polygon).unwrap();
        let mut checkpoint = query.checkpoint();
        checkpoint.edge_index = checkpoint.polygon.len() + 1;
        assert_eq!(
            PointContainment::restore(checkpoint).err(),
            Some(ContainmentError::InvalidCheckpoint)
        );

        let polygon = ValidatedPolygon::new(vec![(0.0, 0.0), (2.0, 0.0), (0.0, 2.0)]).unwrap();
        let query = PointContainment::new((3.0, 3.0), polygon).unwrap();
        let mut checkpoint = query.checkpoint();
        checkpoint.edge_index = checkpoint.polygon.len();
        assert_eq!(
            PointContainment::restore(checkpoint).err(),
            Some(ContainmentError::InvalidCheckpoint)
        );
    }
}
