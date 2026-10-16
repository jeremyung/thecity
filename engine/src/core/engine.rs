use crate::*;
use std::collections::{BTreeSet, VecDeque};

/// Authoritative deterministic simulation. Public methods form the coarse WASM boundary.
pub struct Engine {
    pub(crate) state: WorldState,
    /// Regenerated master plan for `state.masterplan`; never serialized.
    pub(crate) plan: Option<std::sync::Arc<infrastructure::masterplan_growth::PlanRuntime>>,
    /// Bounded station-core renewal index derived from the regenerated plan.
    pub(crate) height_redevelopment_lots: Vec<usize>,
    pub(crate) planned_progress: Vec<f32>,
    pub(crate) wait_scratch: Vec<u16>,
    pub(crate) lane_order: Vec<usize>,
    pub(crate) district_runtime: population::district_controller::Runtime,
    pub(crate) live_access: Option<legacy::access_live::LiveAccess>,
    /// Non-persisted switch used to compare the first district-policy
    /// experiment against legacy scheduling. Production defaults to false.
    pub(crate) district_policy_experiment: bool,
    #[cfg(test)]
    pub(crate) experiments: experiments::ExperimentState,
}

impl Engine {
    /// Advance the persisted generation only after a committed change to the
    /// geometry, connectivity, reservation, or pending-road inputs used by
    /// access proposal validation. Never call from ordinary tick/revision
    /// updates.
    pub(crate) fn mark_access_geometry_changed(&mut self) {
        self.state.bump_access_geometry_revision();
    }

    /// `None` is fail-closed: a proposal cannot be admitted against an
    /// exhausted generation because subsequent mutations cannot be ordered.
    pub(crate) fn access_geometry_revision_token(&self) -> Option<u64> {
        (!self.state.access_geometry_revision_exhausted)
            .then_some(self.state.access_geometry_revision)
    }

    /// Roads/nodes change through the access generation. Platform construction
    /// and service readiness are independent, so include their discrete phases.
    /// Keep u64 generations as text across the JS boundary. On exhaustion the
    /// tick forces a fresh structural snapshot for each movement publication.
    pub fn presentation_revision(&self) -> String {
        let platform_phase = self.station_construction_phase();
        let generation = match self.access_geometry_revision_token() {
            Some(value) => format!("g{value}"),
            None => format!("t{}", self.state.tick),
        };
        format!(
            "{generation}:{}:{}:{}",
            platform_phase, self.state.station_ready as u8, self.state.station_online as u8
        )
    }

    /// New world grown from the seeded master plan.
    pub fn new(seed: u32) -> Self {
        Self::construct(seed, true, None)
    }

    /// New world using the retired incremental frontier/infill planner. Kept
    /// only for the legacy test fixtures until they are removed.
    #[cfg(feature = "legacy")]
    pub fn new_legacy(seed: u32) -> Self {
        Self::construct(seed, false, None)
    }

    pub(crate) fn construct(
        seed: u32,
        masterplan: bool,
        initial_plan: Option<infrastructure::masterplan::MasterPlan>,
    ) -> Self {
        let seed64 = (seed as u64).wrapping_add(0x9e3779b97f4a7c15).max(1);
        let mut engine = Self {
            plan: None,
            height_redevelopment_lots: Vec::new(),
            state: WorldState {
                schema_version: SCHEMA_VERSION,
                activity_schema_version: ACTIVITY_SCHEMA_VERSION,
                engine_version: ENGINE_VERSION.into(),
                seed,
                rng_growth: seed64 ^ 0xa0761d6478bd642f,
                rng_schedule: seed64 ^ 0xe7037ed1a0b428db,
                rng_weather: seed64 ^ 0x8ebc6af09c88c6e3,
                tick: 0,
                development_rate: 4,
                development_accum: 0,
                revision: 1,
                access_geometry_revision: 0,
                access_geometry_revision_exhausted: false,
                next_id: 1,
                next_event_id: 1,
                nodes: vec![],
                roads: vec![],
                parcels: vec![],
                parcel_frontages: vec![],
                buildings: vec![],
                projects: vec![],
                travelers: vec![],
                neighborhood_errands: population::neighborhood_errands::ErrandState::default(),
                expressway_traffic: transit::expressway_traffic::TrafficState::default(),
                residents: vec![],
                trains: vec![],
                bus_line: None,
                buses: vec![],
                next_bus_evaluation_tick: 0,
                pending_alight_ids: Vec::new(),
                pending_alight_cursor: 0,
                events: VecDeque::new(),
                population: 3,
                demand: 0.6,
                growth_cursor: 0,
                road_origin_cursor: 0,
                road_infill_due: 0,
                frontage_road_cursor: 0,
                frontage_slot_cursor: 0,
                neighborhood_plan: None,
                pending_interior_access: None,
                pending_rear_extensions: Vec::new(),
                rear_extension_cursor: 0,
                building_original_footprints: Vec::new(),
                height_redevelopment: None,
                height_redevelopment_cursor: 0,
                interior_access_cursor: 0,
                next_face_access_tick: 0,
                exhausted_access_faces: Vec::new(),
                live_access_checkpoint: None,
                masterplan: None,
                neighborhood_road_ids: BTreeSet::new(),
                neighborhood_anchor_cursor: 0,
                station_online: false,
                station_ready: false,
                weather_day: 0,
                weather_kind: WeatherKind::Clear,
                weather_intensity: 0.0,
                last_departure_day: u32::MAX,
                last_renewal_day: u32::MAX,
                last_business_turnover_day: 0,
                next_delivery_tick: TICKS_PER_DAY / 6,
                delivery_cursor: 0,
                district_controller:
                    population::district_controller::DistrictControllerState::default(),
            },
            planned_progress: Vec::new(),
            wait_scratch: Vec::new(),
            lane_order: Vec::new(),
            district_runtime: population::district_controller::Runtime::empty(),
            live_access: None,
            district_policy_experiment: false,
            #[cfg(test)]
            experiments: Default::default(),
        };
        if masterplan {
            engine.initialize_masterplan_world(initial_plan);
        } else {
            engine.initialize_world();
        }
        engine.refresh_height_redevelopment_lots();
        engine
    }

    pub(crate) fn id(&mut self) -> u32 {
        let id = self.state.next_id;
        self.state.next_id = self.state.next_id.saturating_add(1);
        id
    }

    pub(crate) fn initialize_world(&mut self) {
        // Every seed shares the river crossing and founding conditions, while its basin bends and parcel fabric vary.
        let gateway = self.add_node(365.0, 42.0, NodeKind::Gateway);
        let east = self.add_node(-18.0, 42.0, NodeKind::Junction);
        let west = self.add_node(-184.0, 42.0, NodeKind::Junction);
        let center = self.add_node(72.0, 42.0, NodeKind::Junction);
        let founder_z = 6.0;
        let founder_x = self.river_center_x_at(founder_z) + 55.0;
        let market = self.add_node(founder_x, 28.0, NodeKind::Market);
        let station = self.add_node(153.0, 58.0, NodeKind::Station);
        let (a, b) = (gateway, center);
        self.add_road(a, b, RoadKind::Arterial, Stage::Open, 1.0);
        self.add_road(center, east, RoadKind::Local, Stage::Open, 1.0);
        self.add_road(center, market, RoadKind::Local, Stage::Open, 1.0);
        // Keep the founding street graph planar: the market already connects
        // to center, so a direct market-to-station chord would cross the
        // center-to-east lane. The station instead joins at center.
        self.add_road(center, station, RoadKind::Local, Stage::Open, 1.0);
        self.add_road(east, west, RoadKind::Bridge, Stage::Survey, 0.0);
        let bridge_id = self.state.roads.last().unwrap().id;
        let parcel_id = self.add_parcel(founder_x, founder_z, 24.0, 18.0, market, 0.93, false);
        self.remember_nearest_frontage(parcel_id, market);
        let founder = self.add_building(
            parcel_id,
            BuildingKind::Shop,
            2,
            Stage::Open,
            1.0,
            12,
            "Kissa Aoba",
            true,
            vec!["Opened as the first neighborhood shop".into()],
        );
        let _ = founder;
        self.state.population = 8;
        let parcel = self.add_parcel(91.0, 72.0, 19.0, 16.0, center, 0.82, false);
        self.remember_nearest_frontage(parcel, center);
        let parcel = self.add_parcel(145.0, 18.0, 21.0, 18.0, market, 0.88, false);
        self.remember_nearest_frontage(parcel, market);
        let parcel = self.add_parcel(180.0, 94.0, 25.0, 20.0, station, 0.86, false);
        self.remember_nearest_frontage(parcel, station);
        let rail_b = self.add_node(300.0, 115.0, NodeKind::RailGateway);
        let rail_id = self.add_road(station, rail_b, RoadKind::Rail, Stage::Foundation, 0.0);
        let rail_project_id = self.id();
        self.state.projects.push(Project {
            id: rail_project_id,
            kind: ProjectKind::Rail,
            from: Some(station),
            to: Some(rail_b),
            parcel_id: None,
            stage: Stage::Survey,
            progress: 0.0,
            reason: "A short rail link is built from the station toward the regional gateway"
                .into(),
            duration: 4.0,
            started_tick: 0,
        });
        let station_id = self.id();
        self.state.projects.push(Project {
            id: station_id,
            kind: ProjectKind::Station,
            from: Some(station),
            to: Some(rail_b),
            parcel_id: None,
            stage: Stage::Platform,
            progress: 0.0,
            reason: "A rail stop is planned where the growing neighborhood meets the river route"
                .into(),
            duration: 6.0,
            started_tick: 0,
        });
        let bridge_project_id = self.id();
        self.state.projects.push(Project {
            id: bridge_project_id,
            kind: ProjectKind::Road,
            from: Some(east),
            to: Some(west),
            parcel_id: None,
            stage: Stage::Survey,
            progress: 0.0,
            reason: "The first cross-river bridge connects the founding lane to the western bank"
                .into(),
            duration: 8.0,
            started_tick: 0,
        });
        let _ = (bridge_id, rail_id, founder);
        self.record(
            EventKind::FoundingShopOpened,
            founder,
            "A small kissa opens beside the first lane".into(),
        );
        self.recompute_connectivity();
        self.state.revision += 1;
    }

    pub(crate) fn record(&mut self, kind: EventKind, entity_id: u32, reason: String) {
        let id = self.state.next_event_id;
        self.state.next_event_id = self.state.next_event_id.saturating_add(1);
        self.state.events.push_back(Event {
            id,
            tick: self.state.tick,
            kind,
            entity_id,
            reason,
        });
        while self.state.events.len() > MAX_EVENTS {
            self.state.events.pop_front();
        }
    }

    pub fn set_development_rate(&mut self, rate: u32) {
        self.state.development_rate = rate.clamp(0, 60);
    }
}
