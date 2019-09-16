use crate::construction::constraints::{ConstraintPipeline, TimingConstraintModule};
use crate::construction::heuristics::evaluators::InsertionEvaluator;
use crate::construction::states::*;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_timing;
use crate::helpers::construction::states::test_insertion_progress;
use crate::helpers::models::domain::{create_empty_problem, create_empty_problem_with_constraint};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::ActivityBuilder;
use crate::models::common::{Cost, Location, Schedule, TimeWindow};
use crate::models::problem::{Fleet, Job, Single, VehicleDetail};
use crate::models::solution::{Place, Registry};
use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;

type JobPlace = crate::models::problem::Place;

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

fn create_test_insertion_context(registry: Registry) -> InsertionContext {
    let route_ctx = RouteContext::new(registry.next().next().unwrap());
    let mut routes: HashSet<RouteContext> = HashSet::new();
    routes.insert(route_ctx);
    let mut constraint = create_constraint_pipeline_with_timing();
    create_insertion_context(registry, constraint, routes)
}

mod multi {
    use super::*;
    use crate::construction::heuristics::evaluators::get_job_permutations;
    use crate::construction::heuristics::evaluators::get_permutations;
    use crate::models::problem::Multi;

    fn assert_activities(success: InsertionSuccess, expected: Vec<(usize, Location)>) {
        assert_eq!(success.activities.len(), expected.len());
        success.activities.iter().zip(expected.iter()).for_each(|((activity, position), (index, location))| {
            assert_eq!(&activity.place.location, location);
            assert_eq!(position, index);
        });
    }

    #[test]
    fn can_generate_permutations() {
        let mut permutations = get_permutations(3);

        assert_eq!(permutations.next().unwrap(), vec![0, 1, 2]);
        assert_eq!(permutations.next().unwrap(), vec![1, 0, 2]);
        assert_eq!(permutations.next().unwrap(), vec![2, 0, 1]);
        assert_eq!(permutations.next().unwrap(), vec![0, 2, 1]);
        assert_eq!(permutations.next().unwrap(), vec![1, 2, 0]);
        assert_eq!(permutations.next().unwrap(), vec![2, 1, 0]);
        assert_eq!(permutations.next(), None);
    }

    #[test]
    fn can_generate_job_permutations() {
        let multi = if let Job::Multi(multi) =
            test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)], vec![Some(3)]])
        {
            multi
        } else {
            panic!()
        };

        let job_permutations = get_job_permutations(&multi);

        assert_eq!(job_permutations.len(), 3);
    }

    #[test]
    fn can_insert_job_with_location_into_empty_tour_impl() {
        let s1_location: Option<Location> = Some(3);
        let s2_location: Option<Location> = Some(7);
        let cost: Cost = 28.0;

        let registry = Registry::new(&Fleet::new(
            vec![test_driver_with_costs(empty_costs())],
            vec![VehicleBuilder::new().id("v1").build()],
        ));
        let job = MultiBuilder::new()
            .job(SingleBuilder::new().id("s1").location(s1_location).build())
            .job(SingleBuilder::new().id("s2").location(s2_location).build())
            .build_as_job_ref();
        let ctx = create_test_insertion_context(registry);

        let result = InsertionEvaluator::new().evaluate(&job, &ctx);

        if let InsertionResult::Success(success) = result {
            assert_activities(success, vec![(0, 3), (1, 7)]);
        } else {
            assert!(false);
        }
    }
}

mod single {
    use super::*;

    parameterized_test! {can_insert_job_with_location_into_empty_tour, job, {
        can_insert_job_with_location_into_empty_tour_impl(job);
    }}

    can_insert_job_with_location_into_empty_tour! {
        case1: Arc::new(test_single_job()),
        case2: Arc::new(test_single_job_with_location(None)),
    }

    fn can_insert_job_with_location_into_empty_tour_impl(job: Arc<Job>) {
        let registry = Registry::new(&Fleet::new(
            vec![test_driver_with_costs(empty_costs())],
            vec![VehicleBuilder::new().id("v1").build()],
        ));
        let ctx = create_test_insertion_context(registry);

        let result = InsertionEvaluator::new().evaluate(&job, &ctx);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.activities.len(), 1);
            assert_eq!(success.activities.first().unwrap().1, 0);
            assert_eq!(success.activities.first().unwrap().0.place.location, DEFAULT_JOB_LOCATION);
        } else {
            assert!(false);
        }
    }

    parameterized_test! {can_insert_job_with_location_into_tour_with_two_activities_and_variations, (places, location, index), {
        let job = Arc::new(Job::Single(
            Single {
                places,
                dimens: Default::default()
            }
        ));
        can_insert_job_with_location_into_tour_with_two_activities_and_variations_impl(job, location, index);
    }}

    can_insert_job_with_location_into_tour_with_two_activities_and_variations! {
        // vary times
        case01: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] }], 3, 0),
        case02: (vec![JobPlace { location: Some(8), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] }], 8, 1),
        case03: (vec![JobPlace { location: Some(7), duration: 0.0, times: vec![TimeWindow {start: 15.0, end: 20.0}] }], 7, 2),
        case04: (vec![JobPlace { location: Some(7), duration: 0.0, times: vec![TimeWindow {start: 15.0, end: 20.0}, TimeWindow {start: 7.0, end: 8.0}] }], 7, 1),

        // vary locations
        case05: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] }], 3, 0),
        case06: (vec![JobPlace { location: Some(20), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] },
                     JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] }], 3, 0),

        // vary locations and times
        case07: (vec![JobPlace { location: Some(20), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] },
                      JobPlace { location: Some(3), duration: 0.0, times: vec![TimeWindow {start: 0.0, end: 2.0}] }], 20, 1),
        case08: (vec![JobPlace { location: Some(12), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] },
                      JobPlace { location: Some(11), duration: 0.0, times: vec![DEFAULT_JOB_TIME_WINDOW] }], 11, 1),
    }

    fn can_insert_job_with_location_into_tour_with_two_activities_and_variations_impl(
        job: Arc<Job>,
        location: Location,
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

    parameterized_test! {can_insert_job_with_two_vehicles_and_various_time_constraints, (job_location, v1_end_location, v2_end_location, expected_used_vehicle, cost), {
        can_insert_job_with_two_vehicles_and_various_time_constraints_impl(job_location, v1_end_location, v2_end_location, expected_used_vehicle, cost);
    }}

    can_insert_job_with_two_vehicles_and_various_time_constraints! {
        case1: (3, Some(0), Some(20), "v1", (3.0 + 3.0) * 2.0),
        case2: (27, Some(0), Some(20), "v2", (7.0 + 7.0) * 2.0),
        case3: (11, Some(12), Some(20), "v1", (12.0 + 12.0)),
    }

    fn can_insert_job_with_two_vehicles_and_various_time_constraints_impl(
        job_location: Location,
        v1_end_location: Option<Location>,
        v2_end_location: Option<Location>,
        expected_used_vehicle: &str,
        cost: Cost,
    ) {
        let registry = Registry::new(&Fleet::new(
            vec![test_driver_with_costs(empty_costs())],
            vec![
                VehicleBuilder::new()
                    .id("v1")
                    .details(vec![VehicleDetail {
                        start: Some(0),
                        end: v1_end_location,
                        time: Some(TimeWindow { start: 0.0, end: 100.0 }),
                    }])
                    .build(),
                VehicleBuilder::new()
                    .id("v2")
                    .details(vec![VehicleDetail {
                        start: Some(20),
                        end: v2_end_location,
                        time: Some(TimeWindow { start: 0.0, end: 100.0 }),
                    }])
                    .build(),
            ],
        ));
        let job = Arc::new(test_single_job_with_location(Some(job_location)));
        let ctx = create_test_insertion_context(registry);

        let result = InsertionEvaluator::new().evaluate(&job, &ctx);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.activities.len(), 1);
            assert_eq!(
                get_vehicle_id(success.context.route.read().unwrap().actor.vehicle.deref()),
                &expected_used_vehicle.to_owned()
            );
            assert_eq!(compare_floats(&success.cost, &cost), Ordering::Equal);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn can_detect_and_return_insertion_violation() {
        let registry = Registry::new(&Fleet::new(
            vec![test_driver_with_costs(empty_costs())],
            vec![VehicleBuilder::new().id("v1").build()],
        ));
        let job = Arc::new(test_single_job_with_location(Some(1111)));
        let ctx = create_test_insertion_context(registry);

        let result = InsertionEvaluator::new().evaluate(&job, &ctx);

        if let InsertionResult::Failure(failure) = result {
            assert_eq!(failure.constraint, 1);
        } else {
            assert!(false);
        }
    }
}
