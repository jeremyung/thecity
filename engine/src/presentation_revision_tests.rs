use super::*;
use std::collections::BTreeSet;

fn presentation_geometry_signature(engine: &Engine) -> String {
    let snapshot = engine.snapshot();
    let open_roads: Vec<_> = snapshot.roads.iter().filter(|road| road.stage == "open")
        .map(|road| (road.id, road.from, road.to, road.kind.as_str(), road.connected, road.lanes))
        .collect();
    let endpoint_ids: BTreeSet<_> = open_roads.iter().flat_map(|road| [road.1, road.2]).collect();
    let platform_ids: BTreeSet<_> = snapshot.rail_platforms.iter().map(|platform| platform.node_id).collect();
    let relevant_nodes: Vec<_> = snapshot.nodes.iter()
        .filter(|node| endpoint_ids.contains(&node.id) || platform_ids.contains(&node.id))
        .map(|node| (node.id, node.x, node.z, node.kind.as_str()))
        .collect();
    let open_ids: BTreeSet<_> = open_roads.iter().map(|road| road.0).collect();
    let decks: Vec<_> = snapshot.road_decks.iter().filter(|deck| open_ids.contains(&deck.road_id)).collect();
    serde_json::to_string(&(relevant_nodes, open_roads, decks, snapshot.rail_platforms)).unwrap()
}

#[test]
fn snapshot_and_getter_share_presentation_revision() {
    let engine = Engine::new(87231);
    assert_eq!(engine.snapshot().presentation_revision, engine.presentation_revision());
}

#[test]
fn tick_and_weather_alone_do_not_change_presentation_revision() {
    let mut engine = Engine::new(87231);
    engine.set_development_rate(0);
    let before = engine.presentation_revision();
    engine.state.weather_kind = "rain".into();
    engine.state.weather_intensity = 0.8;
    // step() can change reservations even with development paused. Isolate
    // clock/weather writes here; the growth test below covers real stepping.
    engine.state.tick += 100;
    assert_eq!(engine.presentation_revision(), before);
    assert_eq!(engine.snapshot().presentation_revision, before);
}

#[test]
fn presentation_revision_tracks_station_visual_threshold_and_service_state() {
    let mut engine = Engine::new(87231);
    engine.state.projects.iter_mut().find(|project| project.kind == "station").unwrap().progress = 0.2;
    let at_threshold = engine.presentation_revision();
    engine.state.projects.iter_mut().find(|project| project.kind == "station").unwrap().progress = 0.199;
    assert_eq!(engine.presentation_revision(), at_threshold);
    engine.state.projects.iter_mut().find(|project| project.kind == "station").unwrap().progress = 0.201;
    let platform_built = engine.presentation_revision();
    assert_ne!(platform_built, at_threshold);

    engine.state.station_ready = true;
    let ready = engine.presentation_revision();
    assert_ne!(ready, platform_built);
    engine.state.station_online = true;
    let online = engine.presentation_revision();
    assert_ne!(online, ready);
    assert_eq!(engine.snapshot().presentation_revision, online);
}

#[test]
fn expressway_completion_through_complete_project_changes_presentation_revision() {
    let mut engine = Engine::new(87231);
    engine.set_development_rate(16);
    let expressway = (0..40_000).step_by(80).find_map(|_| {
        engine.step(80);
        engine.state.projects.iter().find(|project| project.kind == "expressway" && project.progress < 1.0).cloned()
    }).expect("seeded growth should start an expressway project");
    let before = engine.presentation_revision();
    engine.complete_project(expressway.id);
    assert_ne!(engine.presentation_revision(), before);
    assert!(engine.state.roads.iter().any(|road| road.kind == "expressway"
        && road.stage == "open" && (road.from == expressway.from.unwrap()) && (road.to == expressway.to.unwrap())));
}

#[test]
fn presentation_revision_preserves_u64_generations_and_uses_tick_after_exhaustion() {
    let mut engine = Engine::new(87231);
    engine.state.access_geometry_revision = 9_007_199_254_740_993;
    let exact = engine.presentation_revision();
    assert!(exact.starts_with("g9007199254740993:"));

    engine.state.access_geometry_revision = u64::MAX;
    engine.state.access_geometry_revision_exhausted = true;
    engine.state.tick = 41;
    let exhausted = engine.presentation_revision();
    assert!(exhausted.starts_with("t41:"));
    engine.state.tick += 1;
    assert_ne!(engine.presentation_revision(), exhausted);
}

#[test]
fn unchanged_presentation_revision_implies_stable_open_infrastructure_and_restore() {
    let mut engine = Engine::new(87231);
    engine.set_development_rate(16);
    for _ in 0..150 {
        let revision = engine.presentation_revision();
        let signature = presentation_geometry_signature(&engine);
        engine.step(80);
        if engine.presentation_revision() == revision {
            assert_eq!(presentation_geometry_signature(&engine), signature);
        }
    }

    let saved = engine.save_json();
    let restored = Engine::restore_json(&saved).unwrap();
    assert_eq!(restored.presentation_revision(), engine.presentation_revision());
    assert_eq!(presentation_geometry_signature(&restored), presentation_geometry_signature(&engine));
}
