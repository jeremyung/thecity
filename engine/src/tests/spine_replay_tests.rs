//! Integration regressions owned by the primary review, using normal engine ticks.
use crate::spine_plan::{SpinePlan, SpineStage, RECIPE_VERSION};
use crate::*;

fn reserved_rectangle(depth: f32) -> (Engine, [u32; 2]) {
    let mut engine = Engine::new_legacy(48123);
    let gateway = engine
        .state
        .nodes
        .iter()
        .find(|n| n.kind == "gateway")
        .unwrap()
        .id;
    let points = [
        (270.0, 200.0),
        (352.0, 200.0),
        (352.0, 200.0 + depth),
        (270.0, 200.0 + depth),
    ];
    let nodes = points.map(|(x, z)| engine.add_node(x, z, "junction"));
    engine.add_road(gateway, nodes[0], "local", "open", 1.0);
    let roads: Vec<u32> = (0..4)
        .map(|i| engine.add_road(nodes[i], nodes[(i + 1) % 4], "local", "open", 1.0))
        .collect();
    engine.recompute_connectivity();
    engine
        .state
        .neighborhood_road_ids
        .extend(roads.iter().copied());
    engine.state.neighborhood_plan = Some(NeighborhoodPlan {
        anchor_node: nodes[0],
        segments: (0..4).map(|i| (nodes[i], nodes[(i + 1) % 4])).collect(),
        next_segment: 4,
        coordinated_lots: true,
        selected_lots: None,
        spine: Some(SpinePlan {
            corners: points.map(|(x, z)| blocks::BlockPoint { x, z }),
            recipe_version: RECIPE_VERSION,
            stage: SpineStage::Reserved,
        }),
    });
    (engine, [roads[3], roads[1]])
}

#[test]
fn spine_normal_ticks_match_uninterrupted_from_every_saved_stage() {
    for depth in [82.0, 110.0] {
        for stage in 0..4 {
            let (mut uninterrupted, hosts) = reserved_rectangle(depth);
            if stage == 1 || stage == 2 {
                uninterrupted
                    .state
                    .neighborhood_plan
                    .as_mut()
                    .unwrap()
                    .spine
                    .as_mut()
                    .unwrap()
                    .stage = SpineStage::PerimeterReady {
                    host_road_ids: hosts,
                };
            }
            if stage == 2 {
                assert!(uninterrupted.advance_neighborhood_spine());
            }
            if stage == 3 {
                // The real perimeter scheduler creates a Project first; there is
                // no Road record until that project opens the final edge.
                let index = uninterrupted
                    .state
                    .roads
                    .iter()
                    .position(|r| r.id == hosts[0])
                    .unwrap();
                let road = uninterrupted.state.roads.remove(index);
                uninterrupted.state.neighborhood_road_ids.remove(&road.id);
                let id = uninterrupted.id();
                uninterrupted.record_project(Project {
                    id,
                    kind: "road".into(),
                    from: Some(road.from),
                    to: Some(road.to),
                    parcel_id: None,
                    stage: "survey".into(),
                    progress: 0.0,
                    reason: "Final reserved perimeter street".into(),
                    duration: 3.0,
                    started_tick: uninterrupted.state.tick,
                });
                uninterrupted.recompute_connectivity();
            }
            uninterrupted.validate().unwrap();
            let checkpoint = uninterrupted.save_json();
            let mut resumed = Engine::restore_json(&checkpoint).unwrap();
            assert_eq!(
                resumed.save_json(),
                checkpoint,
                "stage {stage} canonical restore"
            );
            let initial_parcels = uninterrupted.state.parcels.len();
            let mut finished = false;
            for _ in 0..120 {
                uninterrupted.step(100);
                resumed.step(100);
                assert_eq!(
                    uninterrupted.save_json(),
                    resumed.save_json(),
                    "stage {stage} tick {}",
                    resumed.state.tick
                );
                resumed.validate().unwrap();
                if resumed.state.neighborhood_plan.is_none() {
                    finished = true;
                    break;
                }
            }
            assert!(
                finished,
                "stage {stage} must complete through the normal scheduler"
            );
            assert!(resumed.state.parcels.len() >= initial_parcels + 20);
            let new_frontages = resumed
                .state
                .parcel_frontages
                .iter()
                .filter(|f| {
                    resumed.state.parcels.iter().any(|p| {
                        p.id == f.parcel_id
                            && p.x > 270.0
                            && p.x < 352.0
                            && p.z > 200.0
                            && p.z < 200.0 + depth
                    })
                })
                .count();
            assert_eq!(
                new_frontages, 20,
                "the reserved block must admit its whole canonical layout"
            );
        }
    }
}
