use super::*;
use crate::rail_alignment::Alignment;

#[test]
#[ignore]
fn export_current_plan_alignment_prototypes() {
    let output = std::env::var("RAIL_ALIGNMENT_REPORT").expect("RAIL_ALIGNMENT_REPORT path");
    let mut cases = Vec::new();
    for seed in [87231, 0, 5] {
        let engine = crate::Engine::new(seed);
        let plan = &engine.plan.as_ref().unwrap().plan;
        let station = plan.nodes[plan.station].p;
        let path = Alignment::new(station, crate::masterplan::EDGE, seed);
        cases.push(
            serde_json::json!({"seed":seed,"station":[station.x,station.z],
            "length":path.length(),"samples":path.samples().iter().map(|(d,p)|
                [*d,p.x,p.z]).collect::<Vec<_>>() }),
        );
    }
    std::fs::write(
        output,
        serde_json::to_string_pretty(&serde_json::json!({
            "scope":"Geometry prototype only; not wired to live rail or parcel reservations",
            "cases":cases
        }))
        .unwrap(),
    )
    .unwrap();
}
