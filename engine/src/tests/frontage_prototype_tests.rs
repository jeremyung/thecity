//! Offline single-frontage fixture prototype. Never used by city generation.
use super::*;

#[test]
#[ignore = "requires matching completed candidate save and writes admission diagnostic"]
#[cfg(feature = "block-proportion-candidate")]
fn parent_passage_runtime_admission_probe() {
    let raw = std::fs::read_to_string(std::env::var("PASSAGE_SAVE").unwrap()).unwrap();
    let engine = crate::Engine::restore_json(&raw).expect("matching mature candidate restores");
    let before = engine.save_json();
    let witness: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../validation/parent-passage-connection.json"
        ))
        .expect("local evidence file; not versioned"),
    )
    .unwrap();
    let fit = &witness["largestAlternativeRectangle"];
    let start = &witness["entry"]["point"];
    let target: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../validation/parent-block-access.json"
        ))
        .expect("local evidence file; not versioned"),
    )
    .unwrap();
    let dx = (target["target"][0].as_f64().unwrap() - start[0].as_f64().unwrap()) as f32;
    let dz = (target["target"][1].as_f64().unwrap() - start[1].as_f64().unwrap()) as f32;
    let side = fit["side"].as_f64().unwrap() as f32;
    let rect = crate::geometry::OrientedRect {
        x: fit["center"][0].as_f64().unwrap() as f32,
        z: fit["center"][1].as_f64().unwrap() as f32,
        width: fit["width"].as_f64().unwrap() as f32,
        depth: fit["depth"].as_f64().unwrap() as f32,
        rotation: (side * dz).atan2(-side * dx),
    };
    let road = engine
        .state
        .network
        .roads
        .iter()
        .find(|r| r.id == witness["runtimeRoadId"].as_u64().unwrap() as u32)
        .unwrap();
    let a = engine.node(road.from).unwrap();
    let b = engine.node(road.to).unwrap();
    let road_side = if (b.x - a.x) * (rect.z - a.z) - (b.z - a.z) * (rect.x - a.x) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let facing = (road_side * (b.z - a.z)).atan2(-road_side * (b.x - a.x));
    let snapped = crate::geometry::OrientedRect {
        rotation: facing,
        ..rect
    };
    let road_conflicts = |r: &crate::geometry::OrientedRect| {
        engine
            .state
            .network
            .roads
            .iter()
            .filter(|road| road.stage == crate::Stage::Open)
            .filter_map(|road| {
                let a = engine.node(road.from)?;
                let b = engine.node(road.to)?;
                let distance = crate::geometry::rect_segment_distance(r, a.x, a.z, b.x, b.z);
                (distance < 5.0).then(
                    || serde_json::json!({"road":road.id,"kind":road.kind,"distance":distance}),
                )
            })
            .collect::<Vec<_>>()
    };
    let parcel_conflicts = |r: &crate::geometry::OrientedRect| {
        engine
            .state
            .land
            .parcels
            .iter()
            .filter(|p| crate::geometry::rects_overlap(r, &crate::parcel_envelope(p), 2.0))
            .map(|p| p.id)
            .collect::<Vec<_>>()
    };
    let report = serde_json::json!({"runtimeRestores":true,"road":road.id,
        "passageFacingRotation":rect.rotation,"roadFacingRotation":facing,
        "passageFacingGenericGeometryClear":engine.oriented_parcel_geometry_clear(&rect),
        "roadFacingGenericGeometryClear":engine.oriented_parcel_geometry_clear(&snapped),
        "passageFacingRoadConflicts":road_conflicts(&rect),"roadFacingRoadConflicts":road_conflicts(&snapped),
        "passageFacingParcelConflicts":parcel_conflicts(&rect),"roadFacingParcelConflicts":parcel_conflicts(&snapped),
        "limitations":["Generic infill geometry checks differ from pure master-plan admission.",
        "No parcel was inserted or journey admitted; release_plan_lots currently snaps to road facing."]});
    assert!(
        !engine.oriented_parcel_geometry_clear(&rect),
        "geometric fit is not generic admission"
    );
    assert!(
        !engine.oriented_parcel_geometry_clear(&snapped),
        "road snapping is not a valid workaround"
    );
    assert!(!parcel_conflicts(&rect).is_empty());
    assert!(!road_conflicts(&snapped).is_empty());
    assert_eq!(
        before,
        engine.save_json(),
        "read-only diagnostic must not mutate the city"
    );
    std::fs::write(
        std::env::var("PASSAGE_ADMISSION_REPORT").unwrap(),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
