//! WASM boundary for profiling and resuming frozen access discovery independently
//! of the live scheduler. Creating or advancing a job never mutates a city.
use crate::{access_capture, access_discovery_job::DiscoveryJob, Engine};
use wasm_bindgen::prelude::*;

const MAX_ADVANCE_WORK: usize = 65_536;

#[wasm_bindgen]
pub struct AccessDiscovery {
    inner: DiscoveryJob,
}

#[wasm_bindgen]
impl AccessDiscovery {
    /// Capture a validated city save atomically, then own only discovery inputs.
    #[wasm_bindgen(constructor)]
    pub fn new(city_save: &str) -> Result<AccessDiscovery, JsValue> {
        let engine = Engine::restore_json(city_save).map_err(|e| JsValue::from_str(&e))?;
        let input = access_capture::capture(&engine)
            .map_err(|e| JsValue::from_str(&format!("capture: {e:?}")))?;
        let inner = DiscoveryJob::new(input)
            .map_err(|e| JsValue::from_str(&format!("discovery: {e:?}")))?;
        Ok(Self { inner })
    }

    /// Fixed logical work, independent of host timing; hard cap limits one call.
    pub fn advance(&mut self, requested_work: u32) -> String {
        let step = self.inner.advance((requested_work as usize).min(MAX_ADVANCE_WORK));
        let (count, error) = match self.inner.descriptors() {
            Some(Ok(output)) => (Some(output.len()), None),
            Some(Err(error)) => (None, Some(format!("{error:?}"))),
            None => (None, None),
        };
        serde_json::json!({"work_used":step.work_used,"done":step.done,
            "descriptor_count":count,"error":error}).to_string()
    }

    /// Explicit persistence boundary; do not serialize this on every tick.
    pub fn save(&self) -> String {
        serde_json::to_string(&self.inner.checkpoint()).expect("validated finite discovery state")
    }

    pub fn restore(json: &str) -> Result<AccessDiscovery, JsValue> {
        let checkpoint = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("discovery JSON: {e}")))?;
        let inner = DiscoveryJob::restore(checkpoint)
            .map_err(|e| JsValue::from_str(&format!("discovery checkpoint: {e:?}")))?;
        Ok(Self { inner })
    }

    /// Exact Float32 bit tuples make cross-runtime parity independently testable.
    /// Export only at completion; this is a diagnostic boundary, not a tick path.
    pub fn descriptor_bits(&self) -> Result<String, JsValue> {
        let output = self.inner.descriptors()
            .ok_or_else(|| JsValue::from_str("discovery is unfinished"))?
            .map_err(|e| JsValue::from_str(&format!("discovery: {e:?}")))?;
        let bits: Vec<_> = output.iter().map(|c| [c.host_id,c.host_t.to_bits(),c.end_x.to_bits(),c.end_z.to_bits()]).collect();
        Ok(serde_json::to_string(&bits).expect("integer descriptor bits"))
    }
}

/// Shared captured world for profiling candidate validation in the actual WASM
/// runtime. Queries never mutate the captured city or attach to its scheduler.
#[wasm_bindgen]
pub struct AccessProposalWorld {
    context: std::sync::Arc<crate::access_proposal_validation::ProposalValidationContext>,
}

#[wasm_bindgen]
impl AccessProposalWorld {
    #[wasm_bindgen(constructor)]
    pub fn new(city_save: &str) -> Result<Self, JsValue> {
        let engine = Engine::restore_json(city_save).map_err(|e| JsValue::from_str(&e))?;
        let input = crate::access_proposal_capture::capture(&engine)
            .map_err(|e| JsValue::from_str(&format!("proposal capture: {e:?}")))?;
        let context = crate::access_proposal_validation::ProposalValidationContext::new(input)
            .map_err(|e| JsValue::from_str(&format!("proposal input: {e:?}")))?;
        Ok(Self { context })
    }

    pub fn query(&self, candidate_json: &str) -> Result<AccessProposalCheck, JsValue> {
        let candidate = serde_json::from_str(candidate_json)
            .map_err(|e| JsValue::from_str(&format!("candidate JSON: {e}")))?;
        Ok(AccessProposalCheck { inner: crate::access_proposal_validation::ProposalValidator::new(self.context.clone(), candidate) })
    }

    /// A checkpoint is bound to this world's immutable context fingerprint.
    pub fn restore_query(&self, checkpoint_json: &str) -> Result<AccessProposalCheck, JsValue> {
        let checkpoint = serde_json::from_str(checkpoint_json)
            .map_err(|e| JsValue::from_str(&format!("proposal JSON: {e}")))?;
        let inner = crate::access_proposal_validation::ProposalValidator::restore(checkpoint,self.context.clone())
            .map_err(|e| JsValue::from_str(&format!("proposal checkpoint: {e:?}")))?;
        Ok(AccessProposalCheck { inner })
    }
}

#[wasm_bindgen]
pub struct AccessProposalCheck {
    inner: crate::access_proposal_validation::ProposalValidator,
}

#[wasm_bindgen]
impl AccessProposalCheck {
    pub fn advance(&mut self, requested_work: u32) -> String {
        let step = self.inner.advance((requested_work as usize).min(MAX_ADVANCE_WORK));
        serde_json::json!({"work_used":step.work_used,"done":step.done}).to_string()
    }

    pub fn save(&self) -> String {
        serde_json::to_string(&self.inner.checkpoint()).expect("finite proposal checkpoint")
    }

    pub fn result(&self) -> String {
        match self.inner.outcome() {
            None => serde_json::json!({"done":false}),
            Some(Ok(proposal)) => serde_json::json!({"done":true,"proposal":proposal}),
            Some(Err(reason)) => serde_json::json!({"done":true,"proposal":null,"rejection":format!("{reason:?}")}),
        }.to_string()
    }
}
