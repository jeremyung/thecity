//! Ignored, test-only geometry diagnostic for hypothetical rear parcel extensions.
//!
//! This does not admit redevelopment or change the saved world. It asks whether
//! the exact parcel geometry gates would allow additional rear lot depth while
//! retaining each existing road-facing edge.

use super::{geometry, Building, CollisionContext, Engine, Parcel, ParcelFrontage};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const DEPTH_CAP: f32 = 32.0;
const MAX_EXAMPLES: usize = 12;

#[derive(Clone)]
struct Candidate {
    parcel: Parcel,
    building: Building,
    frontage: ParcelFrontage,
    road_from: (f32, f32),
    road_to: (f32, f32),
    road_point: (f32, f32),
    outward: (f32, f32),
    rect: geometry::OrientedRect,
}

fn source_path() -> PathBuf {
    std::env::var_os("CITY_TRACE_SAVE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(".tools/narrow-frontage-review/final-save.json")
        })
}

fn projected_road_point(center: (f32, f32), from: (f32, f32), to: (f32, f32)) -> (f32, f32) {
    let dx = to.0 - from.0;
    let dz = to.1 - from.1;
    let length2 = dx * dx + dz * dz;
    let t = if length2 <= f32::EPSILON {
        0.0
    } else {
        (((center.0 - from.0) * dx + (center.1 - from.1) * dz) / length2).clamp(0.0, 1.0)
    };
    (from.0 + dx * t, from.1 + dz * t)
}

fn oriented_candidate(
    parcel: &Parcel,
    frontage: &ParcelFrontage,
    road_from: (f32, f32),
    road_to: (f32, f32),
) -> Option<CandidateGeometry> {
    let rotation = parcel.rotation?;
    let (sin, cos) = rotation.sin_cos();
    // Match geometry::OrientedRect's Three.js rotation convention exactly:
    // local width is (cos, -sin), local depth is (sin, cos).
    let width_axis = (cos, -sin);
    let depth_axis = (sin, cos);
    let road_point = projected_road_point((parcel.x, parcel.z), road_from, road_to);
    let center_from_road = (parcel.x - road_point.0, parcel.z - road_point.1);
    let road_dx = road_to.0 - road_from.0;
    let road_dz = road_to.1 - road_from.1;
    let road_length = road_dx.hypot(road_dz);
    if road_length <= f32::EPSILON || !matches!(frontage.side, -1 | 1) {
        return None;
    }
    let expected_front = (
        -road_dz / road_length * frontage.side as f32,
        road_dx / road_length * frontage.side as f32,
    );
    let expected_offset =
        center_from_road.0 * expected_front.0 + center_from_road.1 * expected_front.1;
    let frame_alignment = (depth_axis.0 * expected_front.0 + depth_axis.1 * expected_front.1).abs();
    let side = (center_from_road.0 * depth_axis.0 + center_from_road.1 * depth_axis.1).signum();
    if side == 0.0 {
        return None;
    }
    let depth_offset = center_from_road.0 * depth_axis.0 + center_from_road.1 * depth_axis.1;
    let lateral_offset = center_from_road.0 * width_axis.0 + center_from_road.1 * width_axis.1;
    // A frontage should lie across the parcel's width axis and on its front
    // half. Reject ambiguous/misaligned metadata rather than guessing a rear.
    if depth_offset.abs() < parcel.depth * 0.25
        || lateral_offset.abs() > parcel.width * 0.5 + 3.0
        || expected_offset < parcel.depth * 0.25
        || frame_alignment < 0.95
    {
        return None;
    }
    let rect = geometry::OrientedRect {
        x: parcel.x,
        z: parcel.z,
        width: parcel.width,
        depth: parcel.depth,
        rotation,
    };
    Some(CandidateGeometry {
        frontage: frontage.clone(),
        road_point,
        outward: (depth_axis.0 * side, depth_axis.1 * side),
        rect,
    })
}

struct CandidateGeometry {
    frontage: ParcelFrontage,
    road_point: (f32, f32),
    outward: (f32, f32),
    rect: geometry::OrientedRect,
}

fn build_candidates(engine: &Engine) -> Vec<Candidate> {
    let mut result = Vec::new();
    let mut buildings_by_parcel = BTreeMap::new();
    for building in &engine.state.buildings {
        buildings_by_parcel
            .entry(building.parcel_id)
            .or_insert(building);
    }
    let mut frontage_by_parcel = BTreeMap::new();
    for frontage in &engine.state.parcel_frontages {
        frontage_by_parcel
            .entry(frontage.parcel_id)
            .or_insert(frontage);
    }
    let mut road_positions = BTreeMap::new();
    for road in &engine.state.roads {
        if road.stage != "open"
            || !road.connected
            || matches!(road.kind.as_str(), "rail" | "bridge")
        {
            continue;
        }
        let (Some(from), Some(to)) = (engine.node(road.from), engine.node(road.to)) else {
            continue;
        };
        road_positions.insert(road.id, ((from.x, from.z), (to.x, to.z)));
    }

    let mut parcels: Vec<_> = engine.state.parcels.iter().collect();
    parcels.sort_by_key(|parcel| parcel.id);
    for parcel in parcels {
        let (Some(building), Some(frontage)) = (
            buildings_by_parcel.get(&parcel.id),
            frontage_by_parcel.get(&parcel.id),
        ) else {
            continue;
        };
        if building.stage != "open"
            || building.anchor
            || !(1..=8).contains(&building.floors)
            || parcel.protected
            || parcel.reservation.is_some()
            || !engine.node_connected(parcel.access_node)
        {
            continue;
        }
        let Some(&(road_from, road_to)) = road_positions.get(&frontage.road_id) else {
            continue;
        };
        let Some(geometry) = oriented_candidate(parcel, frontage, road_from, road_to) else {
            continue;
        };
        result.push(Candidate {
            parcel: parcel.clone(),
            building: (*building).clone(),
            frontage: geometry.frontage,
            road_from,
            road_to,
            road_point: geometry.road_point,
            outward: geometry.outward,
            rect: geometry.rect,
        });
    }
    result
}

fn extended(candidate: &Candidate, delta: f32) -> geometry::OrientedRect {
    geometry::OrientedRect {
        x: candidate.rect.x + candidate.outward.0 * delta * 0.5,
        z: candidate.rect.z + candidate.outward.1 * delta * 0.5,
        width: candidate.rect.width,
        depth: candidate.rect.depth + delta,
        rotation: candidate.rect.rotation,
    }
}

fn contains_point(rect: geometry::OrientedRect, point: (f32, f32), tolerance: f32) -> bool {
    let dx = point.0 - rect.x;
    let dz = point.1 - rect.z;
    let (sin, cos) = rect.rotation.sin_cos();
    let width = dx * cos + dz * -sin;
    let depth = dx * sin + dz * cos;
    width.abs() <= rect.width * 0.5 + tolerance && depth.abs() <= rect.depth * 0.5 + tolerance
}

fn footprint_range(parcels: &[Parcel], parcel_id: u32) -> Option<std::ops::Range<usize>> {
    let target = parcels.iter().position(|parcel| parcel.id == parcel_id)?;
    let start = parcels[..target]
        .iter()
        .map(|parcel| if parcel.rotation.is_none() { 2 } else { 1 })
        .sum::<usize>();
    let count = if parcels[target].rotation.is_none() {
        2
    } else {
        1
    };
    Some(start..start + count)
}

fn without_own_footprint(
    context: &CollisionContext,
    parcel_id: u32,
    parcels: &[Parcel],
) -> CollisionContext {
    let Some(range) = footprint_range(parcels, parcel_id) else {
        return context.clone();
    };
    let mut copy = context.clone();
    if range.end <= copy.parcel_footprints.len() {
        copy.parcel_footprints.drain(range);
    }
    copy
}

fn nominal_added_area(candidate: &Candidate, delta: f32) -> f32 {
    candidate.parcel.width * delta
}

fn evaluate_one(
    engine: &Engine,
    candidate: &Candidate,
    delta: f32,
    context: &CollisionContext,
    parcels: &[Parcel],
    towers: &[(f32, f32)],
) -> bool {
    if delta > 0.0 && candidate.parcel.depth + delta > DEPTH_CAP + f32::EPSILON {
        return false;
    }
    let rect = extended(candidate, delta);
    let local_context = without_own_footprint(context, candidate.parcel.id, parcels);
    let (min_x, min_z, max_x, max_z) = rect.bounds();
    engine.oriented_parcel_geometry_clear_with_context(&rect, &local_context)
        && !Engine::overlaps_station_tower_envelopes(
            towers,
            rect.x,
            rect.z,
            max_x - min_x,
            max_z - min_z,
        )
        && engine.node_connected(candidate.parcel.access_node)
}

fn rect_json(rect: geometry::OrientedRect) -> Value {
    json!({"x": rect.x, "z": rect.z, "width": rect.width, "depth": rect.depth, "rotation": rect.rotation})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_parcel(id: u32, x: f32, rotation: Option<f32>) -> Parcel {
        Parcel {
            id,
            x,
            z: 0.0,
            width: 10.0,
            depth: 12.0,
            rotation,
            access_node: 1,
            suitability: 1.0,
            protected: false,
            reservation: None,
            open_space: None,
        }
    }

    #[test]
    fn own_footprint_removal_accounts_for_legacy_duplicate() {
        let parcels = vec![
            test_parcel(10, 10.0, None),
            test_parcel(20, 30.0, Some(0.25)),
        ];
        let legacy_axis = geometry::OrientedRect {
            x: 10.0,
            z: 0.0,
            width: 10.0,
            depth: 12.0,
            rotation: 0.0,
        };
        let legacy_rendered = geometry::OrientedRect {
            rotation: 0.5,
            ..legacy_axis
        };
        let explicit = geometry::OrientedRect {
            x: 30.0,
            z: 0.0,
            width: 10.0,
            depth: 12.0,
            rotation: 0.25,
        };
        let context = CollisionContext {
            parcel_footprints: vec![legacy_axis, legacy_rendered, explicit],
            open_roads: Vec::new(),
            pending_roads: Vec::new(),
        };

        let without_explicit = without_own_footprint(&context, 20, &parcels);
        assert_eq!(
            without_explicit.parcel_footprints,
            vec![legacy_axis, legacy_rendered]
        );
        let without_legacy = without_own_footprint(&context, 10, &parcels);
        assert_eq!(without_legacy.parcel_footprints, vec![explicit]);
    }
}

fn example(candidate: &Candidate, delta: f32) -> Value {
    let after = extended(candidate, delta);
    json!({
        "parcel_id": candidate.parcel.id,
        "building_id": candidate.building.id,
        "building_kind": candidate.building.kind,
        "floors": candidate.building.floors,
        "frontage_road_id": candidate.frontage.road_id,
        "road_from": candidate.road_from,
        "road_to": candidate.road_to,
        "road_projection": candidate.road_point,
        "before": rect_json(candidate.rect),
        "after": rect_json(after),
        "added_nominal_area_m2": nominal_added_area(candidate, delta),
    })
}

#[test]
#[ignore = "bounded hypothetical rear parcel extension diagnostic; writes optional JSON report"]
fn rear_extension_geometry_opportunity() {
    let input_path = source_path()
        .canonicalize()
        .unwrap_or_else(|_| source_path());
    let input = fs::read_to_string(&input_path).expect("read diagnostic save");
    let engine = Engine::restore_json(&input).expect("diagnostic save restores");
    let before = engine.save_json();
    let candidates = build_candidates(&engine);
    let parcels = engine.state.parcels.clone();
    let base_context = engine.collision_context();
    let towers = engine.available_station_tower_envelopes();

    let baseline_valid_count = candidates
        .iter()
        .filter(|candidate| evaluate_one(&engine, candidate, 0.0, &base_context, &parcels, &towers))
        .count();
    let mut deltas = Vec::new();
    for delta in [4.0_f32, 8.0, 12.0] {
        let mut independent_count = 0;
        let mut independent_area = 0.0;
        let mut examples = Vec::new();
        for candidate in &candidates {
            if evaluate_one(&engine, candidate, delta, &base_context, &parcels, &towers) {
                independent_count += 1;
                independent_area += nominal_added_area(candidate, delta);
                if examples.len() < MAX_EXAMPLES {
                    examples.push(example(candidate, delta));
                }
                let expanded = extended(candidate, delta);
                let before_front = (
                    candidate.rect.x - candidate.outward.0 * candidate.rect.depth * 0.5,
                    candidate.rect.z - candidate.outward.1 * candidate.rect.depth * 0.5,
                );
                let after_front = (
                    expanded.x - candidate.outward.0 * expanded.depth * 0.5,
                    expanded.z - candidate.outward.1 * expanded.depth * 0.5,
                );
                assert!((before_front.0 - after_front.0).abs() < 0.001);
                assert!((before_front.1 - after_front.1).abs() < 0.001);
                for corner in candidate.rect.corners() {
                    assert!(
                        contains_point(expanded, corner, 0.001),
                        "extended parcel must contain original footprint"
                    );
                }
            }
        }

        deltas.push(json!({
            "delta_m": delta,
            "independent_feasible_parcels": independent_count,
            "independent_added_nominal_area_m2": independent_area,
            "independent_examples_capped": examples,
        }));
    }

    let mut greedy_context = base_context.clone();
    let mut greedy_count = 0;
    let mut greedy_area = 0.0;
    let mut greedy_examples = Vec::new();
    let mut accepted_rects = Vec::new();
    let mut accepted_extensions = BTreeMap::new();
    for candidate in &candidates {
        let Some(delta) = [12.0_f32, 8.0, 4.0].into_iter().find(|delta| {
            evaluate_one(
                &engine,
                candidate,
                *delta,
                &greedy_context,
                &parcels,
                &towers,
            )
        }) else {
            continue;
        };
        let expanded = extended(candidate, delta);
        let Some(range) = footprint_range(&parcels, candidate.parcel.id) else {
            continue;
        };
        assert_eq!(range.len(), 1, "eligible candidate has explicit rotation");
        greedy_context.parcel_footprints[range.start] = expanded;
        for prior in &accepted_rects {
            assert!(!geometry::rects_overlap(&expanded, prior, 2.0));
        }
        accepted_rects.push(expanded);
        accepted_extensions.insert(candidate.parcel.id, (expanded, delta));
        greedy_count += 1;
        greedy_area += nominal_added_area(candidate, delta);
        if greedy_examples.len() < MAX_EXAMPLES {
            greedy_examples.push(example(candidate, delta));
        }
    }
    let snapshot_dir = std::env::var_os("REAR_EXTENSION_SNAPSHOT_DIR").map(PathBuf::from);
    if let Some(directory) = snapshot_dir {
        fs::create_dir_all(&directory).expect("create snapshot output directory");
        let before_snapshot: Value =
            serde_json::from_str(&engine.snapshot_json()).expect("parse source snapshot");
        let mut after_snapshot = before_snapshot.clone();
        let buildings = after_snapshot
            .get_mut("buildings")
            .and_then(Value::as_array_mut)
            .expect("snapshot has buildings array");
        for building in buildings {
            let parcel_id = building.get("parcel_id").and_then(Value::as_u64);
            let Some((rect, _delta)) =
                parcel_id.and_then(|id| accepted_extensions.get(&(id as u32)))
            else {
                continue;
            };
            building["x"] = json!(rect.x);
            building["z"] = json!(rect.z);
            building["width"] = json!(rect.width);
            building["depth"] = json!(rect.depth);
            building["rotation"] = json!(rect.rotation);
        }
        after_snapshot["diagnostic"] = json!({
            "classification": "hypothetical rear-extension rendering only; not simulation state, legal retrofit, or redevelopment result",
            "selection": "stable parcel-ID order; largest feasible requested extension among 12, 8, 4 metres",
            "accepted_extensions": accepted_extensions.iter().map(|(id, (rect, delta))| json!({"parcel_id": id, "delta_m": delta, "rect": rect_json(*rect)})).collect::<Vec<_>>(),
        });
        fs::write(
            directory.join("before-snapshot.json"),
            serde_json::to_vec(&before_snapshot).unwrap(),
        )
        .expect("write original snapshot");
        fs::write(
            directory.join("hypothetical-after-snapshot.json"),
            serde_json::to_vec(&after_snapshot).unwrap(),
        )
        .expect("write hypothetical snapshot");
    }

    assert_eq!(
        engine.save_json(),
        before,
        "source save must remain unchanged"
    );
    let report = json!({
        "diagnostic": "rear_extension_geometry_opportunity",
        "classification": "hypothetical geometry opportunity only; not a legal retrofit, redevelopment, demand, capacity, or lifecycle claim",
        "source_path": input_path,
        "seed": engine.state.seed,
        "tick": engine.state.tick,
        "source_unchanged": engine.save_json() == before,
        "eligible_parcel_buildings": candidates.len(),
        "baseline_valid_candidates_delta_0": baseline_valid_count,
        "reserved_parcels_excluded": engine.state.parcels.iter().filter(|parcel| parcel.reservation.is_some()).count(),
        "eligibility": "open non-anchor buildings with 1..=8 floors on unreserved parcels, explicit parcel rotation, frontage metadata to an open connected local road, connected parcel access, and unambiguous source-geometry frontage alignment",
        "preserved_checks": ["existing parcel footprints other than candidate's own", "protected area", "river", "world bounds", "open and pending roads", "interior access reservations", "neighborhood corridors", "station tower envelopes checked against the expanded rectangle's world-axis bounds", "connected access node"],
        "extension_depth_cap_m": DEPTH_CAP,
        "stable_greedy": {
            "feasible_parcels": greedy_count,
            "added_nominal_area_m2": greedy_area,
            "selection": "ascending parcel id; largest feasible requested extension among 12, 8, 4 metres; each accepted extension replaces only its own footprint in copied context",
            "examples_capped": greedy_examples,
        },
        "deltas": deltas,
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if let Some(path) = std::env::var_os("REAR_EXTENSION_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap())
            .expect("write diagnostic report");
    }
}
