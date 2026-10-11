//! Resumable sampling of viable grid centers inside one traced road face.
//!
//! Bounds/containment/corner scans use the frozen face polygon and viability
//! queries share one frozen world projection. A checkpoint serializes that
//! polygon once; nested query state stores only cursors and fingerprints.

use crate::face_target_containment::{
    ContainmentError, PointContainment, PointContainmentState, ValidatedPolygon,
};
use crate::face_target_viability::{
    FrozenViabilityContext, ViabilityCheckpoint, ViabilityError, ViabilityQuery,
};
use crate::geometry::OrientedRect;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_FACE_VERTICES: usize = 1024;
const GRID_SPACING_M: f32 = 8.0;
const GRID_MAX_SIDE: usize = 24;
const NOMINAL_SITE_SIZE_M: f32 = 8.0;
const MAX_FACE_COORDINATE: f32 = 100_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SamplingError {
    InvalidPolygon,
    InvalidCheckpoint,
    Containment(ContainmentError),
    Viability(ViabilityError),
    InvalidRuntimeState,
}

impl From<ContainmentError> for SamplingError {
    fn from(value: ContainmentError) -> Self {
        Self::Containment(value)
    }
}

impl From<ViabilityError> for SamplingError {
    fn from(value: ViabilityError) -> Self {
        Self::Viability(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Bounds,
    GridCell,
    CenterPoint,
    CornerStart,
    CornerPoint,
    ViabilityStart,
    Viability,
    NextRotation,
    Accept,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SamplerState {
    phase: Phase,
    bounds_cursor: usize,
    min_x: Option<f32>,
    min_z: Option<f32>,
    max_x: Option<f32>,
    max_z: Option<f32>,
    columns: usize,
    rows: usize,
    cell_index: usize,
    center: (f32, f32),
    rect: OrientedRect,
    rotation_index: u8,
    corner_index: u8,
    point_query: Option<PointContainmentState>,
    viability_query: Option<ViabilityCheckpoint>,
    centers: Vec<(f32, f32)>,
    failed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FaceSamplerCheckpoint {
    schema: u8,
    polygon: Vec<(f32, f32)>,
    polygon_fingerprint: u64,
    context_fingerprint: u64,
    state: SamplerState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdvanceResult {
    pub work_used: usize,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct FaceSampler {
    polygon: ValidatedPolygon,
    context: Arc<FrozenViabilityContext>,
    state: SamplerState,
}

impl FaceSampler {
    pub(crate) fn new(
        polygon: Vec<(f32, f32)>,
        context: Arc<FrozenViabilityContext>,
    ) -> Result<Self, SamplingError> {
        if polygon.len() < 3 || polygon.len() > MAX_FACE_VERTICES {
            return Err(SamplingError::InvalidPolygon);
        }
        if polygon
            .iter()
            .any(|(x, z)| x.abs() > MAX_FACE_COORDINATE || z.abs() > MAX_FACE_COORDINATE)
        {
            return Err(SamplingError::InvalidPolygon);
        }
        let polygon = ValidatedPolygon::new(polygon)?;
        Self::with_polygon(polygon, context)
    }

    /// Start sampling a polygon that was already validated while a bounded
    /// caller-owned preparation stage copied it. This avoids rescanning each
    /// face synchronously at the handoff boundary.
    pub(crate) fn with_polygon(
        polygon: ValidatedPolygon,
        context: Arc<FrozenViabilityContext>,
    ) -> Result<Self, SamplingError> {
        if polygon.points().len() < 3 || polygon.points().len() > MAX_FACE_VERTICES {
            return Err(SamplingError::InvalidPolygon);
        }
        Ok(Self {
            polygon,
            context,
            state: SamplerState {
                phase: Phase::Bounds,
                bounds_cursor: 0,
                min_x: None,
                min_z: None,
                max_x: None,
                max_z: None,
                columns: 0,
                rows: 0,
                cell_index: 0,
                center: (0.0, 0.0),
                rect: OrientedRect {
                    x: 0.0,
                    z: 0.0,
                    width: NOMINAL_SITE_SIZE_M,
                    depth: NOMINAL_SITE_SIZE_M,
                    rotation: 0.0,
                },
                rotation_index: 0,
                corner_index: 0,
                point_query: None,
                viability_query: None,
                centers: Vec::new(),
                failed: false,
            },
        })
    }

    pub(crate) fn restore(
        checkpoint: FaceSamplerCheckpoint,
        context: Arc<FrozenViabilityContext>,
    ) -> Result<Self, SamplingError> {
        if checkpoint.schema != CHECKPOINT_SCHEMA
            || checkpoint.context_fingerprint != context.fingerprint()
        {
            return Err(SamplingError::InvalidCheckpoint);
        }
        let polygon = ValidatedPolygon::new(checkpoint.polygon)
            .map_err(|_| SamplingError::InvalidCheckpoint)?;
        if checkpoint.polygon_fingerprint != polygon.fingerprint() {
            return Err(SamplingError::InvalidCheckpoint);
        }
        validate_state(&checkpoint.state, &polygon)?;
        if let Some(point_query) = checkpoint.state.point_query.clone() {
            PointContainment::restore_state(point_query, polygon.clone())?;
        }
        if let Some(viability_query) = checkpoint.state.viability_query.clone() {
            ViabilityQuery::restore(viability_query, context.clone())?;
        }
        Ok(Self {
            polygon,
            context,
            state: checkpoint.state,
        })
    }

    /// The polygon is copied once at this explicit checkpoint boundary. The
    /// nested containment cursor references it by fingerprint, not by copy.
    pub(crate) fn checkpoint(&self) -> FaceSamplerCheckpoint {
        FaceSamplerCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            polygon: self.polygon.checkpoint_points(),
            polygon_fingerprint: self.polygon.fingerprint(),
            context_fingerprint: self.context.fingerprint(),
            state: self.state.clone(),
        }
    }

    pub(crate) fn centers(&self) -> Option<&[(f32, f32)]> {
        (self.state.phase == Phase::Done && !self.state.failed).then_some(&self.state.centers)
    }

    pub(crate) fn into_centers(self) -> Option<Vec<(f32, f32)>> {
        (self.state.phase == Phase::Done && !self.state.failed).then_some(self.state.centers)
    }

    pub(crate) fn error(&self) -> Option<SamplingError> {
        self.state
            .failed
            .then_some(SamplingError::InvalidRuntimeState)
    }

    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || self.state.phase == Phase::Done {
            return AdvanceResult {
                work_used: 0,
                done: self.state.phase == Phase::Done,
            };
        }
        self.settle_completed_queries();
        let mut work_used = 0;
        while work_used < budget && self.state.phase != Phase::Done {
            match self.state.phase {
                Phase::Bounds => {
                    let (x, z) = self.polygon.points()[self.state.bounds_cursor];
                    self.state.min_x = Some(self.state.min_x.map_or(x, |prior| prior.min(x)));
                    self.state.min_z = Some(self.state.min_z.map_or(z, |prior| prior.min(z)));
                    self.state.max_x = Some(self.state.max_x.map_or(x, |prior| prior.max(x)));
                    self.state.max_z = Some(self.state.max_z.map_or(z, |prior| prior.max(z)));
                    self.state.bounds_cursor += 1;
                    work_used += 1;
                    if self.state.bounds_cursor == self.polygon.points().len() {
                        self.finish_bounds();
                    }
                }
                Phase::GridCell => {
                    let total = self.state.rows * self.state.columns;
                    if self.state.cell_index >= total {
                        self.state.phase = Phase::Done;
                        break;
                    }
                    let row = self.state.cell_index / self.state.columns;
                    let column = self.state.cell_index % self.state.columns;
                    let min_x = self.state.min_x.expect("finished face bounds");
                    let max_x = self.state.max_x.expect("finished face bounds");
                    let min_z = self.state.min_z.expect("finished face bounds");
                    let max_z = self.state.max_z.expect("finished face bounds");
                    let center = (
                        min_x + (column as f32 + 0.5) * (max_x - min_x) / self.state.columns as f32,
                        min_z + (row as f32 + 0.5) * (max_z - min_z) / self.state.rows as f32,
                    );
                    self.state.center = center;
                    self.state.rect = OrientedRect {
                        x: center.0,
                        z: center.1,
                        width: NOMINAL_SITE_SIZE_M,
                        depth: NOMINAL_SITE_SIZE_M,
                        rotation: 0.0,
                    };
                    self.state.rotation_index = 0;
                    self.state.corner_index = 0;
                    match PointContainment::new(center, self.polygon.clone()) {
                        Ok(query) => {
                            self.state.point_query = Some(query.checkpoint_state());
                            self.state.phase = Phase::CenterPoint;
                        }
                        Err(_) => self.fail(),
                    }
                    work_used += 1;
                }
                Phase::CenterPoint | Phase::CornerPoint => {
                    let remaining = budget - work_used;
                    let Some(point_state) = self.state.point_query.take() else {
                        self.fail();
                        continue;
                    };
                    let Ok(mut query) =
                        PointContainment::restore_state(point_state, self.polygon.clone())
                    else {
                        self.fail();
                        continue;
                    };
                    let step = query.advance(remaining);
                    work_used += step.work_used;
                    self.state.point_query = Some(query.checkpoint_state());
                    self.settle_completed_queries();
                }
                Phase::CornerStart => {
                    let corner = self.state.rect.corners()[self.state.corner_index as usize];
                    match PointContainment::new(corner, self.polygon.clone()) {
                        Ok(query) => {
                            self.state.point_query = Some(query.checkpoint_state());
                            self.state.phase = Phase::CornerPoint;
                        }
                        Err(_) => self.fail(),
                    }
                    work_used += 1;
                }
                Phase::ViabilityStart => {
                    match ViabilityQuery::new(self.state.rect, self.context.clone()) {
                        Ok(query) => {
                            self.state.viability_query = Some(query.checkpoint());
                            self.state.phase = Phase::Viability;
                        }
                        Err(_) => self.fail(),
                    }
                    work_used += 1;
                }
                Phase::Viability => {
                    let remaining = budget - work_used;
                    let Some(viability_state) = self.state.viability_query.take() else {
                        self.fail();
                        continue;
                    };
                    let Ok(mut query) =
                        ViabilityQuery::restore(viability_state, self.context.clone())
                    else {
                        self.fail();
                        continue;
                    };
                    let step = query.advance(remaining);
                    work_used += step.work_used;
                    self.state.viability_query = Some(query.checkpoint());
                    self.settle_completed_queries();
                }
                Phase::NextRotation => {
                    self.state.rotation_index = 1;
                    self.state.rect.rotation = std::f32::consts::FRAC_PI_2;
                    self.state.corner_index = 0;
                    self.state.phase = Phase::CornerStart;
                    work_used += 1;
                }
                Phase::Accept => {
                    self.state.centers.push(self.state.center);
                    self.state.cell_index += 1;
                    self.state.phase = Phase::GridCell;
                    work_used += 1;
                }
                Phase::Done => break,
            }
            self.settle_completed_queries();
        }
        self.settle_completed_queries();
        AdvanceResult {
            work_used,
            done: self.state.phase == Phase::Done,
        }
    }

    fn finish_bounds(&mut self) {
        let min_x = self.state.min_x.expect("nonempty polygon");
        let max_x = self.state.max_x.expect("nonempty polygon");
        let min_z = self.state.min_z.expect("nonempty polygon");
        let max_z = self.state.max_z.expect("nonempty polygon");
        self.state.columns =
            (((max_x - min_x) / GRID_SPACING_M).ceil() as usize).clamp(1, GRID_MAX_SIDE);
        self.state.rows =
            (((max_z - min_z) / GRID_SPACING_M).ceil() as usize).clamp(1, GRID_MAX_SIDE);
        self.state.phase = Phase::GridCell;
    }

    fn fail(&mut self) {
        self.state.phase = Phase::Done;
        self.state.failed = true;
        self.state.cell_index = self.state.columns * self.state.rows;
        self.state.point_query = None;
        self.state.viability_query = None;
    }

    /// Complete only already-finished nested queries and bounded enum/cursor
    /// transitions; this never scans a polygon or context array.
    fn settle_completed_queries(&mut self) {
        loop {
            match self.state.phase {
                Phase::GridCell
                    if self.state.columns > 0
                        && self.state.rows > 0
                        && self.state.cell_index >= self.state.columns * self.state.rows =>
                {
                    self.state.phase = Phase::Done;
                }
                Phase::CenterPoint
                    if self
                        .state
                        .point_query
                        .as_ref()
                        .is_some_and(|query| query.result().is_some()) =>
                {
                    let inside = self.state.point_query.take().unwrap().result().unwrap();
                    if inside {
                        self.state.rect.rotation = 0.0;
                        self.state.rotation_index = 0;
                        self.state.corner_index = 0;
                        self.state.phase = Phase::CornerStart;
                    } else {
                        self.state.cell_index += 1;
                        self.state.phase = Phase::GridCell;
                    }
                }
                Phase::CornerPoint
                    if self
                        .state
                        .point_query
                        .as_ref()
                        .is_some_and(|query| query.result().is_some()) =>
                {
                    let inside = self.state.point_query.take().unwrap().result().unwrap();
                    if !inside {
                        self.state.phase = if self.state.rotation_index == 0 {
                            Phase::NextRotation
                        } else {
                            self.state.cell_index += 1;
                            Phase::GridCell
                        };
                    } else {
                        self.state.corner_index += 1;
                        self.state.phase = if self.state.corner_index >= 4 {
                            Phase::ViabilityStart
                        } else {
                            Phase::CornerStart
                        };
                    }
                }
                Phase::Viability
                    if self
                        .state
                        .viability_query
                        .as_ref()
                        .is_some_and(|query| query.result().is_some()) =>
                {
                    let viable = self.state.viability_query.take().unwrap().result().unwrap();
                    if viable {
                        self.state.phase = Phase::Accept;
                    } else {
                        self.state.phase = if self.state.rotation_index == 0 {
                            Phase::NextRotation
                        } else {
                            self.state.cell_index += 1;
                            Phase::GridCell
                        };
                    }
                }
                _ => break,
            }
        }
    }
}

fn validate_state(state: &SamplerState, polygon: &ValidatedPolygon) -> Result<(), SamplingError> {
    let count = polygon.points().len();
    if state.bounds_cursor > count
        || state.centers.len() > GRID_MAX_SIDE * GRID_MAX_SIDE
        || state.rotation_index > 1
        || state.corner_index > 4
        || !state.center.0.is_finite()
        || !state.center.1.is_finite()
        || !state.rect.finite_positive()
        || state
            .centers
            .iter()
            .any(|(x, z)| !x.is_finite() || !z.is_finite())
        || [state.min_x, state.min_z, state.max_x, state.max_z]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err(SamplingError::InvalidCheckpoint);
    }
    if state.phase == Phase::Bounds {
        if state.columns != 0
            || state.rows != 0
            || state.cell_index != 0
            || state.bounds_cursor == count
            || (state.bounds_cursor == 0)
                != (state.min_x.is_none()
                    && state.min_z.is_none()
                    && state.max_x.is_none()
                    && state.max_z.is_none())
        {
            return Err(SamplingError::InvalidCheckpoint);
        }
        for &(x, z) in polygon.points().iter().take(state.bounds_cursor) {
            if state.min_x.is_some_and(|v| x < v)
                || state.max_x.is_some_and(|v| x > v)
                || state.min_z.is_some_and(|v| z < v)
                || state.max_z.is_some_and(|v| z > v)
            {
                return Err(SamplingError::InvalidCheckpoint);
            }
        }
    } else if state.bounds_cursor != count
        || state.columns == 0
        || state.columns > GRID_MAX_SIDE
        || state.rows == 0
        || state.rows > GRID_MAX_SIDE
        || state.cell_index > state.columns * state.rows
        || state.min_x.is_none()
        || state.min_z.is_none()
        || state.max_x.is_none()
        || state.max_z.is_none()
        || state.min_x.unwrap() > state.max_x.unwrap()
        || state.min_z.unwrap() > state.max_z.unwrap()
    {
        return Err(SamplingError::InvalidCheckpoint);
    }

    let needs_point = matches!(state.phase, Phase::CenterPoint | Phase::CornerPoint);
    let needs_viability = state.phase == Phase::Viability;
    if state.point_query.is_some() != needs_point
        || state.viability_query.is_some() != needs_viability
    {
        return Err(SamplingError::InvalidCheckpoint);
    }
    if state.phase == Phase::CenterPoint && state.rotation_index != 0 {
        return Err(SamplingError::InvalidCheckpoint);
    }
    match state.phase {
        Phase::CornerStart if state.corner_index >= 4 => {
            return Err(SamplingError::InvalidCheckpoint)
        }
        Phase::CornerPoint if state.corner_index >= 4 => {
            return Err(SamplingError::InvalidCheckpoint)
        }
        Phase::ViabilityStart | Phase::Viability | Phase::Accept if state.corner_index != 4 => {
            return Err(SamplingError::InvalidCheckpoint)
        }
        Phase::GridCell if state.cell_index >= state.columns * state.rows => {
            return Err(SamplingError::InvalidCheckpoint)
        }
        _ => {}
    }
    if matches!(state.phase, Phase::Done)
        && state.cell_index != state.columns * state.rows
        && !state.failed
    {
        return Err(SamplingError::InvalidCheckpoint);
    }
    if state.failed
        && (state.phase != Phase::Done
            || state.point_query.is_some()
            || state.viability_query.is_some())
    {
        return Err(SamplingError::InvalidCheckpoint);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face_target_viability::{FrozenViabilityInput, Segment};

    fn context() -> Arc<FrozenViabilityContext> {
        FrozenViabilityContext::new(FrozenViabilityInput {
            parcel_footprints: vec![],
            reserved_lots: vec![],
            open_roads: vec![],
            pending_roads: vec![],
            river_segments: vec![],
            neighborhood_segments: vec![],
            neighborhood_interior: None,
        })
        .unwrap()
    }

    fn point_in_face(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
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

    fn expected_centers(polygon: &[(f32, f32)]) -> Vec<(f32, f32)> {
        let min_x = polygon.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = polygon
            .iter()
            .map(|p| p.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_z = polygon.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_z = polygon
            .iter()
            .map(|p| p.1)
            .fold(f32::NEG_INFINITY, f32::max);
        let columns = (((max_x - min_x) / GRID_SPACING_M).ceil() as usize).clamp(1, GRID_MAX_SIDE);
        let rows = (((max_z - min_z) / GRID_SPACING_M).ceil() as usize).clamp(1, GRID_MAX_SIDE);
        let mut centers = Vec::new();
        for row in 0..rows {
            for column in 0..columns {
                let center = (
                    min_x + (column as f32 + 0.5) * (max_x - min_x) / columns as f32,
                    min_z + (row as f32 + 0.5) * (max_z - min_z) / rows as f32,
                );
                if !point_in_face(center, polygon) {
                    continue;
                }
                for rotation in [0.0, std::f32::consts::FRAC_PI_2] {
                    let rect = OrientedRect {
                        x: center.0,
                        z: center.1,
                        width: NOMINAL_SITE_SIZE_M,
                        depth: NOMINAL_SITE_SIZE_M,
                        rotation,
                    };
                    if rect
                        .corners()
                        .into_iter()
                        .all(|corner| point_in_face(corner, polygon))
                    {
                        centers.push(center);
                        break;
                    }
                }
            }
        }
        centers
    }

    fn run(polygon: Vec<(f32, f32)>, quota: usize) -> (Vec<(f32, f32)>, usize) {
        let context = context();
        let mut sampler = FaceSampler::new(polygon, context.clone()).unwrap();
        assert_eq!(
            sampler.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        let mut total_work = 0;
        for _ in 0..2_000_000 {
            let step = sampler.advance(quota);
            assert!(step.work_used <= quota);
            total_work += step.work_used;
            if step.done {
                break;
            }
            assert!(step.work_used > 0, "sampler must make bounded progress");
            let bytes = serde_json::to_vec(&sampler.checkpoint()).unwrap();
            let checkpoint = serde_json::from_slice(&bytes).unwrap();
            sampler = FaceSampler::restore(checkpoint, context.clone()).unwrap();
        }
        let centers = sampler.centers().expect("sampler completes").to_vec();
        assert_eq!(
            sampler.advance(quota),
            AdvanceResult {
                work_used: 0,
                done: true
            }
        );
        (centers, total_work)
    }

    #[test]
    fn concave_face_centers_match_reference_order_at_multiple_quotas() {
        let polygon = vec![
            (-5.0, -5.0),
            (35.0, -5.0),
            (35.0, 35.0),
            (20.0, 35.0),
            (20.0, 10.0),
            (10.0, 10.0),
            (10.0, 35.0),
            (-5.0, 35.0),
        ];
        let expected = expected_centers(&polygon);
        let mut prior_work = None;
        for quota in [1, 7, 48] {
            let (centers, work) = run(polygon.clone(), quota);
            assert_eq!(centers, expected);
            if let Some(previous) = prior_work {
                assert_eq!(work, previous);
            }
            prior_work = Some(work);
        }
        assert!(!expected.is_empty());
    }

    #[test]
    fn checkpoint_stores_one_polygon_and_rejects_bad_cursors() {
        let polygon = vec![(-20.0, -20.0), (50.0, -20.0), (50.0, 50.0), (-20.0, 50.0)];
        let context = context();
        let mut sampler = FaceSampler::new(polygon.clone(), context.clone()).unwrap();
        sampler.advance(2);
        let json = serde_json::to_value(sampler.checkpoint()).unwrap();
        assert_eq!(json["polygon"].as_array().unwrap().len(), polygon.len());
        assert!(json["state"]["point_query"].get("polygon").is_none());
        assert!(json["state"]["viability_query"].is_null());

        let mut checkpoint = sampler.checkpoint();
        checkpoint.state.bounds_cursor = usize::MAX;
        assert_eq!(
            FaceSampler::restore(checkpoint, context).err(),
            Some(SamplingError::InvalidCheckpoint)
        );
    }

    #[test]
    fn viability_projection_rejects_each_candidate_without_changing_sampling_order() {
        let polygon = vec![(-10.0, -10.0), (50.0, -10.0), (50.0, 50.0), (-10.0, 50.0)];
        let blocked = FrozenViabilityContext::new(FrozenViabilityInput {
            parcel_footprints: vec![OrientedRect {
                x: 20.0,
                z: 20.0,
                width: 100.0,
                depth: 100.0,
                rotation: 0.0,
            }],
            reserved_lots: vec![],
            open_roads: vec![],
            pending_roads: vec![],
            river_segments: vec![],
            neighborhood_segments: vec![],
            neighborhood_interior: None,
        })
        .unwrap();
        let mut sampler = FaceSampler::new(polygon, blocked).unwrap();
        while !sampler.advance(7).done {}
        assert!(sampler.centers().unwrap().is_empty());

        // Keep imports exercised here to make explicit that segment-bearing
        // projections are accepted and remain in their frozen input order.
        let _sample_context = FrozenViabilityContext::new(FrozenViabilityInput {
            parcel_footprints: vec![],
            reserved_lots: vec![],
            open_roads: vec![Segment {
                ax: -5.0,
                az: -5.0,
                bx: -5.0,
                bz: 5.0,
            }],
            pending_roads: vec![],
            river_segments: vec![],
            neighborhood_segments: vec![],
            neighborhood_interior: None,
        })
        .unwrap();
    }
}
