use crate::*;

/// Signed offset of the driving lane from the road centre line, measured along
/// (-dz, dx), the counter-clockwise normal of the heading in the x-z plane.
/// The renderer is right-handed with y up, where that normal is the driver's
/// RIGHT, so Tokyo's left-hand traffic needs the negative side.
pub(crate) const KERB_LANE: f32 = -1.65;

/// Bus-stop poles stand on the kerb side, along the same normal.
pub(crate) const BUS_STOP_KERB: f32 = -6.2;

// Movement-buffer kinds for road vehicle bodies (web/src/render/vehicles.ts).
// Kind 1 stays the private compact so older traces and tools keep working.
pub(crate) const VEHICLE_COMPACT: u8 = 1;

pub(crate) const VEHICLE_KEI: u8 = 4;

pub(crate) const VEHICLE_TAXI: u8 = 5;

pub(crate) const VEHICLE_VAN: u8 = 6;

pub(crate) const VEHICLE_TRUCK: u8 = 7;

pub(crate) const VEHICLE_MAIL: u8 = 8;

/// Bumper-to-bumper allowance between queued road vehicles.
pub(crate) const VEHICLE_QUEUE_MARGIN: f32 = 0.5;

/// The historical fixed queue spacing; short bodies never queue tighter.
pub(crate) const VEHICLE_MIN_GAP: f32 = 4.5;

pub(crate) fn vehicle_hash(id: u32) -> u32 {
    id.wrapping_mul(2_654_435_761) >> 16
}

/// Body length in metres; matches web/src/render/vehicle-manifest.json.
pub(crate) fn vehicle_length(body: u8) -> f32 {
    match body {
        VEHICLE_KEI | VEHICLE_MAIL => 3.4,
        VEHICLE_TAXI => 4.4,
        VEHICLE_VAN | VEHICLE_TRUCK => 4.7,
        _ => 4.0,
    }
}

pub(crate) fn vehicle_name(body: u8) -> &'static str {
    match body {
        VEHICLE_KEI => "kei",
        VEHICLE_TAXI => "taxi",
        VEHICLE_VAN => "van",
        VEHICLE_TRUCK => "truck",
        VEHICLE_MAIL => "mail",
        _ => "compact",
    }
}

/// Where an access path meets the road, on a 10 cm grid, as a queue key.
pub(crate) fn driveway_mouth(point: WorldPoint) -> (i32, i32) {
    (
        (point.x * 10.0).round() as i32,
        (point.z * 10.0).round() as i32,
    )
}

/// Park a car that reached the end of its route at the kerb end of its final
/// lane while its driveway mouth is busy. It stays a road traveler there, so
/// it is still drawn and still holds back the cars queued behind it.
pub(crate) fn hold_at_kerb(t: &mut Traveler) {
    if let Some(&last) = t.route_lengths.last() {
        t.edge_index = t.route_lengths.len() - 1;
        t.edge_progress = last.max(1.0);
    }
}

/// Centre-to-centre following distance for two queued road vehicles.
pub(crate) fn vehicle_gap(lead: u8, follower: u8) -> f32 {
    ((vehicle_length(lead) + vehicle_length(follower)) * 0.5 + VEHICLE_QUEUE_MARGIN)
        .max(VEHICLE_MIN_GAP)
}

impl Engine {
    /// The body a road trip uses, derived from the trip so saves need no new field.
    /// Deliveries run vans, trucks and mail vans; residents drive kei cars and
    /// compacts, and station trips are more often taxis.
    /// `station` is `self.station_node()`, looked up once per pass by the caller.
    pub(crate) fn vehicle_body(t: &Traveler, station: Option<u32>) -> u8 {
        if t.kind != 1 {
            return t.kind;
        }
        let h = vehicle_hash(t.id);
        if t.activity.is_delivery() {
            return match h % 6 {
                0 | 1 => VEHICLE_TRUCK,
                2 => VEHICLE_MAIL,
                _ => VEHICLE_VAN,
            };
        }
        let station_trip =
            station.is_some_and(|s| t.destination == s || t.route.first() == Some(&s));
        if h % 10 == 0 || (station_trip && h % 3 == 0) {
            VEHICLE_TAXI
        } else if h % 10 < 5 {
            VEHICLE_KEI
        } else {
            VEHICLE_COMPACT
        }
    }

    pub(crate) fn signalized_state(state: &WorldState, node_id: u32) -> bool {
        state
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .is_some_and(|n| {
                matches!(
                    n.kind,
                    NodeKind::Junction | NodeKind::Market | NodeKind::Station
                )
            })
    }

    pub(crate) fn junction_red_state(
        state: &WorldState,
        traveler: &Traveler,
        node_id: u32,
    ) -> bool {
        if !Self::signalized_state(state, node_id) {
            return false;
        }
        if traveler.edge_index + 2 >= traveler.route.len() {
            return false;
        }
        let from = traveler.route[traveler.edge_index];
        let next = traveler.route[traveler.edge_index + 2];
        let (Some(a), Some(b)) = (
            state.nodes.iter().find(|n| n.id == from),
            state.nodes.iter().find(|n| n.id == node_id),
        ) else {
            return false;
        };
        let (Some(c), Some(d)) = (
            state.nodes.iter().find(|n| n.id == node_id),
            state.nodes.iter().find(|n| n.id == next),
        ) else {
            return false;
        };
        if traveler.kind == 0 {
            let phase = (state.tick + node_id as u64 * 19) % 240;
            return phase >= 80;
        }
        let dx = d.x - c.x;
        let dz = d.z - c.z;
        let horizontal = dx.abs() >= dz.abs();
        let approach = (state.tick + node_id as u64 * 37 + from as u64 * 11) % 240;
        let aligned = if horizontal {
            approach < 120
        } else {
            approach >= 120
        };
        let _incoming_heading = (b.z - a.z).atan2(b.x - a.x);
        !aligned
    }

    pub(crate) fn road_lateral_offset(
        &self,
        traveler: &Traveler,
        from: u32,
        to: u32,
        dx: f32,
        dz: f32,
        len: f32,
    ) -> (f32, f32) {
        let road = self.state.roads.iter().find(|r| {
            r.stage == Stage::Open
                && r.kind != RoadKind::Rail
                && ((r.from == from && r.to == to) || (r.from == to && r.to == from))
        });
        let walking_offset = road
            .map(|r| pedestrian_road_offset(Some(r.kind)))
            .unwrap_or_else(|| pedestrian_road_offset(None));
        let lateral = if traveler.kind == 1 {
            KERB_LANE
        } else if traveler.id % 2 == 0 {
            walking_offset
        } else {
            -walking_offset
        };
        let nx = -dz / len;
        let nz = dx / len;
        (nx * lateral, nz * lateral)
    }
}
