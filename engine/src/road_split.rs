use crate::network_split;
use crate::{
    point_segment_distance, Engine, Node, Road, RoadSplitError, RoadSplitOutcome, MAX,
    MAX_BUS_ROUTE_NODES, MAX_ROUTE_NODES, MAX_SEGMENTS, MIN,
};

struct PreparedRoadSplit {
    expected_next_id: u32,
    host_index: usize,
    host_road_id: u32,
    a_id: u32,
    b_id: u32,
    kind: String,
    stage: String,
    progress: f32,
    connected: bool,
    lanes: u8,
    split_node_id: u32,
    new_road_id: u32,
    split_x: f32,
    split_z: f32,
    first_length: f32,
    second_length: f32,
    frontage_remaps: Vec<(usize, bool, f32)>,
    traveler_remaps: Vec<(Vec<u32>, Vec<f32>, usize, f32)>,
    bus_route: Option<Vec<u32>>,
    train_routes: Vec<Vec<u32>>,
}

impl Engine {
    fn prepare_open_road_split(
        &self,
        host_road_id: u32,
        t: f32,
        prior: Option<&PreparedRoadSplit>,
        authorized_spine_pair: bool,
    ) -> Result<PreparedRoadSplit, RoadSplitError> {
        if prior.is_some_and(|split| split.host_road_id == host_road_id) {
            return Err(RoadSplitError::IneligibleRoad);
        }
        let extra = usize::from(prior.is_some());
        let expected_next_id = self
            .state
            .next_id
            .checked_add((extra * 2) as u32)
            .ok_or(RoadSplitError::Capacity)?;
        let host_index = self
            .state
            .roads
            .iter()
            .position(|road| road.id == host_road_id)
            .ok_or(RoadSplitError::MissingRoad)?;
        let host = &self.state.roads[host_index];
        if host.stage != "open"
            || !host.connected
            || !matches!(host.kind.as_str(), "local" | "arterial")
            || host.from == host.to
        {
            return Err(RoadSplitError::IneligibleRoad);
        }
        if !t.is_finite() || !(0.0..1.0).contains(&t) {
            return Err(RoadSplitError::InvalidFraction);
        }
        if (self.state.neighborhood_plan.is_some() && !authorized_spine_pair)
            || self.state.pending_interior_access.is_some()
        {
            return Err(RoadSplitError::ActivePlan);
        }
        let (a_id, b_id, kind, stage, progress, connected, lanes) = (
            host.from,
            host.to,
            host.kind.clone(),
            host.stage.clone(),
            host.progress,
            host.connected,
            host.lanes,
        );
        if self.state.projects.iter().any(|project| {
            project.kind == "road"
                && project.progress < 1.0
                && matches!((project.from, project.to), (Some(from), Some(to))
                    if (from == a_id && to == b_id) || (from == b_id && to == a_id))
        }) {
            return Err(RoadSplitError::ActiveProject);
        }
        if self.state.roads.len().saturating_add(extra) >= MAX_SEGMENTS
            || (self.state.neighborhood_road_ids.contains(&host_road_id)
                && self.state.neighborhood_road_ids.len().saturating_add(extra) >= MAX_SEGMENTS)
            || expected_next_id.checked_add(2).is_none()
        {
            return Err(RoadSplitError::Capacity);
        }
        // Preserve Engine::node's first-source-match semantics while avoiding
        // a complete node scan for each road during split preflight.
        let mut node_index = std::collections::BTreeMap::new();
        for node in &self.state.nodes {
            node_index.entry(node.id).or_insert(node);
        }
        let prospective_split_node_id = expected_next_id;
        let a = node_index.get(&a_id).ok_or(RoadSplitError::IneligibleRoad)?;
        let b = node_index.get(&b_id).ok_or(RoadSplitError::IneligibleRoad)?;
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz);
        if !length.is_finite() || length <= 0.0 {
            return Err(RoadSplitError::IneligibleRoad);
        }
        let first_length = length * t;
        let second_length = length - first_length;
        if first_length < 10.0 || second_length < 10.0 {
            return Err(RoadSplitError::SegmentTooShort);
        }
        let split_x = a.x + dx * t;
        let split_z = a.z + dz * t;
        if !split_x.is_finite()
            || !split_z.is_finite()
            || !(MIN..=MAX).contains(&split_x)
            || !(MIN..=MAX).contains(&split_z)
        {
            return Err(RoadSplitError::IneligibleRoad);
        }
        if prior.is_some_and(|split| (split.split_x - split_x).hypot(split.split_z - split_z) < 1.0)
        {
            return Err(RoadSplitError::IneligibleRoad);
        }
        if self.state.nodes.iter().any(|node| {
            node.id != a_id && node.id != b_id && (node.x - split_x).hypot(node.z - split_z) < 1.0
        }) || self
            .state
            .roads
            .iter()
            .filter(|road| road.id != host_road_id)
            .any(|road| {
                let (Some(from), Some(to)) = (node_index.get(&road.from), node_index.get(&road.to)) else {
                    return true;
                };
                point_segment_distance(split_x, split_z, from.x, from.z, to.x, to.z) < 0.5
            })
        {
            return Err(RoadSplitError::IneligibleRoad);
        }

        // Validate every frontage remap before mutation. Its parcel's access
        // node must stay an endpoint of the half that receives the frontage.
        let mut parcel_index = std::collections::BTreeMap::new();
        for parcel in &self.state.parcels {
            parcel_index.entry(parcel.id).or_insert(parcel);
        }
        let mut frontage_remaps = Vec::new();
        for (index, frontage) in self.state.parcel_frontages.iter().enumerate() {
            if frontage.road_id != host_road_id {
                continue;
            }
            if !frontage.along.is_finite() || !(0.0..=1.0).contains(&frontage.along) {
                return Err(RoadSplitError::UnsafeFrontage);
            }
            let parcel = parcel_index.get(&frontage.parcel_id)
                .ok_or(RoadSplitError::UnsafeFrontage)?;
            let (new_half, along) = if frontage.along < t {
                (false, frontage.along / t)
            } else if frontage.along > t {
                (true, (frontage.along - t) / (1.0 - t))
            } else if parcel.access_node == a_id {
                (false, 1.0)
            } else if parcel.access_node == b_id {
                (true, 0.0)
            } else {
                return Err(RoadSplitError::UnsafeFrontage);
            };
            let access_node_is_endpoint = if new_half {
                parcel.access_node == b_id
            } else {
                parcel.access_node == a_id
            };
            if !access_node_is_endpoint || !along.is_finite() || !(0.0..=1.0).contains(&along) {
                return Err(RoadSplitError::UnsafeFrontage);
            }
            frontage_remaps.push((index, new_half, along));
        }

        // Precompute traveler routes and cursor positions. Traveler lengths
        // are saved state, so split each affected saved length by the same
        // fraction and copy untouched lengths byte-for-byte.
        let mut traveler_remaps = Vec::with_capacity(self.state.travelers.len());
        for (index, traveler) in self.state.travelers.iter().enumerate() {
            let (route, route_lengths, old_edge_index, old_edge_progress) = match prior {
                Some(split) => {
                    let remap = &split.traveler_remaps[index];
                    (&remap.0, &remap.1, remap.2, remap.3)
                }
                None => (
                    &traveler.route,
                    &traveler.route_lengths,
                    traveler.edge_index,
                    traveler.edge_progress,
                ),
            };
            if route_lengths.len() != route.len().saturating_sub(1)
                || old_edge_index > route_lengths.len()
                || !old_edge_progress.is_finite()
                || old_edge_progress < 0.0
            {
                return Err(RoadSplitError::InvalidActorRoute);
            }
            let mut expanded = network_split::expand_route(
                &route,
                a_id,
                b_id,
                prospective_split_node_id,
                first_length,
                second_length,
            );
            if expanded.route.len() > route.len() && expanded.route.len() > MAX_ROUTE_NODES {
                return Err(RoadSplitError::Capacity);
            }
            let mut lengths = Vec::with_capacity(expanded.route.len().saturating_sub(1));
            for (edge_index, old_length) in route_lengths.iter().copied().enumerate() {
                if !old_length.is_finite() || old_length <= 0.0 {
                    return Err(RoadSplitError::InvalidActorRoute);
                }
                match expanded.first_split_lengths[edge_index] {
                    Some(_) if route[edge_index] == a_id => {
                        let first = old_length * t;
                        expanded.first_split_lengths[edge_index] = Some(first);
                        lengths.push(first);
                        lengths.push(old_length - first);
                    }
                    Some(_) => {
                        let first = old_length * (1.0 - t);
                        expanded.first_split_lengths[edge_index] = Some(first);
                        lengths.push(first);
                        lengths.push(old_length - first);
                    }
                    None => lengths.push(old_length),
                }
            }
            let (edge_index, edge_progress) = network_split::remap_edge_position(
                &expanded,
                route_lengths.len(),
                old_edge_index,
                old_edge_progress,
            )
            .ok_or(RoadSplitError::InvalidActorRoute)?;
            traveler_remaps.push((expanded.route, lengths, edge_index, edge_progress));
        }

        let bus_route = if let Some(line) = self.state.bus_line.as_ref() {
            let input = prior
                .and_then(|split| split.bus_route.as_ref())
                .unwrap_or(&line.route);
            let route = network_split::expand_route(
                input,
                a_id,
                b_id,
                prospective_split_node_id,
                first_length,
                second_length,
            )
            .route;
            if route.len() > input.len() && route.len() > MAX_BUS_ROUTE_NODES {
                return Err(RoadSplitError::Capacity);
            }
            Some(route)
        } else {
            None
        };
        let mut train_routes = Vec::with_capacity(self.state.trains.len());
        for (index, train) in self.state.trains.iter().enumerate() {
            let input = prior
                .map(|split| &split.train_routes[index])
                .unwrap_or(&train.route);
            let route = network_split::expand_route(
                input,
                a_id,
                b_id,
                prospective_split_node_id,
                first_length,
                second_length,
            )
            .route;
            if route.len() > input.len() && route.len() > MAX_ROUTE_NODES {
                return Err(RoadSplitError::Capacity);
            }
            train_routes.push(route);
        }

        let prepared = PreparedRoadSplit {
            expected_next_id,
            host_index,
            host_road_id,
            a_id,
            b_id,
            kind,
            stage,
            progress,
            connected,
            lanes,
            split_node_id: prospective_split_node_id,
            new_road_id: prospective_split_node_id + 1,
            split_x,
            split_z,
            first_length,
            second_length,
            frontage_remaps,
            traveler_remaps,
            bus_route,
            train_routes,
        };
        Ok(prepared)
    }

    fn commit_open_road_split(&mut self, prepared: PreparedRoadSplit) -> RoadSplitOutcome {
        debug_assert_eq!(self.state.next_id, prepared.expected_next_id);
        debug_assert_eq!(
            self.state
                .roads
                .get(prepared.host_index)
                .map(|road| road.id),
            Some(prepared.host_road_id)
        );
        debug_assert_eq!(
            self.state
                .roads
                .get(prepared.host_index)
                .map(|road| (road.from, road.to)),
            Some((prepared.a_id, prepared.b_id))
        );

        let split_node_id = self.id();
        debug_assert_eq!(split_node_id, prepared.split_node_id);
        let new_road_id = self.id();
        debug_assert_eq!(new_road_id, prepared.new_road_id);
        self.state.nodes.push(Node {
            id: split_node_id,
            x: prepared.split_x,
            z: prepared.split_z,
            kind: "junction".into(),
        });
        {
            let retained = &mut self.state.roads[prepared.host_index];
            retained.to = split_node_id;
            retained.length = prepared.first_length;
        }
        self.state.roads.push(Road {
            id: new_road_id,
            from: split_node_id,
            to: prepared.b_id,
            kind: prepared.kind,
            stage: prepared.stage,
            progress: prepared.progress,
            connected: prepared.connected,
            length: prepared.second_length,
            lanes: prepared.lanes,
        });
        for (index, new_half, along) in prepared.frontage_remaps {
            let frontage = &mut self.state.parcel_frontages[index];
            frontage.road_id = if new_half {
                new_road_id
            } else {
                prepared.host_road_id
            };
            frontage.along = along;
        }
        for (traveler, (route, lengths, edge_index, edge_progress)) in self
            .state
            .travelers
            .iter_mut()
            .zip(prepared.traveler_remaps)
        {
            traveler.route = route;
            traveler.route_lengths = lengths;
            traveler.edge_index = edge_index;
            traveler.edge_progress = edge_progress;
        }
        if let (Some(line), Some(route)) = (&mut self.state.bus_line, prepared.bus_route) {
            line.route = route;
        }
        for (train, route) in self.state.trains.iter_mut().zip(prepared.train_routes) {
            train.route = route;
        }
        if self
            .state
            .neighborhood_road_ids
            .remove(&prepared.host_road_id)
        {
            self.state
                .neighborhood_road_ids
                .insert(prepared.host_road_id);
            self.state.neighborhood_road_ids.insert(new_road_id);
        }
        self.recompute_connectivity();
        if let Some(line) = self.state.bus_line.as_mut() {
            line.graph_revision = self
                .state
                .roads
                .iter()
                .filter(|road| road.stage == "open" && road.connected && road.kind != "rail")
                .count() as u64;
        }
        self.mark_access_geometry_changed();
        self.state.revision = self.state.revision.saturating_add(1);
        RoadSplitOutcome {
            split_node_id,
            retained_road_id: prepared.host_road_id,
            new_road_id,
        }
    }

    /// Preflight both hosts against the same world, composing actor routes in
    /// memory. A rejection allocates no IDs and changes no authoritative state.
    /// The existing active-plan guard still applies until an owned spine
    /// lifecycle supplies narrowly scoped authorization.
    pub(super) fn try_split_open_road_pair(
        &mut self,
        hosts: [(u32, f32); 2],
    ) -> Result<[RoadSplitOutcome; 2], RoadSplitError> {
        if self.state.pending_interior_access.is_some() {
            return Err(RoadSplitError::ActivePlan);
        }
        let authorized = if let Some(plan) = self.state.neighborhood_plan.as_ref() {
            let segment_matches = |edge: usize, road_id: u32| {
                let Some((from, to)) = plan.segments.get(edge) else {
                    return false;
                };
                self.state.roads.iter().any(|road| {
                    road.id == road_id
                        && ((road.from == *from && road.to == *to)
                            || (road.from == *to && road.to == *from))
                })
            };
            self.validate_neighborhood_spine(plan).is_ok()
                && plan.next_segment == plan.segments.len()
                && plan.segments.len() == 4
                && hosts[0].1 == 0.5
                && hosts[1].1 == 0.5
                && segment_matches(3, hosts[0].0)
                && segment_matches(1, hosts[1].0)
                && plan.spine.as_ref().is_some_and(|spine| {
                    matches!(
                        spine.stage,
                        crate::spine_plan::SpineStage::PerimeterReady { host_road_ids }
                            if host_road_ids == [hosts[0].0, hosts[1].0]
                    )
                })
        } else {
            // Paired splits are also a general atomic network primitive. The
            // canonical spine checks above only authorize its narrow use while
            // a neighborhood reservation is active.
            false
        };
        if self.state.neighborhood_plan.is_some() && !authorized {
            return Err(RoadSplitError::ActivePlan);
        }
        let first = self.prepare_open_road_split(hosts[0].0, hosts[0].1, None, authorized)?;
        let second =
            self.prepare_open_road_split(hosts[1].0, hosts[1].1, Some(&first), authorized)?;
        let first = self.commit_open_road_split(first);
        let second = self.commit_open_road_split(second);
        Ok([first, second])
    }

    /// Split an already-open ordinary road without changing its geometry or
    /// introducing a shortcut between its old endpoints. Preparation is
    /// read-only; the commit runs only after every validation succeeds.
    pub(super) fn try_split_open_road(
        &mut self,
        host_road_id: u32,
        t: f32,
    ) -> Result<RoadSplitOutcome, RoadSplitError> {
        let prepared = self.prepare_open_road_split(host_road_id, t, None, false)?;
        Ok(self.commit_open_road_split(prepared))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bus, BusLine, Train, Traveler};

    fn adjacent_hosts(engine: &Engine) -> Option<(Road, Road, Vec<u32>)> {
        let roads: Vec<Road> = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
                    && road.length >= 20.0
            })
            .cloned()
            .collect();
        for (i, first) in roads.iter().enumerate() {
            for second in roads.iter().skip(i + 1) {
                let shared = [first.from, first.to]
                    .into_iter()
                    .find(|node| *node == second.from || *node == second.to);
                let Some(shared) = shared else { continue };
                let first_outer = if first.from == shared {
                    first.to
                } else {
                    first.from
                };
                let second_outer = if second.from == shared {
                    second.to
                } else {
                    second.from
                };
                if first_outer != second_outer {
                    return Some((
                        first.clone(),
                        second.clone(),
                        vec![first_outer, shared, second_outer],
                    ));
                }
            }
        }
        None
    }

    fn actor_fixture(seed: u32) -> (Engine, [u32; 2], Vec<u32>) {
        let mut engine = Engine::new(seed);
        let (first, second, forward) =
            adjacent_hosts(&engine).expect("seed should provide a connected pair of open roads");
        let reverse = forward.iter().copied().rev().collect::<Vec<_>>();
        engine.state.travelers.clear();
        engine.state.buses.clear();
        engine.state.bus_line = None;
        engine.state.trains.clear();
        engine.state.residents.clear();

        for (route, edge_index, fraction) in [
            (forward.clone(), 0usize, 0.31f32),
            (forward.clone(), 1usize, 0.67f32),
            (reverse.clone(), 0usize, 0.42f32),
            (reverse.clone(), 1usize, 0.58f32),
        ] {
            let route_lengths = engine.route_lengths(&route);
            let edge_progress = route_lengths[edge_index] * fraction;
            let id = engine.id();
            engine.state.travelers.push(Traveler {
                id,
                kind: 0,
                route: route.clone(),
                route_lengths,
                edge_index,
                edge_progress,
                speed: 1.0,
                destination: *route.last().unwrap(),
                activity: "paired split regression".into(),
                resident: 0,
                name: format!("Split actor {id}"),
                origin_building: None,
                destination_building: None,
                wait_ticks: 0,
                origin_access: None,
                destination_access: None,
                access_phase: 1,
                access_progress: 0.0,
            });
        }

        let forward_lengths = engine.route_lengths(&forward);
        let line_id = engine.id();
        engine.state.bus_line = Some(BusLine {
            id: line_id,
            stops: forward.clone(),
            route: forward.clone(),
            active: true,
            graph_revision: 4,
        });
        let bus_id = engine.id();
        engine.state.buses.push(Bus {
            id: bus_id,
            line_id,
            progress: forward_lengths[0] + forward_lengths[1] * 0.43,
            direction: 1,
            dwell_ticks: 0,
            passengers: Vec::new(),
            next_departure_tick: 900,
            last_stop: Some(forward[0]),
        });
        let reverse_lengths = engine.route_lengths(&reverse);
        let train_id = engine.id();
        engine.state.station_online = true;
        engine.state.trains.push(Train {
            id: train_id,
            route: reverse,
            distance: reverse_lengths[0] + reverse_lengths[1] * 0.36,
            speed: 8.0,
            direction: 1,
            dwell_ticks: 0,
            passengers: Vec::new(),
        });

        (engine, [first.id, second.id], forward)
    }

    #[test]
    fn paired_split_matches_two_sequential_splits_with_routes_and_transit() {
        let (mut paired, host_ids, forward) = actor_fixture(731);
        let (mut sequential, sequential_host_ids, _) = actor_fixture(731);
        assert_eq!(host_ids, sequential_host_ids);
        assert_eq!(paired.save_json(), sequential.save_json());
        let paired_outcomes = paired
            .try_split_open_road_pair([(host_ids[0], 0.5), (host_ids[1], 0.5)])
            .unwrap();
        let first = sequential.try_split_open_road(host_ids[0], 0.5).unwrap();
        let second = sequential.try_split_open_road(host_ids[1], 0.5).unwrap();
        assert_eq!(paired_outcomes, [first, second]);
        assert_eq!(paired.save_json(), sequential.save_json());

        let reverse = forward.iter().copied().rev().collect::<Vec<_>>();
        for traveler in &paired.state.travelers {
            assert!(traveler.route.contains(&paired_outcomes[0].split_node_id));
            assert!(traveler.route.contains(&paired_outcomes[1].split_node_id));
            assert_eq!(traveler.route_lengths.len(), traveler.route.len() - 1);
        }
        assert_eq!(paired.state.travelers[0].route[0], forward[0]);
        assert_eq!(paired.state.travelers[2].route[0], reverse[0]);
        assert_eq!(paired.state.bus_line.as_ref().unwrap().route[0], forward[0]);
        assert_eq!(paired.state.bus_line.as_ref().unwrap().route.len(), 5);
        assert_eq!(paired.state.trains[0].route[0], reverse[0]);
        assert_eq!(paired.state.trains[0].route.len(), 5);
        paired.validate().unwrap();
    }

    #[test]
    fn paired_split_rejections_are_atomic_and_reject_duplicate_hosts() {
        let (mut engine, hosts, _) = actor_fixture(732);
        for (second_host, second_t, expected) in [
            (u32::MAX, 0.5, RoadSplitError::MissingRoad),
            (hosts[1], f32::NAN, RoadSplitError::InvalidFraction),
            (hosts[0], 0.5, RoadSplitError::IneligibleRoad),
        ] {
            let before = engine.save_json();
            assert_eq!(
                engine.try_split_open_road_pair([(hosts[0], 0.5), (second_host, second_t)]),
                Err(expected)
            );
            assert_eq!(engine.save_json(), before);
        }
    }

    #[test]
    fn combined_segment_and_route_capacity_failures_leave_state_unchanged() {
        let (mut segment_limited, hosts, _) = actor_fixture(733);
        let filler = segment_limited.state.roads[0].clone();
        segment_limited.state.roads.resize(MAX_SEGMENTS - 1, filler);
        let before = segment_limited.save_json();
        assert!(segment_limited
            .prepare_open_road_split(hosts[0], 0.5, None, false)
            .is_ok());
        assert_eq!(segment_limited.save_json(), before);
        assert_eq!(
            segment_limited.try_split_open_road_pair([(hosts[0], 0.5), (hosts[1], 0.5)]),
            Err(RoadSplitError::Capacity)
        );
        assert_eq!(segment_limited.save_json(), before);

        let (mut route_limited, hosts, forward) = actor_fixture(734);
        let excluded: std::collections::BTreeSet<u32> = forward.iter().copied().collect();
        let spare: Vec<u32> = route_limited
            .state
            .nodes
            .iter()
            .map(|node| node.id)
            .filter(|id| !excluded.contains(id))
            .take(2)
            .collect();
        assert_eq!(spare.len(), 2);
        let mut route = forward;
        while route.len() < MAX_ROUTE_NODES - 1 {
            route.push(spare[route.len() % 2]);
        }
        let route_lengths = route_limited.route_lengths(&route);
        assert!(route_lengths
            .iter()
            .all(|length| length.is_finite() && *length > 0.0));
        let traveler = &mut route_limited.state.travelers[0];
        traveler.route = route;
        traveler.route_lengths = route_lengths;
        traveler.edge_index = 1;
        traveler.edge_progress = traveler.route_lengths[1] * 0.3;
        let before = route_limited.save_json();
        assert!(route_limited
            .prepare_open_road_split(hosts[0], 0.5, None, false)
            .is_ok());
        assert_eq!(route_limited.save_json(), before);
        assert_eq!(
            route_limited.try_split_open_road_pair([(hosts[0], 0.5), (hosts[1], 0.5)]),
            Err(RoadSplitError::Capacity)
        );
        assert_eq!(route_limited.save_json(), before);
    }
}
