use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Node {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub kind: NodeKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Road {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub kind: RoadKind,
    pub stage: Stage,
    pub progress: f32,
    pub connected: bool,
    pub length: f32,
    pub lanes: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadSplitOutcome {
    pub split_node_id: u32,
    pub retained_road_id: u32,
    pub new_road_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoadSplitError {
    MissingRoad,
    IneligibleRoad,
    InvalidFraction,
    SegmentTooShort,
    ActivePlan,
    ActiveProject,
    UnsafeFrontage,
    InvalidActorRoute,
    Capacity,
}

pub(crate) fn lane_key(traveler: &Traveler, from: u32, to: u32) -> (u32, u32, u8, u8) {
    let sidewalk = if traveler.kind == 0 {
        (traveler.id & 1) as u8
    } else {
        2
    };
    (from, to, traveler.kind, sidewalk)
}

pub(crate) use city_plan::cross_section::{pedestrian_road_offset, road_width};

impl Engine {
    pub(crate) fn station_node(&self) -> Option<u32> {
        self.state
            .network
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Station)
            .map(|n| n.id)
    }

    pub(crate) fn node(&self, id: u32) -> Option<&Node> {
        self.state.network.nodes.iter().find(|n| n.id == id)
    }

    pub(crate) fn add_node(&mut self, x: f32, z: f32, kind: NodeKind) -> u32 {
        let id = self.id();
        self.state.network.nodes.push(Node {
            id,
            x: x.clamp(MIN, MAX),
            z: z.clamp(MIN, MAX),
            kind,
        });
        self.mark_access_geometry_changed();
        id
    }

    pub(crate) fn add_road(
        &mut self,
        from: u32,
        to: u32,
        kind: RoadKind,
        stage: Stage,
        progress: f32,
    ) -> u32 {
        let id = self.id();
        let len = match (self.node(from), self.node(to)) {
            (Some(a), Some(b)) => distance(a, b),
            _ => 1.0,
        };
        self.state.network.roads.push(Road {
            id,
            from,
            to,
            kind,
            stage,
            progress,
            connected: false,
            length: len.max(1.0),
            lanes: if kind == RoadKind::Arterial { 2 } else { 1 },
        });
        self.mark_access_geometry_changed();
        id
    }

    pub(crate) fn node_connected(&self, id: u32) -> bool {
        self.state
            .network
            .roads
            .iter()
            .any(|r| r.stage == Stage::Open && r.connected && (r.from == id || r.to == id))
    }

    pub(crate) fn recompute_connectivity(&mut self) {
        let gateway = self
            .state
            .network
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Gateway)
            .map(|n| n.id);
        let Some(start) = gateway else { return };
        let mut seen = BTreeSet::new();
        let mut q = VecDeque::from([start]);
        seen.insert(start);
        // Preserve source-road neighbor order while avoiding a full road
        // scan for every visited node in a large connected district.
        let mut adjacency: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for road in self
            .state
            .network
            .roads
            .iter()
            .filter(|road| road.stage == Stage::Open)
        {
            adjacency.entry(road.from).or_default().push(road.to);
            if road.from != road.to {
                adjacency.entry(road.to).or_default().push(road.from);
            }
        }
        while let Some(n) = q.pop_front() {
            if let Some(links) = adjacency.get(&n) {
                for &next in links {
                    if seen.insert(next) {
                        q.push_back(next);
                    }
                }
            }
        }
        let mut connectivity_changed = false;
        for road in &mut self.state.network.roads {
            let connected =
                road.stage == Stage::Open && seen.contains(&road.from) && seen.contains(&road.to);
            connectivity_changed |= road.connected != connected;
            road.connected = connected;
        }
        if connectivity_changed {
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn node_has_open_road(&self, node_id: u32) -> bool {
        self.state.network.roads.iter().any(|road| {
            road.connected
                && road.stage == Stage::Open
                && road.kind != RoadKind::Rail
                && (road.from == node_id || road.to == node_id)
        })
    }

    pub(crate) fn route_lengths(&self, route: &[u32]) -> Vec<f32> {
        route
            .windows(2)
            .map(|edge| match (self.node(edge[0]), self.node(edge[1])) {
                (Some(a), Some(b)) => distance(a, b).max(1.0),
                _ => 1.0,
            })
            .collect()
    }

    pub(crate) fn route(&self, start: u32, goal: u32, rail: bool) -> Vec<u32> {
        self.route_avoiding(start, goal, rail, &BTreeSet::new())
    }

    pub(crate) fn route_avoiding(
        &self,
        start: u32,
        goal: u32,
        rail: bool,
        blocked: &BTreeSet<u32>,
    ) -> Vec<u32> {
        self.route_where(start, goal, blocked, |road| {
            (rail && road.kind == RoadKind::Rail) || (!rail && road.kind != RoadKind::Rail)
        })
    }

    pub(crate) fn route_where(
        &self,
        start: u32,
        goal: u32,
        blocked: &BTreeSet<u32>,
        eligible: impl Fn(&Road) -> bool,
    ) -> Vec<u32> {
        if start == goal {
            return vec![start];
        }
        // Keep source order, including parallel edges, so equal-cost choices
        // match the full-road scan. This index lives only for this immutable
        // query; road openings and splits cannot leave a stale route cache.
        let mut adjacency = Vec::with_capacity(self.state.network.roads.len() * 2);
        for (index, road) in self
            .state
            .network
            .roads
            .iter()
            .enumerate()
            .filter(|(_, road)| road.connected && road.stage == Stage::Open && eligible(road))
        {
            adjacency.push((road.from, index));
            if road.to != road.from {
                adjacency.push((road.to, index));
            }
        }
        adjacency.sort_unstable();
        let mut dist: BTreeMap<u32, f32> = BTreeMap::new();
        let mut prev: BTreeMap<u32, u32> = BTreeMap::new();
        let mut open: BTreeSet<(u32, u32)> = BTreeSet::new();
        dist.insert(start, 0.0);
        open.insert((0, start));
        while let Some((cost_bits, current)) = open.iter().next().copied() {
            open.remove(&(cost_bits, current));
            if current == goal {
                break;
            }
            let base = *dist.get(&current).unwrap_or(&f32::INFINITY);
            let first = adjacency.partition_point(|(node, _)| *node < current);
            for &(_, index) in adjacency[first..]
                .iter()
                .take_while(|(node, _)| *node == current)
            {
                let r = &self.state.network.roads[index];
                let next = if r.from == current { r.to } else { r.from };
                if blocked.contains(&next) && next != goal {
                    continue;
                }
                let candidate = base + r.length;
                if candidate < *dist.get(&next).unwrap_or(&f32::INFINITY) {
                    dist.insert(next, candidate);
                    prev.insert(next, current);
                    open.insert((candidate.to_bits(), next));
                }
            }
        }
        if !prev.contains_key(&goal) {
            return vec![];
        }
        let mut path = vec![goal];
        let mut cur = goal;
        while cur != start && path.len() < MAX_ROUTE_NODES {
            let Some(p) = prev.get(&cur).copied() else {
                return vec![];
            };
            cur = p;
            path.push(cur);
        }
        // A capped suffix does not start at the requested origin. Consumers
        // interpret any nonempty path as a usable trip, so fail closed instead
        // of spawning an actor partway along an oversized journey.
        if cur != start {
            return vec![];
        }
        path.reverse();
        path
    }
}
