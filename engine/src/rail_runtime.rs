//! Authoritative per-road rail samples shared by motion and presentation.
use crate::{
    masterplan::{MasterPlan, RoadKind, P},
    rail_alignment::Alignment,
    Engine,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RailAlignmentView {
    pub road_id: u32,
    /// [arc distance, x, z, forward unit tangent x, forward unit tangent z].
    /// Distances and tangents run from the road's `from` node to its `to` node.
    pub samples: Vec<[f32; 5]>,
}

#[derive(Clone, Debug)]
pub(crate) struct Path {
    pub(crate) samples: Vec<[f32; 5]>,
}

impl Path {
    fn new(alignment: &Alignment, from: f32, to: f32) -> Self {
        let direction = if to > from { 1.0 } else { -1.0 };
        let mut xs: Vec<f32> = alignment
            .samples()
            .iter()
            .map(|s| s.1.x)
            .filter(|x| *x > from.min(to) && *x < from.max(to))
            .collect();
        if direction < 0.0 {
            xs.reverse();
        }
        xs.insert(0, from);
        xs.push(to);
        let mut samples = Vec::with_capacity(xs.len());
        let mut previous = alignment.at_x(from).0;
        let mut distance = 0.0;
        for x in xs {
            let (p, tangent) = alignment.at_x(x);
            let dx = p.x - previous.x;
            let dz = p.z - previous.z;
            distance += (dx * dx + dz * dz).sqrt();
            samples.push([
                distance,
                p.x,
                p.z,
                tangent.x * direction,
                tangent.z * direction,
            ]);
            previous = p;
        }
        Self { samples }
    }

    pub(crate) fn length(&self) -> f32 {
        self.samples.last().unwrap()[0]
    }

    pub(crate) fn at(&self, distance: f32) -> (P, P) {
        let distance = distance.clamp(0.0, self.length());
        let i = self
            .samples
            .partition_point(|s| s[0] < distance)
            .saturating_sub(1)
            .min(self.samples.len() - 2);
        let a = self.samples[i];
        let b = self.samples[i + 1];
        let t = (distance - a[0]) / (b[0] - a[0]);
        let blend = |index| a[index] + (b[index] - a[index]) * t;
        let (ux, uz) = (blend(3), blend(4));
        let length = (ux * ux + uz * uz).sqrt();
        (
            P {
                x: blend(1),
                z: blend(2),
            },
            P {
                x: ux / length,
                z: uz / length,
            },
        )
    }
}

pub(crate) fn paths(plan: &MasterPlan) -> BTreeMap<usize, Path> {
    let Some(alignment) = &plan.rail_alignment else {
        return BTreeMap::new();
    };
    plan.roads
        .iter()
        .enumerate()
        .filter(|(_, road)| road.kind == RoadKind::Rail)
        .map(|(index, road)| {
            (
                index,
                Path::new(alignment, plan.nodes[road.a].p.x, plan.nodes[road.b].p.x),
            )
        })
        .collect()
}

impl Engine {
    pub(crate) fn rail_alignment_views(&self) -> Vec<RailAlignmentView> {
        let (Some(runtime), Some(progress)) = (&self.plan, &self.state.masterplan) else {
            return Vec::new();
        };
        runtime
            .rail_paths
            .iter()
            .filter_map(|(index, path)| {
                let road_id = *progress.road_ids.get(*index)?;
                (road_id != 0).then(|| RailAlignmentView {
                    road_id,
                    samples: path.samples.clone(),
                })
            })
            .collect()
    }

    /// Route-oriented pose; the straight fallback is for plans without a curve.
    pub(crate) fn rail_pose(&self, from: u32, to: u32, distance: f32) -> Option<(P, P)> {
        if let (Some(runtime), Some(progress)) = (&self.plan, &self.state.masterplan) {
            for (index, path) in &runtime.rail_paths {
                if progress.road_ids[*index] == 0 {
                    continue;
                }
                let road = runtime.plan.roads[*index];
                let (a, b) = (progress.node_ids[road.a], progress.node_ids[road.b]);
                if a == from && b == to {
                    return Some(path.at(distance));
                }
                if a == to && b == from {
                    let (p, tangent) = path.at(path.length() - distance);
                    return Some((
                        p,
                        P {
                            x: -tangent.x,
                            z: -tangent.z,
                        },
                    ));
                }
            }
        }
        let (a, b) = (self.node(from)?, self.node(to)?);
        let (dx, dz) = (b.x - a.x, b.z - a.z);
        let length = crate::distance(a, b).max(1.0);
        let t = (distance / length).clamp(0.0, 1.0);
        Some((
            P {
                x: a.x + dx * t,
                z: a.z + dz * t,
            },
            P {
                x: dx / length,
                z: dz / length,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(seed: u32) -> Engine {
        let reference = Engine::new(seed);
        let plan = crate::masterplan::curved_rail_candidate(&reference.masterplan_geography());
        Engine::construct(seed, true, Some(plan))
    }

    #[test]
    fn realized_rail_and_snapshot_share_arc_length_and_station_tangents() {
        for seed in [87231, 0, 5] {
            let engine = candidate(seed);
            let snapshot = engine.snapshot();
            assert_eq!(snapshot.rail_alignments.len(), 1);
            let view = &snapshot.rail_alignments[0];
            let road = snapshot
                .roads
                .iter()
                .find(|r| r.id == view.road_id)
                .unwrap();
            let start = engine.node(road.from).unwrap();
            let end = engine.node(road.to).unwrap();
            assert_eq!(road.length, view.samples.last().unwrap()[0]);
            assert!(road.length > crate::distance(start, end));
            for sample in &view.samples {
                let (p, tangent) = engine.rail_pose(road.from, road.to, sample[0]).unwrap();
                assert!((p.x - sample[1]).abs() < 0.0001 && (p.z - sample[2]).abs() < 0.0001);
                assert!(
                    (tangent.x - sample[3]).abs() < 0.0001
                        && (tangent.z - sample[4]).abs() < 0.0001
                );
            }
            for layout in &snapshot.rail_platforms {
                assert!(
                    layout.uz.abs() < 0.0001,
                    "platform followed endpoint chord instead of straight approach"
                );
                let from_station = layout.node_id == road.from;
                for distance in [
                    0.0,
                    crate::TRAIN_HALF_LENGTH,
                    crate::RAIL_PLATFORM_END,
                    crate::RAIL_GATEWAY_STAIR_BOTTOM,
                ] {
                    if from_station && distance > crate::RAIL_PLATFORM_END {
                        continue;
                    }
                    let distance = if from_station {
                        distance
                    } else {
                        road.length - distance
                    };
                    assert!(
                        engine
                            .rail_pose(road.from, road.to, distance)
                            .unwrap()
                            .1
                            .z
                            .abs()
                            < 0.0001
                    );
                }
            }
            let mut changed = engine.plan.as_ref().unwrap().plan.clone();
            let digest = changed.digest();
            changed.rail_alignment = None;
            assert_ne!(
                digest,
                changed.digest(),
                "rail shape must participate in plan identity"
            );
        }
    }

    #[test]
    fn curved_train_moves_by_arc_distance_and_reverses_without_chord_snap() {
        let mut engine = candidate(87231);
        let rail = engine
            .state
            .roads
            .iter_mut()
            .find(|r| r.kind == "rail")
            .unwrap();
        rail.stage = "open".into();
        rail.progress = 1.0;
        rail.connected = true;
        let (from, to, length) = (rail.from, rail.to, rail.length);
        engine.state.station_online = true;
        engine.state.station_ready = true;
        engine.advance_trains();
        let train = &mut engine.state.trains[0];
        train.distance = length * 0.55;
        train.direction = 1;
        train.dwell_ticks = 0;
        let initial = train.distance;
        let speed = train.speed;
        engine.advance_trains();
        let train = &engine.state.trains[0];
        assert!((train.distance - initial - speed / crate::TICKS_PER_SECOND as f32).abs() < 0.0001);
        let (position, tangent) = engine.rail_pose(from, to, train.distance).unwrap();
        let movement = engine
            .movement_rows(true)
            .into_iter()
            .find(|r| r.kind == 2)
            .unwrap();
        assert_eq!((movement.x, movement.z), (position.x, position.z));
        assert!((movement.heading - tangent.z.atan2(tangent.x)).abs() < 0.0001);
        assert!(movement.resolved);
        let (a, b) = (engine.node(from).unwrap(), engine.node(to).unwrap());
        let t = train.distance / length;
        assert!(
            (position.z - (a.z + (b.z - a.z) * t)).abs() > 0.5,
            "fixture must exercise a real bend"
        );

        let train = &mut engine.state.trains[0];
        train.distance = length - crate::TRAIN_HALF_LENGTH - 0.1;
        train.dwell_ticks = 0;
        engine.advance_trains();
        let train = &engine.state.trains[0];
        assert_eq!(train.direction, -1);
        assert_eq!(train.distance, length - crate::TRAIN_HALF_LENGTH);
        let movement = engine
            .movement_rows(false)
            .into_iter()
            .find(|r| r.kind == 2)
            .unwrap();
        let expected = engine.rail_pose(from, to, train.distance).unwrap().0;
        assert_eq!((movement.x, movement.z), (expected.x, expected.z));
        assert!(movement.heading > 3.0);
    }

    #[test]
    fn production_curved_rail_save_replays() {
        for seed in [87231, 0, 5] {
            let mut live = Engine::new(seed);
            assert!(!live.snapshot().rail_alignments.is_empty());
            live.step(400);
            let saved = live.save_json();
            let mut restored = Engine::restore_json(&saved).unwrap();
            assert_eq!(saved, restored.save_json());
            live.step(400);
            restored.step(400);
            assert_eq!(live.save_json(), restored.save_json());
        }
    }

    #[test]
    fn persisted_rail_length_must_match_the_regenerated_curve() {
        let mut engine = candidate(0);
        engine.validate_masterplan_progress().unwrap();
        engine
            .state
            .roads
            .iter_mut()
            .find(|r| r.kind == "rail")
            .unwrap()
            .length += 1.0;
        assert!(engine
            .validate_masterplan_progress()
            .unwrap_err()
            .contains("arc length"));
    }

    #[test]
    #[ignore]
    fn export_sampling_fixture() {
        let output = std::env::var("RAIL_SAMPLING_FIXTURE").expect("RAIL_SAMPLING_FIXTURE path");
        let mut cases = Vec::new();
        for seed in [87231, 0, 5] {
            let engine = candidate(seed);
            for (road, path) in &engine.plan.as_ref().unwrap().rail_paths {
                let probes: Vec<_> = (0..=64)
                    .map(|i| {
                        let distance = path.length() * i as f32 / 64.0;
                        let (p, t) = path.at(distance);
                        [distance, p.x, p.z, t.x, t.z]
                    })
                    .collect();
                cases.push(serde_json::json!({"seed":seed,"road_id":road,
                    "samples":path.samples,"probes":probes}));
            }
        }
        std::fs::write(output, serde_json::to_string(&serde_json::json!({
            "scope":"Native candidate path samples and probes, both route directions; not a WASM parity proof",
            "cases":cases
        })).unwrap()).unwrap();
    }
}
