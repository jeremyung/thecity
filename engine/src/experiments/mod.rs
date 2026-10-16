//! Test-only experiments and diagnostics. Nothing here is compiled into the
//! application; production code paths reach it only through `#[cfg(test)]` hooks.

/// Counters and switches for paired scheduler experiments, held on the engine only
/// in test builds.
#[derive(Clone, Debug, Default)]
pub(crate) struct ExperimentState {
    pub(crate) district_policy_comparisons: u32,
    pub(crate) district_policy_access_wins: u32,
    pub(crate) district_policy_frontier_wins: u32,
    pub(crate) district_policy_diagnostics: crate::district_policy::ExperimentDiagnostics,
    /// Test-only scheduler experiment: offer a real interior-access project
    /// before progressive frontage release, after ordinary usable infill.
    pub(crate) early_access_experiment: bool,
    pub(crate) early_access_attempted_tick: Option<u64>,
    pub(crate) early_access_admissions: u32,
    pub(crate) early_access_attempts: u32,
    /// Test-only control for paired comparisons after early block reservation
    /// became the production scheduling behavior.
    pub(crate) early_block_reservation_disable_override: bool,
    pub(crate) early_block_reservation_attempts: u32,
    pub(crate) early_block_reservation_admissions: u32,
    pub(crate) early_block_reservation_noops: u32,
    pub(crate) early_block_reservation_started_tick: Option<u64>,
    pub(crate) early_block_reservation_duration_ticks: u64,
    pub(crate) interior_access_completions: u32,
    /// Test-only opening-time access experiment. Production remains unchanged.
    pub(crate) road_opening_access_experiment: bool,
    /// Narrow follow-up experiment: permit opening-time access only when the
    /// newly formed face contains sampled unserved interior and a wholly
    /// contained access proposal. The legacy opening experiment remains a
    /// separate control for matched comparisons.
    pub(crate) face_conditioned_access_experiment: bool,
    /// Test-only admission treatment allowing a canonical clear-lot subset on
    /// otherwise ordinary full-block candidates.
    pub(crate) partial_full_block_experiment: bool,
    pub(crate) road_opening_access_attempts: u32,
    pub(crate) road_opening_access_admissions: u32,
    pub(crate) road_opening_access_branch: Option<u32>,
}

pub(crate) mod access_bend_diagnostic;
pub(crate) mod access_density_diagnostic;
pub(crate) mod access_discovery_work_diagnostic;
pub(crate) mod block_subdivision_diagnostic;
pub(crate) mod compact_frontage_diagnostic;
pub(crate) mod density_gap_diagnosis;
pub(crate) mod early_access_experiment;
pub(crate) mod early_block_reservation_experiment;
pub(crate) mod initial_depth_diagnostic;
pub(crate) mod mature_planner_trace_tests;
pub(crate) mod mature_void_diagnostic;
pub(crate) mod partial_block_diagnostic;
pub(crate) mod rear_extension_diagnostic;
pub(crate) mod road_opening_access_experiment;
pub(crate) mod road_split_profile;
pub(crate) mod skyline_allocation_diagnostic;
pub(crate) mod spine_ranking_experiment_tests;
