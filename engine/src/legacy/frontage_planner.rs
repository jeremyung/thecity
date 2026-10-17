use crate::*;
use std::collections::BTreeMap;

#[cfg(feature = "legacy")]
pub(crate) const MAX_FRONTAGE_ROADS_PER_EVALUATION: usize = 24;

#[cfg(feature = "legacy")]
pub(crate) const MAX_FRONTAGE_SITES_PER_EVALUATION: usize = 48;

pub(crate) const MAX_FRONTAGE_GRID_SITES_PER_ROAD: usize = 34;

pub(crate) const MAX_FRONTAGE_GAP_LOTS_PER_SIDE: usize = 8;

#[cfg(feature = "legacy")]
pub(crate) const FRONTAGE_GAP_DEPTH: f32 = 12.0;

#[cfg(feature = "legacy")]
pub(crate) const FRONTAGE_GAP_OFFSET: f32 = 15.0;

#[cfg(feature = "legacy")]
pub(crate) const FRONTAGE_SHALLOW_DEPTH: f32 = 8.0;

#[cfg(feature = "legacy")]
pub(crate) const FRONTAGE_SHALLOW_OFFSET: f32 = 12.0;

pub(crate) const MAX_FRONTAGE_SITES_PER_ROAD: usize =
    2 * MAX_FRONTAGE_GRID_SITES_PER_ROAD + 4 * MAX_FRONTAGE_GAP_LOTS_PER_SIDE;

pub(crate) const FRONTAGE_SLOT_SPACING: f32 = 16.0;

pub(crate) const FRONTAGE_ENDPOINT_CLEARANCE: f32 = 9.0;

impl Engine {
    pub(crate) fn add_frontage_parcels(&mut self, road_id: u32) {
        let Some(road) = self
            .state
            .network
            .roads
            .iter()
            .find(|r| r.id == road_id)
            .cloned()
        else {
            return;
        };
        if road.kind == RoadKind::Rail || road.kind == RoadKind::Bridge {
            return;
        }
        let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
        else {
            return;
        };
        let neighborhood_road = self.is_neighborhood_plan_road(road_id);
        let (lot_width, lot_depth) = if neighborhood_road {
            (NEIGHBORHOOD_PARCEL_SIZE, NEIGHBORHOOD_PARCEL_SIZE)
        } else {
            (14.0, 12.0)
        };
        let candidates = if neighborhood_road {
            self.frontage_candidate_sites_with_layout(
                &a,
                &b,
                lot_width,
                lot_depth,
                NEIGHBORHOOD_FRONTAGE_SPACING,
            )
        } else {
            self.frontage_candidate_sites_with_size(&a, &b, lot_width, lot_depth)
        };
        // Keep a small, bounded station-core pipeline reserved from the first
        // connected streets. Waiting until rail service opens lets ordinary
        // frontage from adjacent roads consume the only viable tower plots.
        // These remain undeveloped until service and local demand support them.
        let reserve_tower_frontage =
            self.station_tower_frontage_reservations() < MIN_STATION_TOWER_FRONTAGE_RESERVATIONS;
        let mut tower_frontage_reserved = false;
        if reserve_tower_frontage {
            if let Some(index) = self.station_tower_candidate_index(road_id, &a, &b, &candidates) {
                let tower_candidate = candidates[index];
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let length = dx.hypot(dz).max(1.0);
                let nx = -dz / length;
                let nz = dx / length;
                let x = a.x
                    + dx * tower_candidate.3
                    + nx * tower_candidate.4 as f32 * STATION_TOWER_ROAD_OFFSET;
                let z = a.z
                    + dz * tower_candidate.3
                    + nz * tower_candidate.4 as f32 * STATION_TOWER_ROAD_OFFSET;
                if self.interior_plan_allows_frontage_candidate(
                    &a,
                    &b,
                    x,
                    z,
                    STATION_TOWER_PARCEL_WIDTH,
                    STATION_TOWER_PARCEL_DEPTH,
                    tower_candidate.4,
                ) && self.parcel_geometry_clear(
                    x,
                    z,
                    STATION_TOWER_PARCEL_WIDTH,
                    STATION_TOWER_PARCEL_DEPTH,
                ) {
                    let parcel_id = self.add_parcel(
                        x,
                        z,
                        STATION_TOWER_PARCEL_WIDTH,
                        STATION_TOWER_PARCEL_DEPTH,
                        tower_candidate.2,
                        0.9,
                        false,
                    );
                    if parcel_id != 0 {
                        self.state.land.parcel_frontages.push(ParcelFrontage {
                            parcel_id,
                            road_id,
                            along: tower_candidate.3,
                            side: tower_candidate.4,
                        });
                        self.mark_access_geometry_changed();
                        tower_frontage_reserved = true;
                    }
                }
            }
        }
        // A new street gets a small pair of starter lots. Remaining slots are
        // unlocked later as occupied frontage creates sustained local demand.
        let mut center_sites = candidates.clone();
        let reserved_tower_site = tower_frontage_reserved
            .then(|| self.station_tower_candidate_index(road_id, &a, &b, &center_sites))
            .flatten()
            .map(|index| (center_sites[index].3, center_sites[index].4));
        let available_tower_envelopes = self.available_station_tower_envelopes();
        center_sites.sort_by(|left, right| {
            (left.3 - 0.5)
                .abs()
                .total_cmp(&(right.3 - 0.5).abs())
                .then_with(|| left.4.cmp(&right.4))
        });
        for (x, z, access, along, side) in center_sites
            .into_iter()
            .filter(|site| neighborhood_road || (site.3 - 0.5).abs() < 0.001)
        {
            // Leave this midpoint clear for a genuine tower footprint until
            // station service and local demand make it developable.
            if reserved_tower_site.is_some_and(|(tower_along, tower_side)| {
                side == tower_side && (along - tower_along).abs() < 0.001
            }) {
                continue;
            }
            if Self::overlaps_station_tower_envelopes(
                &available_tower_envelopes,
                x,
                z,
                lot_width,
                lot_depth,
            ) {
                continue;
            }
            if !self
                .interior_plan_allows_frontage_candidate(&a, &b, x, z, lot_width, lot_depth, side)
                || !self.parcel_geometry_clear(x, z, lot_width, lot_depth)
            {
                continue;
            }
            let parcel_id = self.add_parcel(x, z, lot_width, lot_depth, access, 0.78, false);
            if parcel_id == 0 {
                break;
            }
            self.state.land.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn frontage_candidate_sites(
        &self,
        a: &Node,
        b: &Node,
    ) -> Vec<(f32, f32, u32, f32, i8)> {
        self.frontage_candidate_sites_with_depth(a, b, 12.0)
    }

    pub(crate) fn frontage_candidate_sites_with_depth(
        &self,
        a: &Node,
        b: &Node,
        lot_depth: f32,
    ) -> Vec<(f32, f32, u32, f32, i8)> {
        self.frontage_candidate_sites_with_size(a, b, 14.0, lot_depth)
    }

    pub(crate) fn frontage_candidate_sites_with_size(
        &self,
        a: &Node,
        b: &Node,
        lot_width: f32,
        lot_depth: f32,
    ) -> Vec<(f32, f32, u32, f32, i8)> {
        self.frontage_candidate_sites_with_layout(a, b, lot_width, lot_depth, FRONTAGE_SLOT_SPACING)
    }

    pub(crate) fn frontage_candidate_sites_with_layout(
        &self,
        a: &Node,
        b: &Node,
        lot_width: f32,
        lot_depth: f32,
        spacing: f32,
    ) -> Vec<(f32, f32, u32, f32, i8)> {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let max_steps =
            (((length * 0.5 - FRONTAGE_ENDPOINT_CLEARANCE) / spacing).floor() as usize).min(8);
        let positions: Vec<f32> = (-(max_steps as isize)..=max_steps as isize)
            .map(|step| (0.5 + step as f32 * spacing / length).clamp(0.0, 1.0))
            .collect();
        let mut sites = Vec::with_capacity(positions.len() * 2);
        for along in positions {
            for side in [-1.0_f32, 1.0] {
                let x = a.x + dx * along + nx * side * 15.0;
                let z = a.z + dz * along + nz * side * 15.0;
                if !self.parcel_clear_of_river(x, z, lot_width, lot_depth) {
                    continue;
                }
                let access = frontage_access_node(a, b, along);
                sites.push((x, z, access, along, side as i8));
            }
        }
        sites
    }

    #[cfg(test)]
    pub(crate) fn frontage_sites(&self, a: &Node, b: &Node) -> Vec<(f32, f32, u32, f32, i8)> {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let mut sites = Vec::with_capacity(2);
        for side in [-1.0_f32, 1.0] {
            let along = 0.5;
            let x = a.x + dx * along + nx * side * 15.0;
            let z = a.z + dz * along + nz * side * 15.0;
            if !self.parcel_geometry_clear(x, z, 14.0, 12.0) {
                continue;
            }
            let access = frontage_access_node(a, b, along);
            sites.push((x, z, access, along, side as i8));
        }
        sites
    }

    /// Count a small number of genuinely usable lots along a proposed road.
    /// The frontage sampler is capped at 34 positions total. Both the legacy
    /// axis-aligned admission envelope and the actual road-facing footprint
    /// must be clear, so each counted site can survive ordinary parcel
    /// admission after construction. Adjacent sample positions have 16 m
    /// center spacing for 14 m frontage, leaving a 2 m gap.
    #[cfg(feature = "legacy")]
    pub(crate) fn viable_frontage_benefit_count(&self, a: &Node, b: &Node, limit: usize) -> usize {
        self.viable_frontage_benefit_sites(a, b, limit).len()
    }

    /// Return the same mutually compatible opportunities used by road admission.
    /// These are potential lots, not reservations or promised occupied floor area.
    #[cfg(feature = "legacy")]
    pub(crate) fn viable_frontage_benefit_sites(
        &self,
        a: &Node,
        b: &Node,
        limit: usize,
    ) -> Vec<world::geometry::OrientedRect> {
        if limit == 0 {
            return Vec::new();
        }
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let candidates = self.frontage_candidate_sites_with_layout(a, b, 14.0, 12.0, 16.0);
        let context = self.collision_context();
        let mut accepted_axis: Vec<world::geometry::OrientedRect> =
            Vec::with_capacity(limit.min(2));
        let mut accepted_oriented: Vec<world::geometry::OrientedRect> =
            Vec::with_capacity(limit.min(2));
        for (x, z, _, _, side) in candidates {
            let axis = world::geometry::OrientedRect {
                x,
                z,
                width: 14.0,
                depth: 12.0,
                rotation: 0.0,
            };
            if !self.parcel_geometry_clear(x, z, 14.0, 12.0) {
                continue;
            }
            if accepted_axis
                .iter()
                .any(|existing| world::geometry::rects_overlap(&axis, existing, 2.0))
                || accepted_oriented
                    .iter()
                    .any(|existing| world::geometry::rects_overlap(&axis, existing, 2.0))
            {
                continue;
            }
            let oriented = world::geometry::OrientedRect {
                rotation: (side as f32 * dz).atan2(-side as f32 * dx),
                ..axis
            };
            if !self.oriented_parcel_geometry_clear_with_context(&oriented, &context)
                || accepted_axis
                    .iter()
                    .any(|existing| world::geometry::rects_overlap(&oriented, existing, 2.0))
                || accepted_oriented
                    .iter()
                    .any(|existing| world::geometry::rects_overlap(&oriented, existing, 2.0))
            {
                continue;
            }
            accepted_axis.push(axis);
            accepted_oriented.push(oriented);
            if accepted_oriented.len() >= limit {
                return accepted_oriented;
            }
        }
        accepted_oriented
    }

    /// Project reserved footprints into each roadside strip. Subdivision only
    /// proposes sites; admission still checks their complete oriented geometry.
    #[cfg(feature = "legacy")]
    pub(crate) fn frontage_gap_candidates(
        &self,
        a: &Node,
        b: &Node,
        context: &CollisionContext,
        tower_envelopes: &[(f32, f32)],
    ) -> Vec<((f32, f32, u32, f32, i8), f32)> {
        self.frontage_gap_candidates_with_layout(
            a,
            b,
            context,
            tower_envelopes,
            FRONTAGE_GAP_DEPTH,
            FRONTAGE_GAP_OFFSET,
        )
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn frontage_gap_candidates_with_layout(
        &self,
        a: &Node,
        b: &Node,
        context: &CollisionContext,
        tower_envelopes: &[(f32, f32)],
        lot_depth: f32,
        center_offset: f32,
    ) -> Vec<((f32, f32, u32, f32, i8), f32)> {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz);
        if !length.is_finite()
            || length <= 0.0
            || !lot_depth.is_finite()
            || lot_depth <= 0.0
            || !center_offset.is_finite()
            || center_offset < lot_depth * 0.5 + 2.0
        {
            return Vec::new();
        }
        let (ux, uz) = (dx / length, dz / length);
        let (nx, nz) = (-uz, ux);
        let mut result = Vec::new();
        for side in [-1_i8, 1] {
            let mut exclusions = Vec::new();
            let towers = tower_envelopes
                .iter()
                .map(|&(x, z)| world::geometry::OrientedRect {
                    x,
                    z,
                    width: STATION_TOWER_PARCEL_WIDTH,
                    depth: STATION_TOWER_PARCEL_DEPTH,
                    rotation: 0.0,
                });
            for footprint in context.parcel_footprints.iter().copied().chain(towers) {
                let mut along_min = f32::INFINITY;
                let mut along_max = f32::NEG_INFINITY;
                let mut normal_min = f32::INFINITY;
                let mut normal_max = f32::NEG_INFINITY;
                for (x, z) in footprint.corners() {
                    let (rx, rz) = (x - a.x, z - a.z);
                    let along = rx * ux + rz * uz;
                    let normal = (rx * nx + rz * nz) * side as f32;
                    along_min = along_min.min(along);
                    along_max = along_max.max(along);
                    normal_min = normal_min.min(normal);
                    normal_max = normal_max.max(normal);
                }
                // Project obstacles touching the parcel strip, including a
                // 2m safety buffer. The default layout is 12m deep at 15m;
                // diagnostics can compare shallower alternatives without
                // changing production candidate ordering.
                let strip_min = center_offset - lot_depth * 0.5 - 2.0;
                let strip_max = center_offset + lot_depth * 0.5 + 2.0;
                if normal_max >= strip_min && normal_min <= strip_max {
                    exclusions.push(infrastructure::frontage::Interval {
                        start: along_min - 2.0,
                        end: along_max + 2.0,
                    });
                }
            }
            for interval in infrastructure::frontage::subdivide_frontage(
                length,
                &exclusions,
                FRONTAGE_ENDPOINT_CLEARANCE,
                8.0,
                14.0,
                2.2,
                MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
            ) {
                let along = (interval.start + interval.end) * 0.5 / length;
                let x = a.x + dx * along + nx * side as f32 * center_offset;
                let z = a.z + dz * along + nz * side as f32 * center_offset;
                result.push((
                    (x, z, frontage_access_node(a, b, along), along, side),
                    interval.end - interval.start,
                ));
            }
        }
        result
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn frontage_candidates_with_shallow_fallback(
        &self,
        road_id: u32,
        a: &Node,
        b: &Node,
        collision_context: &CollisionContext,
        tower_envelopes: &[(f32, f32)],
        grid_width: f32,
        grid_depth: f32,
        spacing: f32,
    ) -> (Vec<(f32, f32, u32, f32, i8)>, Vec<(f32, f32)>, bool) {
        let mut candidates =
            self.frontage_candidate_sites_with_layout(a, b, grid_width, grid_depth, spacing);
        let tower_candidate = self.station_tower_candidate_index(road_id, a, b, &candidates);
        // Preserve the existing frontage ordering and station-tower priority.
        if let Some(index) = tower_candidate {
            candidates.swap(0, index);
        }
        let grid_candidate_count = candidates.len();
        let mut dimensions = vec![(grid_width, grid_depth); candidates.len()];
        for (site, width) in self.frontage_gap_candidates(a, b, collision_context, tower_envelopes)
        {
            candidates.push(site);
            dimensions.push((width, FRONTAGE_GAP_DEPTH));
        }
        for (site, width) in self.frontage_gap_candidates_with_layout(
            a,
            b,
            collision_context,
            tower_envelopes,
            FRONTAGE_SHALLOW_DEPTH,
            FRONTAGE_SHALLOW_OFFSET,
        ) {
            let (x, z, _, _, _) = site;
            let duplicate_shape = candidates.iter().zip(dimensions.iter()).any(
                |(candidate, (old_width, old_depth))| {
                    (candidate.0 - x).hypot(candidate.1 - z) <= 1.0e-4
                        && (*old_width - width).abs() <= 1.0e-4
                        && (*old_depth - FRONTAGE_SHALLOW_DEPTH).abs() <= 1.0e-4
                },
            );
            if !duplicate_shape {
                candidates.push(site);
                dimensions.push((width, FRONTAGE_SHALLOW_DEPTH));
            }
        }
        // A legal narrow lot may fit at a grid midpoint even when projected
        // gap exclusions shift their centers or the ordinary grid lot is too
        // wide. Append the existing minimum-width shape without changing the
        // old grid/gap/shallow prefix or any admission clearance. The persisted
        // scan still visits at most MAX_FRONTAGE_SITES_PER_EVALUATION sites.
        for index in 0..grid_candidate_count {
            let site = candidates[index];
            let duplicate_shape = candidates.iter().zip(dimensions.iter()).any(|(old, size)| {
                (old.0 - site.0).hypot(old.1 - site.1) <= 1.0e-4
                    && (size.0 - 8.0).abs() <= 1.0e-4
                    && (size.1 - FRONTAGE_GAP_DEPTH).abs() <= 1.0e-4
            });
            if !duplicate_shape {
                candidates.push(site);
                dimensions.push((8.0, FRONTAGE_GAP_DEPTH));
            }
        }
        debug_assert!(candidates.len() <= MAX_FRONTAGE_SITES_PER_ROAD);
        (candidates, dimensions, tower_candidate.is_some())
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn try_progressive_frontage_infill(&mut self) -> bool {
        if self.state.land.parcels.len() >= MAX_PARCELS
            || self.state.construction.buildings.len() >= MAX_BUILDINGS
            || self.state.people.demand < 1.2
            || self
                .state
                .construction
                .buildings
                .iter()
                .filter(|b| site_active(b))
                .count()
                < 8
        {
            return false;
        }

        let mut road_index = self
            .state
            .legacy
            .frontage_road_cursor
            .min(self.state.network.roads.len());
        let mut slot_index = self.state.legacy.frontage_slot_cursor as usize;
        let mut roads_visited = 0;
        let mut sites_visited = 0;
        let mut wrapped = false;
        // This evaluation can inspect up to 48 sites. The envelope set does
        // not change unless a parcel is admitted, which returns immediately.
        let tower_envelopes = self.available_station_tower_envelopes();
        let collision_context = self.collision_context();
        let mut candidate_cache = BTreeMap::new();
        while roads_visited < MAX_FRONTAGE_ROADS_PER_EVALUATION
            && sites_visited < MAX_FRONTAGE_SITES_PER_EVALUATION
        {
            if road_index >= self.state.network.roads.len() {
                if wrapped {
                    self.state.legacy.frontage_road_cursor = 0;
                    self.state.legacy.frontage_slot_cursor = 0;
                    return false;
                }
                road_index = 0;
                slot_index = 0;
                wrapped = true;
            }
            let road = &self.state.network.roads[road_index];
            let road_id = road.id;
            if matches!(road.kind, RoadKind::Rail | RoadKind::Bridge) {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            }
            // This scan is bounded and persistent. Skip temporarily unusable
            // roads so they do not starve later streets; cursor wrap revisits
            // them after they open or reconnect.
            if road.stage != Stage::Open || !road.connected {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            }
            let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
            else {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            };
            let (lot_width, lot_depth) = if self.is_neighborhood_plan_road(road_id) {
                (NEIGHBORHOOD_PARCEL_SIZE, NEIGHBORHOOD_PARCEL_SIZE)
            } else {
                (14.0, 12.0)
            };
            let spacing = if self.is_neighborhood_plan_road(road_id) {
                NEIGHBORHOOD_FRONTAGE_SPACING
            } else {
                FRONTAGE_SLOT_SPACING
            };
            // Reuse each road's projected gaps while visiting its slots. This
            // cache lives only until a single admission or evaluation ends.
            let (candidates, dimensions, has_tower_candidate) =
                candidate_cache.entry(road_id).or_insert_with(|| {
                    self.frontage_candidates_with_shallow_fallback(
                        road_id,
                        &a,
                        &b,
                        &collision_context,
                        &tower_envelopes,
                        lot_width,
                        lot_depth,
                        spacing,
                    )
                });
            if slot_index >= candidates.len() {
                road_index += 1;
                slot_index = 0;
                roads_visited += 1;
                continue;
            }
            let (x, z, access_node, along, side) = candidates[slot_index];
            let (lot_width, lot_depth) = dimensions[slot_index];
            let tower_site = slot_index == 0 && *has_tower_candidate;
            slot_index += 1;
            sites_visited += 1;
            self.state.legacy.frontage_road_cursor = road_index;
            self.state.legacy.frontage_slot_cursor = slot_index.min(u16::MAX as usize) as u16;

            // Keep one compact station-frontage opportunity clear until rail
            // service is real. The deterministic cursor will revisit it after
            // the line opens; ordinary sites continue to infill meanwhile.
            if tower_site && !self.state.transit.station_online {
                continue;
            }
            if !tower_site
                && Self::overlaps_station_tower_envelopes(
                    &tower_envelopes,
                    x,
                    z,
                    lot_width,
                    lot_depth,
                )
            {
                continue;
            }

            let (parcel_x, parcel_z, parcel_width, parcel_depth) = if tower_site {
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let length = dx.hypot(dz).max(1.0);
                let nx = -dz / length;
                let nz = dx / length;
                (
                    a.x + dx * along + nx * side as f32 * STATION_TOWER_ROAD_OFFSET,
                    a.z + dz * along + nz * side as f32 * STATION_TOWER_ROAD_OFFSET,
                    STATION_TOWER_PARCEL_WIDTH,
                    STATION_TOWER_PARCEL_DEPTH,
                )
            } else {
                (x, z, 14.0, 12.0)
            };
            let (parcel_width, parcel_depth) = if tower_site {
                (parcel_width, parcel_depth)
            } else {
                (lot_width, lot_depth)
            };
            let parcel_rotation = if tower_site {
                None
            } else {
                Some((side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x)))
            };
            let geometry_clear = if let Some(rotation) = parcel_rotation {
                self.progressive_frontage_geometry_clear(
                    &world::geometry::OrientedRect {
                        x: parcel_x,
                        z: parcel_z,
                        width: parcel_width,
                        depth: parcel_depth,
                        rotation,
                    },
                    &collision_context,
                )
            } else {
                // Tower opportunities keep their established admission path.
                self.parcel_geometry_clear(parcel_x, parcel_z, parcel_width, parcel_depth)
            };
            if !geometry_clear {
                continue;
            }
            let nearby = self.state.construction.buildings.iter().filter(|building| {
                building.stage == Stage::Open
                    && (building.x - parcel_x).hypot(building.z - parcel_z) <= 78.0
            });
            let mut local_open = 0usize;
            let mut local_occupied = 0usize;
            for building in nearby {
                local_open += 1;
                local_occupied += usize::from(site_active(building));
            }
            if local_open < 4 || local_occupied < 3 {
                continue;
            }
            let parcel_id = self.add_parcel(
                parcel_x,
                parcel_z,
                parcel_width,
                parcel_depth,
                access_node,
                if tower_site { 0.9 } else { 0.74 },
                false,
            );
            if parcel_id == 0 {
                return false;
            }
            self.state.land.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
            if let Some(rotation) = parcel_rotation {
                if let Some(parcel) = self
                    .state
                    .land
                    .parcels
                    .iter_mut()
                    .find(|p| p.id == parcel_id)
                {
                    parcel.rotation = Some(rotation);
                }
            }
            self.state.legacy.road_infill_due =
                self.state.legacy.road_infill_due.saturating_add(1).min(9);
            self.state.revision = self.state.revision.saturating_add(1);
            return true;
        }
        self.state.legacy.frontage_road_cursor = road_index.min(self.state.network.roads.len());
        self.state.legacy.frontage_slot_cursor = slot_index.min(u16::MAX as usize) as u16;
        false
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn create_frontier_parcels(&mut self) {
        if self.state.land.parcels.len() >= MAX_PARCELS {
            return;
        }
        let mut candidates: Vec<(Node, f32)> = self
            .state
            .network
            .nodes
            .iter()
            .filter(|node| self.node_connected(node.id))
            .filter_map(|node| {
                let distance = self
                    .state
                    .construction
                    .buildings
                    .iter()
                    .filter(|building| site_active(building))
                    .map(|building| (building.x - node.x).hypot(building.z - node.z))
                    .min_by(f32::total_cmp)?;
                (distance <= 110.0).then_some((node.clone(), distance))
            })
            .collect();
        if candidates.is_empty() {
            return;
        }
        candidates.sort_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| a.id.cmp(&b.id)));
        let first = self.state.construction.growth_cursor % candidates.len();
        for attempt in 0..candidates.len() {
            let (node, _) = &candidates[(first + attempt) % candidates.len()];
            let offsets = if attempt % 2 == 0 {
                [-0.5_f32, 0.5]
            } else {
                [0.5_f32, -0.5]
            };
            for offset in offsets {
                let x = (node.x + offset * 27.0).clamp(MIN + 18.0, MAX - 18.0);
                let z = (node.z + if offset < 0.0 { 19.0 } else { -19.0 })
                    .clamp(MIN + 18.0, MAX - 18.0);
                if !self.parcel_geometry_clear(x, z, 14.0, 12.0) {
                    continue;
                }
                let suitability = 0.55 + random_unit(&mut self.state.rng_growth) * 0.4;
                let parcel_id = self.add_parcel(x, z, 14.0, 12.0, node.id, suitability, false);
                self.remember_nearest_frontage(parcel_id, node.id);
                self.state.construction.growth_cursor =
                    self.state.construction.growth_cursor.saturating_add(1);
                return;
            }
        }
    }
}
