//! Versioned integer sidecar for movement poses and infrastructure provenance.
use crate::{NodeKind, RoadKind, Stage};

#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub(crate) actor_id: u32,
    pub(crate) x: f32,
    pub(crate) z: f32,
    pub(crate) heading: f32,
    pub(crate) kind: u32,
    pub(crate) surface_y: f32,
    pub(crate) resolved: bool,
    pub(crate) road_ids: Vec<u32>,
    pub(crate) platform_node_ids: Vec<u32>,
}

impl Row {
    pub(crate) fn encode(rows: &[Self]) -> Vec<u32> {
        let mut out = vec![1, rows.len() as u32];
        for row in rows {
            out.extend_from_slice(&[
                row.actor_id,
                row.x.to_bits(),
                row.z.to_bits(),
                row.heading.to_bits(),
                row.kind,
                row.surface_y.to_bits(),
                u32::from(row.resolved),
                row.road_ids.len() as u32,
                row.platform_node_ids.len() as u32,
            ]);
            out.extend_from_slice(&row.road_ids);
            out.extend_from_slice(&row.platform_node_ids);
        }
        out
    }
}

impl crate::Engine {
    pub(crate) fn platform_support_road(&self, node: u32) -> Option<u32> {
        let (station, gateway) = self.rail_terminals()?;
        if node != station && node != gateway {
            return None;
        }
        // The platform frame is authored for this service axis. Additional
        // rail branches at a terminal do not change that platform's support.
        self.state
            .roads
            .iter()
            .find(|road| {
                road.kind == RoadKind::Rail
                    && road.stage == Stage::Open
                    && road.connected
                    && ((road.from == station && road.to == gateway)
                        || (road.from == gateway && road.to == station))
            })
            .map(|road| road.id)
    }
    /// Resolve the authored access path, never a nearest-road approximation.
    /// Its route endpoint and its building/queue/platform both support the walk.
    pub(crate) fn access_movement_support(
        &self,
        traveler: &crate::Traveler,
        path: &crate::AccessPath,
    ) -> Option<(Vec<u32>, Vec<u32>)> {
        let at_start = traveler.access_phase == 0;
        let edge = if at_start {
            traveler.route.get(..2)?
        } else {
            traveler.route.get(traveler.route.len().checked_sub(2)?..)?
        };
        let road_for = |edge: &[u32]| {
            self.state
                .roads
                .iter()
                .find(|road| {
                    road.stage == Stage::Open
                        && road.connected
                        && road.kind != RoadKind::Rail
                        && ((road.from == edge[0] && road.to == edge[1])
                            || (road.from == edge[1] && road.to == edge[0]))
                })
                .map(|road| road.id)
        };
        let mut roads = vec![road_for(edge)?];
        let building = if at_start {
            traveler.origin_building
        } else {
            traveler.destination_building
        };
        if let Some(building) = building {
            let (id, _) = self.building_access_segment(building)?;
            self.state
                .roads
                .iter()
                .find(|road| road.id == id && road.stage == Stage::Open && road.connected)?;
            if !roads.contains(&id) {
                roads.push(id);
            }
            return Some((roads, Vec::new()));
        }
        let first = *path.points.first()?;
        let last = *path.points.last()?;
        let endpoint =
            self.route_endpoint_position(&traveler.route, at_start, traveler.id, traveler.kind)?;
        let actual_endpoint = if at_start { last } else { first };
        if actual_endpoint.x != endpoint.x || actual_endpoint.z != endpoint.z {
            return None;
        }
        let same = |candidate: &crate::AccessPath| {
            candidate.surface_y == path.surface_y
                && candidate.points.len() == path.points.len()
                && candidate
                    .points
                    .iter()
                    .zip(&path.points)
                    .all(|(a, b)| a.x == b.x && a.z == b.z)
        };
        if let Some((station, gateway)) = self.rail_terminals() {
            for (at_station, node) in [(true, station), (false, gateway)] {
                let candidate = if at_start {
                    self.platform_exit_path_from_position(first, last, at_station)
                } else {
                    self.platform_entry_path_from_position(first, last, at_station, traveler.id)
                };
                if candidate.as_ref().is_some_and(same) {
                    roads.push(self.platform_support_road(node)?);
                    return Some((roads, vec![node]));
                }
            }
        }
        // Bus access is a saved two-point connector. Check its queue endpoint
        // against the exact directional queue construction, not its proximity.
        let resident = self
            .state
            .residents
            .iter()
            .find(|resident| resident.id == traveler.resident)?;
        let stop = if at_start {
            *traveler.route.first()?
        } else {
            *traveler.route.last()?
        };
        let slot = if at_start { 0 } else { resident.queue_slot? };
        let (queue, queue_edge) =
            self.bus_queue_position_with_edge(stop, self.bus_trip_direction(resident)?, slot)?;
        let candidate = crate::AccessPath {
            points: if at_start {
                vec![queue, endpoint]
            } else {
                vec![endpoint, queue]
            },
            surface_y: Vec::new(),
        };
        if !same(&candidate) {
            return None;
        }
        let id = road_for(&queue_edge)?;
        if !roads.contains(&id) {
            roads.push(id);
        }
        Some((roads, Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::Row;
    use crate::Engine;

    #[test]
    fn packet_has_version_header_and_exact_integer_bit_payload() {
        let rows = [Row {
            actor_id: 7,
            x: 1.25,
            z: -2.0,
            heading: 0.5,
            kind: 3,
            surface_y: 8.0,
            resolved: false,
            road_ids: vec![11, 12],
            platform_node_ids: vec![19],
        }];
        assert_eq!(
            Row::encode(&rows),
            vec![
                1,
                1,
                7,
                1.25f32.to_bits(),
                (-2.0f32).to_bits(),
                0.5f32.to_bits(),
                3,
                8.0f32.to_bits(),
                0,
                2,
                1,
                11,
                12,
                19
            ]
        );
    }

    #[test]
    fn engine_packet_poses_match_legacy_order_and_resolved_roads_are_open() {
        let mut engine = Engine::new(87231);
        engine.step(240);
        let packet = engine.movement_packet();
        let legacy = engine.movement_buffer();
        assert_eq!(packet[0], 1);
        let count = packet[1] as usize;
        let mut cursor = 2;
        let mut poses = Vec::with_capacity(count * 6);
        for _ in 0..count {
            let row = &packet[cursor..cursor + 9];
            cursor += 9;
            poses.extend_from_slice(&[
                row[0] as f32,
                f32::from_bits(row[1]),
                f32::from_bits(row[2]),
                f32::from_bits(row[3]),
                row[4] as f32,
                f32::from_bits(row[5]),
            ]);
            let road_count = row[7] as usize;
            let platform_count = row[8] as usize;
            if row[6] == 1 {
                assert!(road_count > 0 || platform_count > 0);
                for id in &packet[cursor..cursor + road_count] {
                    let road = engine
                        .state
                        .roads
                        .iter()
                        .find(|road| road.id == *id)
                        .expect("resolved support references a road");
                    assert_eq!(road.stage, crate::Stage::Open);
                    if road.kind == crate::RoadKind::Expressway {
                        assert_eq!(row[4], 1);
                        assert!(engine.state.nodes.iter().any(|node| node.id == road.from));
                        assert!(engine.state.nodes.iter().any(|node| node.id == road.to));
                    } else {
                        assert!(road.connected, "ordinary movement road must be connected");
                    }
                }
            }
            cursor += road_count + platform_count;
        }
        assert_eq!(cursor, packet.len());
        assert_eq!(poses, legacy);
        assert_eq!(count * 6, legacy.len());
    }

    #[test]
    fn current_format_save_resume_preserves_evolving_movement_packets() {
        let mut engine = Engine::new(87231);
        let road = engine
            .state
            .roads
            .iter()
            .find(|road| {
                road.kind != crate::RoadKind::Rail
                    && road.stage == crate::Stage::Open
                    && road.connected
            })
            .expect("fixture has a connected open road")
            .clone();
        let a = engine.node(road.from).unwrap().clone();
        let b = engine.node(road.to).unwrap().clone();
        let length = (b.x - a.x).hypot(b.z - a.z);
        let traveler_id = engine.id();
        engine.state.travelers.push(crate::Traveler {
            id: traveler_id,
            kind: 1,
            route: vec![road.from, road.to],
            route_lengths: vec![length],
            edge_index: 0,
            edge_progress: length * 0.25,
            speed: 1.0,
            destination: road.to,
            activity: "movement_packet_test".into(),
            resident: 0,
            name: "Movement packet test traveler".into(),
            origin_building: None,
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        });
        let save = engine.save_json();
        let mut resumed = Engine::restore_json(&save).expect("current-format save restores");
        assert_eq!(resumed.save_json(), save);

        let mut observed_actors = 0usize;
        for _ in 0..24 {
            engine.step(1);
            resumed.step(1);
            let packet = engine.movement_packet();
            assert_eq!(packet, resumed.movement_packet());
            observed_actors += packet[1] as usize;
        }
        assert!(
            observed_actors > 0,
            "fixture must produce actor observations"
        );
    }

    #[test]
    fn bus_packet_support_tracks_direction_sensitive_junction_edge() {
        let mut engine = Engine::new_legacy(217);
        let mut node_ids = Vec::new();
        for (x, z) in [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0)] {
            let id = engine.id();
            engine.state.nodes.push(crate::Node {
                id,
                x,
                z,
                kind: crate::NodeKind::Junction,
            });
            node_ids.push(id);
        }
        let mut road_ids = Vec::new();
        for edge in node_ids.windows(2) {
            let id = engine.id();
            engine.state.roads.push(crate::Road {
                id,
                from: edge[0],
                to: edge[1],
                kind: crate::RoadKind::Local,
                stage: crate::Stage::Open,
                progress: 1.0,
                connected: true,
                length: 40.0,
                lanes: 2,
            });
            road_ids.push(id);
        }
        let line_id = engine.id();
        engine.state.bus_line = Some(crate::BusLine {
            id: line_id,
            stops: vec![node_ids[0], node_ids[2]],
            route: node_ids.clone(),
            active: true,
            graph_revision: 1,
        });
        let bus_id = engine.id();
        engine.state.buses.push(crate::Bus {
            id: bus_id,
            line_id,
            progress: 40.0,
            direction: 1,
            dwell_ticks: 0,
            passengers: Vec::new(),
            next_departure_tick: 0,
            last_stop: None,
        });
        let line = engine.state.bus_line.as_ref().unwrap().clone();
        let junction_distance = 40.0f32;
        let mut exact_junction_roads = Vec::new();

        for direction in [1, -1] {
            for offset in [-0.0005, 0.0, 0.0005] {
                let bus_snapshot = {
                    let bus = &mut engine.state.buses[0];
                    bus.direction = direction;
                    bus.progress = junction_distance + offset;
                    bus.clone()
                };
                let (_, expected_edge) = engine
                    .bus_pose_with_edge(&line, &bus_snapshot)
                    .expect("bus pose selects a route edge");
                let expected_road = road_ids[node_ids
                    .windows(2)
                    .position(|edge| edge == expected_edge)
                    .expect("selected edge belongs to test route")];
                let packet = engine.movement_packet();
                let mut cursor = 2;
                let mut found = None;
                for _ in 0..packet[1] {
                    let row = &packet[cursor..cursor + 9];
                    cursor += 9;
                    let road_count = row[7] as usize;
                    let platform_count = row[8] as usize;
                    if row[0] == bus_id {
                        found = Some(packet[cursor..cursor + road_count].to_vec());
                    }
                    cursor += road_count + platform_count;
                }
                assert_eq!(
                    found.expect("bus has a movement packet row"),
                    vec![expected_road]
                );
                if offset == 0.0 {
                    exact_junction_roads.push(expected_road);
                }
            }
        }
        assert_ne!(exact_junction_roads[0], exact_junction_roads[1]);
    }

    #[test]
    fn waiting_bus_packet_uses_queue_selected_road_without_changing_pose() {
        let mut engine = Engine::new_legacy(217);
        let mut node_ids = Vec::new();
        for (x, z) in [(0.0, 0.0), (40.0, 0.0), (80.0, 0.0), (40.0, 40.0)] {
            let id = engine.id();
            engine.state.nodes.push(crate::Node {
                id,
                x,
                z,
                kind: crate::NodeKind::Junction,
            });
            node_ids.push(id);
        }
        // The stop is node_ids[1]. Its route neighbors are west/east, and the
        // fourth node gives it another valid incident road for the ambiguity case.
        let mut road_ids = Vec::new();
        for (from, to) in [
            (node_ids[0], node_ids[1]),
            (node_ids[1], node_ids[2]),
            (node_ids[1], node_ids[3]),
        ] {
            let id = engine.id();
            let a = engine.node(from).unwrap().clone();
            let b = engine.node(to).unwrap().clone();
            engine.state.roads.push(crate::Road {
                id,
                from,
                to,
                kind: crate::RoadKind::Local,
                stage: crate::Stage::Open,
                progress: 1.0,
                connected: true,
                length: (b.x - a.x).hypot(b.z - a.z),
                lanes: 2,
            });
            road_ids.push(id);
        }
        let line_id = engine.id();
        engine.state.bus_line = Some(crate::BusLine {
            id: line_id,
            stops: vec![node_ids[0], node_ids[1], node_ids[2]],
            route: vec![node_ids[0], node_ids[1], node_ids[2]],
            active: true,
            graph_revision: 1,
        });
        let resident_id = engine.id();
        let home_building = engine.state.buildings[0].id;
        engine.state.residents.push(crate::Resident {
            id: resident_id,
            name: "Queue support test".into(),
            home_building,
            work_building: None,
            home_node: node_ids[0],
            work_node: Some(node_ids[2]),
            mode: 0,
            routine: "worker".into(),
            activity: "waiting".into(),
            at_work: false,
            departure_tick: 0,
            return_tick: 0,
            last_departure_day: u32::MAX,
            last_return_day: u32::MAX,
            history: Vec::new(),
            rail_commuter: false,
            transfer: "waiting_bus".into(),
            train_id: None,
            queue_slot: Some(2),
            bus_id: None,
            bus_origin_stop: Some(node_ids[0]),
            bus_destination_stop: Some(node_ids[2]),
            bus_stop_node: Some(node_ids[1]),
            pending_alight: None,
        });
        let line = engine.state.bus_line.as_ref().unwrap().clone();
        let expected = [(1, road_ids[1]), (-1, road_ids[0])];
        for (direction, expected_road) in expected {
            {
                let resident = &mut engine.state.residents[0];
                if direction > 0 {
                    resident.bus_origin_stop = Some(node_ids[0]);
                    resident.bus_destination_stop = Some(node_ids[2]);
                } else {
                    resident.bus_origin_stop = Some(node_ids[2]);
                    resident.bus_destination_stop = Some(node_ids[0]);
                }
            }
            let (expected_position, edge) = engine
                .bus_queue_position_with_edge(node_ids[1], direction, 2)
                .expect("queue location has a selected route edge");
            assert_eq!(
                engine
                    .bus_queue_position(node_ids[1], direction, 2)
                    .unwrap()
                    .x
                    .to_bits(),
                expected_position.x.to_bits()
            );
            assert_eq!(
                engine
                    .bus_queue_position(node_ids[1], direction, 2)
                    .unwrap()
                    .z
                    .to_bits(),
                expected_position.z.to_bits()
            );
            let edge_road = road_ids[if edge == [node_ids[1], node_ids[2]] {
                1
            } else {
                0
            }];
            assert_eq!(edge_road, expected_road);

            let packet = engine.movement_packet();
            let mut cursor = 2;
            let mut found = None;
            for _ in 0..packet[1] {
                let row = &packet[cursor..cursor + 9];
                cursor += 9;
                let road_count = row[7] as usize;
                let platform_count = row[8] as usize;
                if row[0] == resident_id {
                    found = Some((
                        row[1],
                        row[2],
                        row[6],
                        packet[cursor..cursor + road_count].to_vec(),
                    ));
                }
                cursor += road_count + platform_count;
            }
            let (x_bits, z_bits, resolved, roads) =
                found.expect("waiting rider is in movement packet");
            assert_eq!(x_bits, expected_position.x.to_bits());
            assert_eq!(z_bits, expected_position.z.to_bits());
            assert_eq!(resolved, 1);
            assert_eq!(roads, vec![expected_road]);
        }
        assert_eq!(line.route[1], node_ids[1]);
    }

    #[test]
    fn exact_rail_access_paths_resolve_route_and_platform_but_perturbations_do_not() {
        let mut engine = Engine::new_legacy(48);
        let (from, to, rail_id) = {
            let rail = engine
                .state
                .roads
                .iter_mut()
                .find(|road| road.kind == crate::RoadKind::Rail)
                .expect("legacy fixture has a rail road");
            rail.stage = crate::Stage::Open;
            rail.progress = 1.0;
            (rail.from, rail.to, rail.id)
        };
        let station = [from, to]
            .into_iter()
            .find(|id| {
                engine
                    .node(*id)
                    .is_some_and(|node| node.kind == crate::NodeKind::Station)
            })
            .expect("rail fixture has a station terminal");
        let gateway = if station == from { to } else { from };
        engine.state.station_online = true;

        for (at_station, terminal) in [(true, station), (false, gateway)] {
            let terminal_node = engine.node(terminal).unwrap().clone();
            let ground = engine.id();
            engine.state.nodes.push(crate::Node {
                id: ground,
                x: terminal_node.x + 35.0,
                z: terminal_node.z + 20.0,
                kind: crate::NodeKind::Junction,
            });
            let ground_node = engine.node(ground).unwrap().clone();
            let local_road = engine.id();
            engine.state.roads.push(crate::Road {
                id: local_road,
                from: terminal,
                to: ground,
                kind: crate::RoadKind::Local,
                stage: crate::Stage::Open,
                progress: 1.0,
                connected: true,
                length: (ground_node.x - terminal_node.x).hypot(ground_node.z - terminal_node.z),
                lanes: 2,
            });
            engine.recompute_connectivity();

            let exit_id = engine.id();
            let exit_route = vec![terminal, ground];
            let exit_path = engine
                .platform_exit_path(&exit_route, at_station, 1, exit_id)
                .expect("canonical platform exit path");
            let exit = crate::Traveler {
                id: exit_id,
                kind: 0,
                route: exit_route,
                route_lengths: vec![40.0],
                edge_index: 0,
                edge_progress: 0.0,
                speed: 1.0,
                destination: ground,
                activity: "test_rail_exit".into(),
                resident: 0,
                name: "Rail exit test".into(),
                origin_building: None,
                destination_building: None,
                wait_ticks: 0,
                origin_access: Some(exit_path.clone()),
                destination_access: None,
                access_phase: 0,
                access_progress: 0.0,
            };
            assert_eq!(
                engine.access_movement_support(&exit, &exit_path),
                Some((vec![local_road, rail_id], vec![terminal]))
            );
            let mut perturbed_exit = exit_path.clone();
            perturbed_exit.points[0].x += 0.01;
            assert_eq!(engine.access_movement_support(&exit, &perturbed_exit), None);

            let entry_id = engine.id();
            let entry_route = vec![ground, terminal];
            let entry_path = engine
                .platform_entry_path(&entry_route, at_station, 1, entry_id)
                .expect("canonical platform entry path");
            let entry = crate::Traveler {
                id: entry_id,
                kind: 0,
                route: entry_route,
                route_lengths: vec![40.0],
                edge_index: 0,
                edge_progress: 0.0,
                speed: 1.0,
                destination: terminal,
                activity: "test_rail_entry".into(),
                resident: 0,
                name: "Rail entry test".into(),
                origin_building: None,
                destination_building: None,
                wait_ticks: 0,
                origin_access: None,
                destination_access: Some(entry_path.clone()),
                access_phase: 2,
                access_progress: 0.0,
            };
            assert_eq!(
                engine.access_movement_support(&entry, &entry_path),
                Some((vec![local_road, rail_id], vec![terminal]))
            );
            let mut perturbed_entry = entry_path.clone();
            perturbed_entry.points[0].x += 0.01;
            assert_eq!(
                engine.access_movement_support(&entry, &perturbed_entry),
                None
            );
        }
    }

    #[test]
    fn extra_open_rail_branch_does_not_ambiguate_waiting_platform_support() {
        let mut engine = Engine::new_legacy(48);
        engine.state.station_online = true;
        let (from, to, service_rail_id) = {
            let rail = engine
                .state
                .roads
                .iter_mut()
                .find(|road| road.kind == crate::RoadKind::Rail)
                .expect("legacy fixture has rail service");
            rail.stage = crate::Stage::Open;
            rail.progress = 1.0;
            rail.connected = true;
            (rail.from, rail.to, rail.id)
        };
        let station = [from, to]
            .into_iter()
            .find(|id| {
                engine
                    .node(*id)
                    .is_some_and(|node| node.kind == crate::NodeKind::Station)
            })
            .expect("service has station terminal");
        let gateway = if station == from { to } else { from };
        let terminal = engine.node(station).unwrap().clone();
        let branch_node = engine.id();
        engine.state.nodes.push(crate::Node {
            id: branch_node,
            x: terminal.x + 30.0,
            z: terminal.z + 30.0,
            kind: crate::NodeKind::Junction,
        });
        let branch_road_id = engine.id();
        engine.state.roads.push(crate::Road {
            id: branch_road_id,
            from: station,
            to: branch_node,
            kind: crate::RoadKind::Rail,
            stage: crate::Stage::Open,
            progress: 1.0,
            connected: true,
            length: 42.0,
            lanes: 2,
        });
        let resident_id = engine.id();
        let building = engine.state.buildings[0].id;
        engine.state.residents.push(crate::Resident {
            id: resident_id,
            name: "Platform branch test".into(),
            home_building: building,
            work_building: None,
            home_node: station,
            work_node: None,
            mode: 0,
            routine: "worker".into(),
            activity: "waiting".into(),
            at_work: false,
            departure_tick: 0,
            return_tick: 0,
            last_departure_day: u32::MAX,
            last_return_day: u32::MAX,
            history: Vec::new(),
            rail_commuter: true,
            transfer: "waiting_station".into(),
            train_id: None,
            queue_slot: Some(1),
            bus_id: None,
            bus_origin_stop: None,
            bus_destination_stop: None,
            bus_stop_node: None,
            pending_alight: None,
        });

        let packet = engine.movement_packet();
        let mut cursor = 2;
        let mut found = None;
        for _ in 0..packet[1] {
            let row = &packet[cursor..cursor + 9];
            cursor += 9;
            let road_count = row[7] as usize;
            let platform_count = row[8] as usize;
            if row[0] == resident_id {
                found = Some((
                    row[6],
                    packet[cursor..cursor + road_count].to_vec(),
                    packet[cursor + road_count..cursor + road_count + platform_count].to_vec(),
                ));
            }
            cursor += road_count + platform_count;
        }
        let (resolved, road_ids, platform_ids) = found.expect("waiting resident has movement row");
        assert_eq!(resolved, 1);
        assert_eq!(road_ids, vec![service_rail_id]);
        assert!(!road_ids.contains(&branch_road_id));
        assert_eq!(platform_ids, vec![station]);
        assert_ne!(station, gateway);
    }
}
