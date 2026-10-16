use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_POPULATION: u32 = 30_000;

// A bounded cast of named inhabitants gives the scene persistent stories without
// simulating every represented citizen as an individual actor.
pub(crate) const MAX_RESIDENTS: usize = 800;

pub(crate) const MAX_ACTIVE_DELIVERIES: usize = 8;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Resident {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) home_building: u32,
    pub(crate) work_building: Option<u32>,
    pub(crate) home_node: u32,
    pub(crate) work_node: Option<u32>,
    pub(crate) mode: u8,
    pub(crate) routine: String,
    pub(crate) activity: String,
    pub(crate) at_work: bool,
    pub(crate) departure_tick: u64,
    pub(crate) return_tick: u64,
    pub(crate) last_departure_day: u32,
    pub(crate) last_return_day: u32,
    pub(crate) history: Vec<String>,
    #[serde(default)]
    pub(crate) rail_commuter: bool,
    #[serde(default)]
    pub(crate) transfer: String,
    #[serde(default)]
    pub(crate) train_id: Option<u32>,
    #[serde(default)]
    pub(crate) queue_slot: Option<u16>,
    #[serde(default)]
    pub(crate) bus_id: Option<u32>,
    #[serde(default)]
    pub(crate) bus_origin_stop: Option<u32>,
    #[serde(default)]
    pub(crate) bus_destination_stop: Option<u32>,
    #[serde(default)]
    pub(crate) bus_stop_node: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pending_alight: Option<transit::transit_recovery::PendingAlight>,
}

impl Engine {
    pub(crate) fn advance_district_measurement(&mut self) {
        let tick = self.state.tick;
        if self.district_runtime.accumulator.is_none()
            && self.state.district_controller.pending.is_none()
            && tick >= self.state.district_controller.next_refresh_tick
        {
            let (buildings, frontages) = population::district_controller::make_samples(
                &self.state.buildings,
                &self.state.parcel_frontages,
            );
            let Ok(accumulator) = population::district_density::MeasurementAccumulator::begin(
                population::district_controller::DISTRICT_GRID,
                &buildings,
                &frontages,
            ) else {
                self.state.district_controller.next_refresh_tick =
                    tick.saturating_add(population::district_controller::REFRESH_INTERVAL_TICKS);
                return;
            };
            let Ok(checkpoint) = accumulator.checkpoint() else {
                self.state.district_controller.next_refresh_tick =
                    tick.saturating_add(population::district_controller::REFRESH_INTERVAL_TICKS);
                return;
            };
            self.state.district_controller.pending = Some(checkpoint);
            self.state.district_controller.pending_capture_tick = tick;
            self.district_runtime.accumulator = Some(accumulator);
        }

        let Some(accumulator) = self.district_runtime.accumulator.as_mut() else {
            return;
        };
        // The input snapshot is frozen at capture. Only its bounded cursor is
        // copied into serialized state on each fixed simulation tick.
        if accumulator
            .advance(population::district_density::MAX_MEASUREMENT_SAMPLES_PER_ADVANCE)
            .is_err()
        {
            self.district_runtime.accumulator = None;
            self.state.district_controller.pending = None;
            self.state.district_controller.pending_capture_tick = 0;
            self.state.district_controller.next_refresh_tick =
                tick.saturating_add(population::district_controller::REFRESH_INTERVAL_TICKS);
            return;
        }
        let processed = accumulator.processed_samples();
        let complete = accumulator.is_complete();
        if complete {
            let accumulator = self.district_runtime.accumulator.take().unwrap();
            let mut checkpoint = self
                .state
                .district_controller
                .pending
                .take()
                .expect("active measurement has a persisted checkpoint");
            checkpoint.processed_samples = processed;
            self.district_runtime.metrics = Some(
                accumulator
                    .finish()
                    .expect("successful completed measurement can be finalized"),
            );
            let controller = &mut self.state.district_controller;
            controller.completed_capture_tick = controller.pending_capture_tick;
            controller.completed_tick = tick;
            controller.completed = Some(checkpoint);
            controller.pending_capture_tick = 0;
            controller.next_refresh_tick =
                tick.saturating_add(population::district_controller::REFRESH_INTERVAL_TICKS);
        } else if let Some(checkpoint) = self.state.district_controller.pending.as_mut() {
            checkpoint.processed_samples = processed;
        }
    }

    pub(crate) fn sync_residents(&mut self) {
        if self.state.residents.len() >= MAX_RESIDENTS {
            return;
        }
        let homes: Vec<(u32, u16, u32)> = self
            .state
            .buildings
            .iter()
            .filter(|b| is_housing(b) && b.stage == "open" && b.occupancy > 0)
            .filter_map(|b| {
                self.state
                    .parcels
                    .iter()
                    .find(|p| p.id == b.parcel_id)
                    .map(|p| (b.id, b.occupancy, p.access_node))
            })
            .collect();
        let destinations: Vec<(u32, u32, String, u16)> = self
            .state
            .buildings
            .iter()
            .filter(|b| is_commercial(b) && site_active(b))
            .filter_map(|b| {
                self.state
                    .parcels
                    .iter()
                    .find(|p| p.id == b.parcel_id)
                    .map(|p| (b.id, p.access_node, b.kind.clone(), b.capacity))
            })
            .collect();
        if destinations.is_empty() {
            return;
        }
        let mut current_by_home = BTreeMap::<u32, u16>::new();
        for resident in &self.state.residents {
            *current_by_home.entry(resident.home_building).or_default() += 1;
        }
        let mut assigned_by_work = BTreeMap::<u32, u16>::new();
        for resident in &self.state.residents {
            if resident.routine == "worker" {
                let Some(work) = resident.work_building else {
                    continue;
                };
                *assigned_by_work.entry(work).or_default() += 1;
            }
        }
        let mut existing_worker_count = self
            .state
            .residents
            .iter()
            .filter(|r| r.routine == "worker")
            .count();
        let max_jobs: usize = destinations
            .iter()
            .map(|(_, _, kind, capacity)| {
                if kind == "office" {
                    *capacity as usize
                } else {
                    (*capacity as usize / 2).max(1)
                }
            })
            .sum();
        for (home_id, occupancy, home_node) in homes {
            let current = *current_by_home.get(&home_id).unwrap_or(&0);
            let available =
                occupancy.saturating_sub(self.aggregate_participants_at_home(home_id) as u16);
            for ordinal in current..available {
                if self.state.residents.len() >= MAX_RESIDENTS {
                    return;
                }
                let mut choices: Vec<(u32, u32, String, u16)> = destinations.clone();
                choices.sort_by_key(|(id, _, _, _)| {
                    (assigned_by_work.get(id).copied().unwrap_or(0), *id)
                });
                let use_job = existing_worker_count < max_jobs && ordinal % 3 != 2;
                let work = if use_job {
                    choices
                        .iter()
                        .find(|(id, _, kind, capacity)| {
                            let jobs = if kind == "office" {
                                *capacity
                            } else {
                                (*capacity / 2).max(1)
                            };
                            assigned_by_work.get(id).copied().unwrap_or(0) < jobs
                        })
                        .cloned()
                        .or_else(|| choices.first().cloned())
                } else {
                    choices
                        .iter()
                        .find(|(_, _, kind, _)| kind == "shop" || kind == "cafe")
                        .cloned()
                        .or_else(|| choices.first().cloned())
                };
                let Some((local_work_id, local_work_node, _, _)) = work else {
                    continue;
                };
                let first_names = [
                    "Aiko", "Haru", "Mina", "Ren", "Sora", "Yui", "Kai", "Nana", "Tomo", "Emi",
                    "Riku", "Mei",
                ];
                let family_names = [
                    "Sato",
                    "Tanaka",
                    "Suzuki",
                    "Kato",
                    "Ito",
                    "Watanabe",
                    "Kobayashi",
                    "Nakamura",
                ];
                let first = first_names
                    [(rng_next(&mut self.state.rng_schedule) as usize) % first_names.len()];
                let family = family_names
                    [(rng_next(&mut self.state.rng_schedule) as usize) % family_names.len()];
                let is_night = use_job && rng_next(&mut self.state.rng_schedule) % 9 == 0;
                let rail_stops = self.rail_terminals();
                let rail_commuter = use_job
                    && !is_night
                    && rail_stops.is_some_and(|(station, _)| {
                        home_node == station || self.route(home_node, station, false).len() >= 2
                    })
                    && rng_next(&mut self.state.rng_schedule) % 5 == 0;
                let (work_id, work_node) = if rail_commuter {
                    (
                        None,
                        rail_stops
                            .map(|(_, gateway)| gateway)
                            .unwrap_or(local_work_node),
                    )
                } else {
                    (Some(local_work_id), local_work_node)
                };
                let routine = if rail_commuter {
                    "rail_commuter"
                } else if use_job {
                    "worker"
                } else {
                    "neighbor"
                }
                .to_string();
                if use_job && !rail_commuter {
                    existing_worker_count += 1;
                    *assigned_by_work.entry(local_work_id).or_default() += 1;
                }
                let id = self.id();
                let (leave, back) = if is_night {
                    (
                        19 * 60 + rng_next(&mut self.state.rng_schedule) % 180,
                        3 * 60 + rng_next(&mut self.state.rng_schedule) % 180,
                    )
                } else {
                    (
                        7 * 60 + rng_next(&mut self.state.rng_schedule) % 150,
                        16 * 60 + rng_next(&mut self.state.rng_schedule) % 240,
                    )
                };
                let departure_tick = next_clock_deadline(self.state.tick, leave as u16);
                let mut return_tick = next_clock_deadline(self.state.tick, back as u16);
                if return_tick <= departure_tick {
                    return_tick = return_tick.saturating_add(TICKS_PER_DAY);
                }
                let mode = if !rail_commuter && rng_next(&mut self.state.rng_schedule) % 5 == 0 {
                    1
                } else {
                    0
                };
                self.state.residents.push(Resident {
                    id,
                    name: format!("{first} {family}"),
                    home_building: home_id,
                    work_building: work_id,
                    home_node,
                    work_node: Some(work_node),
                    mode,
                    routine,
                    activity: "at_home".into(),
                    at_work: false,
                    departure_tick,
                    return_tick,
                    last_departure_day: u32::MAX,
                    last_return_day: u32::MAX,
                    history: vec![format!("Moved into a home near access node {home_node}")],
                    rail_commuter,
                    transfer: if rail_commuter { "home" } else { "" }.into(),
                    train_id: None,
                    queue_slot: None,
                    bus_id: None,
                    bus_origin_stop: None,
                    bus_destination_stop: None,
                    bus_stop_node: None,
                    pending_alight: None,
                });
            }
        }
    }

    pub(crate) fn schedule_residents(&mut self, day: u32) {
        if self.state.travelers.len() >= MAX_TRAVELERS {
            return;
        }
        let active: BTreeSet<u32> = self
            .state
            .travelers
            .iter()
            .filter(|t| t.resident != 0)
            .map(|t| t.resident)
            .collect();
        for index in 0..self.state.residents.len() {
            if self.state.travelers.len() >= MAX_TRAVELERS {
                break;
            }
            let resident = self.state.residents[index].clone();
            if active.contains(&resident.id) {
                continue;
            }
            if matches!(
                resident.transfer.as_str(),
                "walking_to_bus"
                    | "waiting_bus"
                    | "riding_bus"
                    | "walking_from_bus"
                    | "waiting_alight"
            ) {
                continue;
            }
            if resident.rail_commuter {
                self.schedule_rail_resident(index, day);
                continue;
            }
            let (origin, goal, activity, destination_building) = if resident.at_work {
                if self.state.tick < resident.return_tick {
                    continue;
                }
                let Some(work_node) = resident.work_node else {
                    continue;
                };
                (
                    work_node,
                    resident.home_node,
                    "return",
                    Some(resident.home_building),
                )
            } else {
                if self.state.tick < resident.departure_tick {
                    continue;
                }
                let Some(work_node) = resident.work_node else {
                    continue;
                };
                let activity = if resident.routine == "worker" {
                    if resident.departure_tick % TICKS_PER_DAY > TICKS_PER_DAY / 2 {
                        "night_shift"
                    } else {
                        "commute"
                    }
                } else {
                    "stroll"
                };
                (
                    resident.home_node,
                    work_node,
                    activity,
                    resident.work_building,
                )
            };
            let origin_building = if resident.at_work {
                resident.work_building.unwrap_or(resident.home_building)
            } else {
                resident.home_building
            };
            if self.schedule_bus_trip(
                index,
                origin,
                goal,
                activity,
                origin_building,
                destination_building,
            ) {
                continue;
            }
            let route = self.route(origin, goal, false);
            if route.len() < 2 {
                continue;
            }
            let route_lengths = self.route_lengths(&route);
            let speed = if resident.mode == 1 { 7.0 } else { 1.5 };
            let origin_building = Some(origin_building);
            let mut traveler = Traveler {
                id: resident.id,
                kind: resident.mode,
                route,
                route_lengths,
                edge_index: 0,
                edge_progress: 0.0,
                speed,
                destination: goal,
                activity: activity.into(),
                resident: resident.id,
                name: resident.name.clone(),
                origin_building,
                destination_building,
                wait_ticks: 0,
                origin_access: None,
                destination_access: None,
                access_phase: 1,
                access_progress: 0.0,
            };
            if let Some(building) = origin_building {
                traveler.origin_access = self.building_access_path(
                    building,
                    &traveler.route,
                    true,
                    traveler.id,
                    traveler.kind,
                );
            }
            if let Some(building) = destination_building {
                traveler.destination_access = self.building_access_path(
                    building,
                    &traveler.route,
                    false,
                    traveler.id,
                    traveler.kind,
                );
            }
            if traveler.origin_access.is_some() {
                traveler.access_phase = 0;
            }
            self.state.travelers.push(traveler);
            if let Some(r) = self.state.residents.get_mut(index) {
                if r.at_work {
                    r.last_return_day = day;
                    r.return_tick = r.return_tick.saturating_add(TICKS_PER_DAY);
                    r.activity = "returning_home".into();
                } else {
                    r.last_departure_day = day;
                    r.departure_tick = r.departure_tick.saturating_add(TICKS_PER_DAY);
                    r.activity = if r.routine == "worker" {
                        "commuting"
                    } else {
                        "strolling"
                    }
                    .into();
                }
                push_history(
                    &mut r.history,
                    format!("Departed on {activity} at tick {}", self.state.tick),
                );
            }
            self.record(
                "ResidentTripStarted",
                resident.id,
                format!("{} begins a purposeful {activity} trip", resident.name),
            );
        }
    }

    pub(crate) fn schedule_deliveries(&mut self) {
        if self.state.tick < self.state.next_delivery_tick {
            return;
        }
        self.state.next_delivery_tick = self
            .state
            .next_delivery_tick
            .saturating_add(TICKS_PER_DAY / 7);
        let active_deliveries = self
            .state
            .travelers
            .iter()
            .filter(|t| t.activity.starts_with("delivery"))
            .count();
        if active_deliveries >= MAX_ACTIVE_DELIVERIES || self.state.travelers.len() >= MAX_TRAVELERS
        {
            return;
        }
        let Some(gateway) = self
            .state
            .nodes
            .iter()
            .find(|n| n.kind == "gateway")
            .map(|n| n.id)
        else {
            return;
        };
        let businesses: Vec<(u32, u32, String)> = self
            .state
            .buildings
            .iter()
            .filter(|b| matches!(b.kind.as_str(), "shop" | "cafe") && site_active(b))
            .filter_map(|b| {
                self.state
                    .parcels
                    .iter()
                    .find(|p| p.id == b.parcel_id)
                    .map(|p| (b.id, p.access_node, b.name.clone()))
            })
            .collect();
        if businesses.is_empty() {
            return;
        }
        let (building_id, destination, shop_name) =
            businesses[self.state.delivery_cursor % businesses.len()].clone();
        let route = self.route(gateway, destination, false);
        if route.len() < 2 {
            return;
        }
        let route_lengths = self.route_lengths(&route);
        let id = self.id();
        self.state.delivery_cursor = self.state.delivery_cursor.saturating_add(1);
        let mut traveler = Traveler {
            id,
            kind: 1,
            route,
            route_lengths,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 9.0,
            destination,
            activity: "delivery".into(),
            resident: 0,
            name: "Neighborhood Supply Van".into(),
            origin_building: None,
            destination_building: Some(building_id),
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        };
        traveler.destination_access = self.building_access_path(
            building_id,
            &traveler.route,
            false,
            traveler.id,
            traveler.kind,
        );
        self.state.travelers.push(traveler);
        self.record(
            "DeliveryDispatched",
            id,
            format!("A delivery van leaves the gateway for {shop_name}"),
        );
    }

    pub(crate) fn resident_location_node(&self, resident: &Resident) -> Option<u32> {
        if self
            .state
            .travelers
            .iter()
            .any(|traveler| traveler.resident == resident.id)
        {
            return None;
        }
        match resident.transfer.as_str() {
            "home" => Some(resident.home_node),
            "waiting_bus" => resident.bus_stop_node,
            "waiting_alight" => resident
                .pending_alight
                .as_ref()
                .map(|pending| pending.stop_node),
            "riding_bus" => None,
            "waiting_station" => self.rail_terminals().map(|(station, _)| station),
            "waiting_gateway" | "working_external" => resident.work_node,
            "riding_outbound" | "riding_inbound" => None,
            _ if resident.activity == "at_home" => Some(resident.home_node),
            _ if resident.activity == "at_destination" => resident.work_node,
            _ => None,
        }
    }
}
