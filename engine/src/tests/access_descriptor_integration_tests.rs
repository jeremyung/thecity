//! Integration gate against immutable mature search inputs and Float32 outputs.
use crate::{access_descriptor_search::DescriptorSearch, tests::access_reference};

#[test]
#[ignore = "mature descriptor parity across work quotas and serialized resumes"]
fn incremental_descriptors_match_frozen_mature_reference() {
    let reference = access_reference::load();
    let mut runs = Vec::new();
    let mut expected_work = None;
    for quotas in [vec![1], vec![7], vec![48], vec![3, 127, 11, 48, 1]] {
        let started = std::time::Instant::now();
        let mut search = DescriptorSearch::new(
            &reference.face,
            &reference.hosts,
            &reference.nodes,
            &reference.parcels,
            &reference.targets,
        )
        .unwrap();
        let construction_us = started.elapsed().as_micros();
        assert_eq!(search.advance(0).work_used, 0);
        assert!(search.descriptors().is_none());
        let mut uninterrupted = DescriptorSearch::new(
            &reference.face,
            &reference.hosts,
            &reference.nodes,
            &reference.parcels,
            &reference.targets,
        )
        .unwrap();
        while !uninterrupted.advance(4096).done {}
        let uninterrupted_bits = uninterrupted
            .descriptors()
            .unwrap()
            .iter()
            .map(|c| {
                [
                    c.host_id,
                    c.host_t.to_bits(),
                    c.end_x.to_bits(),
                    c.end_z.to_bits(),
                ]
            })
            .collect::<Vec<_>>();
        for (i, (a, b)) in uninterrupted_bits
            .iter()
            .zip(&reference.ordered_bits)
            .enumerate()
        {
            assert_eq!(a, b, "uninterrupted {i}");
        }
        let initial = serde_json::to_vec(&search.checkpoint()).unwrap();
        search = DescriptorSearch::restore(serde_json::from_slice(&initial).unwrap()).unwrap();
        let mut total_work = 0usize;
        let mut calls = 0usize;
        let mut last_saved_work = 0usize;
        let mut resume_count = 1usize;
        let mut max_checkpoint_bytes = initial.len();
        loop {
            let budget = quotas[calls % quotas.len()];
            let result = search.advance(budget);
            assert!(result.work_used <= budget, "quota exceeded");
            total_work += result.work_used;
            calls += 1;
            if result.done {
                break;
            }
            assert!(result.work_used > 0, "unfinished search makes progress");
            assert!(total_work < 20_000_000, "bounded mature search terminates");
            if total_work - last_saved_work >= 16_381 {
                let bytes = serde_json::to_vec(&search.checkpoint()).unwrap();
                max_checkpoint_bytes = max_checkpoint_bytes.max(bytes.len());
                let decoded = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    serde_json::to_value(search.checkpoint()).unwrap(),
                    serde_json::to_value(&decoded).unwrap(),
                    "checkpoint JSON values at work {total_work}"
                );
                search = DescriptorSearch::restore(decoded).unwrap_or_else(|error| {
                    let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    panic!("restore failed at phase {}, work {total_work}, quotas {quotas:?}: {error:?}", saved["phase"]);
                });
                last_saved_work = total_work;
                resume_count += 1;
            }
        }
        let bytes = serde_json::to_vec(&search.checkpoint()).unwrap();
        max_checkpoint_bytes = max_checkpoint_bytes.max(bytes.len());
        search = DescriptorSearch::restore(serde_json::from_slice(&bytes).unwrap()).unwrap_or_else(
            |error| {
                let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                panic!(
                    "restore failed at phase {}, work {total_work}, quotas {quotas:?}: {error:?}",
                    saved["phase"]
                );
            },
        );
        let output = search.descriptors().unwrap();
        let bits = output
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                assert_eq!(
                    candidate.tie_index, index as u8,
                    "global exhaustive tie-index contract"
                );
                [
                    candidate.host_id,
                    candidate.host_t.to_bits(),
                    candidate.end_x.to_bits(),
                    candidate.end_z.to_bits(),
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(
            bits.len(),
            reference.ordered_bits.len(),
            "frozen mature count at quotas {quotas:?}, work={total_work}, resumes={resume_count}"
        );
        for (index, (actual, expected)) in bits.iter().zip(&reference.ordered_bits).enumerate() {
            assert_eq!(
                actual, expected,
                "frozen mature geometry/order at descriptor {index}, quotas {quotas:?}"
            );
        }
        if let Some(previous) = expected_work {
            assert_eq!(total_work, previous, "quota-independent work accounting");
        }
        expected_work = Some(total_work);
        let after = search.advance(48);
        assert!(after.done);
        assert_eq!(after.work_used, 0);
        runs.push(serde_json::json!({
            "quotas": quotas, "work_units": total_work, "advance_calls": calls,
            "resume_count": resume_count + 1, "max_checkpoint_json_bytes": max_checkpoint_bytes,
            "synchronous_construction_us": construction_us,
            "total_including_serialization_us": started.elapsed().as_micros(),
            "descriptor_count": bits.len()
        }));
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/access-descriptor-incremental-check.json");
    std::fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({
        "scope": "native descriptor preparation parity; not production scheduling or browser timing",
        "reference": "tests/fixtures/validation/access-descriptor-reference.json", "runs": runs
    })).unwrap()).unwrap();
}
