use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[ts(export)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Bounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Terrain {
    pub river: Vec<Point>,
    pub river_half_width: f32,
    pub protected: Vec<Vec<Point>>,
    pub gateways: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub(crate) struct WorldPoint {
    pub(crate) x: f32,
    pub(crate) z: f32,
}

pub(crate) fn distance(a: &Node, b: &Node) -> f32 {
    world::geometry::distance_coords(a.x, a.z, b.x, b.z)
}

pub(crate) fn point_segment_distance(x: f32, z: f32, ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    world::geometry::point_segment_distance(x, z, ax, az, bx, bz)
}

#[cfg(feature = "legacy")]
pub(crate) fn segments_distance(a: &Node, b: &Node, c: &Node, d: &Node) -> f32 {
    world::geometry::segments_distance_coords(a.x, a.z, b.x, b.z, c.x, c.z, d.x, d.z)
}

pub(crate) fn segments_distance_coords(
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
    cx: f32,
    cz: f32,
    dx: f32,
    dz: f32,
) -> f32 {
    world::geometry::segments_distance_coords(ax, az, bx, bz, cx, cz, dx, dz)
}

pub(crate) fn in_protected_land(x: f32, z: f32) -> bool {
    (-350.0..=-260.0).contains(&x) && (-350.0..=-270.0).contains(&z)
}

pub(crate) fn corridor_crosses_protected(ax: f32, az: f32, bx: f32, bz: f32) -> bool {
    (0..=16).any(|i| {
        let t = i as f32 / 16.0;
        in_protected_land(ax + (bx - ax) * t, az + (bz - az) * t)
    })
}
