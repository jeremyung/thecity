//! Atomic, read-only projection from the current Engine state into the
//! immutable input used by `ProposalValidator`.
//!
//! This capture is synchronous at one simulation boundary. It owns the exact
//! source-ordered geometry needed by a later quota-stepped proposal search;
//! it does not truncate any valid engine vector to the face-discovery limits.

use crate::legacy::access_proposal_validation::{
    AdmissionGates, ProjectEndpoints, ProposalHost, ProposalNode, ProposalProject, ProposalRoad,
    ProposalValidationInput,
};
use crate::legacy::face_target_viability::{FrozenViabilityInput, Segment};
use crate::{Engine, MAX_PARCELS, MAX_PROJECTS, MAX_SEGMENTS};
use std::collections::BTreeMap;

#[cfg(feature = "legacy")]
const MAX_NODES: usize = crate::legacy::access_candidates::MAX_PREPARED_INPUT_NODES;
#[cfg(feature = "legacy")]
const MAX_PROJECT_RECORDS: usize = MAX_PROJECTS + crate::COMPLETED_PROJECT_HISTORY;
#[cfg(feature = "legacy")]
const MAX_NEIGHBORHOOD_SEGMENTS: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(feature = "legacy")]
pub(crate) enum CaptureError {
    TooManyNodes,
    TooManyRoads,
    TooManyProjects,
    TooManyParcels,
    TooManyFootprints,
    TooManyFrontages,
    TooManyNeighborhoodSegments,
    TooManyReservedLots,
    TooManyPendingRoads,
}

/// Capture every road/node/project and the complete collision projection in
/// source order. Eligible hosts alone are sorted by road id, matching the
/// live proposal evaluator. The engine remains unchanged on success or error.
#[cfg(feature = "legacy")]
pub(crate) fn capture(engine: &Engine) -> Result<ProposalValidationInput, CaptureError> {
    capture_inner(engine, None)
}

#[cfg(feature = "legacy")]
pub(crate) fn capture_with_viability(
    captured: crate::legacy::access_capture::CapturedViability<'_>,
) -> Result<ProposalValidationInput, CaptureError> {
    let (engine, viability) = captured.into_parts();
    capture_inner(engine, Some(viability))
}

#[cfg(feature = "legacy")]
fn capture_inner(
    engine: &Engine,
    shared_viability: Option<FrozenViabilityInput>,
) -> Result<ProposalValidationInput, CaptureError> {
    let state = &engine.state;
    if state.nodes.len() > MAX_NODES {
        return Err(CaptureError::TooManyNodes);
    }
    if state.roads.len() > MAX_SEGMENTS {
        return Err(CaptureError::TooManyRoads);
    }
    if state.projects.len() > MAX_PROJECT_RECORDS {
        return Err(CaptureError::TooManyProjects);
    }
    if state.parcels.len() > MAX_PARCELS {
        return Err(CaptureError::TooManyParcels);
    }
    if state.parcel_frontages.len() > MAX_PARCELS {
        return Err(CaptureError::TooManyFrontages);
    }
    let footprint_count = state.parcels.iter().fold(0_usize, |count, parcel| {
        count.saturating_add(1 + usize::from(parcel.rotation.is_none()))
    });
    if footprint_count > MAX_PARCELS.saturating_mul(2) {
        return Err(CaptureError::TooManyFootprints);
    }
    if state.neighborhood_plan.as_ref().is_some_and(|plan| {
        plan.segments.len().saturating_sub(plan.next_segment) > MAX_NEIGHBORHOOD_SEGMENTS
    }) {
        return Err(CaptureError::TooManyNeighborhoodSegments);
    }
    if state
        .pending_interior_access
        .as_ref()
        .is_some_and(|plan| plan.selected_lots.len() > MAX_PARCELS)
    {
        return Err(CaptureError::TooManyReservedLots);
    }
    if state
        .projects
        .iter()
        .filter(|project| project.kind == "road" && project.progress < 1.0)
        .count()
        > MAX_PROJECTS
    {
        return Err(CaptureError::TooManyPendingRoads);
    }

    // Engine::node resolves duplicate ids to their first source occurrence.
    // Build that index once rather than repeating a linear scan per road or
    // project. BTreeMap preserves deterministic construction and lookup.
    let mut node_positions = BTreeMap::new();
    for node in &state.nodes {
        node_positions.entry(node.id).or_insert((node.x, node.z));
    }

    let mut hosts = state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && matches!(road.kind.as_str(), "local" | "arterial")
        })
        .filter_map(|road| {
            let (ax, az) = *node_positions.get(&road.from)?;
            let (bx, bz) = *node_positions.get(&road.to)?;
            Some(ProposalHost {
                id: road.id,
                from: road.from,
                to: road.to,
                segment: Segment { ax, az, bx, bz },
            })
        })
        .collect::<Vec<_>>();
    hosts.sort_by_key(|host| host.id);

    let nodes = state
        .nodes
        .iter()
        .map(|node| ProposalNode {
            id: node.id,
            x: node.x,
            z: node.z,
        })
        .collect::<Vec<_>>();

    let roads = state
        .roads
        .iter()
        .map(|road| ProposalRoad {
            id: road.id,
            is_open: road.stage == "open",
            segment: node_positions
                .get(&road.from)
                .zip(node_positions.get(&road.to))
                .map(|(&(ax, az), &(bx, bz))| Segment { ax, az, bx, bz }),
        })
        .collect::<Vec<_>>();

    let projects = state
        .projects
        .iter()
        .map(|project| {
            let endpoints = match project.from.zip(project.to) {
                None => ProjectEndpoints::NoIds,
                Some((from, to)) => node_positions
                    .get(&from)
                    .zip(node_positions.get(&to))
                    .map(|(&(ax, az), &(bx, bz))| {
                        ProjectEndpoints::Segment(Segment { ax, az, bx, bz })
                    })
                    .unwrap_or(ProjectEndpoints::MissingNode),
            };
            ProposalProject {
                active_road: project.kind == "road" && project.progress < 1.0,
                endpoints,
            }
        })
        .collect::<Vec<_>>();

    let pending_access_branch = state.pending_interior_access.as_ref().and_then(|plan| {
        let road = state
            .roads
            .iter()
            .find(|road| road.id == plan.branch_road_id)?;
        let (ax, az) = *node_positions.get(&road.from)?;
        let (bx, bz) = *node_positions.get(&road.to)?;
        Some(Segment { ax, az, bx, bz })
    });

    let viability = shared_viability
        .unwrap_or_else(|| crate::legacy::access_capture::capture_viability(engine));

    Ok(ProposalValidationInput {
        hosts,
        nodes,
        roads,
        projects,
        viability,
        gates: AdmissionGates {
            demand: state.demand,
            has_pending_interior_plan: state.pending_interior_access.is_some(),
            has_neighborhood_plan: state.neighborhood_plan.is_some(),
            road_count: state.roads.len(),
            project_count: state.projects.len(),
            parcel_count: state.parcels.len(),
            frontage_count: state.parcel_frontages.len(),
            next_id: state.next_id,
            project_admission_available: engine.project_admission_available(),
            pending_access_branch,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mature() -> Engine {
        Engine::restore_json(include_str!(
            "../../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json"
        ))
        .expect("mature fixture restores")
    }

    fn assert_matches_previous_projection(engine: &Engine) {
        let segment = |from, to| {
            engine
                .node(from)
                .zip(engine.node(to))
                .map(|(a, b)| Segment {
                    ax: a.x,
                    az: a.z,
                    bx: b.x,
                    bz: b.z,
                })
        };
        let mut hosts = engine
            .state
            .roads
            .iter()
            .filter(|road| {
                road.stage == "open"
                    && road.connected
                    && matches!(road.kind.as_str(), "local" | "arterial")
            })
            .filter_map(|road| {
                segment(road.from, road.to).map(|segment| ProposalHost {
                    id: road.id,
                    from: road.from,
                    to: road.to,
                    segment,
                })
            })
            .collect::<Vec<_>>();
        hosts.sort_by_key(|host| host.id);
        let expected = ProposalValidationInput {
            hosts,
            nodes: engine
                .state
                .nodes
                .iter()
                .map(|node| ProposalNode {
                    id: node.id,
                    x: node.x,
                    z: node.z,
                })
                .collect(),
            roads: engine
                .state
                .roads
                .iter()
                .map(|road| ProposalRoad {
                    id: road.id,
                    is_open: road.stage == "open",
                    segment: segment(road.from, road.to),
                })
                .collect(),
            projects: engine
                .state
                .projects
                .iter()
                .map(|project| ProposalProject {
                    active_road: project.kind == "road" && project.progress < 1.0,
                    endpoints: match project.from.zip(project.to) {
                        None => ProjectEndpoints::NoIds,
                        Some((from, to)) => segment(from, to)
                            .map(ProjectEndpoints::Segment)
                            .unwrap_or(ProjectEndpoints::MissingNode),
                    },
                })
                .collect(),
            viability: crate::legacy::access_capture::capture(engine)
                .unwrap()
                .viability,
            gates: AdmissionGates {
                demand: engine.state.demand,
                has_pending_interior_plan: engine.state.pending_interior_access.is_some(),
                has_neighborhood_plan: engine.state.neighborhood_plan.is_some(),
                road_count: engine.state.roads.len(),
                project_count: engine.state.projects.len(),
                parcel_count: engine.state.parcels.len(),
                frontage_count: engine.state.parcel_frontages.len(),
                next_id: engine.state.next_id,
                project_admission_available: engine.project_admission_available(),
                pending_access_branch: engine.state.pending_interior_access.as_ref().and_then(
                    |plan| {
                        engine
                            .state
                            .roads
                            .iter()
                            .find(|road| road.id == plan.branch_road_id)
                            .and_then(|road| segment(road.from, road.to))
                    },
                ),
            },
        };
        assert_eq!(capture(engine).unwrap(), expected);
    }

    #[test]
    fn initial_and_mature_capture_match_the_existing_geometry_projection() {
        let initial = Engine::new_legacy(17);
        let before = initial.save_json();
        let captured = capture(&initial).unwrap();
        assert_eq!(
            captured.viability,
            crate::legacy::access_capture::capture(&initial)
                .unwrap()
                .viability
        );
        assert_matches_previous_projection(&initial);
        assert_eq!(initial.save_json(), before);

        let mature = mature();
        let before = mature.save_json();
        let captured = capture(&mature).unwrap();
        assert_eq!(
            captured.viability,
            crate::legacy::access_capture::capture(&mature)
                .unwrap()
                .viability
        );
        assert_matches_previous_projection(&mature);
        assert_eq!(mature.save_json(), before);
        assert!(captured
            .hosts
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id));
    }

    #[test]
    fn capture_supports_more_than_face_tracer_host_limit_without_truncation() {
        let mut engine = Engine::new_legacy(7);
        let mut next_id = engine.state.next_id;
        for index in 0..513 {
            let from = next_id;
            next_id += 1;
            let to = next_id;
            next_id += 1;
            engine.state.nodes.push(crate::Node {
                id: from,
                x: -300.0 + (index % 25) as f32 * 10.0,
                z: -300.0 + (index / 25) as f32 * 10.0,
                kind: "junction".into(),
            });
            engine.state.nodes.push(crate::Node {
                id: to,
                x: -299.0 + (index % 25) as f32 * 10.0,
                z: -300.0 + (index / 25) as f32 * 10.0,
                kind: "junction".into(),
            });
            engine.state.roads.push(crate::Road {
                id: next_id,
                from,
                to,
                kind: "local".into(),
                stage: "open".into(),
                progress: 1.0,
                connected: true,
                length: 1.0,
                lanes: 2,
            });
            next_id += 1;
        }
        engine.state.next_id = next_id;
        let captured = capture(&engine).unwrap();
        assert!(captured.hosts.len() >= 513);
        assert_eq!(captured.roads.len(), engine.state.roads.len());
        assert!(
            crate::legacy::access_proposal_validation::ProposalValidationContext::new(captured)
                .is_ok()
        );
    }

    #[test]
    fn over_cap_state_fails_before_projection_and_does_not_mutate_engine() {
        let mut engine = Engine::new_legacy(19);
        for index in 0..=MAX_NODES {
            engine.state.nodes.push(crate::Node {
                id: 100_000 + index as u32,
                x: index as f32,
                z: 0.0,
                kind: "junction".into(),
            });
        }
        let before = engine.save_json();
        assert_eq!(capture(&engine), Err(CaptureError::TooManyNodes));
        assert_eq!(engine.save_json(), before);
    }
}
