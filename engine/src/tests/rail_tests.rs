use super::*;

#[test]
fn inbound_train_passenger_waits_at_platform_until_home_route_opens() {
    let (mut engine, station, gateway) = rail_test_engine(477);
    let resident_id = add_rail_test_resident(&mut engine, gateway);
    let station_position = engine.node(station).unwrap();
    let home_node = engine.add_node(
        station_position.x + 18.0,
        station_position.z - 18.0,
        "junction",
    );
    let train_id = engine.id();
    let resident = engine
        .state
        .residents
        .iter_mut()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    resident.home_node = home_node;
    resident.at_work = true;
    resident.transfer = "riding_inbound".into();
    resident.train_id = Some(train_id);
    engine.state.trains.push(Train {
        id: train_id,
        route: vec![station, gateway],
        distance: TRAIN_HALF_LENGTH,
        speed: 8.0,
        direction: 1,
        dwell_ticks: 0,
        passengers: vec![resident_id],
    });

    assert!(engine.route(station, home_node, false).is_empty());
    engine.handle_train_terminal(train_id, true, station, gateway);
    let waiting = engine
        .state
        .residents
        .iter()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    assert_eq!(waiting.transfer, "waiting_alight");
    assert_eq!(waiting.train_id, None);
    assert_eq!(waiting.pending_alight.as_ref().unwrap().stop_node, station);
    assert_eq!(engine.state.trains[0].passengers, Vec::<u32>::new());
    let platform_position = waiting.pending_alight.as_ref().unwrap().wait_position;
    assert_eq!(engine.resident_location_node(waiting), Some(station));
    engine.validate().unwrap();

    let save = engine.save_json();
    let mut restored = Engine::restore_json(&save).unwrap();
    assert_eq!(restored.save_json(), save);
    restored.add_road(station, home_node, "local", "open", 1.0);
    restored.recompute_connectivity();
    restored.step(1);
    let traveler = restored
        .state
        .travelers
        .iter()
        .find(|traveler| traveler.resident == resident_id)
        .unwrap();
    assert_eq!(traveler.activity, "walking_home");
    assert_eq!(traveler.access_phase, 0);
    let origin = &traveler.origin_access.as_ref().unwrap().points[0];
    assert_eq!(origin.x, platform_position.x);
    assert_eq!(origin.z, platform_position.z);
    assert_eq!(restored.state.residents[0].transfer, "walking_home");
    assert!(restored.state.trains[0].passengers.is_empty());
    restored.validate().unwrap();
}

#[test]
fn master_plan_keeps_a_station_plaza_and_spaced_neighborhood_shrines() {
    let mut engine = Engine::new(3);
    engine.set_development_rate(60);
    for _ in 0..12 {
        engine.step(2_000);
    }
    let built: BTreeSet<u32> = engine.state.buildings.iter().map(|b| b.parcel_id).collect();
    let kind = |kind: &str| -> Vec<Parcel> {
        engine
            .state
            .parcels
            .iter()
            .filter(|p| p.open_space.as_deref() == Some(kind))
            .cloned()
            .collect()
    };
    let (plazas, shrines, parks) = (kind("plaza"), kind("shrine"), kind("park"));
    assert!(
        !plazas.is_empty() && plazas.len() <= 2,
        "plaza lots: {}",
        plazas.len()
    );
    assert!(
        !shrines.is_empty() && shrines.len() <= 6,
        "shrine lots: {}",
        shrines.len()
    );
    assert!(!parks.is_empty());
    let station = engine
        .plan
        .as_ref()
        .and_then(|runtime| {
            runtime
                .plan
                .nodes
                .iter()
                .find(|n| n.kind == masterplan::NodeKind::Station)
        })
        .map(|n| n.p)
        .expect("master plan has a station");
    for plaza in &plazas {
        assert!((plaza.x - station.x).hypot(plaza.z - station.z) <= 45.0 + 1e-3);
    }
    for (i, shrine) in shrines.iter().enumerate() {
        assert!((shrine.x - station.x).hypot(shrine.z - station.z) >= 150.0);
        for other in &shrines[i + 1..] {
            assert!((shrine.x - other.x).hypot(shrine.z - other.z) >= 260.0);
        }
    }
    for space in plazas.iter().chain(&shrines).chain(&parks) {
        assert!(space.protected && !built.contains(&space.id));
    }
    for event in ["PlazaReserved", "ShrineReserved"] {
        assert!(
            engine.state.events.iter().any(|e| e.kind == event),
            "{event}"
        );
    }
    engine.validate().expect("open space keeps the city valid");
    let restored = Engine::restore_json(&engine.save_json()).expect("save restores");
    let open_spaces = |engine: &Engine| -> Vec<(u32, Option<String>)> {
        engine
            .state
            .parcels
            .iter()
            .map(|p| (p.id, p.open_space.clone()))
            .collect()
    };
    assert_eq!(open_spaces(&restored), open_spaces(&engine));
}

#[test]
fn train_shuttle_dwells_and_reverses_inside_rail_end_clearances() {
    let mut engine = Engine::new_legacy(77);
    let rail = engine
        .state
        .roads
        .iter_mut()
        .find(|road| road.kind == "rail")
        .expect("founding plan includes a rail corridor");
    rail.stage = "open".into();
    rail.connected = true;
    let rail_length = rail.length;
    engine.state.station_ready = true;
    engine.state.station_online = true;
    let min_center = TRAIN_HALF_LENGTH;
    let max_center = rail_length - TRAIN_HALF_LENGTH;
    assert!(max_center > min_center);

    let mut last_position: Option<(f32, f32)> = None;
    let mut last_direction = 1;
    let mut reversals = 0;
    for _ in 0..2_000 {
        engine.state.tick += 1;
        engine.advance_trains();
        let train = &engine.state.trains[0];
        assert!(train.distance >= min_center - 0.001);
        assert!(train.distance <= max_center + 0.001);
        let movement = engine.movement_buffer();
        let position = (movement[1], movement[2]);
        if train.direction != last_direction {
            reversals += 1;
            if let Some(previous) = last_position {
                assert!(
                    (position.0 - previous.0).hypot(position.1 - previous.1) < 1.0,
                    "consist jumped while reversing"
                );
            }
            last_direction = train.direction;
        }
        last_position = Some(position);
    }
    assert!(reversals >= 4, "shuttle did not complete a round trip");
    assert!(engine
        .state
        .events
        .iter()
        .any(|event| event.kind == "TrainGatewayStop"));
    assert!(engine
        .state
        .events
        .iter()
        .any(|event| event.kind == "TrainStationStop"));
}

#[test]
fn train_doors_open_only_during_full_open_phase_and_resume_deterministically() {
    let (mut engine, station, gateway) = rail_test_engine(85);
    let resident_id = add_rail_test_resident(&mut engine, gateway);
    {
        let resident = engine
            .state
            .residents
            .iter_mut()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        resident.transfer = "waiting_station".into();
        resident.activity = "waiting_for_train".into();
    }
    let train_id = engine.id();
    engine.state.trains.push(Train {
        id: train_id,
        route: vec![station, gateway],
        distance: TRAIN_HALF_LENGTH,
        speed: 8.0,
        direction: 1,
        dwell_ticks: TRAIN_STATION_DWELL_TICKS,
        passengers: Vec::new(),
    });
    assert_eq!(engine.snapshot().trains[0].door_open_fraction, 0.0);
    for _ in 0..19 {
        engine.state.tick += 1;
        engine.advance_trains();
    }
    assert_eq!(engine.state.trains[0].dwell_ticks, 201);
    assert!(engine.state.trains[0].passengers.is_empty());
    assert!((engine.snapshot().trains[0].door_open_fraction - 0.95).abs() < 0.001);

    engine.state.tick = 0;
    engine.advance_trains();
    assert_eq!(engine.state.trains[0].dwell_ticks, 200);
    assert_eq!(engine.state.trains[0].passengers, vec![resident_id]);
    assert_eq!(engine.snapshot().trains[0].door_open_fraction, 1.0);
    assert_eq!(engine.snapshot().trains[0].door_terminal, Some(station));

    let resident = engine
        .state
        .residents
        .iter_mut()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    resident.transfer = "riding_inbound".into();
    resident.train_id = Some(train_id);
    engine.state.trains[0].dwell_ticks = 201;
    engine.state.tick = 1;
    assert_eq!(engine.state.trains[0].passengers, vec![resident_id]);
    assert_eq!(engine.snapshot().trains[0].door_open_fraction, 0.95);
    assert!(!engine.state.events.iter().any(|event| {
        event.kind == "TrainPassengerAlighted" && event.tick == engine.state.tick
    }));
    engine.advance_trains();
    assert_eq!(engine.state.trains[0].dwell_ticks, 200);
    assert!(engine.state.trains[0].passengers.is_empty());
    assert!(engine.state.events.iter().any(|event| {
        event.kind == "TrainPassengerAlighted" && event.tick == engine.state.tick
    }));

    engine.state.trains[0].dwell_ticks = TRAIN_DOOR_PHASE_TICKS;
    engine.state.tick = 23_000;
    engine.state.trains[0].passengers.push(resident_id);
    let resident = engine
        .state
        .residents
        .iter_mut()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    resident.transfer = "riding_outbound".into();
    resident.train_id = Some(train_id);
    let save = engine.save_json();
    let mut resumed = Engine::restore_json(&save).unwrap();
    assert_eq!(engine.snapshot().trains[0].door_open_fraction, 1.0);
    let station_distance = engine.state.trains[0].distance;
    for _ in 0..TRAIN_DOOR_PHASE_TICKS - 1 {
        engine.state.tick += 1;
        resumed.state.tick += 1;
        engine.advance_trains();
        resumed.advance_trains();
        assert_eq!(engine.state.trains[0].distance, station_distance);
        assert!(engine.snapshot().trains[0].door_open_fraction > 0.0);
        assert_eq!(engine.save_json(), resumed.save_json());
    }
    engine.state.tick += 1;
    resumed.state.tick += 1;
    engine.advance_trains();
    resumed.advance_trains();
    assert_eq!(engine.snapshot().trains[0].door_open_fraction, 0.0);
    assert_eq!(engine.state.trains[0].distance, station_distance);
    assert_eq!(engine.save_json(), resumed.save_json());

    engine.state.tick += 1;
    resumed.state.tick += 1;
    engine.advance_trains();
    resumed.advance_trains();
    assert!(engine.state.trains[0].distance > station_distance);
    assert_eq!(engine.save_json(), resumed.save_json());
    assert_eq!(engine.state.trains[0].id, train_id);
}

#[test]
fn train_door_buffer_matches_snapshot_metadata_and_is_bounded() {
    let (mut engine, station, gateway) = rail_test_engine(86);
    let train_id = engine.id();
    engine.state.trains.push(Train {
        id: train_id,
        route: vec![station, gateway],
        distance: TRAIN_HALF_LENGTH,
        speed: 8.0,
        direction: 1,
        dwell_ticks: TRAIN_STATION_DWELL_TICKS,
        passengers: Vec::new(),
    });
    let assert_matches_snapshot = |engine: &Engine| {
        let doors = engine.train_doors();
        let snapshot = engine.snapshot();
        assert_eq!(
            doors.len(),
            snapshot.trains.len().min(MAX_PRESENTED_TRAINS) * 3
        );
        for (index, train) in snapshot
            .trains
            .iter()
            .take(MAX_PRESENTED_TRAINS)
            .enumerate()
        {
            assert_eq!(doors[index * 3], train.id as f32);
            assert_eq!(
                doors[index * 3 + 1],
                train.door_terminal.map(|id| id as f32).unwrap_or(-1.0)
            );
            assert_eq!(doors[index * 3 + 2], train.door_open_fraction);
        }
    };
    assert_matches_snapshot(&engine);
    let mut restored = Engine::restore_json(&engine.save_json()).unwrap();
    for _ in 0..TRAIN_STATION_DWELL_TICKS + 5 {
        engine.step(1);
        restored.step(1);
        assert_matches_snapshot(&engine);
        assert_eq!(engine.train_doors(), restored.train_doors());
    }
    engine.state.trains[0].dwell_ticks = TRAIN_DOOR_PHASE_TICKS;
    assert_matches_snapshot(&engine);
    let mut restored = Engine::restore_json(&engine.save_json()).unwrap();
    assert_matches_snapshot(&restored);
    engine.state.trains[0].dwell_ticks = 0;
    restored.state.trains[0].dwell_ticks = 0;
    assert_matches_snapshot(&engine);
    assert_matches_snapshot(&restored);
}

#[test]
fn rail_service_holds_at_station_during_quiet_hours_and_reopens_at_dawn() {
    let (mut engine, station, gateway) = rail_test_engine(84);
    engine.state.tick = 19_500; // 01:30 city time
    engine.advance_trains();
    let train = &engine.state.trains[0];
    assert_eq!(train.distance, TRAIN_HALF_LENGTH);
    assert_eq!(train.dwell_ticks, TRAIN_STATION_DWELL_TICKS);

    let resident_id = add_rail_test_resident(&mut engine, gateway);
    engine.schedule_residents(0);
    assert_eq!(
        engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .unwrap()
            .transfer,
        "home",
        "a new commute should wait at home during quiet hours"
    );

    engine.state.tick = 23_000; // 05:00 city time
    engine.schedule_residents(0);
    let commuter = engine
        .state
        .residents
        .iter()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    assert_eq!(commuter.transfer, "walking_to_station");
    assert!(
        commuter.home_node == station
            || engine.route(commuter.home_node, station, false).len() >= 2
    );
}

#[test]
fn rail_transfer_boards_to_capacity_and_keeps_platform_queue_visible_and_bounded() {
    let (mut engine, _station, gateway) = rail_test_engine(82);
    for _ in 0..RAIL_TRAIN_CAPACITY + 4 {
        let id = add_rail_test_resident(&mut engine, gateway);
        let resident = engine
            .state
            .residents
            .iter_mut()
            .find(|resident| resident.id == id)
            .unwrap();
        resident.transfer = "waiting_station".into();
        resident.activity = "waiting_for_train".into();
    }
    engine.advance_trains();
    engine.state.trains[0].dwell_ticks = TRAIN_STATION_DWELL_TICKS;
    for _ in 0..208 {
        engine.state.tick += 1;
        engine.advance_trains();
    }
    let train = &engine.state.trains[0];
    assert_eq!(train.passengers.len(), RAIL_TRAIN_CAPACITY);
    assert_eq!(engine.rail_waiting_count(true), 4);
    assert!(engine.rail_waiting_count(true) <= RAIL_QUEUE_CAPACITY);
    assert!(engine.movement_buffer().len() >= (4 + 1) * 6);
    engine.validate().unwrap();
    let save = engine.save_json();
    assert_eq!(Engine::restore_json(&save).unwrap().save_json(), save);
}

#[test]
fn named_rail_commuter_walks_boards_works_returns_and_replays_from_a_save() {
    let (mut engine, station, gateway) = rail_test_engine(83);
    engine.state.development_rate = 0;
    let resident_id = add_rail_test_resident(&mut engine, gateway);
    let home_node = engine
        .state
        .residents
        .iter()
        .find(|resident| resident.id == resident_id)
        .unwrap()
        .home_node;
    assert!(home_node == station || engine.route(home_node, station, false).len() >= 2);

    let mut saved_onboard = None;
    for _ in 0..5_000 {
        engine.step_one();
        if engine
            .state
            .residents
            .iter()
            .any(|resident| resident.id == resident_id && resident.train_id.is_some())
        {
            saved_onboard = Some(engine.save_json());
            break;
        }
    }
    let save = saved_onboard.expect("resident should board a train within the commute window");
    let mut replay = Engine::restore_json(&save).unwrap();
    assert!(replay
        .state
        .trains
        .iter()
        .any(|train| train.passengers.contains(&resident_id)));
    for _ in 0..6_000 {
        engine.step_one();
        replay.step_one();
    }
    assert_eq!(engine.save_json(), replay.save_json());
    let resident = engine
        .state
        .residents
        .iter()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    assert_eq!(resident.transfer, "home");
    assert!(resident
        .history
        .iter()
        .any(|entry| entry.contains("Boarded")));
    assert!(resident
        .history
        .iter()
        .any(|entry| entry.contains("walking home")));
    assert!(engine
        .state
        .events
        .iter()
        .any(|event| event.kind == "RailWorkStarted"));
    assert!(engine
        .state
        .events
        .iter()
        .any(|event| event.kind == "TrainPassengerAlighted"));
    let inspected: serde_json::Value =
        serde_json::from_str(&engine.inspect_json(resident_id)).unwrap();
    assert_eq!(inspected["entityType"], "resident");
    assert_eq!(inspected["transfer"], "home");
    engine.validate().unwrap();
}

#[test]
fn station_queue_positions_are_rail_aligned_reserved_and_continuous() {
    let (mut engine, _station, gateway) = rail_test_engine(48);
    engine.state.development_rate = 0;
    let first_id = add_rail_test_resident(&mut engine, gateway);
    let first_index = engine
        .state
        .residents
        .iter()
        .position(|resident| resident.id == first_id)
        .unwrap();
    engine.schedule_rail_resident(first_index, 0);
    let slot = engine.state.residents[first_index].queue_slot.unwrap();
    let platform = engine.platform_position(true, slot).unwrap();
    let entry = engine
        .state
        .travelers
        .iter()
        .find(|traveler| traveler.id == first_id)
        .unwrap()
        .destination_access
        .clone()
        .expect("station trip needs a platform entry connector");
    assert_eq!(
        entry.surface_y,
        vec![-1.0, 0.27, 0.27, 0.27, 0.27, 8.0, 8.0, 8.0]
    );
    let mut corrupt: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    let actor = corrupt["travelers"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"].as_u64() == Some(first_id as u64))
        .unwrap();
    let x = actor["destination_access"]["points"][1]["x"]
        .as_f64()
        .unwrap();
    actor["destination_access"]["points"][1]["x"] = serde_json::json!(x + 0.25);
    assert!(Engine::restore_json(&corrupt.to_string())
        .err()
        .unwrap()
        .contains("rail access geometry"));
    let mut mid_access = Engine::restore_json(&engine.save_json()).unwrap();
    let stair_progress = entry.points[..5]
        .windows(2)
        .map(|p| (p[1].x - p[0].x).hypot(p[1].z - p[0].z))
        .sum::<f32>()
        + (entry.points[5].x - entry.points[4].x).hypot(entry.points[5].z - entry.points[4].z)
            * 0.5;
    let traveler = mid_access
        .state
        .travelers
        .iter_mut()
        .find(|traveler| traveler.id == first_id)
        .unwrap();
    traveler.access_phase = 2;
    traveler.access_progress = stair_progress;
    let mid_access_save = mid_access.save_json();
    let mid_access_movement = mid_access.movement_buffer();
    let resumed_mid_access = Engine::restore_json(&mid_access_save).unwrap();
    assert_eq!(resumed_mid_access.movement_buffer(), mid_access_movement);
    assert!((entry.points.last().unwrap().x - platform.x).abs() < 0.01);
    assert!((entry.points.last().unwrap().z - platform.z).abs() < 0.01);
    for _ in 0..4_000 {
        engine.state.tick += 1;
        engine.advance_travelers();
        if engine.state.residents[first_index].transfer == "waiting_station" {
            break;
        }
    }
    assert_eq!(
        engine.state.residents[first_index].transfer,
        "waiting_station"
    );
    let position_for = |engine: &Engine, id: u32| {
        engine
            .movement_buffer()
            .chunks_exact(6)
            .find(|actor| actor[0] as u32 == id)
            .map(|actor| WorldPoint {
                x: actor[1],
                z: actor[2],
            })
            .unwrap()
    };
    let waiting_position = position_for(&engine, first_id);
    assert!((waiting_position.x - platform.x).abs() < 0.01);
    assert!((waiting_position.z - platform.z).abs() < 0.01);
    let waiting_movement = engine.movement_buffer();
    let waiting_record = waiting_movement
        .chunks_exact(6)
        .find(|record| record[0] as u32 == first_id)
        .unwrap();
    assert_eq!(waiting_record[5], 8.0);
    let second_id = add_rail_test_resident(&mut engine, gateway);
    let second_index = engine
        .state
        .residents
        .iter()
        .position(|resident| resident.id == second_id)
        .unwrap();
    engine.schedule_rail_resident(second_index, 0);
    assert_ne!(engine.state.residents[second_index].queue_slot, Some(slot));
    let still_waiting = position_for(&engine, first_id);
    assert!((still_waiting.x - waiting_position.x).abs() < 0.01);
    assert!((still_waiting.z - waiting_position.z).abs() < 0.01);
    engine.validate().unwrap();
}

#[test]
fn rail_platform_slots_fit_both_oriented_platforms() {
    let (engine, station, gateway) = rail_test_engine(48);
    let (_, _, ux, uz) = engine.rail_axis().unwrap();
    for (node_id, dx, dz) in [(station, ux, uz), (gateway, -ux, -uz)] {
        let node = engine.node(node_id).unwrap();
        let (nx, nz) = (dz, -dx);
        for slot in 0..RAIL_QUEUE_CAPACITY as u16 {
            let p = engine.platform_position(node_id == station, slot).unwrap();
            let local_x = (p.x - node.x) * nx + (p.z - node.z) * nz;
            let local_z = (p.x - node.x) * dx + (p.z - node.z) * dz;
            assert!((2.6..=7.8).contains(&local_x), "slot {slot} x={local_x}");
            assert!(
                (-1.0..=RAIL_PLATFORM_END).contains(&local_z),
                "slot {slot} z={local_z}"
            );
            assert!(
                local_x + 0.45 <= 5.125,
                "slot {slot} intersects bench clearance"
            );
        }
    }
    let views = engine.rail_platform_views();
    assert_eq!(views.len(), 2);
    assert!(views
        .iter()
        .all(|view| view.surface_y == 8.0 && view.stair_steps == 44));
}

#[test]
fn rail_access_connectors_follow_rotated_and_translated_axes() {
    let (mut engine, station, gateway) = rail_test_engine(49);
    let route = vec![station, gateway];
    for (angle, origin) in [
        (0.0_f32, WorldPoint { x: 30.0, z: -60.0 }),
        (
            std::f32::consts::FRAC_PI_2,
            WorldPoint { x: -80.0, z: 45.0 },
        ),
        (std::f32::consts::PI, WorldPoint { x: 100.0, z: 120.0 }),
        (
            -std::f32::consts::FRAC_PI_2,
            WorldPoint {
                x: -120.0,
                z: -100.0,
            },
        ),
        (
            0.63,
            WorldPoint {
                x: 150.0,
                z: -170.0,
            },
        ),
    ] {
        let end = WorldPoint {
            x: origin.x + angle.cos() * 300.0,
            z: origin.z + angle.sin() * 300.0,
        };
        for node in &mut engine.state.nodes {
            if node.id == station {
                node.x = origin.x;
                node.z = origin.z;
            }
            if node.id == gateway {
                node.x = end.x;
                node.z = end.z;
            }
        }
        for at_station in [true, false] {
            let terminal = if at_station { station } else { gateway };
            let queue = engine.platform_position(at_station, 95).unwrap();
            let path = engine
                .platform_entry_path(&route, at_station, 95, 900_001)
                .unwrap();
            let exit = engine
                .platform_exit_path(&route, at_station, 95, 900_001)
                .unwrap();
            assert_eq!(path.points.len(), 8);
            assert_eq!(exit.points.len(), 8);
            assert!((path.points[7].x - queue.x).hypot(path.points[7].z - queue.z) < 0.01);
            assert!((exit.points[0].x - queue.x).hypot(exit.points[0].z - queue.z) < 0.01);
            let axis = engine.rail_axis().unwrap();
            let (ux, uz) = if at_station {
                (axis.2, axis.3)
            } else {
                (-axis.2, -axis.3)
            };
            let terminal_point = engine.node(terminal).unwrap();
            let (nx, nz) = (uz, -ux);
            for p in path.points.iter().skip(1).chain(exit.points.iter().take(7)) {
                let lx = (p.x - terminal_point.x) * nx + (p.z - terminal_point.z) * nz;
                let lz = (p.x - terminal_point.x) * ux + (p.z - terminal_point.z) * uz;
                assert!(
                    (if at_station {
                        -21.51..=RAIL_PLATFORM_END + 0.01
                    } else {
                        -1.51..=RAIL_GATEWAY_STAIR_BOTTOM + 1.51
                    })
                    .contains(&lz),
                    "rotated connector point local z={lz}"
                );
                assert!(
                    (3.39..=7.51).contains(&lx),
                    "rotated connector point local x={lx}"
                );
            }
        }
    }
}

#[test]
fn mature_neighborhoods_keep_low_rise_core_and_tall_station_cluster() {
    let mut signatures = BTreeSet::new();
    for seed in 0..6 {
        let mut engine = Engine::new_legacy(seed);
        engine.set_development_rate(60);
        for _ in 0..36 {
            engine.step(2_000);
        }
        let station = engine
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "station")
            .unwrap();
        let market = engine
            .state
            .nodes
            .iter()
            .find(|node| node.kind == "market")
            .unwrap();
        let open: Vec<&Building> = engine
            .state
            .buildings
            .iter()
            .filter(|building| building.stage == "open")
            .collect();
        let station_core: Vec<_> = open
            .iter()
            .copied()
            .filter(|building| (building.x - station.x).hypot(building.z - station.z) <= 112.0)
            .collect();
        let station_towers = station_core
            .iter()
            .filter(|building| building.floors >= 20)
            .count();
        let historic_core: Vec<_> = open
            .iter()
            .copied()
            .filter(|building| {
                (building.x - market.x).hypot(building.z - market.z) <= 118.0
                    && (building.x - station.x).hypot(building.z - station.z) > 112.0
            })
            .collect();
        let outer: Vec<_> = open
            .iter()
            .copied()
            .filter(|building| {
                (building.x - station.x).hypot(building.z - station.z) > 205.0
                    && (building.x - market.x).hypot(building.z - market.z) > 118.0
            })
            .collect();
        let share = |buildings: &[&Building], predicate: &dyn Fn(&Building) -> bool| {
            buildings
                .iter()
                .filter(|building| predicate(building))
                .count() as f32
                / buildings.len().max(1) as f32
        };
        let mean_floors = |buildings: &[&Building]| {
            buildings
                .iter()
                .map(|building| building.floors as f32)
                .sum::<f32>()
                / buildings.len().max(1) as f32
        };
        assert!(
            station_core.len() >= 12,
            "seed {seed} lacks a station district"
        );
        assert!(
            historic_core.len() >= 8,
            "seed {seed} lacks a founding core"
        );
        assert!(outer.len() >= 80, "seed {seed} lacks outer neighborhoods");
        assert!(
            share(&station_core, &|building| building.floors >= 5) >= 0.45,
            "seed {seed} station district did not form a taller cluster"
        );
        assert!(
            mean_floors(&station_core) >= mean_floors(&outer) + 1.0,
            "seed {seed} has no height gradient between station and outer neighborhoods"
        );
        assert!(
            share(&historic_core, &|building| building.floors <= 3) >= 0.72,
            "seed {seed} lost its low-rise founding streets"
        );
        assert!(
            historic_core
                .iter()
                .any(|building| building.kind == "shop" || building.kind == "cafe"),
            "seed {seed} lost everyday commerce from its founding core"
        );
        assert!(
            share(&outer, &|building| building.floors <= 3) >= 0.68,
            "seed {seed} outer neighborhoods became uniformly mid-rise"
        );
        let kinds: BTreeSet<&str> = open.iter().map(|building| building.kind.as_str()).collect();
        assert!(kinds.len() >= 4, "seed {seed} lost mixed neighborhood uses");
        println!(
            "district mix seed={seed} buildings={} historic={} lowrise={:.0}% station={} tall={:.0}% towers={} mean_floors={:.2} outer={} lowrise={:.0}% mean_floors={:.2} uses={:?}",
            open.len(),
            historic_core.len(),
            share(&historic_core, &|building| building.floors <= 3) * 100.0,
            station_core.len(),
            share(&station_core, &|building| building.floors >= 5) * 100.0,
            station_towers,
            mean_floors(&station_core),
            outer.len(),
            share(&outer, &|building| building.floors <= 3) * 100.0,
            mean_floors(&outer),
            kinds,
        );
        signatures.insert((
            station_core.len(),
            historic_core.len(),
            outer.len(),
            open.iter().filter(|building| building.floors >= 5).count(),
        ));
        engine.validate().unwrap();
    }
    assert!(
        signatures.len() > 1,
        "different seeds produced one fixed district mix"
    );
}

#[test]
fn seed_87231_builds_bounded_station_towers_on_connected_large_lots() {
    let mut engine = Engine::new_legacy(87_231);
    engine.set_development_rate(16);
    for _ in 0..121 {
        engine.step(2_000);
    }
    engine.validate().unwrap();
    let station = engine
        .state
        .nodes
        .iter()
        .find(|node| node.kind == "station")
        .unwrap();
    let towers: Vec<_> = engine
        .state
        .buildings
        .iter()
        .filter(|building| {
            building.stage == "open"
                && building.floors >= 20
                && (building.x - station.x).hypot(building.z - station.z) <= 112.0
        })
        .collect();
    let snapshot = engine.snapshot();
    for tower in &towers {
        let parcel = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == tower.parcel_id)
            .unwrap();
        assert!(parcel.width >= STATION_TOWER_PARCEL_WIDTH);
        assert!(parcel.depth >= STATION_TOWER_PARCEL_DEPTH);
        assert!(engine.node_connected(parcel.access_node));
        assert!(snapshot
            .building_access
            .iter()
            .any(|access| access.building_id == tower.id));
        assert!(matches!(tower.kind.as_str(), "office" | "apartment"));
        assert_eq!(tower.width, parcel.width);
        assert_eq!(tower.depth, parcel.depth);
    }
    let founder = engine
        .state
        .buildings
        .iter()
        .find(|building| building.anchor && building.name == "Kissa Aoba")
        .expect("the founding shop remains a protected low-rise anchor");
    assert!(founder.floors <= 3);
    let midrise = engine
        .state
        .buildings
        .iter()
        .filter(|building| {
            let distance = (building.x - station.x).hypot(building.z - station.z);
            building.stage == "open" && (82.0..205.0).contains(&distance)
        })
        .filter(|building| (5..=12).contains(&building.floors))
        .count();
    let mut candidate_sites = 0usize;
    let mut geometry_valid_sites = 0usize;
    let mut demand_ready_sites = 0usize;
    for road in engine.state.roads.iter().filter(|road| {
        road.stage == "open" && road.connected && !matches!(road.kind.as_str(), "rail" | "bridge")
    }) {
        let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
            continue;
        };
        for (x, z, _, _, _) in engine.frontage_candidate_sites(a, b) {
            candidate_sites += 1;
            if !engine.parcel_geometry_clear(x, z, 14.0, 12.0) {
                continue;
            }
            geometry_valid_sites += 1;
            let mut local_open = 0usize;
            let mut local_occupied = 0usize;
            for building in engine.state.buildings.iter().filter(|building| {
                building.stage == "open" && (building.x - x).hypot(building.z - z) <= 78.0
            }) {
                local_open += 1;
                local_occupied += usize::from(site_active(building));
            }
            if local_open >= 4 && local_occupied >= 3 && engine.state.demand >= 1.2 {
                demand_ready_sites += 1;
            }
        }
    }
    let building_footprint: f32 = engine
        .state
        .buildings
        .iter()
        .filter(|building| building.stage != "demolished")
        .map(|building| building.width * building.depth)
        .sum();
    let planned_segment_count = engine
        .state
        .neighborhood_plan
        .as_ref()
        .map(|plan| {
            plan.segments
                .iter()
                .filter(|(from, to)| {
                    engine.state.roads.iter().any(|road| {
                        road.stage == "open"
                            && ((road.from == *from && road.to == *to)
                                || (road.from == *to && road.to == *from))
                    })
                })
                .count()
        })
        .unwrap_or(0);
    let mut historic_graph: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for road in engine
        .state
        .roads
        .iter()
        .filter(|road| engine.state.neighborhood_road_ids.contains(&road.id))
    {
        historic_graph.entry(road.from).or_default().insert(road.to);
        historic_graph.entry(road.to).or_default().insert(road.from);
    }
    let mut remaining_nodes: BTreeSet<u32> = historic_graph.keys().copied().collect();
    let mut completed_neighborhood_blocks = 0usize;
    while let Some(root) = remaining_nodes.iter().next().copied() {
        let mut component = BTreeSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !component.insert(node) {
                continue;
            }
            remaining_nodes.remove(&node);
            if let Some(neighbors) = historic_graph.get(&node) {
                stack.extend(neighbors.iter().copied());
            }
        }
        let degree_sum: usize = component
            .iter()
            .filter_map(|node| historic_graph.get(node))
            .map(BTreeSet::len)
            .sum();
        let edges = degree_sum / 2;
        completed_neighborhood_blocks += edges.saturating_add(1).saturating_sub(component.len());
    }
    let mut planned_road_ids = engine.state.neighborhood_road_ids.clone();
    if let Some(plan) = &engine.state.neighborhood_plan {
        for (from, to) in &plan.segments {
            if let Some(road) = engine.state.roads.iter().find(|road| {
                (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
            }) {
                planned_road_ids.insert(road.id);
            }
        }
    }
    let planned_parcel_ids: BTreeSet<u32> = engine
        .state
        .parcel_frontages
        .iter()
        .filter(|frontage| planned_road_ids.contains(&frontage.road_id))
        .map(|frontage| frontage.parcel_id)
        .collect();
    let planned_buildings: Vec<_> = engine
        .state
        .buildings
        .iter()
        .filter(|building| planned_parcel_ids.contains(&building.parcel_id))
        .collect();
    let planned_residential = planned_buildings
        .iter()
        .filter(|building| matches!(building.kind.as_str(), "home" | "apartment"))
        .count();
    let planned_residential_occupied: Vec<_> = planned_buildings
        .iter()
        .filter(|building| {
            matches!(building.kind.as_str(), "home" | "apartment")
                && building.stage == "open"
                && building.occupancy > 0
        })
        .collect();
    let planned_commercial = planned_buildings
        .iter()
        .filter(|building| !matches!(building.kind.as_str(), "home" | "apartment"))
        .count();
    let planned_parcel_area: f32 = engine
        .state
        .parcels
        .iter()
        .filter(|parcel| planned_parcel_ids.contains(&parcel.id))
        .map(|parcel| parcel.width * parcel.depth)
        .sum();
    let planned_occupied_footprint: f32 = planned_residential_occupied
        .iter()
        .map(|building| building.width * building.depth)
        .sum();
    println!(
        "station skyline seed=87231 tick={} buildings={} parcels={} towers={} midrise_fringe={} footprint_m2={:.0} coverage={:.1}% frontage_candidates={} geometry_valid={} demand_ready={} stage={} completed_blocks={} block_plan={}/4 planned_lots={} plan_parcel_area_m2={:.0} planned_buildings={} plan_residential={} plan_commercial={} plan_occupied_residences={} plan_occupied_footprint_m2={:.0}",
        engine.state.tick,
        engine.state.buildings.len(),
        engine.state.parcels.len(),
        towers.len(),
        midrise,
        building_footprint,
        building_footprint / 640_000.0 * 100.0,
        candidate_sites,
        geometry_valid_sites,
        demand_ready_sites,
        snapshot.stage,
        completed_neighborhood_blocks,
        planned_segment_count,
        planned_parcel_ids.len(),
        planned_parcel_area,
        planned_buildings.len(),
        planned_residential,
        planned_commercial,
        planned_residential_occupied.len(),
        planned_occupied_footprint,
    );
    assert!(
        !planned_residential_occupied.is_empty(),
        "block frontage added no occupied residential stock"
    );
    assert!(
        towers.len() >= 2,
        "expected a compact cluster of real towers"
    );
    assert!(
        completed_neighborhood_blocks >= 2,
        "seed 87231 formed only {completed_neighborhood_blocks} completed compact blocks by tick {}",
        engine.state.tick
    );
    assert!(engine.state.neighborhood_road_ids.iter().all(|id| {
        engine
            .state
            .roads
            .iter()
            .any(|road| road.id == *id && road.stage == "open" && road.connected)
    }));
    assert!(
        midrise >= 8,
        "station fringe lacks 5–12 storey midrise fabric"
    );
    assert_eq!(
        engine.save_json(),
        Engine::restore_json(&engine.save_json())
            .unwrap()
            .save_json()
    );
}

#[test]
fn station_tower_frontages_are_reserved_before_rail_and_small_lots() {
    let mut engine = Engine::new_legacy(31415);
    engine.state.nodes.clear();
    engine.state.roads.clear();
    engine.state.parcels.clear();
    engine.state.parcel_frontages.clear();
    engine.state.buildings.clear();
    engine.state.station_online = false;
    let station = engine.add_node(0.0, 200.0, "station");
    let left_a = engine.add_node(-40.0, 150.0, "junction");
    let left_b = engine.add_node(40.0, 150.0, "junction");
    let right_a = engine.add_node(-40.0, 250.0, "junction");
    let right_b = engine.add_node(40.0, 250.0, "junction");
    let lower = engine.add_road(left_a, left_b, "local", "open", 1.0);
    let upper = engine.add_road(right_a, right_b, "local", "open", 1.0);
    let growth_rng = engine.state.rng_growth;

    engine.add_frontage_parcels(lower);
    assert_eq!(engine.station_tower_frontage_reservations(), 1);
    let first_reservation = engine
        .state
        .parcels
        .iter()
        .find(|parcel| parcel.width >= STATION_TOWER_PARCEL_WIDTH)
        .unwrap();
    assert!(engine
        .state
        .parcels
        .iter()
        .filter(|parcel| parcel.width < STATION_TOWER_PARCEL_WIDTH)
        .all(|parcel| {
            (parcel.x - first_reservation.x).abs()
                >= (parcel.width + first_reservation.width) * 0.5 + 2.0
                || (parcel.z - first_reservation.z).abs()
                    >= (parcel.depth + first_reservation.depth) * 0.5 + 2.0
        }));
    engine.add_frontage_parcels(upper);

    let reservations: Vec<_> = engine
        .state
        .parcels
        .iter()
        .filter(|parcel| {
            parcel.width >= STATION_TOWER_PARCEL_WIDTH && parcel.depth >= STATION_TOWER_PARCEL_DEPTH
        })
        .collect();
    assert_eq!(reservations.len(), MIN_STATION_TOWER_FRONTAGE_RESERVATIONS);
    assert_eq!(engine.station_tower_frontage_reservations(), 2);
    assert!(engine.state.parcel_frontages.iter().any(|frontage| {
        frontage.road_id == lower
            && reservations
                .iter()
                .any(|parcel| parcel.id == frontage.parcel_id)
    }));
    assert!(engine.state.parcel_frontages.iter().any(|frontage| {
        frontage.road_id == upper
            && reservations
                .iter()
                .any(|parcel| parcel.id == frontage.parcel_id)
    }));
    assert!(engine.state.buildings.is_empty());
    assert_eq!(engine.state.rng_growth, growth_rng);
    assert_eq!(
        engine.save_json(),
        Engine::restore_json(&engine.save_json())
            .unwrap()
            .save_json()
    );
    assert_eq!(station, engine.state.nodes[0].id);
}
