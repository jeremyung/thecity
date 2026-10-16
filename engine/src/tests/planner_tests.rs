use super::*;

#[test]
fn early_growth_fills_connected_blocks_before_expanding_the_map() {
    let mut engine = Engine::new_legacy(87_231);
    engine.set_development_rate(16);
    let milestones = [
        (1_000, 5, 9),
        (3_000, 11, 16),
        (6_000, 16, 22),
        (12_000, 22, 30),
    ];
    let mut previous_tick = 0;
    for (tick, minimum_buildings, maximum_roads) in milestones {
        let mut remaining = tick - previous_tick;
        while remaining > 0 {
            let batch = remaining.min(2_000);
            engine.step(batch);
            remaining -= batch;
        }
        previous_tick = tick;
        assert_eq!(
            engine.state.tick, tick as u64,
            "milestone must advance exactly"
        );
        engine.validate().unwrap();
        let snapshot = engine.snapshot();
        let open_roads = snapshot
            .roads
            .iter()
            .filter(|road| road.stage == Stage::Open)
            .count();
        let open_buildings = snapshot
            .buildings
            .iter()
            .filter(|building| building.stage == Stage::Open)
            .count();
        let building_projects = snapshot
            .projects
            .iter()
            .filter(|project| project.kind == ProjectKind::Building && project.progress < 1.0)
            .count();
        assert!(
            open_buildings + building_projects >= minimum_buildings,
            "tick {tick}: only {} open and {building_projects} planned buildings",
            open_buildings
        );
        assert!(
            open_roads <= maximum_roads,
            "tick {tick}: {open_roads} open roads outran neighborhood infill"
        );
        println!(
            "growth milestone tick={tick} roads={open_roads} buildings={open_buildings} planned_buildings={building_projects} population={}",
            snapshot.stats.population
        );
    }
}

#[test]
fn road_planner_keeps_frontier_candidates_connected_and_bounded() {
    let mut engine = Engine::new_legacy(0);
    engine.set_development_rate(60);
    for _ in 0..36 {
        engine.step(2_000);
    }
    engine.plan_road();
    let after = engine
        .state
        .projects
        .iter()
        .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        .count();
    let admitted: Vec<_> = engine
        .state
        .projects
        .iter()
        .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        .collect();
    for project in &admitted {
        let endpoints = project
            .from
            .zip(project.to)
            .and_then(|(from, to)| Some((engine.node(from)?.clone(), engine.node(to)?.clone())));
        let frontage_count = endpoints
            .as_ref()
            .map(|(from, to)| engine.viable_frontage_benefit_count(from, to, 2))
            .unwrap_or(0);
        assert!(
            frontage_count >= 2,
            "planner admitted a road without viable frontage"
        );
        assert!(
            endpoints.is_some_and(|(from, to)| {
                engine.node_connected(from.id) || engine.node_connected(to.id)
            }),
            "frontier road must attach to the connected network"
        );
    }
    assert!(
        after <= MAX_PROJECTS,
        "frontier probing exceeded the active project cap"
    );
    let milestone = engine.snapshot();
    let active_sites = engine
        .state
        .buildings
        .iter()
        .filter(|b| site_active(b))
        .count();
    let developable_lots = engine
        .state
        .parcels
        .iter()
        .filter(|p| {
            !p.protected
                && engine.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                && engine.node_connected(p.access_node)
        })
        .count();
    let profile = |engine: &Engine| {
        let building_footprint: f32 = engine
            .state
            .buildings
            .iter()
            .filter(|building| building.stage != Stage::Demolished)
            .map(|building| building.width * building.depth)
            .sum();
        let open_ordinary: Vec<_> = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
            })
            .collect();
        let ordinary_length: f32 = open_ordinary.iter().map(|road| road.length).sum();
        let active = engine
            .state
            .buildings
            .iter()
            .filter(|b| site_active(b))
            .count();
        let usable_empty = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| {
                !parcel.protected
                    && parcel.reservation.is_none()
                    && engine.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
                    && engine.node_connected(parcel.access_node)
                    && !engine
                        .state
                        .buildings
                        .iter()
                        .any(|building| building.parcel_id == parcel.id)
            })
            .count();
        format!(
            "tick={} buildings={} footprint_m2={:.0} roads={} ordinary_length_m={:.0} active_sites={} unbuilt_usable={} historic_roads={} demand={:.2} population={} plan={:?} infill_due={} road_projects={} building_projects={}",
            engine.state.tick,
            engine.state.buildings.len(),
            building_footprint,
            open_ordinary.len(),
            ordinary_length,
            active,
            usable_empty,
            engine.state.neighborhood_road_ids.len(),
            engine.state.demand,
            engine.state.population,
            engine
                .state
                .neighborhood_plan
                .as_ref()
                .map(|plan| (plan.next_segment, plan.segments.len())),
            engine.state.road_infill_due,
            engine
                .state
                .projects
                .iter()
                .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
                .count(),
            engine
                .state
                .projects
                .iter()
                .filter(|project| project.kind == ProjectKind::Building && project.progress < 1.0)
                .count(),
        )
    };
    println!("seed0 density profile at 72k: {}", profile(&engine));
    assert!(
        matches!(milestone.stage.as_str(), "TransitCity" | "Metropolis" | "Maturity"),
        "frontier case did not reach transit-era growth: stage={} population={} active_sites={} developable_lots={} infill={:.3}",
        milestone.stage,
        milestone.stats.population,
        active_sites,
        developable_lots,
        active_sites as f32 / developable_lots.max(1) as f32,
    );
    assert!(active_sites >= 200);
    let ordinary_roads: BTreeSet<u32> = engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.connected
                && road.stage == Stage::Open
                && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
        })
        .map(|road| road.id)
        .collect();
    let frontage_count = engine
        .state
        .parcel_frontages
        .iter()
        .filter(|frontage| ordinary_roads.contains(&frontage.road_id))
        .count();
    assert!(frontage_count as f32 / ordinary_roads.len() as f32 > 2.0);
    assert!(engine.state.parcels.iter().all(|parcel| {
        engine.parcel_clear_of_river(parcel.x, parcel.z, parcel.width, parcel.depth)
            && engine.node_connected(parcel.access_node)
    }));
    assert!(engine
        .state
        .roads
        .iter()
        .filter(|road| road.stage == Stage::Open)
        .all(|road| { road.connected || road.kind == RoadKind::Bridge }));
    engine.validate().unwrap();
    for _ in 0..85 {
        engine.step(2_000);
        engine.validate().unwrap();
    }
    println!("seed0 density profile at 242k: {}", profile(&engine));
    let mut probe = Engine::restore_json(&engine.save_json()).unwrap();
    let mut probe_project = false;
    for _ in 0..200 {
        probe.plan_road();
        if probe
            .state
            .projects
            .iter()
            .any(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        {
            probe_project = true;
            break;
        }
    }
    println!(
        "seed0 bounded road-plan probe found project={probe_project} cursor={}",
        probe.state.road_origin_cursor
    );
    assert!(
        engine.state.travelers.len() > 0,
        "mature city stopped daily trips"
    );
    assert!(engine
        .state
        .buildings
        .iter()
        .any(|building| { is_commercial(building) && site_active(building) }));
    assert!(engine.state.buildings.iter().all(|building| {
        !is_commercial(building) || !building.business_active || building.vacant_days == 0
    }));
}

#[test]
fn rotated_parcel_corner_blocks_a_corridor_that_passes_center_clearance() {
    let mut engine = Engine::new_legacy(3);
    engine.state.parcels.clear();
    engine.state.parcels.push(Parcel {
        id: 10_000,
        x: 0.0,
        z: 0.0,
        width: 14.0,
        depth: 12.0,
        rotation: Some(std::f32::consts::FRAC_PI_4),
        access_node: 0,
        suitability: 1.0,
        protected: false,
        reservation: None,
        open_space: None,
    });
    let context = engine.collision_context();
    let road_z = 12.1;
    let center_clearance = 14.0_f32.max(12.0) * 0.5 + 5.0;
    assert!(road_z > center_clearance);
    let footprint_clearance =
        geometry::rect_segment_distance(&context.parcel_footprints[0], -20.0, road_z, 20.0, road_z);
    assert!(
        footprint_clearance < 5.0,
        "footprint clearance: {footprint_clearance}"
    );
    assert!(
        !engine.road_corridor_clear_of_parcels_with_context(-20.0, road_z, 20.0, road_z, &context,)
    );
    assert!(engine.road_corridor_clear_of_parcels_with_context(-20.0, 15.0, 20.0, 15.0, &context,));
}

#[test]
fn parcel_corridor_uses_exact_clearance_instead_of_max_dimension_radius() {
    let mut engine = Engine::new_legacy(3);
    engine.state.parcels.clear();
    engine.state.parcels.push(Parcel {
        id: 10_000,
        x: 0.0,
        z: 0.0,
        width: 20.0,
        depth: 12.0,
        rotation: Some(0.0),
        access_node: 0,
        suitability: 1.0,
        protected: false,
        reservation: None,
        open_space: None,
    });
    let context = engine.collision_context();

    // The exact edge clearance is 8 m, although the old max-dimension
    // radius was 15 m and rejected this perfectly parallel street.
    assert!(engine.road_corridor_clear_of_parcels_with_context(-20.0, 14.0, 20.0, 14.0, &context,));
    assert!(!engine.road_corridor_clear_of_parcels_with_context(-20.0, 10.0, 20.0, 10.0, &context,));
    assert!(engine.road_corridor_clear_of_parcels_with_context(-20.0, 16.0, 20.0, 16.0, &context,));
}

#[test]
fn seed_87231_growth_remains_valid_and_canonical_after_corridor_checks() {
    let mut engine = Engine::new_legacy(87_231);
    engine.set_development_rate(16);
    for _ in 0..121 {
        engine.step(2_000);
    }
    assert_eq!(engine.state.tick, 242_000);
    engine.validate().unwrap();
    let save = engine.save_json();
    let restored = Engine::restore_json(&save).unwrap();
    assert_eq!(restored.save_json(), save);
}

#[test]
fn bounded_frontier_cursor_reaches_later_viable_origins() {
    let mut engine = Engine::new_legacy(87231);
    engine.set_development_rate(60);
    engine.step(12_000);
    assert!(
        engine
            .state
            .roads
            .iter()
            .filter(|road| road.stage == Stage::Open)
            .count()
            < 40
    );
    let open_buildings: Vec<&Building> = engine
        .state
        .buildings
        .iter()
        .filter(|building| building.stage == Stage::Open)
        .collect();
    let center = open_buildings.iter().fold((0.0, 0.0), |(x, z), building| {
        (
            x + building.x / open_buildings.len() as f32,
            z + building.z / open_buildings.len() as f32,
        )
    });
    let connected: BTreeSet<u32> = engine
        .state
        .roads
        .iter()
        .filter(|road| road.connected && road.stage == Stage::Open)
        .flat_map(|road| [road.from, road.to])
        .collect();
    let mut origins: Vec<&Node> = engine
        .state
        .nodes
        .iter()
        .filter(|node| connected.contains(&node.id))
        .filter(|node| {
            open_buildings
                .iter()
                .map(|building| (building.x - node.x).hypot(building.z - node.z))
                .min_by(f32::total_cmp)
                .is_some_and(|distance| distance <= 110.0)
        })
        .collect();
    origins.sort_by(|a, b| {
        let ar = (a.x - center.0).hypot(a.z - center.1);
        let br = (b.x - center.0).hypot(b.z - center.1);
        br.total_cmp(&ar).then_with(|| a.id.cmp(&b.id))
    });
    let blocked_origin = origins[0].id;
    let neighbor = engine
        .state
        .roads
        .iter()
        .find(|road| road.connected && (road.from == blocked_origin || road.to == blocked_origin))
        .map(|road| {
            if road.from == blocked_origin {
                road.to
            } else {
                road.from
            }
        })
        .unwrap();
    let blocker_id = engine.id();
    engine.state.projects.push(Project {
        id: blocker_id,
        kind: ProjectKind::Road,
        from: Some(blocked_origin),
        to: Some(neighbor),
        parcel_id: None,
        stage: Stage::Survey,
        progress: 0.2,
        reason: "test blocker for one frontier origin".into(),
        duration: 2.0,
        started_tick: engine.state.tick,
    });
    engine.state.road_origin_cursor = 0;
    engine.plan_road();
    let later_project = engine
        .state
        .projects
        .iter()
        .find(|project| {
            project.kind == ProjectKind::Road && project.id != blocker_id && project.progress < 1.0
        })
        .expect("bounded origin search should advance past the blocked origin");
    assert_ne!(later_project.from, Some(blocked_origin));
    engine
        .state
        .projects
        .retain(|project| project.id != blocker_id);
    let project = engine
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        .cloned()
        .unwrap();
    let from = project.from.and_then(|id| engine.node(id)).unwrap();
    let to = project.to.and_then(|id| engine.node(id)).unwrap();
    assert!(engine.node_connected(from.id));
    assert!(engine.ordinary_road_clear_of_river(from.x, from.z, to.x, to.z, 4.5));
    engine.validate().unwrap();
}

#[test]
fn spine_candidate_validation_and_score_include_deep_road_cost() {
    let corners = [
        WorldPoint { x: 250.0, z: 250.0 },
        WorldPoint { x: 332.0, z: 250.0 },
        WorldPoint { x: 332.0, z: 332.0 },
        WorldPoint { x: 250.0, z: 332.0 },
    ];
    let corners_block = corners.map(|point| blocks::BlockPoint {
        x: point.x,
        z: point.z,
    });
    let deep = spine_plan::SpinePlan {
        corners: corners_block,
        recipe_version: spine_plan::RECIPE_VERSION,
        stage: spine_plan::SpineStage::Reserved,
    };
    let lots = Engine::spine_candidate_lots(&deep).expect("82m square has a canonical spine");
    assert!(lots.len() >= 4);
    let deep_candidate = Engine::ranked_neighborhood_candidate(
        1,
        corners,
        [Some(1), None, None, None],
        None,
        None,
        &lots,
        [1, 3, 0],
        Some(deep.clone()),
    );
    assert_eq!(deep_candidate.score.new_road_length_cm, 41_000);
    assert_eq!(deep_candidate.score.new_road_count, 7);

    let shallow_corners = [
        corners[0],
        corners[1],
        WorldPoint { x: 332.0, z: 300.0 },
        WorldPoint { x: 250.0, z: 300.0 },
    ];
    let shallow = blocks::subdivide_rectangular_block(
        shallow_corners.map(|point| blocks::BlockPoint {
            x: point.x,
            z: point.z,
        }),
        8.0,
        11.0,
        2.2,
    );
    let conventional_candidate = Engine::ranked_neighborhood_candidate(
        1,
        shallow_corners,
        [Some(1), None, None, None],
        None,
        None,
        &shallow,
        [1, 1, 0],
        None,
    );
    assert!(development::neighborhood_score_is_better(
        conventional_candidate.score,
        deep_candidate.score,
    ));
    let shallow_as_spine = spine_plan::SpinePlan {
        corners: shallow_corners.map(|point| blocks::BlockPoint {
            x: point.x,
            z: point.z,
        }),
        recipe_version: spine_plan::RECIPE_VERSION,
        stage: spine_plan::SpineStage::Reserved,
    };
    assert!(shallow_as_spine.validate().is_err());

    let mut engine = engine_with_developed_plan_site_without_reusable_edge();
    let planner_anchor = engine.node(engine.state.nodes[2].id).unwrap().clone();
    let collision = engine.collision_context();
    let tower_envelopes = engine.available_station_tower_envelopes();
    let mut feasible_spines = 0;
    let mut feasible_example = None;
    for (ux, uz, vx, vz) in [
        (1.0_f32, 0.0_f32, 0.0_f32, 1.0_f32),
        (1.0, 0.0, 0.0, -1.0),
        (-1.0, 0.0, 0.0, 1.0),
        (-1.0, 0.0, 0.0, -1.0),
        (0.0, 1.0, 1.0, 0.0),
        (0.0, 1.0, -1.0, 0.0),
        (0.0, -1.0, 1.0, 0.0),
        (0.0, -1.0, -1.0, 0.0),
    ] {
        let candidate_corners = [
            WorldPoint {
                x: planner_anchor.x,
                z: planner_anchor.z,
            },
            WorldPoint {
                x: planner_anchor.x + ux * 82.0,
                z: planner_anchor.z + uz * 82.0,
            },
            WorldPoint {
                x: planner_anchor.x + ux * 82.0 + vx * 82.0,
                z: planner_anchor.z + uz * 82.0 + vz * 82.0,
            },
            WorldPoint {
                x: planner_anchor.x + vx * 82.0,
                z: planner_anchor.z + vz * 82.0,
            },
        ];
        let plan = spine_plan::SpinePlan {
            corners: candidate_corners.map(|point| blocks::BlockPoint {
                x: point.x,
                z: point.z,
            }),
            recipe_version: spine_plan::RECIPE_VERSION,
            stage: spine_plan::SpineStage::Reserved,
        };
        if engine.neighborhood_candidate_clear_with_layout(
            &planner_anchor,
            &candidate_corners,
            &tower_envelopes,
            &collision,
            [Some(planner_anchor.id), None, None, None],
            None,
            None,
            Some(&plan),
        ) {
            feasible_spines += 1;
            feasible_example = Some((candidate_corners, plan));
        }
    }
    assert!(
        feasible_spines > 0,
        "developed catchment has a real feasible deep candidate"
    );
    let (valid_corners, valid_plan) = feasible_example.unwrap();
    let before = engine.save_json();
    let accepts = |engine: &Engine, plan: &spine_plan::SpinePlan| {
        engine.neighborhood_candidate_clear_with_layout(
            &planner_anchor,
            &valid_corners,
            &tower_envelopes,
            &collision,
            [Some(planner_anchor.id), None, None, None],
            None,
            None,
            Some(plan),
        )
    };
    // Establish that every unchanged predicate accepts this exact example.
    assert!(accepts(&engine, &valid_plan));
    let mut mismatched = valid_plan.clone();
    for corner in &mut mismatched.corners {
        corner.x += 1.0;
    }
    assert!(
        mismatched.validate().is_ok(),
        "translation preserves recipe geometry"
    );
    assert!(!accepts(&engine, &mismatched));
    let mut wrong_stage = valid_plan.clone();
    wrong_stage.stage = spine_plan::SpineStage::PerimeterReady {
        host_road_ids: [10, 11],
    };
    assert!(
        wrong_stage.validate().is_ok(),
        "stage IDs are structurally valid"
    );
    assert!(!accepts(&engine, &wrong_stage));
    assert_eq!(
        engine.save_json(),
        before,
        "candidate checks must not mutate the world"
    );
    let prior_next_id = engine.state.next_id;
    engine.state.next_id = u32::MAX - 1;
    let exhausted = engine.save_json();
    assert!(
        !accepts(&engine, &valid_plan),
        "reserve all eventual IDs before admission"
    );
    assert_eq!(engine.save_json(), exhausted);
    engine.state.next_id = prior_next_id;
    assert!(accepts(&engine, &valid_plan));
    assert!(engine.try_create_neighborhood_plan());
    assert!(engine
        .state
        .neighborhood_plan
        .as_ref()
        .unwrap()
        .spine
        .is_none());
}

#[test]
fn neighborhood_plan_reserves_connected_corridors_and_replays_pending_work() {
    let mut engine = engine_with_developed_plan_site_without_reusable_edge();
    assert!(engine.try_create_neighborhood_plan());
    let plan = engine.state.neighborhood_plan.clone().unwrap();
    assert_eq!(plan.segments.len(), 4);
    assert_eq!(plan.next_segment, 0);

    // A lot envelope over a future street is unavailable before any road
    // project opens; this is the reservation that prevents lots sealing
    // the planned block.
    let (from, to) = plan.segments[2];
    let a = engine.node(from).unwrap().clone();
    let b = engine.node(to).unwrap().clone();
    let center = WorldPoint {
        x: (a.x + b.x) * 0.5,
        z: (a.z + b.z) * 0.5,
    };
    assert!(!engine.parcel_geometry_clear(
        center.x,
        center.z,
        NEIGHBORHOOD_PARCEL_SIZE,
        NEIGHBORHOOD_PARCEL_SIZE
    ));

    assert!(engine.plan_next_neighborhood_segment());
    let pending = engine
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        .cloned()
        .expect("first plan edge is ordinary road construction");
    assert_eq!(pending.from, Some(plan.segments[0].0));
    assert_eq!(pending.to, Some(plan.segments[0].1));
    assert!(!engine.state.roads.iter().any(|road| {
        road.from == plan.segments[0].0
            && road.to == plan.segments[0].1
            && road.stage == Stage::Open
    }));
    let (reserved_from, reserved_to) = plan.segments[2];
    let blocked_a = engine.node(reserved_from).unwrap();
    let blocked_b = engine.node(reserved_to).unwrap();
    let crossing = Node {
        id: u32::MAX - 20,
        x: (blocked_a.x + blocked_b.x) * 0.5,
        z: (blocked_a.z + blocked_b.z) * 0.5 - 30.0,
        kind: NodeKind::Candidate,
    };
    let crossing_end = Node {
        id: u32::MAX - 21,
        x: crossing.x,
        z: crossing.z + 60.0,
        kind: NodeKind::Candidate,
    };
    assert!(!engine.neighborhood_corridor_clear_for_road(&crossing, &crossing_end, crossing.id));
    let (active_from, active_to) = plan.segments[0];
    let active_a = engine.node(active_from).unwrap();
    let active_b = engine.node(active_to).unwrap();
    let dx = active_b.x - active_a.x;
    let dz = active_b.z - active_a.z;
    let length = dx.hypot(dz).max(1.0);
    let active_lot = WorldPoint {
        x: (active_a.x + active_b.x) * 0.5 - dz / length * 15.0,
        z: (active_a.z + active_b.z) * 0.5 + dx / length * 15.0,
    };
    assert!(engine.parcel_geometry_clear(
        active_lot.x,
        active_lot.z,
        NEIGHBORHOOD_PARCEL_SIZE,
        NEIGHBORHOOD_PARCEL_SIZE
    ));

    let save = engine.save_json();
    let mut replay = Engine::restore_json(&save).unwrap();
    assert_eq!(engine.save_json(), replay.save_json());
    let pending_id = pending.id;
    engine
        .state
        .projects
        .iter_mut()
        .find(|project| project.id == pending_id)
        .unwrap()
        .progress = 1.0;
    replay
        .state
        .projects
        .iter_mut()
        .find(|project| project.id == pending_id)
        .unwrap()
        .progress = 1.0;
    engine.complete_project(pending_id);
    replay.complete_project(pending_id);
    assert_eq!(engine.save_json(), replay.save_json());
    let opened = engine
        .state
        .roads
        .iter()
        .find(|road| road.from == plan.segments[0].0 && road.to == plan.segments[0].1)
        .unwrap();
    assert_eq!(opened.stage, Stage::Open);
    assert!(opened.connected);
    assert!(engine.state.parcels.iter().any(|parcel| {
        engine
            .state
            .parcel_frontages
            .iter()
            .any(|frontage| frontage.parcel_id == parcel.id && frontage.road_id == opened.id)
    }));

    // A road can only be scheduled after its predecessor is connected.
    assert!(engine.plan_next_neighborhood_segment());
    let second = engine
        .state
        .projects
        .iter()
        .find(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        .cloned()
        .unwrap();
    assert_eq!(second.from, Some(plan.segments[1].0));
    assert!(engine.node_connected(second.from.unwrap()));

    for segment_index in 1..plan.segments.len() {
        let (from, to) = plan.segments[segment_index];
        let project = if segment_index == 1 {
            second.clone()
        } else {
            assert!(engine.plan_next_neighborhood_segment());
            engine
                .state
                .projects
                .iter()
                .find(|project| {
                    project.kind == ProjectKind::Road
                        && project.progress < 1.0
                        && project.from == Some(from)
                        && project.to == Some(to)
                })
                .cloned()
                .expect("each next perimeter street is built through a project")
        };
        assert_eq!(project.from, Some(from));
        assert_eq!(project.to, Some(to));
        engine
            .state
            .projects
            .iter_mut()
            .find(|candidate| candidate.id == project.id)
            .unwrap()
            .progress = 1.0;
        engine.complete_project(project.id);
        let road = engine
            .state
            .roads
            .iter()
            .find(|road| road.from == from && road.to == to)
            .unwrap();
        assert_eq!(road.stage, Stage::Open);
        assert!(road.connected);
    }
    assert!(engine.state.neighborhood_plan.is_none());
    assert_eq!(engine.state.neighborhood_road_ids.len(), 4);
    assert!(plan.segments.iter().all(|(from, to)| {
        engine
            .state
            .roads
            .iter()
            .find(|road| {
                (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
            })
            .is_some_and(|road| engine.is_neighborhood_plan_road(road.id))
    }));

    // Legacy saves retained the completed plan itself as their only
    // classification source. Restoring that shape migrates its roads to
    // the historic index and retires the completed active plan once.
    let mut legacy_completed: serde_json::Value =
        serde_json::from_str(&engine.save_json()).unwrap();
    let mut completed_plan = plan.clone();
    completed_plan.next_segment = plan.segments.len();
    legacy_completed["neighborhood_plan"] = serde_json::to_value(&completed_plan).unwrap();
    legacy_completed
        .as_object_mut()
        .unwrap()
        .remove("neighborhood_road_ids");
    let migrated = Engine::restore_json(&legacy_completed.to_string()).unwrap();
    assert!(migrated.state.neighborhood_plan.is_none());
    assert_eq!(migrated.state.neighborhood_road_ids.len(), 4);
    assert!(plan.segments.iter().all(|(from, to)| {
        migrated
            .state
            .roads
            .iter()
            .find(|road| {
                (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
            })
            .is_some_and(|road| migrated.is_neighborhood_plan_road(road.id))
    }));
    let migrated_save = migrated.save_json();
    assert_eq!(
        migrated_save,
        Engine::restore_json(&migrated_save).unwrap().save_json()
    );

    let accessible_lots: Vec<_> = engine
        .state
        .parcels
        .iter()
        .filter(|parcel| {
            engine.node_connected(parcel.access_node)
                && engine.state.parcel_frontages.iter().any(|frontage| {
                    frontage.parcel_id == parcel.id
                        && engine
                            .state
                            .roads
                            .iter()
                            .any(|road| road.id == frontage.road_id && road.connected)
                })
        })
        .collect();
    let plan_center = WorldPoint {
        x: plan
            .segments
            .iter()
            .map(|(from, _)| engine.node(*from).unwrap().x)
            .sum::<f32>()
            * 0.25,
        z: plan
            .segments
            .iter()
            .map(|(from, _)| engine.node(*from).unwrap().z)
            .sum::<f32>()
            * 0.25,
    };
    let mut plan_lot_count = 0;
    for (from, to) in &plan.segments {
        let a = engine.node(*from).unwrap();
        let b = engine.node(*to).unwrap();
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = dx.hypot(dz).max(1.0);
        let midpoint = WorldPoint {
            x: (a.x + b.x) * 0.5,
            z: (a.z + b.z) * 0.5,
        };
        let interior_side = if (plan_center.x - midpoint.x) * (-dz / length)
            + (plan_center.z - midpoint.z) * (dx / length)
            >= 0.0
        {
            1
        } else {
            -1
        };
        let road = engine
            .state
            .roads
            .iter()
            .find(|road| road.from == *from && road.to == *to)
            .unwrap();
        let road_lots: Vec<_> = engine
            .state
            .parcel_frontages
            .iter()
            .filter(|frontage| frontage.road_id == road.id)
            .collect();
        plan_lot_count += road_lots.len();
        if length >= 70.0 {
            assert!(
                road_lots
                    .iter()
                    .filter(|frontage| frontage.side == interior_side)
                    .count()
                    >= 2,
                "long edge has fewer than two clear inside lots"
            );
        }
    }
    assert!(plan_lot_count >= 12, "plan frontage lots: {plan_lot_count}");
    assert!(
        accessible_lots.len() >= 12,
        "lots: {}",
        accessible_lots.len()
    );
    let developed = accessible_lots[0];
    let building_id = engine.add_building(
        developed.id,
        BuildingKind::Home,
        3,
        Stage::Open,
        1.0,
        12,
        "New block machiya",
        false,
        vec!["Built on a connected block street".into()],
    );
    assert!(engine.building_access_segment(building_id).is_some());
}

#[test]
fn neighborhood_block_is_reserved_before_frontier_and_starts_next_cadence() {
    let mut engine = engine_with_developed_plan_site_without_reusable_edge();
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
    engine.state.tick = 80;

    assert_ne!(engine.state.tick % 1_600, 0);
    assert_eq!(engine.state.road_infill_due, 0);
    assert!(!engine.neighborhood_block_has_unbuilt_frontage());
    assert!(engine.state.projects.is_empty());
    assert!(engine.state.neighborhood_plan.is_none());

    engine.evaluate_growth();

    let plan = engine
        .state
        .neighborhood_plan
        .as_ref()
        .expect("a feasible neighborhood is reserved at the ordinary road cadence");
    assert_eq!(
        plan.next_segment, 0,
        "reservation does not start work immediately"
    );
    assert!(!engine
        .state
        .projects
        .iter()
        .any(|project| project.kind == ProjectKind::Road && project.progress < 1.0));

    engine.state.tick = 160;
    engine.evaluate_growth();
    let plan = engine.state.neighborhood_plan.as_ref().unwrap();
    assert_eq!(plan.next_segment, 1);
    assert!(engine
        .state
        .projects
        .iter()
        .any(|project| { project.kind == ProjectKind::Road && project.progress < 1.0 }));
    engine.validate().unwrap();
}

#[test]
fn coordinated_block_reuses_open_edge_and_waits_for_three_new_streets() {
    let mut base = engine_with_developed_plan_site();
    let mut engine = Engine::restore_json(&base.save_json()).unwrap();
    assert!(engine.try_create_neighborhood_plan());
    let plan = engine.state.neighborhood_plan.clone().unwrap();
    assert_eq!(plan.segments.len(), 4);
    assert_eq!(plan.next_segment, 0);
    let (reuse_from, reuse_to) = plan.segments[0];
    let reused = engine
        .state
        .roads
        .iter()
        .find(|road| {
            road.stage == Stage::Open
                && ((road.from == reuse_from && road.to == reuse_to)
                    || (road.from == reuse_to && road.to == reuse_from))
        })
        .cloned()
        .expect("the first plan edge reuses an existing open street");
    assert!(reused.connected);
    assert_eq!(
        engine
            .state
            .roads
            .iter()
            .filter(|road| {
                (road.from == reuse_from && road.to == reuse_to)
                    || (road.from == reuse_to && road.to == reuse_from)
            })
            .count(),
        1,
        "reuse must not duplicate the existing road"
    );

    // The candidate validator must reject occupied interior geometry
    // while accepting the exact same rectangle before that lot exists.
    let corners: [WorldPoint; 4] = plan
        .segments
        .iter()
        .map(|(from, _)| {
            let node = engine.node(*from).unwrap();
            WorldPoint {
                x: node.x,
                z: node.z,
            }
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let anchor = base.node(plan.anchor_node).unwrap().clone();
    let tower_envelopes = base.available_station_tower_envelopes();
    let context = base.collision_context();
    let available = base.available_coordinated_block_lots(&corners, &context);
    assert_eq!(available.len(), 10);
    assert!(base.neighborhood_candidate_clear_with_edge(
        &anchor,
        &corners,
        &tower_envelopes,
        &context,
        [Some(reuse_from), Some(reuse_to), None, None],
        Some((0, reused.id)),
        Some(&available),
    ));
    let blocked_lot = *available
        .iter()
        .find(|lot| lot.frontage_edge == 0 && (0.4..0.6).contains(&lot.frontage_t))
        .expect("fixture has a center lot away from the new perimeter roads");
    let occupied_probe = base.add_parcel(
        blocked_lot.center.x,
        blocked_lot.center.z,
        2.0,
        2.0,
        plan.anchor_node,
        0.5,
        false,
    );
    assert_ne!(occupied_probe, 0, "the collision probe must be admitted");
    let occupied_context = base.collision_context();
    let filtered = base.available_coordinated_block_lots(&corners, &occupied_context);
    assert_eq!(filtered.len(), 9, "occupied frontage lots remain untouched");
    assert!(!filtered.contains(&blocked_lot));
    assert!(base.neighborhood_candidate_clear_with_edge(
        &anchor,
        &corners,
        &tower_envelopes,
        &occupied_context,
        [Some(reuse_from), Some(reuse_to), None, None],
        Some((0, reused.id)),
        Some(&filtered),
    ));
    let original_parcel_count = base.state.parcels.len();
    let original_frontages = base.state.parcel_frontages.clone();
    let new_corner_c = base.add_node(corners[2].x, corners[2].z, NodeKind::Junction);
    let new_corner_d = base.add_node(corners[3].x, corners[3].z, NodeKind::Junction);
    let selected_plan = NeighborhoodPlan {
        anchor_node: plan.anchor_node,
        segments: vec![
            (reuse_from, reuse_to),
            (reuse_to, new_corner_c),
            (new_corner_c, new_corner_d),
            (new_corner_d, reuse_from),
        ],
        next_segment: 0,
        coordinated_lots: true,
        selected_lots: Some(filtered.clone()),
        spine: None,
    };
    base.state.neighborhood_plan = Some(selected_plan.clone());
    base.state.neighborhood_road_ids.insert(reused.id);
    let selected = selected_plan.selected_lots.as_ref().unwrap();
    assert!(selected.len() >= 4 && selected.len() < 10);
    assert_eq!(base.state.parcels.len(), original_parcel_count);
    assert_eq!(
        serde_json::to_value(&base.state.parcel_frontages).unwrap(),
        serde_json::to_value(&original_frontages).unwrap()
    );
    assert_eq!(base.coordinated_plan_lot_count(), Some(selected.len()));
    let mut partial_capacity = Engine::restore_json(&base.save_json()).unwrap();
    let reserved_partial_lots = partial_capacity.coordinated_plan_lot_count().unwrap();
    while partial_capacity.state.parcels.len() + reserved_partial_lots < MAX_PARCELS {
        assert_ne!(
            partial_capacity.add_parcel(0.0, 0.0, 1.0, 1.0, plan.anchor_node, 0.5, false),
            0
        );
    }
    assert_eq!(
        partial_capacity.state.parcels.len() + reserved_partial_lots,
        MAX_PARCELS
    );
    assert_eq!(
        partial_capacity.add_parcel(0.0, 0.0, 1.0, 1.0, plan.anchor_node, 0.5, false),
        0,
        "the exact selected-lot count stays reserved until the block opens"
    );
    let partial_save = base.save_json();
    assert_eq!(
        Engine::restore_json(&partial_save).unwrap().save_json(),
        partial_save
    );
    let mut duplicate_layout: serde_json::Value = serde_json::from_str(&partial_save).unwrap();
    let layout = duplicate_layout["neighborhood_plan"]["selected_lots"]
        .as_array_mut()
        .unwrap();
    layout[1] = layout[0].clone();
    assert!(Engine::restore_json(&duplicate_layout.to_string()).is_err());
    let mut noncanonical_layout: serde_json::Value = serde_json::from_str(&partial_save).unwrap();
    noncanonical_layout["neighborhood_plan"]["selected_lots"][0]["center"]["x"] =
        serde_json::json!(selected[0].center.x + 0.5);
    assert!(Engine::restore_json(&noncanonical_layout.to_string()).is_err());
    let mut partial_replay = Engine::restore_json(&partial_save).unwrap();
    let partial_lot_count = selected.len();
    for edge in 1..4 {
        for world in [&mut base, &mut partial_replay] {
            assert!(world.plan_next_neighborhood_segment());
            let (from, to) = selected_plan.segments[edge];
            let project = world
                .state
                .projects
                .iter_mut()
                .find(|project| {
                    project.kind == ProjectKind::Road
                        && project.progress < 1.0
                        && project.from == Some(from)
                        && project.to == Some(to)
                })
                .expect("each selected shared-edge perimeter extension is scheduled");
            let id = project.id;
            project.progress = 1.0;
            world.complete_project(id);
        }
        assert_eq!(base.save_json(), partial_replay.save_json());
        let admitted_lots = base
            .state
            .parcels
            .iter()
            .filter(|parcel| parcel.rotation.is_some())
            .count();
        if edge < 3 {
            assert_eq!(
                admitted_lots, 0,
                "selected lots wait for the full perimeter"
            );
        } else {
            assert_eq!(admitted_lots, partial_lot_count);
            assert!(base.state.parcels.len() >= original_parcel_count + partial_lot_count);
            for lot in selected {
                assert_eq!(
                    base.state
                        .parcels
                        .iter()
                        .filter(|parcel| {
                            (parcel.x - lot.center.x).abs() < 1.0e-4
                                && (parcel.z - lot.center.z).abs() < 1.0e-4
                                && (parcel.width - lot.width).abs() < 1.0e-4
                                && (parcel.depth - lot.depth).abs() < 1.0e-4
                        })
                        .count(),
                    1,
                    "each selected parcel is inserted exactly once"
                );
            }
        }
    }
    assert!(base.state.neighborhood_plan.is_none());
    for frontage in &original_frontages {
        assert!(
            base.state.parcel_frontages.iter().any(|current| {
                current.parcel_id == frontage.parcel_id
                    && current.road_id == frontage.road_id
                    && (current.along - frontage.along).abs() < 1.0e-5
                    && current.side == frontage.side
            }),
            "pre-existing frontage metadata remains intact"
        );
    }
    for lot in selected {
        let parcel_id = base
            .state
            .parcels
            .iter()
            .find(|parcel| {
                (parcel.x - lot.center.x).abs() < 1.0e-4
                    && (parcel.z - lot.center.z).abs() < 1.0e-4
                    && (parcel.width - lot.width).abs() < 1.0e-4
                    && (parcel.depth - lot.depth).abs() < 1.0e-4
            })
            .unwrap()
            .id;
        assert!(
            base.state
                .parcel_frontages
                .iter()
                .any(|frontage| frontage.parcel_id == parcel_id),
            "every selected lot has frontage metadata"
        );
    }
    assert!(base.state.parcel_frontages.len() >= original_frontages.len() + partial_lot_count);
    base.validate().unwrap();
    let mut replay = Engine::restore_json(&engine.save_json()).unwrap();
    assert_eq!(engine.save_json(), replay.save_json());
    assert!(engine.plan_next_neighborhood_segment());
    assert!(replay.plan_next_neighborhood_segment());
    assert_eq!(engine.save_json(), replay.save_json());
    assert_eq!(
        engine
            .state
            .neighborhood_plan
            .as_ref()
            .unwrap()
            .next_segment,
        2
    );
    assert!(engine
        .state
        .projects
        .iter()
        .all(|project| { project.from != Some(reuse_from) || project.to != Some(reuse_to) }));
    assert!(engine.state.neighborhood_road_ids.contains(&reused.id));
    assert_eq!(
        engine
            .state
            .parcels
            .iter()
            .filter(|parcel| parcel.rotation.is_some())
            .count(),
        0
    );

    for edge in 1..4 {
        for world in [&mut engine, &mut replay] {
            if edge > 1 {
                assert!(world.plan_next_neighborhood_segment());
            }
            let (from, to) = plan.segments[edge];
            let project = world
                .state
                .projects
                .iter_mut()
                .find(|project| {
                    project.kind == ProjectKind::Road
                        && project.progress < 1.0
                        && project.from == Some(from)
                        && project.to == Some(to)
                })
                .expect("each remaining edge uses normal road construction");
            let id = project.id;
            project.progress = 1.0;
            world.complete_project(id);
        }
        assert_eq!(engine.save_json(), replay.save_json());
        let lots = engine
            .state
            .parcels
            .iter()
            .filter(|parcel| parcel.rotation.is_some())
            .count();
        if edge < 3 {
            assert_eq!(lots, 0, "interior lots wait for the final perimeter edge");
        } else {
            assert_eq!(lots, 10);
        }
    }
    assert!(engine.state.neighborhood_plan.is_none());
    assert_eq!(engine.state.neighborhood_road_ids.len(), 4);
    engine.validate().unwrap();
    let canonical = engine.save_json();
    assert_eq!(
        Engine::restore_json(&canonical).unwrap().save_json(),
        canonical
    );
}

#[test]
fn neighborhood_plan_is_bounded_and_legacy_saves_restore_without_one() {
    let mut engine = engine_with_developed_plan_site_without_reusable_edge();
    assert!(engine.try_create_neighborhood_plan());
    assert!(engine.state.roads.len() + engine.state.projects.len() <= MAX_SEGMENTS);
    assert!(engine.state.parcels.len() <= MAX_PARCELS);
    assert!(engine.state.buildings.len() <= MAX_BUILDINGS);
    let mut legacy: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    legacy.as_object_mut().unwrap().remove("neighborhood_plan");
    legacy
        .as_object_mut()
        .unwrap()
        .remove("neighborhood_anchor_cursor");
    let restored = Engine::restore_json(&legacy.to_string()).unwrap();
    assert!(restored.state.neighborhood_plan.is_none());
    assert_eq!(restored.state.neighborhood_anchor_cursor, 0);

    let mut malformed: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    malformed["neighborhood_plan"] = serde_json::json!({
        "anchor_node": engine.state.neighborhood_plan.as_ref().unwrap().anchor_node,
        "segments": [],
        "next_segment": 0
    });
    assert!(Engine::restore_json(&malformed.to_string()).is_err());
}

#[test]
fn neighborhood_candidate_cap_resumes_at_later_anchor_without_allocating_losers() {
    let mut engine = engine_with_developed_plan_site_without_reusable_edge();
    let first_anchor = engine.state.nodes[2].id;
    let original_node_count = engine.state.nodes.len();

    // Put the first anchor at the start of the scan and give it enough
    // connected spoke edges to consume the shared-edge candidate budget.
    // Its degree makes every block invalid, so this anchor cannot win.
    let first_point = engine.node(first_anchor).unwrap().clone();
    for spoke in 0..16 {
        let angle = -0.9 + 1.8 * spoke as f32 / 15.0;
        let endpoint = engine.add_node(
            first_point.x + angle.cos() * 75.0,
            first_point.z + angle.sin() * 75.0,
            NodeKind::Junction,
        );
        engine.add_road(first_anchor, endpoint, RoadKind::Local, Stage::Open, 1.0);
    }

    // Add a separate later catchment in the same connected road system.
    // Its degree and parcels admit an ordinary full-block candidate.
    let later_anchor = engine.add_node(-250.0, -250.0, NodeKind::Junction);
    let far_south = engine.add_node(360.0, -350.0, NodeKind::Junction);
    let west_south = engine.add_node(-160.0, -350.0, NodeKind::Junction);
    let later_feeder = engine.add_node(-190.0, -310.0, NodeKind::Junction);
    let gateway = engine.state.nodes[0].id;
    engine.add_road(gateway, far_south, RoadKind::Local, Stage::Open, 1.0);
    engine.add_road(far_south, west_south, RoadKind::Local, Stage::Open, 1.0);
    engine.add_road(west_south, later_feeder, RoadKind::Local, Stage::Open, 1.0);
    engine.add_road(
        later_feeder,
        later_anchor,
        RoadKind::Local,
        Stage::Open,
        1.0,
    );
    engine.recompute_connectivity();
    for row in 0..3 {
        for column in 0..4 {
            let parcel = engine.add_parcel(
                -185.0 + column as f32 * 8.0,
                -242.0 + row as f32 * 8.0,
                6.0,
                6.0,
                later_anchor,
                0.8,
                false,
            );
            let building = engine.add_building(
                parcel,
                BuildingKind::Home,
                2,
                Stage::Open,
                1.0,
                8,
                "Later occupied machiya",
                false,
                Vec::new(),
            );
            engine
                .state
                .buildings
                .iter_mut()
                .find(|candidate| candidate.id == building)
                .unwrap()
                .occupancy = 4;
        }
    }

    // Start on the crowded first anchor. It reaches the 32 shared
    // candidate cap, but all candidates fail its degree constraint. The
    // cursor must point past this anchor, and losing probes allocate no
    // corner nodes or plan state.
    engine.state.neighborhood_anchor_cursor = 2;
    let before_first_scan_nodes = engine.state.nodes.len();
    assert!(!engine.try_create_neighborhood_plan());
    assert_eq!(
        engine.state.nodes.len(),
        before_first_scan_nodes,
        "rejected candidates must not allocate corner nodes"
    );
    assert!(engine.state.neighborhood_plan.is_none());
    assert_eq!(engine.state.neighborhood_anchor_cursor, 3);

    // The following planner evaluation starts at that later cursor and
    // finds the independently eligible catchment rather than rescanning
    // or starving it behind the capped anchor.
    assert!(engine.try_create_neighborhood_plan());
    let plan = engine.state.neighborhood_plan.as_ref().unwrap();
    assert_eq!(plan.anchor_node, later_anchor);
    assert!(engine.state.nodes.len() >= original_node_count + 16 + 3);
}

#[test]
fn coordinated_block_reserves_all_parcel_slots_until_retirement() {
    let mut engine = engine_with_developed_plan_site();
    assert!(engine.try_create_neighborhood_plan());
    let reserved = engine.coordinated_plan_lot_count().unwrap();
    assert_eq!(reserved, 10);

    while engine.state.parcels.len() + reserved < MAX_PARCELS {
        let parcel = engine.add_parcel(0.0, 0.0, 1.0, 1.0, 2, 0.5, false);
        assert_ne!(
            parcel, 0,
            "exterior parcel admission should use unreserved capacity"
        );
    }
    assert_eq!(engine.state.parcels.len(), MAX_PARCELS - reserved);
    assert_eq!(engine.add_parcel(1.0, 0.0, 1.0, 1.0, 2, 0.5, false), 0);
    assert_eq!(engine.state.parcels.len() + reserved, MAX_PARCELS);

    // A new coordinated block must fail atomically when the world has
    // fewer parcel slots than its deterministic subdivision requires.
    let mut near_cap = engine_with_developed_plan_site();
    while near_cap.state.parcels.len() < MAX_PARCELS - reserved + 1 {
        assert_ne!(near_cap.add_parcel(0.0, 0.0, 1.0, 1.0, 2, 0.5, false), 0);
    }
    let before_nodes = near_cap.state.nodes.len();
    let before_parcels = near_cap.state.parcels.len();
    assert!(!near_cap.try_create_neighborhood_plan());
    assert_eq!(near_cap.state.nodes.len(), before_nodes);
    assert_eq!(near_cap.state.parcels.len(), before_parcels);

    // Invalid restored state cannot claim more slots than the hard cap.
    let mut invalid = engine_with_developed_plan_site();
    assert!(invalid.try_create_neighborhood_plan());
    while invalid.state.parcels.len() < MAX_PARCELS - reserved + 1 {
        invalid.state.parcels.push(Parcel {
            id: 1_000_000 + invalid.state.parcels.len() as u32,
            x: 0.0,
            z: 0.0,
            width: 1.0,
            depth: 1.0,
            rotation: None,
            access_node: 2,
            suitability: 0.5,
            protected: false,
            reservation: None,
            open_space: None,
        });
    }
    assert!(invalid
        .validate()
        .unwrap_err()
        .contains("coordinated block parcel reservation"));
}

#[test]
fn coordinated_block_road_clearance_protects_interior_and_allows_perimeter() {
    let mut engine = engine_with_developed_plan_site();
    assert!(engine.try_create_neighborhood_plan());
    let plan = engine.state.neighborhood_plan.as_ref().unwrap();
    let anchor = engine.node(plan.anchor_node).unwrap().clone();
    let interior = engine.coordinated_block_interior(&plan.segments).unwrap();
    let spur = Node {
        id: u32::MAX,
        x: interior.x,
        z: interior.z,
        kind: NodeKind::Candidate,
    };
    assert!(!engine.neighborhood_corridor_clear_for_road(&anchor, &spur, anchor.id));

    let (_, next_id) = plan.segments[0];
    let perimeter_end = engine.node(next_id).unwrap().clone();
    assert!(engine.neighborhood_corridor_clear_for_road(&anchor, &perimeter_end, anchor.id));
}

#[test]
fn oriented_admission_respects_legacy_saved_and_rendered_envelopes() {
    let mut engine = Engine::new_legacy(87231);
    let mut building = engine.state.buildings[0].clone();
    engine.state.parcels.clear();
    engine.state.buildings.clear();
    engine.state.roads.clear();
    engine.state.projects.clear();
    engine.state.parcel_frontages.clear();
    engine.state.neighborhood_plan = None;
    let probes =
        [(262.0, 250.0), (250.0, 262.0), (280.0, 280.0)].map(|(x, z)| geometry::OrientedRect {
            x,
            z,
            width: 4.0,
            depth: 4.0,
            rotation: 0.0,
        });
    for probe in &probes {
        assert!(engine.oriented_parcel_geometry_clear(probe));
    }
    let parcel = engine.add_parcel(250.0, 250.0, 30.0, 8.0, 2, 0.7, false);
    building.parcel_id = parcel;
    building.rotation = std::f32::consts::FRAC_PI_2;
    engine.state.buildings.push(building);
    let stored = parcel_envelope(&engine.state.parcels[0]);
    let rendered = geometry::OrientedRect {
        rotation: std::f32::consts::FRAC_PI_2,
        ..stored
    };
    assert!(geometry::rects_overlap(&probes[0], &stored, 0.0));
    assert!(!geometry::rects_overlap(&probes[0], &rendered, 2.0));
    assert!(!geometry::rects_overlap(&probes[1], &stored, 2.0));
    assert!(geometry::rects_overlap(&probes[1], &rendered, 0.0));
    let context = engine.collision_context();
    for (probe, expected) in probes.iter().zip([false, false, true]) {
        assert_eq!(engine.oriented_parcel_geometry_clear(probe), expected);
        assert_eq!(
            engine.oriented_parcel_geometry_clear_with_context(probe, &context),
            expected
        );
    }
}

#[test]
fn frontier_proposal_scan_is_read_only_and_plan_road_replays() {
    let mut engine = Engine::new_legacy(0);
    // Use a developing city: the old 72,000-tick checkpoint can have no
    // proposal in its bounded scan. Keep the positive scan/admission case
    // separate from the exhausted-city case below. Independent live access
    // search interaction/replay is covered in access_live_engine.
    engine.state.next_face_access_tick = u64::MAX;
    engine.set_development_rate(60);
    for _ in 0..6 {
        engine.step(2_000);
    }
    let open_buildings: Vec<Building> = engine
        .state
        .buildings
        .iter()
        .filter(|building| site_active(building))
        .cloned()
        .collect();
    let bridgehead_headings = engine.eligible_bridgehead_headings(&open_buildings);
    let collision_context = engine.collision_context();
    let before_chord = engine.save_json();
    assert!(!engine.try_plan_local_chord(&open_buildings, &collision_context));
    let before = engine.save_json();
    let before_rng = engine.state.rng_growth;
    let before_cursor = engine.state.road_origin_cursor;
    let scan =
        engine.propose_frontier_road(&open_buildings, &bridgehead_headings, &collision_context);
    let repeated =
        engine.propose_frontier_road(&open_buildings, &bridgehead_headings, &collision_context);
    assert_eq!(
        engine.save_json(),
        before,
        "frontier scan mutated serialized state"
    );
    assert_eq!(engine.state.rng_growth, before_rng);
    assert_eq!(engine.state.road_origin_cursor, before_cursor);
    let proposal = scan
        .proposal
        .expect("fixture should contain a viable frontier proposal");
    let repeated_proposal = repeated.proposal.expect("scan result is repeatable");
    assert_eq!(proposal.start_id, repeated_proposal.start_id);
    assert_eq!(proposal.target_id, repeated_proposal.target_id);
    assert_eq!(proposal.end_x, repeated_proposal.end_x);
    assert_eq!(proposal.end_z, repeated_proposal.end_z);
    assert_eq!(proposal.bridgehead, repeated_proposal.bridgehead);
    assert_eq!(scan.next_cursor, repeated.next_cursor);
    assert_eq!(scan.next_rng, repeated.next_rng);
    assert_eq!(scan.allow_access_fallback, repeated.allow_access_fallback);

    let mut exhausted = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/mature-world.ranked-blocks.json"
    ))
    .expect("mature public city fixture restores");
    exhausted.state.demand = 1.2;
    let exhausted_buildings: Vec<Building> = exhausted
        .state
        .buildings
        .iter()
        .filter(|building| site_active(building))
        .cloned()
        .collect();
    let exhausted_bridgeheads = exhausted.eligible_bridgehead_headings(&exhausted_buildings);
    let exhausted_context = exhausted.collision_context();
    assert!(!exhausted.try_plan_local_chord(&exhausted_buildings, &exhausted_context));
    let exhausted_before = exhausted.save_json();
    let exhausted_scan = exhausted.propose_frontier_road(
        &exhausted_buildings,
        &exhausted_bridgeheads,
        &exhausted_context,
    );
    assert!(exhausted_scan.proposal.is_none());
    assert!(exhausted_scan.allow_access_fallback);
    assert_eq!(exhausted.save_json(), exhausted_before);

    let mut left = Engine::restore_json(&before_chord).unwrap();
    let mut right = Engine::restore_json(&before_chord).unwrap();
    let project_ids_before: BTreeSet<_> = left.state.projects.iter().map(|p| p.id).collect();
    left.plan_road();
    right.plan_road();
    assert!(
        left.state.projects.iter().any(|p| {
            !project_ids_before.contains(&p.id)
                && p.kind == ProjectKind::Road
                && p.from == Some(proposal.start_id)
        }),
        "replay must admit the enumerated frontier, not be a no-op"
    );
    assert_eq!(
        left.save_json(),
        right.save_json(),
        "planner replay diverged"
    );
    let admitted = left.save_json();
    right = Engine::restore_json(&admitted).expect("admitted frontier restores");
    left.step(800);
    right.step(800);
    assert_eq!(
        left.save_json(),
        right.save_json(),
        "frontier construction replay diverged"
    );
}
