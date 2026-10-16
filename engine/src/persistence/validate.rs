use crate::*;
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_access_route_joins()?;
        self.validate_expressway_traffic()?;
        self.validate_neighborhood_errands()?;
        self.validate_masterplan_progress()?;
        legacy::access_live_engine::validate_face_history(&self.state.exhausted_access_faces)?;
        if self.state.access_geometry_revision_exhausted
            && self.state.access_geometry_revision != u64::MAX
        {
            return Err("exhausted access geometry revision is not saturated".into());
        }
        self.state.district_controller.validate(self.state.tick)?;
        let building_rotations: BTreeMap<u32, f32> = self
            .state
            .buildings
            .iter()
            .map(|building| (building.parcel_id, building.rotation))
            .collect();
        if self.state.road_infill_due > 9 {
            return Err("road infill quota exceeded its cap".into());
        }
        if self.state.frontage_road_cursor > self.state.roads.len()
            || self.state.frontage_slot_cursor as usize > MAX_FRONTAGE_SITES_PER_ROAD
            || self.state.road_origin_cursor > self.state.nodes.len()
            || self.state.neighborhood_anchor_cursor > self.state.nodes.len()
        {
            return Err("frontage scan cursor is outside its bounded range".into());
        }
        if self.state.interior_access_cursor
            > self
                .state
                .roads
                .len()
                .saturating_mul(legacy::interior_access::PROPOSALS_PER_HOST)
        {
            return Err("interior access scan cursor is outside its bounded range".into());
        }
        if self.state.rear_extension_cursor > self.state.buildings.len() {
            return Err("rear extension scan cursor is outside its bounded range".into());
        }
        self.validate_rear_extensions()?;
        self.validate_building_original_footprints()?;
        self.validate_height_redevelopment()?;
        if let Some(plan) = &self.state.pending_interior_access {
            if self.state.neighborhood_plan.is_some() {
                return Err("neighborhood and interior access plans cannot overlap".into());
            }
            if !plan.host_t.is_finite()
                || !(0.0..1.0).contains(&plan.host_t)
                || !(2..=legacy::interior_access::MAX_SELECTED_LOTS)
                    .contains(&plan.selected_lots.len())
                || self
                    .state
                    .parcels
                    .len()
                    .saturating_add(plan.selected_lots.len())
                    > MAX_PARCELS
                || self
                    .state
                    .parcel_frontages
                    .len()
                    .saturating_add(plan.selected_lots.len())
                    > MAX_PARCELS
                || self
                    .state
                    .next_id
                    .checked_add(plan.selected_lots.len() as u32)
                    .is_none()
                || self
                    .state
                    .roads
                    .len()
                    .saturating_add(self.state.projects.len())
                    > MAX_SEGMENTS
            {
                return Err(
                    "interior access plan has invalid geometry or capacity reservation".into(),
                );
            }
            let retained = self
                .state
                .roads
                .iter()
                .find(|road| road.id == plan.host_road_id)
                .ok_or_else(|| "interior access plan references a missing host half".to_string())?;
            let second = self
                .state
                .roads
                .iter()
                .find(|road| road.id == plan.host_second_half_road_id)
                .ok_or_else(|| {
                    "interior access plan references a missing second host half".to_string()
                })?;
            let branch = self
                .state
                .roads
                .iter()
                .find(|road| road.id == plan.branch_road_id)
                .ok_or_else(|| "interior access plan references a missing branch".to_string())?;
            if retained.stage != "open"
                || second.stage != "open"
                || !retained.connected
                || !second.connected
                || retained.id == second.id
                || retained.id == branch.id
                || second.id == branch.id
                || retained.to != plan.split_node_id
                || second.from != plan.split_node_id
                || retained.kind != second.kind
                || !matches!(retained.kind.as_str(), "local" | "arterial")
                || branch.from != plan.split_node_id
                || branch.to != plan.branch_endpoint_id
                || branch.kind != "local"
                || branch.stage == "open"
                || !matches!(
                    branch.stage.as_str(),
                    "survey" | "clearing" | "structure" | "surface" | "finishing"
                )
            {
                return Err("interior access plan road references are inconsistent".into());
            }
            let project = self
                .state
                .projects
                .iter()
                .find(|project| project.id == plan.project_id)
                .ok_or_else(|| "interior access plan references a missing project".to_string())?;
            if project.kind != "road"
                || project.progress >= 1.0
                || project.from != Some(branch.from)
                || project.to != Some(branch.to)
                || project.progress != branch.progress
                || project.stage != branch.stage
            {
                return Err("interior access project is not pending on its branch".into());
            }
            let original_a = self
                .node(retained.from)
                .ok_or_else(|| "interior access host origin is missing".to_string())?;
            let split = self
                .node(plan.split_node_id)
                .ok_or_else(|| "interior access split junction is missing".to_string())?;
            let original_b = self
                .node(second.to)
                .ok_or_else(|| "interior access host destination is missing".to_string())?;
            if original_a.id == split.id
                || original_a.id == original_b.id
                || split.id == original_b.id
                || plan.branch_endpoint_id == original_a.id
                || plan.branch_endpoint_id == split.id
                || plan.branch_endpoint_id == original_b.id
                || !(0.0..1.0).contains(&plan.host_t)
            {
                return Err("interior access host split has invalid ids or fraction".into());
            }
            let host_dx = original_b.x - original_a.x;
            let host_dz = original_b.z - original_a.z;
            let host_length = host_dx.hypot(host_dz);
            let expected_split = (
                original_a.x + host_dx * plan.host_t,
                original_a.z + host_dz * plan.host_t,
            );
            if !host_length.is_finite()
                || host_length <= 20.0
                || (split.x - expected_split.0).hypot(split.z - expected_split.1) > 0.01
                || (retained.length - host_length * plan.host_t).abs() > 0.01
                || (second.length - host_length * (1.0 - plan.host_t)).abs() > 0.01
            {
                return Err(
                    "interior access split does not match its original host fraction".into(),
                );
            }
            let (branch_a, branch_b) = (
                self.node(branch.from).unwrap(),
                self.node(branch.to).unwrap(),
            );
            let (dx, dz) = (branch_b.x - branch_a.x, branch_b.z - branch_a.z);
            let branch_length = dx.hypot(dz);
            if !branch_length.is_finite()
                || branch_length < 24.0 - 0.01
                || branch_length > 60.0 + 0.01
                || branch.connected
                || [branch_a.x, branch_a.z, branch_b.x, branch_b.z]
                    .iter()
                    .any(|value| *value < MIN + 24.0 || *value > MAX - 24.0)
                || !self.ordinary_road_clear_of_river(
                    branch_a.x, branch_a.z, branch_b.x, branch_b.z, 4.5,
                )
                || corridor_crosses_protected(branch_a.x, branch_a.z, branch_b.x, branch_b.z)
            {
                return Err("interior access branch has invalid length".into());
            }
            // The saved branch must meet the same minimum junction angle as a
            // live road proposal. Check every existing ordinary approach at
            // the split while excluding this plan's own pending branch project.
            let branch_heading = dz.atan2(dx);
            let junction_heading_clear = |bearing: f32| {
                let difference = (branch_heading - bearing).rem_euclid(std::f32::consts::TAU);
                difference.min(std::f32::consts::TAU - difference) >= std::f32::consts::PI / 6.0
            };
            for road in self.state.roads.iter().filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
                    && (road.from == plan.split_node_id || road.to == plan.split_node_id)
            }) {
                let neighbor_id = if road.from == plan.split_node_id {
                    road.to
                } else {
                    road.from
                };
                let neighbor = self.node(neighbor_id).ok_or_else(|| {
                    "interior access junction has a missing road endpoint".to_string()
                })?;
                let bearing = (neighbor.z - split.z).atan2(neighbor.x - split.x);
                if !junction_heading_clear(bearing) {
                    return Err("interior access branch violates junction angle clearance".into());
                }
            }
            for other in self.state.projects.iter().filter(|other| {
                other.id != plan.project_id
                    && other.kind == "road"
                    && other.progress < 1.0
                    && (other.from == Some(plan.split_node_id)
                        || other.to == Some(plan.split_node_id))
            }) {
                let neighbor_id = if other.from == Some(plan.split_node_id) {
                    other.to
                } else {
                    other.from
                };
                let neighbor = neighbor_id.and_then(|id| self.node(id)).ok_or_else(|| {
                    "interior access junction has a missing project endpoint".to_string()
                })?;
                let bearing = (neighbor.z - split.z).atan2(neighbor.x - split.x);
                if !junction_heading_clear(bearing) {
                    return Err("interior access branch violates junction angle clearance".into());
                }
            }
            if self.state.nodes.iter().any(|node| {
                node.id != plan.branch_endpoint_id
                    && (node.x - branch_b.x).hypot(node.z - branch_b.z) < 18.0
            }) {
                return Err("interior access endpoint violates junction spacing".into());
            }
            for road in self.state.roads.iter().filter(|road| {
                road.stage == "open"
                    && road.id != retained.id
                    && road.id != second.id
                    && road.id != branch.id
            }) {
                let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                    return Err("interior access branch intersects an invalid road".into());
                };
                if world::geometry::segments_distance_coords(
                    branch_a.x, branch_a.z, branch_b.x, branch_b.z, a.x, a.z, b.x, b.z,
                ) < 8.0
                {
                    return Err("interior access branch intersects an existing road".into());
                }
            }
            let (ux, uz) = (dx / branch_length, dz / branch_length);
            let (nx, nz) = (-uz, ux);
            let branch_context = self.collision_context();
            if !parcel_envelopes_clear_of_road(
                &branch_context,
                branch_a.x,
                branch_a.z,
                branch_b.x,
                branch_b.z,
            ) || self
                .state
                .projects
                .iter()
                .filter(|other| {
                    other.id != plan.project_id && other.kind == "road" && other.progress < 1.0
                })
                .any(|other| {
                    let Some((from, to)) = other.from.zip(other.to) else {
                        return false;
                    };
                    let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                        return true;
                    };
                    world::geometry::segments_distance_coords(
                        branch_a.x, branch_a.z, branch_b.x, branch_b.z, a.x, a.z, b.x, b.z,
                    ) < 8.0
                })
            {
                return Err(
                    "interior access branch overlaps reserved road or parcel geometry".into(),
                );
            }
            let mut reserved_rects = Vec::with_capacity(plan.selected_lots.len());
            let collision_context = branch_context;
            for lot in &plan.selected_lots {
                let rect = world::geometry::OrientedRect {
                    x: lot.x,
                    z: lot.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                };
                if !rect.finite_positive()
                    || !lot.along.is_finite()
                    || !(0.0..=1.0).contains(&lot.along)
                    || !matches!(lot.side, -1 | 1)
                    || !(8.0..=14.0).contains(&lot.width)
                    || (lot.depth - 12.0).abs() > 0.001
                    || ((lot.x - branch_a.x) * ux + (lot.z - branch_a.z) * uz
                        - lot.along * branch_length)
                        .abs()
                        > 0.01
                    || (((lot.x - branch_a.x) * nx + (lot.z - branch_a.z) * nz)
                        - lot.side as f32 * 15.0)
                        .abs()
                        > 0.01
                    || (lot.rotation - (lot.side as f32 * dz).atan2(-lot.side as f32 * dx)).abs()
                        > 0.001
                    || !self.oriented_parcel_geometry_clear_with_context_mode(
                        &rect,
                        &collision_context,
                        true,
                    )
                    || reserved_rects
                        .iter()
                        .any(|other: &world::geometry::OrientedRect| {
                            world::geometry::rects_overlap(&rect, other, 2.0)
                        })
                {
                    return Err("interior access lot reservation is invalid or conflicting".into());
                }
                reserved_rects.push(rect);
            }
        }
        if let Some(plan) = &self.state.neighborhood_plan {
            if plan.segments.len() != 4 || plan.next_segment > plan.segments.len() {
                return Err("neighborhood plan is outside its bounded shape".into());
            }
            if plan.coordinated_lots {
                let mut corners = [infrastructure::blocks::BlockPoint { x: 0.0, z: 0.0 }; 4];
                for index in 0..4 {
                    if plan.segments[index].1 != plan.segments[(index + 1) % 4].0 {
                        return Err("coordinated block perimeter is not closed".into());
                    }
                    let Some(node) = self.node(plan.segments[index].0) else {
                        return Err("coordinated block references a missing junction".into());
                    };
                    corners[index] = infrastructure::blocks::BlockPoint {
                        x: node.x,
                        z: node.z,
                    };
                }
                let canonical_lots =
                    infrastructure::blocks::subdivide_rectangular_block(corners, 8.0, 11.0, 2.2);
                let lot_count = if let Some(spine) = &plan.spine {
                    self.validate_neighborhood_spine(plan)?;
                    spine
                        .validate()
                        .map_err(|_| "neighborhood spine geometry is invalid")?
                        .lots
                        .len()
                } else if let Some(selected) = &plan.selected_lots {
                    if !plan.coordinated_lots
                        || !self.selected_block_layout_valid(&plan.segments, selected)
                    {
                        return Err("coordinated block selected-lot layout is invalid".into());
                    }
                    selected.len()
                } else {
                    canonical_lots.len()
                };
                if lot_count < 4 {
                    return Err("coordinated block has invalid geometry".into());
                }
                if self.state.parcels.len().saturating_add(lot_count) > MAX_PARCELS {
                    return Err("coordinated block parcel reservation exceeds capacity".into());
                }
            } else if plan.selected_lots.is_some() {
                return Err("selected block lots require a coordinated block plan".into());
            }
            if plan.spine.is_some() && !plan.coordinated_lots {
                return Err("neighborhood spine requires coordinated lots".into());
            }
            if self.node(plan.anchor_node).is_none()
                || plan
                    .segments
                    .iter()
                    .any(|(from, to)| self.node(*from).is_none() || self.node(*to).is_none())
            {
                return Err("neighborhood plan references a missing junction".into());
            }
        }
        if self.state.neighborhood_road_ids.len() > MAX_SEGMENTS
            || self
                .state
                .neighborhood_road_ids
                .iter()
                .any(|id| !self.state.roads.iter().any(|road| road.id == *id))
        {
            return Err("historic neighborhood road index is invalid or over capacity".into());
        }
        if self.state.buildings.len() > MAX_BUILDINGS {
            return Err("building cap exceeded".into());
        }
        if self.state.roads.len() > MAX_SEGMENTS {
            return Err("network segment cap exceeded".into());
        }
        if self.state.travelers.len() > MAX_TRAVELERS {
            return Err("traveler cap exceeded".into());
        }
        if self.state.residents.len() > MAX_RESIDENTS {
            return Err("resident cast cap exceeded".into());
        }
        if self.state.parcels.len() > MAX_PARCELS {
            return Err("parcel cap exceeded".into());
        }
        if self.state.parcel_frontages.len() > MAX_PARCELS {
            return Err("parcel frontage cap exceeded".into());
        }
        for frontage in &self.state.parcel_frontages {
            if !self
                .state
                .parcels
                .iter()
                .any(|parcel| parcel.id == frontage.parcel_id)
                || !self
                    .state
                    .roads
                    .iter()
                    .any(|road| road.id == frontage.road_id)
                || !frontage.along.is_finite()
                || !(0.0..=1.0).contains(&frontage.along)
                || !matches!(frontage.side, -1 | 1)
            {
                return Err(format!(
                    "parcel {} has invalid frontage metadata",
                    frontage.parcel_id
                ));
            }
        }
        if self.state.projects.len() > MAX_PROJECTS + COMPLETED_PROJECT_HISTORY {
            return Err("project history cap exceeded".into());
        }
        if self.state.population > MAX_POPULATION {
            return Err("represented population cap exceeded".into());
        }
        if self.state.pending_alight_ids.len()
            > transit::transit_recovery::MAX_PENDING_ALIGHT_WAITERS
        {
            return Err("pending transit alighting queue exceeded its cap".into());
        }
        if (self.state.pending_alight_ids.is_empty() && self.state.pending_alight_cursor != 0)
            || (!self.state.pending_alight_ids.is_empty()
                && self.state.pending_alight_cursor >= self.state.pending_alight_ids.len())
        {
            return Err("pending transit alighting cursor is invalid".into());
        }
        let mut pending_alight_ids = BTreeSet::new();
        for resident_id in &self.state.pending_alight_ids {
            if !pending_alight_ids.insert(*resident_id) {
                return Err(format!("resident {resident_id} is queued to alight twice"));
            }
            if !self.state.residents.iter().any(|resident| {
                resident.id == *resident_id
                    && resident.transfer == "waiting_alight"
                    && resident.pending_alight.is_some()
                    && resident.bus_id.is_none()
                    && resident.train_id.is_none()
            }) {
                return Err(format!(
                    "pending alight {resident_id} has no waiting resident"
                ));
            }
        }
        let mut resident_ids = BTreeSet::new();
        for resident in &self.state.residents {
            if !resident_ids.insert(resident.id) {
                return Err(format!("duplicate resident id {}", resident.id));
            }
            let home = self
                .state
                .buildings
                .iter()
                .find(|b| b.id == resident.home_building)
                .ok_or_else(|| format!("resident {} references a missing home", resident.id))?;
            if !matches!(home.kind.as_str(), "home" | "apartment") {
                return Err(format!(
                    "resident {} is assigned to a non-residential building",
                    resident.id
                ));
            }
            if !self.state.nodes.iter().any(|n| n.id == resident.home_node) {
                return Err(format!(
                    "resident {} references a missing home access node",
                    resident.id
                ));
            }
            if resident
                .work_node
                .is_some_and(|node| !self.state.nodes.iter().any(|n| n.id == node))
            {
                return Err(format!(
                    "resident {} references a missing work access node",
                    resident.id
                ));
            }
            if resident.history.len() > 8 {
                return Err(format!("resident {} history exceeded its cap", resident.id));
            }
            if let Some(pending) = &resident.pending_alight {
                if resident.transfer != "waiting_alight"
                    || resident.bus_id.is_some()
                    || resident.train_id.is_some()
                    || !pending_alight_ids.contains(&resident.id)
                    || !pending.wait_position.x.is_finite()
                    || !pending.wait_position.z.is_finite()
                    || !self
                        .state
                        .nodes
                        .iter()
                        .any(|node| node.id == pending.stop_node)
                    || !self
                        .state
                        .nodes
                        .iter()
                        .any(|node| node.id == pending.destination_node)
                    || !self
                        .state
                        .buildings
                        .iter()
                        .any(|building| building.id == pending.destination_building)
                    || !matches!(
                        pending.activity.as_str(),
                        "commute" | "return" | "walking_home"
                    )
                    || !matches!(
                        pending.transfer_after_route.as_str(),
                        "walking_from_bus" | "walking_home"
                    )
                {
                    return Err(format!(
                        "resident {} has an invalid pending alight",
                        resident.id
                    ));
                }
            } else if resident.transfer == "waiting_alight" {
                return Err(format!(
                    "resident {} is waiting without an alight target",
                    resident.id
                ));
            }
            if resident.rail_commuter
                && !matches!(
                    resident.transfer.as_str(),
                    "home"
                        | "walking_to_station"
                        | "waiting_station"
                        | "riding_outbound"
                        | "working_external"
                        | "waiting_gateway"
                        | "riding_inbound"
                        | "walking_home"
                        | "waiting_alight"
                )
            {
                return Err(format!(
                    "resident {} has an invalid rail transfer state",
                    resident.id
                ));
            }
            let riding = matches!(
                resident.transfer.as_str(),
                "riding_outbound" | "riding_inbound"
            );
            if riding != resident.train_id.is_some() {
                return Err(format!(
                    "resident {} has inconsistent train occupancy",
                    resident.id
                ));
            }
            if resident.transfer == "riding_bus" && resident.bus_id.is_none() {
                return Err(format!(
                    "resident {} is riding without a bus assignment",
                    resident.id
                ));
            }
            if matches!(
                resident.transfer.as_str(),
                "walking_to_bus" | "waiting_bus" | "riding_bus" | "walking_from_bus"
            ) && (resident.bus_origin_stop.is_none() || resident.bus_destination_stop.is_none())
            {
                return Err(format!(
                    "resident {} has an incomplete bus journey",
                    resident.id
                ));
            }
        }
        if self.rail_waiting_count(true) > RAIL_QUEUE_CAPACITY
            || self.rail_waiting_count(false) > RAIL_QUEUE_CAPACITY
        {
            return Err("rail terminal queue exceeded its capacity".into());
        }
        if self.state.buses.len() > MAX_BUSES {
            return Err("bus fleet exceeded its cap".into());
        }
        if self.bus_waiting_count() > BUS_QUEUE_CAPACITY {
            return Err("bus stop queues exceeded their shared capacity".into());
        }
        if let Some(line) = &self.state.bus_line {
            if line.stops.len() > MAX_BUS_STOPS || line.route.len() > MAX_BUS_ROUTE_NODES {
                return Err("bus line exceeded its route or stop cap".into());
            }
            if line.active && line.stops.len() < 3 {
                return Err("active bus line has fewer than three stops".into());
            }
            if line.stops.iter().any(|stop| !line.route.contains(stop)) {
                return Err("bus line contains a stop outside its route".into());
            }
            for edge in line.route.windows(2) {
                if !self.state.roads.iter().any(|road| {
                    road.stage == "open"
                        && road.connected
                        && road.kind != "rail"
                        && ((road.from == edge[0] && road.to == edge[1])
                            || (road.from == edge[1] && road.to == edge[0]))
                }) {
                    return Err("bus line uses a disconnected or non-road edge".into());
                }
            }
        }
        let mut bus_passengers = BTreeSet::new();
        for bus in &self.state.buses {
            if bus.passengers.len() > BUS_CAPACITY {
                return Err(format!("bus {} exceeded passenger capacity", bus.id));
            }
            let line = self
                .state
                .bus_line
                .as_ref()
                .filter(|line| line.id == bus.line_id)
                .ok_or_else(|| format!("bus {} references a missing line", bus.id))?;
            let route_length: f32 = self.route_lengths(&line.route).iter().sum();
            if !bus.progress.is_finite()
                || bus.progress < 0.0
                || bus.progress > route_length + 0.01
                || !matches!(bus.direction, -1 | 1)
            {
                return Err(format!("bus {} has invalid movement state", bus.id));
            }
            for resident_id in &bus.passengers {
                if !bus_passengers.insert(*resident_id) {
                    return Err(format!("resident {resident_id} boarded more than one bus"));
                }
                if !self.state.residents.iter().any(|resident| {
                    resident.id == *resident_id
                        && resident.bus_id == Some(bus.id)
                        && resident.transfer == "riding_bus"
                }) {
                    return Err(format!("bus {} has an unlinked passenger", bus.id));
                }
            }
        }
        for resident in &self.state.residents {
            if (resident.transfer == "riding_bus") != bus_passengers.contains(&resident.id) {
                return Err(format!(
                    "resident {} has inconsistent bus occupancy",
                    resident.id
                ));
            }
        }
        for (at_station, states) in [
            (true, ["walking_to_station", "waiting_station"]),
            (false, ["waiting_gateway", "waiting_gateway"]),
        ] {
            let mut slots = BTreeSet::new();
            for resident in self
                .state
                .residents
                .iter()
                .filter(|resident| states.contains(&resident.transfer.as_str()))
            {
                if let Some(slot) = resident.queue_slot {
                    if slot >= RAIL_QUEUE_CAPACITY as u16 || !slots.insert(slot) {
                        return Err(format!(
                            "resident {} has an invalid or duplicate platform slot at {}",
                            resident.id,
                            if at_station { "station" } else { "gateway" }
                        ));
                    }
                }
            }
        }
        let mut passenger_ids = BTreeSet::new();
        for train in &self.state.trains {
            if train.passengers.len() > RAIL_TRAIN_CAPACITY {
                return Err(format!("train {} exceeded passenger capacity", train.id));
            }
            for resident_id in &train.passengers {
                if !passenger_ids.insert(*resident_id) {
                    return Err(format!(
                        "resident {resident_id} boarded more than one train"
                    ));
                }
                if !self.state.residents.iter().any(|resident| {
                    resident.id == *resident_id && resident.train_id == Some(train.id)
                }) {
                    return Err(format!("train {} has an unlinked passenger", train.id));
                }
            }
        }
        for resident in &self.state.residents {
            if let Some(train_id) = resident.train_id {
                if !passenger_ids.contains(&resident.id)
                    || !self.state.trains.iter().any(|train| train.id == train_id)
                {
                    return Err(format!(
                        "resident {} references a missing passenger seat",
                        resident.id
                    ));
                }
            }
        }
        let mut active_actor_ids = BTreeSet::new();
        for traveler in &self.state.travelers {
            if !active_actor_ids.insert(traveler.id) {
                return Err(format!("duplicate active traveler id {}", traveler.id));
            }
            if traveler.route.is_empty() || traveler.route_lengths.len() + 1 != traveler.route.len()
            {
                return Err(format!("traveler {} has a malformed route", traveler.id));
            }
            if traveler.edge_index > traveler.route_lengths.len()
                || !traveler.edge_progress.is_finite()
                || traveler.edge_progress < 0.0
                || traveler.speed <= 0.0
            {
                return Err(format!(
                    "traveler {} has invalid movement state",
                    traveler.id
                ));
            }
            if !matches!(traveler.access_phase, 0..=2)
                || !traveler.access_progress.is_finite()
                || traveler.access_progress < 0.0
            {
                return Err(format!(
                    "traveler {} has invalid access-path state",
                    traveler.id
                ));
            }
            if traveler.access_phase == 0 && traveler.origin_access.is_none() {
                return Err(format!(
                    "traveler {} is missing its origin connector",
                    traveler.id
                ));
            }
            if traveler.access_phase == 2 && traveler.destination_access.is_none() {
                return Err(format!(
                    "traveler {} is missing its destination connector",
                    traveler.id
                ));
            }
            let active_access_length = match traveler.access_phase {
                0 => traveler.origin_access.as_ref().map(AccessPath::length),
                2 => traveler.destination_access.as_ref().map(AccessPath::length),
                _ => None,
            };
            for path in [
                traveler.origin_access.as_ref(),
                traveler.destination_access.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                let path_requires_rail_surface = (traveler.activity == "walking_to_station"
                    && traveler
                        .destination_access
                        .as_ref()
                        .is_some_and(|rail| std::ptr::eq(rail, path)))
                    || (traveler.activity == "walking_home"
                        && traveler
                            .origin_access
                            .as_ref()
                            .is_some_and(|rail| std::ptr::eq(rail, path))
                        && self.state.residents.iter().any(|resident| {
                            resident.id == traveler.resident && resident.rail_commuter
                        }));
                // Historical legacy-planner fixtures predate explicit rail surfaces.
                // Current master-plan worlds must always carry them; obsolete master plans
                // are rejected by the layout-version digest before this validation.
                if self.state.masterplan.is_some()
                    && path_requires_rail_surface
                    && path.surface_y.len() != path.points.len()
                {
                    return Err(format!(
                        "traveler {} has a rail access path without surface elevations",
                        traveler.id
                    ));
                }
                if path_requires_rail_surface && !path.surface_y.is_empty() {
                    let matches =
                        path.points
                            .first()
                            .zip(path.points.last())
                            .is_some_and(|(&from, &to)| {
                                [true, false].into_iter().any(|at_station| {
                                    let expected = if traveler.activity == "walking_to_station" {
                                        self.platform_entry_path_from_position(
                                            from,
                                            to,
                                            at_station,
                                            traveler.id,
                                        )
                                    } else {
                                        self.platform_exit_path_from_position(from, to, at_station)
                                    };
                                    expected.is_some_and(|expected| {
                                        expected.surface_y == path.surface_y
                                            && expected.points.len() == path.points.len()
                                            && expected
                                                .points
                                                .iter()
                                                .zip(&path.points)
                                                .all(|(a, b)| a.x == b.x && a.z == b.z)
                                    })
                                })
                            });
                    if !matches {
                        return Err(format!("traveler {} has obsolete or invalid rail access geometry; start a fresh world", traveler.id));
                    }
                }
                // Door + three reserved passage points + curb + street-end +
                // route-end. Keep a finite bound for the candidate save format.
                let max_access_points = if path_requires_rail_surface {
                    8
                } else if cfg!(feature = "flag-passage-candidate")
                    || cfg!(feature = "parent-passage-candidate")
                {
                    7
                } else {
                    5
                };
                if !(2..=max_access_points).contains(&path.points.len())
                    || (!path.surface_y.is_empty()
                        && (path.surface_y.len() != path.points.len()
                            || path.surface_y.iter().any(|height| {
                                !height.is_finite()
                                    || (*height < 0.0 && *height != -1.0)
                                    || *height > 16.0
                            })))
                    || path.points.iter().any(|point| {
                        !point.x.is_finite()
                            || !point.z.is_finite()
                            || point.x < MIN - 1.0
                            || point.x > MAX + 1.0
                            || point.z < MIN - 1.0
                            || point.z > MAX + 1.0
                    })
                {
                    return Err(format!(
                        "traveler {} has invalid access geometry: phase={} progress={:.3} path_length={:.3} points={:?}",
                        traveler.id,
                        traveler.access_phase,
                        traveler.access_progress,
                        path.length(),
                        path.points
                    ));
                }
            }
            if active_access_length
                .is_some_and(|length| traveler.access_progress > length + traveler.speed)
            {
                return Err(format!(
                    "traveler {} exceeded its active access path",
                    traveler.id
                ));
            }
            if traveler.resident != 0 && !resident_ids.contains(&traveler.resident) {
                return Err(format!(
                    "traveler {} references a missing resident",
                    traveler.id
                ));
            }
        }
        if !self.state.station_online && !self.state.trains.is_empty() {
            return Err("train service exists before station and rail are usable".into());
        }
        if self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .count()
            > MAX_PROJECTS
        {
            return Err("project cap exceeded".into());
        }
        let ids: BTreeSet<u32> = self.state.nodes.iter().map(|n| n.id).collect();
        if self
            .state
            .roads
            .iter()
            .any(|r| !ids.contains(&r.from) || !ids.contains(&r.to))
        {
            return Err("road references missing node".into());
        }
        for b in &self.state.buildings {
            if b.occupancy > b.capacity {
                return Err(format!("building {} occupancy exceeds capacity", b.id));
            }
            let p = self
                .state
                .parcels
                .iter()
                .find(|p| p.id == b.parcel_id)
                .ok_or_else(|| format!("building {} references missing parcel", b.id))?;
            if b.stage == "open" && !self.node_connected(p.access_node) {
                return Err(format!(
                    "open building {} lacks connected road access",
                    b.id
                ));
            }
            if !(MIN..=MAX).contains(&b.x) || !(MIN..=MAX).contains(&b.z) {
                return Err(format!("building {} outside boundary", b.id));
            }
            if p.protected {
                return Err(format!("building {} occupies protected land", b.id));
            }
            if self
                .state
                .buildings
                .iter()
                .any(|other| other.id != b.id && other.parcel_id == b.parcel_id)
            {
                return Err(format!(
                    "parcel {} contains multiple buildings",
                    b.parcel_id
                ));
            }
        }
        for (i, p) in self.state.parcels.iter().enumerate() {
            let footprint = parcel_envelope(p);
            if !footprint.finite_positive() {
                return Err(format!("parcel {} has invalid geometry", p.id));
            }
            if let Some(rotation) = p.rotation {
                let facing = self.frontage_rotation(p.id);
                if !self
                    .state
                    .parcel_frontages
                    .iter()
                    .any(|frontage| frontage.parcel_id == p.id)
                    || (rotation.sin() - facing.sin()).abs() > 1e-4
                    || (rotation.cos() - facing.cos()).abs() > 1e-4
                {
                    return Err(format!(
                        "parcel {} orientation does not face its frontage",
                        p.id
                    ));
                }
            }
            let (min_x, min_z, max_x, max_z) = footprint.bounds();
            if min_x < MIN || max_x > MAX || min_z < MIN || max_z > MAX {
                return Err(format!("parcel {} outside boundary", p.id));
            }
            // Master-plan lots tile blocks and share lot lines; only real
            // overlap beyond float noise is invalid.
            let party_wall = self.masterplan_active();
            let shrink = |rect: world::geometry::OrientedRect| {
                if party_wall {
                    world::geometry::OrientedRect {
                        width: rect.width - 0.1,
                        depth: rect.depth - 0.1,
                        ..rect
                    }
                } else {
                    rect
                }
            };
            for other in self.state.parcels.iter().skip(i + 1) {
                if party_wall && ((other.x - p.x).abs() > 60.0 || (other.z - p.z).abs() > 60.0) {
                    continue;
                }
                if world::geometry::rects_overlap(
                    &shrink(footprint),
                    &shrink(parcel_envelope(other)),
                    0.0,
                ) {
                    return Err(format!("parcels {} and {} overlap", p.id, other.id));
                }
            }
            let mut road_envelopes = vec![footprint];
            let road_clearance = if p.rotation.is_some() { 3.0 } else { 5.0 };
            let legacy_center_clearance = (p.width.max(p.depth) * 0.5 + 3.0).max(8.0);
            if p.rotation.is_none() {
                let rendered_rotation = building_rotations
                    .get(&p.id)
                    .copied()
                    .unwrap_or_else(|| self.frontage_rotation(p.id));
                let mut rendered = footprint;
                rendered.rotation = rendered_rotation;
                road_envelopes.push(rendered);
            }
            for road in self
                .state
                .roads
                .iter()
                .filter(|r| r.stage == "open" && r.kind != "expressway")
            {
                let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                    continue;
                };
                let nearest_envelope_distance = road_envelopes
                    .iter()
                    .map(|envelope| {
                        world::geometry::rect_segment_distance(envelope, a.x, a.z, b.x, b.z)
                    })
                    .fold(f32::INFINITY, f32::min);
                // Lanes are 4 m shared surfaces; master-plan lots start at
                // 2.3 m, retaining a small margin over this runtime envelope.
                let road_clearance = match road.kind.as_str() {
                    "lane" => 2.2,
                    "local" => f32::min(road_clearance, 3.4),
                    _ => road_clearance,
                };
                let exact_clearance_ok = nearest_envelope_distance >= road_clearance;
                let legacy_clearance_ok = p.rotation.is_none()
                    && point_segment_distance(p.x, p.z, a.x, a.z, b.x, b.z)
                        >= legacy_center_clearance;
                if !exact_clearance_ok && !legacy_clearance_ok {
                    return Err(format!(
                        "parcel {} intersects road {} clearance: parcel=({:.2},{:.2}) road=({:.2},{:.2})-({:.2},{:.2}) distance={:.2} clearance={:.2}",
                        p.id,
                        road.id,
                        p.x,
                        p.z,
                        a.x,
                        a.z,
                        b.x,
                        b.z,
                        nearest_envelope_distance,
                        road_clearance
                    ));
                }
            }
            if let Some(building_id) = p.reservation {
                let building = self
                    .state
                    .buildings
                    .iter()
                    .find(|b| b.id == building_id && b.parcel_id == p.id)
                    .ok_or_else(|| format!("parcel {} reserves a missing building", p.id))?;
                let rear_extension_reserved = self
                    .state
                    .pending_rear_extensions
                    .iter()
                    .any(|plan| plan.building_id == building_id && plan.parcel_id == p.id);
                if building.stage == "open" && !rear_extension_reserved {
                    return Err(format!("parcel {} reserves an open building", p.id));
                }
                if !self.state.projects.iter().any(|project| {
                    project.kind == "building"
                        && project.parcel_id == Some(p.id)
                        && project.progress < 1.0
                }) && !self.state.pending_rear_extensions.iter().any(|plan| {
                    plan.building_id == building_id
                        && plan.parcel_id == p.id
                        && self.state.projects.iter().any(|project| {
                            project.kind == "extension"
                                && project.id == plan.project_id
                                && project.progress < 1.0
                        })
                }) && !self
                    .state
                    .height_redevelopment
                    .as_ref()
                    .is_some_and(|plan| {
                        plan.building_id == building_id
                            && plan.parcel_id == p.id
                            && self.state.projects.iter().any(|project| {
                                project.id == plan.project_id
                                    && project.kind == "redevelopment"
                                    && project.progress < 1.0
                            })
                    })
                {
                    return Err(format!(
                        "parcel {} has no active construction project",
                        p.id
                    ));
                }
            }
        }
        Ok(())
    }
}
