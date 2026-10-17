use crate::*;

impl Engine {
    pub fn save_json(&self) -> String {
        if let Some(job) = &self.live_access {
            let mut saved = self.state.clone();
            saved.legacy.live_access_checkpoint = Some(job.checkpoint());
            serde_json::to_string(&saved).expect("serializing engine state cannot fail")
        } else {
            serde_json::to_string(&self.state).expect("serializing engine state cannot fail")
        }
    }

    pub fn restore_json(json: &str) -> Result<Self, String> {
        let mut state: WorldState = serde_json::from_str(json).map_err(|e| e.to_string())?;
        legacy::access_live_engine::validate_face_history(&state.legacy.exhausted_access_faces)?;
        if state.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported engine save schema {}",
                state.schema_version
            ));
        }
        if state.activity_schema_version != ACTIVITY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported business activity schema {}; start a fresh world",
                state.activity_schema_version
            ));
        }
        // Validate frozen checkpoint shapes and timeline before replaying any
        // saved prefix to reconstruct derived accumulator state.
        state.people.district_controller.validate(state.tick)?;
        let district_runtime =
            population::district_controller::Runtime::restore(&state.people.district_controller)?;
        let mut migrated_access_geometry = false;
        if let Some(plan) = &state.legacy.neighborhood_plan {
            for (from, to) in &plan.segments {
                if let Some(road) = state.network.roads.iter().find(|road| {
                    (road.from == *from && road.to == *to) || (road.from == *to && road.to == *from)
                }) {
                    migrated_access_geometry |= state.legacy.neighborhood_road_ids.insert(road.id);
                }
            }
        }
        if migrated_access_geometry {
            state.bump_access_geometry_revision();
        }
        let live_access = state
            .legacy
            .live_access_checkpoint
            .take()
            .map(legacy::access_live::LiveAccess::restore)
            .transpose()
            .map_err(|error| format!("invalid live access checkpoint: {error:?}"))?;
        let mut engine = Self {
            state,
            plan: None,
            height_redevelopment_lots: Vec::new(),
            live_access,
            planned_progress: Vec::new(),
            wait_scratch: Vec::new(),
            lane_order: Vec::new(),
            district_runtime,
            district_policy_experiment: false,
            #[cfg(test)]
            experiments: Default::default(),
        };
        engine.load_masterplan()?;
        engine.refresh_height_redevelopment_lots();
        // Validate the serialized active plan before canonicalizing a
        // completed one; malformed empty plans must not disappear through
        // vacuous all() checks during migration.
        engine.validate().map_err(|e| e.to_string())?;
        if !engine
            .state
            .legacy
            .neighborhood_plan
            .as_ref()
            .is_some_and(|plan| plan.spine.is_some())
        {
            engine.retire_completed_neighborhood_plan();
        }
        engine.validate().map_err(|e| e.to_string())?;
        Ok(engine)
    }
}
