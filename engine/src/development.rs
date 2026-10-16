//! Bounded, deterministic selection of the next eligible building site.
//!
//! Eligibility is computed by the engine. This module only scores a stable
//! slice of that eligible list; it does not scan parcels or mutate simulation
//! state.

pub(crate) const MAX_BUILDING_CANDIDATES_PER_EVALUATION: usize = 32;
#[cfg(feature = "legacy")]
pub(crate) const MAX_SHARED_BLOCK_CANDIDATES_PER_EVALUATION: usize = 32;
#[cfg(feature = "legacy")]
pub(crate) const MAX_FULL_BLOCK_CANDIDATES_PER_EVALUATION: usize = 32;
#[cfg(feature = "legacy")]
pub(crate) const MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION: usize = 24;
const LOCAL_NEIGHBORHOOD_RADIUS: f32 = 78.0;

/// Integer-only score for comparing the usable parcel envelope admitted by a
/// neighborhood plan with the length of road that still needs construction.
/// The area is a nominal site-capacity measure, not a floor-area estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct NeighborhoodBenefitScore {
    pub usable_sites: u16,
    pub nominal_area_cm2: u64,
    pub new_road_length_cm: u32,
    pub new_road_count: u8,
    pub stable_key: [u32; 3],
}

/// Separate caps ensure each plan type gets a chance to compete even when one
/// candidate class has many more geometrically plausible alternatives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) struct NeighborhoodCandidateBudget {
    pub shared: usize,
    pub full: usize,
    pub spine: usize,
}

#[cfg(any(test, feature = "legacy"))]
impl NeighborhoodCandidateBudget {
    #[cfg(feature = "legacy")]
    pub fn claim_shared(&mut self) -> bool {
        if self.shared >= MAX_SHARED_BLOCK_CANDIDATES_PER_EVALUATION {
            return false;
        }
        self.shared += 1;
        true
    }

    #[cfg(feature = "legacy")]
    pub fn claim_full(&mut self) -> bool {
        if self.full >= MAX_FULL_BLOCK_CANDIDATES_PER_EVALUATION {
            return false;
        }
        self.full += 1;
        true
    }

    #[cfg(feature = "legacy")]
    pub fn claim_spine(&mut self) -> bool {
        if self.spine >= MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION {
            return false;
        }
        self.spine += 1;
        true
    }

    #[cfg(feature = "legacy")]
    pub fn shared_limit_reached(&self) -> bool {
        self.shared >= MAX_SHARED_BLOCK_CANDIDATES_PER_EVALUATION
    }

    #[cfg(feature = "legacy")]
    pub fn full_limit_reached(&self) -> bool {
        self.full >= MAX_FULL_BLOCK_CANDIDATES_PER_EVALUATION
    }

    #[cfg(any(test, feature = "legacy"))]
    pub fn spine_limit_reached(&self) -> bool {
        self.spine >= MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION
    }

    #[cfg(feature = "legacy")]
    pub fn any_limit_reached(&self) -> bool {
        // Spine is an additional candidate class. Exhausting its independent
        // allowance must not stop ordinary shared/full candidates from being
        // considered at later anchors.
        self.shared_limit_reached() || self.full_limit_reached()
    }
}

/// Returns true when `candidate` provides more nominal parcel area per meter
/// of new road. Stable tie-breaks keep equal-value plans deterministic.
#[cfg(feature = "legacy")]
pub(crate) fn neighborhood_score_is_better(
    candidate: NeighborhoodBenefitScore,
    current: NeighborhoodBenefitScore,
) -> bool {
    let candidate_cost = candidate.new_road_length_cm.max(1) as u128;
    let current_cost = current.new_road_length_cm.max(1) as u128;
    let candidate_value = candidate.nominal_area_cm2 as u128 * current_cost;
    let current_value = current.nominal_area_cm2 as u128 * candidate_cost;

    if candidate_value != current_value {
        return candidate_value > current_value;
    }
    if candidate.usable_sites != current.usable_sites {
        return candidate.usable_sites > current.usable_sites;
    }
    if candidate.new_road_length_cm != current.new_road_length_cm {
        return candidate.new_road_length_cm < current.new_road_length_cm;
    }
    if candidate.new_road_count != current.new_road_count {
        return candidate.new_road_count < current.new_road_count;
    }
    candidate.stable_key < current.stable_key
}

/// Experimental comparison for controlled measurement only. It prioritizes
/// usable lots per new road metre, then nominal parcel envelope area per metre,
/// and finally applies the existing deterministic tie breakers. This remains
/// separate from the production comparator until realized outcomes are reviewed.
#[cfg(any(test, feature = "legacy"))]
pub(crate) fn neighborhood_sites_per_road_is_better(
    candidate: NeighborhoodBenefitScore,
    current: NeighborhoodBenefitScore,
) -> bool {
    let candidate_cost = candidate.new_road_length_cm.max(1) as u128;
    let current_cost = current.new_road_length_cm.max(1) as u128;
    let candidate_sites = candidate.usable_sites as u128 * current_cost;
    let current_sites = current.usable_sites as u128 * candidate_cost;
    if candidate_sites != current_sites {
        return candidate_sites > current_sites;
    }

    let candidate_area = candidate.nominal_area_cm2 as u128 * current_cost;
    let current_area = current.nominal_area_cm2 as u128 * candidate_cost;
    if candidate_area != current_area {
        return candidate_area > current_area;
    }
    if candidate.usable_sites != current.usable_sites {
        return candidate.usable_sites > current.usable_sites;
    }
    if candidate.new_road_length_cm != current.new_road_length_cm {
        return candidate.new_road_length_cm < current.new_road_length_cm;
    }
    if candidate.new_road_count != current.new_road_count {
        return candidate.new_road_count < current.new_road_count;
    }
    candidate.stable_key < current.stable_key
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BuildingSiteCandidate {
    /// Index into the engine's parcel array.
    pub parcel_index: usize,
    pub parcel_id: u32,
    pub x: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct OpenBuildingPoint {
    pub x: f32,
    pub z: f32,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BuildingSiteSelection {
    pub parcel_index: usize,
    pub next_cursor: usize,
    pub inspected: usize,
}

#[derive(Clone, Copy, Debug)]
struct Score {
    candidate_index: usize,
    established: bool,
    has_open_neighbor: bool,
    nearest_open: f32,
    parcel_id: u32,
}

/// Score at most 32 eligible candidates, beginning at `cursor` and wrapping
/// once through `candidates`. Established infill sites (three active nearby
/// buildings) outrank isolated sites. If no established site is in the
/// current window, prefer a site near any open building, then the closest
/// open building, with parcel ID as the stable tie-breaker.
pub(crate) fn select_building_site(
    candidates: &[BuildingSiteCandidate],
    buildings: &[OpenBuildingPoint],
    cursor: usize,
) -> Option<BuildingSiteSelection> {
    if candidates.is_empty() {
        return None;
    }

    let start = cursor % candidates.len();
    let inspected = candidates.len().min(MAX_BUILDING_CANDIDATES_PER_EVALUATION);
    let mut best: Option<Score> = None;

    for offset in 0..inspected {
        let candidate_index = (start + offset) % candidates.len();
        let candidate = candidates[candidate_index];
        let mut active_neighbors = 0usize;
        let mut open_neighbors = 0usize;
        let mut nearest_open = f32::INFINITY;

        // One pass over open buildings per scored site. The eligible site
        // window is capped above, keeping this work bounded independently of
        // how many lots exist in the city.
        for building in buildings {
            let distance = (building.x - candidate.x).hypot(building.z - candidate.z);
            if distance < nearest_open {
                nearest_open = distance;
            }
            if distance <= LOCAL_NEIGHBORHOOD_RADIUS {
                open_neighbors += 1;
                if building.active {
                    active_neighbors += 1;
                }
            }
        }

        let score = Score {
            candidate_index,
            established: active_neighbors >= 3,
            has_open_neighbor: open_neighbors > 0,
            nearest_open,
            parcel_id: candidate.parcel_id,
        };
        if best.map_or(true, |current| score_is_better(score, current)) {
            best = Some(score);
        }
    }

    let next_cursor = start.saturating_add(inspected) % candidates.len();
    Some(BuildingSiteSelection {
        parcel_index: candidates[best?.candidate_index].parcel_index,
        next_cursor,
        inspected,
    })
}

fn score_is_better(candidate: Score, current: Score) -> bool {
    if candidate.established != current.established {
        return candidate.established;
    }
    if !candidate.established && candidate.has_open_neighbor != current.has_open_neighbor {
        return candidate.has_open_neighbor;
    }
    candidate
        .nearest_open
        .total_cmp(&current.nearest_open)
        .then_with(|| candidate.parcel_id.cmp(&current.parcel_id))
        .is_lt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_score(
        usable_sites: u16,
        area_m2: u64,
        road_m: u32,
        road_count: u8,
        stable_key: [u32; 3],
    ) -> NeighborhoodBenefitScore {
        NeighborhoodBenefitScore {
            usable_sites,
            nominal_area_cm2: area_m2 * 10_000,
            new_road_length_cm: road_m * 100,
            new_road_count: road_count,
            stable_key,
        }
    }

    #[test]
    fn neighborhood_ranking_compares_site_benefit_per_new_road_cost() {
        let efficient_shared = plan_score(6, 1_600, 210, 3, [5, 70, 1]);
        let larger_full_block = plan_score(10, 2_100, 300, 4, [2, 0, 3]);
        assert!(neighborhood_score_is_better(
            efficient_shared,
            larger_full_block
        ));
        assert!(!neighborhood_score_is_better(
            larger_full_block,
            efficient_shared
        ));

        let compact_shared = plan_score(6, 1_200, 220, 3, [5, 70, 1]);
        let highly_utilized_full = plan_score(10, 2_800, 300, 4, [2, 0, 3]);
        assert!(neighborhood_score_is_better(
            highly_utilized_full,
            compact_shared
        ));
        assert!(!neighborhood_score_is_better(
            compact_shared,
            highly_utilized_full
        ));
    }

    #[test]
    fn neighborhood_score_ties_use_sites_cost_and_stable_key() {
        let fewer_sites = plan_score(4, 800, 200, 3, [1, 2, 3]);
        let more_sites = plan_score(8, 1_600, 400, 4, [9, 8, 7]);
        assert!(neighborhood_score_is_better(more_sites, fewer_sites));

        let shorter_road = plan_score(6, 900, 150, 4, [9, 8, 7]);
        let longer_road = plan_score(6, 1_200, 200, 3, [1, 2, 3]);
        assert!(neighborhood_score_is_better(shorter_road, longer_road));

        let fewer_edges = plan_score(6, 1_200, 200, 3, [9, 8, 7]);
        let more_edges = plan_score(6, 1_200, 200, 4, [1, 2, 3]);
        assert!(neighborhood_score_is_better(fewer_edges, more_edges));

        let stable_later = plan_score(6, 1_200, 200, 3, [7, 1, 2]);
        let stable_first = plan_score(6, 1_200, 200, 3, [2, 5, 1]);
        assert!(neighborhood_score_is_better(stable_first, stable_later));
        assert!(!neighborhood_score_is_better(stable_later, stable_first));
    }

    #[test]
    fn neighborhood_candidate_budgets_allow_both_types_and_stop_at_their_caps() {
        let mut budget = NeighborhoodCandidateBudget::default();
        for _ in 0..MAX_SHARED_BLOCK_CANDIDATES_PER_EVALUATION {
            assert!(budget.claim_shared());
        }
        assert!(!budget.claim_shared());
        assert!(budget.shared_limit_reached());
        assert!(
            !budget.full_limit_reached(),
            "full-block candidates still get their window"
        );
        for _ in 0..MAX_FULL_BLOCK_CANDIDATES_PER_EVALUATION {
            assert!(budget.claim_full());
        }
        assert!(!budget.claim_full());
        assert_eq!(budget.shared + budget.full, 64);
        for _ in 0..MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION {
            assert!(budget.claim_spine());
        }
        assert!(!budget.claim_spine());
        assert_eq!(budget.spine, MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION);

        let mut spine_only = NeighborhoodCandidateBudget::default();
        for _ in 0..MAX_SPINE_BLOCK_CANDIDATES_PER_EVALUATION {
            assert!(spine_only.claim_spine());
        }
        assert!(spine_only.spine_limit_reached());
        assert!(
            !spine_only.any_limit_reached(),
            "spine exhaustion must not stop ordinary candidates at later anchors"
        );
    }

    #[test]
    fn established_infill_beats_a_closer_or_isolated_candidate() {
        let candidates = [
            BuildingSiteCandidate {
                parcel_index: 0,
                parcel_id: 8,
                x: 90.0,
                z: 0.0,
            },
            BuildingSiteCandidate {
                parcel_index: 1,
                parcel_id: 3,
                x: 0.0,
                z: 0.0,
            },
        ];
        let buildings = [
            OpenBuildingPoint {
                x: 0.0,
                z: 18.0,
                active: true,
            },
            OpenBuildingPoint {
                x: 18.0,
                z: 0.0,
                active: true,
            },
            OpenBuildingPoint {
                x: -18.0,
                z: 0.0,
                active: true,
            },
            OpenBuildingPoint {
                x: 88.0,
                z: 0.0,
                active: true,
            },
        ];

        let selected = select_building_site(&candidates, &buildings, 0).unwrap();
        assert_eq!(selected.parcel_index, 1);
        assert_eq!(selected.inspected, 2);
        assert_eq!(selected.next_cursor, 0);
    }

    #[test]
    fn scoring_window_is_bounded_and_cursor_wraps() {
        let candidates: Vec<_> = (0..40)
            .map(|index| BuildingSiteCandidate {
                parcel_index: index,
                parcel_id: index as u32 + 1,
                x: 300.0 + (index % 8) as f32 * 14.0,
                z: 300.0 + (index / 8) as f32 * 14.0,
            })
            .collect();
        let buildings = [OpenBuildingPoint {
            x: 0.0,
            z: 0.0,
            active: true,
        }];

        let selected = select_building_site(&candidates, &buildings, 38).unwrap();
        assert_eq!(selected.inspected, MAX_BUILDING_CANDIDATES_PER_EVALUATION);
        assert_eq!(selected.next_cursor, 30);
        assert!(
            selected.parcel_index == 38
                || selected.parcel_index == 39
                || selected.parcel_index < 30,
            "selection must come from the wrapped 32-candidate window"
        );

        let overflow_cursor = select_building_site(&candidates, &buildings, usize::MAX).unwrap();
        assert_eq!(overflow_cursor.inspected, 32);
        assert_eq!(overflow_cursor.next_cursor, (usize::MAX % 40 + 32) % 40);
    }

    #[test]
    fn founding_fallback_uses_stable_parcel_id_when_no_open_buildings_exist() {
        let candidates = [
            BuildingSiteCandidate {
                parcel_index: 0,
                parcel_id: 20,
                x: 0.0,
                z: 0.0,
            },
            BuildingSiteCandidate {
                parcel_index: 1,
                parcel_id: 10,
                x: 500.0,
                z: 500.0,
            },
        ];
        let selected = select_building_site(&candidates, &[], 0).unwrap();
        assert_eq!(selected.parcel_index, 1);
    }
}
