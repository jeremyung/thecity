//! Engine orchestration for master-plan growth.
//!
//! The pure plan (`masterplan.rs`) is regenerated from the seed on creation and
//! restore. Saves persist only realization progress: which plan nodes, roads
//! and lots exist in the world. Growth realizes eligible streets from the open
//! network, releases their lots as parcels when they open, and lets ordinary
//! demand-gated building projects fill those parcels.

use crate::BuildingKind;
use crate::EventKind;
use crate::{ProjectKind, Stage};
use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::infrastructure::masterplan::{
    self, District, Geography, MasterPlan, NodeKind, PlanLot, RoadKind, P,
};
use crate::{
    frontage_access_node, random_unit, site_active, site_vacant, Engine, ParcelFrontage, Project,
    MAX_PROJECTS, RIVER_BANK_BUFFER, RIVER_HALF_WIDTH, TICKS_PER_DAY,
};

/// Street-facing plan lots this close to the planned station become its plaza.
const STATION_PLAZA_RADIUS: f32 = 45.0;
const MAX_STATION_PLAZA_LOTS: usize = 2;
/// Shrines stay out of the station core.
const SHRINE_STATION_CLEARANCE: f32 = 150.0;
/// One neighborhood shrine per this distance, at most six per city.
const SHRINE_SPACING: f32 = 260.0;
const MAX_SHRINES: usize = 6;
const SHRINE_AREA: std::ops::RangeInclusive<f32> = 150.0..=600.0;

/// Persisted realization state. Indices follow the regenerated plan; zero
/// means "not yet realized" because world IDs start at one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct MasterplanProgress {
    pub(crate) node_ids: Vec<u32>,
    pub(crate) road_ids: Vec<u32>,
    pub(crate) lot_parcels: Vec<u32>,
    #[serde(default)]
    pub(crate) rail_extension_started: bool,
    #[serde(default)]
    pub(crate) expressway_next: u32,
    /// Digest of the plan this progress belongs to.
    pub(crate) plan_digest: u64,
}

/// Regenerated plan plus derived lookup tables. Never serialized.
#[derive(Clone, Debug)]
pub(crate) struct PlanRuntime {
    pub(crate) plan: MasterPlan,
    pub(crate) lots_by_road: Vec<Vec<usize>>,
    pub(crate) expressway_network: crate::transit::expressway_traffic::Network,
    pub(crate) rail_paths: BTreeMap<usize, crate::transit::rail_runtime::Path>,
}

impl PlanRuntime {
    fn new(plan: MasterPlan) -> Self {
        let mut lots_by_road = vec![Vec::new(); plan.roads.len()];
        for (index, lot) in plan.lots.iter().enumerate() {
            lots_by_road[lot.road].push(index);
        }
        Self {
            rail_paths: crate::transit::rail_runtime::paths(&plan),
            expressway_network: crate::transit::expressway_traffic::Network::from_plan(&plan),
            plan,
            lots_by_road,
        }
    }
}

const MAX_ACTIVE_ROAD_PROJECTS: usize = 3;
/// Station-core towers (16-32 floors) per city.
const MAX_STATION_TOWERS: usize = 8;
const MAX_ACTIVE_BUILDING_PROJECTS: usize = 9;
const BUILDINGS_PER_EVALUATION: usize = 3;
/// Keep a modest stock of released parcels ahead of construction.
const RELEASED_PARCEL_TARGET: usize = 36;

impl Engine {
    pub(crate) fn masterplan_geography(&self) -> Geography {
        let river = self.river_control_points();
        Geography {
            seed: self.state.seed,
            river: river.map(|p| P { x: p.x, z: p.z }).to_vec(),
            river_clearance: RIVER_HALF_WIDTH + RIVER_BANK_BUFFER,
            protected: (-350.0, -350.0, -260.0, -270.0),
        }
    }

    /// Regenerate the plan for a restored world and check that the saved
    /// progress belongs to it.
    pub(crate) fn load_masterplan(&mut self) -> Result<(), String> {
        let Some(progress) = self.state.masterplan.as_ref() else {
            self.plan = None;
            if cfg!(feature = "legacy") {
                return Ok(());
            }
            return Err(
                "this save has no master plan; legacy worlds are no longer supported".into(),
            );
        };
        let plan = masterplan::generate(&self.masterplan_geography());
        if progress.node_ids.len() != plan.nodes.len()
            || progress.road_ids.len() != plan.roads.len()
            || progress.lot_parcels.len() != plan.lots.len()
            || progress.expressway_next as usize > plan.expressway.len()
            || progress.plan_digest != plan.digest()
        {
            return Err("saved master-plan progress does not match the regenerated plan".into());
        }
        self.plan = Some(Arc::new(PlanRuntime::new(plan)));
        Ok(())
    }

    pub(crate) fn masterplan_active(&self) -> bool {
        self.state.masterplan.is_some() && self.plan.is_some()
    }

    fn plan_runtime(&self) -> Arc<PlanRuntime> {
        self.plan.clone().expect("master plan is loaded")
    }

    /// Founding conditions for a master-plan world.
    pub(crate) fn initialize_masterplan_world(&mut self, initial_plan: Option<MasterPlan>) {
        let plan =
            initial_plan.unwrap_or_else(|| masterplan::generate(&self.masterplan_geography()));
        self.state.masterplan = Some(MasterplanProgress {
            node_ids: vec![0; plan.nodes.len()],
            road_ids: vec![0; plan.roads.len()],
            lot_parcels: vec![0; plan.lots.len()],
            rail_extension_started: false,
            expressway_next: 0,
            plan_digest: plan.digest(),
        });
        self.plan = Some(Arc::new(PlanRuntime::new(plan)));
        let runtime = self.plan_runtime();
        let plan = &runtime.plan;

        // Open the gateway arterial and the founding row, releasing its lots.
        for &road in &plan.founding_roads {
            self.realize_plan_road(road, Stage::Open, 1.0);
        }
        if let Some(node) = self.plan_node_id(plan.founding_market) {
            if let Some(node) = self
                .state
                .nodes
                .iter_mut()
                .find(|candidate| candidate.id == node)
            {
                node.kind = crate::NodeKind::Market;
            }
        }
        self.recompute_connectivity();
        for &road in &plan.founding_roads {
            self.release_plan_lots(road);
        }

        // The founding shop takes the released lot closest to the market.
        let market = self
            .state
            .nodes
            .iter()
            .find(|node| node.kind == crate::NodeKind::Market)
            .map(|node| (node.x, node.z));
        let founder_parcel = market.and_then(|(mx, mz)| {
            self.state
                .parcels
                .iter()
                .min_by(|a, b| {
                    (a.x - mx)
                        .hypot(a.z - mz)
                        .total_cmp(&(b.x - mx).hypot(b.z - mz))
                })
                .map(|parcel| parcel.id)
        });
        if let Some(parcel_id) = founder_parcel {
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
            if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
                parcel.reservation = None;
            }
            self.record(
                EventKind::FoundingShopOpened,
                founder,
                "A small kissa opens beside the first lane".into(),
            );
        }
        self.state.population = 8;

        // The served rail segment and station platform begin construction at
        // founding; service starts once streets reach the station.
        let station = self.realize_plan_node(plan.station);
        let rail_gateway = self.realize_plan_node(plan.rail_gateway);
        self.realize_plan_road(plan.rail_service, Stage::Foundation, 0.0);
        let rail_project_id = self.id();
        self.state.projects.push(Project {
            id: rail_project_id,
            kind: ProjectKind::Rail,
            from: Some(station),
            to: Some(rail_gateway),
            parcel_id: None,
            stage: Stage::Survey,
            progress: 0.0,
            reason: "An elevated rail link is built from the station toward the regional gateway"
                .into(),
            duration: 6.0,
            started_tick: 0,
        });
        let station_project_id = self.id();
        self.state.projects.push(Project {
            id: station_project_id,
            kind: ProjectKind::Station,
            from: Some(station),
            to: Some(rail_gateway),
            parcel_id: None,
            stage: Stage::Platform,
            progress: 0.0,
            reason: "A rail stop is planned where the arterial grid meets the rail corridor".into(),
            duration: 8.0,
            started_tick: 0,
        });
        self.recompute_connectivity();
        self.state.revision += 1;
    }

    pub(crate) fn plan_node_id(&self, index: usize) -> Option<u32> {
        let id = *self.state.masterplan.as_ref()?.node_ids.get(index)?;
        (id != 0).then_some(id)
    }

    fn realize_plan_node(&mut self, index: usize) -> u32 {
        if let Some(id) = self.plan_node_id(index) {
            return id;
        }
        let node = self.plan_runtime().plan.nodes[index];
        let id = self.add_node(node.p.x, node.p.z, node.kind.world());
        if let Some(progress) = self.state.masterplan.as_mut() {
            progress.node_ids[index] = id;
        }
        id
    }

    fn realize_plan_road(&mut self, index: usize, stage: Stage, progress: f32) -> u32 {
        if let Some(id) = self
            .state
            .masterplan
            .as_ref()
            .map(|p| p.road_ids[index])
            .filter(|id| *id != 0)
        {
            return id;
        }
        let road = self.plan_runtime().plan.roads[index];
        let a = self.realize_plan_node(road.a);
        let b = self.realize_plan_node(road.b);
        let id = self.add_road(a, b, road.kind.world(), stage, progress);
        if let Some(path) = self.plan_runtime().rail_paths.get(&index) {
            if let Some(realized) = self.state.roads.iter_mut().find(|r| r.id == id) {
                realized.length = path.length();
            }
        }
        if let Some(state) = self.state.masterplan.as_mut() {
            state.road_ids[index] = id;
        }
        id
    }

    fn plan_road_index(&self, road_id: u32) -> Option<usize> {
        self.state
            .masterplan
            .as_ref()?
            .road_ids
            .iter()
            .position(|id| *id == road_id)
    }

    /// Admit every plan lot fronting this opened plan road as a parcel.
    fn release_plan_lots(&mut self, road_index: usize) {
        let runtime = self.plan_runtime();
        let Some(road_id) = self
            .state
            .masterplan
            .as_ref()
            .map(|p| p.road_ids[road_index])
            .filter(|id| *id != 0)
        else {
            return;
        };
        let Some(road) = self
            .state
            .roads
            .iter()
            .find(|road| road.id == road_id)
            .cloned()
        else {
            return;
        };
        if road.stage != Stage::Open {
            return;
        }
        let (Some(a), Some(b)) = (self.node(road.from).cloned(), self.node(road.to).cloned())
        else {
            return;
        };
        let mut released = 0;
        for &lot_index in &runtime.lots_by_road[road_index] {
            if self
                .state
                .masterplan
                .as_ref()
                .is_some_and(|p| p.lot_parcels[lot_index] != 0)
            {
                continue;
            }
            let lot = runtime.plan.lots[lot_index];
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let entrance = lot
                .passage
                .map_or(P { x: lot.x, z: lot.z }, |points| points[2]);
            let along = (((entrance.x - a.x) * dx + (entrance.z - a.z) * dz) / (dx * dx + dz * dz))
                .clamp(0.0, 1.0);
            let side_point = if lot.passage_rotation().is_some() {
                entrance
            } else {
                P { x: lot.x, z: lot.z }
            };
            let side = if dx * (side_point.z - a.z) - dz * (side_point.x - a.x) >= 0.0 {
                1
            } else {
                -1
            };
            let access = frontage_access_node(&a, &b, along);
            let suitability = match lot.district {
                District::StationCore => 0.95,
                District::StationFringe => 0.88,
                District::Compact | District::Strips => 0.84,
                District::Waterfront => 0.8,
            };
            let parcel_id = self.add_parcel(
                lot.x,
                lot.z,
                lot.width,
                lot.depth,
                access,
                suitability,
                false,
            );
            if parcel_id == 0 {
                break;
            }
            let open_space = self.open_space_for_released_lot(&lot, &runtime.plan);
            if let Some(parcel) = self.state.parcels.last_mut() {
                parcel.rotation = Some(lot.rotation);
                if let Some(kind) = open_space {
                    parcel.protected = true;
                    parcel.open_space = Some(kind.into());
                }
            }
            match open_space {
                Some("plaza") => self.record(
                    EventKind::PlazaReserved,
                    parcel_id,
                    "A lot beside the planned station is kept as its open plaza".into(),
                ),
                Some(_) => self.record(
                    EventKind::ShrineReserved,
                    parcel_id,
                    "A new neighborhood keeps a corner lot for its shrine".into(),
                ),
                None => {}
            }
            self.state.parcel_frontages.push(ParcelFrontage {
                parcel_id,
                road_id,
                along,
                side,
            });
            // Direct frontage follows the realized segment (split junctions
            // are not bit-exactly collinear). Passage lots keep their own facing.
            let facing = lot
                .passage_rotation()
                .unwrap_or_else(|| self.frontage_rotation(parcel_id));
            if let Some(parcel) = self.state.parcels.iter_mut().find(|p| p.id == parcel_id) {
                parcel.rotation = Some(facing);
            }
            if let Some(progress) = self.state.masterplan.as_mut() {
                progress.lot_parcels[lot_index] = parcel_id;
            }
            released += 1;
        }
        if released > 0 {
            self.mark_access_geometry_changed();
        }
    }

    pub(crate) fn masterplan_lot_for_parcel(&self, parcel_id: u32) -> Option<masterplan::PlanLot> {
        let index = self
            .state
            .masterplan
            .as_ref()?
            .lot_parcels
            .iter()
            .position(|id| *id == parcel_id)?;
        Some(self.plan.as_ref()?.plan.lots[index])
    }

    /// Surface only realized building sites, not the stock of released parcels.
    /// Building records survive vacancy/renewal, so their ground does too.
    pub(crate) fn masterplan_ground(&self) -> Vec<crate::GroundPatch> {
        let (Some(progress), Some(runtime)) = (self.state.masterplan.as_ref(), self.plan.as_ref())
        else {
            return Vec::new();
        };
        let developed: std::collections::BTreeSet<u32> = self
            .state
            .buildings
            .iter()
            .map(|building| building.parcel_id)
            .collect();
        progress
            .lot_parcels
            .iter()
            .enumerate()
            .filter_map(|(index, parcel)| {
                if *parcel == 0 || !developed.contains(parcel) {
                    return None;
                }
                let lot = &runtime.plan.lots[index];
                let outline = &runtime.plan.lot_ground[index];
                Some(crate::GroundPatch {
                    block: lot.block,
                    points: outline.iter().map(|p| [p.x, p.z]).collect(),
                })
            })
            .collect()
    }

    /// Deck elevations for realized expressway roads, derived from the
    /// regenerated plan and its persisted road ID mapping.
    pub(crate) fn masterplan_road_decks(&self) -> Vec<crate::RoadDeckView> {
        let (Some(progress), Some(runtime)) = (self.state.masterplan.as_ref(), self.plan.as_ref())
        else {
            return Vec::new();
        };
        runtime
            .plan
            .roads
            .iter()
            .enumerate()
            .filter_map(|(index, road)| {
                if road.kind != RoadKind::Expressway {
                    return None;
                }
                let road_id = *progress.road_ids.get(index)?;
                let controls = road.curve?;
                (road_id != 0).then_some(crate::RoadDeckView {
                    road_id,
                    deck_height: road.deck_height,
                    controls: controls.map(|p| [p.x, p.z]),
                })
            })
            .collect()
    }

    /// Keep visible construction state on pre-added plan road records.
    pub(crate) fn sync_masterplan_road_projects(&mut self) {
        if !self.masterplan_active() {
            return;
        }
        let updates: Vec<(u32, u32, Stage, f32)> = self
            .state
            .projects
            .iter()
            .filter(|p| {
                matches!(p.kind, ProjectKind::Road | ProjectKind::Expressway) && p.progress < 1.0
            })
            .filter_map(|p| Some((p.from?, p.to?, p.stage, p.progress)))
            .collect();
        for (from, to, stage, progress) in updates {
            if let Some(road) = self.state.roads.iter_mut().find(|road| {
                road.stage != Stage::Open
                    && ((road.from == from && road.to == to)
                        || (road.from == to && road.to == from))
            }) {
                road.stage = stage;
                road.progress = progress;
            }
        }
    }

    /// Handle completion of master-plan road and expressway projects. Returns
    /// true when the project was consumed.
    pub(crate) fn masterplan_complete_project(&mut self, project: &Project) -> bool {
        if !self.masterplan_active()
            || !matches!(project.kind, ProjectKind::Road | ProjectKind::Expressway)
        {
            return false;
        }
        let (Some(from), Some(to)) = (project.from, project.to) else {
            return false;
        };
        let Some(road_id) = self
            .state
            .roads
            .iter()
            .find(|road| {
                road.stage != Stage::Open
                    && ((road.from == from && road.to == to)
                        || (road.from == to && road.to == from))
            })
            .map(|road| road.id)
        else {
            return false;
        };
        let mut kind = None;
        if let Some(road) = self.state.roads.iter_mut().find(|road| road.id == road_id) {
            road.stage = Stage::Open;
            road.progress = 1.0;
            kind = Some(road.kind);
        }
        if project.kind == ProjectKind::Expressway {
            self.record(
                EventKind::ExpresswayOpened,
                road_id,
                "An elevated expressway section opens above the city".into(),
            );
            return true;
        }
        self.recompute_connectivity();
        if self
            .state
            .roads
            .iter()
            .any(|r| r.id == road_id && r.connected)
        {
            let label = match kind {
                Some(crate::RoadKind::Arterial) => "An arterial",
                Some(crate::RoadKind::Lane) => "A narrow lane",
                Some(crate::RoadKind::Bridge) => "A bridge",
                _ => "A local street",
            };
            self.record(
                EventKind::RoadOpened,
                road_id,
                format!("{label} opens and extends the connected street network"),
            );
        }
        if let Some(index) = self.plan_road_index(road_id) {
            self.release_plan_lots(index);
        }
        // Opening a connection can also connect earlier roads; release lots
        // on every open plan road whose frontage was waiting.
        self.state.station_online =
            self.state.station_ready
                && self.state.roads.iter().any(|r| {
                    r.kind == crate::RoadKind::Rail && r.stage == Stage::Open && r.connected
                });
        true
    }

    fn active_projects(&self, kind: ProjectKind) -> usize {
        self.state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0 && p.kind == kind)
            .count()
    }

    /// Bounded growth evaluation for master-plan worlds.
    pub(crate) fn masterplan_growth(&mut self) {
        let active = self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .count();
        if active >= MAX_PROJECTS {
            return;
        }
        let day = (self.state.tick / TICKS_PER_DAY) as u32;
        self.start_transit_infrastructure();

        let open = self
            .state
            .buildings
            .iter()
            .filter(|b| b.stage == Stage::Open)
            .count();
        let vacant = self
            .state
            .buildings
            .iter()
            .filter(|b| site_vacant(b))
            .count();
        let vacancy = vacant as f32 / open.max(1) as f32;
        let available = self.available_masterplan_parcels();

        let mut building_slots = MAX_ACTIVE_BUILDING_PROJECTS
            .saturating_sub(self.active_projects(ProjectKind::Building))
            .min(BUILDINGS_PER_EVALUATION);
        if vacancy > 0.14 && vacant > 2 {
            building_slots = 0;
        }
        for _ in 0..building_slots.min(available) {
            let before = self.state.projects.len();
            self.plan_building(day);
            if self.state.projects.len() == before {
                break;
            }
        }

        if available < RELEASED_PARCEL_TARGET
            && self.active_projects(ProjectKind::Road) < MAX_ACTIVE_ROAD_PROJECTS
            && self
                .state
                .projects
                .iter()
                .filter(|p| p.progress < 1.0)
                .count()
                < MAX_PROJECTS
        {
            self.start_masterplan_road();
        }
    }

    /// Open space kept when a plan lot is first released: up to two
    /// street-facing lots beside the planned station form its plaza, and each
    /// residential neighborhood keeps one ordinary lot for a shrine. Spacing
    /// and caps make the choice deterministic in release order.
    fn open_space_for_released_lot(
        &self,
        lot: &PlanLot,
        plan: &MasterPlan,
    ) -> Option<&'static str> {
        if lot.flag {
            return None;
        }
        let reserved = |kind: &'static str| {
            self.state
                .parcels
                .iter()
                .filter(move |p| p.open_space.as_deref() == Some(kind))
        };
        if let Some(station) = plan
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Station)
        {
            let distance = (lot.x - station.p.x).hypot(lot.z - station.p.z);
            if distance <= STATION_PLAZA_RADIUS {
                return (reserved("plaza").count() < MAX_STATION_PLAZA_LOTS).then_some("plaza");
            }
            if distance < SHRINE_STATION_CLEARANCE {
                return None;
            }
        }
        if !matches!(lot.district, District::Compact | District::Strips)
            || !SHRINE_AREA.contains(&(lot.width * lot.depth))
            || reserved("shrine").count() >= MAX_SHRINES
            || reserved("shrine").any(|p| (p.x - lot.x).hypot(p.z - lot.z) < SHRINE_SPACING)
        {
            return None;
        }
        Some("shrine")
    }

    fn available_masterplan_parcels(&self) -> usize {
        let built: std::collections::BTreeSet<u32> =
            self.state.buildings.iter().map(|b| b.parcel_id).collect();
        self.state
            .parcels
            .iter()
            .filter(|p| {
                p.reservation.is_none()
                    && !p.protected
                    && !built.contains(&p.id)
                    && self.node_connected(p.access_node)
            })
            .count()
    }

    fn start_masterplan_road(&mut self) {
        let runtime = self.plan_runtime();
        let plan = &runtime.plan;
        let Some(progress) = self.state.masterplan.as_ref() else {
            return;
        };
        let connected: BTreeMap<u32, bool> = self
            .state
            .nodes
            .iter()
            .map(|node| (node.id, self.node_connected(node.id)))
            .collect();
        let busy: std::collections::BTreeSet<u32> = self
            .state
            .projects
            .iter()
            .filter(|p| p.progress < 1.0)
            .flat_map(|p| p.from.into_iter().chain(p.to))
            .collect();
        let points: Vec<(f32, f32, bool)> = self
            .state
            .buildings
            .iter()
            .filter(|b| b.stage == Stage::Open)
            .map(|b| (b.x, b.z, site_active(b)))
            .collect();
        let station = plan.nodes[plan.station].p;
        let station_reached = progress.road_ids.iter().enumerate().any(|(index, id)| {
            *id != 0
                && (plan.roads[index].a == plan.station || plan.roads[index].b == plan.station)
                && plan.roads[index].kind.is_street()
                && self
                    .state
                    .roads
                    .iter()
                    .any(|r| r.id == *id && r.stage == Stage::Open)
        });
        let population = self.state.population;
        let mut best: Option<(f32, usize)> = None;
        for (index, road) in plan.roads.iter().enumerate() {
            if progress.road_ids[index] != 0 || !road.kind.is_street() {
                continue;
            }
            let a_id = progress.node_ids[road.a];
            let b_id = progress.node_ids[road.b];
            let a_open = a_id != 0 && connected.get(&a_id).copied().unwrap_or(false);
            let b_open = b_id != 0 && connected.get(&b_id).copied().unwrap_or(false);
            if !(a_open || b_open) || busy.contains(&a_id) || busy.contains(&b_id) {
                continue;
            }
            if road.kind == RoadKind::Bridge && population < 600 {
                continue;
            }
            let (pa, pb) = (plan.nodes[road.a].p, plan.nodes[road.b].p);
            let mid = P {
                x: (pa.x + pb.x) * 0.5,
                z: (pa.z + pb.z) * 0.5,
            };
            let lots = runtime.lots_by_road[index].len() as f32;
            let nearby = points
                .iter()
                .filter(|(x, z, _)| (x - mid.x).hypot(z - mid.z) < 110.0)
                .map(|(_, _, active)| if *active { 1.0 } else { 0.4 })
                .sum::<f32>();
            let station_distance = (mid.x - station.x).hypot(mid.z - station.z);
            let mut score = lots * 1.2 + nearby.min(30.0) * 0.6;
            score += match road.kind {
                RoadKind::Arterial => 6.0,
                RoadKind::Bridge => 14.0,
                RoadKind::Lane => -2.0,
                _ => 0.0,
            };
            if !station_reached {
                score += 30.0 / (1.0 + station_distance / 60.0);
            } else {
                score += 8.0 / (1.0 + station_distance / 120.0);
            }
            // Deterministic variety without consuming the growth stream.
            let jitter = ((index as u32).wrapping_mul(2_654_435_761) ^ self.state.seed) % 1000;
            score += jitter as f32 * 0.002;
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, index));
            }
        }
        let Some((_, index)) = best else {
            return;
        };
        let kind = plan.roads[index].kind;
        let id = self.realize_plan_road(index, Stage::Survey, 0.0);
        let Some(road) = self.state.roads.iter().find(|r| r.id == id).cloned() else {
            return;
        };
        let duration = match kind {
            RoadKind::Arterial => 5.0,
            RoadKind::Bridge => 9.0,
            RoadKind::Lane => 2.5,
            _ => 3.5,
        } + random_unit(&mut self.state.rng_growth) * 2.0;
        let reason = match kind {
            RoadKind::Arterial => {
                "An arterial section is built to carry growing neighborhood traffic"
            }
            RoadKind::Bridge => "A bridge links the two banks as both neighborhoods grow",
            RoadKind::Lane => "A narrow lane opens the block for small frontage lots",
            _ => "A local street opens new frontage beside growing demand",
        };
        let project_id = self.id();
        self.record_project(Project {
            id: project_id,
            kind: ProjectKind::Road,
            from: Some(road.from),
            to: Some(road.to),
            parcel_id: None,
            stage: Stage::Survey,
            progress: 0.0,
            reason: reason.into(),
            duration,
            started_tick: self.state.tick,
        });
        self.state.revision += 1;
    }

    fn start_transit_infrastructure(&mut self) {
        let runtime = self.plan_runtime();
        let plan = &runtime.plan;
        let Some(progress) = self.state.masterplan.clone() else {
            return;
        };
        if !progress.rail_extension_started
            && self.state.station_online
            && self.state.population >= 1_500
        {
            let station = self.realize_plan_node(plan.station);
            let terminus = self.realize_plan_node(plan.rail_terminus);
            self.realize_plan_road(plan.rail_extension, Stage::Foundation, 0.0);
            let id = self.id();
            self.record_project(Project {
                id,
                kind: ProjectKind::Rail,
                from: Some(station),
                to: Some(terminus),
                parcel_id: None,
                stage: Stage::Survey,
                progress: 0.0,
                reason: "The elevated line is extended west across the river".into(),
                duration: 12.0,
                started_tick: self.state.tick,
            });
            if let Some(state) = self.state.masterplan.as_mut() {
                state.rail_extension_started = true;
            }
            return;
        }
        let next = progress.expressway_next as usize;
        if next < plan.expressway.len()
            && self.state.population >= 3_500
            && self.active_projects(ProjectKind::Expressway) == 0
        {
            let index = plan.expressway[next];
            let id = self.realize_plan_road(index, Stage::Survey, 0.0);
            let Some(road) = self.state.roads.iter().find(|r| r.id == id).cloned() else {
                return;
            };
            let project_id = self.id();
            self.record_project(Project {
                id: project_id,
                kind: ProjectKind::Expressway,
                from: Some(road.from),
                to: Some(road.to),
                parcel_id: None,
                stage: Stage::Survey,
                progress: 0.0,
                reason: "An elevated expressway section is built above the river and arterials"
                    .into(),
                duration: 14.0,
                started_tick: self.state.tick,
            });
            if let Some(state) = self.state.masterplan.as_mut() {
                state.expressway_next += 1;
            }
        }
    }

    /// District-conditioned use and height for master-plan lots.
    pub(crate) fn masterplan_adjust_profile(
        &mut self,
        parcel_id: u32,
        kind: BuildingKind,
        capacity: u16,
        floors: u8,
    ) -> (BuildingKind, u16, u8) {
        let Some(lot) = self.masterplan_lot_for_parcel(parcel_id) else {
            return (kind, capacity, floors);
        };
        let roll = random_unit(&mut self.state.rng_growth);
        let station_core = lot.district == District::StationCore && self.state.station_online;
        // A small tower cluster on the largest served station-core lots.
        let towers = self
            .state
            .buildings
            .iter()
            .filter(|b| b.floors >= 14)
            .count();
        if station_core
            && !lot.flag
            && lot.width * lot.depth >= 280.0
            && towers < MAX_STATION_TOWERS
            && roll < 0.2
        {
            let floors = 16 + (roll * 90.0) as u8 % 17;
            let kind = if roll < 0.11 {
                BuildingKind::Office
            } else {
                BuildingKind::Apartment
            };
            return (kind, floors as u16 * 5, floors);
        }
        // Compact Tokyo fabric has almost no single-storey buildings: shops
        // are two- and three-storey shop-houses and homes are mostly 2-3.
        if !matches!(lot.district, District::StationCore) {
            // Interior "mansion" blocks: small 5-6 storey apartments and
            // offices scattered through the low-rise fabric give Tokyo's p90.
            let floors = match (kind, floors) {
                (_, 1) => 2 + u8::from(roll < 0.35),
                (BuildingKind::Home, 2) if roll < 0.55 => 3,
                (BuildingKind::Shop | BuildingKind::Cafe, 2) if roll < 0.35 => 3,
                (BuildingKind::Apartment | BuildingKind::Office, _) if roll < 0.45 => {
                    5 + u8::from(roll < 0.2)
                }
                _ => floors,
            };
            if floors != 0 && !lot.arterial_frontage {
                let capacity = match kind {
                    BuildingKind::Home => 6 + floors as u16 * 2,
                    BuildingKind::Apartment => floors as u16 * 5,
                    _ => capacity,
                };
                if kind != BuildingKind::Home || floors <= 3 {
                    return (kind, capacity, floors);
                }
            }
        }
        if lot.arterial_frontage || station_core {
            // Arterials and served station cores carry a mid-rise street wall.
            if kind == BuildingKind::Home && roll < 0.65 {
                let floors = 4 + (roll * 10.0) as u8 % 5;
                return (BuildingKind::Apartment, 6 + floors as u16 * 3, floors);
            }
            if matches!(kind, BuildingKind::Office | BuildingKind::Apartment) {
                let floors = floors.max(5);
                return (kind, capacity.max(floors as u16 * 3), floors);
            }
        }
        if lot.district == District::Waterfront && kind == BuildingKind::Home && roll < 0.3 {
            return (BuildingKind::Apartment, 18, 5);
        }
        (kind, capacity, floors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::geometry;

    fn grown(seed: u32, ticks: u32) -> Engine {
        let mut engine = Engine::new(seed);
        engine.set_development_rate(16);
        let mut left = ticks;
        while left > 0 {
            let step = left.min(2_000);
            engine.step(step);
            left -= step;
        }
        engine
    }

    fn summary(engine: &Engine) -> String {
        let open = engine
            .state
            .buildings
            .iter()
            .filter(|b| b.stage == Stage::Open)
            .count();
        let roads = engine
            .state
            .roads
            .iter()
            .filter(|r| r.stage == Stage::Open)
            .count();
        let footprint: f32 = engine
            .state
            .buildings
            .iter()
            .filter(|b| b.stage == Stage::Open)
            .map(|b| b.width * b.depth)
            .sum();
        format!(
            "tick {} buildings {} open {} roads {} parcels {} population {} station {} footprint {:.0}",
            engine.state.tick,
            engine.state.buildings.len(),
            open,
            roads,
            engine.state.parcels.len(),
            engine.state.population,
            engine.state.station_online,
            footprint
        )
    }

    #[test]
    fn founding_opens_gateway_streets_and_founder() {
        let engine = Engine::new(87231);
        assert!(engine.masterplan_active());
        assert!(engine
            .state
            .roads
            .iter()
            .any(|r| r.kind == crate::RoadKind::Arterial && r.connected));
        assert!(engine
            .state
            .buildings
            .iter()
            .any(|b| b.anchor && b.stage == Stage::Open));
        assert!(
            engine.state.parcels.len() >= 4,
            "founding row releases lots"
        );
        let ground = engine.masterplan_ground();
        let developed_parcels: std::collections::BTreeSet<u32> = engine
            .state
            .buildings
            .iter()
            .map(|building| building.parcel_id)
            .collect();
        let mapped_developed = engine
            .state
            .masterplan
            .as_ref()
            .unwrap()
            .lot_parcels
            .iter()
            .filter(|parcel| **parcel != 0 && developed_parcels.contains(parcel))
            .count();
        assert_eq!(ground.len(), mapped_developed);
        assert!(
            ground.len() < engine.state.parcels.len(),
            "released vacant parcels stay unpaved"
        );
        engine.validate().unwrap();
    }

    #[test]
    fn ground_tracks_buildings_and_survives_save_restore_and_vacancy() {
        let mut engine = grown(87231, 6_000);
        let before = engine.masterplan_ground();
        assert!(!before.is_empty(), "normal growth realizes buildings");
        let built_parcels: std::collections::BTreeSet<u32> = engine
            .state
            .buildings
            .iter()
            .map(|building| building.parcel_id)
            .collect();
        let expected = engine
            .state
            .masterplan
            .as_ref()
            .unwrap()
            .lot_parcels
            .iter()
            .filter(|parcel| **parcel != 0 && built_parcels.contains(parcel))
            .count();
        assert_eq!(before.len(), expected);

        let restored = Engine::restore_json(&engine.save_json()).unwrap();
        assert_eq!(
            serde_json::to_value(&before).unwrap(),
            serde_json::to_value(restored.masterplan_ground()).unwrap()
        );

        {
            let building = engine
                .state
                .buildings
                .iter_mut()
                .find(|building| building.stage == Stage::Open)
                .expect("growth has an open building");
            building.occupancy = 0;
            building.vacant_days = 500;
        }
        assert_eq!(
            serde_json::to_value(&before).unwrap(),
            serde_json::to_value(engine.masterplan_ground()).unwrap(),
            "vacancy retains developed ground"
        );
        engine
            .state
            .buildings
            .iter_mut()
            .find(|building| building.stage == Stage::Open)
            .unwrap()
            .stage = Stage::Demolished;
        assert_eq!(
            serde_json::to_value(&before).unwrap(),
            serde_json::to_value(engine.masterplan_ground()).unwrap(),
            "retained demolition record preserves ground"
        );
    }

    #[test]
    fn lot_ground_polygons_contain_their_parcel_rectangles_across_seeds() {
        fn contains(poly: &[P], point: P) -> bool {
            let mut inside = false;
            for i in 0..poly.len() {
                let a = poly[i];
                let b = poly[(i + 1) % poly.len()];
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let length2 = dx * dx + dz * dz;
                let t = if length2 > 0.0 {
                    ((point.x - a.x) * dx + (point.z - a.z) * dz) / length2
                } else {
                    0.0
                }
                .clamp(0.0, 1.0);
                // The existing rectangle fitter samples 2 cm inside its front/back
                // slices; allow 5 cm at boundary corners, not interior gaps.
                if (point.x - a.x - t * dx).hypot(point.z - a.z - t * dz) <= 0.05 {
                    return true;
                }
                if (a.z > point.z) != (b.z > point.z)
                    && point.x < (b.x - a.x) * (point.z - a.z) / (b.z - a.z) + a.x
                {
                    inside = !inside;
                }
            }
            inside
        }

        for seed in [1, 7, 87231, 99173] {
            let plan = masterplan::generate(&Geography {
                seed,
                river: [
                    P {
                        x: -112.0,
                        z: -400.0,
                    },
                    P {
                        x: -98.0,
                        z: -190.0,
                    },
                    P { x: -112.0, z: 20.0 },
                    P { x: -94.0, z: 210.0 },
                    P {
                        x: -109.0,
                        z: 400.0,
                    },
                ]
                .to_vec(),
                river_clearance: RIVER_HALF_WIDTH + RIVER_BANK_BUFFER,
                protected: (-350.0, -350.0, -260.0, -270.0),
            });
            assert_eq!(plan.lot_ground.len(), plan.lots.len(), "seed {seed}");
            for (lot, polygon) in plan.lots.iter().zip(&plan.lot_ground) {
                assert!(polygon.len() >= 3, "seed {seed}: degenerate ground cell");
                let ux = P {
                    x: lot.facing.z * lot.width * 0.5,
                    z: -lot.facing.x * lot.width * 0.5,
                };
                let uz = P {
                    x: lot.facing.x * lot.depth * 0.5,
                    z: lot.facing.z * lot.depth * 0.5,
                };
                for sx in [-1.0, 1.0] {
                    for sz in [-1.0, 1.0] {
                        let corner = P {
                            x: lot.x + ux.x * sx + uz.x * sz,
                            z: lot.z + ux.z * sx + uz.z * sz,
                        };
                        assert!(
                            contains(polygon, corner),
                            "seed {seed}: parcel outside ground polygon"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn master_plan_city_grows_connected_without_overlaps_and_replays() {
        let mut engine = grown(87231, 40_000);
        eprintln!("{}", summary(&engine));
        assert!(engine.state.buildings.len() > 60, "{}", summary(&engine));
        engine.validate().unwrap();
        // Every parcel with a building is reachable.
        for building in engine
            .state
            .buildings
            .iter()
            .filter(|b| b.stage == Stage::Open)
        {
            let parcel = engine
                .state
                .parcels
                .iter()
                .find(|p| p.id == building.parcel_id)
                .unwrap();
            assert!(
                engine.node_connected(parcel.access_node),
                "inaccessible building"
            );
        }
        // No parcel overlaps another.
        let rects: Vec<geometry::OrientedRect> = engine
            .state
            .parcels
            .iter()
            .map(|p| geometry::OrientedRect {
                x: p.x,
                z: p.z,
                width: p.width - 0.1,
                depth: p.depth - 0.1,
                rotation: p.rotation.unwrap_or(0.0),
            })
            .collect();
        for (i, a) in rects.iter().enumerate() {
            for b in rects.iter().skip(i + 1) {
                if (a.x - b.x).abs() < 40.0 && (a.z - b.z).abs() < 40.0 {
                    assert!(!geometry::rects_overlap(a, b, 0.0), "parcels overlap");
                }
            }
        }
        // Save/restore continues identically.
        let saved = engine.save_json();
        let mut restored = Engine::restore_json(&saved).unwrap();
        engine.step(1_500);
        restored.step(1_500);
        assert_eq!(engine.save_json(), restored.save_json());
    }

    #[test]
    #[cfg(feature = "flag-passage-candidate")]
    fn generated_rear_passage_survives_renovation_and_replay() {
        for seed in [87231, 0, 5] {
            let mut engine = grown(seed, 40_000);
            let id = engine
                .state
                .buildings
                .iter()
                .find(|b| {
                    b.stage == crate::Stage::Open
                        && crate::is_commercial(b)
                        && !engine.errand_references_building(b.id)
                        && engine
                            .masterplan_lot_for_parcel(b.parcel_id)
                            .is_some_and(|lot| lot.passage.is_some())
                })
                .expect("generated rear commercial site")
                .id;
            // Arrange vacancy on real generated geometry. This exercises renewal,
            // not the separate policy that decides when a business closes.
            for building in &mut engine.state.buildings {
                building.vacant_days = 0;
                if building.id == id {
                    building.business_active = false;
                    building.vacant_days = 6;
                }
            }
            engine.state.demand = 0.0; // Isolate renovation from height redevelopment.
            let before = engine.building_access_segment(id).unwrap();
            assert!(before.1.len() > 2);
            let saved = engine.save_json();
            let mut restored = Engine::restore_json(&saved).unwrap();
            for city in [&mut engine, &mut restored] {
                city.renewal(3);
            }
            assert_eq!(
                engine
                    .state
                    .buildings
                    .iter()
                    .find(|b| b.id == id)
                    .unwrap()
                    .stage,
                Stage::Renovation
            );
            // Restore while the building is actually undergoing work, then use
            // the production fixed-step scheduler through completion.
            restored = Engine::restore_json(&engine.save_json()).unwrap();
            let mut reopened = false;
            for _ in 0..400 {
                engine.step(1);
                restored.step(1);
                assert_eq!(engine.save_json(), restored.save_json());
                let after = engine.building_access_segment(id).unwrap();
                assert_eq!(before.0, after.0);
                assert_eq!(before.1.len(), after.1.len());
                for (a, b) in before.1.iter().zip(&after.1) {
                    assert_eq!(
                        (a.x.to_bits(), a.z.to_bits()),
                        (b.x.to_bits(), b.z.to_bits())
                    );
                }
                if engine
                    .state
                    .buildings
                    .iter()
                    .find(|b| b.id == id)
                    .unwrap()
                    .stage
                    == Stage::Open
                {
                    reopened = true;
                    break;
                }
            }
            assert!(reopened, "seed {seed}: renovation must finish");
            engine.validate().unwrap();
            Engine::restore_json(&engine.save_json()).unwrap();
        }
    }

    #[test]
    fn same_seed_master_plan_growth_is_deterministic() {
        let a = grown(7, 6_000);
        let b = grown(7, 6_000);
        assert_eq!(a.save_json(), b.save_json());
    }

    #[test]
    fn snapshot_reports_realized_expressway_decks_from_plan_mappings() {
        let engine = grown(87231, 160_000);
        engine.validate().unwrap();
        let snapshot = engine.snapshot();
        let restored = Engine::restore_json(&engine.save_json()).unwrap();
        assert_eq!(snapshot.road_decks, restored.snapshot().road_decks);
        assert_eq!(
            snapshot.road_decks.len(),
            engine.plan_runtime().plan.expressway.len()
        );
        for deck in &snapshot.road_decks {
            let road = snapshot
                .roads
                .iter()
                .find(|road| road.id == deck.road_id)
                .unwrap();
            assert_eq!(road.kind, crate::RoadKind::Expressway);
            let plan_index = engine.plan_road_index(deck.road_id).unwrap();
            assert_eq!(
                deck.deck_height,
                engine.plan_runtime().plan.roads[plan_index].deck_height
            );
            assert_eq!(
                deck.controls,
                engine.plan_runtime().plan.roads[plan_index]
                    .curve
                    .unwrap()
                    .map(|p| [p.x, p.z])
            );
        }
        assert!(snapshot
            .road_decks
            .iter()
            .any(|deck| deck.deck_height == 11.0));
        assert!(snapshot
            .road_decks
            .iter()
            .any(|deck| deck.deck_height == 20.0));
    }

    #[test]
    fn constructing_expressway_does_not_rebend_existing_sections() {
        let mut engine = Engine::new(87231);
        engine.set_development_rate(16);
        while engine.snapshot().road_decks.is_empty() && engine.state.tick < 20_000 {
            engine.step(80);
        }
        let first = engine.snapshot().road_decks;
        assert!(!first.is_empty());
        let saved = engine.save_json();
        let mut restored = Engine::restore_json(&saved).unwrap();
        for _ in 0..40 {
            engine.step(80);
            restored.step(80);
            assert_eq!(engine.save_json(), restored.save_json());
        }
        let later = engine.snapshot().road_decks;
        assert!(later.len() > first.len());
        for deck in first {
            assert_eq!(
                &deck,
                later.iter().find(|d| d.road_id == deck.road_id).unwrap()
            );
        }
    }

    /// `MASTERPLAN_TICKS=240000 MASTERPLAN_SNAPSHOT=/path cargo test --release masterplan_growth::tests::long_run -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn long_run() {
        let ticks: u32 = std::env::var("MASTERPLAN_TICKS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(240_000);
        let seed: u32 = std::env::var("MASTERPLAN_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(87231);
        let mut engine = Engine::new(seed);
        engine.set_development_rate(16);
        let started = std::time::Instant::now();
        let mut done = 0;
        while done < ticks {
            engine.step(2_000);
            done += 2_000;
            if done % 20_000 == 0 {
                eprintln!(
                    "{} ({:.1}s)",
                    summary(&engine),
                    started.elapsed().as_secs_f32()
                );
            }
        }
        engine.validate().unwrap();
        if let Ok(path) = std::env::var("MASTERPLAN_SNAPSHOT") {
            std::fs::write(&path, engine.snapshot_json()).unwrap();
            std::fs::write(format!("{path}.save.json"), engine.save_json()).unwrap();
        }
    }
}
