//! Test-only paired experiment for deeper initial parcels at building admission.
//!
//! The treatment runs only after the ordinary site selector and building
//! profile chooser have made their normal decisions. It may move a selected
//! vacant ordinary frontage parcel's rear edge outward, but it retains the
//! production demand, access, reservation, construction, and capacity rules.

use super::{geometry, CollisionContext, Engine, NeighborhoodProfile, Parcel};
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

const BASE_DEPTH: f32 = 12.0;
const EXPERIMENT_DEPTH: f32 = 18.0;

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static DIAGNOSTICS: RefCell<DepthDiagnostics> = RefCell::new(DepthDiagnostics::default());
}

#[derive(Clone, Debug, Default, Serialize)]
struct DepthDiagnostics {
    selected_vacant_parcels: u64,
    rejection_reasons: BTreeMap<&'static str, u64>,
    eligible: u64,
    accepted: u64,
    added_nominal_footprint_m2: f64,
    examples: Vec<DepthExample>,
}

#[derive(Clone, Debug, Serialize)]
struct DepthExample {
    parcel_id: u32,
    station_id: u32,
    road_id: u32,
    original_center: [f32; 2],
    expanded_center: [f32; 2],
    width: f32,
    old_depth: f32,
    new_depth: f32,
}

pub(crate) struct InitialDepthExperimentGuard(bool);

impl InitialDepthExperimentGuard {
    fn enabled() -> Self {
        Self(ENABLED.with(|flag| flag.replace(true)))
    }
}

impl Drop for InitialDepthExperimentGuard {
    fn drop(&mut self) {
        ENABLED.with(|flag| flag.set(self.0));
    }
}

fn reset_diagnostics() {
    DIAGNOSTICS.with(|diagnostics| *diagnostics.borrow_mut() = DepthDiagnostics::default());
}

fn take_diagnostics() -> DepthDiagnostics {
    DIAGNOSTICS.with(|diagnostics| std::mem::take(&mut *diagnostics.borrow_mut()))
}

fn reject(reason: &'static str) {
    DIAGNOSTICS.with(|diagnostics| {
        let mut diagnostics = diagnostics.borrow_mut();
        *diagnostics.rejection_reasons.entry(reason).or_default() += 1;
    });
}

/// Expand in local -Z so the existing local +Z road-facing edge stays fixed.
fn expanded_rect(original: geometry::OrientedRect) -> geometry::OrientedRect {
    let extension = EXPERIMENT_DEPTH - original.depth;
    let local_front = (original.rotation.sin(), original.rotation.cos());
    geometry::OrientedRect {
        x: original.x - local_front.0 * extension * 0.5,
        z: original.z - local_front.1 * extension * 0.5,
        depth: EXPERIMENT_DEPTH,
        ..original
    }
}

fn own_footprint_index(parcels: &[Parcel], parcel_id: u32) -> Option<usize> {
    let mut footprint_index = 0usize;
    for parcel in parcels {
        if parcel.id == parcel_id {
            return Some(footprint_index);
        }
        // collision_context retains both historical axis and inferred/rendered
        // orientations for every parcel without an explicit saved rotation.
        footprint_index += if parcel.rotation.is_none() { 2 } else { 1 };
    }
    None
}

fn ordinary_connected_road_count(engine: &Engine) -> usize {
    engine
        .state
        .roads
        .iter()
        .filter(|road| {
            road.stage == "open"
                && road.connected
                && !matches!(road.kind.as_str(), "rail" | "bridge")
        })
        .count()
}

fn routable_station_for(engine: &Engine, access_node: u32) -> Option<u32> {
    engine
        .state
        .nodes
        .iter()
        .filter(|station| station.kind == "station" && engine.node_has_open_road(station.id))
        .find(|station| {
            station.id == access_node || engine.route(station.id, access_node, false).len() >= 2
        })
        .map(|station| station.id)
}

fn record_selected() {
    DIAGNOSTICS.with(|diagnostics| {
        diagnostics.borrow_mut().selected_vacant_parcels += 1;
    });
}

/// Called from the ordinary building reservation path after profile selection.
/// Returns the original parcel on any failed gate; it never consumes RNG.
pub(super) fn maybe_expand_selected_parcel(engine: &mut Engine, mut parcel: Parcel) -> Parcel {
    if !ENABLED.with(Cell::get) {
        return parcel;
    }
    record_selected();

    if parcel.protected || parcel.reservation.is_some() {
        reject("protected_or_reserved");
        return parcel;
    }
    if parcel.rotation.is_none() {
        reject("missing_explicit_rotation");
        return parcel;
    }
    if (parcel.depth - BASE_DEPTH).abs() > 1.0e-4 {
        reject("not_standard_12m_depth");
        return parcel;
    }
    if !engine.state.station_online {
        reject("station_offline");
        return parcel;
    }
    if ordinary_connected_road_count(engine) < 40 {
        reject("under_40_connected_ordinary_roads");
        return parcel;
    }
    if engine.neighborhood_profile(parcel.x, parcel.z) != NeighborhoodProfile::StationCore {
        reject("outside_station_core_profile");
        return parcel;
    }
    let Some(frontage) = engine
        .state
        .parcel_frontages
        .iter()
        .find(|frontage| frontage.parcel_id == parcel.id)
    else {
        reject("missing_frontage_record");
        return parcel;
    };
    let Some(road) = engine
        .state
        .roads
        .iter()
        .find(|road| road.id == frontage.road_id)
    else {
        reject("missing_frontage_road");
        return parcel;
    };
    let frontage_road_id = road.id;
    if road.stage != "open"
        || !road.connected
        || matches!(road.kind.as_str(), "rail" | "bridge")
        || engine.is_neighborhood_plan_road(frontage_road_id)
    {
        reject("not_open_ordinary_frontage");
        return parcel;
    }
    let Some(station_id) = routable_station_for(engine, parcel.access_node) else {
        reject("no_routable_station_access");
        return parcel;
    };

    DIAGNOSTICS.with(|diagnostics| diagnostics.borrow_mut().eligible += 1);
    let rotation = parcel.rotation.expect("explicit rotation checked above");
    let original = geometry::OrientedRect {
        x: parcel.x,
        z: parcel.z,
        width: parcel.width,
        depth: parcel.depth,
        rotation,
    };
    let expanded = expanded_rect(original);
    let Some(footprint_index) = own_footprint_index(&engine.state.parcels, parcel.id) else {
        reject("own_footprint_not_found");
        return parcel;
    };
    let mut context = engine.collision_context();
    if footprint_index >= context.parcel_footprints.len() {
        reject("own_footprint_index_invalid");
        return parcel;
    }
    context.parcel_footprints.remove(footprint_index);

    let tower_envelopes = engine.available_station_tower_envelopes();
    if Engine::overlaps_station_tower_envelopes(
        &tower_envelopes,
        expanded.x,
        expanded.z,
        expanded.width,
        expanded.depth,
    ) {
        reject("station_tower_reservation");
        return parcel;
    }
    if !engine.progressive_frontage_geometry_clear(&expanded, &context) {
        // This production predicate includes bounds, river, protected land,
        // all other parcel footprints, active interior reservations, open and
        // pending road clearance, and the neighborhood corridor reservation.
        reject("exact_geometry_or_live_reservation");
        return parcel;
    }

    parcel.x = expanded.x;
    parcel.z = expanded.z;
    parcel.depth = expanded.depth;
    if let Some(stored) = engine.state.parcels.iter_mut().find(|p| p.id == parcel.id) {
        stored.x = parcel.x;
        stored.z = parcel.z;
        stored.depth = parcel.depth;
    } else {
        reject("selected_parcel_disappeared");
        return parcel;
    }
    engine.mark_access_geometry_changed();
    DIAGNOSTICS.with(|diagnostics| {
        let mut diagnostics = diagnostics.borrow_mut();
        diagnostics.accepted += 1;
        diagnostics.added_nominal_footprint_m2 +=
            f64::from(parcel.width * (EXPERIMENT_DEPTH - BASE_DEPTH));
        if diagnostics.examples.len() < 12 {
            diagnostics.examples.push(DepthExample {
                parcel_id: parcel.id,
                station_id,
                road_id: frontage_road_id,
                original_center: [original.x, original.z],
                expanded_center: [expanded.x, expanded.z],
                width: parcel.width,
                old_depth: BASE_DEPTH,
                new_depth: EXPERIMENT_DEPTH,
            });
        }
    });
    parcel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_expansion_preserves_local_positive_z_frontage_edge() {
        for rotation in [-2.3_f32, -0.4, 0.0, 0.8, 2.7] {
            let original = geometry::OrientedRect {
                x: 114.0,
                z: -198.0,
                width: 14.0,
                depth: BASE_DEPTH,
                rotation,
            };
            let expanded = expanded_rect(original);
            let before = original.corners();
            let after = expanded.corners();
            assert!((expanded.depth - EXPERIMENT_DEPTH).abs() < 1.0e-5);
            assert!((before[2].0 - after[2].0).abs() < 1.0e-4);
            assert!((before[2].1 - after[2].1).abs() < 1.0e-4);
            assert!((before[3].0 - after[3].0).abs() < 1.0e-4);
            assert!((before[3].1 - after[3].1).abs() < 1.0e-4);
        }
    }

    #[test]
    fn own_footprint_mapping_skips_both_legacy_envelopes() {
        let parcel = |id, rotation| Parcel {
            id,
            x: 0.0,
            z: 0.0,
            width: 10.0,
            depth: 12.0,
            suitability: 0.5,
            access_node: 0,
            protected: false,
            rotation,
            reservation: None,
        };
        let parcels = vec![parcel(1, None), parcel(2, Some(0.7)), parcel(3, None)];
        assert_eq!(own_footprint_index(&parcels, 2), Some(2));
        assert_eq!(own_footprint_index(&parcels, 3), Some(3));
    }

    #[test]
    fn exact_progressive_geometry_rejects_blockers_and_world_bounds() {
        let engine = Engine::new_legacy(0);
        let rect = geometry::OrientedRect {
            x: 340.0,
            z: 340.0,
            width: 14.0,
            depth: 18.0,
            rotation: 0.3,
        };
        let empty = CollisionContext {
            parcel_footprints: Vec::new(),
            open_roads: Vec::new(),
            pending_roads: Vec::new(),
        };
        let mut clear_rect = None;
        // Pick a deterministic point outside the protected center and river,
        // rather than baking assumptions about the river's course into test.
        for x in (80..360).step_by(20) {
            for z in (80..360).step_by(20) {
                let probe = geometry::OrientedRect {
                    x: x as f32,
                    z: z as f32,
                    ..rect
                };
                if engine.progressive_frontage_geometry_clear(&probe, &empty) {
                    clear_rect = Some(probe);
                    break;
                }
            }
            if clear_rect.is_some() {
                break;
            }
        }
        let probe = clear_rect.expect("sample should find legal geometry on ordinary land");
        assert!(engine.progressive_frontage_geometry_clear(&probe, &empty));
        let mut blocked = empty.clone();
        blocked.parcel_footprints.push(probe);
        assert!(!engine.progressive_frontage_geometry_clear(&probe, &blocked));
        let out_of_bounds = geometry::OrientedRect { x: 399.0, ..probe };
        assert!(!engine.progressive_frontage_geometry_clear(&out_of_bounds, &empty));
    }
}

#[derive(Clone, Debug, Serialize)]
struct ArmResult {
    seed: u32,
    treatment: bool,
    tick: u64,
    development_rate: u32,
    buildings: usize,
    roads: usize,
    ordinary_roads: usize,
    parcels: usize,
    nominal_footprint_m2: f64,
    nominal_floor_area_m2: f64,
    capacity_slots: u64,
    capacity_by_kind: BTreeMap<String, u64>,
    initial_depth_diagnostics: Option<DepthDiagnostics>,
    canonical_restore: bool,
    same_target_2000_tick_replay: bool,
    save_path: String,
    snapshot_path: String,
    save_sha256: String,
    snapshot_sha256: String,
}

fn metrics(engine: &Engine) -> (f64, f64, u64, BTreeMap<String, u64>) {
    let mut footprint = 0.0_f64;
    let mut floor_area = 0.0_f64;
    let mut capacity = 0_u64;
    let mut by_kind = BTreeMap::new();
    for building in &engine.state.buildings {
        let area = f64::from(building.width) * f64::from(building.depth);
        footprint += area;
        floor_area += area * f64::from(building.floors);
        capacity += u64::from(building.capacity);
        *by_kind.entry(building.kind.clone()).or_default() += u64::from(building.capacity);
    }
    (footprint, floor_area, capacity, by_kind)
}

fn run_arm(seed: u32, treatment: bool, raw_dir: &std::path::Path) -> ArmResult {
    let mut engine = Engine::new_legacy(seed);
    engine.set_development_rate(16);
    if treatment {
        reset_diagnostics();
    }
    // Keep the treatment active through both sides of the post-save replay so
    // later selected sites receive the same geometry policy on each side.
    let _treatment_guard = treatment.then(InitialDepthExperimentGuard::enabled);
    for _ in 0..121 {
        engine.step(2_000);
    }
    engine.validate().unwrap_or_else(|error| {
        panic!("seed {seed} treatment {treatment} failed validation: {error}")
    });
    assert_eq!(engine.state.tick, 242_000);
    assert_eq!(engine.state.development_rate, 16);
    let diagnostics = treatment.then(take_diagnostics);
    let (nominal_footprint_m2, nominal_floor_area_m2, capacity_slots, capacity_by_kind) =
        metrics(&engine);
    let buildings = engine.state.buildings.len();
    let roads = engine.state.roads.len();
    let ordinary_roads = ordinary_connected_road_count(&engine);
    let parcels = engine.state.parcels.len();
    let suffix = if treatment { "depth18" } else { "baseline" };
    let save_path = raw_dir.join(format!("seed-{seed}-{suffix}-tick242000-save.json"));
    let snapshot_path = raw_dir.join(format!("seed-{seed}-{suffix}-tick242000-snapshot.json"));
    let save = engine.save_json();
    let snapshot = engine.snapshot_json();
    std::fs::write(&save_path, &save).expect("write raw save");
    std::fs::write(&snapshot_path, &snapshot).expect("write raw snapshot");
    let save_sha256 = sha256_file(&save_path).expect("hash raw save");
    let snapshot_sha256 = sha256_file(&snapshot_path).expect("hash raw snapshot");

    let mut resumed = Engine::restore_json(&save).expect("canonical restore at target tick");
    resumed.validate().expect("restored target state validates");
    assert_eq!(resumed.save_json(), save);
    engine.step(2_000);
    resumed.step(2_000);
    let same_target_2000_tick_replay = engine.save_json() == resumed.save_json();
    assert!(
        same_target_2000_tick_replay,
        "same-target 2,000-tick replay diverged"
    );

    ArmResult {
        seed,
        treatment,
        tick: 242_000,
        development_rate: 16,
        buildings,
        roads,
        ordinary_roads,
        parcels,
        nominal_footprint_m2,
        nominal_floor_area_m2,
        capacity_slots,
        capacity_by_kind,
        initial_depth_diagnostics: diagnostics,
        canonical_restore: true,
        same_target_2000_tick_replay,
        save_path: save_path.display().to_string(),
        snapshot_path: snapshot_path.display().to_string(),
        save_sha256,
        snapshot_sha256,
    }
}

fn sha256_file(path: &std::path::Path) -> Option<String> {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .map(str::to_owned)
}

fn source_hashes(workspace: &std::path::Path) -> serde_json::Value {
    let git = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned());
    let hashes = Command::new("shasum")
        .args([
            "-a",
            "256",
            "engine/src/lib.rs",
            "engine/src/initial_depth_diagnostic.rs",
        ])
        .current_dir(workspace)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
    serde_json::json!({
        "gitHead": git,
        "sha256": hashes,
    })
}

#[test]
#[ignore = "bounded paired 242,000-tick two-seed initial-depth experiment"]
fn paired_station_core_initial_depth_experiment() {
    let engine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = engine_dir
        .parent()
        .expect("engine crate has repository parent");
    let raw_dir = workspace.join(".tools/initial-depth-review");
    std::fs::create_dir_all(&raw_dir).expect("create ignored raw artifact directory");
    let mut arms = Vec::new();
    for seed in [0, 87_231] {
        arms.push(run_arm(seed, false, &raw_dir));
        arms.push(run_arm(seed, true, &raw_dir));
    }
    let report = serde_json::json!({
        "experiment": "test-only preconstruction parcel depth increase",
        "sourceHashes": source_hashes(workspace),
        "target": {
            "tick": 242000,
            "developmentRate": 16,
            "seeds": [0, 87231],
        },
        "treatment": {
            "eligible": "selected vacant, explicit-rotation, 12 m deep ordinary-frontage parcel; station online; StationCore profile; at least 40 connected ordinary roads; ordinary walking-network route from an open station",
            "depthMeters": { "baseline": 12, "treatment": 18 },
            "frontage": "preserve the existing local +Z front edge, road ID, frontage metadata, and access node",
            "capacity": "unchanged use/floor-based capacity slots; footprint and floor area reported separately",
            "geometry": "production oriented progressive-frontage gate after removing only the candidate parcel's own collision envelope; tower reservation envelopes retained",
        },
        "limitations": [
            "capped to the ordinary scheduler's selected vacant parcel stream; this does not exhaust every empty parcel",
            "native paired growth evidence only; it is not a browser/WASM parity or visual acceptance claim",
            "nominal area sums are not rendered union coverage or a calibrated capacity measure",
        ],
        "arms": arms,
    });
    let report_path = workspace.join("validation/initial-depth-experiment.json");
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap())
        .expect("write compact report");
    eprintln!("initial-depth report: {}", report_path.display());
}
