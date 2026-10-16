//! wasm-bindgen bindings: the coarse boundary the worker calls. Bindings only.
use super::Engine;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct CityEngine {
    inner: Engine,
}
#[wasm_bindgen]
impl CityEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32) -> CityEngine {
        CityEngine {
            inner: Engine::new(seed),
        }
    }
    /// Legacy incremental planner, kept for comparison fixtures only.
    #[cfg(feature = "legacy")]
    #[wasm_bindgen(js_name=newLegacy)]
    pub fn new_legacy(seed: u32) -> CityEngine {
        CityEngine {
            inner: Engine::new_legacy(seed),
        }
    }
    pub fn step(&mut self, ticks: u32) {
        self.inner.step(ticks)
    }
    #[wasm_bindgen(js_name=snapshot)]
    pub fn snapshot(&self) -> String {
        self.inner.snapshot_json()
    }
    pub fn presentation_revision(&self) -> String {
        self.inner.presentation_revision()
    }
    #[wasm_bindgen(js_name=movements)]
    pub fn movements(&self) -> Vec<f32> {
        self.inner.movement_buffer()
    }
    #[wasm_bindgen(js_name=movement_packet)]
    pub fn movement_packet(&self) -> Vec<u32> {
        self.inner.movement_packet()
    }
    #[wasm_bindgen(js_name=train_doors)]
    pub fn train_doors(&self) -> Vec<f32> {
        self.inner.train_doors()
    }
    #[wasm_bindgen(js_name=inspect)]
    pub fn inspect(&self, id: u32) -> String {
        self.inner.inspect_json(id)
    }
    #[wasm_bindgen(js_name=save)]
    pub fn save(&self) -> String {
        self.inner.save_json()
    }
    #[wasm_bindgen(js_name=restore)]
    pub fn restore(json: &str) -> Result<CityEngine, JsValue> {
        Engine::restore_json(json)
            .map(|inner| CityEngine { inner })
            .map_err(|e| JsValue::from_str(&e))
    }
    /// Opt-in benchmark boundary. Actor preflight and commit remain in one
    /// atomic call; city restore and save serialization are timed separately.
    #[cfg(feature = "profiling")]
    pub fn split_open_road(&mut self, road_id: u32, fraction: f32) -> Result<String, JsValue> {
        self.inner
            .try_split_open_road(road_id, fraction)
            .map(|outcome| {
                serde_json::json!({
                "split_node_id": outcome.split_node_id,
                "retained_road_id": outcome.retained_road_id,
                "new_road_id": outcome.new_road_id,
                })
                .to_string()
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }
    #[wasm_bindgen(js_name=set_development_rate)]
    pub fn set_development_rate(&mut self, rate: u32) {
        self.inner.set_development_rate(rate)
    }
}
