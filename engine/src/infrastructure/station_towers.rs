use crate::*;
use std::collections::BTreeSet;

pub(crate) const STATION_TOWER_PARCEL_WIDTH: f32 = 24.0;

pub(crate) const STATION_TOWER_PARCEL_DEPTH: f32 = 20.0;

pub(crate) const MIN_STATION_TOWER_FRONTAGE_RESERVATIONS: usize = 2;

pub(crate) const STATION_TOWER_ROAD_OFFSET: f32 = 20.0;

pub(crate) fn stateless_tower_site_hash(seed: u32, x: f32, z: f32) -> u64 {
    let qx = ((x + 400.0) * 10.0).round() as i64 as u64;
    let qz = ((z + 400.0) * 10.0).round() as i64 as u64;
    let mut value = (seed as u64)
        ^ qx.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ qz.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

impl Engine {
    pub(crate) fn is_station_tower_parcel(&self, parcel: &Parcel) -> bool {
        if !self.state.transit.station_online
            || self.neighborhood_profile(parcel.x, parcel.z) != NeighborhoodProfile::StationCore
            || !self
                .state
                .land
                .parcel_frontages
                .iter()
                .any(|frontage| frontage.parcel_id == parcel.id)
        {
            return false;
        }
        self.is_station_tower_lot(parcel)
    }

    pub(crate) fn is_station_tower_lot(&self, parcel: &Parcel) -> bool {
        if parcel.width < STATION_TOWER_PARCEL_WIDTH || parcel.depth < STATION_TOWER_PARCEL_DEPTH {
            return false;
        }
        self.state
            .network
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Station)
            .any(|station| (station.x - parcel.x).hypot(station.z - parcel.z) <= 105.0)
    }

    pub(crate) fn station_tower_frontage_reservations(&self) -> usize {
        let frontage_ids: BTreeSet<u32> = self
            .state
            .land
            .parcel_frontages
            .iter()
            .map(|frontage| frontage.parcel_id)
            .collect();
        self.state
            .land
            .parcels
            .iter()
            .filter(|parcel| {
                parcel.width >= STATION_TOWER_PARCEL_WIDTH
                    && parcel.depth >= STATION_TOWER_PARCEL_DEPTH
                    && self.is_station_tower_lot(parcel)
            })
            .filter(|parcel| frontage_ids.contains(&parcel.id))
            // Only the first two reservations affect admission. Capping this
            // derived count also avoids scanning the rest of a mature parcel set.
            .take(MIN_STATION_TOWER_FRONTAGE_RESERVATIONS)
            .count()
    }

    pub(crate) fn available_station_tower_envelopes(&self) -> Vec<(f32, f32)> {
        let remaining = MIN_STATION_TOWER_FRONTAGE_RESERVATIONS
            .saturating_sub(self.station_tower_frontage_reservations());
        if remaining == 0 {
            return Vec::new();
        }

        let mut candidates: Vec<(f32, u64, f32, f32)> = Vec::new();
        for road in self.state.network.roads.iter().filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        }) {
            let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                continue;
            };
            self.push_available_station_tower_candidate(road.id, a, b, &mut candidates);
        }
        // A planned street can reserve its exact future parcel envelope before
        // another frontage slot is filled beside the construction corridor.
        for project in self
            .state
            .construction
            .projects
            .iter()
            .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        {
            let (Some(from), Some(to)) = (project.from, project.to) else {
                continue;
            };
            let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                continue;
            };
            self.push_available_station_tower_candidate(project.id, a, b, &mut candidates);
        }

        candidates.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.3.total_cmp(&right.3))
        });
        let mut reserved = Vec::with_capacity(remaining);
        for (_, _, x, z) in candidates {
            if reserved.iter().any(|(other_x, other_z): &(f32, f32)| {
                (x - *other_x).abs() < STATION_TOWER_PARCEL_WIDTH + 2.0
                    && (z - *other_z).abs() < STATION_TOWER_PARCEL_DEPTH + 2.0
            }) {
                continue;
            }
            reserved.push((x, z));
            if reserved.len() == remaining {
                break;
            }
        }
        reserved
    }

    pub(crate) fn push_available_station_tower_candidate(
        &self,
        road_id: u32,
        a: &Node,
        b: &Node,
        candidates: &mut Vec<(f32, u64, f32, f32)>,
    ) {
        let sites = self.frontage_candidate_sites(a, b);
        let Some(index) = self.station_tower_candidate_index(road_id, a, b, &sites) else {
            return;
        };
        let (_, _, _, along, side) = sites[index];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz).max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let x = a.x + dx * along + nx * side as f32 * STATION_TOWER_ROAD_OFFSET;
        let z = a.z + dz * along + nz * side as f32 * STATION_TOWER_ROAD_OFFSET;
        // Only hold a site that could accept a real large parcel right now;
        // this keeps the reservation finite and lets every unrelated lot grow.
        if !self.parcel_geometry_clear(x, z, STATION_TOWER_PARCEL_WIDTH, STATION_TOWER_PARCEL_DEPTH)
        {
            return;
        }
        let station_distance = self
            .state
            .network
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Station)
            .map(|station| (station.x - x).hypot(station.z - z))
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY);
        let tie_break = stateless_tower_site_hash(self.state.seed, x, z);
        candidates.push((station_distance, tie_break, x, z));
    }

    pub(crate) fn overlaps_station_tower_envelopes(
        envelopes: &[(f32, f32)],
        x: f32,
        z: f32,
        width: f32,
        depth: f32,
    ) -> bool {
        envelopes.iter().any(|(tower_x, tower_z)| {
            (x - tower_x).abs() < (width + STATION_TOWER_PARCEL_WIDTH) * 0.5 + 2.0
                && (z - tower_z).abs() < (depth + STATION_TOWER_PARCEL_DEPTH) * 0.5 + 2.0
        })
    }

    /// Selects a midpoint tower-sized site on at most one side of an eligible
    /// street. Its same-side small starter lot is withheld, so the larger
    /// footprint can use the same street frontage without overlapping it.
    pub(crate) fn station_tower_candidate_index(
        &self,
        _road_id: u32,
        a: &Node,
        b: &Node,
        candidates: &[(f32, f32, u32, f32, i8)],
    ) -> Option<usize> {
        let station = self
            .state
            .network
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Station)?;
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz);
        if length < 42.0 {
            return None;
        }
        let ux = dx / length;
        let uz = dz / length;
        let nx = -uz;
        let nz = ux;
        let station_along =
            (((station.x - a.x) * dx + (station.z - a.z) * dz) / (length * length)).clamp(0.0, 1.0);
        let projected_x = a.x + dx * station_along;
        let projected_z = a.z + dz * station_along;
        let side_offset = (station.x - projected_x) * nx + (station.z - projected_z) * nz;
        let station_side = if side_offset.abs() < 0.01 {
            if stateless_tower_site_hash(self.state.seed, projected_x, projected_z) & 1 == 0 {
                1
            } else {
                -1
            }
        } else if side_offset > 0.0 {
            1
        } else {
            -1
        };

        candidates
            .iter()
            .enumerate()
            .filter(|(_, (_, _, _, along, side))| {
                (*along - 0.5).abs() < 0.001 && *side == station_side
            })
            .map(|(index, (_, _, _, along, side))| {
                let x = a.x + dx * *along + nx * *side as f32 * STATION_TOWER_ROAD_OFFSET;
                let z = a.z + dz * *along + nz * *side as f32 * STATION_TOWER_ROAD_OFFSET;
                (
                    index,
                    (x - station.x).hypot(z - station.z),
                    (*along - station_along).abs(),
                )
            })
            .filter(|(_, distance, _)| *distance <= 100.0)
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.2.total_cmp(&right.2))
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|(index, _, _)| index)
    }
}
