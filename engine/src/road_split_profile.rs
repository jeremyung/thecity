//! Native-only measurements for the synchronous open-road split transaction.
//!
//! Register from `lib.rs` with `#[cfg(test)] mod road_split_profile;`.

use crate::{
    point_segment_distance, Engine, Node, Road, RoadSplitError, Traveler, MAX_ROUTE_NODES,
    MAX_SEGMENTS, MAX_TRAVELERS,
};
use std::time::{Duration, Instant};

const SEED: u32 = 87_231;

fn split_host(engine: &Engine) -> Road {
    engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
                && road.length >= 20.0
        })
        .max_by(|a, b| a.length.total_cmp(&b.length))
        .expect("fresh world has a split-capable open road")
        .clone()
}

fn route_away_from_split(engine: &Engine, host: &Road) -> Road {
    let a = engine.node(host.from).unwrap();
    let b = engine.node(host.to).unwrap();
    let (sx, sz) = ((a.x + b.x) * 0.5, (a.z + b.z) * 0.5);
    engine
        .state
        .roads
        .iter()
        .filter(|road| road.id != host.id && road.stage == "open" && road.length > 0.0)
        .find(|road| {
            let (Some(from), Some(to)) = (engine.node(road.from), engine.node(road.to)) else {
                return false;
            };
            point_segment_distance(sx, sz, from.x, from.z, to.x, to.z) >= 1.0
        })
        .expect("fresh world has another valid road away from split point")
        .clone()
}

fn route_after_host_split(
    engine: &Engine,
    host: &Road,
    route_nodes: usize,
) -> (Vec<u32>, Vec<f32>) {
    let a = engine.node(host.from).unwrap();
    let b = engine.node(host.to).unwrap();
    let (sx, sz) = ((a.x + b.x) * 0.5, (a.z + b.z) * 0.5);
    let continuation = engine
        .state
        .roads
        .iter()
        .filter(|road| road.id != host.id && road.stage == "open")
        .find_map(|road| {
            let other = if road.from == host.to {
                road.to
            } else if road.to == host.to {
                road.from
            } else {
                return None;
            };
            let (Some(from), Some(to)) = (engine.node(road.from), engine.node(road.to)) else {
                return None;
            };
            (point_segment_distance(sx, sz, from.x, from.z, to.x, to.z) >= 1.0)
                .then_some((other, road.length))
        })
        .expect("host endpoint has a connected continuation away from its midpoint");

    // One occurrence of the host edge expands by exactly one node. A 127
    // node input therefore reaches (but does not exceed) the 128-node cap.
    let mut route = vec![host.from, host.to];
    while route.len() < route_nodes.saturating_sub(1) {
        route.push(if route.last() == Some(&host.to) {
            continuation.0
        } else {
            host.to
        });
    }
    let mut lengths = vec![host.length];
    lengths.resize(route.len() - 1, continuation.1);
    (route, lengths)
}

fn add_travelers(
    engine: &mut Engine,
    host: &Road,
    count: usize,
    route_nodes: usize,
    all_affected: bool,
) {
    let route_road = route_away_from_split(engine, host);
    let long_route: Vec<u32> = (0..route_nodes)
        .map(|index| {
            if index % 2 == 0 {
                route_road.from
            } else {
                route_road.to
            }
        })
        .collect();
    let long_edge_lengths = vec![route_road.length; route_nodes - 1];

    for index in 0..count {
        let affected = index == 0 || all_affected;
        let (route, route_lengths, edge_index, edge_progress) = if affected {
            // One actor is actively travelling over the host edge, so the
            // transaction must insert the split node and remap its cursor.
            let (route, lengths) = route_after_host_split(engine, host, route_nodes);
            (route, lengths, 0, host.length * 0.73)
        } else {
            (
                long_route.clone(),
                long_edge_lengths.clone(),
                (route_nodes - 2).min(63),
                route_road.length * 0.41,
            )
        };
        let id = engine.id();
        engine.state.travelers.push(Traveler {
            id,
            kind: 0,
            route,
            route_lengths,
            edge_index,
            edge_progress,
            speed: 1.0,
            destination: if affected { host.to } else { route_road.to },
            activity: "road split profile".into(),
            resident: 0,
            name: format!("Profile traveler {index}"),
            origin_building: None,
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        });
    }
}

fn add_disjoint_road_geometry(engine: &mut Engine, host: &Road, target_roads: usize) {
    assert!(target_roads <= MAX_SEGMENTS);
    let a = engine.node(host.from).unwrap();
    let b = engine.node(host.to).unwrap();
    let sx = a.x + (b.x - a.x) * 0.5;
    let sz = a.z + (b.z - a.z) * 0.5;

    // Use many small, valid detached survey segments on a regular grid. Each
    // has unique endpoints and IDs; they stay well outside the host split
    // point's one-metre node and half-metre segment exclusion radii.
    let mut index = 0usize;
    while engine.state.roads.len() < target_roads {
        let col = index % 71;
        let row = index / 71;
        assert!(row < 71, "grid is large enough for MAX_SEGMENTS");
        let x = -385.0 + col as f32 * 11.0;
        let z = -385.0 + row as f32 * 11.0;
        index += 1;
        let cx = x + 0.5;
        let cz = z;
        if (cx - sx).hypot(cz - sz) < 4.0 {
            continue;
        }
        let from = engine.id();
        let to = engine.id();
        engine.state.nodes.push(Node {
            id: from,
            x,
            z,
            kind: "junction".into(),
        });
        engine.state.nodes.push(Node {
            id: to,
            x: x + 1.0,
            z,
            kind: "junction".into(),
        });
        let road_id = engine.id();
        engine.state.roads.push(Road {
            id: road_id,
            from,
            to,
            kind: "local".into(),
            stage: "survey".into(),
            progress: 0.0,
            connected: false,
            length: 1.0,
            lanes: 1,
        });
    }
}

fn add_connected_road_geometry(engine: &mut Engine, host: &Road, target_roads: usize) {
    assert!(target_roads <= MAX_SEGMENTS);
    let a = engine.node(host.from).unwrap();
    let b = engine.node(host.to).unwrap();
    let sx = a.x + (b.x - a.x) * 0.5;
    let sz = a.z + (b.z - a.z) * 0.5;

    // Snake a connected open-road chain through a north-edge strip beyond
    // seeded parcels. This stays eligible and lets commit's connectivity pass
    // visit nearly 5,000 open segments rather than scanning detached roads.
    let mut path = Vec::with_capacity(7 * 756);
    for row in 0..7 {
        let z = 260.0 + row as f32 * 4.0;
        if row % 2 == 0 {
            for col in 0..756 {
                path.push((365.0 - col as f32, z));
            }
        } else {
            for col in (0..756).rev() {
                path.push((365.0 - col as f32, z));
            }
        }
    }
    assert!(path
        .iter()
        .all(|(x, z)| x.abs() <= 400.0 && z.abs() <= 400.0));
    assert!(path.iter().all(|(_, z)| *z > 250.0));
    assert!(path.iter().all(|(x, z)| (x - sx).hypot(z - sz) >= 1.0));
    let needed = target_roads.saturating_sub(engine.state.roads.len());
    if needed == 0 {
        return;
    }

    let gateway = engine
        .state
        .nodes
        .iter()
        .find(|node| node.kind == "gateway")
        .expect("fresh world has a gateway")
        .clone();
    let mut previous = gateway.id;
    let mut previous_point = (gateway.x, gateway.z);
    for point in std::iter::once(path[0]).chain(path.into_iter().skip(1)) {
        if engine.state.roads.len() >= target_roads {
            break;
        }
        let node_id = engine.id();
        engine.state.nodes.push(Node {
            id: node_id,
            x: point.0,
            z: point.1,
            kind: "junction".into(),
        });
        let road_id = engine.id();
        engine.state.roads.push(Road {
            id: road_id,
            from: previous,
            to: node_id,
            kind: "local".into(),
            stage: "open".into(),
            progress: 1.0,
            connected: true,
            length: (point.0 - previous_point.0).hypot(point.1 - previous_point.1),
            lanes: 1,
        });
        previous = node_id;
        previous_point = point;
    }
    assert_eq!(engine.state.roads.len(), target_roads);
}

fn route_position(engine: &Engine, traveler: &Traveler) -> (f32, f32) {
    if traveler.edge_index >= traveler.route_lengths.len() {
        let node = engine.node(*traveler.route.last().unwrap()).unwrap();
        return (node.x, node.z);
    }
    let a = engine.node(traveler.route[traveler.edge_index]).unwrap();
    let b = engine
        .node(traveler.route[traveler.edge_index + 1])
        .unwrap();
    let fraction =
        (traveler.edge_progress / traveler.route_lengths[traveler.edge_index]).clamp(0.0, 1.0);
    (a.x + (b.x - a.x) * fraction, a.z + (b.z - a.z) * fraction)
}

fn make_case(
    travelers: usize,
    route_nodes: usize,
    roads: usize,
    connected_geometry: bool,
    all_affected: bool,
) -> (Engine, u32) {
    let mut engine = Engine::new(SEED);
    let host = split_host(&engine);
    add_travelers(&mut engine, &host, travelers, route_nodes, all_affected);
    let target_roads = roads.max(engine.state.roads.len());
    if target_roads > engine.state.roads.len() {
        if connected_geometry {
            add_connected_road_geometry(&mut engine, &host, target_roads);
        } else {
            add_disjoint_road_geometry(&mut engine, &host, target_roads);
        }
    }
    (engine, host.id)
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    sorted[((sorted.len() - 1) as f64 * p).ceil() as usize]
}

fn export_stress_fixture_if_requested() {
    if std::env::var_os("EXPORT_ROAD_SPLIT_PROFILE").is_none() {
        return;
    }
    let connected = std::env::var_os("ROAD_SPLIT_PROFILE_CONNECTED").is_some();
    let all_affected = std::env::var_os("ROAD_SPLIT_PROFILE_ALL_AFFECTED").is_some();
    let (engine, host_road_id) = make_case(
        MAX_TRAVELERS,
        MAX_ROUTE_NODES,
        MAX_SEGMENTS - 1,
        connected,
        all_affected,
    );
    engine.validate().expect("stress fixture is valid");
    let fixture = serde_json::json!({
        "schema": 1,
        "connected_geometry": connected,
        "all_affected": all_affected,
        "host_road_id": host_road_id,
        "split_fraction": 0.5,
        "save_json": engine.save_json(),
    });
    let path = std::env::var_os("ROAD_SPLIT_PROFILE_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation/road-split-profile-stress.json"));
    std::fs::write(
        &path,
        serde_json::to_vec(&fixture).expect("profile fixture serializes"),
    )
    .expect("write opted-in profile fixture");
    println!("road_split_profile exported {}", path.display());
}

fn measure(
    label: &str,
    travelers: usize,
    route_nodes: usize,
    roads: usize,
    connected_geometry: bool,
    samples: usize,
) {
    let (base, host_id) = make_case(travelers, route_nodes, roads, connected_geometry, false);
    let roads_before = base.state.roads.len();
    let nodes_before = base.state.nodes.len();
    let saved = base.save_json();
    let mut durations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let mut engine = Engine::restore_json(&saved).expect("profile fixture restores");
        let start = Instant::now();
        let outcome = engine
            .try_split_open_road(host_id, 0.5)
            .expect("profile split succeeds");
        durations.push(start.elapsed());
        assert!(engine.node(outcome.split_node_id).is_some());
        assert_eq!(engine.state.roads.len(), roads_before + 1);
    }
    durations.sort_unstable();
    let total = durations.iter().copied().sum::<Duration>();
    let mean = total / samples as u32;
    println!(
        "road_split_profile {label}: travelers={travelers}, route_nodes={route_nodes} (affected actor expands {} to {}), roads_before={roads_before}, nodes_before={nodes_before}, samples={samples}, mean_us={}, p50_us={}, p95_us={}, max_us={}",
        route_nodes.saturating_sub(1),
        route_nodes,
        mean.as_micros(),
        percentile(&durations, 0.50).as_micros(),
        percentile(&durations, 0.95).as_micros(),
        durations.last().unwrap().as_micros(),
    );
}

#[test]
fn atomic_split_preserves_actor_positions_and_late_failure_writes_nothing() {
    let (mut engine, host_id) = make_case(32, 128, 0, false, false);
    let roads_before = engine.state.roads.len();
    let before_positions: Vec<_> = engine
        .state
        .travelers
        .iter()
        .map(|actor| route_position(&engine, actor))
        .collect();
    let before_routes: Vec<_> = engine
        .state
        .travelers
        .iter()
        .map(|actor| actor.route.clone())
        .collect();
    let outcome = engine.try_split_open_road(host_id, 0.5).unwrap();
    assert_eq!(engine.state.roads.len(), roads_before + 1);
    let affected = &engine.state.travelers[0];
    assert_eq!(affected.route.len(), 128);
    assert_eq!(affected.route[1], outcome.split_node_id);
    for (index, actor) in engine.state.travelers.iter().enumerate() {
        let after = route_position(&engine, actor);
        assert!((before_positions[index].0 - after.0).abs() < 1e-4);
        assert!((before_positions[index].1 - after.1).abs() < 1e-4);
        if index > 0 {
            assert_eq!(actor.route, before_routes[index]);
        }
    }

    let (mut rejected, host_id) = make_case(8, 128, 0, false, false);
    rejected
        .state
        .travelers
        .last_mut()
        .unwrap()
        .route_lengths
        .clear();
    let before = rejected.save_json();
    assert_eq!(
        rejected.try_split_open_road(host_id, 0.5),
        Err(RoadSplitError::InvalidActorRoute)
    );
    assert_eq!(rejected.save_json(), before);
}

#[test]
#[ignore = "native road-split timing profile; run explicitly with --ignored"]
fn profile_atomic_open_road_split_cost() {
    // Keep setup/restore outside the timed region. The operation includes the
    // full read-only preflight plus commit/connectivity refresh.
    export_stress_fixture_if_requested();
    measure("representative", 4, 16, 6, false, 25);
    measure("traveler_cap", MAX_TRAVELERS, MAX_ROUTE_NODES, 6, false, 15);
    measure(
        "road_cap_with_traveler_cap",
        MAX_TRAVELERS,
        MAX_ROUTE_NODES,
        MAX_SEGMENTS - 1,
        false,
        25,
    );
    measure(
        "connected_road_cap_with_traveler_cap",
        MAX_TRAVELERS,
        MAX_ROUTE_NODES,
        MAX_SEGMENTS - 1,
        true,
        25,
    );
    if std::env::var_os("ROAD_SPLIT_PROFILE_ALL_AFFECTED").is_some() {
        let (base, host_id) = make_case(
            MAX_TRAVELERS,
            MAX_ROUTE_NODES,
            MAX_SEGMENTS - 1,
            true,
            true,
        );
        base.validate().expect("all-affected profile case is valid");
        let saved = base.save_json();
        let samples = 25;
        let mut durations = Vec::with_capacity(samples);
        for _ in 0..samples {
            let mut engine = Engine::restore_json(&saved).expect("all-affected profile fixture restores");
            let start = Instant::now();
            engine.try_split_open_road(host_id, 0.5).expect("all-affected split succeeds");
            durations.push(start.elapsed());
        }
        durations.sort_unstable();
        println!(
            "road_split_profile all_affected_connected_road_cap: travelers={MAX_TRAVELERS}, affected_travelers={MAX_TRAVELERS}, route_nodes={}, roads_before={}, nodes_before={}, samples={samples}, mean_us={}, p50_us={}, p95_us={}, max_us={}",
            MAX_ROUTE_NODES,
            base.state.roads.len(),
            base.state.nodes.len(),
            (durations.iter().copied().sum::<Duration>() / samples as u32).as_micros(),
            percentile(&durations, 0.50).as_micros(),
            percentile(&durations, 0.95).as_micros(),
            durations.last().unwrap().as_micros(),
        );
    }
}
