use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_TRAVELERS: usize = 2_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Traveler {
    pub(crate) id: u32,
    pub(crate) kind: u8,
    pub(crate) route: Vec<u32>,
    pub(crate) route_lengths: Vec<f32>,
    pub(crate) edge_index: usize,
    pub(crate) edge_progress: f32,
    pub(crate) speed: f32,
    pub(crate) destination: u32,
    pub(crate) activity: TravelerActivity,
    pub(crate) resident: u32,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) origin_building: Option<u32>,
    #[serde(default)]
    pub(crate) destination_building: Option<u32>,
    #[serde(default)]
    pub(crate) wait_ticks: u16,
    #[serde(default)]
    pub(crate) origin_access: Option<AccessPath>,
    #[serde(default)]
    pub(crate) destination_access: Option<AccessPath>,
    #[serde(default = "default_access_phase")]
    pub(crate) access_phase: u8,
    #[serde(default)]
    pub(crate) access_progress: f32,
}

pub(crate) fn default_access_phase() -> u8 {
    1
}

impl Engine {
    /// Cars queue on driveways the way they do on roads. Access paths share
    /// geometry at both ends: departures leave a shared door and merge where
    /// neighbouring driveways meet the road; arrivals do the reverse. Each car
    /// therefore queues against both ends of its path, ordered by distance from
    /// that end. Returns, per traveler, the furthest access progress it may reach
    /// this tick (one queue gap behind the car ahead; ties go to the lower id),
    /// and the road mouths whose arrival driveway is still occupied.
    pub(crate) fn driveway_queues(&self) -> (Vec<f32>, BTreeSet<(i32, i32)>) {
        let station = self.station_node();
        let travelers = &self.state.people.travelers;
        let mut limit = vec![f32::INFINITY; travelers.len()];
        // (phase, queues at the path start?, end point) -> (index, rank, path length).
        // Rank grows toward the back of the queue.
        let mut lanes: BTreeMap<(u8, bool, (i32, i32)), Vec<(usize, f32, f32)>> = BTreeMap::new();
        for (index, t) in travelers.iter().enumerate() {
            let path = match (t.kind, t.access_phase) {
                (1, 0) => t.origin_access.as_ref(),
                (1, 2) => t.destination_access.as_ref(),
                _ => None,
            };
            let Some(path) = path else { continue };
            let (Some(start), Some(end)) = (path.points.first(), path.points.last()) else {
                continue;
            };
            let length = path.length();
            // Leaving the start: further along is ahead. Approaching the end: closer is ahead.
            lanes
                .entry((t.access_phase, true, driveway_mouth(*start)))
                .or_default()
                .push((index, -t.access_progress, length));
            lanes
                .entry((t.access_phase, false, driveway_mouth(*end)))
                .or_default()
                .push((index, length - t.access_progress, length));
        }
        let mut entry_busy = BTreeSet::new();
        for ((phase, at_start, point), list) in &mut lanes {
            list.sort_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then(travelers[a.0].id.cmp(&travelers[b.0].id))
            });
            for pair in list.windows(2) {
                let ((lead, lead_rank, _), (follower, _, length)) = (pair[0], pair[1]);
                let gap = vehicle_gap(
                    Self::vehicle_body(&travelers[lead], station),
                    Self::vehicle_body(&travelers[follower], station),
                );
                let allowed = if *at_start {
                    -lead_rank - gap
                } else {
                    length - (lead_rank + gap)
                };
                limit[follower] = limit[follower].min(allowed);
            }
            // Arrival paths start at the road: the mouth is busy until the last car clears it.
            if *phase == 2 && *at_start {
                let last = &travelers[list.last().unwrap().0];
                if last.access_progress
                    < vehicle_length(Self::vehicle_body(last, station)) + VEHICLE_QUEUE_MARGIN
                {
                    entry_busy.insert(*point);
                }
            }
        }
        (limit, entry_busy)
    }

    pub(crate) fn advance_travelers(&mut self) {
        if self.state.people.travelers.is_empty() {
            return;
        }
        let count = self.state.people.travelers.len();
        self.planned_progress.resize(count, 0.0);
        self.wait_scratch.resize(count, 0);
        let mut skip_road = vec![false; count];
        let mut arrivals = Vec::new();
        let (driveway_limit, mut driveway_entry_busy) = self.driveway_queues();
        for index in 0..count {
            let traveler = &mut self.state.people.travelers[index];
            match traveler.access_phase {
                0 => {
                    skip_road[index] = true;
                    if let Some(path) = &traveler.origin_access {
                        let next =
                            traveler.access_progress + traveler.speed / TICKS_PER_SECOND as f32;
                        traveler.access_progress =
                            next.min(driveway_limit[index].max(traveler.access_progress));
                        if traveler.access_progress >= path.length() {
                            traveler.access_phase = 1;
                            traveler.access_progress = 0.0;
                        }
                    } else {
                        traveler.access_phase = 1;
                        traveler.access_progress = 0.0;
                    }
                }
                2 => {
                    skip_road[index] = true;
                    if let Some(path) = &traveler.destination_access {
                        let next =
                            traveler.access_progress + traveler.speed / TICKS_PER_SECOND as f32;
                        traveler.access_progress =
                            next.min(driveway_limit[index].max(traveler.access_progress));
                        if traveler.access_progress >= path.length() {
                            arrivals.push(traveler.clone());
                        }
                    } else {
                        arrivals.push(traveler.clone());
                    }
                }
                _ => {
                    traveler.access_phase = 1;
                    // A car held at the kerb by a busy driveway retries here, before
                    // any signal can stop it short of the mouth it already reached.
                    let at_kerb = traveler.kind == 1
                        && traveler.edge_index + 1 == traveler.route_lengths.len()
                        && traveler
                            .route_lengths
                            .last()
                            .is_some_and(|&len| traveler.edge_progress >= len.max(1.0));
                    if traveler.edge_index >= traveler.route_lengths.len() || at_kerb {
                        // A car waits at the kerb while another is still at the driveway mouth.
                        let mouth = traveler
                            .destination_access
                            .as_ref()
                            .and_then(|path| path.points.first())
                            .map(|p| driveway_mouth(*p));
                        let held = traveler.kind == 1
                            && mouth.is_some_and(|m| driveway_entry_busy.contains(&m));
                        if held {
                            // Still a road traveler: drawn, and queued ahead of its followers.
                            // (Saves from before kerb holding kept the car past its route.)
                            hold_at_kerb(traveler);
                            skip_road[index] = traveler.edge_index >= traveler.route_lengths.len();
                        } else {
                            skip_road[index] = true;
                            if traveler.destination_access.is_some() {
                                traveler.access_phase = 2;
                                traveler.access_progress = 0.0;
                                if traveler.kind == 1 {
                                    driveway_entry_busy.extend(mouth);
                                }
                            } else {
                                arrivals.push(traveler.clone());
                            }
                        }
                    }
                }
            }
            self.planned_progress[index] = traveler.edge_progress;
            self.wait_scratch[index] = traveler.wait_ticks;
        }
        for index in 0..count {
            let t = &self.state.people.travelers[index];
            if skip_road[index] {
                continue;
            }
            if t.edge_index >= t.route_lengths.len() {
                self.planned_progress[index] = t.edge_progress;
                self.wait_scratch[index] = t.wait_ticks;
                continue;
            }
            let to = t.route[t.edge_index + 1];
            let len = t.route_lengths[t.edge_index].max(1.0);
            let desired = t.edge_progress + t.speed / TICKS_PER_SECOND as f32;
            let mut target = desired;
            let mut red = false;
            if desired >= len - 3.6 {
                red = Self::junction_red_state(&self.state, t, to);
                if red {
                    target = target.min(len - 3.6).max(t.edge_progress);
                }
            }
            if red {
                self.wait_scratch[index] = t.wait_ticks.saturating_add(1);
            } else {
                self.wait_scratch[index] = 0;
            }
            self.planned_progress[index] = target;
        }
        for index in 0..count {
            self.state.people.travelers[index].wait_ticks = self.wait_scratch[index];
        }
        // Deterministic car-following queues by directed lane. Followers slow and stop
        // behind the leader before reaching the intersection approach. Runs every
        // tick: the red-light stop above also does, and a sparser queue let
        // followers creep onto the leader at the stop line between passes.
        {
            let station = self.station_node();
            self.lane_order.clear();
            self.lane_order.extend((0..count).filter(|index| {
                !skip_road[*index]
                    && self.state.people.travelers[*index].access_phase == 1
                    && self.state.people.travelers[*index].edge_index
                        < self.state.people.travelers[*index].route_lengths.len()
            }));
            self.lane_order.sort_by(|&a, &b| {
                let ta = &self.state.people.travelers[a];
                let tb = &self.state.people.travelers[b];
                let ka = ta
                    .route
                    .get(ta.edge_index..=ta.edge_index + 1)
                    .map(|r| lane_key(ta, r[0], r[1]))
                    .unwrap_or((u32::MAX, u32::MAX, u8::MAX, u8::MAX));
                let kb = tb
                    .route
                    .get(tb.edge_index..=tb.edge_index + 1)
                    .map(|r| lane_key(tb, r[0], r[1]))
                    .unwrap_or((u32::MAX, u32::MAX, u8::MAX, u8::MAX));
                ka.cmp(&kb)
                    .then_with(|| tb.edge_progress.total_cmp(&ta.edge_progress))
                    .then_with(|| ta.id.cmp(&tb.id))
            });
            let mut start = 0;
            while start < self.lane_order.len() {
                let first = self.lane_order[start];
                let Some(key) = self.state.people.travelers[first]
                    .route
                    .get(
                        self.state.people.travelers[first].edge_index
                            ..=self.state.people.travelers[first].edge_index + 1,
                    )
                    .map(|r| lane_key(&self.state.people.travelers[first], r[0], r[1]))
                else {
                    start += 1;
                    continue;
                };
                let mut end = start + 1;
                while end < self.lane_order.len() {
                    let next = self.lane_order[end];
                    let next_key = self.state.people.travelers[next]
                        .route
                        .get(
                            self.state.people.travelers[next].edge_index
                                ..=self.state.people.travelers[next].edge_index + 1,
                        )
                        .map(|r| lane_key(&self.state.people.travelers[next], r[0], r[1]));
                    if next_key != Some(key) {
                        break;
                    }
                    end += 1;
                }
                let len = self.state.people.travelers[first].route_lengths
                    [self.state.people.travelers[first].edge_index]
                    .max(1.0);
                for i in start + 1..end {
                    let lead = self.lane_order[i - 1];
                    let follower = self.lane_order[i];
                    let (lead_t, follower_t) = (
                        &self.state.people.travelers[lead],
                        &self.state.people.travelers[follower],
                    );
                    let gap = if lead_t.kind == 1 && follower_t.kind == 1 {
                        vehicle_gap(
                            Self::vehicle_body(lead_t, station),
                            Self::vehicle_body(follower_t, station),
                        )
                    } else if lead_t.kind == 1 || follower_t.kind == 1 {
                        VEHICLE_MIN_GAP
                    } else {
                        1.6
                    };
                    let safe = if self.planned_progress[lead] >= len {
                        len - gap
                    } else {
                        self.planned_progress[lead] - gap
                    };
                    self.planned_progress[follower] = self.planned_progress[follower]
                        .min(safe.max(self.state.people.travelers[follower].edge_progress));
                }
                start = end;
            }
        }
        // Buses are persistent vehicles outside the pedestrian/car traveler list.
        // Include them in the same directed left-lane following relationship so a
        // car cannot advance through a stopped or slow bus at a shared road edge.
        if let Some(line) = &self.state.transit.bus_line {
            let lengths = self.route_lengths(&line.route);
            let bus_lanes: Vec<(u32, u32, f32)> = self
                .state
                .transit
                .buses
                .iter()
                .filter(|bus| bus.line_id == line.id)
                .filter_map(|bus| {
                    self.bus_segment_at_progress(&line.route, &lengths, bus.progress, bus.direction)
                        .map(|(_segment, from, to, local, _length)| (from, to, local))
                })
                .collect();
            for index in 0..count {
                let traveler = &self.state.people.travelers[index];
                if traveler.kind != 1
                    || traveler.access_phase != 1
                    || traveler.edge_index >= traveler.route_lengths.len()
                {
                    continue;
                }
                let from = traveler.route[traveler.edge_index];
                let to = traveler.route[traveler.edge_index + 1];
                for (bus_from, bus_to, bus_local) in &bus_lanes {
                    if from != *bus_from || to != *bus_to {
                        continue;
                    }
                    let ahead = *bus_local - traveler.edge_progress;
                    if ahead > 0.0 && ahead < 12.0 {
                        let allowed = traveler.edge_progress + (ahead - 10.0).max(0.0);
                        self.planned_progress[index] = self.planned_progress[index].min(allowed);
                    }
                }
            }
        }
        // Merge gate: a car enters a road lane only when the last car to enter
        // it is a queue gap down the road. Cars are admitted in index order and
        // each admission becomes the lane's new tail, so two cars arriving from
        // different approaches on one tick cannot both enter.
        let station = self.station_node();
        let mut lane_tail: BTreeMap<(u32, u32), (f32, u8)> = BTreeMap::new();
        for (index, t) in self.state.people.travelers.iter().enumerate() {
            if t.kind != 1 || skip_road[index] {
                continue;
            }
            if let Some(r) = t.route.get(t.edge_index..=t.edge_index + 1) {
                let entry = lane_tail
                    .entry((r[0], r[1]))
                    .or_insert((f32::INFINITY, VEHICLE_COMPACT));
                if t.edge_progress < entry.0 {
                    *entry = (t.edge_progress, Self::vehicle_body(t, station));
                }
            }
        }
        let (travelers, planned) = (&mut self.state.people.travelers, &self.planned_progress);
        for (index, t) in travelers.iter_mut().enumerate() {
            if skip_road[index] {
                continue;
            }
            if t.edge_index >= t.route_lengths.len() {
                arrivals.push(t.clone());
                continue;
            }
            let body = Self::vehicle_body(t, station);
            let mut target = planned[index];
            if t.kind == 1 {
                // Stop a queue gap behind the last car into the next lane, not at the node.
                let edge_len = t.route_lengths[t.edge_index].max(1.0);
                let next_lane = t
                    .route
                    .get(t.edge_index + 1..=t.edge_index + 2)
                    .map(|r| (r[0], r[1]));
                if let Some(tail) = next_lane.and_then(|lane| lane_tail.get(&lane)) {
                    let gap = vehicle_gap(tail.1, body);
                    if tail.0 < gap {
                        target = target.min(edge_len - (gap - tail.0)).max(t.edge_progress);
                    }
                }
            }
            let mut remaining = (target - t.edge_progress).max(0.0);
            while remaining > 0.0 && t.edge_index < t.route_lengths.len() {
                let edge_len = t.route_lengths[t.edge_index].max(1.0);
                let available = edge_len - t.edge_progress;
                if remaining + 0.0001 < available {
                    t.edge_progress += remaining;
                    break;
                }
                let next_lane = t
                    .route
                    .get(t.edge_index + 1..=t.edge_index + 2)
                    .map(|r| (r[0], r[1]));
                if t.kind == 1 {
                    if let Some(tail) = next_lane.and_then(|lane| lane_tail.get(&lane)) {
                        if tail.0 < vehicle_gap(tail.1, body) {
                            // Wait at the junction for the lane ahead to clear.
                            t.edge_progress = edge_len;
                            break;
                        }
                    }
                }
                remaining = (remaining - available).max(0.0);
                t.edge_index += 1;
                t.edge_progress = 0.0;
                if t.kind == 1 {
                    if let Some(lane) = next_lane {
                        lane_tail.insert(
                            lane,
                            (
                                remaining
                                    .min(t.route_lengths.get(t.edge_index).copied().unwrap_or(0.0)),
                                body,
                            ),
                        );
                    }
                }
            }
            if t.edge_index >= t.route_lengths.len() {
                let mouth = t
                    .destination_access
                    .as_ref()
                    .and_then(|path| path.points.first())
                    .map(|p| driveway_mouth(*p));
                let held = t.kind == 1 && mouth.is_some_and(|m| driveway_entry_busy.contains(&m));
                // A held car waits at the kerb end of its last lane and retries
                // admission each tick as it tries to advance past the end.
                if held {
                    hold_at_kerb(t);
                } else {
                    if t.destination_access.is_some() {
                        t.access_phase = 2;
                        t.access_progress = 0.0;
                        if t.kind == 1 {
                            driveway_entry_busy.extend(mouth);
                        }
                    } else {
                        arrivals.push(t.clone());
                    }
                }
            }
        }
        let arrived_ids: BTreeSet<u32> = arrivals.iter().map(|t| t.id).collect();
        self.state
            .people
            .travelers
            .retain(|t| !arrived_ids.contains(&t.id));
        let mut return_vehicles = Vec::new();
        for arrived in arrivals {
            if self.handle_neighborhood_errand_arrival(&arrived) {
                continue;
            }
            if arrived.resident != 0 {
                if let Some(resident) = self
                    .state
                    .people
                    .residents
                    .iter_mut()
                    .find(|r| r.id == arrived.resident)
                {
                    match arrived.activity {
                        TravelerActivity::WalkingToBus => {
                            resident.transfer = Transfer::WaitingBus;
                            resident.activity = ResidentActivity::WaitingForBus;
                            resident.bus_stop_node = resident.bus_origin_stop;
                        }
                        TravelerActivity::Commute
                            if resident.transfer == Transfer::WalkingFromBus =>
                        {
                            resident.at_work = true;
                            resident.activity = ResidentActivity::AtDestination;
                            resident.transfer = Transfer::None;
                            resident.bus_id = None;
                            resident.bus_origin_stop = None;
                            resident.bus_destination_stop = None;
                            resident.bus_stop_node = None;
                            resident.queue_slot = None;
                        }
                        TravelerActivity::Return
                            if resident.transfer == Transfer::WalkingFromBus =>
                        {
                            resident.at_work = false;
                            resident.activity = ResidentActivity::AtHome;
                            resident.transfer = Transfer::None;
                            resident.bus_id = None;
                            resident.bus_origin_stop = None;
                            resident.bus_destination_stop = None;
                            resident.bus_stop_node = None;
                            resident.queue_slot = None;
                        }
                        TravelerActivity::WalkingToStation => {
                            resident.transfer = Transfer::WaitingStation;
                            resident.activity = ResidentActivity::WaitingForTrain;
                            resident.at_work = false;
                        }
                        TravelerActivity::WalkingHome => {
                            resident.transfer = Transfer::Home;
                            resident.activity = ResidentActivity::AtHome;
                            resident.at_work = false;
                            resident.train_id = None;
                            resident.queue_slot = None;
                        }
                        _ => {
                            resident.at_work = arrived.activity != TravelerActivity::Return;
                            resident.activity = if resident.at_work {
                                ResidentActivity::AtDestination
                            } else {
                                ResidentActivity::AtHome
                            };
                        }
                    }
                    push_history(
                        &mut resident.history,
                        format!(
                            "Arrived after {} at tick {}",
                            arrived.activity, self.state.tick
                        ),
                    );
                }
                self.record(
                    EventKind::ResidentTripArrived,
                    arrived.id,
                    format!("{} arrives after {}", arrived.name, arrived.activity),
                );
            } else if arrived.activity == TravelerActivity::Delivery {
                let mut returning = arrived.clone();
                returning.route.reverse();
                returning.route_lengths.reverse();
                returning.edge_index = 0;
                returning.edge_progress = 0.0;
                returning.destination = *returning.route.last().unwrap_or(&returning.destination);
                returning.origin_building = arrived.destination_building;
                returning.destination_building = None;
                returning.activity = TravelerActivity::DeliveryReturn;
                returning.wait_ticks = 0;
                returning.origin_access = returning.origin_building.and_then(|building| {
                    self.building_access_path(
                        building,
                        &returning.route,
                        true,
                        returning.id,
                        returning.kind,
                    )
                });
                returning.destination_access = None;
                returning.access_phase = if returning.origin_access.is_some() {
                    0
                } else {
                    1
                };
                returning.access_progress = 0.0;
                return_vehicles.push(returning);
                self.record(
                    EventKind::DeliveryArrived,
                    arrived.id,
                    format!(
                        "A supply van reaches {}",
                        arrived.destination_building.unwrap_or(0)
                    ),
                );
            } else if arrived.activity == TravelerActivity::DeliveryReturn {
                self.record(
                    EventKind::DeliveryReturned,
                    arrived.id,
                    "A supply van returns to the regional gateway".into(),
                );
            }
        }
        self.state.people.travelers.extend(return_vehicles);
    }
}
