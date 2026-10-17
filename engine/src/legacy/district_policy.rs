//! First-pass district-aware comparison for already feasible road projects.
//!
//! All coverage targets are provisional calibration values for the experiment;
//! they are not approved art targets or universal Tokyo density rules.

#[cfg(feature = "legacy")]
use crate::legacy::project_scoring::{LiveCatchment, VacancyPolicy};
use crate::population::district_density::DistrictMetrics;
#[cfg(feature = "legacy")]
use crate::population::district_density::{DistrictTargets, TargetRange};

#[cfg(feature = "legacy")]
pub(crate) const METRICS_MAX_AGE_TICKS: u64 = 400;
#[cfg(feature = "legacy")]
pub(crate) const FORECAST_UTILIZATION: f64 = 0.5;
#[cfg(feature = "legacy")]
pub(crate) const HOUSING_VACANCY_LIMIT: f64 = 0.25;
#[cfg(feature = "legacy")]
pub(crate) const COMMERCIAL_VACANCY_LIMIT: f64 = 0.35;
#[cfg(feature = "legacy")]
pub(crate) const MAX_FRONTIER_SITES: usize = 50;
#[cfg(feature = "legacy")]
pub(crate) const MAX_ACCESS_PROPOSALS: usize = 48;
#[cfg(feature = "legacy")]
pub(crate) const MAX_ACCESS_LOTS_PER_PROPOSAL: usize = 16;
#[cfg(feature = "legacy")]
pub(crate) const MAX_COMPARED_PROJECTS: usize = 1 + MAX_ACCESS_PROPOSALS;
#[cfg(any(test, feature = "legacy"))]
pub(crate) const MAX_FORECAST_RECTS: usize =
    MAX_FRONTIER_SITES + MAX_ACCESS_PROPOSALS * MAX_ACCESS_LOTS_PER_PROPOSAL;

/// Test-only evidence for calibration runs. It is deliberately absent from
/// production Engine state and save data.
#[cfg(test)]
#[derive(Clone, Debug, Default)]
pub(crate) struct ExperimentDiagnostics {
    pub comparisons: u32,
    pub frontier_proposals: u64,
    pub frontier_geometry_candidates: u64,
    pub frontier_source_feasible_candidates: u64,
    pub frontier_source_vacancy_rejections: u64,
    pub frontier_sites: u64,
    pub frontier_nominal_area_m2: f64,
    pub frontier_ranked: u64,
    pub frontier_top_score_sum: f64,
    pub frontier_top_score_count: u64,
    pub frontier_top_score_max: f64,
    pub access_proposals: u64,
    pub access_geometry_candidates: u64,
    pub access_source_feasible_candidates: u64,
    pub access_source_vacancy_rejections: u64,
    pub access_sites: u64,
    pub access_nominal_area_m2: f64,
    pub access_ranked: u64,
    pub access_top_score_sum: f64,
    pub access_top_score_count: u64,
    pub access_top_score_max: f64,
    pub paired_ranked_comparisons: u64,
    pub paired_frontier_score_wins: u64,
    pub paired_access_score_wins: u64,
    pub paired_score_ties: u64,
    pub paired_frontier_admissions: u64,
    pub paired_access_admissions: u64,
    pub paired_score_difference_sum: f64,
    pub frontier_wins: u64,
    pub access_wins: u64,
    pub no_geometry: u64,
    pub no_source_feasible: u64,
    pub no_cell_forecast: u64,
    pub no_metrics: u64,
    pub score_errors: u64,
    pub ranked_admission_rejections: u64,
    pub representative_comparisons: u32,
    pub records: Vec<DecisionRecord>,
}

#[cfg(test)]
#[derive(Clone, Debug, Default)]
pub(crate) struct DecisionRecord {
    pub comparison: u32,
    pub tick: u64,
    pub frontier_proposal: bool,
    pub frontier_geometry_candidates: usize,
    pub frontier_source_feasible_candidates: usize,
    pub frontier_sites: usize,
    pub frontier_area_m2: f64,
    pub frontier_source_rejected: bool,
    pub frontier_top_score: Option<f64>,
    pub frontier_ranked: usize,
    pub access_proposals: usize,
    pub access_geometry_candidates: usize,
    pub access_source_feasible_candidates: usize,
    pub access_sites: usize,
    pub access_area_m2: f64,
    pub access_vacancy_rejections: usize,
    pub access_top_score: Option<f64>,
    pub access_ranked: usize,
    pub winner: Option<&'static str>,
    pub no_admission_reason: Option<&'static str>,
}

#[cfg(test)]
impl ExperimentDiagnostics {
    pub(crate) fn observe(&mut self, record: DecisionRecord) {
        self.comparisons = self.comparisons.saturating_add(1);
        self.frontier_proposals += u64::from(record.frontier_proposal);
        self.frontier_geometry_candidates += record.frontier_geometry_candidates as u64;
        self.frontier_source_feasible_candidates +=
            record.frontier_source_feasible_candidates as u64;
        self.frontier_source_vacancy_rejections += u64::from(record.frontier_source_rejected);
        self.frontier_sites += record.frontier_sites as u64;
        self.frontier_nominal_area_m2 += record.frontier_area_m2;
        self.frontier_ranked += record.frontier_ranked as u64;
        if let Some(score) = record.frontier_top_score {
            self.frontier_top_score_sum += score;
            self.frontier_top_score_count += 1;
            self.frontier_top_score_max = self.frontier_top_score_max.max(score);
        }
        self.access_proposals += record.access_proposals as u64;
        self.access_geometry_candidates += record.access_geometry_candidates as u64;
        self.access_source_feasible_candidates += record.access_source_feasible_candidates as u64;
        self.access_source_vacancy_rejections += record.access_vacancy_rejections as u64;
        self.access_sites += record.access_sites as u64;
        self.access_nominal_area_m2 += record.access_area_m2;
        self.access_ranked += record.access_ranked as u64;
        if let Some(score) = record.access_top_score {
            self.access_top_score_sum += score;
            self.access_top_score_count += 1;
            self.access_top_score_max = self.access_top_score_max.max(score);
        }
        if let (Some(frontier), Some(access)) = (record.frontier_top_score, record.access_top_score)
        {
            self.paired_ranked_comparisons += 1;
            self.paired_score_difference_sum += frontier - access;
            match frontier.total_cmp(&access) {
                std::cmp::Ordering::Greater => self.paired_frontier_score_wins += 1,
                std::cmp::Ordering::Less => self.paired_access_score_wins += 1,
                std::cmp::Ordering::Equal => self.paired_score_ties += 1,
            }
            match record.winner {
                Some("frontier") => self.paired_frontier_admissions += 1,
                Some("access") => self.paired_access_admissions += 1,
                _ => {}
            }
        }
        match record.winner {
            Some("frontier") => self.frontier_wins += 1,
            Some("access") => self.access_wins += 1,
            _ => match record.no_admission_reason {
                Some("no_geometry") => self.no_geometry += 1,
                Some("no_source_feasible") => self.no_source_feasible += 1,
                Some("no_cell_forecast") => self.no_cell_forecast += 1,
                Some("no_metrics") => self.no_metrics += 1,
                Some("score_error") => self.score_errors += 1,
                _ => self.ranked_admission_rejections += 1,
            },
        }

        // Retain representative decision records, not the many empty scans
        // after geometry is exhausted. Keep the first four and a rolling tail.
        let index = self.comparisons;
        let representative = record.frontier_sites > 0
            || record.access_sites > 0
            || record
                .no_admission_reason
                .is_some_and(|reason| reason != "no_geometry");
        if representative {
            self.representative_comparisons = self.representative_comparisons.saturating_add(1);
            if self.records.len() < 32 {
                self.records.push(DecisionRecord {
                    comparison: index,
                    ..record
                });
            } else if self.records.len() > 4 {
                let slot = 4 + ((self.representative_comparisons.saturating_sub(33) as usize) % 28);
                self.records[slot] = DecisionRecord {
                    comparison: index,
                    ..record
                };
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg(feature = "legacy")]
pub(crate) struct ServedBuilding {
    pub x: f32,
    pub z: f32,
    pub housing_capacity: f64,
    pub housing_vacant: f64,
    pub commercial_capacity: f64,
    pub commercial_vacant: f64,
}

#[cfg(feature = "legacy")]
pub(crate) fn catchment_at(buildings: &[ServedBuilding], x: f32, z: f32) -> LiveCatchment {
    let mut catchment = LiveCatchment::default();
    for building in buildings {
        if (building.x - x).hypot(building.z - z) > 110.0 {
            continue;
        }
        catchment.housing_capacity += building.housing_capacity;
        catchment.housing_vacant_capacity += building.housing_vacant;
        catchment.commercial_capacity += building.commercial_capacity;
        catchment.commercial_vacant_capacity += building.commercial_vacant;
    }
    catchment
}

#[cfg(feature = "legacy")]
pub(crate) fn vacancy_policy() -> VacancyPolicy {
    VacancyPolicy {
        housing_limit: HOUSING_VACANCY_LIMIT,
        commercial_limit: COMMERCIAL_VACANCY_LIMIT,
    }
}

/// Create per-cell experimental targets. The max is validated metadata only;
/// the scorer rewards progress toward the minimum and does not enforce a cap.
#[cfg(feature = "legacy")]
pub(crate) fn targets(
    metrics: &DistrictMetrics,
    provisional_minima: &[f64],
) -> Vec<DistrictTargets> {
    assert_eq!(metrics.cells.len(), provisional_minima.len());
    provisional_minima
        .iter()
        .map(|minimum| DistrictTargets {
            built_envelope_coverage_proxy: TargetRange {
                min: *minimum,
                max: *minimum + 0.10,
            },
            floor_area_ratio_proxy: TargetRange { min: 0.0, max: 0.0 },
            occupied_frontage_m: TargetRange { min: 0.0, max: 0.0 },
            housing_vacancy_expansion_limit: HOUSING_VACANCY_LIMIT,
            commercial_vacancy_expansion_limit: COMMERCIAL_VACANCY_LIMIT,
        })
        .collect()
}

/// Reject additional capacity when a served source market has substantial
/// vacancy. A market with no current capacity is neutral during bootstrap.
#[cfg(feature = "legacy")]
pub(crate) fn source_market_accepts_capacity(catchment: LiveCatchment) -> bool {
    (catchment.housing_capacity <= 0.0
        || catchment.housing_vacant_capacity / catchment.housing_capacity < HOUSING_VACANCY_LIMIT)
        && (catchment.commercial_capacity <= 0.0
            || catchment.commercial_vacant_capacity / catchment.commercial_capacity
                < COMMERCIAL_VACANCY_LIMIT)
}

#[cfg(feature = "legacy")]
pub(crate) fn demand_fit(demand: f32) -> f64 {
    ((demand - 0.8_f32) as f64 / 1.2).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_vacancy_gates_only_markets_with_positive_capacity() {
        assert!(source_market_accepts_capacity(LiveCatchment::default()));
        assert!(source_market_accepts_capacity(LiveCatchment {
            housing_capacity: 100.0,
            housing_vacant_capacity: 24.9,
            ..LiveCatchment::default()
        }));
        assert!(!source_market_accepts_capacity(LiveCatchment {
            housing_capacity: 100.0,
            housing_vacant_capacity: 25.0,
            commercial_capacity: 80.0,
            commercial_vacant_capacity: 0.0,
        }));
        assert!(!source_market_accepts_capacity(LiveCatchment {
            housing_capacity: 100.0,
            housing_vacant_capacity: 0.0,
            commercial_capacity: 80.0,
            commercial_vacant_capacity: 28.0,
        }));
    }

    #[test]
    fn experiment_targets_are_provisional_and_weights_remain_explicit_at_call_site() {
        assert_eq!(FORECAST_UTILIZATION, 0.5);
        assert_eq!(METRICS_MAX_AGE_TICKS, 400);
        assert_eq!(demand_fit(0.8), 0.0);
        assert!(demand_fit(2.0) > 0.0);
    }

    #[test]
    fn catchment_uses_only_served_buildings_inside_the_origin_radius() {
        let buildings = [
            ServedBuilding {
                x: 0.0,
                z: 0.0,
                housing_capacity: 20.0,
                housing_vacant: 2.0,
                commercial_capacity: 6.0,
                commercial_vacant: 0.0,
            },
            ServedBuilding {
                x: 110.01,
                z: 0.0,
                housing_capacity: 100.0,
                housing_vacant: 100.0,
                commercial_capacity: 0.0,
                commercial_vacant: 0.0,
            },
        ];
        assert_eq!(
            catchment_at(&buildings, 0.0, 0.0),
            LiveCatchment {
                housing_capacity: 20.0,
                housing_vacant_capacity: 2.0,
                commercial_capacity: 6.0,
                commercial_vacant_capacity: 0.0,
            }
        );
    }

    #[test]
    fn test_diagnostics_aggregate_all_comparisons_and_keep_at_most_32_samples() {
        let mut diagnostics = ExperimentDiagnostics::default();
        for tick in 0..1_000 {
            diagnostics.observe(DecisionRecord {
                tick,
                frontier_proposal: true,
                frontier_sites: 4,
                frontier_area_m2: 672.0,
                frontier_top_score: Some(0.25),
                frontier_ranked: 1,
                access_proposals: 48,
                access_sites: 96,
                access_area_m2: 12_288.0,
                access_top_score: Some(0.5),
                access_ranked: 8,
                winner: Some("access"),
                ..DecisionRecord::default()
            });
        }
        assert_eq!(diagnostics.comparisons, 1_000);
        assert_eq!(diagnostics.frontier_proposals, 1_000);
        assert_eq!(diagnostics.access_proposals, 48_000);
        assert_eq!(diagnostics.access_wins, 1_000);
        assert_eq!(diagnostics.frontier_top_score_count, 1_000);
        assert_eq!(diagnostics.access_top_score_max, 0.5);
        assert_eq!(diagnostics.records.len(), 32);
        assert!(diagnostics.records.iter().all(|record| record.tick < 1_000));
    }
}
