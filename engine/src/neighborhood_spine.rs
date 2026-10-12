//! Runtime lifecycle for an optional center street in a coordinated block.
//!
//! Candidate selection is intentionally owned by the neighborhood planner; this
//! module only advances an already persisted, canonical spine reservation.

use super::*;
use crate::spine_plan::{SpinePlan, SpineStage};

const CORNER_TOLERANCE: f32 = 0.002;
const SPINE_SPLIT_T: f32 = 0.5;
const SPINE_LOT_SUITABILITY: f32 = 0.82;

impl Engine {
    /// Count road records still promised by a preconstruction spine reservation.
    /// Count every perimeter segment without a road record, including one
    /// represented only by a pending project. Each will still consume a record.
    pub(super) fn spine_reserved_road_records(&self) -> usize {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return 0;
        };
        let Some(spine) = plan.spine.as_ref() else {
            return 0;
        };
        if matches!(spine.stage, SpineStage::UnderConstruction { .. }) {
            return 0;
        }
        let perimeter = plan
            .segments
            .iter()
            .filter(|(from, to)| {
                !self.state.roads.iter().any(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                })
            })
            .count();
        perimeter + 3 // two split halves and the new spine road
    }

    /// Advance a reserved spine through perimeter readiness and visible road
    /// construction. Returns true when it changed state or owns the current
    /// neighborhood planning opportunity.
    pub(super) fn advance_neighborhood_spine(&mut self) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        let Some(spine) = plan.spine.as_ref() else {
            return false;
        };
        if plan.next_segment != plan.segments.len() || plan.segments.len() != 4 {
            return false;
        }
        if spine.validate().is_err() || !self.spine_corners_match_plan(spine) {
            return false;
        }

        if matches!(spine.stage, SpineStage::Reserved) {
            if !self.spine_perimeter_ready() {
                return false;
            }
            let Some(hosts) = self.resolve_spine_hosts(&plan.segments) else {
                return false;
            };
            if let Some(active) = self.state.neighborhood_plan.as_mut() {
                if let Some(spine) = active.spine.as_mut() {
                    spine.stage = SpineStage::PerimeterReady {
                        host_road_ids: hosts,
                    };
                }
            }
            self.state.revision = self.state.revision.saturating_add(1);
        }

        let current = self
            .state
            .neighborhood_plan
            .as_ref()
            .and_then(|plan| plan.spine.as_ref())
            .map(|spine| spine.stage.clone());
        let Some(SpineStage::PerimeterReady { host_road_ids }) = current else {
            return false;
        };
        if !self.spine_hosts_match_segments(&host_road_ids) {
            return false;
        }
        self.start_neighborhood_spine(&host_road_ids)
    }

    fn start_neighborhood_spine(&mut self, host_road_ids: &[u32; 2]) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        let Some(spine) = plan.spine.as_ref() else {
            return false;
        };
        let Ok(layout) = spine.validate() else {
            return false;
        };
        let lot_count = layout.lots.len();
        // Preflight every resource consumed by the two splits, new spine road,
        // visible road project, and promised frontage lots before either host
        // road is mutated.
        if self.state.pending_interior_access.is_some()
            || self.state.roads.len().saturating_add(3) > MAX_SEGMENTS
            || self.state.parcels.len().saturating_add(lot_count) > MAX_PARCELS
            || self.state.parcel_frontages.len().saturating_add(lot_count) > MAX_PARCELS
            || !self.project_admission_available()
            || self
                .state
                .next_id
                .checked_add((6 + lot_count) as u32)
                .is_none()
        {
            return false;
        }

        let Ok(splits) = self.try_split_open_road_pair([
            (host_road_ids[0], SPINE_SPLIT_T),
            (host_road_ids[1], SPINE_SPLIT_T),
        ]) else {
            return false;
        };
        let [start, end] = [splits[0].split_node_id, splits[1].split_node_id];
        let road_id = self.add_road(start, end, "local", "survey", 0.0);
        self.state.neighborhood_road_ids.insert(road_id);
        let project_id = self.id();
        let duration = 3.0 + random_unit(&mut self.state.rng_growth) * 4.0;
        self.record_project(Project {
            id: project_id,
            kind: "road".into(),
            from: Some(start),
            to: Some(end),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.0,
            reason: "A reserved neighborhood spine connects the block’s new side streets".into(),
            duration,
            started_tick: self.state.tick,
        });
        if let Some(spine) = self
            .state
            .neighborhood_plan
            .as_mut()
            .and_then(|plan| plan.spine.as_mut())
        {
            spine.stage = SpineStage::UnderConstruction {
                host_road_ids: *host_road_ids,
                split_node_ids: [start, end],
                spine_road_id: road_id,
                project_id,
            };
        }
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }

    pub(super) fn sync_neighborhood_spine_project(
        &mut self,
        project_id: u32,
        stage: &str,
        progress: f32,
    ) {
        let road_id = self
            .state
            .neighborhood_plan
            .as_ref()
            .and_then(|plan| plan.spine.as_ref())
            .and_then(|spine| match spine.stage {
                SpineStage::UnderConstruction {
                    spine_road_id,
                    project_id: id,
                    ..
                } if id == project_id => Some(spine_road_id),
                _ => None,
            });
        if let Some(road_id) = road_id {
            if let Some(road) = self.state.roads.iter_mut().find(|road| road.id == road_id) {
                road.stage = stage.to_owned();
                road.progress = progress;
            }
        }
    }

    pub(super) fn complete_neighborhood_spine_project(&mut self, project_id: u32) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        let Some(spine) = plan.spine.as_ref() else {
            return false;
        };
        let SpineStage::UnderConstruction {
            host_road_ids,
            split_node_ids,
            spine_road_id,
            project_id: expected_project,
        } = spine.stage
        else {
            return false;
        };
        if expected_project != project_id {
            return false;
        }
        let Some(project) = self
            .state
            .projects
            .iter()
            .find(|p| p.id == project_id)
            .cloned()
        else {
            return false;
        };
        let Some(branch) = self
            .state
            .roads
            .iter()
            .find(|r| r.id == spine_road_id)
            .cloned()
        else {
            self.keep_spine_project_retryable(project_id, spine_road_id);
            return true;
        };
        let Ok(layout) = spine.validate() else {
            self.keep_spine_project_retryable(project_id, spine_road_id);
            return true;
        };
        if project.kind != "road"
            || project.progress < 1.0
            || project.from != Some(split_node_ids[0])
            || project.to != Some(split_node_ids[1])
            || branch.from != split_node_ids[0]
            || branch.to != split_node_ids[1]
            || branch.stage == "open"
            || branch.kind != "local"
            || layout.lots.len() < crate::spine_plan::MIN_SPINE_LOTS
            || self.state.parcels.len().saturating_add(layout.lots.len()) > MAX_PARCELS
            || self
                .state
                .parcel_frontages
                .len()
                .saturating_add(layout.lots.len())
                > MAX_PARCELS
            || self
                .state
                .next_id
                .checked_add(layout.lots.len() as u32)
                .is_none()
            || !self.spine_hosts_are_split_and_connected(host_road_ids, split_node_ids)
        {
            self.keep_spine_project_retryable(project_id, spine_road_id);
            return true;
        }

        let (Some(start), Some(end)) = (self.node(branch.from), self.node(branch.to)) else {
            self.keep_spine_project_retryable(project_id, spine_road_id);
            return true;
        };
        let (start_x, start_z, end_x, end_z) = (start.x, start.z, end.x, end.z);
        let mut context = self.collision_context();
        context.pending_roads.retain(|road| {
            !((road.ax == start_x && road.az == start_z && road.bx == end_x && road.bz == end_z)
                || (road.bx == start_x
                    && road.bz == start_z
                    && road.ax == end_x
                    && road.az == end_z))
        });
        context.open_roads.push(CollisionSegment {
            ax: start_x,
            az: start_z,
            bx: end_x,
            bz: end_z,
        });
        let mut promised: Vec<geometry::OrientedRect> = Vec::with_capacity(layout.lots.len());
        for lot in &layout.lots {
            let rect = geometry::OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if !self.oriented_parcel_geometry_clear_with_context_mode(&rect, &context, true)
                || promised
                    .iter()
                    .any(|other| geometry::rects_overlap(&rect, other, 2.0))
            {
                self.keep_spine_project_retryable(project_id, spine_road_id);
                return true;
            }
            promised.push(rect);
        }

        let segments = self
            .state
            .neighborhood_plan
            .as_ref()
            .unwrap()
            .segments
            .clone();
        let perimeter_ids = perimeter_road_ids(&self.state.roads, &segments);
        let mut frontage_specs = Vec::with_capacity(layout.lots.len());
        for lot in &layout.lots {
            let road_id = match lot.frontage {
                crate::spine_blocks::SpineFrontage::Perimeter(edge) if edge == 0 || edge == 2 => {
                    perimeter_ids[edge]
                }
                crate::spine_blocks::SpineFrontage::Perimeter(_) => 0,
                crate::spine_blocks::SpineFrontage::Spine(_) => spine_road_id,
            };
            let Some(road) = self.state.roads.iter().find(|road| {
                road.id == road_id
                    && if road_id == spine_road_id {
                        road.stage != "open"
                    } else {
                        road.stage == "open" && road.connected
                    }
            }) else {
                self.keep_spine_project_retryable(project_id, spine_road_id);
                return true;
            };
            let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                self.keep_spine_project_retryable(project_id, spine_road_id);
                return true;
            };
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let length_sq = dx * dx + dz * dz;
            if !length_sq.is_finite() || length_sq <= f32::EPSILON {
                self.keep_spine_project_retryable(project_id, spine_road_id);
                return true;
            }
            let along = (((lot.center.x - a.x) * dx + (lot.center.z - a.z) * dz) / length_sq)
                .clamp(0.0, 1.0);
            let side = if dx * (lot.center.z - a.z) - dz * (lot.center.x - a.x) >= 0.0 {
                1
            } else {
                -1
            };
            let access = frontage_access_node(a, b, along);
            if !self.node_connected(access) {
                self.keep_spine_project_retryable(project_id, spine_road_id);
                return true;
            }
            frontage_specs.push((road_id, along, side, access));
        }
        // The split endpoints are already connected through their perimeter
        // hosts. Opening the spine therefore cannot create disconnected travel.
        if let Some(road) = self.state.roads.iter_mut().find(|r| r.id == spine_road_id) {
            road.stage = "open".into();
            road.progress = 1.0;
        }
        self.recompute_connectivity();
        if !self
            .state
            .roads
            .iter()
            .any(|r| r.id == spine_road_id && r.connected)
        {
            if let Some(road) = self.state.roads.iter_mut().find(|r| r.id == spine_road_id) {
                road.stage = "finishing".into();
                road.progress = 0.999;
            }
            self.recompute_connectivity();
            self.keep_spine_project_retryable(project_id, spine_road_id);
            return true;
        }

        // Releasing the plan reservation immediately before insertion avoids
        // double-counting its own promised parcels in add_parcel().
        self.state.neighborhood_plan = None;
        for (edge, road_id) in perimeter_ids.into_iter().enumerate() {
            if (edge == 0 || edge == 2) && road_id != 0 {
                self.state.neighborhood_road_ids.insert(road_id);
            }
        }
        self.state.neighborhood_road_ids.insert(spine_road_id);
        for (lot, (road_id, along, side, access)) in layout.lots.iter().zip(frontage_specs) {
            let parcel_id = self.add_parcel(
                lot.center.x,
                lot.center.z,
                lot.width,
                lot.depth,
                access,
                SPINE_LOT_SUITABILITY,
                false,
            );
            assert_ne!(parcel_id, 0, "spine parcel capacity was preflighted");
            self.state.parcels.last_mut().unwrap().rotation = Some(lot.rotation);
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
        }
        self.state.revision = self.state.revision.saturating_add(1);
        self.record(
            "NeighborhoodSpineOpened",
            spine_road_id,
            format!("A shared neighborhood spine opens with {} connected frontage sites", layout.lots.len()),
        );
        true
    }

    fn keep_spine_project_retryable(&mut self, project_id: u32, spine_road_id: u32) {
        if let Some(project) = self.state.projects.iter_mut().find(|p| p.id == project_id) {
            project.progress = 0.999;
            project.stage = "finishing".into();
        }
        if let Some(road) = self.state.roads.iter_mut().find(|r| r.id == spine_road_id) {
            if road.stage != "open" {
                road.progress = 0.999;
                road.stage = "finishing".into();
            }
        }
    }

    pub(super) fn validate_neighborhood_spine(
        &self,
        plan: &NeighborhoodPlan,
    ) -> Result<(), String> {
        let Some(spine) = &plan.spine else {
            return Ok(());
        };
        let layout = spine
            .validate()
            .map_err(|_| "neighborhood spine geometry or stage is invalid")?;
        if !plan.coordinated_lots
            || plan.selected_lots.is_some()
            || !self.spine_corners_match_plan(spine)
        {
            return Err("neighborhood spine must match a canonical coordinated perimeter".into());
        }
        if self.state.parcels.len().saturating_add(layout.lots.len()) > MAX_PARCELS
            || self
                .state
                .parcel_frontages
                .len()
                .saturating_add(layout.lots.len())
                > MAX_PARCELS
        {
            return Err("neighborhood spine parcel reservation exceeds capacity".into());
        }
        let preconstruction = matches!(
            spine.stage,
            SpineStage::Reserved | SpineStage::PerimeterReady { .. }
        );
        let mut future_perimeter_roads = 0_usize;
        let mut future_perimeter_ids = 0_usize;
        if preconstruction {
            for (from, to) in &plan.segments {
                let has_road = self.state.roads.iter().any(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                });
                if !has_road {
                    future_perimeter_roads += 1;
                    let has_project = self.state.projects.iter().any(|project| {
                        project.kind == "road"
                            && project.progress < 1.0
                            && ((project.from == Some(*from) && project.to == Some(*to))
                                || (project.from == Some(*to) && project.to == Some(*from)))
                    });
                    // The project identifier already exists when construction
                    // is pending; only its completed road record remains.
                    future_perimeter_ids += 1 + usize::from(!has_project);
                }
            }
        }
        let future_spine_roads = if preconstruction { 3 } else { 0 };
        let future_spine_ids = if preconstruction { 6 } else { 0 };
        let reserved_roads = future_perimeter_roads + future_spine_roads;
        let reserved_ids = future_perimeter_ids + future_spine_ids + layout.lots.len();
        if self.state.roads.len().saturating_add(reserved_roads) > MAX_SEGMENTS
            || self
                .state
                .next_id
                .checked_add(reserved_ids as u32)
                .is_none()
        {
            return Err("neighborhood spine road or project reservation exceeds capacity".into());
        }
        if !self.spine_world_geometry_safe(plan, spine, &layout) {
            return Err("neighborhood spine geometry conflicts with the saved world".into());
        }
        match &spine.stage {
            SpineStage::Reserved => Ok(()),
            SpineStage::PerimeterReady { host_road_ids } => {
                if plan.next_segment != plan.segments.len()
                    || !self.spine_hosts_match_segments(host_road_ids)
                    || !self.spine_perimeter_ready()
                {
                    return Err("neighborhood spine host references are invalid".into());
                }
                Ok(())
            }
            SpineStage::UnderConstruction {
                host_road_ids,
                split_node_ids,
                spine_road_id,
                project_id,
            } => {
                if plan.next_segment != plan.segments.len()
                    || !self.spine_hosts_are_split_and_connected(*host_road_ids, *split_node_ids)
                {
                    return Err("neighborhood spine split host references are invalid".into());
                }
                for edge in [0_usize, 2] {
                    let (from, to) = plan.segments[edge];
                    if !self.state.roads.iter().any(|road| {
                        road.stage == "open"
                            && road.connected
                            && ((road.from == from && road.to == to)
                                || (road.from == to && road.to == from))
                    }) {
                        return Err("neighborhood spine perimeter edge is no longer open".into());
                    }
                }
                for (slot, point) in layout.spine.iter().enumerate() {
                    let Some(node) = self.node(split_node_ids[slot]) else {
                        return Err("neighborhood spine split junction is missing".into());
                    };
                    if (node.x - point.x).hypot(node.z - point.z) > CORNER_TOLERANCE {
                        return Err(
                            "neighborhood spine split junction differs from its recipe".into()
                        );
                    }
                }
                let branch = self
                    .state
                    .roads
                    .iter()
                    .find(|r| r.id == *spine_road_id)
                    .ok_or_else(|| "neighborhood spine road is missing".to_string())?;
                let project = self
                    .state
                    .projects
                    .iter()
                    .find(|p| p.id == *project_id)
                    .ok_or_else(|| "neighborhood spine project is missing".to_string())?;
                if branch.from != split_node_ids[0]
                    || branch.to != split_node_ids[1]
                    || branch.kind != "local"
                    || branch.stage == "open"
                    || branch.connected
                    || branch.stage != project.stage
                    || (branch.progress - project.progress).abs() > 1.0e-6
                    || project.kind != "road"
                    || project.from != Some(split_node_ids[0])
                    || project.to != Some(split_node_ids[1])
                    || project.progress < 0.0
                    || project.progress >= 1.0
                {
                    return Err("neighborhood spine construction project is inconsistent".into());
                }
                if self.state.roads.len() > MAX_SEGMENTS
                    || self
                        .state
                        .projects
                        .iter()
                        .filter(|p| p.progress < 1.0)
                        .count()
                        > MAX_PROJECTS
                    || self
                        .state
                        .next_id
                        .checked_add(layout.lots.len() as u32)
                        .is_none()
                {
                    return Err(
                        "neighborhood spine construction reservation exceeds capacity".into(),
                    );
                }
                Ok(())
            }
        }
    }

    fn spine_corners_match_plan(&self, spine: &SpinePlan) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        if plan.segments.len() != 4 {
            return false;
        }
        for (index, (from, to)) in plan.segments.iter().enumerate() {
            let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                return false;
            };
            let corner = spine.corners[index];
            if (corner.x - a.x).abs() > CORNER_TOLERANCE
                || (corner.z - a.z).abs() > CORNER_TOLERANCE
            {
                return false;
            }
            let next = spine.corners[(index + 1) % 4];
            if (next.x - b.x).abs() > CORNER_TOLERANCE || (next.z - b.z).abs() > CORNER_TOLERANCE {
                return false;
            }
        }
        true
    }

    fn resolve_spine_hosts(&self, segments: &[(u32, u32)]) -> Option<[u32; 2]> {
        if segments.len() != 4 {
            return None;
        }
        let mut ids = [0; 2];
        for (slot, edge) in [3_usize, 1].into_iter().enumerate() {
            let (from, to) = segments[edge];
            let road = self.state.roads.iter().find(|road| {
                road.stage == "open"
                    && road.connected
                    && road.kind != "rail"
                    && ((road.from == from && road.to == to)
                        || (road.from == to && road.to == from))
            })?;
            ids[slot] = road.id;
        }
        (ids[0] != ids[1]).then_some(ids)
    }

    fn spine_hosts_match_segments(&self, ids: &[u32; 2]) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        [3_usize, 1].into_iter().enumerate().all(|(slot, edge)| {
            let Some((from, to)) = plan.segments.get(edge) else {
                return false;
            };
            self.state.roads.iter().any(|road| {
                road.id == ids[slot]
                    && road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            })
        }) && ids[0] != ids[1]
    }

    fn spine_perimeter_ready(&self) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        plan.segments.iter().all(|(from, to)| {
            self.state.roads.iter().any(|road| {
                road.stage == "open"
                    && road.connected
                    && road.kind != "rail"
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            })
        })
    }

    fn spine_hosts_are_split_and_connected(&self, ids: [u32; 2], split_nodes: [u32; 2]) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        [3_usize, 1].into_iter().enumerate().all(|(slot, edge)| {
            let (from, to) = plan.segments[edge];
            let (Some(a), Some(b), Some(split)) =
                (self.node(from), self.node(to), self.node(split_nodes[slot]))
            else {
                return false;
            };
            let expected_x = (a.x + b.x) * 0.5;
            let expected_z = (a.z + b.z) * 0.5;
            if (split.x - expected_x).hypot(split.z - expected_z) > CORNER_TOLERANCE {
                return false;
            }
            let halves = self
                .state
                .roads
                .iter()
                .filter(|road| {
                    road.stage == "open"
                        && road.connected
                        && road.kind != "rail"
                        && ((road.from == from && road.to == split_nodes[slot])
                            || (road.to == from && road.from == split_nodes[slot])
                            || (road.from == split_nodes[slot] && road.to == to)
                            || (road.to == split_nodes[slot] && road.from == to))
                })
                .collect::<Vec<_>>();
            let touches_from = halves
                .iter()
                .filter(|road| {
                    (road.from == from && road.to == split_nodes[slot])
                        || (road.to == from && road.from == split_nodes[slot])
                })
                .count();
            let touches_to = halves
                .iter()
                .filter(|road| {
                    (road.from == to && road.to == split_nodes[slot])
                        || (road.to == to && road.from == split_nodes[slot])
                })
                .count();
            halves.len() == 2
                && touches_from == 1
                && touches_to == 1
                && halves.iter().any(|road| road.id == ids[slot])
        })
    }

    pub(super) fn spine_world_geometry_safe(
        &self,
        plan: &NeighborhoodPlan,
        spine: &SpinePlan,
        layout: &crate::spine_blocks::SpineBlockLayout,
    ) -> bool {
        let context = self.collision_context();
        self.spine_world_geometry_safe_with_context(plan, spine, layout, &context)
    }

    pub(super) fn spine_world_geometry_safe_with_context(
        &self,
        plan: &NeighborhoodPlan,
        spine: &SpinePlan,
        layout: &crate::spine_blocks::SpineBlockLayout,
        context: &CollisionContext,
    ) -> bool {
        let mut lots: Vec<geometry::OrientedRect> = Vec::with_capacity(layout.lots.len());
        for lot in &layout.lots {
            let rect = geometry::OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if !self.oriented_parcel_geometry_clear_with_context_mode(&rect, &context, true)
                || lots
                    .iter()
                    .any(|other| geometry::rects_overlap(&rect, other, 2.0))
            {
                return false;
            }
            lots.push(rect);
        }

        let [a, b] = layout.spine;
        let length = (b.x - a.x).hypot(b.z - a.z);
        let corridor = geometry::OrientedRect {
            x: (a.x + b.x) * 0.5,
            z: (a.z + b.z) * 0.5,
            width: length,
            depth: 9.0,
            rotation: (-(b.z - a.z)).atan2(b.x - a.x),
        };
        let (min_x, min_z, max_x, max_z) = corridor.bounds();
        let (corridor_a, corridor_b) = (layout.spine[0], layout.spine[1]);
        let protected = geometry::OrientedRect {
            x: -305.0,
            z: -310.0,
            width: 90.0,
            depth: 80.0,
            rotation: 0.0,
        };
        if !corridor.finite_positive()
            || min_x < MIN + 4.0
            || min_z < MIN + 4.0
            || max_x > MAX - 4.0
            || max_z > MAX - 4.0
            || !self.parcel_clear_of_river(corridor.x, corridor.z, max_x - min_x, max_z - min_z)
            || !parcel_envelopes_clear_of_road(
                context,
                corridor_a.x,
                corridor_a.z,
                corridor_b.x,
                corridor_b.z,
            )
            || geometry::rects_overlap(&corridor, &protected, 0.0)
        {
            return false;
        }

        let split_data = match spine.stage {
            SpineStage::UnderConstruction {
                host_road_ids,
                split_node_ids,
                spine_road_id,
                ..
            } => Some((host_road_ids, split_node_ids, spine_road_id)),
            _ => None,
        };
        let is_perimeter_or_spine_road = |road: &Road| {
            if let Some((host_ids, split_ids, spine_road_id)) = split_data {
                if road.id == spine_road_id || host_ids.contains(&road.id) {
                    return true;
                }
                if split_ids.iter().any(|split| {
                    plan.segments.iter().any(|(from, to)| {
                        (road.from == *from && road.to == *split)
                            || (road.to == *from && road.from == *split)
                            || (road.from == *to && road.to == *split)
                            || (road.to == *to && road.from == *split)
                    })
                }) {
                    return true;
                }
            }
            plan.segments.iter().any(|(from, to)| {
                (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
            })
        };
        for road in self
            .state
            .roads
            .iter()
            .filter(|road| road.stage == "open" && road.kind != "rail")
        {
            if is_perimeter_or_spine_road(road) {
                continue;
            }
            let (Some(from), Some(to)) = (self.node(road.from), self.node(road.to)) else {
                return false;
            };
            if geometry::segments_distance_coords(a.x, a.z, b.x, b.z, from.x, from.z, to.x, to.z)
                < 8.0
            {
                return false;
            }
        }
        let current_project_id = match spine.stage {
            SpineStage::UnderConstruction { project_id, .. } => Some(project_id),
            _ => None,
        };
        for project in self
            .state
            .projects
            .iter()
            .filter(|project| project.progress < 1.0 && Some(project.id) != current_project_id)
        {
            if plan.segments.iter().any(|(a, b)| {
                matches!((project.from, project.to), (Some(from), Some(to))
                    if (from == *a && to == *b) || (from == *b && to == *a))
            }) {
                continue;
            }
            let (Some(from), Some(to)) = (
                project.from.and_then(|id| self.node(id)),
                project.to.and_then(|id| self.node(id)),
            ) else {
                continue;
            };
            if geometry::segments_distance_coords(a.x, a.z, b.x, b.z, from.x, from.z, to.x, to.z)
                < 8.0
            {
                return false;
            }
        }
        true
    }
}

fn perimeter_road_ids(roads: &[Road], segments: &[(u32, u32)]) -> [u32; 4] {
    let mut ids = [0; 4];
    for (index, (from, to)) in segments.iter().enumerate().take(4) {
        if let Some(road) = roads.iter().find(|road| {
            road.stage == "open"
                && road.connected
                && ((road.from == *from && road.to == *to)
                    || (road.from == *to && road.to == *from))
        }) {
            ids[index] = road.id;
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_fixture(seed: u32) -> (Engine, [u32; 4]) {
        let mut engine = Engine::new_legacy(seed);
        let gateway = engine
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "gateway")
            .unwrap()
            .id;
        let points = [
            (270.0, 200.0),
            (352.0, 200.0),
            (352.0, 282.0),
            (270.0, 282.0),
        ];
        let corners = points.map(|(x, z)| engine.add_node(x, z, "junction"));
        let connector = engine.add_road(gateway, corners[0], "local", "open", 1.0);
        let perimeter = (0..4)
            .map(|index| {
                engine.add_road(
                    corners[index],
                    corners[(index + 1) % 4],
                    "local",
                    "open",
                    1.0,
                )
            })
            .collect::<Vec<_>>();
        engine.recompute_connectivity();
        assert!(
            engine
                .state
                .roads
                .iter()
                .find(|road| road.id == connector)
                .unwrap()
                .connected
        );
        assert!(perimeter.iter().all(|id| engine
            .state
            .roads
            .iter()
            .find(|road| road.id == *id)
            .unwrap()
            .connected));
        let coords = corners.map(|id| {
            let node = engine.node(id).unwrap();
            crate::blocks::BlockPoint {
                x: node.x,
                z: node.z,
            }
        });
        let segments = (0..4)
            .map(|index| (corners[index], corners[(index + 1) % 4]))
            .collect();
        engine.state.neighborhood_road_ids.extend(perimeter);
        engine.state.neighborhood_plan = Some(NeighborhoodPlan {
            anchor_node: corners[0],
            segments,
            next_segment: 4,
            coordinated_lots: true,
            selected_lots: None,
            spine: Some(SpinePlan {
                corners: coords,
                recipe_version: crate::spine_plan::RECIPE_VERSION,
                stage: SpineStage::Reserved,
            }),
        });
        engine.state.revision = engine.state.revision.saturating_add(1);
        (engine, corners)
    }

    #[test]
    fn spine_lifecycle_restores_each_stage_and_admits_all_twenty_frontages() {
        let (engine, corners) = square_fixture(9191);
        engine.validate().unwrap();
        let reserved_save = engine.save_json();
        let mut restored = Engine::restore_json(&reserved_save).unwrap();
        assert_eq!(restored.save_json(), reserved_save);
        let host_ids = [
            restored
                .state
                .roads
                .iter()
                .find(|road| road.from == corners[3] && road.to == corners[0])
                .unwrap()
                .id,
            restored
                .state
                .roads
                .iter()
                .find(|road| road.from == corners[1] && road.to == corners[2])
                .unwrap()
                .id,
        ];
        restored
            .state
            .neighborhood_plan
            .as_mut()
            .unwrap()
            .spine
            .as_mut()
            .unwrap()
            .stage = SpineStage::PerimeterReady {
            host_road_ids: host_ids,
        };
        let ready_save = restored.save_json();
        restored = Engine::restore_json(&ready_save).unwrap();
        assert_eq!(restored.save_json(), ready_save);
        assert!(restored.advance_neighborhood_spine());
        let stage = &restored
            .state
            .neighborhood_plan
            .as_ref()
            .unwrap()
            .spine
            .as_ref()
            .unwrap()
            .stage;
        let SpineStage::UnderConstruction {
            spine_road_id,
            project_id,
            ..
        } = *stage
        else {
            panic!("perimeter-ready lifecycle should start the visible spine project");
        };
        let road = restored
            .state
            .roads
            .iter()
            .find(|road| road.id == spine_road_id)
            .unwrap();
        let project = restored
            .state
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .unwrap();
        assert_ne!(road.stage, "open");
        assert!(!road.connected);
        assert_eq!(road.stage, project.stage);
        assert_eq!(road.progress, project.progress);
        restored.validate().unwrap();

        let construction_save = restored.save_json();
        let mut resumed = Engine::restore_json(&construction_save).unwrap();
        assert_eq!(resumed.save_json(), construction_save);
        let parcels_before = resumed.state.parcels.len();
        let frontages_before = resumed.state.parcel_frontages.len();
        for _ in 0..16 {
            if resumed.state.neighborhood_plan.is_none() {
                break;
            }
            resumed.advance_projects();
        }
        assert!(
            resumed.state.neighborhood_plan.is_none(),
            "stage={:?} projects={:?}",
            resumed
                .state
                .neighborhood_plan
                .as_ref()
                .and_then(|p| p.spine.as_ref())
                .map(|s| &s.stage),
            resumed
                .state
                .projects
                .iter()
                .map(|p| (p.id, p.kind.as_str(), p.progress, p.stage.as_str()))
                .collect::<Vec<_>>()
        );
        let opened = resumed
            .state
            .roads
            .iter()
            .find(|road| road.id == spine_road_id)
            .unwrap();
        assert_eq!(opened.stage, "open");
        assert!(opened.connected);
        assert_eq!(resumed.state.parcels.len() - parcels_before, 20);
        assert_eq!(resumed.state.parcel_frontages.len() - frontages_before, 20);
        assert!(
            resumed
                .state
                .parcel_frontages
                .iter()
                .filter(|frontage| frontage.road_id == spine_road_id)
                .count()
                >= 4
        );
        assert!(
            resumed
                .state
                .parcel_frontages
                .iter()
                .filter(|frontage| frontage.road_id != spine_road_id
                    && resumed
                        .state
                        .neighborhood_road_ids
                        .contains(&frontage.road_id))
                .count()
                > 0
        );
        resumed.validate().unwrap();
        let complete = resumed.save_json();
        let mut a = Engine::restore_json(&complete).unwrap();
        let mut b = Engine::restore_json(&complete).unwrap();
        for _ in 0..200 {
            a.step(1);
            b.step(1);
        }
        assert_eq!(a.save_json(), b.save_json());
    }

    #[test]
    fn temporary_project_pressure_does_not_invalidate_a_reserved_spine_save() {
        let (mut engine, _) = square_fixture(9195);
        let template = engine.state.projects[0].clone();
        while engine
            .state
            .projects
            .iter()
            .filter(|project| project.progress < 1.0)
            .count()
            < MAX_PROJECTS
        {
            let mut project = template.clone();
            project.id = engine.id();
            engine.state.projects.push(project);
        }
        assert!(!engine.project_admission_available());
        let save = engine.save_json();
        let restored =
            Engine::restore_json(&save).expect("slot pressure should leave the plan retryable");
        assert_eq!(restored.save_json(), save);
    }

    #[test]
    fn reserved_spine_corridor_rejects_a_parcel_in_the_street_gap() {
        let (mut engine, _) = square_fixture(9196);
        let plan = engine.state.neighborhood_plan.clone().unwrap();
        let id = engine.id();
        let spine = plan.spine.as_ref().unwrap();
        let layout = spine.validate().unwrap();
        let (a, b) = (layout.spine[0], layout.spine[1]);
        engine.state.parcels.push(crate::Parcel {
            id,
            x: (a.x + b.x) * 0.5,
            z: (a.z + b.z) * 0.5,
            width: 4.0,
            depth: 4.0,
            rotation: None,
            access_node: plan.anchor_node,
            suitability: 0.7,
            protected: false,
            reservation: None,
        });
        assert!(!engine.spine_world_geometry_safe(&plan, spine, &layout));
    }

    #[test]
    fn completed_cursor_with_pending_fourth_road_project_restores() {
        let (mut engine, corners) = square_fixture(9193);
        let road_id = engine
            .state
            .roads
            .iter()
            .find(|road| road.from == corners[3] && road.to == corners[0])
            .unwrap()
            .id;
        let project_id = engine.id();
        let road = engine
            .state
            .roads
            .iter_mut()
            .find(|road| road.id == road_id)
            .unwrap();
        road.stage = "survey".into();
        road.progress = 0.2;
        road.connected = false;
        engine.record_project(Project {
            id: project_id,
            kind: "road".into(),
            from: Some(corners[3]),
            to: Some(corners[0]),
            parcel_id: None,
            stage: "survey".into(),
            progress: 0.2,
            reason: "pending final perimeter edge".into(),
            duration: 5.0,
            started_tick: engine.state.tick,
        });
        engine.validate().unwrap();
        let restored = Engine::restore_json(&engine.save_json()).unwrap();
        assert_eq!(restored.save_json(), engine.save_json());
    }

    #[test]
    fn spine_stage_references_are_validated_and_failed_pair_is_atomic() {
        let (mut engine, corners) = square_fixture(9192);
        let host_ids = [
            engine
                .state
                .roads
                .iter()
                .find(|road| road.from == corners[3] && road.to == corners[0])
                .unwrap()
                .id,
            engine
                .state
                .roads
                .iter()
                .find(|road| road.from == corners[1] && road.to == corners[2])
                .unwrap()
                .id,
        ];
        engine
            .state
            .neighborhood_plan
            .as_mut()
            .unwrap()
            .spine
            .as_mut()
            .unwrap()
            .stage = SpineStage::PerimeterReady {
            host_road_ids: host_ids,
        };
        engine.validate().unwrap();
        let before = engine.save_json();
        let SpineStage::PerimeterReady { host_road_ids } = engine
            .state
            .neighborhood_plan
            .as_ref()
            .unwrap()
            .spine
            .as_ref()
            .unwrap()
            .stage
        else {
            unreachable!()
        };
        let result = engine.try_split_open_road_pair([(host_road_ids[0], 0.5), (u32::MAX, 0.5)]);
        assert_eq!(result, Err(RoadSplitError::ActivePlan));
        assert_eq!(engine.save_json(), before);

        let mut malformed = serde_json::from_str::<serde_json::Value>(&before).unwrap();
        malformed["neighborhood_plan"]["spine"]["stage"]["PerimeterReady"]["host_road_ids"][0] =
            serde_json::json!(u32::MAX);
        assert!(Engine::restore_json(&malformed.to_string()).is_err());
    }

    #[test]
    fn reversed_host_directions_keep_split_and_frontage_access_valid() {
        let (mut engine, corners) = square_fixture(9194);
        let host_ids = [
            engine
                .state
                .roads
                .iter()
                .find(|road| road.from == corners[3] && road.to == corners[0])
                .unwrap()
                .id,
            engine
                .state
                .roads
                .iter()
                .find(|road| road.from == corners[1] && road.to == corners[2])
                .unwrap()
                .id,
        ];
        for id in host_ids {
            let road = engine
                .state
                .roads
                .iter_mut()
                .find(|road| road.id == id)
                .unwrap();
            std::mem::swap(&mut road.from, &mut road.to);
        }
        engine.recompute_connectivity();
        engine.validate().unwrap();
        assert!(engine.advance_neighborhood_spine());
        let project_id = match engine
            .state
            .neighborhood_plan
            .as_ref()
            .unwrap()
            .spine
            .as_ref()
            .unwrap()
            .stage
        {
            SpineStage::UnderConstruction { project_id, .. } => project_id,
            _ => unreachable!(),
        };
        engine.validate().unwrap();
        for _ in 0..16 {
            engine.advance_projects();
            if engine.state.neighborhood_plan.is_none() {
                break;
            }
        }
        assert!(
            engine.state.neighborhood_plan.is_none(),
            "spine project {project_id} did not complete"
        );
        engine.validate().unwrap();
    }
}
