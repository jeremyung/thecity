//! Plan-only audit of large station-core lot availability and realized towers.

use crate::masterplan::{self, District};
use crate::Engine;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const SEEDS: [u32; 4] = [0, 5, 77, 87_231];

fn area_band(area: f32) -> &'static str {
    match area {
        a if a < 100.0 => "0-99",
        a if a < 200.0 => "100-199",
        a if a < 280.0 => "200-279",
        a if a < 400.0 => "280-399",
        a if a < 600.0 => "400-599",
        a if a < 1_000.0 => "600-999",
        _ => "1000+",
    }
}

fn seed_report(seed: u32, fixture: Option<&Value>) -> Value {
    let engine = Engine::new(seed);
    let plan = masterplan::generate(&engine.masterplan_geography());
    let core: Vec<_> = plan
        .lots
        .iter()
        .enumerate()
        .filter(|(_, lot)| lot.district == District::StationCore)
        .collect();
    let mut area_distribution = BTreeMap::<String, usize>::new();
    for (_, lot) in &core {
        *area_distribution
            .entry(area_band(lot.width * lot.depth).into())
            .or_default() += 1;
    }
    let mut core_areas: Vec<f32> = core.iter().map(|(_, lot)| lot.width * lot.depth).collect();
    core_areas.sort_by(f32::total_cmp);
    let eligible: Vec<_> = core
        .iter()
        .filter(|(_, lot)| !lot.flag && lot.width * lot.depth >= 280.0)
        .map(|(index, lot)| {
            json!({
                "lot_index": index,
                "x": lot.x,
                "z": lot.z,
                "area_m2": lot.width * lot.depth,
                "width_m": lot.width,
                "depth_m": lot.depth,
                "flag": lot.flag,
                "arterial_frontage": lot.arterial_frontage,
            })
        })
        .collect();

    let realized = fixture.map(|save| {
        assert_eq!(save["seed"].as_u64(), Some(seed as u64));
        let lot_parcels = save["masterplan"]["lot_parcels"]
            .as_array()
            .expect("fixture masterplan lot_parcels");
        assert_eq!(lot_parcels.len(), plan.lots.len(), "fixture plan lot count differs");
        assert_eq!(
            save["masterplan"]["plan_digest"].as_u64(),
            Some(plan.digest()),
            "fixture plan digest differs"
        );
        let buildings: BTreeMap<u32, &Value> = save["buildings"]
            .as_array()
            .expect("fixture buildings")
            .iter()
            .filter_map(|building| {
                Some((building["parcel_id"].as_u64()? as u32, building))
            })
            .collect();
        let lot_records: Vec<_> = core
            .iter()
            .filter(|(_, lot)| !lot.flag && lot.width * lot.depth >= 280.0)
            .map(|(index, lot)| {
                let parcel_id = lot_parcels[*index].as_u64().unwrap_or_default() as u32;
                let building = buildings.get(&parcel_id).copied();
                json!({
                    "lot_index": index,
                    "parcel_id": (parcel_id != 0).then_some(parcel_id),
                    "x": lot.x,
                    "z": lot.z,
                    "area_m2": lot.width * lot.depth,
                    "realized": building.is_some(),
                    "floors": building.and_then(|b| b["floors"].as_u64()),
                    "kind": building.and_then(|b| b["kind"].as_str()),
                })
            })
            .collect();
        json!({
            "station_online": save["station_online"],
            "eligible_lots": lot_records,
            "eligible_realized_count": lot_records.iter().filter(|r| r["realized"] == true).count(),
            "eligible_tower_count": lot_records.iter().filter(|r| r["floors"].as_u64().unwrap_or(0) >= 14).count(),
        })
    });

    json!({
        "seed": seed,
        "station": {"x": plan.nodes[plan.station].p.x, "z": plan.nodes[plan.station].p.z},
        "station_core_lot_count": core.len(),
        "station_core_area_distribution_m2": area_distribution,
        "station_core_lot_areas_m2": core_areas,
        "station_core_area_min_m2": core_areas.first(),
        "station_core_area_median_m2": core_areas.get(core_areas.len() / 2),
        "station_core_area_max_m2": core_areas.last(),
        "eligible_nonflag_core_lot_count": eligible.len(),
        "eligible_area_min_m2": eligible.iter().map(|lot| lot["area_m2"].as_f64().unwrap()).reduce(f64::min),
        "eligible_area_max_m2": eligible.iter().map(|lot| lot["area_m2"].as_f64().unwrap()).reduce(f64::max),
        "eligible_lots": eligible,
        "fixture_realization": realized,
    })
}

#[test]
#[ignore = "bounded plan-only station tower eligibility audit; writes JSON evidence"]
fn diagnose_station_tower_allocation_opportunity() {
    let fixture_path = std::env::var("SKYLINE_ALLOCATION_FIXTURE").ok();
    let fixture = fixture_path.as_ref().map(|path| {
        let text = std::fs::read_to_string(path).expect("read optional realization fixture");
        serde_json::from_str::<Value>(&text).expect("parse optional realization fixture")
    });
    let reports: Vec<_> = SEEDS
        .iter()
        .map(|seed| {
            seed_report(
                *seed,
                (*seed == 87_231).then_some(fixture.as_ref()).flatten(),
            )
        })
        .collect();
    let report = json!({
        "diagnostic": "station tower allocation opportunity",
        "tower_site_rule": "non-flag StationCore lot with width*depth >= 280m2",
        "random_selection_probability_per_selected_eligible_lot": 0.2,
        "seeds": reports,
        "fixture_path": fixture_path,
        "note": "Plan-only counts measure geometric opportunities. Optional fixture join reports realized buildings for eligible lots; it does not simulate random selection or growth."
    });
    let output_path = std::env::var("SKYLINE_ALLOCATION_REPORT")
        .unwrap_or_else(|_| "/tmp/thecity-skyline-allocation-diagnostic.json".into());
    std::fs::write(&output_path, serde_json::to_vec_pretty(&report).unwrap())
        .expect("write skyline allocation report");
    println!("{report}");
}
