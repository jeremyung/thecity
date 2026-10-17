use crate::*;

impl Engine {
    pub fn movement_buffer(&self) -> Vec<f32> {
        self.movement_rows(false)
            .into_iter()
            .flat_map(|row| {
                [
                    row.actor_id as f32,
                    row.x,
                    row.z,
                    row.heading,
                    row.kind as f32,
                    row.surface_y,
                ]
            })
            .collect()
    }

    /// Versioned movement poses and their same-traversal infrastructure support.
    pub fn movement_packet(&self) -> Vec<u32> {
        persistence::movement_support::Row::encode(&self.movement_rows(true))
    }

    pub(crate) fn movement_rows(
        &self,
        include_support: bool,
    ) -> Vec<persistence::movement_support::Row> {
        let mut out = Vec::with_capacity(
            self.state.people.travelers.len()
                + self.state.transit.expressway_traffic.trips.len()
                + self.state.transit.trains.len()
                + self.state.transit.buses.len()
                + self.state.people.residents.len(),
        );
        let station = self.station_node();
        for t in &self.state.people.travelers {
            if t.access_phase == 0 || t.access_phase == 2 {
                // A car still at the start of its driveway has not pulled out:
                // it waits off-screen behind the car ahead (see driveway_queues).
                if t.kind == 1 && t.access_phase == 0 && t.access_progress <= 0.0 {
                    continue;
                }
                let path = if t.access_phase == 0 {
                    t.origin_access.as_ref()
                } else {
                    t.destination_access.as_ref()
                };
                if let Some((point, heading, surface_y)) =
                    path.and_then(|path| path.sample_surface(t.access_progress))
                {
                    let support = if include_support {
                        path.and_then(|path| self.access_movement_support(t, path))
                    } else {
                        None
                    };
                    let resolved = support.is_some();
                    let (road_ids, platform_node_ids) = support.unwrap_or_default();
                    out.push(persistence::movement_support::Row {
                        actor_id: t.id,
                        x: point.x,
                        z: point.z,
                        heading,
                        kind: Self::vehicle_body(t, station) as u32,
                        surface_y,
                        resolved,
                        road_ids,
                        platform_node_ids,
                    });
                }
                continue;
            }
            if t.edge_index + 1 >= t.route.len() {
                continue;
            }
            let Some(a) = self.node(t.route[t.edge_index]) else {
                continue;
            };
            let Some(b) = self.node(t.route[t.edge_index + 1]) else {
                continue;
            };
            let len = distance(a, b).max(1.0);
            let f = (t.edge_progress / len).clamp(0.0, 1.0);
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let mut x = a.x + dx * f;
            let mut z = a.z + dz * f;
            let lateral = self.road_lateral_offset(t, a.id, b.id, dx, dz, len);
            x += lateral.0;
            z += lateral.1;
            let heading = (b.z - a.z).atan2(b.x - a.x);
            let road_ids = if include_support {
                self.state
                    .network
                    .roads
                    .iter()
                    .find(|r| {
                        r.stage == Stage::Open
                            && r.connected
                            && r.kind != RoadKind::Rail
                            && ((r.from == a.id && r.to == b.id)
                                || (r.from == b.id && r.to == a.id))
                    })
                    .map(|r| vec![r.id])
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            out.push(persistence::movement_support::Row {
                actor_id: t.id,
                x,
                z,
                heading,
                kind: Self::vehicle_body(t, station) as u32,
                surface_y: -1.0,
                resolved: road_ids.len() == 1,
                road_ids,
                platform_node_ids: Vec::new(),
            });
        }
        if let Some((station, gateway)) = self.rail_terminals() {
            for (at_station, node_id) in [(true, station), (false, gateway)] {
                let Some(node) = self.node(node_id) else {
                    continue;
                };
                let mut waiting: Vec<&Resident> = self
                    .state
                    .people
                    .residents
                    .iter()
                    .filter(|resident| {
                        resident.transfer
                            == if at_station {
                                Transfer::WaitingStation
                            } else {
                                Transfer::WaitingGateway
                            }
                    })
                    .collect();
                waiting.sort_by_key(|resident| resident.id);
                for (slot, resident) in waiting.into_iter().enumerate() {
                    let slot = resident.queue_slot.unwrap_or(slot as u16);
                    let position = self
                        .platform_position(at_station, slot)
                        .unwrap_or(WorldPoint {
                            x: node.x,
                            z: node.z,
                        });
                    let rail_ids: Vec<u32> = if include_support {
                        self.platform_support_road(node_id).into_iter().collect()
                    } else {
                        Vec::new()
                    };
                    out.push(persistence::movement_support::Row {
                        actor_id: resident.id,
                        x: position.x,
                        z: position.z,
                        heading: 0.0,
                        kind: 0,
                        surface_y: 8.0,
                        resolved: rail_ids.len() == 1,
                        road_ids: rail_ids,
                        platform_node_ids: vec![node_id],
                    });
                }
            }
        }
        let mut bus_waiting: Vec<&Resident> = self
            .state
            .people
            .residents
            .iter()
            .filter(|resident| resident.transfer == Transfer::WaitingBus)
            .collect();
        bus_waiting.sort_by_key(|resident| resident.id);
        for resident in bus_waiting {
            let Some(stop) = resident.bus_stop_node else {
                continue;
            };
            let Some(slot) = resident.queue_slot else {
                continue;
            };
            let direction = self.bus_trip_direction(resident).unwrap_or(1);
            let Some((position, edge)) = self.bus_queue_position_with_edge(stop, direction, slot)
            else {
                continue;
            };
            let road_ids: Vec<u32> = if include_support {
                self.state
                    .network
                    .roads
                    .iter()
                    .find(|r| {
                        r.stage == Stage::Open
                            && r.connected
                            && r.kind != RoadKind::Rail
                            && ((r.from == edge[0] && r.to == edge[1])
                                || (r.from == edge[1] && r.to == edge[0]))
                    })
                    .map(|r| r.id)
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            };
            out.push(persistence::movement_support::Row {
                actor_id: resident.id,
                x: position.x,
                z: position.z,
                heading: 0.0,
                kind: 0,
                surface_y: -1.0,
                resolved: road_ids.len() == 1,
                road_ids,
                platform_node_ids: Vec::new(),
            });
        }
        let mut pending_alights: Vec<&Resident> = self
            .state
            .people
            .residents
            .iter()
            .filter(|resident| resident.transfer == Transfer::WaitingAlight)
            .collect();
        pending_alights.sort_by_key(|resident| resident.id);
        for resident in pending_alights {
            let Some(pending) = &resident.pending_alight else {
                continue;
            };
            let surface_y = if pending.transfer_after_route == Transfer::WalkingHome
                && self.rail_terminals().is_some_and(|(station, gateway)| {
                    pending.stop_node == station || pending.stop_node == gateway
                }) {
                8.0
            } else {
                -1.0
            };
            let rail_ids: Vec<u32> = if include_support {
                self.platform_support_road(pending.stop_node)
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            };
            let resolved = surface_y == 8.0 && rail_ids.len() == 1;
            out.push(persistence::movement_support::Row {
                actor_id: resident.id,
                x: pending.wait_position.x,
                z: pending.wait_position.z,
                heading: 0.0,
                kind: 0,
                surface_y,
                resolved,
                road_ids: rail_ids,
                platform_node_ids: if resolved {
                    vec![pending.stop_node]
                } else {
                    Vec::new()
                },
            });
        }
        for t in &self.state.transit.trains {
            if t.route.len() < 2 {
                continue;
            }
            let Some((point, tangent)) = self.rail_pose(t.route[0], t.route[1], t.distance) else {
                continue;
            };
            let forward_heading = tangent.z.atan2(tangent.x);
            let heading = if t.direction >= 0 {
                forward_heading
            } else {
                forward_heading + std::f32::consts::PI
            };
            let x = point.x;
            let z = point.z;
            let road_ids: Vec<u32> = if include_support {
                t.route
                    .windows(2)
                    .filter_map(|edge| {
                        self.state
                            .network
                            .roads
                            .iter()
                            .find(|r| {
                                r.kind == RoadKind::Rail
                                    && r.stage == Stage::Open
                                    && r.connected
                                    && ((r.from == edge[0] && r.to == edge[1])
                                        || (r.from == edge[1] && r.to == edge[0]))
                            })
                            .map(|r| r.id)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            out.push(persistence::movement_support::Row {
                actor_id: t.id,
                x,
                z,
                heading,
                kind: 2,
                surface_y: -1.0,
                resolved: road_ids.len() == t.route.len() - 1,
                road_ids,
                platform_node_ids: Vec::new(),
            });
        }
        if let Some(line) = &self.state.transit.bus_line {
            for bus in &self.state.transit.buses {
                let Some(((x, z, heading), edge)) = self.bus_pose_with_edge(line, bus) else {
                    continue;
                };
                let road_ids: Vec<u32> = if include_support {
                    self.state
                        .network
                        .roads
                        .iter()
                        .find(|r| {
                            r.stage == Stage::Open
                                && r.connected
                                && r.kind != RoadKind::Rail
                                && ((r.from == edge[0] && r.to == edge[1])
                                    || (r.from == edge[1] && r.to == edge[0]))
                        })
                        .map(|r| r.id)
                        .into_iter()
                        .collect()
                } else {
                    Vec::new()
                };
                out.push(persistence::movement_support::Row {
                    actor_id: bus.id,
                    x,
                    z,
                    heading,
                    kind: 3,
                    surface_y: -1.0,
                    resolved: road_ids.len() == 1,
                    road_ids,
                    platform_node_ids: Vec::new(),
                });
            }
        }
        // Expressway route provenance uses plan-road indices; it is resolved in
        // the dedicated helper where the same sampled pose is emitted.
        self.append_expressway_movement_rows(&mut out, include_support);
        out
    }
}
