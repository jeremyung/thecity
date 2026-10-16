use crate::*;
use std::collections::{BTreeMap, BTreeSet};

/// Occupied homes within this distance form a pocket park's catchment.
pub(crate) const POCKET_PARK_CATCHMENT: f32 = 70.0;

/// Occupied homes a catchment needs before a vacant lot becomes a park.
pub(crate) const POCKET_PARK_MIN_HOMES: usize = 8;

/// Minimum distance between pocket parks, so each serves its own streets.
pub(crate) const POCKET_PARK_SPACING: f32 = 120.0;

/// A quarter day between pocket park reviews.
pub(crate) const POCKET_PARK_INTERVAL_TICKS: u64 = TICKS_PER_DAY / 4;

/// City-wide ceiling: at most one pocket park per this many occupied homes.
pub(crate) const HOMES_PER_POCKET_PARK: usize = 30;

/// Ordinary neighborhood lots only; station towers and tiny slivers are skipped.
pub(crate) const POCKET_PARK_AREA: std::ops::RangeInclusive<f32> = 80.0..=520.0;

impl Engine {
    pub(crate) fn development_day(&mut self, day: u32) {
        let occupied: usize = self
            .state
            .buildings
            .iter()
            .filter(|b| site_active(b))
            .count();
        let vacancies: usize = self
            .state
            .buildings
            .iter()
            .filter(|b| site_vacant(b))
            .count();
        self.state.demand = (0.35 + self.state.population as f32 / 42.0 + occupied as f32 * 0.055
            - vacancies as f32 * 0.18)
            .clamp(0.12, 4.0);
        let population = self.state.population;
        let mut household_arrivals = 0u32;
        for b in &mut self.state.buildings {
            if site_vacant(b) {
                b.vacant_days = b.vacant_days.saturating_add(1);
            }
            if b.stage == "open" && is_housing(b) && b.occupancy < b.capacity && b.capacity > 0 {
                let arrivals = ((population as f32 / 100.0).ceil() as u16).max(1);
                let previous = b.occupancy;
                b.occupancy = (b.occupancy + arrivals).min(b.capacity);
                household_arrivals += (b.occupancy - previous) as u32;
            }
        }
        self.state.population = self
            .state
            .population
            .saturating_add(household_arrivals)
            .min(MAX_POPULATION);
        self.advance_projects();
        self.sync_residents();
        if day > 0 && day % 3 == 0 && self.state.last_renewal_day != day {
            self.renewal(day);
        }
    }

    /// Once enough households live around a still-vacant master-plan lot, the
    /// neighborhood keeps it as protected open space. Protected parcels are
    /// never developed; the renderer draws vacant ones as pocket parks.
    /// Selection is deterministic and draws no random numbers.
    pub(crate) fn dedicate_pocket_park(&mut self) {
        if !self.masterplan_active() {
            return;
        }
        let homes: Vec<(f32, f32)> = self
            .state
            .buildings
            .iter()
            .filter(|b| is_housing(b) && site_active(b))
            .map(|b| (b.x, b.z))
            .collect();
        let built: BTreeSet<u32> = self.state.buildings.iter().map(|b| b.parcel_id).collect();
        // Only pocket parks count toward the quota and spacing; the station
        // plaza and neighborhood shrines are separate open space.
        let parks: Vec<(f32, f32)> = self
            .state
            .parcels
            .iter()
            .filter(|p| is_pocket_park(p) && !built.contains(&p.id))
            .map(|p| (p.x, p.z))
            .collect();
        if homes.len() < POCKET_PARK_MIN_HOMES
            || parks.len().saturating_mul(HOMES_PER_POCKET_PARK) >= homes.len()
        {
            return;
        }
        let mut best: Option<(usize, u32)> = None;
        for parcel in &self.state.parcels {
            if parcel.protected
                || parcel.reservation.is_some()
                || built.contains(&parcel.id)
                || !POCKET_PARK_AREA.contains(&(parcel.width * parcel.depth))
                || parks
                    .iter()
                    .any(|&(x, z)| (x - parcel.x).hypot(z - parcel.z) < POCKET_PARK_SPACING)
                || !self.node_connected(parcel.access_node)
                || !self.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                || self.is_station_tower_lot(parcel)
            {
                continue;
            }
            let nearby = homes
                .iter()
                .filter(|&&(x, z)| (x - parcel.x).hypot(z - parcel.z) <= POCKET_PARK_CATCHMENT)
                .count();
            if nearby >= POCKET_PARK_MIN_HOMES && best.is_none_or(|(count, _)| nearby > count) {
                best = Some((nearby, parcel.id));
            }
        }
        let Some((nearby, parcel_id)) = best else {
            return;
        };
        if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
            parcel.protected = true;
            parcel.open_space = Some("park".into());
        }
        self.record(
            "ParkDedicated",
            parcel_id,
            format!("A vacant lot becomes a pocket park for {nearby} nearby homes"),
        );
        self.state.revision = self.state.revision.saturating_add(1);
    }

    pub(crate) fn evaluate_growth(&mut self) {
        if self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .count()
            >= MAX_PROJECTS
        {
            return;
        }
        if self.state.buildings.len() >= MAX_BUILDINGS || self.state.roads.len() >= MAX_SEGMENTS {
            return;
        }
        if self.masterplan_active() {
            self.masterplan_growth();
            return;
        }
        #[cfg(feature = "legacy")]
        self.evaluate_legacy_growth();
    }

    pub(crate) fn renewal(&mut self, day: u32) {
        if !self.project_admission_available() {
            return;
        }
        self.state.last_renewal_day = day;
        if self.try_start_height_redevelopment() {
            return;
        }
        self.try_close_business_for_turnover(day);
        let candidates: Vec<u32> = self
            .state
            .buildings
            .iter()
            .filter(|b| {
                !b.anchor
                    && !self.errand_references_building(b.id)
                    && site_vacant(b)
                    && b.vacant_days > 5
                    && !self
                        .state
                        .pending_rear_extensions
                        .iter()
                        .any(|plan| plan.building_id == b.id)
            })
            .map(|b| b.id)
            .collect();
        if candidates.is_empty() {
            return;
        }
        let i = (rng_next(&mut self.state.rng_growth) as usize) % candidates.len();
        let id = candidates[i];
        let (parcel_id, msg) = if let Some(b) = self.state.buildings.iter_mut().find(|b| b.id == id)
        {
            let msg = format!("{} is renovated after a quiet period", b.name);
            b.history.push(msg.clone());
            if b.history.len() > 10 {
                b.history.remove(0);
            }
            b.stage = "renovation".into();
            b.progress = 0.0;
            b.vacant_days = 0;
            b.business_active = false;
            (b.parcel_id, msg)
        } else {
            return;
        };
        if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
            parcel.reservation = Some(id);
        }
        self.mark_access_geometry_changed();
        let pid = self.id();
        self.record_project(Project {
            id: pid,
            kind: "building".into(),
            from: None,
            to: None,
            parcel_id: Some(parcel_id),
            stage: "structure".into(),
            progress: 0.0,
            reason: msg,
            duration: 2.0,
            started_tick: self.state.tick,
        });
        self.record(
            "RenovationStarted",
            id,
            "A vacant property is renewed for a new neighborhood use".into(),
        );
        self.state.revision += 1;
    }

    pub(crate) fn try_close_business_for_turnover(&mut self, day: u32) -> bool {
        if day == 0 || day % 30 != 0 || self.state.last_business_turnover_day == day {
            return false;
        }
        self.state.last_business_turnover_day = day;
        let mut referenced = BTreeSet::new();
        for resident in &self.state.residents {
            if let Some(building_id) = resident.work_building {
                referenced.insert(building_id);
            }
        }
        for traveler in &self.state.travelers {
            if let Some(building_id) = traveler.destination_building {
                referenced.insert(building_id);
            }
            if let Some(building_id) = traveler.origin_building {
                referenced.insert(building_id);
            }
        }
        let mut active_by_kind = BTreeMap::<String, usize>::new();
        for building in &self.state.buildings {
            if is_commercial(building) && site_active(building) {
                *active_by_kind.entry(building.kind.clone()).or_default() += 1;
            }
        }
        let eligible: Vec<u32> = self
            .state
            .buildings
            .iter()
            .filter(|building| {
                is_commercial(building)
                    && building.business_active
                    && building.stage == "open"
                    && !building.anchor
                    && !self
                        .state
                        .pending_rear_extensions
                        .iter()
                        .any(|plan| plan.building_id == building.id)
                    && day.saturating_sub(building.built_day) >= 30
                    && !referenced.contains(&building.id)
                    && !self.errand_references_building(building.id)
                    && active_by_kind.get(&building.kind).copied().unwrap_or(0) > 1
            })
            .map(|building| building.id)
            .collect();
        if eligible.is_empty() || rng_next(&mut self.state.rng_growth) % 16 != 0 {
            return false;
        }
        let chosen = eligible[rng_next(&mut self.state.rng_growth) as usize % eligible.len()];
        let Some(building) = self
            .state
            .buildings
            .iter_mut()
            .find(|building| building.id == chosen)
        else {
            return false;
        };
        let reason = format!(
            "Business closed: {} pauses for a seeded tenant change on day {}",
            building.name, day
        );
        push_history(&mut building.history, reason.clone());
        building.business_active = false;
        building.occupancy = 0;
        building.vacant_days = 0;
        self.record("BusinessClosed", chosen, reason);
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }
}
