//! Tick-boundary integration of frozen discovery and fresh admission.
use crate::{access_capture, access_live::LiveAccess, access_planning_job::AccessPlanningJob,
    access_proposal_capture, Engine, WorldPoint, MAX_PARCELS, MAX_SEGMENTS};

const WORK_PER_TICK: usize = 4096;
const SEARCH_INTERVAL_TICKS: u64 = 1200;

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
        let Ok(job) = AccessPlanningJob::new(discovery, proposal, revision) else { return false; };
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
}
