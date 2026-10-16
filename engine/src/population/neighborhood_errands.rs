//! Persisted lifecycle for bounded household errands to nearby active shops and cafes.
use serde::{Deserialize, Serialize};

const MAX_ACTIVE: usize = 128;
const MAX_HOME_OUTINGS: u32 = 4;
const MAX_PARTICIPANTS: u32 = 1;
// The calendar is compressed: keep everyday visits genuinely local.
const MAX_WALK_LEG_METERS: f32 = 180.0;
const DWELL_MIN: u64 = 250;
const DWELL_SPAN: u64 = 501;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ScanCursor {
    /// Raw building array position, advanced one slot per scheduling call.
    home_index: u64,
    /// Raw building array position, advanced eight slots per scheduling call.
    destination_index: u64,
    /// Rotation supplied to the bounded pure admission helper.
    destination_round: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Phase {
    Outbound,
    Dwelling,
    Returning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Outing {
    home_id: u32,
    destination_id: u32,
    traveler_id: u32,
    participants: u32,
    phase: Phase,
    dwell_until: u64,
    dwell_ticks: u64,
    return_traveler: crate::Traveler,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ErrandState {
    cursor: ScanCursor,
    outings: Vec<Outing>,
}

impl ErrandState {
    pub(crate) fn is_empty(&self) -> bool {
        self.outings.is_empty()
    }
}

impl crate::Engine {
    fn building_under_extension(&self, building_id: u32) -> bool {
        let parcel_id = self
            .state
            .buildings
            .iter()
            .find(|b| b.id == building_id)
            .map(|b| b.parcel_id);
        self.state
            .pending_rear_extensions
            .iter()
            .any(|plan| plan.building_id == building_id)
            || self
                .state
                .projects
                .iter()
                .any(|project| project.kind == "extension" && project.parcel_id == parcel_id)
    }

    pub(crate) fn schedule_neighborhood_errands(&mut self) {
        self.advance_neighborhood_errand_dwell();
        if self.state.neighborhood_errands.outings.len() >= MAX_ACTIVE
            || self.state.travelers.len() >= crate::MAX_TRAVELERS
            || self.state.buildings.is_empty()
        {
            return;
        }
        let minute = crate::clock_at(self.state.tick).1 as u16;
        if !(crate::population::neighborhood_demand::ERRAND_START_MINUTE
            ..crate::population::neighborhood_demand::ERRAND_END_MINUTE)
            .contains(&minute)
        {
            return;
        }
        let buildings_len = self.state.buildings.len();
        let cursor = self.state.neighborhood_errands.cursor;
        let hi = (cursor.home_index % buildings_len as u64) as usize;
        let home_building = &self.state.buildings[hi];
        let mut homes = Vec::with_capacity(1);
        if is_active_home(home_building) && !self.building_under_extension(home_building.id) {
            let home_id = home_building.id;
            homes.push(crate::population::neighborhood_demand::Home {
                id: home_id,
                x: home_building.x,
                z: home_building.z,
                occupied_people: home_building.occupancy as u32,
                named_residents: self
                    .state
                    .residents
                    .iter()
                    .filter(|r| r.home_building == home_id)
                    .count() as u32,
                active_aggregate_participants: self.aggregate_participants_at_home(home_id),
                active_outings: self
                    .state
                    .neighborhood_errands
                    .outings
                    .iter()
                    .filter(|o| o.home_id == home_id)
                    .count() as u32,
                outing_quota: MAX_HOME_OUTINGS,
            });
        }

        let dest_start = (cursor.destination_index % buildings_len as u64) as usize;
        let mut destinations =
            Vec::with_capacity(crate::population::neighborhood_demand::MAX_DESTINATION_CHECKS);
        let destination_slots =
            buildings_len.min(crate::population::neighborhood_demand::MAX_DESTINATION_CHECKS);
        let mut raw_offset = 0usize;
        while raw_offset < destination_slots {
            let b = &self.state.buildings[(dest_start + raw_offset) % buildings_len];
            if is_active_business(b) && !self.building_under_extension(b.id) {
                let active_visitor_count = self
                    .state
                    .travelers
                    .iter()
                    .filter(|t| {
                        t.destination_building == Some(b.id)
                            && !t.activity.starts_with("neighborhood_errand_")
                    })
                    .count() as u32;
                let reserved = self
                    .state
                    .neighborhood_errands
                    .outings
                    .iter()
                    .filter(|o| o.destination_id == b.id && o.phase != Phase::Returning)
                    .map(|o| o.participants)
                    .sum();
                destinations.push(crate::population::neighborhood_demand::Destination {
                    id: b.id,
                    x: b.x,
                    z: b.z,
                    open: true,
                    visitor_capacity: b.capacity as u32,
                    current_visitors: active_visitor_count,
                    reserved_visitors: reserved,
                });
            }
            raw_offset += 1;
        }
        destinations.sort_by_key(|d| d.id);
        let next_home = (hi + 1) % buildings_len;
        let next_destination =
            (dest_start + destination_slots + usize::from(next_home == 0)) % buildings_len;
        let next_round = if next_home == 0 {
            cursor.destination_round.wrapping_add(1)
        } else {
            cursor.destination_round
        };
        self.state.neighborhood_errands.cursor = ScanCursor {
            home_index: next_home as u64,
            destination_index: next_destination as u64,
            destination_round: next_round,
        };
        let result = crate::population::neighborhood_demand::scan(
            &homes,
            &destinations,
            minute,
            self.state.seed as u64,
            self.state.neighborhood_errands.outings.len() as u32,
            crate::population::neighborhood_demand::ScanCursor {
                home_index: 0,
                destination_round: cursor.destination_round,
            },
        );
        let Some(proposal) = result.proposal else {
            return;
        };
        self.admit_errand(proposal.home_id, proposal.destination_id);
    }

    fn admit_errand(&mut self, home_id: u32, destination_id: u32) {
        if self.state.travelers.len() >= crate::MAX_TRAVELERS
            || self.state.neighborhood_errands.outings.len() >= MAX_ACTIVE
        {
            return;
        }
        let Some(home) = self
            .state
            .buildings
            .iter()
            .find(|b| b.id == home_id)
            .cloned()
        else {
            return;
        };
        let Some(destination) = self
            .state
            .buildings
            .iter()
            .find(|b| b.id == destination_id)
            .cloned()
        else {
            return;
        };
        let Some(home_parcel) = self.state.parcels.iter().find(|p| p.id == home.parcel_id) else {
            return;
        };
        let Some(destination_parcel) = self
            .state
            .parcels
            .iter()
            .find(|p| p.id == destination.parcel_id)
        else {
            return;
        };
        let route = self.route(
            home_parcel.access_node,
            destination_parcel.access_node,
            false,
        );
        if route.len() < 2 {
            return;
        }
        let id = self.state.next_id;
        let mut outbound = make_traveler(
            id,
            &route,
            self,
            home_id,
            destination_id,
            "neighborhood_errand_outbound",
        );
        outbound.origin_access = self.building_access_path(home_id, &route, true, id, 0);
        outbound.destination_access =
            self.building_access_path(destination_id, &route, false, id, 0);
        if outbound.origin_access.is_none() || outbound.destination_access.is_none() {
            return;
        }
        let walk_distance = outbound.route_lengths.iter().sum::<f32>()
            + outbound.origin_access.as_ref().unwrap().length()
            + outbound.destination_access.as_ref().unwrap().length();
        if walk_distance > MAX_WALK_LEG_METERS {
            return;
        }
        let returning_route = route.iter().rev().copied().collect::<Vec<_>>();
        let mut returning = make_traveler(
            id,
            &returning_route,
            self,
            destination_id,
            home_id,
            "neighborhood_errand_returning",
        );
        returning.origin_access =
            self.building_access_path(destination_id, &returning.route, true, id, 0);
        returning.destination_access =
            self.building_access_path(home_id, &returning.route, false, id, 0);
        if returning.origin_access.is_none() || returning.destination_access.is_none() {
            return;
        }
        let admitted_id = self.id();
        debug_assert_eq!(admitted_id, id);
        let dwell_ticks = DWELL_MIN
            + mix(self.state.seed as u64 ^ (id as u64).rotate_left(21) ^ self.state.tick)
                % DWELL_SPAN;
        self.state.neighborhood_errands.outings.push(Outing {
            home_id,
            destination_id,
            traveler_id: id,
            participants: 1,
            phase: Phase::Outbound,
            dwell_until: 0,
            dwell_ticks,
            return_traveler: returning,
        });
        self.state.travelers.push(outbound);
    }

    pub(crate) fn handle_neighborhood_errand_arrival(
        &mut self,
        traveler: &crate::Traveler,
    ) -> bool {
        let Some(index) = self
            .state
            .neighborhood_errands
            .outings
            .iter()
            .position(|o| o.traveler_id == traveler.id)
        else {
            return false;
        };
        let outing = &mut self.state.neighborhood_errands.outings[index];
        match outing.phase {
            Phase::Outbound if traveler.activity == "neighborhood_errand_outbound" => {
                outing.phase = Phase::Dwelling;
                outing.dwell_until = self.state.tick.saturating_add(outing.dwell_ticks);
                true
            }
            Phase::Returning if traveler.activity == "neighborhood_errand_returning" => {
                self.state.neighborhood_errands.outings.remove(index);
                true
            }
            _ => false,
        }
    }

    fn advance_neighborhood_errand_dwell(&mut self) {
        let room = crate::MAX_TRAVELERS.saturating_sub(self.state.travelers.len());
        if room == 0 {
            return;
        }
        for outing in self
            .state
            .neighborhood_errands
            .outings
            .iter_mut()
            .filter(|o| o.phase == Phase::Dwelling && self.state.tick >= o.dwell_until)
            .take(room)
        {
            outing.phase = Phase::Returning;
            self.state.travelers.push(outing.return_traveler.clone());
        }
    }

    pub(crate) fn errand_references_building(&self, id: u32) -> bool {
        self.state
            .neighborhood_errands
            .outings
            .iter()
            .any(|o| o.home_id == id || o.destination_id == id)
    }

    pub(crate) fn errand_references_road_edge(&self, from: u32, to: u32) -> bool {
        self.state
            .neighborhood_errands
            .outings
            .iter()
            .any(|outing| {
                let route = &outing.return_traveler.route;
                route.windows(2).any(|edge| {
                    (edge[0] == from && edge[1] == to) || (edge[0] == to && edge[1] == from)
                })
            })
    }

    pub(crate) fn aggregate_participants_at_home(&self, id: u32) -> u32 {
        self.state
            .neighborhood_errands
            .outings
            .iter()
            .filter(|o| o.home_id == id)
            .map(|o| o.participants)
            .sum()
    }

    pub(crate) fn validate_neighborhood_errands(&self) -> Result<(), String> {
        let state = &self.state.neighborhood_errands;
        if state.outings.len() > MAX_ACTIVE {
            return Err("too many active neighborhood errands".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for outing in &state.outings {
            if !ids.insert(outing.traveler_id)
                || outing.traveler_id == 0
                || outing.traveler_id >= self.state.next_id
                || outing.participants != MAX_PARTICIPANTS
                || !(DWELL_MIN..DWELL_MIN + DWELL_SPAN).contains(&outing.dwell_ticks)
                || self
                    .state
                    .residents
                    .iter()
                    .any(|r| r.id == outing.traveler_id)
            {
                return Err("invalid neighborhood errand identity/accounting".into());
            }
            let home = self
                .state
                .buildings
                .iter()
                .find(|b| b.id == outing.home_id && is_active_home(b))
                .ok_or("errand home is missing or inactive")?;
            let dest = self
                .state
                .buildings
                .iter()
                .find(|b| b.id == outing.destination_id && is_active_business(b))
                .ok_or("errand business is missing or inactive")?;
            let home_node = self
                .state
                .parcels
                .iter()
                .find(|p| p.id == home.parcel_id)
                .map(|p| p.access_node)
                .ok_or("errand home parcel missing")?;
            let dest_node = self
                .state
                .parcels
                .iter()
                .find(|p| p.id == dest.parcel_id)
                .map(|p| p.access_node)
                .ok_or("errand destination parcel missing")?;
            let t = &outing.return_traveler;
            if t.id != outing.traveler_id
                || t.activity != "neighborhood_errand_returning"
                || t.resident != 0
                || t.origin_building != Some(outing.destination_id)
                || t.destination_building != Some(outing.home_id)
                || t.kind != 0
                || t.speed != 1.2
                || t.edge_index != 0
                || t.edge_progress != 0.0
                || t.access_progress != 0.0
                || t.wait_ticks != 0
                || t.route.first().copied() != Some(dest_node)
                || t.route.last().copied() != Some(home_node)
                || t.destination != home_node
                || t.route.len() < 2
                || t.route_lengths.len() + 1 != t.route.len()
                || t.origin_access
                    .as_ref()
                    .map_or(true, |p| p.points.len() < 2)
                || t.destination_access
                    .as_ref()
                    .map_or(true, |p| p.points.len() < 2)
                || t.access_phase != 0
                || !self.errand_route_valid(&t.route, &t.route_lengths)
                || !self.access_paths_match(
                    t.origin_access.as_ref().unwrap(),
                    self.building_access_path(outing.destination_id, &t.route, true, t.id, 0)
                        .as_ref(),
                )
                || !self.access_paths_match(
                    t.destination_access.as_ref().unwrap(),
                    self.building_access_path(outing.home_id, &t.route, false, t.id, 0)
                        .as_ref(),
                )
            {
                return Err("invalid neighborhood errand return route or access plan".into());
            }
            let named = self
                .state
                .residents
                .iter()
                .filter(|r| r.home_building == outing.home_id)
                .count() as u32;
            if named.saturating_add(self.aggregate_participants_at_home(outing.home_id))
                > home.occupancy as u32
            {
                return Err("neighborhood errand participants exceed home occupancy".into());
            }
            let travelers: Vec<_> = self
                .state
                .travelers
                .iter()
                .filter(|t| t.id == outing.traveler_id)
                .collect();
            match outing.phase {
                Phase::Outbound
                    if travelers.len() != 1
                        || travelers[0].activity != "neighborhood_errand_outbound"
                        || travelers[0].origin_building != Some(outing.home_id)
                        || travelers[0].destination_building != Some(outing.destination_id)
                        || travelers[0].resident != 0
                        || travelers[0].kind != 0
                        || travelers[0].access_phase > 2
                        || travelers[0].destination != dest_node
                        || travelers[0]
                            .route
                            .iter()
                            .rev()
                            .copied()
                            .ne(t.route.iter().copied())
                        || travelers[0]
                            .route_lengths
                            .iter()
                            .rev()
                            .copied()
                            .ne(t.route_lengths.iter().copied())
                        || !self.errand_route_valid(
                            &travelers[0].route,
                            &travelers[0].route_lengths,
                        )
                        || travelers[0].origin_access.as_ref().map_or(true, |p| {
                            !self.access_paths_match(
                                p,
                                self.building_access_path(
                                    outing.home_id,
                                    &travelers[0].route,
                                    true,
                                    travelers[0].id,
                                    0,
                                )
                                .as_ref(),
                            )
                        })
                        || travelers[0].destination_access.as_ref().map_or(true, |p| {
                            !self.access_paths_match(
                                p,
                                self.building_access_path(
                                    outing.destination_id,
                                    &travelers[0].route,
                                    false,
                                    travelers[0].id,
                                    0,
                                )
                                .as_ref(),
                            )
                        }) =>
                {
                    return Err("outbound errand traveler missing or mismatched".into())
                }
                Phase::Dwelling if !travelers.is_empty() || outing.dwell_until == 0 => {
                    return Err("dwelling errand accounting mismatch".into())
                }
                Phase::Returning
                    if travelers.len() != 1
                        || travelers[0].activity != "neighborhood_errand_returning"
                        || travelers[0].origin_building != Some(outing.destination_id)
                        || travelers[0].destination_building != Some(outing.home_id)
                        || travelers[0].resident != 0
                        || travelers[0].kind != 0
                        || travelers[0].access_phase > 2
                        || travelers[0].route != t.route
                        || travelers[0].route_lengths != t.route_lengths
                        || travelers[0].origin_access.as_ref().map_or(true, |p| {
                            !self.access_paths_match(p, t.origin_access.as_ref())
                        })
                        || travelers[0].destination_access.as_ref().map_or(true, |p| {
                            !self.access_paths_match(p, t.destination_access.as_ref())
                        }) =>
                {
                    return Err("returning errand traveler missing or mismatched".into())
                }
                _ => {}
            }
        }
        for traveler in self
            .state
            .travelers
            .iter()
            .filter(|t| t.activity.starts_with("neighborhood_errand_"))
        {
            if !ids.contains(&traveler.id) {
                return Err("orphan neighborhood errand traveler".into());
            }
        }
        for dest in self
            .state
            .buildings
            .iter()
            .filter(|b| is_active_business(b))
        {
            let reserved: u32 = state
                .outings
                .iter()
                .filter(|o| o.destination_id == dest.id && o.phase != Phase::Returning)
                .map(|o| o.participants)
                .sum();
            if reserved > dest.capacity as u32 {
                return Err("neighborhood errand reservations exceed visitor capacity".into());
            }
        }
        Ok(())
    }

    fn errand_route_valid(&self, route: &[u32], lengths: &[f32]) -> bool {
        if route.len() < 2 || lengths.len() + 1 != route.len() {
            return false;
        }
        route.windows(2).zip(lengths).all(|(edge, length)| {
            self.state.roads.iter().any(|road| {
                road.connected
                    && road.stage == "open"
                    && road.kind != "rail"
                    && ((road.from == edge[0] && road.to == edge[1])
                        || (road.from == edge[1] && road.to == edge[0]))
            }) && self
                .node(edge[0])
                .zip(self.node(edge[1]))
                .is_some_and(|(a, b)| {
                    length.is_finite() && (*length - crate::distance(a, b).max(1.0)).abs() < 0.01
                })
        })
    }

    fn access_paths_match(
        &self,
        stored: &crate::AccessPath,
        regenerated: Option<&crate::AccessPath>,
    ) -> bool {
        let Some(expected) = regenerated else {
            return false;
        };
        let shaped = |p: &crate::AccessPath| {
            p.points.len() >= 2
                && (p.surface_y.is_empty() || p.surface_y.len() == p.points.len())
                && p.points.iter().all(|p| p.x.is_finite() && p.z.is_finite())
                && p.surface_y.iter().all(|y| y.is_finite())
        };
        shaped(stored)
            && shaped(expected)
            && stored.points.len() == expected.points.len()
            && stored
                .points
                .iter()
                .zip(&expected.points)
                .all(|(a, b)| (a.x - b.x).hypot(a.z - b.z) < 0.01)
            && (stored.surface_y.is_empty() || stored.surface_y == expected.surface_y)
    }
}

fn make_traveler(
    id: u32,
    route: &[u32],
    engine: &crate::Engine,
    origin: u32,
    destination: u32,
    activity: &str,
) -> crate::Traveler {
    crate::Traveler {
        id,
        kind: 0,
        route: route.to_vec(),
        route_lengths: engine.route_lengths(route),
        edge_index: 0,
        edge_progress: 0.0,
        speed: 1.2,
        destination: route.last().copied().unwrap_or(0),
        activity: activity.into(),
        resident: 0,
        name: "Neighborhood visitor".into(),
        origin_building: Some(origin),
        destination_building: Some(destination),
        wait_ticks: 0,
        origin_access: None,
        destination_access: None,
        access_phase: 0,
        access_progress: 0.0,
    }
}

fn is_active_home(b: &crate::Building) -> bool {
    b.stage == "open" && matches!(b.kind.as_str(), "home" | "apartment") && b.occupancy > 0
}
fn is_active_business(b: &crate::Building) -> bool {
    b.stage == "open" && matches!(b.kind.as_str(), "shop" | "cafe") && b.business_active
}
fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errand_engine() -> crate::Engine {
        let mut engine = crate::Engine::new_legacy(731);
        engine.state.buildings.clear();
        engine.state.parcels.clear();
        engine.state.parcel_frontages.clear();
        engine.state.travelers.clear();
        engine.state.residents.clear();
        engine.state.neighborhood_errands = ErrandState::default();
        let road = engine
            .state
            .roads
            .iter()
            .filter(|r| r.connected && r.stage == "open" && r.kind != "rail")
            .min_by(|a, b| a.length.total_cmp(&b.length))
            .cloned()
            .expect("founding road");
        let a = engine.node(road.from).map(|p| (p.x, p.z)).unwrap();
        let b = engine.node(road.to).map(|p| (p.x, p.z)).unwrap();
        let dx = b.0 - a.0;
        let dz = b.1 - a.1;
        let len = dx.hypot(dz).max(1.0);
        let nx = -dz / len;
        let nz = dx / len;
        let home_parcel = engine.add_parcel(
            a.0 + nx * 14.0,
            a.1 + nz * 14.0,
            8.0,
            8.0,
            road.from,
            1.0,
            false,
        );
        engine.remember_nearest_frontage(home_parcel, road.from);
        let home = engine.add_building(
            home_parcel,
            "home",
            2,
            "open",
            1.0,
            8,
            "Home",
            false,
            Vec::new(),
        );
        engine
            .state
            .buildings
            .iter_mut()
            .find(|b| b.id == home)
            .unwrap()
            .occupancy = 1;
        let shop_parcel = engine.add_parcel(
            b.0 + nx * 14.0,
            b.1 + nz * 14.0,
            8.0,
            8.0,
            road.to,
            1.0,
            false,
        );
        engine.remember_nearest_frontage(shop_parcel, road.to);
        engine.add_building(
            shop_parcel,
            "shop",
            1,
            "open",
            1.0,
            8,
            "Shop",
            false,
            Vec::new(),
        );
        // 06:00 simulation origin plus three hours.
        engine.state.tick = crate::TICKS_PER_DAY * 3 / 24;
        engine
    }

    #[test]
    fn dwell_span_is_bounded_and_seeded() {
        let mut values = std::collections::BTreeSet::new();
        for seed in 0..5000 {
            let ticks = DWELL_MIN + mix(seed) % DWELL_SPAN;
            assert!((DWELL_MIN..DWELL_MIN + DWELL_SPAN).contains(&ticks));
            values.insert(ticks);
        }
        assert!(values.len() > 400);
    }

    #[test]
    fn outbound_dwell_return_and_save_restore_are_bijective() {
        let mut engine = errand_engine();
        engine.schedule_neighborhood_errands();
        assert_eq!(
            engine.state.neighborhood_errands.outings.len(),
            1,
            "fixture must admit one real routed errand"
        );
        engine.validate_neighborhood_errands().unwrap();
        let outbound = &engine.state.travelers[0];
        assert_eq!(outbound.access_phase, 0);
        assert!(outbound.origin_access.as_ref().unwrap().points.len() >= 2);
        assert!(outbound.destination_access.as_ref().unwrap().points.len() >= 2);
        let mut restored = crate::Engine::restore_json(&engine.save_json()).unwrap();
        let mut restored_at_dwell = false;
        let mut restored_at_return = false;
        for tick in 0..40_000 {
            advance_one_trip_tick(&mut engine);
            advance_one_trip_tick(&mut restored);
            assert_eq!(
                engine.save_json(),
                restored.save_json(),
                "live and restored states diverged at trip tick {tick}"
            );
            engine.validate_neighborhood_errands().unwrap();
            if engine
                .state
                .neighborhood_errands
                .outings
                .first()
                .is_some_and(|o| o.phase == Phase::Dwelling)
                && restored
                    .state
                    .neighborhood_errands
                    .outings
                    .first()
                    .is_some_and(|o| o.phase == Phase::Dwelling)
                && !restored_at_dwell
            {
                restored = crate::Engine::restore_json(&engine.save_json()).unwrap();
                restored_at_dwell = true;
            }
            if engine
                .state
                .neighborhood_errands
                .outings
                .first()
                .is_some_and(|o| o.phase == Phase::Returning)
                && !restored_at_return
            {
                restored = crate::Engine::restore_json(&engine.save_json()).unwrap();
                restored_at_return = true;
            }
            if engine.state.neighborhood_errands.outings.is_empty() {
                break;
            }
        }
        assert!(
            engine.state.neighborhood_errands.outings.is_empty(),
            "physical errand did not finish within the movement bound"
        );
        assert!(engine.state.travelers.is_empty());
        assert!(restored_at_dwell && restored_at_return);
        engine.validate_neighborhood_errands().unwrap();
    }

    fn advance_one_trip_tick(engine: &mut crate::Engine) {
        engine.state.tick = engine.state.tick.saturating_add(1);
        if engine.state.tick % 10 == 0 {
            engine.schedule_neighborhood_errands();
        }
        engine.advance_travelers();
        engine.advance_neighborhood_errand_dwell();
    }

    #[test]
    fn saved_errands_reject_corrupt_return_state_and_missing_ledger() {
        let mut engine = errand_engine();
        engine.schedule_neighborhood_errands();
        let original: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
        for (field, value) in [
            ("speed", serde_json::json!(0)),
            ("edge_index", serde_json::json!(1)),
            ("access_progress", serde_json::json!(999)),
        ] {
            let mut damaged = original.clone();
            damaged["neighborhood_errands"]["outings"][0]["return_traveler"][field] = value;
            assert!(
                crate::Engine::restore_json(&damaged.to_string()).is_err(),
                "accepted corrupt {field}"
            );
        }
        let mut missing = original;
        missing["neighborhood_errands"]["outings"] = serde_json::json!([]);
        assert!(crate::Engine::restore_json(&missing.to_string()).is_err());
    }

    #[test]
    fn away_participant_exhausts_single_person_residual() {
        let mut engine = errand_engine();
        engine.schedule_neighborhood_errands();
        assert_eq!(
            engine.aggregate_participants_at_home(engine.state.buildings[0].id),
            1
        );
        for _ in 0..8 {
            engine.schedule_neighborhood_errands();
        }
        assert_eq!(engine.state.neighborhood_errands.outings.len(), 1);
    }
}
