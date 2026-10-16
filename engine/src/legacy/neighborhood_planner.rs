use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "legacy")]
pub(crate) const MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION: usize = 24;

#[cfg(feature = "legacy")]
pub(crate) const MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION: usize = 24;

#[cfg(feature = "legacy")]
pub(crate) const NEIGHBORHOOD_BLOCK_LENGTH: f32 = 82.0;

#[cfg(feature = "legacy")]
pub(crate) const NEIGHBORHOOD_BLOCK_WIDTH: f32 = 50.0;

pub(crate) const NEIGHBORHOOD_PARCEL_SIZE: f32 = 16.0;

pub(crate) const NEIGHBORHOOD_FRONTAGE_SPACING: f32 = 18.0;

/// A small connected street ladder reserved before its roads and frontage lots
/// are built. The planner keeps one bounded block plan per world in this slice.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NeighborhoodPlan {
    pub(crate) anchor_node: u32,
    /// Ordered perimeter edges. Adjacent edges are scheduled one at a time,
    /// allowing each new junction to join the live graph before the next road.
    pub(crate) segments: Vec<(u32, u32)>,
    pub(crate) next_segment: usize,
    /// Legacy plans keep road-by-road lots; new plans reserve their interior
    /// until the complete connected perimeter can admit coordinated parcels.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) coordinated_lots: bool,
    /// Exact collision-free lots promised by a shared-edge plan. Older and
    /// four-new-edge plans continue to derive their full legacy layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) selected_lots: Option<Vec<infrastructure::blocks::BlockLot>>,
    /// Optional center-spine lifecycle. Older saves omit this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) spine: Option<legacy::spine_plan::SpinePlan>,
}

/// A validated, ephemeral block option. Nodes are allocated only after the
/// bounded candidate window has been ranked and this option wins.
#[cfg(feature = "legacy")]
pub(crate) struct NeighborhoodCandidate {
    pub(crate) anchor_node: u32,
    pub(crate) corners: [WorldPoint; 4],
    pub(crate) corner_node_ids: [Option<u32>; 4],
    pub(crate) reused_edge: Option<(usize, u32)>,
    pub(crate) selected_lots: Option<Vec<infrastructure::blocks::BlockLot>>,
    pub(crate) spine: Option<legacy::spine_plan::SpinePlan>,
    pub(crate) score: infrastructure::development::NeighborhoodBenefitScore,
}

#[cfg(test)]
std::thread_local! {
    static SPINE_RANKING_BY_SITES_PER_ROAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) struct SpineRankingExperimentGuard(bool);

#[cfg(test)]
impl SpineRankingExperimentGuard {
    pub(crate) fn by_sites_per_road() -> Self {
        let previous = SPINE_RANKING_BY_SITES_PER_ROAD.with(|flag| flag.replace(true));
        Self(previous)
    }
}

#[cfg(test)]
impl Drop for SpineRankingExperimentGuard {
    fn drop(&mut self) {
        SPINE_RANKING_BY_SITES_PER_ROAD.with(|flag| flag.set(self.0));
    }
}

#[cfg(feature = "legacy")]
pub(crate) fn neighborhood_candidate_is_better(
    candidate: infrastructure::development::NeighborhoodBenefitScore,
    current: infrastructure::development::NeighborhoodBenefitScore,
) -> bool {
    #[cfg(test)]
    if SPINE_RANKING_BY_SITES_PER_ROAD.with(std::cell::Cell::get) {
        return infrastructure::development::neighborhood_sites_per_road_is_better(
            candidate, current,
        );
    }
    infrastructure::development::neighborhood_score_is_better(candidate, current)
}

impl Engine {
    #[cfg(feature = "legacy")]
    pub(crate) fn try_reserve_early_coordinated_block(
        &mut self,
        active_road_projects: usize,
    ) -> bool {
        #[cfg(test)]
        if self.experiments.early_block_reservation_disable_override {
            return false;
        }
        if self.state.tick % 80 != 0
            || self.state.neighborhood_plan.is_some()
            || self.state.pending_interior_access.is_some()
            || active_road_projects > 0
        {
            return false;
        }
        self.record_early_block_attempt();
        if self.try_create_neighborhood_plan() {
            self.record_early_block_admission();
            true
        } else {
            self.record_early_block_noop();
            false
        }
    }

    #[cfg(test)]
    pub(crate) fn record_early_block_attempt(&mut self) {
        self.experiments.early_block_reservation_attempts = self
            .experiments
            .early_block_reservation_attempts
            .saturating_add(1);
    }

    #[cfg(not(test))]
    #[cfg(feature = "legacy")]
    pub(crate) fn record_early_block_attempt(&mut self) {}

    #[cfg(test)]
    pub(crate) fn record_early_block_admission(&mut self) {
        self.experiments.early_block_reservation_admissions = self
            .experiments
            .early_block_reservation_admissions
            .saturating_add(1);
        self.experiments.early_block_reservation_started_tick = Some(self.state.tick);
    }

    #[cfg(not(test))]
    #[cfg(feature = "legacy")]
    pub(crate) fn record_early_block_admission(&mut self) {}

    #[cfg(test)]
    pub(crate) fn record_early_block_noop(&mut self) {
        self.experiments.early_block_reservation_noops = self
            .experiments
            .early_block_reservation_noops
            .saturating_add(1);
    }

    #[cfg(not(test))]
    #[cfg(feature = "legacy")]
    pub(crate) fn record_early_block_noop(&mut self) {}

    pub(crate) fn is_neighborhood_plan_road(&self, road_id: u32) -> bool {
        self.state.neighborhood_road_ids.contains(&road_id)
            || self.state.neighborhood_plan.as_ref().is_some_and(|plan| {
                self.state.roads.iter().any(|road| {
                    road.id == road_id
                        && plan.segments.iter().any(|(from, to)| {
                            (road.from == *from && road.to == *to)
                                || (road.from == *to && road.to == *from)
                        })
                })
            })
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn neighborhood_block_has_unbuilt_frontage(&self) -> bool {
        let block_parcels: BTreeSet<u32> = self
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| self.state.neighborhood_road_ids.contains(&frontage.road_id))
            .map(|frontage| frontage.parcel_id)
            .collect();
        if block_parcels.is_empty() {
            return false;
        }
        let built_parcels: BTreeSet<u32> = self
            .state
            .buildings
            .iter()
            .map(|building| building.parcel_id)
            .collect();
        self.state.parcels.iter().any(|parcel| {
            block_parcels.contains(&parcel.id)
                && !parcel.protected
                && parcel.reservation.is_none()
                && (self.state.station_online || !self.is_station_tower_lot(parcel))
                && self.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                && self.node_connected(parcel.access_node)
                && !built_parcels.contains(&parcel.id)
        })
    }

    pub(crate) fn remember_neighborhood_plan_road(&mut self, road_id: u32, from: u32, to: u32) {
        let belongs_to_plan = self.state.neighborhood_plan.as_ref().is_some_and(|plan| {
            plan.segments
                .iter()
                .any(|(a, b)| (*a == from && *b == to) || (*a == to && *b == from))
        });
        if belongs_to_plan && self.state.neighborhood_road_ids.insert(road_id) {
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn retire_completed_neighborhood_plan(&mut self) -> bool {
        let Some(plan) = self.state.neighborhood_plan.as_ref() else {
            return false;
        };
        if plan.segments.len() != 4 || plan.next_segment != plan.segments.len() {
            return false;
        }
        if plan.spine.is_some() {
            let changed = self.advance_neighborhood_spine();
            if changed {
                self.mark_access_geometry_changed();
            }
            return changed;
        }
        let segments = plan.segments.clone();
        let coordinated_lots = plan.coordinated_lots;
        let selected_lots = plan.selected_lots.clone();
        let ready = segments.iter().all(|(from, to)| {
            self.state.roads.iter().any(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            }) && !self.state.projects.iter().any(|project| {
                project.kind == ProjectKind::Road
                    && project.progress < 1.0
                    && project
                        .from
                        .zip(project.to)
                        .is_some_and(|(a, b)| (a == *from && b == *to) || (a == *to && b == *from))
            })
        });
        if !ready {
            return false;
        }
        if coordinated_lots
            && selected_lots
                .as_ref()
                .is_some_and(|lots| !self.selected_block_lots_can_be_admitted(&segments, lots))
        {
            // Keep the completed plan and its interior reservation intact if
            // an external mutation invalidated a promised lot.
            return false;
        }
        for (from, to) in &segments {
            if let Some(road) = self.state.roads.iter().find(|road| {
                (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
            }) {
                self.state.neighborhood_road_ids.insert(road.id);
            }
        }
        self.state.neighborhood_plan = None;
        // Clearing a coordinated plan releases its reserved interior even if
        // the plan admitted no new parcels (legacy plans included).
        self.mark_access_geometry_changed();
        if coordinated_lots {
            self.admit_completed_block_lots(&segments, selected_lots.as_deref());
        }
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }

    pub(crate) fn admit_completed_block_lots(
        &mut self,
        segments: &[(u32, u32)],
        selected_lots: Option<&[infrastructure::blocks::BlockLot]>,
    ) {
        if segments.len() != 4 {
            return;
        }
        let mut corners = [infrastructure::blocks::BlockPoint { x: 0.0, z: 0.0 }; 4];
        let mut road_ids = [0_u32; 4];
        for (index, (from, to)) in segments.iter().enumerate() {
            let Some(node) = self.node(*from) else {
                return;
            };
            corners[index] = infrastructure::blocks::BlockPoint {
                x: node.x,
                z: node.z,
            };
            let Some(road) = self.state.roads.iter().find(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            }) else {
                return;
            };
            road_ids[index] = road.id;
        }
        let generated_lots;
        let lots = if let Some(selected) = selected_lots {
            selected
        } else {
            generated_lots =
                infrastructure::blocks::subdivide_rectangular_block(corners, 8.0, 11.0, 2.2);
            &generated_lots
        };
        for lot in lots {
            let rect = world::geometry::OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if selected_lots.is_none() && !self.oriented_parcel_geometry_clear(&rect) {
                continue;
            }
            let road_id = road_ids[lot.frontage_edge];
            let Some(road) = self.state.roads.iter().find(|road| road.id == road_id) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                continue;
            };
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            // Frontage metadata follows the actual road direction even when
            // the stored block edge was planned with reversed endpoints.
            let along =
                (((rect.x - a.x) * dx + (rect.z - a.z) * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
            let side = if dx * (rect.z - a.z) - dz * (rect.x - a.x) >= 0.0 {
                1
            } else {
                -1
            };
            let access = frontage_access_node(a, b, along);
            let parcel_id =
                self.add_parcel(rect.x, rect.z, rect.width, rect.depth, access, 0.82, false);
            if parcel_id == 0 {
                break;
            }
            if let Some(parcel) = self.state.parcels.last_mut() {
                parcel.rotation = Some(rect.rotation);
            }
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn coordinated_block_lots(
        &self,
        segments: &[(u32, u32)],
    ) -> Option<Vec<infrastructure::blocks::BlockLot>> {
        if segments.len() != 4 {
            return None;
        }
        let mut corners = [infrastructure::blocks::BlockPoint { x: 0.0, z: 0.0 }; 4];
        for (index, (from, _)) in segments.iter().enumerate() {
            let node = self.node(*from)?;
            corners[index] = infrastructure::blocks::BlockPoint {
                x: node.x,
                z: node.z,
            };
        }
        Some(infrastructure::blocks::subdivide_rectangular_block(
            corners, 8.0, 11.0, 2.2,
        ))
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn available_coordinated_block_lots(
        &self,
        corners: &[WorldPoint; 4],
        context: &CollisionContext,
    ) -> Vec<infrastructure::blocks::BlockLot> {
        let canonical = infrastructure::blocks::subdivide_rectangular_block(
            corners.map(|point| infrastructure::blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            8.0,
            11.0,
            2.2,
        );
        canonical
            .into_iter()
            .filter(|lot| {
                self.oriented_parcel_geometry_clear_with_context(
                    &world::geometry::OrientedRect {
                        x: lot.center.x,
                        z: lot.center.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    },
                    context,
                )
            })
            .collect()
    }

    pub(crate) fn coordinated_plan_lot_count(&self) -> Option<usize> {
        let plan = self.state.neighborhood_plan.as_ref()?;
        if !plan.coordinated_lots {
            return None;
        }
        if let Some(spine) = &plan.spine {
            return spine.validate().ok().map(|layout| layout.lots.len());
        }
        if let Some(lots) = &plan.selected_lots {
            Some(lots.len())
        } else {
            self.coordinated_block_lots(&plan.segments)
                .map(|lots| lots.len())
        }
    }

    pub(crate) fn selected_block_lots_can_be_admitted(
        &self,
        segments: &[(u32, u32)],
        lots: &[infrastructure::blocks::BlockLot],
    ) -> bool {
        if self.state.parcels.len().saturating_add(lots.len()) > MAX_PARCELS
            || !self.selected_block_layout_valid(segments, lots)
        {
            return false;
        }
        segments.iter().all(|(from, to)| {
            self.state.roads.iter().any(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && ((road.from == *from && road.to == *to)
                        || (road.from == *to && road.to == *from))
            })
        })
    }

    pub(crate) fn selected_block_layout_valid(
        &self,
        segments: &[(u32, u32)],
        lots: &[infrastructure::blocks::BlockLot],
    ) -> bool {
        if lots.len() < 4 || lots.len() > 64 {
            return false;
        }
        let Some(canonical) = self.coordinated_block_lots(segments) else {
            return false;
        };
        let Some((reuse_from, reuse_to)) = segments.first() else {
            return false;
        };
        if !self.state.roads.iter().any(|road| {
            road.stage == Stage::Open
                && road.connected
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
                && ((road.from == *reuse_from && road.to == *reuse_to)
                    || (road.from == *reuse_to && road.to == *reuse_from))
        }) {
            return false;
        }
        for index in 1..segments.len() {
            let Some((from, to)) = segments.get(index) else {
                return false;
            };
            let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                return false;
            };
            if distance(a, b) >= 70.0
                && lots.iter().filter(|lot| lot.frontage_edge == index).count() < 2
            {
                return false;
            }
        }
        let context = self.collision_context();
        for (index, lot) in lots.iter().enumerate() {
            if !canonical.contains(lot)
                || !lot.center.x.is_finite()
                || !lot.center.z.is_finite()
                || !lot.width.is_finite()
                || lot.width <= 0.0
                || !lot.depth.is_finite()
                || lot.depth <= 0.0
                || !lot.rotation.is_finite()
                || !lot.frontage_t.is_finite()
                || !(0.0..=1.0).contains(&lot.frontage_t)
                || lot.frontage_edge >= 4
            {
                return false;
            }
            let rect = world::geometry::OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if !self.oriented_parcel_geometry_clear_with_context_mode(&rect, &context, true)
                || lots[..index].iter().any(|other| {
                    world::geometry::rects_overlap(
                        &rect,
                        &world::geometry::OrientedRect {
                            x: other.center.x,
                            z: other.center.z,
                            width: other.width,
                            depth: other.depth,
                            rotation: other.rotation,
                        },
                        2.0,
                    )
                })
            {
                return false;
            }
            let Some((from, to)) = segments.get(lot.frontage_edge) else {
                return false;
            };
            if !self.node(*from).is_some_and(|node| {
                (node.x - lot.center.x).is_finite() && (node.z - lot.center.z).is_finite()
            }) || self.node(*to).is_none()
            {
                return false;
            }
        }
        true
    }

    pub(crate) fn coordinated_block_interior(
        &self,
        segments: &[(u32, u32)],
    ) -> Option<world::geometry::OrientedRect> {
        if segments.len() != 4 {
            return None;
        }
        let vertices: Vec<_> = segments
            .iter()
            .map(|(from, _)| self.node(*from))
            .collect::<Option<_>>()?;
        let dx = vertices[1].x - vertices[0].x;
        let dz = vertices[1].z - vertices[0].z;
        Some(world::geometry::OrientedRect {
            x: vertices.iter().map(|node| node.x).sum::<f32>() * 0.25,
            z: vertices.iter().map(|node| node.z).sum::<f32>() * 0.25,
            width: dx.hypot(dz) - 10.0,
            depth: distance(vertices[1], vertices[2]) - 10.0,
            rotation: -dz.atan2(dx),
        })
    }

    pub(crate) fn neighborhood_corridor_clear_for_parcel(
        &self,
        x: f32,
        z: f32,
        width: f32,
        depth: f32,
    ) -> bool {
        let Some(plan) = &self.state.neighborhood_plan else {
            return true;
        };
        if plan.coordinated_lots && plan.segments.len() == 4 {
            if let Some(interior) = self.coordinated_block_interior(&plan.segments) {
                let candidate = world::geometry::OrientedRect {
                    x,
                    z,
                    width,
                    depth,
                    rotation: 0.0,
                };
                if world::geometry::rects_overlap(&interior, &candidate, 0.0) {
                    return false;
                }
            }
        }
        let clearance = (width.max(depth) * 0.5 + 5.0).max(9.0);
        plan.segments
            .iter()
            .skip(plan.next_segment)
            .all(|(from, to)| {
                let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                    return true;
                };
                point_segment_distance(x, z, a.x, a.z, b.x, b.z) >= clearance
            })
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn neighborhood_corridor_clear_for_road(
        &self,
        start: &Node,
        end: &Node,
        start_id: u32,
    ) -> bool {
        let Some(plan) = &self.state.neighborhood_plan else {
            return true;
        };
        if plan.coordinated_lots {
            let Some(interior) = self.coordinated_block_interior(&plan.segments) else {
                return false;
            };
            let same_point =
                |a: &Node, b: &Node| (a.x - b.x).abs() <= 1.0e-4 && (a.z - b.z).abs() <= 1.0e-4;
            let is_perimeter_edge = plan.segments.iter().any(|(from, to)| {
                let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                    return false;
                };
                (same_point(start, a) && same_point(end, b))
                    || (same_point(start, b) && same_point(end, a))
            });
            if is_perimeter_edge {
                return true;
            }
            if !interior.finite_positive()
                || world::geometry::rect_segment_distance(&interior, start.x, start.z, end.x, end.z)
                    <= 4.5
            {
                return false;
            }
        }
        plan.segments
            .iter()
            .skip(plan.next_segment)
            .all(|(from, to)| {
                let (Some(a), Some(b)) = (self.node(*from), self.node(*to)) else {
                    return true;
                };
                let shares_node = *from == start_id || *to == start_id;
                shares_node || segments_distance(start, end, a, b) > 8.0
            })
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn neighborhood_candidate_clear(
        &self,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        tower_envelopes: &[(f32, f32)],
        collision_context: &CollisionContext,
    ) -> bool {
        let mut corner_ids = [None; 4];
        corner_ids[0] = Some(anchor.id);
        self.neighborhood_candidate_clear_with_edge(
            anchor,
            corners,
            tower_envelopes,
            collision_context,
            corner_ids,
            None,
            None,
        )
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn neighborhood_candidate_clear_with_edge(
        &self,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        tower_envelopes: &[(f32, f32)],
        collision_context: &CollisionContext,
        corner_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<&[infrastructure::blocks::BlockLot]>,
    ) -> bool {
        self.neighborhood_candidate_clear_with_layout(
            anchor,
            corners,
            tower_envelopes,
            collision_context,
            corner_ids,
            reused_edge,
            selected_lots,
            None,
        )
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn neighborhood_candidate_clear_with_layout(
        &self,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        tower_envelopes: &[(f32, f32)],
        collision_context: &CollisionContext,
        corner_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<&[infrastructure::blocks::BlockLot]>,
        spine_plan: Option<&legacy::spine_plan::SpinePlan>,
    ) -> bool {
        let result = self.neighborhood_candidate_clear_with_layout_reason(
            anchor,
            corners,
            tower_envelopes,
            collision_context,
            corner_ids,
            reused_edge,
            selected_lots,
            spine_plan,
        );
        #[cfg(test)]
        legacy::neighborhood_admission::record_outcome(&result, anchor.id, corners);
        result.is_ok()
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn ranked_neighborhood_candidate(
        anchor_node: u32,
        corners: [WorldPoint; 4],
        corner_node_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<Vec<infrastructure::blocks::BlockLot>>,
        score_lots: &[infrastructure::blocks::BlockLot],
        stable_key: [u32; 3],
        spine: Option<legacy::spine_plan::SpinePlan>,
    ) -> NeighborhoodCandidate {
        let nominal_area_cm2 = score_lots.iter().fold(0_u64, |area, lot| {
            let lot_area = (lot.width as f64 * lot.depth as f64 * 10_000.0).round() as u64;
            area.saturating_add(lot_area)
        });
        let mut new_road_length_cm = (0..4).fold(0_u32, |length, edge| {
            if reused_edge.is_some_and(|(reused, _)| reused == edge) {
                return length;
            }
            let from = corners[edge];
            let to = corners[(edge + 1) % 4];
            let edge_length_cm =
                ((to.x - from.x).hypot(to.z - from.z) as f64 * 100.0).round() as u32;
            length.saturating_add(edge_length_cm)
        });
        let spine_road_count = if let Some(plan) = &spine {
            if let Ok(layout) = plan.validate() {
                new_road_length_cm = new_road_length_cm.saturating_add(
                    ((layout.spine[1].x - layout.spine[0].x)
                        .hypot(layout.spine[1].z - layout.spine[0].z) as f64
                        * 100.0)
                        .round() as u32,
                );
            }
            3
        } else {
            0
        };
        NeighborhoodCandidate {
            anchor_node,
            corners,
            corner_node_ids,
            reused_edge,
            score: infrastructure::development::NeighborhoodBenefitScore {
                usable_sites: score_lots.len().min(u16::MAX as usize) as u16,
                nominal_area_cm2,
                new_road_length_cm,
                new_road_count: (4 - usize::from(reused_edge.is_some())) as u8 + spine_road_count,
                stable_key,
            },
            selected_lots,
            spine,
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn spine_candidate_lots(
        plan: &legacy::spine_plan::SpinePlan,
    ) -> Option<Vec<infrastructure::blocks::BlockLot>> {
        let layout = plan.validate().ok()?;
        Some(
            layout
                .lots
                .iter()
                .map(|lot| infrastructure::blocks::BlockLot {
                    center: lot.center,
                    frontage_edge: match lot.frontage {
                        legacy::spine_blocks::SpineFrontage::Perimeter(edge) => edge,
                        legacy::spine_blocks::SpineFrontage::Spine(_) => usize::MAX,
                    },
                    frontage_t: lot.frontage_t,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                })
                .collect(),
        )
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn try_create_neighborhood_plan(&mut self) -> bool {
        if self.state.neighborhood_plan.is_some()
            || self.state.pending_interior_access.is_some()
            || self.state.demand < 1.1
            || self
                .state
                .buildings
                .iter()
                .filter(|b| site_active(b))
                .count()
                < 10
            || self.state.population < 24
            || self.state.roads.len() + self.state.projects.len() + 3 > MAX_SEGMENTS
        {
            return false;
        }
        let node_count = self.state.nodes.len();
        if node_count == 0 {
            return false;
        }
        let tower_envelopes = self.available_station_tower_envelopes();
        let collision_context = self.collision_context();
        let mut connected = BTreeSet::new();
        let mut reusable_edges_by_node: BTreeMap<u32, Vec<Road>> = BTreeMap::new();
        for road in self
            .state
            .roads
            .iter()
            .filter(|road| road.connected && road.stage == Stage::Open)
        {
            connected.insert(road.from);
            connected.insert(road.to);
            if !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge) {
                reusable_edges_by_node
                    .entry(road.from)
                    .or_default()
                    .push(road.clone());
                reusable_edges_by_node
                    .entry(road.to)
                    .or_default()
                    .push(road.clone());
            }
        }
        let attempts = node_count.min(MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION);
        let first = self.state.neighborhood_anchor_cursor % node_count;
        let mut examined_reuse_edges = BTreeSet::new();
        let mut budget = infrastructure::development::NeighborhoodCandidateBudget::default();
        let mut best_candidate: Option<NeighborhoodCandidate> = None;
        for offset in 0..attempts {
            let index = (first + offset) % node_count;
            self.state.neighborhood_anchor_cursor = (index + 1) % node_count;
            let anchor = self.state.nodes[index].clone();
            if !connected.contains(&anchor.id) || self.ordinary_road_degree(anchor.id) == 0 {
                continue;
            }
            let (mut local_buildings, mut occupied_buildings) = (0, 0);
            for building in self.state.buildings.iter().filter(|b| site_active(b)) {
                if (building.x - anchor.x).hypot(building.z - anchor.z) <= 112.0 {
                    local_buildings += 1;
                    occupied_buildings += usize::from(site_active(building));
                }
            }
            if local_buildings < 8 || occupied_buildings < 4 {
                continue;
            }

            // Evaluate shared-edge and full-block candidates within separate
            // fixed budgets, then compare their usable site benefit per new
            // road length. This keeps either candidate class from hiding the
            // other through iteration order.
            if !budget.shared_limit_reached()
                && examined_reuse_edges.len() < MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION
            {
                if let Some(edges) = reusable_edges_by_node.get(&anchor.id) {
                    for road in edges {
                        if examined_reuse_edges.contains(&road.id) {
                            continue;
                        }
                        if examined_reuse_edges.len() >= MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION
                        {
                            break;
                        }
                        examined_reuse_edges.insert(road.id);
                        let (Some(start), Some(end)) =
                            (self.node(road.from).cloned(), self.node(road.to).cloned())
                        else {
                            continue;
                        };
                        let edge_length = distance(&start, &end);
                        let extrusion = if (70.0..=90.0).contains(&edge_length) {
                            NEIGHBORHOOD_BLOCK_WIDTH
                        } else if (42.0..=58.0).contains(&edge_length) {
                            NEIGHBORHOOD_BLOCK_LENGTH
                        } else {
                            continue;
                        };
                        let perpendicular_x = -(end.z - start.z) / edge_length;
                        let perpendicular_z = (end.x - start.x) / edge_length;
                        let side_seed = stateless_tower_site_hash(
                            self.state.seed ^ road.id,
                            (start.x + end.x) * 0.5,
                            (start.z + end.z) * 0.5,
                        );
                        let sides = if side_seed & 1 == 0 {
                            [1.0_f32, -1.0]
                        } else {
                            [-1.0_f32, 1.0]
                        };
                        for (side_index, side) in sides.into_iter().enumerate() {
                            if !budget.claim_shared() {
                                break;
                            }
                            let offset_x = perpendicular_x * extrusion * side;
                            let offset_z = perpendicular_z * extrusion * side;
                            let corners = [
                                WorldPoint {
                                    x: start.x,
                                    z: start.z,
                                },
                                WorldPoint { x: end.x, z: end.z },
                                WorldPoint {
                                    x: end.x + offset_x,
                                    z: end.z + offset_z,
                                },
                                WorldPoint {
                                    x: start.x + offset_x,
                                    z: start.z + offset_z,
                                },
                            ];
                            let selected_lots =
                                self.available_coordinated_block_lots(&corners, &collision_context);
                            let corner_ids = [Some(road.from), Some(road.to), None, None];
                            if self.neighborhood_candidate_clear_with_edge(
                                &anchor,
                                &corners,
                                &tower_envelopes,
                                &collision_context,
                                corner_ids,
                                Some((0, road.id)),
                                Some(&selected_lots),
                            ) {
                                let candidate = Self::ranked_neighborhood_candidate(
                                    road.from,
                                    corners,
                                    [Some(road.from), Some(road.to), None, None],
                                    Some((0, road.id)),
                                    Some(selected_lots.clone()),
                                    &selected_lots,
                                    [
                                        anchor.id,
                                        0,
                                        road.id.saturating_mul(2).saturating_add(side_index as u32),
                                    ],
                                    None,
                                );
                                if best_candidate.as_ref().map_or(true, |best| {
                                    neighborhood_candidate_is_better(candidate.score, best.score)
                                }) {
                                    best_candidate = Some(candidate);
                                }
                            }

                            // A deep square spine option competes with the
                            // ordinary block on this shared edge using the
                            // same benefit-per-centerline-metre score.
                            if (70.0..=90.0).contains(&edge_length) && budget.claim_spine() {
                                let spine_offset_x =
                                    perpendicular_x * NEIGHBORHOOD_BLOCK_LENGTH * side;
                                let spine_offset_z =
                                    perpendicular_z * NEIGHBORHOOD_BLOCK_LENGTH * side;
                                let spine_corners = [
                                    WorldPoint {
                                        x: start.x,
                                        z: start.z,
                                    },
                                    WorldPoint { x: end.x, z: end.z },
                                    WorldPoint {
                                        x: end.x + spine_offset_x,
                                        z: end.z + spine_offset_z,
                                    },
                                    WorldPoint {
                                        x: start.x + spine_offset_x,
                                        z: start.z + spine_offset_z,
                                    },
                                ];
                                let spine = legacy::spine_plan::SpinePlan {
                                    corners: spine_corners.map(|point| {
                                        infrastructure::blocks::BlockPoint {
                                            x: point.x,
                                            z: point.z,
                                        }
                                    }),
                                    recipe_version: legacy::spine_plan::RECIPE_VERSION,
                                    stage: legacy::spine_plan::SpineStage::Reserved,
                                };
                                if self.neighborhood_candidate_clear_with_layout(
                                    &anchor,
                                    &spine_corners,
                                    &tower_envelopes,
                                    &collision_context,
                                    corner_ids,
                                    Some((0, road.id)),
                                    None,
                                    Some(&spine),
                                ) {
                                    if let Some(score_lots) = Self::spine_candidate_lots(&spine) {
                                        let candidate = Self::ranked_neighborhood_candidate(
                                            road.from,
                                            spine_corners,
                                            corner_ids,
                                            Some((0, road.id)),
                                            None,
                                            &score_lots,
                                            [
                                                anchor.id,
                                                2,
                                                road.id
                                                    .saturating_mul(2)
                                                    .saturating_add(side_index as u32),
                                            ],
                                            Some(spine),
                                        );
                                        if best_candidate.as_ref().map_or(true, |best| {
                                            neighborhood_candidate_is_better(
                                                candidate.score,
                                                best.score,
                                            )
                                        }) {
                                            best_candidate = Some(candidate);
                                        }
                                    }
                                }
                            }
                        }
                        if budget.shared_limit_reached() {
                            break;
                        }
                    }
                }
            }

            // Four new streets add two incident roads at this anchor. Keep
            // the older junction-degree constraint for that fallback shape;
            // a reused edge above contributes only one new incident road.
            if self.ordinary_road_degree(anchor.id) > 2 {
                if budget.shared_limit_reached() {
                    break;
                }
                continue;
            }

            let start = stateless_tower_site_hash(self.state.seed, anchor.x, anchor.z) as usize % 8;
            for rotation in 0..8 {
                if !budget.claim_full() {
                    break;
                }
                let option = (start + rotation) % 8;
                let (ux, uz, vx, vz) = match option {
                    0 => (1.0, 0.0, 0.0, 1.0),
                    1 => (1.0, 0.0, 0.0, -1.0),
                    2 => (-1.0, 0.0, 0.0, 1.0),
                    3 => (-1.0, 0.0, 0.0, -1.0),
                    4 => (0.0, 1.0, 1.0, 0.0),
                    5 => (0.0, 1.0, -1.0, 0.0),
                    6 => (0.0, -1.0, 1.0, 0.0),
                    _ => (0.0, -1.0, -1.0, 0.0),
                };
                let corners = [
                    WorldPoint {
                        x: anchor.x,
                        z: anchor.z,
                    },
                    WorldPoint {
                        x: anchor.x + ux * NEIGHBORHOOD_BLOCK_LENGTH,
                        z: anchor.z + uz * NEIGHBORHOOD_BLOCK_LENGTH,
                    },
                    WorldPoint {
                        x: anchor.x
                            + ux * NEIGHBORHOOD_BLOCK_LENGTH
                            + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                        z: anchor.z
                            + uz * NEIGHBORHOOD_BLOCK_LENGTH
                            + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                    },
                    WorldPoint {
                        x: anchor.x + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                        z: anchor.z + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                    },
                ];
                #[cfg(test)]
                let partial_lots = if self.experiments.partial_full_block_experiment {
                    let lots = self.available_coordinated_block_lots(&corners, &collision_context);
                    let canonical_count = infrastructure::blocks::subdivide_rectangular_block(
                        corners.map(|point| infrastructure::blocks::BlockPoint {
                            x: point.x,
                            z: point.z,
                        }),
                        8.0,
                        11.0,
                        2.2,
                    )
                    .len();
                    (lots.len() < canonical_count).then_some(lots)
                } else {
                    None
                };
                #[cfg(test)]
                let candidate_clear = if let Some(lots) = partial_lots.as_deref() {
                    let candidate_result = self.neighborhood_candidate_clear_with_layout_reason(
                        &anchor,
                        &corners,
                        &tower_envelopes,
                        &collision_context,
                        [Some(anchor.id), None, None, None],
                        None,
                        Some(lots),
                        None,
                    );
                    legacy::neighborhood_admission::record_outcome(
                        &candidate_result,
                        anchor.id,
                        &corners,
                    );
                    {
                        let canonical_count = infrastructure::blocks::subdivide_rectangular_block(
                            corners.map(|point| infrastructure::blocks::BlockPoint {
                                x: point.x,
                                z: point.z,
                            }),
                            8.0,
                            11.0,
                            2.2,
                        )
                        .len();
                        experiments::partial_block_diagnostic::record_partial_candidate(
                            anchor.id,
                            &corners,
                            canonical_count,
                            lots,
                            &candidate_result,
                        );
                    }
                    candidate_result.is_ok()
                } else {
                    self.neighborhood_candidate_clear(
                        &anchor,
                        &corners,
                        &tower_envelopes,
                        &collision_context,
                    )
                };
                #[cfg(not(test))]
                let candidate_clear = self.neighborhood_candidate_clear(
                    &anchor,
                    &corners,
                    &tower_envelopes,
                    &collision_context,
                );
                if candidate_clear {
                    #[cfg(test)]
                    let (score_lots, selected_lots) = if let Some(lots) = partial_lots {
                        (lots, None)
                    } else {
                        let canonical_lots = infrastructure::blocks::subdivide_rectangular_block(
                            corners.map(|point| infrastructure::blocks::BlockPoint {
                                x: point.x,
                                z: point.z,
                            }),
                            8.0,
                            11.0,
                            2.2,
                        );
                        (canonical_lots, None)
                    };
                    #[cfg(not(test))]
                    let (score_lots, selected_lots) = (
                        infrastructure::blocks::subdivide_rectangular_block(
                            corners.map(|point| infrastructure::blocks::BlockPoint {
                                x: point.x,
                                z: point.z,
                            }),
                            8.0,
                            11.0,
                            2.2,
                        ),
                        None,
                    );
                    let candidate = Self::ranked_neighborhood_candidate(
                        anchor.id,
                        corners,
                        [Some(anchor.id), None, None, None],
                        None,
                        selected_lots,
                        &score_lots,
                        [anchor.id, 1, option as u32],
                        None,
                    );
                    if best_candidate.as_ref().map_or(true, |best| {
                        neighborhood_candidate_is_better(candidate.score, best.score)
                    }) {
                        best_candidate = Some(candidate);
                    }
                }

                if budget.claim_spine() {
                    let spine_corners = [
                        WorldPoint {
                            x: anchor.x,
                            z: anchor.z,
                        },
                        WorldPoint {
                            x: anchor.x + ux * NEIGHBORHOOD_BLOCK_LENGTH,
                            z: anchor.z + uz * NEIGHBORHOOD_BLOCK_LENGTH,
                        },
                        WorldPoint {
                            x: anchor.x
                                + ux * NEIGHBORHOOD_BLOCK_LENGTH
                                + vx * NEIGHBORHOOD_BLOCK_LENGTH,
                            z: anchor.z
                                + uz * NEIGHBORHOOD_BLOCK_LENGTH
                                + vz * NEIGHBORHOOD_BLOCK_LENGTH,
                        },
                        WorldPoint {
                            x: anchor.x + vx * NEIGHBORHOOD_BLOCK_LENGTH,
                            z: anchor.z + vz * NEIGHBORHOOD_BLOCK_LENGTH,
                        },
                    ];
                    let spine = legacy::spine_plan::SpinePlan {
                        corners: spine_corners.map(|point| infrastructure::blocks::BlockPoint {
                            x: point.x,
                            z: point.z,
                        }),
                        recipe_version: legacy::spine_plan::RECIPE_VERSION,
                        stage: legacy::spine_plan::SpineStage::Reserved,
                    };
                    let spine_corner_ids = [Some(anchor.id), None, None, None];
                    if self.neighborhood_candidate_clear_with_layout(
                        &anchor,
                        &spine_corners,
                        &tower_envelopes,
                        &collision_context,
                        spine_corner_ids,
                        None,
                        None,
                        Some(&spine),
                    ) {
                        if let Some(score_lots) = Self::spine_candidate_lots(&spine) {
                            let candidate = Self::ranked_neighborhood_candidate(
                                anchor.id,
                                spine_corners,
                                spine_corner_ids,
                                None,
                                None,
                                &score_lots,
                                [anchor.id, 3, option as u32],
                                Some(spine),
                            );
                            if best_candidate.as_ref().map_or(true, |best| {
                                neighborhood_candidate_is_better(candidate.score, best.score)
                            }) {
                                best_candidate = Some(candidate);
                            }
                        }
                    }
                }
            }
            // Stop at the anchor where either class exhausts its allowance.
            // The cursor already points to the next anchor, so candidates
            // beyond that point are reached on the next planning evaluation.
            if budget.any_limit_reached() {
                break;
            }
        }
        let Some(candidate) = best_candidate else {
            return false;
        };
        let mut ids = candidate.corner_node_ids;
        for index in 0..4 {
            if ids[index].is_none() {
                ids[index] = Some(self.add_node(
                    candidate.corners[index].x,
                    candidate.corners[index].z,
                    NodeKind::Junction,
                ));
            }
        }
        let ids = ids.map(|id| id.expect("validated neighborhood corner has a node"));
        let segments = (0..4)
            .map(|index| (ids[index], ids[(index + 1) % 4]))
            .collect();
        if let Some((_, road_id)) = candidate.reused_edge {
            self.state.neighborhood_road_ids.insert(road_id);
        }
        self.state.neighborhood_plan = Some(NeighborhoodPlan {
            anchor_node: candidate.anchor_node,
            segments,
            next_segment: 0,
            coordinated_lots: true,
            selected_lots: candidate.selected_lots,
            spine: candidate.spine,
        });
        self.mark_access_geometry_changed();
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn plan_next_neighborhood_segment(&mut self) -> bool {
        loop {
            let Some(plan) = self.state.neighborhood_plan.as_ref() else {
                return false;
            };
            if plan.next_segment >= plan.segments.len() {
                let changed = self.advance_neighborhood_spine();
                if changed {
                    self.mark_access_geometry_changed();
                }
                return changed;
            }
            let (from, to) = plan.segments[plan.next_segment];
            if let Some(road_id) = self
                .state
                .roads
                .iter()
                .find(|road| {
                    road.stage == Stage::Open
                        && ((road.from == from && road.to == to)
                            || (road.from == to && road.to == from))
                })
                .map(|road| road.id)
            {
                self.remember_neighborhood_plan_road(road_id, from, to);
                self.state.neighborhood_plan.as_mut().unwrap().next_segment += 1;
                self.retire_completed_neighborhood_plan();
                continue;
            }
            if !self.node_connected(from)
                || self.state.roads.len() + self.state.projects.len() >= MAX_SEGMENTS
                || !self.project_admission_available()
            {
                return true;
            }
            let id = self.id();
            let duration = 3.0 + random_unit(&mut self.state.rng_growth) * 4.0;
            let started_tick = self.state.tick;
            self.record_project(Project {
                id,
                kind: ProjectKind::Road,
                from: Some(from),
                to: Some(to),
                parcel_id: None,
                stage: Stage::Survey,
                progress: 0.0,
                reason: "A reserved local street opens a compact neighborhood block".into(),
                duration,
                started_tick,
            });
            self.state.neighborhood_plan.as_mut().unwrap().next_segment += 1;
            self.state.revision = self.state.revision.saturating_add(1);
            return true;
        }
    }

    /// Reserve a real local-street chord inside a developed neighborhood
    /// before extending the outer edge. A chord joins two existing connected
    /// junctions, so its construction cannot introduce a disconnected segment
    /// or a speculative endpoint inside a future parcel.
    #[cfg(feature = "legacy")]
    pub(crate) fn try_plan_local_chord(
        &mut self,
        open_buildings: &[Building],
        collision_context: &CollisionContext,
    ) -> bool {
        if !self.project_admission_available()
            || open_buildings.len() < 12
            || self.state.demand <= 0.8
            || self
                .state
                .projects
                .iter()
                .any(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        {
            return false;
        }

        let connected: BTreeSet<u32> = self
            .state
            .roads
            .iter()
            .filter(|road| {
                road.connected
                    && road.stage == Stage::Open
                    && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
            })
            .flat_map(|road| [road.from, road.to])
            .collect();
        let mut origins: Vec<(Node, f32)> = self
            .state
            .nodes
            .iter()
            .filter(|node| connected.contains(&node.id))
            .filter_map(|node| {
                let nearest = open_buildings
                    .iter()
                    .map(|building| (building.x - node.x).hypot(building.z - node.z))
                    .min_by(f32::total_cmp)?;
                (nearest <= 110.0).then_some((node.clone(), nearest))
            })
            .collect();
        if origins.len() < 2 {
            return false;
        }
        let center = open_buildings.iter().fold((0.0, 0.0), |(x, z), building| {
            (
                x + building.x / open_buildings.len() as f32,
                z + building.z / open_buildings.len() as f32,
            )
        });
        origins.sort_by(|(a, _), (b, _)| {
            let a_radius = (a.x - center.0).hypot(a.z - center.1);
            let b_radius = (b.x - center.0).hypot(b.z - center.1);
            b_radius.total_cmp(&a_radius).then_with(|| a.id.cmp(&b.id))
        });

        let first = self.state.road_origin_cursor % origins.len();
        let count = origins.len().min(MAX_ROAD_ORIGINS_PER_EVALUATION);
        for offset in 0..count {
            let origin_index = (first + offset) % origins.len();
            let start = origins[origin_index].0.clone();
            self.state.road_origin_cursor = (origin_index + 1) % origins.len();
            if start.id >= 7 && self.ordinary_road_degree(start.id) >= 4 {
                continue;
            }

            let mut targets: Vec<(Node, f32)> = self
                .state
                .nodes
                .iter()
                .filter(|node| node.id > start.id && connected.contains(&node.id))
                .filter_map(|node| {
                    let length = distance(&start, node);
                    (45.0..=90.0)
                        .contains(&length)
                        .then_some((node.clone(), length))
                })
                .collect();
            targets.sort_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| a.id.cmp(&b.id)));

            for (target, _) in targets {
                if target.id >= 7 && self.ordinary_road_degree(target.id) >= 4 {
                    continue;
                }
                if self.state.roads.iter().any(|road| {
                    (road.from == start.id && road.to == target.id)
                        || (road.from == target.id && road.to == start.id)
                }) || self.state.projects.iter().any(|project| {
                    project.kind == ProjectKind::Road
                        && project.progress < 1.0
                        && ((project.from == Some(start.id) && project.to == Some(target.id))
                            || (project.from == Some(target.id) && project.to == Some(start.id)))
                }) {
                    continue;
                }
                let forward = (target.z - start.z).atan2(target.x - start.x);
                let reverse = (start.z - target.z).atan2(start.x - target.x);
                if !self.ordinary_heading_clear(start.id, forward)
                    || !self.ordinary_heading_clear(target.id, reverse)
                    || in_protected_land(target.x, target.z)
                    || corridor_crosses_protected(start.x, start.z, target.x, target.z)
                    || !self.ordinary_road_clear_of_river(start.x, start.z, target.x, target.z, 4.5)
                {
                    continue;
                }

                let midpoint = WorldPoint {
                    x: (start.x + target.x) * 0.5,
                    z: (start.z + target.z) * 0.5,
                };
                let local_buildings = open_buildings.iter().filter(|building| {
                    (building.x - midpoint.x).hypot(building.z - midpoint.z) <= 100.0
                });
                let mut local_open = 0usize;
                let mut local_occupied = 0usize;
                for building in local_buildings {
                    local_open += 1;
                    local_occupied += usize::from(site_active(building));
                }
                if local_open < 8 || local_occupied < 4 {
                    continue;
                }

                // Leave a clear corridor for the full street width and its
                // construction tolerance. Existing parcels near either
                // junction are safe only when their center lies outside this
                // reserved strip.
                let corridor_clear = self.road_corridor_clear_of_parcels_with_context(
                    start.x,
                    start.z,
                    target.x,
                    target.z,
                    collision_context,
                );
                let road_clear = self
                    .state
                    .roads
                    .iter()
                    .filter(|road| road.stage == Stage::Open)
                    .all(|road| {
                        if road.from == start.id
                            || road.to == start.id
                            || road.from == target.id
                            || road.to == target.id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                            return true;
                        };
                        segments_distance(&start, &target, a, b) > 8.0
                    });
                let projects_clear = self
                    .state
                    .projects
                    .iter()
                    .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
                    .all(|project| {
                        let (Some(from), Some(to)) = (project.from, project.to) else {
                            return true;
                        };
                        if from == start.id
                            || to == start.id
                            || from == target.id
                            || to == target.id
                        {
                            return true;
                        }
                        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                            return true;
                        };
                        segments_distance(&start, &target, a, b) > 8.0
                    });
                if !corridor_clear
                    || !road_clear
                    || !projects_clear
                    || !self.neighborhood_corridor_clear_for_road(&start, &target, start.id)
                {
                    continue;
                }

                // Admission also requires usable lots on the chosen, actual
                // corridor. This uses the same clearance checks as parcels
                // created when construction completes.
                if self.viable_frontage_benefit_count(&start, &target, 2) < 2 {
                    continue;
                }

                let id = self.id();
                let duration = 3.0 + random_unit(&mut self.state.rng_growth) * 4.0;
                self.record_project(Project {
                    id,
                    kind: ProjectKind::Road,
                    from: Some(start.id),
                    to: Some(target.id),
                    parcel_id: None,
                    stage: Stage::Survey,
                    progress: 0.0,
                    reason:
                        "A local connector closes a developed block and opens new street frontage"
                            .into(),
                    duration,
                    started_tick: self.state.tick,
                });
                self.state.revision = self.state.revision.saturating_add(1);
                return true;
            }
        }
        false
    }
}
