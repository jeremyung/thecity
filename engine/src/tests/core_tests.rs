use super::*;

#[test]
fn project_history_admission_trims_completed_and_respects_active_cap() {
    let mut engine = Engine::new_legacy(88);
    engine.state.construction.projects.clear();
    let make_project = |id, progress| Project {
        id,
        kind: ProjectKind::Road,
        from: Some(1),
        to: Some(2),
        parcel_id: None,
        stage: if progress >= 1.0 {
            Stage::Open
        } else {
            Stage::Survey
        },
        progress,
        reason: "test admission boundary".into(),
        duration: 1.0,
        started_tick: 0,
    };
    for id in 1..(MAX_PROJECTS + COMPLETED_PROJECT_HISTORY) as u32 {
        engine
            .state
            .construction
            .projects
            .push(make_project(id, 1.0));
    }
    let active_id = MAX_PROJECTS as u32 + COMPLETED_PROJECT_HISTORY as u32;
    engine
        .state
        .construction
        .projects
        .push(make_project(active_id, 0.5));
    assert_eq!(
        engine.state.construction.projects.len(),
        MAX_PROJECTS + COMPLETED_PROJECT_HISTORY
    );

    assert!(engine.project_admission_available());
    assert_eq!(
        engine.state.construction.projects.len(),
        MAX_PROJECTS + COMPLETED_PROJECT_HISTORY
    );
    let next_id = active_id + 1;
    engine.record_project(make_project(next_id, 0.0));
    assert!(engine.state.construction.projects.len() <= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY);
    assert!(engine
        .state
        .construction
        .projects
        .iter()
        .any(|project| project.id == active_id));
    assert!(engine
        .state
        .construction
        .projects
        .iter()
        .any(|project| project.id == next_id));
    assert!(!engine
        .state
        .construction
        .projects
        .iter()
        .any(|project| project.id == 1));

    engine.state.construction.projects.clear();
    for id in 1..=MAX_PROJECTS as u32 {
        engine
            .state
            .construction
            .projects
            .push(make_project(id, 0.0));
    }
    let before: Vec<u32> = engine
        .state
        .construction
        .projects
        .iter()
        .map(|project| project.id)
        .collect();
    assert!(!engine.project_admission_available());
    assert_eq!(
        engine
            .state
            .construction
            .projects
            .iter()
            .map(|project| project.id)
            .collect::<Vec<_>>(),
        before
    );
}

#[test]
fn same_seed_and_ticks_have_identical_state() {
    let mut a = Engine::new_legacy(42);
    let mut b = Engine::new_legacy(42);
    for _ in 0..32 {
        a.step(2_000);
        b.step(2_000);
    }
    assert_eq!(a.state.tick, 64_000);
    assert_eq!(b.state.tick, 64_000);
    assert_eq!(a.save_json(), b.save_json());
}

#[test]
fn accelerated_growth_changes_measured_stage_without_a_calendar_unlock() {
    let mut engine = Engine::new_legacy(87231);
    assert_eq!(engine.snapshot().stage, "Founding");
    engine.set_development_rate(60);
    engine.step(1_000);
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.day, 0, "the test must remain within the first day");
    assert_ne!(snapshot.stage, "Founding");
    assert!(snapshot.stats.buildings >= 3);
}

#[test]
fn missing_parcel_rotation_keeps_legacy_and_rendered_envelopes() {
    let mut engine = Engine::new_legacy(4);
    engine.state.land.parcels.clear();
    engine.state.construction.buildings.clear();
    engine.state.land.parcels.push(Parcel {
        id: 10_001,
        x: 0.0,
        z: 0.0,
        width: 40.0,
        depth: 12.0,
        rotation: None,
        access_node: 0,
        suitability: 1.0,
        protected: false,
        reservation: None,
        open_space: None,
    });
    engine.state.construction.buildings.push(Building {
        id: 10_002,
        parcel_id: 10_001,
        x: 0.0,
        z: 0.0,
        rotation: std::f32::consts::FRAC_PI_2,
        width: 40.0,
        depth: 12.0,
        floors: 1,
        kind: BuildingKind::Home,
        stage: Stage::Open,
        progress: 1.0,
        capacity: 1,
        occupancy: 1,
        built_day: 0,
        name: "Legacy orientation probe".into(),
        appearance_seed: 1,
        history: Vec::new(),
        anchor: false,
        vacant_days: 0,
        business_active: false,
    });

    let context = engine.collision_context();
    assert_eq!(context.parcel_footprints.len(), 2);
    assert_eq!(context.parcel_footprints[0].rotation, 0.0);
    assert_eq!(
        context.parcel_footprints[1].rotation,
        std::f32::consts::FRAC_PI_2
    );

    // The rendered rotated envelope clears this segment by 9 m, but the
    // historical axis-aligned reservation still intersects it. Existing
    // saves therefore keep both geometry constraints during migration.
    assert!(
        geometry::rect_segment_distance(&context.parcel_footprints[1], 15.0, 0.0, 18.0, 0.0,)
            >= 5.0
    );
    assert!(
        geometry::rect_segment_distance(&context.parcel_footprints[0], 15.0, 0.0, 18.0, 0.0,) < 5.0
    );
    assert!(!engine.road_corridor_clear_of_parcels_with_context(15.0, 0.0, 18.0, 0.0, &context,));
}

#[test]
fn mature_city_grows_a_beneficial_road_beyond_old_open_road_stop() {
    let mut engine = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/mature-world.bounded-selection.json"
    ))
    .expect("mature public city fixture should restore");
    let initial_next_id = engine.state.next_id;
    let initial_open_ordinary = engine
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        })
        .count();
    assert!(initial_open_ordinary > 160);

    let mut opened_road = None;
    let mut occupied_parcel = None;
    for _ in 0..128 {
        engine.step(2_000);
        opened_road = engine
            .state
            .network
            .roads
            .iter()
            .find(|road| {
                road.id >= initial_next_id
                    && road.kind != RoadKind::Bridge
                    && road.kind != RoadKind::Rail
                    && road.stage == Stage::Open
                    && road.connected
            })
            .map(|road| road.id);
        if let Some(road_id) = opened_road {
            let accessible: BTreeSet<u32> = engine
                .state
                .land
                .parcel_frontages
                .iter()
                .filter(|frontage| frontage.road_id == road_id)
                .filter_map(|frontage| {
                    let parcel = engine
                        .state
                        .land
                        .parcels
                        .iter()
                        .find(|parcel| parcel.id == frontage.parcel_id)?;
                    engine
                        .node_connected(parcel.access_node)
                        .then_some(parcel.id)
                })
                .collect();
            occupied_parcel = engine
                .state
                .construction
                .buildings
                .iter()
                .find(|building| {
                    building.id >= initial_next_id
                        && accessible.contains(&building.parcel_id)
                        && site_active(building)
                })
                .map(|building| building.parcel_id);
            if occupied_parcel.is_some() {
                break;
            }
        }
    }
    let road_id = opened_road.expect("growth should open an ordinary road past 160");
    let final_open_ordinary = engine
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        })
        .count();
    assert!(final_open_ordinary > initial_open_ordinary);
    let occupied_parcel = occupied_parcel.expect(
        "the new road should lead to a connected, occupied building site through construction",
    );
    let new_frontages = engine
        .state
        .land
        .parcel_frontages
        .iter()
        .filter(|frontage| frontage.road_id == road_id)
        .count();
    println!(
        "road-benefit continuation tick={} ordinary_roads={} road_id={} frontage_sites={} occupied_parcel={}",
        engine.state.tick,
        engine
            .state
            .network.roads
            .iter()
            .filter(|road| road.stage == Stage::Open && road.connected && road.kind == RoadKind::Local)
            .count(),
        road_id,
        new_frontages,
        occupied_parcel
    );
    let frontage = engine
        .state
        .land
        .parcel_frontages
        .iter()
        .find(|frontage| frontage.road_id == road_id && frontage.parcel_id == occupied_parcel)
        .expect("occupied building must have frontage on the new road");
    let parcel = engine
        .state
        .land
        .parcels
        .iter()
        .find(|parcel| parcel.id == frontage.parcel_id)
        .unwrap();
    assert!(engine.node_connected(parcel.access_node));
    engine.validate().unwrap();

    let saved = engine.save_json();
    let mut replay = Engine::restore_json(&saved).unwrap();
    engine.step(2_000);
    replay.step(2_000);
    assert_eq!(engine.save_json(), replay.save_json());
}

#[test]
fn open_bridge_head_can_seed_only_a_connected_bank_with_opposite_catchment() {
    fn bridge_world(stage: Stage, active_opposite: bool) -> (Engine, u32) {
        let mut engine = Engine::new_legacy(87231);
        engine.state.network.nodes.clear();
        engine.state.network.roads.clear();
        engine.state.land.parcels.clear();
        engine.state.land.parcel_frontages.clear();
        engine.state.construction.buildings.clear();
        engine.state.construction.projects.clear();
        engine.state.legacy.neighborhood_plan = None;
        engine.state.legacy.neighborhood_road_ids.clear();
        engine.state.people.demand = 3.0;
        engine.state.tick = 8_000;

        let gateway = engine.add_node(70.0, 42.0, NodeKind::Gateway);
        let east = engine.add_node(-18.0, 42.0, NodeKind::Junction);
        // Follow the current seeded river's perpendicular bank offset.
        // The old fixed -184m endpoint lies inside its bank buffer.
        let bank = river_course::bank_alignment(&river_course::sampled(engine.state.seed), -47.0);
        let edge = bank
            .windows(2)
            .find(|pair| pair[0].z <= 42.0 && pair[1].z >= 42.0)
            .unwrap();
        let t = (42.0 - edge[0].z) / (edge[1].z - edge[0].z);
        let west_x = edge[0].x + (edge[1].x - edge[0].x) * t;
        let west = engine.add_node(west_x, 42.0, NodeKind::Junction);
        assert!(engine.ordinary_road_clear_of_river(west_x, 42.0, west_x - 70.0, 42.0, 4.5));
        engine.add_road(gateway, east, RoadKind::Local, Stage::Open, 1.0);
        let bridge = engine.add_road(
            east,
            west,
            RoadKind::Bridge,
            stage,
            if stage == Stage::Open { 1.0 } else { 0.0 },
        );
        engine.recompute_connectivity();
        if stage == Stage::Open {
            assert!(engine
                .state
                .network
                .roads
                .iter()
                .any(|road| road.id == bridge && road.connected));
        }

        if active_opposite {
            let parcel = engine.add_parcel(-18.0, 28.0, 14.0, 12.0, east, 1.0, false);
            let building = engine.add_building(
                parcel,
                BuildingKind::Apartment,
                3,
                Stage::Open,
                1.0,
                12,
                "East Bank House",
                false,
                vec!["Settled beside the bridge approach".into()],
            );
            engine
                .state
                .construction
                .buildings
                .iter_mut()
                .find(|site| site.id == building)
                .unwrap()
                .occupancy = 6;
        }
        (engine, west)
    }

    let (mut closed, closed_west) = bridge_world(Stage::Survey, true);
    closed.plan_road();
    assert!(!closed
        .state
        .construction
        .projects
        .iter()
        .any(|project| { project.kind == ProjectKind::Road && project.from == Some(closed_west) }));

    let (mut disconnected, disconnected_west) = bridge_world(Stage::Open, true);
    disconnected
        .state
        .network
        .roads
        .iter_mut()
        .find(|road| road.kind == RoadKind::Bridge)
        .unwrap()
        .connected = false;
    disconnected.plan_road();
    assert!(!disconnected
        .state
        .construction
        .projects
        .iter()
        .any(|project| {
            project.kind == ProjectKind::Road && project.from == Some(disconnected_west)
        }));

    let (mut quiet, quiet_west) = bridge_world(Stage::Open, false);
    quiet.plan_road();
    assert!(!quiet
        .state
        .construction
        .projects
        .iter()
        .any(|project| { project.kind == ProjectKind::Road && project.from == Some(quiet_west) }));

    let (mut blocked, blocked_west) = bridge_world(Stage::Open, true);
    blocked
        .state
        .network
        .nodes
        .iter_mut()
        .find(|node| node.id == blocked_west)
        .unwrap()
        .x = -184.0;
    assert!(!blocked.ordinary_road_clear_of_river(-184.0, 42.0, -254.0, 42.0, 4.5));
    blocked.plan_road();
    assert!(
        !blocked.state.construction.projects.iter().any(|project| {
            project.kind == ProjectKind::Road && project.from == Some(blocked_west)
        }),
        "eligible bridgeheads must not bypass river clearance"
    );

    let (mut active, active_west) = bridge_world(Stage::Open, true);
    active.plan_road();
    let first_bank_road = active.state.construction.projects.iter().find(|project| {
        project.kind == ProjectKind::Road
            && project.from == Some(active_west)
            && project.progress < 1.0
    });
    assert!(
        first_bank_road.is_some(),
        "an open bridge with an opposite active catchment should admit a bank-entry road; access={:?}, roads={:?}",
        active.state.legacy.pending_interior_access.as_ref().map(|plan| (plan.host_road_id, plan.host_t)),
        active.state.construction.projects.iter().filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0).map(|project| (project.from, project.to, &project.reason)).collect::<Vec<_>>()
    );
    let first_bank_road = first_bank_road.unwrap();
    let first_bank_project_id = first_bank_road.id;
    let to = first_bank_road.to.unwrap();
    let west = active.node(active_west).unwrap();
    let target = active.node(to).unwrap();
    assert!((target.x - west.x).hypot(target.z - west.z) <= 90.0);
    assert!(active.frontage_sites(west, target).len() > 0);
    active.plan_road();
    assert_eq!(
        active
            .state
            .construction.projects
            .iter()
            .filter(|project| project.kind == ProjectKind::Road && project.from == Some(active_west))
            .count(),
        1,
        "a pending bridgehead connection must suppress repeated pioneer roads"
    );
    active.recompute_connectivity();
    assert!(active.node_connected(active_west));
    let first_bank_duration = active
        .state
        .construction
        .projects
        .iter()
        .find(|project| project.id == first_bank_project_id)
        .unwrap()
        .duration
        .ceil() as usize;
    for _ in 0..first_bank_duration {
        active.advance_projects();
    }
    let buildings: Vec<Building> = active
        .state
        .construction
        .buildings
        .iter()
        .filter(|building| site_active(building))
        .cloned()
        .collect();
    assert!(!active
        .eligible_bridgehead_headings(&buildings)
        .contains_key(&active_west));
    active.validate().unwrap();
    assert_eq!(
        active.save_json(),
        Engine::restore_json(&active.save_json())
            .unwrap()
            .save_json()
    );
}

#[test]
fn current_city_bridge_grows_occupied_far_bank_neighborhood() {
    for seed in [87231, 0, 5] {
        let mut engine = Engine::new(seed);
        engine.set_development_rate(16);
        let mut bridge_id = None;
        let mut saw_bridge_construction = false;
        let mut saw_bank_road_construction = false;
        let mut saw_bank_building_construction = false;
        let mut settled = false;
        for _ in 0..3_200 {
            engine.step(20);
            if bridge_id.is_none() {
                bridge_id = engine
                    .state
                    .network
                    .roads
                    .iter()
                    .find(|r| r.kind == RoadKind::Bridge)
                    .map(|r| r.id);
            }
            let Some(bridge) = engine
                .state
                .network
                .roads
                .iter()
                .find(|r| Some(r.id) == bridge_id)
            else {
                continue;
            };
            saw_bridge_construction |= bridge.stage != Stage::Open;
            let a = engine.node(bridge.from).unwrap();
            let b = engine.node(bridge.to).unwrap();
            let west = if a.x < b.x { a.id } else { b.id };
            let west_side = |x: f32, z: f32| x < engine.river_center_x_at(z) - RIVER_HALF_WIDTH;
            saw_bank_road_construction |= engine.state.construction.projects.iter().any(|p| {
                p.kind == ProjectKind::Road
                    && p.progress < 1.0
                    && p.from.zip(p.to).is_some_and(|(a, b)| {
                        engine.node(a).is_some_and(|n| west_side(n.x, n.z))
                            && engine.node(b).is_some_and(|n| west_side(n.x, n.z))
                    })
            });
            saw_bank_building_construction |= engine
                .state
                .construction
                .buildings
                .iter()
                .any(|b| west_side(b.x, b.z) && b.stage != Stage::Open);
            let bank_connected = engine.state.network.roads.iter().any(|r| {
                r.stage == Stage::Open
                    && r.connected
                    && !matches!(r.kind, RoadKind::Rail | RoadKind::Bridge)
                    && (r.from == west || r.to == west)
            });
            let housing = engine.state.construction.buildings.iter().find(|b| {
                west_side(b.x, b.z) && b.stage == Stage::Open && is_housing(b) && b.occupancy > 0
            });
            if bridge.stage == Stage::Open && bridge.connected && bank_connected {
                if let Some(house) = housing {
                    assert!(
                        engine.building_access_segment(house.id).is_some(),
                        "seed {seed}: missing house access"
                    );
                    let parcel = engine
                        .state
                        .land
                        .parcels
                        .iter()
                        .find(|p| p.id == house.parcel_id)
                        .unwrap();
                    assert!(engine.node_connected(parcel.access_node));
                    settled = true;
                    break;
                }
            }
        }
        assert!(
            saw_bridge_construction && saw_bank_road_construction && saw_bank_building_construction,
            "seed {seed}: missing construction lifecycle"
        );
        assert!(
            settled,
            "seed {seed}: no occupied connected far-bank neighborhood by tick {}",
            engine.state.tick
        );
        engine.validate().unwrap();
        let mut resumed = Engine::restore_json(&engine.save_json()).unwrap();
        engine.step(2_000);
        resumed.step(2_000);
        assert_eq!(
            engine.save_json(),
            resumed.save_json(),
            "seed {seed}: replay diverged"
        );
    }
}

#[test]
fn cached_neighborhood_collisions_match_reference_after_world_edits() {
    let probes = [
        geometry::OrientedRect {
            x: 145.0,
            z: 190.0,
            width: 18.0,
            depth: 12.0,
            rotation: 0.0,
        },
        geometry::OrientedRect {
            x: 168.0,
            z: 205.0,
            width: 21.0,
            depth: 13.0,
            rotation: 0.63,
        },
        geometry::OrientedRect {
            x: -18.0,
            z: 42.0,
            width: 15.0,
            depth: 10.0,
            rotation: -0.41,
        },
        geometry::OrientedRect {
            x: 0.0,
            z: 300.0,
            width: 16.0,
            depth: 14.0,
            rotation: 1.2,
        },
    ];
    let assert_matches = |engine: &Engine| {
        let cached = engine.collision_context();
        for probe in &probes {
            assert_eq!(
                engine.oriented_parcel_geometry_clear_with_context(probe, &cached),
                engine.oriented_parcel_geometry_clear(probe),
                "cached collision decision changed for probe {probe:?}"
            );
        }
    };

    // Existing legacy parcels inherit the first building's rendered
    // orientation when their optional parcel rotation is absent.
    let mut with_building = engine_with_developed_plan_site();
    let legacy_parcel = with_building.state.land.parcels[0].id;
    with_building.state.land.parcels[0].rotation = None;
    with_building.state.construction.buildings[0].parcel_id = legacy_parcel;
    with_building.state.construction.buildings[0].rotation = 0.73;
    assert_matches(&with_building);

    // Empty parcels with saved orientation take the explicit rotation.
    let mut with_empty_rotated_parcel = engine_with_developed_plan_site();
    let empty = with_empty_rotated_parcel.add_parcel(28.0, 112.0, 17.0, 11.0, 2, 0.6, false);
    with_empty_rotated_parcel
        .state
        .land
        .parcels
        .iter_mut()
        .find(|parcel| parcel.id == empty)
        .unwrap()
        .rotation = Some(-0.52);
    assert_matches(&with_empty_rotated_parcel);

    // Road removals and pending construction both alter the cached segment
    // vectors while preserving the reference check's outcome.
    let mut with_pending_road = engine_with_developed_plan_site();
    let road = with_pending_road.state.network.roads[0].clone();
    with_pending_road.state.network.roads[0].stage = Stage::Survey;
    let project_id = with_pending_road.id();
    with_pending_road.state.construction.projects.push(Project {
        id: project_id,
        kind: ProjectKind::Road,
        from: Some(road.from),
        to: Some(road.to),
        parcel_id: None,
        stage: Stage::Survey,
        progress: 0.25,
        reason: "Collision context test road".into(),
        duration: 4.0,
        started_tick: 0,
    });
    assert_matches(&with_pending_road);
}

#[test]
fn bounded_world_keeps_open_buildings_connected() {
    let mut e = Engine::new_legacy(19);
    e.set_development_rate(60);
    for _ in 0..12 {
        e.step(2_000);
        e.validate().unwrap();
    }
    assert!(e.state.construction.buildings.len() <= MAX_BUILDINGS);
    assert!(e.state.network.roads.len() <= MAX_SEGMENTS);
}

#[test]
fn twelve_seed_corpus_stays_bounded_and_grows_as_a_connected_city() {
    use std::time::Instant;
    let started = Instant::now();
    let mut totals = (0usize, 0usize, 0u32, 0usize);
    let mut saw_ordinary_loop = false;
    let mut bus_service_seeds = 0usize;
    for seed in 0..12 {
        let mut engine = Engine::new_legacy(seed);
        engine.set_development_rate(60);
        let mut max_travelers = 0usize;
        let mut max_night_travelers = 0usize;
        for _ in 0..36 {
            engine.step(2_000);
            engine
                .validate()
                .unwrap_or_else(|err| panic!("seed {seed}: {err}"));
            max_travelers = max_travelers.max(engine.state.people.travelers.len());
            let minute = engine.snapshot().minute_of_day;
            if minute >= 1230.0 || minute < 390.0 {
                max_night_travelers = max_night_travelers.max(
                    engine
                        .state
                        .people
                        .travelers
                        .iter()
                        .filter(|t| {
                            t.activity == TravelerActivity::NightShift
                                || t.activity == TravelerActivity::Return
                        })
                        .count(),
                );
            }
        }
        let snapshot = engine.snapshot();
        let open_roads = snapshot
            .roads
            .iter()
            .filter(|r| r.stage == Stage::Open)
            .count();
        assert!(
            snapshot
                .parcels
                .iter()
                .all(|parcel| engine.parcel_clear_of_river(
                    parcel.x,
                    parcel.z,
                    parcel.width,
                    parcel.depth
                )),
            "seed {seed} developed a parcel inside the water/bank reserve"
        );
        assert!(
            snapshot
                .roads
                .iter()
                .filter(|road| {
                    road.stage == Stage::Open
                        && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
                })
                .all(|road| {
                    let from = engine.node(road.from).unwrap();
                    let to = engine.node(road.to).unwrap();
                    let half_width = if road.kind == RoadKind::Arterial {
                        7.0
                    } else {
                        4.5
                    };
                    engine.ordinary_road_clear_of_river(from.x, from.z, to.x, to.z, half_width)
                }),
            "seed {seed} built an ordinary road inside the water/bank reserve"
        );
        for node in engine
            .state
            .network
            .nodes
            .iter()
            .filter(|node| node.id >= 7)
        {
            let incident: Vec<_> = engine
                .state
                .network
                .roads
                .iter()
                .filter(|road| {
                    road.connected
                        && road.stage == Stage::Open
                        && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
                        && (road.from == node.id || road.to == node.id)
                })
                .collect();
            assert!(
                incident.len() <= 4,
                "seed {seed} generated junction {} with degree {}",
                node.id,
                incident.len()
            );
            let headings: Vec<f32> = incident
                .iter()
                .filter_map(|road| {
                    let neighbor_id = if road.from == node.id {
                        road.to
                    } else {
                        road.from
                    };
                    engine
                        .node(neighbor_id)
                        .map(|neighbor| (neighbor.z - node.z).atan2(neighbor.x - node.x))
                })
                .collect();
            for i in 0..headings.len() {
                for j in (i + 1)..headings.len() {
                    let delta = (headings[i] - headings[j]).rem_euclid(std::f32::consts::TAU);
                    assert!(
                        delta.min(std::f32::consts::TAU - delta)
                            >= std::f32::consts::PI / 6.0 - 0.04,
                        "seed {seed} generated acute road branches at junction {}",
                        node.id
                    );
                }
            }
        }
        let ordinary_roads: Vec<_> = engine
            .state
            .network
            .roads
            .iter()
            .filter(|road| {
                road.connected
                    && road.stage == Stage::Open
                    && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
            })
            .collect();
        for road in &ordinary_roads {
            let mut frontier = vec![road.from];
            let mut visited = vec![road.from];
            while let Some(node_id) = frontier.pop() {
                for adjacent in ordinary_roads.iter().filter(|candidate| {
                    candidate.id != road.id
                        && (candidate.from == node_id || candidate.to == node_id)
                }) {
                    let next = if adjacent.from == node_id {
                        adjacent.to
                    } else {
                        adjacent.from
                    };
                    if next == road.to {
                        saw_ordinary_loop = true;
                        break;
                    }
                    if !visited.contains(&next) {
                        visited.push(next);
                        frontier.push(next);
                    }
                }
                if saw_ordinary_loop {
                    break;
                }
            }
            if saw_ordinary_loop {
                break;
            }
        }
        assert!(
            open_roads >= 6,
            "seed {seed} failed to extend its road network"
        );
        assert!(
            snapshot.buildings.len() >= 8,
            "seed {seed} failed to establish its neighborhood"
        );
        assert!(
            engine.state.transit.station_online,
            "seed {seed} did not open rail service"
        );
        let rail_commuters = engine
            .state
            .people
            .residents
            .iter()
            .filter(|resident| resident.rail_commuter)
            .count();
        assert!(
            rail_commuters > 0,
            "seed {seed} generated no external rail commuters"
        );
        let rail_boardings = engine
            .state
            .events
            .iter()
            .filter(|event| event.kind == EventKind::TrainPassengerBoarded)
            .count();
        assert!(
            rail_boardings > 0,
            "seed {seed} commuters never boarded a train"
        );
        assert!(
            engine
                .state
                .events
                .iter()
                .any(|event| event.kind == EventKind::RailWorkStarted),
            "seed {seed} has no completed external rail commute"
        );
        assert!(
            !engine.state.transit.trains.is_empty(),
            "seed {seed} has no active train"
        );
        assert!(engine.state.transit.buses.len() <= MAX_BUSES);
        if let Some(line) = &engine.state.transit.bus_line {
            bus_service_seeds += 1;
            assert!(line.active);
            assert!((3..=MAX_BUS_STOPS).contains(&line.stops.len()));
            assert!(line.route.len() <= MAX_BUS_ROUTE_NODES);
            assert!(engine
                .state
                .transit
                .buses
                .iter()
                .all(|bus| bus.line_id == line.id && bus.passengers.len() <= BUS_CAPACITY));
            assert!(engine.bus_waiting_count() <= BUS_QUEUE_CAPACITY);
        }
        assert!(
            max_travelers >= 20,
            "seed {seed} has too little visible daytime activity"
        );
        assert!(
            max_night_travelers > 0,
            "seed {seed} has no visible late-night activity"
        );
        assert!(
            matches!(snapshot.stage.as_str(), "TransitCity" | "Metropolis" | "Maturity"),
            "seed {seed} did not establish transit-era growth: stage={} buildings={} open_roads={} population={}",
            snapshot.stage,
            snapshot.buildings.len(),
            open_roads,
            snapshot.stats.population
        );
        assert!(snapshot.stats.active_projects as usize <= MAX_PROJECTS);
        assert!(
            engine.state.construction.projects.len() <= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY
        );
        assert!(engine.state.people.population <= MAX_POPULATION);
        totals.0 += snapshot.buildings.len();
        totals.1 += open_roads;
        totals.2 += snapshot.stats.population;
        totals.3 += snapshot.stats.active_travelers as usize;
        println!(
            "seed={seed} tick={} stage={} buildings={} open_roads={} population={} travelers={} rail_commuters={rail_commuters} rail_boardings={rail_boardings} projects={} parcels={}",
            snapshot.tick,
            snapshot.stage,
            snapshot.stats.buildings,
            open_roads,
            snapshot.stats.population,
            max_travelers,
            snapshot.stats.active_projects,
            snapshot.parcels.len(),
        );
    }
    println!(
        "12-seed profile: elapsed_ms={} mean_buildings={} mean_open_roads={} mean_population={} mean_final_travelers={} bus_service_seeds={}",
        started.elapsed().as_millis(),
        totals.0 / 12,
        totals.1 / 12,
        totals.2 / 12,
        totals.3 / 12,
        bus_service_seeds,
    );
    assert!(
        saw_ordinary_loop,
        "the seeded road corpus formed no street loops"
    );
    assert!(
        bus_service_seeds > 0,
        "no seed produced a viable neighborhood bus line"
    );
}

#[test]
fn active_commerce_does_not_age_into_false_vacancy() {
    let mut engine = Engine::new_legacy(71);
    engine.set_development_rate(60);
    for _ in 0..36 {
        engine.step(2_000);
    }
    let commercial_ids: BTreeSet<u32> = engine
        .state
        .construction
        .buildings
        .iter()
        .filter(|building| is_commercial(building))
        .map(|building| building.id)
        .collect();
    for business in engine
        .state
        .construction
        .buildings
        .iter()
        .filter(|building| is_commercial(building) && building.stage == Stage::Open)
    {
        assert!(business.business_active);
        assert_eq!(business.vacant_days, 0);
    }
    assert!(!engine.state.events.iter().any(|event| {
        event.kind == EventKind::RenovationStarted && commercial_ids.contains(&event.entity_id)
    }));
    engine.validate().unwrap();
    let save = engine.save_json();
    let restored = Engine::restore_json(&save).unwrap();
    assert_eq!(restored.save_json(), save);
}

#[test]
fn active_commerce_is_not_vacant_when_residential_occupancy_is_zero() {
    let mut engine = Engine::new_legacy(605);
    let founder = &engine.state.construction.buildings[0];
    assert_eq!(founder.occupancy, 0);
    assert!(founder.business_active);
    assert!(site_active(founder));
    assert!(!site_vacant(founder));

    let cafe_id = engine.add_building(
        engine.state.land.parcels[1].id,
        BuildingKind::Cafe,
        2,
        Stage::Open,
        1.0,
        10,
        "Active Cafe",
        false,
        Vec::new(),
    );
    let before = engine
        .state
        .construction
        .buildings
        .iter()
        .find(|building| building.id == cafe_id)
        .unwrap()
        .vacant_days;
    engine.development_day(1);
    let cafe = engine
        .state
        .construction
        .buildings
        .iter()
        .find(|building| building.id == cafe_id)
        .unwrap();
    assert!(site_active(cafe));
    assert!(!site_vacant(cafe));
    assert_eq!(cafe.vacant_days, before);
}

#[test]
fn long_run_keeps_state_collections_bounded_after_expansion() {
    use std::time::Instant;
    let mut engine = Engine::new_legacy(1204);
    engine.set_development_rate(60);
    let started = Instant::now();
    let mut max_travelers = 0usize;
    for batch in 0..150 {
        engine.step(2_000);
        max_travelers = max_travelers.max(engine.state.people.travelers.len());
        if batch % 10 == 0 {
            engine.validate().unwrap();
        }
    }
    engine.validate().unwrap();
    let snapshot = engine.snapshot();
    assert!(engine.state.events.len() <= MAX_EVENTS);
    assert!(engine.state.construction.projects.len() <= MAX_PROJECTS + COMPLETED_PROJECT_HISTORY);
    assert!(engine.state.land.parcels.len() <= MAX_BUILDINGS * 2);
    assert!(engine.state.construction.buildings.len() <= MAX_BUILDINGS);
    assert!(engine.state.network.roads.len() <= MAX_SEGMENTS);
    assert!(max_travelers <= MAX_TRAVELERS);
    let ordinary_open_roads: BTreeSet<u32> = engine
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        })
        .map(|road| road.id)
        .collect();
    let frontage_lots = engine
        .state
        .land
        .parcel_frontages
        .iter()
        .filter(|frontage| ordinary_open_roads.contains(&frontage.road_id))
        .count();
    let frontage_per_road = frontage_lots as f32 / ordinary_open_roads.len().max(1) as f32;
    assert!(
        frontage_per_road > 2.0,
        "mature ordinary streets average only {frontage_per_road:.2} frontage lots"
    );
    println!(
        "long-run profile: tick={} stage={} buildings={} parcels={} open_roads={} ordinary_frontage_lots={} lots_per_ordinary_road={:.2} population={} max_travelers={} projects={} events={} elapsed_ms={}",
        snapshot.tick,
        snapshot.stage,
        engine.state.construction.buildings.len(),
        engine.state.land.parcels.len(),
        engine.state.network.roads.iter().filter(|r| r.stage == Stage::Open).count(),
        frontage_lots,
        frontage_per_road,
        snapshot.stats.population,
        max_travelers,
        engine.state.construction.projects.len(),
        engine.state.events.len(),
        started.elapsed().as_millis(),
    );
}

#[test]
fn river_crossing_and_founder_are_present() {
    let e = Engine::new_legacy(5);
    let s = e.snapshot();
    assert!(s.roads.iter().any(|r| r.kind == RoadKind::Bridge));
    let founder = s
        .buildings
        .iter()
        .find(|b| b.name == "Kissa Aoba" && b.anchor)
        .expect("the founding shop remains present");
    assert!(e.parcel_clear_of_river(founder.x, founder.z, founder.width, founder.depth));
    assert!(e.distance_to_river(founder.x, founder.z) < 80.0);
    assert!(s.terrain.river.len() >= 4);
    for seed in 0..5 {
        let world = Engine::new_legacy(seed);
        let snapshot = world.snapshot();
        let shop = snapshot
            .buildings
            .iter()
            .find(|building| building.name == "Kissa Aoba")
            .unwrap();
        assert!(
            world.parcel_clear_of_river(shop.x, shop.z, shop.width, shop.depth),
            "seed {seed}: founder footprint infringes the water/bank clearance"
        );
        assert!(
            world.distance_to_river(shop.x, shop.z) <= 70.0,
            "seed {seed}: founder is no longer a waterfront building"
        );
        assert!(snapshot
            .parcels
            .iter()
            .all(|parcel| world.parcel_clear_of_river(
                parcel.x,
                parcel.z,
                parcel.width,
                parcel.depth
            )));
        assert!(
            snapshot
                .roads
                .iter()
                .filter(|road| {
                    road.stage == Stage::Open
                        && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
                })
                .all(|road| {
                    let from = world.node(road.from).unwrap();
                    let to = world.node(road.to).unwrap();
                    world.ordinary_road_clear_of_river(from.x, from.z, to.x, to.z, 4.5)
                }),
            "seed {seed}: an ordinary initial road enters the river reserve"
        );
        world.validate().unwrap();
    }
}

#[test]
fn time_and_rate_are_integer_deterministic() {
    let mut e = Engine::new_legacy(2);
    assert_eq!(e.snapshot().minute_of_day, 360.0);
    for _ in 0..10 {
        e.step(2_000);
    }
    let t = e.snapshot();
    assert_eq!(t.day, 0);
    assert!((t.minute_of_day - 120.0).abs() < 0.1);
    e.set_development_rate(0);
    let pop = e.state.people.population;
    e.step(1000);
    assert_eq!(e.state.people.population, pop);
}

#[test]
#[ignore = "bounded diagnostic over mature public-WASM save fixtures"]
fn diagnose_mature_neighborhood_and_road_proposal_rejections() {
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
        let audit = audit_mature_neighborhood_proposals(&engine);
        println!(
            "{name}: tick={} seed={} buildings={} roads={} parcels={} active_projects={} demand={} population={} anchor_cursor={} nodes={} neighborhood_audit={audit:?}",
            engine.state.tick,
            engine.state.seed,
            engine.state.construction.buildings.len(),
            engine.state.network.roads.len(),
            engine.state.land.parcels.len(),
            engine.state.construction.projects.iter().filter(|project| project.progress < 1.0).count(),
            engine.state.people.demand,
            engine.state.people.population,
            engine.state.legacy.neighborhood_anchor_cursor,
            engine.state.network.nodes.len(),
        );

        // Exercise the real bounded planner over multiple cursor windows,
        // without advancing time or modifying the saved fixture.
        let mut planner_probe =
            Engine::restore_json(&engine.save_json()).expect("diagnostic clone restores");
        let mut selected_anchor = None;
        let start_cursor = planner_probe.state.legacy.neighborhood_anchor_cursor;
        for _ in 0..=planner_probe.state.network.nodes.len() {
            if planner_probe.try_create_neighborhood_plan() {
                selected_anchor = planner_probe
                    .state
                    .legacy
                    .neighborhood_plan
                    .as_ref()
                    .map(|plan| plan.anchor_node);
                break;
            }
            if planner_probe.state.legacy.neighborhood_anchor_cursor == start_cursor {
                break;
            }
        }
        println!("{name}: real_planner_selected_anchor={selected_anchor:?}");

        // Probe the actual road planner through several rotated origin
        // windows. If it finds a street, report its endpoint coordinates.
        let mut road_probe =
            Engine::restore_json(&engine.save_json()).expect("diagnostic clone restores");
        let mut road_proposal = None;
        for _ in 0..32 {
            let before = road_probe.state.construction.projects.len();
            road_probe.plan_road();
            if road_probe.state.construction.projects.len() > before {
                if let Some(project) = road_probe
                    .state
                    .construction
                    .projects
                    .iter()
                    .rev()
                    .find(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
                {
                    let from = project.from.and_then(|id| road_probe.node(id));
                    let to = project.to.and_then(|id| road_probe.node(id));
                    road_proposal = Some((
                        project.id,
                        from.map(|node| (node.id, node.x, node.z)),
                        to.map(|node| (node.id, node.x, node.z)),
                        project.reason.clone(),
                    ));
                }
                break;
            }
        }
        println!("{name}: real_road_proposal={road_proposal:?}");
    }
}

#[test]
fn road_split_rejections_leave_state_and_ids_unchanged() {
    let mut engine = Engine::new_legacy(8);
    let host = engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.kind == RoadKind::Arterial && road.stage == Stage::Open)
        .unwrap()
        .clone();
    let before = engine.save_json();
    let before_access_revision = engine.state.network.access_geometry_revision;
    assert_eq!(
        engine.try_split_open_road(host.id, 0.001),
        Err(RoadSplitError::SegmentTooShort)
    );
    assert_eq!(engine.save_json(), before);
    assert_eq!(
        engine.state.network.access_geometry_revision,
        before_access_revision
    );

    let parcel_id = engine.add_parcel(250.0, 60.0, 14.0, 12.0, host.from, 0.8, false);
    engine.state.land.parcel_frontages.push(ParcelFrontage {
        parcel_id,
        road_id: host.id,
        along: 0.75,
        side: -1,
    });
    let before = engine.save_json();
    assert_eq!(
        engine.try_split_open_road(host.id, 0.5),
        Err(RoadSplitError::UnsafeFrontage)
    );
    assert_eq!(engine.save_json(), before);

    // An otherwise valid host cannot be split when the route expansion
    // would cross the public per-traveler route bound.
    engine.state.land.parcel_frontages.clear();
    engine
        .state
        .land
        .parcels
        .retain(|parcel| parcel.id != parcel_id);
    let mut route = Vec::with_capacity(MAX_ROUTE_NODES);
    for index in 0..MAX_ROUTE_NODES {
        route.push(if index % 2 == 0 { host.from } else { host.to });
    }
    let long_traveler_id = engine.id();
    engine.state.people.travelers.push(Traveler {
        id: long_traveler_id,
        kind: 1,
        route,
        route_lengths: vec![host.length; MAX_ROUTE_NODES - 1],
        edge_index: 0,
        edge_progress: 0.0,
        speed: 1.0,
        destination: host.to,
        activity: crate::TravelerActivity::Stroll,
        resident: 0,
        name: "Long route".into(),
        origin_building: None,
        destination_building: None,
        wait_ticks: 0,
        origin_access: None,
        destination_access: None,
        access_phase: 1,
        access_progress: 0.0,
    });
    let before = engine.save_json();
    assert_eq!(
        engine.try_split_open_road(host.id, 0.5),
        Err(RoadSplitError::Capacity)
    );
    assert_eq!(engine.save_json(), before);

    let mut bus_engine = Engine::new_legacy(9);
    let bus_host = bus_engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.kind == RoadKind::Arterial && road.stage == Stage::Open)
        .unwrap()
        .clone();
    let long_bus_route: Vec<_> = (0..MAX_BUS_ROUTE_NODES)
        .map(|index| {
            if index % 2 == 0 {
                bus_host.from
            } else {
                bus_host.to
            }
        })
        .collect();
    let bus_line_id = bus_engine.id();
    bus_engine.state.transit.bus_line = Some(BusLine {
        id: bus_line_id,
        stops: vec![bus_host.from, bus_host.to, bus_host.from],
        route: long_bus_route,
        active: true,
        graph_revision: 4,
    });
    let before = bus_engine.save_json();
    assert_eq!(
        bus_engine.try_split_open_road(bus_host.id, 0.5),
        Err(RoadSplitError::Capacity)
    );
    assert_eq!(bus_engine.save_json(), before);

    let mut project_engine = Engine::new_legacy(10);
    let project_host = project_engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.kind == RoadKind::Arterial && road.stage == Stage::Open)
        .unwrap()
        .clone();
    let project_id = project_engine.id();
    project_engine.state.construction.projects.push(Project {
        id: project_id,
        kind: ProjectKind::Road,
        from: Some(project_host.from),
        to: Some(project_host.to),
        parcel_id: None,
        stage: Stage::Survey,
        progress: 0.5,
        reason: "Active host construction".into(),
        duration: 8.0,
        started_tick: 0,
    });
    let before = project_engine.save_json();
    assert_eq!(
        project_engine.try_split_open_road(project_host.id, 0.5),
        Err(RoadSplitError::ActiveProject)
    );
    assert_eq!(project_engine.save_json(), before);

    let mut plan_engine = Engine::new_legacy(11);
    let plan_host = plan_engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.kind == RoadKind::Arterial && road.stage == Stage::Open)
        .unwrap()
        .clone();
    plan_engine.state.legacy.neighborhood_plan = Some(NeighborhoodPlan {
        anchor_node: plan_host.from,
        segments: vec![(plan_host.from, plan_host.to); 4],
        next_segment: 0,
        coordinated_lots: false,
        selected_lots: None,
        spine: None,
    });
    let before = plan_engine.save_json();
    assert_eq!(
        plan_engine.try_split_open_road(plan_host.id, 0.5),
        Err(RoadSplitError::ActivePlan)
    );
    assert_eq!(plan_engine.save_json(), before);
}

#[test]
fn indexed_connectivity_matches_source_order_scan() {
    for seed in [1, 23, 87231] {
        let mut engine = Engine::new_legacy(seed);
        let gateway = engine
            .state
            .network
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Gateway)
            .unwrap()
            .id;
        let a = engine.add_node(120.0, 120.0, NodeKind::Junction);
        let b = engine.add_node(160.0, 120.0, NodeKind::Junction);
        let c = engine.add_node(200.0, 120.0, NodeKind::Junction);
        // Include disconnected components, cycles and parallel edges;
        // the scan oracle below is the previous connectivity algorithm.
        engine.add_road(a, b, RoadKind::Local, Stage::Open, 1.0);
        engine.add_road(b, c, RoadKind::Local, Stage::Open, 1.0);
        engine.add_road(c, a, RoadKind::Local, Stage::Open, 1.0);
        engine.add_road(a, b, RoadKind::Local, Stage::Open, 1.0);
        for connected in [false, true] {
            if connected {
                engine.add_road(gateway, a, RoadKind::Local, Stage::Open, 1.0);
            }
            let mut seen = BTreeSet::from([gateway]);
            let mut queue = VecDeque::from([gateway]);
            while let Some(node) = queue.pop_front() {
                for road in &engine.state.network.roads {
                    if road.stage == Stage::Open && (road.from == node || road.to == node) {
                        let next = if road.from == node {
                            road.to
                        } else {
                            road.from
                        };
                        if seen.insert(next) {
                            queue.push_back(next);
                        }
                    }
                }
            }
            let expected = engine
                .state
                .network
                .roads
                .iter()
                .map(|r| r.stage == Stage::Open && seen.contains(&r.from) && seen.contains(&r.to))
                .collect::<Vec<_>>();
            engine.recompute_connectivity();
            assert_eq!(
                engine
                    .state
                    .network
                    .roads
                    .iter()
                    .map(|r| r.connected)
                    .collect::<Vec<_>>(),
                expected
            );
            let revision = engine.access_geometry_revision_token();
            engine.recompute_connectivity();
            assert_eq!(engine.access_geometry_revision_token(), revision);
        }
    }
}
