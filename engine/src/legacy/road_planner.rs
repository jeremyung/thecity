use crate::*;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "legacy")]
pub(crate) const MAX_ROAD_ORIGINS_PER_EVALUATION: usize = 6;

#[cfg(feature = "legacy")]
pub(crate) fn district_frontier_candidate_id(start_id: u32, target_id: Option<u32>) -> u64 {
    0x4000_0000_0000_0000 | ((start_id as u64) << 32) | target_id.unwrap_or(0) as u64
}

#[cfg(feature = "legacy")]
pub(crate) fn district_access_candidate_id(
    proposal: &legacy::interior_access::AccessProposal,
) -> u64 {
    0x8000_0000_0000_0000
        | ((proposal.score.host_id as u64) << 16)
        | ((proposal.score.fraction_index as u64) << 12)
        | ((proposal.score.side_index as u64) << 8)
        | proposal.score.length_index as u64
}

#[cfg(feature = "legacy")]
pub(crate) fn road_demand_is_sufficient(demand: f32) -> bool {
    demand > 0.8
}

impl Engine {
    /// Incremental frontier/infill growth for worlds without a master plan.
    #[cfg(feature = "legacy")]
    pub(crate) fn evaluate_legacy_growth(&mut self) {
        // Bridge opens before western growth can be considered.
        let day = (self.state.tick / TICKS_PER_DAY) as u32;
        let open_roads = self
            .state
            .roads
            .iter()
            .filter(|road| road.stage == "open")
            .count();
        let homes = self
            .state
            .buildings
            .iter()
            .filter(|b| is_housing(b) && site_active(b))
            .count();
        let businesses = self
            .state
            .buildings
            .iter()
            .filter(|b| is_commercial(b) && site_active(b))
            .count();
        let active = self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .count();
        if open_roads < 3 && active < MAX_PROJECTS {
            return;
        }
        // Finish starter parcels first, then spend each new street's infill quota
        // before considering another frontier extension.
        if self.state.buildings.len() < 5
            && !self.state.parcels.iter().any(|p| p.reservation.is_some())
        {
            self.plan_building(day);
            return;
        }
        let usable_infill = self.state.parcels.iter().any(|parcel| {
            !parcel.protected
                && parcel.reservation.is_none()
                && (self.state.station_online || !self.is_station_tower_lot(parcel))
                && self.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                && !self
                    .state
                    .buildings
                    .iter()
                    .any(|b| b.parcel_id == parcel.id)
                && self.node_connected(parcel.access_node)
        });
        let active_building_projects = self
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "building" && project.progress < 1.0)
            .count();
        let active_road_projects = self
            .state
            .projects
            .iter()
            .filter(|project| project.kind == "road" && project.progress < 1.0)
            .count();
        // Reserve a coordinated block before ordinary infill and building
        // project gates can spend its interior frontage. Reservation only
        // records the plan; perimeter construction remains on the existing
        // later 80-tick scheduler.
        let early_block_reserved = self.try_reserve_early_coordinated_block(active_road_projects);
        if self.state.road_infill_due > 0 {
            if usable_infill {
                if self.state.tick % 80 == 0 {
                    self.plan_building(day);
                }
                return;
            }
            // This quota prioritizes newly opened frontage while usable lots
            // remain. Do not let an exhausted lot pool freeze network growth.
            self.state.road_infill_due = 0;
        }
        if usable_infill || active_building_projects > 0 {
            if usable_infill && self.state.tick % 80 == 0 {
                self.plan_building(day);
            }
            return;
        }
        let connected_ordinary_roads = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
            })
            .count();
        #[cfg(test)]
        if self.experiments.early_access_experiment
            && connected_ordinary_roads >= 40
            && self.state.tick % 160 == 0
            && active_road_projects == 0
            && active == 0
            && self.state.neighborhood_plan.is_none()
            && !self.neighborhood_block_has_unbuilt_frontage()
            && homes + businesses >= 3
            && road_demand_is_sufficient(self.state.demand)
            && self
                .state
                .roads
                .len()
                .saturating_add(self.state.projects.len())
                < MAX_SEGMENTS
        {
            let active_buildings: Vec<Building> = self
                .state
                .buildings
                .iter()
                .filter(|building| site_active(building))
                .cloned()
                .collect();
            let bridgehead_due = !self
                .eligible_bridgehead_headings(&active_buildings)
                .is_empty();
            if !bridgehead_due {
                self.experiments.early_access_attempted_tick = Some(self.state.tick);
                self.experiments.early_access_attempts =
                    self.experiments.early_access_attempts.saturating_add(1);
                if self.try_plan_interior_access() {
                    self.experiments.early_access_admissions =
                        self.experiments.early_access_admissions.saturating_add(1);
                    return;
                }
            }
        }
        // As established blocks fill, release one additional roadside lot at a
        // time. Candidate work is persisted and capped so older worlds can
        // acquire infill without a full-map scan in a single simulation update.
        if connected_ordinary_roads >= 40
            && self.state.tick % 160 == 0
            && self.try_progressive_frontage_infill()
        {
            return;
        }
        if active_road_projects > 0 {
            return;
        }
        // Give compact blocks the same bounded opportunity cadence as road
        // extensions, after existing usable frontage has had priority. The
        // former 1,600-tick phase made block planning depend on tick alignment
        // and often let frontier roads consume the available geometry first.
        if self.state.neighborhood_plan.is_none()
            && self.state.road_infill_due == 0
            && active == 0
            && !self.neighborhood_block_has_unbuilt_frontage()
            && self.state.tick % 80 == 0
            && self.try_create_neighborhood_plan()
        {
            self.plan_next_neighborhood_segment();
            return;
        }
        if self
            .state
            .neighborhood_plan
            .as_ref()
            .is_some_and(|plan| plan.next_segment < plan.segments.len() || plan.spine.is_some())
            && self.state.tick % 80 == 0
        {
            if early_block_reserved {
                return;
            }
            self.plan_next_neighborhood_segment();
            return;
        }
        if connected_ordinary_roads >= 40
            && self.state.tick % 160 == 0
            && self.state.demand > 1.1
            && self.try_start_rear_extension()
        {
            return;
        }
        let connected_nodes: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|r| r.connected && r.stage == "open")
            .flat_map(|r| [r.from, r.to])
            .collect();
        let isolated = self
            .state
            .roads
            .iter()
            .filter(|r| r.kind == "bridge" && r.stage != "open")
            .count()
            > 0;
        // This is the production demand boundary for every road planner,
        // including the district-policy comparison below. Keep ordinary
        // capacity proposals out of that comparison at demand <= 0.8.
        let needs_road = homes + businesses >= 3
            && self.state.roads.len() + self.state.projects.len() < MAX_SEGMENTS
            && road_demand_is_sufficient(self.state.demand);
        if needs_road && !isolated && connected_nodes.len() > 1 && self.state.tick % 80 == 0 {
            self.plan_road();
            return;
        }
        if (self.state.demand > 0.75 || day < 5) && self.state.tick % 80 == 0 {
            self.plan_building(day);
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn ordinary_road_bearings(&self, node_id: u32) -> Vec<f32> {
        let mut bearings = Vec::new();
        for road in self.state.roads.iter().filter(|road| {
            road.connected
                && road.stage == "open"
                && !matches!(road.kind.as_str(), "rail" | "bridge")
                && (road.from == node_id || road.to == node_id)
        }) {
            let neighbor_id = if road.from == node_id {
                road.to
            } else {
                road.from
            };
            if let Some(neighbor) = self.node(neighbor_id) {
                if let Some(node) = self.node(node_id) {
                    bearings.push((neighbor.z - node.z).atan2(neighbor.x - node.x));
                }
            }
        }
        for project in self.state.projects.iter().filter(|project| {
            project.kind == "road"
                && project.progress < 1.0
                && (project.from == Some(node_id) || project.to == Some(node_id))
        }) {
            let neighbor_id = if project.from == Some(node_id) {
                project.to
            } else {
                project.from
            };
            if let (Some(node), Some(neighbor)) =
                (self.node(node_id), neighbor_id.and_then(|id| self.node(id)))
            {
                bearings.push((neighbor.z - node.z).atan2(neighbor.x - node.x));
            }
        }
        bearings
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn ordinary_road_degree(&self, node_id: u32) -> usize {
        let open = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.connected
                    && road.stage == "open"
                    && !matches!(road.kind.as_str(), "rail" | "bridge")
                    && (road.from == node_id || road.to == node_id)
            })
            .count();
        let planned = self
            .state
            .projects
            .iter()
            .filter(|project| {
                project.kind == "road"
                    && project.progress < 1.0
                    && (project.from == Some(node_id) || project.to == Some(node_id))
            })
            .count();
        open + planned
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn ordinary_heading_clear(&self, node_id: u32, heading: f32) -> bool {
        const MIN_JUNCTION_ANGLE: f32 = std::f32::consts::PI / 6.0;
        self.ordinary_road_bearings(node_id).iter().all(|existing| {
            let difference = (heading - existing).rem_euclid(std::f32::consts::TAU);
            difference.min(std::f32::consts::TAU - difference) >= MIN_JUNCTION_ANGLE
        })
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn road_extension_headings(&self, node_id: u32) -> Vec<f32> {
        let mut headings = Vec::new();
        for bearing in self.ordinary_road_bearings(node_id) {
            // A continuation extends the corridor; the perpendicular choices
            // create the cross streets that turn a string of lots into blocks.
            for heading in [
                bearing + std::f32::consts::PI,
                bearing + std::f32::consts::FRAC_PI_2,
                bearing - std::f32::consts::FRAC_PI_2,
            ] {
                let normalized = heading.rem_euclid(std::f32::consts::TAU);
                if !headings.iter().any(|existing: &f32| {
                    let difference = (normalized - existing).abs();
                    difference.min(std::f32::consts::TAU - difference) < 0.12
                }) {
                    headings.push(normalized);
                }
            }
        }
        headings
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn eligible_bridgehead_headings(
        &self,
        open_buildings: &[Building],
    ) -> BTreeMap<u32, f32> {
        let mut headings = BTreeMap::new();
        for bridge in self
            .state
            .roads
            .iter()
            .filter(|road| road.kind == "bridge" && road.stage == "open" && road.connected)
        {
            for (head_id, opposite_id) in [(bridge.from, bridge.to), (bridge.to, bridge.from)] {
                let Some(head) = self.node(head_id) else {
                    continue;
                };
                let Some(opposite) = self.node(opposite_id) else {
                    continue;
                };
                let ordinary_open_or_planned = self.state.roads.iter().any(|road| {
                    (road.from == head_id || road.to == head_id)
                        && road.stage == "open"
                        && road.connected
                        && !matches!(road.kind.as_str(), "rail" | "bridge")
                }) || self.state.projects.iter().any(|project| {
                    let (Some(from), Some(to)) = (project.from, project.to) else {
                        return false;
                    };
                    if project.kind != "road"
                        || project.progress >= 1.0
                        || (from != head_id && to != head_id)
                    {
                        return false;
                    }
                    let matches_nonordinary_road = self.state.roads.iter().any(|road| {
                        matches!(road.kind.as_str(), "rail" | "bridge")
                            && ((road.from == from && road.to == to)
                                || (road.to == from && road.from == to))
                    });
                    !matches_nonordinary_road
                });
                if ordinary_open_or_planned {
                    continue;
                }
                let nearest_opposite_catchment = open_buildings
                    .iter()
                    .map(|building| (building.x - opposite.x).hypot(building.z - opposite.z))
                    .min_by(f32::total_cmp);
                if !nearest_opposite_catchment.is_some_and(|distance| distance <= 110.0) {
                    continue;
                }
                let outward_heading = (head.z - opposite.z).atan2(head.x - opposite.x);
                headings.insert(head_id, outward_heading);
            }
        }
        headings
    }

    /// Enumerate the first viable frontier proposal without mutating the
    /// engine. The local RNG and cursor follow the legacy search order so the
    /// caller can commit precisely the state the mutating planner used.
    #[cfg(feature = "legacy")]
    pub(crate) fn propose_frontier_road(
        &self,
        open_buildings: &[Building],
        bridgehead_headings: &BTreeMap<u32, f32>,
        collision_context: &CollisionContext,
    ) -> legacy::frontier::FrontierScan {
        let center = open_buildings.iter().fold((0.0, 0.0), |(x, z), building| {
            (
                x + building.x / open_buildings.len() as f32,
                z + building.z / open_buildings.len() as f32,
            )
        });
        let connected_nodes: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
            .flat_map(|road| [road.from, road.to])
            .collect();
        // An open bridge can leave its far endpoint connected but outside the
        // normal occupied-frontier radius. Admit exactly one bank-entry origin
        // while the opposite bank has an active catchment and no ordinary road
        // has already claimed the bridgehead.
        let mut candidates: Vec<(Node, f32, Option<f32>)> = self
            .state
            .nodes
            .iter()
            .filter(|node| connected_nodes.contains(&node.id))
            .filter_map(|node| {
                let nearest_occupied_block = open_buildings
                    .iter()
                    .map(|building| (building.x - node.x).hypot(building.z - node.z))
                    .min_by(f32::total_cmp)?;
                // Start at the edge of a developed block, never at an unserved
                // network tip that is already far from the neighborhood. A
                // bridgehead is the sole exception, and only while its opposite
                // bank is developed and this bank has no ordinary connection.
                if nearest_occupied_block <= 110.0 {
                    Some((
                        node.clone(),
                        nearest_occupied_block,
                        bridgehead_headings.get(&node.id).copied(),
                    ))
                } else {
                    bridgehead_headings
                        .get(&node.id)
                        .copied()
                        .map(|heading| (node.clone(), nearest_occupied_block, Some(heading)))
                }
            })
            .collect();
        // Prefer a viable frontage near the developed edge, while randomizing
        // among a few similarly placed nodes to keep seeded layouts distinct.
        candidates.sort_by(|(a, _, _), (b, _, _)| {
            let a_radius = (a.x - center.0).hypot(a.z - center.1);
            let b_radius = (b.x - center.0).hypot(b.z - center.1);
            b_radius.total_cmp(&a_radius).then_with(|| a.id.cmp(&b.id))
        });
        if candidates.is_empty() {
            return legacy::frontier::FrontierScan {
                proposal: None,
                next_cursor: self.state.road_origin_cursor,
                next_rng: self.state.rng_growth,
                allow_access_fallback: false,
            };
        }

        let mut rng = self.state.rng_growth;
        let choice_count = candidates.len();
        let first_choice = self.state.road_origin_cursor % choice_count;
        let origin_count = choice_count.min(MAX_ROAD_ORIGINS_PER_EVALUATION);
        let mut next_cursor = (first_choice + origin_count) % choice_count;
        for attempt in 0..origin_count {
            let idx = (first_choice + attempt) % choice_count;
            next_cursor = (idx + 1) % choice_count;
            let start = candidates[idx].0.clone();
            let bridgehead_heading = candidates[idx].2;
            if self
                .state
                .projects
                .iter()
                .any(|p| p.kind == "road" && p.from == Some(start.id) && p.progress < 1.0)
            {
                continue;
            }
            let generated_junction = start.id >= 7;
            if generated_junction && self.ordinary_road_degree(start.id) >= 4 {
                continue;
            }
            let mut headings = bridgehead_heading
                .map(|heading| vec![heading])
                .unwrap_or_else(|| self.road_extension_headings(start.id));
            if headings.is_empty() {
                // Match the legacy seeded fallback for rail/bridge anchors.
                headings.push(random_unit(&mut rng) * std::f32::consts::TAU);
            }
            let rotate = (rng_next(&mut rng) as usize) % headings.len();
            headings.rotate_left(rotate);
            let mut loop_targets: Vec<Node> = self
                .state
                .nodes
                .iter()
                .filter(|node| node.id != start.id && connected_nodes.contains(&node.id))
                .filter(|node| {
                    let distance = (node.x - start.x).hypot(node.z - start.z);
                    distance >= 45.0
                        && distance <= 90.0
                        && !self.state.roads.iter().any(|road| {
                            (road.from == start.id && road.to == node.id)
                                || (road.to == start.id && road.from == node.id)
                        })
                        && !self.state.projects.iter().any(|project| {
                            project.kind == "road"
                                && project.progress < 1.0
                                && ((project.from == Some(start.id) && project.to == Some(node.id))
                                    || (project.from == Some(node.id)
                                        && project.to == Some(start.id)))
                        })
                })
                .cloned()
                .collect();
            for base_angle in headings.iter().copied().flat_map(|heading| [heading; 3]) {
                let angle = base_angle + (random_unit(&mut rng) - 0.5) * 0.18;
                if !self.ordinary_heading_clear(start.id, angle) {
                    continue;
                }
                let length = 50.0 + random_unit(&mut rng) * 38.0;
                let mut x = start.x + angle.cos() * length;
                let mut z = start.z + angle.sin() * length;
                x = x.clamp(MIN + 25.0, MAX - 25.0);
                z = z.clamp(MIN + 25.0, MAX - 25.0);
                if !self.ordinary_heading_clear(start.id, (z - start.z).atan2(x - start.x)) {
                    continue;
                }
                if in_protected_land(x, z) || corridor_crosses_protected(start.x, start.z, x, z) {
                    continue;
                }
                let point = Node {
                    id: u32::MAX,
                    x,
                    z,
                    kind: "candidate".into(),
                };
                let try_loop = !loop_targets.is_empty() && random_unit(&mut rng) < 0.28;
                let mut loop_target = None;
                if try_loop {
                    let target_index = (rng_next(&mut rng) as usize) % loop_targets.len();
                    let target = loop_targets.swap_remove(target_index);
                    let target_angle = (target.z - start.z).atan2(target.x - start.x);
                    if self.ordinary_heading_clear(start.id, target_angle)
                        && self.ordinary_heading_clear(
                            target.id,
                            (start.z - target.z).atan2(start.x - target.x),
                        )
                        && (target.id < 7 || self.ordinary_road_degree(target.id) < 4)
                        && !in_protected_land(target.x, target.z)
                        && !corridor_crosses_protected(start.x, start.z, target.x, target.z)
                    {
                        loop_target = Some(target.clone());
                    }
                }
                let endpoint = loop_target.as_ref().unwrap_or(&point);
                if !self.ordinary_road_clear_of_river(start.x, start.z, endpoint.x, endpoint.z, 4.5)
                {
                    continue;
                }
                if loop_target.is_none()
                    && self
                        .state
                        .nodes
                        .iter()
                        .any(|node| (node.x - x).hypot(node.z - z) < 18.0)
                {
                    continue;
                }
                let corridor_clear = self.road_corridor_clear_of_parcels_with_context(
                    start.x,
                    start.z,
                    endpoint.x,
                    endpoint.z,
                    collision_context,
                );
                let endpoint_id = loop_target.as_ref().map(|node| node.id);
                let road_clear = self
                    .state
                    .roads
                    .iter()
                    .filter(|road| road.stage == "open")
                    .all(|road| {
                        if road.from == start.id
                            || road.to == start.id
                            || Some(road.from) == endpoint_id
                            || Some(road.to) == endpoint_id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                            return true;
                        };
                        segments_distance(&start, endpoint, a, b) > 8.0
                    });
                let projects_clear = self
                    .state
                    .projects
                    .iter()
                    .filter(|project| project.kind == "road" && project.progress < 1.0)
                    .all(|project| {
                        let (Some(from), Some(to)) = (project.from, project.to) else {
                            return true;
                        };
                        if from == start.id
                            || to == start.id
                            || Some(from) == endpoint_id
                            || Some(to) == endpoint_id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                            return true;
                        };
                        segments_distance(&start, endpoint, a, b) > 8.0
                    });
                if !corridor_clear
                    || !road_clear
                    || !projects_clear
                    || !self.neighborhood_corridor_clear_for_road(&start, endpoint, start.id)
                    || self.viable_frontage_benefit_count(&start, endpoint, 2) < 2
                {
                    continue;
                }
                return legacy::frontier::FrontierScan {
                    proposal: Some(legacy::frontier::FrontierProposal {
                        start_id: start.id,
                        target_id: endpoint_id,
                        end_x: endpoint.x,
                        end_z: endpoint.z,
                        bridgehead: bridgehead_heading.is_some(),
                    }),
                    next_cursor,
                    next_rng: rng,
                    allow_access_fallback: false,
                };
            }
        }
        legacy::frontier::FrontierScan {
            proposal: None,
            next_cursor,
            next_rng: rng,
            allow_access_fallback: true,
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn plan_road(&mut self) {
        self.plan_road_with_policy(self.district_policy_experiment);
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn plan_road_with_policy(&mut self, use_district_policy: bool) {
        if self.state.roads.len() + self.state.projects.len() + self.spine_reserved_road_records()
            >= MAX_SEGMENTS
            || !self.project_admission_available()
        {
            return;
        }
        let open_buildings: Vec<Building> = self
            .state
            .buildings
            .iter()
            .filter(|building| site_active(building))
            .cloned()
            .collect();
        if open_buildings.is_empty() {
            return;
        }
        // Preserve the existing bridgehead opportunity for frontier planning;
        // interior access remains a fallback after viable outward proposals.
        let bridgehead_headings = self.eligible_bridgehead_headings(&open_buildings);
        let collision_context = self.collision_context();
        if self.try_plan_local_chord(&open_buildings, &collision_context) {
            return;
        }
        let scan =
            self.propose_frontier_road(&open_buildings, &bridgehead_headings, &collision_context);
        // An already-selected bridgehead is an infrastructure prerequisite:
        // keep its admission ahead of density scoring exactly as before.
        if scan
            .proposal
            .as_ref()
            .is_some_and(|proposal| proposal.bridgehead)
        {
            let proposal = scan.proposal.clone().unwrap();
            self.commit_frontier_road(scan, proposal);
            return;
        }
        let can_compare = use_district_policy
            && (scan.proposal.is_some() || scan.allow_access_fallback)
            && self.district_runtime.metrics.is_some()
            && self
                .state
                .tick
                .saturating_sub(self.state.district_controller.completed_capture_tick)
                <= legacy::district_policy::METRICS_MAX_AGE_TICKS
            && road_demand_is_sufficient(self.state.demand);
        if can_compare {
            #[cfg(test)]
            {
                self.experiments.district_policy_comparisons = self
                    .experiments
                    .district_policy_comparisons
                    .saturating_add(1);
            }
            let access_scan = self.propose_interior_access();
            // Both read-only scans advance their deterministic cursors once,
            // even when their proposal loses this comparison.
            self.state.road_origin_cursor = scan.next_cursor;
            self.state.rng_growth = scan.next_rng;
            self.state.interior_access_cursor = access_scan.next_cursor;
            if self.try_admit_ranked_district_road(scan.proposal, access_scan.proposals) {
                return;
            }
            return;
        }
        let Some(proposal) = scan.proposal.clone() else {
            if scan.allow_access_fallback && self.access_fallback_not_attempted_this_tick() {
                self.state.road_origin_cursor = scan.next_cursor;
                self.state.rng_growth = scan.next_rng;
                self.try_plan_interior_access();
            }
            return;
        };
        debug_assert_eq!(
            proposal.bridgehead,
            bridgehead_headings.contains_key(&proposal.start_id)
        );
        self.commit_frontier_road(scan, proposal);
    }

    #[cfg(test)]
    pub(crate) fn access_fallback_not_attempted_this_tick(&self) -> bool {
        self.experiments.early_access_attempted_tick != Some(self.state.tick)
    }

    #[cfg(not(test))]
    #[cfg(feature = "legacy")]
    pub(crate) fn access_fallback_not_attempted_this_tick(&self) -> bool {
        true
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn commit_frontier_road(
        &mut self,
        scan: legacy::frontier::FrontierScan,
        proposal: legacy::frontier::FrontierProposal,
    ) {
        self.state.road_origin_cursor = scan.next_cursor;
        self.state.rng_growth = scan.next_rng;
        if !self.project_admission_available() {
            return;
        }
        let to = proposal
            .target_id
            .unwrap_or_else(|| self.add_node(proposal.end_x, proposal.end_z, "junction"));
        let id = self.id();
        let reason = "A connected street extension serves demand from nearby homes and businesses";
        let duration = 3.0 + random_unit(&mut self.state.rng_growth) * 4.0;
        self.record_project(Project {
            id,
            kind: "road".into(),
            from: Some(proposal.start_id),
            to: Some(to),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.0,
            reason: reason.into(),
            duration,
            started_tick: self.state.tick,
        });
        self.state.revision += 1;
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn try_admit_ranked_district_road(
        &mut self,
        frontier_proposal: Option<legacy::frontier::FrontierProposal>,
        access_proposals: Vec<legacy::interior_access::AccessProposal>,
    ) -> bool {
        debug_assert!(access_proposals.len() <= legacy::district_policy::MAX_ACCESS_PROPOSALS);
        #[cfg(test)]
        let mut diagnostic = legacy::district_policy::DecisionRecord {
            tick: self.state.tick,
            frontier_proposal: frontier_proposal.is_some(),
            access_proposals: access_proposals.len(),
            ..legacy::district_policy::DecisionRecord::default()
        };
        let Some(metrics) = self.district_runtime.metrics.as_ref() else {
            #[cfg(test)]
            {
                diagnostic.no_admission_reason = Some("no_metrics");
                self.experiments
                    .district_policy_diagnostics
                    .observe(diagnostic);
            }
            return false;
        };
        let targets = self.district_targets(metrics);
        let mut score_candidates =
            Vec::with_capacity(legacy::district_policy::MAX_COMPARED_PROJECTS);
        let mut access_by_id = BTreeMap::new();
        let mut frontier_by_id = None;
        let building_samples = self.live_catchment_buildings();
        let demand_fit = legacy::district_policy::demand_fit(self.state.demand);

        if let Some(proposal) = frontier_proposal {
            let Some(start) = self.node(proposal.start_id) else {
                #[cfg(test)]
                {
                    diagnostic.no_admission_reason = Some("no_geometry");
                    self.experiments
                        .district_policy_diagnostics
                        .observe(diagnostic);
                }
                return false;
            };
            let endpoint = if let Some(target_id) = proposal.target_id {
                let Some(node) = self.node(target_id) else {
                    #[cfg(test)]
                    {
                        diagnostic.no_admission_reason = Some("no_geometry");
                        self.experiments
                            .district_policy_diagnostics
                            .observe(diagnostic);
                    }
                    return false;
                };
                node.clone()
            } else {
                Node {
                    id: u32::MAX,
                    x: proposal.end_x,
                    z: proposal.end_z,
                    kind: "candidate".into(),
                }
            };
            let rects = self.viable_frontage_benefit_sites(
                start,
                &endpoint,
                legacy::district_policy::MAX_FRONTIER_SITES,
            );
            #[cfg(test)]
            {
                diagnostic.frontier_geometry_candidates = usize::from(!rects.is_empty());
                diagnostic.frontier_sites = rects.len();
                diagnostic.frontier_area_m2 = rects
                    .iter()
                    .map(|rect| (rect.width * rect.depth) as f64)
                    .sum();
            }
            let stable_id = district_frontier_candidate_id(proposal.start_id, proposal.target_id);
            let catchment =
                legacy::district_policy::catchment_at(&building_samples, start.x, start.z);
            let source_market_accepts =
                legacy::district_policy::source_market_accepts_capacity(catchment);
            let feasible = self.state.demand > 0.8 && source_market_accepts;
            #[cfg(test)]
            {
                diagnostic.frontier_source_rejected =
                    diagnostic.frontier_geometry_candidates > 0 && !source_market_accepts;
                diagnostic.frontier_source_feasible_candidates = usize::from(
                    diagnostic.frontier_geometry_candidates > 0 && source_market_accepts,
                );
            }
            let road_cost = (endpoint.x - start.x).hypot(endpoint.z - start.z) as f64;
            if let Some(candidate) = self.make_district_score_candidate(
                stable_id,
                population::district_density::CandidateKind::Expansion,
                road_cost,
                demand_fit,
                catchment,
                feasible,
                &rects,
                metrics,
            ) {
                score_candidates.push(candidate);
                frontier_by_id = Some((stable_id, proposal));
            }
        }

        for proposal in access_proposals {
            #[cfg(test)]
            {
                diagnostic.access_sites += proposal.selected_lots.len();
                diagnostic.access_geometry_candidates +=
                    usize::from(!proposal.selected_lots.is_empty());
                diagnostic.access_area_m2 += proposal
                    .selected_lots
                    .iter()
                    .map(|lot| (lot.width * lot.depth) as f64)
                    .sum::<f64>();
            }
            let Some(host) = self
                .state
                .roads
                .iter()
                .find(|road| road.id == proposal.host_id)
            else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(host.from), self.node(host.to)) else {
                continue;
            };
            let source_x = a.x + (b.x - a.x) * proposal.host_t;
            let source_z = a.z + (b.z - a.z) * proposal.host_t;
            let catchment =
                legacy::district_policy::catchment_at(&building_samples, source_x, source_z);
            let source_market_accepts =
                legacy::district_policy::source_market_accepts_capacity(catchment);
            let feasible = self.state.demand > 0.8 && source_market_accepts;
            let rects: Vec<_> = proposal
                .selected_lots
                .iter()
                .map(|lot| world::geometry::OrientedRect {
                    x: lot.x,
                    z: lot.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                })
                .collect();
            #[cfg(test)]
            {
                if !source_market_accepts && !rects.is_empty() {
                    diagnostic.access_vacancy_rejections += 1;
                } else if !rects.is_empty() {
                    diagnostic.access_source_feasible_candidates += 1;
                }
            }
            debug_assert!(rects.len() <= legacy::district_policy::MAX_ACCESS_LOTS_PER_PROPOSAL);
            let stable_id = district_access_candidate_id(&proposal);
            let road_cost = proposal.score.road_length_mm as f64 / 1_000.0;
            if let Some(candidate) = self.make_district_score_candidate(
                stable_id,
                population::district_density::CandidateKind::InteriorAccess,
                road_cost,
                demand_fit,
                catchment,
                feasible,
                &rects,
                metrics,
            ) {
                score_candidates.push(candidate);
                access_by_id.insert(stable_id, proposal);
            }
        }

        if score_candidates.is_empty() {
            #[cfg(test)]
            {
                diagnostic.no_admission_reason = Some(
                    if diagnostic.frontier_sites == 0 && diagnostic.access_sites == 0 {
                        "no_geometry"
                    } else if diagnostic.frontier_source_rejected
                        || diagnostic.access_vacancy_rejections > 0
                    {
                        "no_source_feasible"
                    } else {
                        "no_cell_forecast"
                    },
                );
                self.experiments
                    .district_policy_diagnostics
                    .observe(diagnostic);
            }
            return false;
        }
        debug_assert!(score_candidates.len() <= legacy::district_policy::MAX_COMPARED_PROJECTS);
        let weights = population::district_density::ScoreWeights {
            coverage: 1.0,
            floor_area: 0.0,
            frontage: 0.0,
            demand: 0.2,
            road_cost_scale_m: 60.0,
        };
        let ranked = match legacy::project_scoring::rank_projects(
            metrics,
            &score_candidates,
            &targets,
            legacy::district_policy::vacancy_policy(),
            weights,
        ) {
            Ok(ranked) => ranked,
            Err(_) => {
                #[cfg(test)]
                {
                    diagnostic.no_admission_reason = Some("score_error");
                    self.experiments
                        .district_policy_diagnostics
                        .observe(diagnostic);
                }
                return false;
            }
        };
        if ranked.is_empty() {
            #[cfg(test)]
            {
                diagnostic.no_admission_reason = Some(
                    if diagnostic.frontier_source_rejected
                        || diagnostic.access_vacancy_rejections > 0
                    {
                        "no_source_feasible"
                    } else {
                        "no_cell_forecast"
                    },
                );
                self.experiments
                    .district_policy_diagnostics
                    .observe(diagnostic);
            }
            return false;
        }
        #[cfg(test)]
        {
            for candidate in &ranked {
                match candidate.kind {
                    population::district_density::CandidateKind::Expansion => {
                        diagnostic.frontier_ranked += 1;
                        if diagnostic.frontier_top_score.is_none() {
                            diagnostic.frontier_top_score = Some(candidate.score);
                        }
                    }
                    population::district_density::CandidateKind::InteriorAccess => {
                        diagnostic.access_ranked += 1;
                        if diagnostic.access_top_score.is_none() {
                            diagnostic.access_top_score = Some(candidate.score);
                        }
                    }
                    population::district_density::CandidateKind::Infill => {}
                }
            }
        }
        for candidate in ranked {
            if candidate.kind == population::district_density::CandidateKind::InteriorAccess {
                let Some(proposal) = access_by_id.remove(&candidate.stable_id) else {
                    continue;
                };
                if self.admit_interior_access(
                    proposal.host_id,
                    proposal.host_t,
                    WorldPoint {
                        x: proposal.end_x,
                        z: proposal.end_z,
                    },
                    proposal.selected_lots,
                ) {
                    #[cfg(test)]
                    {
                        self.experiments.district_policy_access_wins = self
                            .experiments
                            .district_policy_access_wins
                            .saturating_add(1);
                        diagnostic.winner = Some("access");
                        self.experiments
                            .district_policy_diagnostics
                            .observe(diagnostic);
                    }
                    return true;
                }
            } else if let Some((stable_id, proposal)) = frontier_by_id.take() {
                if stable_id == candidate.stable_id {
                    let scan = legacy::frontier::FrontierScan {
                        proposal: None,
                        next_cursor: self.state.road_origin_cursor,
                        next_rng: self.state.rng_growth,
                        allow_access_fallback: false,
                    };
                    self.commit_frontier_road(scan, proposal);
                    #[cfg(test)]
                    {
                        self.experiments.district_policy_frontier_wins = self
                            .experiments
                            .district_policy_frontier_wins
                            .saturating_add(1);
                        diagnostic.winner = Some("frontier");
                        self.experiments
                            .district_policy_diagnostics
                            .observe(diagnostic);
                    }
                    return true;
                }
                frontier_by_id = Some((stable_id, proposal));
            }
        }
        #[cfg(test)]
        {
            diagnostic.no_admission_reason = Some("ranked_admission_rejected");
            self.experiments
                .district_policy_diagnostics
                .observe(diagnostic);
        }
        false
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn district_targets(
        &self,
        metrics: &population::district_density::DistrictMetrics,
    ) -> Vec<population::district_density::DistrictTargets> {
        let minima: Vec<f64> = metrics
            .cells
            .iter()
            .map(|cell| {
                let x = ((cell.min_x + cell.max_x) * 0.5) as f32;
                let z = ((cell.min_z + cell.max_z) * 0.5) as f32;
                match self.neighborhood_profile(x, z) {
                    NeighborhoodProfile::HistoricCore => 0.32,
                    NeighborhoodProfile::StationCore => 0.40,
                    NeighborhoodProfile::StationFringe => 0.35,
                    NeighborhoodProfile::OuterNeighborhood => 0.28,
                }
            })
            .collect();
        legacy::district_policy::targets(metrics, &minima)
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn live_catchment_buildings(&self) -> Vec<legacy::district_policy::ServedBuilding> {
        let connected_nodes: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == "open")
            .flat_map(|road| [road.from, road.to])
            .collect();
        let access_nodes: BTreeMap<u32, u32> = self
            .state
            .parcels
            .iter()
            .map(|parcel| (parcel.id, parcel.access_node))
            .collect();
        self.state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .filter(|building| {
                access_nodes
                    .get(&building.parcel_id)
                    .is_some_and(|node_id| connected_nodes.contains(node_id))
            })
            .map(|building| legacy::district_policy::ServedBuilding {
                x: building.x,
                z: building.z,
                housing_capacity: if is_housing(building) {
                    building.capacity as f64
                } else {
                    0.0
                },
                housing_vacant: if is_housing(building) {
                    building.capacity.saturating_sub(building.occupancy) as f64
                } else {
                    0.0
                },
                commercial_capacity: if is_commercial(building) {
                    building.capacity as f64
                } else {
                    0.0
                },
                commercial_vacant: if is_commercial(building) && !building.business_active {
                    building.capacity as f64
                } else {
                    0.0
                },
            })
            .collect()
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn make_district_score_candidate(
        &self,
        stable_id: u64,
        kind: population::district_density::CandidateKind,
        road_cost_m: f64,
        demand_fit: f64,
        catchment: legacy::project_scoring::LiveCatchment,
        feasible: bool,
        rects: &[world::geometry::OrientedRect],
        metrics: &population::district_density::DistrictMetrics,
    ) -> Option<legacy::project_scoring::ProjectScoreCandidate> {
        if rects.is_empty() {
            return None;
        }
        let mut gains = BTreeMap::<usize, f64>::new();
        for rect in rects {
            let clipped =
                population::district_density::clipped_envelope_areas(metrics, *rect).ok()?;
            for (cell_id, area) in clipped {
                *gains.entry(cell_id).or_default() += area;
            }
        }
        if gains.is_empty() {
            return None;
        }
        let cell_impacts = gains
            .into_iter()
            .map(
                |(cell_id, area)| legacy::project_scoring::ForecastCellImpact {
                    cell_id,
                    forecast_envelope_gain_m2: area * legacy::district_policy::FORECAST_UTILIZATION,
                    forecast_floor_area_gain_m2: 0.0,
                    forecast_occupied_frontage_gain_m: 0.0,
                    // Positive one-unit flags activate vacancy relevance; these
                    // are not estimates of the future building capacity.
                    forecast_housing_capacity_gain: 1.0,
                    forecast_commercial_capacity_gain: 1.0,
                },
            )
            .collect();
        Some(legacy::project_scoring::ProjectScoreCandidate {
            stable_id,
            kind,
            feasible,
            road_cost_m,
            demand_fit,
            live_catchment: catchment,
            cell_impacts,
        })
    }
}
