//! Diagnostic-only comparison of deterministic work by top-level discovery stage.

#[cfg(test)]
mod tests {
    use crate::{access_capture, access_discovery_job::DiscoveryJob, Engine};
    use std::collections::BTreeMap;

    const SAVE: &str = include_str!("../../tests/fixtures/access-joins/early-reservation-wasm-87231-save.json");
    const CHUNK: usize = 65_536;

    fn outer_stage(job: &DiscoveryJob) -> String {
        let value = serde_json::to_value(job.checkpoint()).unwrap();
        let stage = value["stage"].as_object().unwrap();
        let name = stage.keys().next().unwrap();
        name.clone()
    }

    fn run_breakdown(mut job: DiscoveryJob) -> (BTreeMap<String, usize>, usize, usize) {
        let mut totals = BTreeMap::<String, usize>::new();
        let mut total = 0usize;
        while job.descriptors().is_none() {
            let before_stage = outer_stage(&job);
            let before = job.checkpoint();
            let step = job.advance(CHUNK);
            assert!(step.work_used > 0);
            let after_stage = outer_stage(&job);
            if before_stage == after_stage {
                *totals.entry(before_stage).or_default() += step.work_used;
                total += step.work_used;
                continue;
            }

            // Find the exact work unit where this top-level stage changes.
            // The number of top-level handoffs is small; this keeps the
            // diagnostic exact without serializing every one-unit advance.
            let mut low = 1usize;
            let mut high = step.work_used;
            while low < high {
                let middle = low + (high - low) / 2;
                let mut probe = DiscoveryJob::restore(before.clone()).unwrap();
                probe.advance(middle);
                if outer_stage(&probe) == before_stage {
                    low = middle + 1;
                } else {
                    high = middle;
                }
            }
            *totals.entry(before_stage.clone()).or_default() += low;
            total += low;
            job = DiscoveryJob::restore(before).unwrap();
            let boundary = job.advance(low);
            assert_eq!(boundary.work_used, low);
            assert_ne!(outer_stage(&job), before_stage);
        }
        let descriptor_count = job.descriptors().unwrap().as_ref().unwrap().len();
        (totals, total, descriptor_count)
    }

    #[test]
    #[ignore = "diagnostic native stage-work breakdown for native/WASM comparison"]
    fn export_native_top_level_work_breakdown() {
        let engine = Engine::restore_json(SAVE).unwrap();
        let input = access_capture::capture(&engine).unwrap();
        let job = DiscoveryJob::new(input).unwrap();
        let (stages, total, descriptor_count) = run_breakdown(job);
        let report = serde_json::json!({
            "source_save": "validation/early-reservation-wasm-87231-save.json",
            "source_save_sha256": "f5aecc3530a8607f115a352fe1caef73bdbcafaefd23d0d53410bdea9610aa2e",
            "chunk_work": CHUNK,
            "stages": stages,
            "total_work": total,
            "descriptor_count": descriptor_count,
            "scope": "native top-level stage accounting; binary-searches exact transition boundaries"
        });
        std::fs::write(
            "../validation/access-discovery-native-work-breakdown.json",
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        assert_eq!(total, 6_232_851);
        assert_eq!(descriptor_count, 2069);
    }
}
