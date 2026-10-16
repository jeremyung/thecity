//! Tests against real restored worlds; this adapter is not a runtime controller.
use super::*;
use crate::district_density::{measure, BuildingSample, BuildingUse, GridSpec};

#[test]
fn mature_district_measurements_conserve_capacity_without_mutating_worlds() {
    for fixture in [
        include_str!(
            "../../tests/fixtures/access-joins/interior-access-verified-87231.json.final.json"
        ),
        include_str!(
            "../../tests/fixtures/access-joins/interior-access-verified-seed0.json.final.json"
        ),
    ] {
        let engine = Engine::restore_json(fixture).expect("mature fixture restores");
        let before = engine.save_json();
        let mut samples: Vec<_> = engine
            .state
            .buildings
            .iter()
            .map(|building| BuildingSample {
                id: building.id,
                footprint: geometry::OrientedRect {
                    x: building.x,
                    z: building.z,
                    width: building.width,
                    depth: building.depth,
                    rotation: building.rotation,
                },
                floors: building.floors,
                use_class: if is_housing(building) {
                    BuildingUse::Housing
                } else if is_commercial(building) {
                    BuildingUse::Commercial
                } else {
                    BuildingUse::Other
                },
                // Empty open buildings must remain in the vacancy denominator.
                open: building.stage == "open",
                resident_occupancy: building.occupancy,
                capacity: building.capacity,
                business_active: building.business_active,
            })
            .collect();
        let grid = GridSpec {
            min_x: MIN as f64,
            max_x: MAX as f64,
            min_z: MIN as f64,
            max_z: MAX as f64,
            cell_size: 100.0,
        };
        let metrics = measure(grid, &samples, &[]).expect("valid world fits measurement caps");
        samples.reverse();
        assert_eq!(metrics, measure(grid, &samples, &[]).unwrap());
        // Real fractional building geometry must retain exact measurements
        // through the JSON boundary, not just through an in-memory clone.
        let mut partial =
            district_density::MeasurementAccumulator::begin(grid, &samples, &[]).unwrap();
        partial.advance(16).unwrap();
        let checkpoint_json = serde_json::to_string(&partial.checkpoint().unwrap()).unwrap();
        let checkpoint = serde_json::from_str(&checkpoint_json).unwrap();
        let mut resumed = district_density::MeasurementAccumulator::resume(checkpoint).unwrap();
        assert_eq!(partial, resumed);
        while !resumed.is_complete() {
            resumed.advance(16).unwrap();
        }
        assert_eq!(resumed.finish().unwrap(), metrics);
        let open: Vec<_> = engine
            .state
            .buildings
            .iter()
            .filter(|b| b.stage == "open")
            .collect();
        let housing_capacity: f64 = open
            .iter()
            .filter(|b| is_housing(b))
            .map(|b| b.capacity as f64)
            .sum();
        let housing_occupied: f64 = open
            .iter()
            .filter(|b| is_housing(b))
            .map(|b| b.occupancy as f64)
            .sum();
        let commercial_capacity: f64 = open
            .iter()
            .filter(|b| is_commercial(b))
            .map(|b| b.capacity as f64)
            .sum();
        let active_commercial: f64 = open
            .iter()
            .filter(|b| is_commercial(b) && b.business_active)
            .map(|b| b.capacity as f64)
            .sum();
        for (actual, expected) in [
            (
                metrics.cells.iter().map(|c| c.gross_area).sum::<f64>(),
                640_000.0,
            ),
            (
                metrics.cells.iter().map(|c| c.housing_capacity).sum(),
                housing_capacity,
            ),
            (
                metrics.cells.iter().map(|c| c.housing_occupied).sum(),
                housing_occupied,
            ),
            (
                metrics
                    .cells
                    .iter()
                    .map(|c| c.housing_vacant_capacity)
                    .sum(),
                housing_capacity - housing_occupied,
            ),
            (
                metrics.cells.iter().map(|c| c.commercial_capacity).sum(),
                commercial_capacity,
            ),
            (
                metrics
                    .cells
                    .iter()
                    .map(|c| c.active_commercial_capacity)
                    .sum(),
                active_commercial,
            ),
        ] {
            assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
        }
        assert!(metrics
            .cells
            .iter()
            .any(|c| c.built_envelope_coverage_proxy() > 0.0));
        assert!(metrics
            .cells
            .iter()
            .all(|c| c.floor_area_ratio_proxy() >= c.built_envelope_coverage_proxy()));
        assert_eq!(
            before,
            engine.save_json(),
            "measurement changes no authoritative state"
        );
    }
}
