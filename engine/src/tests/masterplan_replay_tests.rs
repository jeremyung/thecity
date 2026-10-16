//! Continue the live engine, rather than a second restored copy, across saves
//! taken during each new-world construction lifecycle.
use crate::*;

#[test]
fn live_masterplan_construction_matches_restored_continuation() {
    for seed in [0, 87_231] {
        let mut engine = Engine::new(seed);
        engine.set_development_rate(16);
        let mut covered = BTreeSet::new();
        while engine.state.tick < 160_000 && covered.len() < 4 {
            let active: BTreeSet<String> = engine
                .state
                .projects
                .iter()
                .filter(|p| {
                    p.progress > 0.0
                        && p.progress < 1.0
                        && ["building", "road", "rail", "expressway"].contains(&p.kind.as_str())
                })
                .map(|p| p.kind.clone())
                .filter(|kind| !covered.contains(kind))
                .collect();
            if active.is_empty() {
                engine.step(80);
                continue;
            }
            engine
                .validate()
                .expect("live construction state validates");
            let saved = engine.save_json();
            let saved_tick = engine.state.tick;
            let mut restored = Engine::restore_json(&saved).expect("restore active construction");
            assert_eq!(
                saved,
                restored.save_json(),
                "canonical seed {seed} tick {saved_tick}"
            );
            if let Some(directory) = std::env::var_os("MASTERPLAN_REPLAY_FIXTURE_DIR") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                for kind in &active {
                    std::fs::write(directory.join(format!("seed-{seed}-{kind}.json")), &saved)
                        .unwrap();
                }
            }
            // Cover intermediate publication boundaries as well as the final
            // state; the original engine keeps its pre-save in-memory caches.
            for _ in 0..20 {
                engine.step(80);
                restored.step(80);
                assert_eq!(
                    engine.save_json(),
                    restored.save_json(),
                    "seed {seed}, saved at {saved_tick}, kinds {active:?}"
                );
            }
            engine.validate().unwrap();
            restored.validate().unwrap();
            eprintln!("seed {seed}: replayed {active:?} from tick {saved_tick}");
            covered.extend(active);
        }
        assert_eq!(
            covered,
            BTreeSet::from([
                "building".to_owned(),
                "road".to_owned(),
                "rail".to_owned(),
                "expressway".to_owned()
            ]),
            "seed {seed} never exercised every construction kind"
        );
    }
}
