use super::*;

#[test]
fn bus_passenger_waits_for_disconnected_walk_route_and_resumes_after_save() {
    let (mut engine, ids) = bus_test_engine(1);
    let resident_id = ids[0];
    let line = engine.state.bus_line.as_ref().unwrap().clone();
    let stop = *line.stops.last().unwrap();
    let (stop_x, stop_z) = {
        let stop_position = engine.node(stop).unwrap();
        (stop_position.x, stop_position.z)
    };
    let target = engine.add_node(stop_x + 18.0, stop_z - 18.0, NodeKind::Junction);
    let bus_id = engine.state.buses[0].id;
    let resident = engine
        .state
        .residents
        .iter_mut()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    resident.transfer = "riding_bus".into();
    resident.bus_id = Some(bus_id);
    resident.bus_origin_stop = Some(line.stops[0]);
    resident.bus_destination_stop = Some(stop);
    resident.bus_stop_node = Some(stop);
    resident.queue_slot = Some(2);
    resident.work_node = Some(target);
    engine.state.buses[0].direction = 1;
    engine.state.buses[0].passengers = vec![resident_id];

    assert!(engine.route(stop, target, false).is_empty());
    engine.alight_bus_passenger(0, resident_id, stop);
    let waiting = engine
        .state
        .residents
        .iter()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    assert_eq!(waiting.transfer, "waiting_alight");
    assert_eq!(waiting.pending_alight.as_ref().unwrap().stop_node, stop);
    assert_eq!(waiting.bus_id, None);
    assert!(engine.state.buses[0].passengers.is_empty());
    assert_eq!(engine.resident_location_node(waiting), Some(stop));
    let saved_position = waiting.pending_alight.as_ref().unwrap().wait_position;
    let failed_revision = waiting
        .pending_alight
        .as_ref()
        .unwrap()
        .last_checked_geometry_revision;
    assert_eq!(failed_revision, Some(engine.state.access_geometry_revision));
    engine.advance_pending_alights();
    assert_eq!(
        engine.state.residents[0]
            .pending_alight
            .as_ref()
            .unwrap()
            .last_checked_geometry_revision,
        failed_revision,
        "the failed alight must not route-search again on an unchanged graph"
    );
    assert!(engine
        .state
        .travelers
        .iter()
        .all(|traveler| traveler.resident != resident_id));
    let movement_before_save = engine.movement_buffer();
    let waiting_movement = movement_before_save
        .chunks_exact(6)
        .find(|movement| movement[0] == resident_id as f32)
        .expect("pending alighter is visible in movement output");
    assert_eq!(waiting_movement[1], saved_position.x);
    assert_eq!(waiting_movement[2], saved_position.z);
    engine.validate().unwrap();

    engine.set_development_rate(0);
    let save = engine.save_json();
    let mut restored = Engine::restore_json(&save).unwrap();
    assert_eq!(restored.save_json(), save);
    assert_eq!(restored.movement_buffer(), movement_before_save);
    restored.step(80);
    assert_eq!(restored.save_json(), {
        let mut control = Engine::restore_json(&save).unwrap();
        control.step(80);
        control.save_json()
    });
    let pending = restored.state.residents[0].pending_alight.as_ref().unwrap();
    assert!(pending.last_checked_geometry_revision.is_some());
    assert!(pending.retry_after_tick > restored.state.tick);
    assert!(restored.route(stop, target, false).is_empty());

    restored.add_road(stop, target, RoadKind::Local, Stage::Open, 1.0);
    restored.recompute_connectivity();
    assert!(!restored.route(stop, target, false).is_empty());
    restored.step(1);
    let resumed = restored
        .state
        .travelers
        .iter()
        .find(|traveler| traveler.resident == resident_id)
        .expect("legal route should resume the resident's walk");
    assert_eq!(resumed.activity, "commute");
    assert_eq!(resumed.access_phase, 0);
    let origin_access = resumed.origin_access.as_ref().unwrap();
    assert_eq!(origin_access.points[0].x, saved_position.x);
    assert_eq!(origin_access.points[0].z, saved_position.z);
    assert_eq!(resumed.edge_progress, 0.0);
    assert!(restored.state.residents[0].pending_alight.is_none());
    assert_eq!(restored.state.residents[0].transfer, "walking_from_bus");
    assert_eq!(restored.state.buses[0].passengers.len(), 0);
    restored.validate().unwrap();
}

#[test]
fn pending_alight_scheduler_checks_at_most_eight_waiters_per_tick() {
    let (mut engine, ids) = bus_test_engine(10);
    let line = engine.state.bus_line.as_ref().unwrap().clone();
    let stop = line.stops[0];
    let stop_position = engine.node(stop).unwrap();
    let target = engine.add_node(
        stop_position.x + 18.0,
        stop_position.z - 18.0,
        NodeKind::Junction,
    );
    let building = engine.state.residents[0].home_building;
    for id in ids {
        let resident = engine
            .state
            .residents
            .iter_mut()
            .find(|resident| resident.id == id)
            .unwrap();
        resident.work_node = Some(target);
        let position = engine
            .node(stop)
            .map(|node| WorldPoint {
                x: node.x,
                z: node.z,
            })
            .unwrap();
        assert!(engine.queue_pending_alight(
            id,
            stop,
            target,
            building,
            "commute",
            "walking_from_bus",
            position,
            false,
        ));
    }
    engine.state.tick = 40;
    engine.advance_pending_alights();
    let checked = engine
        .state
        .residents
        .iter()
        .filter(|resident| {
            resident
                .pending_alight
                .as_ref()
                .is_some_and(|pending| pending.last_checked_geometry_revision.is_some())
        })
        .count();
    assert_eq!(checked, 8);
    assert_eq!(engine.state.pending_alight_ids.len(), 10);
    engine.validate().unwrap();
}

#[test]
fn pending_alight_retries_when_destination_opens_without_geometry_change() {
    let (mut engine, ids) = bus_test_engine(1);
    let resident_id = ids[0];
    let stop = *engine
        .state
        .bus_line
        .as_ref()
        .unwrap()
        .stops
        .last()
        .unwrap();
    let (stop_x, stop_z) = {
        let stop_position = engine.node(stop).unwrap();
        (stop_position.x, stop_position.z)
    };
    let target = engine.add_node(stop_x + 18.0, stop_z - 18.0, NodeKind::Junction);
    let building_id = engine.state.residents[0].home_building;
    engine.add_road(stop, target, RoadKind::Local, Stage::Open, 1.0);
    engine.recompute_connectivity();
    let position = WorldPoint {
        x: stop_x,
        z: stop_z,
    };
    let origin_stop = engine
        .state
        .bus_line
        .as_ref()
        .unwrap()
        .stops
        .first()
        .copied();
    {
        let resident = engine
            .state
            .residents
            .iter_mut()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        resident.bus_origin_stop = origin_stop;
        resident.bus_destination_stop = Some(stop);
    }
    assert!(engine.queue_pending_alight(
        resident_id,
        stop,
        target,
        building_id,
        "commute",
        "walking_from_bus",
        position,
        false,
    ));

    let building = engine
        .state
        .buildings
        .iter_mut()
        .find(|building| building.id == building_id)
        .unwrap();
    building.stage = Stage::Structure;
    let revision = engine.state.access_geometry_revision;
    engine.advance_pending_alights();
    let pending = engine.state.residents[0].pending_alight.as_ref().unwrap();
    assert_eq!(pending.last_checked_geometry_revision, None);
    assert_eq!(engine.state.access_geometry_revision, revision);
    assert!(engine
        .state
        .travelers
        .iter()
        .all(|traveler| traveler.resident != resident_id));

    engine
        .state
        .buildings
        .iter_mut()
        .find(|building| building.id == building_id)
        .unwrap()
        .stage = Stage::Open;
    engine.advance_pending_alights();
    assert!(engine.state.residents[0].pending_alight.is_none());
    assert!(engine
        .state
        .travelers
        .iter()
        .any(|traveler| traveler.resident == resident_id));
    engine.validate().unwrap();
}

#[test]
fn full_pending_alight_queue_keeps_bus_passenger_seated() {
    let (mut engine, ids) = bus_test_engine(1);
    let resident_id = ids[0];
    let line = engine.state.bus_line.as_ref().unwrap().clone();
    let stop = *line.stops.last().unwrap();
    let target = engine.add_node(340.0, -340.0, NodeKind::Junction);
    let bus_id = engine.state.buses[0].id;
    let resident = &mut engine.state.residents[0];
    resident.at_work = false;
    resident.work_node = Some(target);
    resident.transfer = "riding_bus".into();
    resident.bus_id = Some(bus_id);
    resident.bus_origin_stop = Some(line.stops[0]);
    resident.bus_destination_stop = Some(stop);
    engine.state.buses[0].passengers = vec![resident_id];
    engine.state.pending_alight_ids = (0..transit_recovery::MAX_PENDING_ALIGHT_WAITERS)
        .map(|index| 900_000 + index as u32)
        .collect();
    engine.state.pending_alight_cursor = 0;

    engine.alight_bus_passenger(0, resident_id, stop);
    assert_eq!(engine.state.buses[0].passengers, vec![resident_id]);
    assert_eq!(engine.state.residents[0].transfer, "riding_bus");
    assert_eq!(engine.state.residents[0].bus_id, Some(bus_id));
    assert!(engine.state.residents[0].pending_alight.is_none());
    assert!(engine.state.residents[0]
        .history
        .last()
        .unwrap()
        .contains("queue is full"));
}

#[test]
fn named_resident_walks_rides_alights_and_returns_by_bus_with_replay() {
    let (mut engine, ids) = bus_test_engine(1);
    let resident_id = ids[0];
    engine.schedule_residents(0);
    assert_eq!(engine.state.residents[0].transfer, "walking_to_bus");
    let mut saw_waiting = false;
    for _ in 0..2_000 {
        engine.step(1);
        let resident = engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        if resident.transfer == "waiting_bus" {
            saw_waiting = true;
            break;
        }
        if resident.transfer == "riding_bus" {
            saw_waiting = true;
            break;
        }
    }
    assert!(saw_waiting, "resident never reached the stop queue");
    for _ in 0..100 {
        if engine.state.residents[0].transfer == "riding_bus" {
            break;
        }
        engine.step(1);
    }
    assert_eq!(engine.state.residents[0].transfer, "riding_bus");
    assert_eq!(engine.state.buses[0].passengers, vec![resident_id]);
    assert!(!engine
        .state
        .travelers
        .iter()
        .any(|traveler| traveler.id == resident_id));
    engine.state.buses[0].dwell_ticks = 0;
    let save = engine.save_json();
    let mut restored = Engine::restore_json(&save).unwrap();
    assert_eq!(restored.save_json(), save);
    let mut saw_alight = false;
    for _ in 0..4_000 {
        engine.step(1);
        restored.step(1);
        assert_eq!(engine.save_json(), restored.save_json());
        if engine.state.residents[0].transfer == "walking_from_bus" {
            saw_alight = true;
            break;
        }
    }
    assert!(saw_alight, "bus never reached the rider's destination stop");
    for _ in 0..4_000 {
        engine.step(1);
        restored.step(1);
        assert_eq!(engine.save_json(), restored.save_json());
        if engine.state.residents[0].at_work {
            break;
        }
    }
    assert!(
        engine.state.residents[0].at_work,
        "rider did not reach the workplace door"
    );
    assert_eq!(engine.state.residents[0].transfer, "");
    engine.state.residents[0].return_tick = engine.state.tick;
    engine.schedule_residents(0);
    assert_eq!(engine.state.residents[0].transfer, "walking_to_bus");
    let mut boarded_return = false;
    let return_walker = engine
        .state
        .travelers
        .iter()
        .find(|traveler| traveler.resident == resident_id)
        .expect("return rider should have a connected walking leg");
    let access_length: f32 = return_walker
        .origin_access
        .as_ref()
        .map(|access| {
            access
                .points
                .windows(2)
                .map(|points| (points[1].x - points[0].x).hypot(points[1].z - points[0].z))
                .sum()
        })
        .unwrap_or(0.0);
    let walk_ticks = ((return_walker.route_lengths.iter().sum::<f32>() + access_length)
        * TICKS_PER_SECOND as f32
        / return_walker.speed.max(0.1))
    .ceil() as u64;
    let line = engine.state.bus_line.as_ref().unwrap();
    let route_length: f32 = engine.route_lengths(&line.route).iter().sum();
    let service_cycle_ticks = (route_length * 2.0 / (7.5 / TICKS_PER_SECOND as f32)).ceil() as u64
        + line.stops.len() as u64 * 2 * BUS_DWELL_TICKS as u64
        + BUS_HEADWAY_TICKS * 2
        + line.route.len() as u64 * 2 * 240;
    // Include the walk to the stop, a full route cycle, per-stop dwell,
    // terminal headway and a complete signal phase at each route junction.
    let return_boarding_ticks = walk_ticks + service_cycle_ticks;
    for _ in 0..return_boarding_ticks {
        engine.step(1);
        if engine.state.residents[0].transfer == "riding_bus" {
            boarded_return = true;
            break;
        }
    }
    assert!(
        boarded_return,
        "resident never boarded the return bus at tick{}: resident={:?}, buses={:?}, line={:?}, stops={:?}, vehicles={:?}",
        engine.state.tick,
        engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id),
        engine
            .state
            .buses
            .iter()
            .map(|bus| (
                bus.progress,
                bus.direction,
                bus.dwell_ticks,
                bus.last_stop,
                bus.passengers.clone()
            ))
            .collect::<Vec<_>>(),
        engine
            .state
            .bus_line
            .as_ref()
            .map(|line| (&line.route, &line.stops)),
        engine.state.events.iter().filter(|event| event.kind == EventKind::BusStop).map(|event| (event.tick, event.reason.clone())).collect::<Vec<_>>(),
        engine.state.travelers.iter().map(|traveler| (traveler.kind, &traveler.route, traveler.edge_index, traveler.edge_progress, &traveler.activity, traveler.wait_ticks)).collect::<Vec<_>>()
    );
    let return_save = engine.save_json();
    let mut return_restored = Engine::restore_json(&return_save).unwrap();
    assert_eq!(return_restored.save_json(), return_save);
    let mut returned = false;
    for _ in 0..10_000 {
        engine.step(1);
        return_restored.step(1);
        assert_eq!(engine.save_json(), return_restored.save_json());
        if !engine.state.residents[0].at_work && engine.state.residents[0].transfer.is_empty() {
            returned = true;
            break;
        }
    }
    assert!(returned, "resident did not return home by the bus line");
    engine.validate().unwrap();
}

#[test]
fn bus_capacity_keeps_overflow_queue_visible_and_bounded() {
    let (mut engine, ids) = bus_test_engine(BUS_CAPACITY + 6);
    engine.schedule_residents(0);
    for _ in 0..3_000 {
        engine.step(1);
        if engine.state.buses[0].passengers.len() == BUS_CAPACITY && engine.bus_waiting_count() == 6
        {
            break;
        }
    }
    assert_eq!(engine.state.buses[0].passengers.len(), BUS_CAPACITY);
    assert_eq!(engine.bus_waiting_count(), 6);
    assert!(engine.bus_waiting_count() <= BUS_QUEUE_CAPACITY);
    assert!(engine.state.buses[0]
        .passengers
        .iter()
        .all(|id| ids.contains(id)));
    assert!(engine.state.residents.iter().all(|resident| {
        resident.transfer != "riding_bus"
            || !engine
                .state
                .travelers
                .iter()
                .any(|traveler| traveler.id == resident.id)
    }));
    let waiting_movement_count = engine
        .movement_buffer()
        .chunks_exact(6)
        .filter(|movement| ids.iter().any(|id| movement[0] == *id as f32))
        .count();
    assert_eq!(
        waiting_movement_count, 6,
        "only the overflow queue should remain individually visible while riders are aboard"
    );
    engine.validate().unwrap();
    let save = engine.save_json();
    let mut restored = Engine::restore_json(&save).unwrap();
    engine.step(800);
    restored.step(800);
    assert_eq!(engine.save_json(), restored.save_json());
}

#[test]
fn quiet_hours_do_not_start_new_bus_rides_but_current_riders_finish() {
    let (mut engine, ids) = bus_test_engine(1);
    engine.schedule_residents(0);
    let resident_id = ids[0];
    engine
        .state
        .travelers
        .retain(|traveler| traveler.id != resident_id);
    let line = engine.state.bus_line.as_ref().unwrap().clone();
    let resident = engine
        .state
        .residents
        .iter_mut()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    resident.transfer = "waiting_bus".into();
    resident.bus_origin_stop = Some(line.stops[0]);
    resident.bus_destination_stop = Some(line.stops[2]);
    resident.bus_stop_node = resident.bus_origin_stop;
    resident.queue_slot = Some(0);
    engine.state.tick = 19_500; // 01:30, inside the quiet service window
    let start_stop = line.stops[0];
    engine.handle_bus_stop(0, start_stop, &line, true);
    assert_eq!(engine.state.residents[0].transfer, "waiting_bus");
    assert!(engine.state.buses[0].passengers.is_empty());

    let destination_stop = line.stops[2];
    engine.state.buses[0].passengers.push(resident_id);
    engine.state.buses[0].dwell_ticks = 0;
    engine.state.buses[0].direction = 1;
    engine.state.buses[0].progress = engine.bus_stop_distance(destination_stop).unwrap() - 0.2;
    let resident = engine
        .state
        .residents
        .iter_mut()
        .find(|resident| resident.id == resident_id)
        .unwrap();
    resident.transfer = "riding_bus".into();
    let bus_id = engine.state.buses[0].id;
    resident.bus_id = Some(bus_id);
    engine.advance_buses();
    assert_eq!(engine.state.residents[0].transfer, "walking_from_bus");
    assert!(engine.state.buses[0].passengers.is_empty());
}

#[test]
fn legacy_save_without_bus_keys_restores_with_empty_service() {
    let (engine, _) = bus_test_engine(1);
    let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    let state = legacy.as_object_mut().unwrap();
    state.remove("bus_line");
    state.remove("buses");
    state.remove("next_bus_evaluation_tick");
    if let Some(residents) = state
        .get_mut("residents")
        .and_then(serde_json::Value::as_array_mut)
    {
        for resident in residents {
            if let Some(object) = resident.as_object_mut() {
                object.remove("bus_id");
                object.remove("bus_origin_stop");
                object.remove("bus_destination_stop");
                object.remove("bus_stop_node");
            }
        }
    }
    let restored = Engine::restore_json(&legacy.to_string()).unwrap();
    assert!(restored.state.bus_line.is_none());
    assert!(restored.state.buses.is_empty());
    assert_eq!(restored.snapshot().buses.len(), 0);
}

#[test]
fn bus_movement_uses_kind_three_and_left_hand_lane() {
    let (engine, _) = bus_test_engine(0);
    let line = engine.state.bus_line.as_ref().unwrap();
    let bus = &engine.state.buses[0];
    let (x, z, heading) = engine.bus_pose(line, bus).unwrap();
    let movement = engine.movement_buffer();
    let actor = movement
        .chunks_exact(6)
        .find(|entry| entry[0] == bus.id as f32)
        .unwrap();
    assert_eq!(actor[4], 3.0);
    assert!((actor[1] - x).abs() < 0.001);
    assert!((actor[2] - z).abs() < 0.001);
    assert!((actor[3] - heading).abs() < 0.001);
    let route = &line.route;
    let a = engine.node(route[0]).unwrap();
    let b = engine.node(route[1]).unwrap();
    let normal = (-(b.z - a.z) / distance(a, b), (b.x - a.x) / distance(a, b));
    assert!((x - a.x - normal.0 * KERB_LANE).abs() < 0.001);
    assert!((z - a.z - normal.1 * KERB_LANE).abs() < 0.001);
    let views = engine.snapshot().bus_lines[0].stop_positions.clone();
    assert_eq!(views.len(), line.stops.len() * 2);
    assert!(views.iter().any(|view| view.direction == 1));
    assert!(views.iter().any(|view| view.direction == -1));
}

#[test]
fn dwelling_bus_uses_the_departure_curb_on_curved_stops_in_both_directions() {
    let (mut engine, _) = bus_test_engine(0);
    let route = engine.state.bus_line.as_ref().unwrap().route.clone();
    let positions = [
        (0.0, 0.0),
        (20.0, 0.0),
        (2.0, 2.0),
        (42.0, 14.0),
        (62.0, 14.0),
    ];
    for (node_id, (x, z)) in route.iter().zip(positions) {
        let node = engine
            .state
            .nodes
            .iter_mut()
            .find(|node| node.id == *node_id)
            .unwrap();
        node.x = x;
        node.z = z;
    }
    let stop = route[1];
    let line = engine.state.bus_line.as_mut().unwrap();
    line.stops = vec![route[0], stop, route[3]];
    let line = line.clone();
    let progress = distance(engine.node(route[0]).unwrap(), engine.node(stop).unwrap());
    engine.state.buses[0].progress = progress;
    engine.state.buses[0].dwell_ticks = 80;
    engine.state.buses[0].last_stop = Some(stop);

    for direction in [1_i8, -1_i8] {
        engine.state.buses[0].direction = direction;
        let view = engine
            .bus_stop_views(&line)
            .into_iter()
            .find(|view| view.node_id == stop && view.direction == direction)
            .unwrap();
        let (x, z, heading) = engine.bus_pose(&line, &engine.state.buses[0]).unwrap();
        let dx = heading.cos();
        let dz = heading.sin();
        let nx = -dz;
        let nz = dx;
        let node = engine.node(stop).unwrap();
        let bus_lateral = (x - node.x) * nx + (z - node.z) * nz;
        let marker_lateral = (view.x - node.x) * nx + (view.z - node.z) * nz;
        assert!((heading - view.heading).sin().abs() < 0.0001);
        assert!((bus_lateral - KERB_LANE).abs() < 0.001);
        assert!((marker_lateral - BUS_STOP_KERB).abs() < 0.001);
        assert!(bus_lateral * marker_lateral > 0.0);
        let queue = engine.bus_queue_position(stop, direction, 0).unwrap();
        let queue_lateral = (queue.x - node.x) * nx + (queue.z - node.z) * nz;
        assert!((queue_lateral - BUS_STOP_KERB).abs() < 0.001);
    }
}

#[test]
fn bus_stop_selection_keeps_fractional_forward_and_reverse_stops_pending() {
    let stops = [(6, 0.0), (5, 10.0), (4, 20.0)];
    for remainder in [0.0005_f32, 0.05, 0.125, 0.249] {
        let forward_progress = 10.0 - remainder;
        assert_eq!(
            Engine::next_bus_stop(&stops, 1, forward_progress, 20.0, 6, 4),
            (5, 10.0),
            "forward stop was dropped with {remainder}m remaining"
        );

        let reverse_progress = 10.0 + remainder;
        assert_eq!(
            Engine::next_bus_stop(&stops, -1, reverse_progress, 20.0, 6, 4),
            (5, 10.0),
            "reverse stop was dropped with {remainder}m remaining"
        );
    }
    assert_eq!(
        Engine::next_bus_stop(&stops, 1, 10.0, 20.0, 6, 4),
        (4, 20.0),
        "forward bus must skip its exact current stop"
    );
    assert_eq!(
        Engine::next_bus_stop(&stops, -1, 10.0, 20.0, 6, 4),
        (6, 0.0),
        "reverse bus must skip its exact current stop"
    );
}

#[test]
fn reverse_bus_obeys_signal_for_its_actual_outgoing_route_leg() {
    let (mut engine, _) = bus_test_engine(0);
    let line = engine.state.bus_line.as_ref().unwrap().clone();
    let lengths = engine.route_lengths(&line.route);
    let junction_segment = 1;
    let segment_offset: f32 = lengths[..junction_segment].iter().sum();
    let segment_length = lengths[junction_segment];
    let bus = &mut engine.state.buses[0];
    bus.progress = segment_offset + 3.5;
    bus.direction = -1;
    bus.dwell_ticks = 0;
    bus.last_stop = None;

    let from = line.route[junction_segment + 1];
    let junction = line.route[junction_segment];
    let outgoing = line.route[junction_segment - 1];
    assert!(Engine::signalized_state(&engine.state, junction));
    let (Some(junction_node), Some(outgoing_node)) = (engine.node(junction), engine.node(outgoing))
    else {
        panic!("test route nodes must exist");
    };
    let horizontal =
        (outgoing_node.x - junction_node.x).abs() >= (outgoing_node.z - junction_node.z).abs();
    let signal_base = junction as u64 * 37 + from as u64 * 11;
    // Pick a phase that is red for the reverse bus's westbound outgoing leg.
    let red_phase = if horizontal { 120 } else { 0 };
    engine.state.tick = (red_phase + 240 - signal_base % 240) % 240;
    let before = engine.state.buses[0].progress;
    engine.advance_buses();
    assert!(
        (engine.state.buses[0].progress - before).abs() < 0.0001,
        "reverse bus crossed its red stop line"
    );

    // The opposite phase is green, so the same bus can clear the junction.
    engine.state.tick = engine.state.tick.saturating_add(120);
    engine.advance_buses();
    assert!(
        engine.state.buses[0].progress < before,
        "reverse bus did not proceed on green"
    );
    assert!(segment_length > 3.5);
}

#[test]
fn cars_keep_a_following_gap_behind_a_stopped_bus() {
    let (mut engine, _) = bus_test_engine(0);
    let line = engine.state.bus_line.as_ref().unwrap().clone();
    let lengths = engine.route_lengths(&line.route);
    let offset: f32 = lengths[0];
    engine.state.buses[0].progress = offset + 17.0;
    engine.state.buses[0].dwell_ticks = 100;
    engine.state.buses[0].last_stop = Some(line.route[2]);
    let from = line.route[1];
    let to = line.route[2];
    let edge_length = lengths[1];
    engine.state.travelers.push(Traveler {
        id: 900_001,
        kind: 1,
        route: vec![from, to],
        route_lengths: vec![edge_length],
        edge_index: 0,
        edge_progress: 8.0,
        speed: 9.0,
        destination: to,
        activity: "test_bus_following".into(),
        resident: 0,
        name: "Following car".into(),
        origin_building: None,
        destination_building: None,
        wait_ticks: 0,
        origin_access: None,
        destination_access: None,
        access_phase: 1,
        access_progress: 0.0,
    });
    engine.advance_travelers();
    assert!(engine.state.travelers[0].edge_progress <= 8.001);

    engine.state.travelers.clear();
    engine.state.buses[0].progress = 5.0;
    engine.state.buses[0].dwell_ticks = 0;
    engine.state.travelers.push(Traveler {
        id: 900_002,
        kind: 1,
        route: vec![line.route[0], line.route[1]],
        route_lengths: vec![lengths[0]],
        edge_index: 0,
        edge_progress: 12.0,
        speed: 9.0,
        destination: line.route[1],
        activity: "leading_car".into(),
        resident: 0,
        name: "Leading car".into(),
        origin_building: None,
        destination_building: None,
        wait_ticks: 0,
        origin_access: None,
        destination_access: None,
        access_phase: 1,
        access_progress: 0.0,
    });
    engine.advance_buses();
    assert!(engine.state.buses[0].progress <= 5.001);
}

#[test]
fn following_bus_holds_its_position_without_being_forced_backward() {
    let (mut engine, _) = bus_test_engine(0);
    let line = engine.state.bus_line.as_ref().unwrap().clone();
    let mut leader = engine.state.buses[0].clone();
    leader.id = engine.id();
    leader.progress = 0.0;
    leader.direction = -1;
    leader.dwell_ticks = 0;
    leader.last_stop = None;
    engine.state.buses[0].progress = 10.0;
    engine.state.buses[0].direction = -1;
    engine.state.buses[0].dwell_ticks = 0;
    engine.state.buses[0].last_stop = None;
    engine.state.buses.push(leader);

    let before = engine.state.buses[0].progress;
    engine.advance_buses();
    assert!(engine.state.buses[0].progress <= before + 0.0001);
    assert!(engine.state.buses[0].progress >= before - 0.0001);
    assert_eq!(engine.state.buses[1].progress, 0.0);
    assert!(engine
        .state
        .bus_line
        .as_ref()
        .is_some_and(|current| current.id == line.id));
}

#[test]
fn seeded_neighborhood_bus_carries_residents_on_connected_roads() {
    let mut engine = Engine::new_legacy(0);
    engine.set_development_rate(60);
    for _ in 0..36 {
        engine.step(2_000);
    }
    let line = engine
        .state
        .bus_line
        .as_ref()
        .expect("mature seed should open a bus line");
    assert!(line.active);
    assert!(line.stops.iter().all(|stop| line.route.contains(stop)));
    assert!(line
        .route
        .windows(2)
        .all(|edge| engine.state.roads.iter().any(|road| {
            road.stage == Stage::Open
                && road.connected
                && road.kind != RoadKind::Rail
                && ((road.from == edge[0] && road.to == edge[1])
                    || (road.from == edge[1] && road.to == edge[0]))
        })));
    let boardings = engine
        .state
        .events
        .iter()
        .filter(|event| event.kind == EventKind::BusPassengerBoarded)
        .count();
    assert!(boardings > 0, "bus line never carried a resident");
    assert!(engine
        .state
        .buses
        .iter()
        .all(|bus| bus.passengers.len() <= BUS_CAPACITY));
    engine.validate().unwrap();
}

#[test]
fn seed_five_reverse_bus_corridor_is_ordered_and_waiting_trip_resolves() {
    let mut engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/bus-seed5-legacy-queue.json"
    ))
    .expect("historical queue fixture should restore with compatible defaults");
    let line = engine
        .state
        .bus_line
        .as_ref()
        .expect("seed should open bus service");
    let unique: BTreeSet<u32> = line.route.iter().copied().collect();
    assert_eq!(
        unique.len(),
        line.route.len(),
        "service route must not double back through itself"
    );
    let stop_positions: Vec<usize> = line
        .stops
        .iter()
        .map(|stop| line.route.iter().position(|node| node == stop).unwrap())
        .collect();
    assert!(stop_positions.windows(2).all(|pair| pair[0] < pair[1]));

    let resident_id = 46;
    let resident = engine
        .state
        .residents
        .iter()
        .find(|resident| resident.id == resident_id)
        .expect("historical reverse-queue resident should exist");
    assert_eq!(resident.transfer, "waiting_bus");
    assert_eq!(engine.bus_trip_direction(resident), Some(-1));
    let mut boarded = false;
    for _ in 0..24_000 {
        engine.step(1);
        let resident = engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        if resident.transfer == "riding_bus" && resident.bus_id.is_some() {
            boarded = true;
            break;
        }
    }
    assert!(
        boarded,
        "resident waiting for reverse service did not board an available reverse bus"
    );

    let mut returned_home = false;
    for _ in 0..8_000 {
        engine.step(1);
        let resident = engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id)
            .unwrap();
        if !resident.at_work && resident.transfer.is_empty() {
            returned_home = true;
            break;
        }
    }
    assert!(
        returned_home,
        "the queued resident did not finish the bus return trip: resident={:?}, buses={:?}",
        engine
            .state
            .residents
            .iter()
            .find(|resident| resident.id == resident_id),
        engine
            .state
            .buses
            .iter()
            .map(|bus| (
                bus.progress,
                bus.direction,
                bus.dwell_ticks,
                bus.passengers.len()
            ))
            .collect::<Vec<_>>()
    );
    engine.validate().unwrap();
}

#[test]
fn generated_seed_five_bus_shuttle_runs_both_directions_after_bridge_growth() {
    let mut engine = Engine::new_legacy(5);
    engine.set_development_rate(60);
    for _ in 0..36 {
        engine.step(2_000);
    }
    let line = engine
        .state
        .bus_line
        .as_ref()
        .expect("seed five should have active bus service")
        .clone();
    assert!(line.active);
    assert!(line
        .route
        .windows(2)
        .all(|edge| engine.state.roads.iter().any(|road| {
            road.stage == Stage::Open
                && road.connected
                && ((road.from == edge[0] && road.to == edge[1])
                    || (road.from == edge[1] && road.to == edge[0]))
        })));
    let served_pairs = engine
        .state
        .residents
        .iter()
        .filter(|resident| !resident.rail_commuter)
        .filter_map(|resident| resident.work_node.map(|work| (resident.home_node, work)))
        .filter(|(home, work)| home != work)
        .filter(|(home, work)| {
            let nearest = |endpoint| {
                line.stops
                    .iter()
                    .enumerate()
                    .filter_map(|(index, stop)| {
                        let path = engine.route(endpoint, *stop, false);
                        if path.is_empty() {
                            return None;
                        }
                        let length: f32 = engine.route_lengths(&path).iter().sum();
                        (length <= 75.0).then_some((index, length))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            };
            matches!((nearest(*home), nearest(*work)), (Some((a, _)), Some((b, _))) if a != b)
        })
        .count();
    assert!(
        served_pairs > 0,
        "generated line has no resident OD pairs within walking range"
    );
    let route_length = line
        .route
        .windows(2)
        .map(|edge| {
            let a = engine.node(edge[0]).unwrap();
            let b = engine.node(edge[1]).unwrap();
            (a.x - b.x).hypot(a.z - b.z)
        })
        .sum::<f32>();
    let round_trip_ticks = (route_length / 7.5 * TICKS_PER_SECOND as f32 * 2.0).ceil() as usize
        + line.stops.len() * BUS_DWELL_TICKS as usize * 2
        + 1_000;
    let mut directions = BTreeSet::new();
    for _ in 0..round_trip_ticks {
        engine.step(1);
        directions.extend(engine.state.buses.iter().map(|bus| bus.direction));
        if directions.contains(&-1) && directions.contains(&1) {
            break;
        }
    }
    assert!(
        directions.contains(&-1) && directions.contains(&1),
        "generated bus service did not traverse its complete reverse and forward cycle within {round_trip_ticks} ticks"
    );
    let boarding_deadline = engine.state.tick.saturating_add(24_000);
    while engine.state.tick < boarding_deadline
        && !engine
            .state
            .residents
            .iter()
            .any(|resident| resident.transfer == "riding_bus")
    {
        engine.step(1);
    }
    assert!(
        engine
            .state
            .residents
            .iter()
            .any(|resident| resident.transfer == "riding_bus"),
        "generated service covered {served_pairs} resident pairs but no natural rider boarded"
    );
    assert!(engine
        .state
        .buses
        .iter()
        .all(|bus| bus.passengers.len() <= BUS_CAPACITY));
    engine.validate().unwrap();
}

#[test]
fn pending_bus_alighter_at_rail_terminal_stays_on_ground() {
    let (mut engine, station, _gateway) = rail_test_engine(50);
    let bus_resident_id = add_rail_test_resident(&mut engine, station);
    let bus_resident = engine
        .state
        .residents
        .iter()
        .find(|resident| resident.id == bus_resident_id)
        .unwrap()
        .clone();
    let position = engine.platform_position(true, 3).unwrap();
    assert!(engine.queue_pending_alight(
        bus_resident_id,
        station,
        bus_resident.home_node,
        bus_resident.home_building,
        "commute",
        "walking_from_bus",
        position,
        false
    ));
    let movement = engine.movement_buffer();
    let record = movement
        .chunks_exact(6)
        .find(|record| record[0] == bus_resident_id as f32)
        .unwrap();
    assert_eq!(record[5], -1.0);
}

#[test]
fn cars_held_at_a_busy_driveway_stay_on_the_road_and_queue() {
    let (mut engine, len, ids) = arrival_contention_engine();
    engine.advance_travelers();
    let held = engine
        .state
        .travelers
        .iter()
        .find(|t| t.id == ids[1])
        .unwrap()
        .clone();
    assert_eq!(
        (held.access_phase, held.edge_index),
        (1, 0),
        "held at the kerb on its final lane"
    );
    assert!((held.edge_progress - len.max(1.0)).abs() < 1e-4);
    assert!(
        movement_of(&engine, ids[1]).is_some(),
        "a held car is still drawn"
    );

    let mut admitted = Vec::new();
    for tick in 0..600 {
        let gap = VEHICLE_MIN_GAP - 1e-3;
        // Every road traveler is drawn, and queued cars on the lane keep their gap.
        let road: Vec<&Traveler> = engine
            .state
            .travelers
            .iter()
            .filter(|t| ids.contains(&t.id) && t.access_phase == 1)
            .collect();
        for t in &road {
            assert!(
                t.edge_index < t.route_lengths.len(),
                "tick {tick}: {} left its route",
                t.id
            );
            assert!(
                movement_of(&engine, t.id).is_some(),
                "tick {tick}: {} is not drawn",
                t.id
            );
        }
        for a in &road {
            for b in &road {
                if a.id < b.id {
                    assert!(
                        (a.edge_progress - b.edge_progress).abs() >= gap,
                        "tick {tick}: {} and {} overlap: {:?} {:?}",
                        a.id,
                        b.id,
                        (a.edge_index, a.edge_progress, a.wait_ticks),
                        (b.edge_index, b.edge_progress)
                    );
                }
            }
        }
        // Only one car at a time is in the first car length of the driveway.
        let in_mouth = engine
            .state
            .travelers
            .iter()
            .filter(|t| ids.contains(&t.id) && t.access_phase == 2 && t.access_progress < 4.5)
            .count();
        assert!(
            in_mouth <= 1,
            "tick {tick}: {in_mouth} cars share the driveway mouth"
        );
        for t in &engine.state.travelers {
            if ids.contains(&t.id) && t.access_phase == 2 && !admitted.contains(&t.id) {
                admitted.push(t.id);
            }
        }
        if !engine.state.travelers.iter().any(|t| ids.contains(&t.id)) {
            break;
        }
        engine.advance_travelers();
    }
    // Admitted in road order, and every car arrives.
    assert_eq!(admitted, ids);
    assert!(
        !engine.state.travelers.iter().any(|t| ids.contains(&t.id)),
        "all arrivals complete"
    );
}

#[test]
fn seeded_business_turnover_defers_jobs_and_deliveries_then_reopens() {
    let mut engine = Engine::new_legacy(606);
    let home_parcel = engine.state.parcels[1].id;
    let cafe_parcel = engine.state.parcels[2].id;
    let other_cafe_parcel = engine.state.parcels[3].id;
    let home_id = engine.add_building(
        home_parcel,
        BuildingKind::Home,
        2,
        Stage::Open,
        1.0,
        6,
        "Turnover Home",
        false,
        Vec::new(),
    );
    let cafe_id = engine.add_building(
        cafe_parcel,
        BuildingKind::Cafe,
        2,
        Stage::Open,
        1.0,
        12,
        "Cafe A",
        false,
        Vec::new(),
    );
    let other_cafe_id = engine.add_building(
        other_cafe_parcel,
        BuildingKind::Cafe,
        2,
        Stage::Open,
        1.0,
        12,
        "Cafe B",
        false,
        Vec::new(),
    );
    for building in &mut engine.state.buildings {
        if building.id == home_id {
            building.occupancy = 1;
        }
        if building.id == cafe_id || building.id == other_cafe_id {
            building.built_day = 0;
        }
    }
    let home_node = engine
        .state
        .parcels
        .iter()
        .find(|parcel| parcel.id == home_parcel)
        .unwrap()
        .access_node;
    engine.state.residents.push(Resident {
        id: 900_001,
        name: "Assigned worker".into(),
        home_building: home_id,
        work_building: Some(cafe_id),
        home_node,
        work_node: Some(home_node),
        mode: 0,
        routine: "neighbor".into(),
        activity: "home".into(),
        at_work: false,
        departure_tick: 0,
        return_tick: 0,
        last_departure_day: u32::MAX,
        last_return_day: u32::MAX,
        history: Vec::new(),
        rail_commuter: false,
        transfer: "home".into(),
        train_id: None,
        queue_slot: None,
        bus_id: None,
        bus_origin_stop: None,
        bus_destination_stop: None,
        bus_stop_node: None,
        pending_alight: None,
    });
    engine.state.travelers.push(Traveler {
        id: 900_002,
        kind: 1,
        route: Vec::new(),
        route_lengths: Vec::new(),
        edge_index: 0,
        edge_progress: 0.0,
        speed: 9.0,
        destination: 0,
        activity: "delivery".into(),
        resident: 0,
        name: "Delivery van".into(),
        origin_building: None,
        destination_building: Some(other_cafe_id),
        wait_ticks: 0,
        origin_access: None,
        destination_access: None,
        access_phase: 1,
        access_progress: 0.0,
    });
    assert!(!engine.try_close_business_for_turnover(30));
    assert!(engine
        .state
        .buildings
        .iter()
        .filter(|building| building.id == cafe_id || building.id == other_cafe_id)
        .all(|building| building.business_active));
    engine.state.residents.clear();
    engine.state.travelers.clear();

    let initial = engine.save_json();
    let mut replay = Engine::restore_json(&initial).unwrap();
    let mut closed_id = None;
    let mut closed_on_day = None;
    for day in (30..=30 + 30 * 63).step_by(30) {
        let closed = engine.try_close_business_for_turnover(day);
        assert_eq!(closed, replay.try_close_business_for_turnover(day));
        if closed {
            closed_on_day = Some(day);
            closed_id = engine
                .state
                .buildings
                .iter()
                .find(|building| !building.business_active && is_commercial(building))
                .map(|building| building.id);
            break;
        }
    }
    assert!(
        closed_id.is_some(),
        "seeded turnover selected no eligible tenant"
    );
    assert_eq!(engine.save_json(), replay.save_json());
    let closed_id = closed_id.unwrap();
    let closed = engine
        .state
        .buildings
        .iter_mut()
        .find(|building| building.id == closed_id)
        .unwrap();
    assert_eq!(closed.occupancy, 0);
    assert!(!closed.business_active);
    assert!(closed
        .history
        .iter()
        .any(|entry| entry.starts_with("Business closed:")));
    assert!(engine
        .state
        .events
        .iter()
        .any(|event| { event.kind == EventKind::BusinessClosed && event.entity_id == closed_id }));
    engine.sync_residents();
    assert!(engine
        .state
        .residents
        .iter()
        .all(|resident| resident.work_building != Some(closed_id)));
    engine.state.next_delivery_tick = engine.state.tick;
    engine.schedule_deliveries();
    assert!(engine
        .state
        .travelers
        .iter()
        .all(|traveler| traveler.destination_building != Some(closed_id)));

    let other_cafe_id = engine
        .state
        .buildings
        .iter()
        .find(|building| {
            building.id != closed_id && building.kind == BuildingKind::Cafe && site_active(building)
        })
        .unwrap()
        .id;
    engine
        .state
        .buildings
        .iter_mut()
        .find(|b| b.id == closed_id)
        .unwrap()
        .vacant_days = 6;
    engine.state.last_renewal_day = u32::MAX;
    engine.renewal(closed_on_day.unwrap() + 3);
    assert_eq!(
        engine
            .state
            .buildings
            .iter()
            .find(|b| b.id == closed_id)
            .unwrap()
            .stage,
        Stage::Renovation
    );
    for _ in 0..2 {
        engine.advance_projects();
    }
    let reopened = engine
        .state
        .buildings
        .iter()
        .find(|building| building.id == closed_id)
        .unwrap();
    assert_eq!(reopened.stage, Stage::Open);
    assert!(reopened.business_active);
    assert_eq!(reopened.vacant_days, 0);
    assert!(engine.state.events.iter().any(|event| {
        event.kind == EventKind::BusinessReopened && event.entity_id == closed_id
    }));
    assert!(site_active(
        engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == other_cafe_id)
            .unwrap()
    ));
    engine.validate().unwrap();
    let save = engine.save_json();
    let mut restored = Engine::restore_json(&save).unwrap();
    engine.step(2_000);
    restored.step(2_000);
    assert_eq!(engine.save_json(), restored.save_json());
}

#[test]
fn road_split_preserves_frontage_travelers_bus_train_and_replay() {
    let mut engine = Engine::new_legacy(7);
    let host = engine
        .state
        .roads
        .iter()
        .find(|road| road.kind == RoadKind::Arterial && road.stage == Stage::Open)
        .unwrap()
        .clone();
    engine.state.neighborhood_road_ids.insert(host.id);
    let a = engine.node(host.from).unwrap().clone();
    let b = engine.node(host.to).unwrap().clone();
    let east = engine
        .state
        .roads
        .iter()
        .find(|road| road.id != host.id && (road.from == b.id || road.to == b.id))
        .map(|road| {
            if road.from == b.id {
                road.to
            } else {
                road.from
            }
        })
        .unwrap();
    let t = 0.6;
    let host_length = distance(&a, &b);
    let route = vec![a.id, b.id, east, b.id, a.id];
    let route_lengths = engine.route_lengths(&route);

    let parcel_id = engine.add_parcel(250.0, 60.0, 14.0, 12.0, a.id, 0.8, false);
    engine.state.parcel_frontages.push(ParcelFrontage {
        parcel_id,
        road_id: host.id,
        along: 0.35,
        side: -1,
    });
    let building_id = engine.add_building(
        parcel_id,
        BuildingKind::Home,
        2,
        Stage::Open,
        1.0,
        8,
        "Split Test Home",
        false,
        vec!["Opened beside the arterial".into()],
    );
    let before_access = engine.building_access_segment(building_id).unwrap();

    let second_parcel = engine.add_parcel(120.0, 75.0, 14.0, 12.0, b.id, 0.8, false);
    engine.state.parcel_frontages.push(ParcelFrontage {
        parcel_id: second_parcel,
        road_id: host.id,
        along: 0.85,
        side: -1,
    });
    let second_building_id = engine.add_building(
        second_parcel,
        BuildingKind::Home,
        2,
        Stage::Open,
        1.0,
        8,
        "Second Split Test Home",
        false,
        vec!["Opened beside the far half".into()],
    );
    let before_second_access = engine.building_access_segment(second_building_id).unwrap();

    let actor_positions = [
        (0, host_length * 0.3, 1),
        (0, host_length * 0.8, 1),
        (1, route_lengths[1] * 0.45, 1),
        (3, host_length * 0.8, 1),
        (4, 0.0, 0),
    ];
    for (index, (edge_index, edge_progress, access_phase)) in
        actor_positions.into_iter().enumerate()
    {
        let id = engine.id();
        let mut traveler = Traveler {
            id,
            kind: 0,
            route: route.clone(),
            route_lengths: route_lengths.clone(),
            edge_index,
            edge_progress,
            speed: 1.2,
            destination: *route.last().unwrap(),
            activity: "split regression".into(),
            resident: 0,
            name: format!("Split walker {index}"),
            origin_building: None,
            destination_building: None,
            wait_ticks: 0,
            origin_access: None,
            destination_access: None,
            access_phase,
            access_progress: if access_phase == 0 { 2.75 } else { 0.0 },
        };
        if access_phase == 0 {
            let join = engine
                .route_endpoint_position(&traveler.route, true, traveler.id, traveler.kind)
                .unwrap();
            traveler.origin_access = Some(AccessPath {
                points: vec![
                    WorldPoint {
                        x: join.x,
                        z: join.z + 16.0,
                    },
                    join,
                ],
                surface_y: Vec::new(),
            });
        }
        engine.state.travelers.push(traveler);
    }
    let before_positions: Vec<_> = engine
        .state
        .travelers
        .iter()
        .map(|traveler| route_position(&engine, traveler))
        .collect();
    let original_access_progress = engine
        .state
        .travelers
        .iter()
        .map(|traveler| (traveler.access_phase, traveler.access_progress))
        .collect::<Vec<_>>();

    let line_id = engine.id();
    let line_stops = vec![a.id, b.id, east];
    engine.state.bus_line = Some(BusLine {
        id: line_id,
        stops: line_stops.clone(),
        route: vec![a.id, b.id, east],
        active: true,
        graph_revision: 4,
    });
    let bus_id = engine.id();
    let bus_resident_id = engine.id();
    engine.state.buses.push(Bus {
        id: bus_id,
        line_id,
        progress: host_length * 0.72,
        direction: 1,
        dwell_ticks: 0,
        passengers: vec![bus_resident_id],
        next_departure_tick: 500,
        last_stop: Some(a.id),
    });
    engine.state.residents.push(split_test_resident(
        bus_resident_id,
        building_id,
        a.id,
        east,
        "riding_bus",
        None,
        Some(bus_id),
        Some((a.id, east)),
    ));
    let bus_pose_before = engine
        .bus_pose(
            engine.state.bus_line.as_ref().unwrap(),
            &engine.state.buses[0],
        )
        .unwrap();

    let train_id = engine.id();
    let train_resident_id = engine.id();
    engine.state.station_online = true;
    engine.state.trains.push(Train {
        id: train_id,
        route: vec![a.id, b.id],
        distance: host_length * 0.72,
        speed: 8.0,
        direction: 1,
        dwell_ticks: 0,
        passengers: vec![train_resident_id],
    });
    engine.state.residents.push(split_test_resident(
        train_resident_id,
        building_id,
        a.id,
        b.id,
        "riding_outbound",
        Some(train_id),
        None,
        None,
    ));
    let revision_before = engine.state.revision;
    let access_revision_before = engine.state.access_geometry_revision;
    let old_road_count = engine.state.roads.len();

    let outcome = engine.try_split_open_road(host.id, t).unwrap();
    assert!(engine.state.access_geometry_revision > access_revision_before);
    assert_eq!(outcome.retained_road_id, host.id);
    assert_eq!(engine.state.roads.len(), old_road_count + 1);
    let retained = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == outcome.retained_road_id)
        .unwrap();
    let appended = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == outcome.new_road_id)
        .unwrap();
    assert_eq!((retained.from, retained.to), (a.id, outcome.split_node_id));
    assert_eq!((appended.from, appended.to), (outcome.split_node_id, b.id));
    assert!(
        engine
            .state
            .roads
            .iter()
            .any(|road| { road.from == a.id && road.to == b.id && road.id != host.id })
            == false
    );
    assert_eq!(engine.state.revision, revision_before + 1);

    for (index, traveler) in engine.state.travelers.iter().enumerate() {
        let after = route_position(&engine, traveler);
        assert!((before_positions[index].x - after.x).abs() < 1e-3);
        assert!((before_positions[index].z - after.z).abs() < 1e-3);
        assert_eq!(
            (traveler.access_phase, traveler.access_progress),
            original_access_progress[index]
        );
    }
    let after_access = engine.building_access_segment(building_id).unwrap();
    assert_eq!(after_access.0, host.id);
    assert!((before_access.1[0].x - after_access.1[0].x).abs() < 1e-4);
    assert!((before_access.1[0].z - after_access.1[0].z).abs() < 1e-4);
    assert!((before_access.1[1].x - after_access.1[1].x).abs() < 1e-3);
    assert!((before_access.1[1].z - after_access.1[1].z).abs() < 1e-3);
    let after_second_access = engine.building_access_segment(second_building_id).unwrap();
    assert_eq!(after_second_access.0, outcome.new_road_id);
    assert!((before_second_access.1[0].x - after_second_access.1[0].x).abs() < 1e-4);
    assert!((before_second_access.1[0].z - after_second_access.1[0].z).abs() < 1e-4);
    assert!((before_second_access.1[1].x - after_second_access.1[1].x).abs() < 1e-3);
    assert!((before_second_access.1[1].z - after_second_access.1[1].z).abs() < 1e-3);
    assert_eq!(engine.state.bus_line.as_ref().unwrap().stops, line_stops);
    assert_eq!(
        engine.state.bus_line.as_ref().unwrap().route,
        vec![a.id, outcome.split_node_id, b.id, east]
    );
    assert_eq!(engine.state.bus_line.as_ref().unwrap().graph_revision, 5);
    assert_eq!(engine.state.buses[0].progress, host_length * 0.72);
    assert_eq!(engine.state.buses[0].passengers, vec![bus_resident_id]);
    let bus_pose_after = engine
        .bus_pose(
            engine.state.bus_line.as_ref().unwrap(),
            &engine.state.buses[0],
        )
        .unwrap();
    assert!((bus_pose_before.0 - bus_pose_after.0).abs() < 1e-3);
    assert!((bus_pose_before.1 - bus_pose_after.1).abs() < 1e-3);
    assert_eq!(engine.state.trains[0].distance, host_length * 0.72);
    assert_eq!(engine.state.trains[0].passengers, vec![train_resident_id]);
    assert_eq!(engine.state.residents[0].home_node, a.id);
    assert_eq!(engine.state.residents[0].work_node, Some(east));
    assert!(engine.state.neighborhood_road_ids.contains(&host.id));
    assert!(engine
        .state
        .neighborhood_road_ids
        .contains(&outcome.new_road_id));
    engine.validate().unwrap();

    let canonical = engine.save_json();
    let mut resumed = Engine::restore_json(&canonical).unwrap();
    assert_eq!(resumed.save_json(), canonical);
    engine.step(200);
    resumed.step(200);
    assert_eq!(engine.save_json(), resumed.save_json());
}
