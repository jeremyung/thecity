use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const BUS_CAPACITY: usize = 24;

pub(crate) const BUS_QUEUE_CAPACITY: usize = 48;

pub(crate) const MAX_BUSES: usize = 2;

pub(crate) const MAX_BUS_STOPS: usize = 5;

pub(crate) const MAX_BUS_ROUTE_NODES: usize = 128;

pub(crate) const BUS_HEADWAY_TICKS: u64 = 240;

pub(crate) const BUS_DWELL_TICKS: u16 = 80;

pub(crate) const BUS_RETRY_TICKS: u64 = 2_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct BusLine {
    pub(crate) id: u32,
    pub(crate) stops: Vec<u32>,
    pub(crate) route: Vec<u32>,
    #[serde(default)]
    pub(crate) active: bool,
    #[serde(default)]
    pub(crate) graph_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Bus {
    pub(crate) id: u32,
    pub(crate) line_id: u32,
    pub(crate) progress: f32,
    pub(crate) direction: i8,
    #[serde(default)]
    pub(crate) dwell_ticks: u16,
    #[serde(default)]
    pub(crate) passengers: Vec<u32>,
    #[serde(default)]
    pub(crate) next_departure_tick: u64,
    #[serde(default)]
    pub(crate) last_stop: Option<u32>,
}

impl Engine {
    pub(crate) fn ensure_bus_service(&mut self) {
        if self.state.tick < self.state.transit.next_bus_evaluation_tick {
            return;
        }
        self.state.transit.next_bus_evaluation_tick =
            self.state.tick.saturating_add(BUS_RETRY_TICKS);
        if self
            .state
            .transit
            .bus_line
            .as_ref()
            .is_some_and(|line| line.active)
        {
            if self.state.transit.buses.len() < MAX_BUSES {
                let waiting = self.bus_waiting_count();
                if waiting >= 12 {
                    if let Some((line_id, terminal)) = self
                        .state
                        .transit
                        .bus_line
                        .as_ref()
                        .map(|line| (line.id, line.stops.last().copied()))
                        .and_then(|(line_id, terminal)| {
                            terminal.map(|terminal| (line_id, terminal))
                        })
                    {
                        if let Some(progress) = self.bus_stop_distance(terminal) {
                            let bus_id = self.id();
                            self.state.transit.buses.push(Bus {
                                id: bus_id,
                                line_id,
                                progress,
                                direction: -1,
                                dwell_ticks: BUS_DWELL_TICKS,
                                passengers: Vec::new(),
                                next_departure_tick: self
                                    .state
                                    .tick
                                    .saturating_add(BUS_HEADWAY_TICKS),
                                last_stop: Some(terminal),
                            });
                            self.record(
                                EventKind::BusServiceExpanded,
                                bus_id,
                                "A second neighborhood bus joins the line after demand grows"
                                    .into(),
                            );
                        }
                    }
                }
            }
            return;
        }
        let open_roads = self
            .state
            .network
            .roads
            .iter()
            .filter(|road| {
                road.kind != RoadKind::Rail && road.stage == Stage::Open && road.connected
            })
            .count();
        let developed: Vec<u32> = self
            .state
            .construction
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .filter_map(|building| {
                self.state
                    .land
                    .parcels
                    .iter()
                    .find(|parcel| parcel.id == building.parcel_id)
                    .map(|parcel| parcel.access_node)
            })
            .filter(|node| self.node_has_open_road(*node))
            .collect();
        if !self.state.transit.station_online || open_roads < 6 || developed.len() < 3 {
            return;
        }
        let mut candidates: Vec<u32> = developed;
        candidates.sort_unstable();
        candidates.dedup();
        if candidates.len() > 32 {
            let stride = (candidates.len() + 31) / 32;
            candidates = candidates.into_iter().step_by(stride).take(32).collect();
        }
        if candidates.len() < 3 {
            return;
        }
        let Some(station) = self
            .state
            .network
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Station && self.node_has_open_road(node.id))
            .map(|node| node.id)
        else {
            return;
        };
        // Build the route around real recurring resident trips. For each distinct
        // home/work access node, cache only short reachable walks to candidate
        // stops; the same nearest-stop rule is used by schedule_bus_trip below.
        let mut stop_candidates = candidates;
        stop_candidates.push(station);
        stop_candidates.sort_unstable();
        stop_candidates.dedup();
        let demand: Vec<(u32, u32)> = self
            .state
            .people
            .residents
            .iter()
            .filter(|resident| !resident.rail_commuter)
            .filter_map(|resident| resident.work_node.map(|work| (resident.home_node, work)))
            .filter(|(home, work)| home != work)
            .collect();
        if demand.is_empty() {
            return;
        }
        let endpoints: BTreeSet<u32> = demand
            .iter()
            .flat_map(|(home, work)| [*home, *work])
            .collect();
        let mut endpoint_stop_distance = BTreeMap::<u32, BTreeMap<u32, f32>>::new();
        for endpoint in endpoints {
            let mut reachable = BTreeMap::new();
            for stop in stop_candidates.iter().copied() {
                let (Some(a), Some(b)) = (self.node(endpoint), self.node(stop)) else {
                    continue;
                };
                if distance(a, b) > 75.0 {
                    continue;
                }
                let path = self.route(endpoint, stop, false);
                if path.is_empty() {
                    continue;
                }
                let length: f32 = self.route_lengths(&path).iter().sum();
                if length <= 75.0 {
                    reachable.insert(stop, length);
                }
            }
            endpoint_stop_distance.insert(endpoint, reachable);
        }

        let route_score = |stops: &[u32], route_length: f32| -> (usize, usize, f32) {
            let nearest = |endpoint: u32| -> Option<(usize, f32)> {
                stops
                    .iter()
                    .enumerate()
                    .filter_map(|(index, stop)| {
                        endpoint_stop_distance
                            .get(&endpoint)
                            .and_then(|distances| distances.get(stop))
                            .map(|distance| (index, *distance))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            };
            let mut covered_pairs = 0usize;
            let mut endpoint_count = 0usize;
            let mut walk_cost = 0.0f32;
            for (home, work) in &demand {
                let home_stop = nearest(*home);
                let work_stop = nearest(*work);
                if let Some((_, distance)) = home_stop {
                    endpoint_count += 1;
                    walk_cost += distance;
                }
                if let Some((_, distance)) = work_stop {
                    endpoint_count += 1;
                    walk_cost += distance;
                }
                if let (Some((home_index, _)), Some((work_index, _))) = (home_stop, work_stop) {
                    if home_index != work_index {
                        covered_pairs += 1;
                    }
                }
            }
            (
                covered_pairs,
                endpoint_count,
                walk_cost + route_length * 0.35,
            )
        };

        let mut stops = vec![station];
        let mut route = vec![station];
        let mut remaining = stop_candidates;
        remaining.retain(|candidate| *candidate != station);
        while stops.len() < MAX_BUS_STOPS {
            let last = *stops.last().unwrap_or(&station);
            let visited: BTreeSet<u32> = route.iter().copied().collect();
            let current_length: f32 = self.route_lengths(&route).iter().sum();
            let mut best: Option<((usize, usize, f32), u32, Vec<u32>, Vec<u32>)> = None;
            for candidate in remaining.iter().copied() {
                if candidate == last || visited.contains(&candidate) {
                    continue;
                }
                // Keep the service corridor simple. A shortest connector that doubles back
                // through an earlier stop makes node-based stop distances ambiguous and can
                // leave residents queued on a direction the buses never serve.
                let connector = self.bus_route_avoiding(last, candidate, &visited);
                if connector.len() < 2 || connector.len() > MAX_BUS_ROUTE_NODES {
                    continue;
                }
                let Some(a) = self.node(last) else { continue };
                let Some(b) = self.node(candidate) else {
                    continue;
                };
                let separation = distance(a, b);
                if separation < 42.0 {
                    continue;
                }
                if route
                    .len()
                    .saturating_add(connector.len().saturating_sub(1))
                    > MAX_BUS_ROUTE_NODES
                {
                    continue;
                }
                let mut proposed_route = route.clone();
                proposed_route.extend(connector.iter().copied().skip(1));
                let mut proposed_stops = stops.clone();
                proposed_stops.push(candidate);
                let added_length: f32 = self.route_lengths(&connector).iter().sum();
                let score = route_score(&proposed_stops, current_length + added_length);
                let is_better = best.as_ref().is_none_or(|(best_score, best_stop, _, _)| {
                    score.0 > best_score.0
                        || (score.0 == best_score.0 && score.1 > best_score.1)
                        || (score.0 == best_score.0
                            && score.1 == best_score.1
                            && (score.2 < best_score.2
                                || (score.2 == best_score.2 && candidate < *best_stop)))
                });
                if is_better {
                    best = Some((score, candidate, connector, proposed_route));
                }
            }
            let Some((_score, stop, _connector, proposed_route)) = best else {
                break;
            };
            route = proposed_route;
            stops.push(stop);
            remaining.retain(|candidate| *candidate != stop);
        }
        if stops.len() < 3
            || route.len() > MAX_BUS_ROUTE_NODES
            || route_score(&stops, self.route_lengths(&route).iter().sum()).0 == 0
        {
            return;
        }
        let line_id = self.id();
        self.state.transit.bus_line = Some(BusLine {
            id: line_id,
            stops: stops.clone(),
            route,
            active: true,
            graph_revision: open_roads as u64,
        });
        let bus_id = self.id();
        self.state.transit.buses.push(Bus {
            id: bus_id,
            line_id,
            progress: 0.0,
            direction: 1,
            dwell_ticks: BUS_DWELL_TICKS,
            passengers: Vec::new(),
            next_departure_tick: self.state.tick.saturating_add(BUS_HEADWAY_TICKS),
            last_stop: stops.first().copied(),
        });
        self.record(
            EventKind::BusServiceStarted,
            bus_id,
            format!(
                "Neighborhood bus service opens with {} connected stops",
                stops.len()
            ),
        );
    }

    pub(crate) fn bus_stop_distance(&self, stop: u32) -> Option<f32> {
        let line = self.state.transit.bus_line.as_ref()?;
        let index = line.route.iter().position(|node| *node == stop)?;
        Some(self.route_lengths(&line.route[..=index]).iter().sum())
    }

    pub(crate) fn bus_waiting_count(&self) -> usize {
        self.state
            .people
            .residents
            .iter()
            .filter(|resident| {
                matches!(
                    resident.transfer,
                    Transfer::WalkingToBus | Transfer::WaitingBus
                )
            })
            .count()
    }

    pub(crate) fn bus_platform_waiting_count(&self) -> usize {
        self.state
            .people
            .residents
            .iter()
            .filter(|resident| resident.transfer == Transfer::WaitingBus)
            .count()
    }

    pub(crate) fn bus_queue_position(
        &self,
        node_id: u32,
        direction: i8,
        slot: u16,
    ) -> Option<WorldPoint> {
        self.bus_queue_position_with_edge(node_id, direction, slot)
            .map(|(point, _)| point)
    }

    pub(crate) fn bus_queue_position_with_edge(
        &self,
        node_id: u32,
        direction: i8,
        slot: u16,
    ) -> Option<(WorldPoint, [u32; 2])> {
        let node = self.node(node_id)?;
        let line = self.state.transit.bus_line.as_ref()?;
        let route_index = line
            .route
            .iter()
            .position(|candidate| *candidate == node_id)?;
        let other_id = if direction >= 0 {
            line.route.get(route_index + 1).or_else(|| {
                route_index
                    .checked_sub(1)
                    .and_then(|index| line.route.get(index))
            })
        } else {
            route_index
                .checked_sub(1)
                .and_then(|index| line.route.get(index))
                .or_else(|| line.route.get(route_index + 1))
        };
        let other_id = *other_id?;
        let other = self.node(other_id)?;
        let length = distance(node, other).max(1.0);
        let along = 3.0 + slot as f32 * 1.25;
        Some((
            WorldPoint {
                x: node.x + (other.x - node.x) / length * along
                    - (other.z - node.z) / length * BUS_STOP_KERB,
                z: node.z
                    + (other.z - node.z) / length * along
                    + (other.x - node.x) / length * BUS_STOP_KERB,
            },
            [node_id, other_id],
        ))
    }

    pub(crate) fn bus_stop_views(&self, line: &BusLine) -> Vec<BusStopView> {
        let mut views = Vec::with_capacity(line.stops.len() * 2);
        for stop in &line.stops {
            let Some(index) = line.route.iter().position(|node| node == stop) else {
                continue;
            };
            let Some(node) = self.node(*stop) else {
                continue;
            };
            for direction in [1_i8, -1_i8] {
                let adjacent = if direction >= 0 {
                    line.route
                        .get(index + 1)
                        .or_else(|| index.checked_sub(1).and_then(|i| line.route.get(i)))
                } else {
                    index
                        .checked_sub(1)
                        .and_then(|i| line.route.get(i))
                        .or_else(|| line.route.get(index + 1))
                };
                let Some(next_id) = adjacent else { continue };
                let Some(next) = self.node(*next_id) else {
                    continue;
                };
                let dx = next.x - node.x;
                let dz = next.z - node.z;
                let length = dx.hypot(dz).max(1.0);
                let nx = -dz / length;
                let nz = dx / length;
                let curb = BUS_STOP_KERB;
                views.push(BusStopView {
                    node_id: *stop,
                    x: node.x + nx * curb,
                    z: node.z + nz * curb,
                    heading: dz.atan2(dx),
                    direction,
                });
            }
        }
        views
    }

    pub(crate) fn allocate_bus_queue_slot(&self) -> Option<u16> {
        let used: BTreeSet<u16> = self
            .state
            .people
            .residents
            .iter()
            .filter(|resident| {
                matches!(
                    resident.transfer,
                    Transfer::WalkingToBus | Transfer::WaitingBus
                )
            })
            .filter_map(|resident| resident.queue_slot)
            .collect();
        (0..BUS_QUEUE_CAPACITY as u16).find(|slot| !used.contains(slot))
    }

    pub(crate) fn schedule_bus_trip(
        &mut self,
        index: usize,
        origin: u32,
        goal: u32,
        activity: TravelerActivity,
        origin_building: u32,
        destination_building: Option<u32>,
    ) -> bool {
        let Some(line) = self
            .state
            .transit
            .bus_line
            .clone()
            .filter(|line| line.active)
        else {
            return false;
        };
        if self.rail_service_quiet()
            || self.bus_waiting_count() >= BUS_QUEUE_CAPACITY
            || line.stops.len() < 3
        {
            return false;
        }
        let nearest_stop = |engine: &Self, node: u32| -> Option<(usize, u32)> {
            line.stops
                .iter()
                .enumerate()
                .filter_map(|(stop_index, stop)| {
                    let path = engine.route(node, *stop, false);
                    if path.is_empty() {
                        return None;
                    }
                    let length: f32 = engine.route_lengths(&path).iter().sum();
                    (length <= 75.0).then_some((stop_index, *stop, length))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
                .map(|(stop_index, stop, _)| (stop_index, stop))
        };
        let (Some((origin_index, origin_stop)), Some((destination_index, destination_stop))) =
            (nearest_stop(self, origin), nearest_stop(self, goal))
        else {
            return false;
        };
        if origin_index == destination_index {
            return false;
        }
        let direction = if origin_index < destination_index {
            1
        } else {
            -1
        };
        let Some(queue_slot) = self.allocate_bus_queue_slot() else {
            return false;
        };
        let route = self.route(origin, origin_stop, false);
        if route.is_empty() || route.len() > MAX_ROUTE_NODES {
            return false;
        }
        let id = self.state.people.residents[index].id;
        let origin_queue = self.bus_queue_position(origin_stop, direction, queue_slot);
        let route_lengths = self.route_lengths(&route);
        let mut traveler = Traveler {
            id,
            kind: 0,
            route,
            route_lengths,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 1.5,
            destination: origin_stop,
            activity: TravelerActivity::WalkingToBus,
            resident: id,
            name: self.state.people.residents[index].name.clone(),
            origin_building: Some(origin_building),
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        };
        traveler.origin_access =
            self.building_access_path(origin_building, &traveler.route, true, id, 0);
        if let Some(queue_position) = origin_queue {
            let end = self.route_endpoint_position(&traveler.route, false, id, 0);
            if let Some(start) = end {
                traveler.destination_access = Some(AccessPath {
                    points: vec![start, queue_position],
                    surface_y: Vec::new(),
                });
            }
        }
        if traveler.origin_access.is_some() {
            traveler.access_phase = 0;
        }
        self.state.people.travelers.push(traveler);
        if let Some(resident) = self.state.people.residents.get_mut(index) {
            resident.transfer = Transfer::WalkingToBus;
            resident.activity = ResidentActivity::WalkingToBus;
            resident.bus_id = None;
            resident.bus_origin_stop = Some(origin_stop);
            resident.bus_destination_stop = Some(destination_stop);
            resident.bus_stop_node = Some(origin_stop);
            resident.queue_slot = Some(queue_slot);
            if activity == TravelerActivity::Return {
                resident.last_return_day = clock_at(self.state.tick).0;
                resident.return_tick = resident.return_tick.saturating_add(TICKS_PER_DAY);
                resident.activity = ResidentActivity::ReturningHome;
            } else {
                resident.last_departure_day = clock_at(self.state.tick).0;
                resident.departure_tick = resident.departure_tick.saturating_add(TICKS_PER_DAY);
                resident.activity = ResidentActivity::Commuting;
            }
            push_history(
                &mut resident.history,
                format!(
                    "Started a bus trip toward stop {destination_stop} ({direction}) at tick {}",
                    self.state.tick
                ),
            );
        }
        self.record(
            EventKind::BusTripStarted,
            id,
            format!(
                "A resident walks from building {origin_building} toward bus stop {origin_stop}"
            ),
        );
        let _ = destination_building;
        true
    }

    pub(crate) fn bus_route_avoiding(
        &self,
        start: u32,
        goal: u32,
        blocked: &BTreeSet<u32>,
    ) -> Vec<u32> {
        // Full-size neighborhood buses stay on collectors and wider ground
        // streets. Residents can still walk through lanes to reach these stops.
        self.route_where(start, goal, blocked, |road| {
            matches!(
                road.kind,
                RoadKind::Local | RoadKind::Arterial | RoadKind::Bridge
            )
        })
    }

    pub(crate) fn advance_buses(&mut self) {
        let Some(line) = self
            .state
            .transit
            .bus_line
            .clone()
            .filter(|line| line.active)
        else {
            return;
        };
        if line.route.len() < 2 || line.route.len() > MAX_BUS_ROUTE_NODES {
            return;
        }
        let lengths = self.route_lengths(&line.route);
        let route_length: f32 = lengths.iter().sum();
        if route_length <= 0.0 {
            return;
        }
        let stop_distances: Vec<(u32, f32)> = line
            .stops
            .iter()
            .filter_map(|stop| {
                self.bus_stop_distance(*stop)
                    .map(|progress| (*stop, progress))
            })
            .collect();
        let quiet = self.rail_service_quiet();
        for bus_index in 0..self.state.transit.buses.len() {
            if self.state.transit.buses[bus_index].dwell_ticks > 0 {
                let at_terminal = self.state.transit.buses[bus_index].progress <= 0.01
                    || self.state.transit.buses[bus_index].progress >= route_length - 0.01;
                let stop = self.state.transit.buses[bus_index].last_stop.or_else(|| {
                    self.stop_at_progress(
                        &stop_distances,
                        self.state.transit.buses[bus_index].progress,
                    )
                });
                if let Some(stop) = stop {
                    self.handle_bus_stop(bus_index, stop, &line, quiet);
                }
                if (quiet && self.state.transit.buses[bus_index].passengers.is_empty())
                    || (at_terminal
                        && self.state.tick
                            < self.state.transit.buses[bus_index].next_departure_tick)
                {
                    self.state.transit.buses[bus_index].dwell_ticks = 1;
                } else {
                    self.state.transit.buses[bus_index].dwell_ticks = self.state.transit.buses
                        [bus_index]
                        .dwell_ticks
                        .saturating_sub(1);
                }
                continue;
            }
            if quiet && self.state.transit.buses[bus_index].passengers.is_empty() {
                // Empty buses wait at the next stop during the quiet service window.
            }
            let direction = self.state.transit.buses[bus_index].direction;
            let progress = self.state.transit.buses[bus_index].progress;
            let next_stop = Self::next_bus_stop(
                &stop_distances,
                direction,
                progress,
                route_length,
                line.stops.first().copied().unwrap_or(0),
                line.stops.last().copied().unwrap_or(0),
            );
            let speed = 7.5_f32 / TICKS_PER_SECOND as f32;
            let desired = (progress + speed * direction as f32).clamp(0.0, route_length);
            let mut target = desired;
            if let Some((segment, from, to, edge_progress, edge_length)) =
                self.bus_segment_at_progress(&line.route, &lengths, progress, direction)
            {
                if edge_progress >= edge_length - 3.6 {
                    let signalized = Self::signalized_state(&self.state, to);
                    let outgoing_index = if direction >= 0 {
                        segment
                            .checked_add(2)
                            .filter(|index| *index < line.route.len())
                    } else {
                        segment.checked_sub(1)
                    };
                    let at_junction = outgoing_index.is_some();
                    if signalized && at_junction {
                        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                            continue;
                        };
                        let (Some(c), Some(d)) = (
                            self.node(to),
                            self.node(line.route[outgoing_index.unwrap()]),
                        ) else {
                            continue;
                        };
                        let dx = d.x - c.x;
                        let dz = d.z - c.z;
                        let horizontal = dx.abs() >= dz.abs();
                        let phase = (self.state.tick + to as u64 * 37 + from as u64 * 11) % 240;
                        let aligned = if horizontal {
                            phase < 120
                        } else {
                            phase >= 120
                        };
                        let _heading = (b.z - a.z).atan2(b.x - a.x);
                        if !aligned {
                            let stop_progress = if direction >= 0 {
                                progress - edge_progress + edge_length - 3.6
                            } else {
                                progress - (edge_length - edge_progress) + 3.6
                            };
                            target = if direction >= 0 {
                                target.min(progress.max(stop_progress))
                            } else {
                                target.max(progress.min(stop_progress))
                            };
                        }
                    }
                }
                for car in self.state.people.travelers.iter().filter(|traveler| {
                    traveler.kind == 1
                        && traveler.access_phase == 1
                        && traveler.edge_index < traveler.route_lengths.len()
                        && traveler.route[traveler.edge_index] == from
                        && traveler.route[traveler.edge_index + 1] == to
                }) {
                    let ahead = car.edge_progress - edge_progress;
                    if ahead > 0.0 && ahead < 14.0 {
                        let travel_room = (ahead - 11.0).max(0.0);
                        target = if direction >= 0 {
                            target.min(progress + travel_room)
                        } else {
                            target.max(progress - travel_room)
                        };
                    }
                }
            }
            // Maintain a deterministic gap when buses travel in the same direction.
            for other_index in 0..self.state.transit.buses.len() {
                if other_index == bus_index
                    || self.state.transit.buses[other_index].direction != direction
                    || self.state.transit.buses[other_index].dwell_ticks > 0
                {
                    continue;
                }
                let other = self.state.transit.buses[other_index].progress;
                let gap = if direction > 0 {
                    other - progress
                } else {
                    progress - other
                };
                if gap > 0.0 && gap < 24.0 {
                    target = if direction > 0 {
                        target.min((other - 24.0).max(progress))
                    } else {
                        target.max((other + 24.0).min(progress))
                    };
                }
            }
            let stop_crossed = if direction > 0 {
                next_stop.1 <= target + 0.0001 && next_stop.1 > progress
            } else {
                next_stop.1 >= target - 0.0001 && next_stop.1 < progress
            };
            if stop_crossed {
                target = next_stop.1;
                let bus = &mut self.state.transit.buses[bus_index];
                bus.progress = target;
                bus.last_stop = Some(next_stop.0);
                bus.dwell_ticks = BUS_DWELL_TICKS;
                if target <= 0.01 || target >= route_length - 0.01 {
                    bus.direction = -bus.direction;
                    bus.next_departure_tick = self.state.tick.saturating_add(BUS_HEADWAY_TICKS);
                }
                self.record(
                    EventKind::BusStop,
                    self.state.transit.buses[bus_index].id,
                    format!(
                        "Bus {} arrives at stop {}",
                        self.state.transit.buses[bus_index].id, next_stop.0
                    ),
                );
                self.handle_bus_stop(bus_index, next_stop.0, &line, quiet);
            } else {
                self.state.transit.buses[bus_index].progress = target;
            }
        }
    }

    pub(crate) fn stop_at_progress(&self, stops: &[(u32, f32)], progress: f32) -> Option<u32> {
        stops
            .iter()
            .find(|(_, distance)| (*distance - progress).abs() < 0.75)
            .map(|(stop, _)| *stop)
    }

    pub(crate) fn next_bus_stop(
        stops: &[(u32, f32)],
        direction: i8,
        progress: f32,
        route_length: f32,
        first_stop: u32,
        last_stop: u32,
    ) -> (u32, f32) {
        // Exclude only the stop the bus is exactly dwelling at. Any positive
        // tolerance can skip a stop when the remaining distance falls below
        // that tolerance before the bus reaches it.
        if direction >= 0 {
            stops
                .iter()
                .filter(|(_, distance)| *distance > progress)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .copied()
                .unwrap_or((last_stop, route_length))
        } else {
            stops
                .iter()
                .filter(|(_, distance)| *distance < progress)
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .copied()
                .unwrap_or((first_stop, 0.0))
        }
    }

    pub(crate) fn bus_segment_at_progress(
        &self,
        route: &[u32],
        lengths: &[f32],
        progress: f32,
        direction: i8,
    ) -> Option<(usize, u32, u32, f32, f32)> {
        let mut offset = 0.0;
        for (index, length) in lengths.iter().copied().enumerate() {
            let at_or_before_edge_end = if direction >= 0 {
                progress < offset + length - 0.001
            } else {
                progress <= offset + length + 0.001
            };
            if at_or_before_edge_end || index + 1 == lengths.len() {
                if direction >= 0 {
                    return Some((
                        index,
                        route[index],
                        route[index + 1],
                        progress - offset,
                        length,
                    ));
                }
                return Some((
                    index,
                    route[index + 1],
                    route[index],
                    offset + length - progress,
                    length,
                ));
            }
            offset += length;
        }
        None
    }

    pub(crate) fn handle_bus_stop(
        &mut self,
        bus_index: usize,
        stop: u32,
        line: &BusLine,
        quiet: bool,
    ) {
        let bus_id = self.state.transit.buses[bus_index].id;
        let mut alight: Vec<u32> = self.state.transit.buses[bus_index]
            .passengers
            .iter()
            .copied()
            .filter(|resident_id| {
                self.state
                    .people
                    .residents
                    .iter()
                    .find(|resident| resident.id == *resident_id)
                    .is_some_and(|resident| resident.bus_destination_stop == Some(stop))
            })
            .collect();
        alight.sort_unstable();
        for resident_id in alight {
            self.alight_bus_passenger(bus_index, resident_id, stop);
        }
        if quiet || self.state.transit.buses[bus_index].passengers.len() >= BUS_CAPACITY {
            return;
        }
        let direction = self.state.transit.buses[bus_index].direction;
        let stop_index = line.stops.iter().position(|candidate| *candidate == stop);
        let mut waiting: Vec<u32> = self
            .state
            .people
            .residents
            .iter()
            .filter(|resident| {
                resident.transfer == Transfer::WaitingBus && resident.bus_origin_stop == Some(stop)
            })
            .filter(|resident| {
                let Some(origin) = stop_index else {
                    return false;
                };
                let Some(destination) = resident
                    .bus_destination_stop
                    .and_then(|node| line.stops.iter().position(|candidate| *candidate == node))
                else {
                    return false;
                };
                if direction >= 0 {
                    destination > origin
                } else {
                    destination < origin
                }
            })
            .map(|resident| resident.id)
            .collect();
        waiting.sort_unstable();
        for resident_id in waiting {
            if self.state.transit.buses[bus_index].passengers.len() >= BUS_CAPACITY {
                break;
            }
            self.state.transit.buses[bus_index]
                .passengers
                .push(resident_id);
            if let Some(resident) = self
                .state
                .people
                .residents
                .iter_mut()
                .find(|resident| resident.id == resident_id)
            {
                resident.transfer = Transfer::RidingBus;
                resident.activity = ResidentActivity::AboardBus;
                resident.bus_id = Some(bus_id);
                resident.bus_stop_node = Some(stop);
                resident.queue_slot = None;
                push_history(
                    &mut resident.history,
                    format!(
                        "Boarded bus {bus_id} at stop {stop} at tick {}",
                        self.state.tick
                    ),
                );
            }
            self.record(
                EventKind::BusPassengerBoarded,
                resident_id,
                format!("A resident boards bus {bus_id} at stop {stop}"),
            );
        }
    }

    pub(crate) fn alight_bus_passenger(&mut self, bus_index: usize, resident_id: u32, stop: u32) {
        let Some(resident_index) = self
            .state
            .people
            .residents
            .iter()
            .position(|resident| resident.id == resident_id)
        else {
            return;
        };
        let resident = self.state.people.residents[resident_index].clone();
        let target_node = if resident.at_work {
            resident.home_node
        } else {
            resident.work_node.unwrap_or(resident.home_node)
        };
        let destination_building = if resident.at_work {
            resident.home_building
        } else {
            resident.work_building.unwrap_or(resident.home_building)
        };
        let wait_position = self
            .state
            .transit
            .buses
            .get(bus_index)
            .and_then(|bus| {
                self.bus_queue_position(stop, bus.direction, resident.queue_slot.unwrap_or(0))
            })
            .or_else(|| {
                self.node(stop).map(|node| WorldPoint {
                    x: node.x,
                    z: node.z,
                })
            });
        let Some(wait_position) = wait_position else {
            return;
        };
        let activity = if resident.at_work {
            TravelerActivity::Return
        } else {
            TravelerActivity::Commute
        };
        let route_attempted = self.state.people.travelers.len() < MAX_TRAVELERS;
        let route = if route_attempted {
            self.route(stop, target_node, false)
        } else {
            Vec::new()
        };
        if route.is_empty() || route.len() > MAX_ROUTE_NODES {
            let queued = self.queue_pending_alight(
                resident_id,
                stop,
                target_node,
                destination_building,
                activity,
                Transfer::WalkingFromBus,
                wait_position,
                route_attempted,
            );
            if !queued {
                let reason = format!("Still aboard at stop {stop}: pedestrian route queue is full");
                let current = &mut self.state.people.residents[resident_index];
                if current.history.last() != Some(&reason) {
                    push_history(&mut current.history, reason);
                }
                return;
            }
            let bus_id = self.state.transit.buses[bus_index].id;
            self.state.transit.buses[bus_index]
                .passengers
                .retain(|id| *id != resident_id);
            push_history(
                &mut self.state.people.residents[resident_index].history,
                format!(
                    "Waited at stop {stop} for a legal walking route at tick {}",
                    self.state.tick
                ),
            );
            self.record(
                EventKind::BusPassengerWaitingForRoute,
                resident_id,
                format!("A resident alights from bus {bus_id} into the bounded route queue at stop {stop}"),
            );
            return;
        }
        let mut traveler = Traveler {
            id: resident.id,
            kind: 0,
            route_lengths: self.route_lengths(&route),
            route,
            edge_index: 0,
            edge_progress: 0.0,
            speed: 1.5,
            destination: target_node,
            activity,
            resident: resident.id,
            name: resident.name.clone(),
            origin_building: None,
            destination_building: Some(destination_building),
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        };
        if let Some(slot) = resident.queue_slot.or(Some(0)) {
            if let Some(queue) =
                self.bus_queue_position(stop, self.state.transit.buses[bus_index].direction, slot)
            {
                if let Some(endpoint) =
                    self.route_endpoint_position(&traveler.route, true, resident.id, 0)
                {
                    traveler.origin_access = Some(AccessPath {
                        points: vec![queue, endpoint],
                        surface_y: Vec::new(),
                    });
                }
            }
        }
        traveler.destination_access =
            self.building_access_path(destination_building, &traveler.route, false, resident.id, 0);
        if traveler.origin_access.is_some() {
            traveler.access_phase = 0;
        }
        let bus_id = self.state.transit.buses[bus_index].id;
        self.state.transit.buses[bus_index]
            .passengers
            .retain(|id| *id != resident_id);
        if let Some(resident) = self.state.people.residents.get_mut(resident_index) {
            resident.transfer = Transfer::WalkingFromBus;
            resident.activity = if resident.at_work {
                ResidentActivity::WalkingHomeFromBus
            } else {
                ResidentActivity::WalkingToWorkFromBus
            };
            resident.bus_id = None;
            resident.bus_stop_node = Some(stop);
            resident.queue_slot = None;
            push_history(
                &mut resident.history,
                format!(
                    "Alighted from bus {bus_id} at stop {stop} at tick {}",
                    self.state.tick
                ),
            );
        }
        self.state.people.travelers.push(traveler);
        self.record(
            EventKind::BusPassengerAlighted,
            resident_id,
            format!("A resident leaves bus {bus_id} at stop {stop}"),
        );
    }

    pub(crate) fn bus_trip_direction(&self, resident: &Resident) -> Option<i8> {
        let line = self.state.transit.bus_line.as_ref()?;
        let origin = line
            .stops
            .iter()
            .position(|stop| Some(*stop) == resident.bus_origin_stop)?;
        let destination = line
            .stops
            .iter()
            .position(|stop| Some(*stop) == resident.bus_destination_stop)?;
        Some(if destination > origin { 1 } else { -1 })
    }

    pub(crate) fn bus_next_stop(&self, bus: &Bus, line: &BusLine) -> Option<u32> {
        let first = *line.stops.first()?;
        let last = *line.stops.last()?;
        let distances: Vec<(u32, f32)> = line
            .stops
            .iter()
            .filter_map(|stop| {
                self.bus_stop_distance(*stop)
                    .map(|distance| (*stop, distance))
            })
            .collect();
        Some(
            Self::next_bus_stop(
                &distances,
                bus.direction,
                bus.progress,
                self.route_lengths(&line.route).iter().sum(),
                first,
                last,
            )
            .0,
        )
    }

    pub(crate) fn bus_waiting_at(&self, stop: Option<u32>) -> usize {
        let Some(stop) = stop else { return 0 };
        self.state
            .people
            .residents
            .iter()
            .filter(|resident| {
                resident.transfer == Transfer::WaitingBus && resident.bus_stop_node == Some(stop)
            })
            .count()
    }

    pub(crate) fn bus_pose(&self, line: &BusLine, bus: &Bus) -> Option<(f32, f32, f32)> {
        self.bus_pose_with_edge(line, bus).map(|(pose, _)| pose)
    }

    // Return provenance from the exact direction-sensitive edge selection used
    // for the pose, including the tolerance at shared junctions.
    pub(crate) fn bus_pose_with_edge(
        &self,
        line: &BusLine,
        bus: &Bus,
    ) -> Option<((f32, f32, f32), [u32; 2])> {
        let lengths = self.route_lengths(&line.route);
        let mut offset = 0.0;
        for (index, length) in lengths.iter().copied().enumerate() {
            let at_or_before_edge_end = if bus.direction >= 0 {
                bus.progress < offset + length - 0.001
            } else {
                bus.progress <= offset + length + 0.001
            };
            if at_or_before_edge_end || index + 1 == lengths.len() {
                let a = self.node(line.route[index])?;
                let b = self.node(line.route[index + 1])?;
                let f = ((bus.progress - offset) / length.max(1.0)).clamp(0.0, 1.0);
                let mut heading = (b.z - a.z).atan2(b.x - a.x);
                if bus.direction < 0 {
                    heading += std::f32::consts::PI;
                }
                let nx = -heading.sin();
                let nz = heading.cos();
                return Some((
                    (
                        a.x + (b.x - a.x) * f + nx * KERB_LANE,
                        a.z + (b.z - a.z) * f + nz * KERB_LANE,
                        heading,
                    ),
                    [a.id, b.id],
                ));
            }
            offset += length;
        }
        None
    }
}
