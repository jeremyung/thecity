//! Contract checks for passage-facing parcels; not generator/visual acceptance.
use crate::*;

fn passage_world() -> (Engine, usize, masterplan::PlanLot) {
    let source = Engine::new(0);
    let mut plan = source.plan.as_ref().unwrap().plan.clone();
    let market = plan.nodes[plan.founding_market].p;
    let index = plan
        .lots
        .iter()
        .enumerate()
        .filter(|(_, lot)| {
            plan.founding_roads.contains(&lot.road) && lot.width > 9.0 && lot.depth > 9.0
        })
        .max_by(|(_, a), (_, b)| {
            ((a.x - market.x).hypot(a.z - market.z))
                .total_cmp(&((b.x - market.x).hypot(b.z - market.z)))
        })
        .map(|(i, _)| i)
        .expect("a roomy founding lot");
    let lot = &mut plan.lots[index];
    let entrance = masterplan::P {
        x: lot.x + lot.facing.x * (lot.depth / 2.0 + 0.5),
        z: lot.z + lot.facing.z * (lot.depth / 2.0 + 0.5),
    };
    // The small rotated footprint stays inside the original cleared lot.
    lot.width = 7.0;
    lot.depth = 7.0;
    lot.rotation += 0.35;
    lot.facing = masterplan::P {
        x: lot.rotation.sin(),
        z: lot.rotation.cos(),
    };
    let threshold = masterplan::P {
        x: lot.x + lot.facing.x * 4.4,
        z: lot.z + lot.facing.z * 4.4,
    };
    lot.passage = Some([
        threshold,
        masterplan::P {
            x: (threshold.x + entrance.x) * 0.5,
            z: (threshold.z + entrance.z) * 0.5,
        },
        entrance,
    ]);
    lot.flag = true;
    let expected = *lot;
    (Engine::construct(0, true, Some(plan)), index, expected)
}

#[test]
fn passage_facing_survives_release_building_and_access() {
    let (mut engine, index, lot) = passage_world();
    let parcel_id = engine.state.masterplan.as_ref().unwrap().lot_parcels[index];
    assert_ne!(parcel_id, 0);
    let parcel = engine
        .state
        .parcels
        .iter()
        .find(|p| p.id == parcel_id)
        .unwrap();
    assert_eq!(parcel.rotation, Some(lot.rotation));
    assert_eq!(engine.frontage_rotation(parcel_id), lot.rotation);
    assert!(!engine
        .state
        .buildings
        .iter()
        .any(|b| b.parcel_id == parcel_id));
    let building_id = engine.add_building(
        parcel_id,
        "home",
        2,
        "open",
        1.0,
        8,
        "Passage test",
        false,
        vec![],
    );
    let building = engine
        .state
        .buildings
        .iter()
        .find(|b| b.id == building_id)
        .unwrap();
    assert_eq!(building.rotation, lot.rotation);
    let (road, points) = engine.building_access_segment(building_id).unwrap();
    assert_eq!(
        road,
        engine.state.masterplan.as_ref().unwrap().road_ids[lot.road]
    );
    assert_eq!(points.len(), 5);
    for (actual, expected) in points[1..4].iter().zip(lot.passage.unwrap()) {
        assert_eq!((actual.x, actual.z), (expected.x, expected.z));
    }
    engine.validate_masterplan_progress().unwrap();
    engine.validate().unwrap();
    // Street access remains on the real road, while a corrupt building-facing
    // orientation is rejected rather than silently repaired on save/load.
    engine
        .state
        .parcels
        .iter_mut()
        .find(|p| p.id == parcel_id)
        .unwrap()
        .rotation = Some(lot.rotation + 0.1);
    assert!(engine.validate_masterplan_progress().is_err());
    assert!(engine.validate().is_err());
}

#[test]
fn candidate_without_new_passages_replays_current_save() {
    let mut live = Engine::new(0);
    live.step(400);
    let mut restored = Engine::restore_json(&live.save_json()).unwrap();
    live.step(100);
    restored.step(100);
    assert_eq!(live.save_json(), restored.save_json());
    live.validate().unwrap();
    restored.validate().unwrap();
}

#[test]
fn generated_parent_sites_build_with_clear_access_and_replay() {
    for seed in [87231, 0, 5] {
        let mut engine = Engine::new(seed);
        engine.set_development_rate(16);
        let sites: Vec<_> = engine
            .plan
            .as_ref()
            .unwrap()
            .plan
            .lots
            .iter()
            .enumerate()
            .filter(|(_, lot)| lot.passage.is_some())
            .map(|(i, _)| i)
            .collect();
        assert!(
            !sites.is_empty(),
            "seed {seed}: candidate must exercise generated sites"
        );
        for _ in 0..50 {
            engine.step(2000);
        }
        engine.validate().unwrap();
        for &index in &sites {
            let parcel_id = engine.state.masterplan.as_ref().unwrap().lot_parcels[index];
            let building = engine
                .state
                .buildings
                .iter()
                .find(|b| b.parcel_id == parcel_id)
                .expect("generated passage site completed by mature checkpoint");
            let (road, path) = engine.building_access_segment(building.id).unwrap();
            let parcel = engine
                .state
                .parcels
                .iter()
                .find(|p| p.id == parcel_id)
                .unwrap();
            assert!(engine.node_connected(parcel.access_node));
            assert!(engine
                .state
                .roads
                .iter()
                .any(|r| r.id == road && r.stage == "open"));
            for other in &engine.state.parcels {
                if other.id == parcel_id {
                    continue;
                }
                let rect = parcel_envelope(other);
                for pair in path.windows(2) {
                    assert!(
                        geometry::rect_segment_distance(
                            &rect, pair[0].x, pair[0].z, pair[1].x, pair[1].z
                        ) >= 1.08,
                        "seed {seed}: passage parcel {parcel_id} crosses parcel {}",
                        other.id
                    );
                }
            }
        }
        if let Ok(directory) = std::env::var("PARENT_PASSAGE_FIXTURES") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                format!("{directory}/seed{seed}-tick100000.save.json"),
                engine.save_json(),
            )
            .unwrap();
            std::fs::write(
                format!("{directory}/seed{seed}-tick100000.world.json"),
                engine.snapshot_json(),
            )
            .unwrap();
        }
        let mut restored = Engine::restore_json(&engine.save_json()).unwrap();
        engine.step(100);
        restored.step(100);
        assert_eq!(engine.save_json(), restored.save_json(), "seed {seed}");
        eprintln!("seed {seed}: {} generated passage sites built with clear access; 100-tick replay matched",sites.len());
    }
}
