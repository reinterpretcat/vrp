use crate::construction::enablers::{JobTie, VehicleTie};
use crate::helpers::TestTransportCost;
use std::sync::Arc;
use vrp_core::construction::enablers::{create_typed_actor_groups, ScheduleKeys};
use vrp_core::construction::features::create_minimize_transport_costs_feature;
use vrp_core::construction::heuristics::{RegistryContext, SolutionContext, StateKeyRegistry};
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::solution::*;
use vrp_core::models::GoalContextBuilder;
use vrp_core::utils::DefaultRandom;

const DEFAULT_VEHICLE_COSTS: Costs =
    Costs { fixed: 100.0, per_distance: 1.0, per_driving_time: 1.0, per_waiting_time: 1.0, per_service_time: 1.0 };
pub const DEFAULT_JOB_LOCATION: Location = 0;
pub const DEFAULT_JOB_DURATION: Duration = 0.0;
pub const DEFAULT_JOB_TIME_SPAN: TimeSpan = TimeSpan::Window(TimeWindow { start: 0., end: 1000. });
pub const DEFAULT_ACTIVITY_TIME_WINDOW: TimeWindow = TimeWindow { start: 0., end: 1000. };
pub const DEFAULT_ACTIVITY_SCHEDULE: Schedule = Schedule { departure: 0.0, arrival: 0.0 };

pub fn test_driver() -> Driver {
    Driver { costs: DEFAULT_VEHICLE_COSTS, dimens: Default::default(), details: vec![] }
}

pub fn test_vehicle(id: &str) -> Vehicle {
    test_vehicle_impl(id, false)
}

fn test_vehicle_impl(id: &str, has_open_end: bool) -> Vehicle {
    let mut dimens = Dimensions::default();
    dimens.set_vehicle_id(id.to_string()).set_vehicle_type(id.to_owned()).set_shift_index(0);

    Vehicle {
        profile: Profile::default(),
        costs: DEFAULT_VEHICLE_COSTS,
        dimens,
        details: vec![VehicleDetail {
            start: Some(VehiclePlace { location: 0, time: Default::default() }),
            end: if has_open_end { None } else { Some(VehiclePlace { location: 0, time: Default::default() }) },
        }],
    }
}

pub fn test_fleet() -> Fleet {
    Fleet::new(vec![Arc::new(test_driver())], vec![Arc::new(test_vehicle("v1"))], |actors| {
        create_typed_actor_groups(actors, |a| {
            a.vehicle.dimens.get_vehicle_type().cloned().expect("no vehicle type set")
        })
    })
}

pub fn create_route_with_activities(fleet: &Fleet, vehicle: &str, activities: Vec<Activity>) -> Route {
    let actor = fleet.actors.iter().find(|a| a.vehicle.dimens.get_vehicle_id().unwrap() == vehicle).unwrap().clone();
    let mut tour = Tour::new(&actor);

    activities.into_iter().enumerate().for_each(|(index, a)| {
        tour.insert_at(a, index + 1);
    });

    Route { actor, tour }
}

pub fn create_activity_at_location(location: Location) -> Activity {
    Activity {
        place: vrp_core::models::solution::Place {
            idx: 0,
            location,
            duration: DEFAULT_JOB_DURATION,
            time: DEFAULT_ACTIVITY_TIME_WINDOW,
        },
        schedule: DEFAULT_ACTIVITY_SCHEDULE,
        job: None,
        commute: None,
    }
}

pub fn create_activity_with_job_at_location(job: Arc<Single>, location: Location) -> Activity {
    Activity { job: Some(job), ..create_activity_at_location(location) }
}

pub fn create_single(id: &str) -> Arc<Single> {
    let mut single = create_single_with_location(Some(DEFAULT_JOB_LOCATION));
    single.dimens.set_job_id(id.to_string()).set_job_type("delivery".to_string());

    Arc::new(single)
}

pub fn create_single_with_location(location: Option<Location>) -> Single {
    Single {
        places: vec![vrp_core::models::problem::Place {
            location,
            duration: DEFAULT_JOB_DURATION,
            times: vec![DEFAULT_JOB_TIME_SPAN],
        }],
        dimens: Default::default(),
    }
}

pub fn single_demand_as_multi(pickup: (i32, i32), delivery: (i32, i32)) -> Demand<MultiDimLoad> {
    let make = |value| {
        if value == 0 {
            MultiDimLoad::default()
        } else {
            MultiDimLoad::new(vec![value])
        }
    };

    Demand { pickup: (make(pickup.0), make(pickup.1)), delivery: (make(delivery.0), make(delivery.1)) }
}

pub fn create_solution_context_for_fleet(fleet: &Fleet) -> SolutionContext {
    let feature = create_minimize_transport_costs_feature(
        "min-costs",
        TestTransportCost::new_shared(),
        Arc::new(SimpleActivityCost::default()),
        ScheduleKeys::from(&mut StateKeyRegistry::default()),
        1,
    )
    .expect("cannot create transport cost feature");
    let goal = GoalContextBuilder::with_features(vec![feature])
        .expect("cannot create builder")
        .set_goal(&["min-costs"], &["min-costs"])
        .expect("cannot set goal")
        .build()
        .expect("cannot build goal context");
    let registry = Registry::new(fleet, Arc::new(DefaultRandom::default()));

    SolutionContext {
        required: vec![],
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        state: Default::default(),
        routes: Default::default(),
        registry: RegistryContext::new(&goal, registry),
    }
}
