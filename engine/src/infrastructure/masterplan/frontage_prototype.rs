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
    let rect = crate::world::geometry::OrientedRect {
        x: fit["center"][0].as_f64().unwrap() as f32,
        z: fit["center"][1].as_f64().unwrap() as f32,
        width: fit["width"].as_f64().unwrap() as f32,
        depth: fit["depth"].as_f64().unwrap() as f32,
        rotation: (side * dz).atan2(-side * dx),
    };
    let road = engine
        .state
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
    let snapped = crate::world::geometry::OrientedRect {
        rotation: facing,
        ..rect
    };
    let road_conflicts = |r: &crate::world::geometry::OrientedRect| {
        engine
            .state
            .roads
            .iter()
            .filter(|road| road.stage == crate::Stage::Open)
            .filter_map(|road| {
                let a = engine.node(road.from)?;
                let b = engine.node(road.to)?;
                let distance = crate::world::geometry::rect_segment_distance(r, a.x, a.z, b.x, b.z);
                (distance < 5.0).then(
                    || serde_json::json!({"road":road.id,"kind":road.kind,"distance":distance}),
                )
            })
            .collect::<Vec<_>>()
    };
    let parcel_conflicts = |r: &crate::world::geometry::OrientedRect| {
        engine
            .state
            .parcels
            .iter()
            .filter(|p| crate::world::geometry::rects_overlap(r, &crate::parcel_envelope(p), 2.0))
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

// Finite reservation only: passage edges are not public-road frontage.
// Split off each exterior half-plane before continuing with the remainder,
// so the returned convex pieces cannot overlap one another.
fn reserve_passage(poly: &Poly, start: P, end: P, width: f32) -> (Vec<Poly>, Poly) {
    assert!(end.sub(start).len() > 0.01 && width > 0.0);
    let u = end.sub(start).unit();
    let v = P::new(-u.z, u.x);
    let planes = [
        (start, u),
        (end, u.scale(-1.0)),
        (start.sub(v.scale(width / 2.0)), v),
        (start.add(v.scale(width / 2.0)), v.scale(-1.0)),
    ];
    let mut reserved = poly.clone();
    for (c, n) in planes {
        reserved = clip(&reserved, c, n, None);
    }
    if reserved.len() < 3 || area(&reserved) == 0.0 {
        return (vec![poly.clone()], Vec::new());
    }
    let mut remainder = poly.clone();
    let mut pieces = Vec::new();
    for (c, n) in planes {
        let outside = clip(&remainder, c, n.scale(-1.0), None);
        if outside.len() >= 3 && area(&outside) > 0.0 {
            pieces.push(outside);
        }
        remainder = clip(&remainder, c, n, None);
    }
    (pieces, reserved)
}

#[test]
fn passage_is_finite_and_does_not_invent_frontage() {
    let poly = vec![
        (P::new(0.0, 0.0), Some(61)),
        (P::new(20.0, 0.0), None),
        (P::new(20.0, 20.0), None),
        (P::new(0.0, 20.0), None),
    ];
    let (parts, reserved) = reserve_passage(&poly, P::new(10.0, 5.0), P::new(10.0, 15.0), 2.2);
    assert!((area(&reserved) - 22.0).abs() < 0.001);
    assert!((parts.iter().map(area).sum::<f32>() - 378.0).abs() < 0.001);
    for part in parts {
        for (i, (p, label)) in part.iter().enumerate() {
            if label.is_some() {
                assert_eq!(*label, Some(61));
                assert_eq!(p.z, 0.0);
                assert_eq!(part[(i + 1) % part.len()].0.z, 0.0);
            }
        }
    }
    let (untouched, reserved) = reserve_passage(&poly, P::new(30.0, 5.0), P::new(30.0, 15.0), 2.2);
    assert!(reserved.is_empty());
    assert_eq!(untouched.len(), 1);
    for (actual, expected) in untouched[0].iter().zip(&poly) {
        assert_eq!(
            (actual.0.x, actual.0.z, actual.1),
            (expected.0.x, expected.0.z, expected.1)
        );
    }
}

#[test]
#[ignore = "exports offline parent reservation for independent geometry checks"]
fn export_parent_passage_reservation() {
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../validation/fixtures/seed0-parent-block-reserved.json"
        ))
        .expect("local evidence file; not versioned"),
    )
    .unwrap();
    let witness: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../validation/parent-block-access.json"
        ))
        .expect("local evidence file; not versioned"),
    )
    .unwrap();
    let point = |v: &serde_json::Value| {
        P::new(v[0].as_f64().unwrap() as f32, v[1].as_f64().unwrap() as f32)
    };
    let encode = |p: &Poly| {
        p.iter()
            .map(|(p, line)| serde_json::json!({"point":[p.x,p.z],"line":line}))
            .collect::<Vec<_>>()
    };
    let mut routes = Vec::new();
    for entry in witness["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["straightRouteClear"] == true)
    {
        let start = point(&entry["point"]);
        let end = point(&witness["target"]);
        let mut pieces = Vec::new();
        for input in fixture["pieces"].as_array().unwrap() {
            let poly: Poly = input["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (point(&v["point"]), v["line"].as_u64().map(|x| x as usize)))
                .collect();
            let (remaining, reserved) = reserve_passage(&poly, start, end, 2.2);
            assert!(
                (remaining.iter().map(area).sum::<f32>() + area(&reserved) - area(&poly)).abs()
                    < 0.02
            );
            pieces.push(serde_json::json!({"block":input["block"],"remaining":remaining.iter().map(&encode).collect::<Vec<_>>(),"reserved":encode(&reserved)}));
        }
        routes.push(serde_json::json!({"entry":entry,"pieces":pieces}));
    }
    let report = serde_json::json!({"widthM":2.2,"target":witness["target"],"routes":routes,
        "status":"Offline reservation only; no road connection, parcel admission or visual acceptance."});
    std::fs::write(
        std::env::var("PASSAGE_REPORT").unwrap(),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}

fn cells(poly: &Poly, edge: usize) -> Vec<Poly> {
    let a = poly[edge].0;
    let delta = poly[(edge + 1) % poly.len()].0.sub(a);
    let length = delta.len();
    let u = delta.unit();
    // Balanced compact frontages; no extrapolation beyond either street end.
    let count = (length / 7.1).round().max(1.0) as usize;
    (0..count)
        .map(|i| {
            let lo = a.add(u.scale(length * i as f32 / count as f32));
            let hi = a.add(u.scale(length * (i + 1) as f32 / count as f32));
            clip(&clip(poly, lo, u, None), hi, u.scale(-1.0), None)
        })
        .filter(|p| p.len() >= 3)
        .collect()
}

#[test]
fn finite_frontage_cells_cover_rectangle_without_invented_edges() {
    let poly = vec![
        (P::new(0.0, 0.0), Some(61)),
        (P::new(28.0, 0.0), None),
        (P::new(28.0, 20.0), None),
        (P::new(0.0, 20.0), None),
    ];
    let parts = cells(&poly, 0);
    assert_eq!(parts.len(), 4);
    assert!((parts.iter().map(area).sum::<f32>() - 560.0).abs() < 0.01);
    for part in &parts {
        assert_eq!(part.iter().filter(|(_, label)| label.is_some()).count(), 1);
        let (_, w, d, _) = inscribed_rect_frame(
            part,
            P::new(0.0, 0.0),
            P::new(1.0, 0.0),
            P::new(0.0, 1.0),
            28.0,
        )
        .unwrap();
        assert!((w - 7.0).abs() < 0.01 && d >= 19.9);
    }
}

#[test]
#[ignore = "writes offline comparison report"]
fn export_reserved_wedge_prototype() {
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../validation/fixtures/seed0-block76-frontage.json"
        ))
        .expect("local evidence file; not versioned"),
    )
    .unwrap();
    let row = &fixture["rows"][0];
    let poly: Poly = row["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                P::new(
                    v["point"][0].as_f64().unwrap() as f32,
                    v["point"][1].as_f64().unwrap() as f32,
                ),
                v["line"].as_u64().map(|n| n as usize),
            )
        })
        .collect();
    let edges: Vec<_> = (0..poly.len()).filter(|&i| poly[i].1.is_some()).collect();
    assert_eq!(
        edges.len(),
        1,
        "prototype only covers a single real street edge"
    );
    let edge = edges[0];
    let a = poly[edge].0;
    let u = poly[(edge + 1) % poly.len()].0.sub(a).unit();
    let v = P::new(-u.z, u.x);
    let parts = cells(&poly, edge);
    let fitted: Vec<_> = parts
        .iter()
        .filter_map(|p| inscribed_rect_frame(p, a, u, v, 28.0))
        .map(|(c, w, d, _)| serde_json::json!({"center":[c.x,c.z],"width":w,"depth":d,"area":w*d}))
        .collect();
    let mut baseline = Vec::new();
    let mut losses = Vec::new();
    let mut rng = Rng(17);
    partition_block(
        &poly,
        District::Compact,
        76,
        &|_| false,
        &mut rng,
        &mut baseline,
        false,
        &mut losses,
    );
    let report = serde_json::json!({"fixture":"seed0-block76-frontage.json",
        "blockArea":area(&poly),"cellArea":parts.iter().map(area).sum::<f32>(),
        "residualOutsideCells":area(&poly)-parts.iter().map(area).sum::<f32>(),
        "cells":parts.iter().map(|p|p.iter().map(|(p,_)|[p.x,p.z]).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "fitted":fitted,"fixtureBaselineArea":baseline.iter().map(|p|p.lot.width*p.lot.depth).sum::<f32>(),
        "fixtureBaselineLots":baseline.len(),"baselineRng":17,
        "limitations":["Fixture baseline uses fixed RNG17, not the original city RNG at this block.",
        "Fits precede final road, geography and elevated clearance admission.",
        "Residual requires explicit access or open-space design; it is not assigned fictitious frontage.",
        "Single-frontage offline prototype, not a production generator or multi-edge corner solution."]});
    let path = std::env::var("FRONTAGE_REPORT").expect("absolute FRONTAGE_REPORT required");
    std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
}
