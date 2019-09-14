use crate::construction::constraints::{ConstraintPipeline, TimingConstraintModule};
use crate::construction::heuristics::evaluators::InsertionEvaluator;
use crate::construction::states::{InsertionContext, InsertionResult, RouteContext, RouteState, SolutionContext};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_timing;
use crate::helpers::construction::states::test_insertion_progress;
use crate::helpers::models::domain::{create_empty_problem, create_empty_problem_with_constraint};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::ActivityBuilder;
use crate::models::common::{Location, Schedule, TimeWindow};
use crate::models::problem::{Fleet, Job};
use crate::models::solution::{Place, Registry};
use std::collections::HashSet;
use std::sync::Arc;

fn create_insertion_context(
    registry: Registry,
    constraint: ConstraintPipeline,
    routes: HashSet<RouteContext>,
) -> InsertionContext {
    InsertionContext {
        progress: test_insertion_progress(),
        problem: create_empty_problem_with_constraint(constraint),
        solution: Arc::new(SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned: Default::default(),
            routes,
            registry: Arc::new(registry),
        }),
        random: Arc::new("".to_string()),
    }
}

parameterized_test! {can_insert_service_with_location_into_empty_tour, job, {
    can_insert_service_with_location_into_empty_tour_impl(job);
}}

can_insert_service_with_location_into_empty_tour! {
    case1: Arc::new(test_single_job()),
    case2: Arc::new(test_single_job_with_location(None)),
}

fn can_insert_service_with_location_into_empty_tour_impl(job: Arc<Job>) {
    let registry = Registry::new(&Fleet::new(
        vec![test_driver_with_costs(empty_costs())],
        vec![VehicleBuilder::new().id("v1").build()],
    ));
    let mut routes: HashSet<RouteContext> = HashSet::new();
    routes.insert(RouteContext::new(registry.next().next().unwrap()));
    let ctx = create_insertion_context(registry, ConstraintPipeline::new(), routes);

    let result = InsertionEvaluator::new().evaluate(&job, &ctx);

    if let InsertionResult::Success(success) = result {
        assert_eq!(success.activities.len(), 1);
        assert_eq!(success.activities.first().unwrap().1, 0);
        assert_eq!(success.activities.first().unwrap().0.place.location, DEFAULT_JOB_LOCATION);
    } else {
        assert!(false);
    }
}

parameterized_test! {can_insert_service_with_location_into_tour_with_two_activities_and_time_window_variations, (location, tws, index), {
    can_insert_service_with_location_into_tour_with_two_activities_and_time_window_variations_impl(location, tws, index);
}}

can_insert_service_with_location_into_tour_with_two_activities_and_time_window_variations! {
    case1: (3, vec![DEFAULT_JOB_TIME_WINDOW], 0),
    case2: (8, vec![DEFAULT_JOB_TIME_WINDOW], 1),
    case3: (7, vec![TimeWindow {start: 15.0, end: 20.0}], 2),
    case4: (7, vec![TimeWindow {start: 15.0, end: 20.0}, TimeWindow {start: 7.0, end: 8.0}], 1),
}

fn can_insert_service_with_location_into_tour_with_two_activities_and_time_window_variations_impl(
    location: Location,
    tws: Vec<TimeWindow>,
    index: usize,
) {
    let registry = Registry::new(&Fleet::new(
        vec![test_driver_with_costs(empty_costs())],
        vec![VehicleBuilder::new().id("v1").build()],
    ));
    let mut route_ctx = RouteContext::new(registry.next().next().unwrap());
    route_ctx
        .route
        .write()
        .unwrap()
        .tour
        .insert_at(
            Box::new(
                ActivityBuilder::new()
                    .place(Place { location: 5, duration: 0.0, time: DEFAULT_JOB_TIME_WINDOW.clone() })
                    .schedule(Schedule { arrival: 5.0, departure: 5.0 })
                    .build(),
            ),
            1,
        )
        .insert_at(
            Box::new(
                ActivityBuilder::new()
                    .place(Place { location: 10, duration: 0.0, time: DEFAULT_JOB_TIME_WINDOW.clone() })
                    .schedule(Schedule { arrival: 10.0, departure: 10.0 })
                    .build(),
            ),
            2,
        );
    let mut routes: HashSet<RouteContext> = HashSet::new();
    routes.insert(route_ctx);
    let job = Arc::new(Job::Single(SingleBuilder::new().location(Some(location)).duration(0.0).times(tws).build()));
    let mut constraint = create_constraint_pipeline_with_timing();
    let ctx = create_insertion_context(registry, constraint, routes);

    let result = InsertionEvaluator::new().evaluate(&job, &ctx);

    if let InsertionResult::Success(success) = result {
        assert_eq!(success.activities.len(), 1);
        assert_eq!(success.activities.first().unwrap().1, index);
        assert_eq!(success.activities.first().unwrap().0.place.location, location);
    } else {
        assert!(false);
    }
}
