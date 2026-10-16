use super::*;

#[test]
fn shrines_and_plazas_do_not_use_the_pocket_park_quota_or_spacing() {
    let mut engine = Engine::new(3);
    engine.set_development_rate(60);
    for _ in 0..8 {
        engine.step(2_000);
    }
    let parks = |engine: &Engine| -> Vec<u32> {
        engine
            .state
            .parcels
            .iter()
            .filter(|p| p.open_space.as_deref() == Some("park"))
            .map(|p| p.id)
            .collect()
    };
    // Precondition: this city can dedicate one more park right now.
    let mut probe = Engine::restore_json(&engine.save_json()).expect("copy restores");
    let before = parks(&probe);
    probe.dedicate_pocket_park();
    let expected: Vec<u32> = parks(&probe)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(expected.len(), 1, "fixture should have a park candidate");

    // Enough shrine and plaza lots to exhaust the quota if they counted as
    // parks, placed beside the chosen lot so park spacing would block it.
    let homes = engine
        .state
        .buildings
        .iter()
        .filter(|b| is_housing(b) && site_active(b))
        .count();
    let site = engine
        .state
        .parcels
        .iter()
        .find(|p| p.id == expected[0])
        .cloned()
        .unwrap();
    let node = site.access_node;
    for i in 0..(homes / HOMES_PER_POCKET_PARK + 2) {
        let id = engine.add_parcel(
            site.x + 0.5 + i as f32 * 0.01,
            site.z,
            6.0,
            6.0,
            node,
            0.5,
            true,
        );
        assert_ne!(id, 0);
        let parcel = engine
            .state
            .parcels
            .iter_mut()
            .find(|p| p.id == id)
            .unwrap();
        parcel.open_space = Some(if i == 0 { "plaza" } else { "shrine" }.into());
    }
    engine.dedicate_pocket_park();
    let after = parks(&engine);
    assert!(
        after.contains(&expected[0]),
        "shrines and plazas blocked a pocket park"
    );
    assert_eq!(after.len(), before.len() + 1);
}

#[test]
fn mature_neighborhoods_keep_spaced_pocket_parks_on_vacant_lots() {
    let mut engine = Engine::new(3);
    engine.set_development_rate(60);
    for _ in 0..12 {
        engine.step(2_000);
    }
    let built: BTreeSet<u32> = engine.state.buildings.iter().map(|b| b.parcel_id).collect();
    let parks: Vec<&Parcel> = engine
        .state
        .parcels
        .iter()
        .filter(|p| p.protected && !built.contains(&p.id))
        .filter(|p| p.open_space.as_deref() == Some("park"))
        .collect();
    assert!(
        parks.len() >= 2,
        "growing neighborhoods dedicate pocket parks"
    );
    let homes = engine
        .state
        .buildings
        .iter()
        .filter(|b| is_housing(b) && site_active(b))
        .count();
    assert!(parks.len() * HOMES_PER_POCKET_PARK <= homes + HOMES_PER_POCKET_PARK);
    for (i, park) in parks.iter().enumerate() {
        assert!(POCKET_PARK_AREA.contains(&(park.width * park.depth)));
        assert!(park.reservation.is_none());
        for other in &parks[i + 1..] {
            assert!((park.x - other.x).hypot(park.z - other.z) >= POCKET_PARK_SPACING);
        }
    }
    assert!(engine
        .state
        .events
        .iter()
        .any(|event| event.kind == "ParkDedicated"));
    engine.validate().expect("parks keep the city valid");

    // Dedication is part of the deterministic simulation.
    let mut replay = Engine::new(3);
    replay.set_development_rate(60);
    for _ in 0..12 {
        replay.step(2_000);
    }
    let protected = |engine: &Engine| -> Vec<u32> {
        engine
            .state
            .parcels
            .iter()
            .filter(|p| p.protected)
            .map(|p| p.id)
            .collect()
    };
    assert_eq!(protected(&engine), protected(&replay));
}
