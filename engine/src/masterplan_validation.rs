//! Consistency checks for the persisted realization of a regenerated plan.

use std::collections::BTreeSet;

use crate::{frontage_access_node, Engine};

impl Engine {
    /// Verify that persisted plan indices still refer to the world entities
    /// they claim to realize. The plan itself is regenerated at load time.
    pub(crate) fn validate_masterplan_progress(&self) -> Result<(), String> {
        let Some(progress) = self.state.masterplan.as_ref() else {
            return if self.plan.is_none() {
                Ok(())
            } else {
                Err("runtime master plan exists without saved progress".into())
            };
        };
        let Some(runtime) = self.plan.as_ref() else {
            return Err("saved master-plan progress has no regenerated plan".into());
        };
        let plan = &runtime.plan;
        if progress.node_ids.len() != plan.nodes.len()
            || progress.road_ids.len() != plan.roads.len()
            || progress.lot_parcels.len() != plan.lots.len()
            || progress.plan_digest != plan.digest()
            || progress.expressway_next as usize > plan.expressway.len()
        {
            return Err("saved master-plan progress shape or digest is invalid".into());
        }

        let nodes = self
            .state
            .nodes
            .iter()
            .map(|n| (n.id, n))
            .collect::<std::collections::BTreeMap<_, _>>();
        let roads = self
            .state
            .roads
            .iter()
            .map(|r| (r.id, r))
            .collect::<std::collections::BTreeMap<_, _>>();
        let parcels = self
            .state
            .parcels
            .iter()
            .map(|p| (p.id, p))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut seen_nodes = BTreeSet::new();
        let mut seen_roads = BTreeSet::new();
        let mut seen_parcels = BTreeSet::new();

        for (i, id) in progress.node_ids.iter().copied().enumerate() {
            if id == 0 {
                continue;
            }
            if !seen_nodes.insert(id) {
                return Err(format!("master-plan node mapping {i} is not injective"));
            }
            let Some(node) = nodes.get(&id) else {
                return Err(format!(
                    "master-plan node mapping {i} references missing node {id}"
                ));
            };
            let expected = plan.nodes[i];
            let kind_ok = node.kind == expected.kind.as_str()
                || (i == plan.founding_market && node.kind == "market");
            if !near(node.x, expected.p.x, 1.0e-3)
                || !near(node.z, expected.p.z, 1.0e-3)
                || !kind_ok
            {
                return Err(format!(
                    "master-plan node mapping {i} does not match planned identity/geometry"
                ));
            }
        }
        for &i in &plan.founding_roads {
            if progress.road_ids.get(i).copied().unwrap_or(0) == 0 {
                return Err(format!("founding master-plan road mapping {i} is missing"));
            }
        }
        for (i, id) in progress.road_ids.iter().copied().enumerate() {
            if id == 0 {
                continue;
            }
            if !seen_roads.insert(id) {
                return Err(format!("master-plan road mapping {i} is not injective"));
            }
            let Some(road) = roads.get(&id) else {
                return Err(format!(
                    "master-plan road mapping {i} references missing road {id}"
                ));
            };
            let expected = plan.roads[i];
            let (Some(a), Some(b)) = (
                progress.node_ids.get(expected.a),
                progress.node_ids.get(expected.b),
            ) else {
                return Err(format!(
                    "master-plan road mapping {i} has invalid endpoint indices"
                ));
            };
            if *a == 0
                || *b == 0
                || road.kind != expected.kind.as_str()
                || !((road.from == *a && road.to == *b) || (road.from == *b && road.to == *a))
            {
                return Err(format!(
                    "master-plan road mapping {i} does not match planned endpoints/kind"
                ));
            }
        }

        // Founding lots are always released during initialization. Beyond
        // founding, zero means the frontage road has not opened yet (or its
        // parcel could not be admitted at the world cap).
        for &road_index in &plan.founding_roads {
            for (lot_index, _) in plan
                .lots
                .iter()
                .enumerate()
                .filter(|(_, lot)| lot.road == road_index)
            {
                if progress.lot_parcels[lot_index] == 0 {
                    return Err(format!(
                        "founding master-plan lot mapping {lot_index} is missing"
                    ));
                }
            }
        }
        for (i, id) in progress.lot_parcels.iter().copied().enumerate() {
            if id == 0 {
                continue;
            }
            if !seen_parcels.insert(id) {
                return Err(format!("master-plan lot mapping {i} is not injective"));
            }
            let Some(parcel) = parcels.get(&id) else {
                return Err(format!(
                    "master-plan lot mapping {i} references missing parcel {id}"
                ));
            };
            let lot = plan.lots[i];
            if !near(parcel.x, lot.x, 1.0e-3)
                || !near(parcel.z, lot.z, 1.0e-3)
                || !near(parcel.width, lot.width, 1.0e-3)
                || !near(parcel.depth, lot.depth, 1.0e-3)
            {
                return Err(format!(
                    "master-plan lot mapping {i} does not match planned geometry"
                ));
            }
            let road_id = progress.road_ids.get(lot.road).copied().unwrap_or(0);
            let Some(frontage) = self
                .state
                .parcel_frontages
                .iter()
                .find(|f| f.parcel_id == id)
            else {
                return Err(format!("master-plan lot mapping {i} has no frontage"));
            };
            if road_id == 0
                || frontage.road_id != road_id
                || !frontage.along.is_finite()
                || !(0.0..=1.0).contains(&frontage.along)
                || !matches!(frontage.side, -1 | 1)
            {
                return Err(format!(
                    "master-plan lot mapping {i} has inconsistent frontage"
                ));
            }
            let Some(road) = roads.get(&road_id) else {
                return Err(format!(
                    "master-plan lot mapping {i} frontage road is missing"
                ));
            };
            let (Some(a), Some(b)) = (nodes.get(&road.from), nodes.get(&road.to)) else {
                return Err(format!(
                    "master-plan lot mapping {i} frontage endpoint is missing"
                ));
            };
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let len2 = dx * dx + dz * dz;
            let projected = ((lot.x - a.x) * dx + (lot.z - a.z) * dz) / len2;
            let side = if dx * (lot.z - a.z) - dz * (lot.x - a.x) >= 0.0 {
                1
            } else {
                -1
            };
            let facing = (frontage.side as f32 * dz).atan2(-frontage.side as f32 * dx);
            if side != frontage.side
                || !near(projected.clamp(0.0, 1.0), frontage.along, 1.0e-4)
                || !parcel
                    .rotation
                    .is_some_and(|rotation| angle_near(rotation, facing, 1.0e-4))
                || parcel.access_node != frontage_access_node(a, b, frontage.along)
            {
                return Err(format!(
                    "master-plan lot mapping {i} has inconsistent frontage rotation"
                ));
            }
        }

        if seen_nodes.len() != nodes.len()
            || seen_roads.len() != roads.len()
            || seen_parcels.len() != parcels.len()
        {
            return Err("world contains entities missing from master-plan mappings".into());
        }
        if progress.rail_extension_started != (progress.road_ids[plan.rail_extension] != 0) {
            return Err("master-plan rail extension flag disagrees with its road".into());
        }
        if progress.rail_extension_started {
            if progress.road_ids[plan.rail_extension] == 0
                || progress.node_ids[plan.rail_terminus] == 0
            {
                return Err(
                    "master-plan rail extension flag has no realized extension entities".into(),
                );
            }
        }
        for &road_index in plan
            .expressway
            .iter()
            .take(progress.expressway_next as usize)
        {
            if progress.road_ids[road_index] == 0 {
                return Err("master-plan expressway cursor exceeds realized sections".into());
            }
        }
        for &road_index in plan
            .expressway
            .iter()
            .skip(progress.expressway_next as usize)
        {
            if progress.road_ids[road_index] != 0 {
                return Err("master-plan expressway exists beyond its cursor".into());
            }
        }
        Ok(())
    }
}

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= tolerance
}

fn angle_near(a: f32, b: f32, tolerance: f32) -> bool {
    let delta = (a - b).rem_euclid(std::f32::consts::TAU);
    delta.min(std::f32::consts::TAU - delta) <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn assert_corrupts<F>(mut corrupt: F)
    where
        F: FnMut(&mut Value),
    {
        let engine = Engine::new(87231);
        let mut save: Value = serde_json::from_str(&engine.save_json()).unwrap();
        corrupt(&mut save);
        assert!(
            Engine::restore_json(&save.to_string()).is_err(),
            "corrupt mapping was accepted"
        );
    }

    #[test]
    fn validates_initial_and_growth_saves() {
        let initial = Engine::new(87231);
        initial.validate_masterplan_progress().unwrap();
        let mut grown = Engine::new(87231);
        for _ in 0..6 {
            grown.step(2_000);
        }
        let restored = Engine::restore_json(&grown.save_json()).unwrap();
        restored.validate_masterplan_progress().unwrap();
    }

    #[test]
    fn rejects_missing_and_duplicate_node_mappings() {
        assert_corrupts(|s| {
            let ids = s["masterplan"]["node_ids"].as_array_mut().unwrap();
            let first = ids
                .iter()
                .position(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap();
            ids[first] = u32::MAX.into();
        });
        assert_corrupts(|s| {
            let ids = s["masterplan"]["node_ids"].as_array_mut().unwrap();
            let nonzero = ids
                .iter()
                .position(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap();
            let target = ids
                .iter()
                .enumerate()
                .find(|(i, id)| *i != nonzero && id.as_u64().unwrap_or(0) != 0)
                .unwrap()
                .0;
            let duplicate = ids[nonzero].clone();
            ids[target] = duplicate;
        });
    }

    #[test]
    fn rejects_missing_duplicate_and_swapped_road_mappings() {
        assert_corrupts(|s| {
            let ids = s["masterplan"]["road_ids"].as_array_mut().unwrap();
            let first = ids
                .iter()
                .position(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap();
            ids[first] = u32::MAX.into();
        });
        assert_corrupts(|s| {
            let ids = s["masterplan"]["road_ids"].as_array_mut().unwrap();
            let first = ids
                .iter()
                .position(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap();
            let target = ids
                .iter()
                .enumerate()
                .find(|(i, id)| *i != first && id.as_u64().unwrap_or(0) != 0)
                .unwrap()
                .0;
            let duplicate = ids[first].clone();
            ids[target] = duplicate;
        });
        assert_corrupts(|s| {
            let ids = s["masterplan"]["road_ids"].as_array_mut().unwrap();
            let first = ids
                .iter()
                .position(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap();
            let second = ids
                .iter()
                .enumerate()
                .find(|(i, id)| *i != first && id.as_u64().unwrap_or(0) != 0)
                .unwrap()
                .0;
            ids.swap(first, second);
        });
    }

    #[test]
    fn rejects_missing_duplicate_and_misplaced_lot_mappings() {
        assert_corrupts(|s| {
            let ids = s["masterplan"]["lot_parcels"].as_array_mut().unwrap();
            let first = ids
                .iter()
                .position(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap();
            ids[first] = u32::MAX.into();
        });
        assert_corrupts(|s| {
            let ids = s["masterplan"]["lot_parcels"].as_array_mut().unwrap();
            let first = ids
                .iter()
                .position(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap();
            let target = ids
                .iter()
                .enumerate()
                .find(|(i, id)| *i != first && id.as_u64().unwrap_or(0) != 0)
                .unwrap()
                .0;
            let duplicate = ids[first].clone();
            ids[target] = duplicate;
        });
        assert_corrupts(|s| {
            let id = s["masterplan"]["lot_parcels"]
                .as_array()
                .unwrap()
                .iter()
                .find(|id| id.as_u64().unwrap_or(0) != 0)
                .unwrap()
                .as_u64()
                .unwrap();
            let parcels = s["parcels"].as_array_mut().unwrap();
            let parcel = parcels
                .iter_mut()
                .find(|p| p["id"].as_u64() == Some(id))
                .unwrap();
            parcel["x"] = (parcel["x"].as_f64().unwrap() + 0.25).into();
        });
    }
    #[test]
    fn rejects_erased_mappings_and_invalid_frontage() {
        for field in ["node_ids", "road_ids", "lot_parcels"] {
            assert_corrupts(|s| {
                let ids = s["masterplan"][field].as_array_mut().unwrap();
                let i = ids.iter().position(|id| id.as_u64().unwrap() != 0).unwrap();
                ids[i] = 0.into();
            });
        }
        assert_corrupts(|s| s["parcels"][0]["rotation"] = Value::Null);
        assert_corrupts(|s| s["parcels"][0]["access_node"] = u32::MAX.into());
        assert_corrupts(|s| {
            let f = &mut s["parcel_frontages"][0];
            f["side"] = (-f["side"].as_i64().unwrap()).into();
        });
        assert_corrupts(|s| {
            let f = &mut s["parcel_frontages"][0];
            f["along"] = ((f["along"].as_f64().unwrap() + 0.2) % 1.0).into();
        });
    }
}
