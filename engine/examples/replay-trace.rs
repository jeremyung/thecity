use serde_json::{json, Value};
use tokyo_city_engine::Engine;

fn structures(snapshot: &Value) -> Value {
    let buildings = snapshot["buildings"].as_array().unwrap();
    let parcels = snapshot["parcels"].as_array().unwrap();
    let roads = snapshot["roads"].as_array().unwrap();
    let nodes = snapshot["nodes"].as_array().unwrap();
    let projects = snapshot["projects"].as_array().unwrap();
    json!({
        "tick": snapshot["tick"],
        "stage": snapshot["stage"],
        "population": snapshot["stats"]["population"],
        "nodes": nodes.iter().map(|n| json!({"id":n["id"],"x":n["x"],"z":n["z"],"kind":n["kind"]})).collect::<Vec<_>>(),
        "roads": roads.iter().map(|r| json!({"id":r["id"],"from":r["from"],"to":r["to"],"kind":r["kind"],"stage":r["stage"],"connected":r["connected"],"length":r["length"]})).collect::<Vec<_>>(),
        "parcels": parcels.iter().map(|p| json!({"id":p["id"],"x":p["x"],"z":p["z"],"width":p["width"],"depth":p["depth"],"access_node":p["access_node"],"protected":p["protected"],"reservation":p["reservation"]})).collect::<Vec<_>>(),
        "buildings": buildings.iter().map(|b| json!({"id":b["id"],"parcel_id":b["parcel_id"],"x":b["x"],"z":b["z"],"width":b["width"],"depth":b["depth"],"floors":b["floors"],"kind":b["kind"],"stage":b["stage"],"progress":b["progress"],"capacity":b["capacity"],"occupancy":b["occupancy"],"built_day":b["built_day"],"anchor":b["anchor"]})).collect::<Vec<_>>(),
        "projects": projects.iter().map(|p| json!({"id":p["id"],"kind":p["kind"],"from":p["from"],"to":p["to"],"parcel_id":p["parcel_id"],"stage":p["stage"],"progress":p["progress"],"duration":p["duration"],"reason":p["reason"],"started_tick":p["started_tick"]})).collect::<Vec<_>>(),
        "development_rate": snapshot["development_rate"],
    })
}

fn internal(state: &Value) -> Value {
    json!({
        "rng_growth": state["rng_growth"].as_u64().map(|v| v.to_string()),
        "rng_schedule": state["rng_schedule"].as_u64().map(|v| v.to_string()),
        "rng_weather": state["rng_weather"].as_u64().map(|v| v.to_string()),
        "development_accum": state["development_accum"],
        "growth_cursor": state["growth_cursor"],
        "road_origin_cursor": state["road_origin_cursor"],
        "road_infill_due": state["road_infill_due"],
        "frontage_road_cursor": state["frontage_road_cursor"],
        "frontage_slot_cursor": state["frontage_slot_cursor"],
        "neighborhood_plan": state["neighborhood_plan"],
        "demand": state["demand"],
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(87_231);
    let rate = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(16);
    let target = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(242_000u32);
    let interval = args
        .get(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000u32)
        .max(1);
    let mut engine = Engine::new(seed);
    engine.set_development_rate(rate);
    let snapshot: Value = serde_json::from_str(&engine.snapshot_json()).unwrap();
    let state: Value = serde_json::from_str(&engine.save_json()).unwrap();
    println!(
        "{}",
        json!({"structures":structures(&snapshot),"internal":internal(&state)})
    );
    let mut elapsed = 0;
    while elapsed < target {
        let delta = interval.min(target - elapsed);
        engine.step(delta);
        elapsed += delta;
        let snapshot: Value = serde_json::from_str(&engine.snapshot_json()).unwrap();
        let state: Value = serde_json::from_str(&engine.save_json()).unwrap();
        println!(
            "{}",
            json!({"structures":structures(&snapshot),"internal":internal(&state)})
        );
    }
}
