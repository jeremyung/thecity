//! Bounded, deterministic district measurements and project scoring.
//!
//! Coverage is a clipped nominal-building-envelope proxy; overlaps are summed.
//! Envelope × floors is a floor-area proxy. All targets and score weights are
//! caller supplied; this module does not encode a universal city density target
//! or determine net developable land. Measurement is bounded but recomputes the
//! whole supplied world; a runtime caller must schedule/cache it rather than
//! invoke it every simulation tick.

use crate::geometry::OrientedRect;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_DISTRICT_CELLS: usize = 256;
pub(crate) const MAX_DENSITY_BUILDINGS: usize = 2_000;
pub(crate) const MAX_DENSITY_FRONTAGES: usize = 5_000;
#[cfg(feature = "legacy")]
pub(crate) const MAX_PROJECT_CANDIDATES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct GridSpec {
    pub min_x: f64,
    pub max_x: f64,
    pub min_z: f64,
    pub max_z: f64,
    pub cell_size: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MeasureError {
    InvalidGrid,
    TooManyCells,
    TooManySamples,
    InvalidFootprint,
    NonFiniteGeometry,
    InvalidBuilding,
    DuplicateId,
    IncompleteMeasurement,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum BuildingUse {
    Housing,
    Commercial,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct BuildingSample {
    pub id: u32,
    pub footprint: OrientedRect,
    pub floors: u8,
    pub use_class: BuildingUse,
    pub open: bool,
    /// Current housed residents. Ignored for commercial and other uses.
    pub resident_occupancy: u16,
    pub capacity: u16,
    pub business_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct FrontageSample {
    pub id: u32,
    pub start_x: f64,
    pub start_z: f64,
    pub end_x: f64,
    pub end_z: f64,
    pub occupied: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CellMetrics {
    pub id: usize,
    pub column: usize,
    pub row: usize,
    pub min_x: f64,
    pub max_x: f64,
    pub min_z: f64,
    pub max_z: f64,
    pub gross_area: f64,
    /// Sum of open nominal building envelopes clipped to the cell; may overlap.
    pub built_envelope_area_proxy: f64,
    /// Sum of clipped nominal building envelope × floors, a floor-area proxy.
    pub floor_area_proxy: f64,
    pub housing_capacity: f64,
    pub housing_occupied: f64,
    pub housing_vacant_capacity: f64,
    pub commercial_capacity: f64,
    pub active_commercial_capacity: f64,
    /// A building contributes once to each cell its envelope intersects.
    pub intersecting_active_commercial_buildings: u32,
    /// Intersection counts; do not sum across cells as unique buildings.
    pub intersecting_inactive_commercial_buildings: u32,
    pub usable_frontage_m: f64,
    pub occupied_frontage_m: f64,
}

impl CellMetrics {
    pub(crate) fn built_envelope_coverage_proxy(&self) -> f64 {
        ratio(self.built_envelope_area_proxy, self.gross_area)
    }

    pub(crate) fn floor_area_ratio_proxy(&self) -> f64 {
        ratio(self.floor_area_proxy, self.gross_area)
    }

    pub(crate) fn housing_vacancy_ratio(&self) -> f64 {
        ratio(self.housing_vacant_capacity, self.housing_capacity)
    }

    pub(crate) fn commercial_vacancy_ratio(&self) -> f64 {
        ratio(
            self.commercial_capacity - self.active_commercial_capacity,
            self.commercial_capacity,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DistrictMetrics {
    pub columns: usize,
    pub rows: usize,
    pub cells: Vec<CellMetrics>,
}

impl DistrictMetrics {
    pub(crate) fn cell(&self, id: usize) -> Option<&CellMetrics> {
        self.cells.get(id)
    }
}

/// Measure fixed cells aligned to the finite-world origin. Input order does not
/// affect results: all samples are processed in ascending ID order. This scans
/// every supplied sample and is intended for scheduled/cached evaluations, not
/// each simulation tick.
pub(crate) const MAX_MEASUREMENT_SAMPLES_PER_ADVANCE: usize = 16;

/// Frozen input and cursor needed to reconstruct an in-progress measurement.
/// Restore replays the processed prefix from these saved samples, so the result
/// never depends on later world mutations or elapsed time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct MeasurementCheckpoint {
    pub(crate) grid: GridSpec,
    pub(crate) buildings: Vec<BuildingSample>,
    pub(crate) frontages: Vec<FrontageSample>,
    pub(crate) processed_samples: usize,
}

/// Incremental, deterministic district measurement over a private frozen copy
/// of the source samples. `advance` performs at most 16 samples and each sample
/// visits no more than the 256-cell district cap. Starting copies and sorts at
/// most 7,000 samples once. Checkpoint restore replays the frozen prefix to
/// rebuild partial metrics; it does not consult a changed live world. Stored
/// samples, metrics, and scratch use about 0.3 MiB at configured caps on a
/// 64-bit target, excluding caller-owned inputs, checkpoint copies, and
/// transient polygon clipping allocations.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeasurementAccumulator {
    grid: GridSpec,
    columns: usize,
    rows: usize,
    buildings: Vec<BuildingSample>,
    frontages: Vec<FrontageSample>,
    building_cursor: usize,
    frontage_cursor: usize,
    frontages_unique_checked: bool,
    frontages_have_duplicate: bool,
    metrics: DistrictMetrics,
    area_by_cell: Vec<f64>,
    failure: Option<MeasureError>,
}

impl MeasurementAccumulator {
    pub(crate) fn begin(
        grid: GridSpec,
        buildings: &[BuildingSample],
        frontages: &[FrontageSample],
    ) -> Result<Self, MeasureError> {
        let (columns, rows, cell_count) = validate_grid(grid)?;
        if buildings.len() > MAX_DENSITY_BUILDINGS || frontages.len() > MAX_DENSITY_FRONTAGES {
            return Err(MeasureError::TooManySamples);
        }
        let sorted_buildings = sorted_unique(buildings, |sample| sample.id)?;
        let mut sorted_frontages = frontages.to_vec();
        sorted_frontages.sort_by_key(|sample| sample.id);
        let frontages_have_duplicate = sorted_frontages
            .windows(2)
            .any(|pair| pair[0].id == pair[1].id);
        Ok(Self {
            grid,
            columns,
            rows,
            buildings: sorted_buildings,
            frontages: sorted_frontages,
            building_cursor: 0,
            frontage_cursor: 0,
            frontages_unique_checked: false,
            frontages_have_duplicate,
            metrics: empty_metrics(grid, columns, rows, cell_count),
            area_by_cell: vec![0.0; cell_count],
            failure: None,
        })
    }

    /// Resume from the exact frozen inputs and number of completed samples.
    /// Reconstructing partial metrics costs at most the already processed
    /// sample count and preserves the original sorted accumulation order.
    pub(crate) fn resume(checkpoint: MeasurementCheckpoint) -> Result<Self, MeasureError> {
        validate_checkpoint(&checkpoint)?;
        let total = checkpoint.buildings.len() + checkpoint.frontages.len();
        if checkpoint.processed_samples > total {
            return Err(MeasureError::IncompleteMeasurement);
        }
        let mut accumulator = Self::begin(
            checkpoint.grid,
            &checkpoint.buildings,
            &checkpoint.frontages,
        )?;
        for _ in 0..checkpoint.processed_samples {
            if accumulator.advance(1)? != 1 {
                return Err(MeasureError::IncompleteMeasurement);
            }
        }
        Ok(accumulator)
    }

    /// Process at most the requested number of samples, clamped to 16.
    /// Passing zero is a no-op. The returned count is the number completed.
    pub(crate) fn advance(&mut self, max_samples: usize) -> Result<usize, MeasureError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        let budget = max_samples.min(MAX_MEASUREMENT_SAMPLES_PER_ADVANCE);
        let mut processed = 0;
        while processed < budget {
            if self.building_cursor < self.buildings.len() {
                let building = self.buildings[self.building_cursor];
                if let Err(error) = self.process_building(building) {
                    self.failure = Some(error);
                    return Err(error);
                }
                self.building_cursor += 1;
                processed += 1;
                continue;
            }
            if self.frontage_cursor < self.frontages.len() {
                if !self.frontages_unique_checked {
                    if self.frontages_have_duplicate {
                        self.failure = Some(MeasureError::DuplicateId);
                        return Err(MeasureError::DuplicateId);
                    }
                    self.frontages_unique_checked = true;
                }
                let frontage = self.frontages[self.frontage_cursor];
                if ![
                    frontage.start_x,
                    frontage.start_z,
                    frontage.end_x,
                    frontage.end_z,
                ]
                .into_iter()
                .all(f64::is_finite)
                {
                    self.failure = Some(MeasureError::NonFiniteGeometry);
                    return Err(MeasureError::NonFiniteGeometry);
                }
                accumulate_segment(
                    &mut self.metrics.cells,
                    self.grid,
                    frontage,
                    |cell, length| {
                        cell.usable_frontage_m += length;
                        if frontage.occupied {
                            cell.occupied_frontage_m += length;
                        }
                    },
                );
                self.frontage_cursor += 1;
                processed += 1;
                continue;
            }
            break;
        }
        Ok(processed)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.failure.is_none()
            && self.building_cursor == self.buildings.len()
            && self.frontage_cursor == self.frontages.len()
    }

    pub(crate) fn metrics(&self) -> Option<&DistrictMetrics> {
        self.is_complete().then_some(&self.metrics)
    }

    pub(crate) fn processed_samples(&self) -> usize {
        self.building_cursor + self.frontage_cursor
    }

    pub(crate) fn checkpoint(&self) -> Result<MeasurementCheckpoint, MeasureError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        Ok(MeasurementCheckpoint {
            grid: self.grid,
            buildings: self.buildings.clone(),
            frontages: self.frontages.clone(),
            processed_samples: self.processed_samples(),
        })
    }

    pub(crate) fn finish(self) -> Result<DistrictMetrics, MeasureError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if self.building_cursor != self.buildings.len()
            || self.frontage_cursor != self.frontages.len()
        {
            return Err(MeasureError::IncompleteMeasurement);
        }
        Ok(self.metrics)
    }

    fn process_building(&mut self, building: BuildingSample) -> Result<(), MeasureError> {
        if building.floors == 0
            || (building.use_class == BuildingUse::Housing
                && building.resident_occupancy > building.capacity)
        {
            return Err(MeasureError::InvalidBuilding);
        }
        if !building.open {
            return Ok(());
        }
        self.area_by_cell.fill(0.0);
        let total_clipped_area = accumulate_rect(
            &mut self.metrics.cells,
            self.columns,
            self.grid,
            building.footprint,
            |cell, area, cell_id| {
                self.area_by_cell[cell_id] = area;
                cell.built_envelope_area_proxy += area;
                cell.floor_area_proxy += area * building.floors as f64;
            },
        )?;
        if total_clipped_area <= 0.0 {
            return Ok(());
        }
        for (cell_id, clipped_area) in self.area_by_cell.iter().copied().enumerate() {
            if clipped_area <= 0.0 {
                continue;
            }
            let share = clipped_area / total_clipped_area;
            let cell = &mut self.metrics.cells[cell_id];
            match building.use_class {
                BuildingUse::Housing => {
                    let capacity = building.capacity as f64 * share;
                    let occupied = building.resident_occupancy as f64 * share;
                    cell.housing_capacity += capacity;
                    cell.housing_occupied += occupied;
                    cell.housing_vacant_capacity += capacity - occupied;
                }
                BuildingUse::Commercial => {
                    cell.commercial_capacity += building.capacity as f64 * share;
                    if building.business_active {
                        cell.active_commercial_capacity += building.capacity as f64 * share;
                        cell.intersecting_active_commercial_buildings += 1;
                    } else {
                        cell.intersecting_inactive_commercial_buildings += 1;
                    }
                }
                BuildingUse::Other => {}
            }
        }
        Ok(())
    }
}

/// Validate the complete frozen source before accepting a saved checkpoint.
/// In particular, validation covers samples beyond the processed prefix so a
/// malformed deferred sample cannot be introduced after simulation resumes.
pub(crate) fn validate_checkpoint(checkpoint: &MeasurementCheckpoint) -> Result<(), MeasureError> {
    validate_grid(checkpoint.grid)?;
    if checkpoint.buildings.len() > MAX_DENSITY_BUILDINGS
        || checkpoint.frontages.len() > MAX_DENSITY_FRONTAGES
    {
        return Err(MeasureError::TooManySamples);
    }
    let total = checkpoint
        .buildings
        .len()
        .checked_add(checkpoint.frontages.len())
        .ok_or(MeasureError::TooManySamples)?;
    if checkpoint.processed_samples > total {
        return Err(MeasureError::IncompleteMeasurement);
    }

    for pair in checkpoint.buildings.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(MeasureError::DuplicateId);
        }
        if pair[0].id > pair[1].id {
            return Err(MeasureError::InvalidCheckpoint);
        }
    }
    for building in &checkpoint.buildings {
        if building.floors == 0
            || (building.use_class == BuildingUse::Housing
                && building.resident_occupancy > building.capacity)
        {
            return Err(MeasureError::InvalidBuilding);
        }
        let footprint = building.footprint;
        if [
            footprint.x,
            footprint.z,
            footprint.width,
            footprint.depth,
            footprint.rotation,
        ]
        .into_iter()
        .any(|value| !value.is_finite())
        {
            return Err(MeasureError::NonFiniteGeometry);
        }
        if !footprint.finite_positive() {
            return Err(MeasureError::InvalidFootprint);
        }
        if footprint
            .corners()
            .iter()
            .any(|(x, z)| !x.is_finite() || !z.is_finite())
        {
            return Err(MeasureError::NonFiniteGeometry);
        }
    }

    for pair in checkpoint.frontages.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(MeasureError::DuplicateId);
        }
        if pair[0].id > pair[1].id {
            return Err(MeasureError::InvalidCheckpoint);
        }
    }
    if checkpoint.frontages.iter().any(|frontage| {
        ![
            frontage.start_x,
            frontage.start_z,
            frontage.end_x,
            frontage.end_z,
        ]
        .into_iter()
        .all(f64::is_finite)
    }) {
        return Err(MeasureError::NonFiniteGeometry);
    }
    Ok(())
}

fn empty_metrics(
    grid: GridSpec,
    columns: usize,
    rows: usize,
    cell_count: usize,
) -> DistrictMetrics {
    let mut cells = Vec::with_capacity(cell_count);
    for row in 0..rows {
        let min_z = grid.min_z + row as f64 * grid.cell_size;
        let max_z = (min_z + grid.cell_size).min(grid.max_z);
        for column in 0..columns {
            let min_x = grid.min_x + column as f64 * grid.cell_size;
            let max_x = (min_x + grid.cell_size).min(grid.max_x);
            cells.push(CellMetrics {
                id: row * columns + column,
                column,
                row,
                min_x,
                max_x,
                min_z,
                max_z,
                gross_area: (max_x - min_x) * (max_z - min_z),
                built_envelope_area_proxy: 0.0,
                floor_area_proxy: 0.0,
                housing_capacity: 0.0,
                housing_occupied: 0.0,
                housing_vacant_capacity: 0.0,
                commercial_capacity: 0.0,
                active_commercial_capacity: 0.0,
                intersecting_active_commercial_buildings: 0,
                intersecting_inactive_commercial_buildings: 0,
                usable_frontage_m: 0.0,
                occupied_frontage_m: 0.0,
            });
        }
    }
    DistrictMetrics {
        columns,
        rows,
        cells,
    }
}

pub(crate) fn measure(
    grid: GridSpec,
    buildings: &[BuildingSample],
    frontages: &[FrontageSample],
) -> Result<DistrictMetrics, MeasureError> {
    let mut accumulator = MeasurementAccumulator::begin(grid, buildings, frontages)?;
    while !accumulator.is_complete() {
        accumulator.advance(MAX_MEASUREMENT_SAMPLES_PER_ADVANCE)?;
    }
    accumulator.finish()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg(feature = "legacy")]
pub(crate) enum CandidateKind {
    Infill,
    InteriorAccess,
    Expansion,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(any(test, feature = "legacy"))]
pub(crate) struct ProjectCandidate {
    pub stable_id: u64,
    pub kind: CandidateKind,
    pub cell_id: usize,
    /// Geometry, access, economic, and resource gates have already accepted it.
    pub feasible: bool,
    pub built_envelope_gain_proxy: f64,
    pub floor_area_gain_proxy: f64,
    pub frontage_gain_m: f64,
    pub road_cost_m: f64,
    pub demand_fit: f64,
    pub housing_capacity_gain: f64,
    pub commercial_capacity_gain: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct TargetRange {
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct DistrictTargets {
    /// Explicit deficit thresholds as fractions of gross cell area. Scoring
    /// rewards progress only toward each range's `min`; `max` is validated
    /// configuration metadata and is not an upper-bound gate.
    pub built_envelope_coverage_proxy: TargetRange,
    /// Floor-area proxy per gross cell area. Only `min` is scored.
    pub floor_area_ratio_proxy: TargetRange,
    /// Occupied frontage in meters. Only `min` is scored.
    pub occupied_frontage_m: TargetRange,
    pub housing_vacancy_expansion_limit: f64,
    pub commercial_vacancy_expansion_limit: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct ScoreWeights {
    pub coverage: f64,
    pub floor_area: f64,
    pub frontage: f64,
    pub demand: f64,
    /// Road cost scale in meters; costs lower score through a bounded divisor.
    pub road_cost_scale_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(any(test, feature = "legacy"))]
pub(crate) struct RankedCandidate {
    pub stable_id: u64,
    pub kind: CandidateKind,
    pub score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScoreError {
    TooManyCandidates,
    InvalidConfiguration,
    InvalidCandidate,
    DuplicateCandidateId,
    TargetCountMismatch,
}

/// Score at most 128 already-feasible projects and return a stable descending
/// ranking. Scores address deficits below target minima, with no upper-range
/// enforcement or hysteresis. Vacancy suppresses expansions only when the
/// relevant use has positive vacant capacity. Geometry and delivery access
/// remain authoritative upstream prerequisites.
#[cfg(any(test, feature = "legacy"))]
pub(crate) fn rank_candidates(
    metrics: &DistrictMetrics,
    candidates: &[ProjectCandidate],
    targets: DistrictTargets,
    weights: ScoreWeights,
) -> Result<Vec<RankedCandidate>, ScoreError> {
    rank_candidates_with_targets(
        metrics,
        candidates,
        CandidateTargets::Uniform(targets),
        weights,
    )
}

/// Score feasible projects against the targets for the candidate's assigned
/// district cell. `cell_id` selects a scoring district; it does not grant
/// permission to infer vacancy from a destination cell. Callers with projects
/// spanning multiple cells must allocate gains consistently or aggregate them
/// before ranking.
#[cfg(any(test, feature = "legacy"))]
pub(crate) fn rank_candidates_by_district(
    metrics: &DistrictMetrics,
    candidates: &[ProjectCandidate],
    targets: &[DistrictTargets],
    weights: ScoreWeights,
) -> Result<Vec<RankedCandidate>, ScoreError> {
    rank_candidates_with_targets(
        metrics,
        candidates,
        CandidateTargets::ByCell(targets),
        weights,
    )
}

#[derive(Clone, Copy)]
#[cfg(any(test, feature = "legacy"))]
enum CandidateTargets<'a> {
    Uniform(DistrictTargets),
    ByCell(&'a [DistrictTargets]),
}

#[cfg(any(test, feature = "legacy"))]
impl<'a> CandidateTargets<'a> {
    #[cfg(any(test, feature = "legacy"))]
    fn copy_for_cell(self, cell_id: usize) -> Option<DistrictTargets> {
        match self {
            Self::Uniform(targets) => Some(targets),
            Self::ByCell(targets) => targets.get(cell_id).copied(),
        }
    }

    fn validate_count(self, cell_count: usize) -> Result<(), ScoreError> {
        if let Self::ByCell(targets) = self {
            if targets.len() != cell_count {
                return Err(ScoreError::TargetCountMismatch);
            }
        }
        Ok(())
    }

    #[cfg(any(test, feature = "legacy"))]
    fn all(self) -> impl Iterator<Item = DistrictTargets> + 'a {
        let (uniform, by_cell) = match self {
            Self::Uniform(target) => (Some(target), None),
            Self::ByCell(targets) => (None, Some(targets)),
        };
        uniform.into_iter().chain(
            by_cell
                .into_iter()
                .flat_map(|targets| targets.iter().copied()),
        )
    }
}

#[cfg(any(test, feature = "legacy"))]
fn rank_candidates_with_targets(
    metrics: &DistrictMetrics,
    candidates: &[ProjectCandidate],
    targets: CandidateTargets<'_>,
    weights: ScoreWeights,
) -> Result<Vec<RankedCandidate>, ScoreError> {
    if candidates.len() > MAX_PROJECT_CANDIDATES {
        return Err(ScoreError::TooManyCandidates);
    }
    let mut stable_ids: Vec<_> = candidates
        .iter()
        .map(|candidate| candidate.stable_id)
        .collect();
    stable_ids.sort_unstable();
    if stable_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ScoreError::DuplicateCandidateId);
    }
    targets.validate_count(metrics.cells.len())?;
    if !valid_weights(weights) || !targets.all().all(valid_targets) {
        return Err(ScoreError::InvalidConfiguration);
    }

    let mut ranked = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let numeric = [
            candidate.built_envelope_gain_proxy,
            candidate.floor_area_gain_proxy,
            candidate.frontage_gain_m,
            candidate.road_cost_m,
            candidate.demand_fit,
            candidate.housing_capacity_gain,
            candidate.commercial_capacity_gain,
        ];
        if numeric
            .into_iter()
            .any(|value| !value.is_finite() || value < 0.0)
            || candidate.demand_fit > 1.0
        {
            return Err(ScoreError::InvalidCandidate);
        }
        if !candidate.feasible {
            continue;
        }
        let Some(cell) = metrics
            .cells
            .get(candidate.cell_id)
            .filter(|cell| cell.id == candidate.cell_id)
        else {
            return Err(ScoreError::InvalidCandidate);
        };
        let Some(target) = targets.copy_for_cell(candidate.cell_id) else {
            return Err(ScoreError::InvalidCandidate);
        };
        if candidate.kind == CandidateKind::Expansion {
            let housing_vacancy = cell.housing_vacancy_ratio();
            let commercial_vacancy = cell.commercial_vacancy_ratio();
            if (candidate.housing_capacity_gain > 0.0
                && cell.housing_vacant_capacity > 0.0
                && housing_vacancy >= target.housing_vacancy_expansion_limit)
                || (candidate.commercial_capacity_gain > 0.0
                    && cell.commercial_capacity > cell.active_commercial_capacity
                    && commercial_vacancy >= target.commercial_vacancy_expansion_limit)
            {
                continue;
            }
        }
        let coverage_min = cell.gross_area * target.built_envelope_coverage_proxy.min;
        let floor_min = cell.gross_area * target.floor_area_ratio_proxy.min;
        let frontage_min = target.occupied_frontage_m.min;
        let coverage_need = (coverage_min - cell.built_envelope_area_proxy).max(0.0);
        let floor_need = (floor_min - cell.floor_area_proxy).max(0.0);
        let frontage_need = (frontage_min - cell.occupied_frontage_m).max(0.0);
        let coverage_gain =
            candidate.built_envelope_gain_proxy.min(coverage_need) / coverage_min.max(1.0);
        let floor_gain = candidate.floor_area_gain_proxy.min(floor_need) / floor_min.max(1.0);
        let frontage_gain = candidate.frontage_gain_m.min(frontage_need) / frontage_min.max(1.0);
        let benefit = weights.coverage * coverage_gain
            + weights.floor_area * floor_gain
            + weights.frontage * frontage_gain
            + weights.demand * candidate.demand_fit;
        let cost_divisor = 1.0 + candidate.road_cost_m / weights.road_cost_scale_m;
        let score = benefit / cost_divisor;
        if !score.is_finite() {
            return Err(ScoreError::InvalidCandidate);
        }
        ranked.push(RankedCandidate {
            stable_id: candidate.stable_id,
            kind: candidate.kind,
            score,
        });
    }
    ranked.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.stable_id.cmp(&right.stable_id))
    });
    Ok(ranked)
}

#[cfg(any(test, feature = "legacy"))]
fn valid_targets(targets: DistrictTargets) -> bool {
    let ranges = [
        targets.built_envelope_coverage_proxy,
        targets.floor_area_ratio_proxy,
        targets.occupied_frontage_m,
    ];
    ranges
        .into_iter()
        .flat_map(|range| [range.min, range.max])
        .all(|value| value.is_finite() && value >= 0.0)
        && ranges.into_iter().all(|range| range.min <= range.max)
        && (0.0..=1.0).contains(&targets.housing_vacancy_expansion_limit)
        && (0.0..=1.0).contains(&targets.commercial_vacancy_expansion_limit)
        && targets.housing_vacancy_expansion_limit.is_finite()
        && targets.commercial_vacancy_expansion_limit.is_finite()
}

#[cfg(any(test, feature = "legacy"))]
fn valid_weights(weights: ScoreWeights) -> bool {
    [
        weights.coverage,
        weights.floor_area,
        weights.frontage,
        weights.demand,
        weights.road_cost_scale_m,
    ]
    .into_iter()
    .all(|value| value.is_finite() && value >= 0.0)
        && weights.road_cost_scale_m > 0.0
}

fn ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

fn validate_grid(grid: GridSpec) -> Result<(usize, usize, usize), MeasureError> {
    if ![
        grid.min_x,
        grid.max_x,
        grid.min_z,
        grid.max_z,
        grid.cell_size,
    ]
    .into_iter()
    .all(f64::is_finite)
        || grid.max_x <= grid.min_x
        || grid.max_z <= grid.min_z
        || grid.cell_size <= 0.0
    {
        return Err(MeasureError::InvalidGrid);
    }
    let columns = ((grid.max_x - grid.min_x) / grid.cell_size).ceil() as usize;
    let rows = ((grid.max_z - grid.min_z) / grid.cell_size).ceil() as usize;
    let count = columns
        .checked_mul(rows)
        .ok_or(MeasureError::TooManyCells)?;
    if columns == 0 || rows == 0 || count > MAX_DISTRICT_CELLS {
        return Err(MeasureError::TooManyCells);
    }
    Ok((columns, rows, count))
}

fn sorted_unique<T: Copy>(samples: &[T], id: impl Fn(&T) -> u32) -> Result<Vec<T>, MeasureError> {
    let mut sorted = samples.to_vec();
    sorted.sort_by_key(|sample| id(sample));
    if sorted.windows(2).any(|pair| id(&pair[0]) == id(&pair[1])) {
        return Err(MeasureError::DuplicateId);
    }
    Ok(sorted)
}

fn accumulate_rect(
    cells: &mut [CellMetrics],
    columns: usize,
    grid: GridSpec,
    rect: OrientedRect,
    mut add: impl FnMut(&mut CellMetrics, f64, usize),
) -> Result<f64, MeasureError> {
    if !rect.finite_positive() {
        return Err(MeasureError::InvalidFootprint);
    }
    let polygon: Vec<(f64, f64)> = rect
        .corners()
        .into_iter()
        .map(|(x, z)| (x as f64, z as f64))
        .collect();
    if polygon
        .iter()
        .any(|(x, z)| !x.is_finite() || !z.is_finite())
    {
        return Err(MeasureError::NonFiniteGeometry);
    }
    let (min_x, max_x, min_z, max_z) = polygon_bounds(&polygon);
    let rows = cells.len() / columns;
    if max_x <= grid.min_x || min_x >= grid.max_x || max_z <= grid.min_z || min_z >= grid.max_z {
        return Ok(0.0);
    }
    let start_column = cell_index(min_x, grid.min_x, grid.cell_size, columns);
    let end_column = cell_index(max_x, grid.min_x, grid.cell_size, columns);
    let start_row = cell_index(min_z, grid.min_z, grid.cell_size, rows);
    let end_row = cell_index(max_z, grid.min_z, grid.cell_size, rows);
    let mut total = 0.0;
    for row in start_row..=end_row {
        for column in start_column..=end_column {
            let id = row * columns + column;
            let cell = &cells[id];
            let clipped = clip_to_rect(&polygon, cell.min_x, cell.max_x, cell.min_z, cell.max_z);
            let area = polygon_area(&clipped);
            if area > 0.0 {
                add(&mut cells[id], area, id);
                total += area;
            }
        }
    }
    Ok(total)
}

/// Allocate one proposed envelope across existing district cells. This returns
/// geometry only: callers must label any resulting benefit as a forecast and
/// must not add it to measured built coverage before construction completes.
#[cfg(feature = "legacy")]
pub(crate) fn clipped_envelope_areas(
    metrics: &DistrictMetrics,
    rect: OrientedRect,
) -> Result<Vec<(usize, f64)>, MeasureError> {
    if !rect.finite_positive() {
        return Err(MeasureError::InvalidFootprint);
    }
    if metrics.cells.len() > MAX_DISTRICT_CELLS {
        return Err(MeasureError::TooManyCells);
    }
    let polygon: Vec<_> = rect
        .corners()
        .into_iter()
        .map(|(x, z)| (x as f64, z as f64))
        .collect();
    if polygon
        .iter()
        .any(|(x, z)| !x.is_finite() || !z.is_finite())
    {
        return Err(MeasureError::NonFiniteGeometry);
    }
    let (min_x, max_x, min_z, max_z) = polygon_bounds(&polygon);
    let mut result = Vec::new();
    for (index, cell) in metrics.cells.iter().enumerate() {
        if cell.id != index {
            return Err(MeasureError::InvalidGrid);
        }
        if max_x <= cell.min_x || min_x >= cell.max_x || max_z <= cell.min_z || min_z >= cell.max_z
        {
            continue;
        }
        let area = polygon_area(&clip_to_rect(
            &polygon, cell.min_x, cell.max_x, cell.min_z, cell.max_z,
        ));
        if area > 0.0 {
            result.push((cell.id, area));
        }
    }
    Ok(result)
}

fn cell_index(value: f64, origin: f64, size: f64, len: usize) -> usize {
    (((value - origin) / size).floor() as isize).clamp(0, len as isize - 1) as usize
}

fn polygon_bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for &(x, z) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }
    (min_x, max_x, min_z, max_z)
}

fn clip_to_rect(
    polygon: &[(f64, f64)],
    min_x: f64,
    max_x: f64,
    min_z: f64,
    max_z: f64,
) -> Vec<(f64, f64)> {
    let mut clipped = polygon.to_vec();
    clipped = clip_edge(
        &clipped,
        |x, _| x >= min_x,
        |a, b| intersect_vertical(a, b, min_x),
    );
    clipped = clip_edge(
        &clipped,
        |x, _| x <= max_x,
        |a, b| intersect_vertical(a, b, max_x),
    );
    clipped = clip_edge(
        &clipped,
        |_, z| z >= min_z,
        |a, b| intersect_horizontal(a, b, min_z),
    );
    clip_edge(
        &clipped,
        |_, z| z <= max_z,
        |a, b| intersect_horizontal(a, b, max_z),
    )
}

fn clip_edge(
    polygon: &[(f64, f64)],
    inside: impl Fn(f64, f64) -> bool,
    intersection: impl Fn((f64, f64), (f64, f64)) -> (f64, f64),
) -> Vec<(f64, f64)> {
    if polygon.is_empty() {
        return Vec::new();
    }
    let mut output = Vec::with_capacity(polygon.len() + 2);
    let mut previous = *polygon.last().unwrap();
    let mut previous_inside = inside(previous.0, previous.1);
    for &current in polygon {
        let current_inside = inside(current.0, current.1);
        match (previous_inside, current_inside) {
            (true, true) => output.push(current),
            (true, false) => output.push(intersection(previous, current)),
            (false, true) => {
                output.push(intersection(previous, current));
                output.push(current);
            }
            (false, false) => {}
        }
        previous = current;
        previous_inside = current_inside;
    }
    output
}

fn intersect_vertical(a: (f64, f64), b: (f64, f64), x: f64) -> (f64, f64) {
    let dx = b.0 - a.0;
    if dx.abs() <= f64::EPSILON {
        (x, a.1)
    } else {
        let t = ((x - a.0) / dx).clamp(0.0, 1.0);
        (x, a.1 + (b.1 - a.1) * t)
    }
}

fn intersect_horizontal(a: (f64, f64), b: (f64, f64), z: f64) -> (f64, f64) {
    let dz = b.1 - a.1;
    if dz.abs() <= f64::EPSILON {
        (a.0, z)
    } else {
        let t = ((z - a.1) / dz).clamp(0.0, 1.0);
        (a.0 + (b.0 - a.0) * t, z)
    }
}

fn polygon_area(points: &[(f64, f64)]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut twice_area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        twice_area += points[index].0 * points[next].1 - points[next].0 * points[index].1;
    }
    (twice_area * 0.5).abs()
}

fn accumulate_segment(
    cells: &mut [CellMetrics],
    grid: GridSpec,
    frontage: FrontageSample,
    mut add: impl FnMut(&mut CellMetrics, f64),
) {
    let (dx, dz) = (
        frontage.end_x - frontage.start_x,
        frontage.end_z - frontage.start_z,
    );
    let length = dx.hypot(dz);
    if length <= 0.0 || !length.is_finite() {
        return;
    }
    // Clip each finite cell independently. Grid size and input count are capped.
    for cell in cells {
        // Treat cells as half-open at their maximum edges (except the outer
        // domain edge) so a frontage exactly on a grid line belongs once.
        let on_max_x = frontage.start_x == cell.max_x && frontage.end_x == cell.max_x;
        let on_max_z = frontage.start_z == cell.max_z && frontage.end_z == cell.max_z;
        if (on_max_x && cell.max_x < grid.max_x) || (on_max_z && cell.max_z < grid.max_z) {
            continue;
        }
        let Some((start, end)) = clip_line_to_rect(
            (frontage.start_x, frontage.start_z),
            (frontage.end_x, frontage.end_z),
            (cell.min_x, cell.max_x, cell.min_z, cell.max_z),
        ) else {
            continue;
        };
        let part = (end.0 - start.0).hypot(end.1 - start.1);
        if part > 0.0 {
            add(cell, part);
        }
    }
}

fn clip_line_to_rect(
    start: (f64, f64),
    end: (f64, f64),
    bounds: (f64, f64, f64, f64),
) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dz) = (end.0 - start.0, end.1 - start.1);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, start.0 - bounds.0),
        (dx, bounds.1 - start.0),
        (-dz, start.1 - bounds.2),
        (dz, bounds.3 - start.1),
    ] {
        if p.abs() <= f64::EPSILON {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let ratio = q / p;
        if p < 0.0 {
            t0 = t0.max(ratio);
        } else {
            t1 = t1.min(ratio);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((
        (start.0 + dx * t0, start.1 + dz * t0),
        (start.0 + dx * t1, start.1 + dz * t1),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forecast_envelope_allocation_conserves_area_across_cells_and_world_edge() {
        let metrics = measure(grid(), &[], &[]).unwrap();
        let before = metrics.clone();
        let across = clipped_envelope_areas(&metrics, rect(0.0, 0.0, 20.0, 40.0, 0.37)).unwrap();
        assert_eq!(across.len(), 4);
        assert!((across.iter().map(|(_, area)| area).sum::<f64>() - 800.0).abs() < 0.001);
        let edge = clipped_envelope_areas(&metrics, rect(100.0, -50.0, 20.0, 20.0, 0.0)).unwrap();
        assert_eq!(edge, vec![(1, 200.0)]);
        let touching =
            clipped_envelope_areas(&metrics, rect(110.0, -50.0, 20.0, 20.0, 0.0)).unwrap();
        assert!(touching.is_empty());
        assert_eq!(
            metrics, before,
            "forecast allocation must not become measured development"
        );
    }

    fn grid() -> GridSpec {
        GridSpec {
            min_x: -100.0,
            max_x: 100.0,
            min_z: -100.0,
            max_z: 100.0,
            cell_size: 100.0,
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

    fn targets() -> DistrictTargets {
        DistrictTargets {
            built_envelope_coverage_proxy: TargetRange {
                min: 0.25,
                max: 0.3,
            },
            floor_area_ratio_proxy: TargetRange { min: 0.9, max: 1.0 },
            occupied_frontage_m: TargetRange {
                min: 70.0,
                max: 80.0,
            },
            housing_vacancy_expansion_limit: 0.2,
            commercial_vacancy_expansion_limit: 0.2,
        }
    }

    fn weights() -> ScoreWeights {
        ScoreWeights {
            coverage: 1.0,
            floor_area: 1.0,
            frontage: 1.0,
            demand: 1.0,
            road_cost_scale_m: 100.0,
        }
    }

    fn sample_buildings(count: usize) -> Vec<BuildingSample> {
        (0..count)
            .map(|id| BuildingSample {
                id: (count - id) as u32,
                footprint: rect(
                    -72.03125 + id as f32 * 7.0,
                    -68.0625 + id as f32 * 5.0,
                    9.125,
                    7.375,
                    id as f32 * 0.07125,
                ),
                floors: (id % 5 + 1) as u8,
                use_class: if id % 2 == 0 {
                    BuildingUse::Housing
                } else {
                    BuildingUse::Commercial
                },
                open: id % 4 != 0,
                resident_occupancy: if id % 2 == 0 { (id % 6) as u16 } else { 0 },
                capacity: 8,
                business_active: id % 3 == 0,
            })
            .collect()
    }

    fn sample_frontages(count: usize) -> Vec<FrontageSample> {
        (0..count)
            .map(|id| FrontageSample {
                id: (count - id) as u32,
                start_x: -95.123456789 + id as f64 * 3.0,
                start_z: -80.234567891 + id as f64 * 1.5,
                end_x: -90.345678912 + id as f64 * 3.0,
                end_z: -76.456789123 + id as f64 * 1.5,
                occupied: id % 2 == 0,
            })
            .collect()
    }

    #[test]
    fn incremental_measurement_resumes_every_sample_boundary_from_frozen_inputs() {
        let mut buildings = sample_buildings(20);
        let mut frontages = sample_frontages(20);
        let expected = measure(grid(), &buildings, &frontages).unwrap();
        let mut accumulator =
            MeasurementAccumulator::begin(grid(), &buildings, &frontages).unwrap();
        assert!(accumulator.metrics().is_none());
        assert_eq!(accumulator.advance(0).unwrap(), 0);

        // The job must retain its own immutable view even when the live inputs change.
        buildings[0].floors = 0;
        frontages[0].start_x = f64::NAN;
        while !accumulator.is_complete() {
            assert_eq!(accumulator.advance(1).unwrap(), 1);
            let checkpoint_json =
                serde_json::to_string(&accumulator.checkpoint().unwrap()).unwrap();
            let checkpoint: MeasurementCheckpoint = serde_json::from_str(&checkpoint_json).unwrap();
            let resumed = MeasurementAccumulator::resume(checkpoint).unwrap();
            assert_eq!(resumed, accumulator);
            accumulator = resumed;
        }
        assert_eq!(accumulator.metrics(), Some(&expected));
        assert_eq!(accumulator.finish().unwrap(), expected);
    }

    #[test]
    fn resume_rejects_malformed_checkpoint_metadata_and_unprocessed_samples() {
        let buildings = sample_buildings(3);
        let frontages = sample_frontages(3);
        let base = MeasurementAccumulator::begin(grid(), &buildings, &frontages)
            .unwrap()
            .checkpoint()
            .unwrap();

        let mut invalid_count = base.clone();
        invalid_count.processed_samples = buildings.len() + frontages.len() + 1;
        assert_eq!(
            MeasurementAccumulator::resume(invalid_count),
            Err(MeasureError::IncompleteMeasurement)
        );

        let mut too_many = base.clone();
        too_many.buildings = sample_buildings(MAX_DENSITY_BUILDINGS + 1);
        assert_eq!(
            MeasurementAccumulator::resume(too_many),
            Err(MeasureError::TooManySamples)
        );

        let mut too_many_frontages = base.clone();
        too_many_frontages.frontages = sample_frontages(MAX_DENSITY_FRONTAGES + 1);
        assert_eq!(
            MeasurementAccumulator::resume(too_many_frontages),
            Err(MeasureError::TooManySamples)
        );

        let mut duplicate_building = base.clone();
        duplicate_building.buildings[1].id = duplicate_building.buildings[0].id;
        assert_eq!(
            MeasurementAccumulator::resume(duplicate_building),
            Err(MeasureError::DuplicateId)
        );

        let mut duplicate_frontage = base.clone();
        duplicate_frontage.frontages[1].id = duplicate_frontage.frontages[0].id;
        assert_eq!(
            MeasurementAccumulator::resume(duplicate_frontage),
            Err(MeasureError::DuplicateId)
        );

        // Malformed samples beyond the zero-length processed prefix are still rejected.
        let mut nonfinite_building = base.clone();
        nonfinite_building.processed_samples = 0;
        nonfinite_building.buildings[2].footprint.rotation = f32::NAN;
        assert_eq!(
            MeasurementAccumulator::resume(nonfinite_building),
            Err(MeasureError::NonFiniteGeometry)
        );

        let mut nonfinite_frontage = base;
        nonfinite_frontage.processed_samples = 0;
        nonfinite_frontage.frontages[2].end_z = f64::INFINITY;
        assert_eq!(
            MeasurementAccumulator::resume(nonfinite_frontage),
            Err(MeasureError::NonFiniteGeometry)
        );
    }

    #[test]
    fn measurement_advance_clamps_work_and_zero_budget_is_a_noop() {
        let buildings = sample_buildings(20);
        let mut accumulator = MeasurementAccumulator::begin(grid(), &buildings, &[]).unwrap();
        assert_eq!(accumulator.advance(0).unwrap(), 0);
        assert_eq!(accumulator.processed_samples(), 0);
        assert_eq!(
            accumulator.advance(usize::MAX).unwrap(),
            MAX_MEASUREMENT_SAMPLES_PER_ADVANCE
        );
        assert_eq!(
            accumulator.processed_samples(),
            MAX_MEASUREMENT_SAMPLES_PER_ADVANCE
        );
    }

    #[test]
    fn invalid_incremental_samples_fail_terminally_and_hide_partial_metrics() {
        let invalid = [BuildingSample {
            id: 1,
            footprint: rect(0.0, 0.0, 10.0, 10.0, 0.0),
            floors: 0,
            use_class: BuildingUse::Other,
            open: true,
            resident_occupancy: 0,
            capacity: 0,
            business_active: false,
        }];
        let mut accumulator = MeasurementAccumulator::begin(grid(), &invalid, &[]).unwrap();
        assert_eq!(accumulator.advance(1), Err(MeasureError::InvalidBuilding));
        assert_eq!(accumulator.advance(1), Err(MeasureError::InvalidBuilding));
        assert!(accumulator.metrics().is_none());
        assert_eq!(accumulator.checkpoint(), Err(MeasureError::InvalidBuilding));
    }

    #[test]
    fn rotated_envelopes_are_clipped_to_fixed_district_cells() {
        let building = BuildingSample {
            id: 1,
            footprint: rect(0.0, 0.0, 40.0, 20.0, std::f32::consts::FRAC_PI_4),
            floors: 1,
            use_class: BuildingUse::Other,
            open: true,
            resident_occupancy: 0,
            capacity: 0,
            business_active: false,
        };
        let metrics = measure(grid(), &[building], &[]).unwrap();
        assert_eq!((metrics.columns, metrics.rows), (2, 2));
        let total: f64 = metrics
            .cells
            .iter()
            .map(|cell| cell.built_envelope_area_proxy)
            .sum();
        assert!((total - 800.0).abs() < 0.01);
        assert!(metrics.cells.iter().all(|cell| cell.id < 4));
        assert!(
            metrics
                .cells
                .iter()
                .filter(|cell| cell.built_envelope_area_proxy > 0.0)
                .count()
                == 4
        );
    }

    #[test]
    fn district_clipping_keeps_gross_denominator_and_clips_world_edges() {
        let edge_grid = GridSpec {
            min_x: -100.0,
            max_x: 70.0,
            min_z: -100.0,
            max_z: 60.0,
            cell_size: 100.0,
        };
        let building = BuildingSample {
            id: 1,
            footprint: rect(65.0, 55.0, 30.0, 30.0, 0.0),
            floors: 1,
            use_class: BuildingUse::Other,
            open: true,
            resident_occupancy: 0,
            capacity: 0,
            business_active: false,
        };
        let metrics = measure(edge_grid, &[building], &[]).unwrap();
        assert_eq!((metrics.columns, metrics.rows), (2, 2));
        let edge = metrics.cell(3).unwrap();
        assert!((edge.gross_area - 4_200.0).abs() < 1e-9);
        assert!((edge.built_envelope_area_proxy - 400.0).abs() < 1e-6);
    }

    #[test]
    fn invalid_and_overlarge_grids_fail_closed() {
        let invalid = GridSpec {
            cell_size: 0.0,
            ..grid()
        };
        assert_eq!(
            measure(invalid, &[], &[]).unwrap_err(),
            MeasureError::InvalidGrid
        );
        let overlarge = GridSpec {
            min_x: 0.0,
            max_x: 800.0,
            min_z: 0.0,
            max_z: 800.0,
            cell_size: 10.0,
        };
        assert_eq!(
            measure(overlarge, &[], &[]).unwrap_err(),
            MeasureError::TooManyCells
        );
        let invalid_building = BuildingSample {
            id: 1,
            footprint: rect(0.0, 0.0, -2.0, 4.0, 0.0),
            floors: 1,
            use_class: BuildingUse::Other,
            open: true,
            resident_occupancy: 0,
            capacity: 0,
            business_active: false,
        };
        assert_eq!(
            measure(grid(), &[invalid_building], &[]).unwrap_err(),
            MeasureError::InvalidFootprint
        );
    }

    #[test]
    fn grid_line_frontage_is_attributed_once_and_clipped_to_domain() {
        let frontage = FrontageSample {
            id: 1,
            start_x: 0.0,
            start_z: -150.0,
            end_x: 0.0,
            end_z: 150.0,
            occupied: true,
        };
        let metrics = measure(grid(), &[], &[frontage]).unwrap();
        let total: f64 = metrics
            .cells
            .iter()
            .map(|cell| cell.usable_frontage_m)
            .sum();
        let occupied: f64 = metrics
            .cells
            .iter()
            .map(|cell| cell.occupied_frontage_m)
            .sum();
        assert!((total - 200.0).abs() < 1e-9);
        assert!((occupied - 200.0).abs() < 1e-9);
    }

    #[test]
    fn building_floor_area_and_housing_commercial_vacancy_are_separate() {
        let buildings = [
            BuildingSample {
                id: 2,
                footprint: rect(20.0, 20.0, 20.0, 20.0, 0.0),
                floors: 4,
                use_class: BuildingUse::Housing,
                open: true,
                resident_occupancy: 6,
                capacity: 10,
                business_active: false,
            },
            BuildingSample {
                id: 1,
                footprint: rect(30.0, 20.0, 20.0, 20.0, 0.0),
                floors: 2,
                use_class: BuildingUse::Commercial,
                open: true,
                resident_occupancy: 0,
                capacity: 8,
                business_active: false,
            },
        ];
        let metrics = measure(grid(), &buildings, &[]).unwrap();
        let reordered = [buildings[1], buildings[0]];
        assert_eq!(metrics, measure(grid(), &reordered, &[]).unwrap());
        let cell = metrics.cell(3).unwrap();
        assert!((cell.built_envelope_area_proxy - 800.0).abs() < 1e-6);
        assert!((cell.floor_area_proxy - 2_400.0).abs() < 1e-6);
        assert!((cell.housing_vacant_capacity - 4.0).abs() < 1e-6);
        assert_eq!(cell.intersecting_inactive_commercial_buildings, 1);
        assert_eq!(cell.intersecting_active_commercial_buildings, 0);
    }

    #[test]
    fn vacant_capacity_suppresses_expansion_but_keeps_infill_ranked() {
        let buildings = [BuildingSample {
            id: 1,
            footprint: rect(20.0, 20.0, 20.0, 20.0, 0.0),
            floors: 2,
            use_class: BuildingUse::Housing,
            open: true,
            resident_occupancy: 0,
            capacity: 10,
            business_active: false,
        }];
        let metrics = measure(grid(), &buildings, &[]).unwrap();
        let candidates = [
            ProjectCandidate {
                stable_id: 8,
                kind: CandidateKind::Expansion,
                cell_id: 3,
                feasible: true,
                built_envelope_gain_proxy: 200.0,
                floor_area_gain_proxy: 400.0,
                frontage_gain_m: 20.0,
                road_cost_m: 10.0,
                demand_fit: 1.0,
                housing_capacity_gain: 20.0,
                commercial_capacity_gain: 0.0,
            },
            ProjectCandidate {
                stable_id: 5,
                kind: CandidateKind::Infill,
                cell_id: 3,
                feasible: true,
                built_envelope_gain_proxy: 100.0,
                floor_area_gain_proxy: 200.0,
                frontage_gain_m: 10.0,
                road_cost_m: 0.0,
                demand_fit: 0.5,
                housing_capacity_gain: 0.0,
                commercial_capacity_gain: 0.0,
            },
        ];
        let ranked = rank_candidates(&metrics, &candidates, targets(), weights()).unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].stable_id, 5);
        assert_eq!(ranked[0].kind, CandidateKind::Infill);
    }

    #[test]
    fn zero_vacancy_limit_does_not_suppress_expansion_when_cell_has_no_capacity() {
        let metrics = measure(grid(), &[], &[]).unwrap();
        let candidate = ProjectCandidate {
            stable_id: 9,
            kind: CandidateKind::Expansion,
            cell_id: 0,
            feasible: true,
            built_envelope_gain_proxy: 100.0,
            floor_area_gain_proxy: 200.0,
            frontage_gain_m: 10.0,
            road_cost_m: 20.0,
            demand_fit: 0.8,
            housing_capacity_gain: 20.0,
            commercial_capacity_gain: 0.0,
        };
        let no_vacancy_target = DistrictTargets {
            housing_vacancy_expansion_limit: 0.0,
            commercial_vacancy_expansion_limit: 0.0,
            ..targets()
        };
        let ranked = rank_candidates(&metrics, &[candidate], no_vacancy_target, weights()).unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].stable_id, 9);
    }

    #[test]
    fn candidate_ranking_is_stable_and_rejects_infeasible_projects() {
        let metrics = measure(grid(), &[], &[]).unwrap();
        let make = |stable_id, kind| ProjectCandidate {
            stable_id,
            kind,
            cell_id: 0,
            feasible: true,
            built_envelope_gain_proxy: 100.0,
            floor_area_gain_proxy: 200.0,
            frontage_gain_m: 10.0,
            road_cost_m: 0.0,
            demand_fit: 0.5,
            housing_capacity_gain: 0.0,
            commercial_capacity_gain: 0.0,
        };
        let mut candidates = vec![
            make(8, CandidateKind::Expansion),
            make(5, CandidateKind::InteriorAccess),
            make(4, CandidateKind::Infill),
            ProjectCandidate {
                feasible: false,
                ..make(1, CandidateKind::Infill)
            },
        ];
        let first = rank_candidates(&metrics, &candidates, targets(), weights()).unwrap();
        candidates.reverse();
        let second = rank_candidates(&metrics, &candidates, targets(), weights()).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .map(|entry| entry.stable_id)
                .collect::<Vec<_>>(),
            vec![4, 5, 8]
        );
    }

    #[test]
    fn candidate_ranking_uses_the_targets_and_metrics_of_its_district_cell() {
        let mut metrics = measure(grid(), &[], &[]).unwrap();
        // Cell 0 has already met its lower coverage target; cell 1 is empty.
        metrics.cells[0].built_envelope_area_proxy = 2_500.0;
        let local_targets = [
            DistrictTargets {
                built_envelope_coverage_proxy: TargetRange {
                    min: 0.25,
                    max: 0.4,
                },
                ..targets()
            },
            DistrictTargets {
                built_envelope_coverage_proxy: TargetRange { min: 0.1, max: 0.2 },
                ..targets()
            },
            targets(),
            targets(),
        ];
        let make = |stable_id, cell_id| ProjectCandidate {
            stable_id,
            kind: CandidateKind::Infill,
            cell_id,
            feasible: true,
            built_envelope_gain_proxy: 1_000.0,
            floor_area_gain_proxy: 0.0,
            frontage_gain_m: 0.0,
            road_cost_m: 0.0,
            demand_fit: 0.2,
            housing_capacity_gain: 0.0,
            commercial_capacity_gain: 0.0,
        };
        let ranked = rank_candidates_by_district(
            &metrics,
            &[make(10, 0), make(20, 1)],
            &local_targets,
            weights(),
        )
        .unwrap();
        assert_eq!(
            ranked
                .iter()
                .map(|entry| entry.stable_id)
                .collect::<Vec<_>>(),
            vec![20, 10]
        );
        assert!(ranked[0].score > ranked[1].score);
    }

    #[test]
    fn district_target_count_and_candidate_cell_indices_are_exact() {
        let metrics = measure(grid(), &[], &[]).unwrap();
        assert_eq!(
            rank_candidates_by_district(&metrics, &[], &[targets(); 3], weights()).unwrap_err(),
            ScoreError::TargetCountMismatch
        );
        let invalid_cell = ProjectCandidate {
            stable_id: 1,
            kind: CandidateKind::Infill,
            cell_id: metrics.cells.len(),
            feasible: true,
            built_envelope_gain_proxy: 1.0,
            floor_area_gain_proxy: 1.0,
            frontage_gain_m: 1.0,
            road_cost_m: 0.0,
            demand_fit: 1.0,
            housing_capacity_gain: 0.0,
            commercial_capacity_gain: 0.0,
        };
        assert_eq!(
            rank_candidates_by_district(
                &metrics,
                &[invalid_cell],
                &vec![targets(); metrics.cells.len()],
                weights(),
            )
            .unwrap_err(),
            ScoreError::InvalidCandidate
        );
    }

    #[test]
    fn uniform_wrapper_matches_repeated_per_cell_targets() {
        let metrics = measure(grid(), &[], &[]).unwrap();
        let candidates = [
            ProjectCandidate {
                stable_id: 12,
                kind: CandidateKind::Expansion,
                cell_id: 0,
                feasible: true,
                built_envelope_gain_proxy: 100.0,
                floor_area_gain_proxy: 200.0,
                frontage_gain_m: 10.0,
                road_cost_m: 8.0,
                demand_fit: 0.5,
                housing_capacity_gain: 0.0,
                commercial_capacity_gain: 0.0,
            },
            ProjectCandidate {
                stable_id: 4,
                kind: CandidateKind::Infill,
                cell_id: 3,
                feasible: true,
                built_envelope_gain_proxy: 100.0,
                floor_area_gain_proxy: 200.0,
                frontage_gain_m: 10.0,
                road_cost_m: 8.0,
                demand_fit: 0.5,
                housing_capacity_gain: 0.0,
                commercial_capacity_gain: 0.0,
            },
        ];
        let repeated = vec![targets(); metrics.cells.len()];
        assert_eq!(
            rank_candidates(&metrics, &candidates, targets(), weights()).unwrap(),
            rank_candidates_by_district(&metrics, &candidates, &repeated, weights()).unwrap(),
        );
    }

    #[test]
    fn invalid_target_ranges_and_candidate_overflow_fail_closed() {
        let metrics = measure(grid(), &[], &[]).unwrap();
        let invalid_targets = DistrictTargets {
            built_envelope_coverage_proxy: TargetRange { min: 0.4, max: 0.3 },
            ..targets()
        };
        assert_eq!(
            rank_candidates(&metrics, &[], invalid_targets, weights()).unwrap_err(),
            ScoreError::InvalidConfiguration
        );
        let candidate = ProjectCandidate {
            stable_id: 1,
            kind: CandidateKind::Infill,
            cell_id: 0,
            feasible: true,
            built_envelope_gain_proxy: 1.0,
            floor_area_gain_proxy: 1.0,
            frontage_gain_m: 1.0,
            road_cost_m: 0.0,
            demand_fit: 1.0,
            housing_capacity_gain: 0.0,
            commercial_capacity_gain: 0.0,
        };
        let candidates = vec![candidate; MAX_PROJECT_CANDIDATES + 1];
        assert_eq!(
            rank_candidates(&metrics, &candidates, targets(), weights()).unwrap_err(),
            ScoreError::TooManyCandidates
        );
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let metrics = measure(grid(), &[], &[]).unwrap();
        let candidate = ProjectCandidate {
            stable_id: 7,
            kind: CandidateKind::Infill,
            cell_id: 0,
            feasible: true,
            built_envelope_gain_proxy: 1.0,
            floor_area_gain_proxy: 1.0,
            frontage_gain_m: 1.0,
            road_cost_m: 0.0,
            demand_fit: 1.0,
            housing_capacity_gain: 0.0,
            commercial_capacity_gain: 0.0,
        };
        assert_eq!(
            rank_candidates(&metrics, &[candidate, candidate], targets(), weights()).unwrap_err(),
            ScoreError::DuplicateCandidateId
        );
    }

    #[test]
    fn work_caps_match_world_entity_bounds() {
        let building = BuildingSample {
            id: 0,
            footprint: rect(10.0, 10.0, 2.0, 2.0, 0.0),
            floors: 1,
            use_class: BuildingUse::Other,
            open: true,
            resident_occupancy: 0,
            capacity: 0,
            business_active: false,
        };
        let buildings: Vec<_> = (0..MAX_DENSITY_BUILDINGS)
            .map(|id| BuildingSample {
                id: id as u32,
                ..building
            })
            .collect();
        assert!(measure(grid(), &buildings, &[]).is_ok());
        let too_many: Vec<_> = (0..=MAX_DENSITY_BUILDINGS)
            .map(|id| BuildingSample {
                id: id as u32,
                ..building
            })
            .collect();
        assert_eq!(
            measure(grid(), &too_many, &[]).unwrap_err(),
            MeasureError::TooManySamples
        );

        let frontage = FrontageSample {
            id: 0,
            start_x: -50.0,
            start_z: 0.0,
            end_x: 50.0,
            end_z: 0.0,
            occupied: false,
        };
        let frontages: Vec<_> = (0..MAX_DENSITY_FRONTAGES)
            .map(|id| FrontageSample {
                id: id as u32,
                ..frontage
            })
            .collect();
        assert!(measure(grid(), &[], &frontages).is_ok());
        let too_many: Vec<_> = (0..=MAX_DENSITY_FRONTAGES)
            .map(|id| FrontageSample {
                id: id as u32,
                ..frontage
            })
            .collect();
        assert_eq!(
            measure(grid(), &[], &too_many).unwrap_err(),
            MeasureError::TooManySamples
        );
    }
}
