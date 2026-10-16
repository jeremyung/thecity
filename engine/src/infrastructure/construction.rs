use crate::*;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_PROJECTS: usize = 12;

pub(crate) const COMPLETED_PROJECT_HISTORY: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Building {
    pub id: u32,
    pub parcel_id: u32,
    pub x: f32,
    pub z: f32,
    pub rotation: f32,
    pub width: f32,
    pub depth: f32,
    pub floors: u8,
    pub kind: BuildingKind,
    pub stage: Stage,
    pub progress: f32,
    pub capacity: u16,
    pub occupancy: u16,
    pub built_day: u32,
    pub name: String,
    pub appearance_seed: u32,
    pub history: Vec<String>,
    pub anchor: bool,
    pub vacant_days: u16,
    #[serde(default)]
    pub business_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: u32,
    pub kind: ProjectKind,
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub parcel_id: Option<u32>,
    pub stage: Stage,
    pub progress: f32,
    pub reason: String,
    pub duration: f32,
    pub started_tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildingOriginalFootprint {
    pub building_id: u32,
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub rotation: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NeighborhoodProfile {
    HistoricCore,
    StationCore,
    StationFringe,
    OuterNeighborhood,
}

/// A protected lot kept as a pocket park. Untagged protected lots from older
/// saves are parks too; shrines and the station plaza are not.
pub(crate) fn is_pocket_park(parcel: &Parcel) -> bool {
    parcel.protected && !matches!(parcel.open_space.as_deref(), Some("shrine" | "plaza"))
}

pub(crate) fn is_housing(building: &Building) -> bool {
    matches!(building.kind, BuildingKind::Home | BuildingKind::Apartment)
}

pub(crate) fn is_commercial_kind(kind: BuildingKind) -> bool {
    matches!(
        kind,
        BuildingKind::Shop | BuildingKind::Cafe | BuildingKind::Office
    )
}

pub(crate) fn is_commercial(building: &Building) -> bool {
    is_commercial_kind(building.kind)
}

pub(crate) fn site_active(building: &Building) -> bool {
    if building.stage != Stage::Open {
        return false;
    }
    if is_housing(building) {
        building.occupancy > 0
    } else if is_commercial(building) {
        building.business_active
    } else {
        false
    }
}

pub(crate) fn site_vacant(building: &Building) -> bool {
    if building.stage != Stage::Open {
        return false;
    }
    if is_housing(building) {
        building.occupancy == 0
    } else if is_commercial(building) {
        !building.business_active
    } else {
        false
    }
}

impl Engine {
    pub(crate) fn add_building(
        &mut self,
        parcel: u32,
        kind: BuildingKind,
        floors: u8,
        stage: Stage,
        progress: f32,
        capacity: u16,
        name: &str,
        anchor: bool,
        history: Vec<String>,
    ) -> u32 {
        let Some(p) = self.state.parcels.iter().find(|p| p.id == parcel).cloned() else {
            return 0;
        };
        let id = self.id();
        let appearance_seed = rng_next(&mut self.state.rng_growth);
        let rotation = if anchor {
            0.0
        } else {
            p.rotation.unwrap_or_else(|| self.frontage_rotation(parcel))
        };
        self.state.buildings.push(Building {
            id,
            parcel_id: parcel,
            x: p.x,
            z: p.z,
            rotation,
            width: p.width,
            depth: p.depth,
            floors,
            kind,
            stage,
            progress,
            capacity,
            occupancy: 0,
            built_day: 0,
            name: name.into(),
            appearance_seed,
            history,
            anchor,
            vacant_days: 0,
            business_active: is_commercial_kind(kind) && stage == Stage::Open,
        });
        id
    }

    pub(crate) fn advance_projects(&mut self) {
        let mut completed = Vec::new();
        let mut spine_road_progress = Vec::new();
        for p in &mut self.state.projects {
            if p.progress >= 1.0 {
                continue;
            }
            let work = 1.0 / p.duration.max(1.0);
            p.progress = (p.progress + work).min(1.0);
            p.stage = if p.progress < 0.22 {
                Stage::Survey
            } else if p.progress < 0.48 {
                Stage::Clearing
            } else if p.progress < 0.73 {
                Stage::Structure
            } else if p.progress < 0.92 {
                Stage::Surface
            } else {
                Stage::Finishing
            };
            if p.kind == ProjectKind::Building {
                if let Some(parcel_id) = p.parcel_id {
                    if let Some(building) = self
                        .state
                        .buildings
                        .iter_mut()
                        .find(|b| b.parcel_id == parcel_id && b.stage != Stage::Open)
                    {
                        building.stage = p.stage.clone();
                        building.progress = p.progress;
                    }
                }
            }
            if p.kind == ProjectKind::Road
                && self
                    .state
                    .pending_interior_access
                    .as_ref()
                    .is_some_and(|plan| plan.project_id == p.id)
            {
                if let Some(plan) = self.state.pending_interior_access.as_ref() {
                    if let Some(road) = self
                        .state
                        .roads
                        .iter_mut()
                        .find(|road| road.id == plan.branch_road_id)
                    {
                        road.stage = p.stage.clone();
                        road.progress = p.progress;
                    }
                }
            }
            if p.kind == ProjectKind::Road {
                spine_road_progress.push((p.id, p.stage.clone(), p.progress));
            }
            if p.progress >= 1.0 {
                completed.push(p.id);
            }
        }
        for (project_id, stage, progress) in spine_road_progress {
            self.sync_neighborhood_spine_project(project_id, stage, progress);
        }
        self.advance_height_redevelopment();
        self.sync_masterplan_road_projects();
        for id in completed {
            self.complete_project(id);
        }
    }

    pub(crate) fn complete_project(&mut self, id: u32) {
        let Some(p) = self.state.projects.iter().find(|p| p.id == id).cloned() else {
            return;
        };
        if self
            .state
            .pending_interior_access
            .as_ref()
            .is_some_and(|plan| plan.project_id == id)
        {
            self.complete_interior_access_project(id);
            return;
        }
        if self.complete_height_redevelopment(id) {
            return;
        }
        if p.kind == ProjectKind::Extension {
            self.complete_rear_extension_project(id);
            return;
        }
        if self.masterplan_complete_project(&p) {
            self.mark_access_geometry_changed();
            self.state.revision = self.state.revision.saturating_add(1);
            self.trim_completed_projects();
            return;
        }
        if self.complete_neighborhood_spine_project(id) {
            self.retire_completed_neighborhood_plan();
            self.mark_access_geometry_changed();
            self.state.revision = self.state.revision.saturating_add(1);
            self.trim_completed_projects();
            return;
        }
        match p.kind {
            ProjectKind::Road => {
                if let (Some(a), Some(b)) = (p.from, p.to) {
                    let kind = if let (Some(na), Some(nb)) = (self.node(a), self.node(b)) {
                        if (na.x - nb.x).abs() > 100.0 && (na.x < -100.0) != (nb.x < -100.0) {
                            RoadKind::Bridge
                        } else {
                            RoadKind::Local
                        }
                    } else {
                        RoadKind::Local
                    };
                    let rid = if let Some(road) = self.state.roads.iter_mut().find(|r| {
                        ((r.from == a && r.to == b) || (r.from == b && r.to == a))
                            && r.stage != Stage::Open
                    }) {
                        road.stage = Stage::Open;
                        road.progress = 1.0;
                        road.id
                    } else {
                        self.add_road(a, b, kind, Stage::Open, 1.0)
                    };
                    self.recompute_connectivity();
                    self.remember_neighborhood_plan_road(rid, a, b);
                    if let Some(r) = self.state.roads.iter_mut().find(|r| r.id == rid) {
                        if r.connected {
                            self.record(
                                EventKind::RoadOpened,
                                rid,
                                format!(
                                    "A {} opens and extends the connected street network",
                                    kind
                                ),
                            );
                        }
                    }
                    if self.state.roads.iter().any(|r| r.id == rid && r.connected) {
                        #[cfg(test)]
                        let frontage_roads = if self.try_plan_access_at_road_opening(rid) {
                            self.state
                                .pending_interior_access
                                .as_ref()
                                .map(|plan| vec![plan.host_road_id, plan.host_second_half_road_id])
                                .unwrap_or_else(|| vec![rid])
                        } else {
                            vec![rid]
                        };
                        #[cfg(not(test))]
                        let frontage_roads = vec![rid];
                        let previous_parcels = self.state.parcels.len();
                        for frontage_road in frontage_roads {
                            self.add_frontage_parcels(frontage_road);
                        }
                        if self.state.parcels.len() > previous_parcels {
                            self.state.road_infill_due =
                                self.state.road_infill_due.saturating_add(2).min(9);
                        }
                    }
                }
            }
            ProjectKind::Building => {
                if let Some(pid) = p.parcel_id {
                    let mut released_parcel_reservation = false;
                    if let Some(parcel) = self.state.parcels.iter_mut().find(|x| x.id == pid) {
                        released_parcel_reservation = parcel.reservation.take().is_some();
                    }
                    if released_parcel_reservation {
                        self.mark_access_geometry_changed();
                    }
                    if let Some(b) = self
                        .state
                        .buildings
                        .iter_mut()
                        .find(|b| b.parcel_id == pid && b.stage != Stage::Open)
                    {
                        b.stage = Stage::Open;
                        b.progress = 1.0;
                        b.built_day = (self.state.tick / TICKS_PER_DAY) as u32;
                        b.vacant_days = 0;
                        let id = b.id;
                        let kind = b.kind.clone();
                        let name = b.name.clone();
                        let reopening = is_commercial(b)
                            && b.history
                                .iter()
                                .any(|entry| entry.starts_with("Business closed:"));
                        let added = if is_housing(b) {
                            b.occupancy = b.capacity.min(4);
                            b.occupancy as u32
                        } else {
                            b.occupancy = 0;
                            b.business_active = is_commercial(b);
                            0
                        };
                        if reopening {
                            self.record(
                                EventKind::BusinessReopened,
                                id,
                                format!("{} reopens as a {} after tenant renewal", name, kind),
                            );
                        } else {
                            self.record(
                                EventKind::BuildingOpened,
                                id,
                                format!("{} opens as a {}", name, kind),
                            );
                        }
                        self.state.population = self.state.population.saturating_add(added);
                        self.state.population = self.state.population.min(MAX_POPULATION);
                    }
                }
            }
            ProjectKind::Rail => {
                let mut opened_rail = false;
                if let Some(road) = self.state.roads.iter_mut().find(|r| {
                    r.kind == RoadKind::Rail && Some(r.from) == p.from && Some(r.to) == p.to
                }) {
                    opened_rail = road.stage != Stage::Open;
                    road.stage = Stage::Open;
                    road.progress = 1.0;
                }
                if opened_rail {
                    self.mark_access_geometry_changed();
                }
                self.recompute_connectivity();
                self.record(
                    EventKind::RailOpened,
                    id,
                    "The completed rail link joins the connected transport network".into(),
                );
                self.state.station_online =
                    self.state.station_ready
                        && self.state.roads.iter().any(|r| {
                            r.kind == RoadKind::Rail && r.stage == Stage::Open && r.connected
                        });
                if self.state.station_online {
                    self.record(
                        EventKind::StationServiceStarted,
                        id,
                        "The completed platform and rail link begin service".into(),
                    );
                }
            }
            ProjectKind::Station => {
                self.state.station_ready = true;
                self.state.station_online = self
                    .state
                    .roads
                    .iter()
                    .any(|r| r.kind == RoadKind::Rail && r.stage == Stage::Open && r.connected);
                if self.state.station_online {
                    self.record(
                        EventKind::StationServiceStarted,
                        id,
                        "The station begins service after its line and platform are ready".into(),
                    );
                }
            }
            _ => {}
        }
        self.retire_completed_neighborhood_plan();
        if p.kind == ProjectKind::Road {
            // The active-road-project corridor is removed at completion even
            // when malformed/legacy project references meant no road opened.
            self.mark_access_geometry_changed();
        }
        self.state.revision = self.state.revision.saturating_add(1);
        self.trim_completed_projects();
    }

    pub(crate) fn trim_completed_projects(&mut self) {
        while self.state.projects.len() > MAX_PROJECTS + COMPLETED_PROJECT_HISTORY {
            let Some(index) = self.state.projects.iter().position(|p| p.progress >= 1.0) else {
                break;
            };
            self.state.projects.remove(index);
        }
    }

    pub(crate) fn project_admission_available(&self) -> bool {
        let active = self
            .state
            .projects
            .iter()
            .filter(|project| project.progress < 1.0)
            .count();
        active < MAX_PROJECTS
            && (self.state.projects.len() < MAX_PROJECTS + COMPLETED_PROJECT_HISTORY
                || self
                    .state
                    .projects
                    .iter()
                    .any(|project| project.progress >= 1.0))
    }

    pub(crate) fn record_project(&mut self, project: Project) {
        let adds_pending_road = project.kind == ProjectKind::Road && project.progress < 1.0;
        while self.state.projects.len() >= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY {
            let Some(index) = self
                .state
                .projects
                .iter()
                .position(|entry| entry.progress >= 1.0)
            else {
                unreachable!("active project capacity must leave completed history to trim");
            };
            self.state.projects.remove(index);
        }
        self.state.projects.push(project);
        if adds_pending_road {
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn neighborhood_profile(&self, x: f32, z: f32) -> NeighborhoodProfile {
        let station_distance = self
            .state
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Station)
            .map(|node| (node.x - x).hypot(node.z - z))
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY);
        if self.state.station_online && station_distance <= 112.0 {
            return NeighborhoodProfile::StationCore;
        }
        let market_distance = self
            .state
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Market)
            .map(|node| (node.x - x).hypot(node.z - z))
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY);
        if market_distance <= 118.0 {
            NeighborhoodProfile::HistoricCore
        } else if self.state.station_online && station_distance <= 205.0 {
            NeighborhoodProfile::StationFringe
        } else {
            NeighborhoodProfile::OuterNeighborhood
        }
    }

    pub(crate) fn choose_building_profile(
        &mut self,
        parcel: &Parcel,
        homes: usize,
        shops: usize,
    ) -> (BuildingKind, u16, u8, &'static str) {
        let profile = self.neighborhood_profile(parcel.x, parcel.z);
        let station_tower = self.is_station_tower_parcel(parcel);
        let mut weights = match profile {
            // Keep the first market streets small, walkable, and mixed with
            // neighborhood commerce even after the rail line opens.
            NeighborhoodProfile::HistoricCore => [48.0, 10.0, 24.0, 4.0, 14.0],
            // Concentrate the tallest fabric around the platform itself.
            NeighborhoodProfile::StationCore => [8.0, 10.0, 12.0, 30.0, 40.0],
            NeighborhoodProfile::StationFringe => [38.0, 10.0, 15.0, 14.0, 23.0],
            NeighborhoodProfile::OuterNeighborhood => [70.0, 7.0, 10.0, 4.0, 9.0],
        };
        if station_tower {
            // Reserve large station-facing lots for substantial employment and
            // housing, rather than letting them become oversized low-rise pads.
            weights = [0.0, 0.0, 0.0, 46.0, 54.0];
        }

        // Local balancing creates a varied street face without making districts
        // look like a checkerboard. Keep the census neighborhood-sized and use
        // exact use classes so homes and apartments can still form coherent runs.
        let mut local_use_counts = [0usize; 5];
        let mut local_floor_counts = [0usize; 46];
        for building in self.state.buildings.iter().filter(|building| {
            building.stage == Stage::Open
                && (building.x - parcel.x).hypot(building.z - parcel.z) <= 58.0
        }) {
            let index = match building.kind {
                BuildingKind::Home => Some(0),
                BuildingKind::Shop => Some(1),
                BuildingKind::Cafe => Some(2),
                BuildingKind::Office => Some(3),
                BuildingKind::Apartment => Some(4),
            };
            if let Some(index) = index {
                local_use_counts[index] += 1;
            }
            if let Some(count) = local_floor_counts.get_mut(building.floors as usize) {
                *count += 1;
            }
        }
        for (weight, count) in weights.iter_mut().zip(local_use_counts) {
            *weight /= 1.0 + count as f32 * 0.62;
        }
        if homes < 3 {
            weights = [100.0, 0.0, 0.0, 0.0, 0.0];
        } else if shops < 2 {
            weights = [0.0, 58.0, 42.0, 0.0, 0.0];
        }
        let total: f32 = weights.iter().sum();
        let mut choice = random_unit(&mut self.state.rng_growth) * total;
        let mut selected = 0usize;
        for (index, weight) in weights.iter().enumerate() {
            if choice < *weight {
                selected = index;
                break;
            }
            choice -= *weight;
        }
        let kind = [
            BuildingKind::Home,
            BuildingKind::Shop,
            BuildingKind::Cafe,
            BuildingKind::Office,
            BuildingKind::Apartment,
        ][selected];
        let floor_options: &[(u8, f32)] = match (profile, kind) {
            (NeighborhoodProfile::HistoricCore, BuildingKind::Home) => {
                &[(1, 18.0), (2, 58.0), (3, 24.0)]
            }
            (NeighborhoodProfile::HistoricCore, BuildingKind::Shop | BuildingKind::Cafe) => {
                &[(1, 68.0), (2, 32.0)]
            }
            (NeighborhoodProfile::HistoricCore, BuildingKind::Office) => &[(2, 55.0), (3, 45.0)],
            (NeighborhoodProfile::HistoricCore, BuildingKind::Apartment) => {
                &[(2, 38.0), (3, 47.0), (4, 15.0)]
            }
            (_, BuildingKind::Office) if station_tower => &[
                (20, 12.0),
                (24, 16.0),
                (28, 18.0),
                (32, 18.0),
                (36, 16.0),
                (40, 12.0),
                (45, 8.0),
            ],
            (_, BuildingKind::Apartment) if station_tower => &[
                (20, 14.0),
                (24, 17.0),
                (28, 19.0),
                (32, 18.0),
                (36, 14.0),
                (40, 11.0),
                (45, 7.0),
            ],
            (NeighborhoodProfile::StationCore, BuildingKind::Home) => &[
                (3, 8.0),
                (4, 20.0),
                (5, 28.0),
                (6, 22.0),
                (7, 14.0),
                (8, 8.0),
            ],
            (NeighborhoodProfile::StationCore, BuildingKind::Shop | BuildingKind::Cafe) => {
                &[(2, 48.0), (3, 38.0), (4, 14.0)]
            }
            (NeighborhoodProfile::StationCore, BuildingKind::Office) => &[
                (5, 12.0),
                (6, 18.0),
                (7, 18.0),
                (8, 18.0),
                (9, 14.0),
                (10, 10.0),
                (12, 10.0),
            ],
            (NeighborhoodProfile::StationCore, BuildingKind::Apartment) => &[
                (6, 12.0),
                (7, 17.0),
                (8, 18.0),
                (9, 18.0),
                (10, 16.0),
                (12, 19.0),
            ],
            (NeighborhoodProfile::StationFringe, BuildingKind::Home) => &[
                (2, 8.0),
                (3, 24.0),
                (4, 30.0),
                (5, 22.0),
                (6, 12.0),
                (8, 4.0),
            ],
            (NeighborhoodProfile::StationFringe, BuildingKind::Shop | BuildingKind::Cafe) => {
                &[(1, 35.0), (2, 52.0), (3, 13.0)]
            }
            (NeighborhoodProfile::StationFringe, BuildingKind::Office) => &[
                (4, 10.0),
                (5, 18.0),
                (6, 20.0),
                (7, 18.0),
                (8, 14.0),
                (10, 12.0),
                (12, 8.0),
            ],
            (NeighborhoodProfile::StationFringe, BuildingKind::Apartment) => &[
                (4, 7.0),
                (5, 14.0),
                (6, 18.0),
                (7, 18.0),
                (8, 16.0),
                (10, 15.0),
                (12, 12.0),
            ],
            (NeighborhoodProfile::OuterNeighborhood, BuildingKind::Home) => {
                &[(1, 22.0), (2, 55.0), (3, 23.0)]
            }
            (NeighborhoodProfile::OuterNeighborhood, BuildingKind::Shop | BuildingKind::Cafe) => {
                &[(1, 72.0), (2, 28.0)]
            }
            (NeighborhoodProfile::OuterNeighborhood, BuildingKind::Office) => {
                &[(2, 32.0), (3, 68.0)]
            }
            (NeighborhoodProfile::OuterNeighborhood, BuildingKind::Apartment) => {
                &[(3, 44.0), (4, 46.0), (5, 10.0)]
            }
            _ => &[(2, 30.0), (3, 52.0), (4, 18.0)],
        };
        let mut floor_weight: Vec<(u8, f32)> = floor_options
            .iter()
            .map(|(floor, weight)| {
                let neighbors = local_floor_counts
                    .get(*floor as usize)
                    .copied()
                    .unwrap_or(0);
                (*floor, *weight / (1.0 + neighbors as f32 * 0.55))
            })
            .collect();
        let total_floor_weight: f32 = floor_weight.iter().map(|(_, weight)| *weight).sum();
        let mut floor_roll = random_unit(&mut self.state.rng_growth) * total_floor_weight;
        let mut floors = floor_weight.last().map(|(floor, _)| *floor).unwrap_or(2);
        for (floor, weight) in floor_weight.drain(..) {
            if floor_roll < weight {
                floors = floor;
                break;
            }
            floor_roll -= weight;
        }
        let capacity = match kind {
            BuildingKind::Home => 6 + floors as u16 * 2,
            BuildingKind::Apartment => floors as u16 * 5,
            BuildingKind::Office => floors as u16 * 5,
            _ => 6 + floors as u16 * 2,
        };
        let name = match (profile, kind, station_tower) {
            (_, BuildingKind::Office, true) => "Station Tower Offices",
            (_, BuildingKind::Apartment, true) => "Station Tower Residences",
            (NeighborhoodProfile::HistoricCore, BuildingKind::Home, false) => "Machiya House",
            (NeighborhoodProfile::HistoricCore, BuildingKind::Shop, false) => "Market Shop",
            (NeighborhoodProfile::HistoricCore, BuildingKind::Cafe, false) => "Kissa Komorebi",
            (NeighborhoodProfile::StationCore, BuildingKind::Office, false) => "Station Office",
            (NeighborhoodProfile::StationCore, BuildingKind::Apartment, false) => {
                "Station Apartments"
            }
            (NeighborhoodProfile::StationCore, BuildingKind::Shop, false) => "Station Kiosk",
            (NeighborhoodProfile::StationCore, BuildingKind::Cafe, false) => "Platform Cafe",
            (NeighborhoodProfile::StationFringe, BuildingKind::Apartment, false) => {
                "Railway Apartments"
            }
            (NeighborhoodProfile::StationFringe, BuildingKind::Office, false) => {
                "Neighborhood Office"
            }
            (NeighborhoodProfile::OuterNeighborhood, BuildingKind::Home, false) => "Lane House",
            (_, BuildingKind::Apartment, false) => "Walk-up Apartments",
            (_, BuildingKind::Office, false) => "Small Office",
            (_, BuildingKind::Shop, false) => "Corner Store",
            (_, BuildingKind::Cafe, false) => "Kissa Aoba",
            _ => "Riverside House",
        };
        (kind, capacity, floors, name)
    }

    pub(crate) fn plan_building(&mut self, day: u32) {
        if self.state.buildings.len() + self.state.projects.len() >= MAX_BUILDINGS
            || !self.project_admission_available()
        {
            return;
        }
        let mut choices: Vec<usize> = self
            .state
            .parcels
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                !p.protected
                    && p.reservation.is_none()
                    && (self.state.station_online || !self.is_station_tower_lot(p))
                    && self.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                    && self.state.buildings.iter().all(|b| b.parcel_id != p.id)
                    && self.node_connected(p.access_node)
            })
            .map(|(i, _)| i)
            .collect();
        #[cfg(feature = "legacy")]
        if choices.is_empty() && !self.masterplan_active() {
            self.create_frontier_parcels();
            choices = self
                .state
                .parcels
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    !p.protected
                        && p.reservation.is_none()
                        && (self.state.station_online || !self.is_station_tower_lot(p))
                        && self.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                        && self.state.buildings.iter().all(|b| b.parcel_id != p.id)
                        && self.node_connected(p.access_node)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if choices.is_empty() {
            return;
        }
        let scored_candidates: Vec<_> = choices
            .iter()
            .map(|&parcel_index| {
                let parcel = &self.state.parcels[parcel_index];
                infrastructure::development::BuildingSiteCandidate {
                    parcel_index,
                    parcel_id: parcel.id,
                    x: parcel.x,
                    z: parcel.z,
                }
            })
            .collect();
        let open_buildings: Vec<_> = self
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == Stage::Open)
            .map(|building| infrastructure::development::OpenBuildingPoint {
                x: building.x,
                z: building.z,
                active: site_active(building),
            })
            .collect();
        let Some(selection) = infrastructure::development::select_building_site(
            &scored_candidates,
            &open_buildings,
            self.state.growth_cursor,
        ) else {
            return;
        };
        self.state.growth_cursor = selection.next_cursor;
        let parcel = self.state.parcels[selection.parcel_index].clone();
        let homes = self
            .state
            .buildings
            .iter()
            .filter(|b| matches!(b.kind, BuildingKind::Home | BuildingKind::Apartment))
            .count();
        let shops = self
            .state
            .buildings
            .iter()
            .filter(|b| b.kind == BuildingKind::Shop || b.kind == BuildingKind::Cafe)
            .count();
        let (kind, capacity, floors, name) = self.choose_building_profile(&parcel, homes, shops);
        let (kind, capacity, floors) =
            self.masterplan_adjust_profile(parcel.id, kind, capacity, floors);
        // Test-only matched experiment: before reserving a normal building
        // project, try a deeper initial parcel while retaining the original
        // parcel for use/floor selection and every production admission gate.
        #[cfg(test)]
        let parcel =
            experiments::initial_depth_diagnostic::maybe_expand_selected_parcel(self, parcel);
        let history = vec![format!(
            "Reserved after sustained neighborhood demand on day {}",
            day
        )];
        let building = self.add_building(
            parcel.id,
            kind,
            floors,
            Stage::Foundation,
            0.0,
            capacity,
            name,
            false,
            history,
        );
        if let Some(p) = self.state.parcels.iter_mut().find(|p| p.id == parcel.id) {
            p.reservation = Some(building);
        }
        self.mark_access_geometry_changed();
        let project_id = self.id();
        let duration = 2.0 + random_unit(&mut self.state.rng_growth) * 4.0;
        self.record_project(Project {
            id: project_id,
            kind: ProjectKind::Building,
            from: None,
            to: None,
            parcel_id: Some(parcel.id),
            stage: Stage::Foundation,
            progress: 0.0,
            reason: format!("{} is built to meet nearby demand", name),
            duration,
            started_tick: self.state.tick,
        });
        self.state.road_infill_due = self.state.road_infill_due.saturating_sub(1);
        self.state.revision += 1;
    }
}
