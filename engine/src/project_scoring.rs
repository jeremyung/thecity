//! Deterministic multi-cell project ranking over explicit forecast gains.
//!
//! `DistrictMetrics` describes a frozen measured city. Candidate gains are
//! forecasts supplied by the caller and are never written into those metrics.
//! The caller is responsible for allocating a project's geometry consistently
//! across cells; this module checks for duplicate cell entries but does not
//! attempt to reconstruct or revalidate geometry conservation from summaries.

#[cfg(feature = "legacy")]
use crate::district_density::{
    CandidateKind, DistrictTargets, ScoreWeights, MAX_PROJECT_CANDIDATES,
};
use crate::district_density::{DistrictMetrics, MAX_DISTRICT_CELLS};

#[cfg(feature = "legacy")]
const MAX_IMPACTS_PER_PROJECT: usize = MAX_DISTRICT_CELLS;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct ForecastCellImpact {
    pub cell_id: usize,
    /// Forecast increment to the nominal envelope-area proxy, in m².
    pub forecast_envelope_gain_m2: f64,
    /// Forecast increment to the nominal floor-area proxy, in m².
    pub forecast_floor_area_gain_m2: f64,
    /// Forecast increment to occupied frontage, in meters.
    pub forecast_occupied_frontage_gain_m: f64,
    /// Forecast housing capacity increment. Used for vacancy relevance only.
    pub forecast_housing_capacity_gain: f64,
    /// Forecast commercial capacity increment. Used for vacancy relevance only.
    pub forecast_commercial_capacity_gain: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct LiveCatchment {
    /// Current served housing capacity, aggregated by the live caller.
    pub housing_capacity: f64,
    /// Current vacant housing capacity in that same served catchment.
    pub housing_vacant_capacity: f64,
    /// Current served commercial capacity, aggregated by the live caller.
    pub commercial_capacity: f64,
    /// Current vacant commercial capacity in that same served catchment.
    pub commercial_vacant_capacity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct VacancyPolicy {
    /// Expansion is suppressed at or above this served-catchment vacancy ratio.
    pub housing_limit: f64,
    /// Expansion is suppressed at or above this served-catchment vacancy ratio.
    pub commercial_limit: f64,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct ProjectScoreCandidate {
    pub stable_id: u64,
    pub kind: CandidateKind,
    /// Geometry, access, economic, and resource gates have accepted it.
    pub feasible: bool,
    /// One-time project road cost; do not repeat it in cell impacts.
    pub road_cost_m: f64,
    /// One-time project demand fit in [0, 1].
    pub demand_fit: f64,
    /// The live served catchment is calculated by the caller, independently
    /// of the possibly stale destination-cell measurement cache.
    pub live_catchment: LiveCatchment,
    pub cell_impacts: Vec<ForecastCellImpact>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct RankedProject {
    pub stable_id: u64,
    pub kind: CandidateKind,
    pub score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) enum ProjectScoreError {
    TooManyProjects,
    TooManyImpacts,
    InvalidConfiguration,
    InvalidProject,
    DuplicateProjectId,
    DuplicateCellImpact,
    InvalidCell,
    TargetCountMismatch,
}

/// Rank at most 128 feasible, already-gated projects. Cell benefits are
/// accumulated against each cell's own deficit. Road cost and demand are each
/// applied once per project. Expansion gates use only `live_catchment`, never
/// destination-cell vacancy. Frontage weighting can be set to zero until a
/// reliable frontage measurement is available.
#[cfg(feature = "legacy")]
pub(crate) fn rank_projects(
    metrics: &DistrictMetrics,
    candidates: &[ProjectScoreCandidate],
    targets: &[DistrictTargets],
    vacancy_policy: VacancyPolicy,
    weights: ScoreWeights,
) -> Result<Vec<RankedProject>, ProjectScoreError> {
    if candidates.len() > MAX_PROJECT_CANDIDATES {
        return Err(ProjectScoreError::TooManyProjects);
    }
    if targets.len() != metrics.cells.len() {
        return Err(ProjectScoreError::TargetCountMismatch);
    }
    if !valid_weights(weights)
        || !finite_nonnegative(vacancy_policy.housing_limit)
        || !finite_nonnegative(vacancy_policy.commercial_limit)
        || vacancy_policy.housing_limit > 1.0
        || vacancy_policy.commercial_limit > 1.0
        || !targets.iter().copied().all(valid_targets)
    {
        return Err(ProjectScoreError::InvalidConfiguration);
    }

    let mut ids: Vec<u64> = candidates
        .iter()
        .map(|candidate| candidate.stable_id)
        .collect();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ProjectScoreError::DuplicateProjectId);
    }

    let mut ranked = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !finite_nonnegative(candidate.road_cost_m)
            || !candidate.demand_fit.is_finite()
            || !(0.0..=1.0).contains(&candidate.demand_fit)
            || !valid_catchment(candidate.live_catchment)
        {
            return Err(ProjectScoreError::InvalidProject);
        }
        if candidate.cell_impacts.len() > MAX_IMPACTS_PER_PROJECT {
            return Err(ProjectScoreError::TooManyImpacts);
        }

        let mut impacts: Vec<ForecastCellImpact> = candidate.cell_impacts.clone();
        impacts.sort_by_key(|impact| impact.cell_id);
        if impacts
            .windows(2)
            .any(|pair| pair[0].cell_id == pair[1].cell_id)
        {
            return Err(ProjectScoreError::DuplicateCellImpact);
        }
        for impact in &impacts {
            if impact.cell_id >= metrics.cells.len()
                || metrics.cells[impact.cell_id].id != impact.cell_id
            {
                return Err(ProjectScoreError::InvalidCell);
            }
            if ![
                impact.forecast_envelope_gain_m2,
                impact.forecast_floor_area_gain_m2,
                impact.forecast_occupied_frontage_gain_m,
                impact.forecast_housing_capacity_gain,
                impact.forecast_commercial_capacity_gain,
            ]
            .into_iter()
            .all(finite_nonnegative)
            {
                return Err(ProjectScoreError::InvalidProject);
            }
        }

        if !candidate.feasible {
            continue;
        }
        if candidate.kind == CandidateKind::Expansion
            && expansion_blocked(candidate, vacancy_policy)
        {
            continue;
        }

        let mut benefit = weights.demand * candidate.demand_fit;
        for impact in impacts {
            let cell = &metrics.cells[impact.cell_id];
            let target = targets[impact.cell_id];
            let coverage_min = cell.gross_area * target.built_envelope_coverage_proxy.min;
            let floor_min = cell.gross_area * target.floor_area_ratio_proxy.min;
            let frontage_min = target.occupied_frontage_m.min;
            let coverage_need = (coverage_min - cell.built_envelope_area_proxy).max(0.0);
            let floor_need = (floor_min - cell.floor_area_proxy).max(0.0);
            let frontage_need = (frontage_min - cell.occupied_frontage_m).max(0.0);
            benefit += weights.coverage * impact.forecast_envelope_gain_m2.min(coverage_need)
                / coverage_min.max(1.0);
            benefit += weights.floor_area * impact.forecast_floor_area_gain_m2.min(floor_need)
                / floor_min.max(1.0);
            benefit += weights.frontage
                * impact.forecast_occupied_frontage_gain_m.min(frontage_need)
                / frontage_min.max(1.0);
        }
        let score = benefit / (1.0 + candidate.road_cost_m / weights.road_cost_scale_m);
        if !score.is_finite() {
            return Err(ProjectScoreError::InvalidProject);
        }
        ranked.push(RankedProject {
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

#[cfg(feature = "legacy")]
fn expansion_blocked(candidate: &ProjectScoreCandidate, policy: VacancyPolicy) -> bool {
    let catchment = candidate.live_catchment;
    (candidate
        .cell_impacts
        .iter()
        .any(|impact| impact.forecast_housing_capacity_gain > 0.0)
        && catchment.housing_vacant_capacity > 0.0
        && catchment.housing_vacant_capacity / catchment.housing_capacity >= policy.housing_limit)
        || (candidate
            .cell_impacts
            .iter()
            .any(|impact| impact.forecast_commercial_capacity_gain > 0.0)
            && catchment.commercial_vacant_capacity > 0.0
            && catchment.commercial_vacant_capacity / catchment.commercial_capacity
                >= policy.commercial_limit)
}

#[cfg(feature = "legacy")]
fn valid_catchment(catchment: LiveCatchment) -> bool {
    let fields = [
        catchment.housing_capacity,
        catchment.housing_vacant_capacity,
        catchment.commercial_capacity,
        catchment.commercial_vacant_capacity,
    ];
    fields.into_iter().all(finite_nonnegative)
        && catchment.housing_vacant_capacity <= catchment.housing_capacity
        && catchment.commercial_vacant_capacity <= catchment.commercial_capacity
}

#[cfg(feature = "legacy")]
fn valid_targets(targets: DistrictTargets) -> bool {
    let ranges = [
        targets.built_envelope_coverage_proxy,
        targets.floor_area_ratio_proxy,
        targets.occupied_frontage_m,
    ];
    ranges.into_iter().all(|range| {
        range.min.is_finite() && range.max.is_finite() && range.min >= 0.0 && range.max >= range.min
    })
}

#[cfg(feature = "legacy")]
fn valid_weights(weights: ScoreWeights) -> bool {
    [
        weights.coverage,
        weights.floor_area,
        weights.frontage,
        weights.demand,
    ]
    .into_iter()
    .all(finite_nonnegative)
        && weights.road_cost_scale_m.is_finite()
        && weights.road_cost_scale_m > 0.0
}

#[cfg(feature = "legacy")]
fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::district_density::{CellMetrics, TargetRange};

    fn targets(minimum: f64) -> DistrictTargets {
        DistrictTargets {
            built_envelope_coverage_proxy: TargetRange {
                min: minimum,
                max: minimum + 0.2,
            },
            floor_area_ratio_proxy: TargetRange {
                min: minimum * 2.0,
                max: minimum * 2.0 + 0.2,
            },
            occupied_frontage_m: TargetRange {
                min: 50.0,
                max: 80.0,
            },
            housing_vacancy_expansion_limit: 1.0,
            commercial_vacancy_expansion_limit: 1.0,
        }
    }

    fn metrics(cell_count: usize) -> DistrictMetrics {
        DistrictMetrics {
            columns: cell_count,
            rows: 1,
            cells: (0..cell_count)
                .map(|id| CellMetrics {
                    id,
                    column: id,
                    row: 0,
                    min_x: id as f64 * 100.0,
                    max_x: (id + 1) as f64 * 100.0,
                    min_z: 0.0,
                    max_z: 100.0,
                    gross_area: 10_000.0,
                    built_envelope_area_proxy: 0.0,
                    floor_area_proxy: 0.0,
                    housing_capacity: 900.0,
                    housing_occupied: 900.0,
                    housing_vacant_capacity: 0.0,
                    commercial_capacity: 700.0,
                    active_commercial_capacity: 700.0,
                    intersecting_active_commercial_buildings: 0,
                    intersecting_inactive_commercial_buildings: 0,
                    usable_frontage_m: 100.0,
                    occupied_frontage_m: 0.0,
                })
                .collect(),
        }
    }

    fn weights() -> ScoreWeights {
        ScoreWeights {
            coverage: 1.0,
            floor_area: 0.5,
            frontage: 0.0,
            demand: 0.25,
            road_cost_scale_m: 100.0,
        }
    }

    fn policy() -> VacancyPolicy {
        VacancyPolicy {
            housing_limit: 0.2,
            commercial_limit: 0.2,
        }
    }

    fn impact(cell_id: usize, envelope: f64) -> ForecastCellImpact {
        ForecastCellImpact {
            cell_id,
            forecast_envelope_gain_m2: envelope,
            forecast_floor_area_gain_m2: envelope * 2.0,
            forecast_occupied_frontage_gain_m: 10.0,
            forecast_housing_capacity_gain: 0.0,
            forecast_commercial_capacity_gain: 0.0,
        }
    }

    fn project(
        id: u64,
        kind: CandidateKind,
        impacts: Vec<ForecastCellImpact>,
    ) -> ProjectScoreCandidate {
        ProjectScoreCandidate {
            stable_id: id,
            kind,
            feasible: true,
            road_cost_m: 100.0,
            demand_fit: 0.8,
            live_catchment: LiveCatchment::default(),
            cell_impacts: impacts,
        }
    }

    #[test]
    fn sums_forecast_gains_against_each_districts_own_targets() {
        let world = metrics(2);
        let per_cell_targets = [targets(0.1), targets(0.02)];
        let candidate = project(
            1,
            CandidateKind::Infill,
            vec![impact(0, 300.0), impact(1, 300.0)],
        );
        let ranked =
            rank_projects(&world, &[candidate], &per_cell_targets, policy(), weights()).unwrap();
        // Cell 0 earns 30% toward its local deficit; cell 1 is capped at
        // 200 m² envelope and 400 m² floor-area gains.
        let score = ranked[0].score;
        let expected = (0.3 + 0.5 * 0.3 + 1.0 + 0.5 + 0.25 * 0.8) / 2.0;
        assert!((score - expected).abs() < 1e-12);
    }

    #[test]
    fn road_cost_and_demand_apply_once_for_a_multicell_project() {
        let world = metrics(2);
        let per_cell_targets = [targets(0.1); 2];
        let mut candidate = project(
            1,
            CandidateKind::Infill,
            vec![impact(0, 1_000.0), impact(1, 1_000.0)],
        );
        let ranked = rank_projects(
            &world,
            &[candidate.clone()],
            &per_cell_targets,
            policy(),
            weights(),
        )
        .unwrap();
        let one_cell = rank_projects(
            &world,
            &[project(2, CandidateKind::Infill, vec![impact(0, 1_000.0)])],
            &per_cell_targets,
            policy(),
            weights(),
        )
        .unwrap();
        assert!((ranked[0].score - one_cell[0].score * 2.0 + 0.1).abs() < 1e-12);
        // If either project-level term were repeated per cell, this equality fails.
        assert_eq!(ranked[0].score, 1.6);
        candidate.road_cost_m = 300.0;
        let costly =
            rank_projects(&world, &[candidate], &per_cell_targets, policy(), weights()).unwrap();
        assert!(costly[0].score < ranked[0].score);
    }

    #[test]
    fn expansion_uses_live_served_catchment_not_stale_destination_metrics() {
        let mut world = metrics(1);
        world.cells[0].housing_vacant_capacity = 800.0;
        let mut candidate = project(
            1,
            CandidateKind::Expansion,
            vec![ForecastCellImpact {
                forecast_housing_capacity_gain: 20.0,
                ..impact(0, 1_000.0)
            }],
        );
        candidate.live_catchment = LiveCatchment {
            housing_capacity: 100.0,
            housing_vacant_capacity: 0.0,
            commercial_capacity: 0.0,
            commercial_vacant_capacity: 0.0,
        };
        assert_eq!(
            rank_projects(
                &world,
                &[candidate.clone()],
                &[targets(0.1)],
                policy(),
                weights()
            )
            .unwrap()
            .len(),
            1
        );

        candidate.live_catchment.housing_vacant_capacity = 30.0;
        assert!(
            rank_projects(&world, &[candidate], &[targets(0.1)], policy(), weights())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn fractional_live_catchment_vacancy_uses_true_capacity_denominator() {
        let world = metrics(1);
        let candidate = ProjectScoreCandidate {
            live_catchment: LiveCatchment {
                housing_capacity: 0.5,
                housing_vacant_capacity: 0.25,
                ..LiveCatchment::default()
            },
            cell_impacts: vec![ForecastCellImpact {
                forecast_housing_capacity_gain: 1.0,
                ..impact(0, 1_000.0)
            }],
            ..project(1, CandidateKind::Expansion, Vec::new())
        };
        assert!(
            rank_projects(&world, &[candidate], &[targets(0.1)], policy(), weights())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn validates_caps_cells_duplicates_and_nonfinite_forecasts() {
        let world = metrics(1);
        let target = [targets(0.1)];
        let mut candidate = project(1, CandidateKind::Infill, vec![impact(0, 1.0)]);
        candidate.cell_impacts.push(impact(0, 2.0));
        assert_eq!(
            rank_projects(&world, &[candidate], &target, policy(), weights()),
            Err(ProjectScoreError::DuplicateCellImpact)
        );

        let mut invalid = project(1, CandidateKind::Infill, vec![impact(2, 1.0)]);
        assert_eq!(
            rank_projects(&world, &[invalid.clone()], &target, policy(), weights()),
            Err(ProjectScoreError::InvalidCell)
        );
        invalid.cell_impacts = vec![ForecastCellImpact {
            forecast_envelope_gain_m2: f64::NAN,
            ..impact(0, 1.0)
        }];
        assert_eq!(
            rank_projects(&world, &[invalid], &target, policy(), weights()),
            Err(ProjectScoreError::InvalidProject)
        );

        let too_many = project(
            2,
            CandidateKind::Infill,
            (0..=MAX_IMPACTS_PER_PROJECT)
                .map(|id| impact(id, 1.0))
                .collect(),
        );
        assert_eq!(
            rank_projects(&world, &[too_many], &target, policy(), weights()),
            Err(ProjectScoreError::TooManyImpacts)
        );

        let too_many_projects: Vec<_> = (0..=MAX_PROJECT_CANDIDATES)
            .map(|id| project(id as u64, CandidateKind::Infill, Vec::new()))
            .collect();
        assert_eq!(
            rank_projects(&world, &too_many_projects, &target, policy(), weights()),
            Err(ProjectScoreError::TooManyProjects)
        );

        assert_eq!(
            rank_projects(&world, &[], &[], policy(), weights()),
            Err(ProjectScoreError::TargetCountMismatch)
        );
    }

    #[test]
    fn stable_ties_use_kind_then_project_id() {
        let world = metrics(1);
        let candidates = [
            project(9, CandidateKind::Expansion, vec![impact(0, 100.0)]),
            project(4, CandidateKind::Infill, vec![impact(0, 100.0)]),
            project(2, CandidateKind::Infill, vec![impact(0, 100.0)]),
        ];
        let first =
            rank_projects(&world, &candidates, &[targets(0.1)], policy(), weights()).unwrap();
        let second =
            rank_projects(&world, &candidates, &[targets(0.1)], policy(), weights()).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .map(|entry| entry.stable_id)
                .collect::<Vec<_>>(),
            vec![2, 4, 9]
        );
    }
}
