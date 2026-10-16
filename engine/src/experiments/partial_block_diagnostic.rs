//! Test-only diagnostic for full blocks with a feasible subset of canonical lots.
//!
//! Ordinary full-block geometry and road checks stay unchanged. The experiment
//! only permits explicitly selected, individually clear canonical lots when a
//! legacy all-canonical-lots check would reject a full block.

use crate::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize)]
struct PartialFullBlockWitness {
    anchor_node: u32,
    canonical_lots: usize,
    selected_lots: usize,
    lots_by_edge: [usize; 4],
    reason: &'static str,
    corners: [[f32; 2]; 4],
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct PartialFullBlockCapture {
    candidates: usize,
    selected_lot_counts: BTreeMap<usize, usize>,
    rejection_counts: BTreeMap<&'static str, usize>,
    witnesses: Vec<PartialFullBlockWitness>,
}

#[derive(Default)]
struct PartialCaptureState {
    result: PartialFullBlockCapture,
}

std::thread_local! {
    static PARTIAL_CAPTURE: std::cell::RefCell<Option<PartialCaptureState>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn capture_partial_candidates<R>(f: impl FnOnce() -> R) -> (R, PartialFullBlockCapture) {
    PARTIAL_CAPTURE.with(|capture| {
        assert!(
            capture.borrow().is_none(),
            "partial-block capture cannot nest"
        );
        *capture.borrow_mut() = Some(PartialCaptureState::default());
    });
    let _guard = PartialCaptureGuard;
    let result = f();
    let collected = PARTIAL_CAPTURE.with(|capture| {
        capture
            .borrow_mut()
            .take()
            .expect("partial-block capture was installed")
            .result
    });
    (result, collected)
}

struct PartialCaptureGuard;

impl Drop for PartialCaptureGuard {
    fn drop(&mut self) {
        PARTIAL_CAPTURE.with(|capture| {
            capture.borrow_mut().take();
        });
    }
}

pub(crate) fn record_partial_candidate(
    anchor: u32,
    corners: &[WorldPoint; 4],
    canonical_count: usize,
    lots: &[blocks::BlockLot],
    result: &Result<(), neighborhood_admission::NeighborhoodRejection>,
) {
    PARTIAL_CAPTURE.with(|capture| {
        let mut capture = capture.borrow_mut();
        let Some(state) = capture.as_mut() else {
            return;
        };
        state.result.candidates += 1;
        *state
            .result
            .selected_lot_counts
            .entry(lots.len())
            .or_default() += 1;
        let reason = result
            .as_ref()
            .err()
            .map_or("accepted", |reason| reason.label());
        if result.is_err() {
            *state.result.rejection_counts.entry(reason).or_default() += 1;
        }
        let reason_witnesses = state
            .result
            .witnesses
            .iter()
            .filter(|witness| witness.reason == reason)
            .count();
        if reason_witnesses < 3 && state.result.witnesses.len() < 24 {
            let mut lots_by_edge = [0; 4];
            for lot in lots {
                if let Some(count) = lots_by_edge.get_mut(lot.frontage_edge) {
                    *count += 1;
                }
            }
            state.result.witnesses.push(PartialFullBlockWitness {
                anchor_node: anchor,
                canonical_lots: canonical_count,
                selected_lots: lots.len(),
                lots_by_edge,
                reason,
                corners: corners.map(|point| [point.x, point.z]),
            });
        }
    });
}

#[derive(Serialize)]
struct CandidateOutcome {
    accepted: bool,
    planner_calls: usize,
    anchors_visited: usize,
    anchor_count: usize,
    anchor_node: Option<u32>,
    plan_segments: Vec<(u32, u32)>,
    road_projects_after_replay: usize,
    selected_lot_count: usize,
    selected_lots_by_edge: [usize; 4],
    reasons: BTreeMap<&'static str, usize>,
    witnesses: Vec<neighborhood_admission::RejectionWitness>,
    partial_full_block: PartialFullBlockCapture,
    clone_started_from_source: bool,
    valid_after_eval: bool,
    canonical_restore_matches: bool,
    replay_2000_matches: Option<bool>,
}

#[derive(Serialize)]
struct PartialBlockReport {
    input_path: String,
    input_sha256: Option<String>,
    source_sha256: BTreeMap<String, Option<String>>,
    seed: u32,
    tick: u64,
    ordinary_full_block_partial_lots_enabled: bool,
    source_save_unchanged: bool,
    baseline: CandidateOutcome,
    partial: CandidateOutcome,
}

fn input_save_path() -> PathBuf {
    if let Some(path) = std::env::var_os("CITY_TRACE_SAVE") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine crate has repository parent")
        .join(".tools/narrow-frontage-review/final-save.json")
}

fn file_sha256(path: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(path)
        .output()
        .or_else(|_| std::process::Command::new("sha256sum").arg(path).output())
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .split_whitespace()
        .next()
        .map(str::to_owned)
}

fn outcome(mut engine: Engine, partial_enabled: bool, original_save: &str) -> CandidateOutcome {
    let clone_started_from_source = engine.save_json() == original_save;
    assert!(
        clone_started_from_source,
        "each treatment starts from exact source save"
    );
    engine.experiments.partial_full_block_experiment = partial_enabled;
    let (((planner_calls, anchors_visited), capture), partial_full_block) =
        capture_partial_candidates(|| {
            neighborhood_admission::capture_rejections(|| {
                let anchor_count = engine.state.nodes.len();
                let mut visited = std::collections::BTreeSet::new();
                for call in 1..=25 {
                    let before = engine.state.neighborhood_anchor_cursor % anchor_count.max(1);
                    let accepted = engine.try_create_neighborhood_plan();
                    let after = engine.state.neighborhood_anchor_cursor % anchor_count.max(1);
                    let mut index = before;
                    while index != after {
                        visited.insert(index);
                        index = (index + 1) % anchor_count;
                    }
                    if accepted {
                        return (call, visited.len());
                    }
                }
                (25, visited.len())
            })
        });
    let valid_after_eval = engine.validate().is_ok();
    let plan = engine.state.neighborhood_plan.as_ref();
    let plan_segments = plan.map_or_else(Vec::new, |plan| plan.segments.clone());
    let accepted = plan.is_some();
    let anchor_node = plan.map(|plan| plan.anchor_node);
    let planned_lots = plan.map(|plan| {
        plan.selected_lots.clone().unwrap_or_else(|| {
            engine
                .coordinated_block_lots(&plan.segments)
                .unwrap_or_default()
        })
    });
    let mut selected_lots_by_edge = [0; 4];
    if let Some(lots) = planned_lots.as_ref() {
        for lot in lots {
            if let Some(count) = selected_lots_by_edge.get_mut(lot.frontage_edge) {
                *count += 1;
            }
        }
    }
    let canonical_save = engine.save_json();
    let canonical_restore_matches = Engine::restore_json(&canonical_save)
        .ok()
        .is_some_and(|restored| restored.save_json() == canonical_save);
    let (replay_2000_matches, road_projects_after_replay) = if canonical_restore_matches && accepted
    {
        let mut resumed = Engine::restore_json(&canonical_save).ok();
        if let Some(resumed) = resumed.as_mut() {
            resumed.experiments.partial_full_block_experiment = partial_enabled;
            engine.step(2_000);
            resumed.step(2_000);
            let matches = engine.save_json() == resumed.save_json();
            let road_projects = engine
                .state
                .projects
                .iter()
                .filter(|project| project.kind == ProjectKind::Road && project.progress < 1.0)
                .count();
            (Some(matches), road_projects)
        } else {
            (Some(false), 0)
        }
    } else {
        (None, 0)
    };
    CandidateOutcome {
        accepted,
        planner_calls,
        anchors_visited,
        anchor_count: engine.state.nodes.len(),
        anchor_node,
        plan_segments,
        road_projects_after_replay,
        selected_lot_count: planned_lots.as_ref().map_or(0, Vec::len),
        selected_lots_by_edge,
        reasons: capture.reason_counts,
        witnesses: capture.witnesses,
        partial_full_block,
        clone_started_from_source,
        valid_after_eval,
        canonical_restore_matches,
        replay_2000_matches,
    }
}

impl Engine {
    /// Minimum useful subset for the test-only partial full-block treatment.
    /// The experiment imposes a floor of two clear lots on each long new
    /// frontage edge; this is a diagnostic usefulness threshold, not a claimed
    /// production rule. Road and exact lot-clearance checks remain unchanged.
    #[cfg(test)]
    pub(crate) fn partial_full_block_lots_useful(
        corners: &[WorldPoint; 4],
        lots: &[blocks::BlockLot],
    ) -> bool {
        if lots.len() < 4 {
            return false;
        }
        if lots
            .iter()
            .enumerate()
            .any(|(index, lot)| lots[..index].contains(lot))
        {
            return false;
        }
        (0..4).all(|edge| {
            let a = corners[edge];
            let b = corners[(edge + 1) % 4];
            let length = (b.x - a.x).hypot(b.z - a.z);
            length < 70.0 || lots.iter().filter(|lot| lot.frontage_edge == edge).count() >= 2
        })
    }
}

#[test]
fn partial_full_block_usefulness_requires_four_lots_and_both_long_edges() {
    let corners = [
        blocks::BlockPoint { x: 0.0, z: 0.0 },
        blocks::BlockPoint { x: 82.0, z: 0.0 },
        blocks::BlockPoint { x: 82.0, z: 50.0 },
        blocks::BlockPoint { x: 0.0, z: 50.0 },
    ];
    let lots = blocks::subdivide_rectangular_block(corners, 8.0, 11.0, 2.2);
    let long_edges: Vec<usize> = (0..4)
        .filter(|edge| {
            let a = corners[*edge];
            let b = corners[(*edge + 1) % 4];
            (b.x - a.x).hypot(b.z - a.z) >= 70.0
        })
        .collect();
    assert_eq!(long_edges.len(), 2);
    let mut valid_subset = Vec::new();
    for edge in &long_edges {
        valid_subset.extend(
            lots.iter()
                .copied()
                .filter(|lot| lot.frontage_edge == *edge)
                .take(2),
        );
    }
    assert_eq!(valid_subset.len(), 4);
    assert!(Engine::partial_full_block_lots_useful(
        &corners.map(|point| WorldPoint {
            x: point.x,
            z: point.z
        }),
        &valid_subset
    ));
    let one_long_edge_only: Vec<_> = lots
        .iter()
        .copied()
        .filter(|lot| lot.frontage_edge == long_edges[0])
        .take(4)
        .collect();
    assert_eq!(one_long_edge_only.len(), 4);
    assert!(!Engine::partial_full_block_lots_useful(
        &corners.map(|point| WorldPoint {
            x: point.x,
            z: point.z
        }),
        &one_long_edge_only
    ));
    assert!(!Engine::partial_full_block_lots_useful(
        &corners.map(|point| WorldPoint {
            x: point.x,
            z: point.z
        }),
        &valid_subset[..3]
    ));
}

#[test]
fn partial_full_block_experiment_hook_defaults_off_and_is_not_persisted() {
    let engine = Engine::new_legacy(87231);
    assert!(!engine.experiments.partial_full_block_experiment);
    let save = engine.save_json();
    let restored = Engine::restore_json(&save).expect("restore default engine");
    assert!(!restored.experiments.partial_full_block_experiment);
    assert_eq!(restored.save_json(), save);
}

#[test]
#[ignore = "bounded mature-state partial full-block geometry diagnostic"]
fn mature_full_block_partial_lot_diagnostic() {
    let path = input_save_path();
    let source_save = std::fs::read_to_string(&path).expect("read mature diagnostic save");
    let source = Engine::restore_json(&source_save).expect("restore mature diagnostic save");
    let source_canonical = source.save_json();
    let seed = source.state.seed;
    let tick = source.state.tick;

    // The baseline and treatment begin from the exact same serialized state;
    // the experiment flag exists only in the test Engine value.
    let baseline = outcome(
        Engine::restore_json(&source_canonical).expect("restore baseline clone"),
        false,
        &source_canonical,
    );
    let partial = outcome(
        Engine::restore_json(&source_canonical).expect("restore treatment clone"),
        true,
        &source_canonical,
    );
    assert_eq!(
        source.save_json(),
        source_canonical,
        "source save is immutable"
    );
    assert!(baseline.clone_started_from_source);
    assert!(partial.clone_started_from_source);
    assert!(baseline.valid_after_eval);
    assert!(partial.valid_after_eval);
    assert!(baseline.canonical_restore_matches);
    assert!(partial.canonical_restore_matches);
    if !baseline.accepted {
        assert_eq!(baseline.anchors_visited, baseline.anchor_count);
    }
    if partial.accepted {
        assert_eq!(
            partial.selected_lots_by_edge.iter().sum::<usize>(),
            partial.selected_lot_count
        );
        assert!(partial.selected_lot_count >= 4);
        assert_eq!(partial.replay_2000_matches, Some(true));
        assert!(
            partial.road_projects_after_replay > 0,
            "normal scheduler starts block construction"
        );
    }

    let report = PartialBlockReport {
        input_path: path.display().to_string(),
        input_sha256: file_sha256(&path),
        source_sha256: [
            (
                "engine/src/lib.rs".to_string(),
                file_sha256(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")),
            ),
            (
                "engine/src/neighborhood_admission.rs".to_string(),
                file_sha256(
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("src/neighborhood_admission.rs"),
                ),
            ),
            (
                "engine/src/partial_block_diagnostic.rs".to_string(),
                file_sha256(
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("src/partial_block_diagnostic.rs"),
                ),
            ),
        ]
        .into_iter()
        .collect(),
        seed,
        tick,
        ordinary_full_block_partial_lots_enabled: true,
        source_save_unchanged: source.save_json() == source_canonical,
        baseline,
        partial,
    };
    if let Some(output) = std::env::var_os("PARTIAL_BLOCK_REPORT") {
        let output = PathBuf::from(output);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).expect("create report directory");
        }
        std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap())
            .expect("write partial block report");
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
