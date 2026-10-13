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
    /// 0 site, 1 supports, 2 deck/access, 3 canopy, 4 complete.
    pub construction_phase: u8,
    /// Local ground centerline: street, side approach, apron, stair foot.
    pub ground_path: [WorldPoint; 5],
    pub ground_width: f32,
}

impl Engine {
    pub(super) fn station_construction_phase(&self) -> u8 {
        let progress = self.state.projects.iter().find(|p| p.kind == "station")
            .map_or(1.0, |p| p.progress);
        // The eight-work-step station reaches .75 and .875 before completion;
        // late surfacing must expose the canopy before service starts.
        if progress < 0.48 { 0 } else if progress < 0.73 { 1 }
        else if progress < 0.85 { 2 } else if progress < 1.0 { 3 } else { 4 }
    }

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
                let bottom = if node_id == gateway { 67.0 } else { -20.0 };
                let apron = bottom + if node_id == gateway { 1.5 } else { -1.5 };
                Some(RailPlatformView {
                    node_id,
                    x: node.x,
                    z: node.z,
                    ux,
                    uz,
                    surface_y: 8.0,
                    stair_x: 4.05,
                    // The boundary gateway enters from the inward end of its platform.
                    stair_bottom_z: if node_id == gateway { 67.0 } else { -20.0 },
                    stair_top_z: if node_id == gateway { 48.0 } else { -1.0 },
                    ground_y: 0.27,
                    stair_steps: 44,
                    construction_phase: self.station_construction_phase(),
                    ground_path: [WorldPoint { x: 0.0, z: 0.0 },
                        WorldPoint { x: 7.5, z: -1.5 }, WorldPoint { x: 7.5, z: apron },
                        WorldPoint { x: 4.05, z: apron }, WorldPoint { x: 4.05, z: bottom }],
                    ground_width: 1.6,
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

    fn platform_stairs(&self, at_station: bool) -> Option<(WorldPoint, WorldPoint)> {
        let (station, gateway, _, _) = self.rail_axis()?;
        let node_id = if at_station { station } else { gateway };
        let layout = self.rail_platform_views().into_iter().find(|p| p.node_id == node_id)?;
        let point = |z: f32| WorldPoint {
            x: layout.x + layout.uz * layout.stair_x + layout.ux * z,
            z: layout.z - layout.ux * layout.stair_x + layout.uz * z,
        };
        Some((point(layout.stair_bottom_z), point(layout.stair_top_z)))
    }

    /// Ground approach stays outside stair stringers and platform support columns.
    /// Cross onto the stair axis only beyond the lower tread, never beneath it.
    fn platform_ground_approach(&self, at_station: bool) -> Option<[WorldPoint; 3]> {
        let (station, gateway, _, _) = self.rail_axis()?;
        let id = if at_station { station } else { gateway };
        let layout = self.rail_platform_views().into_iter().find(|p| p.node_id == id)?;
        Some([1, 2, 3].map(|i| {
            let p = layout.ground_path[i];
            WorldPoint { x: layout.x + layout.uz * p.x + layout.ux * p.z,
                z: layout.z - layout.ux * p.x + layout.uz * p.z }
        }))
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
        let (stair_bottom, stair_top) = self.platform_stairs(at_station)?;
        let approach = self.platform_ground_approach(at_station)?;
        let aisle =
            self.platform_point(at_station, 4.05, self.platform_local_z(at_station, to)?)?;
        Some(AccessPath {
            points: vec![from, approach[0], approach[1], approach[2], stair_bottom, stair_top, aisle, to],
            surface_y: vec![-1.0, 0.27, 0.27, 0.27, 0.27, 8.0, 8.0, 8.0],
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
        let (stair_bottom, stair_top) = self.platform_stairs(at_station)?;
        let approach = self.platform_ground_approach(at_station)?;
        let queue_aisle =
            self.platform_point(at_station, 4.05, self.platform_local_z(at_station, from)?)?;
        Some(AccessPath {
            points: vec![from, queue_aisle, stair_top, stair_bottom, approach[2], approach[1], approach[0], to],
            surface_y: vec![8.0, 8.0, 8.0, 0.27, 0.27, 0.27, 0.27, -1.0],
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_stairs_and_walkers_enter_from_inside_each_world_edge() {
        for (gx, gz, sx, sz) in [
            (392.0, 0.0, 80.0, 0.0), (-392.0, 0.0, -80.0, 0.0),
            (0.0, 392.0, 0.0, 80.0), (0.0, -392.0, 0.0, -80.0),
        ] {
            let mut engine = Engine::new_legacy(48);
            let rail = engine.state.roads.iter_mut().find(|r| r.kind == "rail").unwrap();
            rail.stage = "open".into();
            rail.progress = 1.0;
            engine.state.station_ready = true;
            engine.state.station_online = true;
            engine.recompute_connectivity();
            let (station, gateway, _, _) = engine.rail_axis().unwrap();
            for node in &mut engine.state.nodes {
                if node.id == station { node.x = sx; node.z = sz; }
                if node.id == gateway { node.x = gx; node.z = gz; }
            }
            let layout = engine.rail_platform_views().into_iter()
                .find(|p| p.node_id == gateway).unwrap();
            // Full tread footprint, not merely its centerline, must remain in bounds.
            for x in [layout.stair_x - 1.2, layout.stair_x + 1.2] {
                for z in [layout.stair_bottom_z + 0.01, layout.stair_top_z - 0.01] {
                    let p = engine.platform_point(false, x, z).unwrap();
                    assert!(p.x.abs() <= 400.0 && p.z.abs() <= 400.0, "{p:?}");
                }
            }
            for at_station in [true, false] {
                let id = if at_station { station } else { gateway };
                let p = engine.rail_platform_views().into_iter().find(|p| p.node_id == id).unwrap();
                let ground: Vec<_> = p.ground_path.iter().map(|v|
                    engine.platform_point(at_station, v.x, v.z).unwrap()).collect();
                let queue = engine.platform_position(at_station, 0).unwrap();
                let entry = engine.platform_entry_path_from_position(ground[0], queue, at_station, 1).unwrap();
                let exit = engine.platform_exit_path_from_position(queue, ground[0], at_station).unwrap();
                assert_eq!(&entry.points[..5], ground.as_slice());
                assert_eq!(exit.points[3..].iter().rev().copied().collect::<Vec<_>>(), ground);
                assert!(p.ground_path[1].x - p.ground_width / 2.0 > 6.4);
            }
            assert_eq!(layout.stair_top_z, 48.0); // joins the inward slab edge
            assert_eq!(layout.stair_bottom_z - layout.stair_top_z, 19.0);
            let bottom = engine.platform_point(false, layout.stair_x, layout.stair_bottom_z).unwrap();
            let top = engine.platform_point(false, layout.stair_x, layout.stair_top_z).unwrap();
            for slot in [0, 95] {
                let queue = engine.platform_position(false, slot).unwrap();
                let entry = engine.platform_entry_path_from_position(bottom, queue, false, 1).unwrap();
                let exit = engine.platform_exit_path_from_position(queue, bottom, false).unwrap();
                assert_eq!((entry.points[4].x, entry.points[4].z), (bottom.x, bottom.z));
                assert_eq!((entry.points[5].x, entry.points[5].z), (top.x, top.z));
                assert_eq!((exit.points[2].x, exit.points[2].z), (top.x, top.z));
                assert_eq!((exit.points[3].x, exit.points[3].z), (bottom.x, bottom.z));
                assert_eq!(&entry.surface_y[4..6], &[layout.ground_y, layout.surface_y]);
            }
        }
    }
}
