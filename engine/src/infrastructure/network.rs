use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Road {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub kind: String,
    pub stage: String,
    pub progress: f32,
    pub connected: bool,
    pub length: f32,
    pub lanes: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RoadSplitOutcome {
    pub(crate) split_node_id: u32,
    pub(crate) retained_road_id: u32,
    pub(crate) new_road_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoadSplitError {
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

/// Rendered carriageway width. Tokyo local streets are narrow shared surfaces;
/// only arterials carry raised sidewalks.
pub(crate) fn road_width(kind: &str) -> f32 {
    match kind {
        "arterial" => 16.0,
        "bridge" => 16.0,
        "local" => 6.5,
        "lane" => 4.0,
        _ => 9.0,
    }
}

/// Shared streets carry walkers inside the painted edge; arterials use the
/// raised sidewalk outside the carriageway. All access and movement consumers
/// must use this same offset so their route endpoints meet.
pub(crate) fn pedestrian_road_offset(kind: &str) -> f32 {
    let half_width = road_width(kind) * 0.5;
    if matches!(kind, "local" | "lane") {
        half_width - 0.72
    } else {
        half_width + 0.72
    }
}

impl Engine {
    pub(crate) fn station_node(&self) -> Option<u32> {
        self.state
            .nodes
            .iter()
            .find(|n| n.kind == "station")
            .map(|n| n.id)
    }

    pub(crate) fn node(&self, id: u32) -> Option<&Node> {
        self.state.nodes.iter().find(|n| n.id == id)
    }

    pub(crate) fn add_node(&mut self, x: f32, z: f32, kind: &str) -> u32 {
        let id = self.id();
        self.state.nodes.push(Node {
            id,
            x: x.clamp(MIN, MAX),
            z: z.clamp(MIN, MAX),
            kind: kind.into(),
        });
        self.mark_access_geometry_changed();
        id
    }

    pub(crate) fn add_road(
        &mut self,
        from: u32,
        to: u32,
        kind: &str,
        stage: &str,
        progress: f32,
    ) -> u32 {
        let id = self.id();
        let len = match (self.node(from), self.node(to)) {
            (Some(a), Some(b)) => distance(a, b),
            _ => 1.0,
        };
        self.state.roads.push(Road {
            id,
            from,
            to,
            kind: kind.into(),
            stage: stage.into(),
            progress,
            connected: false,
            length: len.max(1.0),
            lanes: if kind == "arterial" { 2 } else { 1 },
        });
        self.mark_access_geometry_changed();
        id
    }

    pub(crate) fn node_connected(&self, id: u32) -> bool {
        self.state
            .roads
            .iter()
            .any(|r| r.stage == "open" && r.connected && (r.from == id || r.to == id))
    }

    pub(crate) fn recompute_connectivity(&mut self) {
        let gateway = self
            .state
            .nodes
            .iter()
            .find(|n| n.kind == "gateway")
            .map(|n| n.id);
        let Some(start) = gateway else { return };
        let mut seen = BTreeSet::new();
        let mut q = VecDeque::from([start]);
        seen.insert(start);
        // Preserve source-road neighbor order while avoiding a full road
        // scan for every visited node in a large connected district.
        let mut adjacency: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for road in self.state.roads.iter().filter(|road| road.stage == "open") {
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
        for road in &mut self.state.roads {
            let connected =
                road.stage == "open" && seen.contains(&road.from) && seen.contains(&road.to);
            connectivity_changed |= road.connected != connected;
            road.connected = connected;
        }
        if connectivity_changed {
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn node_has_open_road(&self, node_id: u32) -> bool {
        self.state.roads.iter().any(|road| {
            road.connected
                && road.stage == "open"
                && road.kind != "rail"
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
            (rail && road.kind == "rail") || (!rail && road.kind != "rail")
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
        let mut adjacency = Vec::with_capacity(self.state.roads.len() * 2);
        for (index, road) in self
            .state
            .roads
            .iter()
            .enumerate()
            .filter(|(_, road)| road.connected && road.stage == "open" && eligible(road))
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
                let r = &self.state.roads[index];
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
