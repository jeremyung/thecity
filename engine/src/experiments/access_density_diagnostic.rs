//! One-face rejection audit for the mature fragmented-access save.

use crate::legacy::access_discovery_job::DiscoveryJob;
use crate::legacy::access_proposal_validation::{
    ProposalValidationContext, ProposalValidator, RejectReason,
};
use crate::*;
use serde_json::json;

#[test]
#[ignore = "bounded one-face proposal rejection diagnosis; writes evidence to /tmp"]
fn diagnose_one_exhausted_face() {
    const TARGET: &[u32] = &[2656, 2657, 2772, 2773, 3131, 3010, 3011, 2895];
    const MAX_DISCOVERY_WORK: usize = 6_000_000;
    const MAX_VALIDATION_WORK: usize = 100_000_000;
    let save_path = std::env::var("DENSITY_AUDIT_SAVE")
        .expect("set DENSITY_AUDIT_SAVE to the save file to audit");
    let report_path = std::env::var("DENSITY_AUDIT_REPORT")
        .unwrap_or_else(|_| "/tmp/thecity-face-reject-diagnostic.json".into());
    let save = std::fs::read_to_string(&save_path).expect("read audit save");
    let engine = Engine::restore_json(&save).expect("restore mature save");
    let discovery_input = access_capture::capture(&engine).expect("capture discovery");
    let proposal_input = access_proposal_capture::capture(&engine).expect("capture validator");
    let footprint_data = proposal_input.viability.clone();
    let mut excluded = engine.state.legacy.exhausted_access_faces.clone();
    excluded.retain(|identity| identity.as_slice() != TARGET);
    let mut discovery =
        DiscoveryJob::new_excluding(discovery_input, excluded).expect("start discovery");
    let mut discovery_work = 0;
    while discovery.descriptors().is_none() {
        let step = discovery.advance(4096);
        discovery_work += step.work_used;
        assert!(
            discovery_work <= MAX_DISCOVERY_WORK,
            "discovery exceeded audit work ceiling"
        );
        assert!(step.work_used > 0 || step.done, "discovery stalled");
    }
    let selected = discovery
        .selected_face_identity()
        .unwrap_or_default()
        .to_vec();
    assert_eq!(
        selected.as_slice(),
        TARGET,
        "exclusions did not isolate target face"
    );
    let candidates = discovery.descriptors().unwrap().unwrap();
    let context =
        ProposalValidationContext::new(proposal_input.clone()).expect("validation context");
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut branch_conflict_detail = std::collections::BTreeMap::<String, usize>::new();
    // Nested prefixes distinguish an obstructed entrance from a straight ray
    // which only hits a parcel deeper inside the face. These are geometry
    // probes, not acceptance tests for an alternative road shape.
    let prefix_lengths = [0.0_f32, 5.0, 10.0, 20.0];
    let mut parcel_conflict_prefix_counts = [0usize; 4];
    let mut prefix_geometry_checks = 0usize;
    let mut samples = Vec::new();
    let mut validation_work = 0usize;
    for candidate in candidates {
        let mut validator = ProposalValidator::new(context.clone(), *candidate);
        while validator.outcome().is_none() {
            let step = validator.advance(4096);
            validation_work = validation_work.saturating_add(step.work_used);
            assert!(
                validation_work <= MAX_VALIDATION_WORK,
                "validation exceeded audit work ceiling"
            );
            assert!(step.work_used > 0 || step.done, "validator stalled");
        }
        match validator.outcome().unwrap() {
            Err(reason) => {
                *counts.entry(format!("{reason:?}")).or_default() += 1;
                if reason == RejectReason::BranchReservedOrParcelConflict {
                    let host = proposal_input
                        .hosts
                        .iter()
                        .find(|host| host.id == candidate.host_id)
                        .unwrap();
                    let sx =
                        host.segment.ax + (host.segment.bx - host.segment.ax) * candidate.host_t;
                    let sz =
                        host.segment.az + (host.segment.bz - host.segment.az) * candidate.host_t;
                    let reserved = footprint_data.reserved_lots.iter().any(|lot| {
                        geometry::rect_segment_distance(
                            lot,
                            sx,
                            sz,
                            candidate.end_x,
                            candidate.end_z,
                        ) < 5.0
                    });
                    let parcel = footprint_data.parcel_footprints.iter().any(|lot| {
                        geometry::rect_segment_distance(
                            lot,
                            sx,
                            sz,
                            candidate.end_x,
                            candidate.end_z,
                        ) < 5.0
                    });
                    let dx = candidate.end_x - sx;
                    let dz = candidate.end_z - sz;
                    let length = dx.hypot(dz);
                    assert!(length.is_finite() && length > 0.0);
                    for (index, prefix_length) in prefix_lengths.iter().enumerate() {
                        let t = (prefix_length / length).min(1.0);
                        let blocked = footprint_data.parcel_footprints.iter().any(|lot| {
                            prefix_geometry_checks += 1;
                            geometry::rect_segment_distance(lot, sx, sz, sx + dx * t, sz + dz * t)
                                < 5.0
                        });
                        parcel_conflict_prefix_counts[index] += usize::from(blocked);
                    }
                    let detail = match (reserved, parcel) {
                        (true, true) => "reserved_lot_and_existing_parcel",
                        (true, false) => "reserved_lot",
                        (false, true) => "existing_parcel_footprint",
                        (false, false) => "pending_branch_or_other_branch_stage",
                    };
                    *branch_conflict_detail.entry(detail.into()).or_default() += 1;
                }
                if samples.len() < 12 {
                    samples
                        .push(json!({"candidate": candidate, "rejection": format!("{reason:?}")}));
                }
            }
            Ok(proposal) => {
                *counts.entry("Accepted".into()).or_default() += 1;
                if samples.len() < 12 {
                    samples.push(json!({"candidate": candidate, "accepted_lots": proposal.selected_lots.len(), "area_cm2": proposal.score.site_area}));
                }
            }
        }
    }
    assert!(parcel_conflict_prefix_counts
        .windows(2)
        .all(|pair| pair[0] <= pair[1]));
    assert!(
        parcel_conflict_prefix_counts[3]
            <= counts
                .get("BranchReservedOrParcelConflict")
                .copied()
                .unwrap_or(0)
    );
    let report = json!({
        "tick": engine.state.tick,
        "save_path": save_path,
        "target_face_identity_as_captured": selected,
        "discovery_work": discovery_work,
        "discovery_work_ceiling": MAX_DISCOVERY_WORK,
        "validation_work": validation_work,
        "validation_work_ceiling": MAX_VALIDATION_WORK,
        "descriptor_count": candidates.len(),
        "rejection_counts": counts,
        "branch_conflict_spatial_subcounts": branch_conflict_detail,
        "parcel_conflict_prefix_lengths_m": prefix_lengths,
        "parcel_conflict_prefix_counts": parcel_conflict_prefix_counts,
        "prefix_geometry_checks": prefix_geometry_checks,
        "first_samples": samples,
        "note": "Outcomes use the target face selected from the supplied save and the exact proposal validator on that save's captured geometry. No live admission or alternate routing was attempted; results describe one face and one saved city state."
    });
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap())
        .expect("write diagnostic");
    println!("{report}");
}
