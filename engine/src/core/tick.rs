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
        if day > self.state.weather_day {
            self.state.weather_day = day;
            let roll = random_unit(&mut self.state.rng_weather);
            if roll < 0.56 {
                self.state.weather_kind = WeatherKind::Clear;
                self.state.weather_intensity = 0.0;
            } else if roll < 0.84 {
                self.state.weather_kind = WeatherKind::Overcast;
                self.state.weather_intensity =
                    0.2 + random_unit(&mut self.state.rng_weather) * 0.25;
            } else {
                self.state.weather_kind = WeatherKind::Rain;
                self.state.weather_intensity =
                    0.35 + random_unit(&mut self.state.rng_weather) * 0.65;
            }
            self.state.revision = self.state.revision.saturating_add(1);
        }
        self.state.development_accum = self
            .state
            .development_accum
            .saturating_add(self.state.development_rate as u64);
        while self.state.development_accum >= DEVELOPMENT_TICKS_PER_DAY {
            self.state.development_accum -= DEVELOPMENT_TICKS_PER_DAY;
            self.development_day(day);
        }
        if self.state.tick % 40 == 0 {
            self.evaluate_growth();
        }
        if self.state.tick % POCKET_PARK_INTERVAL_TICKS == 0 && self.state.development_rate > 0 {
            self.dedicate_pocket_park();
        }
        self.trim_completed_projects();
        #[cfg(feature = "legacy")]
        self.advance_face_access();
        if self.state.tick % 200 == 0 {
            self.ensure_bus_service();
            self.schedule_residents(day);
            self.schedule_deliveries();
        }
        if self.state.tick % 10 == 0 {
            self.schedule_neighborhood_errands();
        }
        self.advance_travelers();
        self.advance_expressway_traffic();
        self.advance_trains();
        self.advance_buses();
        self.advance_pending_alights();
        if self.state.tick % 40 == 0 {
            self.state.revision = self.state.revision.saturating_add(1);
        }
        self.advance_district_measurement();
        #[cfg(test)]
        if self
            .experiments
            .early_block_reservation_started_tick
            .is_some()
        {
            if self.state.neighborhood_plan.is_some() {
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
