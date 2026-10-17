//! The tick schedule. Each system declares its phase and cadence; `SYSTEMS`
//! runs them in order every tick. To add a system, write its function in the
//! owning domain module and register it here at the position its phase needs.
//! Order is part of determinism: moving an entry changes replays.
use crate::*;

/// Where a system sits in the tick. Phases run in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Phase {
    /// Clock-driven world conditions (weather).
    Environment,
    /// Growth, construction and renewal.
    Development,
    /// Schedules trips for residents, deliveries and errands.
    Demand,
    /// Moves actors along their routes.
    Movement,
    /// Publication bookkeeping for the renderer.
    Publication,
    /// Bounded background measurement.
    Measurement,
}

/// How often a system runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cadence {
    EveryTick,
    /// Runs when `tick % period == 0`.
    Every(u64),
}

impl Cadence {
    fn due(self, tick: u64) -> bool {
        match self {
            Cadence::EveryTick => true,
            Cadence::Every(period) => tick % period == 0,
        }
    }
}

/// Values every system may read for the current tick.
pub(crate) struct TickContext {
    pub(crate) day: u32,
}

pub(crate) struct System {
    pub(crate) name: &'static str,
    pub(crate) phase: Phase,
    pub(crate) cadence: Cadence,
    pub(crate) run: fn(&mut Engine, &TickContext),
}

pub(crate) const SYSTEMS: &[System] = &[
    System {
        name: "weather",
        phase: Phase::Environment,
        cadence: Cadence::EveryTick,
        run: |engine, ctx| engine.advance_weather(ctx.day),
    },
    System {
        name: "development",
        phase: Phase::Development,
        cadence: Cadence::EveryTick,
        run: |engine, ctx| engine.accrue_development(ctx.day),
    },
    System {
        name: "growth",
        phase: Phase::Development,
        cadence: Cadence::Every(40),
        run: |engine, _| engine.evaluate_growth(),
    },
    System {
        name: "pocket_parks",
        phase: Phase::Development,
        cadence: Cadence::Every(POCKET_PARK_INTERVAL_TICKS),
        run: |engine, _| {
            if engine.state.development_rate > 0 {
                engine.dedicate_pocket_park();
            }
        },
    },
    System {
        name: "project_history",
        phase: Phase::Development,
        cadence: Cadence::EveryTick,
        run: |engine, _| engine.trim_completed_projects(),
    },
    System {
        name: "legacy_face_access",
        phase: Phase::Development,
        cadence: Cadence::EveryTick,
        run: |_engine, _| {
            #[cfg(feature = "legacy")]
            _engine.advance_face_access();
        },
    },
    System {
        name: "bus_service",
        phase: Phase::Demand,
        cadence: Cadence::Every(200),
        run: |engine, _| engine.ensure_bus_service(),
    },
    System {
        name: "resident_trips",
        phase: Phase::Demand,
        cadence: Cadence::Every(200),
        run: |engine, ctx| engine.schedule_residents(ctx.day),
    },
    System {
        name: "deliveries",
        phase: Phase::Demand,
        cadence: Cadence::Every(200),
        run: |engine, _| engine.schedule_deliveries(),
    },
    System {
        name: "neighborhood_errands",
        phase: Phase::Demand,
        cadence: Cadence::Every(10),
        run: |engine, _| engine.schedule_neighborhood_errands(),
    },
    System {
        name: "travelers",
        phase: Phase::Movement,
        cadence: Cadence::EveryTick,
        run: |engine, _| engine.advance_travelers(),
    },
    System {
        name: "expressway_traffic",
        phase: Phase::Movement,
        cadence: Cadence::EveryTick,
        run: |engine, _| engine.advance_expressway_traffic(),
    },
    System {
        name: "trains",
        phase: Phase::Movement,
        cadence: Cadence::EveryTick,
        run: |engine, _| engine.advance_trains(),
    },
    System {
        name: "buses",
        phase: Phase::Movement,
        cadence: Cadence::EveryTick,
        run: |engine, _| engine.advance_buses(),
    },
    System {
        name: "pending_alights",
        phase: Phase::Movement,
        cadence: Cadence::EveryTick,
        run: |engine, _| engine.advance_pending_alights(),
    },
    System {
        name: "periodic_revision",
        phase: Phase::Publication,
        cadence: Cadence::Every(40),
        run: |engine, _| engine.state.revision = engine.state.revision.saturating_add(1),
    },
    System {
        name: "district_measurement",
        phase: Phase::Measurement,
        cadence: Cadence::EveryTick,
        run: |engine, _| engine.advance_district_measurement(),
    },
];

impl Engine {
    pub(crate) fn run_systems(&mut self, ctx: &TickContext) {
        let tick = self.state.tick;
        for system in SYSTEMS {
            if system.cadence.due(tick) {
                (system.run)(self, ctx);
            }
        }
    }

    pub(crate) fn advance_weather(&mut self, day: u32) {
        if day <= self.state.weather_day {
            return;
        }
        self.state.weather_day = day;
        let roll = random_unit(&mut self.state.rng_weather);
        if roll < 0.56 {
            self.state.weather_kind = WeatherKind::Clear;
            self.state.weather_intensity = 0.0;
        } else if roll < 0.84 {
            self.state.weather_kind = WeatherKind::Overcast;
            self.state.weather_intensity = 0.2 + random_unit(&mut self.state.rng_weather) * 0.25;
        } else {
            self.state.weather_kind = WeatherKind::Rain;
            self.state.weather_intensity = 0.35 + random_unit(&mut self.state.rng_weather) * 0.65;
        }
        self.state.revision = self.state.revision.saturating_add(1);
    }

    pub(crate) fn accrue_development(&mut self, day: u32) {
        self.state.development_accum = self
            .state
            .development_accum
            .saturating_add(self.state.development_rate as u64);
        while self.state.development_accum >= DEVELOPMENT_TICKS_PER_DAY {
            self.state.development_accum -= DEVELOPMENT_TICKS_PER_DAY;
            self.development_day(day);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systems_are_registered_in_phase_order_with_unique_names() {
        let mut names = std::collections::BTreeSet::new();
        for pair in SYSTEMS.windows(2) {
            assert!(
                pair[0].phase <= pair[1].phase,
                "{} before {}",
                pair[0].name,
                pair[1].name
            );
        }
        for system in SYSTEMS {
            assert!(
                names.insert(system.name),
                "duplicate system {}",
                system.name
            );
        }
    }
}
