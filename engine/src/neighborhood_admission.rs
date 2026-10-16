use super::*;
#[cfg(test)]
use serde::Serialize;
#[cfg(test)]
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NeighborhoodRejection {
    AnchorNotInCandidateCorners,
    RoadSegmentCapacity,
    SelectedLotsConflictWithSpine,
    AnchorDegreeLimit,
    CornerNodeCoordinateMismatch,
    SpineReservationMismatch,
    InvalidSpineLayout,
    TooFewProposedLots,
    TooManyProposedLots,
    NoncanonicalSelectedLots,
    ParcelCapacity,
    FrontageCapacity,
    ProposedLotGeometry,
    IncompleteFullBlockLots,
    SpineIdCapacity,
    SpineWorldGeometry,
    InsufficientSharedEdgeFrontage,
    ReusedEdgeUnavailable,
    RoadCornerOutOfBounds,
    RoadCrossesRiverBuffer,
    RoadCrossesProtectedLand,
    StartNodeHeadingConflict,
    EndNodeHeadingConflict,
    RoadCorridorParcelConflict,
    ExistingRoadProximity,
    ActiveRoadProjectProximity,
    InsufficientInteriorFrontage,
    StationTowerRoadClearance,
}

impl NeighborhoodRejection {
    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::AnchorNotInCandidateCorners => "anchor_not_in_candidate_corners",
            Self::RoadSegmentCapacity => "road_segment_capacity",
            Self::SelectedLotsConflictWithSpine => "selected_lots_conflict_with_spine",
            Self::AnchorDegreeLimit => "anchor_degree_limit",
            Self::CornerNodeCoordinateMismatch => "corner_node_coordinate_mismatch",
            Self::SpineReservationMismatch => "spine_reservation_mismatch",
            Self::InvalidSpineLayout => "invalid_spine_layout",
            Self::TooFewProposedLots => "too_few_proposed_lots",
            Self::TooManyProposedLots => "too_many_proposed_lots",
            Self::NoncanonicalSelectedLots => "noncanonical_selected_lots",
            Self::ParcelCapacity => "parcel_capacity",
            Self::FrontageCapacity => "frontage_capacity",
            Self::ProposedLotGeometry => "proposed_lot_geometry",
            Self::IncompleteFullBlockLots => "incomplete_full_block_lots",
            Self::SpineIdCapacity => "spine_id_capacity",
            Self::SpineWorldGeometry => "spine_world_geometry",
            Self::InsufficientSharedEdgeFrontage => "insufficient_shared_edge_frontage",
            Self::ReusedEdgeUnavailable => "reused_edge_unavailable",
            Self::RoadCornerOutOfBounds => "road_corner_out_of_bounds",
            Self::RoadCrossesRiverBuffer => "road_crosses_river_buffer",
            Self::RoadCrossesProtectedLand => "road_crosses_protected_land",
            Self::StartNodeHeadingConflict => "start_node_heading_conflict",
            Self::EndNodeHeadingConflict => "end_node_heading_conflict",
            Self::RoadCorridorParcelConflict => "road_corridor_parcel_conflict",
            Self::ExistingRoadProximity => "existing_road_proximity",
            Self::ActiveRoadProjectProximity => "active_road_project_proximity",
            Self::InsufficientInteriorFrontage => "insufficient_interior_frontage",
            Self::StationTowerRoadClearance => "station_tower_road_clearance",
        }
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Serialize)]
pub(super) struct LotGeometryWitness {
    pub(super) failed_lot_rect: [f32; 5],
    pub(super) failure_category: &'static str,
    pub(super) first_overlapping_parcel_footprint_index: Option<usize>,
}

#[cfg(test)]
#[derive(Clone, Debug, Serialize)]
pub(super) struct RejectionWitness {
    pub(super) anchor_node: u32,
    pub(super) reason: &'static str,
    pub(super) corners: [[f32; 2]; 4],
    pub(super) lot_geometry: Option<LotGeometryWitness>,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct RejectionCapture {
    pub(super) accepted_candidates: usize,
    pub(super) reason_counts: BTreeMap<&'static str, usize>,
    pub(super) witnesses: Vec<RejectionWitness>,
}

#[cfg(test)]
#[derive(Default)]
struct CaptureState {
    result: RejectionCapture,
    pending_lot_geometry: Option<LotGeometryWitness>,
}

#[cfg(test)]
std::thread_local! {
    static REJECTION_CAPTURE: std::cell::RefCell<Option<CaptureState>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn capture_rejections<R>(f: impl FnOnce() -> R) -> (R, RejectionCapture) {
    REJECTION_CAPTURE.with(|capture| {
        assert!(
            capture.borrow().is_none(),
            "rejection capture scopes cannot nest"
        );
        *capture.borrow_mut() = Some(CaptureState::default());
    });
    let _guard = CaptureGuard;
    let result = f();
    let collected = REJECTION_CAPTURE.with(|capture| {
        capture
            .borrow_mut()
            .take()
            .expect("capture scope was installed")
            .result
    });
    (result, collected)
}

#[cfg(test)]
struct CaptureGuard;

#[cfg(test)]
impl Drop for CaptureGuard {
    fn drop(&mut self) {
        REJECTION_CAPTURE.with(|capture| {
            capture.borrow_mut().take();
        });
    }
}

#[cfg(test)]
pub(super) fn record_outcome(
    result: &Result<(), NeighborhoodRejection>,
    anchor_node: u32,
    corners: &[WorldPoint; 4],
) {
    REJECTION_CAPTURE.with(|capture| {
        let mut capture = capture.borrow_mut();
        let Some(state) = capture.as_mut() else {
            return;
        };
        match result {
            Ok(()) => state.result.accepted_candidates += 1,
            Err(reason) => {
                *state
                    .result
                    .reason_counts
                    .entry(reason.label())
                    .or_default() += 1;
                let witness_count = state
                    .result
                    .witnesses
                    .iter()
                    .filter(|witness| witness.reason == reason.label())
                    .count();
                if witness_count < 8 {
                    state.result.witnesses.push(RejectionWitness {
                        anchor_node,
                        reason: reason.label(),
                        corners: corners.map(|point| [point.x, point.z]),
                        lot_geometry: if *reason == NeighborhoodRejection::ProposedLotGeometry {
                            state.pending_lot_geometry.take()
                        } else {
                            None
                        },
                    });
                }
                state.pending_lot_geometry = None;
            }
        }
    });
}

#[cfg(test)]
fn record_lot_geometry_failure(
    engine: &Engine,
    proposed_lots: &[blocks::BlockLot],
    context: &CollisionContext,
) {
    REJECTION_CAPTURE.with(|capture| {
        let mut capture = capture.borrow_mut();
        let Some(state) = capture.as_mut() else {
            return;
        };
        let recorded = state
            .result
            .witnesses
            .iter()
            .filter(|witness| witness.reason == "proposed_lot_geometry")
            .count();
        if recorded >= 8 {
            return;
        }
        let Some(lot) = proposed_lots.iter().find(|lot| {
            !engine.oriented_parcel_geometry_clear_with_context(
                &geometry::OrientedRect {
                    x: lot.center.x,
                    z: lot.center.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                },
                context,
            )
        }) else {
            return;
        };
        let rect = geometry::OrientedRect {
            x: lot.center.x,
            z: lot.center.z,
            width: lot.width,
            depth: lot.depth,
            rotation: lot.rotation,
        };
        let first_overlapping_parcel_footprint_index = context
            .parcel_footprints
            .iter()
            .position(|existing| geometry::rects_overlap(&rect, existing, 2.0));
        let failure_category = if !rect.finite_positive() {
            "invalid_footprint"
        } else if !engine.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth) {
            "river_clearance"
        } else {
            let (min_x, min_z, max_x, max_z) = rect.bounds();
            let protected = geometry::OrientedRect {
                x: -305.0,
                z: -310.0,
                width: 90.0,
                depth: 80.0,
                rotation: 0.0,
            };
            if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
                "world_bounds"
            } else if geometry::rects_overlap(&rect, &protected, 0.0) {
                "protected_site_overlap"
            } else if first_overlapping_parcel_footprint_index.is_some() {
                "existing_parcel_clearance"
            } else if !engine.interior_plan_allows_parcel(&rect, false) {
                "pending_interior_plan_reservation"
            } else if context.open_roads.iter().any(|road| {
                geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 5.0
            }) {
                "open_road_footprint_clearance"
            } else if context.pending_roads.iter().any(|road| {
                geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 7.0
            }) {
                "pending_road_footprint_clearance"
            } else {
                "neighborhood_corridor_clearance"
            }
        };
        state.pending_lot_geometry = Some(LotGeometryWitness {
            failed_lot_rect: [rect.x, rect.z, rect.width, rect.depth, rect.rotation],
            failure_category,
            first_overlapping_parcel_footprint_index,
        });
    });
}

impl Engine {
    pub(super) fn neighborhood_candidate_clear_with_layout_reason(
        &self,
        anchor: &Node,
        corners: &[WorldPoint; 4],
        tower_envelopes: &[(f32, f32)],
        collision_context: &CollisionContext,
        corner_ids: [Option<u32>; 4],
        reused_edge: Option<(usize, u32)>,
        selected_lots: Option<&[blocks::BlockLot]>,
        spine_plan: Option<&spine_plan::SpinePlan>,
    ) -> Result<(), NeighborhoodRejection> {
        if !corner_ids.contains(&Some(anchor.id)) {
            return Err(NeighborhoodRejection::AnchorNotInCandidateCorners);
        }
        let required_segments = 4 - usize::from(reused_edge.is_some());
        let spine_road_reservation = if spine_plan.is_some() { 3 } else { 0 };
        if self
            .state
            .roads
            .len()
            .saturating_add(self.state.projects.len())
            .saturating_add(required_segments)
            .saturating_add(spine_road_reservation)
            > MAX_SEGMENTS
        {
            return Err(NeighborhoodRejection::RoadSegmentCapacity);
        }
        if selected_lots.is_some() && spine_plan.is_some() {
            return Err(NeighborhoodRejection::SelectedLotsConflictWithSpine);
        }
        let mut new_incident_roads = [0usize; 4];
        for index in 0..4 {
            if reused_edge.is_some_and(|(edge_index, _)| edge_index == index) {
                continue;
            }
            new_incident_roads[index] += 1;
            new_incident_roads[(index + 1) % 4] += 1;
        }
        for (index, node_id) in corner_ids.iter().enumerate() {
            if let Some(node_id) = node_id {
                if self.ordinary_road_degree(*node_id) + new_incident_roads[index] > 4 {
                    return Err(NeighborhoodRejection::AnchorDegreeLimit);
                }
            }
        }
        let mut corner_nodes = Vec::with_capacity(4);
        for index in 0..4 {
            if let Some(node) = corner_ids[index].and_then(|node_id| self.node(node_id)) {
                if (node.x - corners[index].x).abs() > 1.0e-4
                    || (node.z - corners[index].z).abs() > 1.0e-4
                {
                    return Err(NeighborhoodRejection::CornerNodeCoordinateMismatch);
                }
                corner_nodes.push(node.clone());
            } else {
                corner_nodes.push(Node {
                    id: u32::MAX - index as u32,
                    x: corners[index].x,
                    z: corners[index].z,
                    kind: "candidate".into(),
                });
            }
        }
        let center = WorldPoint {
            x: corners.iter().map(|point| point.x).sum::<f32>() * 0.25,
            z: corners.iter().map(|point| point.z).sum::<f32>() * 0.25,
        };
        let canonical_lots = blocks::subdivide_rectangular_block(
            corners.map(|point| blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            8.0,
            11.0,
            2.2,
        );
        let spine_layout = match spine_plan {
            Some(plan) => {
                if plan.stage != spine_plan::SpineStage::Reserved
                    || plan.corners.iter().zip(corners).any(|(saved, proposed)| {
                        (saved.x - proposed.x).abs() > 1.0e-4
                            || (saved.z - proposed.z).abs() > 1.0e-4
                    })
                {
                    return Err(NeighborhoodRejection::SpineReservationMismatch);
                }
                match plan.validate() {
                    Ok(layout) => Some(layout),
                    Err(_) => return Err(NeighborhoodRejection::InvalidSpineLayout),
                }
            }
            None => None,
        };
        let spine_lots: Vec<blocks::BlockLot> = spine_layout
            .as_ref()
            .map(|layout| {
                layout
                    .lots
                    .iter()
                    .map(|lot| blocks::BlockLot {
                        center: lot.center,
                        frontage_edge: match lot.frontage {
                            spine_blocks::SpineFrontage::Perimeter(edge) => edge,
                            spine_blocks::SpineFrontage::Spine(_) => usize::MAX,
                        },
                        frontage_t: lot.frontage_t,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let proposed_lots = selected_lots
            .or_else(|| (!spine_lots.is_empty()).then_some(spine_lots.as_slice()))
            .unwrap_or(&canonical_lots);
        // Reserve the same deterministic lot set that will be admitted once
        // the perimeter opens. Check this before scanning world geometry.
        // Keep these ordered checks identical to the former short-circuit OR.
        if proposed_lots.len() < 4 {
            return Err(NeighborhoodRejection::TooFewProposedLots);
        }
        if proposed_lots.len() > 64 {
            return Err(NeighborhoodRejection::TooManyProposedLots);
        }
        if spine_plan.is_none()
            && proposed_lots
                .iter()
                .any(|lot| !canonical_lots.contains(lot))
        {
            return Err(NeighborhoodRejection::NoncanonicalSelectedLots);
        }
        if self.state.parcels.len().saturating_add(proposed_lots.len()) > MAX_PARCELS {
            return Err(NeighborhoodRejection::ParcelCapacity);
        }
        if self
            .state
            .parcel_frontages
            .len()
            .saturating_add(proposed_lots.len())
            > MAX_PARCELS
        {
            return Err(NeighborhoodRejection::FrontageCapacity);
        }
        if proposed_lots.iter().any(|lot| {
            !self.oriented_parcel_geometry_clear_with_context(
                &geometry::OrientedRect {
                    x: lot.center.x,
                    z: lot.center.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                },
                collision_context,
            )
        }) {
            #[cfg(test)]
            record_lot_geometry_failure(self, proposed_lots, collision_context);
            return Err(NeighborhoodRejection::ProposedLotGeometry);
        }
        #[cfg(test)]
        if self.experiments.partial_full_block_experiment
            && reused_edge.is_none()
            && spine_plan.is_none()
            && selected_lots.is_some()
            && !Engine::partial_full_block_lots_useful(corners, proposed_lots)
        {
            return Err(NeighborhoodRejection::InsufficientInteriorFrontage);
        }
        #[cfg(test)]
        let partial_full_block_is_explicit = self.experiments.partial_full_block_experiment
            && selected_lots.is_some_and(|selected| {
                selected.len() < canonical_lots.len()
                    && selected.iter().enumerate().all(|(index, lot)| {
                        canonical_lots.contains(lot) && !selected[..index].contains(lot)
                    })
                    && Engine::partial_full_block_lots_useful(corners, selected)
            });
        #[cfg(not(test))]
        let partial_full_block_is_explicit = false;
        if spine_plan.is_none()
            && reused_edge.is_none()
            && proposed_lots.len() != canonical_lots.len()
            && !partial_full_block_is_explicit
        {
            return Err(NeighborhoodRejection::IncompleteFullBlockLots);
        }
        if let (Some(plan), Some(layout)) = (spine_plan, spine_layout.as_ref()) {
            let missing_corners = corner_ids.iter().filter(|id| id.is_none()).count();
            let future_perimeter_ids = required_segments.saturating_mul(2);
            let future_spine_ids = 6_usize.saturating_add(layout.lots.len());
            if self
                .state
                .next_id
                .checked_add(
                    missing_corners
                        .saturating_add(future_perimeter_ids)
                        .saturating_add(future_spine_ids) as u32,
                )
                .is_none()
            {
                return Err(NeighborhoodRejection::SpineIdCapacity);
            }
            let temporary_plan = NeighborhoodPlan {
                anchor_node: anchor.id,
                segments: (0..4)
                    .map(|index| {
                        let from = corner_ids[index].unwrap_or(u32::MAX - index as u32);
                        let to_index = (index + 1) % 4;
                        let to = corner_ids[to_index].unwrap_or(u32::MAX - to_index as u32);
                        (from, to)
                    })
                    .collect(),
                next_segment: 0,
                coordinated_lots: true,
                selected_lots: None,
                spine: Some(plan.clone()),
            };
            if !self.spine_world_geometry_safe_with_context(
                &temporary_plan,
                plan,
                layout,
                collision_context,
            ) {
                return Err(NeighborhoodRejection::SpineWorldGeometry);
            }
        }
        if spine_plan.is_none() {
            if let Some((reused_index, _)) = reused_edge {
                for index in 0..4 {
                    if index == reused_index {
                        continue;
                    }
                    let edge_length =
                        distance(&corner_nodes[index], &corner_nodes[(index + 1) % 4]);
                    if edge_length >= 70.0
                        && proposed_lots
                            .iter()
                            .filter(|lot| lot.frontage_edge == index)
                            .count()
                            < 2
                    {
                        return Err(NeighborhoodRejection::InsufficientSharedEdgeFrontage);
                    }
                }
            }
        }
        for index in 0..4 {
            let a = &corner_nodes[index];
            let b = &corner_nodes[(index + 1) % 4];
            let is_reused_edge = reused_edge.is_some_and(|(edge_index, road_id)| {
                edge_index == index
                    && self.state.roads.iter().any(|road| {
                        road.id == road_id
                            && road.stage == "open"
                            && road.connected
                            && !matches!(road.kind.as_str(), "rail" | "bridge")
                            && ((road.from == a.id && road.to == b.id)
                                || (road.from == b.id && road.to == a.id))
                    })
            });
            if reused_edge.is_some_and(|(edge_index, _)| edge_index == index) && !is_reused_edge {
                return Err(NeighborhoodRejection::ReusedEdgeUnavailable);
            }
            if a.x < MIN + 24.0 || a.x > MAX - 24.0 || a.z < MIN + 24.0 || a.z > MAX - 24.0 {
                return Err(NeighborhoodRejection::RoadCornerOutOfBounds);
            }
            if !self.ordinary_road_clear_of_river(a.x, a.z, b.x, b.z, 4.5) {
                return Err(NeighborhoodRejection::RoadCrossesRiverBuffer);
            }
            if corridor_crosses_protected(a.x, a.z, b.x, b.z) {
                return Err(NeighborhoodRejection::RoadCrossesProtectedLand);
            }
            if !is_reused_edge {
                let heading = (b.z - a.z).atan2(b.x - a.x);
                if corner_ids[index]
                    .is_some_and(|node_id| !self.ordinary_heading_clear(node_id, heading))
                {
                    return Err(NeighborhoodRejection::StartNodeHeadingConflict);
                }
                if corner_ids[(index + 1) % 4].is_some_and(|node_id| {
                    !self.ordinary_heading_clear(node_id, (a.z - b.z).atan2(a.x - b.x))
                }) {
                    return Err(NeighborhoodRejection::EndNodeHeadingConflict);
                }
            }
            // Existing frontage parcels are expected to sit beside a reused
            // open street. Its corridor was already admitted and is not new
            // construction, so do not reject the block because of those
            // lawful neighbors.
            if !is_reused_edge
                && !self.road_corridor_clear_of_parcels_with_context(
                    a.x,
                    a.z,
                    b.x,
                    b.z,
                    collision_context,
                )
            {
                return Err(NeighborhoodRejection::RoadCorridorParcelConflict);
            }
            for road in self.state.roads.iter().filter(|road| road.stage == "open") {
                let (Some(old_a), Some(old_b)) = (self.node(road.from), self.node(road.to)) else {
                    continue;
                };
                if is_reused_edge && reused_edge.is_some_and(|(_, road_id)| road.id == road_id) {
                    continue;
                }
                let shares_corner = [corner_ids[index], corner_ids[(index + 1) % 4]]
                    .into_iter()
                    .flatten()
                    .any(|node_id| road.from == node_id || road.to == node_id);
                if !shares_corner && segments_distance(a, b, old_a, old_b) <= 8.0 {
                    return Err(NeighborhoodRejection::ExistingRoadProximity);
                }
            }
            if self.state.projects.iter().any(|project| {
                if project.kind != "road" || project.progress >= 1.0 {
                    return false;
                }
                let (Some(from), Some(to)) = (project.from, project.to) else {
                    return false;
                };
                let (Some(old_a), Some(old_b)) = (self.node(from), self.node(to)) else {
                    return false;
                };
                let shares_corner = [corner_ids[index], corner_ids[(index + 1) % 4]]
                    .into_iter()
                    .flatten()
                    .any(|node_id| from == node_id || to == node_id);
                !shares_corner && segments_distance(a, b, old_a, old_b) <= 8.0
            }) {
                return Err(NeighborhoodRejection::ActiveRoadProjectProximity);
            }

            let nx = -(b.z - a.z) / distance(a, b).max(1.0);
            let nz = (b.x - a.x) / distance(a, b).max(1.0);
            let midpoint_x = (a.x + b.x) * 0.5;
            let midpoint_z = (a.z + b.z) * 0.5;
            let interior_side =
                if (center.x - midpoint_x) * nx + (center.z - midpoint_z) * nz >= 0.0 {
                    1
                } else {
                    -1
                };
            if selected_lots.is_none() && spine_plan.is_none() {
                let usable_inner_sites = self
                    .frontage_candidate_sites_with_layout(
                        a,
                        b,
                        NEIGHBORHOOD_PARCEL_SIZE,
                        NEIGHBORHOOD_PARCEL_SIZE,
                        NEIGHBORHOOD_FRONTAGE_SPACING,
                    )
                    .into_iter()
                    .filter(|(x, z, _, _, side)| {
                        if *side != interior_side
                            || !self.parcel_geometry_clear(
                                *x,
                                *z,
                                NEIGHBORHOOD_PARCEL_SIZE,
                                NEIGHBORHOOD_PARCEL_SIZE,
                            )
                            || Self::overlaps_station_tower_envelopes(
                                tower_envelopes,
                                *x,
                                *z,
                                NEIGHBORHOOD_PARCEL_SIZE,
                                NEIGHBORHOOD_PARCEL_SIZE,
                            )
                        {
                            return false;
                        }
                        !(0..4).filter(|other| *other != index).any(|other| {
                            let c = &corner_nodes[other];
                            let d = &corner_nodes[(other + 1) % 4];
                            point_segment_distance(*x, *z, c.x, c.z, d.x, d.z)
                                < NEIGHBORHOOD_PARCEL_SIZE * 0.5 + 5.0
                        })
                    })
                    .count();
                let needed_inner_sites = if distance(a, b) >= 70.0 { 2 } else { 1 };
                if usable_inner_sites < needed_inner_sites {
                    return Err(NeighborhoodRejection::InsufficientInteriorFrontage);
                }
            }
            if tower_envelopes
                .iter()
                .any(|(x, z)| point_segment_distance(*x, *z, a.x, a.z, b.x, b.z) < 24.0)
            {
                return Err(NeighborhoodRejection::StationTowerRoadClearance);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod capture_tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn rejection_capture_clears_after_panic() {
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _ = capture_rejections(|| panic!("exercise capture cleanup"));
        }));
        assert!(panicked.is_err());
        let (_, capture) = capture_rejections(|| ());
        assert_eq!(capture.accepted_candidates, 0);
        assert!(capture.reason_counts.is_empty());
    }

    #[test]
    fn nested_rejection_capture_is_rejected_and_cleared() {
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let _ = capture_rejections(|| {
                let _ = capture_rejections(|| ());
            });
        }));
        assert!(panicked.is_err());
        let (_, capture) = capture_rejections(|| ());
        assert_eq!(capture.accepted_candidates, 0);
        assert!(capture.reason_counts.is_empty());
    }
}
