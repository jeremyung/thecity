use crate::*;

/// Exact engine-side gate audit for the raster diagnostic's leading candidate.
/// This mutates only an in-memory restored copy and is ignored in normal runs.
#[test]
#[ignore = "exact read-only candidate gate audit for the tick-242000 checkpoint"]
fn audit_largest_face_direct_access_candidate() {
    use serde_json::json;

    const SAVE: &str =
        include_str!("../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
    let source = Engine::restore_json(SAVE).expect("matching mature checkpoint restores");
    assert_eq!(source.state.tick, 242_000);
    assert_eq!(source.state.seed, 87_231);

    let mut split_probe = Engine::restore_json(SAVE).unwrap();
    let split = split_probe.try_split_open_road(3317, 0.72);
    let split_ok = split.is_ok();
    let split = split.ok();
    let mut diagnostic = json!({
        "checkpoint_tick": source.state.tick,
        "host_road_id": 3317,
        "host_t": 0.72,
        "split_preflight_passed": split_ok,
    });
    let Some(split) = split else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../validation/face-access-exact-gates.json");
        std::fs::write(path, serde_json::to_vec_pretty(&diagnostic).unwrap()).unwrap();
        println!("{}", diagnostic);
        return;
    };

    let split_node = split_probe.node(split.split_node_id).unwrap().clone();
    let end = WorldPoint { x: 207.0, z: 1.0 };
    let branch_length = distance(
        &split_node,
        &Node {
            id: u32::MAX,
            x: end.x,
            z: end.z,
            kind: NodeKind::Junction,
        },
    );
    let endpoint_spacing = split_probe
        .state
        .network
        .nodes
        .iter()
        .map(|node| (node.x - end.x).hypot(node.z - end.z))
        .fold(f32::INFINITY, f32::min);
    let heading = (end.z - split_node.z).atan2(end.x - split_node.x);
    let context = split_probe.collision_context();
    let gates = json!({
        "split_x": split_node.x,
        "split_z": split_node.z,
        "branch_end_x": end.x,
        "branch_end_z": end.z,
        "branch_length_m": branch_length,
        "branch_endpoint_bounds": end.x >= MIN + 24.0 && end.x <= MAX - 24.0 && end.z >= MIN + 24.0 && end.z <= MAX - 24.0,
        "endpoint_node_spacing_m": endpoint_spacing,
        "endpoint_spacing_passed": endpoint_spacing >= 18.0,
        "river_clearance_passed": split_probe.ordinary_road_clear_of_river(split_node.x, split_node.z, end.x, end.z, 4.5),
        "protected_land_passed": !corridor_crosses_protected(split_node.x, split_node.z, end.x, end.z),
        "existing_parcel_corridor_passed": split_probe.road_corridor_clear_of_parcels_with_context(split_node.x, split_node.z, end.x, end.z, &context),
        "junction_heading_passed": split_probe.ordinary_heading_clear(split_node.id, heading),
        "endpoint_degree": split_probe.ordinary_road_degree(split_probe.state.network.roads.iter().find(|r|r.id==3317).unwrap().to),
    });

    // Check the straight chord against every existing road. The two split host
    // halves meet this branch at its start; all other edges retain the ordinary
    // eight-metre centerline separation rule.
    let road_separations_passed = split_probe.state.network.roads.iter().all(|road| {
        if road.from == split.split_node_id || road.to == split.split_node_id {
            return true;
        }
        let (Some(a), Some(b)) = (split_probe.node(road.from), split_probe.node(road.to)) else {
            return false;
        };
        geometry::segments_distance_coords(
            split_node.x,
            split_node.z,
            end.x,
            end.z,
            a.x,
            a.z,
            b.x,
            b.z,
        ) >= 8.0
    });
    let bend = WorldPoint { x: 229.0, z: 1.0 };
    let dogleg_legs = [
        (split_node.x, split_node.z, bend.x, bend.z),
        (bend.x, bend.z, 207.0, 1.0),
    ];
    let dogleg_clearance = dogleg_legs.iter().map(|(ax,az,bx,bz)| json!({
        "length_m":(bx-ax).hypot(bz-az),
        "river":split_probe.ordinary_road_clear_of_river(*ax,*az,*bx,*bz,4.5),
        "protected":!corridor_crosses_protected(*ax,*az,*bx,*bz),
        "parcel_corridor":split_probe.road_corridor_clear_of_parcels_with_context(*ax,*az,*bx,*bz,&context),
        "nearest_existing_node_m":split_probe.state.network.nodes.iter().map(|n|(n.x-(*bx)).hypot(n.z-(*bz))).fold(f32::INFINITY,f32::min),
    })).collect::<Vec<_>>();
    let mut dogleg_context = context.clone();
    for (ax, az, bx, bz) in dogleg_legs {
        dogleg_context
            .open_roads
            .push(CollisionSegment { ax, az, bx, bz });
    }
    let dogleg_lots = [
        geometry::OrientedRect {
            x: 224.0,
            z: 13.0,
            width: 8.0,
            depth: 8.0,
            rotation: std::f32::consts::PI,
        },
        geometry::OrientedRect {
            x: 214.0,
            z: 13.0,
            width: 8.0,
            depth: 8.0,
            rotation: std::f32::consts::PI,
        },
    ];
    let dogleg_lot_checks=dogleg_lots.iter().map(|rect| json!({
        "x":rect.x,"z":rect.z,"width":rect.width,"depth":rect.depth,
        "engine_parcel_and_frontage_clear":split_probe.progressive_frontage_geometry_clear(rect,&dogleg_context),
    })).collect::<Vec<_>>();

    // Derive ordinary frontage lots using the same deterministic subdivision
    // and door-facing convention as the access planner, then call its exact
    // parcel, road and terrain predicates.
    let intervals = frontage::subdivide_frontage(
        branch_length,
        &[],
        FRONTAGE_ENDPOINT_CLEARANCE,
        8.0,
        14.0,
        2.2,
        MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
    );
    let dx = end.x - split_node.x;
    let dz = end.z - split_node.z;
    let nx = -dz / branch_length;
    let nz = dx / branch_length;
    let mut proposed = Vec::new();
    for side in [-1_i8, 1] {
        for interval in &intervals {
            let along = (interval.start + interval.end) * 0.5;
            let x = split_node.x + dx * (along / branch_length) + nx * side as f32 * 15.0;
            let z = split_node.z + dz * (along / branch_length) + nz * side as f32 * 15.0;
            let rect = geometry::OrientedRect {
                x,
                z,
                width: interval.end - interval.start,
                depth: 12.0,
                rotation: (side as f32 * dz).atan2(-side as f32 * dx),
            };
            let mut proposed_context = context.clone();
            proposed_context.open_roads.push(CollisionSegment {
                ax: split_node.x,
                az: split_node.z,
                bx: end.x,
                bz: end.z,
            });
            let oriented_ok =
                split_probe.oriented_parcel_geometry_clear_with_context(&rect, &proposed_context);
            let center_ok = split_probe.progressive_frontage_center_clear(&rect, &proposed_context);
            let clear = oriented_ok && center_ok;
            let nearest_parcel = context
                .parcel_footprints
                .iter()
                .enumerate()
                .map(|(index, footprint)| {
                    (
                        index,
                        geometry::rect_segment_distance(
                            footprint,
                            split_node.x,
                            split_node.z,
                            end.x,
                            end.z,
                        ),
                    )
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            proposed.push(json!({
                "x": rect.x,
                "z": rect.z,
                "width": rect.width,
                "depth": rect.depth,
                "rotation": rect.rotation,
                "minimum_width_passed": rect.width >= 8.0,
                "terrain_and_full_parcel_admission_passed": clear,
                "oriented_overlap_and_terrain_passed": oriented_ok,
                "street_center_clearance_passed": center_ok,
                "nearest_existing_parcel_corridor_distance_m": nearest_parcel.map(|(_, distance)| distance),
                "frontage_along": along / branch_length,
                "frontage_access_node": frontage_access_node(
                    &split_probe.node(split.split_node_id).unwrap().clone(),
                    &Node { id: end_node_id_for_test(), x: end.x, z: end.z, kind: NodeKind::Junction },
                    along / branch_length,
                ),
            }));
        }
    }
    let viable_lots = proposed
        .iter()
        .filter(|lot| lot["terrain_and_full_parcel_admission_passed"] == true)
        .count();
    diagnostic["direct_chord"] = json!({
        "gates": gates,
        "all_other_road_separations_passed": road_separations_passed,
        "lot_candidates": proposed,
        "viable_lot_count": viable_lots,
        "minimum_two_lots_passed": viable_lots >= 2,
        "would_be_admissible_geometry": split_ok && road_separations_passed && viable_lots >= 2
            && gates["branch_endpoint_bounds"] == true
            && gates["endpoint_spacing_passed"] == true
            && gates["river_clearance_passed"] == true
            && gates["protected_land_passed"] == true
            && gates["existing_parcel_corridor_passed"] == true
            && gates["junction_heading_passed"] == true,
        "two_segment_raster_route_candidate": {
            "legs":dogleg_clearance,
            "lot_checks":dogleg_lot_checks,
            "current_single_branch_lifecycle_supports_two_legs":false
        },
    });

    let current_host_scan = source.propose_interior_access_for_host(3317);
    diagnostic["existing_exact_planner_host_scan"] = json!({
        "proposal_count": current_host_scan.proposals.len(),
        "next_cursor": current_host_scan.next_cursor,
        "proposals": current_host_scan.proposals.iter().map(|proposal| json!({
            "host_t": proposal.host_t,
            "end_x": proposal.end_x,
            "end_z": proposal.end_z,
            "lot_count": proposal.selected_lots.len(),
            "lot_area_m2": proposal.score.site_area as f64 / 10_000.0,
        })).collect::<Vec<_>>(),
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/face-access-exact-gates.json");
    std::fs::write(path, serde_json::to_vec_pretty(&diagnostic).unwrap()).unwrap();
    println!("{}", diagnostic);
}

fn end_node_id_for_test() -> u32 {
    u32::MAX
}

fn generalized_access_candidate_engine() -> (Engine, usize) {
    const SAVE: &str =
        include_str!("../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
    let mut engine = Engine::restore_json(SAVE).expect("mature checkpoint restores");
    let host_t = 0.72;
    let end = WorldPoint { x: 181.0, z: 15.0 };
    let host_a = engine
        .node(
            engine
                .state
                .network
                .roads
                .iter()
                .find(|r| r.id == 3317)
                .unwrap()
                .from,
        )
        .unwrap()
        .clone();
    let host_b = engine
        .node(
            engine
                .state
                .network
                .roads
                .iter()
                .find(|r| r.id == 3317)
                .unwrap()
                .to,
        )
        .unwrap()
        .clone();
    let branch_length = (end.x - (host_a.x + (host_b.x - host_a.x) * host_t))
        .hypot(end.z - (host_a.z + (host_b.z - host_a.z) * host_t));
    let intervals = frontage::subdivide_frontage(
        branch_length,
        &[],
        FRONTAGE_ENDPOINT_CLEARANCE,
        8.0,
        14.0,
        2.2,
        MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
    );

    // The saved candidate was discovered by a bounded endpoint scan. Rebuild
    // its lots with the production branch-tangent/perpendicular convention.
    let start = WorldPoint {
        x: host_a.x + (host_b.x - host_a.x) * host_t,
        z: host_a.z + (host_b.z - host_a.z) * host_t,
    };
    let dx = end.x - start.x;
    let dz = end.z - start.z;
    let length = dx.hypot(dz);
    let nx = -dz / length;
    let nz = dx / length;
    let mut proposal_context = engine.collision_context();
    proposal_context.open_roads.push(CollisionSegment {
        ax: start.x,
        az: start.z,
        bx: end.x,
        bz: end.z,
    });
    let mut lots = Vec::new();
    for side in [-1_i8, 1] {
        for interval in &intervals {
            let along_meters = (interval.start + interval.end) * 0.5;
            let lot = interior_access::InteriorAccessLot {
                x: start.x + dx * (along_meters / length) + nx * side as f32 * 15.0,
                z: start.z + dz * (along_meters / length) + nz * side as f32 * 15.0,
                width: interval.end - interval.start,
                depth: 12.0,
                rotation: (side as f32 * dz).atan2(-side as f32 * dx),
                along: along_meters / length,
                side,
            };
            let rect = geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if engine.progressive_frontage_geometry_clear(&rect, &proposal_context) {
                lots.push(lot);
            }
        }
    }
    assert!(
        branch_length >= 24.0 && branch_length <= 60.0,
        "candidate branch length {branch_length}"
    );
    assert!(
        lots.len() >= 2,
        "candidate must reserve at least two exact-clear lots"
    );
    let parcels_before = engine.state.land.parcels.len();
    let lot_count = lots.len();
    assert!(engine.admit_interior_access(3317, host_t, end, lots));
    engine
        .validate()
        .expect("generalized geometry validation accepts this saved plan");
    assert_eq!(engine.state.land.parcels.len(), parcels_before);
    (engine, lot_count)
}

#[test]
fn explicit_angled_access_candidate_uses_branch_frontage_and_restores() {
    const SAVE: &str =
        include_str!("../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
    let source = Engine::restore_json(SAVE).expect("mature checkpoint restores");
    let before = source.save_json();
    let candidate = interior_access::AccessGeometryCandidate {
        host_id: 3317,
        host_t: 0.72,
        end_x: 181.0,
        end_z: 15.0,
        tie_index: 0,
    };

    let scan = source.propose_interior_access_from_geometry_candidates(0, &[candidate]);
    assert_eq!(source.save_json(), before, "proposal scan is read-only");
    assert_eq!(
        scan.proposals.len(),
        1,
        "the known angled branch remains feasible"
    );
    let proposal = scan.proposals.into_iter().next().unwrap();
    assert!(proposal.selected_lots.len() >= 2);

    let start = {
        let host = source
            .state
            .network
            .roads
            .iter()
            .find(|road| road.id == 3317)
            .unwrap();
        let a = source.node(host.from).unwrap();
        let b = source.node(host.to).unwrap();
        WorldPoint {
            x: a.x + (b.x - a.x) * proposal.host_t,
            z: a.z + (b.z - a.z) * proposal.host_t,
        }
    };
    let end = WorldPoint {
        x: proposal.end_x,
        z: proposal.end_z,
    };
    let dx = end.x - start.x;
    let dz = end.z - start.z;
    let length = dx.hypot(dz);
    let tangent_x = dx / length;
    let tangent_z = dz / length;
    for lot in &proposal.selected_lots {
        let along = lot.along * length;
        let expected_x = start.x + tangent_x * along - tangent_z * lot.side as f32 * 15.0;
        let expected_z = start.z + tangent_z * along + tangent_x * lot.side as f32 * 15.0;
        assert!((lot.x - expected_x).abs() < 1e-3);
        assert!((lot.z - expected_z).abs() < 1e-3);
    }

    let mut admitted = Engine::restore_json(SAVE).unwrap();
    assert!(admitted.admit_interior_access(
        proposal.host_id,
        proposal.host_t,
        end,
        proposal.selected_lots,
    ));
    admitted
        .validate()
        .expect("admitted candidate passes full world validation");
    let saved = admitted.save_json();
    let restored = Engine::restore_json(&saved).expect("pending angled plan restores");
    assert_eq!(
        restored.save_json(),
        saved,
        "canonical save is stable after restore"
    );
}

/// Offline local sensitivity map around the independently validated branch.
/// Every point goes through the production read-only proposal evaluator; this
/// fixture coordinate is diagnostic-only and does not feed candidate generation.
#[test]
#[ignore = "offline local geometry sensitivity map around known feasible branch"]
fn map_known_branch_local_sensitivity() {
    use serde_json::json;

    const SAVE: &str =
        include_str!("../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
    const HOST_ID: u32 = 3317;
    const BASE_T: f32 = 0.72;
    const BASE_END_X: f32 = 181.0;
    const BASE_END_Z: f32 = 15.0;

    let engine = Engine::restore_json(SAVE).expect("mature checkpoint restores");
    let host = engine
        .state
        .network
        .roads
        .iter()
        .find(|road| road.id == HOST_ID)
        .unwrap();
    let a = engine.node(host.from).unwrap();
    let b = engine.node(host.to).unwrap();
    let host_length = (b.x - a.x).hypot(b.z - a.z);

    let split_offsets = (-4..=4).map(|step| step as f32 * 0.25).collect::<Vec<_>>();
    let endpoint_offsets = (-4..=4).map(|step| step as f32 * 0.5).collect::<Vec<_>>();
    let base_split_distance = BASE_T * host_length;
    let mut candidates = Vec::with_capacity(split_offsets.len() * endpoint_offsets.len().pow(2));
    let mut metadata = Vec::with_capacity(candidates.capacity());
    for split_offset_m in &split_offsets {
        for dx in &endpoint_offsets {
            for dz in &endpoint_offsets {
                let split_distance = base_split_distance + split_offset_m;
                candidates.push(interior_access::AccessGeometryCandidate {
                    host_id: HOST_ID,
                    host_t: split_distance / host_length,
                    end_x: BASE_END_X + dx,
                    end_z: BASE_END_Z + dz,
                    tie_index: 0,
                });
                metadata.push((*split_offset_m, *dx, *dz));
            }
        }
    }
    assert_eq!(candidates.len(), 729);

    let mut accepted = Vec::new();
    let mut batch_count = 0usize;
    for (batch_index, batch) in candidates
        .chunks(interior_access::MAX_PROPOSALS_PER_EVALUATION)
        .enumerate()
    {
        let scan = engine.propose_interior_access_from_geometry_candidates(0, batch);
        for proposal in scan.proposals {
            let local_index = batch_index * interior_access::MAX_PROPOSALS_PER_EVALUATION
                + batch
                    .iter()
                    .position(|candidate| {
                        candidate.host_t == proposal.host_t
                            && candidate.end_x == proposal.end_x
                            && candidate.end_z == proposal.end_z
                    })
                    .expect("accepted proposal corresponds to a candidate");
            let (split_offset_m, endpoint_dx, endpoint_dz) = metadata[local_index];
            accepted.push(json!({
                "split_offset_m": split_offset_m,
                "split_t": proposal.host_t,
                "endpoint_dx_m": endpoint_dx,
                "endpoint_dz_m": endpoint_dz,
                "endpoint_x": proposal.end_x,
                "endpoint_z": proposal.end_z,
                "usable_lots": proposal.selected_lots.len(),
                "selected_area_m2": proposal.score.site_area as f64 / 10_000.0,
            }));
        }
        batch_count += 1;
    }

    let split_min = accepted
        .iter()
        .map(|item| item["split_offset_m"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    let split_max = accepted
        .iter()
        .map(|item| item["split_offset_m"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max);
    let dx_min = accepted
        .iter()
        .map(|item| item["endpoint_dx_m"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    let dx_max = accepted
        .iter()
        .map(|item| item["endpoint_dx_m"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max);
    let dz_min = accepted
        .iter()
        .map(|item| item["endpoint_dz_m"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    let dz_max = accepted
        .iter()
        .map(|item| item["endpoint_dz_m"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max);
    let report = json!({
        "diagnostic": "local exact-evaluator sensitivity map; not production candidate generation",
        "source_save": "tests/fixtures/validation/early-reservation-wasm-87231-save.json",
        "source_save_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
        "tick": engine.state.tick,
        "seed": engine.state.seed,
        "host_road_id": HOST_ID,
        "base": {"split_t": BASE_T, "endpoint_x": BASE_END_X, "endpoint_z": BASE_END_Z, "host_length_m": host_length},
        "sweep": {
            "split_offsets_m": split_offsets,
            "endpoint_dx_m": endpoint_offsets,
            "endpoint_dz_m": endpoint_offsets,
            "candidate_count": candidates.len(),
            "evaluator_batch_size": interior_access::MAX_PROPOSALS_PER_EVALUATION,
            "evaluator_batches": batch_count,
        },
        "accepted_count": accepted.len(),
        "accepted_range": if accepted.is_empty() { serde_json::Value::Null } else { json!({
            "split_offset_m": [split_min, split_max],
            "endpoint_dx_m": [dx_min, dx_max],
            "endpoint_dz_m": [dz_min, dz_max],
        })},
        "accepted": accepted,
        "interpretation_limit": "This measures only a local box around one known branch; it does not establish globally discoverable or production-scale candidate coverage."
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/face-access-sensitivity.json");
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{}", report);
}

#[test]
fn geometry_valid_noncanonical_interior_access_plan_restores_and_completes() {
    let (mut engine, lot_count) = generalized_access_candidate_engine();
    let parcels_before = engine.state.land.parcels.len();
    let canonical = engine.save_json();
    let mut resumed = Engine::restore_json(&canonical).expect("candidate pending plan restores");
    assert_eq!(resumed.save_json(), canonical);
    let branch_id = engine
        .state
        .legacy
        .pending_interior_access
        .as_ref()
        .unwrap()
        .branch_road_id;
    let plan = engine
        .state
        .legacy
        .pending_interior_access
        .as_ref()
        .unwrap()
        .clone();

    let mut near_endpoint = Engine::restore_json(&canonical).unwrap();
    near_endpoint
        .state
        .legacy
        .pending_interior_access
        .as_mut()
        .unwrap()
        .host_t = 0.01;
    assert!(Engine::restore_json(&near_endpoint.save_json()).is_err());

    let mut acute = Engine::restore_json(&canonical).unwrap();
    let acute_plan = acute
        .state
        .legacy
        .pending_interior_access
        .as_ref()
        .unwrap()
        .clone();
    let retained = acute
        .state
        .network
        .roads
        .iter()
        .find(|r| r.id == acute_plan.host_road_id)
        .unwrap()
        .clone();
    let second = acute
        .state
        .network
        .roads
        .iter()
        .find(|r| r.id == acute_plan.host_second_half_road_id)
        .unwrap()
        .clone();
    let host_a = acute.node(retained.from).unwrap().clone();
    let host_b = acute.node(second.to).unwrap().clone();
    let split = acute.node(acute_plan.split_node_id).unwrap().clone();
    let ux = (host_b.x - host_a.x).hypot(host_b.z - host_a.z);
    let dx = (host_b.x - host_a.x) / ux;
    let dz = (host_b.z - host_a.z) / ux;
    let branch_length = 24.0;
    let endpoint = acute
        .state
        .network
        .nodes
        .iter_mut()
        .find(|n| n.id == acute_plan.branch_endpoint_id)
        .unwrap();
    endpoint.x = split.x + dx * branch_length;
    endpoint.z = split.z + dz * branch_length;
    acute
        .state
        .network
        .roads
        .iter_mut()
        .find(|r| r.id == acute_plan.branch_road_id)
        .unwrap()
        .length = branch_length;
    assert!(acute.validate().unwrap_err().contains("junction angle"));

    let mut nonfinite = Engine::restore_json(&canonical).unwrap();
    nonfinite
        .state
        .legacy
        .pending_interior_access
        .as_mut()
        .unwrap()
        .selected_lots[0]
        .rotation = f32::NAN;
    assert!(nonfinite.validate().is_err());

    for instance in [&mut engine, &mut resumed] {
        let project = instance
            .state
            .construction
            .projects
            .iter_mut()
            .find(|p| p.id == plan.project_id)
            .unwrap();
        project.duration = 1.0;
        instance.advance_projects();
        instance
            .validate()
            .expect("completed noncanonical plan remains valid");
    }
    assert_eq!(
        engine.save_json(),
        resumed.save_json(),
        "completion and replay agree"
    );
    assert!(engine.state.legacy.pending_interior_access.is_none());
    assert!(engine
        .state
        .network
        .roads
        .iter()
        .any(|r| r.id == branch_id && r.stage == Stage::Open && r.connected));
    assert_eq!(engine.state.land.parcels.len(), parcels_before + lot_count);
}

fn diagnostic_point_in_face(point: (f32, f32), polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let (ax, az) = polygon[previous];
        let (bx, bz) = polygon[current];
        let cross = (bx - ax) * (point.1 - az) - (bz - az) * (point.0 - ax);
        if cross.abs() <= 1.0e-4
            && point.0 >= ax.min(bx) - 1.0e-4
            && point.0 <= ax.max(bx) + 1.0e-4
            && point.1 >= az.min(bz) - 1.0e-4
            && point.1 <= az.max(bz) + 1.0e-4
        {
            return true;
        }
        if (az > point.1) != (bz > point.1) && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn diagnostic_rect_inside_face(rect: &geometry::OrientedRect, polygon: &[(f32, f32)]) -> bool {
    rect.corners()
        .into_iter()
        .all(|corner| diagnostic_point_in_face(corner, polygon))
}

#[test]
#[ignore = "offline exhaustive prepared-page diagnostic on a mature face"]
fn exhaustive_prepared_pages_measure_mature_face_access_candidates() {
    let diagnostic_started = std::time::Instant::now();
    const SAVE: &str =
        include_str!("../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
    let source = Engine::restore_json(SAVE).expect("current mature checkpoint restores");
    let source_before = source.save_json();
    let ordinary = source
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .collect::<Vec<_>>();
    let used_nodes = ordinary
        .iter()
        .flat_map(|road| [road.from, road.to])
        .collect::<BTreeSet<_>>();
    let nodes = source
        .state
        .network
        .nodes
        .iter()
        .filter(|node| used_nodes.contains(&node.id))
        .map(|node| road_faces::RoadFaceNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();
    let edges = ordinary
        .iter()
        .map(|road| road_faces::RoadFaceEdge {
            id: road.id,
            from: road.from,
            to: road.to,
        })
        .collect::<Vec<_>>();
    let faces = road_faces::trace_faces(&nodes, &edges, None)
        .expect("mature connected ordinary graph is planar");
    let context = source.collision_context();
    // Explicit golden capture remains independent of routine parity reruns.
    let export_targets = std::env::var_os("EXPORT_FACE_TARGET_REFERENCE").is_some();
    let reference_faces = serde_json::to_value(&faces).unwrap();
    let mut reference_face_samples = Vec::new();
    let mut ranked_faces = faces
        .into_iter()
        .map(|face| {
            let min_x = face.polygon.iter().map(|point| point.0).fold(f32::INFINITY, f32::min);
            let max_x = face.polygon.iter().map(|point| point.0).fold(f32::NEG_INFINITY, f32::max);
            let min_z = face.polygon.iter().map(|point| point.1).fold(f32::INFINITY, f32::min);
            let max_z = face.polygon.iter().map(|point| point.1).fold(f32::NEG_INFINITY, f32::max);
            let columns = (((max_x - min_x) / 8.0).ceil() as usize).clamp(1, 24);
            let rows = (((max_z - min_z) / 8.0).ceil() as usize).clamp(1, 24);
            let mut viable_centers = Vec::new();
            for row in 0..rows {
                for column in 0..columns {
                    let point = (
                        min_x + (column as f32 + 0.5) * (max_x - min_x) / columns as f32,
                        min_z + (row as f32 + 0.5) * (max_z - min_z) / rows as f32,
                    );
                    if !diagnostic_point_in_face(point, &face.polygon) {
                        continue;
                    }
                    for rotation in [0.0, std::f32::consts::FRAC_PI_2] {
                        let rect = geometry::OrientedRect {
                            x: point.0,
                            z: point.1,
                            width: 8.0,
                            depth: 8.0,
                            rotation,
                        };
                        if diagnostic_rect_inside_face(&rect, &face.polygon)
                            && source.progressive_frontage_geometry_clear(&rect, &context)
                        {
                            viable_centers.push(point);
                            break;
                        }
                    }
                }
            }
            {
                reference_face_samples.push(serde_json::json!({
                    "identity": face.identity,
                    "viable_count": viable_centers.len(),
                    "center_bits": viable_centers.iter().map(|&(x,z)| [x.to_bits(), z.to_bits()]).collect::<Vec<_>>()
                }));
            }
            (viable_centers.len(), face.area, face.identity.clone(), face, viable_centers)
        })
        .filter(|entry| entry.0 > 0)
        .collect::<Vec<_>>();
    ranked_faces.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.total_cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    let reference_ranking = ranked_faces.iter().map(|entry| {
        serde_json::json!({ "identity": entry.2, "viable_count": entry.0, "area_bits": entry.1.to_bits() })
    }).collect::<Vec<_>>();
    let (viable_sites, _, _, target_face, viable_centers) = ranked_faces
        .into_iter()
        .next()
        .expect("at least one face contains unparcelized space");
    let host_by_id = ordinary
        .iter()
        .map(|road| (road.id, *road))
        .collect::<std::collections::BTreeMap<_, _>>();
    let target_hosts = target_face
        .boundary_edge_ids
        .iter()
        .filter_map(|edge_id| host_by_id.get(edge_id))
        .filter_map(|road| {
            let (a, b) = (source.node(road.from)?, source.node(road.to)?);
            Some(access_candidates::BoundaryHost {
                road_id: road.id,
                from: road.from,
                to: road.to,
                ax: a.x,
                az: a.z,
                bx: b.x,
                bz: b.z,
            })
        })
        .collect::<Vec<_>>();
    let mut scored_targets = Vec::new();
    for point in viable_centers {
        let clearance = context
            .parcel_footprints
            .iter()
            .map(|parcel| {
                ((point.0 - parcel.x).hypot(point.1 - parcel.z)
                    - 0.5 * parcel.width.hypot(parcel.depth))
                .max(0.0)
            })
            .chain(context.open_roads.iter().map(|road| {
                point_segment_distance(point.0, point.1, road.ax, road.az, road.bx, road.bz)
            }))
            .fold(f32::INFINITY, f32::min);
        scored_targets.push((clearance, point.0, point.1));
    }
    scored_targets.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| left.1.total_cmp(&right.1))
            .then_with(|| left.2.total_cmp(&right.2))
    });
    let reference_scores = scored_targets
        .iter()
        .map(|&(score, x, z)| [score.to_bits(), x.to_bits(), z.to_bits()])
        .collect::<Vec<_>>();
    let mut unserved_targets = Vec::new();
    for (_, x, z) in scored_targets {
        let far_from_nodes = source
            .state
            .network
            .nodes
            .iter()
            .all(|node| (node.x - x).hypot(node.z - z) >= 20.0);
        let far_from_targets = unserved_targets
            .iter()
            .all(|(old_x, old_z)| (old_x - x).hypot(old_z - z) >= 18.0);
        if far_from_nodes && far_from_targets {
            unserved_targets.push((x, z));
            if unserved_targets.len() == 12 {
                break;
            }
        }
    }
    let candidate_nodes = source
        .state
        .network
        .nodes
        .iter()
        .map(|node| access_candidates::CandidateNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();
    // Real mature geometry must preserve the offline host/split sequence
    // across quotas, including serialized pauses inside projection/sorting.
    let mut ordered_hosts = target_hosts.clone();
    ordered_hosts.sort_by_key(|host| host.road_id);
    let expected_splits = ordered_hosts
        .iter()
        .flat_map(|host| {
            access_candidates::host_split_samples(
                *host,
                &candidate_nodes,
                &context.parcel_footprints,
            )
            .into_iter()
            .map(move |along| {
                (
                    host.road_id,
                    along.to_bits(),
                    (along
                        / crate::legacy::access_math::length(host.bx - host.ax, host.bz - host.az))
                    .to_bits(),
                )
            })
        })
        .collect::<Vec<_>>();
    let mut preparation_work_by_quota = Vec::new();
    for quota in [1, 7, 48] {
        let mut job = access_search::AccessHostPreparation::new(
            &target_face,
            &target_hosts,
            &candidate_nodes,
            &context.parcel_footprints,
        )
        .unwrap();
        let mut calls = 0usize;
        let mut work = 0usize;
        loop {
            let result = job.advance(quota);
            assert!(result.work_used <= quota);
            work += result.work_used;
            calls += 1;
            if result.done {
                break;
            }
            assert!(
                result.work_used > 0,
                "unfinished host preparation makes budgeted progress"
            );
            if calls % 193 == 0 {
                let encoded = serde_json::to_vec(&job.checkpoint()).unwrap();
                job = access_search::AccessHostPreparation::restore(
                    serde_json::from_slice(&encoded).unwrap(),
                )
                .unwrap();
            }
            assert!(calls < 1_000_000, "host preparation terminates");
        }
        let actual = job
            .split_samples()
            .unwrap()
            .iter()
            .map(|sample| {
                (
                    sample.road_id,
                    sample.along_m.to_bits(),
                    sample.host_t.to_bits(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual, expected_splits,
            "mature host split parity at quota {quota}"
        );
        preparation_work_by_quota.push(work);
    }
    assert!(
        preparation_work_by_quota
            .windows(2)
            .all(|pair| pair[0] == pair[1]),
        "work accounting is quota independent"
    );
    let preparation_started = std::time::Instant::now();
    let prepared = access_candidates::PreparedAccessSearch::prepare(
        &target_face,
        &target_hosts,
        &candidate_nodes,
        &context.parcel_footprints,
        &unserved_targets,
    );
    let preparation_elapsed_ms = preparation_started.elapsed().as_millis();
    let mut candidates = Vec::with_capacity(prepared.len());
    let mut cursor = 0usize;
    let mut page_count = 0usize;
    let mut page_work_max = 0usize;
    let mut paged_exact_proposals = Vec::new();
    let page_evaluation_started = std::time::Instant::now();
    loop {
        let page = prepared.page(cursor);
        page_work_max = page_work_max.max(page.work_units);
        let scan = source.propose_interior_access_from_geometry_candidates(0, &page.candidates);
        paged_exact_proposals.extend(scan.proposals);
        candidates.extend(page.candidates);
        page_count += 1;
        cursor = page.next_cursor;
        if page.exhausted {
            break;
        }
    }
    // Explicit export only: keep the golden independent from routine reruns
    // of the implementation whose output will later be compared against it.
    if std::env::var_os("EXPORT_ACCESS_REFERENCE").is_some() {
        let reference = serde_json::json!({
            "schema": 1,
            "source_save": "tests/fixtures/validation/early-reservation-wasm-87231-save.json",
            "source_save_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
            "seed": source.state.seed,
            "tick": source.state.tick,
            "mode": "PreparedAccessSearch exhaustive geometry; floats also recorded as bits for exact parity",
            "face": {
                "identity": target_face.identity,
                "vertex_ids": target_face.vertex_ids,
                "polygon": target_face.polygon,
                "area": target_face.area,
                "boundary_edge_ids": target_face.boundary_edge_ids,
                "contains_proposed_edge": target_face.contains_proposed_edge
            },
            "hosts": target_hosts,
            "nodes": candidate_nodes,
            "parcels": context.parcel_footprints,
            "targets": unserved_targets,
            "descriptor_count": candidates.len(),
            "ordered_descriptor_bits": candidates.iter().map(|candidate| {
                [candidate.host_id, candidate.host_t.to_bits(), candidate.end_x.to_bits(), candidate.end_z.to_bits()]
            }).collect::<Vec<_>>(),
            "tie_index_contract": "Underlying exhaustive output uses global ordinal as u8; page() resets tie_index to ordinal within the 48-item page. Geometry/order bits above exclude this paging detail."
        });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/validation/access-descriptor-reference.json");
        std::fs::write(path, serde_json::to_vec_pretty(&reference).unwrap()).unwrap();
    }
    {
        let reference = serde_json::json!({
            "schema": 1,
            "source_save": "tests/fixtures/validation/early-reservation-wasm-87231-save.json",
            "source_save_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
            "seed": source.state.seed, "tick": source.state.tick,
            "faces": reference_faces,
            "face_samples": reference_face_samples,
            "ranked_viable_faces": reference_ranking,
            "selected_face_identity": target_face.identity,
            "selected_viable_count": viable_sites,
            "ordered_score_x_z_bits": reference_scores,
            "target_bits": unserved_targets.iter().map(|&(x,z)| [x.to_bits(),z.to_bits()]).collect::<Vec<_>>(),
            "hosts": target_hosts,
            "ordered_descriptor_bits": candidates.iter().map(|candidate| {
                [candidate.host_id, candidate.host_t.to_bits(), candidate.end_x.to_bits(), candidate.end_z.to_bits()]
            }).collect::<Vec<_>>()
        });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/validation/face-target-reference.json");
        if export_targets {
            std::fs::write(path, serde_json::to_vec_pretty(&reference).unwrap()).unwrap();
        } else {
            let expected: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            // Compare the wire representation on both sides: serde_json's
            // parsed numeric values can differ from in-memory f32-to-Value
            // promotion even when their serialized documents are identical.
            let reference: serde_json::Value =
                serde_json::from_slice(&serde_json::to_vec(&reference).unwrap()).unwrap();
            for key in reference.as_object().unwrap().keys() {
                assert!(
                    reference[key] == expected[key],
                    "frozen face-target reference differs at {key}"
                );
            }
        }
    }
    let page_evaluation_elapsed_ms = page_evaluation_started.elapsed().as_millis();
    let complete_diagnostic_elapsed_ms = diagnostic_started.elapsed().as_millis();
    assert_eq!(cursor, prepared.len());
    assert!(page_work_max <= interior_access::MAX_PROPOSALS_PER_EVALUATION);
    let proposal_count = paged_exact_proposals.len();
    let mut feasible = paged_exact_proposals
        .into_iter()
        .filter_map(|mut proposal| {
            let host = host_by_id.get(&proposal.host_id)?;
            let (host_a, host_b) = (source.node(host.from)?, source.node(host.to)?);
            let start = WorldPoint {
                x: host_a.x + (host_b.x - host_a.x) * proposal.host_t,
                z: host_a.z + (host_b.z - host_a.z) * proposal.host_t,
            };
            if !(0..=32).all(|sample| {
                let t = sample as f32 / 32.0;
                diagnostic_point_in_face(
                    (
                        start.x + (proposal.end_x - start.x) * t,
                        start.z + (proposal.end_z - start.z) * t,
                    ),
                    &target_face.polygon,
                )
            }) {
                return None;
            }
            proposal.selected_lots.retain(|lot| {
                diagnostic_rect_inside_face(
                    &geometry::OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    },
                    &target_face.polygon,
                )
            });
            if proposal.selected_lots.len() < 2 {
                return None;
            }
            proposal.score.site_area = proposal
                .selected_lots
                .iter()
                .map(|lot| (lot.width * lot.depth * 10_000.0).round() as u32)
                .sum();
            Some(proposal)
        })
        .collect::<Vec<_>>();
    feasible.sort_by(|left, right| right.score.compare(left.score));
    assert_eq!(
        source.save_json(),
        source_before,
        "candidate enumeration remains read-only"
    );
    let Some(proposal) = feasible.first().cloned() else {
        eprintln!(
            "no admissible straight-branch candidate after complete paged scan: viable_sites={viable_sites}, faces={:?}, hosts={}, candidates={}, preparation_counted_units={}, pages={}, max_page_work={}, proposals={}",
            target_face.identity,
            target_hosts.len(),
            candidates.len(),
            prepared.preparation_counted_units,
            page_count,
            page_work_max,
            proposal_count,
        );
        let report = serde_json::json!({
            "diagnostic": "complete prepared-page search; diagnostic only, no runtime policy",
            "source_save_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
            "seed": source.state.seed,
            "tick": source.state.tick,
            "face_identity": target_face.identity,
            "face_area_m2": target_face.area,
            "host_count": target_hosts.len(),
            "prepared_descriptor_count": prepared.len(),
            "preparation_counted_units": prepared.preparation_counted_units,
            "preparation_elapsed_ms": preparation_elapsed_ms,
            "page_evaluation_elapsed_ms": page_evaluation_elapsed_ms,
            "complete_diagnostic_including_host_parity_elapsed_ms": complete_diagnostic_elapsed_ms,
            "complete_diagnostic_including_host_parity_elapsed_ms": complete_diagnostic_elapsed_ms,
            "page_size": interior_access::MAX_PROPOSALS_PER_EVALUATION,
            "page_count": page_count,
            "max_page_work_units": page_work_max,
            "exact_proposal_count": proposal_count,
            "face_contained_exact_proposal_count": 0,
            "generator_preparation_bound": access_candidates::MAX_PREPARED_DESCRIPTORS,
            "nested_preparation_caps": {
                "hosts": 24, "split_points_per_host": 4, "headings_per_split": 69,
                "lengths_per_heading": 4, "targets_per_split": 12,
                "nearest_blockers_per_split": 4, "face_vertices": access_candidates::MAX_FACE_VERTICES_FOR_PREPARED_SEARCH,
                "input_nodes_max": access_candidates::MAX_PREPARED_INPUT_NODES,
                "input_parcels_max": access_candidates::MAX_PREPARED_INPUT_PARCELS
            },
            "limitations": "All prepared descriptors were visited in stable pages. Exact evaluator checks engine feasibility; face containment is checked afterward. This diagnostic does not mutate the world or select production growth."
        });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../validation/face-access-paged-search.json");
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        assert_eq!(
            proposal_count, 0,
            "expected no accepted proposal under restored split policy"
        );
        return;
    };

    let mut admitted = Engine::restore_json(SAVE).expect("fresh copy for atomic admission");
    let before_save = admitted.save_json();
    assert!(admitted.admit_interior_access(
        proposal.host_id,
        proposal.host_t,
        WorldPoint {
            x: proposal.end_x,
            z: proposal.end_z
        },
        proposal.selected_lots.clone(),
    ));
    admitted
        .validate()
        .expect("geometry-discovered plan fully validates");
    let pending = admitted
        .state
        .legacy
        .pending_interior_access
        .as_ref()
        .unwrap();
    assert_eq!(pending.selected_lots.len(), proposal.selected_lots.len());
    assert_eq!(
        Engine::restore_json(&admitted.save_json())
            .unwrap()
            .save_json(),
        admitted.save_json()
    );
    assert_eq!(
        source.save_json(),
        before_save,
        "enumeration and evaluation are read-only"
    );
    // Follow the discovered route through ordinary fixed-step construction,
    // preserving the engine's project duration and all other city activity.
    let branch_id = pending.branch_road_id;
    let pending_save = admitted.save_json();
    let mut resumed = Engine::restore_json(&pending_save).unwrap();
    let started_tick = admitted.state.tick;
    let mut opened_tick = None;
    for _ in 0..100 {
        admitted.step(200);
        resumed.step(200);
        assert_eq!(
            admitted.save_json(),
            resumed.save_json(),
            "discovered road construction replays across restore"
        );
        admitted
            .validate()
            .expect("discovered road construction remains valid");
        let branch = admitted
            .state
            .network
            .roads
            .iter()
            .find(|road| road.id == branch_id)
            .unwrap();
        let frontage_count = admitted
            .state
            .land
            .parcel_frontages
            .iter()
            .filter(|frontage| frontage.road_id == branch_id)
            .count();
        if branch.stage == Stage::Open {
            assert!(branch.connected);
            assert_eq!(frontage_count, proposal.selected_lots.len());
            assert!(admitted
                .state
                .legacy
                .pending_interior_access
                .as_ref()
                .is_none_or(|plan| plan.branch_road_id != branch_id));
            opened_tick = Some(admitted.state.tick);
            break;
        }
        assert_eq!(
            frontage_count, 0,
            "dependent lots must wait for their road to open"
        );
        // Exercise a mid-construction resume, not just the initial admission.
        resumed = Engine::restore_json(&resumed.save_json()).unwrap();
    }
    let opened_tick = opened_tick.expect("discovered access road opens within 20,000 normal ticks");
    let report = serde_json::json!({
        "diagnostic": "exhaustive prepared-page search on a mature road face; diagnostic only",
        "source_save": "tests/fixtures/validation/early-reservation-wasm-87231-save.json",
        "source_save_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
        "seed": source.state.seed,
        "tick": source.state.tick,
        "face_identity": target_face.identity,
        "face_area_m2": target_face.area,
        "boundary_host_count": target_hosts.len(),
        "search": {
            "prepared_descriptor_count": prepared.len(),
            "descriptor_cap": access_candidates::MAX_PREPARED_DESCRIPTORS,
            "preparation_counted_units": prepared.preparation_counted_units,
            "preparation_counted_units_definition": "split prefilter bundles plus endpoint descriptor checks; does not count all nested ray/polygon/target/parcel operations",
            "page_size": interior_access::MAX_PROPOSALS_PER_EVALUATION,
            "page_count": page_count,
            "max_page_work_units": page_work_max,
            "face_vertex_cap": access_candidates::MAX_FACE_VERTICES_FOR_PREPARED_SEARCH,
            "preparation_elapsed_ms": preparation_elapsed_ms,
            "page_evaluation_elapsed_ms": page_evaluation_elapsed_ms,
            "complete_diagnostic_including_host_parity_elapsed_ms": complete_diagnostic_elapsed_ms,
        },
        "exact_engine_proposal_count_before_face_filter": proposal_count,
        "face_contained_proposal_count": feasible.len(),
        "selected_candidate": {
            "host_road_id": proposal.host_id,
            "host_t": proposal.host_t,
            "endpoint_x": proposal.end_x,
            "endpoint_z": proposal.end_z,
            "selected_lots": proposal.selected_lots,
            "selected_nominal_envelope_m2": proposal.score.site_area as f64 / 10_000.0,
        },
        "admission_and_validation_passed": true,
        "canonical_restore_passed": true,
        "incremental_host_preparation": {
            "quotas": [1, 7, 48],
            "split_count": expected_splits.len(),
            "work_units_by_quota": preparation_work_by_quota,
            "bit_exact_reference_parity": true,
            "serialized_resume_interval_calls": 193,
            "scope": "host interval/split stage only; heading and endpoint search remain synchronous"
        },
        "normal_construction_replay": {
            "started_tick": started_tick,
            "observed_open_tick": opened_tick,
            "check_interval_ticks": 200,
            "connected_road_and_dependent_lots_verified": true,
            "mid_construction_restore_verified": true
        },
        "production_runtime_changed": false,
        "limitations": "This demonstrates discovery in one mature fixture face, not a general growth policy or density win. Preparation is synchronous/offline; its counted work proxy excludes nested geometry operations."
    });
    let report_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/face-access-paged-search.json");
    std::fs::write(report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!(
        "geometry access candidate admitted after complete page scan: viable_sites={viable_sites} face={:?} hosts={} candidates={} preparation_counted_units={} pages={} max_page_work={} exact_proposals={} host={} t={} endpoint=({}, {}) lots={} area_m2={:.2}",
        target_face.identity,
        target_hosts.len(),
        candidates.len(),
        prepared.preparation_counted_units,
        page_count,
        page_work_max,
        proposal_count,
        proposal.host_id,
        proposal.host_t,
        proposal.end_x,
        proposal.end_z,
        proposal.selected_lots.len(),
        proposal.score.site_area as f64 / 10_000.0,
    );
}

#[test]
#[ignore = "exports the generalized geometry-valid pending access plan for WASM integration checks"]
fn export_generalized_access_pending_fixture() {
    let (engine, _) = generalized_access_candidate_engine();
    let save = engine.save_json();
    let snapshot = engine.snapshot_json();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation");
    std::fs::write(root.join("generalized-access-pending-save.json"), &save).unwrap();
    std::fs::write(
        root.join("generalized-access-pending-snapshot.json"),
        &snapshot,
    )
    .unwrap();
    let source = std::fs::read(root.join("early-reservation-wasm-87231-save.json")).unwrap();
    let provenance = serde_json::json!({
        "source_save": "early-reservation-wasm-87231-save.json",
        "source_save_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
        "save_sha256": "37e00c1ed96f26a631ef576b87acf902053da01a705f81c697cc7d84dc2f802e",
        "snapshot_sha256": "aad17b327bf83629a7c92fe872d95345a6f2bd8edef098fae64552426964c9fb",
        "source_byte_length": source.len(),
        "candidate": {"host_road_id":3317,"host_t":0.72,"endpoint_x":181.0,"endpoint_z":15.0},
        "purpose": "Native-generated pending save for WASM save/restore and continuation validation",
        "tick": engine.state.tick,
        "seed": engine.state.seed,
        "pending_plan": engine.state.legacy.pending_interior_access,
        "save_file": "generalized-access-pending-save.json",
        "snapshot_file": "generalized-access-pending-snapshot.json"
    });
    std::fs::write(
        root.join("generalized-access-pending-provenance.json"),
        serde_json::to_vec_pretty(&provenance).unwrap(),
    )
    .unwrap();
    println!(
        "exported pending access fixture at tick {} with {} lots",
        engine.state.tick,
        engine
            .state
            .legacy
            .pending_interior_access
            .as_ref()
            .unwrap()
            .selected_lots
            .len()
    );
}

#[test]
#[ignore = "bounded exact-engine scan of viable access proposals inside the largest face"]
fn scan_exact_planner_for_access_into_largest_face() {
    use serde_json::json;
    use std::collections::BTreeSet;

    const SAVE: &str =
        include_str!("../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
    let engine = Engine::restore_json(SAVE).expect("matching checkpoint restores");
    let before = engine.save_json();
    let ordinary: Vec<_> = engine
        .state
        .network
        .roads
        .iter()
        .filter(|r| {
            r.stage == Stage::Open
                && r.connected
                && matches!(r.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .collect();
    let used_nodes: BTreeSet<u32> = ordinary.iter().flat_map(|r| [r.from, r.to]).collect();
    let face_nodes: Vec<_> = engine
        .state
        .network
        .nodes
        .iter()
        .filter(|n| used_nodes.contains(&n.id))
        .map(|n| road_faces::RoadFaceNode {
            id: n.id,
            x: n.x,
            z: n.z,
        })
        .collect();
    let face_edges: Vec<_> = ordinary
        .iter()
        .map(|r| road_faces::RoadFaceEdge {
            id: r.id,
            from: r.from,
            to: r.to,
        })
        .collect();
    let faces = road_faces::trace_faces(&face_nodes, &face_edges, None)
        .expect("checkpoint ordinary graph is planar");
    let target = (223.53677_f32, -6.73742_f32);
    let point_in_poly = |x: f32, z: f32, polygon: &[(f32, f32)]| {
        let mut inside = false;
        for i in 0..polygon.len() {
            let (ax, az) = polygon[i];
            let (bx, bz) = polygon[(i + polygon.len() - 1) % polygon.len()];
            if (az > z) != (bz > z) && x < (bx - ax) * (z - az) / (bz - az) + ax {
                inside = !inside;
            }
        }
        inside
    };
    let face = faces
        .iter()
        .find(|face| point_in_poly(target.0, target.1, &face.polygon))
        .expect("largest free component lies in a traced bounded face");
    let path_inside = |ax: f32, az: f32, bx: f32, bz: f32| {
        let length = (bx - ax).hypot(bz - az);
        let steps = (length / 0.5).ceil().max(1.0) as usize;
        (1..=steps).all(|step| {
            let t = step as f32 / steps as f32;
            point_in_poly(ax + (bx - ax) * t, az + (bz - az) * t, &face.polygon)
        })
    };
    let lot_inside = |lot: &interior_access::InteriorAccessLot| {
        let rect = geometry::OrientedRect {
            x: lot.x,
            z: lot.z,
            width: lot.width,
            depth: lot.depth,
            rotation: lot.rotation,
        };
        let corners = rect.corners();
        if !point_in_poly(rect.x, rect.z, &face.polygon)
            || corners
                .iter()
                .any(|p| !point_in_poly(p.0, p.1, &face.polygon))
        {
            return false;
        }
        (0..4).all(|i| {
            path_inside(
                corners[i].0,
                corners[i].1,
                corners[(i + 1) % 4].0,
                corners[(i + 1) % 4].1,
            )
        })
    };

    let proposal_count = ordinary
        .len()
        .saturating_mul(interior_access::PROPOSALS_PER_HOST);
    let mut exact_proposals = 0usize;
    let mut branch_in_face = 0usize;
    let mut lots_in_face = 0usize;
    let mut winners = Vec::new();
    for cursor in (0..proposal_count).step_by(interior_access::MAX_PROPOSALS_PER_EVALUATION) {
        let scan = engine.propose_interior_access_from(None, cursor);
        exact_proposals += scan.proposals.len();
        for proposal in scan.proposals {
            let host = engine
                .state
                .network
                .roads
                .iter()
                .find(|r| r.id == proposal.host_id)
                .unwrap();
            let a = engine.node(host.from).unwrap();
            let b = engine.node(host.to).unwrap();
            let sx = a.x + (b.x - a.x) * proposal.host_t;
            let sz = a.z + (b.z - a.z) * proposal.host_t;
            if !path_inside(sx, sz, proposal.end_x, proposal.end_z) {
                continue;
            }
            branch_in_face += 1;
            let valid_lots: Vec<_> = proposal
                .selected_lots
                .iter()
                .copied()
                .filter(lot_inside)
                .collect();
            if valid_lots.len() < 2 {
                continue;
            }
            lots_in_face += 1;
            let mut split_probe = Engine::restore_json(SAVE).unwrap();
            let split = match split_probe.try_split_open_road(proposal.host_id, proposal.host_t) {
                Ok(split) => split,
                Err(_) => continue,
            };
            let split_node = split_probe.node(split.split_node_id).unwrap().clone();
            let end = WorldPoint {
                x: proposal.end_x,
                z: proposal.end_z,
            };
            let branch_context = split_probe.collision_context();
            let branch_gates = json!({
                "bounds": end.x>=MIN+24.0&&end.x<=MAX-24.0&&end.z>=MIN+24.0&&end.z<=MAX-24.0,
                "river": split_probe.ordinary_road_clear_of_river(split_node.x,split_node.z,end.x,end.z,4.5),
                "protected": !corridor_crosses_protected(split_node.x,split_node.z,end.x,end.z),
                "parcels": split_probe.road_corridor_clear_of_parcels_with_context(split_node.x,split_node.z,end.x,end.z,&branch_context),
                "endpoint_spacing_m": split_probe.state.network.nodes.iter().map(|n|(n.x-end.x).hypot(n.z-end.z)).fold(f32::INFINITY,f32::min),
                "junction_heading": split_probe.ordinary_heading_clear(split.split_node_id,(end.z-split_node.z).atan2(end.x-split_node.x)),
            });
            let branch_context_ok = branch_gates["bounds"] == true
                && branch_gates["river"] == true
                && branch_gates["protected"] == true
                && branch_gates["parcels"] == true
                && branch_gates["endpoint_spacing_m"].as_f64().unwrap_or(0.0) >= 18.0
                && branch_gates["junction_heading"] == true;
            let mut proposed_context = branch_context.clone();
            proposed_context.open_roads.push(CollisionSegment {
                ax: split_node.x,
                az: split_node.z,
                bx: end.x,
                bz: end.z,
            });
            let lots_pass = valid_lots.iter().all(|lot| {
                lot.width >= 8.0 && lot.depth >= 8.0 && {
                    let rect = geometry::OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    };
                    split_probe.progressive_frontage_geometry_clear(&rect, &proposed_context)
                }
            });
            if !branch_context_ok || !lots_pass {
                continue;
            }
            winners.push(json!({
                "host_road_id":proposal.host_id,"host_t":proposal.host_t,
                "start_x":split_node.x,"start_z":split_node.z,
                "end_x":proposal.end_x,"end_z":proposal.end_z,
                "branch_length_m":(proposal.end_x-split_node.x).hypot(proposal.end_z-split_node.z),
                "site_area_m2":valid_lots.iter().map(|l|l.width*l.depth).sum::<f32>(),
                "selected_lots":valid_lots.iter().map(|l|json!({"x":l.x,"z":l.z,"width":l.width,"depth":l.depth,"rotation":l.rotation,"along":l.along,"side":l.side})).collect::<Vec<_>>(),
                "branch_gates":branch_gates,
                "split_preflight":"pass",
                "lot_admission":"pass",
            }));
        }
    }
    assert_eq!(
        engine.save_json(),
        before,
        "proposal enumeration is read-only"
    );
    winners.sort_by(|a, b| {
        let score = |v: &serde_json::Value| {
            v["site_area_m2"].as_f64().unwrap_or(0.0) / v["branch_length_m"].as_f64().unwrap_or(1.0)
        };
        score(b).total_cmp(&score(a))
    });
    let report = json!({
        "checkpoint_tick":engine.state.tick,
        "faces":faces.len(),
        "target_face_vertex_ids":face.vertex_ids,
        "target_face_area_m2":face.area,
        "candidate_budget":proposal_count,
        "exact_engine_proposals":exact_proposals,
        "branches_inside_target_face":branch_in_face,
        "branches_with_at_least_two_lots_inside_face":lots_in_face,
        "exactly_admissible_candidates":winners.len(),
        "best_candidates":winners.into_iter().take(8).collect::<Vec<_>>(),
        "source_save_unchanged":engine.save_json()==before,
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/face-access-exact-planner-scan.json");
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{}", report);

    // The prior JS route had a dogleg. Search a deterministic 2m endpoint grid
    // for a single straight branch from the same exact-preflighted host split.
    // This is an offline test probe, not a production candidate generator.
    let mut split_probe = Engine::restore_json(SAVE).unwrap();
    let split = split_probe
        .try_split_open_road(3317, 0.72)
        .expect("host split preflight");
    let start = split_probe.node(split.split_node_id).unwrap().clone();
    let context = split_probe.collision_context();
    let min_x = face
        .polygon
        .iter()
        .map(|p| p.0)
        .fold(f32::INFINITY, f32::min);
    let max_x = face
        .polygon
        .iter()
        .map(|p| p.0)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_z = face
        .polygon
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min);
    let max_z = face
        .polygon
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut rejected = BTreeMap::<&'static str, usize>::new();
    let mut straight_winners = Vec::new();
    let mut typed_straight_winner: Option<(
        f32,
        WorldPoint,
        Vec<interior_access::InteriorAccessLot>,
    )> = None;
    let mut checked = 0usize;
    let mut endpoint_x = (min_x / 2.0).floor() * 2.0 + 1.0;
    while endpoint_x <= max_x {
        let mut endpoint_z = (min_z / 2.0).floor() * 2.0 + 1.0;
        while endpoint_z <= max_z {
            let sample_z = endpoint_z;
            endpoint_z += 2.0;
            let dx = endpoint_x - start.x;
            let dz = sample_z - start.z;
            let length = dx.hypot(dz);
            if !(26.0..=60.0).contains(&length)
                || !path_inside(start.x, start.z, endpoint_x, sample_z)
            {
                continue;
            }
            checked += 1;
            let reason = if endpoint_x < MIN + 24.0
                || endpoint_x > MAX - 24.0
                || sample_z < MIN + 24.0
                || sample_z > MAX - 24.0
            {
                Some("bounds")
            } else if split_probe
                .state
                .network
                .nodes
                .iter()
                .any(|n| (n.x - endpoint_x).hypot(n.z - sample_z) < 18.0)
            {
                Some("junction_spacing")
            } else if !split_probe.ordinary_heading_clear(split.split_node_id, dz.atan2(dx)) {
                Some("junction_angle")
            } else if !split_probe
                .ordinary_road_clear_of_river(start.x, start.z, endpoint_x, sample_z, 4.5)
            {
                Some("river")
            } else if corridor_crosses_protected(start.x, start.z, endpoint_x, sample_z) {
                Some("protected")
            } else if !split_probe.road_corridor_clear_of_parcels_with_context(
                start.x, start.z, endpoint_x, sample_z, &context,
            ) {
                Some("parcel_corridor")
            } else if split_probe.state.network.roads.iter().any(|road| {
                if road.from == split.split_node_id || road.to == split.split_node_id {
                    return false;
                }
                let (Some(a), Some(b)) = (split_probe.node(road.from), split_probe.node(road.to))
                else {
                    return true;
                };
                geometry::segments_distance_coords(
                    start.x, start.z, endpoint_x, sample_z, a.x, a.z, b.x, b.z,
                ) < 8.0
            }) {
                Some("road_spacing")
            } else {
                None
            };
            if let Some(reason) = reason {
                *rejected.entry(reason).or_default() += 1;
                continue;
            }
            let mut proposed_context = context.clone();
            proposed_context.open_roads.push(CollisionSegment {
                ax: start.x,
                az: start.z,
                bx: endpoint_x,
                bz: sample_z,
            });
            let intervals = frontage::subdivide_frontage(
                length,
                &[],
                FRONTAGE_ENDPOINT_CLEARANCE,
                8.0,
                14.0,
                2.2,
                MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
            );
            let nx = -dz / length;
            let nz = dx / length;
            let mut lots = Vec::new();
            for side in [-1_i8, 1] {
                for interval in &intervals {
                    let along = (interval.start + interval.end) * 0.5;
                    let rect = geometry::OrientedRect {
                        // Use the perpendicular offset implied by frontage rotation
                        // (local +Z points from the lot toward the road).
                        x: start.x + dx * (along / length) + nx * side as f32 * 15.0,
                        z: start.z + dz * (along / length) + nz * side as f32 * 15.0,
                        width: interval.end - interval.start,
                        depth: 12.0,
                        rotation: (side as f32 * dz).atan2(-side as f32 * dx),
                    };
                    let corners = rect.corners();
                    let in_face = point_in_poly(rect.x, rect.z, &face.polygon)
                        && corners
                            .iter()
                            .all(|p| point_in_poly(p.0, p.1, &face.polygon));
                    if in_face
                        && split_probe.progressive_frontage_geometry_clear(&rect, &proposed_context)
                    {
                        lots.push(rect);
                    }
                }
            }
            if lots.len() < 2 {
                *rejected.entry("fewer_than_two_exact_lots").or_default() += 1;
                continue;
            }
            if typed_straight_winner.is_none() {
                let ux = dx / length;
                let uz = dz / length;
                let lot_nx = -uz;
                let lot_nz = ux;
                let typed = lots
                    .iter()
                    .map(|rect| {
                        let along = ((rect.x - start.x) * ux + (rect.z - start.z) * uz) / length;
                        let side = ((((rect.x - (start.x + ux * along * length)) * lot_nx
                            + (rect.z - (start.z + uz * along * length)) * lot_nz)
                            / 15.0)
                            .round() as i8)
                            .clamp(-1, 1);
                        interior_access::InteriorAccessLot {
                            x: rect.x,
                            z: rect.z,
                            width: rect.width,
                            depth: rect.depth,
                            rotation: rect.rotation,
                            along,
                            side,
                        }
                    })
                    .collect();
                typed_straight_winner = Some((
                    0.72,
                    WorldPoint {
                        x: endpoint_x,
                        z: sample_z,
                    },
                    typed,
                ));
            }
            straight_winners.push(json!({
                "host_road_id":3317,"host_t":0.72,"start_x":start.x,"start_z":start.z,
                "end_x":endpoint_x,"end_z":sample_z,"length_m":length,
                "site_area_m2":lots.iter().map(|r|r.width*r.depth).sum::<f32>(),
                "lot_rectangles":lots.iter().map(|r|json!({"x":r.x,"z":r.z,"width":r.width,"depth":r.depth,"rotation":r.rotation})).collect::<Vec<_>>(),
            }));
        }
        endpoint_x += 2.0;
    }
    straight_winners.sort_by(|a, b| {
        let score = |v: &serde_json::Value| {
            v["site_area_m2"].as_f64().unwrap_or(0.0) / v["length_m"].as_f64().unwrap_or(1.0)
        };
        score(b).total_cmp(&score(a))
    });
    let lifecycle=typed_straight_winner.map(|(host_t,end,lots)| {
        let mut candidate=Engine::restore_json(SAVE).unwrap();
        let admitted=candidate.admit_interior_access(3317,host_t,end,lots);
        let validation=candidate.validate().err();
        json!({"admission_call_passed":admitted,"saved_world_validation_passed":validation.is_none(),"validation_error":validation})
    });
    let straight_report = json!({
        "host_road_id":3317,"host_t":0.72,"split_preflight":"passed",
        "face_vertex_ids":face.vertex_ids,"face_area_m2":face.area,
        "endpoint_candidates_inside_face_length_26_to_60m":checked,
        "rejections":rejected,"exact_straight_candidates_with_two_accessible_lots":straight_winners.len(),
        "best_candidates":straight_winners.into_iter().take(12).collect::<Vec<_>>(),
        "existing_lifecycle_probe_for_first_geometry_candidate":lifecycle,
        "source_save_unchanged":engine.save_json()==before,
        "note":"Endpoint grid and face containment are diagnostic sampling; all emitted lots and branch constraints use current engine predicates. No world state was committed.",
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/face-access-straight-candidates.json");
    std::fs::write(path, serde_json::to_vec_pretty(&straight_report).unwrap()).unwrap();
    println!("{}", straight_report);

    // Audit proposals that the saved lifecycle schema can actually serialize:
    // the three supported split fractions and four supported perpendicular
    // branch lengths, on every edge of this same face.
    let mut legal_counts = BTreeMap::<&'static str, usize>::new();
    let mut legal_candidates = Vec::new();
    let mut legal_checked = 0usize;
    let mut host_focus = Vec::new();
    let host_ids: Vec<u32> = face.boundary_edge_ids.iter().copied().collect();
    for host_id in host_ids {
        let Some(host) = engine.state.network.roads.iter().find(|r| r.id == host_id) else {
            continue;
        };
        if !matches!(host.kind, RoadKind::Local | RoadKind::Arterial)
            || host.stage != Stage::Open
            || !host.connected
        {
            continue;
        }
        let (Some(a), Some(b)) = (engine.node(host.from), engine.node(host.to)) else {
            continue;
        };
        let (ax, az, bx, bz) = (a.x, a.z, b.x, b.z);
        let hdx = bx - ax;
        let hdz = bz - az;
        let hlen = hdx.hypot(hdz);
        for host_t in interior_access::FRACTIONS {
            let mut probe = Engine::restore_json(SAVE).unwrap();
            let split = match probe.try_split_open_road(host_id, host_t) {
                Ok(split) => split,
                Err(_) => {
                    *legal_counts.entry("host_split_preflight").or_default() +=
                        interior_access::SIDES.len() * interior_access::LENGTHS.len();
                    continue;
                }
            };
            let start = probe.node(split.split_node_id).unwrap().clone();
            let context = probe.collision_context();
            let ux = hdx / hlen;
            let uz = hdz / hlen;
            for side in interior_access::SIDES {
                let nx = -uz * side as f32;
                let nz = ux * side as f32;
                for length in interior_access::LENGTHS {
                    legal_checked += 1;
                    let end = WorldPoint {
                        x: start.x + nx * length,
                        z: start.z + nz * length,
                    };
                    let reason = if end.x < MIN + 24.0
                        || end.x > MAX - 24.0
                        || end.z < MIN + 24.0
                        || end.z > MAX - 24.0
                    {
                        Some("bounds")
                    } else if !path_inside(start.x, start.z, end.x, end.z) {
                        Some("outside_target_face")
                    } else if !probe.ordinary_heading_clear(
                        split.split_node_id,
                        (end.z - start.z).atan2(end.x - start.x),
                    ) {
                        Some("junction_angle")
                    } else if probe
                        .state
                        .network
                        .nodes
                        .iter()
                        .any(|n| (n.x - end.x).hypot(n.z - end.z) < 18.0)
                    {
                        Some("junction_spacing")
                    } else if !probe
                        .ordinary_road_clear_of_river(start.x, start.z, end.x, end.z, 4.5)
                    {
                        Some("river")
                    } else if corridor_crosses_protected(start.x, start.z, end.x, end.z) {
                        Some("protected")
                    } else if !probe.road_corridor_clear_of_parcels_with_context(
                        start.x, start.z, end.x, end.z, &context,
                    ) {
                        Some("parcel_corridor")
                    } else if probe.state.network.roads.iter().any(|road| {
                        if road.from == split.split_node_id || road.to == split.split_node_id {
                            return false;
                        }
                        let (Some(ra), Some(rb)) = (probe.node(road.from), probe.node(road.to))
                        else {
                            return true;
                        };
                        geometry::segments_distance_coords(
                            start.x, start.z, end.x, end.z, ra.x, ra.z, rb.x, rb.z,
                        ) < 8.0
                    }) {
                        Some("road_spacing")
                    } else {
                        None
                    };
                    if let Some(reason) = reason {
                        *legal_counts.entry(reason).or_default() += 1;
                        if host_id == 3317 {
                            host_focus.push(json!({"host_t":host_t,"side":side,"length_m":length,"rejection":reason}));
                        }
                        continue;
                    }
                    let mut proposed_context = context.clone();
                    proposed_context.open_roads.push(CollisionSegment {
                        ax: start.x,
                        az: start.z,
                        bx: end.x,
                        bz: end.z,
                    });
                    let dx = end.x - start.x;
                    let dz = end.z - start.z;
                    let branch_len = dx.hypot(dz);
                    let nx = -dz / branch_len;
                    let nz = dx / branch_len;
                    let intervals = frontage::subdivide_frontage(
                        length,
                        &[],
                        FRONTAGE_ENDPOINT_CLEARANCE,
                        8.0,
                        14.0,
                        2.2,
                        MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
                    );
                    let mut lots = Vec::new();
                    for lot_side in [-1_i8, 1] {
                        for interval in &intervals {
                            let along = (interval.start + interval.end) * 0.5;
                            let rect = geometry::OrientedRect {
                                x: start.x + dx * (along / branch_len)
                                    - nz * lot_side as f32 * 15.0,
                                z: start.z
                                    + dz * (along / branch_len)
                                    + nx * lot_side as f32 * 15.0,
                                width: interval.end - interval.start,
                                depth: 12.0,
                                rotation: (lot_side as f32 * dz).atan2(-lot_side as f32 * dx),
                            };
                            let corners = rect.corners();
                            let inside = point_in_poly(rect.x, rect.z, &face.polygon)
                                && corners
                                    .iter()
                                    .all(|p| point_in_poly(p.0, p.1, &face.polygon));
                            if inside
                                && probe
                                    .progressive_frontage_geometry_clear(&rect, &proposed_context)
                            {
                                lots.push(rect);
                            }
                        }
                    }
                    if lots.len() < 2 {
                        *legal_counts.entry("fewer_than_two_exact_lots").or_default() += 1;
                        if host_id == 3317 {
                            host_focus.push(json!({"host_t":host_t,"side":side,"length_m":length,"rejection":"fewer_than_two_exact_lots","lot_count":lots.len()}));
                        }
                        continue;
                    }
                    if host_id == 3317 {
                        host_focus.push(json!({"host_t":host_t,"side":side,"length_m":length,"result":"geometry_and_two_lots_pass","lot_count":lots.len()}));
                    }
                    let score = lots.iter().map(|r| r.width * r.depth).sum::<f32>() / length;
                    legal_candidates.push(json!({
                        "host_road_id":host_id,"host_t":host_t,"side":side,"length_m":length,
                        "start_x":start.x,"start_z":start.z,"end_x":end.x,"end_z":end.z,
                        "lot_count":lots.len(),"nominal_lot_area_m2":lots.iter().map(|r|r.width*r.depth).sum::<f32>(),
                        "area_per_road_m":score,
                        "lots":lots.iter().map(|r|json!({"x":r.x,"z":r.z,"width":r.width,"depth":r.depth,"rotation":r.rotation})).collect::<Vec<_>>(),
                    }));
                }
            }
        }
    }
    legal_candidates.sort_by(|a, b| {
        b["area_per_road_m"]
            .as_f64()
            .unwrap_or(0.0)
            .total_cmp(&a["area_per_road_m"].as_f64().unwrap_or(0.0))
    });
    let legal_report = json!({
        "checkpoint_tick":engine.state.tick,"target_face_vertex_ids":face.vertex_ids,"target_face_area_m2":face.area,
        "candidate_budget":legal_checked,"rejections":legal_counts,
        "host_3317_candidate_details":host_focus,
        "admissible_saved_lifecycle_candidates":legal_candidates.len(),
        "best_candidates":legal_candidates.into_iter().take(12).collect::<Vec<_>>(),
        "note":"Candidates use supported split fractions and branch lengths, exact road-split preflight, current terrain/corridor/junction/parcel rules, and door-facing parcel envelopes. No production state was changed.",
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/face-access-legal-candidates.json");
    std::fs::write(path, serde_json::to_vec_pretty(&legal_report).unwrap()).unwrap();
    println!("{}", legal_report);
}

#[test]
#[ignore = "mature graph preflight quota and JSON resume parity"]
fn mature_graph_preflight_preserves_indexed_edges() {
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let roads = source
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .collect::<Vec<_>>();
    let used = roads
        .iter()
        .flat_map(|road| [road.from, road.to])
        .collect::<BTreeSet<_>>();
    let nodes = source
        .state
        .network
        .nodes
        .iter()
        .filter(|node| used.contains(&node.id))
        .map(|node| road_faces::RoadFaceNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();
    let edges = roads
        .iter()
        .map(|road| road_faces::RoadFaceEdge {
            id: road.id,
            from: road.from,
            to: road.to,
        })
        .collect::<Vec<_>>();
    // The existing tracer independently accepts this complete frozen ordinary graph.
    let faces = road_faces::trace_faces(&nodes, &edges, None).unwrap();
    assert!(!faces.is_empty());
    let expected = edges
        .iter()
        .map(|edge| {
            (
                nodes.iter().position(|node| node.id == edge.from).unwrap(),
                nodes.iter().position(|node| node.id == edge.to).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let mut runs = Vec::new();
    let mut expected_work = None;
    let mut expected_bridge_work = None;
    let mut expected_topology_work = None;
    let mut expected_walk_work = None;
    for quota in [1, 7, 48] {
        let mut job = crate::legacy::road_face_preflight::GraphPreflight::new(
            nodes.clone(),
            edges.clone(),
            None,
        )
        .unwrap();
        let initial = serde_json::to_vec(&job.checkpoint()).unwrap();
        job = crate::legacy::road_face_preflight::GraphPreflight::restore(
            serde_json::from_slice(&initial).unwrap(),
        )
        .unwrap();
        assert_eq!(job.advance(0).work_used, 0);
        let mut work = 0;
        let mut calls = 0;
        let mut restores = 1;
        let mut last_save_work = 0;
        loop {
            let result = job.advance(quota);
            assert!(result.work_used <= quota);
            work += result.work_used;
            calls += 1;
            assert!(work < 2_000_000, "mature graph preflight terminates");
            if result.done {
                break;
            }
            assert!(result.work_used > 0);
            if work - last_save_work >= 8_191 {
                let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                job = crate::legacy::road_face_preflight::GraphPreflight::restore(serde_json::from_slice(&bytes).unwrap()).unwrap_or_else(|error| {
                    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    panic!("preflight restore {error:?}: quota={quota} work={work} stage={} edge={} edge_stage={} lookup={} unique={} from={} to={}", value["stage"], value["edge_cursor"], value["edge_stage"], value["lookup_cursor"], value["unique_cursor"], value["from_index"], value["to_index"]);
                });
                restores += 1;
                last_save_work = work;
            }
        }
        let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
        job = crate::legacy::road_face_preflight::GraphPreflight::restore(serde_json::from_slice(&bytes).unwrap()).unwrap_or_else(|error| {
                    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    panic!("preflight restore {error:?}: quota={quota} work={work} stage={} edge={} edge_stage={} lookup={} unique={} from={} to={}", value["stage"], value["edge_cursor"], value["edge_stage"], value["lookup_cursor"], value["unique_cursor"], value["from_index"], value["to_index"]);
                });
        assert_eq!(job.result().unwrap().unwrap(), expected.as_slice());
        assert_eq!(job.advance(quota).work_used, 0);
        if let Some(previous) = expected_work {
            assert_eq!(work, previous);
        }
        expected_work = Some(work);
        let reference_bridges = road_faces::graph_bridge_edges(nodes.len(), &expected);
        let mut bridge_job = crate::legacy::road_face_bridges::BridgeSearch::new(
            nodes.len(),
            job.result().unwrap().unwrap().to_vec(),
        )
        .unwrap();
        let mut bridge_work = 0usize;
        let mut bridge_saves = 0usize;
        while bridge_job.bridges().is_none() {
            let result = bridge_job.advance(quota);
            assert!(result.work_used <= quota);
            assert!(result.done || result.work_used > 0);
            bridge_work += result.work_used;
            assert!(bridge_work < 2_000_000);
            // This graph is small enough to test every returned bridge-search boundary.
            let bytes = serde_json::to_vec(&bridge_job.checkpoint()).unwrap();
            bridge_job = crate::legacy::road_face_bridges::BridgeSearch::restore(serde_json::from_slice(&bytes).unwrap()).unwrap_or_else(|error| {
                let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                panic!("bridge restore {error:?}: quota={quota} work={bridge_work} phase={} node={} index={} compare={} key={} shifts={}", value["phase"], value["sort_node"], value["sort_index"], value["sort_compare"], value["sort_key"], value["sort_shifts"]);
            });
            bridge_saves += 1;
        }
        if let Some(previous) = expected_bridge_work {
            assert_eq!(bridge_work, previous, "quota-independent bridge work");
        }
        expected_bridge_work = Some(bridge_work);
        let actual_bridges = bridge_job.bridges().unwrap();
        assert_eq!(actual_bridges.len(), expected.len());
        for (index, actual) in actual_bridges.iter().enumerate() {
            assert_eq!(
                *actual,
                reference_bridges.contains(&index),
                "bridge edge {index} at quota {quota}"
            );
        }
        let bridge_flags = actual_bridges.to_vec();
        assert_eq!(bridge_job.advance(quota).work_used, 0);
        let mut topology_job = crate::legacy::road_face_topology::TopologySearch::new(
            nodes.clone(),
            edges.clone(),
            expected.clone(),
            bridge_flags,
        )
        .unwrap();
        let mut topology_work = 0usize;
        let mut topology_saves = 0usize;
        while topology_job.topology().is_none() {
            let result = topology_job.advance(quota);
            assert!(result.work_used <= quota && (result.done || result.work_used > 0));
            topology_work += result.work_used;
            assert!(topology_work < 2_000_000);
            let bytes = serde_json::to_vec(&topology_job.checkpoint()).unwrap();
            topology_job = crate::legacy::road_face_topology::TopologySearch::restore(serde_json::from_slice(&bytes).unwrap()).unwrap_or_else(|error| {
                let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                panic!("topology restore {error:?}: quota={quota} work={topology_work} phase={} node={} index={} compare={} step={}", value["phase"], value["sort_node"], value["sort_index"], value["sort_compare"], value["build_step"]);
            });
            topology_saves += 1;
        }
        if let Some(previous) = expected_topology_work {
            assert_eq!(topology_work, previous);
        }
        expected_topology_work = Some(topology_work);
        let topology = topology_job.topology().unwrap();
        let mut visited = vec![false; topology.half_edges.len()];
        let mut actual_faces = Vec::new();
        for start in 0..topology.half_edges.len() {
            if visited[start] {
                continue;
            }
            let mut current = start;
            let mut vertex_indices = Vec::new();
            let mut edge_ids = Vec::new();
            loop {
                assert!(!visited[current], "walk only revisits its starting edge");
                visited[current] = true;
                let half = &topology.half_edges[current];
                vertex_indices.push(half.from);
                edge_ids.push(edges[half.edge_index].id);
                current = topology.next[current];
                if current == start {
                    break;
                }
                assert!(vertex_indices.len() <= topology.half_edges.len());
            }
            let area = vertex_indices
                .iter()
                .enumerate()
                .map(|(i, &a)| {
                    let b = vertex_indices[(i + 1) % vertex_indices.len()];
                    nodes[a].x as f64 * nodes[b].z as f64 - nodes[b].x as f64 * nodes[a].z as f64
                })
                .sum::<f64>()
                * 0.5;
            if area <= 1.0e-3 {
                continue;
            }
            let mut ids = vertex_indices
                .iter()
                .map(|&i| nodes[i].id)
                .collect::<Vec<_>>();
            let mut points = vertex_indices
                .iter()
                .map(|&i| (nodes[i].x.to_bits(), nodes[i].z.to_bits()))
                .collect::<Vec<_>>();
            let first = ids.iter().enumerate().min_by_key(|(_, id)| *id).unwrap().0;
            ids.rotate_left(first);
            edge_ids.rotate_left(first);
            points.rotate_left(first);
            actual_faces.push((ids, edge_ids, points, area.to_bits()));
        }
        actual_faces.sort();
        let mut reference_faces = faces
            .iter()
            .map(|face| {
                (
                    face.vertex_ids.clone(),
                    face.boundary_edge_ids.clone(),
                    face.polygon
                        .iter()
                        .map(|&(x, z)| (x.to_bits(), z.to_bits()))
                        .collect::<Vec<_>>(),
                    face.area.to_bits(),
                )
            })
            .collect::<Vec<_>>();
        reference_faces.sort();
        assert_eq!(
            actual_faces, reference_faces,
            "mature block geometry from topology successors"
        );
        let mut walk_job = crate::legacy::road_face_walk::FaceWalkSearch::new(
            nodes.clone(),
            edges.clone(),
            topology.half_edges.to_vec(),
            topology.next.to_vec(),
            None,
        )
        .unwrap();
        let initial = serde_json::to_vec(&walk_job.checkpoint()).unwrap();
        walk_job = crate::legacy::road_face_walk::FaceWalkSearch::restore(
            serde_json::from_slice(&initial).unwrap(),
        )
        .unwrap();
        let mut walk_work = 0usize;
        let mut walk_saves = 1usize;
        let mut last_walk_save = 0usize;
        while walk_job.faces().is_none() {
            let result = walk_job.advance(quota);
            assert!(result.work_used <= quota && (result.done || result.work_used > 0));
            walk_work += result.work_used;
            assert!(walk_work < 5_000_000, "mature face walk terminates");
            if result.done || walk_work - last_walk_save >= 4_093 {
                let bytes = serde_json::to_vec(&walk_job.checkpoint()).unwrap();
                walk_job = crate::legacy::road_face_walk::FaceWalkSearch::restore(
                    serde_json::from_slice(&bytes).unwrap(),
                )
                .unwrap_or_else(|error| {
                    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    panic!(
                        "face walk restore {error:?}: quota={quota} work={walk_work} phase={}",
                        value["phase"]
                    );
                });
                walk_saves += 1;
                last_walk_save = walk_work;
            }
        }
        assert_eq!(
            walk_job.faces().unwrap().unwrap(),
            faces.as_slice(),
            "complete ordered RoadFace equality at quota {quota}"
        );
        if let Some(previous) = expected_walk_work {
            assert_eq!(walk_work, previous);
        }
        expected_walk_work = Some(walk_work);
        assert_eq!(walk_job.advance(quota).work_used, 0);
        runs.push(serde_json::json!({"quota": quota, "work_units": work, "calls": calls, "restores": restores + 1,
            "bridge_work_units": bridge_work, "bridge_restores": bridge_saves, "bridge_count": reference_bridges.len(), "topology_work_units": topology_work, "topology_restores": topology_saves, "traced_reference_faces": actual_faces.len(), "walk_work_units": walk_work, "walk_restores": walk_saves}));
    }
    let report = serde_json::json!({"scope": "native graph validation, bridge, topology and complete face-walk parity; not production discovery", "nodes": nodes.len(), "edges": edges.len(), "reference_faces": faces.len(), "runs": runs});
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../validation/road-face-preflight-mature.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "mature target-grid containment parity against the frozen face fixture"]
fn mature_target_containment_matches_reference() {
    use crate::legacy::face_target_containment::{PointContainment, ValidatedPolygon};
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/validation/face-target-reference.json"
    ))
    .unwrap();
    let faces: Vec<road_faces::RoadFace> =
        serde_json::from_value(reference["faces"].clone()).unwrap();
    assert_eq!(faces.len(), 49);
    let mut totals = Vec::new();
    for quota in [1, 7, 48] {
        let mut work = 0usize;
        let mut queries = 0usize;
        let mut restores = 0usize;
        for face in &faces {
            let polygon = ValidatedPolygon::new(face.polygon.clone()).unwrap();
            let min_x = face
                .polygon
                .iter()
                .map(|p| p.0)
                .fold(f32::INFINITY, f32::min);
            let max_x = face
                .polygon
                .iter()
                .map(|p| p.0)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_z = face
                .polygon
                .iter()
                .map(|p| p.1)
                .fold(f32::INFINITY, f32::min);
            let max_z = face
                .polygon
                .iter()
                .map(|p| p.1)
                .fold(f32::NEG_INFINITY, f32::max);
            let columns = (((max_x - min_x) / 8.0).ceil() as usize).clamp(1, 24);
            let rows = (((max_z - min_z) / 8.0).ceil() as usize).clamp(1, 24);
            for row in 0..rows {
                for column in 0..columns {
                    let point = (
                        min_x + (column as f32 + 0.5) * (max_x - min_x) / columns as f32,
                        min_z + (row as f32 + 0.5) * (max_z - min_z) / rows as f32,
                    );
                    let mut points = vec![point];
                    for rotation in [0.0, std::f32::consts::FRAC_PI_2] {
                        points.extend(
                            geometry::OrientedRect {
                                x: point.0,
                                z: point.1,
                                width: 8.0,
                                depth: 8.0,
                                rotation,
                            }
                            .corners(),
                        );
                    }
                    for candidate in points {
                        let expected = diagnostic_point_in_face(candidate, &face.polygon);
                        let mut query = PointContainment::new(candidate, polygon.clone()).unwrap();
                        assert_eq!(query.advance(0).work_used, 0);
                        for _ in 0..=face.polygon.len() {
                            let step = query.advance(quota);
                            assert!(step.work_used <= quota);
                            work += step.work_used;
                            // Sample all cursor phases across the mature grid, including terminal states.
                            if queries % 137 == 0 {
                                let bytes = serde_json::to_vec(&query.checkpoint()).unwrap();
                                query = PointContainment::restore(
                                    serde_json::from_slice(&bytes).unwrap(),
                                )
                                .unwrap();
                                restores += 1;
                            }
                            if step.done {
                                break;
                            }
                        }
                        assert_eq!(
                            query.result(),
                            Some(expected),
                            "face {:?}, point {:?}, quota {}",
                            face.identity,
                            candidate,
                            quota
                        );
                        assert_eq!(query.advance(quota).work_used, 0);
                        queries += 1;
                    }
                }
            }
        }
        eprintln!(
            "mature containment quota={quota} queries={queries} work={work} restores={restores}"
        );
        totals.push((queries, work));
    }
    assert!(totals.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
#[ignore = "composed road-face tracing on the frozen mature connected graph"]
fn mature_composed_face_trace_matches_reference() {
    use crate::legacy::road_face_trace::RoadFaceTrace;
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let roads = source
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .collect::<Vec<_>>();
    let used = roads
        .iter()
        .flat_map(|road| [road.from, road.to])
        .collect::<BTreeSet<_>>();
    let nodes = source
        .state
        .network
        .nodes
        .iter()
        .filter(|node| used.contains(&node.id))
        .map(|node| road_faces::RoadFaceNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();
    let edges = roads
        .iter()
        .map(|road| road_faces::RoadFaceEdge {
            id: road.id,
            from: road.from,
            to: road.to,
        })
        .collect::<Vec<_>>();
    let expected = road_faces::trace_faces(&nodes, &edges, None).unwrap();
    assert_eq!(expected.len(), 49);
    let mut previous_work = None;
    for quota in [1, 7, 48] {
        let mut job = RoadFaceTrace::new(nodes.clone(), edges.clone(), None).unwrap();
        let initial = serde_json::to_vec(&job.checkpoint()).unwrap();
        job = RoadFaceTrace::restore(serde_json::from_slice(&initial).unwrap()).unwrap();
        let mut work = 0usize;
        let mut since_save = 0usize;
        let mut saves = 1usize;
        let mut max_bytes = initial.len();
        assert_eq!(job.advance(0).work_used, 0);
        loop {
            let step = job.advance(quota);
            assert!(step.work_used <= quota);
            work += step.work_used;
            since_save += step.work_used;
            assert!(work < 2_000_000, "composed tracer must terminate");
            if step.done || since_save >= 997 {
                let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                max_bytes = max_bytes.max(bytes.len());
                job = RoadFaceTrace::restore(serde_json::from_slice(&bytes).unwrap())
                    .unwrap_or_else(|error| {
                        panic!("composed restore {error:?} quota={quota} work={work}")
                    });
                saves += 1;
                since_save = 0;
            }
            if step.done {
                break;
            }
            assert!(step.work_used > 0);
        }
        assert!(
            job.faces().unwrap().unwrap() == expected.as_slice(),
            "composed face output differs at quota{quota}"
        );
        assert_eq!(job.advance(quota).work_used, 0);
        if let Some(prior) = previous_work {
            assert_eq!(work, prior);
        }
        previous_work = Some(work);
        eprintln!("composed mature trace quota={quota} work={work} restores={saves} max_checkpoint_bytes={max_bytes}");
    }
}

pub(crate) fn frozen_target_viability_input(
    source: &Engine,
) -> crate::legacy::face_target_viability::FrozenViabilityInput {
    use crate::legacy::face_target_viability::{FrozenViabilityInput, Segment};
    let context = source.collision_context();
    let river = source.river_control_points();
    let reserved_lots = source
        .state
        .legacy
        .pending_interior_access
        .as_ref()
        .map(|plan| {
            plan.selected_lots
                .iter()
                .map(|lot| geometry::OrientedRect {
                    x: lot.x,
                    z: lot.z,
                    width: lot.width,
                    depth: lot.depth,
                    rotation: lot.rotation,
                })
                .collect()
        })
        .unwrap_or_default();
    let neighborhood_segments = source
        .state
        .legacy
        .neighborhood_plan
        .as_ref()
        .map(|plan| {
            plan.segments
                .iter()
                .skip(plan.next_segment)
                .filter_map(|&(from, to)| {
                    let (a, b) = (source.node(from)?, source.node(to)?);
                    Some(Segment {
                        ax: a.x,
                        az: a.z,
                        bx: b.x,
                        bz: b.z,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let neighborhood_interior = source
        .state
        .legacy
        .neighborhood_plan
        .as_ref()
        .and_then(|plan| {
            if plan.coordinated_lots && plan.segments.len() == 4 {
                source.coordinated_block_interior(&plan.segments)
            } else {
                None
            }
        });
    FrozenViabilityInput {
        parcel_footprints: context.parcel_footprints,
        reserved_lots,
        open_roads: context
            .open_roads
            .iter()
            .map(|s| Segment {
                ax: s.ax,
                az: s.az,
                bx: s.bx,
                bz: s.bz,
            })
            .collect(),
        pending_roads: context
            .pending_roads
            .iter()
            .map(|s| Segment {
                ax: s.ax,
                az: s.az,
                bx: s.bx,
                bz: s.bz,
            })
            .collect(),
        river_segments: river
            .windows(2)
            .map(|p| Segment {
                ax: p[0].x,
                az: p[0].z,
                bx: p[1].x,
                bz: p[1].z,
            })
            .collect(),
        neighborhood_segments,
        neighborhood_interior,
    }
}

#[test]
#[ignore = "frozen target viability parity against the mature engine predicates"]
fn mature_target_viability_matches_engine() {
    use crate::legacy::face_target_viability::{FrozenViabilityContext, ViabilityQuery};
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let original_context = source.collision_context();
    let frozen = FrozenViabilityContext::new(frozen_target_viability_input(&source)).unwrap();
    let context_bytes = serde_json::to_vec(&frozen.checkpoint()).unwrap();
    let frozen =
        FrozenViabilityContext::restore(serde_json::from_slice(&context_bytes).unwrap()).unwrap();
    eprintln!(
        "mature frozen viability context bytes={}",
        context_bytes.len()
    );
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/validation/face-target-reference.json"
    ))
    .unwrap();
    let faces: Vec<road_faces::RoadFace> =
        serde_json::from_value(reference["faces"].clone()).unwrap();
    let mut expected_work = None;
    for quota in [1, 7, 48] {
        let mut work = 0usize;
        let mut queries = 0usize;
        let mut passed = 0usize;
        for face in &faces {
            let min_x = face
                .polygon
                .iter()
                .map(|p| p.0)
                .fold(f32::INFINITY, f32::min);
            let max_x = face
                .polygon
                .iter()
                .map(|p| p.0)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_z = face
                .polygon
                .iter()
                .map(|p| p.1)
                .fold(f32::INFINITY, f32::min);
            let max_z = face
                .polygon
                .iter()
                .map(|p| p.1)
                .fold(f32::NEG_INFINITY, f32::max);
            let columns = (((max_x - min_x) / 8.0).ceil() as usize).clamp(1, 24);
            let rows = (((max_z - min_z) / 8.0).ceil() as usize).clamp(1, 24);
            for row in 0..rows {
                for column in 0..columns {
                    let x = min_x + (column as f32 + 0.5) * (max_x - min_x) / columns as f32;
                    let z = min_z + (row as f32 + 0.5) * (max_z - min_z) / rows as f32;
                    for rotation in [0.0, std::f32::consts::FRAC_PI_2] {
                        let rect = geometry::OrientedRect {
                            x,
                            z,
                            width: 8.0,
                            depth: 8.0,
                            rotation,
                        };
                        let expected =
                            source.progressive_frontage_geometry_clear(&rect, &original_context);
                        let mut query = ViabilityQuery::new(rect, frozen.clone()).unwrap();
                        let mut query_work = 0usize;
                        loop {
                            let step = query.advance(quota);
                            assert!(step.work_used <= quota);
                            work += step.work_used;
                            query_work += step.work_used;
                            assert!(query_work < 100_000, "viability terminates");
                            if queries % 997 == 0 {
                                let bytes = serde_json::to_vec(&query.checkpoint()).unwrap();
                                query = ViabilityQuery::restore(
                                    serde_json::from_slice(&bytes).unwrap(),
                                    frozen.clone(),
                                )
                                .unwrap();
                            }
                            if step.done {
                                break;
                            }
                            assert!(step.work_used > 0);
                        }
                        assert_eq!(
                            query.result(),
                            Some(expected),
                            "face {:?}, rect {:?},quota{quota}",
                            face.identity,
                            rect
                        );
                        if expected {
                            passed += 1;
                        }
                        queries += 1;
                    }
                }
            }
        }
        if let Some(prior) = expected_work {
            assert_eq!((queries, passed, work), prior);
        }
        expected_work = Some((queries, passed, work));
        eprintln!("mature viability quota={quota} queries={queries} accepted={passed} work={work}");
    }
}

#[test]
#[ignore = "complete face sampler parity against all mature frozen face centers"]
fn mature_face_sampling_matches_reference() {
    use crate::legacy::face_target_sampling::FaceSampler;
    use crate::legacy::face_target_viability::FrozenViabilityContext;
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let frozen = FrozenViabilityContext::new(frozen_target_viability_input(&source)).unwrap();
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/validation/face-target-reference.json"
    ))
    .unwrap();
    let faces: Vec<road_faces::RoadFace> =
        serde_json::from_value(reference["faces"].clone()).unwrap();
    let samples = reference["face_samples"].as_array().unwrap();
    let mut prior_work = None;
    for quota in [1, 7, 48] {
        let mut total_work = 0usize;
        let mut total_centers = 0usize;
        let mut saves = 0usize;
        let mut max_bytes = 0usize;
        for (face, expected) in faces.iter().zip(samples) {
            assert_eq!(
                serde_json::to_value(&face.identity).unwrap(),
                expected["identity"]
            );
            let mut job = FaceSampler::new(face.polygon.clone(), frozen.clone()).unwrap();
            let mut work = 0usize;
            let mut since_save = 0usize;
            assert_eq!(job.advance(0).work_used, 0);
            loop {
                let step = job.advance(quota);
                assert!(step.work_used <= quota);
                work += step.work_used;
                since_save += step.work_used;
                assert!(work < 20_000_000, "face sampling must terminate");
                if step.done || since_save >= 1021 {
                    let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                    max_bytes = max_bytes.max(bytes.len());
                    job = FaceSampler::restore(
                        serde_json::from_slice(&bytes).unwrap(),
                        frozen.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!(
                            "sampler restore {error:?} face={:?} quota={quota} work={work}",
                            face.identity
                        )
                    });
                    saves += 1;
                    since_save = 0;
                }
                if step.done {
                    break;
                }
                assert!(step.work_used > 0);
            }
            let centers = job.centers().unwrap();
            let bits = centers
                .iter()
                .map(|&(x, z)| [x.to_bits(), z.to_bits()])
                .collect::<Vec<_>>();
            assert_eq!(
                serde_json::to_value(bits).unwrap(),
                expected["center_bits"],
                "face {:?} quota{quota}",
                face.identity
            );
            let center_count = centers.len();
            assert_eq!(job.advance(quota).work_used, 0);
            total_work += work;
            total_centers += center_count;
        }
        if let Some(prior) = prior_work {
            assert_eq!(total_work, prior);
        }
        prior_work = Some(total_work);
        eprintln!("mature sampler quota={quota} faces={} centers={total_centers} work={total_work} restores={saves} max_checkpoint_bytes={max_bytes}",faces.len());
    }
}

#[test]
#[ignore = "mature winning-face clearance and target spacing parity"]
fn mature_target_ranking_matches_reference() {
    use crate::legacy::face_target_ranking::TargetRanking;
    use crate::legacy::face_target_viability::Segment;
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let context = source.collision_context();
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/validation/face-target-reference.json"
    ))
    .unwrap();
    let sample = reference["face_samples"]
        .as_array()
        .unwrap()
        .iter()
        .find(|sample| sample["identity"] == reference["selected_face_identity"])
        .unwrap();
    let bits: Vec<[u32; 2]> = serde_json::from_value(sample["center_bits"].clone()).unwrap();
    let centers = bits
        .iter()
        .map(|p| (f32::from_bits(p[0]), f32::from_bits(p[1])))
        .collect::<Vec<_>>();
    assert_eq!(centers.len(), 34);
    let roads = context
        .open_roads
        .iter()
        .map(|s| Segment {
            ax: s.ax,
            az: s.az,
            bx: s.bx,
            bz: s.bz,
        })
        .collect::<Vec<_>>();
    let nodes = source
        .state
        .network
        .nodes
        .iter()
        .map(|n| (n.x, n.z))
        .collect::<Vec<_>>();
    let mut previous_work = None;
    for quota in [1, 7, 48] {
        let mut job = TargetRanking::new(
            centers.clone(),
            context.parcel_footprints.clone(),
            roads.clone(),
            nodes.clone(),
        )
        .unwrap();
        let mut work = 0usize;
        let mut since_save = 0usize;
        let mut saves = 0usize;
        loop {
            let step = job.advance(quota);
            assert!(step.work_used <= quota);
            work += step.work_used;
            since_save += step.work_used;
            assert!(work < 2_000_000);
            if step.done || since_save >= 997 {
                let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                job = TargetRanking::restore(serde_json::from_slice(&bytes).unwrap())
                    .unwrap_or_else(|error| {
                        panic!("ranking restore {error:?} quota={quota} work={work}")
                    });
                since_save = 0;
                saves += 1;
            }
            if step.done {
                break;
            }
            assert!(step.work_used > 0);
        }
        let targets = job.targets().unwrap();
        let actual = targets
            .iter()
            .map(|&(x, z)| [x.to_bits(), z.to_bits()])
            .collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            reference["target_bits"]
        );
        assert_eq!(targets.len(), 6);
        if let Some(prior) = previous_work {
            assert_eq!(work, prior);
        }
        previous_work = Some(work);
        eprintln!("mature target ranking quota={quota} work={work} restores={saves}");
    }
}

#[test]
#[ignore = "streaming mature face selection matches frozen viable-face ranking"]
fn mature_face_selection_matches_reference() {
    use crate::legacy::face_target_selection::FaceSelection;
    use crate::legacy::face_target_viability::FrozenViabilityContext;
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let context = FrozenViabilityContext::new(frozen_target_viability_input(&source)).unwrap();
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/validation/face-target-reference.json"
    ))
    .unwrap();
    let faces: Vec<road_faces::RoadFace> =
        serde_json::from_value(reference["faces"].clone()).unwrap();
    let mut prior_work = None;
    for quota in [1, 7, 48] {
        let mut job = FaceSelection::new(faces.clone(), context.clone()).unwrap();
        let mut work = 0usize;
        let mut since_save = 0usize;
        let mut saves = 0usize;
        let mut max_bytes = 0usize;
        loop {
            let step = job.advance(quota);
            assert!(step.work_used <= quota);
            work += step.work_used;
            since_save += step.work_used;
            assert!(work < 20_000_000);
            if step.done || since_save >= 8191 {
                let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                max_bytes = max_bytes.max(bytes.len());
                job = FaceSelection::restore(
                    serde_json::from_slice(&bytes).unwrap(),
                    context.clone(),
                )
                .unwrap_or_else(|error| {
                    panic!("selection restore {error:?} quota={quota} work={work}")
                });
                since_save = 0;
                saves += 1;
            }
            if step.done {
                break;
            }
            assert!(step.work_used > 0);
        }
        let chosen = job.selection().unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&faces[chosen.face_index].identity).unwrap(),
            reference["selected_face_identity"]
        );
        let sample = reference["face_samples"]
            .as_array()
            .unwrap()
            .iter()
            .find(|sample| sample["identity"] == reference["selected_face_identity"])
            .unwrap();
        let bits = chosen
            .centers
            .iter()
            .map(|&(x, z)| [x.to_bits(), z.to_bits()])
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(bits).unwrap(), sample["center_bits"]);
        assert_eq!(chosen.centers.len(), 34);
        if let Some(prior) = prior_work {
            assert_eq!(work, prior);
        }
        prior_work = Some(work);
        eprintln!("mature selection quota={quota} work={work} restores={saves} max_checkpoint_bytes={max_bytes}");
    }
}

#[test]
#[ignore = "end-to-end frozen graph to exact access proposal through resumable stages"]
fn mature_incremental_pipeline_matches_exact_evaluator() {
    use crate::{
        access_descriptor_search::DescriptorSearch,
        face_target_ranking::TargetRanking,
        face_target_selection::FaceSelection,
        face_target_viability::{FrozenViabilityContext, Segment},
        road_face_trace::RoadFaceTrace,
    };
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let ordinary = source
        .state
        .network
        .roads
        .iter()
        .filter(|road| {
            road.stage == Stage::Open
                && road.connected
                && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .collect::<Vec<_>>();
    let used = ordinary
        .iter()
        .flat_map(|road| [road.from, road.to])
        .collect::<BTreeSet<_>>();
    let nodes = source
        .state
        .network
        .nodes
        .iter()
        .filter(|node| used.contains(&node.id))
        .map(|n| road_faces::RoadFaceNode {
            id: n.id,
            x: n.x,
            z: n.z,
        })
        .collect::<Vec<_>>();
    let edges = ordinary
        .iter()
        .map(|r| road_faces::RoadFaceEdge {
            id: r.id,
            from: r.from,
            to: r.to,
        })
        .collect::<Vec<_>>();
    let collision = source.collision_context();
    let context = FrozenViabilityContext::new(frozen_target_viability_input(&source)).unwrap();
    let reference = crate::tests::access_reference::load();
    let mut prior_work = None;
    let mut report = Vec::new();
    for quota in [7, 48, 4096] {
        let mut stage_work = Vec::new();
        let mut resumes = 0usize;
        // Setup and transfer copies in this integration harness are explicit
        // synchronous work, not hidden in the measured advance budget.
        macro_rules! drain {
            ($job:ident,$restore:expr,$label:literal) => {{
                let mut work = 0usize;
                let mut since_save = 0usize;
                loop {
                    let step = $job.advance(quota);
                    assert!(step.work_used <= quota);
                    work += step.work_used;
                    since_save += step.work_used;
                    assert!(work < 20_000_000, concat!($label, " terminates"));
                    if step.done || since_save >= 16381 {
                        let bytes = serde_json::to_vec(&$job.checkpoint()).unwrap();
                        $job = ($restore)(&bytes);
                        resumes += 1;
                        since_save = 0;
                    }
                    if step.done {
                        break;
                    }
                    assert!(step.work_used > 0);
                }
                stage_work.push(($label, work));
            }};
        }
        let mut trace = RoadFaceTrace::new(nodes.clone(), edges.clone(), None).unwrap();
        drain!(
            trace,
            |bytes: &[u8]| RoadFaceTrace::restore(serde_json::from_slice(bytes).unwrap()).unwrap(),
            "trace"
        );
        let faces = trace.faces().unwrap().unwrap().to_vec();
        let mut selection = FaceSelection::new(faces.clone(), context.clone()).unwrap();
        drain!(
            selection,
            |bytes: &[u8]| FaceSelection::restore(
                serde_json::from_slice(bytes).unwrap(),
                context.clone()
            )
            .unwrap(),
            "selection"
        );
        let selected = selection.selection().unwrap().unwrap();
        let face = &faces[selected.face_index];
        assert_eq!(face.identity, reference.face.identity);
        let mut ranking = TargetRanking::new(
            selected.centers.to_vec(),
            collision.parcel_footprints.clone(),
            collision
                .open_roads
                .iter()
                .map(|r| Segment {
                    ax: r.ax,
                    az: r.az,
                    bx: r.bx,
                    bz: r.bz,
                })
                .collect(),
            source
                .state
                .network
                .nodes
                .iter()
                .map(|n| (n.x, n.z))
                .collect(),
        )
        .unwrap();
        drain!(
            ranking,
            |bytes: &[u8]| TargetRanking::restore(serde_json::from_slice(bytes).unwrap()).unwrap(),
            "ranking"
        );
        let hosts = face
            .boundary_edge_ids
            .iter()
            .map(|id| {
                let road = ordinary.iter().find(|r| r.id == *id).unwrap();
                let a = source.node(road.from).unwrap();
                let b = source.node(road.to).unwrap();
                access_candidates::BoundaryHost {
                    road_id: road.id,
                    from: road.from,
                    to: road.to,
                    ax: a.x,
                    az: a.z,
                    bx: b.x,
                    bz: b.z,
                }
            })
            .collect::<Vec<_>>();
        let candidate_nodes = source
            .state
            .network
            .nodes
            .iter()
            .map(|n| access_candidates::CandidateNode {
                id: n.id,
                x: n.x,
                z: n.z,
            })
            .collect::<Vec<_>>();
        let mut descriptors = DescriptorSearch::from_owned(
            face.clone(),
            hosts,
            candidate_nodes,
            collision.parcel_footprints.clone(),
            ranking.targets().unwrap().to_vec(),
        )
        .unwrap();
        drain!(
            descriptors,
            |bytes: &[u8]| DescriptorSearch::restore(serde_json::from_slice(bytes).unwrap())
                .unwrap(),
            "descriptors"
        );
        let output = descriptors.descriptors().unwrap();
        let bits = output
            .iter()
            .map(|c| {
                [
                    c.host_id,
                    c.host_t.to_bits(),
                    c.end_x.to_bits(),
                    c.end_z.to_bits(),
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(bits, reference.ordered_bits);
        let mut proposals = Vec::new();
        for chunk in output.chunks(interior_access::MAX_PROPOSALS_PER_EVALUATION) {
            let mut page = chunk.to_vec();
            for (index, candidate) in page.iter_mut().enumerate() {
                candidate.tie_index = index as u8;
            }
            proposals.extend(
                source
                    .propose_interior_access_from_geometry_candidates(0, &page)
                    .proposals,
            );
        }
        // The host-edge split samples recover the one known legal route on this
        // fixture: road 3317, 1m inside its far end clearance, opening two lots.
        assert_eq!(
            proposals
                .iter()
                .map(|p| (p.host_id, p.selected_lots.len()))
                .collect::<Vec<_>>(),
            vec![(3317, 2)]
        );
        let total_work = stage_work.iter().map(|(_, work)| work).sum::<usize>();
        if let Some(prior) = prior_work {
            assert_eq!(total_work, prior);
        }
        prior_work = Some(total_work);
        report.push(serde_json::json!({"quota":quota,"stages":stage_work,"total_work":total_work,"resumes":resumes,"descriptors":output.len(),"exact_proposals":proposals.len()}));
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/incremental-access-end-to-end.json");
    std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"scope":"frozen native parity; synchronous input capture, transfers, constructors and exact evaluator remain outside advance accounting; no production scheduling claim","runs":report})).unwrap()).unwrap();
}

#[test]
#[ignore = "measure current synchronous frozen access input capture on mature fixture"]
fn mature_access_capture_cost() {
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let mut durations = Vec::new();
    let mut sizes = Vec::new();
    for _ in 0..32 {
        let start = std::time::Instant::now();
        let ordinary = source
            .state
            .network
            .roads
            .iter()
            .filter(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
            })
            .collect::<Vec<_>>();
        let used = ordinary
            .iter()
            .flat_map(|road| [road.from, road.to])
            .collect::<BTreeSet<_>>();
        let graph_nodes = source
            .state
            .network
            .nodes
            .iter()
            .filter(|node| used.contains(&node.id))
            .map(|n| road_faces::RoadFaceNode {
                id: n.id,
                x: n.x,
                z: n.z,
            })
            .collect::<Vec<_>>();
        let graph_edges = ordinary
            .iter()
            .map(|r| road_faces::RoadFaceEdge {
                id: r.id,
                from: r.from,
                to: r.to,
            })
            .collect::<Vec<_>>();
        let all_nodes = source
            .state
            .network
            .nodes
            .iter()
            .map(|n| access_candidates::CandidateNode {
                id: n.id,
                x: n.x,
                z: n.z,
            })
            .collect::<Vec<_>>();
        let viability = frozen_target_viability_input(&source);
        // Keep ownership through the timer; serialization is measured separately.
        let input = std::hint::black_box((graph_nodes, graph_edges, all_nodes, viability));
        durations.push(start.elapsed().as_nanos());
        let start = std::time::Instant::now();
        let encoded = serde_json::to_vec(&input).unwrap();
        sizes.push((encoded.len(), start.elapsed().as_nanos()));
    }
    durations.sort_unstable();
    let mut encoding_ns = sizes.iter().map(|(_, ns)| *ns).collect::<Vec<_>>();
    encoding_ns.sort_unstable();
    assert!(sizes.iter().all(|(size, _)| *size == sizes[0].0));
    let report = serde_json::json!({
        "scope":"native release only; current synchronous helper measured on one mature fixture, not worst-case caps, WASM, or browser timing",
        "seed":source.state.seed,"tick":source.state.tick,"buildings":source.state.construction.buildings.len(),
        "runs":32,"capture_median_ns":durations[16],"capture_p95_ns":durations[30],"capture_max_ns":durations[31],
        "json_bytes":sizes[0].0,"encode_median_ns":encoding_ns[16],"encode_p95_ns":encoding_ns[30],
        "decision":"measure browser/WASM and cap-scale capture before accepting a synchronous setup boundary; do not treat this sample as a tick-budget guarantee"
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/access-capture-native-profile.json");
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

#[test]
#[ignore = "single persisted discovery job reproduces mature ordered descriptors"]
fn mature_discovery_job_matches_reference() {
    use crate::legacy::access_discovery_job::{DiscoveryInput, DiscoveryJob};
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let ordinary = source
        .state
        .network
        .roads
        .iter()
        .filter(|r| {
            r.stage == Stage::Open
                && r.connected
                && matches!(r.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .collect::<Vec<_>>();
    let used = ordinary
        .iter()
        .flat_map(|r| [r.from, r.to])
        .collect::<BTreeSet<_>>();
    let input = DiscoveryInput {
        graph_nodes: source
            .state
            .network
            .nodes
            .iter()
            .filter(|n| used.contains(&n.id))
            .map(|n| road_faces::RoadFaceNode {
                id: n.id,
                x: n.x,
                z: n.z,
            })
            .collect(),
        graph_edges: ordinary
            .iter()
            .map(|r| road_faces::RoadFaceEdge {
                id: r.id,
                from: r.from,
                to: r.to,
            })
            .collect(),
        all_nodes: source
            .state
            .network
            .nodes
            .iter()
            .map(|n| access_candidates::CandidateNode {
                id: n.id,
                x: n.x,
                z: n.z,
            })
            .collect(),
        viability: frozen_target_viability_input(&source),
    };
    let captured = crate::legacy::access_capture::capture(&source).unwrap();
    assert_eq!(
        serde_json::to_value(&captured).unwrap(),
        serde_json::to_value(&input).unwrap()
    );
    let input = captured;
    let reference = crate::tests::access_reference::load();
    let mut prior_work = None;
    let mut runs = Vec::new();
    for quota in [7, 48, 4096] {
        let mut job = DiscoveryJob::new(input.clone()).unwrap();
        let mut work = 0usize;
        let mut since_save = 0usize;
        let mut saves = 0usize;
        let mut max_bytes = 0usize;
        assert_eq!(job.advance(0).work_used, 0);
        loop {
            let step = job.advance(quota);
            assert!(step.work_used <= quota);
            work += step.work_used;
            since_save += step.work_used;
            assert!(work < 25_000_000, "complete discovery must terminate");
            if step.done || since_save >= 16381 {
                let bytes = serde_json::to_vec(&job.checkpoint()).unwrap();
                max_bytes = max_bytes.max(bytes.len());
                job = DiscoveryJob::restore(serde_json::from_slice(&bytes).unwrap())
                    .unwrap_or_else(|e| {
                        panic!("discovery restore {e:?} quota={quota} work={work}")
                    });
                saves += 1;
                since_save = 0;
            }
            if step.done {
                break;
            }
            assert!(step.work_used > 0);
        }
        let candidates = job.descriptors().unwrap().unwrap();
        let bits = candidates
            .iter()
            .map(|c| {
                [
                    c.host_id,
                    c.host_t.to_bits(),
                    c.end_x.to_bits(),
                    c.end_z.to_bits(),
                ]
            })
            .collect::<Vec<_>>();
        assert!(
            bits == reference.ordered_bits,
            "mature discovery ordered descriptors differ quota{quota}"
        );
        assert_eq!(job.advance(quota).work_used, 0);
        if let Some(prior) = prior_work {
            assert_eq!(work, prior);
        }
        prior_work = Some(work);
        runs.push(serde_json::json!({"quota":quota,"work":work,"restores":saves,"max_checkpoint_bytes":max_bytes,"descriptors":bits.len()}));
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/discovery-job-mature.json");
    std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"scope":"persisted frozen native discovery, not live capture/admission or browser performance","runs":runs})).unwrap()).unwrap();
}

#[test]
#[ignore = "mature candidate validation parity across all discovered descriptors"]
fn mature_incremental_proposal_validation_matches_live_oracle() {
    use crate::legacy::access_proposal_validation::*;
    use crate::legacy::face_target_viability::Segment;
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let segment = |from, to| {
        source
            .node(from)
            .zip(source.node(to))
            .map(|(a, b)| Segment {
                ax: a.x,
                az: a.z,
                bx: b.x,
                bz: b.z,
            })
    };
    let mut hosts = source
        .state
        .network
        .roads
        .iter()
        .filter(|r| {
            r.stage == Stage::Open
                && r.connected
                && matches!(r.kind, RoadKind::Local | RoadKind::Arterial)
        })
        .filter_map(|r| {
            segment(r.from, r.to).map(|segment| ProposalHost {
                id: r.id,
                from: r.from,
                to: r.to,
                segment,
            })
        })
        .collect::<Vec<_>>();
    hosts.sort_by_key(|h| h.id);
    let input = ProposalValidationInput {
        hosts,
        nodes: source
            .state
            .network
            .nodes
            .iter()
            .map(|n| ProposalNode {
                id: n.id,
                x: n.x,
                z: n.z,
            })
            .collect(),
        roads: source
            .state
            .network
            .roads
            .iter()
            .map(|r| ProposalRoad {
                id: r.id,
                is_open: r.stage == Stage::Open,
                segment: segment(r.from, r.to),
            })
            .collect(),
        projects: source
            .state
            .construction
            .projects
            .iter()
            .map(|p| ProposalProject {
                active_road: p.kind == ProjectKind::Road && p.progress < 1.0,
                endpoints: match p.from.zip(p.to) {
                    None => ProjectEndpoints::NoIds,
                    Some((a, b)) => segment(a, b)
                        .map(ProjectEndpoints::Segment)
                        .unwrap_or(ProjectEndpoints::MissingNode),
                },
            })
            .collect(),
        viability: crate::legacy::access_capture::capture(&source)
            .unwrap()
            .viability,
        gates: AdmissionGates {
            demand: source.state.people.demand,
            has_pending_interior_plan: source.state.legacy.pending_interior_access.is_some(),
            has_neighborhood_plan: source.state.legacy.neighborhood_plan.is_some(),
            road_count: source.state.network.roads.len(),
            project_count: source.state.construction.projects.len(),
            parcel_count: source.state.land.parcels.len(),
            frontage_count: source.state.land.parcel_frontages.len(),
            next_id: source.state.next_id,
            project_admission_available: source.project_admission_available(),
            pending_access_branch: None,
        },
    };
    assert!(source.state.legacy.pending_interior_access.is_none());
    let captured = crate::legacy::access_proposal_capture::capture(&source).unwrap();
    assert_eq!(
        serde_json::to_value(&captured).unwrap(),
        serde_json::to_value(&input).unwrap()
    );
    let context = ProposalValidationContext::new(captured).unwrap();
    let reference = crate::tests::access_reference::load();
    let accepted_fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/validation/accepted-access-candidate.json"
    ))
    .unwrap();
    let regression_bits: [u32; 4] =
        serde_json::from_value(accepted_fixture["bits"].clone()).unwrap();
    let mut validation_bits = reference.ordered_bits.clone();
    validation_bits.push(regression_bits);
    let mut totals = Vec::new();
    for quota in [1usize, 7, 48] {
        let mut accepted = Vec::new();
        let mut work = 0usize;
        let mut restores = 0usize;
        for (i, bits) in validation_bits.iter().enumerate() {
            let candidate = interior_access::AccessGeometryCandidate {
                host_id: bits[0],
                host_t: f32::from_bits(bits[1]),
                end_x: f32::from_bits(bits[2]),
                end_z: f32::from_bits(bits[3]),
                tie_index: (i % 48) as u8,
            };
            let expected = source
                .propose_interior_access_from_geometry_candidates(0, &[candidate])
                .proposals;
            let mut query = ProposalValidator::new(context.clone(), candidate);
            assert_eq!(query.advance(0).work_used, 0);
            let mut since_save = 0;
            loop {
                let step = query.advance(quota);
                assert!(step.work_used <= quota);
                work += step.work_used;
                since_save += step.work_used;
                if step.done || since_save >= 4093 {
                    let cp = serde_json::to_string(&query.checkpoint()).unwrap();
                    query = ProposalValidator::restore(
                        serde_json::from_str(&cp).unwrap(),
                        context.clone(),
                    )
                    .unwrap_or_else(|e| {
                        panic!("candidate {i}, quota {quota}, restore {e:?}: {cp}")
                    });
                    restores += 1;
                    since_save = 0;
                }
                if step.done {
                    break;
                }
                assert!(step.work_used > 0);
            }
            let actual = query
                .outcome()
                .unwrap()
                .ok()
                .into_iter()
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "candidate {i}, quota {quota}");
            if i < reference.ordered_bits.len() {
                // Only the edge-sample descriptor matching the frozen legal
                // candidate is accepted on this fixture.
                assert_eq!(
                    actual.len(),
                    usize::from(*bits == regression_bits),
                    "candidate {i}"
                );
            } else {
                assert_eq!(
                    actual.len(),
                    1,
                    "independent legal candidate must still be accepted"
                );
            }
            accepted.extend(actual);
        }
        assert_eq!(accepted.len(), 2);
        assert!(accepted
            .iter()
            .all(|p| p.host_id == 3317 && p.selected_lots.len() == 2));
        assert_eq!(
            (
                accepted[0].host_t,
                accepted[0].end_x,
                accepted[0].end_z,
                &accepted[0].selected_lots
            ),
            (
                accepted[1].host_t,
                accepted[1].end_x,
                accepted[1].end_z,
                &accepted[1].selected_lots
            )
        );
        totals.push(serde_json::json!({"quota":quota,"work":work,"restores":restores,"accepted":accepted.len(),"descriptor_accepted":1,"independent_regression_accepted":1,"proposals":accepted}));
    }
    assert!(totals.windows(2).all(|w| w[0]["work"] == w[1]["work"]));
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../validation/incremental-proposal-validation.json"),
        serde_json::to_vec_pretty(&totals).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "composed discovery to ranked proposal pages on mature city"]
fn mature_access_planning_job_matches_ranked_pages() {
    use crate::legacy::access_planning_job::AccessPlanningJob;
    let source = Engine::restore_json(include_str!(
        "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
    ))
    .unwrap();
    let reference = crate::tests::access_reference::load();
    let mut expected = Vec::new();
    for rows in reference.ordered_bits.chunks(48) {
        let descriptors = rows
            .iter()
            .enumerate()
            .map(|(i, b)| interior_access::AccessGeometryCandidate {
                host_id: b[0],
                host_t: f32::from_bits(b[1]),
                end_x: f32::from_bits(b[2]),
                end_z: f32::from_bits(b[3]),
                tie_index: i as u8,
            })
            .collect::<Vec<_>>();
        let mut proposals = source
            .propose_interior_access_from_geometry_candidates(0, &descriptors)
            .proposals;
        proposals.sort_by(|a, b| b.score.compare(a.score));
        expected.push(proposals);
    }
    let mut reports = Vec::new();
    for quota in [48, 4096] {
        let mut job = AccessPlanningJob::new(
            crate::legacy::access_capture::capture(&source).unwrap(),
            crate::legacy::access_proposal_capture::capture(&source).unwrap(),
            source.access_geometry_revision_token().unwrap(),
        )
        .unwrap();
        let mut work = 0usize;
        let mut since_save = 0;
        let mut restores = 0;
        let mut pages = Vec::new();
        assert_eq!(job.advance(0).work_used, 0);
        loop {
            let step = job.advance(quota);
            assert!(step.work_used <= quota);
            work += step.work_used;
            since_save += step.work_used;
            if since_save >= 100_003 || step.page_ready || step.exhausted {
                let checkpoint = serde_json::to_string(&job.checkpoint()).unwrap();
                job =
                    AccessPlanningJob::restore(serde_json::from_str(&checkpoint).unwrap()).unwrap();
                restores += 1;
                since_save = 0;
            }
            if step.page_ready {
                pages.push(job.ready_page().unwrap().to_vec());
                if !job.continue_next_page().unwrap() {
                    break;
                }
            } else if step.exhausted {
                break;
            } else {
                assert!(step.work_used > 0);
            }
            assert!(work < 20_000_000);
        }
        assert_eq!(pages, expected, "quota {quota}");
        assert!(job.is_exhausted());
        reports.push(serde_json::json!({"quota":quota,"work":work,"restores":restores,"pages":pages.len(),"accepted":pages.iter().map(Vec::len).sum::<usize>()}));
    }
    assert_eq!(reports[0]["work"], reports[1]["work"]);
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../validation/access-planning-mature.json"),
        serde_json::to_vec_pretty(&reports).unwrap(),
    )
    .unwrap();
}
