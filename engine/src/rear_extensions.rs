//! Persisted, inhabited rear-annex construction lifecycle.
//!
//! The existing building remains open at its original capacity and footprint
//! while the added rear strip is under construction. The expanded parcel is
//! reserved immediately; building geometry and capacity change atomically at
//! project completion.

use super::push_history;
use super::{geometry, CollisionContext, Engine, Parcel, Project, RearExtensionView};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const MAX_PENDING_REAR_EXTENSIONS: usize = 2;
const MAX_BUILDINGS_SCANNED_PER_PASS: usize = 8;
const SCAN_INTERVAL: u64 = 160;
const MAX_DEPTH: f32 = 32.0;
const CANDIDATE_DEPTH_INCREASES: [f32; 3] = [12.0, 8.0, 4.0];
const CONSTRUCTION_DURATION: f32 = 24.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RearExtensionPlan {
    pub project_id: u32,
    pub building_id: u32,
    pub parcel_id: u32,
    pub original: geometry::OrientedRect,
    pub expanded: geometry::OrientedRect,
    pub original_capacity: u16,
    pub additional_capacity: u16,
}

fn footprint_range(parcels: &[Parcel], parcel_id: u32) -> Option<std::ops::Range<usize>> {
    let target = parcels.iter().position(|parcel| parcel.id == parcel_id)?;
    let start = parcels[..target]
        .iter()
        .map(|parcel| if parcel.rotation.is_none() { 2 } else { 1 })
        .sum::<usize>();
    let count = if parcels[target].rotation.is_none() {
        2
    } else {
        1
    };
    Some(start..start + count)
}

fn without_own_footprint(
    context: &CollisionContext,
    parcel_id: u32,
    parcels: &[Parcel],
) -> Option<CollisionContext> {
    let range = footprint_range(parcels, parcel_id)?;
    if range.end > context.parcel_footprints.len() {
        return None;
    }
    let mut copy = context.clone();
    copy.parcel_footprints.drain(range);
    Some(copy)
}

fn extra_capacity(capacity: u16, original: geometry::OrientedRect, delta: f32) -> Option<u16> {
    let area = original.width * original.depth;
    let strip_area = original.width * delta;
    if !area.is_finite() || area <= 0.0 || !strip_area.is_finite() || strip_area <= 0.0 {
        return None;
    }
    let additional = ((capacity as f32 * strip_area / area).ceil() as u32).max(1);
    u16::try_from(additional).ok()
}

fn front_edge(rect: geometry::OrientedRect) -> (f32, f32) {
    (
        rect.x + rect.rotation.sin() * rect.depth * 0.5,
        rect.z + rect.rotation.cos() * rect.depth * 0.5,
    )
}

impl Engine {
    pub(crate) fn rear_extension_views(&self) -> Vec<RearExtensionView> {
        self.state
            .pending_rear_extensions
            .iter()
            .filter_map(|plan| {
                let project = self
                    .state
                    .projects
                    .iter()
                    .find(|project| project.id == plan.project_id)?;
                let building = self
                    .state
                    .buildings
                    .iter()
                    .find(|building| building.id == plan.building_id)?;
                let axis = (plan.original.rotation.sin(), plan.original.rotation.cos());
                let depth = plan.expanded.depth - plan.original.depth;
                Some(RearExtensionView {
                    project_id: plan.project_id,
                    building_id: plan.building_id,
                    x: plan.expanded.x - axis.0 * plan.original.depth * 0.5,
                    z: plan.expanded.z - axis.1 * plan.original.depth * 0.5,
                    width: plan.expanded.width,
                    depth,
                    rotation: plan.expanded.rotation,
                    floors: building.floors,
                    stage: project.stage.clone(),
                    progress: project.progress,
                })
            })
            .collect()
    }

    pub(crate) fn try_start_rear_extension(&mut self) -> bool {
        if self.state.tick % SCAN_INTERVAL != 0
            || self.state.demand <= 1.1
            || self.state.pending_rear_extensions.len() >= MAX_PENDING_REAR_EXTENSIONS
            || !self.project_admission_available()
            || self
                .state
                .projects
                .iter()
                .any(|project| project.kind == "road" && project.progress < 1.0)
        {
            return false;
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
        if connected_ordinary_roads < 40 || self.state.buildings.is_empty() {
            return false;
        }

        let count = self.state.buildings.len();
        let tower_envelopes = self.available_station_tower_envelopes();
        let context = self.collision_context();
        for _ in 0..count.min(MAX_BUILDINGS_SCANNED_PER_PASS) {
            let index = self.state.rear_extension_cursor % count;
            self.state.rear_extension_cursor = (index + 1) % count;
            let building = self.state.buildings[index].clone();
            if !self.rear_extension_building_eligible(&building) {
                continue;
            }
            let Some(parcel_index) = self
                .state
                .parcels
                .iter()
                .position(|parcel| parcel.id == building.parcel_id)
            else {
                continue;
            };
            let parcel = self.state.parcels[parcel_index].clone();
            let Some(rotation) = parcel.rotation else {
                continue;
            };
            let original = geometry::OrientedRect {
                x: parcel.x,
                z: parcel.z,
                width: parcel.width,
                depth: parcel.depth,
                rotation,
            };
            if !rects_match_building(original, &building) {
                continue;
            }
            let Some(outward) = self.rear_extension_outward(&parcel, original) else {
                continue;
            };
            // The street-facing entrance is at local +Z. A rear annex must
            // extend along local -Z, keeping the building's front edge fixed.
            let local_z = (rotation.sin(), rotation.cos());
            if outward.0 * local_z.0 + outward.1 * local_z.1 > -0.95 {
                continue;
            }
            let Some(candidate_context) =
                without_own_footprint(&context, parcel.id, &self.state.parcels)
            else {
                continue;
            };
            let Some(candidate) = CANDIDATE_DEPTH_INCREASES.iter().find_map(|delta| {
                let expanded = geometry::OrientedRect {
                    x: original.x + outward.0 * *delta * 0.5,
                    z: original.z + outward.1 * *delta * 0.5,
                    width: original.width,
                    depth: original.depth + *delta,
                    rotation,
                };
                if expanded.depth > MAX_DEPTH
                    || !self
                        .oriented_parcel_geometry_clear_with_context(&expanded, &candidate_context)
                {
                    return None;
                }
                let (min_x, min_z, max_x, max_z) = expanded.bounds();
                if Self::overlaps_station_tower_envelopes(
                    &tower_envelopes,
                    expanded.x,
                    expanded.z,
                    max_x - min_x,
                    max_z - min_z,
                ) {
                    return None;
                }
                let before_front = front_edge(original);
                let after_front = front_edge(expanded);
                if (before_front.0 - after_front.0).hypot(before_front.1 - after_front.1) > 0.01 {
                    return None;
                }
                let additional_capacity = extra_capacity(building.capacity, original, *delta)?;
                building.capacity.checked_add(additional_capacity)?;
                Some((expanded, *delta, additional_capacity))
            }) else {
                continue;
            };
            let (expanded, delta, additional_capacity) = candidate;
            let Some(footprint_range) = footprint_range(&self.state.parcels, parcel.id) else {
                continue;
            };
            if footprint_range.len() != 1 {
                continue;
            }
            let project_id = self.id();
            self.state.parcels[parcel_index].x = expanded.x;
            self.state.parcels[parcel_index].z = expanded.z;
            self.state.parcels[parcel_index].width = expanded.width;
            self.state.parcels[parcel_index].depth = expanded.depth;
            self.state.parcels[parcel_index].reservation = Some(building.id);
            self.state.pending_rear_extensions.push(RearExtensionPlan {
                project_id,
                building_id: building.id,
                parcel_id: parcel.id,
                original,
                expanded,
                original_capacity: building.capacity,
                additional_capacity,
            });
            self.record_project(Project {
                id: project_id,
                kind: "extension".into(),
                from: None,
                to: None,
                parcel_id: Some(parcel.id),
                stage: "survey".into(),
                progress: 0.0,
                reason: format!("Rear annex adds {delta:.0} m of depth to {}", building.name),
                duration: CONSTRUCTION_DURATION,
                started_tick: self.state.tick,
            });
            self.mark_access_geometry_changed();
            self.record(
                "RearExtensionStarted",
                building.id,
                format!("A phased rear annex begins behind {}", building.name),
            );
            self.state.revision = self.state.revision.saturating_add(1);
            return true;
        }
        false
    }

    fn rear_extension_building_eligible(&self, building: &super::Building) -> bool {
        if self.errand_references_building(building.id)
            || building.stage != "open"
            || building.anchor
            || building.floors == 0
            || building.floors > 8
            || building.capacity == 0
            || self
                .state
                .pending_rear_extensions
                .iter()
                .any(|plan| plan.building_id == building.id)
            || self.state.projects.iter().any(|project| {
                project.parcel_id == Some(building.parcel_id)
                    && project.progress < 1.0
                    && matches!(project.kind.as_str(), "building" | "extension")
            })
        {
            return false;
        }
        if super::is_housing(building) {
            if building.occupancy == 0
                || building.occupancy.saturating_mul(5) < building.capacity.saturating_mul(4)
            {
                return false;
            }
        } else if super::is_commercial(building) {
            if !building.business_active {
                return false;
            }
        } else {
            return false;
        }
        let Some(parcel) = self
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == building.parcel_id)
        else {
            return false;
        };
        if parcel.protected || parcel.reservation.is_some() || parcel.rotation.is_none() {
            return false;
        }
        let Some(frontage) = self
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel.id)
        else {
            return false;
        };
        let Some(road) = self
            .state
            .roads
            .iter()
            .find(|road| road.id == frontage.road_id)
        else {
            return false;
        };
        road.stage == "open"
            && road.connected
            && !matches!(road.kind.as_str(), "rail" | "bridge")
            && self.node_connected(parcel.access_node)
    }

    fn rear_extension_outward(
        &self,
        parcel: &Parcel,
        rect: geometry::OrientedRect,
    ) -> Option<(f32, f32)> {
        let frontage = self
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel.id)?;
        let road = self
            .state
            .roads
            .iter()
            .find(|road| road.id == frontage.road_id)?;
        let (a, b) = (self.node(road.from)?, self.node(road.to)?);
        let (dx, dz) = (b.x - a.x, b.z - a.z);
        let length = dx.hypot(dz);
        if length <= f32::EPSILON || !matches!(frontage.side, -1 | 1) {
            return None;
        }
        let along =
            (((rect.x - a.x) * dx + (rect.z - a.z) * dz) / (length * length)).clamp(0.0, 1.0);
        let projection = (a.x + dx * along, a.z + dz * along);
        let from_road = (rect.x - projection.0, rect.z - projection.1);
        let outward = (
            -dz / length * frontage.side as f32,
            dx / length * frontage.side as f32,
        );
        let depth_axis = (rect.rotation.sin(), rect.rotation.cos());
        let width_axis = (rect.rotation.cos(), -rect.rotation.sin());
        let depth_offset = from_road.0 * outward.0 + from_road.1 * outward.1;
        let lateral_offset = from_road.0 * width_axis.0 + from_road.1 * width_axis.1;
        let frame_alignment = (outward.0 * depth_axis.0 + outward.1 * depth_axis.1).abs();
        (depth_offset >= rect.depth * 0.25
            && lateral_offset.abs() <= rect.width * 0.5 + 3.0
            && frame_alignment >= 0.95)
            .then_some(outward)
    }

    fn rear_extension_plan_geometry_is_clear(
        &self,
        plan: &RearExtensionPlan,
        parcel: &Parcel,
    ) -> bool {
        let delta = plan.expanded.depth - plan.original.depth;
        if !CANDIDATE_DEPTH_INCREASES
            .iter()
            .any(|candidate| (candidate - delta).abs() < 0.01)
        {
            return false;
        }
        let Some(outward) = self.rear_extension_outward(parcel, plan.original) else {
            return false;
        };
        let expected = geometry::OrientedRect {
            x: plan.original.x + outward.0 * delta * 0.5,
            z: plan.original.z + outward.1 * delta * 0.5,
            width: plan.original.width,
            depth: plan.original.depth + delta,
            rotation: plan.original.rotation,
        };
        if (expected.x - plan.expanded.x).abs() > 0.01
            || (expected.z - plan.expanded.z).abs() > 0.01
            || (expected.width - plan.expanded.width).abs() > 0.01
            || (expected.depth - plan.expanded.depth).abs() > 0.01
            || (expected.rotation - plan.expanded.rotation).abs() > 0.001
            || (front_edge(plan.original).0 - front_edge(plan.expanded).0)
                .hypot(front_edge(plan.original).1 - front_edge(plan.expanded).1)
                > 0.01
            || extra_capacity(plan.original_capacity, plan.original, delta)
                != Some(plan.additional_capacity)
        {
            return false;
        }
        let Some(context) = without_own_footprint(
            &self.collision_context(),
            plan.parcel_id,
            &self.state.parcels,
        ) else {
            return false;
        };
        if !self.oriented_parcel_geometry_clear_with_context(&plan.expanded, &context) {
            return false;
        }
        let (min_x, min_z, max_x, max_z) = plan.expanded.bounds();
        !Self::overlaps_station_tower_envelopes(
            &self.available_station_tower_envelopes(),
            plan.expanded.x,
            plan.expanded.z,
            max_x - min_x,
            max_z - min_z,
        )
    }

    pub(crate) fn complete_rear_extension_project(&mut self, project_id: u32) {
        let Some(plan_index) = self
            .state
            .pending_rear_extensions
            .iter()
            .position(|plan| plan.project_id == project_id)
        else {
            if self.state.projects.iter().any(|project| {
                project.id == project_id && project.kind == "extension" && project.progress >= 1.0
            }) {
                return;
            }
            self.keep_rear_extension_retryable(project_id);
            return;
        };
        let plan = self.state.pending_rear_extensions[plan_index].clone();
        let project_valid = self.state.projects.iter().any(|project| {
            project.id == project_id
                && project.kind == "extension"
                && project.progress >= 1.0
                && project.parcel_id == Some(plan.parcel_id)
        });
        let parcel = self
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == plan.parcel_id);
        let building = self
            .state
            .buildings
            .iter()
            .find(|building| building.id == plan.building_id);
        let valid = project_valid
            && parcel.is_some_and(|parcel| {
                parcel.reservation == Some(plan.building_id)
                    && parcel_rect(parcel) == plan.expanded
                    && self.rear_extension_plan_geometry_is_clear(&plan, parcel)
            })
            && building.is_some_and(|building| {
                building.parcel_id == plan.parcel_id
                    && building.stage == "open"
                    && building.capacity == plan.original_capacity
                    && rects_match_building(plan.original, building)
                    && building
                        .capacity
                        .checked_add(plan.additional_capacity)
                        .is_some()
            });
        if !valid {
            self.keep_rear_extension_retryable(project_id);
            return;
        }

        if !self
            .state
            .building_original_footprints
            .iter()
            .any(|footprint| footprint.building_id == plan.building_id)
        {
            self.state
                .building_original_footprints
                .push(super::BuildingOriginalFootprint {
                    building_id: plan.building_id,
                    x: plan.original.x,
                    z: plan.original.z,
                    width: plan.original.width,
                    depth: plan.original.depth,
                    rotation: plan.original.rotation,
                });
        }

        let building = self
            .state
            .buildings
            .iter_mut()
            .find(|building| building.id == plan.building_id)
            .expect("preflighted rear extension building remains present");
        building.x = plan.expanded.x;
        building.z = plan.expanded.z;
        building.width = plan.expanded.width;
        building.depth = plan.expanded.depth;
        building.capacity += plan.additional_capacity;
        let parcel = self
            .state
            .parcels
            .iter_mut()
            .find(|parcel| parcel.id == plan.parcel_id)
            .expect("preflighted rear extension parcel remains present");
        parcel.reservation = None;
        self.state.pending_rear_extensions.remove(plan_index);
        if let Some(project) = self
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.stage = "finishing".into();
            project.progress = 1.0;
        }
        push_history(
            &mut self
                .state
                .buildings
                .iter_mut()
                .find(|building| building.id == plan.building_id)
                .expect("building remains present")
                .history,
            format!(
                "Rear annex completed: {:.1} m² of nominal site extension",
                plan.expanded.width * (plan.expanded.depth - plan.original.depth)
            ),
        );
        self.mark_access_geometry_changed();
        self.record(
            "RearExtensionCompleted",
            plan.building_id,
            "A phased rear annex completes without moving the occupied entrance".into(),
        );
        self.state.revision = self.state.revision.saturating_add(1);
        self.trim_completed_projects();
    }

    fn keep_rear_extension_retryable(&mut self, project_id: u32) {
        if let Some(project) = self
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == project_id && project.kind == "extension")
        {
            project.progress = 0.999;
            project.stage = "finishing".into();
        }
    }

    pub(crate) fn validate_rear_extensions(&self) -> Result<(), String> {
        if self.state.pending_rear_extensions.len() > MAX_PENDING_REAR_EXTENSIONS {
            return Err("pending rear-extension cap exceeded".into());
        }
        let mut project_ids = BTreeSet::new();
        let mut building_ids = BTreeSet::new();
        let mut parcel_ids = BTreeSet::new();
        for plan in &self.state.pending_rear_extensions {
            if !project_ids.insert(plan.project_id)
                || !building_ids.insert(plan.building_id)
                || !parcel_ids.insert(plan.parcel_id)
            {
                return Err("rear-extension plans contain duplicate ids".into());
            }
            let project = self
                .state
                .projects
                .iter()
                .find(|project| project.id == plan.project_id)
                .ok_or_else(|| "rear-extension plan references a missing project".to_string())?;
            let parcel = self
                .state
                .parcels
                .iter()
                .find(|parcel| parcel.id == plan.parcel_id)
                .ok_or_else(|| "rear-extension plan references a missing parcel".to_string())?;
            let building = self
                .state
                .buildings
                .iter()
                .find(|building| building.id == plan.building_id)
                .ok_or_else(|| "rear-extension plan references a missing building".to_string())?;
            let expanded_front = front_edge(plan.expanded);
            let original_front = front_edge(plan.original);
            let strip_depth = plan.expanded.depth - plan.original.depth;
            let rects_contain = plan
                .original
                .corners()
                .iter()
                .all(|(x, z)| point_in_rect(plan.expanded, (*x, *z), 0.01));
            let frontage_road_connected = self
                .state
                .parcel_frontages
                .iter()
                .find(|frontage| frontage.parcel_id == parcel.id)
                .and_then(|frontage| {
                    self.state
                        .roads
                        .iter()
                        .find(|road| road.id == frontage.road_id)
                })
                .is_some_and(|road| {
                    road.stage == "open"
                        && road.connected
                        && !matches!(road.kind.as_str(), "rail" | "bridge")
                });
            if project.kind != "extension"
                || project.progress < 0.0
                || project.progress >= 1.0
                || project.parcel_id != Some(plan.parcel_id)
                || !matches!(
                    project.stage.as_str(),
                    "survey" | "clearing" | "structure" | "surface" | "finishing"
                )
                || !project.progress.is_finite()
                || !project.duration.is_finite()
                || project.duration <= 0.0
                || plan.original_capacity == 0
                || plan.additional_capacity == 0
                || plan.original_capacity != building.capacity
                || building.stage != "open"
                || building.anchor
                || building.floors == 0
                || building.floors > 8
                || !(super::is_housing(building)
                    || (super::is_commercial(building) && building.business_active))
                || building.parcel_id != plan.parcel_id
                || building.occupancy > plan.original_capacity
                || !rects_match_building(plan.original, building)
                || parcel.protected
                || parcel.rotation.is_none()
                || parcel.reservation != Some(plan.building_id)
                || parcel_rect(parcel) != plan.expanded
                || !plan.original.finite_positive()
                || !plan.expanded.finite_positive()
                || !rects_contain
                || (expanded_front.0 - original_front.0).hypot(expanded_front.1 - original_front.1)
                    > 0.01
                || (plan.expanded.width - plan.original.width).abs() > 0.01
                || strip_depth <= 0.0
                || plan.expanded.depth > MAX_DEPTH
                || plan.original.rotation != plan.expanded.rotation
                || !frontage_road_connected
                || plan
                    .original_capacity
                    .checked_add(plan.additional_capacity)
                    .is_none()
                || !self.rear_extension_plan_geometry_is_clear(plan, parcel)
                || !self.node_connected(parcel.access_node)
            {
                return Err(format!(
                    "rear-extension plan {} is inconsistent",
                    plan.project_id
                ));
            }
        }
        let orphan_project = self.state.projects.iter().any(|project| {
            project.kind == "extension"
                && project.progress < 1.0
                && !project_ids.contains(&project.id)
        });
        if orphan_project {
            return Err("active extension project has no rear-extension plan".into());
        }
        Ok(())
    }

    pub(crate) fn validate_building_original_footprints(&self) -> Result<(), String> {
        if self.state.building_original_footprints.len() > self.state.buildings.len()
            || self.state.building_original_footprints.len() > super::MAX_BUILDINGS
        {
            return Err("original building footprint metadata exceeds its host cap".into());
        }
        let mut building_ids = BTreeSet::new();
        for original in &self.state.building_original_footprints {
            if !building_ids.insert(original.building_id) {
                return Err("original building footprint metadata contains duplicate hosts".into());
            }
            let host = self
                .state
                .buildings
                .iter()
                .find(|building| building.id == original.building_id)
                .filter(|building| building.stage != "demolished")
                .ok_or_else(|| {
                    format!(
                        "original building footprint {} has no live host",
                        original.building_id
                    )
                })?;
            let original_rect = geometry::OrientedRect {
                x: original.x,
                z: original.z,
                width: original.width,
                depth: original.depth,
                rotation: original.rotation,
            };
            let current_rect = geometry::OrientedRect {
                x: host.x,
                z: host.z,
                width: host.width,
                depth: host.depth,
                rotation: host.rotation,
            };
            let same_rotation = (original.rotation.sin() - host.rotation.sin()).abs() <= 1e-4
                && (original.rotation.cos() - host.rotation.cos()).abs() <= 1e-4;
            let contained = original_rect
                .corners()
                .iter()
                .all(|corner| point_in_rect(current_rect, *corner, 0.01));
            let original_front = front_edge(original_rect);
            let current_front = front_edge(current_rect);
            let same_front = (original_front.0 - current_front.0)
                .hypot(original_front.1 - current_front.1)
                <= 0.01;
            if !original_rect.finite_positive()
                || !host.x.is_finite()
                || !host.z.is_finite()
                || !host.width.is_finite()
                || !host.depth.is_finite()
                || !host.rotation.is_finite()
                || (original.width - host.width).abs() > 0.01
                || !same_rotation
                || !contained
                || !same_front
            {
                return Err(format!(
                    "original building footprint {} contradicts its live host",
                    original.building_id
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn mature_engine_with_extension_site(x: f32, z: f32) -> (Engine, u32) {
        let mut engine = Engine::new_legacy(87231);
        let host = engine
            .state
            .roads
            .iter()
            .find(|road| {
                road.stage == "open"
                    && road.connected
                    && road.kind == "local"
                    && (engine.node(road.from).unwrap().z - engine.node(road.to).unwrap().z).abs()
                        < 0.01
            })
            .cloned()
            .expect("founding world has an open horizontal local road");
        for _ in 0..36 {
            let id = engine.add_road(host.from, host.to, "local", "open", 1.0);
            engine
                .state
                .roads
                .iter_mut()
                .find(|road| road.id == id)
                .unwrap()
                .connected = true;
        }
        engine.recompute_connectivity();
        // This mature fixture has no unrelated active road-construction work.
        engine
            .state
            .projects
            .retain(|project| project.kind != "road" || project.progress >= 1.0);
        engine.state.tick = 160;
        engine.state.demand = 1.2;

        let (a, b) = (
            engine.node(host.from).unwrap(),
            engine.node(host.to).unwrap(),
        );
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length2 = dx * dx + dz * dz;
        let along = (((x - a.x) * dx + (z - a.z) * dz) / length2).clamp(0.0, 1.0);
        let projection = (a.x + dx * along, a.z + dz * along);
        let nx = -dz / length2.sqrt();
        let nz = dx / length2.sqrt();
        let side = if (x - projection.0) * nx + (z - projection.1) * nz >= 0.0 {
            1
        } else {
            -1
        };
        let rotation = (side as f32 * dz).atan2(-side as f32 * dx);
        let parcel_id = engine.id();
        engine.state.parcels.push(Parcel {
            id: parcel_id,
            x,
            z,
            width: 14.0,
            depth: 12.0,
            rotation: Some(rotation),
            access_node: host.from,
            suitability: 0.9,
            protected: false,
            reservation: None,
            open_space: None,
        });
        engine
            .state
            .parcel_frontages
            .push(super::super::ParcelFrontage {
                parcel_id,
                road_id: host.id,
                along,
                side,
            });
        let building_id = engine.id();
        engine.state.buildings.push(super::super::Building {
            id: building_id,
            parcel_id,
            x,
            z,
            rotation,
            width: 14.0,
            depth: 12.0,
            floors: 4,
            kind: "apartment".into(),
            stage: "open".into(),
            progress: 1.0,
            capacity: 20,
            occupancy: 20,
            built_day: 0,
            name: "Test Walk-up".into(),
            appearance_seed: 123,
            history: Vec::new(),
            anchor: false,
            vacant_days: 0,
            business_active: false,
        });
        (engine, building_id)
    }

    fn assert_same_access_segment(
        actual: (u32, Vec<super::super::WorldPoint>),
        expected: &(u32, Vec<super::super::WorldPoint>),
    ) {
        assert_eq!(actual.0, expected.0);
        assert_eq!(actual.1.len(), expected.1.len());
        for (actual, expected) in actual.1.iter().zip(expected.1.iter()) {
            assert!((actual.x - expected.x).abs() < 0.001);
            assert!((actual.z - expected.z).abs() < 0.001);
        }
    }

    #[test]
    fn inhabited_extension_stays_original_until_completion_and_resumes_each_phase() {
        let (mut engine, building_id) = mature_engine_with_extension_site(27.0, 22.0);
        let before_access = engine.building_access_segment(building_id).unwrap();
        let original_building = engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap()
            .clone();
        assert!(
            engine.try_start_rear_extension(),
            "eligible mature site admits"
        );
        let plan = engine.state.pending_rear_extensions[0].clone();
        let project = engine
            .state
            .projects
            .iter()
            .find(|project| project.id == plan.project_id)
            .unwrap()
            .clone();
        assert_eq!(project.kind, "extension");
        assert_eq!(project.parcel_id, Some(plan.parcel_id));
        assert_eq!(
            engine
                .state
                .parcels
                .iter()
                .find(|parcel| parcel.id == plan.parcel_id)
                .unwrap()
                .reservation,
            Some(building_id)
        );
        let local_z = (plan.original.rotation.sin(), plan.original.rotation.cos());
        let strip_probe = geometry::OrientedRect {
            x: plan.original.x - local_z.0 * (plan.original.depth * 0.5 + 1.0),
            z: plan.original.z - local_z.1 * (plan.original.depth * 0.5 + 1.0),
            width: 4.0,
            depth: 2.0,
            rotation: plan.original.rotation,
        };
        assert!(
            !engine.oriented_parcel_geometry_clear_with_context(
                &strip_probe,
                &engine.collision_context()
            ),
            "the expanded parcel footprint immediately blocks conflicting geometry"
        );
        assert_eq!(
            engine
                .state
                .buildings
                .iter()
                .find(|building| building.id == building_id)
                .unwrap()
                .capacity,
            original_building.capacity,
            "capacity stays fixed while the annex is being built"
        );
        assert_same_access_segment(
            engine.building_access_segment(building_id).unwrap(),
            &before_access,
        );
        engine.validate().unwrap();

        let mut replay = Engine::restore_json(&engine.save_json()).unwrap();
        let mut observed_stages = BTreeSet::new();
        for _ in 0..24 {
            engine.advance_projects();
            replay.advance_projects();
            assert_eq!(engine.save_json(), replay.save_json());
            if let Some(project) = engine
                .state
                .projects
                .iter()
                .find(|project| project.id == plan.project_id && project.progress < 1.0)
            {
                observed_stages.insert(project.stage.clone());
                let building = engine
                    .state
                    .buildings
                    .iter()
                    .find(|building| building.id == building_id)
                    .unwrap();
                assert_eq!(building.stage, "open");
                assert_eq!(building.capacity, original_building.capacity);
                assert!((building.x - original_building.x).abs() < 0.001);
                assert!((building.z - original_building.z).abs() < 0.001);
                assert!((building.depth - original_building.depth).abs() < 0.001);
            }
            engine.validate().unwrap();
        }
        assert!(observed_stages.contains("survey"));
        assert!(observed_stages.contains("clearing"));
        assert!(observed_stages.contains("structure"));
        assert!(observed_stages.contains("surface"));
        assert!(observed_stages.contains("finishing"));
        assert!(engine.state.pending_rear_extensions.is_empty());
        let completed = engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap();
        let completed_capacity = completed.capacity;
        let completed_depth = completed.depth;
        assert_eq!(
            completed.capacity,
            plan.original_capacity + plan.additional_capacity
        );
        assert_eq!(completed.stage, "open");
        assert_same_access_segment(
            engine.building_access_segment(building_id).unwrap(),
            &before_access,
        );
        assert_eq!(
            engine
                .state
                .events
                .iter()
                .filter(|event| event.kind == "RearExtensionCompleted"
                    && event.entity_id == building_id)
                .count(),
            1
        );
        let completed_save = engine.save_json();
        let original_footprint = engine.state.building_original_footprints[0];
        let persisted = Engine::restore_json(&completed_save).unwrap();
        assert_eq!(
            persisted.state.building_original_footprints[0],
            original_footprint
        );
        assert_eq!(
            persisted.snapshot().building_original_footprints[0].building_id,
            building_id
        );
        assert_eq!(original_footprint.building_id, building_id);
        assert!((original_footprint.x - original_building.x).abs() < 0.001);
        assert!((original_footprint.z - original_building.z).abs() < 0.001);
        assert!((original_footprint.width - original_building.width).abs() < 0.001);
        assert!((original_footprint.depth - original_building.depth).abs() < 0.001);
        assert!((original_footprint.rotation - original_building.rotation).abs() < 0.001);
        engine.complete_project(plan.project_id);
        assert_eq!(
            engine.save_json(),
            completed_save,
            "completion is idempotent"
        );
        assert_eq!(
            engine
                .state
                .events
                .iter()
                .filter(|event| event.kind == "RearExtensionCompleted"
                    && event.entity_id == building_id)
                .count(),
            1
        );
        let mut corrupted: serde_json::Value = serde_json::from_str(&completed_save).unwrap();
        corrupted["building_original_footprints"][0]["width"] =
            serde_json::json!(original_footprint.width + 1.0);
        assert!(Engine::restore_json(&corrupted.to_string())
            .err()
            .unwrap()
            .contains("original building footprint"));

        // A later annex may extend the same host further, but the saved
        // facade identity must remain the earliest footprint.
        engine
            .state
            .buildings
            .iter_mut()
            .find(|building| building.id == building_id)
            .unwrap()
            .occupancy = completed_capacity;
        engine.state.tick = 320;
        assert!(engine.try_start_rear_extension());
        let second_plan = engine.state.pending_rear_extensions[0].clone();
        assert!((second_plan.original.depth - completed_depth).abs() < 0.001);
        for _ in 0..24 {
            engine.advance_projects();
        }
        assert!(engine.state.pending_rear_extensions.is_empty());
        assert_eq!(
            engine.state.building_original_footprints[0],
            original_footprint
        );
        engine.validate().unwrap();
    }

    #[test]
    fn extension_schema_rejects_legacy_activity_saves() {
        let engine = Engine::new_legacy(13);
        for old_version in [2, 3] {
            let mut value: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
            value["activity_schema_version"] = serde_json::json!(old_version);
            let error = Engine::restore_json(&value.to_string()).err().expect("legacy activity saves are rejected");
            assert!(error.contains("unsupported business activity schema"), "{error}");
        }
    }

    #[test]
    #[ignore = "bounded normal-scheduler fixture export for visual review"]
    fn export_first_mature_rear_extension_fixture() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let input = std::env::var_os("CITY_TRACE_SAVE")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                manifest
                    .parent()
                    .unwrap()
                    .join(".tools/narrow-frontage-review/final-save.json")
            });
        let source = fs::read_to_string(&input).expect("read mature rear-extension save");
        let mut engine = Engine::restore_json(&source).expect("restore mature rear-extension save");
        let source_tick = engine.state.tick;
        let source_seed = engine.state.seed;
        let source_demand = engine.state.demand;
        let max_steps = 48_000;
        let mut simulated_ticks = 0;
        let has_structure_phase =
            |engine: &Engine| {
                engine.state.pending_rear_extensions.iter().any(|plan| {
                    engine.state.projects.iter().any(|project| {
                        project.id == plan.project_id && project.stage == "structure"
                    })
                })
            };
        while simulated_ticks < max_steps
            && (engine.state.pending_rear_extensions.is_empty() || !has_structure_phase(&engine))
        {
            let next = (max_steps - simulated_ticks).min(160);
            engine.step(next);
            simulated_ticks += next;
        }

        let stage = engine
            .state
            .pending_rear_extensions
            .first()
            .and_then(|plan| {
                engine
                    .state
                    .projects
                    .iter()
                    .find(|project| project.id == plan.project_id)
                    .map(|project| project.stage.as_str())
            })
            .unwrap_or("none");
        if !engine.state.pending_rear_extensions.is_empty() {
            engine
                .validate()
                .expect("normal-scheduler extension save passes engine validation");
            if let Some(path) = std::env::var_os("REAR_EXTENSION_FIXTURE_OUT").map(PathBuf::from) {
                fs::write(&path, engine.save_json()).expect("write active extension save");
                eprintln!("rear-extension save: {}", path.display());
            }
            if let Some(path) = std::env::var_os("REAR_EXTENSION_SNAPSHOT_OUT").map(PathBuf::from) {
                fs::write(&path, engine.snapshot_json()).expect("write active extension snapshot");
                eprintln!("rear-extension snapshot: {}", path.display());
            }
        }
        eprintln!(
            "normal-scheduler rear-extension fixture: input={} seed={} source_tick={} source_demand={:.2} simulated_ticks={} final_tick={} buildings={} roads={} pending_extensions={} stage={}",
            input.display(),
            source_seed,
            source_tick,
            source_demand,
            simulated_ticks,
            engine.state.tick,
            engine.state.buildings.len(),
            engine.state.roads.len(),
            engine.state.pending_rear_extensions.len(),
            stage,
        );
        assert!(engine.state.pending_rear_extensions.len() <= MAX_PENDING_REAR_EXTENSIONS);
    }
}

fn parcel_rect(parcel: &Parcel) -> geometry::OrientedRect {
    geometry::OrientedRect {
        x: parcel.x,
        z: parcel.z,
        width: parcel.width,
        depth: parcel.depth,
        rotation: parcel.rotation.unwrap_or(0.0),
    }
}

fn rects_match_building(rect: geometry::OrientedRect, building: &super::Building) -> bool {
    (rect.x - building.x).abs() <= 0.01
        && (rect.z - building.z).abs() <= 0.01
        && (rect.width - building.width).abs() <= 0.01
        && (rect.depth - building.depth).abs() <= 0.01
        && (rect.rotation.sin() - building.rotation.sin()).abs() <= 1e-4
        && (rect.rotation.cos() - building.rotation.cos()).abs() <= 1e-4
}

fn point_in_rect(rect: geometry::OrientedRect, point: (f32, f32), tolerance: f32) -> bool {
    let dx = point.0 - rect.x;
    let dz = point.1 - rect.z;
    let (sin, cos) = rect.rotation.sin_cos();
    (dx * cos - dz * sin).abs() <= rect.width * 0.5 + tolerance
        && (dx * sin + dz * cos).abs() <= rect.depth * 0.5 + tolerance
}
