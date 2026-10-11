//! Tick-boundary integration of frozen discovery and fresh admission.
use crate::{access_capture, access_live::LiveAccess, access_planning_job::AccessPlanningJob,
    access_proposal_capture, Engine, WorldPoint, MAX_PARCELS, MAX_SEGMENTS};

const WORK_PER_TICK: usize = 4096;
const SEARCH_INTERVAL_TICKS: u64 = 1200;
const MAX_HISTORY_FACES: usize = 512;
const MAX_HISTORY_ITEMS: usize = 32_768;

pub(crate) fn validate_face_history(history: &[Vec<u32>]) -> Result<(), String> {
    if history.len() > MAX_HISTORY_FACES
        || history.iter().any(|ids| !(3..=1024).contains(&ids.len()))
        || history.iter().map(Vec::len).sum::<usize>() > MAX_HISTORY_ITEMS
    {
        return Err("invalid exhausted access face history".into());
    }
    Ok(())
}

fn remember_exhausted_face(history: &mut Vec<Vec<u32>>, identity: &[u32]) {
    if history.iter().any(|previous| previous == identity) { return; }
    let mut items: usize = history.iter().map(Vec::len).sum();
    while history.len() >= MAX_HISTORY_FACES || items + identity.len() > MAX_HISTORY_ITEMS {
        items -= history.remove(0).len();
    }
    history.push(identity.to_vec());
}

impl Engine {
    fn face_access_admission_available(&self) -> bool {
        self.access_geometry_revision_token().is_some()
            && self.state.pending_interior_access.is_none()
            && self.state.neighborhood_plan.is_none()
            && self.state.demand > 0.8
            && self.project_admission_available()
            && self.state.roads.len().saturating_add(self.state.projects.len()).saturating_add(3) <= MAX_SEGMENTS
            && self.state.parcels.len().saturating_add(2) <= MAX_PARCELS
            && self.state.next_id.checked_add(5).is_some()
            && !self.state.projects.iter().any(|p| p.kind == "road" && p.progress < 1.0)
    }

    /// Atomic input capture is outside the incremental work counter. Discovery
    /// never holds references into mutable engine storage and never restarts
    /// merely because the geometry revision changes during its search.
    fn start_face_access(&mut self) -> bool {
        if !self.face_access_admission_available() { return false; }
        let Ok(discovery) = access_capture::capture(self) else { return false; };
        let Ok(proposal) = access_proposal_capture::capture(self) else { return false; };
        let Some(revision) = self.access_geometry_revision_token() else { return false; };
        let Ok(job) = AccessPlanningJob::new_excluding(discovery, proposal, revision,
            self.state.exhausted_access_faces.clone()) else { return false; };
        self.live_access = Some(LiveAccess::new(job));
        true
    }

    /// One bounded planning slice and at most one atomic admission per tick.
    /// Runs before actor movement so split preflight/remapping uses the actors'
    /// current routes and positions, never a snapshot from an earlier tick.
    pub(crate) fn advance_face_access(&mut self) {
        if self.live_access.is_none() {
            if self.state.tick < self.state.next_face_access_tick { return; }
            self.state.next_face_access_tick = self.state.tick.saturating_add(SEARCH_INTERVAL_TICKS);
            let roads = self.state.roads.iter().filter(|r| r.stage == "open" && r.connected
                && matches!(r.kind.as_str(), "local" | "arterial")).count();
            if roads < 40 || !self.start_face_access() { return; }
        }
        let mut job = self.live_access.take().expect("started access job");
        if let Some(revision) = job.validation_revision() {
            if self.access_geometry_revision_token() != Some(revision) {
                job.discard_stale_validation();
            }
        }
        let used = job.advance(WORK_PER_TICK);
        if job.needs_validation().is_some() {
            if self.face_access_admission_available() {
                match (access_proposal_capture::capture(self), self.access_geometry_revision_token()) {
                    (Ok(input), Some(revision)) => {
                        if job.begin_validation(input, revision).is_err() { job.reject_or_consume(); }
                    }
                    _ => job.reject_or_consume(),
                }
            } else {
                job.reject_or_consume();
            }
        }
        let validation_work = job.advance_validation(WORK_PER_TICK - used);
        debug_assert!(used + validation_work <= WORK_PER_TICK);
        if let Some((proposal, revision)) = job.ready_proposal() {
            if self.access_geometry_revision_token() == Some(revision)
                && self.face_access_admission_available()
                && self.admit_interior_access(proposal.host_id, proposal.host_t,
                    WorldPoint { x: proposal.end_x, z: proposal.end_z }, proposal.selected_lots.clone())
            {
                self.state.next_face_access_tick = self.state.tick.saturating_add(SEARCH_INTERVAL_TICKS);
                return;
            }
            job.reject_or_consume();
        }
        if job.is_exhausted() {
            // Only completed searches advance the sweep. A failed diagnostic
            // job must not make a face appear searched or reset its history.
            if !job.failed() {
                if let Some(identity) = job.selected_face_identity() {
                    remember_exhausted_face(&mut self.state.exhausted_access_faces, identity);
                } else {
                    // No viable unvisited face remains. Retry all faces on the
                    // next sweep, including ones whose surroundings changed.
                    self.state.exhausted_access_faces.clear();
                }
            }
            self.state.next_face_access_tick = self.state.tick.saturating_add(SEARCH_INTERVAL_TICKS);
        } else {
            self.live_access = Some(job);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_face_sweep_survives_save_and_selects_another_interior() {
        let source = include_str!("../../validation/early-reservation-wasm-87231-save.json");
        let mut city = Engine::restore_json(source).unwrap();
        assert!(city.start_face_access());
        let mut job = city.live_access.take().unwrap();
        // Reject every candidate to exercise a genuinely exhausted search,
        // without mutating geometry or reserving any IDs.
        for _ in 0..100_000 {
            job.advance(WORK_PER_TICK);
            if job.needs_validation().is_some() { job.reject_or_consume(); }
            if job.is_exhausted() { break; }
        }
        assert!(job.is_exhausted() && !job.failed());
        let first = job.selected_face_identity().unwrap().to_vec();
        let next_id = city.state.next_id;
        let road_count = city.state.roads.len();
        city.live_access = Some(job);
        city.advance_face_access();
        assert_eq!(city.state.exhausted_access_faces, vec![first.clone()]);
        let saved = city.save_json();
        let mut city = Engine::restore_json(&saved).unwrap();
        assert_eq!(city.save_json(), saved);
        assert!(city.start_face_access());
        let job = city.live_access.as_mut().unwrap();
        for _ in 0..100_000 {
            job.advance(WORK_PER_TICK);
            if job.needs_validation().is_some() || job.is_exhausted() { break; }
        }
        assert!(!job.failed());
        assert_ne!(job.selected_face_identity().unwrap(), first.as_slice());
        assert_eq!(city.state.next_id, next_id);
        assert_eq!(city.state.roads.len(), road_count);
    }

    #[test]
    fn exhausted_history_is_bounded_and_invalid_saves_are_rejected() {
        let mut history = Vec::new();
        for id in 0..600 {
            remember_exhausted_face(&mut history, &[id, id + 1, id + 2]);
        }
        assert_eq!(history.len(), MAX_HISTORY_FACES);
        assert_eq!(history[0], vec![88, 89, 90]);
        let before = history.clone();
        remember_exhausted_face(&mut history, &[599, 600, 601]);
        assert_eq!(history, before);
        for id in 0..100 {
            remember_exhausted_face(&mut history, &vec![id; 1024]);
        }
        validate_face_history(&history).unwrap();
        assert!(history.iter().map(Vec::len).sum::<usize>() <= MAX_HISTORY_ITEMS);
        let source = include_str!("../../validation/early-reservation-wasm-87231-save.json");
        let mut city = Engine::restore_json(source).unwrap();
        city.state.exhausted_access_faces = vec![vec![1, 2]];
        assert!(Engine::restore_json(&city.save_json()).is_err());
        city.state.exhausted_access_faces = vec![vec![1; 1025]];
        assert!(Engine::restore_json(&city.save_json()).is_err());
    }

    #[test]
    fn mature_live_job_admits_known_route_and_resumes_with_moving_actors() {
        let source = include_str!("../../validation/early-reservation-wasm-87231-save.json");
        let mut uninterrupted = Engine::restore_json(source).unwrap();
        assert!(uninterrupted.start_face_access());
        let mut resumed = Engine::restore_json(&uninterrupted.save_json()).unwrap();
        let mut steps = 0;
        while uninterrupted.state.pending_interior_access.is_none() && steps < 10_000 {
            // Exercise the real tick-boundary hook and actor movement while
            // holding unrelated development constant for a controlled fixture.
            for city in [&mut uninterrupted, &mut resumed] {
                city.state.tick += 1;
                city.advance_face_access();
                city.advance_travelers();
            }
            steps += 1;
            if steps % 173 == 0 {
                resumed = Engine::restore_json(&resumed.save_json()).unwrap();
            }
        }
        assert!(uninterrupted.state.pending_interior_access.is_some(), "live job failed to admit after {steps} slices");
        assert_eq!(uninterrupted.save_json(), resumed.save_json());
        uninterrupted.validate().unwrap();
        let plan = uninterrupted.state.pending_interior_access.as_ref().unwrap();
        assert_eq!(plan.host_road_id, 3317);
        assert_eq!(plan.selected_lots.len(), 2);
        let branch_id = plan.branch_road_id;
        let mut opened = false;
        for index in 0..2000 {
            // After admission, construction and dependent frontage use the
            // ordinary simulation path, including all competing growth.
            uninterrupted.step(1);
            resumed.step(1);
            if index % 97 == 0 {
                resumed = Engine::restore_json(&resumed.save_json()).unwrap();
            }
            let branch = uninterrupted.state.roads.iter().find(|r| r.id == branch_id).unwrap();
            let frontages = uninterrupted.state.parcel_frontages.iter()
                .filter(|f| f.road_id == branch_id).count();
            if branch.stage != "open" {
                assert_eq!(frontages, 0, "dependent lots must wait for road opening");
            } else {
                assert!(branch.connected);
                assert_eq!(frontages, 2);
                opened = true;
                break;
            }
        }
        assert!(opened, "admitted road must complete through ordinary construction");
        assert_eq!(uninterrupted.save_json(), resumed.save_json());
        uninterrupted.validate().unwrap();
    }

    #[test]
    fn real_steps_resume_deterministically_across_geometry_churn() {
        let source = include_str!("../../validation/early-reservation-wasm-87231-save.json");
        let mut uninterrupted = Engine::restore_json(source).unwrap();
        assert!(uninterrupted.start_face_access());
        let mut resumed = Engine::restore_json(&uninterrupted.save_json()).unwrap();
        let initial_geometry_revision = uninterrupted.access_geometry_revision_token();
        let mut steps = 0;

        while uninterrupted.live_access.is_some() && steps < 10_000 {
            uninterrupted.step(1);
            resumed.step(1);
            steps += 1;

            // Insert two isolated but valid world nodes while frozen discovery
            // is pending. They change the live geometry revision without
            // depending on a proposal remaining admissible after the change.
            if steps == 5 || steps == 13 {
                let offset = steps as f32;
                uninterrupted.add_node(490.0 - offset, -490.0 + offset, "test");
                resumed.add_node(490.0 - offset, -490.0 + offset, "test");
                assert!(uninterrupted.live_access.is_some());
                assert_eq!(
                    uninterrupted.access_geometry_revision_token(),
                    resumed.access_geometry_revision_token()
                );
            }

            if steps % 17 == 0 {
                resumed = Engine::restore_json(&resumed.save_json()).unwrap();
            }
        }

        assert!(steps < 10_000, "real-step live job exceeded the bounded fixture run");
        assert_ne!(uninterrupted.access_geometry_revision_token(), initial_geometry_revision);
        assert_eq!(uninterrupted.save_json(), resumed.save_json());
        uninterrupted.validate().unwrap();
        resumed.validate().unwrap();
    }

    #[test]
    fn legacy_preflight_restart_preserves_mature_city_and_is_deterministic() {
        let source = include_str!("../../validation/early-reservation-wasm-87231-save.json");
        let mut city = Engine::restore_json(source).unwrap();
        assert!(city.start_face_access());

        // Advance the real mature fixture until discovery reaches the graph
        // preflight checkpoint, then retain that exact nested save path.
        let path = [
            "live_access_checkpoint", "planning", "discovery", "stage", "Trace", "trace",
            "stage", "Preflight",
        ];
        let mut current = serde_json::from_str::<serde_json::Value>(&city.save_json()).unwrap();
        for _ in 0..100 {
            if path.iter().fold(&current, |value, key| &value[*key]).is_object() { break; }
            city.state.tick += 1;
            city.advance_face_access();
            current = serde_json::from_str(&city.save_json()).unwrap();
        }
        let legacy_preflight = path.iter().fold(&current, |value, key| &value[*key]);
        assert!(legacy_preflight.is_object(), "fixture did not reach nested preflight");
        assert!(legacy_preflight["work_completed"].as_u64().unwrap() > 0);
        assert!(legacy_preflight["nodes"].as_array().unwrap().len() > 100,
            "expected a mature graph preflight, got {} nodes", legacy_preflight["nodes"].as_array().unwrap().len());

        let mut legacy = current.clone();
        let preflight = path.iter().fold(&mut legacy, |value, key| &mut value[*key]);
        preflight["schema"] = serde_json::json!(1);
        for field in [
            "filter_work", "filter_build_cursor", "filter_fallback", "filter_checkpoint",
            "pending_pair", "last_checked_pair",
        ] {
            preflight.as_object_mut().unwrap().remove(field);
        }
        if let Some(path) = std::env::var_os("LEGACY_PREFLIGHT_SAVE_PATH") {
            std::fs::write(path, legacy.to_string()).unwrap();
        }
        let restored = Engine::restore_json(&legacy.to_string()).unwrap();
        let mut expected_world = legacy.clone();
        expected_world.as_object_mut().unwrap().remove("live_access_checkpoint");
        let mut actual_world = serde_json::from_str::<serde_json::Value>(&restored.save_json()).unwrap();
        actual_world.as_object_mut().unwrap().remove("live_access_checkpoint");
        assert_eq!(actual_world, expected_world, "migration must preserve every world field");

        let migrated_save = restored.save_json();
        let migrated_json: serde_json::Value = serde_json::from_str(&migrated_save).unwrap();
        let migrated_preflight = path.iter().fold(&migrated_json, |value, key| &value[*key]);
        assert_eq!(migrated_preflight["schema"], serde_json::json!(2));
        assert_eq!(migrated_preflight["stage"], serde_json::json!("Nodes"));
        assert_eq!(migrated_preflight["node_cursor"], serde_json::json!(0));
        assert_eq!(migrated_preflight["nodes"], legacy_preflight["nodes"]);
        assert_eq!(migrated_preflight["edges"], legacy_preflight["edges"]);
        assert_eq!(migrated_preflight["proposed_edge"], legacy_preflight["proposed_edge"]);
        assert_eq!(Engine::restore_json(&migrated_save).unwrap().save_json(), migrated_save);
        let mut first = Engine::restore_json(&legacy.to_string()).unwrap();
        let mut second = Engine::restore_json(&legacy.to_string()).unwrap();
        for _ in 0..24 {
            first.step(1);
            second.step(1);
        }
        assert_eq!(first.save_json(), second.save_json());

        let mut corrupt_cursor = serde_json::from_str::<serde_json::Value>(&migrated_save).unwrap();
        let preflight = path.iter().fold(&mut corrupt_cursor, |value, key| &mut value[*key]);
        preflight["node_cursor"] = serde_json::json!(usize::MAX);
        assert!(Engine::restore_json(&corrupt_cursor.to_string()).is_err());

        let mut future_schema = serde_json::from_str::<serde_json::Value>(&migrated_save).unwrap();
        let preflight = path.iter().fold(&mut future_schema, |value, key| &mut value[*key]);
        preflight["schema"] = serde_json::json!(3);
        assert!(Engine::restore_json(&future_schema.to_string()).is_err());
    }
}
