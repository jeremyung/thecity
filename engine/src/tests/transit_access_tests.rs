use crate::{BuildingKind, NodeKind, RoadKind, Stage};
use crate::{Bus, BusLine, Engine, Node, Resident, Road};

#[test]
fn scheduled_and_alighting_bus_connectors_end_at_the_correct_route_endpoint() {
    let mut engine = Engine::new_legacy(217);
    let mut make_node = |x: f32, z: f32| {
        let id = engine.id();
        let node = Node {
            id,
            x,
            z,
            kind: NodeKind::Junction,
        };
        engine.state.nodes.push(node);
        id
    };
    let approach = make_node(-20.0, 0.0);
    let first_stop = make_node(0.0, 0.0);
    let middle_stop = make_node(30.0, 0.0);
    let last_stop = make_node(60.0, 0.0);
    let destination = make_node(80.0, 0.0);
    for (from, to) in [
        (approach, first_stop),
        (first_stop, middle_stop),
        (middle_stop, last_stop),
        (last_stop, destination),
    ] {
        let id = engine.id();
        let a = engine.node(from).unwrap();
        let b = engine.node(to).unwrap();
        let length = (b.x - a.x).hypot(b.z - a.z);
        engine.state.roads.push(Road {
            id,
            from,
            to,
            kind: RoadKind::Local,
            stage: Stage::Open,
            progress: 1.0,
            connected: true,
            length,
            lanes: 2,
        });
    }
    let line_id = engine.id();
    engine.state.bus_line = Some(BusLine {
        id: line_id,
        stops: vec![first_stop, middle_stop, last_stop],
        route: vec![first_stop, middle_stop, last_stop],
        active: true,
        graph_revision: 1,
    });
    let bus_id = engine.id();
    engine.state.buses.push(Bus {
        id: bus_id,
        line_id,
        progress: 0.0,
        direction: 1,
        dwell_ticks: 0,
        passengers: Vec::new(),
        next_departure_tick: 0,
        last_stop: None,
    });

    let building = engine.state.buildings[0].id;
    let resident_id = engine.id();
    engine.state.residents.push(Resident {
        id: resident_id,
        name: "Transit endpoint test".into(),
        home_building: building,
        work_building: Some(building),
        home_node: approach,
        work_node: Some(destination),
        mode: 0,
        routine: "worker".into(),
        activity: "at_home".into(),
        at_work: false,
        departure_tick: 0,
        return_tick: 0,
        last_departure_day: u32::MAX,
        last_return_day: u32::MAX,
        history: Vec::new(),
        rail_commuter: false,
        transfer: String::new(),
        train_id: None,
        queue_slot: None,
        bus_id: None,
        bus_origin_stop: None,
        bus_destination_stop: None,
        bus_stop_node: None,
        pending_alight: None,
    });

    assert!(engine.schedule_bus_trip(
        0,
        approach,
        destination,
        "commute",
        building,
        Some(building)
    ));
    let traveler_index = engine
        .state
        .travelers
        .iter()
        .position(|traveler| traveler.id == resident_id)
        .unwrap();
    let scheduled_route = engine.state.travelers[traveler_index].route.clone();
    let scheduled_endpoint = engine
        .route_endpoint_position(&scheduled_route, false, resident_id, 0)
        .unwrap();
    let scheduled_connector = engine.state.travelers[traveler_index]
        .destination_access
        .as_ref()
        .expect("scheduled trip has a queue connector");
    let scheduled_start = scheduled_connector.points.first().unwrap();
    assert_eq!(
        (scheduled_start.x, scheduled_start.z),
        (scheduled_endpoint.x, scheduled_endpoint.z)
    );

    engine.state.travelers.clear();
    {
        let resident = &mut engine.state.residents[0];
        resident.transfer = "riding_bus".into();
        resident.bus_id = Some(bus_id);
        resident.bus_origin_stop = Some(first_stop);
        resident.bus_destination_stop = Some(last_stop);
        resident.bus_stop_node = Some(last_stop);
        resident.queue_slot = Some(2);
    }
    engine.state.buses[0].passengers.push(resident_id);
    engine.alight_bus_passenger(0, resident_id, last_stop);

    let alighting = engine
        .state
        .travelers
        .iter()
        .find(|traveler| traveler.id == resident_id)
        .expect("alighting passenger gets a walking traveler");
    let alighting_endpoint = engine
        .route_endpoint_position(&alighting.route, true, resident_id, 0)
        .unwrap();
    let alighting_connector = alighting
        .origin_access
        .as_ref()
        .expect("alighting trip has a queue connector");
    let alighting_end = alighting_connector.points.last().unwrap();
    assert_eq!(
        (alighting_end.x, alighting_end.z),
        (alighting_endpoint.x, alighting_endpoint.z)
    );
}

#[test]
fn recovered_rail_alight_connector_ends_at_the_route_start_endpoint() {
    let mut engine = Engine::new_legacy(477);
    engine.state.station_online = true;
    let (from, to) = {
        let rail = engine
            .state
            .roads
            .iter_mut()
            .find(|road| road.kind == RoadKind::Rail)
            .unwrap();
        rail.stage = Stage::Open;
        rail.progress = 1.0;
        rail.connected = true;
        (rail.from, rail.to)
    };
    let station = [from, to]
        .into_iter()
        .find(|id| {
            engine
                .node(*id)
                .is_some_and(|node| node.kind == NodeKind::Station)
        })
        .unwrap();
    let destination = {
        let node = engine.node(station).unwrap().clone();
        let id = engine.id();
        engine.state.nodes.push(Node {
            id,
            x: node.x + 24.0,
            z: node.z - 18.0,
            kind: NodeKind::Junction,
        });
        id
    };
    let building = engine.state.buildings[0].id;
    engine.state.buildings[0].kind = BuildingKind::Home;
    engine.state.buildings[0].stage = Stage::Open;
    let resident_id = engine.id();
    engine.state.residents.push(Resident {
        id: resident_id,
        name: "Recovered rail alight".into(),
        home_building: building,
        work_building: None,
        home_node: destination,
        work_node: None,
        mode: 0,
        routine: "worker".into(),
        activity: "waiting_for_route".into(),
        at_work: true,
        departure_tick: 0,
        return_tick: 0,
        last_departure_day: u32::MAX,
        last_return_day: u32::MAX,
        history: Vec::new(),
        rail_commuter: true,
        transfer: "waiting_alight".into(),
        train_id: None,
        queue_slot: None,
        bus_id: None,
        bus_origin_stop: None,
        bus_destination_stop: None,
        bus_stop_node: Some(station),
        pending_alight: None,
    });
    let wait_position = engine.platform_position(true, 1).unwrap();
    assert!(engine.queue_pending_alight(
        resident_id,
        station,
        destination,
        building,
        "return",
        "walking_home",
        wait_position,
        false,
    ));
    let road_id = engine.add_road(station, destination, RoadKind::Local, Stage::Open, 1.0);
    engine
        .state
        .roads
        .iter_mut()
        .find(|road| road.id == road_id)
        .unwrap()
        .connected = true;
    engine.recompute_connectivity();
    engine.advance_pending_alights();

    let traveler = engine
        .state
        .travelers
        .iter()
        .find(|traveler| traveler.id == resident_id)
        .expect("restored route creates a traveler");
    assert_eq!(traveler.route, vec![station, destination]);
    let expected_endpoint = engine
        .route_endpoint_position(&traveler.route, true, resident_id, 0)
        .unwrap();
    let connector = traveler
        .origin_access
        .as_ref()
        .expect("platform exit connector is retained");
    assert_eq!(
        (connector.points[0].x, connector.points[0].z),
        (wait_position.x, wait_position.z)
    );
    let final_point = connector.points.last().unwrap();
    assert_eq!(
        (final_point.x, final_point.z),
        (expected_endpoint.x, expected_endpoint.z)
    );
    assert_eq!(engine.state.residents[0].transfer, "walking_home");
}
