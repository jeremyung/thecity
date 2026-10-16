//! Quota-stepped frozen equivalent of progressive frontage geometry checks.
//!
//! Context arrays retain the source order from `Engine::collision_context`.
//! Each variable-length geometry entry is one deterministic work unit.

use crate::geometry::{self, OrientedRect};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 1;
const MAX_ABS_COORDINATE: f32 = 100_000.0;
const PROTECTED_LAND: OrientedRect = OrientedRect {
    x: -305.0,
    z: -310.0,
    width: 90.0,
    depth: 80.0,
    rotation: 0.0,
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Segment {
    pub ax: f32,
    pub az: f32,
    pub bx: f32,
    pub bz: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct FrozenViabilityInput {
    pub parcel_footprints: Vec<OrientedRect>,
    pub reserved_lots: Vec<OrientedRect>,
    pub open_roads: Vec<Segment>,
    pub pending_roads: Vec<Segment>,
    pub river_segments: Vec<Segment>,
    /// The already-sliced `NeighborhoodPlan.segments.skip(next_segment)`.
    pub neighborhood_segments: Vec<Segment>,
    /// Present only when the live coordinated plan produced its interior rect.
    pub neighborhood_interior: Option<OrientedRect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViabilityError {
    InputTooLarge,
    NonFiniteInput,
    InvalidCheckpoint,
}

/// Validated immutable projection shared by all candidate rectangle queries.
#[derive(Clone, Debug)]
pub(crate) struct FrozenViabilityContext {
    input: Arc<FrozenViabilityInput>,
    fingerprint: u64,
}

impl FrozenViabilityContext {
    pub(crate) fn new(input: FrozenViabilityInput) -> Result<Arc<Self>, ViabilityError> {
        validate_input(&input)?;
        let fingerprint = fingerprint(&input);
        Ok(Arc::new(Self {
            input: Arc::new(input),
            fingerprint,
        }))
    }

    /// Explicit persistence boundary for the shared frozen projection.
    pub(crate) fn checkpoint(&self) -> FrozenViabilityInput {
        (*self.input).clone()
    }

    /// Borrow the immutable captured inputs for downstream stages that must
    /// make quota-accounted copies into their own owned search state.
    pub(crate) fn input(&self) -> &FrozenViabilityInput {
        &self.input
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    /// Restoring a context is the only operation that revalidates its arrays.
    pub(crate) fn restore(input: FrozenViabilityInput) -> Result<Arc<Self>, ViabilityError> {
        Self::new(input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    River,
    BoundsAndProtected,
    Parcels,
    Reservations,
    OpenRoadRect,
    PendingRoadRect,
    NeighborhoodInterior,
    NeighborhoodSegments,
    OpenRoadCenter,
    PendingRoadCenter,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ViabilityCheckpoint {
    schema: u8,
    context_fingerprint: u64,
    rect: OrientedRect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extra_road: Option<Segment>,
    phase: Phase,
    cursor: usize,
    river_min_distance: Option<f32>,
    result: Option<bool>,
}

impl ViabilityCheckpoint {
    pub(crate) fn matches_query(&self, rect: OrientedRect, extra_road: Option<Segment>) -> bool {
        self.rect == rect && self.extra_road == extra_road
    }

    #[cfg(feature = "legacy")]
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
pub(crate) struct ViabilityQuery {
    state: ViabilityCheckpoint,
    context: Arc<FrozenViabilityContext>,
}

impl ViabilityQuery {
    pub(crate) fn new(
        rect: OrientedRect,
        context: Arc<FrozenViabilityContext>,
    ) -> Result<Self, ViabilityError> {
        let invalid_rect = !rect.finite_positive();
        let state = ViabilityCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            context_fingerprint: context.fingerprint,
            rect,
            extra_road: None,
            phase: if invalid_rect {
                Phase::Done
            } else {
                Phase::River
            },
            cursor: 0,
            river_min_distance: None,
            result: invalid_rect.then_some(false),
        };
        Ok(Self { state, context })
    }

    /// Treat one proposed branch as the final open-road entry without copying
    /// the shared world geometry. It participates in both road clearance passes.
    pub(crate) fn new_with_extra_road(
        rect: OrientedRect,
        context: Arc<FrozenViabilityContext>,
        road: Segment,
    ) -> Result<Self, ViabilityError> {
        if !valid_segment(road) {
            return Err(ViabilityError::NonFiniteInput);
        }
        let mut query = Self::new(rect, context)?;
        query.state.extra_road = Some(road);
        Ok(query)
    }

    fn open_road_count(&self) -> usize {
        self.context.input.open_roads.len() + usize::from(self.state.extra_road.is_some())
    }

    fn open_road(&self, index: usize) -> Segment {
        self.context
            .input
            .open_roads
            .get(index)
            .copied()
            .or(self.state.extra_road)
            .expect("validated open-road cursor")
    }

    pub(crate) fn restore(
        state: ViabilityCheckpoint,
        context: Arc<FrozenViabilityContext>,
    ) -> Result<Self, ViabilityError> {
        validate_checkpoint(&state, &context)?;
        Ok(Self { state, context })
    }

    pub(crate) fn checkpoint(&self) -> ViabilityCheckpoint {
        self.state.clone()
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

        self.settle_empty_stages();
        let mut work_used = 0;
        while work_used < budget && self.state.result.is_none() {
            let rect = self.state.rect;
            match self.state.phase {
                Phase::River => {
                    let segment = self.context.input.river_segments[self.state.cursor];
                    let distance = geometry::point_segment_distance(
                        rect.x, rect.z, segment.ax, segment.az, segment.bx, segment.bz,
                    );
                    self.state.river_min_distance = Some(match self.state.river_min_distance {
                        None => distance,
                        Some(previous) if distance.total_cmp(&previous).is_lt() => distance,
                        Some(previous) => previous,
                    });
                    self.state.cursor += 1;
                    work_used += 1;
                }
                Phase::BoundsAndProtected => {
                    let (min_x, min_z, max_x, max_z) = rect.bounds();
                    let out_of_bounds = min_x < crate::MIN + 4.0
                        || min_z < crate::MIN + 4.0
                        || max_x > crate::MAX - 4.0
                        || max_z > crate::MAX - 4.0;
                    let protected_overlap = geometry::rects_overlap(&rect, &PROTECTED_LAND, 0.0);
                    self.state.phase = if out_of_bounds || protected_overlap {
                        self.finish(false);
                        Phase::Done
                    } else {
                        Phase::Parcels
                    };
                    self.state.cursor = 0;
                    work_used += 1;
                }
                Phase::Parcels => {
                    let footprint = &self.context.input.parcel_footprints[self.state.cursor];
                    let overlap = geometry::rects_overlap(&rect, footprint, 2.0);
                    self.state.cursor += 1;
                    if overlap {
                        self.finish(false);
                        self.state.phase = Phase::Done;
                    }
                    work_used += 1;
                }
                Phase::Reservations => {
                    let reserved = &self.context.input.reserved_lots[self.state.cursor];
                    let overlap = geometry::rects_overlap(&rect, reserved, 2.0);
                    self.state.cursor += 1;
                    if overlap {
                        self.finish(false);
                        self.state.phase = Phase::Done;
                    }
                    work_used += 1;
                }
                Phase::OpenRoadRect => {
                    let road = self.open_road(self.state.cursor);
                    let blocked =
                        geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz)
                            < 5.0;
                    self.state.cursor += 1;
                    if blocked {
                        self.finish(false);
                        self.state.phase = Phase::Done;
                    }
                    work_used += 1;
                }
                Phase::PendingRoadRect => {
                    let road = self.context.input.pending_roads[self.state.cursor];
                    let blocked =
                        geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz)
                            < 7.0;
                    self.state.cursor += 1;
                    if blocked {
                        self.finish(false);
                        self.state.phase = Phase::Done;
                    }
                    work_used += 1;
                }
                Phase::NeighborhoodInterior => {
                    if let Some(interior) = self.context.input.neighborhood_interior {
                        let (min_x, min_z, max_x, max_z) = rect.bounds();
                        let candidate = OrientedRect {
                            x: rect.x,
                            z: rect.z,
                            width: max_x - min_x,
                            depth: max_z - min_z,
                            rotation: 0.0,
                        };
                        if geometry::rects_overlap(&interior, &candidate, 0.0) {
                            self.finish(false);
                            self.state.phase = Phase::Done;
                        } else {
                            self.state.phase = Phase::NeighborhoodSegments;
                        }
                        self.state.cursor = 0;
                        work_used += 1;
                    } else {
                        self.state.phase = Phase::NeighborhoodSegments;
                        self.state.cursor = 0;
                    }
                }
                Phase::NeighborhoodSegments => {
                    let segment = self.context.input.neighborhood_segments[self.state.cursor];
                    let (min_x, min_z, max_x, max_z) = rect.bounds();
                    let clearance = (((max_x - min_x).max(max_z - min_z) * 0.5) + 5.0).max(9.0);
                    let blocked = geometry::point_segment_distance(
                        rect.x, rect.z, segment.ax, segment.az, segment.bx, segment.bz,
                    ) < clearance;
                    self.state.cursor += 1;
                    if blocked {
                        self.finish(false);
                        self.state.phase = Phase::Done;
                    }
                    work_used += 1;
                }
                Phase::OpenRoadCenter => {
                    let road = self.open_road(self.state.cursor);
                    let clearance = (rect.width.max(rect.depth) * 0.5 + 4.0).max(9.0);
                    let blocked = geometry::point_segment_distance(
                        rect.x, rect.z, road.ax, road.az, road.bx, road.bz,
                    ) < clearance;
                    self.state.cursor += 1;
                    if blocked {
                        self.finish(false);
                        self.state.phase = Phase::Done;
                    }
                    work_used += 1;
                }
                Phase::PendingRoadCenter => {
                    let road = self.context.input.pending_roads[self.state.cursor];
                    let clearance = (rect.width.max(rect.depth) * 0.5 + 4.0).max(9.0) + 2.0;
                    let blocked = geometry::point_segment_distance(
                        rect.x, rect.z, road.ax, road.az, road.bx, road.bz,
                    ) < clearance;
                    self.state.cursor += 1;
                    if blocked {
                        self.finish(false);
                        self.state.phase = Phase::Done;
                    }
                    work_used += 1;
                }
                Phase::Done => break,
            }
            self.settle_empty_stages();
        }
        self.settle_empty_stages();
        AdvanceResult {
            work_used,
            done: self.state.result.is_some(),
        }
    }

    fn finish(&mut self, result: bool) {
        self.state.phase = Phase::Done;
        self.state.cursor = 0;
        self.state.result = Some(result);
    }

    /// Advance only through empty-array boundaries. This has a fixed maximum
    /// number of enum transitions and performs no geometry-array scans.
    fn settle_empty_stages(&mut self) {
        loop {
            match self.state.phase {
                Phase::River if self.state.cursor == self.context.input.river_segments.len() => {
                    let distance = self.state.river_min_distance.unwrap_or(f32::INFINITY);
                    let radius = (self.state.rect.width * 0.5).hypot(self.state.rect.depth * 0.5);
                    let required = crate::RIVER_HALF_WIDTH + crate::RIVER_BANK_BUFFER + radius;
                    if !(distance >= required) {
                        self.finish(false);
                    } else {
                        self.state.phase = Phase::BoundsAndProtected;
                        self.state.cursor = 0;
                    }
                }
                Phase::Parcels
                    if self.state.cursor == self.context.input.parcel_footprints.len() =>
                {
                    self.state.phase = Phase::Reservations;
                    self.state.cursor = 0;
                }
                Phase::Reservations
                    if self.state.cursor == self.context.input.reserved_lots.len() =>
                {
                    self.state.phase = Phase::OpenRoadRect;
                    self.state.cursor = 0;
                }
                Phase::OpenRoadRect if self.state.cursor == self.open_road_count() => {
                    self.state.phase = Phase::PendingRoadRect;
                    self.state.cursor = 0;
                }
                Phase::PendingRoadRect
                    if self.state.cursor == self.context.input.pending_roads.len() =>
                {
                    self.state.phase = Phase::NeighborhoodInterior;
                    self.state.cursor = 0;
                }
                Phase::NeighborhoodSegments
                    if self.state.cursor == self.context.input.neighborhood_segments.len() =>
                {
                    self.state.phase = Phase::OpenRoadCenter;
                    self.state.cursor = 0;
                }
                Phase::OpenRoadCenter if self.state.cursor == self.open_road_count() => {
                    self.state.phase = Phase::PendingRoadCenter;
                    self.state.cursor = 0;
                }
                Phase::PendingRoadCenter
                    if self.state.cursor == self.context.input.pending_roads.len() =>
                {
                    self.finish(true);
                }
                _ => break,
            }
        }
    }
}

fn valid_segment(segment: Segment) -> bool {
    [segment.ax, segment.az, segment.bx, segment.bz]
        .iter()
        .all(|value| value.is_finite() && value.abs() <= MAX_ABS_COORDINATE)
}

fn validate_input(input: &FrozenViabilityInput) -> Result<(), ViabilityError> {
    if input.parcel_footprints.len() > crate::MAX_PARCELS.saturating_mul(2)
        || input.reserved_lots.len() > crate::MAX_PARCELS
        || input.open_roads.len() > crate::MAX_SEGMENTS
        || input.pending_roads.len() > crate::MAX_PROJECTS
        || input.river_segments.len() > 1024
        || input.neighborhood_segments.len() > 1024
    {
        return Err(ViabilityError::InputTooLarge);
    }
    if input
        .parcel_footprints
        .iter()
        .chain(input.reserved_lots.iter())
        .chain(input.neighborhood_interior.iter())
        .any(|rect| {
            !rect.finite_positive()
                || rect.x.abs() > MAX_ABS_COORDINATE
                || rect.z.abs() > MAX_ABS_COORDINATE
                || rect.width > MAX_ABS_COORDINATE
                || rect.depth > MAX_ABS_COORDINATE
        })
        || input
            .open_roads
            .iter()
            .chain(input.pending_roads.iter())
            .chain(input.river_segments.iter())
            .chain(input.neighborhood_segments.iter())
            .any(|segment| {
                !segment.ax.is_finite()
                    || !segment.az.is_finite()
                    || !segment.bx.is_finite()
                    || !segment.bz.is_finite()
                    || segment.ax.abs() > MAX_ABS_COORDINATE
                    || segment.az.abs() > MAX_ABS_COORDINATE
                    || segment.bx.abs() > MAX_ABS_COORDINATE
                    || segment.bz.abs() > MAX_ABS_COORDINATE
            })
    {
        return Err(ViabilityError::NonFiniteInput);
    }
    Ok(())
}

fn validate_checkpoint(
    state: &ViabilityCheckpoint,
    context: &FrozenViabilityContext,
) -> Result<(), ViabilityError> {
    let valid_rect = state.rect.finite_positive()
        && state.rect.x.abs() <= MAX_ABS_COORDINATE
        && state.rect.z.abs() <= MAX_ABS_COORDINATE
        && state.rect.width <= MAX_ABS_COORDINATE
        && state.rect.depth <= MAX_ABS_COORDINATE;
    let invalid = state.extra_road.is_some_and(|road| !valid_segment(road))
        || state.schema != CHECKPOINT_SCHEMA
        || state.context_fingerprint != context.fingerprint
        || (valid_rect && state.result.is_some() && state.phase != Phase::Done)
        || (!valid_rect && !(state.phase == Phase::Done && state.result == Some(false)))
        || (!valid_rect && state.cursor != 0)
        || (!valid_rect && state.river_min_distance.is_some());
    if invalid {
        return Err(ViabilityError::InvalidCheckpoint);
    }

    let limit = match state.phase {
        Phase::River => context.input.river_segments.len(),
        Phase::Parcels => context.input.parcel_footprints.len(),
        Phase::Reservations => context.input.reserved_lots.len(),
        Phase::OpenRoadRect | Phase::OpenRoadCenter => {
            context.input.open_roads.len() + usize::from(state.extra_road.is_some())
        }
        Phase::PendingRoadRect | Phase::PendingRoadCenter => context.input.pending_roads.len(),
        Phase::NeighborhoodSegments => context.input.neighborhood_segments.len(),
        Phase::BoundsAndProtected | Phase::NeighborhoodInterior | Phase::Done => {
            if state.phase == Phase::Done {
                0
            } else {
                1
            }
        }
    };
    if state.cursor > limit {
        return Err(ViabilityError::InvalidCheckpoint);
    }
    match state.phase {
        Phase::Done if state.result.is_some() => {}
        Phase::Done => return Err(ViabilityError::InvalidCheckpoint),
        _ if state.result.is_some() => return Err(ViabilityError::InvalidCheckpoint),
        _ => {}
    }
    if state
        .river_min_distance
        .is_some_and(|distance| distance.is_nan() || distance < 0.0)
        || (state.phase == Phase::River && state.river_min_distance.is_some() != (state.cursor > 0))
    {
        return Err(ViabilityError::InvalidCheckpoint);
    }
    Ok(())
}

fn fingerprint(input: &FrozenViabilityInput) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    fn add(hash: &mut u64, value: u64) {
        for byte in value.to_le_bytes() {
            *hash ^= u64::from(byte);
            *hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    }
    fn rect(hash: &mut u64, value: OrientedRect) {
        add(hash, u64::from(value.x.to_bits()));
        add(hash, u64::from(value.z.to_bits()));
        add(hash, u64::from(value.width.to_bits()));
        add(hash, u64::from(value.depth.to_bits()));
        add(hash, u64::from(value.rotation.to_bits()));
    }
    fn segments(hash: &mut u64, values: &[Segment]) {
        add(hash, values.len() as u64);
        for value in values {
            add(hash, u64::from(value.ax.to_bits()));
            add(hash, u64::from(value.az.to_bits()));
            add(hash, u64::from(value.bx.to_bits()));
            add(hash, u64::from(value.bz.to_bits()));
        }
    }

    for rects in [&input.parcel_footprints, &input.reserved_lots] {
        add(&mut hash, rects.len() as u64);
        for &value in rects {
            rect(&mut hash, value);
        }
    }
    segments(&mut hash, &input.open_roads);
    segments(&mut hash, &input.pending_roads);
    segments(&mut hash, &input.river_segments);
    segments(&mut hash, &input.neighborhood_segments);
    match input.neighborhood_interior {
        Some(value) => {
            add(&mut hash, 1);
            rect(&mut hash, value);
        }
        None => add(&mut hash, 0),
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_input() -> FrozenViabilityInput {
        FrozenViabilityInput {
            parcel_footprints: vec![],
            reserved_lots: vec![],
            open_roads: vec![],
            pending_roads: vec![],
            river_segments: vec![],
            neighborhood_segments: vec![],
            neighborhood_interior: None,
        }
    }

    fn rect(x: f32, z: f32, width: f32, depth: f32, rotation: f32) -> OrientedRect {
        OrientedRect {
            x,
            z,
            width,
            depth,
            rotation,
        }
    }

    fn seg(ax: f32, az: f32, bx: f32, bz: f32) -> Segment {
        Segment { ax, az, bx, bz }
    }

    #[test]
    fn extra_branch_matches_appended_context_with_every_unit_restore() {
        for base_roads in [vec![], vec![seg(-50.0, -40.0, 50.0, -40.0)]] {
            for branch in [seg(-30.0, 0.0, 30.0, 0.0), seg(40.0, -40.0, 40.0, 40.0)] {
                for z in [0.0, 8.0, 12.0, 15.0, 30.0] {
                    let mut input = empty_input();
                    input.open_roads = base_roads.clone();
                    let context = FrozenViabilityContext::new(input.clone()).unwrap();
                    input.open_roads.push(branch);
                    let appended = FrozenViabilityContext::new(input.clone()).unwrap();
                    let rectangle = rect(0.0, z, 12.0, 12.0, 0.35);
                    let mut extra =
                        ViabilityQuery::new_with_extra_road(rectangle, context.clone(), branch)
                            .unwrap();
                    let mut expected = ViabilityQuery::new(rectangle, appended).unwrap();
                    assert_eq!(extra.advance(0).work_used, 0);
                    loop {
                        let a = extra.advance(1);
                        let b = expected.advance(1);
                        assert_eq!(a, b);
                        assert_eq!(extra.result(), expected.result());
                        let saved = serde_json::to_string(&extra.checkpoint()).unwrap();
                        extra = ViabilityQuery::restore(
                            serde_json::from_str(&saved).unwrap(),
                            context.clone(),
                        )
                        .unwrap();
                        if a.done {
                            break;
                        }
                    }
                    assert_eq!(extra.result(), Some(oracle(rectangle, &input)));
                }
            }
        }
    }

    #[test]
    fn extra_branch_rejects_invalid_geometry_and_cursor() {
        let context = FrozenViabilityContext::new(empty_input()).unwrap();
        let rectangle = rect(0.0, 30.0, 12.0, 12.0, 0.0);
        assert!(ViabilityQuery::new_with_extra_road(
            rectangle,
            context.clone(),
            seg(f32::NAN, 0.0, 1.0, 1.0)
        )
        .is_err());
        let query = ViabilityQuery::new_with_extra_road(
            rectangle,
            context.clone(),
            seg(-30.0, 0.0, 30.0, 0.0),
        )
        .unwrap();
        let mut cp = query.checkpoint();
        cp.phase = Phase::OpenRoadRect;
        cp.cursor = 2;
        assert!(ViabilityQuery::restore(cp, context.clone()).is_err());
        let mut cp = query.checkpoint();
        cp.extra_road.as_mut().unwrap().bx = f32::INFINITY;
        assert!(ViabilityQuery::restore(cp, context).is_err());
    }

    fn oracle(rect: OrientedRect, input: &FrozenViabilityInput) -> bool {
        if !rect.finite_positive() {
            return false;
        }
        let river_distance = input
            .river_segments
            .iter()
            .map(|segment| {
                geometry::point_segment_distance(
                    rect.x, rect.z, segment.ax, segment.az, segment.bx, segment.bz,
                )
            })
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY);
        let river_required = crate::RIVER_HALF_WIDTH
            + crate::RIVER_BANK_BUFFER
            + (rect.width * 0.5).hypot(rect.depth * 0.5);
        if !(river_distance >= river_required) {
            return false;
        }
        let (min_x, min_z, max_x, max_z) = rect.bounds();
        if min_x < crate::MIN + 4.0
            || min_z < crate::MIN + 4.0
            || max_x > crate::MAX - 4.0
            || max_z > crate::MAX - 4.0
            || geometry::rects_overlap(&rect, &PROTECTED_LAND, 0.0)
        {
            return false;
        }
        if input
            .parcel_footprints
            .iter()
            .any(|footprint| geometry::rects_overlap(&rect, footprint, 2.0))
            || input
                .reserved_lots
                .iter()
                .any(|reserved| geometry::rects_overlap(&rect, reserved, 2.0))
        {
            return false;
        }
        if input.open_roads.iter().any(|road| {
            geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 5.0
        }) || input.pending_roads.iter().any(|road| {
            geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 7.0
        }) {
            return false;
        }
        if let Some(interior) = input.neighborhood_interior {
            let candidate = OrientedRect {
                x: rect.x,
                z: rect.z,
                width: max_x - min_x,
                depth: max_z - min_z,
                rotation: 0.0,
            };
            if geometry::rects_overlap(&interior, &candidate, 0.0) {
                return false;
            }
        }
        let neighborhood_clearance = (((max_x - min_x).max(max_z - min_z) * 0.5) + 5.0).max(9.0);
        if input.neighborhood_segments.iter().any(|segment| {
            geometry::point_segment_distance(
                rect.x, rect.z, segment.ax, segment.az, segment.bx, segment.bz,
            ) < neighborhood_clearance
        }) {
            return false;
        }
        let center_clearance = (rect.width.max(rect.depth) * 0.5 + 4.0).max(9.0);
        if input.open_roads.iter().any(|road| {
            geometry::point_segment_distance(rect.x, rect.z, road.ax, road.az, road.bx, road.bz)
                < center_clearance
        }) || input.pending_roads.iter().any(|road| {
            geometry::point_segment_distance(rect.x, rect.z, road.ax, road.az, road.bx, road.bz)
                < center_clearance + 2.0
        }) {
            return false;
        }
        true
    }

    fn run(rect: OrientedRect, input: FrozenViabilityInput, quota: usize) -> (bool, usize) {
        let context = FrozenViabilityContext::new(input).unwrap();
        let mut query = ViabilityQuery::new(rect, context.clone()).unwrap();
        assert_eq!(
            query.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        let mut work = 0;
        for _ in 0..50_000 {
            let step = query.advance(quota);
            assert!(step.work_used <= quota);
            work += step.work_used;
            if step.done {
                break;
            }
            assert!(
                step.work_used > 0,
                "unfinished query must make bounded progress"
            );
            let bytes = serde_json::to_vec(&query.checkpoint()).unwrap();
            query =
                ViabilityQuery::restore(serde_json::from_slice(&bytes).unwrap(), context.clone())
                    .unwrap();
        }
        let result = query.result().expect("query terminates");
        assert_eq!(
            query.advance(quota),
            AdvanceResult {
                work_used: 0,
                done: true
            }
        );
        (result, work)
    }

    #[test]
    fn frozen_predicate_matches_reference_with_resumable_json() {
        let candidate = rect(120.0, 95.0, 8.0, 10.0, 0.31);
        let base = empty_input();
        let mut cases = vec![base.clone()];

        let mut input = base.clone();
        input
            .parcel_footprints
            .push(rect(120.0, 95.0, 11.0, 11.0, 0.0));
        cases.push(input);
        let mut input = base.clone();
        input.reserved_lots.push(rect(120.0, 95.0, 11.0, 11.0, 0.0));
        cases.push(input);
        let mut input = base.clone();
        input.open_roads.push(seg(110.0, 95.0, 130.0, 95.0));
        cases.push(input);
        let mut input = base.clone();
        input.pending_roads.push(seg(110.0, 95.0, 130.0, 95.0));
        cases.push(input);
        let mut input = base.clone();
        input.river_segments.push(seg(120.0, 0.0, 120.0, 200.0));
        cases.push(input);
        let mut input = base.clone();
        input.neighborhood_interior = Some(rect(120.0, 95.0, 40.0, 40.0, 0.0));
        cases.push(input);
        let mut input = base.clone();
        input
            .neighborhood_segments
            .push(seg(100.0, 95.0, 140.0, 95.0));
        cases.push(input);
        let mut input = base.clone();
        input.open_roads.push(seg(120.0, 111.0, 120.0, 120.0));
        cases.push(input);
        let mut input = base;
        input.pending_roads.push(seg(120.0, 113.0, 120.0, 120.0));
        cases.push(input);

        for input in cases {
            let expected = oracle(candidate, &input);
            let mut reference_work = None;
            for quota in [1, 7, 48] {
                let actual = run(candidate, input.clone(), quota);
                assert_eq!(actual.0, expected);
                if let Some(previous) = reference_work {
                    assert_eq!(actual.1, previous, "work must be quota independent");
                }
                reference_work = Some(actual.1);
            }
        }
    }

    #[test]
    fn context_roundtrip_and_mismatched_context_checkpoint_are_checked() {
        let mut input = empty_input();
        input.open_roads.push(seg(-20.5, 30.25, 40.0, -12.75));
        input
            .parcel_footprints
            .push(rect(25.5, 38.25, 9.5, 10.25, 0.41));
        let context = FrozenViabilityContext::new(input).unwrap();
        let frozen = FrozenViabilityContext::restore(context.checkpoint()).unwrap();
        let mut query =
            ViabilityQuery::new(rect(120.0, 95.0, 8.0, 10.0, 0.31), context.clone()).unwrap();
        query.advance(1);
        let bytes = serde_json::to_vec(&query.checkpoint()).unwrap();
        let state: ViabilityCheckpoint = serde_json::from_slice(&bytes).unwrap();
        assert!(ViabilityQuery::restore(state.clone(), frozen).is_ok());

        let mut mismatch = empty_input();
        mismatch.open_roads.push(seg(-19.5, 30.25, 40.0, -12.75));
        let mismatch = FrozenViabilityContext::new(mismatch).unwrap();
        assert_eq!(
            ViabilityQuery::restore(state, mismatch).err(),
            Some(ViabilityError::InvalidCheckpoint)
        );

        let mut forged = query.checkpoint();
        forged.cursor = usize::MAX;
        assert_eq!(
            ViabilityQuery::restore(forged, context).err(),
            Some(ViabilityError::InvalidCheckpoint)
        );
    }

    #[test]
    fn unsafe_context_values_and_rectangles_fail_closed() {
        let mut bad = empty_input();
        bad.river_segments.push(seg(0.0, 0.0, f32::NAN, 1.0));
        assert_eq!(
            FrozenViabilityContext::new(bad).err(),
            Some(ViabilityError::NonFiniteInput)
        );

        let context = FrozenViabilityContext::new(empty_input()).unwrap();
        let mut query = ViabilityQuery::new(rect(f32::NAN, 0.0, 8.0, 8.0, 0.0), context).unwrap();
        assert_eq!(query.result(), Some(false));
        assert_eq!(
            query.advance(8),
            AdvanceResult {
                work_used: 0,
                done: true
            }
        );
    }
}
