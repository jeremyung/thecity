use crate::Engine;

#[test]
fn current_growth_validates_access_joins_rejects_tampering_and_replays() {
    let mut live = Engine::new(87231);
    live.set_development_rate(16);

    // Let the ordinary masterplan scheduler create the trip and its saved
    // access connectors. Some seeds first emit delivery-only connectors, so
    // continue to 12,000 ticks if needed to observe both kinds of join.
    for _ in 0..20 {
        live.step(200);
        if live.state.travelers.iter().any(|traveler|
            traveler.origin_access.is_some() && traveler.destination_access.is_some()) {
            break;
        }
    }
    if !live.state.travelers.iter().any(|traveler|
        traveler.origin_access.is_some() && traveler.destination_access.is_some()) {
        for _ in 0..40 {
            live.step(200);
            if live.state.travelers.iter().any(|traveler|
                traveler.origin_access.is_some() && traveler.destination_access.is_some()) {
                break;
            }
        }
    }

    let both_id = live.state.travelers.iter()
        .find(|traveler| traveler.origin_access.is_some() && traveler.destination_access.is_some())
        .map(|traveler| traveler.id);
    let origin_id = both_id.or_else(|| live.state.travelers.iter()
        .find(|traveler| traveler.origin_access.is_some())
        .map(|traveler| traveler.id))
        .expect("ordinary current growth creates an origin access connector by 12,000 ticks");
    let destination_id = both_id.or_else(|| live.state.travelers.iter()
        .find(|traveler| traveler.destination_access.is_some())
        .map(|traveler| traveler.id))
        .expect("ordinary current growth creates a destination access connector by 12,000 ticks");
    live.validate().expect("naturally generated access joins validate");
    let snapshot_baseline = live.snapshot();
    assert_eq!(snapshot_baseline.tick, live.tick());
    assert_eq!(snapshot_baseline.seed, 87231);
    let canonical = live.save_json();
    let mut resumed = Engine::restore_json(&canonical).expect("valid current-format save restores");
    assert_eq!(resumed.save_json(), canonical);

    for (access_key, traveler_id) in [
        ("origin_access", origin_id),
        ("destination_access", destination_id),
    ] {
        let mut malformed: serde_json::Value = serde_json::from_str(&canonical).unwrap();
        let travelers = malformed["travelers"].as_array_mut().unwrap();
        let saved = travelers.iter_mut().find(|saved| saved["id"] == traveler_id).unwrap();
        let points = saved[access_key]["points"].as_array_mut().unwrap();
        let endpoint = if access_key == "origin_access" { points.last_mut().unwrap() } else { points.first_mut().unwrap() };
        let coordinate = if access_key == "origin_access" { "x" } else { "z" };
        let shifted = endpoint[coordinate].as_f64().unwrap() + 1.0;
        endpoint[coordinate] = serde_json::json!(shifted);
        assert!(Engine::restore_json(&malformed.to_string()).is_err(),
            "tampered {access_key} route join must be rejected");
    }

    for _ in 0..100 {
        live.step(1);
        resumed.step(1);
        assert_eq!(live.movement_packet(), resumed.movement_packet());
        assert_eq!(live.save_json(), resumed.save_json());
    }
}

#[test]
fn generated_delivery_return_origin_connector_meets_outgoing_route_start() {
    let mut engine = Engine::new(87231);
    engine.set_development_rate(16);
    let mut returning = None;
    for _ in 0..60 {
        engine.step(200);
        returning = engine.state.travelers.iter()
            .find(|traveler| traveler.activity == "delivery_return" && traveler.origin_access.is_some())
            .cloned();
        if returning.is_some() {
            break;
        }
    }
    let traveler = returning.expect("ordinary seed growth creates a delivery return with origin access by 12,000 ticks");
    let endpoint = engine.route_endpoint_position(&traveler.route, true, traveler.id, traveler.kind)
        .expect("delivery return route has a starting endpoint");
    let connector_endpoint = traveler.origin_access.as_ref().unwrap().points.last().unwrap();
    assert!((connector_endpoint.x - endpoint.x).hypot(connector_endpoint.z - endpoint.z) <= 0.01);
    let canonical = engine.save_json();
    let restored = Engine::restore_json(&canonical).expect("delivery return save restores");
    assert_eq!(restored.save_json(), canonical);
}
