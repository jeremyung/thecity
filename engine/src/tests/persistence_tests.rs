use super::*;

#[test]
fn building_picker_bounds_window_wraps_cursor_and_replays() {
    let (mut engine, access_node) = building_picker_engine();
    let mut candidate_ids = Vec::new();
    for index in 0..40 {
        let x = 290.0 + (index % 8) as f32 * 13.0;
        let z = 280.0 + (index / 8) as f32 * 13.0;
        candidate_ids.push(engine.add_parcel(x, z, 8.0, 8.0, access_node, 0.8, false));
    }
    engine.state.growth_cursor = 38;

    engine.plan_building(1);
    assert_eq!(engine.state.growth_cursor, 30);
    let selected = engine
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Building)
        .and_then(|project| project.parcel_id)
        .expect("one building project should be planned");
    let selected_index = candidate_ids.iter().position(|id| *id == selected).unwrap();
    assert!(selected_index == 38 || selected_index == 39 || selected_index < 30);

    let saved = engine.save_json();
    let mut replay = Engine::restore_json(&saved).unwrap();
    engine.plan_building(1);
    replay.plan_building(1);
    assert_eq!(engine.save_json(), replay.save_json());
    assert_eq!(engine.state.growth_cursor, replay.state.growth_cursor);
}

#[test]
fn admitted_interior_branch_with_exact_clearance_restores_without_radial_rejection() {
    // This is an organically generated public save captured while an
    // interior-access project is pending. A later infill recipe changes
    // growth timing, so keep the exact-clearance regression tied to the
    // concrete admitted corridor instead of a seed's incidental schedule.
    let saved = include_str!(
        "../../../tests/fixtures/access-joins/interior-access-verified-87231.json.project-3736.pending.json"
    );
    let engine = Engine::restore_json(saved).expect("recorded organic branch restores");
    let plan = engine
        .state
        .pending_interior_access
        .as_ref()
        .expect("fixture captures a pending access plan");
    let branch_id = plan.branch_road_id;
    let branch = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == branch_id)
        .expect("pending branch retains its road record");
    let (a, b) = (
        engine.node(branch.from).unwrap(),
        engine.node(branch.to).unwrap(),
    );
    let context = engine.collision_context();
    assert!(parcel_envelopes_clear_of_road(&context, a.x, a.z, b.x, b.z));

    // The pending branch save proves exact-clearance plans survive a
    // canonical save/restore. Keep a separate concrete geometry probe for
    // the old max-dimension circle false rejection: its 20x12 envelope is
    // 5.2m from the road, while the old 15m radius rejects its 11.2m
    // center distance.
    let radial_probe = geometry::OrientedRect {
        x: 0.0,
        z: 11.2,
        width: 20.0,
        depth: 12.0,
        rotation: 0.0,
    };
    let radial_probe_context = CollisionContext {
        parcel_footprints: vec![radial_probe],
        open_roads: Vec::new(),
        pending_roads: Vec::new(),
    };
    assert!(
        point_segment_distance(0.0, 11.2, -40.0, 0.0, 40.0, 0.0)
            <= radial_probe.width.max(radial_probe.depth) * 0.5 + 5.0,
        "legacy radial rule should reject the probe"
    );
    assert!(parcel_envelopes_clear_of_road(
        &radial_probe_context,
        -40.0,
        0.0,
        40.0,
        0.0,
    ));

    let canonical = engine.save_json();
    let restored = Engine::restore_json(&canonical)
        .expect("exactly clear admitted access plan should restore");
    assert_eq!(restored.save_json(), canonical);
    let mut completed = restored;
    for _ in 0..12 {
        if completed
            .state
            .roads
            .iter()
            .find(|road| road.id == branch_id)
            .is_some_and(|road| road.stage == Stage::Open && road.connected)
        {
            break;
        }
        completed.advance_projects();
    }
    let branch = completed
        .state
        .roads
        .iter()
        .find(|road| road.id == branch_id)
        .expect("completed branch keeps its road identity");
    assert_eq!(branch.stage, Stage::Open);
    assert!(branch.connected);
    let (a, b) = (
        completed.node(branch.from).unwrap(),
        completed.node(branch.to).unwrap(),
    );
    let opened_context = completed.collision_context();
    assert!(parcel_envelopes_clear_of_road(
        &opened_context,
        a.x,
        a.z,
        b.x,
        b.z,
    ));
    completed.validate().expect("opened branch remains valid");
}

#[test]
fn legacy_saves_keep_endpoint_walkers_and_all_parcel_paths_obey_the_cap() {
    let mut engine = Engine::new_legacy(49);
    let founder = engine
        .state
        .buildings
        .iter()
        .find(|building| building.anchor)
        .unwrap()
        .clone();
    let parcel = engine
        .state
        .parcels
        .iter()
        .find(|parcel| parcel.id == founder.parcel_id)
        .unwrap();
    let access_node = parcel.access_node;
    let station = engine
        .state
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Station)
        .unwrap()
        .id;
    let route = engine.route(parcel.access_node, station, false);
    let mut traveler = Traveler {
        id: 700_112,
        kind: 0,
        route_lengths: engine.route_lengths(&route),
        route,
        edge_index: 0,
        edge_progress: 0.0,
        speed: 1.5,
        destination: station,
        activity: TravelerActivity::Commute,
        resident: 0,
        name: "Legacy Walker".into(),
        origin_building: Some(founder.id),
        destination_building: None,
        wait_ticks: 0,
        origin_access: None,
        destination_access: None,
        access_phase: 0,
        access_progress: 0.0,
    };
    traveler.origin_access = engine.building_access_path(
        founder.id,
        &traveler.route,
        true,
        traveler.id,
        traveler.kind,
    );
    engine.state.travelers.push(traveler);
    let mut old_save: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    let state = old_save.as_object_mut().unwrap();
    state.remove("parcel_frontages");
    state.remove("frontage_road_cursor");
    state.remove("frontage_slot_cursor");
    state.remove("road_origin_cursor");
    for traveler in state["travelers"].as_array_mut().unwrap() {
        let traveler = traveler.as_object_mut().unwrap();
        traveler.remove("origin_access");
        traveler.remove("destination_access");
        traveler.remove("access_phase");
        traveler.remove("access_progress");
    }
    for resident in state["residents"].as_array_mut().unwrap() {
        resident.as_object_mut().unwrap().remove("queue_slot");
    }
    let restored = Engine::restore_json(&old_save.to_string()).unwrap();
    let restored_traveler = &restored.state.travelers[0];
    assert_eq!(restored_traveler.access_phase, 1);
    assert!(restored_traveler.origin_access.is_none());
    assert!(restored.state.parcel_frontages.is_empty());
    assert_eq!(restored.state.frontage_road_cursor, 0);
    assert_eq!(restored.state.frontage_slot_cursor, 0);
    assert_eq!(restored.state.road_origin_cursor, 0);
    let movement = restored.movement_buffer();
    assert_eq!(movement[0] as u32, restored_traveler.id);
    let door = engine.state.travelers[0]
        .origin_access
        .as_ref()
        .unwrap()
        .points[0];
    assert!((movement[1] - door.x).hypot(movement[2] - door.z) > 1.0);

    while engine.state.parcels.len() < MAX_PARCELS {
        let _ = engine.add_parcel(0.0, 0.0, 1.0, 1.0, access_node, 0.5, false);
    }
    assert_eq!(
        engine.add_parcel(1.0, 1.0, 1.0, 1.0, access_node, 0.5, false),
        0
    );
    assert_eq!(engine.state.parcels.len(), MAX_PARCELS);
}

#[test]
fn seed_87231_natural_growth_uses_a_saved_partial_shared_edge_layout() {
    let mut engine = Engine::new_legacy(87_231);
    engine.set_development_rate(16);
    let mut observed_shared_plan = false;
    for _ in 0..121 {
        engine.step(2_000);
        if let Some(plan) = engine.state.neighborhood_plan.as_ref() {
            if let Some(selected) = plan.selected_lots.as_ref() {
                assert!(selected.len() >= 4);
                assert!(selected.len() < 10, "existing frontage should be retained");
                engine.validate().unwrap();
                let save = engine.save_json();
                assert_eq!(Engine::restore_json(&save).unwrap().save_json(), save);
                observed_shared_plan = true;
                break;
            }
        }
    }
    assert!(
        observed_shared_plan,
        "seed 87231 should naturally select collision-free lots beside an existing road"
    );
}

#[test]
fn save_restore_preserves_residents_and_active_trips() {
    let mut engine = Engine::new_legacy(91);
    engine.set_development_rate(60);
    for _ in 0..12 {
        engine.step(2_000);
    }
    assert!(!engine.state.residents.is_empty());
    // Capture an active named journey, not only the resident roster and deadlines.
    for _ in 0..120 {
        engine.step(200);
        if !engine.state.travelers.is_empty() {
            break;
        }
    }
    assert!(
        !engine.state.travelers.is_empty(),
        "no scheduled trip became active"
    );
    let active_id = engine.state.travelers[0].id;
    let inspected: serde_json::Value =
        serde_json::from_str(&engine.inspect_json(active_id)).unwrap();
    assert_eq!(inspected["entityType"], "traveler");
    assert!(inspected["name"].as_str().is_some());
    assert!(inspected["activity"].as_str().is_some());
    let save = engine.save_json();
    let mut restored = Engine::restore_json(&save).unwrap();
    assert_eq!(restored.save_json(), save);
    engine.step(2_000);
    restored.step(2_000);
    assert_eq!(restored.save_json(), engine.save_json());
    engine.validate().unwrap();
}

#[test]
fn save_restore_replays_pending_projects() {
    let mut a = Engine::new_legacy(8);
    a.step(1_337);
    assert!(a
        .state
        .projects
        .iter()
        .any(|p| p.progress > 0.0 && p.progress < 1.0));
    let save = a.save_json();
    let mut b = Engine::restore_json(&save).unwrap();
    a.step(2_000);
    b.step(2_000);
    assert_eq!(a.save_json(), b.save_json());
}

#[test]
fn incompatible_activity_saves_are_rejected_without_migration() {
    let engine = Engine::new_legacy(604);
    let current = engine.save_json();
    assert_eq!(Engine::restore_json(&current).unwrap().save_json(), current);
    for version in [0, ACTIVITY_SCHEMA_VERSION - 1, ACTIVITY_SCHEMA_VERSION + 1] {
        let mut save: serde_json::Value = serde_json::from_str(&current).unwrap();
        save["activity_schema_version"] = serde_json::json!(version);
        assert!(Engine::restore_json(&save.to_string()).is_err());
    }
    let mut missing: serde_json::Value = serde_json::from_str(&current).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("neighborhood_errands");
    assert!(Engine::restore_json(&missing.to_string()).is_err());
}
