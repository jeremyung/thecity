use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RailPlatformView {
    pub node_id: u32,
    pub x: f32,
    pub z: f32,
    /// Outward direction from this terminal; local +X is (uz, -ux).
    pub ux: f32,
    pub uz: f32,
    pub surface_y: f32,
    pub stair_x: f32,
    pub stair_bottom_z: f32,
    pub stair_top_z: f32,
    pub ground_y: f32,
    pub stair_steps: u8,
}

impl Engine {
    pub(super) fn rail_platform_views(&self) -> Vec<RailPlatformView> {
        let Some(rail) = self.state.roads.iter().find(|road| road.kind == "rail") else {
            return Vec::new();
        };
        let Some(station) = [rail.from, rail.to]
            .into_iter()
            .find(|id| self.node(*id).is_some_and(|node| node.kind == "station"))
        else {
            return Vec::new();
        };
        let gateway = if station == rail.from {
            rail.to
        } else {
            rail.from
        };
        let (Some(a), Some(b)) = (self.node(station), self.node(gateway)) else {
            return Vec::new();
        };
        let length = distance(a, b).max(1.0);
        let (ux, uz) = ((b.x - a.x) / length, (b.z - a.z) / length);
        [(station, ux, uz), (gateway, -ux, -uz)]
            .into_iter()
            .filter_map(|(node_id, ux, uz)| {
                let node = self.node(node_id)?;
                Some(RailPlatformView {
                    node_id,
                    x: node.x,
                    z: node.z,
                    ux,
                    uz,
                    surface_y: 8.0,
                    stair_x: 4.05,
                    stair_bottom_z: -20.0,
                    stair_top_z: -1.0,
                    ground_y: 0.27,
                    stair_steps: 44,
                })
            })
            .collect()
    }

    pub(super) fn platform_position(&self, at_station: bool, slot: u16) -> Option<WorldPoint> {
        let (station, gateway, ux, uz) = self.rail_axis()?;
        let terminal = if at_station {
            self.node(station)?
        } else {
            self.node(gateway)?
        };
        let (dx, dz) = if at_station { (ux, uz) } else { (-ux, -uz) };
        let (nx, nz) = (dz, -dx);
        let along = 2.0 + (slot / 2) as f32 * 0.9;
        let across = if slot % 2 == 0 { 3.6 } else { 4.5 };
        Some(WorldPoint {
            x: terminal.x + dx * along + nx * across,
            z: terminal.z + dz * along + nz * across,
        })
    }

    fn platform_stairs(&self, at_station: bool) -> Option<(WorldPoint, WorldPoint, WorldPoint)> {
        let (station, gateway, ux, uz) = self.rail_axis()?;
        let terminal = if at_station {
            self.node(station)?
        } else {
            self.node(gateway)?
        };
        let (dx, dz) = if at_station { (ux, uz) } else { (-ux, -uz) };
        let (nx, nz) = (dz, -dx);
        let point = |x: f32, z: f32| WorldPoint {
            x: terminal.x + nx * x + dx * z,
            z: terminal.z + nz * x + dz * z,
        };
        Some((point(4.05, -20.0), point(4.05, -1.0), point(4.05, 1.0)))
    }

    fn platform_point(&self, at_station: bool, local_x: f32, local_z: f32) -> Option<WorldPoint> {
        let (station, gateway, ux, uz) = self.rail_axis()?;
        let terminal = if at_station {
            self.node(station)?
        } else {
            self.node(gateway)?
        };
        let (dx, dz) = if at_station { (ux, uz) } else { (-ux, -uz) };
        Some(WorldPoint {
            x: terminal.x + dz * local_x + dx * local_z,
            z: terminal.z - dx * local_x + dz * local_z,
        })
    }

    fn platform_local_z(&self, at_station: bool, point: WorldPoint) -> Option<f32> {
        let (station, gateway, ux, uz) = self.rail_axis()?;
        let terminal = if at_station {
            self.node(station)?
        } else {
            self.node(gateway)?
        };
        let (dx, dz) = if at_station { (ux, uz) } else { (-ux, -uz) };
        Some((point.x - terminal.x) * dx + (point.z - terminal.z) * dz)
    }

    pub(super) fn platform_entry_path(
        &self,
        route: &[u32],
        at_station: bool,
        slot: u16,
        traveler_id: u32,
    ) -> Option<AccessPath> {
        let from = self.route_endpoint_position(route, false, traveler_id, 0)?;
        let to = self.platform_position(at_station, slot)?;
        self.platform_entry_path_from_position(from, to, at_station, traveler_id)
    }

    pub(super) fn platform_entry_path_from_position(
        &self,
        from: WorldPoint,
        to: WorldPoint,
        at_station: bool,
        _traveler_id: u32,
    ) -> Option<AccessPath> {
        let (stair_bottom, stair_top, _) = self.platform_stairs(at_station)?;
        let aisle =
            self.platform_point(at_station, 4.05, self.platform_local_z(at_station, to)?)?;
        Some(AccessPath {
            points: vec![from, stair_bottom, stair_top, aisle, to],
            surface_y: vec![-1.0, 0.27, 8.0, 8.0, 8.0],
        })
    }

    pub(super) fn platform_exit_path(
        &self,
        route: &[u32],
        at_station: bool,
        slot: u16,
        traveler_id: u32,
    ) -> Option<AccessPath> {
        let from = self.platform_position(at_station, slot)?;
        let to = self.route_endpoint_position(route, true, traveler_id, 0)?;
        self.platform_exit_path_from_position(from, to, at_station)
    }

    pub(super) fn platform_exit_path_from_position(
        &self,
        from: WorldPoint,
        to: WorldPoint,
        at_station: bool,
    ) -> Option<AccessPath> {
        let (stair_bottom, stair_top, _) = self.platform_stairs(at_station)?;
        let queue_aisle =
            self.platform_point(at_station, 4.05, self.platform_local_z(at_station, from)?)?;
        Some(AccessPath {
            points: vec![from, queue_aisle, stair_top, stair_bottom, to],
            surface_y: vec![8.0, 8.0, 8.0, 0.27, -1.0],
        })
    }
}
