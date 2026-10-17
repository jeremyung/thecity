use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildingAccessView {
    pub building_id: u32,
    pub road_id: u32,
    pub points: Vec<Point>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RearExtensionView {
    pub project_id: u32,
    pub building_id: u32,
    /// Center of the added rear strip, excluding the original building.
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub rotation: f32,
    pub floors: u8,
    pub stage: Stage,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadDeckView {
    pub road_id: u32,
    pub deck_height: f32,
    pub controls: [[f32; 2]; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub engine_version: String,
    pub seed: u32,
    pub tick: u64,
    pub day: u32,
    pub minute_of_day: f32,
    pub development_rate: u32,
    pub revision: u64,
    /// Exact presentation generation; only meaningful within the worker epoch.
    pub presentation_revision: String,
    pub stage: String,
    pub weather: Weather,
    pub bounds: Bounds,
    pub terrain: Terrain,
    pub nodes: Vec<Node>,
    pub roads: Vec<Road>,
    #[serde(default)]
    pub road_decks: Vec<RoadDeckView>,
    pub expressway_profile: infrastructure::expressway_profile::ExpresswayProfile,
    #[serde(default)]
    pub rail_platforms: Vec<RailPlatformView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rail_alignments: Vec<RailAlignmentView>,
    pub parcels: Vec<Parcel>,
    pub buildings: Vec<Building>,
    #[serde(default)]
    pub building_access: Vec<BuildingAccessView>,
    #[serde(default)]
    pub rear_extensions: Vec<RearExtensionView>,
    #[serde(default)]
    pub building_original_footprints: Vec<BuildingOriginalFootprint>,
    pub projects: Vec<Project>,
    /// Paved ground of developed master-plan blocks (outline in x/z).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ground: Vec<GroundPatch>,
    pub trains: Vec<TrainView>,
    #[serde(default)]
    pub bus_lines: Vec<BusLineView>,
    #[serde(default)]
    pub buses: Vec<BusView>,
    pub residents: Vec<ResidentView>,
    pub events: Vec<Event>,
    pub stats: Stats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroundPatch {
    pub block: u32,
    pub points: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusLineView {
    pub id: u32,
    pub stops: Vec<u32>,
    pub route: Vec<u32>,
    pub active: bool,
    #[serde(default)]
    pub stop_positions: Vec<BusStopView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusStopView {
    pub node_id: u32,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub direction: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusView {
    pub id: u32,
    pub line_id: u32,
    pub progress: f32,
    pub direction: i8,
    pub dwelling: bool,
    pub passengers: u16,
    pub capacity: u16,
    pub next_stop: Option<u32>,
    pub waiting: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainView {
    pub id: u32,
    pub route: Vec<u32>,
    pub progress: f32,
    pub direction: i8,
    pub dwelling: bool,
    pub door_open_fraction: f32,
    pub door_terminal: Option<u32>,
    pub passengers: u16,
    pub capacity: u16,
    pub station_waiting: u16,
    pub gateway_waiting: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResidentView {
    pub id: u32,
    pub name: String,
    pub home_building: u32,
    pub work_building: Option<u32>,
    pub activity: ResidentActivity,
    pub transfer: Transfer,
    pub train_id: Option<u32>,
    pub location_node: Option<u32>,
    #[serde(default)]
    pub bus_id: Option<u32>,
    #[serde(default)]
    pub bus_origin_stop: Option<u32>,
    #[serde(default)]
    pub bus_destination_stop: Option<u32>,
    #[serde(default)]
    pub bus_stop_node: Option<u32>,
}

impl Engine {
    pub fn inspect_json(&self, id: u32) -> String {
        if let Some(value) = self.inspect_expressway_trip(id) {
            return value.to_string();
        }
        let value = if let Some(b) = self.state.buildings.iter().find(|b| b.id == id) {
            serde_json::json!({"entityType":"building","id":b.id,"name":b.name,"kind":b.kind,"stage":b.stage,"occupancy":b.occupancy,"capacity":b.capacity,"history":b.history})
        } else if let Some(r) = self.state.roads.iter().find(|r| r.id == id) {
            serde_json::json!({"entityType":"road","id":r.id,"kind":r.kind,"stage":r.stage,"connected":r.connected,"length":r.length})
        } else if let Some(t) = self.state.travelers.iter().find(|t| t.id == id) {
            let history: Vec<String> = if t.resident != 0 {
                self.state
                    .residents
                    .iter()
                    .find(|r| r.id == t.resident)
                    .map(|r| r.history.clone())
                    .unwrap_or_default()
            } else {
                self.state
                    .events
                    .iter()
                    .filter(|e| e.entity_id == id)
                    .map(|e| e.reason.clone())
                    .collect()
            };
            serde_json::json!({"entityType":"traveler","id":t.id,"name":t.name,"activity":t.activity,"kind":if t.kind == 1 {"car"} else {"pedestrian"},"vehicle":if t.kind == 1 {Some(vehicle_name(Self::vehicle_body(t, self.station_node())))} else {None},"origin":t.origin_building,"destination":t.destination_building,"history":history,"wait_ticks":t.wait_ticks})
        } else if let Some(r) = self.state.residents.iter().find(|r| r.id == id) {
            serde_json::json!({"entityType":"resident","id":r.id,"name":r.name,"activity":r.activity,"kind":if r.train_id.is_some() {"rail passenger"} else if r.bus_id.is_some() {"bus passenger"} else if r.mode == 1 {"car"} else {"pedestrian"},"origin":r.home_building,"destination":r.work_building,"home_building":r.home_building,"work_building":r.work_building,"transfer":r.transfer,"train_id":r.train_id,"bus_id":r.bus_id,"bus_origin_stop":r.bus_origin_stop,"bus_destination_stop":r.bus_destination_stop,"bus_stop_node":r.bus_stop_node,"pending_alight":r.pending_alight,"location_node":self.resident_location_node(r),"history":r.history})
        } else if let Some(t) = self.state.trains.iter().find(|t| t.id == id) {
            let terminal = if t.direction >= 0 {
                *t.route.last().unwrap_or(&0)
            } else {
                *t.route.first().unwrap_or(&0)
            };
            let history: Vec<String> = self
                .state
                .events
                .iter()
                .filter(|e| e.entity_id == id)
                .map(|e| e.reason.clone())
                .collect();
            serde_json::json!({"entityType":"train","id":t.id,"name":"Neighborhood Shuttle","activity":if t.dwell_ticks > 0 {"at terminal"} else {"in transit"},"kind":"rail","origin":t.route.first(),"destination":terminal,"history":history,"direction":t.direction,"passengers":t.passengers.len(),"capacity":RAIL_TRAIN_CAPACITY,"station_waiting":self.rail_waiting_count(true),"gateway_waiting":self.rail_waiting_count(false)})
        } else if let Some(bus) = self.state.buses.iter().find(|bus| bus.id == id) {
            let line = self
                .state
                .bus_line
                .as_ref()
                .filter(|line| line.id == bus.line_id);
            let next_stop = line.and_then(|line| self.bus_next_stop(bus, line));
            let history: Vec<String> = self
                .state
                .events
                .iter()
                .filter(|event| event.entity_id == id || bus.passengers.contains(&event.entity_id))
                .map(|event| event.reason.clone())
                .collect();
            serde_json::json!({"entityType":"bus","id":bus.id,"name":"Neighborhood Bus","activity":if bus.dwell_ticks > 0 {"at stop"} else {"in service"},"kind":"bus","line_id":bus.line_id,"next_stop":next_stop,"direction":bus.direction,"passengers":bus.passengers.len(),"capacity":BUS_CAPACITY,"waiting":self.bus_waiting_at(next_stop),"history":history})
        } else if let Some(e) = self.state.events.iter().find(|e| e.id == id) {
            serde_json::json!({"entityType":"event","id":e.id,"kind":e.kind,"reason":e.reason,"tick":e.tick})
        } else {
            serde_json::json!({"entityType":"unknown","id":id})
        };
        value.to_string()
    }

    pub fn snapshot_json(&self) -> String {
        serde_json::to_string(&self.snapshot()).expect("serializing snapshot cannot fail")
    }

    pub(crate) fn building_access_views(&self) -> Vec<BuildingAccessView> {
        self.state
            .buildings
            .iter()
            .filter(|building| building.stage != Stage::Demolished)
            .filter_map(|building| {
                let (road_id, access) = self.building_access_segment(building.id)?;
                let road = self.state.roads.iter().find(|road| road.id == road_id)?;
                if road.stage != Stage::Open || !road.connected || road.kind == RoadKind::Rail {
                    return None;
                }
                Some(BuildingAccessView {
                    building_id: building.id,
                    road_id,
                    points: access.iter().map(|p| Point { x: p.x, z: p.z }).collect(),
                })
            })
            .collect()
    }

    pub fn snapshot(&self) -> Snapshot {
        let (day, minute) = clock_at(self.state.tick);
        let road_count = self
            .state
            .roads
            .iter()
            .filter(|r| r.stage == Stage::Open)
            .count();
        let active_sites = self
            .state
            .buildings
            .iter()
            .filter(|b| site_active(b))
            .count();
        let developable_lots = self
            .state
            .parcels
            .iter()
            .filter(|p| {
                !p.protected
                    && self.parcel_clear_of_river(p.x, p.z, p.width, p.depth)
                    && self.node_connected(p.access_node)
            })
            .count();
        let infill_ratio = if developable_lots == 0 {
            0.0
        } else {
            active_sites as f32 / developable_lots as f32
        };
        let stage = if active_sites < 3 || self.state.population < 20 {
            "Founding"
        } else if road_count < 7 || self.state.population < 200 {
            "Settlement"
        } else if road_count >= 100 && active_sites >= 300 && infill_ratio >= 0.9
            || self.state.population >= 24_000
        {
            "Maturity"
        } else if !self.state.station_online || self.state.population < 1500 {
            "Neighborhoods"
        } else if self.state.population < 7000 {
            "TransitCity"
        } else {
            "Metropolis"
        };
        let river = self.river_points();
        let rail_length = self.rail_length_for_doors();
        Snapshot {
            schema_version: SCHEMA_VERSION,
            engine_version: ENGINE_VERSION.into(),
            seed: self.state.seed,
            tick: self.state.tick,
            day,
            minute_of_day: minute,
            development_rate: self.state.development_rate,
            revision: self.state.revision,
            presentation_revision: self.presentation_revision(),
            stage: stage.into(),
            weather: Weather {
                kind: self.state.weather_kind.clone(),
                intensity: self.state.weather_intensity,
            },
            bounds: Bounds {
                min_x: MIN,
                max_x: MAX,
                min_z: MIN,
                max_z: MAX,
            },
            terrain: Terrain {
                river,
                river_half_width: RIVER_HALF_WIDTH,
                protected: vec![vec![
                    Point {
                        x: -350.0,
                        z: -350.0,
                    },
                    Point {
                        x: -260.0,
                        z: -350.0,
                    },
                    Point {
                        x: -260.0,
                        z: -270.0,
                    },
                    Point {
                        x: -350.0,
                        z: -270.0,
                    },
                ]],
                gateways: self
                    .state
                    .nodes
                    .iter()
                    .filter(|n| n.kind == NodeKind::Gateway)
                    .map(|n| n.id)
                    .collect(),
            },
            nodes: self.state.nodes.clone(),
            roads: self.state.roads.clone(),
            road_decks: self.masterplan_road_decks(),
            expressway_profile: infrastructure::expressway_profile::active(),
            rail_platforms: self.rail_platform_views(),
            rail_alignments: self.rail_alignment_views(),
            parcels: self.state.parcels.clone(),
            buildings: self.state.buildings.clone(),
            building_access: self.building_access_views(),
            rear_extensions: self.rear_extension_views(),
            ground: self.masterplan_ground(),
            building_original_footprints: self.state.building_original_footprints.clone(),
            projects: self.state.projects.clone(),
            trains: self
                .state
                .trains
                .iter()
                .map(|t| TrainView {
                    id: t.id,
                    route: t.route.clone(),
                    progress: t.distance,
                    direction: t.direction,
                    dwelling: t.dwell_ticks > 0,
                    door_open_fraction: train_door_state(t, rail_length).1,
                    door_terminal: train_door_state(t, rail_length).0,
                    passengers: t.passengers.len() as u16,
                    capacity: RAIL_TRAIN_CAPACITY as u16,
                    station_waiting: self.rail_waiting_count(true) as u16,
                    gateway_waiting: self.rail_waiting_count(false) as u16,
                })
                .collect(),
            bus_lines: self
                .state
                .bus_line
                .iter()
                .map(|line| BusLineView {
                    id: line.id,
                    stops: line.stops.clone(),
                    route: line.route.clone(),
                    active: line.active,
                    stop_positions: self.bus_stop_views(line),
                })
                .collect(),
            buses: self
                .state
                .buses
                .iter()
                .map(|bus| {
                    let line = self
                        .state
                        .bus_line
                        .as_ref()
                        .filter(|line| line.id == bus.line_id);
                    let next_stop = line.and_then(|line| self.bus_next_stop(bus, line));
                    BusView {
                        id: bus.id,
                        line_id: bus.line_id,
                        progress: bus.progress,
                        direction: bus.direction,
                        dwelling: bus.dwell_ticks > 0,
                        passengers: bus.passengers.len() as u16,
                        capacity: BUS_CAPACITY as u16,
                        next_stop,
                        waiting: self.bus_waiting_at(next_stop) as u16,
                    }
                })
                .collect(),
            residents: self
                .state
                .residents
                .iter()
                .map(|r| ResidentView {
                    id: r.id,
                    name: r.name.clone(),
                    home_building: r.home_building,
                    work_building: r.work_building,
                    activity: r.activity.clone(),
                    transfer: r.transfer.clone(),
                    train_id: r.train_id,
                    location_node: self.resident_location_node(r),
                    bus_id: r.bus_id,
                    bus_origin_stop: r.bus_origin_stop,
                    bus_destination_stop: r.bus_destination_stop,
                    bus_stop_node: r.bus_stop_node,
                })
                .collect(),
            events: self.state.events.iter().cloned().collect(),
            stats: Stats {
                population: self.state.population,
                active_travelers: (self.state.travelers.len()
                    + self.state.expressway_traffic.trips.len()
                    + self.state.pending_alight_ids.len()
                    + self.rail_waiting_count(true)
                    + self.rail_waiting_count(false)
                    + self.bus_platform_waiting_count()) as u32,
                buildings: self.state.buildings.len() as u32,
                active_projects: self
                    .state
                    .projects
                    .iter()
                    .filter(|p| p.progress < 1.0)
                    .count() as u32,
            },
        }
    }
}
