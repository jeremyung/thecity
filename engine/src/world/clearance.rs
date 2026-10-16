use crate::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CollisionSegment {
    pub(crate) ax: f32,
    pub(crate) az: f32,
    pub(crate) bx: f32,
    pub(crate) bz: f32,
}

/// Immutable geometry inputs reused while evaluating several candidate
/// neighborhood orientations against the same world state.
#[derive(Clone, Debug)]
pub(crate) struct CollisionContext {
    pub(crate) parcel_footprints: Vec<world::geometry::OrientedRect>,
    pub(crate) open_roads: Vec<CollisionSegment>,
    pub(crate) pending_roads: Vec<CollisionSegment>,
}

impl Engine {
    pub(crate) fn parcel_geometry_clear(&self, x: f32, z: f32, w: f32, d: f32) -> bool {
        if !self.parcel_clear_of_river(x, z, w, d) {
            return false;
        }
        if x - w * 0.5 < MIN + 4.0
            || x + w * 0.5 > MAX - 4.0
            || z - d * 0.5 < MIN + 4.0
            || z + d * 0.5 > MAX - 4.0
        {
            return false;
        }
        if x + w * 0.5 > -350.0
            && x - w * 0.5 < -260.0
            && z + d * 0.5 > -350.0
            && z - d * 0.5 < -270.0
        {
            return false;
        }
        let candidate = world::geometry::OrientedRect {
            x,
            z,
            width: w,
            depth: d,
            rotation: 0.0,
        };
        if self
            .state
            .parcels
            .iter()
            .any(|p| world::geometry::rects_overlap(&candidate, &parcel_envelope(p), 2.0))
        {
            return false;
        }
        let clearance = (w.max(d) * 0.5 + 4.0).max(9.0);
        for road in self.state.roads.iter().filter(|r| r.stage == Stage::Open) {
            let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
            else {
                continue;
            };
            if point_segment_distance(x, z, a.x, a.z, b.x, b.z) < clearance {
                return false;
            }
        }
        for project in self
            .state
            .projects
            .iter()
            .filter(|p| p.kind == ProjectKind::Road && p.progress < 1.0)
        {
            let (Some(from), Some(to)) = (project.from, project.to) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                continue;
            };
            if point_segment_distance(x, z, a.x, a.z, b.x, b.z) < clearance + 2.0 {
                return false;
            }
        }
        if !self.neighborhood_corridor_clear_for_parcel(x, z, w, d) {
            return false;
        }
        let candidate = world::geometry::OrientedRect {
            x,
            z,
            width: w,
            depth: d,
            rotation: 0.0,
        };
        if !self.interior_plan_allows_parcel(&candidate, false) {
            return false;
        }
        true
    }

    pub(crate) fn oriented_parcel_geometry_clear(
        &self,
        rect: &world::geometry::OrientedRect,
    ) -> bool {
        if !rect.finite_positive()
            || !self.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth)
        {
            return false;
        }
        let (min_x, min_z, max_x, max_z) = rect.bounds();
        if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
            return false;
        }
        let protected = world::geometry::OrientedRect {
            x: -305.0,
            z: -310.0,
            width: 90.0,
            depth: 80.0,
            rotation: 0.0,
        };
        if world::geometry::rects_overlap(rect, &protected, 0.0) {
            return false;
        }
        for parcel in &self.state.parcels {
            // New footprints also respect the actual rendered orientation of
            // legacy frontage, without rewriting historical save geometry.
            let mut existing = parcel_envelope(parcel);
            if parcel.rotation.is_none() {
                // Legacy saves validate their stored axis-aligned envelope.
                // Respect that reservation as well as the rendered footprint;
                // otherwise an admitted lot can fail validation on restore.
                if world::geometry::rects_overlap(rect, &existing, 2.0) {
                    return false;
                }
                existing.rotation = self
                    .state
                    .buildings
                    .iter()
                    .find(|building| building.parcel_id == parcel.id)
                    .map(|building| building.rotation)
                    .unwrap_or_else(|| self.frontage_rotation(parcel.id));
            }
            if world::geometry::rects_overlap(rect, &existing, 2.0) {
                return false;
            }
        }
        for road in self
            .state
            .roads
            .iter()
            .filter(|road| road.stage == Stage::Open)
        {
            let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                continue;
            };
            if world::geometry::rect_segment_distance(rect, a.x, a.z, b.x, b.z) < 5.0 {
                return false;
            }
        }
        for project in self
            .state
            .projects
            .iter()
            .filter(|p| p.kind == ProjectKind::Road && p.progress < 1.0)
        {
            let Some((from, to)) = project.from.zip(project.to) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                continue;
            };
            if world::geometry::rect_segment_distance(rect, a.x, a.z, b.x, b.z) < 7.0 {
                return false;
            }
        }
        self.neighborhood_corridor_clear_for_parcel(rect.x, rect.z, max_x - min_x, max_z - min_z)
            && self.interior_plan_allows_parcel(rect, false)
    }

    pub(crate) fn collision_context(&self) -> CollisionContext {
        let mut node_positions = BTreeMap::new();
        for node in &self.state.nodes {
            node_positions.entry(node.id).or_insert((node.x, node.z));
        }
        let mut roads_by_id = BTreeMap::new();
        for road in &self.state.roads {
            roads_by_id.entry(road.id).or_insert((road.from, road.to));
        }
        let mut building_rotations = BTreeMap::new();
        for building in &self.state.buildings {
            building_rotations
                .entry(building.parcel_id)
                .or_insert(building.rotation);
        }
        let mut frontages = BTreeMap::new();
        for frontage in &self.state.parcel_frontages {
            frontages
                .entry(frontage.parcel_id)
                .or_insert((frontage.road_id, frontage.side));
        }

        let mut parcel_footprints = Vec::with_capacity(self.state.parcels.len());
        for parcel in &self.state.parcels {
            let mut footprint = parcel_envelope(parcel);
            if parcel.rotation.is_none() {
                parcel_footprints.push(footprint);
                footprint.rotation = building_rotations
                    .get(&parcel.id)
                    .copied()
                    .or_else(|| {
                        let (road_id, side) = *frontages.get(&parcel.id)?;
                        let (from, to) = *roads_by_id.get(&road_id)?;
                        let (ax, az) = *node_positions.get(&from)?;
                        let (bx, bz) = *node_positions.get(&to)?;
                        let dx = bx - ax;
                        let dz = bz - az;
                        Some((side as f32 * dz).atan2(-side as f32 * dx))
                    })
                    .unwrap_or(0.0);
            }
            parcel_footprints.push(footprint);
        }

        let mut open_roads = Vec::new();
        for road in self
            .state
            .roads
            .iter()
            .filter(|road| road.stage == Stage::Open)
        {
            let (Some((ax, az)), Some((bx, bz))) =
                (node_positions.get(&road.from), node_positions.get(&road.to))
            else {
                continue;
            };
            open_roads.push(CollisionSegment {
                ax: *ax,
                az: *az,
                bx: *bx,
                bz: *bz,
            });
        }
        let mut pending_roads = Vec::new();
        for project in self
            .state
            .projects
            .iter()
            .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        {
            let Some((from, to)) = project.from.zip(project.to) else {
                continue;
            };
            let (Some((ax, az)), Some((bx, bz))) =
                (node_positions.get(&from), node_positions.get(&to))
            else {
                continue;
            };
            pending_roads.push(CollisionSegment {
                ax: *ax,
                az: *az,
                bx: *bx,
                bz: *bz,
            });
        }

        CollisionContext {
            parcel_footprints,
            open_roads,
            pending_roads,
        }
    }

    /// Protect the legacy and rendered parcel envelopes by their exact
    /// centerline distance. `collision_context` retains both orientations for
    /// legacy parcels whose saved rotation is absent.
    #[cfg(feature = "legacy")]
    pub(crate) fn road_corridor_clear_of_parcels_with_context(
        &self,
        ax: f32,
        az: f32,
        bx: f32,
        bz: f32,
        context: &CollisionContext,
    ) -> bool {
        !self.interior_plan_reserves_road(ax, az, bx, bz)
            && self.interior_plan_lots_clear_of_road(ax, az, bx, bz)
            && parcel_envelopes_clear_of_road(context, ax, az, bx, bz)
    }

    pub(crate) fn oriented_parcel_geometry_clear_with_context(
        &self,
        rect: &world::geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        self.oriented_parcel_geometry_clear_with_context_mode(rect, context, false)
    }

    pub(crate) fn oriented_parcel_geometry_clear_with_context_mode(
        &self,
        rect: &world::geometry::OrientedRect,
        context: &CollisionContext,
        completing_reserved_block: bool,
    ) -> bool {
        if !rect.finite_positive()
            || !self.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth)
        {
            return false;
        }
        let (min_x, min_z, max_x, max_z) = rect.bounds();
        if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
            return false;
        }
        let protected = world::geometry::OrientedRect {
            x: -305.0,
            z: -310.0,
            width: 90.0,
            depth: 80.0,
            rotation: 0.0,
        };
        if world::geometry::rects_overlap(rect, &protected, 0.0) {
            return false;
        }
        for existing in &context.parcel_footprints {
            if world::geometry::rects_overlap(rect, existing, 2.0) {
                return false;
            }
        }
        if !self.interior_plan_allows_parcel(rect, completing_reserved_block) {
            return false;
        }
        for road in &context.open_roads {
            if world::geometry::rect_segment_distance(rect, road.ax, road.az, road.bx, road.bz)
                < 5.0
            {
                return false;
            }
        }
        for project in &context.pending_roads {
            if world::geometry::rect_segment_distance(
                rect, project.ax, project.az, project.bx, project.bz,
            ) < 7.0
            {
                return false;
            }
        }
        completing_reserved_block
            || self.neighborhood_corridor_clear_for_parcel(
                rect.x,
                rect.z,
                max_x - min_x,
                max_z - min_z,
            )
    }

    /// Progressive lots retain the historic centerline clearance while using
    /// their true frontage orientation for parcel, bounds, river, and
    /// protected-land checks. The oriented helper's segment-distance checks
    /// add a footprint safety check; the centerline thresholds below preserve
    /// the ordinary infill spacing policy exactly.
    #[cfg(feature = "legacy")]
    pub(crate) fn progressive_frontage_geometry_clear(
        &self,
        rect: &world::geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        if !self.oriented_parcel_geometry_clear_with_context(rect, context) {
            return false;
        }
        self.progressive_frontage_center_clear(rect, context)
    }

    pub(crate) fn progressive_frontage_geometry_clear_for_interior(
        &self,
        rect: &world::geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        if !self.oriented_parcel_geometry_clear_with_context_mode(rect, context, true) {
            return false;
        }
        self.progressive_frontage_center_clear(rect, context)
    }

    pub(crate) fn progressive_frontage_center_clear(
        &self,
        rect: &world::geometry::OrientedRect,
        context: &CollisionContext,
    ) -> bool {
        let clearance = (rect.width.max(rect.depth) * 0.5 + 4.0).max(9.0);
        context.open_roads.iter().all(|road| {
            point_segment_distance(rect.x, rect.z, road.ax, road.az, road.bx, road.bz) >= clearance
        }) && context.pending_roads.iter().all(|road| {
            point_segment_distance(rect.x, rect.z, road.ax, road.az, road.bx, road.bz)
                >= clearance + 2.0
        })
    }

    pub(crate) fn interior_plan_allows_parcel(
        &self,
        rect: &world::geometry::OrientedRect,
        completing_own_plan: bool,
    ) -> bool {
        let Some(plan) = &self.state.pending_interior_access else {
            return true;
        };
        if completing_own_plan {
            return true;
        }
        plan.selected_lots.iter().all(|lot| {
            let reserved = world::geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            !world::geometry::rects_overlap(rect, &reserved, 2.0)
        })
    }

    pub(crate) fn interior_plan_allows_frontage_candidate(
        &self,
        a: &Node,
        b: &Node,
        x: f32,
        z: f32,
        width: f32,
        depth: f32,
        side: i8,
    ) -> bool {
        if self.state.pending_interior_access.is_none() {
            return true;
        }
        let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
        self.interior_plan_allows_parcel(
            &world::geometry::OrientedRect {
                x,
                z,
                width,
                depth,
                rotation,
            },
            false,
        )
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn interior_plan_reserves_road(&self, ax: f32, az: f32, bx: f32, bz: f32) -> bool {
        let Some(plan) = &self.state.pending_interior_access else {
            return false;
        };
        let Some(road) = self
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
        else {
            return false;
        };
        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
            return true;
        };
        world::geometry::segments_distance_coords(ax, az, bx, bz, a.x, a.z, b.x, b.z) < 8.0
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn interior_plan_lots_clear_of_road(
        &self,
        ax: f32,
        az: f32,
        bx: f32,
        bz: f32,
    ) -> bool {
        self.state
            .pending_interior_access
            .as_ref()
            .is_none_or(|plan| {
                plan.selected_lots.iter().all(|lot| {
                    let rect = world::geometry::OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    };
                    world::geometry::rect_segment_distance(&rect, ax, az, bx, bz) >= 5.0
                })
            })
    }
}
