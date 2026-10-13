//! Bounded, persistent traffic on the two independent master-plan expressway
//! corridors. Geometry is cached from the immutable plan; realization remains
//! an engine-state decision.

use serde::{Deserialize, Serialize};

use crate::masterplan::{MasterPlan, P};
use crate::Engine;

pub(crate) const MAX_EXPRESSWAY_TRIPS: usize = 48;
const TICKS_PER_SECOND: u64 = 20;
const DAY_DEPARTURE_INTERVAL_TICKS: u64 = 120;
const NIGHT_DEPARTURE_INTERVAL_TICKS: u64 = 240;
const SPEED_METRES_PER_SECOND: f32 = 15.0;
const LANE_OFFSET_METRES: f32 = 3.25;
const LANE_SPACING_METRES: f32 = 3.5;
const CURVE_SAMPLES: usize = 64;
const EXACT_F32_ID_LIMIT: u32 = 1 << 24;

#[derive(Clone, Debug)]
pub(crate) struct Network {
    pub(crate) routes: Vec<Route>,
}

#[derive(Clone, Debug)]
pub(crate) struct Route {
    pub(crate) segments: Vec<Segment>,
    pub(crate) length: f32,
    pub(crate) from_node: usize,
    pub(crate) to_node: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Segment {
    pub(crate) road_index: usize,
    pub(crate) from_node: usize,
    pub(crate) to_node: usize,
    pub(crate) controls: [P; 2],
    pub(crate) a: P,
    pub(crate) b: P,
    pub(crate) deck_height: f32,
    pub(crate) start_distance: f32,
    pub(crate) length: f32,
    /// Arc-distance samples at uniformly spaced cubic parameters.
    pub(crate) cumulative: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TrafficState {
    #[serde(default)]
    pub(crate) next_departure_tick: u64,
    #[serde(default)]
    pub(crate) departure_sequence: u32,
    #[serde(default)]
    pub(crate) trips: Vec<Trip>,
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            next_departure_tick: DAY_DEPARTURE_INTERVAL_TICKS,
            departure_sequence: 0,
            trips: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Trip {
    pub(crate) id: u32,
    pub(crate) route_index: usize,
    pub(crate) direction: i8,
    pub(crate) lane: u8,
    /// Distance travelled from the selected regional entry boundary.
    pub(crate) progress: f32,
}

impl Network {
    pub(crate) fn from_plan(plan: &MasterPlan) -> Self {
        let mut adjacency = vec![Vec::<usize>::new(); plan.nodes.len()];
        for &road_index in &plan.expressway {
            if let Some(road) = plan.roads.get(road_index) {
                if road.a < adjacency.len() && road.b < adjacency.len() {
                    adjacency[road.a].push(road_index);
                    adjacency[road.b].push(road_index);
                }
            }
        }
        for edges in &mut adjacency {
            edges.sort_unstable();
        }

        let mut seen = vec![false; plan.roads.len()];
        let mut routes = Vec::new();
        for &seed in &plan.expressway {
            if seen.get(seed).copied().unwrap_or(true) {
                continue;
            }
            let mut component = Vec::new();
            let mut stack = vec![seed];
            seen[seed] = true;
            while let Some(index) = stack.pop() {
                component.push(index);
                let road = plan.roads[index];
                for node in [road.a, road.b] {
                    for &next in &adjacency[node] {
                        if !seen[next] {
                            seen[next] = true;
                            stack.push(next);
                        }
                    }
                }
            }
            component.sort_unstable();
            let boundaries: Vec<usize> = (0..plan.nodes.len())
                .filter(|&node| {
                    adjacency[node]
                        .iter()
                        .any(|r| component.binary_search(r).is_ok())
                })
                .filter(|&node| {
                    plan.nodes[node].p.x.abs() >= 390.0 || plan.nodes[node].p.z.abs() >= 390.0
                })
                .collect();
            if boundaries.len() != 2 {
                continue;
            }
            // The corridor must be a chain. This excludes merges and prevents
            // accidentally joining the separate elevated layers.
            if component.iter().any(|&road| {
                let item = plan.roads[road];
                adjacency[item.a]
                    .iter()
                    .filter(|r| component.binary_search(r).is_ok())
                    .count()
                    > 2
                    || adjacency[item.b]
                        .iter()
                        .filter(|r| component.binary_search(r).is_ok())
                        .count()
                        > 2
            }) {
                continue;
            }
            let mut ordered = Vec::with_capacity(component.len());
            let mut node = boundaries[0];
            let mut previous = None;
            loop {
                let next_road = adjacency[node]
                    .iter()
                    .copied()
                    .find(|r| component.binary_search(r).is_ok() && Some(*r) != previous);
                let Some(road_index) = next_road else {
                    break;
                };
                ordered.push((road_index, node));
                let road = plan.roads[road_index];
                let next_node = if road.a == node { road.b } else { road.a };
                previous = Some(road_index);
                node = next_node;
            }
            if ordered.len() != component.len() || node != boundaries[1] {
                continue;
            }
            let mut distance = 0.0;
            let mut segments = Vec::with_capacity(ordered.len());
            for (road_index, from_node) in ordered {
                let road = plan.roads[road_index];
                let to_node = if road.a == from_node { road.b } else { road.a };
                let a = plan.nodes[from_node].p;
                let b = plan.nodes[to_node].p;
                let Some(controls) = road.curve else { continue };
                let reversed = from_node != road.a;
                let oriented_controls = if reversed {
                    [controls[1], controls[0]]
                } else {
                    controls
                };
                let mut cumulative = Vec::with_capacity(CURVE_SAMPLES + 1);
                cumulative.push(0.0);
                let mut prior = a;
                for sample in 1..=CURVE_SAMPLES {
                    let t = sample as f32 / CURVE_SAMPLES as f32;
                    let point = cubic(a, oriented_controls[0], oriented_controls[1], b, t);
                    let delta_x = point.x - prior.x;
                    let delta_z = point.z - prior.z;
                    let chord = (delta_x * delta_x + delta_z * delta_z).sqrt();
                    cumulative.push(cumulative.last().copied().unwrap_or(0.0) + chord);
                    prior = point;
                }
                let length = *cumulative.last().unwrap_or(&0.0);
                segments.push(Segment {
                    road_index,
                    from_node,
                    to_node,
                    controls: oriented_controls,
                    a,
                    b,
                    deck_height: road.deck_height,
                    start_distance: distance,
                    length,
                    cumulative,
                });
                distance += length;
            }
            if segments.len() == component.len() && distance.is_finite() && distance > 1.0 {
                let deck_height = segments[0].deck_height;
                if segments
                    .iter()
                    .any(|segment| (segment.deck_height - deck_height).abs() > 0.001)
                {
                    continue;
                }
                routes.push(Route {
                    segments,
                    length: distance,
                    from_node: boundaries[0],
                    to_node: boundaries[1],
                });
            }
        }
        routes.sort_by_key(|route| {
            route
                .segments
                .first()
                .map(|s| s.road_index)
                .unwrap_or(usize::MAX)
        });
        Self { routes }
    }
}

fn cubic(a: P, c: P, d: P, b: P, t: f32) -> P {
    let u = 1.0 - t;
    let aa = u * u * u;
    let bb = 3.0 * u * u * t;
    let cc = 3.0 * u * t * t;
    let dd = t * t * t;
    P {
        x: aa * a.x + bb * c.x + cc * d.x + dd * b.x,
        z: aa * a.z + bb * c.z + cc * d.z + dd * b.z,
    }
}
fn cubic_tangent(a: P, c: P, d: P, b: P, t: f32) -> P {
    let u = 1.0 - t;
    P {
        x: 3.0 * u * u * (c.x - a.x) + 6.0 * u * t * (d.x - c.x) + 3.0 * t * t * (b.x - d.x),
        z: 3.0 * u * u * (c.z - a.z) + 6.0 * u * t * (d.z - c.z) + 3.0 * t * t * (b.z - d.z),
    }
}

impl Segment {
    fn sample(&self, local: f32) -> (P, P, f32) {
        let local = local.clamp(0.0, self.length);
        let upper = self
            .cumulative
            .partition_point(|&d| d < local)
            .min(CURVE_SAMPLES);
        let lower = upper.saturating_sub(1);
        let d0 = self.cumulative[lower];
        let d1 = self.cumulative[upper];
        let part = if d1 > d0 {
            (local - d0) / (d1 - d0)
        } else {
            0.0
        };
        let t = (lower as f32 + part) / CURVE_SAMPLES as f32;
        let point = cubic(self.a, self.controls[0], self.controls[1], self.b, t);
        let tangent = cubic_tangent(self.a, self.controls[0], self.controls[1], self.b, t);
        (point, tangent, t)
    }
}

impl Route {
    fn sample(&self, progress: f32, direction: i8, lane: u8) -> Option<(P, f32, f32)> {
        let position = if direction > 0 {
            progress
        } else {
            self.length - progress
        };
        let segment = self
            .segments
            .iter()
            .find(|s| position <= s.start_distance + s.length)
            .or_else(|| self.segments.last())?;
        let local = (position - segment.start_distance).clamp(0.0, segment.length);
        let (mut point, tangent, _) = segment.sample(local);
        let magnitude = (tangent.x * tangent.x + tangent.z * tangent.z)
            .sqrt()
            .max(1.0e-6);
        let route_tx = tangent.x / magnitude;
        let route_tz = tangent.z / magnitude;
        let tx = route_tx * direction as f32;
        let tz = route_tz * direction as f32;
        let side = if direction > 0 { -1.0 } else { 1.0 };
        let offset = side * (LANE_OFFSET_METRES + lane as f32 * LANE_SPACING_METRES);
        // Right normal is (tz,-tx); the forward carriageway uses its negative.
        point.x += route_tz * offset;
        point.z -= route_tx * offset;
        Some((point, tz.atan2(tx), segment.deck_height + 0.15))
    }
}

impl Engine {
    pub(crate) fn append_expressway_movement_rows(&self, out: &mut Vec<crate::movement_support::Row>, include_support: bool) {
        let Some(runtime) = self.plan.as_ref() else { return };
        for trip in &self.state.expressway_traffic.trips {
            let Some(route) = runtime.expressway_network.routes.get(trip.route_index) else { continue };
            let Some((point, heading, height)) = route.sample(trip.progress, trip.direction, trip.lane) else { continue };
            let road_ids = if include_support { self.state.masterplan.as_ref().map(|progress| route.segments.iter()
                .filter_map(|segment| progress.road_ids.get(segment.road_index).copied())
                .collect::<Vec<_>>()).unwrap_or_default() } else { Vec::new() };
            let resolved = include_support && road_ids.len() == route.segments.len() && road_ids.iter().all(|id| *id != 0)
                && route_is_open(self, route);
            out.push(crate::movement_support::Row { actor_id: trip.id, x: point.x, z: point.z, heading, kind: 1, surface_y: height,
                resolved, road_ids: if resolved { road_ids } else { Vec::new() }, platform_node_ids: Vec::new() });
        }
    }

    pub(crate) fn advance_expressway_traffic(&mut self) {
        let Some(runtime) = self.plan.as_ref().cloned() else {
            return;
        };
        if self.state.masterplan.is_none() {
            return;
        }
        let network = &runtime.expressway_network;
        let route_active: Vec<bool> = network
            .routes
            .iter()
            .map(|route| route_is_open(self, route))
            .collect();
        let dt = 1.0 / TICKS_PER_SECOND as f32;
        {
            let state = &mut self.state.expressway_traffic;
            for trip in &mut state.trips {
                if route_active.get(trip.route_index).copied().unwrap_or(false) {
                    trip.progress += SPEED_METRES_PER_SECOND * dt;
                }
            }
            state.trips.retain(|trip| {
                network
                    .routes
                    .get(trip.route_index)
                    .is_some_and(|route| {
                        route_active.get(trip.route_index).copied().unwrap_or(false)
                            && trip.progress < route.length
                    })
            });
        }
        if self.state.tick < self.state.expressway_traffic.next_departure_tick {
            return;
        }
        let (_, minute) = crate::clock_at(self.state.tick);
        let interval = if (360.0..1_320.0).contains(&minute) {
            DAY_DEPARTURE_INTERVAL_TICKS
        } else {
            NIGHT_DEPARTURE_INTERVAL_TICKS
        };
        self.state.expressway_traffic.next_departure_tick = self.state.tick.saturating_add(interval);
        if self.state.expressway_traffic.trips.len() >= MAX_EXPRESSWAY_TRIPS {
            return;
        }
        if network.routes.is_empty() {
            return;
        }
        if self.state.next_id >= EXACT_F32_ID_LIMIT {
            return;
        }
        let sequence = self.state.expressway_traffic.departure_sequence;
        let route_count = network.routes.len();
        let route_index = (0..route_count)
            .map(|offset| (sequence as usize + offset) % route_count)
            .find(|&index| route_active.get(index).copied().unwrap_or(false));
        let Some(route_index) = route_index else { return; };
        let direction = if (sequence / route_count as u32) % 2 == 0 {
            1
        } else {
            -1
        };
        let lane = ((sequence / (route_count as u32 * 2)) % 2) as u8;
        if !route_active.get(route_index).copied().unwrap_or(false) {
            return;
        }
        let id = self.id();
        if id >= EXACT_F32_ID_LIMIT {
            return;
        }
        let state = &mut self.state.expressway_traffic;
        state.departure_sequence = state.departure_sequence.saturating_add(1);
        state.trips.push(Trip {
            id,
            route_index,
            direction,
            lane,
            progress: 0.0,
        });
    }

    pub(crate) fn append_expressway_movements(&self, out: &mut Vec<f32>) {
        let Some(runtime) = self.plan.as_ref() else {
            return;
        };
        for trip in &self.state.expressway_traffic.trips {
            if let Some((point, heading, height)) = runtime
                .expressway_network
                .routes
                .get(trip.route_index)
                .and_then(|route| route.sample(trip.progress, trip.direction, trip.lane))
            {
                out.extend_from_slice(&[trip.id as f32, point.x, point.z, heading, 1.0, height]);
            }
        }
    }

    pub(crate) fn validate_expressway_traffic(&self) -> Result<(), String> {
        let traffic = &self.state.expressway_traffic;
        if traffic.trips.len() > MAX_EXPRESSWAY_TRIPS {
            return Err("too many expressway trips".into());
        }
        let network = self.plan.as_ref().map(|p| &p.expressway_network);
        let mut ids = std::collections::BTreeSet::new();
        for trip in &traffic.trips {
            if trip.id == 0
                || trip.id >= EXACT_F32_ID_LIMIT
                || trip.id >= self.state.next_id
                || !ids.insert(trip.id)
            {
                return Err("invalid or duplicate expressway trip ID".into());
            }
            if trip.direction != -1 && trip.direction != 1 {
                return Err("invalid expressway trip direction".into());
            }
            if trip.lane > 1 {
                return Err("invalid expressway trip lane".into());
            }
            if !trip.progress.is_finite() {
                return Err("invalid expressway trip progress".into());
            }
            let route = network
                .and_then(|n| n.routes.get(trip.route_index))
                .ok_or("expressway trip route is invalid")?;
            if trip.progress < 0.0 || trip.progress >= route.length || !route_is_open(self, route)
            {
                return Err("expressway trip is outside an open connected route".into());
            }
            if self.state.nodes.iter().any(|actor| actor.id == trip.id)
                || self.state.roads.iter().any(|actor| actor.id == trip.id)
                || self.state.parcels.iter().any(|actor| actor.id == trip.id)
                || self.state.buildings.iter().any(|actor| actor.id == trip.id)
                || self.state.projects.iter().any(|actor| actor.id == trip.id)
                || self.state.residents.iter().any(|actor| actor.id == trip.id)
                || self.state.travelers.iter().any(|actor| actor.id == trip.id)
                || self.state.trains.iter().any(|actor| actor.id == trip.id)
                || self.state.buses.iter().any(|actor| actor.id == trip.id)
            {
                return Err("expressway trip ID collides with an active actor".into());
            }
        }
        if traffic.next_departure_tick == 0
            || traffic.next_departure_tick
                > self.state.tick.saturating_add(NIGHT_DEPARTURE_INTERVAL_TICKS)
        {
            return Err("invalid expressway departure deadline".into());
        }
        Ok(())
    }

    pub(crate) fn inspect_expressway_trip(&self, id: u32) -> Option<serde_json::Value> {
        let trip = self
            .state
            .expressway_traffic
            .trips
            .iter()
            .find(|trip| trip.id == id)?;
        let route = self
            .plan
            .as_ref()?
            .expressway_network
            .routes
            .get(trip.route_index)?;
        let masterplan = self.state.masterplan.as_ref()?;
        let from_index = if trip.direction > 0 { route.from_node } else { route.to_node };
        let to_index = if trip.direction > 0 { route.to_node } else { route.from_node };
        let origin = *masterplan.node_ids.get(from_index)?;
        let destination = *masterplan.node_ids.get(to_index)?;
        Some(serde_json::json!({
            "entityType":"traveler", "id":trip.id, "name":"Regional through-traffic",
            "kind":"car", "activity":"Travelling through the city", "origin":origin,
            "destination":destination, "direction":trip.direction, "lane":trip.lane,
            "progress":trip.progress,
            "history":["Entered through a regional boundary", "Continuing toward the opposite regional boundary"]
        }))
    }
}

fn route_is_open(engine: &Engine, route: &Route) -> bool {
    let Some(progress) = engine.state.masterplan.as_ref() else {
        return false;
    };
    route.segments.iter().all(|segment| {
        let Some(&road_id) = progress.road_ids.get(segment.road_index) else {
            return false;
        };
        let (Some(&from_id), Some(&to_id)) = (
            progress.node_ids.get(segment.from_node),
            progress.node_ids.get(segment.to_node),
        ) else {
            return false;
        };
        if road_id == 0 || from_id == 0 || to_id == 0 {
            return false;
        }
        let Some(from) = engine.state.nodes.iter().find(|node| node.id == from_id) else {
            return false;
        };
        let Some(to) = engine.state.nodes.iter().find(|node| node.id == to_id) else {
            return false;
        };
        let endpoints_match = (from.x - segment.a.x).abs() <= 0.001
            && (from.z - segment.a.z).abs() <= 0.001
            && (to.x - segment.b.x).abs() <= 0.001
            && (to.z - segment.b.z).abs() <= 0.001;
        endpoints_match
            && engine.state.roads.iter().any(|road| {
                road.id == road_id
                    && road.kind == "expressway"
                    && road.stage == "open"
                    && ((road.from == from_id && road.to == to_id)
                        || (road.from == to_id && road.to == from_id))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_route_open(engine: &mut Engine, route_index: usize, open: bool) {
        let plan = engine.plan.as_ref().unwrap().plan.clone();
        let route = engine.plan.as_ref().unwrap().expressway_network.routes[route_index].clone();
        for segment in route.segments {
            for node_index in [segment.from_node, segment.to_node] {
                let mapped = engine.state.masterplan.as_ref().unwrap().node_ids[node_index];
                if mapped == 0 {
                    let node_plan = plan.nodes[node_index];
                    let id = engine.add_node(node_plan.p.x, node_plan.p.z, node_plan.kind.as_str());
                    engine.state.masterplan.as_mut().unwrap().node_ids[node_index] = id;
                }
            }
            let road_id = engine.state.masterplan.as_ref().unwrap().road_ids[segment.road_index];
            if road_id == 0 {
                let road_plan = plan.roads[segment.road_index];
                let mapping = &engine.state.masterplan.as_ref().unwrap().node_ids;
                let (from, to) = (mapping[road_plan.a], mapping[road_plan.b]);
                let id = engine.add_road(from, to, "expressway", if open { "open" } else { "construction" }, if open { 1.0 } else { 0.2 });
                engine.state.masterplan.as_mut().unwrap().road_ids[segment.road_index] = id;
            } else if let Some(road) = engine.state.roads.iter_mut().find(|road| road.id == road_id) {
                road.stage = if open { "open" } else { "construction" }.into();
                road.progress = if open { 1.0 } else { 0.2 };
            }
            let road_id = engine.state.masterplan.as_ref().unwrap().road_ids[segment.road_index];
            if let Some(road) = engine.state.roads.iter_mut().find(|road| road.id == road_id) {
                road.connected = open;
            }
        }
    }

    fn ready_engine() -> (Engine, usize) {
        let mut engine = Engine::new(3);
        let route_index = 0;
        set_route_open(&mut engine, route_index, true);
        (engine, route_index)
    }

    fn natural_engine_with_trip() -> Engine {
        let mut engine = Engine::new(3);
        engine.set_development_rate(16);
        for _ in 0..750 {
            if !engine.state.expressway_traffic.trips.is_empty() {
                break;
            }
            engine.step(80);
        }
        assert!(!engine.state.expressway_traffic.trips.is_empty(), "seed 3 did not dispatch expressway traffic within 60,000 ticks");
        engine
    }

    #[test]
    fn plan_contains_two_independent_boundary_chains_with_cubic_decks() {
        let plan = crate::masterplan::generate(&crate::Engine::new(3).masterplan_geography());
        let network = Network::from_plan(&plan);
        assert_eq!(network.routes.len(), 2);
        for route in &network.routes {
            assert!(route.from_node < plan.nodes.len() && route.to_node < plan.nodes.len());
            assert!([route.from_node, route.to_node].iter().all(|&node| {
                plan.nodes[node].p.x.abs() >= 390.0 || plan.nodes[node].p.z.abs() >= 390.0
            }));
            assert!(route.segments.iter().all(|segment| segment.length > 1.0));
            for pair in route.segments.windows(2) {
                assert_eq!(pair[0].to_node, pair[1].from_node);
                let end = pair[0].sample(pair[0].length).0;
                let start = pair[1].sample(0.0).0;
                assert!((end.x - start.x).abs() < 0.01 && (end.z - start.z).abs() < 0.01);
                assert_eq!(pair[0].deck_height, pair[1].deck_height);
            }
            let forward = route.sample(route.length * 0.5, 1, 0).unwrap();
            let reverse = route.sample(route.length * 0.5, -1, 0).unwrap();
            assert!(forward.0.x.is_finite() && forward.0.z.is_finite() && forward.1.is_finite());
            assert!(forward.2 > 0.0);
            let lane_gap = (forward.0.x - reverse.0.x).hypot(forward.0.z - reverse.0.z);
            assert!((lane_gap - LANE_OFFSET_METRES * 2.0).abs() < 0.02);
            let middle = route.length * 0.5;
            let middle_segment = route.segments.iter()
                .find(|segment| middle <= segment.start_distance + segment.length)
                .unwrap();
            let (center_point, tangent, _) = middle_segment.sample(middle - middle_segment.start_distance);
            let forward_cross = tangent.x * (forward.0.z - center_point.z)
                - tangent.z * (forward.0.x - center_point.x);
            let reverse_cross = tangent.x * (reverse.0.z - center_point.z)
                - tangent.z * (reverse.0.x - center_point.x);
            assert!(forward_cross > 0.0 && reverse_cross < 0.0);
        }
    }

    #[test]
    fn mixed_height_chain_is_not_flattened_into_a_route() {
        let mut plan = crate::masterplan::generate(&crate::Engine::new(3).masterplan_geography());
        let initial = Network::from_plan(&plan);
        assert_eq!(initial.routes.len(), 2);
        let index = initial.routes[0].segments[0].road_index;
        plan.roads[index].deck_height += 1.0;
        let changed = Network::from_plan(&plan);
        assert_eq!(changed.routes.len(), 1);
    }

    #[test]
    fn departures_wait_for_a_complete_route_and_stay_bounded() {
        let mut engine = Engine::new(3);
        let route = engine.plan.as_ref().unwrap().expressway_network.routes[0].clone();
        set_route_open(&mut engine, 0, false);
        let one_road = route.segments[0].road_index;
        let road_id = engine.state.masterplan.as_ref().unwrap().road_ids[one_road];
        engine.state.roads.iter_mut().find(|r| r.id == road_id).unwrap().stage = "open".into();
        engine.state.roads.iter_mut().find(|r| r.id == road_id).unwrap().connected = true;
        engine.state.tick = engine.state.expressway_traffic.next_departure_tick;
        engine.advance_expressway_traffic();
        assert!(engine.state.expressway_traffic.trips.is_empty());

        set_route_open(&mut engine, 0, true);
        engine.state.tick = engine.state.expressway_traffic.next_departure_tick;
        engine.advance_expressway_traffic();
        assert_eq!(engine.state.expressway_traffic.trips.len(), 1);

        let mut other = Engine::new(3);
        set_route_open(&mut other, 1, true);
        other.state.tick = other.state.expressway_traffic.next_departure_tick;
        other.advance_expressway_traffic();
        assert_eq!(other.state.expressway_traffic.trips.len(), 1);
        assert_eq!(other.state.expressway_traffic.trips[0].route_index, 1);

        let (mut full, route_index) = ready_engine();
        for index in 0..MAX_EXPRESSWAY_TRIPS {
            let id = full.id();
            full.state.expressway_traffic.trips.push(Trip {
                id, route_index, direction: if index % 2 == 0 { 1 } else { -1 },
                lane: ((index / 2) % 2) as u8, progress: 100.0,
            });
        }
        let deadline = full.state.expressway_traffic.next_departure_tick;
        full.state.tick = deadline;
        full.advance_expressway_traffic();
        assert_eq!(full.state.expressway_traffic.trips.len(), MAX_EXPRESSWAY_TRIPS);
        assert!(full.state.expressway_traffic.next_departure_tick > full.state.tick);
    }

    #[test]
    fn regional_chain_readiness_does_not_use_ground_connectivity() {
        let (mut disconnected_ground, _) = ready_engine();
        let route = disconnected_ground.plan.as_ref().unwrap().expressway_network.routes[0].clone();
        for segment in &route.segments {
            let road_id = disconnected_ground.state.masterplan.as_ref().unwrap().road_ids[segment.road_index];
            disconnected_ground.state.roads.iter_mut().find(|road| road.id == road_id).unwrap().connected = false;
        }
        disconnected_ground.state.tick = disconnected_ground.state.expressway_traffic.next_departure_tick;
        disconnected_ground.advance_expressway_traffic();
        assert_eq!(disconnected_ground.state.expressway_traffic.trips.len(), 1);

        let (mut missing_mapping, _) = ready_engine();
        let segment = &route.segments[route.segments.len() / 2];
        missing_mapping.state.masterplan.as_mut().unwrap().road_ids[segment.road_index] = 0;
        missing_mapping.state.tick = missing_mapping.state.expressway_traffic.next_departure_tick;
        missing_mapping.advance_expressway_traffic();
        assert!(missing_mapping.state.expressway_traffic.trips.is_empty());

        let (mut wrong_endpoints, _) = ready_engine();
        let segment = &route.segments[route.segments.len() / 2];
        let road_id = wrong_endpoints.state.masterplan.as_ref().unwrap().road_ids[segment.road_index];
        let other_node = wrong_endpoints.state.nodes.iter().find(|node| node.id != wrong_endpoints.state.masterplan.as_ref().unwrap().node_ids[segment.from_node]).unwrap().id;
        wrong_endpoints.state.roads.iter_mut().find(|road| road.id == road_id).unwrap().from = other_node;
        wrong_endpoints.state.tick = wrong_endpoints.state.expressway_traffic.next_departure_tick;
        wrong_endpoints.advance_expressway_traffic();
        assert!(wrong_endpoints.state.expressway_traffic.trips.is_empty());

        let (mut missing_middle, _) = ready_engine();
        let segment = &route.segments[route.segments.len() / 2];
        let road_id = missing_middle.state.masterplan.as_ref().unwrap().road_ids[segment.road_index];
        missing_middle.state.roads.iter_mut().find(|road| road.id == road_id).unwrap().stage = "construction".into();
        missing_middle.state.tick = missing_middle.state.expressway_traffic.next_departure_tick;
        missing_middle.advance_expressway_traffic();
        assert!(missing_middle.state.expressway_traffic.trips.is_empty());
    }

    #[test]
    fn trips_retire_at_the_boundary_and_publish_the_curve_height() {
        let (mut engine, route_index) = ready_engine();
        let route = engine.plan.as_ref().unwrap().expressway_network.routes[route_index].clone();
        let id = engine.id();
        engine.state.expressway_traffic.trips.push(Trip {
            id, route_index, direction: 1, lane: 0, progress: route.length - 0.5,
        });
        engine.state.expressway_traffic.next_departure_tick = u64::MAX;
        engine.advance_expressway_traffic();
        assert!(engine.state.expressway_traffic.trips.is_empty());

        let forward = route.sample(route.length * 0.5, 1, 0).unwrap();
        let reverse = route.sample(route.length * 0.5, -1, 0).unwrap();
        assert!((forward.2 - reverse.2).abs() < f32::EPSILON);
        let lane_gap = (forward.0.x - reverse.0.x).hypot(forward.0.z - reverse.0.z);
        assert!((lane_gap - LANE_OFFSET_METRES * 2.0).abs() < 0.02);
    }

    #[test]
    fn save_restore_continues_natural_trip_identically() {
        let mut live = natural_engine_with_trip();
        // The trip may have spawned on the final tick of the 80-tick batch.
        // Advance it onto the corridor before constructing its opposing peer.
        live.step(2);
        let id = live.state.expressway_traffic.trips[0].id;
        let mut opposing = live.state.expressway_traffic.trips[0].clone();
        opposing.id = live.id();
        opposing.direction = -opposing.direction;
        let route_length = live.plan.as_ref().unwrap().expressway_network.routes[opposing.route_index].length;
        opposing.progress = route_length - opposing.progress;
        live.state.expressway_traffic.trips.push(opposing);
        let movement = live.movement_buffer();
        assert_eq!(movement.len() % 6, 0);
        let first = live.state.expressway_traffic.trips[0].id as f32;
        let second = live.state.expressway_traffic.trips[1].id as f32;
        let first_offset = movement.chunks_exact(6).position(|entry| entry[0] == first).unwrap() * 6;
        let second_offset = movement.chunks_exact(6).position(|entry| entry[0] == second).unwrap() * 6;
        assert!((movement[first_offset + 5] - movement[second_offset + 5]).abs() < f32::EPSILON);
        let spacing = (movement[first_offset + 1] - movement[second_offset + 1])
            .hypot(movement[first_offset + 2] - movement[second_offset + 2]);
        assert!((spacing - LANE_OFFSET_METRES * 2.0).abs() < 0.02);
        let save = live.save_json();
        let mut restored = Engine::restore_json(&save).unwrap();
        assert_eq!(restored.save_json(), save);
        for _ in 0..6 {
            live.step(80);
            restored.step(80);
            assert_eq!(live.save_json(), restored.save_json());
            assert_eq!(live.movement_buffer(), restored.movement_buffer());
        }
        assert_eq!(live.state.expressway_traffic.trips[0].id, id);
    }

    #[test]
    fn malformed_saved_trip_state_is_rejected() {
        let engine = natural_engine_with_trip();
        let baseline: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        let reject = |candidate: serde_json::Value| {
            assert!(Engine::restore_json(&candidate.to_string()).is_err());
        };

        let mut wrong_direction = baseline.clone();
        wrong_direction["expressway_traffic"]["trips"][0]["direction"] = serde_json::json!(0);
        reject(wrong_direction);

        let mut outside_route = baseline.clone();
        outside_route["expressway_traffic"]["trips"][0]["progress"] = serde_json::json!(1.0e30);
        reject(outside_route);

        let mut colliding_id = baseline.clone();
        let entity_id = engine.state.nodes[0].id;
        colliding_id["expressway_traffic"]["trips"][0]["id"] = serde_json::json!(entity_id);
        reject(colliding_id);

        let trip = &engine.state.expressway_traffic.trips[0];
        let route = &engine.plan.as_ref().unwrap().expressway_network.routes[trip.route_index];
        let road_id = engine.state.masterplan.as_ref().unwrap().road_ids[route.segments[0].road_index];
        let mut closed_road = baseline;
        for road in closed_road["roads"].as_array_mut().unwrap() {
            if road["id"] == serde_json::json!(road_id) {
                road["stage"] = serde_json::json!("construction");
                break;
            }
        }
        reject(closed_road);
    }
}
