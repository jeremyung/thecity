use crate::*;

pub(crate) const MAX_PENDING_ALIGHT_WAITERS: usize = 256;
const PENDING_ALIGHT_WORK_PER_TICK: usize = 8;
const PENDING_ALIGHT_RETRY_TICKS: u64 = 40;
const PENDING_ALIGHT_FALLBACK_TICKS: u64 = 1_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PendingAlight {
    pub stop_node: u32,
    pub destination_node: u32,
    pub destination_building: u32,
    /// Traveler activity used when the intended walk resumes.
    pub activity: TravelerActivity,
    /// The passenger's actual place in the stop/platform queue.
    pub wait_position: WorldPoint,
    #[serde(default)]
    pub last_checked_geometry_revision: Option<u64>,
    #[serde(default)]
    pub retry_after_tick: u64,
    /// `walking_from_bus` or `walking_home`, restored when a legal route exists.
    pub transfer_after_route: Transfer,
}

impl Engine {
    /// Called before removing a passenger from its vehicle. False means the
    /// bounded waiting area is full and vehicle occupancy must remain intact.
    pub(crate) fn queue_pending_alight(
        &mut self,
        resident_id: u32,
        stop_node: u32,
        destination_node: u32,
        destination_building: u32,
        activity: TravelerActivity,
        transfer_after_route: Transfer,
        wait_position: WorldPoint,
        route_checked: bool,
    ) -> bool {
        if self.state.transit.pending_alight_ids.len() >= MAX_PENDING_ALIGHT_WAITERS {
            return false;
        }
        let geometry_revision = self.access_geometry_revision_token();
        let tick = self.state.tick;
        let Some(resident) = self
            .state
            .people
            .residents
            .iter_mut()
            .find(|resident| resident.id == resident_id)
        else {
            return false;
        };
        if resident.pending_alight.is_some() {
            return false;
        }
        resident.pending_alight = Some(PendingAlight {
            stop_node,
            destination_node,
            destination_building,
            activity,
            wait_position,
            last_checked_geometry_revision: route_checked.then_some(geometry_revision).flatten(),
            retry_after_tick: tick.saturating_add(if route_checked {
                PENDING_ALIGHT_FALLBACK_TICKS
            } else {
                PENDING_ALIGHT_RETRY_TICKS
            }),
            transfer_after_route,
        });
        resident.transfer = Transfer::WaitingAlight;
        resident.activity = ResidentActivity::WaitingForRoute;
        resident.bus_id = None;
        resident.train_id = None;
        resident.bus_stop_node = Some(stop_node);
        self.state.transit.pending_alight_ids.push(resident_id);
        true
    }

    pub(crate) fn advance_pending_alights(&mut self) {
        let examined = self
            .state
            .transit
            .pending_alight_ids
            .len()
            .min(PENDING_ALIGHT_WORK_PER_TICK);
        for _ in 0..examined {
            if self.state.transit.pending_alight_ids.is_empty() {
                self.state.transit.pending_alight_cursor = 0;
                break;
            }
            let index = self.state.transit.pending_alight_cursor
                % self.state.transit.pending_alight_ids.len();
            let resident_id = self.state.transit.pending_alight_ids[index];
            self.state.transit.pending_alight_cursor =
                (index + 1) % self.state.transit.pending_alight_ids.len();
            let Some(resident_index) = self
                .state
                .people
                .residents
                .iter()
                .position(|resident| resident.id == resident_id)
            else {
                self.remove_pending_alight(resident_id);
                continue;
            };
            let Some(pending) = self.state.people.residents[resident_index]
                .pending_alight
                .clone()
            else {
                self.remove_pending_alight(resident_id);
                continue;
            };
            let revision = self.access_geometry_revision_token();
            if let Some((destination_node, destination_building)) =
                self.current_pending_destination(resident_index, &pending)
            {
                if destination_node != pending.destination_node
                    || destination_building != pending.destination_building
                {
                    if let Some(current) = self.state.people.residents[resident_index]
                        .pending_alight
                        .as_mut()
                    {
                        current.destination_node = destination_node;
                        current.destination_building = destination_building;
                        current.last_checked_geometry_revision = None;
                        current.retry_after_tick = self.state.tick;
                    }
                    continue;
                }
            }
            let graph_changed = match revision {
                Some(current) => pending.last_checked_geometry_revision != Some(current),
                None => self.state.tick >= pending.retry_after_tick,
            };
            if self.state.tick < pending.retry_after_tick && !graph_changed {
                continue;
            }
            // Actor capacity is independent of road geometry. Defer without a
            // route search and check again on the bounded cadence.
            if self.state.people.travelers.len() >= MAX_TRAVELERS {
                if let Some(pending) = self.state.people.residents[resident_index]
                    .pending_alight
                    .as_mut()
                {
                    pending.retry_after_tick =
                        self.state.tick.saturating_add(PENDING_ALIGHT_RETRY_TICKS);
                }
                continue;
            }
            let destination_open = self
                .state
                .construction
                .buildings
                .iter()
                .find(|building| building.id == pending.destination_building)
                .is_some_and(|building| {
                    building.stage == Stage::Open
                        && (!is_commercial_kind(building.kind) || building.business_active)
                });
            if !destination_open
                || !self
                    .state
                    .network
                    .nodes
                    .iter()
                    .any(|node| node.id == pending.destination_node)
            {
                if let Some(current) = self.state.people.residents[resident_index]
                    .pending_alight
                    .as_mut()
                {
                    // Availability can change without a road geometry revision
                    // (for example, a building can finish construction). Keep
                    // this waiter eligible for a bounded availability check on
                    // the next scheduler visit.
                    current.last_checked_geometry_revision = None;
                    current.retry_after_tick = self
                        .state
                        .tick
                        .saturating_add(PENDING_ALIGHT_FALLBACK_TICKS);
                }
                continue;
            }
            if !graph_changed {
                if let Some(pending) = self.state.people.residents[resident_index]
                    .pending_alight
                    .as_mut()
                {
                    pending.retry_after_tick = self
                        .state
                        .tick
                        .saturating_add(PENDING_ALIGHT_FALLBACK_TICKS);
                }
                continue;
            }
            let route = self.route(pending.stop_node, pending.destination_node, false);
            if route.is_empty() || route.len() > MAX_ROUTE_NODES {
                if let Some(current) = self.state.people.residents[resident_index]
                    .pending_alight
                    .as_mut()
                {
                    current.last_checked_geometry_revision = revision;
                    current.retry_after_tick = self
                        .state
                        .tick
                        .saturating_add(PENDING_ALIGHT_FALLBACK_TICKS);
                }
                continue;
            }
            let mut traveler = Traveler {
                id: resident_id,
                kind: 0,
                route_lengths: self.route_lengths(&route),
                route,
                edge_index: 0,
                edge_progress: 0.0,
                speed: 1.5,
                destination: pending.destination_node,
                activity: pending.activity.clone(),
                resident: resident_id,
                name: self.state.people.residents[resident_index].name.clone(),
                origin_building: None,
                destination_building: Some(pending.destination_building),
                wait_ticks: 0,
                origin_access: None,
                destination_access: None,
                access_phase: 1,
                access_progress: 0.0,
            };
            let endpoint = self.route_endpoint_position(&traveler.route, true, resident_id, 0);
            if let Some(endpoint) = endpoint {
                if (pending.wait_position.x - endpoint.x)
                    .hypot(pending.wait_position.z - endpoint.z)
                    > 0.01
                {
                    let rail_terminal = (pending.transfer_after_route == Transfer::WalkingHome)
                        .then(|| self.rail_terminals())
                        .flatten()
                        .and_then(|(station, gateway)| {
                            if pending.stop_node == station {
                                Some(true)
                            } else if pending.stop_node == gateway {
                                Some(false)
                            } else {
                                None
                            }
                        });
                    traveler.origin_access = if let Some(at_station) = rail_terminal {
                        self.platform_exit_path_from_position(
                            pending.wait_position,
                            endpoint,
                            at_station,
                        )
                    } else {
                        Some(AccessPath {
                            points: vec![pending.wait_position, endpoint],
                            surface_y: Vec::new(),
                        })
                    };
                    traveler.access_phase = 0;
                }
            }
            traveler.destination_access = self.building_access_path(
                pending.destination_building,
                &traveler.route,
                false,
                resident_id,
                0,
            );
            let name = self.state.people.residents[resident_index].name.clone();
            self.state.people.residents[resident_index]
                .pending_alight
                .take();
            let resident = &mut self.state.people.residents[resident_index];
            resident.transfer = pending.transfer_after_route.clone();
            resident.activity = match pending.transfer_after_route {
                Transfer::WalkingFromBus if pending.activity == TravelerActivity::Return => {
                    ResidentActivity::WalkingHomeFromBus
                }
                Transfer::WalkingFromBus => ResidentActivity::WalkingToWorkFromBus,
                _ => ResidentActivity::WalkingHome,
            };
            push_history(
                &mut resident.history,
                format!(
                    "A route opened from stop {} at tick {}",
                    pending.stop_node, self.state.tick
                ),
            );
            self.state.people.travelers.push(traveler);
            self.remove_pending_alight(resident_id);
            self.record(
                EventKind::TransitAlightRouteResumed,
                resident_id,
                format!(
                    "{name} resumes the intended walk from stop {}",
                    pending.stop_node
                ),
            );
        }
    }

    fn remove_pending_alight(&mut self, resident_id: u32) {
        if let Some(index) = self
            .state
            .transit
            .pending_alight_ids
            .iter()
            .position(|id| *id == resident_id)
        {
            self.state.transit.pending_alight_ids.remove(index);
            if self.state.transit.pending_alight_ids.is_empty() {
                self.state.transit.pending_alight_cursor = 0;
            } else {
                self.state.transit.pending_alight_cursor %=
                    self.state.transit.pending_alight_ids.len();
            }
        }
    }

    fn current_pending_destination(
        &self,
        resident_index: usize,
        pending: &PendingAlight,
    ) -> Option<(u32, u32)> {
        let resident = self.state.people.residents.get(resident_index)?;
        Some(match pending.activity {
            TravelerActivity::Return | TravelerActivity::WalkingHome => {
                (resident.home_node, resident.home_building)
            }
            _ => (
                resident.work_node.unwrap_or(resident.home_node),
                resident.work_building.unwrap_or(resident.home_building),
            ),
        })
    }
}
