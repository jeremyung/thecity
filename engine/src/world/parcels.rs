use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Parcel {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    /// Local footprint orientation for coordinated block lots. Legacy parcels
    /// retain their historical axis-aligned admission envelope when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f32>,
    pub access_node: u32,
    pub suitability: f32,
    pub protected: bool,
    pub reservation: Option<u32>,
    /// Open space a protected, unbuilt parcel holds: "park", "shrine" or
    /// "plaza". Absent for ordinary lots and for parks from older saves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_space: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ParcelFrontage {
    pub(crate) parcel_id: u32,
    pub(crate) road_id: u32,
    pub(crate) along: f32,
    pub(crate) side: i8,
}

pub(crate) fn parcel_envelope(parcel: &Parcel) -> world::geometry::OrientedRect {
    world::geometry::OrientedRect {
        x: parcel.x,
        z: parcel.z,
        width: parcel.width,
        depth: parcel.depth,
        rotation: parcel.rotation.unwrap_or(0.0),
    }
}

pub(crate) fn parcel_envelopes_clear_of_road(
    context: &CollisionContext,
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
) -> bool {
    context
        .parcel_footprints
        .iter()
        .all(|footprint| world::geometry::rect_segment_distance(footprint, ax, az, bx, bz) >= 5.0)
}

pub(crate) fn frontage_access_node(a: &Node, b: &Node, along: f32) -> u32 {
    if along < 0.5 {
        a.id
    } else if along > 0.5 {
        b.id
    } else {
        a.id.min(b.id)
    }
}

impl Engine {
    pub(crate) fn add_parcel(
        &mut self,
        x: f32,
        z: f32,
        w: f32,
        d: f32,
        node: u32,
        s: f32,
        protected: bool,
    ) -> u32 {
        // Keep capacity available for the coordinated lots promised by an
        // active block plan, even when another frontage scan adds a parcel.
        let reserved_lots = self
            .coordinated_plan_lot_count()
            .unwrap_or(0)
            .saturating_add(
                self.state
                    .legacy
                    .pending_interior_access
                    .as_ref()
                    .map_or(0, |plan| plan.selected_lots.len()),
            );
        if self
            .state
            .land
            .parcels
            .len()
            .saturating_add(1)
            .saturating_add(reserved_lots)
            > MAX_PARCELS
        {
            return 0;
        }
        let id = self.id();
        self.state.land.parcels.push(Parcel {
            id,
            x,
            z,
            width: w,
            depth: d,
            rotation: None,
            access_node: node,
            suitability: s,
            protected,
            reservation: None,
            open_space: None,
        });
        self.mark_access_geometry_changed();
        id
    }

    pub(crate) fn remember_nearest_frontage(&mut self, parcel_id: u32, access_node: u32) {
        if parcel_id == 0
            || self
                .state
                .land
                .parcel_frontages
                .iter()
                .any(|f| f.parcel_id == parcel_id)
        {
            return;
        }
        let Some(parcel) = self.state.land.parcels.iter().find(|p| p.id == parcel_id) else {
            return;
        };
        let mut best: Option<(u32, f32, i8, f32)> = None;
        for road in self.state.network.roads.iter().filter(|road| {
            road.stage == Stage::Open
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
                && (road.from == access_node || road.to == access_node)
        }) {
            let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
            else {
                continue;
            };
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let length_sq = dx * dx + dz * dz;
            if length_sq <= f32::EPSILON {
                continue;
            }
            let along =
                (((parcel.x - a.x) * dx + (parcel.z - a.z) * dz) / length_sq).clamp(0.0, 1.0);
            let px = a.x + dx * along;
            let pz = a.z + dz * along;
            let distance = (parcel.x - px).hypot(parcel.z - pz);
            if distance > 34.0
                || best.is_some_and(|(_, _, _, best_distance)| distance >= best_distance)
            {
                continue;
            }
            let nx = -dz / length_sq.sqrt();
            let nz = dx / length_sq.sqrt();
            let side = if (parcel.x - px) * nx + (parcel.z - pz) * nz >= 0.0 {
                1
            } else {
                -1
            };
            best = Some((road.id, along, side, distance));
        }
        if let Some((road_id, along, side, _)) = best {
            self.state.land.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn frontage_rotation(&self, parcel_id: u32) -> f32 {
        if cfg!(feature = "parent-passage-candidate") {
            if let Some(rotation) = self
                .masterplan_lot_for_parcel(parcel_id)
                .and_then(|lot| lot.passage_rotation())
            {
                return rotation;
            }
        }
        let Some(frontage) = self
            .state
            .land
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel_id)
        else {
            return 0.0;
        };
        let Some(road) = self
            .state
            .network
            .roads
            .iter()
            .find(|road| road.id == frontage.road_id)
        else {
            return 0.0;
        };
        let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
            return 0.0;
        };
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        // Local +Z faces inward from this parcel's frontage side toward the road.
        (frontage.side as f32 * dz).atan2(-frontage.side as f32 * dx)
    }
}
