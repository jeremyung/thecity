use super::*;
use crate::height_redevelopment::MIN_BUILDING_AGE_DAYS;
use crate::height_redevelopment::{
    MAX_REDEVELOPED_TOWERS, MIN_DEVELOPMENT_DEMAND, MIN_OFFICE_FLOORS,
};

fn reusable_office_engine() -> (Engine, u32) {
    let mut engine = Engine::new(912);
    let lot_index = engine
        .state
        .masterplan
        .as_ref()
        .unwrap()
        .lot_parcels
        .iter()
        .enumerate()
        .find_map(|(index, parcel_id)| {
            let lot = &engine.plan.as_ref().unwrap().plan.lots[index];
            (*parcel_id != 0
                && !lot.flag
                && lot.district == masterplan::District::StationCore
                && lot.width * lot.depth >= 280.0)
                .then_some(index)
        })
        .unwrap();
    let parcel_id = engine.state.masterplan.as_ref().unwrap().lot_parcels[lot_index];
    engine.state.station_online = true;
    let building_id = engine
        .state
        .buildings
        .iter()
        .find(|building| building.parcel_id == parcel_id)
        .map(|building| building.id)
        .unwrap_or_else(|| {
            engine.add_building(
                parcel_id,
                "office",
                3,
                "open",
                1.0,
                15,
                "Test Office",
                false,
                vec![],
            )
        });
    let building = engine
        .state
        .buildings
        .iter_mut()
        .find(|building| building.id == building_id)
        .unwrap();
    building.kind = "office".into();
    building.floors = 3;
    building.capacity = 15;
    building.occupancy = 0;
    building.stage = "open".into();
    building.business_active = true;
    building.built_day = 0;
    building.anchor = false;
    (engine, building_id)
}

fn station_core_candidate(engine: &Engine, building_id: u32) -> (Building, usize) {
    let building = engine
        .state
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .unwrap()
        .clone();
    let lot_index = engine
        .state
        .masterplan
        .as_ref()
        .unwrap()
        .lot_parcels
        .iter()
        .position(|parcel_id| *parcel_id == building.parcel_id)
        .unwrap();
    (building, lot_index)
}

#[test]
fn height_redevelopment_keeps_the_host_id_and_round_trips_mid_demolition() {
    let (mut engine, building_id) = reusable_office_engine();
    engine.set_development_rate(16);
    assert!(engine.begin_height_redevelopment(building_id));
    let pending = engine.state.height_redevelopment.clone().unwrap();
    assert_eq!(pending.building_id, building_id);
    assert_eq!(
        engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap()
            .stage,
        "demolition"
    );
    engine.validate_height_redevelopment().unwrap();

    engine.step(400);
    assert!(
        engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap()
            .progress
            > 0.0
    );
    engine.validate_height_redevelopment().unwrap();
    let saved = engine.save_json();
    let mut restored =
        Engine::restore_json(&saved).expect("an in-progress demolition must restore");
    assert_eq!(restored.save_json(), saved);
    assert_eq!(
        restored
            .state
            .height_redevelopment
            .as_ref()
            .unwrap()
            .building_id,
        building_id
    );
    engine.step(400);
    restored.step(400);
    let reconstruction = engine
        .state
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .unwrap();
    assert!(matches!(
        reconstruction.stage.as_str(),
        "survey" | "clearing" | "structure" | "surface" | "finishing"
    ));
    let reconstruction_save = engine.save_json();
    let mut restored_reconstruction = Engine::restore_json(&reconstruction_save)
        .expect("an in-progress reconstruction must restore");
    assert_eq!(restored_reconstruction.save_json(), reconstruction_save);

    engine.step(1_300);
    restored.step(1_300);
    restored_reconstruction.step(1_300);
    assert_eq!(
        engine.save_json(),
        restored.save_json(),
        "construction replay must remain deterministic through completion"
    );
    assert_eq!(engine.save_json(), restored_reconstruction.save_json());
    assert!(engine.state.height_redevelopment.is_none());
    assert_eq!(
        engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap()
            .stage,
        "open"
    );
}

#[test]
fn height_redevelopment_completes_in_place_and_releases_its_reservation() {
    let (mut engine, building_id) = reusable_office_engine();
    assert!(engine.begin_height_redevelopment(building_id));
    let plan = engine.state.height_redevelopment.clone().unwrap();
    let project = engine
        .state
        .projects
        .iter_mut()
        .find(|project| project.id == plan.project_id)
        .unwrap();
    project.progress = 1.0;
    engine.advance_height_redevelopment();
    assert_eq!(
        engine
            .state
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap()
            .floors,
        plan.target_floors
    );
    engine.complete_project(plan.project_id);
    let building = engine
        .state
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .unwrap();
    assert_eq!(building.stage, "open");
    assert_eq!(building.floors, plan.target_floors);
    assert!(building.business_active);
    assert_eq!(
        engine
            .state
            .parcels
            .iter()
            .find(|parcel| parcel.id == plan.parcel_id)
            .unwrap()
            .reservation,
        None
    );
    assert!(engine.state.height_redevelopment.is_none());
    assert_eq!(
        engine
            .state
            .buildings
            .iter()
            .filter(|building| building.parcel_id == plan.parcel_id)
            .count(),
        1
    );
}

#[test]
fn actor_references_and_character_fraction_disqualify_office_candidate() {
    let (mut engine, building_id) = reusable_office_engine();
    engine.state.station_online = true;
    let (mut building, lot_index) = station_core_candidate(&engine, building_id);
    let other_office = engine
        .state
        .buildings
        .iter_mut()
        .find(|building| building.id != building_id)
        .unwrap();
    other_office.kind = "office".into();
    other_office.stage = "open".into();
    other_office.business_active = true;
    other_office.capacity = 20;
    building.appearance_seed = 4;
    assert!(!engine.height_redevelopment_candidate(&building, lot_index, MIN_BUILDING_AGE_DAYS));
    building.appearance_seed = 5;
    assert!(
        !site_vacant(&building),
        "zero commercial occupancy is not vacancy while the business is active"
    );
    assert!(engine.height_redevelopment_candidate(&building, lot_index, MIN_BUILDING_AGE_DAYS));
    engine.state.residents.push(Resident {
        id: 900_001,
        name: "Test resident".into(),
        home_building: building_id,
        work_building: None,
        home_node: engine.state.parcels[0].access_node,
        work_node: None,
        mode: 0,
        routine: "worker".into(),
        activity: "home".into(),
        at_work: false,
        departure_tick: 0,
        return_tick: 0,
        last_departure_day: 0,
        last_return_day: 0,
        history: vec![],
        rail_commuter: false,
        transfer: "home".into(),
        queue_slot: None,
        train_id: None,
        bus_id: None,
        bus_origin_stop: None,
        bus_destination_stop: None,
        bus_stop_node: None,
        pending_alight: None,
    });
    assert!(!engine.height_redevelopment_candidate(&building, lot_index, MIN_BUILDING_AGE_DAYS));
}

#[test]
fn station_core_lot_class_and_eight_tower_cap_gate_candidates() {
    let (mut engine, building_id) = reusable_office_engine();
    let (building, core_lot_index) = station_core_candidate(&engine, building_id);
    let other_office = engine
        .state
        .buildings
        .iter_mut()
        .find(|building| building.id != building_id)
        .unwrap();
    other_office.kind = "office".into();
    other_office.stage = "open".into();
    other_office.business_active = true;
    other_office.capacity = 20;
    assert!(engine.height_redevelopment_candidate(
        &building,
        core_lot_index,
        MIN_BUILDING_AGE_DAYS
    ));

    let non_core = engine
        .plan
        .as_ref()
        .unwrap()
        .plan
        .lots
        .iter()
        .position(|lot| !lot.flag && lot.district != masterplan::District::StationCore)
        .unwrap();
    assert!(!engine.height_redevelopment_candidate(&building, non_core, MIN_BUILDING_AGE_DAYS));

    for id in 0..MAX_REDEVELOPED_TOWERS {
        let mut tower = building.clone();
        tower.id = 900_100 + id as u32;
        tower.floors = MIN_OFFICE_FLOORS;
        tower.capacity = tower.floors as u16 * 5;
        engine.state.buildings.push(tower);
    }
    assert!(!engine.height_redevelopment_candidate(
        &building,
        core_lot_index,
        MIN_BUILDING_AGE_DAYS
    ));
}

#[test]
fn ordinary_growth_does_not_start_redevelopment_before_the_mature_gate() {
    let mut engine = Engine::new(913);
    engine.state.station_online = true;
    engine.state.demand = MIN_DEVELOPMENT_DEMAND - 0.01;
    let before = engine.state.projects.len();
    assert!(!engine.try_start_height_redevelopment());
    assert_eq!(engine.state.projects.len(), before);
    assert!(engine.state.height_redevelopment.is_none());

    engine.state.demand = MIN_DEVELOPMENT_DEMAND;
    assert!(
        !engine.try_start_height_redevelopment(),
        "the 300-active-building gate still applies at the demand threshold"
    );
}

#[test]
fn redevelopment_waits_for_eighty_five_percent_lot_realization() {
    let (mut engine, building_id) = reusable_office_engine();
    let template = engine
        .state
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .unwrap()
        .clone();
    for index in 0..300 {
        let mut active = template.clone();
        active.id = 910_000 + index;
        active.stage = "open".into();
        active.business_active = true;
        engine.state.buildings.push(active);
    }
    engine.state.station_online = true;
    engine.state.demand = MIN_DEVELOPMENT_DEMAND;
    let realization = &mut engine.state.masterplan.as_mut().unwrap().lot_parcels;
    let realized_count = ((realization.len() as f32) * 0.84).floor() as usize;
    realization.fill(0);
    realization[..realized_count].fill(template.parcel_id);
    assert!(!engine.try_start_height_redevelopment());
    assert!(engine.state.height_redevelopment.is_none());
    assert!(engine
        .state
        .projects
        .iter()
        .all(|project| project.kind != "redevelopment"));
}

#[test]
fn restore_rejects_a_corrupted_pending_redevelopment_project() {
    let (mut engine, building_id) = reusable_office_engine();
    assert!(engine.begin_height_redevelopment(building_id));
    let mut invalid: serde_json::Value = serde_json::from_str(&engine.save_json()).unwrap();
    let project_id = engine
        .state
        .height_redevelopment
        .as_ref()
        .unwrap()
        .project_id;
    let projects = invalid["projects"].as_array_mut().unwrap();
    let project = projects
        .iter_mut()
        .find(|project| project["id"].as_u64() == Some(project_id as u64))
        .unwrap();
    project["duration"] = serde_json::json!(0.0);
    assert!(Engine::restore_json(&invalid.to_string()).is_err());
}
