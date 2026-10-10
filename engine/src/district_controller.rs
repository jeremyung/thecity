//! Persisted scheduling and frozen inputs for incremental district measurements.
//!
//! The controller only records measurement work. It does not participate in
//! growth decisions; future callers must still check current live geometry and
//! demand before admitting a project.

use crate::district_density::{
    validate_checkpoint, BuildingSample, BuildingUse, DistrictMetrics, FrontageSample, GridSpec,
    MeasureError, MeasurementAccumulator, MeasurementCheckpoint,
};
use crate::{geometry, Building, ParcelFrontage};
use serde::{Deserialize, Serialize};

pub(crate) const REFRESH_INTERVAL_TICKS: u64 = 200;
pub(crate) const DISTRICT_GRID: GridSpec = GridSpec {
    min_x: -400.0,
    max_x: 400.0,
    min_z: -400.0,
    max_z: 400.0,
    cell_size: 100.0,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct DistrictControllerState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<MeasurementCheckpoint>,
    #[serde(default)]
    pub pending_capture_tick: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<MeasurementCheckpoint>,
    #[serde(default)]
    pub completed_capture_tick: u64,
    #[serde(default)]
    pub completed_tick: u64,
    #[serde(default)]
    pub next_refresh_tick: u64,
}

impl DistrictControllerState {
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_none()
            && self.completed.is_none()
            && self.pending_capture_tick == 0
            && self.completed_capture_tick == 0
            && self.completed_tick == 0
            && self.next_refresh_tick == 0
    }

    pub(crate) fn validate(&self, current_tick: u64) -> Result<(), String> {
        if let Some(checkpoint) = &self.completed {
            validate_checkpoint(checkpoint).map_err(measure_error)?;
            if checkpoint.grid != DISTRICT_GRID
                || !checkpoint.frontages.is_empty()
                || checkpoint.processed_samples
                    != checkpoint.buildings.len() + checkpoint.frontages.len()
                || self.completed_capture_tick == 0
                || self.completed_capture_tick > self.completed_tick
                || self.completed_tick > current_tick
                || self.next_refresh_tick
                    < self.completed_tick.saturating_add(REFRESH_INTERVAL_TICKS)
            {
                return Err("district measurement completed checkpoint timeline is invalid".into());
            }
        } else if self.completed_capture_tick != 0 || self.completed_tick != 0 {
            return Err("district measurement metadata has no completed checkpoint".into());
        }

        if let Some(checkpoint) = &self.pending {
            validate_checkpoint(checkpoint).map_err(measure_error)?;
            let total = checkpoint.buildings.len() + checkpoint.frontages.len();
            if checkpoint.grid != DISTRICT_GRID
                || !checkpoint.frontages.is_empty()
                || checkpoint.processed_samples >= total
                || self.pending_capture_tick == 0
                || self.pending_capture_tick > current_tick
                || self.pending_capture_tick < self.next_refresh_tick
                || self
                    .completed
                    .as_ref()
                    .is_some_and(|_| self.pending_capture_tick < self.completed_tick)
            {
                return Err("district measurement pending checkpoint timeline is invalid".into());
            }
        } else if self.pending_capture_tick != 0 {
            return Err("district measurement metadata has no pending checkpoint".into());
        }
        if self.next_refresh_tick > current_tick.saturating_add(REFRESH_INTERVAL_TICKS) {
            return Err(
                "district measurement refresh tick is implausibly far in the future".into(),
            );
        }
        Ok(())
    }
}

pub(crate) struct Runtime {
    pub accumulator: Option<MeasurementAccumulator>,
    pub metrics: Option<DistrictMetrics>,
}

impl Runtime {
    pub(crate) fn empty() -> Self {
        Self {
            accumulator: None,
            metrics: None,
        }
    }

    /// Frontage samples are withheld until frontage records can be mapped to
    /// their authoritative, continuous curb geometry.
    pub(crate) fn frontage_metrics_available(&self) -> bool {
        false
    }

    /// Reconstruct derived state only after the serialized checkpoint has
    /// passed the controller's full structural and timeline validation.
    pub(crate) fn restore(state: &DistrictControllerState) -> Result<Self, String> {
        let metrics = state
            .completed
            .clone()
            .map(|checkpoint| {
                MeasurementAccumulator::resume(checkpoint)
                    .and_then(MeasurementAccumulator::finish)
                    .map_err(measure_error)
            })
            .transpose()?;
        let accumulator = state
            .pending
            .clone()
            .map(MeasurementAccumulator::resume)
            .transpose()
            .map_err(measure_error)?;
        Ok(Self {
            accumulator,
            metrics,
        })
    }
}

pub(crate) fn make_samples(
    buildings: &[Building],
    frontages: &[ParcelFrontage],
) -> (Vec<BuildingSample>, Vec<FrontageSample>) {
    let buildings = buildings
        .iter()
        .map(|building| {
            let use_class = if matches!(building.kind.as_str(), "home" | "apartment") {
                BuildingUse::Housing
            } else if matches!(building.kind.as_str(), "shop" | "cafe" | "office") {
                BuildingUse::Commercial
            } else {
                BuildingUse::Other
            };
            BuildingSample {
                id: building.id,
                footprint: geometry::OrientedRect {
                    x: building.x,
                    z: building.z,
                    width: building.width,
                    depth: building.depth,
                    rotation: building.rotation,
                },
                floors: building.floors,
                use_class,
                open: building.stage == "open",
                resident_occupancy: if use_class == BuildingUse::Housing {
                    building.occupancy
                } else {
                    0
                },
                capacity: building.capacity,
                business_active: building.business_active,
            }
        })
        .collect();

    // Road frontage semantics still need a trustworthy map from frontage
    // records to the continuous curb segment, so current measurements mark
    // frontage data unavailable rather than treating this as zero frontage.
    let _ = frontages;
    (buildings, Vec::new())
}

pub(crate) fn measure_error(error: MeasureError) -> String {
    format!("invalid district measurement checkpoint: {error:?}")
}
