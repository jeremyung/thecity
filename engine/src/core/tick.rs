use crate::*;

impl Engine {
    /// Advance a fixed number of 50ms ticks. Cap batches to keep worker scheduling bounded.
    pub fn step(&mut self, ticks: u32) {
        let count = ticks.min(2_000);
        for _ in 0..count {
            self.step_one();
        }
    }

    pub(crate) fn step_one(&mut self) {
        self.state.tick = self.state.tick.saturating_add(1);
        let (day, _) = clock_at(self.state.tick);
        self.run_systems(&systems::TickContext { day });
        #[cfg(test)]
        if self
            .experiments
            .early_block_reservation_started_tick
            .is_some()
        {
            if self.state.legacy.neighborhood_plan.is_some() {
                self.experiments.early_block_reservation_duration_ticks = self
                    .experiments
                    .early_block_reservation_duration_ticks
                    .saturating_add(1);
            } else {
                self.experiments.early_block_reservation_started_tick = None;
            }
        }
    }

    pub fn tick(&self) -> u64 {
        self.state.tick
    }
}
