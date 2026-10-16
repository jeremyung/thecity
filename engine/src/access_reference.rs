//! Frozen mature inputs and ordered output for incremental search parity.
//! Regeneration is explicit in the ignored discovery diagnostic, never a test side effect.
use crate::{
    access_candidates::{BoundaryHost, CandidateNode},
    geometry::OrientedRect,
    road_faces::RoadFace,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ReferenceFace {
    identity: Vec<u32>,
    vertex_ids: Vec<u32>,
    polygon: Vec<(f32, f32)>,
    area: f64,
    boundary_edge_ids: Vec<u32>,
    contains_proposed_edge: bool,
}

#[derive(Deserialize)]
struct StoredReference {
    schema: u8,
    face: ReferenceFace,
    hosts: Vec<BoundaryHost>,
    nodes: Vec<CandidateNode>,
    parcels: Vec<OrientedRect>,
    targets: Vec<(f32, f32)>,
    descriptor_count: usize,
    ordered_descriptor_bits: Vec<[u32; 4]>,
}

pub(crate) struct DescriptorReference {
    pub face: RoadFace,
    pub hosts: Vec<BoundaryHost>,
    pub nodes: Vec<CandidateNode>,
    pub parcels: Vec<OrientedRect>,
    pub targets: Vec<(f32, f32)>,
    pub ordered_bits: Vec<[u32; 4]>,
}

pub(crate) fn load() -> DescriptorReference {
    let stored: StoredReference = serde_json::from_str(include_str!(
        "../../validation/access-descriptor-reference.json"
    ))
    .unwrap();
    assert_eq!(stored.schema, 1);
    assert_eq!(stored.descriptor_count, 3180);
    assert_eq!(
        stored.ordered_descriptor_bits.len(),
        stored.descriptor_count
    );
    DescriptorReference {
        face: RoadFace {
            identity: stored.face.identity,
            vertex_ids: stored.face.vertex_ids,
            polygon: stored.face.polygon,
            area: stored.face.area,
            boundary_edge_ids: stored.face.boundary_edge_ids,
            contains_proposed_edge: stored.face.contains_proposed_edge,
        },
        hosts: stored.hosts,
        nodes: stored.nodes,
        parcels: stored.parcels,
        targets: stored.targets,
        ordered_bits: stored.ordered_descriptor_bits,
    }
}

#[test]
fn offline_search_matches_frozen_mature_descriptor_sequence() {
    let reference = load();
    let search = crate::access_candidates::PreparedAccessSearch::prepare(
        &reference.face,
        &reference.hosts,
        &reference.nodes,
        &reference.parcels,
        &reference.targets,
    );
    let mut cursor = 0;
    let mut actual = Vec::new();
    loop {
        let page = search.page(cursor);
        for (index, candidate) in page.candidates.iter().enumerate() {
            assert_eq!(candidate.tie_index, index as u8);
            actual.push([
                candidate.host_id,
                candidate.host_t.to_bits(),
                candidate.end_x.to_bits(),
                candidate.end_z.to_bits(),
            ]);
        }
        cursor = page.next_cursor;
        if page.exhausted {
            break;
        }
    }
    assert_eq!(actual, reference.ordered_bits);
}

/// Explicit candidate export for reviewing a geometry change before migrating
/// the frozen golden. This never overwrites the accepted reference.
#[test]
#[ignore]
fn export_candidate_descriptor_sequence_for_cross_target_review() {
    let reference = load();
    let search = crate::access_candidates::PreparedAccessSearch::prepare(
        &reference.face,
        &reference.hosts,
        &reference.nodes,
        &reference.parcels,
        &reference.targets,
    );
    let mut cursor = 0;
    let mut bits = Vec::new();
    loop {
        let page = search.page(cursor);
        bits.extend(page.candidates.iter().map(|c| {
            [
                c.host_id,
                c.host_t.to_bits(),
                c.end_x.to_bits(),
                c.end_z.to_bits(),
            ]
        }));
        cursor = page.next_cursor;
        if page.exhausted {
            break;
        }
    }
    let mut stored: serde_json::Value = serde_json::from_str(include_str!(
        "../../validation/access-descriptor-reference.json"
    ))
    .unwrap();
    stored["descriptor_count"] = serde_json::json!(bits.len());
    stored["ordered_descriptor_bits"] = serde_json::json!(bits);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/access-descriptor-candidate.json");
    std::fs::write(path, serde_json::to_string_pretty(&stored).unwrap() + "\n").unwrap();
}
