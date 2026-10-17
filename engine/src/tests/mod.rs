use super::*;

mod access_descriptor_integration_tests;
mod access_path_validation_tests;
pub(crate) mod access_reference;
mod access_tests;
mod bus_tests;
mod core_tests;
mod district_density_integration_tests;
mod district_tests;
pub(crate) mod face_access_exact_tests;
mod frontage_tests;
mod height_redevelopment_tests;
mod masterplan_replay_tests;
#[cfg(feature = "parent-passage-candidate")]
mod parent_passage_tests;
mod parks_tests;
mod persistence_tests;
mod planner_tests;
mod presentation_revision_tests;
mod rail_tests;
mod route_equivalence_tests;
mod spine_replay_tests;
mod traffic_tests;
mod transit_access_tests;

#[derive(Default, Debug)]
struct MatureProposalAudit {
    anchors_seen: usize,
    anchors_with_local_demand: usize,
    proposal_count: usize,
    reasons: BTreeMap<&'static str, usize>,
    blocked_lot_sites: BTreeMap<&'static str, usize>,
    eligible_reuse_edges: usize,
    windows: usize,
    shared_candidates: usize,
    full_candidates: usize,
    valid_proposals: usize,
}
#[derive(Clone, Copy, Debug)]
struct MidEdgeLotProbe {
    x: f32,
    z: f32,
    width: f32,
    depth: f32,
    rotation: f32,
    side: i8,
    along: f32,
}
#[derive(Clone, Debug)]
struct MidEdgeAccessProbe {
    host_road_id: u32,
    host_t: f32,
    start: WorldPoint,
    end: WorldPoint,
    length: f32,
    lots: Vec<MidEdgeLotProbe>,
}
#[derive(Default, Debug)]
struct MidEdgeProbeAudit {
    hosts: usize,
    attempted: usize,
    rejected: BTreeMap<&'static str, usize>,
    safe_lot_count: usize,
    feasible: Vec<MidEdgeAccessProbe>,
}
fn probe_mid_edge_access(
    engine: &Engine,
    context: &CollisionContext,
    host_road: &Road,
    host_a: &Node,
    host_b: &Node,
    host_t: f32,
    side: i8,
    length: f32,
) -> Result<MidEdgeAccessProbe, &'static str> {
    let host_dx = host_b.x - host_a.x;
    let host_dz = host_b.z - host_a.z;
    let host_length = host_dx.hypot(host_dz);
    if !host_length.is_finite() || host_length <= 0.0 {
        return Err("invalid_host");
    }
    let (host_ux, host_uz) = (host_dx / host_length, host_dz / host_length);
    let (nx, nz) = (-host_uz * side as f32, host_ux * side as f32);
    let start = WorldPoint {
        x: host_a.x + host_dx * host_t,
        z: host_a.z + host_dz * host_t,
    };
    let end = WorldPoint {
        x: start.x + nx * length,
        z: start.z + nz * length,
    };
    if [start.x, start.z, end.x, end.z]
        .iter()
        .any(|v| !v.is_finite() || *v < MIN + 24.0 || *v > MAX - 24.0)
    {
        return Err("bounds");
    }
    if !engine.ordinary_road_clear_of_river(start.x, start.z, end.x, end.z, 4.5) {
        return Err("river");
    }
    if corridor_crosses_protected(start.x, start.z, end.x, end.z) {
        return Err("protected");
    }
    if !engine.road_corridor_clear_of_parcels_with_context(start.x, start.z, end.x, end.z, context)
    {
        return Err("parcel_corridor");
    }
    for road in engine
        .state
        .roads
        .iter()
        .filter(|road| road.stage == Stage::Open)
    {
        if road.id == host_road.id {
            continue;
        }
        let (Some(a), Some(b)) = (engine.node(road.from), engine.node(road.to)) else {
            continue;
        };
        if geometry::segments_distance_coords(start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z)
            < 8.0
        {
            return Err("other_road_intersection");
        }
    }

    let mut proposed_context = context.clone();
    proposed_context.open_roads.push(CollisionSegment {
        ax: start.x,
        az: start.z,
        bx: end.x,
        bz: end.z,
    });
    let mut safe_lots = Vec::new();
    for lot_side in [-1_i8, 1] {
        for interval in frontage::subdivide_frontage(
            length,
            &[],
            FRONTAGE_ENDPOINT_CLEARANCE,
            8.0,
            14.0,
            2.2,
            MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
        ) {
            let along = (interval.start + interval.end) * 0.5;
            let parcel_nx = -nz;
            let parcel_nz = nx;
            let x =
                start.x + (end.x - start.x) * (along / length) + parcel_nx * lot_side as f32 * 15.0;
            let z =
                start.z + (end.z - start.z) * (along / length) + parcel_nz * lot_side as f32 * 15.0;
            let width = interval.end - interval.start;
            let rotation =
                (lot_side as f32 * (end.z - start.z)).atan2(-lot_side as f32 * (end.x - start.x));
            let rect = geometry::OrientedRect {
                x,
                z,
                width,
                depth: 12.0,
                rotation,
            };
            if engine.progressive_frontage_geometry_clear(&rect, &proposed_context) {
                safe_lots.push(MidEdgeLotProbe {
                    x,
                    z,
                    width,
                    depth: 12.0,
                    rotation,
                    side: lot_side,
                    along,
                });
            }
        }
    }
    if safe_lots.len() < 2 {
        return Err("fewer_than_two_safe_lots");
    }
    Ok(MidEdgeAccessProbe {
        host_road_id: host_road.id,
        host_t,
        start,
        end,
        length,
        lots: safe_lots,
    })
}
fn audit_mid_edge_access_candidates(engine: &Engine) -> MidEdgeProbeAudit {
    let context = engine.collision_context();
    let mut audit = MidEdgeProbeAudit::default();
    for host in engine.state.roads.iter().filter(|road| {
        road.stage == Stage::Open
            && road.connected
            && !matches!(road.kind, RoadKind::Bridge | RoadKind::Rail)
    }) {
        let (Some(a), Some(b)) = (engine.node(host.from), engine.node(host.to)) else {
            continue;
        };
        audit.hosts += 1;
        for host_t in [0.25_f32, 0.5, 0.75] {
            for side in [-1_i8, 1] {
                for length in [24.0_f32, 36.0, 48.0, 60.0] {
                    audit.attempted += 1;
                    match probe_mid_edge_access(engine, &context, host, a, b, host_t, side, length)
                    {
                        Ok(candidate) => {
                            audit.safe_lot_count += candidate.lots.len();
                            audit.feasible.push(candidate);
                        }
                        Err(reason) => *audit.rejected.entry(reason).or_default() += 1,
                    }
                }
            }
        }
    }
    audit.feasible.sort_by(|a, b| {
        b.lots
            .len()
            .cmp(&a.lots.len())
            .then_with(|| a.length.total_cmp(&b.length))
            .then_with(|| a.host_road_id.cmp(&b.host_road_id))
            .then_with(|| a.host_t.total_cmp(&b.host_t))
            .then_with(|| a.start.x.total_cmp(&b.start.x))
            .then_with(|| a.start.z.total_cmp(&b.start.z))
    });
    audit.feasible.truncate(8);
    audit
}
fn audit_lot_geometry(
    engine: &Engine,
    lot: &blocks::BlockLot,
    context: &CollisionContext,
) -> &'static str {
    let rect = geometry::OrientedRect {
        x: lot.center.x,
        z: lot.center.z,
        width: lot.width,
        depth: lot.depth,
        rotation: lot.rotation,
    };
    if !rect.finite_positive()
        || !engine.parcel_clear_of_river(rect.x, rect.z, rect.width, rect.depth)
    {
        return "lot_river_or_invalid";
    }
    let (min_x, min_z, max_x, max_z) = rect.bounds();
    if min_x < MIN + 4.0 || min_z < MIN + 4.0 || max_x > MAX - 4.0 || max_z > MAX - 4.0 {
        return "lot_bounds";
    }
    let protected = geometry::OrientedRect {
        x: -305.0,
        z: -310.0,
        width: 90.0,
        depth: 80.0,
        rotation: 0.0,
    };
    if geometry::rects_overlap(&rect, &protected, 0.0) {
        return "lot_protected";
    }
    for parcel in &engine.state.parcels {
        let legacy = parcel_envelope(parcel);
        if parcel.rotation.is_none() && geometry::rects_overlap(&rect, &legacy, 2.0) {
            return "lot_legacy_envelope";
        }
        let rendered = if let Some(rotation) = parcel.rotation {
            geometry::OrientedRect { rotation, ..legacy }
        } else {
            let rotation = engine
                .state
                .buildings
                .iter()
                .find(|building| building.parcel_id == parcel.id)
                .map(|building| building.rotation)
                .unwrap_or_else(|| engine.frontage_rotation(parcel.id));
            geometry::OrientedRect { rotation, ..legacy }
        };
        if geometry::rects_overlap(&rect, &rendered, 2.0) {
            return "lot_rendered_footprint";
        }
    }
    if context.open_roads.iter().any(|road| {
        geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 5.0
    }) || context.pending_roads.iter().any(|road| {
        geometry::rect_segment_distance(&rect, road.ax, road.az, road.bx, road.bz) < 7.0
    }) {
        return "lot_road_corridor";
    }
    "lot_other"
}
fn audit_neighborhood_candidate(
    engine: &Engine,
    anchor: &Node,
    corners: &[WorldPoint; 4],
    corner_ids: [Option<u32>; 4],
    reused_edge: Option<(usize, u32)>,
    selected_lots: Option<&[blocks::BlockLot]>,
    tower_envelopes: &[(f32, f32)],
    context: &CollisionContext,
) -> &'static str {
    if engine.neighborhood_candidate_clear_with_edge(
        anchor,
        corners,
        tower_envelopes,
        context,
        corner_ids,
        reused_edge,
        selected_lots,
    ) {
        return "feasible";
    }
    let required_segments = 4 - usize::from(reused_edge.is_some());
    if engine
        .state
        .roads
        .len()
        .saturating_add(engine.state.projects.len())
        .saturating_add(required_segments)
        > MAX_SEGMENTS
    {
        return "segment_budget";
    }
    let mut new_incident = [0usize; 4];
    for edge in 0..4 {
        if reused_edge.is_some_and(|(reused, _)| reused == edge) {
            continue;
        }
        new_incident[edge] += 1;
        new_incident[(edge + 1) % 4] += 1;
    }
    for (edge, id) in corner_ids.iter().enumerate() {
        if id.is_some_and(|node| engine.ordinary_road_degree(node) + new_incident[edge] > 4) {
            return "junction_degree";
        }
    }
    let canonical = blocks::subdivide_rectangular_block(
        corners.map(|point| blocks::BlockPoint {
            x: point.x,
            z: point.z,
        }),
        8.0,
        11.0,
        2.2,
    );
    let lots = selected_lots.unwrap_or(&canonical);
    if lots.len() < 4 {
        return "fewer_than_four_compatible_lots";
    }
    if lots.len() > 64
        || lots.iter().any(|lot| !canonical.contains(lot))
        || engine.state.parcels.len().saturating_add(lots.len()) > MAX_PARCELS
    {
        return "parcel_capacity_or_noncanonical_lots";
    }
    if let Some(lot) = lots.iter().find(|lot| {
        !engine.oriented_parcel_geometry_clear_with_context(
            &geometry::OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            },
            context,
        )
    }) {
        return audit_lot_geometry(engine, lot, context);
    }
    if let Some((reused_index, _)) = reused_edge {
        for edge in 0..4 {
            if edge != reused_index
                && (distance(
                    &Node {
                        id: 0,
                        x: corners[edge].x,
                        z: corners[edge].z,
                        kind: NodeKind::Junction,
                    },
                    &Node {
                        id: 0,
                        x: corners[(edge + 1) % 4].x,
                        z: corners[(edge + 1) % 4].z,
                        kind: NodeKind::Junction,
                    },
                ) >= 70.0
                    && lots.iter().filter(|lot| lot.frontage_edge == edge).count() < 2)
            {
                return "fewer_than_two_sites_on_long_new_edge";
            }
        }
    }
    let corner_nodes: [Node; 4] = std::array::from_fn(|index| {
        corner_ids[index]
            .and_then(|id| engine.node(id).cloned())
            .unwrap_or(Node {
                id: u32::MAX - index as u32,
                x: corners[index].x,
                z: corners[index].z,
                kind: NodeKind::Candidate,
            })
    });
    for edge in 0..4 {
        let a = &corner_nodes[edge];
        let b = &corner_nodes[(edge + 1) % 4];
        let is_reused = reused_edge.is_some_and(|(reused, road_id)| {
            reused == edge
                && engine.state.roads.iter().any(|road| {
                    road.id == road_id
                        && road.stage == Stage::Open
                        && road.connected
                        && !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge)
                        && ((road.from == a.id && road.to == b.id)
                            || (road.from == b.id && road.to == a.id))
                })
        });
        if reused_edge.is_some_and(|(reused, _)| reused == edge) && !is_reused {
            return "reused_edge_not_open_connected";
        }
        if a.x < MIN + 24.0
            || a.x > MAX - 24.0
            || a.z < MIN + 24.0
            || a.z > MAX - 24.0
            || !engine.ordinary_road_clear_of_river(a.x, a.z, b.x, b.z, 4.5)
            || corridor_crosses_protected(a.x, a.z, b.x, b.z)
        {
            return "road_bounds_river_or_protected";
        }
        if !is_reused {
            let heading = (b.z - a.z).atan2(b.x - a.x);
            if corner_ids[edge].is_some_and(|id| !engine.ordinary_heading_clear(id, heading))
                || corner_ids[(edge + 1) % 4].is_some_and(|id| {
                    !engine.ordinary_heading_clear(id, (a.z - b.z).atan2(a.x - b.x))
                })
            {
                return "junction_heading";
            }
        }
        if !is_reused {
            let legacy = engine.state.parcels.iter().any(|parcel| {
                point_segment_distance(parcel.x, parcel.z, a.x, a.z, b.x, b.z)
                    <= parcel.width.max(parcel.depth) * 0.5 + 5.0
            });
            let exact = context.parcel_footprints.iter().any(|footprint| {
                geometry::rect_segment_distance(footprint, a.x, a.z, b.x, b.z) < 5.0
            });
            if legacy {
                return "road_legacy_parcel_radius";
            }
            if exact {
                return "road_exact_footprint_clearance";
            }
        }
        for road in engine
            .state
            .roads
            .iter()
            .filter(|road| road.stage == Stage::Open)
        {
            let (Some(old_a), Some(old_b)) = (engine.node(road.from), engine.node(road.to)) else {
                continue;
            };
            if is_reused && reused_edge.is_some_and(|(_, id)| id == road.id) {
                continue;
            }
            let shares = [corner_ids[edge], corner_ids[(edge + 1) % 4]]
                .into_iter()
                .flatten()
                .any(|id| road.from == id || road.to == id);
            if !shares && segments_distance(a, b, old_a, old_b) <= 8.0 {
                return "existing_road_segment_intersection";
            }
        }
        if engine.state.projects.iter().any(|project| {
            if project.kind != ProjectKind::Road || project.progress >= 1.0 {
                return false;
            }
            let Some((from, to)) = project.from.zip(project.to) else {
                return false;
            };
            let (Some(old_a), Some(old_b)) = (engine.node(from), engine.node(to)) else {
                return false;
            };
            let shares = [corner_ids[edge], corner_ids[(edge + 1) % 4]]
                .into_iter()
                .flatten()
                .any(|id| from == id || to == id);
            !shares && segments_distance(a, b, old_a, old_b) <= 8.0
        }) {
            return "active_road_project_intersection";
        }
        if !is_reused && engine.state.neighborhood_plan.is_some() {
            return "reserved_neighborhood_corridor";
        }
        if tower_envelopes
            .iter()
            .any(|(x, z)| point_segment_distance(*x, *z, a.x, a.z, b.x, b.z) < 24.0)
        {
            return "station_tower_envelope";
        }
    }
    if selected_lots.is_none() {
        return "legacy_inner_frontage_benefit_or_other";
    }
    "other_geometry_gate"
}
fn audit_mature_neighborhood_proposals(engine: &Engine) -> MatureProposalAudit {
    let mut audit = MatureProposalAudit::default();
    let node_count = engine.state.nodes.len();
    if node_count == 0 {
        return audit;
    }
    let active_buildings: Vec<_> = engine
        .state
        .buildings
        .iter()
        .filter(|building| site_active(building))
        .collect();
    let connected: BTreeSet<u32> = engine
        .state
        .roads
        .iter()
        .filter(|road| road.connected && road.stage == Stage::Open)
        .flat_map(|road| [road.from, road.to])
        .collect();
    let mut reusable: BTreeMap<u32, Vec<Road>> = BTreeMap::new();
    for road in engine
        .state
        .roads
        .iter()
        .filter(|road| road.connected && road.stage == Stage::Open)
    {
        if !matches!(road.kind, RoadKind::Rail | RoadKind::Bridge) {
            reusable.entry(road.from).or_default().push(road.clone());
            reusable.entry(road.to).or_default().push(road.clone());
        }
    }
    let tower_envelopes = engine.available_station_tower_envelopes();
    let collision = engine.collision_context();
    let mut cursor = engine.state.neighborhood_anchor_cursor % node_count;
    let mut visited = BTreeSet::new();
    while visited.len() < node_count && audit.windows <= node_count {
        audit.windows += 1;
        let first = cursor;
        let mut budget = development::NeighborhoodCandidateBudget::default();
        let mut examined_edges = BTreeSet::new();
        for offset in 0..node_count.min(MAX_NEIGHBORHOOD_ANCHORS_PER_EVALUATION) {
            let index = (first + offset) % node_count;
            visited.insert(index);
            cursor = (index + 1) % node_count;
            audit.anchors_seen += 1;
            let anchor = &engine.state.nodes[index];
            if !connected.contains(&anchor.id) || engine.ordinary_road_degree(anchor.id) == 0 {
                continue;
            }
            let local = active_buildings
                .iter()
                .filter(|building| (building.x - anchor.x).hypot(building.z - anchor.z) <= 112.0)
                .count();
            if local < 8 || local < 4 {
                continue;
            }
            audit.anchors_with_local_demand += 1;
            if let Some(edges) = reusable.get(&anchor.id) {
                for road in edges {
                    if examined_edges.contains(&road.id)
                        || examined_edges.len() >= MAX_NEIGHBORHOOD_REUSE_EDGES_PER_EVALUATION
                    {
                        continue;
                    }
                    examined_edges.insert(road.id);
                    let (Some(start), Some(end)) = (engine.node(road.from), engine.node(road.to))
                    else {
                        continue;
                    };
                    let length = distance(start, end);
                    let extrusion = if (70.0..=90.0).contains(&length) {
                        NEIGHBORHOOD_BLOCK_WIDTH
                    } else if (42.0..=58.0).contains(&length) {
                        NEIGHBORHOOD_BLOCK_LENGTH
                    } else {
                        continue;
                    };
                    audit.eligible_reuse_edges += 1;
                    let side_seed = stateless_tower_site_hash(
                        engine.state.seed ^ road.id,
                        (start.x + end.x) * 0.5,
                        (start.z + end.z) * 0.5,
                    );
                    let sides = if side_seed & 1 == 0 {
                        [1.0_f32, -1.0]
                    } else {
                        [-1.0_f32, 1.0]
                    };
                    for (side_index, side) in sides.into_iter().enumerate() {
                        if !budget.claim_shared() {
                            break;
                        }
                        let nx = -(end.z - start.z) / length * side;
                        let nz = (end.x - start.x) / length * side;
                        let corners = [
                            WorldPoint {
                                x: start.x,
                                z: start.z,
                            },
                            WorldPoint { x: end.x, z: end.z },
                            WorldPoint {
                                x: end.x + nx * extrusion,
                                z: end.z + nz * extrusion,
                            },
                            WorldPoint {
                                x: start.x + nx * extrusion,
                                z: start.z + nz * extrusion,
                            },
                        ];
                        let lots = engine.available_coordinated_block_lots(&corners, &collision);
                        let canonical = blocks::subdivide_rectangular_block(
                            corners.map(|point| blocks::BlockPoint {
                                x: point.x,
                                z: point.z,
                            }),
                            8.0,
                            11.0,
                            2.2,
                        );
                        for rejected in canonical.iter().filter(|lot| !lots.contains(lot)) {
                            let blocker = audit_lot_geometry(engine, rejected, &collision);
                            *audit.blocked_lot_sites.entry(blocker).or_default() += 1;
                        }
                        let reason = audit_neighborhood_candidate(
                            engine,
                            anchor,
                            &corners,
                            [Some(road.from), Some(road.to), None, None],
                            Some((0, road.id)),
                            Some(&lots),
                            &tower_envelopes,
                            &collision,
                        );
                        audit.proposal_count += 1;
                        audit.shared_candidates += 1;
                        if reason == "feasible" {
                            audit.valid_proposals += 1;
                        }
                        *audit.reasons.entry(reason).or_default() += 1;
                        let _stable_key =
                            road.id.saturating_mul(2).saturating_add(side_index as u32);
                    }
                    if budget.shared_limit_reached() {
                        break;
                    }
                }
            }
            if engine.ordinary_road_degree(anchor.id) <= 2 {
                let start_option =
                    stateless_tower_site_hash(engine.state.seed, anchor.x, anchor.z) as usize % 8;
                for rotation in 0..8 {
                    if !budget.claim_full() {
                        break;
                    }
                    let option = (start_option + rotation) % 8;
                    let (ux, uz, vx, vz) = match option {
                        0 => (1.0, 0.0, 0.0, 1.0),
                        1 => (1.0, 0.0, 0.0, -1.0),
                        2 => (-1.0, 0.0, 0.0, 1.0),
                        3 => (-1.0, 0.0, 0.0, -1.0),
                        4 => (0.0, 1.0, 1.0, 0.0),
                        5 => (0.0, 1.0, -1.0, 0.0),
                        6 => (0.0, -1.0, 1.0, 0.0),
                        _ => (0.0, -1.0, -1.0, 0.0),
                    };
                    let corners = [
                        WorldPoint {
                            x: anchor.x,
                            z: anchor.z,
                        },
                        WorldPoint {
                            x: anchor.x + ux * NEIGHBORHOOD_BLOCK_LENGTH,
                            z: anchor.z + uz * NEIGHBORHOOD_BLOCK_LENGTH,
                        },
                        WorldPoint {
                            x: anchor.x
                                + ux * NEIGHBORHOOD_BLOCK_LENGTH
                                + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                            z: anchor.z
                                + uz * NEIGHBORHOOD_BLOCK_LENGTH
                                + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                        },
                        WorldPoint {
                            x: anchor.x + vx * NEIGHBORHOOD_BLOCK_WIDTH,
                            z: anchor.z + vz * NEIGHBORHOOD_BLOCK_WIDTH,
                        },
                    ];
                    let reason = audit_neighborhood_candidate(
                        engine,
                        anchor,
                        &corners,
                        [Some(anchor.id), None, None, None],
                        None,
                        None,
                        &tower_envelopes,
                        &collision,
                    );
                    audit.proposal_count += 1;
                    audit.full_candidates += 1;
                    if reason == "feasible" {
                        audit.valid_proposals += 1;
                    }
                    *audit.reasons.entry(reason).or_default() += 1;
                }
            }
            if budget.any_limit_reached() {
                break;
            }
        }
    }
    audit
}
fn building_picker_engine() -> (Engine, u32) {
    let mut engine = Engine::new_legacy(731);
    engine.state.parcels.clear();
    engine.state.parcel_frontages.clear();
    engine.state.buildings.clear();
    engine.state.projects.clear();
    engine.state.residents.clear();
    engine.state.travelers.clear();
    engine.state.buses.clear();
    engine.state.trains.clear();
    engine.state.bus_line = None;
    engine.state.neighborhood_plan = None;
    engine.state.station_online = true;
    engine.state.demand = 4.0;

    let access_node = engine
        .state
        .roads
        .iter()
        .find(|road| road.stage == Stage::Open && road.connected && road.kind != RoadKind::Rail)
        .map(|road| road.from)
        .expect("founding street is open and connected");
    for (x, z) in [(210.0, 200.0), (222.0, 200.0), (216.0, 212.0)] {
        let parcel = engine.add_parcel(x, z, 8.0, 8.0, access_node, 1.0, false);
        let building = engine.add_building(
            parcel,
            BuildingKind::Home,
            2,
            Stage::Open,
            1.0,
            4,
            "Test home",
            false,
            Vec::new(),
        );
        let resident_building = engine
            .state
            .buildings
            .iter_mut()
            .find(|item| item.id == building)
            .unwrap();
        resident_building.occupancy = 2;
        resident_building.built_day = 1;
    }
    (engine, access_node)
}
fn bus_test_engine(residents: usize) -> (Engine, Vec<u32>) {
    let mut engine = Engine::new_legacy(217);
    let founder = engine.state.buildings.first().unwrap().id;
    let parcel_id = engine.state.buildings.first().unwrap().parcel_id;
    engine.state.buildings[0].kind = BuildingKind::Home;
    engine.state.buildings[0].stage = Stage::Open;
    engine.state.buildings[0].occupancy = 1;
    let start = engine
        .state
        .parcels
        .iter()
        .find(|parcel| parcel.id == parcel_id)
        .unwrap()
        .access_node;
    let start_node = engine.node(start).unwrap().clone();
    let mut route = vec![start];
    let mut previous = start;
    for index in 1..=4 {
        let x_offset = match index {
            1 => 15.0,
            2 => 55.0,
            3 => 95.0,
            _ => 135.0,
        };
        let id = engine.id();
        engine.state.nodes.push(Node {
            id,
            x: start_node.x + x_offset,
            z: start_node.z + 7.0,
            kind: NodeKind::Junction,
        });
        let from = engine.node(previous).unwrap().clone();
        let to = engine.node(id).unwrap().clone();
        let road_id = engine.id();
        engine.state.roads.push(Road {
            id: road_id,
            from: previous,
            to: id,
            kind: RoadKind::Local,
            stage: Stage::Open,
            progress: 1.0,
            connected: true,
            length: distance(&from, &to),
            lanes: 2,
        });
        previous = id;
        route.push(id);
    }
    let stops = vec![route[0], route[2], route[3]];
    let line_id = engine.id();
    engine.state.bus_line = Some(BusLine {
        id: line_id,
        stops: stops.clone(),
        route: route[..=3].to_vec(),
        active: true,
        graph_revision: 1,
    });
    let bus_id = engine.id();
    engine.state.buses.push(Bus {
        id: bus_id,
        line_id,
        progress: 0.0,
        direction: 1,
        dwell_ticks: 20_000,
        passengers: Vec::new(),
        next_departure_tick: 20_000,
        last_stop: Some(stops[0]),
    });
    let mut ids = Vec::new();
    for index in 0..residents {
        let id = engine.id();
        ids.push(id);
        engine.state.residents.push(Resident {
            id,
            name: format!("Bus Rider {index}"),
            home_building: founder,
            work_building: Some(founder),
            home_node: route[1],
            work_node: Some(route[4]),
            mode: 0,
            routine: ResidentRoutine::Worker,
            activity: ResidentActivity::AtHome,
            at_work: false,
            departure_tick: 0,
            return_tick: 10_000,
            last_departure_day: u32::MAX,
            last_return_day: u32::MAX,
            history: vec!["Settled near a connected bus corridor".into()],
            rail_commuter: false,
            transfer: Transfer::None,
            train_id: None,
            queue_slot: None,
            bus_id: None,
            bus_origin_stop: None,
            bus_destination_stop: None,
            bus_stop_node: None,
            pending_alight: None,
        });
    }
    (engine, ids)
}
fn rail_test_engine(seed: u32) -> (Engine, u32, u32) {
    let mut engine = Engine::new_legacy(seed);
    let (from, to) = {
        let rail = engine
            .state
            .roads
            .iter_mut()
            .find(|road| road.kind == RoadKind::Rail)
            .unwrap();
        rail.stage = Stage::Open;
        rail.progress = 1.0;
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
    let gateway = if from == station { to } else { from };
    engine.state.station_ready = true;
    engine.state.station_online = true;
    engine.recompute_connectivity();
    (engine, station, gateway)
}
fn add_rail_test_resident(engine: &mut Engine, gateway: u32) -> u32 {
    let existing_home = engine
        .state
        .buildings
        .iter()
        .find(|building| building.name == "Test Rail Home")
        .cloned();
    let (home_id, home_node) = if let Some(home) = existing_home {
        let parcel = engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == home.parcel_id)
            .unwrap();
        if let Some(building) = engine
            .state
            .buildings
            .iter_mut()
            .find(|building| building.id == home.id)
        {
            building.occupancy = building.occupancy.saturating_add(1);
        }
        (home.id, parcel.access_node)
    } else {
        let parcel = engine
            .state
            .parcels
            .iter()
            .find(|parcel| {
                parcel.reservation.is_none()
                    && engine
                        .state
                        .buildings
                        .iter()
                        .all(|building| building.parcel_id != parcel.id)
            })
            .unwrap()
            .clone();
        let home_id = engine.add_building(
            parcel.id,
            BuildingKind::Home,
            2,
            Stage::Open,
            1.0,
            100,
            "Test Rail Home",
            false,
            vec!["Opened for rail transfer test".into()],
        );
        engine.state.buildings.last_mut().unwrap().occupancy = 1;
        (home_id, parcel.access_node)
    };
    let id = engine.id();
    engine.state.residents.push(Resident {
        id,
        name: format!("Commuter {id}"),
        home_building: home_id,
        work_building: None,
        home_node,
        work_node: Some(gateway),
        mode: 0,
        routine: ResidentRoutine::RailCommuter,
        activity: ResidentActivity::AtHome,
        at_work: false,
        departure_tick: 0,
        return_tick: 0,
        last_departure_day: u32::MAX,
        last_return_day: u32::MAX,
        history: vec!["Moved into a home near the station route".into()],
        rail_commuter: true,
        transfer: Transfer::Home,
        train_id: None,
        queue_slot: None,
        bus_id: None,
        bus_origin_stop: None,
        bus_destination_stop: None,
        bus_stop_node: None,
        pending_alight: None,
    });
    id
}
fn engine_with_clear_local_chord() -> (Engine, u32, u32) {
    let mut engine = Engine::new_legacy(87231);
    engine.state.nodes.clear();
    engine.state.roads.clear();
    engine.state.parcels.clear();
    engine.state.parcel_frontages.clear();
    engine.state.buildings.clear();
    engine.state.projects.clear();
    engine.state.travelers.clear();
    engine.state.residents.clear();
    engine.state.trains.clear();
    engine.state.buses.clear();
    engine.state.bus_line = None;
    engine.state.events.clear();
    engine.state.population = 300;
    engine.state.demand = 2.0;
    engine.state.station_online = false;
    engine.state.station_ready = false;
    engine.state.road_origin_cursor = 0;

    let gateway = engine.add_node(365.0, 360.0, NodeKind::Gateway);
    let hub = engine.add_node(30.0, 400.0, NodeKind::Junction);
    let start = engine.add_node(0.0, 300.0, NodeKind::Junction);
    let target = engine.add_node(60.0, 300.0, NodeKind::Junction);
    engine.add_road(gateway, hub, RoadKind::Local, Stage::Open, 1.0);
    engine.add_road(hub, start, RoadKind::Local, Stage::Open, 1.0);
    engine.add_road(hub, target, RoadKind::Local, Stage::Open, 1.0);
    engine.recompute_connectivity();

    // A compact, occupied block sits south of the clear 60 m chord. These
    // real parcels establish local demand without obstructing the corridor.
    for row in 0..2 {
        for column in 0..6 {
            let x = column as f32 * 12.0;
            let z = 230.0 + row as f32 * 12.0;
            let parcel_id = engine.add_parcel(x, z, 8.0, 8.0, hub, 0.85, false);
            let building_id = engine.add_building(
                parcel_id,
                BuildingKind::Home,
                2,
                Stage::Open,
                1.0,
                10,
                "Test Machiya",
                false,
                vec!["Occupied beside an active local lane".into()],
            );
            engine
                .state
                .buildings
                .iter_mut()
                .find(|building| building.id == building_id)
                .unwrap()
                .occupancy = 4;
        }
    }
    (engine, start, target)
}
fn engine_with_developed_plan_site() -> Engine {
    let mut engine = Engine::new_legacy(41);
    engine.state.nodes.clear();
    engine.state.roads.clear();
    engine.state.parcels.clear();
    engine.state.parcel_frontages.clear();
    engine.state.buildings.clear();
    engine.state.projects.clear();
    engine.state.travelers.clear();
    engine.state.residents.clear();
    engine.state.trains.clear();
    engine.state.buses.clear();
    engine.state.bus_line = None;
    engine.state.events.clear();
    engine.state.population = 80;
    engine.state.demand = 2.0;
    engine.state.station_online = false;
    engine.state.station_ready = false;
    engine.state.neighborhood_plan = None;
    engine.state.neighborhood_anchor_cursor = 0;

    let gateway = engine.add_node(365.0, 360.0, NodeKind::Gateway);
    let feeder = engine.add_node(310.0, 315.0, NodeKind::Junction);
    let anchor = engine.add_node(250.0, 250.0, NodeKind::Junction);
    engine.add_road(gateway, feeder, RoadKind::Local, Stage::Open, 1.0);
    engine.add_road(feeder, anchor, RoadKind::Local, Stage::Open, 1.0);
    engine.recompute_connectivity();

    // Real occupied frontage creates local demand west of the prospective
    // block while leaving the block's street corridors and lots clear.
    for row in 0..3 {
        for column in 0..4 {
            let x = 142.0 + column as f32 * 11.0;
            let z = 188.0 + row as f32 * 11.0;
            let parcel = engine.add_parcel(x, z, 7.0, 7.0, anchor, 0.8, false);
            let building = engine.add_building(
                parcel,
                BuildingKind::Home,
                2,
                Stage::Open,
                1.0,
                8,
                "Occupied machiya",
                false,
                vec!["A lived-in street edge".into()],
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
    engine
}
fn engine_with_developed_plan_site_without_reusable_edge() -> Engine {
    let mut engine = engine_with_developed_plan_site();
    // Keep the same occupied catchment, but move the feeder junction so
    // both existing roads fall outside the shared-edge length bands.
    let feeder_id = engine.state.roads[1].from;
    let feeder = engine
        .state
        .nodes
        .iter_mut()
        .find(|node| node.id == feeder_id)
        .unwrap();
    feeder.x = 320.0;
    feeder.z = 320.0;
    engine.recompute_connectivity();
    engine
}
/// A road with several cars arriving at one driveway whose mouth is the
/// final lane end, plus a car already pulling in so the mouth starts busy.
fn arrival_contention_engine() -> (Engine, f32, Vec<u32>) {
    let mut engine = Engine::new_legacy(81);
    let road = engine
        .state
        .roads
        .iter()
        .find(|road| road.kind != RoadKind::Rail && road.stage == Stage::Open)
        .expect("founding road is open");
    let (from, to) = (road.from, road.to);
    let end = engine
        .route_endpoint_position(&[from, to], false, 910_001, 1)
        .expect("arrival route endpoint");
    let len = distance(engine.node(from).unwrap(), engine.node(to).unwrap());
    let driveway = AccessPath {
        points: vec![
            end,
            WorldPoint {
                x: end.x + 30.0,
                z: end.z,
            },
        ],
        surface_y: Vec::new(),
    };
    let car = |id, access_phase, edge_progress: f32| Traveler {
        id,
        kind: 1,
        route: vec![from, to],
        route_lengths: vec![len],
        edge_index: 0,
        edge_progress,
        speed: 6.0,
        destination: to,
        activity: TravelerActivity::Commute,
        resident: 0,
        name: "Arriving car".into(),
        origin_building: None,
        destination_building: Some(77),
        wait_ticks: 0,
        origin_access: None,
        destination_access: Some(driveway.clone()),
        access_phase,
        access_progress: 0.0,
    };
    // 1: in the driveway mouth. 2: 0.1 m from the mouth. 3, 4: queued behind.
    engine.state.travelers.push(car(910_001, 2, len));
    engine.state.travelers.push(car(910_002, 1, len - 0.1));
    engine.state.travelers.push(car(910_003, 1, len - 6.0));
    engine.state.travelers.push(car(910_004, 1, len - 12.0));
    (engine, len, vec![910_001, 910_002, 910_003, 910_004])
}
fn movement_of(engine: &Engine, id: u32) -> Option<(f32, f32)> {
    engine
        .movement_buffer()
        .chunks(6)
        .find(|c| c[0] as u32 == id)
        .map(|c| (c[1], c[2]))
}
fn split_test_resident(
    id: u32,
    building: u32,
    home_node: u32,
    work_node: u32,
    transfer: Transfer,
    train_id: Option<u32>,
    bus_id: Option<u32>,
    bus_stops: Option<(u32, u32)>,
) -> Resident {
    Resident {
        id,
        name: format!("Split Test Resident {id}"),
        home_building: building,
        work_building: Some(building),
        home_node,
        work_node: Some(work_node),
        mode: 0,
        routine: ResidentRoutine::Worker,
        activity: crate::ResidentActivity::Commuting,
        at_work: true,
        departure_tick: 0,
        return_tick: 10_000,
        last_departure_day: u32::MAX,
        last_return_day: u32::MAX,
        history: vec!["Riding a connected route".into()],
        rail_commuter: train_id.is_some(),
        transfer,
        train_id,
        queue_slot: None,
        bus_id,
        bus_origin_stop: bus_stops.map(|stops| stops.0),
        bus_destination_stop: bus_stops.map(|stops| stops.1),
        bus_stop_node: None,
        pending_alight: None,
    }
}
fn route_position(engine: &Engine, traveler: &Traveler) -> WorldPoint {
    if traveler.edge_index >= traveler.route_lengths.len() {
        let node = engine.node(*traveler.route.last().unwrap()).unwrap();
        return WorldPoint {
            x: node.x,
            z: node.z,
        };
    }
    let a = engine.node(traveler.route[traveler.edge_index]).unwrap();
    let b = engine
        .node(traveler.route[traveler.edge_index + 1])
        .unwrap();
    let length = traveler.route_lengths[traveler.edge_index];
    let fraction = (traveler.edge_progress / length).clamp(0.0, 1.0);
    WorldPoint {
        x: a.x + (b.x - a.x) * fraction,
        z: a.z + (b.z - a.z) * fraction,
    }
}
fn synthetic_district_checkpoint(
    count: usize,
    processed: usize,
) -> district_density::MeasurementCheckpoint {
    let samples: Vec<district_density::BuildingSample> = (0..count)
        .map(|index| district_density::BuildingSample {
            id: 10_000 + index as u32,
            footprint: geometry::OrientedRect {
                x: -70.0 + (index % 8) as f32 * 18.0,
                z: -48.0 + (index / 8) as f32 * 18.0,
                width: 10.0,
                depth: 12.0,
                rotation: 0.0,
            },
            floors: 2,
            use_class: district_density::BuildingUse::Housing,
            open: true,
            resident_occupancy: 2,
            capacity: 5,
            business_active: false,
        })
        .collect();
    let mut accumulator = district_density::MeasurementAccumulator::begin(
        district_controller::DISTRICT_GRID,
        &samples,
        &[],
    )
    .unwrap();
    let mut remaining = processed;
    while remaining > 0 {
        let batch = remaining.min(district_density::MAX_MEASUREMENT_SAMPLES_PER_ADVANCE);
        accumulator.advance(batch).unwrap();
        remaining -= batch;
    }
    accumulator.checkpoint().unwrap()
}
fn engine_with_pending_measurement() -> Engine {
    let mut engine = Engine::new_legacy(981);
    engine.state.tick = 1;
    engine.state.district_controller.pending = Some(synthetic_district_checkpoint(40, 16));
    engine.state.district_controller.pending_capture_tick = 1;
    engine.district_runtime =
        district_controller::Runtime::restore(&engine.state.district_controller).unwrap();
    engine
}
