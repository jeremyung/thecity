use crate::*;
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
    pub(crate) fn complete_interior_access_project(&mut self, project_id: u32) {
        let Some(plan) = self.state.legacy.pending_interior_access.clone() else {
            return;
        };
        let Some(project) = self
            .state
            .construction
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
        else {
            return;
        };
        let Some(branch) = self
            .state
            .network
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)
            .cloned()
        else {
            self.keep_interior_project_retryable(project_id);
            return;
        };
        if project.kind != ProjectKind::Road
            || project.from != Some(branch.from)
            || project.to != Some(branch.to)
            || branch.stage == Stage::Open
            || branch.kind == RoadKind::Rail
            || self
                .state
                .land
                .parcels
                .len()
                .saturating_add(plan.selected_lots.len())
                > MAX_PARCELS
            || self
                .state
                .next_id
                .checked_add(plan.selected_lots.len() as u32)
                .is_none()
        {
            self.keep_interior_project_retryable(project_id);
            return;
        }

        let (Some(a), Some(b)) = (self.node(branch.from), self.node(branch.to)) else {
            self.keep_interior_project_retryable(project_id);
            return;
        };
        let (ax, az, bx, bz) = (a.x, a.z, b.x, b.z);
        let mut context = self.collision_context();
        context.pending_roads.retain(|road| {
            !((road.ax == ax && road.az == az && road.bx == bx && road.bz == bz)
                || (road.bx == ax && road.bz == az && road.ax == bx && road.az == bz))
        });
        context.open_roads.push(CollisionSegment {
            ax: a.x,
            az: a.z,
            bx: b.x,
            bz: b.z,
        });
        let mut promised = Vec::with_capacity(plan.selected_lots.len());
        for lot in &plan.selected_lots {
            let rect = world::geometry::OrientedRect {
                x: lot.x,
                z: lot.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            if !self.progressive_frontage_geometry_clear_for_interior(&rect, &context)
                || promised
                    .iter()
                    .any(|other: &world::geometry::OrientedRect| {
                        world::geometry::rects_overlap(&rect, other, 2.0)
                    })
            {
                self.keep_interior_project_retryable(project_id);
                return;
            }
            promised.push(rect);
        }

        if let Some(road) = self
            .state
            .network
            .roads
            .iter_mut()
            .find(|road| road.id == plan.branch_road_id)
        {
            road.stage = Stage::Open;
            road.progress = 1.0;
        }
        self.recompute_connectivity();
        if !self
            .state
            .network
            .roads
            .iter()
            .any(|road| road.id == plan.branch_road_id && road.connected)
        {
            if let Some(road) = self
                .state
                .network
                .roads
                .iter_mut()
                .find(|road| road.id == plan.branch_road_id)
            {
                road.stage = Stage::Survey;
                road.progress = 0.999;
            }
            self.recompute_connectivity();
            self.keep_interior_project_retryable(project_id);
            return;
        }

        self.state.legacy.pending_interior_access = None;
        self.mark_access_geometry_changed();
        for lot in &plan.selected_lots {
            let along_meters = lot.along * branch.length;
            let (Some(from), Some(to)) = (self.node(branch.from), self.node(branch.to)) else {
                unreachable!("preflighted interior access endpoints remain present");
            };
            let access_node = if along_meters <= branch.length * 0.5 {
                from.id
            } else {
                to.id
            };
            let parcel_id =
                self.add_parcel(lot.x, lot.z, lot.width, lot.depth, access_node, 0.72, false);
            assert_ne!(parcel_id, 0, "preflighted access-lot capacity is reserved");
            if let Some(parcel) = self
                .state
                .land
                .parcels
                .iter_mut()
                .find(|p| p.id == parcel_id)
            {
                parcel.rotation = Some(lot.rotation);
            }
            self.state.land.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id: plan.branch_road_id,
                along: lot.along,
                side: lot.side,
            });
            self.mark_access_geometry_changed();
        }
        if let Some(project) = self
            .state
            .construction
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.progress = 1.0;
            project.stage = Stage::Finishing;
        }
        self.state.legacy.road_infill_due = self
            .state
            .legacy
            .road_infill_due
            .saturating_add(plan.selected_lots.len() as u8)
            .min(9);
        self.record(
            EventKind::RoadOpened,
            plan.branch_road_id,
            "A new access street opens with reserved, connected neighborhood frontage".into(),
        );
        #[cfg(test)]
        {
            self.experiments.interior_access_completions = self
                .experiments
                .interior_access_completions
                .saturating_add(1);
        }
        self.state.revision = self.state.revision.saturating_add(1);
        self.trim_completed_projects();
    }

    pub(crate) fn keep_interior_project_retryable(&mut self, project_id: u32) {
        let mut restored_pending_road = false;
        if let Some(project) = self
            .state
            .construction
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            restored_pending_road = project.progress >= 1.0;
            project.progress = 0.999;
            project.stage = Stage::Finishing;
        }
        if restored_pending_road {
            self.mark_access_geometry_changed();
        }
        if let Some(plan) = &self.state.legacy.pending_interior_access {
            if let Some(road) = self
                .state
                .network
                .roads
                .iter_mut()
                .find(|road| road.id == plan.branch_road_id)
            {
                if road.stage != Stage::Open {
                    road.progress = 0.999;
                    road.stage = Stage::Finishing;
                }
            }
        }
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn try_plan_interior_access(&mut self) -> bool {
        let mut scan = self.propose_interior_access();
        self.state.legacy.interior_access_cursor = scan.next_cursor;
        scan.proposals
            .sort_by(|left, right| right.score.compare(left.score));
        for proposal in scan.proposals {
            if self.admit_interior_access(
                proposal.host_id,
                proposal.host_t,
                WorldPoint {
                    x: proposal.end_x,
                    z: proposal.end_z,
                },
                proposal.selected_lots,
            ) {
                return true;
            }
        }
        false
    }

    #[cfg(test)]
    pub(crate) fn try_plan_access_at_road_opening(&mut self, road_id: u32) -> bool {
        if !self.experiments.road_opening_access_experiment
            || self.state.legacy.neighborhood_plan.is_some()
            || self.state.legacy.pending_interior_access.is_some()
            || self.state.legacy.neighborhood_road_ids.contains(&road_id)
        {
            return false;
        }
        let Some(host) = self
            .state
            .network
            .roads
            .iter()
            .find(|road| road.id == road_id)
        else {
            return false;
        };
        if host.stage != Stage::Open
            || !host.connected
            || !matches!(host.kind, RoadKind::Local | RoadKind::Arterial)
        {
            return false;
        }
        let established_roads = self
            .state
            .network
            .roads
            .iter()
            .filter(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
            })
            .count();
        let active_buildings = self
            .state
            .construction
            .buildings
            .iter()
            .filter(|building| building.stage == Stage::Open)
            .count();
        if established_roads < 40 || active_buildings < 8 {
            return false;
        }
        let open_buildings: Vec<_> = self
            .state
            .construction
            .buildings
            .iter()
            .filter(|building| building.stage == Stage::Open)
            .cloned()
            .collect();
        if !self
            .eligible_bridgehead_headings(&open_buildings)
            .is_empty()
        {
            return false;
        }

        self.experiments.road_opening_access_attempts = self
            .experiments
            .road_opening_access_attempts
            .saturating_add(1);
        let mut proposals = if self.experiments.face_conditioned_access_experiment {
            let Some(proposals) = self.face_conditioned_access_proposals(road_id) else {
                return false;
            };
            proposals
        } else {
            self.propose_interior_access_for_host(road_id).proposals
        };
        proposals.sort_by(|left, right| right.score.compare(left.score));
        for proposal in proposals {
            let proposed_host_id = proposal.host_id;
            let proposed_lot_count = proposal.selected_lots.len();
            if self.admit_interior_access(
                proposal.host_id,
                proposal.host_t,
                WorldPoint {
                    x: proposal.end_x,
                    z: proposal.end_z,
                },
                proposal.selected_lots,
            ) {
                if self.experiments.face_conditioned_access_experiment
                    && std::env::var_os("FACE_ACCESS_DEBUG").is_some()
                {
                    eprintln!(
                        "face access admitted trigger={road_id} host={proposed_host_id} lots={proposed_lot_count}"
                    );
                }
                self.experiments.road_opening_access_admissions = self
                    .experiments
                    .road_opening_access_admissions
                    .saturating_add(1);
                self.experiments.road_opening_access_branch = self
                    .state
                    .legacy
                    .pending_interior_access
                    .as_ref()
                    .map(|plan| plan.branch_road_id);
                return true;
            }
        }
        false
    }

    /// Enumerate bounded interior-access proposals without changing simulation
    /// state. The caller commits the returned cursor before attempting admission.
    #[cfg(feature = "legacy")]
    pub(crate) fn propose_interior_access(&self) -> legacy::interior_access::AccessScan {
        self.propose_interior_access_from(None, self.state.legacy.interior_access_cursor)
    }

    #[cfg(any(test, feature = "legacy"))]
    pub(crate) fn propose_interior_access_for_host(
        &self,
        host_id: u32,
    ) -> legacy::interior_access::AccessScan {
        self.propose_interior_access_from(Some(host_id), 0)
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn propose_interior_access_from(
        &self,
        only_host: Option<u32>,
        cursor: usize,
    ) -> legacy::interior_access::AccessScan {
        self.propose_interior_access_from_candidates(only_host, cursor, None)
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn propose_interior_access_from_candidates(
        &self,
        only_host: Option<u32>,
        cursor: usize,
        explicit_candidates: Option<&[legacy::interior_access::AccessCandidate]>,
    ) -> legacy::interior_access::AccessScan {
        self.propose_interior_access_scan(only_host, cursor, explicit_candidates, None)
    }

    #[cfg(test)]
    pub(crate) fn propose_interior_access_from_geometry_candidates(
        &self,
        cursor: usize,
        explicit_candidates: &[legacy::interior_access::AccessGeometryCandidate],
    ) -> legacy::interior_access::AccessScan {
        self.propose_interior_access_scan(None, cursor, None, Some(explicit_candidates))
    }

    #[cfg(feature = "legacy")]
    pub(crate) fn propose_interior_access_scan(
        &self,
        only_host: Option<u32>,
        cursor: usize,
        explicit_candidates: Option<&[legacy::interior_access::AccessCandidate]>,
        geometry_candidates: Option<&[legacy::interior_access::AccessGeometryCandidate]>,
    ) -> legacy::interior_access::AccessScan {
        use legacy::interior_access::{
            AccessProposal, AccessScan, CandidateScore, InteriorAccessLot,
        };

        let unchanged = || legacy::interior_access::AccessScan {
            proposals: Vec::new(),
            next_cursor: cursor,
            #[cfg(test)]
            rejection_counts: [0; 8],
        };

        if self.state.legacy.pending_interior_access.is_some()
            || self.state.legacy.neighborhood_plan.is_some()
            || self.state.people.demand <= 0.8
            || !self.project_admission_available()
            || self
                .state
                .network
                .roads
                .len()
                .saturating_add(self.state.construction.projects.len())
                .saturating_add(3)
                > MAX_SEGMENTS
            || self.state.land.parcels.len().saturating_add(2) > MAX_PARCELS
            || self.state.next_id.checked_add(5).is_none()
        {
            return unchanged();
        }
        if self
            .state
            .construction
            .projects
            .iter()
            .any(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
        {
            return unchanged();
        }

        let explicit_host_ids = explicit_candidates
            .map(|candidates| candidates.iter().map(|candidate| candidate.host_id))
            .into_iter()
            .flatten()
            .chain(
                geometry_candidates
                    .into_iter()
                    .flatten()
                    .map(|candidate| candidate.host_id),
            )
            .collect::<BTreeSet<_>>();
        let explicit_host_ids = (!explicit_host_ids.is_empty()).then_some(explicit_host_ids);
        let mut hosts: Vec<&Road> = self
            .state
            .network
            .roads
            .iter()
            .filter(|road| {
                road.stage == Stage::Open
                    && road.connected
                    && matches!(road.kind, RoadKind::Local | RoadKind::Arterial)
                    && only_host.is_none_or(|host_id| road.id == host_id)
                    && explicit_host_ids
                        .as_ref()
                        .is_none_or(|host_ids| host_ids.contains(&road.id))
            })
            .collect();
        hosts.sort_by_key(|road| road.id);
        if hosts.is_empty() {
            return unchanged();
        }
        let candidate_count = geometry_candidates
            .map(|candidates| candidates.len())
            .or_else(|| explicit_candidates.map(|candidates| candidates.len()))
            .unwrap_or_else(|| {
                hosts
                    .len()
                    .saturating_mul(legacy::interior_access::PROPOSALS_PER_HOST)
            });
        if candidate_count == 0 {
            return unchanged();
        }
        let start_cursor = cursor % candidate_count;
        let inspect_count =
            candidate_count.min(legacy::interior_access::MAX_PROPOSALS_PER_EVALUATION);
        let context = self.collision_context();
        let mut candidates: Vec<AccessProposal> = Vec::new();
        #[cfg(test)]
        let mut rejection_counts = [0usize; 8];
        #[cfg(test)]
        macro_rules! reject_candidate {
            ($bucket:expr) => {{
                rejection_counts[$bucket] += 1;
                continue;
            }};
        }
        let host_indexes =
            (explicit_candidates.is_some() || geometry_candidates.is_some()).then(|| {
                hosts
                    .iter()
                    .enumerate()
                    .map(|(index, road)| (road.id, index))
                    .collect::<BTreeMap<_, _>>()
            });
        for offset in 0..inspect_count {
            let proposal_index = (start_cursor + offset) % candidate_count;
            let (host_index, host_t, fraction_index, side, length_index, explicit_end) =
                if let Some(geometry_candidates) = geometry_candidates {
                    let candidate = geometry_candidates[proposal_index];
                    let Some(host_index) = host_indexes
                        .as_ref()
                        .and_then(|indexes| indexes.get(&candidate.host_id))
                        .copied()
                    else {
                        #[cfg(test)]
                        reject_candidate!(0);
                        #[cfg(not(test))]
                        continue;
                    };
                    (
                        host_index,
                        candidate.host_t,
                        candidate.tie_index as usize,
                        1_i8,
                        0_usize,
                        Some(WorldPoint {
                            x: candidate.end_x,
                            z: candidate.end_z,
                        }),
                    )
                } else if let Some(explicit_candidates) = explicit_candidates {
                    let candidate = explicit_candidates[proposal_index];
                    let Some(host_index) = host_indexes
                        .as_ref()
                        .and_then(|indexes| indexes.get(&candidate.host_id))
                        .copied()
                    else {
                        #[cfg(test)]
                        reject_candidate!(0);
                        #[cfg(not(test))]
                        continue;
                    };
                    (
                        host_index,
                        legacy::interior_access::FRACTIONS
                            .get(candidate.fraction_index as usize)
                            .copied()
                            .unwrap_or(f32::NAN),
                        candidate.fraction_index as usize,
                        candidate.side,
                        candidate.length_index as usize,
                        None,
                    )
                } else {
                    let host_index = proposal_index / legacy::interior_access::PROPOSALS_PER_HOST;
                    let local = proposal_index % legacy::interior_access::PROPOSALS_PER_HOST;
                    let length_index = local % legacy::interior_access::LENGTHS.len();
                    let side_index = (local / legacy::interior_access::LENGTHS.len())
                        % legacy::interior_access::SIDES.len();
                    let fraction_index = local
                        / (legacy::interior_access::LENGTHS.len()
                            * legacy::interior_access::SIDES.len());
                    (
                        host_index,
                        legacy::interior_access::FRACTIONS[fraction_index],
                        fraction_index,
                        legacy::interior_access::SIDES[side_index],
                        length_index,
                        None,
                    )
                };
            let length_from_grid = legacy::interior_access::LENGTHS.get(length_index).copied();
            if !host_t.is_finite()
                || (explicit_end.is_none() && length_from_grid.is_none())
                || (explicit_end.is_none() && !legacy::interior_access::SIDES.contains(&side))
            {
                #[cfg(test)]
                reject_candidate!(0);
                #[cfg(not(test))]
                continue;
            }
            let side_index = usize::from(side == 1);
            let host = hosts[host_index];
            let (Some(host_a), Some(host_b)) = (self.node(host.from), self.node(host.to)) else {
                continue;
            };
            let dx = host_b.x - host_a.x;
            let dz = host_b.z - host_a.z;
            let host_length = dx.hypot(dz);
            if !host_length.is_finite()
                || host_length * host_t < 10.0
                || host_length * (1.0 - host_t) < 10.0
            {
                continue;
            }
            let ux = dx / host_length;
            let uz = dz / host_length;
            let nx = -uz * side as f32;
            let nz = ux * side as f32;
            let start = WorldPoint {
                x: host_a.x + dx * host_t,
                z: host_a.z + dz * host_t,
            };
            let end = explicit_end.unwrap_or_else(|| WorldPoint {
                x: start.x + nx * length_from_grid.unwrap(),
                z: start.z + nz * length_from_grid.unwrap(),
            });
            let length = if explicit_end.is_some() {
                (end.x - start.x).hypot(end.z - start.z)
            } else {
                length_from_grid.unwrap()
            };
            if !length.is_finite() || length < 24.0 - 0.01 || length > 60.0 + 0.01 {
                #[cfg(test)]
                reject_candidate!(0);
                #[cfg(not(test))]
                continue;
            }
            if [start.x, start.z, end.x, end.z]
                .iter()
                .any(|value| !value.is_finite() || *value < MIN + 24.0 || *value > MAX - 24.0)
                || !self.ordinary_road_clear_of_river(start.x, start.z, end.x, end.z, 4.5)
                || corridor_crosses_protected(start.x, start.z, end.x, end.z)
                || !self.road_corridor_clear_of_parcels_with_context(
                    start.x, start.z, end.x, end.z, &context,
                )
            {
                #[cfg(test)]
                reject_candidate!(1);
                #[cfg(not(test))]
                continue;
            }
            let split_x = start.x;
            let split_z = start.z;
            if self.state.network.nodes.iter().any(|node| {
                node.id != host.from
                    && node.id != host.to
                    && (node.x - split_x).hypot(node.z - split_z) < 1.0
            }) || self
                .state
                .network
                .roads
                .iter()
                .filter(|road| road.id != host.id)
                .any(|road| {
                    let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                        return true;
                    };
                    point_segment_distance(split_x, split_z, a.x, a.z, b.x, b.z) < 0.5
                })
            {
                #[cfg(test)]
                reject_candidate!(2);
                #[cfg(not(test))]
                continue;
            }
            let endpoint_too_close = self
                .state
                .network
                .nodes
                .iter()
                .any(|node| (node.x - end.x).hypot(node.z - end.z) < 18.0);
            if endpoint_too_close {
                #[cfg(test)]
                reject_candidate!(3);
                #[cfg(not(test))]
                continue;
            }
            let road_clear = self
                .state
                .network
                .roads
                .iter()
                .filter(|road| road.stage == Stage::Open)
                .all(|road| {
                    if road.id == host.id {
                        return true;
                    }
                    let (Some(a), Some(b)) = (self.node(road.from), self.node(road.to)) else {
                        return false;
                    };
                    world::geometry::segments_distance_coords(
                        start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z,
                    ) >= 8.0
                });
            let projects_clear = self
                .state
                .construction
                .projects
                .iter()
                .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
                .all(|project| {
                    let Some((from, to)) = project.from.zip(project.to) else {
                        return true;
                    };
                    let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
                        return false;
                    };
                    world::geometry::segments_distance_coords(
                        start.x, start.z, end.x, end.z, a.x, a.z, b.x, b.z,
                    ) >= 8.0
                });
            if !road_clear || !projects_clear {
                #[cfg(test)]
                reject_candidate!(4);
                #[cfg(not(test))]
                continue;
            }

            let mut proposed_context = context.clone();
            proposed_context.open_roads.push(CollisionSegment {
                ax: start.x,
                az: start.z,
                bx: end.x,
                bz: end.z,
            });
            let intervals = infrastructure::frontage::subdivide_frontage(
                length,
                &[],
                FRONTAGE_ENDPOINT_CLEARANCE,
                8.0,
                14.0,
                2.2,
                MAX_FRONTAGE_GAP_LOTS_PER_SIDE,
            );
            // Grid candidates travel along the host-normal `nx/nz` already.
            // Explicit geometry candidates can point at any angle, so their
            // frontage must use the actual branch tangent to place lots
            // perpendicular to the constructed road.
            let (lot_tangent_x, lot_tangent_z) = if explicit_end.is_some() {
                ((end.x - start.x) / length, (end.z - start.z) / length)
            } else {
                (nx, nz)
            };
            let mut lots = Vec::new();
            for lot_side in [-1_i8, 1] {
                for interval in &intervals {
                    let along_meters = (interval.start + interval.end) * 0.5;
                    let x = start.x + (end.x - start.x) * (along_meters / length)
                        - lot_tangent_z * lot_side as f32 * 15.0;
                    let z = start.z
                        + (end.z - start.z) * (along_meters / length)
                        + lot_tangent_x * lot_side as f32 * 15.0;
                    let lot = InteriorAccessLot {
                        x,
                        z,
                        width: interval.end - interval.start,
                        depth: 12.0,
                        rotation: (lot_side as f32 * (end.z - start.z))
                            .atan2(-lot_side as f32 * (end.x - start.x)),
                        along: along_meters / length,
                        side: lot_side,
                    };
                    let rect = world::geometry::OrientedRect {
                        x: lot.x,
                        z: lot.z,
                        width: lot.width,
                        depth: lot.depth,
                        rotation: lot.rotation,
                    };
                    if self.progressive_frontage_geometry_clear(&rect, &proposed_context)
                        && lots.len() < legacy::interior_access::MAX_SELECTED_LOTS
                    {
                        lots.push(lot);
                    }
                }
            }
            if lots.len() < 2
                || self.state.land.parcels.len().saturating_add(lots.len()) > MAX_PARCELS
                || self
                    .state
                    .land
                    .parcel_frontages
                    .len()
                    .saturating_add(lots.len())
                    > MAX_PARCELS
            {
                #[cfg(test)]
                reject_candidate!(5);
                #[cfg(not(test))]
                continue;
            }
            let area_cm2 = lots
                .iter()
                .map(|lot| (lot.width * lot.depth * 10_000.0).round() as u32)
                .sum::<u32>();
            let score = CandidateScore {
                site_area: area_cm2,
                road_length_mm: (length * 1_000.0).round() as u32,
                host_id: host.id,
                fraction_index: fraction_index as u8,
                side_index: side_index as u8,
                length_index: length_index as u8,
            };
            candidates.push(AccessProposal {
                score,
                host_id: host.id,
                host_t,
                end_x: end.x,
                end_z: end.z,
                selected_lots: lots,
            });
        }
        AccessScan {
            proposals: candidates,
            next_cursor: (start_cursor + inspect_count) % candidate_count,
            #[cfg(test)]
            rejection_counts,
        }
    }

    /// Commit one geometry-checked proposal through the existing atomic split
    /// preflight. Keeping admission separate lets planners compare proposals
    /// without allocating IDs or changing the network during scoring.
    #[cfg(feature = "legacy")]
    pub(crate) fn admit_interior_access(
        &mut self,
        host_id: u32,
        host_t: f32,
        end: WorldPoint,
        selected_lots: Vec<legacy::interior_access::InteriorAccessLot>,
    ) -> bool {
        // Each proposal has been geometry-checked; split preflight can still
        // reject actor route-cap or frontage remaps. Try the next ranked
        // candidate because split rejection is guaranteed mutation-free.
        if self
            .state
            .network
            .roads
            .len()
            .saturating_add(self.state.construction.projects.len())
            .saturating_add(3)
            > MAX_SEGMENTS
            || self
                .state
                .land
                .parcels
                .len()
                .saturating_add(selected_lots.len())
                > MAX_PARCELS
            || self
                .state
                .land
                .parcel_frontages
                .len()
                .saturating_add(selected_lots.len())
                > MAX_PARCELS
            || self
                .state
                .next_id
                .checked_add(5 + selected_lots.len() as u32)
                .is_none()
            || !self.project_admission_available()
        {
            return false;
        }
        let split = match self.try_split_open_road(host_id, host_t) {
            Ok(split) => split,
            Err(_) => return false,
        };
        self.recompute_connectivity();
        let endpoint_id = self.add_node(end.x, end.z, NodeKind::Junction);
        let branch_road_id = self.add_road(
            split.split_node_id,
            endpoint_id,
            RoadKind::Local,
            Stage::Survey,
            0.0,
        );
        let project_id = self.id();
        let project = Project {
            id: project_id,
            kind: ProjectKind::Road,
            from: Some(split.split_node_id),
            to: Some(endpoint_id),
            parcel_id: None,
            stage: Stage::Survey,
            progress: 0.0,
            reason: "A neighborhood access street is being built from an established corridor"
                .into(),
            duration: (3.0 + ((host_id ^ self.state.seed) % 400) as f32 / 100.0).clamp(3.0, 7.0),
            started_tick: self.state.tick,
        };
        self.record_project(project);
        self.state.legacy.pending_interior_access =
            Some(legacy::interior_access::InteriorAccessPlan {
                host_road_id: host_id,
                host_second_half_road_id: split.new_road_id,
                host_t,
                split_node_id: split.split_node_id,
                branch_endpoint_id: endpoint_id,
                branch_road_id,
                project_id,
                selected_lots,
            });
        self.mark_access_geometry_changed();
        self.state.revision = self.state.revision.saturating_add(1);
        true
    }
}
