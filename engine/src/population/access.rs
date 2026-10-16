use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AccessPath {
    pub(crate) points: Vec<WorldPoint>,
    /// Per-point absolute walking height. Empty paths use the ground/road policy.
    #[serde(default)]
    pub(crate) surface_y: Vec<f32>,
}

impl AccessPath {
    pub(crate) fn length(&self) -> f32 {
        self.points
            .windows(2)
            .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].z - pair[0].z))
            .sum()
    }

    pub(crate) fn sample(&self, mut progress: f32) -> Option<(WorldPoint, f32)> {
        if self.points.len() < 2 {
            return self.points.first().copied().map(|point| (point, 0.0));
        }
        let segments = self.points.len() - 1;
        for index in 0..segments {
            let pair = &self.points[index..=index + 1];
            let dx = pair[1].x - pair[0].x;
            let dz = pair[1].z - pair[0].z;
            let length = dx.hypot(dz);
            if progress <= length || index + 1 == segments {
                let t = if length <= f32::EPSILON {
                    1.0
                } else {
                    (progress / length).clamp(0.0, 1.0)
                };
                return Some((
                    WorldPoint {
                        x: pair[0].x + dx * t,
                        z: pair[0].z + dz * t,
                    },
                    dz.atan2(dx),
                ));
            }
            progress -= length;
        }
        let last = *self.points.last()?;
        let before = self.points[self.points.len() - 2];
        Some((last, (last.z - before.z).atan2(last.x - before.x)))
    }

    pub(crate) fn sample_surface(&self, progress: f32) -> Option<(WorldPoint, f32, f32)> {
        let (point, heading) = self.sample(progress)?;
        if self.surface_y.len() != self.points.len() {
            return Some((point, heading, -1.0));
        }
        let mut remaining = progress.max(0.0);
        for index in 0..self.points.len().saturating_sub(1) {
            let a = self.points[index];
            let b = self.points[index + 1];
            let length = (b.x - a.x).hypot(b.z - a.z);
            if remaining <= length || index + 2 == self.points.len() {
                let t = if length <= f32::EPSILON {
                    1.0
                } else {
                    (remaining / length).clamp(0.0, 1.0)
                };
                let from = self.surface_y[index];
                let to = self.surface_y[index + 1];
                let y = if from < 0.0 && t < 1.0 {
                    -1.0
                } else if to < 0.0 && t >= 1.0 {
                    -1.0
                } else if from < 0.0 {
                    to
                } else if to < 0.0 {
                    from
                } else {
                    from + (to - from) * t
                };
                return Some((point, heading, y));
            }
            remaining -= length;
        }
        Some((point, heading, *self.surface_y.last()?))
    }

    pub(crate) fn reversed(&self) -> Self {
        Self {
            points: self.points.iter().rev().copied().collect(),
            surface_y: self.surface_y.iter().rev().copied().collect(),
        }
    }
}

impl Engine {
    pub(crate) fn route_endpoint_position(
        &self,
        route: &[u32],
        at_start: bool,
        traveler_id: u32,
        kind: u8,
    ) -> Option<WorldPoint> {
        if route.len() < 2 {
            return None;
        }
        let edge = if at_start {
            (route[0], route[1])
        } else {
            (route[route.len() - 2], route[route.len() - 1])
        };
        let (Some(a), Some(b)) = (self.node(edge.0), self.node(edge.1)) else {
            return None;
        };
        let length = distance(a, b).max(1.0);
        let walking_offset = self
            .state
            .roads
            .iter()
            .find(|road| {
                road.stage == Stage::Open
                    && road.kind != RoadKind::Rail
                    && ((road.from == edge.0 && road.to == edge.1)
                        || (road.from == edge.1 && road.to == edge.0))
            })
            .map(|road| pedestrian_road_offset(Some(road.kind)))
            .unwrap_or_else(|| pedestrian_road_offset(None));
        let lateral = if kind == 1 {
            KERB_LANE
        } else if traveler_id % 2 == 0 {
            walking_offset
        } else {
            -walking_offset
        };
        let nx = -(b.z - a.z) / length;
        let nz = (b.x - a.x) / length;
        let node = if at_start { a } else { b };
        Some(WorldPoint {
            x: node.x + nx * lateral,
            z: node.z + nz * lateral,
        })
    }

    pub(crate) fn building_access_path(
        &self,
        building_id: u32,
        route: &[u32],
        from_building: bool,
        traveler_id: u32,
        kind: u8,
    ) -> Option<AccessPath> {
        let (access_road_id, segment) = self.building_access_segment(building_id)?;
        let building = self.state.buildings.iter().find(|b| b.id == building_id)?;
        let parcel = self
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == building.parcel_id)?;
        let route_position =
            self.route_endpoint_position(route, from_building, traveler_id, kind)?;
        let road = self
            .state
            .roads
            .iter()
            .find(|road| road.id == access_road_id)?;
        let frontage = self
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel.id)?;
        let (a, b) = (self.node(road.from)?, self.node(road.to)?);
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = distance(a, b).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let along = frontage.along.clamp(0.0, 1.0);
        let center = WorldPoint {
            x: a.x + dx * along,
            z: a.z + dz * along,
        };
        let sidewalk = pedestrian_road_offset(Some(road.kind));
        let frontage_side = frontage.side as f32;
        let frontage_walk = *segment.last()?;
        let access_node = if from_building {
            *route.first()?
        } else {
            *route.last()?
        };
        if access_node != road.from && access_node != road.to {
            return None;
        }
        let access_walk = if access_node == road.from { a } else { b };
        let access_walk = WorldPoint {
            x: access_walk.x + nx * frontage_side * sidewalk,
            z: access_walk.z + nz * frontage_side * sidewalk,
        };
        let toward_frontage = if access_node == road.from { 1.0 } else { -1.0 };
        let travel_nx = -dz * toward_frontage / length;
        let travel_nz = dx * toward_frontage / length;
        let access_lane = WorldPoint {
            x: if access_node == road.from { a.x } else { b.x } + travel_nx * KERB_LANE,
            z: if access_node == road.from { a.z } else { b.z } + travel_nz * KERB_LANE,
        };
        let frontage_lane = WorldPoint {
            x: center.x + travel_nx * KERB_LANE,
            z: center.z + travel_nz * KERB_LANE,
        };
        let (start_walk, end_walk) = if from_building {
            (frontage_walk, access_walk)
        } else {
            (access_walk, frontage_walk)
        };
        let door = if kind != 1 { segment[0] } else { frontage_lane };
        let mut points = if kind == 1 && from_building {
            vec![door, access_lane, route_position]
        } else if kind == 1 {
            vec![route_position, access_lane, door]
        } else if from_building {
            legacy::access_connector::pedestrian_points(&segment, end_walk, route_position, true)
        } else {
            legacy::access_connector::pedestrian_points(&segment, start_walk, route_position, false)
        };
        points.dedup_by(|right, left| (right.x - left.x).hypot(right.z - left.z) < 0.01);
        (points.len() >= 2).then_some(AccessPath {
            points,
            surface_y: Vec::new(),
        })
    }

    pub(crate) fn building_access_segment(
        &self,
        building_id: u32,
    ) -> Option<(u32, Vec<WorldPoint>)> {
        let building = self.state.buildings.iter().find(|b| b.id == building_id)?;
        let parcel = self
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == building.parcel_id)?;
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
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = distance(a, b).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let along = frontage.along.clamp(0.0, 1.0);
        let center = WorldPoint {
            x: a.x + dx * along,
            z: a.z + dz * along,
        };
        let sidewalk = pedestrian_road_offset(Some(road.kind));
        let side = frontage.side as f32;
        let frontage_walk = WorldPoint {
            x: center.x + nx * side * sidewalk,
            z: center.z + nz * side * sidewalk,
        };
        let door = WorldPoint {
            x: building.x + building.rotation.sin() * (building.depth * 0.5 + 0.2),
            z: building.z + building.rotation.cos() * (building.depth * 0.5 + 0.2),
        };
        let mut points = vec![door];
        if cfg!(feature = "flag-passage-candidate") || cfg!(feature = "parent-passage-candidate") {
            if let Some(passage) = self
                .masterplan_lot_for_parcel(parcel.id)
                .and_then(|lot| lot.passage)
            {
                points.extend(passage.into_iter().map(|p| WorldPoint { x: p.x, z: p.z }));
            }
        }
        points.push(frontage_walk);
        points.dedup_by(|a, b| (a.x - b.x).hypot(a.z - b.z) < 0.01);
        Some((road.id, points))
    }
}
