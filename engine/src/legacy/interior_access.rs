use serde::{Deserialize, Serialize};

pub(crate) const MAX_SELECTED_LOTS: usize = 16;
pub(crate) const MAX_PROPOSALS_PER_EVALUATION: usize = 48;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct InteriorAccessLot {
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub rotation: f32,
    /// Normalized position along the new access street, in its stored direction.
    pub along: f32,
    pub side: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InteriorAccessPlan {
    pub host_road_id: u32,
    pub host_second_half_road_id: u32,
    pub host_t: f32,
    pub split_node_id: u32,
    pub branch_endpoint_id: u32,
    pub branch_road_id: u32,
    pub project_id: u32,
    pub selected_lots: Vec<InteriorAccessLot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CandidateScore {
    pub site_area: u32,
    pub road_length_mm: u32,
    pub host_id: u32,
    pub fraction_index: u8,
    pub side_index: u8,
    pub length_index: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct AccessProposal {
    pub score: CandidateScore,
    pub host_id: u32,
    pub host_t: f32,
    pub end_x: f32,
    pub end_z: f32,
    pub selected_lots: Vec<InteriorAccessLot>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(feature = "legacy")]
pub(crate) struct AccessScan {
    pub proposals: Vec<AccessProposal>,
    pub next_cursor: usize,
    #[cfg(test)]
    pub rejection_counts: [usize; 8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct AccessCandidate {
    pub host_id: u32,
    pub fraction_index: u8,
    pub side: i8,
    pub length_index: u8,
}

/// Geometry-derived proposal used by the bounded diagnostic planner. Unlike
/// the legacy candidate grid, this records an exact split fraction and branch
/// endpoint. It is evaluated by the same collision, frontage, reservation,
/// demand, and capacity gates as ordinary proposals.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct AccessGeometryCandidate {
    pub host_id: u32,
    pub host_t: f32,
    pub end_x: f32,
    pub end_z: f32,
    /// Stable, bounded tie-break index assigned by the deterministic generator.
    pub tie_index: u8,
}

impl CandidateScore {
    /// Compare area per road length using integer cross-products for stable ordering.
    pub fn compare(self, other: Self) -> std::cmp::Ordering {
        (self.site_area as u64 * other.road_length_mm as u64)
            .cmp(&(other.site_area as u64 * self.road_length_mm as u64))
            .then_with(|| self.host_id.cmp(&other.host_id))
            .then_with(|| self.fraction_index.cmp(&other.fraction_index))
            .then_with(|| self.side_index.cmp(&other.side_index))
            .then_with(|| self.length_index.cmp(&other.length_index))
    }
}

#[cfg(feature = "legacy")]
pub(crate) const FRACTIONS: [f32; 3] = [0.25, 0.5, 0.75];
#[cfg(feature = "legacy")]
pub(crate) const SIDES: [i8; 2] = [-1, 1];
#[cfg(feature = "legacy")]
pub(crate) const LENGTHS: [f32; 4] = [24.0, 36.0, 48.0, 60.0];
pub(crate) const PROPOSALS_PER_HOST: usize = 3 * 2 * 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_ranking_compares_area_per_meter_then_stable_geometry() {
        let compact = CandidateScore {
            site_area: 2_000_000,
            road_length_mm: 24_000,
            host_id: 9,
            fraction_index: 0,
            side_index: 0,
            length_index: 0,
        };
        let long = CandidateScore {
            site_area: 4_000_000,
            road_length_mm: 60_000,
            host_id: 1,
            fraction_index: 0,
            side_index: 0,
            length_index: 3,
        };
        assert!(compact.compare(long).is_gt());

        let tied_high_id = CandidateScore {
            host_id: 10,
            ..compact
        };
        assert!(compact.compare(tied_high_id).is_lt());
    }

    #[test]
    fn candidate_enumeration_budget_is_finite_and_stable() {
        assert_eq!(
            PROPOSALS_PER_HOST,
            FRACTIONS.len() * SIDES.len() * LENGTHS.len()
        );
        assert_eq!(MAX_PROPOSALS_PER_EVALUATION, 48);
        assert!(MAX_SELECTED_LOTS <= 16);
    }
}
