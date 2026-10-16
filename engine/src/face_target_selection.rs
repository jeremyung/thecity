//! Streaming selection of the best viable traced face.
//!
//! Faces are visited in their input order. A face wins by viable-center count
//! (descending), area (descending), then canonical identity (ascending). Only
//! the current face and the current best centers are retained as results.

use crate::face_target_containment::{
    polygon_fingerprint_add, polygon_fingerprint_start, ValidatedPolygon,
};
#[cfg(feature = "legacy")]
use crate::face_target_sampling::AdvanceResult;
use crate::face_target_sampling::{FaceSampler, FaceSamplerCheckpoint, SamplingError};
use crate::face_target_viability::FrozenViabilityContext;
use crate::road_faces::RoadFace;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CHECKPOINT_SCHEMA: u8 = 2;
const MAX_FACES: usize = 512;
const MAX_FACE_VERTICES: usize = 1024;
const MAX_TOTAL_FACE_ITEMS: usize = 2048;
const MAX_CENTERS: usize = 24 * 24;
const MAX_COORDINATE: f32 = 100_000.0;
const MAX_EXCLUDED_IDENTITIES: usize = 512;
const MAX_EXCLUDED_IDENTITY_ITEMS: usize = 32_768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionError {
    TooManyFaces,
    TooManyExclusions,
    InvalidExclusion,
    InvalidFace,
    InvalidCheckpoint,
    Sampler(SamplingError),
    InvalidRuntimeState,
}

impl From<SamplingError> for SelectionError {
    fn from(value: SamplingError) -> Self {
        Self::Sampler(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    CheckExclusions,
    NextExclusion,
    SkipExcludedFace,
    CopyPolygon,
    StartSampler,
    Sampling,
    RankCandidate,
    CompareIdentity,
    FinishIdentity,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct State {
    phase: Phase,
    face_index: usize,
    polygon_copy: Vec<(f32, f32)>,
    polygon_fingerprint: u64,
    sampler_active: bool,
    candidate_centers: Vec<(f32, f32)>,
    identity_cursor: usize,
    excluded_index: usize,
    excluded_identity_cursor: usize,
    best_index: Option<usize>,
    best_centers: Vec<(f32, f32)>,
    failed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FaceSelectionCheckpoint {
    schema: u8,
    context_fingerprint: u64,
    faces: Vec<RoadFace>,
    excluded_identities: Vec<Vec<u32>>,
    state: State,
    active_sampler: Option<FaceSamplerCheckpoint>,
}

#[derive(Clone, Copy, Debug)]
#[cfg(feature = "legacy")]
pub(crate) struct SelectionView<'a> {
    pub face_index: usize,
    pub centers: &'a [(f32, f32)],
}

#[derive(Clone, Debug)]
pub(crate) struct FaceSelection {
    faces: Vec<RoadFace>,
    excluded_identities: Vec<Vec<u32>>,
    context: Arc<FrozenViabilityContext>,
    state: State,
    active_sampler: Option<FaceSampler>,
}

impl FaceSelection {
    #[cfg(any(test, feature = "legacy"))]
    pub(crate) fn new(
        faces: Vec<RoadFace>,
        context: Arc<FrozenViabilityContext>,
    ) -> Result<Self, SelectionError> {
        Self::new_excluding(faces, context, Vec::new())
    }

    /// Select the best viable face after excluding canonical identities.
    /// Exclusion comparisons are persisted and charged to `advance`, one
    /// identity item or page transition per work unit.
    #[cfg(feature = "legacy")]
    pub(crate) fn new_excluding(
        faces: Vec<RoadFace>,
        context: Arc<FrozenViabilityContext>,
        excluded_identities: Vec<Vec<u32>>,
    ) -> Result<Self, SelectionError> {
        if faces.len() > MAX_FACES {
            return Err(SelectionError::TooManyFaces);
        }
        validate_faces(&faces)?;
        validate_exclusions(&excluded_identities)?;
        let done = faces.is_empty();
        let has_exclusions = !excluded_identities.is_empty();
        Ok(Self {
            faces,
            excluded_identities,
            context,
            state: State {
                phase: if done {
                    Phase::Done
                } else if has_exclusions {
                    Phase::CheckExclusions
                } else {
                    Phase::CopyPolygon
                },
                face_index: 0,
                polygon_copy: Vec::new(),
                polygon_fingerprint: 0,
                sampler_active: false,
                candidate_centers: Vec::new(),
                identity_cursor: 0,
                best_index: None,
                best_centers: Vec::new(),
                excluded_index: 0,
                excluded_identity_cursor: 0,
                failed: false,
            },
            active_sampler: None,
        })
    }

    pub(crate) fn restore(
        checkpoint: FaceSelectionCheckpoint,
        context: Arc<FrozenViabilityContext>,
    ) -> Result<Self, SelectionError> {
        if checkpoint.schema != CHECKPOINT_SCHEMA
            || checkpoint.context_fingerprint != context.fingerprint()
            || checkpoint.state.sampler_active != checkpoint.active_sampler.is_some()
        {
            return Err(SelectionError::InvalidCheckpoint);
        }
        validate_faces(&checkpoint.faces).map_err(|_| SelectionError::InvalidCheckpoint)?;
        validate_exclusions(&checkpoint.excluded_identities)
            .map_err(|_| SelectionError::InvalidCheckpoint)?;
        validate_state(
            &checkpoint.state,
            &checkpoint.faces,
            &checkpoint.excluded_identities,
        )?;
        let active_sampler = if let Some(sampler) = checkpoint.active_sampler {
            Some(FaceSampler::restore(sampler, context.clone())?)
        } else {
            None
        };
        Ok(Self {
            faces: checkpoint.faces,
            excluded_identities: checkpoint.excluded_identities,
            context,
            state: checkpoint.state,
            active_sampler,
        })
    }

    /// Explicit persistence boundary: the frozen face list and active sampler
    /// polygon are serialized here, never copied during `advance`.
    pub(crate) fn checkpoint(&self) -> FaceSelectionCheckpoint {
        FaceSelectionCheckpoint {
            schema: CHECKPOINT_SCHEMA,
            context_fingerprint: self.context.fingerprint(),
            faces: self.faces.clone(),
            excluded_identities: self.excluded_identities.clone(),
            state: self.state.clone(),
            active_sampler: self.active_sampler.as_ref().map(FaceSampler::checkpoint),
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn selection(&self) -> Option<Option<SelectionView<'_>>> {
        if self.state.phase != Phase::Done || self.state.failed {
            return None;
        }
        Some(self.state.best_index.map(|face_index| SelectionView {
            face_index,
            centers: &self.state.best_centers,
        }))
    }

    /// Move the winner and its centers; swap removal avoids shifting all faces.
    /// Dropping unselected storage remains allocator work outside logical quotas.
    #[cfg(feature = "legacy")]
    pub(crate) fn into_selected(
        mut self,
    ) -> Result<Option<(RoadFace, Vec<(f32, f32)>)>, SelectionError> {
        if self.state.phase != Phase::Done || self.state.failed {
            return Err(SelectionError::InvalidRuntimeState);
        }
        match self.state.best_index {
            Some(index) if index < self.faces.len() => Ok(Some((
                self.faces.swap_remove(index),
                self.state.best_centers,
            ))),
            Some(_) => Err(SelectionError::InvalidRuntimeState),
            None => Ok(None),
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn error(&self) -> Option<SelectionError> {
        self.state
            .failed
            .then_some(SelectionError::InvalidRuntimeState)
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn advance(&mut self, budget: usize) -> AdvanceResult {
        if budget == 0 || self.state.phase == Phase::Done {
            return AdvanceResult {
                work_used: 0,
                done: self.state.phase == Phase::Done,
            };
        }
        let mut work_used = 0;
        while work_used < budget && self.state.phase != Phase::Done {
            match self.state.phase {
                Phase::CheckExclusions => {
                    if self.state.excluded_index >= self.excluded_identities.len() {
                        self.state.excluded_index = 0;
                        self.state.excluded_identity_cursor = 0;
                        self.state.phase = Phase::CopyPolygon;
                        work_used += 1;
                        continue;
                    }
                    let candidate = &self.faces[self.state.face_index].identity;
                    let excluded = &self.excluded_identities[self.state.excluded_index];
                    let common = candidate.len().min(excluded.len());
                    if self.state.excluded_identity_cursor < common {
                        let index = self.state.excluded_identity_cursor;
                        self.state.excluded_identity_cursor += 1;
                        work_used += 1;
                        if candidate[index] != excluded[index] {
                            self.state.phase = Phase::NextExclusion;
                        }
                    } else {
                        work_used += 1;
                        self.state.phase = if candidate.len() == excluded.len() {
                            Phase::SkipExcludedFace
                        } else {
                            Phase::NextExclusion
                        };
                    }
                }
                Phase::NextExclusion => {
                    self.state.excluded_index += 1;
                    self.state.excluded_identity_cursor = 0;
                    self.state.phase =
                        if self.state.excluded_index >= self.excluded_identities.len() {
                            self.state.excluded_index = 0;
                            Phase::CopyPolygon
                        } else {
                            Phase::CheckExclusions
                        };
                    work_used += 1;
                }
                Phase::SkipExcludedFace => {
                    work_used += 1;
                    self.finish_face();
                }
                Phase::CopyPolygon => {
                    let face = &self.faces[self.state.face_index];
                    if self.state.polygon_copy.is_empty()
                        && self.state.polygon_copy.capacity() < face.polygon.len()
                    {
                        // Empty-vector growth moves no prior elements. The
                        // subsequent per-point pushes remain quota-accounted.
                        self.state.polygon_copy.reserve(face.polygon.len());
                    }
                    let Some(&point) = face.polygon.get(self.state.polygon_copy.len()) else {
                        self.fail();
                        continue;
                    };
                    if !valid_coordinate(point.0) || !valid_coordinate(point.1) {
                        self.fail();
                        continue;
                    }
                    if self.state.polygon_copy.is_empty() {
                        self.state.polygon_fingerprint =
                            polygon_fingerprint_start(face.polygon.len());
                    }
                    self.state.polygon_fingerprint =
                        polygon_fingerprint_add(self.state.polygon_fingerprint, point);
                    self.state.polygon_copy.push(point);
                    work_used += 1;
                    if self.state.polygon_copy.len() == face.polygon.len() {
                        self.state.phase = Phase::StartSampler;
                    }
                }
                Phase::StartSampler => {
                    let points = std::mem::take(&mut self.state.polygon_copy);
                    let polygon =
                        ValidatedPolygon::from_validated(points, self.state.polygon_fingerprint);
                    match FaceSampler::with_polygon(polygon, self.context.clone()) {
                        Ok(sampler) => {
                            self.active_sampler = Some(sampler);
                            self.state.sampler_active = true;
                            self.state.phase = Phase::Sampling;
                        }
                        Err(_error) => {
                            self.fail();
                            return AdvanceResult {
                                work_used,
                                done: true,
                            };
                        }
                    }
                    work_used += 1;
                }
                Phase::Sampling => {
                    let remaining = budget - work_used;
                    let Some(mut sampler) = self.active_sampler.take() else {
                        self.fail();
                        continue;
                    };
                    let step = sampler.advance(remaining);
                    work_used += step.work_used;
                    if let Some(error) = sampler.error() {
                        self.fail();
                        let _ = error;
                    } else if step.done {
                        let Some(centers) = sampler.into_centers() else {
                            self.fail();
                            continue;
                        };
                        self.state.candidate_centers = centers;
                        self.state.sampler_active = false;
                        self.state.phase = Phase::RankCandidate;
                    } else {
                        self.active_sampler = Some(sampler);
                    }
                }
                Phase::RankCandidate => {
                    work_used += 1;
                    let candidate_count = self.state.candidate_centers.len();
                    if candidate_count == 0 {
                        self.finish_face();
                        continue;
                    }
                    let Some(best_index) = self.state.best_index else {
                        self.promote_candidate();
                        continue;
                    };
                    let best_count = self.state.best_centers.len();
                    if candidate_count > best_count {
                        self.promote_candidate();
                    } else if candidate_count < best_count {
                        self.finish_face();
                    } else {
                        let candidate_area = self.faces[self.state.face_index].area;
                        let best_area = self.faces[best_index].area;
                        match candidate_area.total_cmp(&best_area) {
                            std::cmp::Ordering::Greater => self.promote_candidate(),
                            std::cmp::Ordering::Less => self.finish_face(),
                            std::cmp::Ordering::Equal => {
                                self.state.identity_cursor = 0;
                                self.state.phase = Phase::CompareIdentity;
                            }
                        }
                    }
                }
                Phase::CompareIdentity => {
                    let candidate = &self.faces[self.state.face_index].identity;
                    let best = &self.faces[self.state.best_index.expect("best candidate")].identity;
                    let common = candidate.len().min(best.len());
                    if self.state.identity_cursor < common {
                        let index = self.state.identity_cursor;
                        self.state.identity_cursor += 1;
                        work_used += 1;
                        match candidate[index].cmp(&best[index]) {
                            std::cmp::Ordering::Less => self.promote_candidate(),
                            std::cmp::Ordering::Greater => self.finish_face(),
                            std::cmp::Ordering::Equal if self.state.identity_cursor == common => {
                                self.state.phase = Phase::FinishIdentity;
                            }
                            std::cmp::Ordering::Equal => {}
                        }
                    } else {
                        self.state.phase = Phase::FinishIdentity;
                    }
                }
                Phase::FinishIdentity => {
                    work_used += 1;
                    let candidate = &self.faces[self.state.face_index].identity;
                    let best = &self.faces[self.state.best_index.expect("best candidate")].identity;
                    if candidate.len() < best.len() {
                        self.promote_candidate();
                    } else {
                        self.finish_face();
                    }
                }
                Phase::Done => break,
            }
        }
        AdvanceResult {
            work_used,
            done: self.state.phase == Phase::Done,
        }
    }

    #[cfg(feature = "legacy")]
    fn promote_candidate(&mut self) {
        self.state.best_index = Some(self.state.face_index);
        self.state.best_centers = std::mem::take(&mut self.state.candidate_centers);
        self.finish_face();
    }

    #[cfg(feature = "legacy")]
    fn finish_face(&mut self) {
        self.state.candidate_centers.clear();
        self.active_sampler = None;
        self.state.sampler_active = false;
        self.state.polygon_copy.clear();
        self.state.polygon_fingerprint = 0;
        self.state.identity_cursor = 0;
        self.state.excluded_index = 0;
        self.state.excluded_identity_cursor = 0;
        self.state.face_index += 1;
        self.state.phase = if self.state.face_index >= self.faces.len() {
            Phase::Done
        } else if self.excluded_identities.is_empty() {
            Phase::CopyPolygon
        } else {
            Phase::CheckExclusions
        };
    }

    #[cfg(feature = "legacy")]
    fn fail(&mut self) {
        self.state.phase = Phase::Done;
        self.state.failed = true;
        self.active_sampler = None;
        self.state.sampler_active = false;
        self.state.candidate_centers.clear();
        self.state.polygon_copy.clear();
    }
}

fn valid_coordinate(value: f32) -> bool {
    value.is_finite() && value.abs() <= MAX_COORDINATE
}

fn validate_face_shape(face: &RoadFace) -> Result<(), SelectionError> {
    if face.identity.is_empty()
        || face.identity.len() > MAX_FACE_VERTICES
        || face.vertex_ids.len() < 3
        || face.vertex_ids.len() > MAX_FACE_VERTICES
        || face.polygon.len() != face.vertex_ids.len()
        || face.boundary_edge_ids.len() != face.vertex_ids.len()
        || face.polygon.len() > MAX_FACE_VERTICES
        || !face.area.is_finite()
        || face.area <= 0.0
    {
        return Err(SelectionError::InvalidFace);
    }
    Ok(())
}

fn validate_faces(faces: &[RoadFace]) -> Result<(), SelectionError> {
    if faces.len() > MAX_FACES {
        return Err(SelectionError::TooManyFaces);
    }
    let mut polygon_items = 0usize;
    let mut identity_items = 0usize;
    for face in faces {
        validate_face_shape(face)?;
        polygon_items = polygon_items.saturating_add(face.polygon.len());
        identity_items = identity_items.saturating_add(face.identity.len());
        if polygon_items > MAX_TOTAL_FACE_ITEMS || identity_items > MAX_TOTAL_FACE_ITEMS {
            return Err(SelectionError::InvalidFace);
        }
    }
    Ok(())
}

fn validate_exclusions(excluded: &[Vec<u32>]) -> Result<(), SelectionError> {
    if excluded.len() > MAX_EXCLUDED_IDENTITIES {
        return Err(SelectionError::TooManyExclusions);
    }
    let mut total_items = 0usize;
    for identity in excluded {
        if identity.len() < 3 || identity.len() > MAX_FACE_VERTICES {
            return Err(SelectionError::InvalidExclusion);
        }
        total_items = total_items.saturating_add(identity.len());
        if total_items > MAX_EXCLUDED_IDENTITY_ITEMS {
            return Err(SelectionError::TooManyExclusions);
        }
    }
    Ok(())
}

fn validate_state(
    state: &State,
    faces: &[RoadFace],
    excluded: &[Vec<u32>],
) -> Result<(), SelectionError> {
    let fail = || SelectionError::InvalidCheckpoint;
    if state.face_index > faces.len()
        || state.polygon_copy.len() > MAX_FACE_VERTICES
        || state.candidate_centers.len() > MAX_CENTERS
        || state.best_centers.len() > MAX_CENTERS
        || state
            .candidate_centers
            .iter()
            .any(|p| !valid_coordinate(p.0) || !valid_coordinate(p.1))
        || state
            .best_centers
            .iter()
            .any(|p| !valid_coordinate(p.0) || !valid_coordinate(p.1))
        || state
            .polygon_copy
            .iter()
            .any(|p| !valid_coordinate(p.0) || !valid_coordinate(p.1))
        || state
            .best_index
            .is_some_and(|index| index >= state.face_index || index >= faces.len())
        || state.identity_cursor > MAX_FACE_VERTICES
        || state.excluded_index > excluded.len()
        || (!matches!(
            state.phase,
            Phase::CheckExclusions | Phase::NextExclusion | Phase::SkipExcludedFace
        ) && (state.excluded_index != 0 || state.excluded_identity_cursor != 0))
    {
        return Err(fail());
    }
    if state.failed {
        return (state.phase == Phase::Done && !state.sampler_active)
            .then_some(())
            .ok_or_else(fail);
    }
    if state.phase == Phase::Done {
        if state.face_index != faces.len()
            || state.sampler_active
            || !state.polygon_copy.is_empty()
            || state.excluded_index != 0
            || state.excluded_identity_cursor != 0
        {
            return Err(fail());
        }
        return Ok(());
    }
    if state.face_index >= faces.len() {
        return Err(fail());
    }
    let polygon_len = faces[state.face_index].polygon.len();
    match state.phase {
        Phase::CheckExclusions | Phase::NextExclusion | Phase::SkipExcludedFace => {
            if excluded.is_empty()
                || state.sampler_active
                || !state.polygon_copy.is_empty()
                || !state.candidate_centers.is_empty()
                || state.excluded_index >= excluded.len()
                || state.excluded_identity_cursor
                    > faces[state.face_index]
                        .identity
                        .len()
                        .min(excluded[state.excluded_index].len())
            {
                return Err(fail());
            }
            let candidate = &faces[state.face_index].identity;
            let identity = &excluded[state.excluded_index];
            let common = candidate.len().min(identity.len());
            let mismatch = state.phase == Phase::NextExclusion
                && state.excluded_identity_cursor > 0
                && candidate[state.excluded_identity_cursor - 1]
                    != identity[state.excluded_identity_cursor - 1];
            let verified_prefix = state.excluded_identity_cursor - usize::from(mismatch);
            if candidate[..verified_prefix] != identity[..verified_prefix] {
                return Err(fail());
            }
            if state.phase == Phase::NextExclusion {
                let length_mismatch =
                    state.excluded_identity_cursor == common && candidate.len() != identity.len();
                if !mismatch && !length_mismatch {
                    return Err(fail());
                }
            }
            if state.phase == Phase::SkipExcludedFace
                && (state.excluded_identity_cursor != candidate.len()
                    || state.excluded_identity_cursor != identity.len())
            {
                return Err(fail());
            }
        }
        Phase::CopyPolygon => {
            if state.sampler_active
                || !state.candidate_centers.is_empty()
                || state.polygon_copy.len() >= polygon_len
                || (state.polygon_copy.is_empty() && state.polygon_fingerprint != 0)
                || state.excluded_index != 0
                || state.excluded_identity_cursor != 0
            {
                return Err(fail());
            }
        }
        Phase::StartSampler => {
            if state.sampler_active
                || !state.candidate_centers.is_empty()
                || state.polygon_copy.len() != polygon_len
            {
                return Err(fail());
            }
        }
        Phase::Sampling => {
            if !state.sampler_active
                || !state.polygon_copy.is_empty()
                || !state.candidate_centers.is_empty()
            {
                return Err(fail());
            }
        }
        Phase::RankCandidate => {
            if state.sampler_active || !state.polygon_copy.is_empty() {
                return Err(fail());
            }
        }
        Phase::CompareIdentity | Phase::FinishIdentity => {
            if state.sampler_active
                || state.best_index.is_none()
                || !state.polygon_copy.is_empty()
                || state.candidate_centers.len() != state.best_centers.len()
                || state.candidate_centers.is_empty()
            {
                return Err(fail());
            }
        }
        Phase::Done => unreachable!(),
    }
    if state.best_index.is_none() && !state.best_centers.is_empty() {
        return Err(fail());
    }
    if state.best_index.is_some() && state.best_centers.is_empty() {
        return Err(fail());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face_target_viability::{FrozenViabilityInput, Segment};

    fn context(blocked: bool) -> Arc<FrozenViabilityContext> {
        FrozenViabilityContext::new(FrozenViabilityInput {
            parcel_footprints: if blocked {
                vec![crate::geometry::OrientedRect {
                    x: 0.0,
                    z: 0.0,
                    width: 10_000.0,
                    depth: 10_000.0,
                    rotation: 0.0,
                }]
            } else {
                vec![]
            },
            reserved_lots: vec![],
            open_roads: vec![],
            pending_roads: vec![],
            river_segments: vec![],
            neighborhood_segments: Vec::<Segment>::new(),
            neighborhood_interior: None,
        })
        .unwrap()
    }

    fn rectangle(identity: &[u32], width: f32, depth: f32) -> RoadFace {
        let polygon = vec![
            (-width / 2.0, -depth / 2.0),
            (width / 2.0, -depth / 2.0),
            (width / 2.0, depth / 2.0),
            (-width / 2.0, depth / 2.0),
        ];
        RoadFace {
            identity: identity.to_vec(),
            vertex_ids: identity.to_vec(),
            boundary_edge_ids: identity.to_vec(),
            area: width as f64 * depth as f64,
            polygon,
            contains_proposed_edge: false,
        }
    }

    fn run(mut selection: FaceSelection, quota: usize, checkpoint: bool) -> SelectionViewOwned {
        assert_eq!(
            selection.advance(0),
            AdvanceResult {
                work_used: 0,
                done: false
            }
        );
        for _ in 0..2_000_000 {
            let step = selection.advance(quota);
            assert!(step.work_used <= quota);
            if step.done {
                return selection
                    .selection()
                    .expect("finished")
                    .map(|view| SelectionViewOwned {
                        face_index: view.face_index,
                        centers: view.centers.to_vec(),
                    })
                    .unwrap_or(SelectionViewOwned {
                        face_index: usize::MAX,
                        centers: vec![],
                    });
            }
            assert!(step.work_used > 0, "selection must make bounded progress");
            if checkpoint {
                let bytes = serde_json::to_vec(&selection.checkpoint()).unwrap();
                let checkpoint: FaceSelectionCheckpoint = serde_json::from_slice(&bytes).unwrap();
                selection = FaceSelection::restore(checkpoint, selection.context.clone()).unwrap();
            }
        }
        panic!("selection did not complete");
    }

    #[derive(Debug, PartialEq)]
    struct SelectionViewOwned {
        face_index: usize,
        centers: Vec<(f32, f32)>,
    }

    #[test]
    fn streams_all_faces_and_ranks_count_before_area() {
        let mut large_face = rectangle(&[5, 6, 7, 8], 192.0, 192.0);
        // Keep the ranking fixture focused on the ordered keys: an upstream
        // traced-face area is metadata here, so intentionally make it smaller
        // than the square's while the elongated face yields more viable sites.
        large_face.area = 1.0;
        let faces = vec![rectangle(&[1, 2, 3, 4], 40.0, 40.0), large_face];
        let expected = run(
            FaceSelection::new(faces.clone(), context(false)).unwrap(),
            7,
            false,
        );
        assert_eq!(expected.face_index, 1);
        assert!(!expected.centers.is_empty());
        // This fixture gives the second face a lower area key but more
        // accepted grid centers, so count remains the primary key.
        assert!(faces[1].area < faces[0].area);
    }

    #[test]
    fn equal_count_and_area_uses_lexicographically_smallest_identity() {
        let faces = vec![
            rectangle(&[9, 2, 3, 4], 40.0, 40.0),
            rectangle(&[1, 2, 3, 4], 40.0, 40.0),
        ];
        let selected = run(FaceSelection::new(faces, context(false)).unwrap(), 1, true);
        assert_eq!(selected.face_index, 1);
        assert_eq!(selected.centers.len(), 25);
    }

    #[test]
    fn no_viable_face_returns_completed_empty_selection() {
        let selection =
            FaceSelection::new(vec![rectangle(&[1, 2, 3, 4], 40.0, 40.0)], context(true)).unwrap();
        let result = run(selection, 48, false);
        assert_eq!(result.face_index, usize::MAX);
        assert!(result.centers.is_empty());
    }

    #[test]
    fn checkpoint_resume_preserves_streaming_result_and_rejects_forged_cursor() {
        let faces = vec![
            rectangle(&[1, 2, 3, 4], 40.0, 40.0),
            rectangle(&[0, 2, 3, 4], 40.0, 40.0),
        ];
        let selection = FaceSelection::new(faces.clone(), context(false)).unwrap();
        let checkpointed = run(selection.clone(), 7, true);
        let uninterrupted = run(selection, 7, false);
        assert_eq!(checkpointed, uninterrupted);

        let mut checkpoint = FaceSelection::new(faces, context(false))
            .unwrap()
            .checkpoint();
        checkpoint.state.face_index = usize::MAX;
        assert_eq!(
            FaceSelection::restore(checkpoint, context(false)).err(),
            Some(SelectionError::InvalidCheckpoint)
        );
    }

    #[test]
    fn empty_input_is_done_without_a_winner() {
        let mut selection = FaceSelection::new(vec![], context(false)).unwrap();
        assert_eq!(
            selection.advance(1),
            AdvanceResult {
                work_used: 0,
                done: true
            }
        );
        assert_eq!(selection.selection().unwrap().map(|v| v.face_index), None);
    }

    #[test]
    fn exclusions_skip_the_default_winner_and_all_excluded_faces() {
        let faces = vec![
            rectangle(&[1, 2, 3, 4], 40.0, 40.0),
            rectangle(&[5, 6, 7, 8], 40.0, 40.0),
        ];
        let default = run(
            FaceSelection::new(faces.clone(), context(false)).unwrap(),
            7,
            false,
        );
        assert_eq!(default.face_index, 0);

        let runner_up = run(
            FaceSelection::new_excluding(
                faces.clone(),
                context(false),
                vec![faces[0].identity.clone()],
            )
            .unwrap(),
            1,
            true,
        );
        assert_eq!(runner_up.face_index, 1);
        assert_eq!(runner_up.centers.len(), default.centers.len());

        let none = run(
            FaceSelection::new_excluding(
                faces.clone(),
                context(false),
                faces.iter().map(|face| face.identity.clone()).collect(),
            )
            .unwrap(),
            1,
            true,
        );
        assert_eq!(none.face_index, usize::MAX);
        assert!(none.centers.is_empty());
    }

    #[test]
    fn exclusions_resume_identically_across_quotas_and_reject_malformed_ids() {
        let faces = vec![
            rectangle(&[1, 2, 3, 4], 40.0, 40.0),
            rectangle(&[5, 6, 7, 8], 40.0, 40.0),
        ];
        let excluded = vec![vec![1, 2, 3, 4], vec![90, 91, 92]];
        let one = run(
            FaceSelection::new_excluding(faces.clone(), context(false), excluded.clone()).unwrap(),
            1,
            true,
        );
        let seven = run(
            FaceSelection::new_excluding(faces.clone(), context(false), excluded.clone()).unwrap(),
            7,
            true,
        );
        let uninterrupted = run(
            FaceSelection::new_excluding(faces.clone(), context(false), excluded).unwrap(),
            7,
            false,
        );
        assert_eq!(one, seven);
        assert_eq!(seven, uninterrupted);

        assert_eq!(
            FaceSelection::new_excluding(faces.clone(), context(false), vec![vec![1, 2]],).err(),
            Some(SelectionError::InvalidExclusion)
        );
        assert_eq!(
            FaceSelection::new_excluding(
                faces.clone(),
                context(false),
                vec![vec![0; MAX_FACE_VERTICES + 1]],
            )
            .err(),
            Some(SelectionError::InvalidExclusion)
        );
        let mut checkpoint =
            FaceSelection::new_excluding(faces, context(false), vec![vec![1, 2, 3]])
                .unwrap()
                .checkpoint();
        checkpoint.excluded_identities[0].clear();
        assert_eq!(
            FaceSelection::restore(checkpoint, context(false)).err(),
            Some(SelectionError::InvalidCheckpoint)
        );
    }
}
