use crate::*;

const SCAN_PER_RENEWAL: usize = 24;
const MIN_REALIZED_LOT_FRACTION: f32 = 0.85;
const MIN_ACTIVE_BUILDINGS: usize = 300;
pub(crate) const MIN_DEVELOPMENT_DEMAND: f32 = 2.5;
pub(crate) const MIN_BUILDING_AGE_DAYS: u32 = 3;
const MIN_REDEVELOPMENT_SITE_AREA: f32 = 280.0;
pub(crate) const MIN_OFFICE_FLOORS: u8 = 16;
const TARGET_FLOOR_INCREMENT: u8 = 6;
const MAX_REDEVELOPMENT_FLOORS: u8 = 24;
pub(crate) const MAX_REDEVELOPED_TOWERS: usize = 8;
const DEMOLITION_FRACTION: f32 = 0.30;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HeightRedevelopment {
    pub(crate) project_id: u32,
    pub(crate) building_id: u32,
    pub(crate) parcel_id: u32,
    pub(crate) original_floors: u8,
    pub(crate) original_capacity: u16,
    pub(crate) target_floors: u8,
    pub(crate) target_capacity: u16,
}

impl Engine {
    pub(crate) fn refresh_height_redevelopment_lots(&mut self) {
        self.height_redevelopment_lots = self
            .plan
            .as_ref()
            .map(|plan| {
                plan.plan
                    .lots
                    .iter()
                    .enumerate()
                    .filter_map(|(index, lot)| {
                        (!lot.flag
                            && lot.district == masterplan::District::StationCore
                            && lot.width * lot.depth >= MIN_REDEVELOPMENT_SITE_AREA)
                            .then_some(index)
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    pub(crate) fn try_start_height_redevelopment(&mut self) -> bool {
        if self.state.construction.height_redevelopment.is_some()
            || !self.project_admission_available()
        {
            return false;
        }
        let Some(plan) = self.plan.as_ref() else {
            return false;
        };
        let Some(progress) = self.state.network.masterplan.as_ref() else {
            return false;
        };
        if !self.state.transit.station_online
            || self.state.people.demand < MIN_DEVELOPMENT_DEMAND
            || plan.plan.lots.is_empty()
            || (progress.lot_parcels.iter().filter(|id| **id != 0).count() as f32)
                / (plan.plan.lots.len() as f32)
                < MIN_REALIZED_LOT_FRACTION
            || self
                .state
                .construction
                .buildings
                .iter()
                .filter(|building| site_active(building))
                .count()
                < MIN_ACTIVE_BUILDINGS
        {
            return false;
        }

        let count = self.height_redevelopment_lots.len();
        if count == 0 {
            return false;
        }
        let start = self.state.construction.height_redevelopment_cursor % count;
        let scanned = SCAN_PER_RENEWAL.min(count);
        let day = (self.state.tick / TICKS_PER_DAY) as u32;
        let mut chosen = None;
        for offset in 0..scanned {
            let lot_index = self.height_redevelopment_lots[(start + offset) % count];
            let Some(parcel_id) = progress
                .lot_parcels
                .get(lot_index)
                .copied()
                .filter(|id| *id != 0)
            else {
                continue;
            };
            let Some(building) = self
                .state
                .construction
                .buildings
                .iter()
                .find(|building| building.parcel_id == parcel_id)
            else {
                continue;
            };
            if !self.height_redevelopment_candidate(building, lot_index, day) {
                continue;
            }
            chosen = Some(building.id);
            break;
        }
        self.state.construction.height_redevelopment_cursor = (start + scanned) % count;
        let Some(building_id) = chosen else {
            return false;
        };
        self.begin_height_redevelopment(building_id)
    }

    pub(crate) fn height_redevelopment_candidate(
        &self,
        building: &Building,
        lot_index: usize,
        day: u32,
    ) -> bool {
        if building.kind != BuildingKind::Office
            || building.stage != Stage::Open
            || building.anchor
            || building.floors == 0
            || building.floors >= MIN_OFFICE_FLOORS
            || building.occupancy != 0
            || !building.business_active
            || day.saturating_sub(building.built_day) < MIN_BUILDING_AGE_DAYS
            || building.appearance_seed % 4 == 0
            || self
                .state
                .construction
                .pending_rear_extensions
                .iter()
                .any(|plan| plan.building_id == building.id)
            || self
                .state
                .construction
                .building_original_footprints
                .iter()
                .any(|entry| entry.building_id == building.id)
            || self.errand_references_building(building.id)
            || self.state.people.residents.iter().any(|resident| {
                resident.home_building == building.id || resident.work_building == Some(building.id)
            })
            || self.state.people.travelers.iter().any(|traveler| {
                traveler.origin_building == Some(building.id)
                    || traveler.destination_building == Some(building.id)
            })
        {
            return false;
        }
        let Some(parcel) = self
            .state
            .land
            .parcels
            .iter()
            .find(|parcel| parcel.id == building.parcel_id)
        else {
            return false;
        };
        if parcel.protected
            || parcel.reservation.is_some()
            || parcel.rotation.is_none()
            || parcel.width * parcel.depth < MIN_REDEVELOPMENT_SITE_AREA
            || !self.state.transit.station_online
        {
            return false;
        }
        let mapped_to_building = self
            .state
            .network
            .masterplan
            .as_ref()
            .and_then(|progress| progress.lot_parcels.get(lot_index))
            == Some(&building.parcel_id);
        if !mapped_to_building
            || self
                .plan
                .as_ref()
                .and_then(|plan| plan.plan.lots.get(lot_index))
                .is_none_or(|lot| {
                    lot.flag
                        || lot.district != masterplan::District::StationCore
                        || lot.width * lot.depth < MIN_REDEVELOPMENT_SITE_AREA
                })
        {
            return false;
        }
        // Work availability comes from explicit resident assignments, not the commercial occupancy field.
        let has_spare_office_capacity = self
            .state
            .construction
            .buildings
            .iter()
            .filter(|other| {
                other.id != building.id && other.kind == BuildingKind::Office && site_active(other)
            })
            .any(|other| {
                let assigned = self
                    .state
                    .people
                    .residents
                    .iter()
                    .filter(|resident| {
                        resident.routine == ResidentRoutine::Worker
                            && resident.work_building == Some(other.id)
                    })
                    .count();
                assigned < other.capacity as usize
            });
        if !has_spare_office_capacity {
            return false;
        }
        let tower_count = self
            .state
            .construction
            .buildings
            .iter()
            .filter(|other| other.floors >= MIN_OFFICE_FLOORS && other.stage != Stage::Demolished)
            .count()
            + self
                .state
                .construction
                .height_redevelopment
                .iter()
                .filter(|pending| pending.target_floors >= MIN_OFFICE_FLOORS)
                .count();
        tower_count < MAX_REDEVELOPED_TOWERS
    }

    pub(crate) fn begin_height_redevelopment(&mut self, building_id: u32) -> bool {
        let Some(index) = self
            .state
            .construction
            .buildings
            .iter()
            .position(|building| building.id == building_id)
        else {
            return false;
        };
        let building = self.state.construction.buildings[index].clone();
        let Some(parcel) = self
            .state
            .land
            .parcels
            .iter_mut()
            .find(|parcel| parcel.id == building.parcel_id)
        else {
            return false;
        };
        if parcel.reservation.is_some() {
            return false;
        }
        let target_floors = building
            .floors
            .saturating_add(TARGET_FLOOR_INCREMENT)
            .min(MAX_REDEVELOPMENT_FLOORS);
        if target_floors <= building.floors {
            return false;
        }
        let target_capacity = target_floors.saturating_mul(5) as u16;
        let reason = format!(
            "{} begins a staged station-core height renewal",
            building.name
        );
        push_history(
            &mut self.state.construction.buildings[index].history,
            reason.clone(),
        );
        self.state.construction.buildings[index].business_active = false;
        self.state.construction.buildings[index].stage = Stage::Demolition;
        self.state.construction.buildings[index].progress = 0.0;
        parcel.reservation = Some(building.id);
        let project_id = self.id();
        self.state.construction.height_redevelopment = Some(HeightRedevelopment {
            project_id,
            building_id,
            parcel_id: building.parcel_id,
            original_floors: building.floors,
            original_capacity: building.capacity,
            target_floors,
            target_capacity,
        });
        self.record_project(Project {
            id: project_id,
            kind: ProjectKind::Redevelopment,
            from: None,
            to: None,
            parcel_id: Some(building.parcel_id),
            stage: Stage::Demolition,
            progress: 0.0,
            reason: reason.clone(),
            duration: 24.0 + 2.0 * target_floors as f32,
            started_tick: self.state.tick,
        });
        self.record(EventKind::BusinessClosed, building_id, reason);
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }

    pub(crate) fn advance_height_redevelopment(&mut self) {
        let Some(plan) = self.state.construction.height_redevelopment.clone() else {
            return;
        };
        let Some(project_index) = self
            .state
            .construction
            .projects
            .iter()
            .position(|project| project.id == plan.project_id)
        else {
            return;
        };
        let progress = self.state.construction.projects[project_index].progress;
        let Some(building_index) = self
            .state
            .construction
            .buildings
            .iter()
            .position(|building| building.id == plan.building_id)
        else {
            return;
        };
        if progress < DEMOLITION_FRACTION {
            let phase = (progress / DEMOLITION_FRACTION).clamp(0.0, 1.0);
            self.state.construction.buildings[building_index].stage = Stage::Demolition;
            self.state.construction.buildings[building_index].progress = phase;
            self.state.construction.projects[project_index].stage = Stage::Demolition;
        } else {
            let phase =
                ((progress - DEMOLITION_FRACTION) / (1.0 - DEMOLITION_FRACTION)).clamp(0.0, 1.0);
            let stage = if phase < 0.22 {
                Stage::Survey
            } else if phase < 0.48 {
                Stage::Clearing
            } else if phase < 0.73 {
                Stage::Structure
            } else if phase < 0.92 {
                Stage::Surface
            } else {
                Stage::Finishing
            };
            let building = &mut self.state.construction.buildings[building_index];
            building.floors = plan.target_floors;
            building.capacity = plan.target_capacity;
            building.stage = stage;
            building.progress = phase;
            self.state.construction.projects[project_index].stage = stage;
        }
    }

    pub(crate) fn complete_height_redevelopment(&mut self, project_id: u32) -> bool {
        let Some(plan) = self
            .state
            .construction
            .height_redevelopment
            .clone()
            .filter(|plan| plan.project_id == project_id)
        else {
            return false;
        };
        if !self
            .state
            .construction
            .projects
            .iter()
            .any(|project| project.id == project_id && project.progress >= 1.0)
        {
            return false;
        }
        if let Some(building) = self
            .state
            .construction
            .buildings
            .iter_mut()
            .find(|building| building.id == plan.building_id)
        {
            building.floors = plan.target_floors;
            building.capacity = plan.target_capacity;
            building.occupancy = 0;
            building.business_active = true;
            building.stage = Stage::Open;
            building.progress = 1.0;
            building.built_day = (self.state.tick / TICKS_PER_DAY) as u32;
            building.vacant_days = 0;
            push_history(
                &mut building.history,
                format!("{} reopens at {} floors", building.name, plan.target_floors),
            );
        }
        if let Some(parcel) = self
            .state
            .land
            .parcels
            .iter_mut()
            .find(|parcel| parcel.id == plan.parcel_id)
        {
            parcel.reservation = None;
        }
        self.state.construction.height_redevelopment = None;
        self.record(
            EventKind::BuildingRedeveloped,
            plan.building_id,
            format!(
                "A station-core office reopens at {} floors",
                plan.target_floors
            ),
        );
        self.state.revision = self.state.revision.saturating_add(1);
        self.trim_completed_projects();
        true
    }

    pub(crate) fn validate_height_redevelopment(&self) -> Result<(), String> {
        if self.state.construction.height_redevelopment_cursor > MAX_PARCELS {
            return Err("height-redevelopment cursor exceeds its cap".into());
        }
        match self.height_redevelopment_lots.len() {
            eligible_count
                if eligible_count > 0
                    && self.state.construction.height_redevelopment_cursor >= eligible_count =>
            {
                return Err(
                    "height-redevelopment cursor exceeds eligible station-core lots".into(),
                );
            }
            0 if self.state.construction.height_redevelopment_cursor != 0 => {
                return Err("height-redevelopment cursor exists without a master plan".into());
            }
            _ => {}
        }
        let Some(plan) = &self.state.construction.height_redevelopment else {
            if self
                .state
                .construction
                .projects
                .iter()
                .any(|project| project.kind == ProjectKind::Redevelopment && project.progress < 1.0)
            {
                return Err("active redevelopment project has no plan".into());
            }
            return Ok(());
        };
        let project = self
            .state
            .construction
            .projects
            .iter()
            .find(|project| project.id == plan.project_id)
            .ok_or("height-redevelopment plan has no project")?;
        let building = self
            .state
            .construction
            .buildings
            .iter()
            .find(|building| building.id == plan.building_id)
            .ok_or("height-redevelopment plan has no building")?;
        let parcel = self
            .state
            .land
            .parcels
            .iter()
            .find(|parcel| parcel.id == plan.parcel_id)
            .ok_or("height-redevelopment plan has no parcel")?;
        let lot_index = self
            .state
            .network
            .masterplan
            .as_ref()
            .and_then(|progress| {
                progress
                    .lot_parcels
                    .iter()
                    .position(|id| *id == plan.parcel_id)
            })
            .ok_or("height-redevelopment parcel is not mapped to a realized master-plan lot")?;
        let lot = self
            .plan
            .as_ref()
            .and_then(|runtime| runtime.plan.lots.get(lot_index))
            .ok_or("height-redevelopment lot is missing from the regenerated plan")?;
        let demolition = project.progress < DEMOLITION_FRACTION;
        let expected_duration = 24.0 + 2.0 * plan.target_floors as f32;
        let expected_building_progress = if demolition {
            project.progress / DEMOLITION_FRACTION
        } else {
            (project.progress - DEMOLITION_FRACTION) / (1.0 - DEMOLITION_FRACTION)
        };
        let expected_stage = if demolition {
            Stage::Demolition
        } else if expected_building_progress < 0.22 {
            Stage::Survey
        } else if expected_building_progress < 0.48 {
            Stage::Clearing
        } else if expected_building_progress < 0.73 {
            Stage::Structure
        } else if expected_building_progress < 0.92 {
            Stage::Surface
        } else {
            Stage::Finishing
        };
        let expected_target_floors = plan
            .original_floors
            .saturating_add(TARGET_FLOOR_INCREMENT)
            .min(MAX_REDEVELOPMENT_FLOORS);
        let valid = project.kind == ProjectKind::Redevelopment
            && project.progress.is_finite()
            && (0.0..1.0).contains(&project.progress)
            && project.duration.is_finite()
            && project.duration == expected_duration
            && project.parcel_id == Some(plan.parcel_id)
            && project.started_tick <= self.state.tick
            && project.from.is_none()
            && project.to.is_none()
            && project.stage == expected_stage
            && building.stage == expected_stage
            && building.progress.is_finite()
            && (building.progress - expected_building_progress).abs() <= 1.0e-5
            && building.id == plan.building_id
            && building.parcel_id == plan.parcel_id
            && !building.anchor
            && building.kind == BuildingKind::Office
            && !building.business_active
            && building.occupancy == 0
            && parcel.reservation == Some(building.id)
            && plan.original_floors > 0
            && plan.original_floors < MIN_OFFICE_FLOORS
            && building.x == parcel.x
            && building.z == parcel.z
            && building.width == parcel.width
            && building.depth == parcel.depth
            && parcel
                .rotation
                .is_some_and(|rotation| (rotation - building.rotation).abs() <= 1.0e-4)
            && parcel.width * parcel.depth >= MIN_REDEVELOPMENT_SITE_AREA
            && !parcel.protected
            && !lot.flag
            && lot.district == masterplan::District::StationCore
            && lot.x == parcel.x
            && lot.z == parcel.z
            && lot.width == parcel.width
            && lot.depth == parcel.depth
            && (lot.rotation - building.rotation).abs() <= 1.0e-4
            && plan.target_floors == expected_target_floors
            && plan.original_capacity > 0
            && plan.target_capacity == plan.target_floors as u16 * 5
            && if demolition {
                building.floors == plan.original_floors
                    && building.capacity == plan.original_capacity
            } else {
                building.floors == plan.target_floors && building.capacity == plan.target_capacity
            };
        if !valid {
            return Err("height-redevelopment plan, host, parcel, and project disagree".into());
        }
        if self.errand_references_building(building.id)
            || self.state.people.residents.iter().any(|resident| {
                resident.home_building == building.id || resident.work_building == Some(building.id)
            })
            || self.state.people.travelers.iter().any(|traveler| {
                traveler.origin_building == Some(building.id)
                    || traveler.destination_building == Some(building.id)
            })
        {
            return Err("height-redevelopment host still has actor references".into());
        }
        if self.state.construction.projects.iter().any(|other| {
            other.id != plan.project_id
                && other.kind == ProjectKind::Redevelopment
                && other.progress < 1.0
        }) {
            return Err("multiple active height-redevelopment projects".into());
        }
        Ok(())
    }
}
