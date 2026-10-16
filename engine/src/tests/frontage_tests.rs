use super::*;

#[test]
fn building_picker_prefers_connected_infill_and_excludes_invalid_sites() {
    let (mut engine, access_node) = building_picker_engine();
    let isolated = engine.add_parcel(350.0, 350.0, 10.0, 10.0, access_node, 0.8, false);
    let infill = engine.add_parcel(216.0, 232.0, 10.0, 10.0, access_node, 0.8, false);
    let protected = engine.add_parcel(240.0, 240.0, 10.0, 10.0, access_node, 0.8, true);
    let disconnected_node = engine.add_node(390.0, 390.0, NodeKind::Junction);
    let disconnected = engine.add_parcel(380.0, 380.0, 10.0, 10.0, disconnected_node, 0.8, false);

    // The former sort-then-cycle picker selected the isolated site at
    // cursor 1 despite having ranked the infill site first.
    engine.state.growth_cursor = 1;
    engine.plan_building(1);
    let project = engine
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Building)
        .expect("one building project should be planned");
    assert_eq!(project.parcel_id, Some(infill));
    assert_ne!(project.parcel_id, Some(isolated));
    assert_ne!(project.parcel_id, Some(protected));
    assert_ne!(project.parcel_id, Some(disconnected));

    let saved = engine.save_json();
    let mut replay = Engine::restore_json(&saved).unwrap();
    engine.step(400);
    replay.step(400);
    assert_eq!(engine.save_json(), replay.save_json());
}

#[test]
fn road_frontage_benefit_accepts_two_sites_on_one_side() {
    let mut engine = Engine::new_legacy(731);
    // A long existing parcel blocks the north side while leaving the south
    // frontage unobstructed. Two same-side lots still represent useful
    // benefit for a road proposal.
    engine.add_parcel(315.0, 315.0, 84.0, 12.0, 1, 0.5, false);
    let a = Node {
        id: u32::MAX - 1,
        x: 280.0,
        z: 300.0,
        kind: NodeKind::Candidate,
    };
    let b = Node {
        id: u32::MAX,
        x: 350.0,
        z: 300.0,
        kind: NodeKind::Candidate,
    };

    assert_eq!(engine.viable_frontage_benefit_count(&a, &b, 2), 2);
}

#[test]
fn compact_infill_continuation_restores_with_exact_legacy_road_clearance() {
    let mut engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/infill-oriented-continued.json"
    ))
    .expect("oriented infill fixture restores before continuation");
    let start_tick = engine.state.tick;
    for _ in 0..32 {
        engine.step(2_000);
    }
    assert_eq!(engine.state.tick - start_tick, 64_000);
    engine
        .validate()
        .expect("continued world passes exact geometry");
    let save = engine.save_json();
    let mut restored =
        Engine::restore_json(&save).expect("compact infill continuation canonical-restores");
    assert_eq!(restored.save_json(), save);
    engine.step(2_000);
    restored.step(2_000);
    assert_eq!(engine.save_json(), restored.save_json());
}

#[test]
fn exhausted_infill_quota_does_not_block_road_growth_or_skip_available_lots() {
    fn developed_engine() -> Engine {
        let mut engine = Engine::new_legacy(5);
        engine.set_development_rate(60);
        for _ in 0..6 {
            engine.step(2_000);
        }
        engine.state.projects.clear();
        engine.state.tick = engine.state.tick.div_ceil(80) * 80;
        engine.state.demand = 4.0;
        engine.state.road_infill_due = 1;
        engine
    }

    // A lot reserved for a future station tower is not currently buildable
    // while the station is offline. It must not leave the infill quota stuck
    // forever and suppress ordinary road planning.
    let mut tower_only = developed_engine();
    tower_only.state.station_online = false;
    let station = tower_only
        .state
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Station)
        .unwrap()
        .clone();
    let access_node = tower_only
        .state
        .roads
        .iter()
        .find(|road| road.stage == Stage::Open && road.connected && road.kind != RoadKind::Rail)
        .unwrap()
        .from;
    let tower_parcel = tower_only.add_parcel(
        station.x,
        station.z + 30.0,
        STATION_TOWER_PARCEL_WIDTH,
        STATION_TOWER_PARCEL_DEPTH,
        access_node,
        1.0,
        false,
    );
    tower_only
        .state
        .parcels
        .retain(|parcel| parcel.id == tower_parcel);
    let initial_parcel_count = tower_only.state.parcels.len();
    assert!(tower_only.is_station_tower_lot(&tower_only.state.parcels[0]));
    assert!(!tower_only.state.station_online);
    assert!(tower_only.node_connected(access_node));
    tower_only.evaluate_growth();
    assert_eq!(tower_only.state.road_infill_due, 0);
    let road_target = tower_only
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        .expect("exhausted infill must yield to the reserved neighborhood road")
        .to
        .unwrap();
    // Coordinated plans construct their road before releasing its frontage.
    // Exercise completion through the production scheduler, not another
    // immediate evaluate_growth call on the same tick.
    tower_only.step(800);
    assert!(
        tower_only
            .state
            .roads
            .iter()
            .any(|road| { road.to == road_target && road.stage == Stage::Open && road.connected }),
        "the admitted neighborhood road must complete and connect"
    );
    assert!(tower_only.state.parcels.len() > initial_parcel_count);
    assert!(
        tower_only.state.buildings.iter().any(|building| {
            tower_only.state.parcels.iter().any(|parcel| {
                parcel.id == building.parcel_id
                    && parcel.id != tower_parcel
                    && !tower_only.is_station_tower_lot(parcel)
                    && tower_only.node_connected(parcel.access_node)
            })
        }),
        "ordinary frontage must enter construction after the road opens"
    );

    // If an ordinary usable lot exists alongside an ineligible tower lot,
    // the scheduler should spend the quota on that lot before road growth.
    let mut mixed = developed_engine();
    mixed.state.station_online = false;
    // Reuse an actual developed lot with a resolved access segment rather
    // than inventing a disconnected lot far from its declared access node.
    let usable_building = mixed
        .state
        .buildings
        .iter()
        .find(|building| {
            mixed.state.parcels.iter().any(|parcel| {
                parcel.id == building.parcel_id
                    && !parcel.protected
                    && parcel.reservation.is_none()
                    && !mixed.is_station_tower_lot(parcel)
                    && mixed.node_connected(parcel.access_node)
                    && mixed.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
            }) && mixed.building_access_segment(building.id).is_some()
        })
        .expect("developed fixture must contain an accessible ordinary lot")
        .clone();
    let usable = usable_building.parcel_id;
    let connected = mixed
        .state
        .parcels
        .iter()
        .find(|p| p.id == usable)
        .unwrap()
        .access_node;
    mixed
        .state
        .buildings
        .retain(|building| building.id != usable_building.id);
    let ineligible = mixed.add_parcel(
        station.x,
        station.z + 30.0,
        STATION_TOWER_PARCEL_WIDTH,
        STATION_TOWER_PARCEL_DEPTH,
        connected,
        1.0,
        false,
    );
    mixed
        .state
        .parcels
        .retain(|parcel| parcel.id == usable || parcel.id == ineligible);
    assert!(!mixed.state.station_online);
    assert!(mixed.state.parcels.iter().any(|parcel| {
        parcel.id == usable
            && !mixed.is_station_tower_lot(parcel)
            && mixed.node_connected(parcel.access_node)
    }));
    mixed.evaluate_growth();
    assert!(
        mixed.state.projects.iter().any(|project| {
            project.kind == ProjectKind::Building
                && project.parcel_id == Some(usable)
                && project.progress < 1.0
        }),
        "usable ordinary frontage was bypassed"
    );
    assert_eq!(mixed.state.road_infill_due, 0);
    assert!(
        !mixed
            .state
            .projects
            .iter()
            .any(|project| { project.kind == ProjectKind::Road && project.progress < 1.0 }),
        "road growth bypassed an ordinary lot while infill was due"
    );
}

#[test]
fn frontage_cursor_accepts_exact_candidate_cap_and_rejects_overflow() {
    let mut engine = Engine::new_legacy(17);
    engine.state.frontage_slot_cursor = MAX_FRONTAGE_SITES_PER_ROAD as u16;
    let at_cap = Engine::restore_json(&engine.save_json()).unwrap();
    assert_eq!(
        at_cap.state.frontage_slot_cursor as usize,
        MAX_FRONTAGE_SITES_PER_ROAD
    );

    engine.state.frontage_slot_cursor = MAX_FRONTAGE_SITES_PER_ROAD as u16 + 1;
    assert!(Engine::restore_json(&engine.save_json()).is_err());

    let a = Node {
        id: 10_001,
        x: 200.0,
        z: -160.0,
        kind: NodeKind::Junction,
    };
    let b = Node {
        id: 10_002,
        x: 200.0,
        z: 160.0,
        kind: NodeKind::Junction,
    };
    let sites = at_cap.frontage_candidate_sites_with_layout(&a, &b, 14.0, 12.0, 18.0);
    // This helper returns only fixed-grid sites. Compact gap candidates
    // are appended separately by progressive infill.
    assert_eq!(sites.len(), MAX_FRONTAGE_GRID_SITES_PER_ROAD);
}

#[test]
fn frontage_gap_default_layout_is_exactly_the_legacy_candidate_sequence() {
    let engine = Engine::new_legacy(71);
    let a = Node {
        id: 10,
        x: 0.0,
        z: 0.0,
        kind: NodeKind::Junction,
    };
    let b = Node {
        id: 20,
        x: 0.0,
        z: 100.0,
        kind: NodeKind::Junction,
    };
    let context = CollisionContext {
        parcel_footprints: Vec::new(),
        open_roads: Vec::new(),
        pending_roads: Vec::new(),
    };
    let save = engine.save_json();
    let actual = engine.frontage_gap_candidates(&a, &b, &context, &[]);
    let legacy_intervals = [
        (9.0_f32, 20.833333_f32),
        (23.033333_f32, 34.866665_f32),
        (37.066666_f32, 48.9_f32),
        (51.1_f32, 62.933334_f32),
        (65.13333_f32, 76.96667_f32),
        (79.16667_f32, 91.0_f32),
    ];
    let mut expected = Vec::new();
    for side in [-1_i8, 1] {
        for (start, end) in legacy_intervals {
            let along_m = (start + end) * 0.5;
            let along = along_m / 100.0;
            expected.push((
                (
                    -side as f32 * FRONTAGE_GAP_OFFSET,
                    along_m,
                    frontage_access_node(&a, &b, along),
                    along,
                    side,
                ),
                end - start,
            ));
        }
    }
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert!((actual.0 .0 - expected.0 .0).abs() < 1e-4);
        assert!((actual.0 .1 - expected.0 .1).abs() < 1e-4);
        assert_eq!(actual.0 .2, expected.0 .2);
        assert!((actual.0 .3 - expected.0 .3).abs() < 1e-5);
        assert_eq!(actual.0 .4, expected.0 .4);
        assert!((actual.1 - expected.1).abs() < 1e-4);
    }
    assert_eq!(
        engine.save_json(),
        save,
        "candidate generation is read-only"
    );
}

#[test]
fn shallow_frontage_fallback_preserves_existing_prefix_and_bounded_shapes() {
    let engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/spine-ranking-seed-87231-by-sites-false.json"
    ))
    .expect("mature fixture restores");
    let context = engine.collision_context();
    let towers = engine.available_station_tower_envelopes();
    let road = engine
        .state
        .roads
        .iter()
        .find(|road| road.stage == Stage::Open && road.connected && road.kind == RoadKind::Local)
        .unwrap();
    let (a, b) = (
        engine.node(road.from).unwrap(),
        engine.node(road.to).unwrap(),
    );
    let (grid_width, grid_depth, grid_spacing) = if engine.is_neighborhood_plan_road(road.id) {
        (
            NEIGHBORHOOD_PARCEL_SIZE,
            NEIGHBORHOOD_PARCEL_SIZE,
            NEIGHBORHOOD_FRONTAGE_SPACING,
        )
    } else {
        (14.0, 12.0, FRONTAGE_SLOT_SPACING)
    };
    let mut grid =
        engine.frontage_candidate_sites_with_layout(a, b, grid_width, grid_depth, grid_spacing);
    let tower_candidate = engine.station_tower_candidate_index(road.id, a, b, &grid);
    if let Some(index) = tower_candidate {
        grid.swap(0, index);
    }
    let standard = engine.frontage_gap_candidates(a, b, &context, &towers);
    let shallow = engine.frontage_gap_candidates_with_layout(
        a,
        b,
        &context,
        &towers,
        FRONTAGE_SHALLOW_DEPTH,
        FRONTAGE_SHALLOW_OFFSET,
    );
    assert!(shallow.len() <= 2 * MAX_FRONTAGE_GAP_LOTS_PER_SIDE);
    let (all, dimensions, has_tower_candidate) = engine.frontage_candidates_with_shallow_fallback(
        road.id,
        a,
        b,
        &context,
        &towers,
        grid_width,
        grid_depth,
        grid_spacing,
    );
    assert_eq!(has_tower_candidate, tower_candidate.is_some());
    assert!(all.len() <= MAX_FRONTAGE_SITES_PER_ROAD);

    assert_eq!(&all[..grid.len()], grid.as_slice());
    let standard_start = grid.len();
    let old_prefix_len = standard_start + standard.len();
    assert_eq!(
        &all[standard_start..old_prefix_len],
        standard
            .iter()
            .map(|(site, _)| *site)
            .collect::<Vec<_>>()
            .as_slice()
    );
    // The standard grid and gap candidates are an unchanged prefix in
    // production; the shallow recipe only appends after them.
    assert!(old_prefix_len <= all.len());
    for ((_, _, _, _, _), (width, depth)) in all
        .iter()
        .skip(old_prefix_len)
        .zip(dimensions.iter().skip(old_prefix_len))
    {
        assert!((8.0..=14.0).contains(width));
        assert!(
            *depth == FRONTAGE_SHALLOW_DEPTH || (*width == 8.0 && *depth == FRONTAGE_GAP_DEPTH)
        );
    }
    let mut shapes = BTreeSet::new();
    for ((x, z, _, _, _), (width, depth)) in all.iter().zip(dimensions.iter()) {
        let key = (
            (x * 10_000.0).round() as i64,
            (z * 10_000.0).round() as i64,
            (width * 10_000.0).round() as i64,
            (depth * 10_000.0).round() as i64,
        );
        assert!(shapes.insert(key), "duplicate candidate shape {key:?}");
    }
    assert!(all.len() <= MAX_FRONTAGE_SITES_PER_ROAD);
}

#[test]
#[ignore = "20k-tick mature-save continuation and construction audit"]
fn shallow_frontage_mature_continuations_construct_and_replay_pending_lots() {
    use serde_json::json;

    let fixtures = [
        (
            0_u32,
            include_str!(
                "../../../tests/fixtures/access-joins/spine-ranking-seed-0-by-sites-false.json"
            ),
        ),
        (
            87_231_u32,
            include_str!(
                "../../../tests/fixtures/access-joins/spine-ranking-seed-87231-by-sites-false.json"
            ),
        ),
    ];
    for (seed, save) in fixtures {
        let mut engine = Engine::restore_json(save).expect("area-policy save restores");
        engine.set_development_rate(16);
        let start_tick = engine.state.tick;
        let initial_roads = engine.state.roads.len();
        let initial_max_parcel_id = engine
            .state
            .parcels
            .iter()
            .map(|parcel| parcel.id)
            .max()
            .unwrap_or(0);
        let mut pending_replay_checked = false;

        for _ in 0..(20_000 / 200) {
            engine.step(200);
            let shallow_ids: BTreeSet<u32> = engine
                .state
                .parcels
                .iter()
                .filter(|parcel| {
                    parcel.id > initial_max_parcel_id
                        && (parcel.depth - FRONTAGE_SHALLOW_DEPTH).abs() < 1.0e-4
                })
                .map(|parcel| parcel.id)
                .collect();
            if !pending_replay_checked
                && engine.state.projects.iter().any(|project| {
                    project.kind == ProjectKind::Building
                        && project.progress < 1.0
                        && project
                            .parcel_id
                            .is_some_and(|id| shallow_ids.contains(&id))
                })
            {
                let pending_save = engine.save_json();
                let mut resumed = Engine::restore_json(&pending_save)
                    .expect("pending shallow construction restores");
                assert_eq!(resumed.save_json(), pending_save);
                let mut twin = Engine::restore_json(&pending_save).unwrap();
                resumed.step(400);
                twin.step(400);
                assert_eq!(resumed.save_json(), twin.save_json());
                pending_replay_checked = true;
            }
        }

        assert_eq!(engine.state.tick, start_tick + 20_000);
        let shallow_parcels: Vec<&Parcel> = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| {
                parcel.id > initial_max_parcel_id
                    && (parcel.depth - FRONTAGE_SHALLOW_DEPTH).abs() < 1.0e-4
            })
            .collect();
        let shallow_ids: BTreeSet<u32> = shallow_parcels.iter().map(|parcel| parcel.id).collect();
        let shallow_buildings: Vec<&Building> = engine
            .state
            .buildings
            .iter()
            .filter(|building| shallow_ids.contains(&building.parcel_id))
            .collect();
        let open_shallow = shallow_buildings
            .iter()
            .filter(|building| building.stage == Stage::Open)
            .count();
        assert!(
            !shallow_parcels.is_empty(),
            "seed {seed}: no fallback parcel admitted"
        );
        assert!(
            open_shallow > 0,
            "seed {seed}: shallow lots did not construct normally"
        );
        assert!(
            pending_replay_checked,
            "seed {seed}: no pending shallow project was captured"
        );
        assert!(shallow_parcels
            .iter()
            .all(|parcel| engine.node_connected(parcel.access_node)));
        assert!(shallow_buildings
            .iter()
            .all(|building| engine.building_access_segment(building.id).is_some()));
        engine
            .validate()
            .expect("shallow continuation remains valid");

        let summary = json!({
            "seed": seed,
            "start_tick": start_tick,
            "end_tick": engine.state.tick,
            "development_rate": engine.state.development_rate,
            "initial_roads": initial_roads,
            "final_roads": engine.state.roads.len(),
            "road_delta": engine.state.roads.len().saturating_sub(initial_roads),
            "shallow_parcels_added": shallow_parcels.len(),
            "shallow_nominal_envelope_area_m2": shallow_parcels.iter().map(|parcel| (parcel.width * parcel.depth) as f64).sum::<f64>(),
            "shallow_buildings": shallow_buildings.len(),
            "shallow_open_buildings": open_shallow,
            "shallow_construction_projects_pending": engine.state.projects.iter().filter(|project| project.kind == ProjectKind::Building && project.progress < 1.0 && project.parcel_id.is_some_and(|id| shallow_ids.contains(&id))).count(),
            "canonical_pending_project_replay_checked": pending_replay_checked,
            "validation_passed": true,
        });
        let validation_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation");
        std::fs::write(
            validation_dir.join(format!("frontage-shallow-seed-{seed}-summary.json")),
            serde_json::to_vec_pretty(&summary).unwrap(),
        )
        .unwrap();
        std::fs::write(
            validation_dir.join(format!("frontage-shallow-seed-{seed}-native-save.json")),
            engine.save_json(),
        )
        .unwrap();
        std::fs::write(
            validation_dir.join(format!("frontage-shallow-seed-{seed}-snapshot.json")),
            engine.snapshot_json(),
        )
        .unwrap();
        println!("{}", serde_json::to_string(&summary).unwrap());
    }
}

#[test]
#[ignore = "matched mature-world shallow-frontage geometry diagnostic"]
fn shallow_frontage_gap_diagnostic_on_area_policy_mature_worlds() {
    use serde_json::json;

    let fixtures = [
        (
            0_u32,
            include_str!(
                "../../../tests/fixtures/access-joins/spine-ranking-seed-0-by-sites-false.json"
            ),
        ),
        (
            87_231_u32,
            include_str!(
                "../../../tests/fixtures/access-joins/spine-ranking-seed-87231-by-sites-false.json"
            ),
        ),
    ];
    let mut worlds = Vec::new();
    for (seed, save) in fixtures {
        let engine = Engine::restore_json(save).expect("area-policy save restores");
        assert!(engine.state.parcels.len() < MAX_PARCELS);
        assert!(engine.state.buildings.len() < MAX_BUILDINGS);
        let towers = engine.available_station_tower_envelopes();
        let base_context = engine.collision_context();
        let evaluate = |depth: f32, offset: f32| {
            let mut context = base_context.clone();
            let mut generated = 0usize;
            let mut tower_blocked = 0usize;
            let mut geometry_blocked = BTreeMap::<String, usize>::new();
            let mut neighborhood_blocked = 0usize;
            let mut accepted = Vec::new();
            let mut widths = Vec::new();
            let mut roads_with_candidates = BTreeSet::new();
            let mut roads_with_sites = BTreeSet::new();

            for road in &engine.state.roads {
                if road.stage != Stage::Open
                    || !road.connected
                    || matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
                {
                    continue;
                }
                let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
                    continue;
                };
                let proposals = engine
                    .frontage_gap_candidates_with_layout(a, b, &context, &towers, depth, offset);
                if !proposals.is_empty() {
                    roads_with_candidates.insert(road.id);
                }
                for ((x, z, access_node, along, side), width) in proposals {
                    generated += 1;
                    if width < 8.0 || width > 14.0 || !engine.node_connected(access_node) {
                        *geometry_blocked
                            .entry("invalid_or_disconnected".into())
                            .or_default() += 1;
                        continue;
                    }
                    if Engine::overlaps_station_tower_envelopes(&towers, x, z, width, depth) {
                        tower_blocked += 1;
                        continue;
                    }
                    let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
                    let rect = geometry::OrientedRect {
                        x,
                        z,
                        width,
                        depth,
                        rotation,
                    };
                    let (min_x, min_z, max_x, max_z) = rect.bounds();
                    let protected = geometry::OrientedRect {
                        x: -305.0,
                        z: -310.0,
                        width: 90.0,
                        depth: 80.0,
                        rotation: 0.0,
                    };
                    let center_clearance = (width.max(depth) * 0.5 + 4.0).max(9.0);
                    let failed = if !engine.parcel_clear_of_river(x, z, width, depth) {
                        Some("river_or_bank")
                    } else if min_x < MIN + 4.0
                        || min_z < MIN + 4.0
                        || max_x > MAX - 4.0
                        || max_z > MAX - 4.0
                    {
                        Some("world_bounds")
                    } else if geometry::rects_overlap(&rect, &protected, 0.0) {
                        Some("protected_land")
                    } else if context
                        .parcel_footprints
                        .iter()
                        .any(|old| geometry::rects_overlap(&rect, old, 2.0))
                    {
                        Some("existing_parcel_footprint")
                    } else if !engine.interior_plan_allows_parcel(&rect, false) {
                        Some("reserved_interior_access_lot")
                    } else if context.open_roads.iter().any(|road| {
                        geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz)
                            < 5.0
                    }) {
                        Some("open_road_footprint_clearance")
                    } else if context.pending_roads.iter().any(|road| {
                        geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz)
                            < 7.0
                    }) {
                        Some("pending_road_footprint_clearance")
                    } else if !engine.neighborhood_corridor_clear_for_parcel(
                        x,
                        z,
                        max_x - min_x,
                        max_z - min_z,
                    ) {
                        Some("reserved_neighborhood_corridor")
                    } else if context.open_roads.iter().any(|road| {
                        point_segment_distance(x, z, road.ax, road.az, road.bx, road.bz)
                            < center_clearance
                    }) {
                        Some("legacy_open_road_center_clearance")
                    } else if context.pending_roads.iter().any(|road| {
                        point_segment_distance(x, z, road.ax, road.az, road.bx, road.bz)
                            < center_clearance + 2.0
                    }) {
                        Some("pending_road_center_clearance")
                    } else {
                        None
                    };
                    let failed = failed.or_else(|| {
                        (!engine.progressive_frontage_geometry_clear(&rect, &context))
                            .then_some("central_progressive_admission_predicate")
                    });
                    if let Some(reason) = failed {
                        *geometry_blocked.entry(reason.into()).or_default() += 1;
                        continue;
                    }
                    let (mut local_open, mut local_active) = (0usize, 0usize);
                    for building in engine.state.buildings.iter().filter(|building| {
                        building.stage == Stage::Open
                            && (building.x - x).hypot(building.z - z) <= 78.0
                    }) {
                        local_open += 1;
                        local_active += usize::from(site_active(building));
                    }
                    if local_open < 4 || local_active < 3 {
                        neighborhood_blocked += 1;
                        continue;
                    }

                    // Mirror sequential admission: accepted compact lots
                    // immediately reserve their exact oriented envelopes
                    // against every later proposal in this diagnostic.
                    context.parcel_footprints.push(rect);
                    roads_with_sites.insert(road.id);
                    widths.push(width);
                    accepted.push(json!({
                        "road_id": road.id,
                        "x": x,
                        "z": z,
                        "width": width,
                        "depth": depth,
                        "rotation": rotation,
                        "access_node": access_node,
                        "along": along,
                        "side": side,
                    }));
                }
            }
            widths.sort_by(f32::total_cmp);
            json!({
                "depth": depth,
                "center_offset": offset,
                "generated_proposals": generated,
                "roads_with_proposals": roads_with_candidates.len(),
                "greedy_admissible_sites": accepted.len(),
                "roads_with_admissible_sites": roads_with_sites.len(),
                "nominal_envelope_area_m2": widths.iter().map(|w| (*w * depth) as f64).sum::<f64>(),
                "width_min_m": widths.first().copied(),
                "width_median_m": widths.get(widths.len() / 2).copied(),
                "width_max_m": widths.last().copied(),
                "blocked_by_tower_reservations": tower_blocked,
                "blocked_by_geometry_or_access": geometry_blocked.values().sum::<usize>(),
                "geometry_rejection_reasons": geometry_blocked,
                "blocked_by_local_building_demand": neighborhood_blocked,
                "sample_sites": accepted.into_iter().take(12).collect::<Vec<_>>(),
            })
        };
        worlds.push(json!({
            "seed": seed,
            "tick": engine.state.tick,
            "buildings": engine.state.buildings.len(),
            "parcels": engine.state.parcels.len(),
            "roads": engine.state.roads.len(),
            "demand": engine.state.demand,
            "baseline_12m_depth_15m_offset": evaluate(12.0, 15.0),
            "shallow_8m_depth_12m_offset": evaluate(8.0, 12.0),
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"worlds": worlds})).unwrap()
    );
}

#[test]
fn frontage_scan_skips_unfinished_road_without_opening_it_and_replays() {
    let mut engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/mature-world.cache-candidate.json"
    ))
    .expect("mature fixture restores");
    let collision_context = engine.collision_context();
    let tower_envelopes = engine.available_station_tower_envelopes();
    let mut target = None;

    for road in &engine.state.roads {
        if road.stage != Stage::Open
            || !road.connected
            || matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        {
            continue;
        }
        let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
            continue;
        };
        let mut candidates =
            engine.frontage_candidate_sites_with_layout(a, b, 14.0, 12.0, FRONTAGE_SLOT_SPACING);
        let tower_candidate = engine.station_tower_candidate_index(road.id, a, b, &candidates);
        if let Some(index) = tower_candidate {
            candidates.swap(0, index);
        }
        let Some(&(x, z, access_node, along, side)) = candidates.first() else {
            continue;
        };
        let tower_site = tower_candidate.is_some();
        if tower_site && !engine.state.station_online {
            continue;
        }
        if !tower_site
            && Engine::overlaps_station_tower_envelopes(&tower_envelopes, x, z, 14.0, 12.0)
        {
            continue;
        }
        let (parcel_x, parcel_z, parcel_width, parcel_depth) = if tower_site {
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let length = dx.hypot(dz).max(1.0);
            let nx = -dz / length;
            let nz = dx / length;
            (
                a.x + dx * along + nx * side as f32 * STATION_TOWER_ROAD_OFFSET,
                a.z + dz * along + nz * side as f32 * STATION_TOWER_ROAD_OFFSET,
                STATION_TOWER_PARCEL_WIDTH,
                STATION_TOWER_PARCEL_DEPTH,
            )
        } else {
            (x, z, 14.0, 12.0)
        };
        let geometry_clear = if tower_site {
            engine.parcel_geometry_clear(parcel_x, parcel_z, parcel_width, parcel_depth)
        } else {
            let rotation = (side as f32 * (b.z - a.z)).atan2(-side as f32 * (b.x - a.x));
            engine.progressive_frontage_geometry_clear(
                &geometry::OrientedRect {
                    x: parcel_x,
                    z: parcel_z,
                    width: parcel_width,
                    depth: parcel_depth,
                    rotation,
                },
                &collision_context,
            )
        };
        if !geometry_clear {
            continue;
        }
        let nearby: Vec<_> = engine
            .state
            .buildings
            .iter()
            .filter(|building| {
                building.stage == Stage::Open
                    && (building.x - parcel_x).hypot(building.z - parcel_z) <= 78.0
            })
            .collect();
        if nearby.len() < 4 || nearby.iter().filter(|b| site_active(b)).count() < 3 {
            continue;
        }
        target = Some((road.id, x, z, access_node, along, side));
        break;
    }
    let (target_road_id, _, _, _, _, _) =
        target.expect("mature city has an eligible later frontage position");

    let blocked_from = engine.add_node(350.0, 350.0, NodeKind::Junction);
    let blocked_to = engine.add_node(370.0, 350.0, NodeKind::Junction);
    let blocked_road_id = engine.add_road(
        blocked_from,
        blocked_to,
        RoadKind::Local,
        Stage::Survey,
        0.5,
    );
    assert_eq!(engine.state.roads.last().unwrap().id, blocked_road_id);
    assert!(!engine.state.roads.last().unwrap().connected);
    let blocked_road = engine.state.roads.pop().unwrap();
    let target_index = engine
        .state
        .roads
        .iter()
        .position(|road| road.id == target_road_id)
        .unwrap();
    let target_road = engine.state.roads.remove(target_index);
    engine.state.roads.insert(0, blocked_road);
    engine.state.roads.insert(1, target_road);
    engine.state.frontage_road_cursor = 0;
    engine.state.frontage_slot_cursor = 0;

    let parcels_before = engine.state.parcels.len();
    assert!(engine.try_progressive_frontage_infill());
    assert_eq!(engine.state.parcels.len(), parcels_before + 1);
    assert_eq!(engine.state.frontage_road_cursor, 1);
    assert_eq!(engine.state.frontage_slot_cursor, 1);
    let admitted = engine.state.parcels.last().unwrap();
    let frontage = engine
        .state
        .parcel_frontages
        .iter()
        .find(|frontage| frontage.parcel_id == admitted.id)
        .unwrap();
    assert_eq!(frontage.road_id, target_road_id);
    assert!(engine.node_connected(admitted.access_node));

    let blocked = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == blocked_road_id)
        .unwrap();
    assert_eq!(blocked.stage, Stage::Survey);
    assert!(!blocked.connected);
    assert!(!engine.node_connected(blocked_from));
    assert!(!engine.node_connected(blocked_to));
    assert!(engine.route(blocked_from, blocked_to, false).is_empty());
    engine.validate().unwrap();

    let save = engine.save_json();
    let mut resumed = Engine::restore_json(&save).unwrap();
    assert_eq!(resumed.save_json(), save);
    engine.step(200);
    resumed.step(200);
    assert_eq!(engine.save_json(), resumed.save_json());
    assert_eq!(
        resumed
            .state
            .roads
            .iter()
            .find(|road| road.id == blocked_road_id)
            .unwrap()
            .stage,
        Stage::Survey
    );
    assert!(!resumed.node_connected(blocked_from));
}

#[test]
fn legacy_city_save_can_unlock_progressive_frontage() {
    let mut engine = Engine::new_legacy(491);
    engine.state.demand = 2.0;
    let first_candidate = engine
        .state
        .roads
        .iter()
        .enumerate()
        .filter(|(_, road)| {
            road.stage == Stage::Open
                && road.connected
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        })
        .find_map(|(road_index, road)| {
            let a = engine.node(road.from)?;
            let b = engine.node(road.to)?;
            engine
                .frontage_candidate_sites(a, b)
                .into_iter()
                .find(|(x, z, _, along, side)| {
                    (*along - 0.5).abs() >= 0.001
                        && *side == -1
                        && engine.parcel_geometry_clear(*x, *z, 14.0, 12.0)
                })
                .map(|site| (road_index, road.id, site))
        })
        .expect("found an available off-center frontage slot");
    let (_, _, (x, z, _, _, _)) = first_candidate;
    let template = engine.state.buildings[0].clone();
    let access_node = engine
        .state
        .parcels
        .iter()
        .find(|parcel| parcel.id == template.parcel_id)
        .unwrap()
        .access_node;
    for index in 0..7 {
        let mut nearby = template.clone();
        nearby.id = 100_000 + index;
        nearby.x = x + index as f32 * 0.2;
        nearby.z = z + index as f32 * 0.2;
        nearby.occupancy = 1;
        nearby.parcel_id = engine.add_parcel(
            -355.0 + index as f32 * 16.0,
            -340.0,
            8.0,
            8.0,
            access_node,
            0.5,
            false,
        );
        engine.state.buildings.push(nearby);
    }

    let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    let state = legacy.as_object_mut().unwrap();
    state.remove("frontage_road_cursor");
    state.remove("frontage_slot_cursor");
    let mut restored = Engine::restore_json(&legacy.to_string()).unwrap();
    let old_parcels = restored.state.parcels.len();
    assert!(restored.try_progressive_frontage_infill());
    assert_eq!(restored.state.parcels.len(), old_parcels + 1);
    assert!(restored
        .state
        .parcel_frontages
        .iter()
        .any(|frontage| frontage.road_id == first_candidate.1));
    let infill = restored
        .state
        .parcels
        .iter()
        .find(|parcel| !engine.state.parcels.iter().any(|old| old.id == parcel.id))
        .expect("the admitted frontage lot is present");
    let infill_id = infill.id;
    let rotation = infill
        .rotation
        .expect("progressive frontage stores its orientation");
    let facing = restored.frontage_rotation(infill_id);
    assert!((rotation.sin() - facing.sin()).abs() < 1e-5);
    assert!((rotation.cos() - facing.cos()).abs() < 1e-5);
    restored.validate().unwrap();

    let save = restored.save_json();
    let mut resumed = Engine::restore_json(&save).unwrap();
    assert_eq!(resumed.save_json(), save);
    restored.step(200);
    resumed.step(200);
    assert_eq!(restored.save_json(), resumed.save_json());
}

#[test]
fn rotated_progressive_frontage_uses_road_facing_footprint() {
    let mut engine = Engine::new_legacy(904);
    let a_id = engine.add_node(200.0, 200.0, NodeKind::Junction);
    let b_id = engine.add_node(280.0, 280.0, NodeKind::Junction);
    let road_id = engine.add_road(a_id, b_id, RoadKind::Local, Stage::Open, 1.0);
    engine.state.roads.last_mut().unwrap().connected = true;
    let (a, b) = (
        engine.node(a_id).unwrap().clone(),
        engine.node(b_id).unwrap().clone(),
    );
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    let length = dx.hypot(dz);
    let nx = -dz / length;
    let nz = dx / length;
    let along = 0.3;
    let side = 1_i8;
    let existing_x = a.x + dx * along + nx * side as f32 * 15.0;
    let existing_z = a.z + dz * along + nz * side as f32 * 15.0;
    let rotation = (side as f32 * dz).atan2(-side as f32 * dx);
    let existing_id = engine.add_parcel(existing_x, existing_z, 14.0, 12.0, a_id, 0.8, false);
    engine.state.parcel_frontages.push(ParcelFrontage {
        parcel_id: existing_id,
        road_id,
        along,
        side,
    });
    engine
        .state
        .parcels
        .iter_mut()
        .find(|p| p.id == existing_id)
        .unwrap()
        .rotation = Some(rotation);

    let candidate_along = along + 16.0 / length;
    let candidate_x = a.x + dx * candidate_along + nx * side as f32 * 15.0;
    let candidate_z = a.z + dz * candidate_along + nz * side as f32 * 15.0;
    let oriented = geometry::OrientedRect {
        x: candidate_x,
        z: candidate_z,
        width: 14.0,
        depth: 12.0,
        rotation,
    };
    let context = engine.collision_context();
    assert!(engine.progressive_frontage_geometry_clear(&oriented, &context));

    let legacy_axis = geometry::OrientedRect {
        rotation: 0.0,
        ..oriented
    };
    assert!(
        !engine.progressive_frontage_geometry_clear(&legacy_axis, &context),
        "the same frontage slot must not be admitted with an axis-aligned footprint"
    );

    let overlapping = geometry::OrientedRect {
        x: a.x + dx * (along + 12.0 / length) + nx * side as f32 * 15.0,
        z: a.z + dz * (along + 12.0 / length) + nz * side as f32 * 15.0,
        ..oriented
    };
    assert!(!engine.progressive_frontage_geometry_clear(&overlapping, &context));
}

#[test]
fn frontage_grid_keeps_midpoint_spacing_clear_and_endpoint_bounded() {
    let engine = Engine::new_legacy(41);
    let a = Node {
        id: 90_001,
        x: 10.0,
        z: 300.0,
        kind: NodeKind::Junction,
    };
    let b = Node {
        id: 90_002,
        x: 90.0,
        z: 300.0,
        kind: NodeKind::Junction,
    };
    let candidates = engine.frontage_candidate_sites(&a, &b);
    let mut along: Vec<f32> = candidates
        .iter()
        .filter(|candidate| candidate.4 == 1)
        .map(|candidate| candidate.3)
        .collect();
    along.sort_by(f32::total_cmp);
    assert_eq!(along.len(), 3);
    assert!(along.iter().any(|value| (*value - 0.5).abs() < 0.0001));
    for pair in along.windows(2) {
        let spacing = (pair[1] - pair[0]) * 80.0;
        assert!((spacing - FRONTAGE_SLOT_SPACING).abs() < 0.001);
    }
    assert!(along.first().unwrap() * 80.0 >= FRONTAGE_ENDPOINT_CLEARANCE);
    assert!((1.0 - along.last().unwrap()) * 80.0 >= FRONTAGE_ENDPOINT_CLEARANCE);
}

#[test]
fn available_connected_infill_precedes_neighborhood_block_planning() {
    let mut engine = engine_with_developed_plan_site();
    let gateway = engine
        .state
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Gateway)
        .unwrap()
        .id;
    let outer = engine.add_node(390.0, 370.0, NodeKind::Junction);
    engine.add_road(gateway, outer, RoadKind::Local, Stage::Open, 1.0);
    engine.recompute_connectivity();
    let access_node = engine
        .state
        .nodes
        .iter()
        .find(|node| node.x == 250.0 && node.z == 250.0)
        .unwrap()
        .id;
    let parcel_id = engine.add_parcel(187.0, 199.0, 14.0, 12.0, access_node, 0.8, false);
    assert_ne!(
        parcel_id, 0,
        "the infill parcel must pass geometry admission"
    );
    assert!(engine.node_connected(access_node));
    engine.state.tick = 80;

    engine.evaluate_growth();

    let plan = engine
        .state
        .neighborhood_plan
        .as_ref()
        .expect("the block reservation may coexist with usable infill");
    assert_eq!(
        plan.next_segment, 0,
        "existing infill keeps priority over road work"
    );
    assert!(engine.state.projects.iter().any(|project| {
        project.kind == ProjectKind::Building
            && project.progress < 1.0
            && project.parcel_id == Some(parcel_id)
    }));
    assert!(engine
        .state
        .parcels
        .iter()
        .find(|parcel| parcel.id == parcel_id)
        .unwrap()
        .reservation
        .is_some());
    assert!(!engine
        .state
        .projects
        .iter()
        .any(|project| project.kind == ProjectKind::Road && project.progress < 1.0));
}

#[test]
fn coordinated_block_reuse_keeps_frontage_valid_when_existing_edge_is_reversed() {
    let mut engine = engine_with_developed_plan_site();
    let road = engine.state.roads[1].id;
    let existing = engine
        .state
        .roads
        .iter_mut()
        .find(|candidate| candidate.id == road)
        .unwrap();
    std::mem::swap(&mut existing.from, &mut existing.to);
    engine.recompute_connectivity();
    assert!(engine.try_create_neighborhood_plan());
    let plan = engine.state.neighborhood_plan.clone().unwrap();
    assert!(engine.state.roads.iter().any(|candidate| {
        candidate.id == road
            && ((candidate.from == plan.segments[0].0 && candidate.to == plan.segments[0].1)
                || (candidate.from == plan.segments[0].1 && candidate.to == plan.segments[0].0))
    }));

    let mut replay = Engine::restore_json(&engine.save_json()).unwrap();
    for edge in 0..4 {
        for world in [&mut engine, &mut replay] {
            assert!(world.plan_next_neighborhood_segment());
            let segment = plan.segments[edge];
            let pending = world.state.projects.iter_mut().find(|project| {
                project.kind == ProjectKind::Road
                    && project.progress < 1.0
                    && project.from == Some(segment.0)
                    && project.to == Some(segment.1)
            });
            if let Some(project) = pending {
                let id = project.id;
                project.progress = 1.0;
                world.complete_project(id);
            }
        }
        assert_eq!(engine.save_json(), replay.save_json());
    }
    assert!(engine.state.neighborhood_plan.is_none());
    assert!(engine
        .state
        .parcels
        .iter()
        .any(|parcel| parcel.rotation.is_some()));
    engine.validate().unwrap();
}
