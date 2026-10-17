use super::*;

#[test]
fn cars_sharing_a_driveway_queue_instead_of_overlapping() {
    let mut engine = Engine::new_legacy(81);
    let road = engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.kind != RoadKind::Rail && road.stage == Stage::Open)
        .expect("founding road is open");
    let (from, to) = (road.from, road.to);
    let len = distance(engine.node(from).unwrap(), engine.node(to).unwrap());
    let driveway = AccessPath {
        points: vec![
            WorldPoint { x: 250.0, z: 80.0 },
            WorldPoint { x: 250.0, z: 50.0 },
        ],
        surface_y: Vec::new(),
    };
    // Two residents of one building leave by car on the same tick.
    for id in [900_001, 900_002] {
        engine.state.people.travelers.push(Traveler {
            id,
            kind: 1,
            route: vec![from, to],
            route_lengths: vec![len],
            edge_index: 0,
            edge_progress: 0.0,
            speed: 6.0,
            destination: to,
            activity: TravelerActivity::Commute,
            resident: 0,
            name: "Driveway car".into(),
            origin_building: Some(77),
            destination_building: None,
            wait_ticks: 0,
            origin_access: Some(driveway.clone()),
            destination_access: None,
            access_phase: 0,
            access_progress: 0.0,
        });
    }
    let progress = |engine: &Engine, id| {
        let t = engine
            .state
            .people
            .travelers
            .iter()
            .find(|t| t.id == id)
            .unwrap();
        (t.access_phase, t.access_progress)
    };
    for _ in 0..400 {
        engine.advance_travelers();
        let (lead, follow) = (progress(&engine, 900_001), progress(&engine, 900_002));
        if lead.0 == 0 && follow.0 == 0 && follow.1 > 0.0 {
            assert!(
                lead.1 - follow.1 >= VEHICLE_MIN_GAP - 1e-3,
                "{lead:?} {follow:?}"
            );
        }
        // Whatever is drawn never overlaps: the waiting car is not emitted.
        let movement = engine.movement_buffer();
        let at = |id: u32| {
            movement
                .chunks(6)
                .find(|c| c[0] as u32 == id)
                .map(|c| (c[1], c[2]))
        };
        if let (Some(a), Some(b)) = (at(900_001), at(900_002)) {
            assert!((a.0 - b.0).hypot(a.1 - b.1) >= VEHICLE_MIN_GAP - 1e-3);
        }
    }
    // The follower is not stuck: both reach the road.
    assert_eq!(progress(&engine, 900_002).0, 1);
}

#[test]
fn a_car_held_at_a_driveway_survives_save_and_restore() {
    let (mut engine, _len, ids) = arrival_contention_engine();
    engine.advance_travelers();
    assert_eq!(
        engine
            .state
            .people
            .travelers
            .iter()
            .find(|t| t.id == ids[1])
            .unwrap()
            .access_phase,
        1
    );
    let mut restored = Engine::restore_json(&engine.save_json()).expect("held city restores");
    for _ in 0..200 {
        engine.advance_travelers();
        restored.advance_travelers();
        assert_eq!(restored.movement_buffer(), engine.movement_buffer());
    }

    // Saves from before kerb holding kept a held car past the end of its route.
    let (mut legacy, len, ids) = arrival_contention_engine();
    let old = legacy
        .state
        .people
        .travelers
        .iter_mut()
        .find(|t| t.id == ids[1])
        .unwrap();
    old.edge_index = 1;
    old.edge_progress = 0.0;
    let mut legacy = Engine::restore_json(&legacy.save_json()).expect("legacy held car restores");
    legacy.advance_travelers();
    let held = legacy
        .state
        .people
        .travelers
        .iter()
        .find(|t| t.id == ids[1])
        .unwrap();
    assert_eq!((held.access_phase, held.edge_index), (1, 0));
    assert!((held.edge_progress - len.max(1.0)).abs() < 1e-4);
    assert!(movement_of(&legacy, ids[1]).is_some());
}

#[test]
fn road_trips_choose_bodies_and_queue_by_length() {
    // Short bodies keep the historical spacing; long ones need more room.
    assert_eq!(
        vehicle_gap(VEHICLE_COMPACT, VEHICLE_COMPACT),
        VEHICLE_MIN_GAP
    );
    assert_eq!(vehicle_gap(VEHICLE_KEI, VEHICLE_KEI), VEHICLE_MIN_GAP);
    assert!((vehicle_gap(VEHICLE_VAN, VEHICLE_TRUCK) - 5.2).abs() < 1e-5);
    assert!(vehicle_gap(VEHICLE_TAXI, VEHICLE_COMPACT) > VEHICLE_MIN_GAP);

    let mut engine = Engine::new_legacy(81);
    let road = engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.kind != RoadKind::Rail && road.stage == Stage::Open)
        .expect("founding road is open");
    let (from, to) = (road.from, road.to);
    let len = distance(engine.node(from).unwrap(), engine.node(to).unwrap());
    let ids: Vec<u32> = (800_000..800_240).collect();
    for &id in &ids {
        engine.state.people.travelers.push(Traveler {
            id,
            kind: 1,
            route: vec![from, to],
            route_lengths: vec![len],
            edge_index: 0,
            edge_progress: len * 0.5,
            speed: 1.0,
            destination: to,
            activity: if id % 2 == 0 {
                TravelerActivity::Delivery
            } else {
                TravelerActivity::Commute
            },
            resident: 0,
            name: "Test vehicle".into(),
            origin_building: None,
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase: 1,
            access_progress: 0.0,
        });
    }
    let movement = engine.movement_buffer();
    let mut delivery = std::collections::BTreeSet::new();
    let mut private = std::collections::BTreeSet::new();
    for chunk in movement.chunks(6) {
        let id = chunk[0] as u32;
        if !ids.contains(&id) {
            continue;
        }
        let kind = chunk[4] as u8;
        if id % 2 == 0 {
            delivery.insert(kind);
        } else {
            private.insert(kind);
        }
    }
    assert_eq!(delivery, [VEHICLE_VAN, VEHICLE_TRUCK, VEHICLE_MAIL].into());
    assert_eq!(private, [VEHICLE_COMPACT, VEHICLE_KEI, VEHICLE_TAXI].into());
    // Bodies are stable: the same trip reports the same body every tick.
    assert_eq!(engine.movement_buffer(), movement);
}

#[test]
fn movement_buffer_places_traffic_in_left_lane_and_on_sidewalks() {
    for (road_kind, expected_offset, surface_edge) in [
        (RoadKind::Local, 2.53, 3.25),
        (RoadKind::Lane, 1.28, 2.0),
        (RoadKind::Arterial, 8.72, 10.5),
    ] {
        for reverse in [false, true] {
            let mut engine = Engine::new_legacy(81);
            let road = engine
                .state
                .network
                .roads
                .iter_mut()
                .find(|road| road.kind != RoadKind::Rail && road.stage == Stage::Open)
                .expect("founding road is open");
            road.kind = road_kind;
            let (from, to) = if reverse {
                (road.to, road.from)
            } else {
                (road.from, road.to)
            };
            let a = engine.node(from).unwrap().clone();
            let b = engine.node(to).unwrap().clone();
            let len = distance(&a, &b);
            let route = vec![from, to];
            for (id, kind) in [(700_001, 1), (700_002, 0), (700_003, 0)] {
                engine.state.people.travelers.push(Traveler {
                    id,
                    kind,
                    route: route.clone(),
                    route_lengths: vec![len],
                    edge_index: 0,
                    edge_progress: len * 0.5,
                    speed: 1.0,
                    destination: to,
                    activity: crate::TravelerActivity::Stroll,
                    resident: 0,
                    name: "Test traveler".into(),
                    origin_building: None,
                    destination_building: None,
                    wait_ticks: 0,
                    origin_access: None,
                    destination_access: None,
                    access_phase: 1,
                    access_progress: 0.0,
                });
            }
            let movement = engine.movement_buffer();
            let center_x = a.x + (b.x - a.x) * 0.5;
            let center_z = a.z + (b.z - a.z) * 0.5;
            let nx = -(b.z - a.z) / len;
            let nz = (b.x - a.x) / len;
            let car = &movement[1..6];
            assert!((car[0] - center_x - nx * KERB_LANE).abs() < 0.001);
            assert!((car[1] - center_z - nz * KERB_LANE).abs() < 0.001);
            for (id, index, sign) in [(700_002, 7, 1.0), (700_003, 13, -1.0)] {
                let pedestrian = &movement[index..index + 5];
                assert!((pedestrian[0] - center_x - nx * expected_offset * sign).abs() < 0.001);
                assert!((pedestrian[1] - center_z - nz * expected_offset * sign).abs() < 0.001);
                assert!(
                    expected_offset + 0.33 < surface_edge,
                    "whole pedestrian fits surface"
                );
                for (at_start, node) in [(true, &a), (false, &b)] {
                    let endpoint = engine
                        .route_endpoint_position(&route, at_start, id, 0)
                        .unwrap();
                    assert!((endpoint.x - node.x - nx * expected_offset * sign).abs() < 0.001);
                    assert!((endpoint.z - node.z - nz * expected_offset * sign).abs() < 0.001);
                }
            }
        }
    }
}
