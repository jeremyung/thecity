use crate::*;
use std::collections::BTreeSet;

fn presentation_geometry_signature(engine: &Engine) -> String {
    let snapshot = engine.snapshot();
    let open_roads: Vec<_> = snapshot
        .roads
        .iter()
        .filter(|road| road.stage == Stage::Open)
        .map(|road| {
            (
                road.id,
                road.from,
                road.to,
                road.kind.as_str(),
                road.connected,
                road.lanes,
            )
        })
        .collect();
    let endpoint_ids: BTreeSet<_> = open_roads
        .iter()
        .flat_map(|road| [road.1, road.2])
        .collect();
    let platform_ids: BTreeSet<_> = snapshot
        .rail_platforms
        .iter()
        .map(|platform| platform.node_id)
        .collect();
    let relevant_nodes: Vec<_> = snapshot
        .nodes
        .iter()
        .filter(|node| endpoint_ids.contains(&node.id) || platform_ids.contains(&node.id))
        .map(|node| (node.id, node.x, node.z, node.kind.as_str()))
        .collect();
    let open_ids: BTreeSet<_> = open_roads.iter().map(|road| road.0).collect();
    let decks: Vec<_> = snapshot
        .road_decks
        .iter()
        .filter(|deck| open_ids.contains(&deck.road_id))
        .collect();
    serde_json::to_string(&(relevant_nodes, open_roads, decks, snapshot.rail_platforms)).unwrap()
}

#[test]
fn snapshot_and_getter_share_presentation_revision() {
    let engine = Engine::new(87231);
    assert_eq!(
        engine.snapshot().presentation_revision,
        engine.presentation_revision()
    );
}

#[test]
fn tick_and_weather_alone_do_not_change_presentation_revision() {
    let mut engine = Engine::new(87231);
    engine.set_development_rate(0);
    let before = engine.presentation_revision();
    engine.state.weather_kind = WeatherKind::Rain;
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
    for (low, high, phase) in [
        (0.0, 0.479, 0),
        (0.48, 0.729, 1),
        (0.73, 0.849, 2),
        (0.85, 0.999, 3),
        (1.0, 1.0, 4),
    ] {
        let before = engine.presentation_revision();
        engine
            .state
            .construction
            .projects
            .iter_mut()
            .find(|p| p.kind == ProjectKind::Station)
            .unwrap()
            .progress = low;
        let revision = engine.presentation_revision();
        if phase > 0 {
            assert_ne!(revision, before);
        }
        for platform in engine.snapshot().rail_platforms {
            assert_eq!(platform.construction_phase, phase);
        }
        engine
            .state
            .construction
            .projects
            .iter_mut()
            .find(|p| p.kind == ProjectKind::Station)
            .unwrap()
            .progress = high;
        assert_eq!(engine.presentation_revision(), revision);
    }
    let built = engine.presentation_revision();
    engine.state.transit.station_ready = true;
    let ready = engine.presentation_revision();
    assert_ne!(ready, built);
    engine.state.transit.station_online = true;
    assert_ne!(engine.presentation_revision(), ready);
    assert_eq!(
        engine.snapshot().presentation_revision,
        engine.presentation_revision()
    );
}

#[test]
fn station_construction_phases_restore_and_continue_during_normal_growth() {
    let mut live = Engine::new(87231);
    live.set_development_rate(16);
    let mut seen = BTreeSet::new();
    for _ in 0..2000 {
        let phase = live.station_construction_phase();
        if seen.insert(phase) {
            let mut restored = Engine::restore_json(&live.save_json()).unwrap();
            assert_eq!(
                presentation_geometry_signature(&restored),
                presentation_geometry_signature(&live)
            );
            live.step(1);
            restored.step(1);
            assert_eq!(live.save_json(), restored.save_json());
            assert_eq!(
                live.presentation_revision(),
                restored.presentation_revision()
            );
        } else {
            live.step(1);
        }
        if phase == 4 {
            break;
        }
    }
    assert_eq!(seen, BTreeSet::from([0, 1, 2, 3, 4]));
}

#[test]
fn expressway_completion_through_complete_project_changes_presentation_revision() {
    let mut engine = Engine::new(87231);
    engine.set_development_rate(16);
    let expressway = (0..40_000)
        .step_by(80)
        .find_map(|_| {
            engine.step(80);
            engine
                .state
                .construction
                .projects
                .iter()
                .find(|project| project.kind == ProjectKind::Expressway && project.progress < 1.0)
                .cloned()
        })
        .expect("seeded growth should start an expressway project");
    let before = engine.presentation_revision();
    engine.complete_project(expressway.id);
    assert_ne!(engine.presentation_revision(), before);
    assert!(engine
        .state
        .network
        .roads
        .iter()
        .any(|road| road.kind == RoadKind::Expressway
            && road.stage == Stage::Open
            && (road.from == expressway.from.unwrap())
            && (road.to == expressway.to.unwrap())));
}

#[test]
fn presentation_revision_preserves_u64_generations_and_uses_tick_after_exhaustion() {
    let mut engine = Engine::new(87231);
    engine.state.network.access_geometry_revision = 9_007_199_254_740_993;
    let exact = engine.presentation_revision();
    assert!(exact.starts_with("g9007199254740993:"));

    engine.state.network.access_geometry_revision = u64::MAX;
    engine.state.network.access_geometry_revision_exhausted = true;
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
    assert_eq!(
        restored.presentation_revision(),
        engine.presentation_revision()
    );
    assert_eq!(
        presentation_geometry_signature(&restored),
        presentation_geometry_signature(&engine)
    );
}
