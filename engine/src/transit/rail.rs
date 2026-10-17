use crate::*;
pub(crate) use city_plan::rail_alignment::{
    RAIL_GATEWAY_STAIR_BOTTOM, RAIL_PLATFORM_END, TRAIN_HALF_LENGTH,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const RAIL_TRAIN_CAPACITY: usize = 48;

// Match the renderer presentation bound; ordinary service currently uses one trainset.
pub(crate) const MAX_PRESENTED_TRAINS: usize = 8;

pub(crate) const RAIL_QUEUE_CAPACITY: usize = 96;

pub(crate) const TRAIN_STATION_DWELL_TICKS: u16 = 220;

pub(crate) const TRAIN_GATEWAY_DWELL_TICKS: u16 = 180;

pub(crate) const TRAIN_DOOR_PHASE_TICKS: u16 = 20;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Train {
    pub(crate) id: u32,
    pub(crate) route: Vec<u32>,
    pub(crate) distance: f32,
    pub(crate) speed: f32,
    #[serde(default = "default_train_direction")]
    pub(crate) direction: i8,
    #[serde(default)]
    pub(crate) dwell_ticks: u16,
    #[serde(default)]
    pub(crate) passengers: Vec<u32>,
}

pub(crate) fn train_door_state(train: &Train, rail_length: f32) -> (Option<u32>, f32) {
    let at_station = train.distance <= TRAIN_HALF_LENGTH + 0.01;
    let at_gateway = train.distance >= rail_length - TRAIN_HALF_LENGTH - 0.01;
    let terminal = if train.dwell_ticks > 0 && at_station {
        train.route.first().copied()
    } else if train.dwell_ticks > 0 && at_gateway {
        train.route.last().copied()
    } else {
        None
    };
    let total = if at_station {
        TRAIN_STATION_DWELL_TICKS
    } else {
        TRAIN_GATEWAY_DWELL_TICKS
    };
    let fraction = if train.dwell_ticks > 0 && (at_station || at_gateway) {
        ((total.saturating_sub(train.dwell_ticks) as f32 / TRAIN_DOOR_PHASE_TICKS as f32)
            .clamp(0.0, 1.0))
        .min((train.dwell_ticks as f32 / TRAIN_DOOR_PHASE_TICKS as f32).clamp(0.0, 1.0))
    } else {
        0.0
    };
    (terminal, fraction)
}

pub(crate) fn default_train_direction() -> i8 {
    1
}

impl Engine {
    pub(crate) fn rail_axis(&self) -> Option<(u32, u32, f32, f32)> {
        let (station, gateway) = self.rail_terminals()?;
        let (_, tangent) = self.rail_pose(station, gateway, 0.0)?;
        Some((station, gateway, tangent.x, tangent.z))
    }

    pub(crate) fn allocate_queue_slot(&self, at_station: bool) -> Option<u16> {
        let used: BTreeSet<u16> = self
            .state
            .people
            .residents
            .iter()
            .filter(|resident| {
                if at_station {
                    matches!(
                        resident.transfer,
                        Transfer::WalkingToStation | Transfer::WaitingStation
                    )
                } else {
                    resident.transfer == Transfer::WaitingGateway
                }
            })
            .filter_map(|resident| resident.queue_slot)
            .collect();
        (0..RAIL_QUEUE_CAPACITY as u16).find(|slot| !used.contains(slot))
    }

    pub(crate) fn rail_terminals(&self) -> Option<(u32, u32)> {
        if !self.state.transit.station_online {
            return None;
        }
        let rail = self.state.network.roads.iter().find(|road| {
            road.kind == RoadKind::Rail && road.stage == Stage::Open && road.connected
        })?;
        let station = [rail.from, rail.to].into_iter().find(|id| {
            self.node(*id)
                .is_some_and(|node| node.kind == NodeKind::Station)
        })?;
        let gateway = if station == rail.from {
            rail.to
        } else {
            rail.from
        };
        Some((station, gateway))
    }

    pub(crate) fn rail_service_quiet(&self) -> bool {
        let (_, minute) = clock_at(self.state.tick);
        (60.0..300.0).contains(&minute)
    }

    pub(crate) fn rail_waiting_count(&self, at_station: bool) -> usize {
        let state = if at_station {
            Transfer::WaitingStation
        } else {
            Transfer::WaitingGateway
        };
        self.state
            .people
            .residents
            .iter()
            .filter(|resident| resident.rail_commuter && resident.transfer == state)
            .count()
    }

    pub(crate) fn rail_platform_demand(&self, at_station: bool) -> usize {
        let waiting = self.rail_waiting_count(at_station);
        let walking = if at_station {
            Some(TravelerActivity::WalkingToStation)
        } else {
            // Return commuters join the gateway queue directly when their shift ends.
            None
        };
        waiting
            + self
                .state
                .people
                .travelers
                .iter()
                .filter(|traveler| Some(traveler.activity) == walking)
                .count()
    }

    pub(crate) fn schedule_rail_resident(&mut self, index: usize, day: u32) {
        let Some((station, _gateway)) = self.rail_terminals() else {
            return;
        };
        let resident = self.state.people.residents[index].clone();
        let quiet = self.rail_service_quiet();
        match resident.transfer {
            Transfer::Home
                if !resident.at_work && self.state.tick >= resident.departure_tick && !quiet =>
            {
                if self.rail_platform_demand(true) >= RAIL_QUEUE_CAPACITY {
                    return;
                }
                let Some(queue_slot) = self.allocate_queue_slot(true) else {
                    return;
                };
                let route = self.route(resident.home_node, station, false);
                if route.is_empty() {
                    return;
                }
                if let Some(current) = self.state.people.residents.get_mut(index) {
                    current.queue_slot = Some(queue_slot);
                }
                if route.len() == 1 {
                    if let Some(current) = self.state.people.residents.get_mut(index) {
                        current.transfer = Transfer::WaitingStation;
                        current.activity = ResidentActivity::WaitingForTrain;
                    }
                } else {
                    let route_lengths = self.route_lengths(&route);
                    let mut traveler = Traveler {
                        id: resident.id,
                        kind: 0,
                        route,
                        route_lengths,
                        edge_index: 0,
                        edge_progress: 0.0,
                        speed: 1.5,
                        destination: station,
                        activity: TravelerActivity::WalkingToStation,
                        resident: resident.id,
                        name: resident.name.clone(),
                        origin_building: Some(resident.home_building),
                        destination_building: None,
                        wait_ticks: 0,
                        origin_access: None,
                        destination_access: None,
                        access_phase: 1,
                        access_progress: 0.0,
                    };
                    traveler.origin_access = self.building_access_path(
                        resident.home_building,
                        &traveler.route,
                        true,
                        traveler.id,
                        0,
                    );
                    traveler.destination_access =
                        self.platform_entry_path(&traveler.route, true, queue_slot, traveler.id);
                    if traveler.origin_access.is_some() {
                        traveler.access_phase = 0;
                    }
                    self.state.people.travelers.push(traveler);
                    if let Some(current) = self.state.people.residents.get_mut(index) {
                        current.transfer = Transfer::WalkingToStation;
                        current.activity = ResidentActivity::WalkingToStation;
                    }
                }
                if let Some(current) = self.state.people.residents.get_mut(index) {
                    current.last_departure_day = day;
                    current.departure_tick = current.departure_tick.saturating_add(TICKS_PER_DAY);
                    push_history(
                        &mut current.history,
                        format!("Started walking to the station at tick {}", self.state.tick),
                    );
                }
                self.record(
                    EventKind::RailTripStarted,
                    resident.id,
                    format!(
                        "{} begins a rail commute toward the regional gateway",
                        resident.name
                    ),
                );
            }
            Transfer::WorkingExternal
                if resident.at_work && self.state.tick >= resident.return_tick && !quiet =>
            {
                let Some(queue_slot) = self.allocate_queue_slot(false) else {
                    return;
                };
                if let Some(current) = self.state.people.residents.get_mut(index) {
                    current.transfer = Transfer::WaitingGateway;
                    current.activity = ResidentActivity::WaitingForTrain;
                    current.queue_slot = Some(queue_slot);
                    current.last_return_day = day;
                    current.return_tick = current.return_tick.saturating_add(TICKS_PER_DAY);
                    push_history(
                        &mut current.history,
                        format!(
                            "Finished an external shift and queued for the return train at tick {}",
                            self.state.tick
                        ),
                    );
                }
                self.record(
                    EventKind::RailReturnQueued,
                    resident.id,
                    format!(
                        "{} waits at the regional gateway for the return shuttle",
                        resident.name
                    ),
                );
            }
            _ => {}
        }
    }

    pub(crate) fn advance_trains(&mut self) {
        let usable_rail = self
            .state
            .network
            .roads
            .iter()
            .find(|r| r.kind == RoadKind::Rail && r.stage == Stage::Open && r.connected)
            .cloned();
        let Some(rail) = usable_rail
            .filter(|r| self.state.transit.station_online && r.length >= TRAIN_HALF_LENGTH * 2.0)
        else {
            self.state.transit.trains.clear();
            return;
        };
        let station = self
            .state
            .network
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Station && (n.id == rail.from || n.id == rail.to))
            .map(|n| n.id);
        let Some(station) = station else {
            self.state.transit.trains.clear();
            return;
        };
        let gateway = if rail.from == station {
            rail.to
        } else {
            rail.from
        };
        let route = vec![station, gateway];
        if self.state.transit.trains.is_empty() {
            let id = self.id();
            self.state.transit.trains.push(Train {
                id,
                route: route.clone(),
                distance: TRAIN_HALF_LENGTH,
                speed: 18.0,
                direction: 1,
                dwell_ticks: TRAIN_STATION_DWELL_TICKS,
                passengers: Vec::new(),
            });
        }
        let mut stops = Vec::new();
        let mut opened_terminals = Vec::new();
        let quiet = self.rail_service_quiet();
        let station_queue_empty = self.rail_waiting_count(true) == 0;
        for train in &mut self.state.transit.trains {
            if train.route != route {
                train.route = route.clone();
                train.distance = TRAIN_HALF_LENGTH;
                train.direction = 1;
                train.dwell_ticks = TRAIN_STATION_DWELL_TICKS;
                train.passengers.clear();
            }
            let at_station = train.direction >= 0 && train.distance <= TRAIN_HALF_LENGTH + 0.01;
            if quiet && at_station && train.passengers.is_empty() && station_queue_empty {
                train.dwell_ticks = TRAIN_STATION_DWELL_TICKS;
                continue;
            }
            if train.dwell_ticks > 0 {
                train.dwell_ticks -= 1;
                let at_station = train.direction >= 0 && train.distance <= TRAIN_HALF_LENGTH + 0.01;
                let dwell_total = if at_station {
                    TRAIN_STATION_DWELL_TICKS
                } else {
                    TRAIN_GATEWAY_DWELL_TICKS
                };
                if train.dwell_ticks == dwell_total - TRAIN_DOOR_PHASE_TICKS {
                    opened_terminals.push((train.id, at_station));
                }
                continue;
            }
            let step = train.speed / TICKS_PER_SECOND as f32;
            if train.direction >= 0 {
                train.distance += step;
                let far_terminal = rail.length - TRAIN_HALF_LENGTH;
                if train.distance >= far_terminal {
                    train.distance = far_terminal;
                    train.direction = -1;
                    train.dwell_ticks = TRAIN_GATEWAY_DWELL_TICKS;
                    stops.push((
                        train.id,
                        EventKind::TrainGatewayStop,
                        "The regional shuttle pauses at its outer terminus",
                    ));
                }
            } else {
                train.distance -= step;
                if train.distance <= TRAIN_HALF_LENGTH {
                    train.distance = TRAIN_HALF_LENGTH;
                    train.direction = 1;
                    train.dwell_ticks = TRAIN_STATION_DWELL_TICKS;
                    stops.push((
                        train.id,
                        EventKind::TrainStationStop,
                        "The shuttle pauses at the neighborhood platform",
                    ));
                }
            }
        }
        for (id, kind, description) in stops {
            self.record(kind, id, description.into());
        }
        for (id, at_station) in opened_terminals {
            self.handle_train_terminal(id, at_station, station, gateway);
        }
        let train_ids: Vec<u32> = self
            .state
            .transit
            .trains
            .iter()
            .map(|train| train.id)
            .collect();
        for id in train_ids {
            self.board_waiting_train(id, station, gateway);
        }
    }

    pub(crate) fn handle_train_terminal(
        &mut self,
        train_id: u32,
        at_station: bool,
        station: u32,
        gateway: u32,
    ) {
        let passenger_ids = self
            .state
            .transit
            .trains
            .iter()
            .find(|train| train.id == train_id)
            .map(|train| train.passengers.clone())
            .unwrap_or_default();
        let mut unloaded = BTreeSet::new();
        for resident_id in passenger_ids {
            let Some(resident_index) = self
                .state
                .people
                .residents
                .iter()
                .position(|resident| resident.id == resident_id)
            else {
                unloaded.insert(resident_id);
                continue;
            };
            let resident = self.state.people.residents[resident_index].clone();
            if at_station && resident.transfer == Transfer::RidingInbound {
                let queue_slot = (resident.id % RAIL_QUEUE_CAPACITY as u32) as u16;
                let wait_position = self.platform_position(true, queue_slot);
                let route_attempted = resident.home_node != station
                    && self.state.people.travelers.len() < MAX_TRAVELERS;
                let route = if resident.home_node == station {
                    vec![station]
                } else if route_attempted {
                    self.route(station, resident.home_node, false)
                } else {
                    Vec::new()
                };
                if route.is_empty() {
                    let queued = wait_position.is_some_and(|wait_position| {
                        self.queue_pending_alight(
                            resident_id,
                            station,
                            resident.home_node,
                            resident.home_building,
                            TravelerActivity::WalkingHome,
                            Transfer::WalkingHome,
                            wait_position,
                            route_attempted,
                        )
                    });
                    if !queued {
                        let reason = format!(
                            "Still aboard at station {station}: pedestrian route queue is full"
                        );
                        let current = &mut self.state.people.residents[resident_index];
                        if current.history.last() != Some(&reason) {
                            push_history(&mut current.history, reason);
                        }
                        continue;
                    }
                    unloaded.insert(resident_id);
                    push_history(
                        &mut self.state.people.residents[resident_index].history,
                        format!(
                            "Waited at station {station} for a legal walking route at tick {}",
                            self.state.tick
                        ),
                    );
                    self.record(
                        EventKind::TrainPassengerWaitingForRoute,
                        resident_id,
                        format!("A resident alights from train {train_id} into the bounded route queue at station {station}"),
                    );
                    continue;
                }
                if route.len() == 1 {
                    unloaded.insert(resident_id);
                    let current = &mut self.state.people.residents[resident_index];
                    current.transfer = Transfer::Home;
                    current.activity = ResidentActivity::AtHome;
                    current.at_work = false;
                    current.train_id = None;
                    current.queue_slot = None;
                    push_history(
                        &mut current.history,
                        format!("Returned home from the gateway at tick {}", self.state.tick),
                    );
                    self.record(
                        EventKind::TrainPassengerAlighted,
                        train_id,
                        format!(
                            "{} completes the inbound trip at the station",
                            resident.name
                        ),
                    );
                } else {
                    unloaded.insert(resident_id);
                    let mut traveler = Traveler {
                        id: resident.id,
                        kind: 0,
                        route_lengths: self.route_lengths(&route),
                        route,
                        edge_index: 0,
                        edge_progress: 0.0,
                        speed: 1.5,
                        destination: resident.home_node,
                        activity: TravelerActivity::WalkingHome,
                        resident: resident.id,
                        name: resident.name.clone(),
                        origin_building: None,
                        destination_building: Some(resident.home_building),
                        wait_ticks: 0,
                        origin_access: None,
                        destination_access: None,
                        access_phase: 1,
                        access_progress: 0.0,
                    };
                    traveler.origin_access =
                        self.platform_exit_path(&traveler.route, true, queue_slot, traveler.id);
                    traveler.destination_access = self.building_access_path(
                        resident.home_building,
                        &traveler.route,
                        false,
                        traveler.id,
                        0,
                    );
                    if traveler.origin_access.is_some() {
                        traveler.access_phase = 0;
                    }
                    self.state.people.travelers.push(traveler);
                    let current = &mut self.state.people.residents[resident_index];
                    current.transfer = Transfer::WalkingHome;
                    current.activity = ResidentActivity::WalkingHome;
                    current.train_id = None;
                    current.queue_slot = None;
                    push_history(
                        &mut current.history,
                        format!(
                            "Arrived by train and started walking home at tick {}",
                            self.state.tick
                        ),
                    );
                    self.record(
                        EventKind::TrainPassengerAlighted,
                        train_id,
                        format!(
                            "{} alights at the station and continues on foot",
                            resident.name
                        ),
                    );
                }
            } else if !at_station && resident.transfer == Transfer::RidingOutbound {
                unloaded.insert(resident_id);
                let current = &mut self.state.people.residents[resident_index];
                current.transfer = Transfer::WorkingExternal;
                current.activity = ResidentActivity::WorkingExternal;
                current.at_work = true;
                current.train_id = None;
                current.work_node = Some(gateway);
                push_history(
                    &mut current.history,
                    format!(
                        "Arrived at the regional gateway for work at tick {}",
                        self.state.tick
                    ),
                );
                self.record(
                    EventKind::RailWorkStarted,
                    resident_id,
                    format!("{} arrives at an external workplace by rail", resident.name),
                );
                self.record(
                    EventKind::TrainPassengerAlighted,
                    train_id,
                    format!("{} alights at the regional gateway", resident.name),
                );
            }
        }
        if !unloaded.is_empty() {
            if let Some(train) = self
                .state
                .transit
                .trains
                .iter_mut()
                .find(|train| train.id == train_id)
            {
                train
                    .passengers
                    .retain(|resident_id| !unloaded.contains(resident_id));
            }
        }
    }

    pub(crate) fn board_waiting_train(&mut self, train_id: u32, station: u32, gateway: u32) {
        let Some((dwell_ticks, passenger_count, direction, distance)) = self
            .state
            .transit
            .trains
            .iter()
            .find(|train| train.id == train_id)
            .map(|train| {
                (
                    train.dwell_ticks,
                    train.passengers.len(),
                    train.direction,
                    train.distance,
                )
            })
        else {
            return;
        };
        let at_station = direction >= 0 && distance <= TRAIN_HALF_LENGTH + 0.01;
        let rail_length = self
            .state
            .network
            .roads
            .iter()
            .find(|road| road.kind == RoadKind::Rail && road.stage == Stage::Open && road.connected)
            .map(|road| road.length)
            .unwrap_or(0.0);
        let at_gateway = direction < 0 && distance >= rail_length - TRAIN_HALF_LENGTH - 0.01;
        let total_dwell = if at_station {
            TRAIN_STATION_DWELL_TICKS
        } else {
            TRAIN_GATEWAY_DWELL_TICKS
        };
        if (!at_station && !at_gateway)
            || dwell_ticks < TRAIN_DOOR_PHASE_TICKS
            || dwell_ticks > total_dwell - TRAIN_DOOR_PHASE_TICKS
            || passenger_count >= RAIL_TRAIN_CAPACITY
        {
            return;
        }
        if self.state.tick % 3 != 0 {
            return;
        }
        let (queue_state, riding_state) = if direction >= 0 && distance <= TRAIN_HALF_LENGTH + 0.01
        {
            (Transfer::WaitingStation, Transfer::RidingOutbound)
        } else if direction < 0 && distance >= rail_length - TRAIN_HALF_LENGTH - 0.01 {
            (Transfer::WaitingGateway, Transfer::RidingInbound)
        } else {
            return;
        };
        let next = self
            .state
            .people
            .residents
            .iter()
            .filter(|resident| resident.rail_commuter && resident.transfer == queue_state)
            .map(|resident| resident.id)
            .min();
        let Some(resident_id) = next else { return };
        let Some(index) = self
            .state
            .people
            .residents
            .iter()
            .position(|resident| resident.id == resident_id)
        else {
            return;
        };
        let resident_name = self.state.people.residents[index].name.clone();
        let current = &mut self.state.people.residents[index];
        current.transfer = riding_state;
        current.activity = ResidentActivity::RidingTrain;
        current.train_id = Some(train_id);
        current.queue_slot = None;
        push_history(
            &mut current.history,
            format!(
                "Boarded the neighborhood shuttle at tick {}",
                self.state.tick
            ),
        );
        if let Some(train) = self
            .state
            .transit
            .trains
            .iter_mut()
            .find(|train| train.id == train_id)
        {
            if train.passengers.len() < RAIL_TRAIN_CAPACITY {
                train.passengers.push(resident_id);
            }
        }
        let board_station = queue_state == Transfer::WaitingStation;
        self.record(
            EventKind::TrainPassengerBoarded,
            train_id,
            format!(
                "{resident_name} boards the shuttle at node {}",
                if board_station { station } else { gateway }
            ),
        );
    }

    /// Compact authoritative train-door presentation sampled at the same tick
    /// as movement_buffer. Tuples are [train_id, terminal_id or -1, fraction].
    pub fn train_doors(&self) -> Vec<f32> {
        let rail_length = self.rail_length_for_doors();
        let mut out = Vec::with_capacity(
            self.state.transit.trains.len().min(MAX_PRESENTED_TRAINS) * wire::TRAIN_DOOR_STRIDE,
        );
        for train in self.state.transit.trains.iter().take(MAX_PRESENTED_TRAINS) {
            let (terminal, fraction) = train_door_state(train, rail_length);
            out.extend_from_slice(&[
                train.id as f32,
                terminal.map(|id| id as f32).unwrap_or(-1.0),
                fraction,
            ]);
        }
        out
    }

    pub(crate) fn rail_length_for_doors(&self) -> f32 {
        self.state
            .network
            .roads
            .iter()
            .find(|road| road.kind == RoadKind::Rail && road.stage == Stage::Open && road.connected)
            .map(|road| road.length)
            .unwrap_or(0.0)
    }
}
