//! Persisted geometry and reservation helpers for a coordinated block spine.
//!
//! This module validates only the plan's canonical geometry and stage IDs.
//! World references, project/parcel capacity, terrain and graph connectivity
//! remain the responsibility of the engine integration layer.

use crate::blocks::BlockPoint;
use crate::geometry::{rect_segment_distance, rects_overlap, OrientedRect};
use crate::spine_blocks::{subdivide_spine_block, SpineBlockLayout};
use serde::{Deserialize, Serialize};

pub(crate) const RECIPE_VERSION: u8 = 1;
pub(crate) const MIN_SPINE_LOTS: usize = 4;
pub(crate) const MAX_SPINE_LOTS: usize = 64;
const STREET_INSET: f32 = 8.0;
const TARGET_FRONTAGE: f32 = 11.0;
const REAR_GAP: f32 = 2.2;
const MIN_LOT_DEPTH: f32 = 8.0;
const SPINE_ROAD_WIDTH: f32 = 9.0;

/// The recipe's host edges are fixed: edge 3's midpoint is the spine start,
/// and edge 1's midpoint is the spine end. Both split parameters are 0.5.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct SpinePlan {
    pub(crate) corners: [BlockPoint; 4],
    pub(crate) recipe_version: u8,
    pub(crate) stage: SpineStage,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum SpineStage {
    /// Geometric reservation exists before perimeter road IDs are available.
    Reserved,
    /// Both opposite perimeter host roads are open and resolved.
    PerimeterReady { host_road_ids: [u32; 2] },
    /// Both hosts have been split and the spine road project is active.
    UnderConstruction {
        host_road_ids: [u32; 2],
        split_node_ids: [u32; 2],
        spine_road_id: u32,
        project_id: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpinePlanError {
    UnsupportedRecipe,
    InvalidGeometry,
    InvalidStageIds,
}

impl SpinePlan {
    /// Rebuild and validate the canonical layout. No saved lot list is trusted
    /// or needed: the corners and recipe version determine every reservation.
    pub(crate) fn validate(&self) -> Result<SpineBlockLayout, SpinePlanError> {
        if self.recipe_version != RECIPE_VERSION {
            return Err(SpinePlanError::UnsupportedRecipe);
        }
        validate_stage(&self.stage)?;
        let layout = subdivide_spine_block(
            self.corners,
            STREET_INSET,
            TARGET_FRONTAGE,
            REAR_GAP,
            MIN_LOT_DEPTH,
        )
        .ok_or(SpinePlanError::InvalidGeometry)?;
        if !(MIN_SPINE_LOTS..=MAX_SPINE_LOTS).contains(&layout.lots.len())
            || !layout
                .spine
                .iter()
                .all(|point| point.x.is_finite() && point.z.is_finite())
        {
            return Err(SpinePlanError::InvalidGeometry);
        }
        Ok(layout)
    }

    /// Returns true when a candidate parcel intersects a reserved lot or the
    /// spine road's physical half-width plus `additional_spine_clearance`.
    /// Invalid candidate geometry fails closed.
    pub(crate) fn parcel_conflicts(
        &self,
        candidate: &OrientedRect,
        lot_gap: f32,
        additional_spine_clearance: f32,
    ) -> bool {
        if !candidate.finite_positive()
            || !lot_gap.is_finite()
            || lot_gap < 0.0
            || !additional_spine_clearance.is_finite()
            || additional_spine_clearance < 0.0
        {
            return true;
        }
        let Ok(layout) = self.validate() else {
            return true;
        };
        if layout
            .lots
            .iter()
            .any(|lot| rects_overlap(candidate, &lot_rect(lot), lot_gap))
        {
            return true;
        }
        let [a, b] = layout.spine;
        rect_segment_distance(candidate, a.x, a.z, b.x, b.z)
            < SPINE_ROAD_WIDTH * 0.5 + additional_spine_clearance
    }

    /// Returns true when a candidate road's width envelope intersects either
    /// the reserved lot envelopes or the spine road envelope. Invalid segments
    /// and malformed saved plans fail closed. `spine_clearance` is additional
    /// to the spine's physical half-width; `lot_clearance` is additional to
    /// the proposed road's physical half-width when checking reserved lots.
    pub(crate) fn road_conflicts(
        &self,
        ax: f32,
        az: f32,
        bx: f32,
        bz: f32,
        road_width: f32,
        lot_clearance: f32,
        spine_clearance: f32,
    ) -> bool {
        if !ax.is_finite()
            || !az.is_finite()
            || !bx.is_finite()
            || !bz.is_finite()
            || !road_width.is_finite()
            || road_width <= 0.0
            || !lot_clearance.is_finite()
            || lot_clearance < 0.0
            || !spine_clearance.is_finite()
            || spine_clearance < 0.0
        {
            return true;
        }
        let length = (bx - ax).hypot(bz - az);
        if !length.is_finite() || length <= f32::EPSILON {
            return true;
        }
        let candidate = OrientedRect {
            x: (ax + bx) * 0.5,
            z: (az + bz) * 0.5,
            width: length,
            depth: road_width,
            rotation: (-(bz - az)).atan2(bx - ax),
        };
        if !candidate.finite_positive() {
            return true;
        }
        let Ok(layout) = self.validate() else {
            return true;
        };
        if layout.lots.iter().any(|lot| {
            rect_segment_distance(&lot_rect(lot), ax, az, bx, bz) < road_width * 0.5 + lot_clearance
        }) {
            return true;
        }
        let [a, b] = layout.spine;
        let spine_length = (b.x - a.x).hypot(b.z - a.z);
        if !spine_length.is_finite() || spine_length <= f32::EPSILON {
            return true;
        }
        let spine = OrientedRect {
            x: (a.x + b.x) * 0.5,
            z: (a.z + b.z) * 0.5,
            width: spine_length,
            depth: SPINE_ROAD_WIDTH,
            rotation: (-(b.z - a.z)).atan2(b.x - a.x),
        };
        rects_overlap(&candidate, &spine, spine_clearance)
    }
}

fn validate_stage(stage: &SpineStage) -> Result<(), SpinePlanError> {
    let ids: Vec<u32> = match stage {
        SpineStage::Reserved => return Ok(()),
        SpineStage::PerimeterReady { host_road_ids } => host_road_ids.to_vec(),
        SpineStage::UnderConstruction {
            host_road_ids,
            split_node_ids,
            spine_road_id,
            project_id,
        } => vec![
            host_road_ids[0],
            host_road_ids[1],
            split_node_ids[0],
            split_node_ids[1],
            *spine_road_id,
            *project_id,
        ],
    };
    if ids.iter().any(|id| *id == 0) {
        return Err(SpinePlanError::InvalidStageIds);
    }
    let mut sorted = ids;
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len()
        != match stage {
            SpineStage::PerimeterReady { .. } => 2,
            SpineStage::UnderConstruction { .. } => 6,
            SpineStage::Reserved => 0,
        }
    {
        return Err(SpinePlanError::InvalidStageIds);
    }
    Ok(())
}

fn lot_rect(lot: &crate::spine_blocks::SpineLot) -> OrientedRect {
    OrientedRect {
        x: lot.center.x,
        z: lot.center.z,
        width: lot.width,
        depth: lot.depth,
        rotation: lot.rotation,
    }
}

#[cfg(test)]
mod tests {
    use super::{SpinePlan, SpinePlanError, SpineStage, MAX_SPINE_LOTS, RECIPE_VERSION};
    use crate::blocks::BlockPoint;
    use crate::geometry::OrientedRect;

    fn plan() -> SpinePlan {
        SpinePlan {
            corners: [
                BlockPoint { x: 0.0, z: 0.0 },
                BlockPoint { x: 82.0, z: 0.0 },
                BlockPoint { x: 82.0, z: 82.0 },
                BlockPoint { x: 0.0, z: 82.0 },
            ],
            recipe_version: RECIPE_VERSION,
            stage: SpineStage::Reserved,
        }
    }

    #[test]
    fn validates_canonical_spine_geometry_and_stage_ids() {
        let mut p = plan();
        let layout = p.validate().unwrap();
        assert_eq!(layout.lots.len(), 20);
        assert!(layout.lots.len() <= MAX_SPINE_LOTS);
        p.stage = SpineStage::PerimeterReady {
            host_road_ids: [21, 34],
        };
        assert!(p.validate().is_ok());
        p.stage = SpineStage::UnderConstruction {
            host_road_ids: [21, 34],
            split_node_ids: [40, 41],
            spine_road_id: 42,
            project_id: 43,
        };
        assert!(p.validate().is_ok());
    }

    #[test]
    fn rejects_unknown_recipe_malformed_geometry_and_duplicate_stage_ids() {
        let mut p = plan();
        p.recipe_version = RECIPE_VERSION + 1;
        assert_eq!(p.validate().unwrap_err(), SpinePlanError::UnsupportedRecipe);
        p = plan();
        p.corners[2].x = f32::NAN;
        assert_eq!(p.validate().unwrap_err(), SpinePlanError::InvalidGeometry);
        p = plan();
        p.stage = SpineStage::UnderConstruction {
            host_road_ids: [21, 34],
            split_node_ids: [40, 41],
            spine_road_id: 42,
            project_id: 21,
        };
        assert_eq!(p.validate().unwrap_err(), SpinePlanError::InvalidStageIds);
    }

    #[test]
    fn parcel_and_road_reservations_fail_closed_and_protect_layout() {
        let p = plan();
        let lot = p.validate().unwrap().lots[0];
        let parcel_on_reserved_lot = OrientedRect {
            x: lot.center.x,
            z: lot.center.z,
            width: lot.width,
            depth: lot.depth,
            rotation: lot.rotation,
        };
        assert!(p.parcel_conflicts(&parcel_on_reserved_lot, 2.0, 8.0));
        assert!(p.road_conflicts(20.0, 20.0, 62.0, 62.0, 9.0, 5.0, 0.0));
        assert!(p.parcel_conflicts(
            &OrientedRect {
                x: f32::NAN,
                z: 0.0,
                width: 10.0,
                depth: 10.0,
                rotation: 0.0,
            },
            2.0,
            8.0,
        ));
        assert!(p.road_conflicts(0.0, 0.0, 0.0, 0.0, 9.0, 5.0, 0.0));
    }

    #[test]
    fn parcel_clearance_includes_physical_spine_and_honors_exact_margin() {
        let p = plan();
        let on_centerline = OrientedRect {
            x: 41.0,
            z: 41.0,
            width: 1.0,
            depth: 1.0,
            rotation: 0.0,
        };
        assert!(p.parcel_conflicts(&on_centerline, 0.0, 0.0));

        let outside_carriageway = OrientedRect {
            x: 41.0,
            z: 46.5,
            width: 1.0,
            depth: 1.0,
            rotation: 0.0,
        };
        assert!(!p.parcel_conflicts(&outside_carriageway, 0.0, 0.0));

        // The nearest parcel edge is exactly 4.5 m (road half-width) + 1.5 m
        // additional clearance from the centerline, so the boundary is legal.
        let exact_margin = OrientedRect {
            x: 41.0,
            z: 47.5,
            width: 2.0,
            depth: 1.0,
            rotation: 0.0,
        };
        assert!(!p.parcel_conflicts(&exact_margin, 0.0, 1.5));
        assert!(p.parcel_conflicts(&exact_margin, 0.0, 1.51));
    }

    #[test]
    fn every_stage_roundtrips_and_zero_ids_are_rejected() {
        let stages = [
            SpineStage::Reserved,
            SpineStage::PerimeterReady {
                host_road_ids: [21, 34],
            },
            SpineStage::UnderConstruction {
                host_road_ids: [21, 34],
                split_node_ids: [40, 41],
                spine_road_id: 42,
                project_id: 43,
            },
        ];
        for stage in stages {
            let p = SpinePlan { stage, ..plan() };
            let json = serde_json::to_string(&p).unwrap();
            let restored: SpinePlan = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, p);
            assert!(restored.validate().is_ok());
        }

        let invalid = SpinePlan {
            stage: SpineStage::PerimeterReady {
                host_road_ids: [21, 0],
            },
            ..plan()
        };
        assert_eq!(
            invalid.validate().unwrap_err(),
            SpinePlanError::InvalidStageIds
        );
    }
}
