use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: u32,
    pub tick: u64,
    pub kind: EventKind,
    pub entity_id: u32,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub population: u32,
    pub active_travelers: u32,
    pub buildings: u32,
    pub active_projects: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weather {
    pub kind: WeatherKind,
    pub intensity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorldState {
    pub(crate) schema_version: u32,
    #[serde(default)]
    pub(crate) activity_schema_version: u8,
    pub(crate) engine_version: String,
    pub(crate) seed: u32,
    pub(crate) rng_growth: u64,
    pub(crate) rng_schedule: u64,
    pub(crate) rng_weather: u64,
    pub(crate) tick: u64,
    pub(crate) development_rate: u32,
    pub(crate) development_accum: u64,
    pub(crate) revision: u64,
    pub(crate) next_id: u32,
    pub(crate) next_event_id: u32,
    pub(crate) events: VecDeque<Event>,
    pub(crate) weather_day: u32,
    pub(crate) weather_kind: WeatherKind,
    pub(crate) weather_intensity: f32,
    #[serde(flatten)]
    pub(crate) network: NetworkState,
    #[serde(flatten)]
    pub(crate) land: LandState,
    #[serde(flatten)]
    pub(crate) construction: ConstructionState,
    #[serde(flatten)]
    pub(crate) people: PeopleState,
    #[serde(flatten)]
    pub(crate) transit: TransitState,
    #[serde(flatten)]
    pub(crate) legacy: LegacyPlannerState,
}

impl WorldState {
    pub(crate) fn bump_access_geometry_revision(&mut self) {
        if self.network.access_geometry_revision_exhausted {
            return;
        }
        match self.network.access_geometry_revision.checked_add(1) {
            Some(next) => self.network.access_geometry_revision = next,
            None => {
                self.network.access_geometry_revision = u64::MAX;
                self.network.access_geometry_revision_exhausted = true;
            }
        }
    }
}
