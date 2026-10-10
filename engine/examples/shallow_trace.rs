use std::collections::BTreeSet;
use std::fs;
use tokyo_city_engine::Engine;

fn hash64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn main() {
    let input = fs::read_to_string("validation/spine-ranking-seed-87231-by-sites-false.json")
        .expect("mature input");
    let mut engine = Engine::restore_json(&input).expect("restore");
    engine.set_development_rate(16);
    let before: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    let initial_save = engine.save_json();
    let initial_tick = before["tick"].as_u64().unwrap();
    let initial_ids: BTreeSet<u64> = before["parcels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_u64().unwrap())
        .collect();
    let initial_parcels = initial_ids.len();
    println!("{{\"tick\":{initial_tick},\"parcels\":{initial_parcels},\"save_hash\":\"{}\",\"event\":\"start\"}}", hash64(initial_save.as_bytes()));
    for _ in 0..100 {
        engine.step(200);
        let raw_save = engine.save_json();
        let snapshot: serde_json::Value = serde_json::from_str(&raw_save).unwrap();
        let tick = snapshot["tick"].as_u64().unwrap();
        if tick == 243_200 || tick == 251_400 {
            fs::write(format!("/tmp/shallow-native-{tick}.json"), &raw_save).unwrap();
        }
        let parcels = snapshot["parcels"].as_array().unwrap();
        let new: Vec<_> = parcels
            .iter()
            .filter(|p| {
                !initial_ids.contains(&p["id"].as_u64().unwrap())
                    && p["depth"].as_f64() == Some(8.0)
            })
            .collect();
        let newest: Vec<_> = new.iter().rev().take(3).map(|p| serde_json::json!({
            "id": p["id"], "x": p["x"], "z": p["z"], "width": p["width"], "depth": p["depth"],
            "road_id": p["road_id"], "access_node": p["access_node"]
        })).collect();
        let added: Vec<_> = new.iter().map(|p| p["id"].as_u64().unwrap()).collect();
        let summary = serde_json::json!({
            "tick": tick,
            "parcel_count": parcels.len(),
            "shallow_count": new.len(),
            "added_ids": added,
            "newest": newest,
            "rng_growth": raw_save
                .split("\"rng_growth\":")
                .nth(1)
                .and_then(|rest| rest.split(',').next())
                .unwrap(),
            "frontage_road_cursor": snapshot["frontage_road_cursor"],
            "frontage_slot_cursor": snapshot["frontage_slot_cursor"],
            "save_hash": hash64(raw_save.as_bytes())
        });
        println!("{}", summary);
    }
}
