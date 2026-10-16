use super::*;

#[test]
fn pedestrian_building_connectors_are_continuous_saved_and_face_frontage() {
    let mut engine = Engine::new_legacy(46);
    engine.state.development_rate = 0;
    let founder = engine
        .state
        .buildings
        .iter()
        .find(|building| building.anchor)
        .unwrap()
        .clone();
    assert_eq!(founder.rotation, 0.0);
    let home_parcel = engine
        .state
        .parcels
        .iter()
        .find(|parcel| {
            parcel.access_node != engine.state.parcels[0].access_node
                && engine
                    .state
                    .parcel_frontages
                    .iter()
                    .any(|frontage| frontage.parcel_id == parcel.id)
        })
        .cloned()
        .expect("founding parcels should carry road frontage metadata");
    let home_id = engine.add_building(
        home_parcel.id,
        BuildingKind::Home,
        2,
        Stage::Open,
        1.0,
        80,
        "Connector Test Home",
        false,
        vec![],
    );
    let route = engine.route(
        engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == founder.parcel_id)
            .unwrap()
            .access_node,
        home_parcel.access_node,
        false,
    );
    assert!(route.len() >= 2);
    let id = 700_111;
    let mut traveler = Traveler {
        id,
        kind: 0,
        route_lengths: engine.route_lengths(&route),
        route,
        edge_index: 0,
        edge_progress: 0.0,
        speed: 1.5,
        destination: home_parcel.access_node,
        activity: "commute".into(),
        resident: 0,
        name: "Connector Test Walker".into(),
        origin_building: Some(founder.id),
        destination_building: Some(home_id),
        wait_ticks: 0,
        origin_access: None,
        destination_access: None,
        access_phase: 1,
        access_progress: 0.0,
    };
    traveler.origin_access = engine.building_access_path(
        founder.id,
        &traveler.route,
        true,
        traveler.id,
        traveler.kind,
    );
    traveler.destination_access =
        engine.building_access_path(home_id, &traveler.route, false, traveler.id, traveler.kind);
    assert!(traveler.origin_access.is_some());
    assert!(traveler.destination_access.is_some());
    let access_snapshot = engine.snapshot();
    let founder_access = access_snapshot
        .building_access
        .iter()
        .find(|access| access.building_id == founder.id)
        .expect("founder access should be exported for its open frontage");
    let founder_path = traveler.origin_access.as_ref().unwrap();
    assert!(
        (founder_access.points[0].x - founder_path.points[0].x)
            .hypot(founder_access.points[0].z - founder_path.points[0].z)
            < 0.01
    );
    assert!(
        (founder_access.points[1].x - founder_path.points[1].x)
            .hypot(founder_access.points[1].z - founder_path.points[1].z)
            < 0.01
    );
    let home_access = access_snapshot
        .building_access
        .iter()
        .find(|access| access.building_id == home_id)
        .expect("home access should be exported for its open frontage");
    let home_path = traveler.destination_access.as_ref().unwrap();
    let home_door = home_path.points.last().unwrap();
    let home_frontage = &home_path.points[home_path.points.len() - 2];
    assert!(
        (home_access.points[0].x - home_door.x).hypot(home_access.points[0].z - home_door.z) < 0.01
    );
    assert!(
        (home_access.points[1].x - home_frontage.x)
            .hypot(home_access.points[1].z - home_frontage.z)
            < 0.01
    );

    let frontage_road = home_access.road_id;
    let original_frontage_road = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == frontage_road)
        .unwrap()
        .clone();
    engine
        .state
        .roads
        .iter_mut()
        .find(|road| road.id == frontage_road)
        .unwrap()
        .connected = false;
    assert!(!engine
        .snapshot()
        .building_access
        .iter()
        .any(|access| access.building_id == home_id));
    let road = engine
        .state
        .roads
        .iter_mut()
        .find(|road| road.id == frontage_road)
        .unwrap();
    road.connected = true;
    road.stage = Stage::Structure;
    assert!(!engine
        .snapshot()
        .building_access
        .iter()
        .any(|access| access.building_id == home_id));
    let road = engine
        .state
        .roads
        .iter_mut()
        .find(|road| road.id == frontage_road)
        .unwrap();
    road.connected = original_frontage_road.connected;
    road.stage = original_frontage_road.stage;
    let door = traveler.origin_access.as_ref().unwrap().points[0];
    let expected_door = WorldPoint {
        x: founder.x + founder.rotation.sin() * (founder.depth * 0.5 + 0.2),
        z: founder.z + founder.rotation.cos() * (founder.depth * 0.5 + 0.2),
    };
    assert!((door.x - expected_door.x).hypot(door.z - expected_door.z) < 0.01);
    traveler.access_phase = 0;
    engine.state.travelers.push(traveler);
    let first = &engine.movement_buffer()[..6];
    assert!((first[1] - door.x).abs() < 0.01 && (first[2] - door.z).abs() < 0.01);
    let save = engine.save_json();
    let mut restored = Engine::restore_json(&save).unwrap();
    for _ in 0..20 {
        engine.state.tick += 1;
        restored.state.tick += 1;
        engine.advance_travelers();
        restored.advance_travelers();
    }
    assert_eq!(engine.save_json(), restored.save_json());
    let mut closest_destination = f32::INFINITY;
    for _ in 0..4_000 {
        let Some(traveler) = engine
            .state
            .travelers
            .iter()
            .find(|traveler| traveler.id == id)
        else {
            break;
        };
        if traveler.access_phase == 2 {
            let door = traveler
                .destination_access
                .as_ref()
                .unwrap()
                .points
                .last()
                .unwrap();
            if let Some(actor) = engine
                .movement_buffer()
                .chunks_exact(6)
                .find(|actor| actor[0] as u32 == id)
            {
                closest_destination =
                    closest_destination.min((actor[1] - door.x).hypot(actor[2] - door.z));
            }
        }
        engine.state.tick += 1;
        engine.advance_travelers();
    }
    assert!(
        closest_destination < 0.25,
        "walker did not reach the destination door: {closest_destination}"
    );
    assert!(!engine
        .state
        .travelers
        .iter()
        .any(|traveler| traveler.id == id));
    engine.validate().unwrap();
}

#[test]
fn progressive_gap_infill_admits_compact_frontage_with_access_and_replay() {
    let mut engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/mature-world.cache-candidate.json"
    ))
    .expect("mature fixture restores");
    let context = engine.collision_context();
    let tower_envelopes = engine.available_station_tower_envelopes();
    let mut target = None;

    'roads: for (road_index, road) in engine.state.roads.iter().enumerate() {
        if road.stage != Stage::Open
            || !road.connected
            || matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        {
            continue;
        }
        let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
            continue;
        };
        let grid =
            engine.frontage_candidate_sites_with_layout(a, b, 14.0, 12.0, FRONTAGE_SLOT_SPACING);
        let gaps = engine.frontage_gap_candidates(a, b, &context, &tower_envelopes);
        for (gap_index, ((x, z, access_node, along, side), width)) in
            gaps.iter().copied().enumerate()
        {
            if !(8.0..14.0).contains(&width)
                || Engine::overlaps_station_tower_envelopes(&tower_envelopes, x, z, width, 12.0)
            {
                continue;
            }
            let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
            let rect = geometry::OrientedRect {
                x,
                z,
                width,
                depth: 12.0,
                rotation,
            };
            if !engine.progressive_frontage_geometry_clear(&rect, &context) {
                continue;
            }
            let (mut local_open, mut local_active) = (0usize, 0usize);
            for building in engine.state.buildings.iter().filter(|building| {
                building.stage == Stage::Open && (building.x - x).hypot(building.z - z) <= 78.0
            }) {
                local_open += 1;
                local_active += usize::from(site_active(building));
            }
            if local_open < 4 || local_active < 3 {
                continue;
            }
            let road_length = (b.x - a.x).hypot(b.z - a.z);
            let left_flanker = engine
                .state
                .parcel_frontages
                .iter()
                .filter(|frontage| {
                    frontage.road_id == road.id && frontage.side == side && frontage.along < along
                })
                .map(|frontage| along - frontage.along)
                .min_by(f32::total_cmp);
            let right_flanker = engine
                .state
                .parcel_frontages
                .iter()
                .filter(|frontage| {
                    frontage.road_id == road.id && frontage.side == side && frontage.along > along
                })
                .map(|frontage| frontage.along - along)
                .min_by(f32::total_cmp);
            if !left_flanker.is_some_and(|distance| distance * road_length <= 32.0)
                || !right_flanker.is_some_and(|distance| distance * road_length <= 32.0)
            {
                continue;
            }
            target = Some((
                road_index,
                road.id,
                grid.len() + gap_index,
                x,
                z,
                width,
                access_node,
                along,
                side,
                rotation,
            ));
            break 'roads;
        }
    }

    let (road_index, road_id, slot, x, z, width, access_node, along, side, rotation) =
        target.expect("mature active frontage has an admissible compact gap lot");
    engine.state.frontage_road_cursor = road_index;
    engine.state.frontage_slot_cursor = slot as u16;
    assert!(slot < MAX_FRONTAGE_SITES_PER_ROAD);
    let old_parcels = engine.state.parcels.len();
    assert!(engine.try_progressive_frontage_infill());
    assert_eq!(engine.state.parcels.len(), old_parcels + 1);
    assert_eq!(engine.state.frontage_road_cursor, road_index);
    assert_eq!(engine.state.frontage_slot_cursor as usize, slot + 1);

    let parcel = engine.state.parcels.last().unwrap().clone();
    assert!((parcel.x - x).hypot(parcel.z - z) < 1e-4);
    assert_eq!(parcel.width, width);
    assert_eq!(parcel.depth, 12.0);
    assert!((8.0..14.0).contains(&parcel.width));
    assert_eq!(parcel.rotation, Some(rotation));
    let frontage = engine.state.parcel_frontages.last().unwrap();
    assert_eq!(frontage.parcel_id, parcel.id);
    assert_eq!(frontage.road_id, road_id);
    assert_eq!(parcel.access_node, access_node);
    assert!((frontage.along - along).abs() < 1e-5);
    assert_eq!(frontage.side, side);
    assert!(engine.node_connected(parcel.access_node));

    let building = engine.add_building(
        parcel.id,
        BuildingKind::Home,
        2,
        Stage::Open,
        1.0,
        8,
        "Gap frontage home",
        false,
        vec!["Built into an existing street gap".into()],
    );
    assert!(engine.building_access_segment(building).is_some());
    engine.validate().unwrap();

    let save = engine.save_json();
    let mut resumed = Engine::restore_json(&save).unwrap();
    assert_eq!(resumed.save_json(), save);
    engine.step(200);
    resumed.step(200);
    assert_eq!(engine.save_json(), resumed.save_json());
    engine.validate().unwrap();
}

#[test]
fn frontage_access_selection_is_projection_based_and_orientation_stable() {
    let engine = Engine::new_legacy(53);
    // This real midpoint geometry exposed a platform-dependent hypot tie:
    // the side offset made one endpoint appear about 2e-5m closer in WASM.
    let a = Node {
        id: 274,
        x: 56.069_992,
        z: 164.448_24,
        kind: NodeKind::Junction,
    };
    let b = Node {
        id: 753,
        x: 117.672_96,
        z: 219.651_73,
        kind: NodeKind::Junction,
    };
    assert_eq!(frontage_access_node(&a, &b, 0.25), a.id);
    assert_eq!(frontage_access_node(&a, &b, 0.75), b.id);
    assert_eq!(frontage_access_node(&a, &b, 0.5), a.id.min(b.id));
    assert_eq!(frontage_access_node(&b, &a, 0.5), a.id.min(b.id));

    let sites = engine.frontage_candidate_sites(&a, &b);
    let midpoint = sites
        .iter()
        .find(|site| site.4 == 1 && site.3 == 0.5)
        .expect("the positive-side midpoint is an eligible frontage site");
    assert_eq!(midpoint.2, a.id.min(b.id));

    let reversed = engine.frontage_candidate_sites(&b, &a);
    let reversed_midpoint = reversed
        .iter()
        .find(|site| site.4 == -1 && site.3 == 0.5)
        .expect("reversing a road preserves the same physical midpoint side");
    assert_eq!(reversed_midpoint.2, a.id.min(b.id));
    assert!((midpoint.0 - reversed_midpoint.0).abs() < 0.0001);
    assert!((midpoint.1 - reversed_midpoint.1).abs() < 0.0001);
}

#[test]
fn coordinated_block_lots_wait_for_perimeter_and_preserve_access_on_restore() {
    let mut engine = engine_with_developed_plan_site_without_reusable_edge();
    assert!(engine.try_create_neighborhood_plan());
    assert!(
        engine
            .state
            .neighborhood_plan
            .as_ref()
            .unwrap()
            .coordinated_lots
    );
    let mut replay = Engine::restore_json(&engine.save_json()).unwrap();
    for edge in 0..4 {
        for world in [&mut engine, &mut replay] {
            assert!(world.plan_next_neighborhood_segment());
            let project = world
                .state
                .projects
                .iter_mut()
                .find(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
                .unwrap();
            let id = project.id;
            project.progress = 1.0;
            world.complete_project(id);
        }
        assert_eq!(engine.save_json(), replay.save_json());
        let count = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| parcel.rotation.is_some())
            .count();
        if edge < 3 {
            assert_eq!(count, 0, "interior parcels must wait for all four streets");
        } else {
            assert_eq!(
                count, 10,
                "both coordinated rows must fill the reserved interior"
            );
        }
    }
    assert!(engine.state.neighborhood_plan.is_none());
    let lots: Vec<_> = engine
        .state
        .parcels
        .iter()
        .filter(|parcel| parcel.rotation.is_some())
        .cloned()
        .collect();
    for parcel in &lots {
        let frontage = engine
            .state
            .parcel_frontages
            .iter()
            .find(|frontage| frontage.parcel_id == parcel.id)
            .unwrap();
        let road = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == frontage.road_id)
            .unwrap();
        assert!(road.stage == Stage::Open && road.connected);
        assert!(engine.node_connected(parcel.access_node));
        let theta = parcel.rotation.unwrap();
        let expected = engine.frontage_rotation(parcel.id);
        assert!(
            (theta.sin() - expected.sin()).abs() < 1e-5
                && (theta.cos() - expected.cos()).abs() < 1e-5
        );
    }
    engine.validate().unwrap();
    let save = engine.save_json();
    let mut resumed = Engine::restore_json(&save).unwrap();
    assert_eq!(resumed.save_json(), save);
    engine.step(2000);
    resumed.step(2000);
    assert_eq!(engine.save_json(), resumed.save_json());
    engine.validate().unwrap();
}

#[test]
fn local_chord_construction_opens_connected_frontage_and_adds_accessible_density() {
    let (engine, start, target) = engine_with_clear_local_chord();
    let save = engine.save_json();
    let mut first = Engine::restore_json(&save).unwrap();
    let mut replay = Engine::restore_json(&save).unwrap();

    first.plan_road();
    replay.plan_road();
    assert_eq!(first.save_json(), replay.save_json());
    let project = first
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        .cloned()
        .expect("a clear occupied neighborhood should reserve a local chord");
    assert_eq!(project.from, Some(start));
    assert_eq!(project.to, Some(target));
    assert!(!first.state.roads.iter().any(|road| {
        road.stage == Stage::Open
            && ((road.from == start && road.to == target)
                || (road.from == target && road.to == start))
    }));

    let (a, b) = (first.node(start).unwrap(), first.node(target).unwrap());
    assert!((45.0..=90.0).contains(&distance(a, b)));
    assert!(first.ordinary_road_clear_of_river(a.x, a.z, b.x, b.z, 4.5));
    let corridor_context = first.collision_context();
    for footprint in &corridor_context.parcel_footprints {
        assert!(geometry::rect_segment_distance(footprint, a.x, a.z, b.x, b.z) >= 5.0);
    }
    for road in first
        .state
        .roads
        .iter()
        .filter(|road| road.stage == Stage::Open)
    {
        if road.from == start || road.to == start || road.from == target || road.to == target {
            continue;
        }
        let (from, to) = (first.node(road.from).unwrap(), first.node(road.to).unwrap());
        assert!(segments_distance(a, b, from, to) > 8.0);
    }

    // The in-flight checkpoint restores the same reserved corridor. It
    // remains unavailable to traffic until the ordinary road project ends.
    let active_save = first.save_json();
    let mut resumed = Engine::restore_json(&active_save).unwrap();
    assert_eq!(resumed.save_json(), active_save);
    let initial_parcels = first.state.parcels.len();
    let initial_local_envelope_area: f32 = first
        .state
        .buildings
        .iter()
        .filter(|building| (building.x - 30.0).hypot(building.z - 300.0) <= 80.0)
        .map(|building| building.width * building.depth)
        .sum();
    for engine in [&mut first, &mut resumed] {
        engine
            .state
            .projects
            .iter_mut()
            .find(|candidate| candidate.id == project.id)
            .unwrap()
            .duration = 2.0;
        engine.advance_projects();
        assert!(!engine.state.roads.iter().any(|road| {
            road.stage == Stage::Open
                && ((road.from == start && road.to == target)
                    || (road.from == target && road.to == start))
        }));
        engine.advance_projects();
        assert!(engine.state.roads.iter().any(|road| {
            road.stage == Stage::Open
                && road.connected
                && ((road.from == start && road.to == target)
                    || (road.from == target && road.to == start))
        }));
        assert!(engine.state.parcels.len() > initial_parcels);
    }
    assert_eq!(first.save_json(), resumed.save_json());

    let new_parcels: BTreeSet<u32> = first
        .state
        .parcel_frontages
        .iter()
        .filter(|frontage| {
            first.state.roads.iter().any(|road| {
                road.id == frontage.road_id
                    && ((road.from == start && road.to == target)
                        || (road.from == target && road.to == start))
            })
        })
        .map(|frontage| frontage.parcel_id)
        .collect();
    assert!(!new_parcels.is_empty());

    first.plan_building((first.state.tick / TICKS_PER_DAY) as u32);
    let building_project = first
        .state
        .projects
        .iter()
        .find(|candidate| {
            candidate.kind == ProjectKind::Building
                && candidate.progress < 1.0
                && candidate
                    .parcel_id
                    .is_some_and(|id| new_parcels.contains(&id))
        })
        .cloned()
        .expect("the completed chord should provide a buildable, connected lot");
    for _ in 0..=6 {
        if first.state.buildings.iter().any(|building| {
            building.parcel_id == building_project.parcel_id.unwrap()
                && building.stage == Stage::Open
        }) {
            break;
        }
        first.advance_projects();
    }
    let new_building = first
        .state
        .buildings
        .iter()
        .find(|building| building.parcel_id == building_project.parcel_id.unwrap())
        .expect("the frontage building opens after construction");
    assert_eq!(new_building.stage, Stage::Open);
    let (access_road, points) = first
        .building_access_segment(new_building.id)
        .expect("the new building has a real frontage access segment");
    assert!(first.state.roads.iter().any(|road| road.id == access_road));
    assert!(points[0].x.is_finite() && points[1].x.is_finite());
    let walk = first
        .building_access_path(new_building.id, &[start, target], true, new_building.id, 0)
        .expect("pedestrian route joins the door to the connected street");
    assert!(walk.points.len() >= 2);
    let final_local_envelope_area: f32 = first
        .state
        .buildings
        .iter()
        .filter(|building| (building.x - 30.0).hypot(building.z - 300.0) <= 80.0)
        .map(|building| building.width * building.depth)
        .sum();
    assert!(final_local_envelope_area >= initial_local_envelope_area * 1.2);
    first.validate().unwrap();
}

#[test]
#[ignore = "bounded offline geometry probe for mid-edge street access on mature saves"]
fn diagnose_mid_edge_access_feasibility_on_mature_saves() {
    for (name, json) in [
        (
            "seed87231",
            include_str!("../../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"),
        ),
        (
            "seed0",
            include_str!("../../../tests/fixtures/access-joins/ranked-blocks-seed0.json"),
        ),
    ] {
        let engine = Engine::restore_json(json).expect("public save should restore natively");
        let audit = audit_mid_edge_access_candidates(&engine);
        println!(
            "{name}: hosts={} attempted={} rejected={:?} safe_lots_in_top8={} feasible_top8={:?}",
            audit.hosts, audit.attempted, audit.rejected, audit.safe_lot_count, audit.feasible,
        );
    }
}

#[test]
#[ignore = "bounded integration probe for mid-edge splits on mature public saves"]
fn mature_fixture_road_splits_preserve_routes_access_and_replay() {
    let cases = [
        (
            "seed87231-host2517",
            include_str!("../../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"),
            2_517,
            0.5,
        ),
        (
            "seed0-host1739",
            include_str!("../../../tests/fixtures/access-joins/ranked-blocks-seed0.json"),
            1_739,
            0.75,
        ),
        (
            "seed0-host1729",
            include_str!("../../../tests/fixtures/access-joins/ranked-blocks-seed0.json"),
            1_729,
            0.75,
        ),
        (
            "seed0-host2970",
            include_str!("../../../tests/fixtures/access-joins/ranked-blocks-seed0.json"),
            2_970,
            0.75,
        ),
    ];

    for (name, fixture, host_id, t) in cases {
        let mut engine = Engine::restore_json(fixture)
            .unwrap_or_else(|error| panic!("{name}: source fixture did not restore: {error}"));
        let host = engine
            .state
            .roads
            .iter()
            .find(|road| road.id == host_id)
            .unwrap_or_else(|| panic!("{name}: host road is missing"))
            .clone();
        assert!(
            host.stage == Stage::Open && host.connected,
            "{name}: host is not open"
        );
        let before_positions: Vec<_> = engine
            .state
            .travelers
            .iter()
            .map(|traveler| route_position(&engine, traveler))
            .collect();
        let before_access: Vec<_> = engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| frontage.road_id == host_id)
            .filter_map(|frontage| {
                let building = engine
                    .state
                    .buildings
                    .iter()
                    .find(|building| building.parcel_id == frontage.parcel_id)?;
                engine
                    .building_access_segment(building.id)
                    .map(|(_, segment)| (building.id, segment))
            })
            .collect();
        let bus_pose = engine.state.bus_line.as_ref().and_then(|line| {
            engine
                .state
                .buses
                .first()
                .and_then(|bus| engine.bus_pose(line, bus))
        });
        let train_distances: Vec<_> = engine
            .state
            .trains
            .iter()
            .map(|train| train.distance)
            .collect();
        let access_count = before_access.len();

        let outcome = engine
            .try_split_open_road(host_id, t)
            .unwrap_or_else(|error| panic!("{name}: expected safe split, got {error:?}"));
        assert_eq!(outcome.retained_road_id, host_id, "{name}");
        for (index, traveler) in engine.state.travelers.iter().enumerate() {
            let after = route_position(&engine, traveler);
            assert!(
                (before_positions[index].x - after.x).abs() < 0.002
                    && (before_positions[index].z - after.z).abs() < 0.002,
                "{name}: traveler {} moved during topology split",
                traveler.id
            );
        }
        for (building_id, before) in before_access {
            let after = engine
                .building_access_segment(building_id)
                .expect("frontage building keeps a door connector");
            assert!(
                (before[0].x - after.1[0].x).abs() < 0.001
                    && (before[0].z - after.1[0].z).abs() < 0.001
                    && (before[1].x - after.1[1].x).abs() < 0.002
                    && (before[1].z - after.1[1].z).abs() < 0.002,
                "{name}: building {building_id} connector moved"
            );
        }
        if let Some(before) = bus_pose {
            let after = engine
                .bus_pose(
                    engine.state.bus_line.as_ref().unwrap(),
                    engine.state.buses.first().unwrap(),
                )
                .unwrap();
            assert!(
                (before.0 - after.0).abs() < 0.002 && (before.1 - after.1).abs() < 0.002,
                "{name}: bus moved during topology split"
            );
        }
        assert_eq!(
            engine
                .state
                .trains
                .iter()
                .map(|train| train.distance)
                .collect::<Vec<_>>(),
            train_distances,
            "{name}: train progress changed"
        );
        engine
            .validate()
            .unwrap_or_else(|error| panic!("{name}: split save failed validation: {error}"));

        let canonical = engine.save_json();
        let mut resumed = Engine::restore_json(&canonical)
            .unwrap_or_else(|error| panic!("{name}: split save did not restore: {error}"));
        assert_eq!(resumed.save_json(), canonical, "{name}: save not canonical");
        engine.step(2_000);
        resumed.step(2_000);
        assert_eq!(
            engine.save_json(),
            resumed.save_json(),
            "{name}: replay diverged"
        );
        println!(
            "{name}: split {} into {}, travelers={}, frontages_preserved={}",
            outcome.retained_road_id,
            outcome.new_road_id,
            before_positions.len(),
            access_count
        );
    }
}

#[test]
fn interior_access_plan_is_visible_reserved_atomic_and_replayable() {
    let mut engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"
    ))
    .expect("mature public city fixture restores");
    assert!(engine.state.neighborhood_plan.is_none());
    assert!(engine.state.pending_interior_access.is_none());
    engine.state.demand = 1.2;

    let proposal_bound = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .count()
        .saturating_mul(interior_access::PROPOSALS_PER_HOST);
    let evaluations = proposal_bound
        .div_ceil(interior_access::MAX_PROPOSALS_PER_EVALUATION)
        .saturating_add(1);
    let mut admitted = false;
    for _ in 0..evaluations {
        if engine.try_plan_interior_access() {
            admitted = true;
            break;
        }
    }
    assert!(admitted, "mature feasible frontage was never admitted");
    let plan = engine.state.pending_interior_access.clone().unwrap();
    assert!((2..=interior_access::MAX_SELECTED_LOTS).contains(&plan.selected_lots.len()));
    let branch = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == plan.branch_road_id)
        .unwrap();
    assert_eq!(branch.stage, Stage::Survey);
    assert!(!branch.connected);
    let project = engine
        .state
        .projects
        .iter()
        .find(|project| project.id == plan.project_id)
        .unwrap();
    assert_eq!(project.kind, ProjectKind::Road);
    assert!(project.progress < 1.0);
    assert_eq!(branch.progress, project.progress);
    let visible = engine.snapshot();
    assert!(visible
        .roads
        .iter()
        .any(|road| road.id == plan.branch_road_id && road.stage != Stage::Open));
    let parcels_before = engine.state.parcels.len();
    for lot in &plan.selected_lots {
        let rect = geometry::OrientedRect {
            x: lot.x,
            z: lot.z,
            width: lot.width,
            depth: lot.depth,
            rotation: lot.rotation,
        };
        assert!(
            !engine.oriented_parcel_geometry_clear(&rect),
            "reserved lot admitted by another path"
        );
    }
    let first_lot = plan.selected_lots[0];
    let branch = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == plan.branch_road_id)
        .unwrap();
    let branch_a = engine.node(branch.from).unwrap();
    let branch_b = engine.node(branch.to).unwrap();
    let branch_length = (branch_b.x - branch_a.x).hypot(branch_b.z - branch_a.z);
    let along_x = (branch_b.x - branch_a.x) / branch_length;
    let along_z = (branch_b.z - branch_a.z) / branch_length;
    let probe_a = (first_lot.x - along_x * 20.0, first_lot.z - along_z * 20.0);
    let probe_b = (first_lot.x + along_x * 20.0, first_lot.z + along_z * 20.0);
    assert!(
        point_segment_distance(
            first_lot.x,
            first_lot.z,
            branch_a.x,
            branch_a.z,
            branch_b.x,
            branch_b.z,
        ) >= 8.0
    );
    assert!(!engine.interior_plan_lots_clear_of_road(probe_a.0, probe_a.1, probe_b.0, probe_b.1));
    assert_eq!(engine.state.parcels.len(), parcels_before);
    engine.advance_projects();
    let progressed_branch = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == plan.branch_road_id)
        .unwrap();
    let progressed_project = engine
        .state
        .projects
        .iter()
        .find(|project| project.id == plan.project_id)
        .unwrap();
    assert!(progressed_project.progress > 0.0 && progressed_project.progress < 1.0);
    assert_eq!(progressed_branch.progress, progressed_project.progress);
    assert!(
        !progressed_branch.connected,
        "construction branch must not be traversable"
    );
    engine
        .validate()
        .expect("pending plan satisfies restore invariants");

    let save = engine.save_json();
    let mut resumed = Engine::restore_json(&save).expect("pending plan restores canonically");
    assert_eq!(resumed.save_json(), save);
    let mut invalid = Engine::restore_json(&save).unwrap();
    let corrupt_plan = invalid.state.pending_interior_access.as_mut().unwrap();
    corrupt_plan.project_id = u32::MAX;
    assert!(Engine::restore_json(&invalid.save_json()).is_err());
    let mut invalid_fraction = Engine::restore_json(&save).unwrap();
    let plan_t = invalid_fraction
        .state
        .pending_interior_access
        .as_ref()
        .unwrap()
        .host_t;
    invalid_fraction
        .state
        .pending_interior_access
        .as_mut()
        .unwrap()
        .host_t = if (plan_t - 0.25).abs() < 0.001 {
        0.75
    } else {
        0.25
    };
    assert!(Engine::restore_json(&invalid_fraction.save_json()).is_err());
    let mut invalid_branch = Engine::restore_json(&save).unwrap();
    let endpoint = invalid_branch
        .state
        .pending_interior_access
        .as_ref()
        .unwrap()
        .branch_endpoint_id;
    let endpoint_node = invalid_branch
        .state
        .nodes
        .iter_mut()
        .find(|node| node.id == endpoint)
        .unwrap();
    endpoint_node.x += 1.0;
    assert!(Engine::restore_json(&invalid_branch.save_json()).is_err());
    let mut blocked = Engine::restore_json(&save).unwrap();
    let existing = blocked.state.parcels[0].clone();
    let blocked_plan = blocked.state.pending_interior_access.as_mut().unwrap();
    blocked_plan.selected_lots[0].x = existing.x;
    blocked_plan.selected_lots[0].z = existing.z;
    blocked
        .state
        .projects
        .iter_mut()
        .find(|project| project.id == plan.project_id)
        .unwrap()
        .duration = 1.0;
    let blocked_parcel_count = blocked.state.parcels.len();
    blocked.advance_projects();
    assert!(blocked.state.pending_interior_access.is_some());
    assert_eq!(blocked.state.parcels.len(), blocked_parcel_count);
    assert_ne!(
        blocked
            .state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
            .unwrap()
            .stage,
        Stage::Open
    );
    assert!(
        blocked
            .state
            .projects
            .iter()
            .find(|project| project.id == plan.project_id)
            .unwrap()
            .progress
            < 1.0
    );
    for instance in [&mut engine, &mut resumed] {
        let project = instance
            .state
            .projects
            .iter_mut()
            .find(|project| project.id == plan.project_id)
            .unwrap();
        project.duration = 1.0;
        instance.advance_projects();
        instance
            .validate()
            .expect("completed access plan preserves invariants");
    }
    assert_eq!(
        engine.save_json(),
        resumed.save_json(),
        "completion replays identically"
    );
    assert!(engine.state.pending_interior_access.is_none());
    let branch = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == plan.branch_road_id)
        .unwrap();
    assert_eq!(branch.stage, Stage::Open);
    assert!(branch.connected);
    assert_eq!(
        engine.state.parcels.len(),
        parcels_before + plan.selected_lots.len()
    );
    assert_eq!(
        engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| frontage.road_id == plan.branch_road_id)
            .count(),
        plan.selected_lots.len()
    );
}

#[test]
fn interior_access_capacity_preflight_does_not_split_or_allocate() {
    let mut engine = Engine::new_legacy(41);
    engine.state.demand = 1.2;
    engine.state.next_id = u32::MAX - 4;
    let before_nodes = engine.state.nodes.clone();
    let before_roads = engine.state.roads.clone();
    let before_projects = engine.state.projects.clone();
    let before_parcels = engine.state.parcels.clone();
    let before_id = engine.state.next_id;
    let before_cursor = engine.state.interior_access_cursor;
    assert!(!engine.try_plan_interior_access());
    assert_eq!(engine.state.nodes.len(), before_nodes.len());
    assert_eq!(engine.state.roads.len(), before_roads.len());
    assert_eq!(engine.state.projects.len(), before_projects.len());
    assert_eq!(engine.state.parcels.len(), before_parcels.len());
    assert_eq!(engine.state.next_id, before_id);
    assert_eq!(engine.state.interior_access_cursor, before_cursor);
    assert!(engine.state.pending_interior_access.is_none());
}

#[test]
fn interior_access_proposal_scan_is_read_only_and_leaves_admission_unchanged() {
    let mut engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"
    ))
    .expect("mature public city fixture restores");
    engine.state.demand = 1.2;
    let proposal_bound = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .count()
        .saturating_mul(interior_access::PROPOSALS_PER_HOST);
    let scan_count = proposal_bound
        .div_ceil(interior_access::MAX_PROPOSALS_PER_EVALUATION)
        .saturating_add(1);
    let mut found_proposal = false;
    for _ in 0..scan_count {
        let before = engine.save_json();
        let before_rng = engine.state.rng_growth;
        let before_id = engine.state.next_id;
        let before_cursor = engine.state.interior_access_cursor;
        let scan = engine.propose_interior_access();
        assert_eq!(
            engine.save_json(),
            before,
            "candidate enumeration mutated serialized state"
        );
        assert_eq!(engine.state.rng_growth, before_rng);
        assert_eq!(engine.state.next_id, before_id);
        assert_eq!(engine.state.interior_access_cursor, before_cursor);
        if !scan.proposals.is_empty() {
            found_proposal = true;
            break;
        }
        engine.state.interior_access_cursor = scan.next_cursor;
    }
    assert!(
        found_proposal,
        "fixture should expose a valid bounded proposal"
    );

    let mut planner = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"
    ))
    .expect("mature public city fixture restores");
    planner.state.demand = 1.2;
    let revision_before_admission = planner.state.access_geometry_revision;
    let mut admitted = false;
    for _ in 0..scan_count {
        if planner.try_plan_interior_access() {
            admitted = true;
            break;
        }
    }
    assert!(
        admitted,
        "production planner should admit an enumerated proposal"
    );
    assert!(planner.state.access_geometry_revision > revision_before_admission);
    assert!(planner.state.pending_interior_access.is_some());
    planner
        .validate()
        .expect("admitted proposal remains connected and restorable");
    let saved = planner.save_json();
    let mut restored = Engine::restore_json(&saved).expect("admission state restores");
    planner.step(2_000);
    restored.step(2_000);
    assert_eq!(planner.save_json(), restored.save_json());
}

#[test]
fn access_geometry_revision_tracks_effective_writers_and_ignores_ticks() {
    let mut engine = Engine::new_legacy(31);
    let initial = engine.state.access_geometry_revision;

    // Snapshot/weather activity is outside the frozen access geometry.
    engine.state.tick = engine.state.tick.saturating_add(40);
    engine.state.weather_intensity = 0.4;
    engine.state.revision = engine.state.revision.saturating_add(1);
    engine.recompute_connectivity();
    assert_eq!(engine.state.access_geometry_revision, initial);

    let node = engine.add_node(260.0, 260.0, NodeKind::Junction);
    let after_node = engine.state.access_geometry_revision;
    assert!(after_node > initial);
    let road = engine.add_road(1, node, RoadKind::Local, Stage::Open, 1.0);
    let after_road = engine.state.access_geometry_revision;
    assert!(after_road > after_node);
    engine.recompute_connectivity();
    assert!(engine.state.access_geometry_revision > after_road);

    let parcel = engine.add_parcel(255.0, 250.0, 8.0, 8.0, node, 0.7, false);
    let after_parcel = engine.state.access_geometry_revision;
    assert!(after_parcel > after_road);
    let before_frontage = engine.state.access_geometry_revision;
    engine.remember_nearest_frontage(parcel, node);
    assert!(engine.state.access_geometry_revision > before_frontage);
    assert!(engine
        .state
        .parcel_frontages
        .iter()
        .any(|frontage| frontage.parcel_id == parcel));

    // The normal building planner reserves a parcel. That reservation
    // changes which sites a concurrent access proposal may consume.
    let before_building_reservation = engine.state.access_geometry_revision;
    engine.plan_building(1);
    assert!(engine.state.access_geometry_revision > before_building_reservation);

    let building_project = engine
        .state
        .projects
        .iter()
        .rev()
        .find(|project| project.kind == ProjectKind::Building && project.progress < 1.0)
        .unwrap()
        .clone();
    let before_reservation_release = engine.state.access_geometry_revision;
    engine.complete_project(building_project.id);
    assert!(engine.state.access_geometry_revision > before_reservation_release);
}

#[test]
fn access_geometry_revision_tracks_road_opening_and_plan_reservations() {
    let mut engine = Engine::new_legacy(33);
    let bridge_project = engine
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Road)
        .unwrap()
        .id;
    let before_open = engine.state.access_geometry_revision;
    engine.complete_project(bridge_project);
    assert!(engine.state.access_geometry_revision > before_open);

    let mut planner = engine_with_developed_plan_site_without_reusable_edge();
    let before_plan = planner.state.access_geometry_revision;
    assert!(planner.try_create_neighborhood_plan());
    assert!(planner.state.access_geometry_revision > before_plan);
    planner.validate().unwrap();
}

#[test]
fn access_geometry_revision_is_backward_compatible_and_fails_closed_at_overflow() {
    let engine = Engine::new_legacy(32);
    let mut old_save: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    let state = old_save.as_object_mut().unwrap();
    state.remove("access_geometry_revision");
    state.remove("access_geometry_revision_exhausted");
    let restored = Engine::restore_json(&old_save.to_string()).unwrap();
    assert_eq!(restored.state.access_geometry_revision, 0);
    assert!(!restored.state.access_geometry_revision_exhausted);
    assert_eq!(restored.access_geometry_revision_token(), Some(0));

    let mut exhausted = restored;
    exhausted.state.access_geometry_revision = u64::MAX;
    assert_eq!(exhausted.access_geometry_revision_token(), Some(u64::MAX));
    exhausted.add_node(0.0, 0.0, NodeKind::Junction);
    assert!(exhausted.state.access_geometry_revision_exhausted);
    assert_eq!(exhausted.state.access_geometry_revision, u64::MAX);
    assert_eq!(exhausted.access_geometry_revision_token(), None);
    exhausted.add_node(1.0, 1.0, NodeKind::Junction);
    assert_eq!(exhausted.access_geometry_revision_token(), None);
    assert!(exhausted.validate().is_ok());
}
