use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn bus_uses_collectors_while_walkers_keep_the_lane_shortcut() {
    let mut engine = engine_with_graph(5, vec![
        road(0, 0, 1, "lane", "open", true, 1.0),
        road(1, 1, 4, "lane", "open", true, 1.0),
        road(2, 0, 2, "local", "open", true, 3.0),
        road(3, 2, 3, "arterial", "open", true, 3.0),
        road(4, 3, 4, "bridge", "open", true, 3.0),
        road(5, 0, 4, "expressway", "open", true, 4.0),
    ]);
    assert_eq!(engine.route(0, 4, false), vec![0, 1, 4]);
    assert_eq!(engine.bus_route_avoiding(0, 4, &BTreeSet::new()), vec![0, 2, 3, 4]);
    assert_eq!(engine.bus_route_avoiding(4, 0, &BTreeSet::new()), vec![4, 3, 2, 0]);
    assert!(engine.bus_route_avoiding(0, 4, &BTreeSet::from([2])).is_empty());
    engine.state.roads[3].stage = "construction".into();
    assert!(engine.bus_route_avoiding(0, 4, &BTreeSet::new()).is_empty());
    assert_eq!(engine.route(0, 4, false), vec![0, 1, 4]);
}

#[test]
fn route_limit_accepts_complete_boundary_and_rejects_suffix() {
    let count = MAX_ROUTE_NODES as u32 + 1;
    let roads = (0..count - 1)
        .map(|id| road(id, id, id + 1, "local", "open", true, 1.0))
        .collect();
    let engine = engine_with_graph(count, roads);
    let valid = engine.route(0, count - 2, false);
    assert_eq!(valid, (0..count - 1).collect::<Vec<_>>());
    let old = route_avoiding_scan(&engine, 0, count - 1, false, &BTreeSet::new());
    assert_eq!(old.len(), MAX_ROUTE_NODES);
    assert_eq!(old.first(), Some(&1)); // concrete witness for the old bug
    assert!(engine.route(0, count - 1, false).is_empty());
    assert!(engine.route(count - 1, 0, false).is_empty());
}

#[test]
fn delivery_does_not_spawn_midway_along_an_oversized_route() {
    let count = MAX_ROUTE_NODES as u32 + 1;
    let roads = (0..count - 1)
        .map(|id| road(id, id, id + 1, "local", "open", true, 1.0))
        .collect();
    let mut engine = engine_with_graph(count, roads);
    engine.state.nodes[0].kind = "gateway".into();
    engine.state.buildings.truncate(1);
    let business = &mut engine.state.buildings[0];
    business.kind = "shop".into();
    business.stage = "open".into();
    business.business_active = true;
    let parcel_id = business.parcel_id;
    engine
        .state
        .parcels
        .iter_mut()
        .find(|p| p.id == parcel_id)
        .unwrap()
        .access_node = count - 1;
    engine.state.travelers.clear();
    engine.state.next_delivery_tick = engine.state.tick;
    let cursor = engine.state.delivery_cursor;
    engine.schedule_deliveries();
    assert!(engine.state.travelers.is_empty());
    assert_eq!(engine.state.delivery_cursor, cursor);
    // Make the journey fit: dispatch must still work from the real gateway.
    engine
        .state
        .parcels
        .iter_mut()
        .find(|p| p.id == parcel_id)
        .unwrap()
        .access_node = count - 2;
    engine.state.next_delivery_tick = engine.state.tick;
    engine.schedule_deliveries();
    assert_eq!(engine.state.travelers.len(), 1);
    assert_eq!(engine.state.travelers[0].route.first(), Some(&0));
    assert_eq!(engine.state.travelers[0].route.last(), Some(&(count - 2)));
}

// Frozen copy of Engine::route_avoiding before adjacency indexing. Keep this
// source-order scan as the behavioral oracle while the production search is
// optimized.
fn route_avoiding_scan(
    engine: &Engine,
    start: u32,
    goal: u32,
    rail: bool,
    blocked: &BTreeSet<u32>,
) -> Vec<u32> {
    if start == goal {
        return vec![start];
    }
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
        for r in engine.state.roads.iter().filter(|r| {
            r.connected
                && r.stage == "open"
                && ((rail && r.kind == "rail") || (!rail && r.kind != "rail"))
                && (r.from == current || r.to == current)
        }) {
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
    path.reverse();
    path
}

fn engine_with_graph(node_count: u32, roads: Vec<Road>) -> Engine {
    let mut engine = Engine::new_legacy(17);
    engine.state.nodes = (0..node_count)
        .map(|id| Node {
            id,
            x: id as f32 * 10.0,
            z: 0.0,
            kind: "junction".into(),
        })
        .collect();
    engine.state.roads = roads;
    engine
}

fn road(
    id: u32,
    from: u32,
    to: u32,
    kind: &str,
    stage: &str,
    connected: bool,
    length: f32,
) -> Road {
    Road {
        id,
        from,
        to,
        kind: kind.into(),
        stage: stage.into(),
        progress: if stage == "open" { 1.0 } else { 0.0 },
        connected,
        length,
        lanes: 1,
    }
}

fn assert_matches_oracle(engine: &Engine, start: u32, goal: u32, rail: bool, blocked: &[u32]) {
    let blocked = blocked.iter().copied().collect::<BTreeSet<_>>();
    let mut expected = route_avoiding_scan(engine, start, goal, rail, &blocked);
    // The sole intentional difference from the old search: oversized routes
    // must not return a suffix missing the requested origin.
    if expected.first().is_some_and(|node| *node != start) {
        expected.clear();
    }
    assert_eq!(
        engine.route_avoiding(start, goal, rail, &blocked),
        expected,
        "start={start}, goal={goal}, rail={rail}, blocked={blocked:?}"
    );
}

#[test]
fn exact_routes_match_scan_for_equal_cost_diamonds_and_source_order() {
    let diamond = vec![
        road(10, 0, 1, "local", "open", true, 1.0),
        road(11, 1, 3, "local", "open", true, 1.0),
        road(12, 0, 2, "local", "open", true, 1.0),
        road(13, 2, 3, "local", "open", true, 1.0),
    ];
    for roads in [diamond.clone(), diamond.into_iter().rev().collect()] {
        let engine = engine_with_graph(4, roads);
        assert_matches_oracle(&engine, 0, 3, false, &[]);
        assert_matches_oracle(&engine, 0, 3, false, &[1]);
        assert_matches_oracle(&engine, 3, 0, false, &[]);
    }
}

#[test]
fn exact_routes_match_scan_for_filters_parallel_edges_and_blocked_nodes() {
    let roads = vec![
        road(0, 0, 1, "local", "open", true, 2.0),
        road(1, 1, 2, "rail", "open", true, 2.0),
        road(2, 0, 2, "arterial", "open", true, 9.0),
        road(3, 0, 2, "local", "open", true, 9.0), // parallel edge
        road(4, 2, 3, "local", "survey", true, 1.0),
        road(5, 2, 3, "local", "open", false, 1.0),
        road(6, 2, 4, "local", "open", true, 1.0),
        road(7, 4, 4, "local", "open", true, 0.0), // self loop
        road(8, 4, 5, "local", "open", true, 1.0),
    ];
    let engine = engine_with_graph(7, roads);
    for (rail, start, goal, blocked) in [
        (false, 0, 3, vec![]),
        (true, 0, 2, vec![]),
        (false, 0, 5, vec![]),
        (false, 0, 5, vec![0]), // blocked start still permits departure
        (false, 0, 5, vec![5]), // goal is allowed despite being blocked
        (false, 0, 5, vec![2]), // blocked intermediate
        (false, 6, 5, vec![]),  // disconnected
        (false, 4, 4, vec![4]), // start equals goal
    ] {
        assert_matches_oracle(&engine, start, goal, rail, &blocked);
    }
}

#[test]
fn exact_routes_match_scan_for_deterministic_generated_graphs() {
    let mut state = 0x7a31_9d27u32;
    for case in 0..80u32 {
        let node_count = 5 + case % 8;
        let mut roads = Vec::new();
        for id in 0..(node_count * 3) {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let from = state % node_count;
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let to = state % node_count;
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let kind = match state % 4 {
                0 => "rail",
                1 => "arterial",
                _ => "local",
            };
            let (stage, connected) = match (state >> 4) % 5 {
                0 => ("survey", true),
                1 => ("open", false),
                _ => ("open", true),
            };
            let length = match (state >> 8) % 5 {
                0 => 1.0,
                1 => 2.0,
                2 => 3.0,
                3 => 7.0,
                _ => 11.0,
            };
            roads.push(road(id, from, to, kind, stage, connected, length));
        }
        if case % 2 == 1 {
            roads.reverse();
        }
        let engine = engine_with_graph(node_count, roads);
        for rail in [false, true] {
            for start in 0..node_count {
                let goal = (start + 3 + case % (node_count - 1)) % node_count;
                let blocked = if case % 3 == 0 {
                    vec![(start + 1) % node_count]
                } else {
                    vec![]
                };
                assert_matches_oracle(&engine, start, goal, rail, &blocked);
            }
        }
    }
}

#[test]
fn large_sparse_id_graph_matches_after_live_road_mutations() {
    const NODE_COUNT: u32 = 200;
    let node_id = |index: u32| 10_003 + index * 17;
    let mut engine = Engine::new_legacy(91);
    engine.state.nodes = (0..NODE_COUNT)
        .map(|index| Node {
            id: node_id(index),
            x: index as f32,
            z: 0.0,
            kind: "junction".into(),
        })
        .collect();

    let mut roads = Vec::new();
    for index in 0..NODE_COUNT - 1 {
        // Road ids and graph node ids are both sparse and unrelated.
        roads.push(road(
            50_000 + index * 23,
            node_id(index),
            node_id(index + 1),
            "local",
            "open",
            true,
            0.35 + (index % 4) as f32 * 0.025,
        ));
    }
    // Fractional shortcuts are present but initially cost more than the
    // corresponding chain segments, retaining a path longer than the cap.
    for index in (0..NODE_COUNT - 2).step_by(11) {
        roads.push(road(
            90_001 + index * 31,
            node_id(index),
            node_id(index + 2),
            "local",
            "open",
            true,
            1.125,
        ));
    }
    roads.push(road(
        120_007,
        node_id(0),
        node_id(1),
        "rail",
        "open",
        true,
        0.75,
    ));
    roads.push(road(
        120_113,
        node_id(1),
        node_id(2),
        "rail",
        "open",
        true,
        0.625,
    ));
    engine.state.roads = roads;

    let long_path =
        engine.route_avoiding(node_id(0), node_id(NODE_COUNT - 1), false, &BTreeSet::new());
    assert_matches_oracle(&engine, node_id(0), node_id(NODE_COUNT - 1), false, &[]);
    assert!(long_path.is_empty());

    let rail_path = engine.route_avoiding(node_id(0), node_id(2), true, &BTreeSet::new());
    assert_matches_oracle(&engine, node_id(0), node_id(2), true, &[]);
    assert_eq!(rail_path, vec![node_id(0), node_id(1), node_id(2)]);

    // Exercise each adjacency input across repeated calls. A per-call index
    // must immediately reflect mutations to status, connectivity, endpoints,
    // cost, and source ordering.
    engine.state.roads[0].stage = "survey".into();
    assert_matches_oracle(&engine, node_id(0), node_id(NODE_COUNT - 1), false, &[]);
    engine.state.roads[0].stage = "open".into();
    engine.state.roads[0].connected = false;
    assert_matches_oracle(&engine, node_id(0), node_id(NODE_COUNT - 1), false, &[]);
    engine.state.roads[0].connected = true;
    engine.state.roads[0].to = node_id(4);
    assert_matches_oracle(&engine, node_id(0), node_id(NODE_COUNT - 1), false, &[]);
    engine.state.roads[0].to = node_id(1);
    engine.state.roads[199].length = 0.0625;
    assert_matches_oracle(&engine, node_id(0), node_id(NODE_COUNT - 1), false, &[]);
    engine.state.roads.reverse();
    assert_matches_oracle(&engine, node_id(0), node_id(NODE_COUNT - 1), false, &[]);
    assert_matches_oracle(&engine, node_id(0), node_id(2), true, &[]);
}
