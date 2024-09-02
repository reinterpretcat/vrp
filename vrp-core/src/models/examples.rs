use crate::construction::features::*;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::solution::Route;
use crate::models::Problem;
use crate::models::*;
use rosomaxa::prelude::GenericResult;
use std::sync::Arc;

struct ExampleTransportCost {}

impl TransportCost for ExampleTransportCost {
    fn duration_approx(&self, _: &Profile, _: Location, _: Location) -> Duration {
        42
    }

    fn distance_approx(&self, _: &Profile, _: Location, _: Location) -> Distance {
        42
    }

    fn duration(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Duration {
        42
    }

    fn distance(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Distance {
        42
    }
}

/// Creates an example jobs used in documentation tests.
fn create_example_jobs() -> GenericResult<Vec<Job>> {
    Ok(vec![SingleBuilder::default()
        .id("job1")
        .location(1)?
        .times(vec![TimeWindow::new(0, 100)])?
        .demand(Demand::delivery(1))
        .build_as_job()?])
}

/// Creates an example vehicles used in documentation tests.
fn create_example_vehicles() -> GenericResult<Vec<Vehicle>> {
    Ok(vec![VehicleBuilder::default()
        .id("v1")
        .set_distance_cost(1.)
        .capacity(SingleDimLoad::new(2))
        .add_detail(
            VehicleDetailBuilder::default()
                .set_start_location(0)
                .set_start_time(0)
                .set_end_location(0)
                .set_end_time(1000)
                .build()?,
        )
        .build()?])
}

/// Creates and example VRP goal: CVRPTW.
fn create_example_goal_ctx(
    transport: Arc<dyn TransportCost + Sync + Send>,
    activity: Arc<dyn ActivityCost + Sync + Send>,
) -> GenericResult<GoalContext> {
    let features = vec![
        MinimizeUnassignedBuilder::new("min_jobs").build()?,
        create_minimize_tours_feature("min_tours")?,
        TransportFeatureBuilder::new("min_distance")
            .set_transport_cost(transport)
            .set_activity_cost(activity)
            .build_minimize_distance()?,
        CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build()?,
    ];

    GoalContextBuilder::with_features(&features)?.build()
}

pub fn build_example_problem() -> GenericResult<Arc<Problem>> {
    let activity: Arc<dyn ActivityCost + Sync + Send> = Arc::new(SimpleActivityCost::default());
    let transport: Arc<dyn TransportCost + Sync + Send> = Arc::new(ExampleTransportCost {});
    let vehicles = create_example_vehicles()?;
    let jobs = create_example_jobs()?;
    let goal = create_example_goal_ctx(transport.clone(), activity.clone())?;

    ProblemBuilder::default()
        .add_jobs(jobs.into_iter())
        .add_vehicles(vehicles.into_iter())
        .with_transport_cost(transport)
        .with_activity_cost(activity)
        .with_goal(goal)
        .build()
        .map(Arc::new)
}

/// Creates an example problem used in documentation tests.
pub fn create_example_problem() -> Arc<Problem> {
    build_example_problem().expect("cannot build example problem")
}
