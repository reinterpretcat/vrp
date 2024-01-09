use crate::construction::features::*;
use crate::construction::heuristics::StateKeyRegistry;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::solution::Route;
use crate::models::Problem;
use crate::models::*;
use rosomaxa::prelude::GenericError;
use std::sync::Arc;

struct ExampleTransportCost {}

impl TransportCost for ExampleTransportCost {
    fn duration_approx(&self, _: &Profile, _: Location, _: Location) -> Duration {
        42.
    }

    fn distance_approx(&self, _: &Profile, _: Location, _: Location) -> Distance {
        42.
    }

    fn duration(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Duration {
        42.
    }

    fn distance(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Distance {
        42.
    }
}

/// Creates an example jobs used in documentation tests.
fn create_example_jobs(fleet: &Fleet, transport: &(dyn TransportCost + Sync + Send)) -> Arc<Jobs> {
    Arc::new(Jobs::new(
        fleet,
        vec![Job::Single(Arc::new(Single {
            places: vec![Place {
                location: Some(1),
                duration: 0.0,
                times: vec![TimeSpan::Window(TimeWindow::new(0., 100.))],
            }],
            dimens: create_example_dimension(),
        }))],
        transport,
    ))
}

/// Creates an example fleet used in documentation tests.
fn create_example_fleet() -> Arc<Fleet> {
    let drivers = vec![Arc::new(Driver {
        costs: Costs { fixed: 0., per_distance: 0., per_driving_time: 0., per_waiting_time: 0., per_service_time: 0. },
        dimens: create_example_dimension(),
        details: vec![],
    })];
    let vehicles = vec![Arc::new(Vehicle {
        profile: Profile::default(),
        costs: Costs { fixed: 0., per_distance: 1., per_driving_time: 0., per_waiting_time: 0., per_service_time: 0. },
        dimens: std::mem::take(Dimensions::default().set_id("v1")),
        details: vec![VehicleDetail {
            start: Some(VehiclePlace { location: 0, time: TimeInterval::default() }),
            end: None,
        }],
    })];

    Arc::new(Fleet::new(drivers, vehicles, Box::new(|_| Box::new(|_| 0))))
}

fn create_example_dimension() -> Dimensions {
    Dimensions::default()
}

/// Creates an extras with all necessary data
fn create_example_extras() -> Extras {
    let mut dimen_registry = DimenKeyRegistry::default();
    let mut state_registry = StateKeyRegistry::default();

    ExtrasBuilder::new(&mut dimen_registry, &mut state_registry).build().expect("cannot build example extras")
}

/// Creates and example VRP goal: CVRPTW.
fn create_example_goal_ctx(
    transport: Arc<dyn TransportCost + Sync + Send>,
    activity: Arc<dyn ActivityCost + Sync + Send>,
    extras: &Extras,
) -> Result<GoalContext, GenericError> {
    let schedule_keys = extras.get_schedule_keys().expect("no schedule keys").clone();
    let capacity_keys = extras.get_capacity_keys().expect("no capacity keys").clone();

    let features = vec![
        create_minimize_unassigned_jobs_feature("min_jobs", Arc::new(|_, _| 1.))?,
        create_minimize_tours_feature("min_tours")?,
        create_minimize_distance_feature("min_distance", transport, activity, schedule_keys, 1)?,
        create_capacity_limit_feature::<SingleDimLoad>("capacity", capacity_keys, 2)?,
    ];

    let goal = Goal::no_alternatives(
        vec![vec!["min_jobs".to_string()], vec!["min_tours".to_string()], vec!["min_distance".to_string()]],
        vec![vec!["min_tours".to_string()], vec!["min_distance".to_string()]],
    );

    GoalContext::new(features.as_slice(), goal)
}

/// Creates an example problem used in documentation tests.
pub fn create_example_problem() -> Arc<Problem> {
    let extras = create_example_extras();
    let activity: Arc<dyn ActivityCost + Sync + Send> = Arc::new(SimpleActivityCost::default());
    let transport: Arc<dyn TransportCost + Sync + Send> = Arc::new(ExampleTransportCost {});
    let fleet = create_example_fleet();
    let jobs = create_example_jobs(&fleet, transport.as_ref());
    let goal = create_example_goal_ctx(transport.clone(), activity.clone(), &extras).unwrap();

    Arc::new(Problem {
        fleet,
        jobs,
        locks: vec![],
        goal: Arc::new(goal),
        activity,
        transport,
        extras: Arc::new(extras),
    })
}
