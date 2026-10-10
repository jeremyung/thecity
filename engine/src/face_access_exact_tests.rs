use crate::*;

/// Exact engine-side gate audit for the raster diagnostic's leading candidate.
/// This mutates only an in-memory restored copy and is ignored in normal runs.
#[test]
#[ignore = "exact read-only candidate gate audit for the tick-242000 checkpoint"]
fn audit_largest_face_direct_access_candidate() {
    use serde_json::json;

    const SAVE: &str = include_str!("../../validation/early-reservation-wasm-87231-save.json");
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
            kind: "junction".into(),
        },
    );
    let endpoint_spacing = split_probe
        .state
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
        "endpoint_degree": split_probe.ordinary_road_degree(split_probe.state.roads.iter().find(|r|r.id==3317).unwrap().to),
    });

    // Check the straight chord against every existing road. The two split host
    // halves meet this branch at its start; all other edges retain the ordinary
    // eight-metre centerline separation rule.
    let road_separations_passed = split_probe.state.roads.iter().all(|road| {
        if road.from == split.split_node_id || road.to == split.split_node_id {
            return true;
        }
        let (Some(a), Some(b)) = (split_probe.node(road.from), split_probe.node(road.to)) else {
            return false;
        };
        geometry::segments_distance_coords(
            split_node.x, split_node.z, end.x, end.z, a.x, a.z, b.x, b.z,
        ) >= 8.0
    });
    let bend = WorldPoint { x: 229.0, z: 1.0 };
    let dogleg_legs = [(split_node.x, split_node.z, bend.x, bend.z), (bend.x, bend.z, 207.0, 1.0)];
    let dogleg_clearance = dogleg_legs.iter().map(|(ax,az,bx,bz)| json!({
        "length_m":(bx-ax).hypot(bz-az),
        "river":split_probe.ordinary_road_clear_of_river(*ax,*az,*bx,*bz,4.5),
        "protected":!corridor_crosses_protected(*ax,*az,*bx,*bz),
        "parcel_corridor":split_probe.road_corridor_clear_of_parcels_with_context(*ax,*az,*bx,*bz,&context),
        "nearest_existing_node_m":split_probe.state.nodes.iter().map(|n|(n.x-(*bx)).hypot(n.z-(*bz))).fold(f32::INFINITY,f32::min),
    })).collect::<Vec<_>>();
    let mut dogleg_context=context.clone();
    for (ax,az,bx,bz) in dogleg_legs {dogleg_context.open_roads.push(CollisionSegment{ax,az,bx,bz});}
    let dogleg_lots=[
        geometry::OrientedRect{x:224.0,z:13.0,width:8.0,depth:8.0,rotation:std::f32::consts::PI},
        geometry::OrientedRect{x:214.0,z:13.0,width:8.0,depth:8.0,rotation:std::f32::consts::PI},
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
            let oriented_ok = split_probe.oriented_parcel_geometry_clear_with_context(&rect, &proposed_context);
            let center_ok = split_probe.progressive_frontage_center_clear(&rect, &proposed_context);
            let clear = oriented_ok && center_ok;
            let nearest_parcel = context.parcel_footprints.iter().enumerate()
                .map(|(index, footprint)| (index, geometry::rect_segment_distance(footprint, split_node.x, split_node.z, end.x, end.z)))
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
                    &Node { id: end_node_id_for_test(), x: end.x, z: end.z, kind: "junction".into() },
                    along / branch_length,
                ),
            }));
        }
    }
    let viable_lots = proposed.iter().filter(|lot| lot["terrain_and_full_parcel_admission_passed"] == true).count();
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
    const SAVE: &str = include_str!("../../validation/early-reservation-wasm-87231-save.json");
    let mut engine = Engine::restore_json(SAVE).expect("mature checkpoint restores");
    let host_t = 0.72;
    let end = WorldPoint { x: 181.0, z: 15.0 };
    let host_a = engine.node(engine.state.roads.iter().find(|r| r.id == 3317).unwrap().from).unwrap().clone();
    let host_b = engine.node(engine.state.roads.iter().find(|r| r.id == 3317).unwrap().to).unwrap().clone();
    let branch_length = (end.x - (host_a.x + (host_b.x-host_a.x)*host_t))
        .hypot(end.z - (host_a.z + (host_b.z-host_a.z)*host_t));
    let intervals = frontage::subdivide_frontage(
        branch_length, &[], FRONTAGE_ENDPOINT_CLEARANCE, 8.0, 14.0, 2.2,
        MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
    );

    // The saved candidate was discovered by a bounded endpoint scan. Rebuild
    // its lots with the production branch-tangent/perpendicular convention.
    let start = WorldPoint {
        x: host_a.x + (host_b.x-host_a.x)*host_t,
        z: host_a.z + (host_b.z-host_a.z)*host_t,
    };
    let dx = end.x-start.x;
    let dz = end.z-start.z;
    let length = dx.hypot(dz);
    let nx = -dz/length;
    let nz = dx/length;
    let mut proposal_context = engine.collision_context();
    proposal_context.open_roads.push(CollisionSegment { ax:start.x, az:start.z, bx:end.x, bz:end.z });
    let mut lots = Vec::new();
    for side in [-1_i8, 1] {
        for interval in &intervals {
            let along_meters=(interval.start+interval.end)*0.5;
            let lot=interior_access::InteriorAccessLot {
                x:start.x+dx*(along_meters/length)+nx*side as f32*15.0,
                z:start.z+dz*(along_meters/length)+nz*side as f32*15.0,
                width:interval.end-interval.start,
                depth:12.0,
                rotation:(side as f32*dz).atan2(-side as f32*dx),
                along:along_meters/length,
                side,
            };
            let rect=geometry::OrientedRect{x:lot.x,z:lot.z,width:lot.width,depth:lot.depth,rotation:lot.rotation};
            if engine.progressive_frontage_geometry_clear(&rect,&proposal_context) {
                lots.push(lot);
            }
        }
    }
    assert!(branch_length >= 24.0 && branch_length <= 60.0, "candidate branch length {branch_length}");
    assert!(lots.len() >= 2, "candidate must reserve at least two exact-clear lots");
    let parcels_before = engine.state.parcels.len();
    let lot_count = lots.len();
    assert!(engine.admit_interior_access(3317, host_t, end, lots));
    engine.validate().expect("generalized geometry validation accepts this saved plan");
    assert_eq!(engine.state.parcels.len(), parcels_before);
    (engine, lot_count)
}

#[test]
fn geometry_valid_noncanonical_interior_access_plan_restores_and_completes() {
    let (mut engine, lot_count) = generalized_access_candidate_engine();
    let parcels_before = engine.state.parcels.len();
    let canonical=engine.save_json();
    let mut resumed=Engine::restore_json(&canonical).expect("candidate pending plan restores");
    assert_eq!(resumed.save_json(),canonical);
    let branch_id=engine.state.pending_interior_access.as_ref().unwrap().branch_road_id;
    let plan=engine.state.pending_interior_access.as_ref().unwrap().clone();

    let mut near_endpoint=Engine::restore_json(&canonical).unwrap();
    near_endpoint.state.pending_interior_access.as_mut().unwrap().host_t=0.01;
    assert!(Engine::restore_json(&near_endpoint.save_json()).is_err());

    let mut acute=Engine::restore_json(&canonical).unwrap();
    let acute_plan=acute.state.pending_interior_access.as_ref().unwrap().clone();
    let retained=acute.state.roads.iter().find(|r|r.id==acute_plan.host_road_id).unwrap().clone();
    let second=acute.state.roads.iter().find(|r|r.id==acute_plan.host_second_half_road_id).unwrap().clone();
    let host_a=acute.node(retained.from).unwrap().clone();
    let host_b=acute.node(second.to).unwrap().clone();
    let split=acute.node(acute_plan.split_node_id).unwrap().clone();
    let ux=(host_b.x-host_a.x).hypot(host_b.z-host_a.z);
    let dx=(host_b.x-host_a.x)/ux;
    let dz=(host_b.z-host_a.z)/ux;
    let branch_length=24.0;
    let endpoint=acute.state.nodes.iter_mut().find(|n|n.id==acute_plan.branch_endpoint_id).unwrap();
    endpoint.x=split.x+dx*branch_length;
    endpoint.z=split.z+dz*branch_length;
    acute.state.roads.iter_mut().find(|r|r.id==acute_plan.branch_road_id).unwrap().length=branch_length;
    assert!(acute.validate().unwrap_err().contains("junction angle"));

    let mut nonfinite=Engine::restore_json(&canonical).unwrap();
    nonfinite.state.pending_interior_access.as_mut().unwrap().selected_lots[0].rotation=f32::NAN;
    assert!(nonfinite.validate().is_err());

    for instance in [&mut engine,&mut resumed] {
        let project=instance.state.projects.iter_mut().find(|p|p.id==plan.project_id).unwrap();
        project.duration=1.0;
        instance.advance_projects();
        instance.validate().expect("completed noncanonical plan remains valid");
    }
    assert_eq!(engine.save_json(),resumed.save_json(),"completion and replay agree");
    assert!(engine.state.pending_interior_access.is_none());
    assert!(engine.state.roads.iter().any(|r|r.id==branch_id && r.stage=="open" && r.connected));
    assert_eq!(engine.state.parcels.len(), parcels_before + lot_count);
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
        if (az > point.1) != (bz > point.1)
            && point.0 < (bx - ax) * (point.1 - az) / (bz - az) + ax
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
#[ignore = "known limitation: current straight-branch candidates do not serve the largest mature gap"]
fn geometry_candidates_probe_largest_mature_gap_single_branch_limit() {
    const SAVE: &str = include_str!("../../validation/early-reservation-wasm-87231-save.json");
    let source = Engine::restore_json(SAVE).expect("current mature checkpoint restores");
    let source_before = source.save_json();
    let ordinary = source
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
        })
        .collect::<Vec<_>>();
    let used_nodes = ordinary
        .iter()
        .flat_map(|road| [road.from, road.to])
        .collect::<BTreeSet<_>>();
    let nodes = source
        .state
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
                .chain(context.open_roads.iter().map(|road| point_segment_distance(point.0, point.1, road.ax, road.az, road.bx, road.bz)))
                .fold(f32::INFINITY, f32::min);
        scored_targets.push((clearance, point.0, point.1));
    }
    scored_targets.sort_by(|left, right| right.0.total_cmp(&left.0).then_with(|| left.1.total_cmp(&right.1)).then_with(|| left.2.total_cmp(&right.2)));
    let mut unserved_targets = Vec::new();
    for (_, x, z) in scored_targets {
        let far_from_nodes = source.state.nodes.iter().all(|node| {
            (node.x - x).hypot(node.z - z) >= 20.0
        });
        let far_from_targets = unserved_targets.iter().all(|(old_x, old_z)| {
            (old_x - x).hypot(old_z - z) >= 18.0
        });
        if far_from_nodes && far_from_targets {
            unserved_targets.push((x,z));
            if unserved_targets.len() == 12 { break; }
        }
    }
    let candidate_nodes = source
        .state
        .nodes
        .iter()
        .map(|node| access_candidates::CandidateNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();
    let candidates = access_candidates::candidates_for_face(
        &target_face,
        &target_hosts,
        &candidate_nodes,
        &context.parcel_footprints,
        &unserved_targets,
    );
    assert!(candidates.len() <= interior_access::MAX_PROPOSALS_PER_EVALUATION);
    let scan = source.propose_interior_access_from_geometry_candidates(0, &candidates);
    let proposal_count = scan.proposals.len();
    let mut feasible = scan
        .proposals
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
    assert_eq!(source.save_json(), source_before, "candidate enumeration remains read-only");
    let Some(proposal) = feasible.first().cloned() else {
        eprintln!(
            "no admissible straight-branch candidate: viable_sites={viable_sites}, faces={:?}, hosts={}, proposals={}, rejection_counts={:?}",
            target_face.identity,
            target_hosts.len(),
            proposal_count,
            scan.rejection_counts,
        );
        return;
    };

    let mut admitted = Engine::restore_json(SAVE).expect("fresh copy for atomic admission");
    let before_save = admitted.save_json();
    assert!(admitted.admit_interior_access(
        proposal.host_id,
        proposal.host_t,
        WorldPoint { x: proposal.end_x, z: proposal.end_z },
        proposal.selected_lots.clone(),
    ));
    admitted.validate().expect("geometry-discovered plan fully validates");
    let pending = admitted.state.pending_interior_access.as_ref().unwrap();
    assert_eq!(pending.selected_lots.len(), proposal.selected_lots.len());
    assert_eq!(Engine::restore_json(&admitted.save_json()).unwrap().save_json(), admitted.save_json());
    assert_eq!(source.save_json(), before_save, "enumeration and evaluation are read-only");
    println!(
        "geometry access candidate admitted: viable_sites={viable_sites} face={:?} hosts={} candidates={} host={} t={} endpoint=({}, {}) lots={} area_m2={:.2}",
        target_face.identity,
        target_hosts.len(),
        candidates.len(),
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
    std::fs::write(root.join("generalized-access-pending-snapshot.json"), &snapshot).unwrap();
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
        "pending_plan": engine.state.pending_interior_access,
        "save_file": "generalized-access-pending-save.json",
        "snapshot_file": "generalized-access-pending-snapshot.json"
    });
    std::fs::write(root.join("generalized-access-pending-provenance.json"), serde_json::to_vec_pretty(&provenance).unwrap()).unwrap();
    println!("exported pending access fixture at tick {} with {} lots",engine.state.tick,engine.state.pending_interior_access.as_ref().unwrap().selected_lots.len());
}

#[test]
#[ignore = "bounded exact-engine scan of viable access proposals inside the largest face"]
fn scan_exact_planner_for_access_into_largest_face() {
    use serde_json::json;
    use std::collections::BTreeSet;

    const SAVE: &str = include_str!("../../validation/early-reservation-wasm-87231-save.json");
    let engine = Engine::restore_json(SAVE).expect("matching checkpoint restores");
    let before = engine.save_json();
    let ordinary: Vec<_> = engine
        .state
        .roads
        .iter()
        .filter(|r| r.stage == "open" && r.connected && matches!(r.kind.as_str(), "local" | "arterial"))
        .collect();
    let used_nodes: BTreeSet<u32> = ordinary.iter().flat_map(|r| [r.from, r.to]).collect();
    let face_nodes: Vec<_> = engine.state.nodes.iter().filter(|n| used_nodes.contains(&n.id))
        .map(|n| road_faces::RoadFaceNode { id:n.id, x:n.x, z:n.z }).collect();
    let face_edges: Vec<_> = ordinary.iter().map(|r| road_faces::RoadFaceEdge { id:r.id, from:r.from, to:r.to }).collect();
    let faces = road_faces::trace_faces(&face_nodes, &face_edges, None).expect("checkpoint ordinary graph is planar");
    let target = (223.53677_f32, -6.73742_f32);
    let point_in_poly = |x:f32,z:f32,polygon:&[(f32,f32)]| {
        let mut inside = false;
        for i in 0..polygon.len() {
            let (ax,az)=polygon[i]; let (bx,bz)=polygon[(i+polygon.len()-1)%polygon.len()];
            if (az > z) != (bz > z) && x < (bx-ax)*(z-az)/(bz-az)+ax { inside = !inside; }
        }
        inside
    };
    let face = faces.iter().find(|face| point_in_poly(target.0,target.1,&face.polygon))
        .expect("largest free component lies in a traced bounded face");
    let path_inside = |ax:f32,az:f32,bx:f32,bz:f32| {
        let length=(bx-ax).hypot(bz-az);
        let steps=(length/0.5).ceil().max(1.0) as usize;
        (1..=steps).all(|step| {
            let t=step as f32/steps as f32;
            point_in_poly(ax+(bx-ax)*t,az+(bz-az)*t,&face.polygon)
        })
    };
    let lot_inside = |lot:&interior_access::InteriorAccessLot| {
        let rect=geometry::OrientedRect{x:lot.x,z:lot.z,width:lot.width,depth:lot.depth,rotation:lot.rotation};
        let corners=rect.corners();
        if !point_in_poly(rect.x,rect.z,&face.polygon) || corners.iter().any(|p| !point_in_poly(p.0,p.1,&face.polygon)) { return false; }
        (0..4).all(|i| path_inside(corners[i].0,corners[i].1,corners[(i+1)%4].0,corners[(i+1)%4].1))
    };

    let proposal_count=ordinary.len().saturating_mul(interior_access::PROPOSALS_PER_HOST);
    let mut exact_proposals=0usize;
    let mut branch_in_face=0usize;
    let mut lots_in_face=0usize;
    let mut winners=Vec::new();
    for cursor in (0..proposal_count).step_by(interior_access::MAX_PROPOSALS_PER_EVALUATION) {
        let scan=engine.propose_interior_access_from(None,cursor);
        exact_proposals += scan.proposals.len();
        for proposal in scan.proposals {
            let host=engine.state.roads.iter().find(|r|r.id==proposal.host_id).unwrap();
            let a=engine.node(host.from).unwrap(); let b=engine.node(host.to).unwrap();
            let sx=a.x+(b.x-a.x)*proposal.host_t; let sz=a.z+(b.z-a.z)*proposal.host_t;
            if !path_inside(sx,sz,proposal.end_x,proposal.end_z) { continue; }
            branch_in_face+=1;
            let valid_lots: Vec<_>=proposal.selected_lots.iter().copied().filter(lot_inside).collect();
            if valid_lots.len()<2 { continue; }
            lots_in_face+=1;
            let mut split_probe=Engine::restore_json(SAVE).unwrap();
            let split=match split_probe.try_split_open_road(proposal.host_id,proposal.host_t) {
                Ok(split)=>split,
                Err(_)=>continue,
            };
            let split_node=split_probe.node(split.split_node_id).unwrap().clone();
            let end=WorldPoint{x:proposal.end_x,z:proposal.end_z};
            let branch_context=split_probe.collision_context();
            let branch_gates=json!({
                "bounds": end.x>=MIN+24.0&&end.x<=MAX-24.0&&end.z>=MIN+24.0&&end.z<=MAX-24.0,
                "river": split_probe.ordinary_road_clear_of_river(split_node.x,split_node.z,end.x,end.z,4.5),
                "protected": !corridor_crosses_protected(split_node.x,split_node.z,end.x,end.z),
                "parcels": split_probe.road_corridor_clear_of_parcels_with_context(split_node.x,split_node.z,end.x,end.z,&branch_context),
                "endpoint_spacing_m": split_probe.state.nodes.iter().map(|n|(n.x-end.x).hypot(n.z-end.z)).fold(f32::INFINITY,f32::min),
                "junction_heading": split_probe.ordinary_heading_clear(split.split_node_id,(end.z-split_node.z).atan2(end.x-split_node.x)),
            });
            let branch_context_ok=branch_gates["bounds"]==true&&branch_gates["river"]==true&&branch_gates["protected"]==true&&branch_gates["parcels"]==true&&branch_gates["endpoint_spacing_m"].as_f64().unwrap_or(0.0)>=18.0&&branch_gates["junction_heading"]==true;
            let mut proposed_context=branch_context.clone();
            proposed_context.open_roads.push(CollisionSegment{ax:split_node.x,az:split_node.z,bx:end.x,bz:end.z});
            let lots_pass=valid_lots.iter().all(|lot| {
                lot.width>=8.0 && lot.depth>=8.0 && {
                    let rect=geometry::OrientedRect{x:lot.x,z:lot.z,width:lot.width,depth:lot.depth,rotation:lot.rotation};
                    split_probe.progressive_frontage_geometry_clear(&rect,&proposed_context)
                }
            });
            if !branch_context_ok || !lots_pass { continue; }
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
    assert_eq!(engine.save_json(),before,"proposal enumeration is read-only");
    winners.sort_by(|a,b| {
        let score=|v:&serde_json::Value| v["site_area_m2"].as_f64().unwrap_or(0.0)/v["branch_length_m"].as_f64().unwrap_or(1.0);
        score(b).total_cmp(&score(a))
    });
    let report=json!({
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
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation/face-access-exact-planner-scan.json");
    std::fs::write(path,serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{}",report);

    // The prior JS route had a dogleg. Search a deterministic 2m endpoint grid
    // for a single straight branch from the same exact-preflighted host split.
    // This is an offline test probe, not a production candidate generator.
    let mut split_probe=Engine::restore_json(SAVE).unwrap();
    let split=split_probe.try_split_open_road(3317,0.72).expect("host split preflight");
    let start=split_probe.node(split.split_node_id).unwrap().clone();
    let context=split_probe.collision_context();
    let min_x=face.polygon.iter().map(|p|p.0).fold(f32::INFINITY,f32::min);
    let max_x=face.polygon.iter().map(|p|p.0).fold(f32::NEG_INFINITY,f32::max);
    let min_z=face.polygon.iter().map(|p|p.1).fold(f32::INFINITY,f32::min);
    let max_z=face.polygon.iter().map(|p|p.1).fold(f32::NEG_INFINITY,f32::max);
    let mut rejected=BTreeMap::<&'static str,usize>::new();
    let mut straight_winners=Vec::new();
    let mut typed_straight_winner:Option<(f32,WorldPoint,Vec<interior_access::InteriorAccessLot>)>=None;
    let mut checked=0usize;
    let mut endpoint_x=(min_x/2.0).floor()*2.0+1.0;
    while endpoint_x<=max_x {
        let mut endpoint_z=(min_z/2.0).floor()*2.0+1.0;
        while endpoint_z<=max_z {
            let sample_z=endpoint_z;
            endpoint_z+=2.0;
            let dx=endpoint_x-start.x; let dz=sample_z-start.z; let length=dx.hypot(dz);
            if !(26.0..=60.0).contains(&length) || !path_inside(start.x,start.z,endpoint_x,sample_z) { continue; }
            checked+=1;
            let reason=if endpoint_x<MIN+24.0||endpoint_x>MAX-24.0||sample_z<MIN+24.0||sample_z>MAX-24.0 {Some("bounds")}
                else if split_probe.state.nodes.iter().any(|n|(n.x-endpoint_x).hypot(n.z-sample_z)<18.0){Some("junction_spacing")}
                else if !split_probe.ordinary_heading_clear(split.split_node_id,dz.atan2(dx)){Some("junction_angle")}
                else if !split_probe.ordinary_road_clear_of_river(start.x,start.z,endpoint_x,sample_z,4.5){Some("river")}
                else if corridor_crosses_protected(start.x,start.z,endpoint_x,sample_z){Some("protected")}
                else if !split_probe.road_corridor_clear_of_parcels_with_context(start.x,start.z,endpoint_x,sample_z,&context){Some("parcel_corridor")}
                else if split_probe.state.roads.iter().any(|road|{
                    if road.from==split.split_node_id||road.to==split.split_node_id {return false;}
                    let (Some(a),Some(b))=(split_probe.node(road.from),split_probe.node(road.to)) else{return true;};
                    geometry::segments_distance_coords(start.x,start.z,endpoint_x,sample_z,a.x,a.z,b.x,b.z)<8.0
                }){Some("road_spacing")}
                else{None};
            if let Some(reason)=reason { *rejected.entry(reason).or_default()+=1; continue; }
            let mut proposed_context=context.clone();
            proposed_context.open_roads.push(CollisionSegment{ax:start.x,az:start.z,bx:endpoint_x,bz:sample_z});
            let intervals=frontage::subdivide_frontage(length,&[],FRONTAGE_ENDPOINT_CLEARANCE,8.0,14.0,2.2,MAX_FRONTAGE_GAP_LOTS_PER_SIDE);
            let nx=-dz/length; let nz=dx/length;
            let mut lots=Vec::new();
            for side in [-1_i8,1] { for interval in &intervals {
                let along=(interval.start+interval.end)*0.5;
                let rect=geometry::OrientedRect{
                    // Use the perpendicular offset implied by frontage rotation
                    // (local +Z points from the lot toward the road).
                    x:start.x+dx*(along/length)+nx*side as f32*15.0,
                    z:start.z+dz*(along/length)+nz*side as f32*15.0,
                    width:interval.end-interval.start,depth:12.0,
                    rotation:(side as f32*dz).atan2(-side as f32*dx),
                };
                let corners=rect.corners();
                let in_face=point_in_poly(rect.x,rect.z,&face.polygon)&&corners.iter().all(|p|point_in_poly(p.0,p.1,&face.polygon));
                if in_face && split_probe.progressive_frontage_geometry_clear(&rect,&proposed_context) { lots.push(rect); }
            }}
            if lots.len()<2 { *rejected.entry("fewer_than_two_exact_lots").or_default()+=1; continue; }
            if typed_straight_winner.is_none() {
                let ux=dx/length;let uz=dz/length;let lot_nx=-uz;let lot_nz=ux;
                let typed=lots.iter().map(|rect| {
                    let along=((rect.x-start.x)*ux+(rect.z-start.z)*uz)/length;
                    let side=((((rect.x-(start.x+ux*along*length))*lot_nx
                        +(rect.z-(start.z+uz*along*length))*lot_nz)/15.0).round() as i8).clamp(-1,1);
                    interior_access::InteriorAccessLot{
                        x:rect.x,z:rect.z,width:rect.width,depth:rect.depth,rotation:rect.rotation,along,side,
                    }
                }).collect();
                typed_straight_winner=Some((0.72,WorldPoint{x:endpoint_x,z:sample_z},typed));
            }
            straight_winners.push(json!({
                "host_road_id":3317,"host_t":0.72,"start_x":start.x,"start_z":start.z,
                "end_x":endpoint_x,"end_z":sample_z,"length_m":length,
                "site_area_m2":lots.iter().map(|r|r.width*r.depth).sum::<f32>(),
                "lot_rectangles":lots.iter().map(|r|json!({"x":r.x,"z":r.z,"width":r.width,"depth":r.depth,"rotation":r.rotation})).collect::<Vec<_>>(),
            }));
        }
        endpoint_x+=2.0;
    }
    straight_winners.sort_by(|a,b| {
        let score=|v:&serde_json::Value|v["site_area_m2"].as_f64().unwrap_or(0.0)/v["length_m"].as_f64().unwrap_or(1.0);
        score(b).total_cmp(&score(a))
    });
    let lifecycle=typed_straight_winner.map(|(host_t,end,lots)| {
        let mut candidate=Engine::restore_json(SAVE).unwrap();
        let admitted=candidate.admit_interior_access(3317,host_t,end,lots);
        let validation=candidate.validate().err();
        json!({"admission_call_passed":admitted,"saved_world_validation_passed":validation.is_none(),"validation_error":validation})
    });
    let straight_report=json!({
        "host_road_id":3317,"host_t":0.72,"split_preflight":"passed",
        "face_vertex_ids":face.vertex_ids,"face_area_m2":face.area,
        "endpoint_candidates_inside_face_length_26_to_60m":checked,
        "rejections":rejected,"exact_straight_candidates_with_two_accessible_lots":straight_winners.len(),
        "best_candidates":straight_winners.into_iter().take(12).collect::<Vec<_>>(),
        "existing_lifecycle_probe_for_first_geometry_candidate":lifecycle,
        "source_save_unchanged":engine.save_json()==before,
        "note":"Endpoint grid and face containment are diagnostic sampling; all emitted lots and branch constraints use current engine predicates. No world state was committed.",
    });
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation/face-access-straight-candidates.json");
    std::fs::write(path,serde_json::to_vec_pretty(&straight_report).unwrap()).unwrap();
    println!("{}",straight_report);

    // Audit proposals that the saved lifecycle schema can actually serialize:
    // the three supported split fractions and four supported perpendicular
    // branch lengths, on every edge of this same face.
    let mut legal_counts=BTreeMap::<&'static str,usize>::new();
    let mut legal_candidates=Vec::new();
    let mut legal_checked=0usize;
    let mut host_focus=Vec::new();
    let host_ids: Vec<u32>=face.boundary_edge_ids.iter().copied().collect();
    for host_id in host_ids {
        let Some(host)=engine.state.roads.iter().find(|r|r.id==host_id) else {continue;};
        if !matches!(host.kind.as_str(),"local"|"arterial") || host.stage!="open" || !host.connected {continue;}
        let (Some(a),Some(b))=(engine.node(host.from),engine.node(host.to)) else {continue;};
        let (ax,az,bx,bz)=(a.x,a.z,b.x,b.z);
        let hdx=bx-ax;let hdz=bz-az;let hlen=hdx.hypot(hdz);
        for host_t in interior_access::FRACTIONS {
            let mut probe=Engine::restore_json(SAVE).unwrap();
            let split=match probe.try_split_open_road(host_id,host_t) {
                Ok(split)=>split,
                Err(_)=>{*legal_counts.entry("host_split_preflight").or_default()+=interior_access::SIDES.len()*interior_access::LENGTHS.len();continue;}
            };
            let start=probe.node(split.split_node_id).unwrap().clone();
            let context=probe.collision_context();
            let ux=hdx/hlen;let uz=hdz/hlen;
            for side in interior_access::SIDES {
                let nx=-uz*side as f32;let nz=ux*side as f32;
                for length in interior_access::LENGTHS {
                    legal_checked+=1;
                    let end=WorldPoint{x:start.x+nx*length,z:start.z+nz*length};
                    let reason=if end.x<MIN+24.0||end.x>MAX-24.0||end.z<MIN+24.0||end.z>MAX-24.0 {Some("bounds")}
                        else if !path_inside(start.x,start.z,end.x,end.z){Some("outside_target_face")}
                        else if !probe.ordinary_heading_clear(split.split_node_id,(end.z-start.z).atan2(end.x-start.x)){Some("junction_angle")}
                        else if probe.state.nodes.iter().any(|n|(n.x-end.x).hypot(n.z-end.z)<18.0){Some("junction_spacing")}
                        else if !probe.ordinary_road_clear_of_river(start.x,start.z,end.x,end.z,4.5){Some("river")}
                        else if corridor_crosses_protected(start.x,start.z,end.x,end.z){Some("protected")}
                        else if !probe.road_corridor_clear_of_parcels_with_context(start.x,start.z,end.x,end.z,&context){Some("parcel_corridor")}
                        else if probe.state.roads.iter().any(|road|{
                            if road.from==split.split_node_id||road.to==split.split_node_id{return false;}
                            let (Some(ra),Some(rb))=(probe.node(road.from),probe.node(road.to)) else{return true;};
                            geometry::segments_distance_coords(start.x,start.z,end.x,end.z,ra.x,ra.z,rb.x,rb.z)<8.0
                        }){Some("road_spacing")}
                        else{None};
                    if let Some(reason)=reason{
                        *legal_counts.entry(reason).or_default()+=1;
                        if host_id==3317 {host_focus.push(json!({"host_t":host_t,"side":side,"length_m":length,"rejection":reason}));}
                        continue;
                    }
                    let mut proposed_context=context.clone();
                    proposed_context.open_roads.push(CollisionSegment{ax:start.x,az:start.z,bx:end.x,bz:end.z});
                    let dx=end.x-start.x;let dz=end.z-start.z;
                    let branch_len=dx.hypot(dz);let nx=-dz/branch_len;let nz=dx/branch_len;
                    let intervals=frontage::subdivide_frontage(length,&[],FRONTAGE_ENDPOINT_CLEARANCE,8.0,14.0,2.2,MAX_FRONTAGE_GAP_LOTS_PER_SIDE);
                    let mut lots=Vec::new();
                    for lot_side in [-1_i8,1]{for interval in &intervals{
                        let along=(interval.start+interval.end)*0.5;
                        let rect=geometry::OrientedRect{
                            x:start.x+dx*(along/branch_len)-nz*lot_side as f32*15.0,
                            z:start.z+dz*(along/branch_len)+nx*lot_side as f32*15.0,
                            width:interval.end-interval.start,depth:12.0,
                            rotation:(lot_side as f32*dz).atan2(-lot_side as f32*dx),
                        };
                        let corners=rect.corners();
                        let inside=point_in_poly(rect.x,rect.z,&face.polygon)&&corners.iter().all(|p|point_in_poly(p.0,p.1,&face.polygon));
                        if inside&&probe.progressive_frontage_geometry_clear(&rect,&proposed_context){lots.push(rect);}
                    }}
                    if lots.len()<2{
                        *legal_counts.entry("fewer_than_two_exact_lots").or_default()+=1;
                        if host_id==3317 {host_focus.push(json!({"host_t":host_t,"side":side,"length_m":length,"rejection":"fewer_than_two_exact_lots","lot_count":lots.len()}));}
                        continue;
                    }
                    if host_id==3317 {host_focus.push(json!({"host_t":host_t,"side":side,"length_m":length,"result":"geometry_and_two_lots_pass","lot_count":lots.len()}));}
                    let score=lots.iter().map(|r|r.width*r.depth).sum::<f32>()/length;
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
    legal_candidates.sort_by(|a,b|b["area_per_road_m"].as_f64().unwrap_or(0.0).total_cmp(&a["area_per_road_m"].as_f64().unwrap_or(0.0)));
    let legal_report=json!({
        "checkpoint_tick":engine.state.tick,"target_face_vertex_ids":face.vertex_ids,"target_face_area_m2":face.area,
        "candidate_budget":legal_checked,"rejections":legal_counts,
        "host_3317_candidate_details":host_focus,
        "admissible_saved_lifecycle_candidates":legal_candidates.len(),
        "best_candidates":legal_candidates.into_iter().take(12).collect::<Vec<_>>(),
        "note":"Candidates use supported split fractions and branch lengths, exact road-split preflight, current terrain/corridor/junction/parcel rules, and door-facing parcel envelopes. No production state was changed.",
    });
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../validation/face-access-legal-candidates.json");
    std::fs::write(path,serde_json::to_vec_pretty(&legal_report).unwrap()).unwrap();
    println!("{}",legal_report);
}
