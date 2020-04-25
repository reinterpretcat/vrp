use crate::construction::heuristics::evaluators::evaluate_job_insertion;
use crate::construction::heuristics::*;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
use crate::helpers::construction::heuristics::{create_insertion_context, create_test_insertion_context};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::create_test_registry;
use crate::helpers::models::solution::ActivityBuilder;
use crate::models::common::{Cost, Location, Schedule, TimeSpan, TimeWindow, Timestamp};
use crate::models::problem::{Job, Single, VehicleDetail};
use crate::models::solution::{Place, Registry, TourActivity};
use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

type JobPlace = crate::models::problem::Place;

fn create_tour_activity_at(loc_and_time: usize) -> TourActivity {
    Box::new(
        ActivityBuilder::default()
            .place(Place { location: loc_and_time, duration: 0.0, time: DEFAULT_JOB_TIME_SPAN.to_time_window(0.) })
            .schedule(Schedule { arrival: loc_and_time as Timestamp, departure: loc_and_time as Timestamp })
            .build(),
    )
}

mod single {
    use super::*;
    use crate::construction::heuristics::evaluators::InsertionPosition;

    parameterized_test! {can_insert_job_with_location_into_empty_tour, job, {
        can_insert_job_with_location_into_empty_tour_impl(job);
    }}

    can_insert_job_with_location_into_empty_tour! {
        case1: Job::Single(Arc::new(test_single())),
        case2: Job::Single(test_single_with_location(None)),
    }

    fn can_insert_job_with_location_into_empty_tour_impl(job: Job) {
        let ctx = create_test_insertion_context(create_test_registry());

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.activities.len(), 1);
            assert_eq!(success.activities.first().unwrap().1, 0);
            assert_eq!(success.activities.first().unwrap().0.place.location, DEFAULT_JOB_LOCATION);
        } else {
            unreachable!()
        }
    }

    parameterized_test! {can_insert_job_with_location_into_tour_with_two_activities_and_variations, (places, location, index), {
        let job = Job::Single(Arc::new(Single { places, dimens: Default::default() }));
        can_insert_job_with_location_into_tour_with_two_activities_and_variations_impl(job, location, index);
    }}

    can_insert_job_with_location_into_tour_with_two_activities_and_variations! {
        // vary times
        case01: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], 3, 0),
        case02: (vec![JobPlace { location: Some(8), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], 8, 1),
        case03: (vec![JobPlace { location: Some(7), duration: 0.0, times: vec![TimeSpan::Window(TimeWindow::new(15.0, 20.0))] }], 7, 2),
        case04: (vec![JobPlace { location: Some(7), duration: 0.0, times: vec![TimeSpan::Window(TimeWindow::new(15.0, 20.0)),
                                                                               TimeSpan::Window(TimeWindow::new(7.0, 8.0))] }], 7, 1),

        // vary locations
        case05: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], 3, 0),
        case06: (vec![JobPlace { location: Some(20), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] },
                     JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], 3, 0),

        // vary locations and times
        case07: (vec![JobPlace { location: Some(20), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] },
                      JobPlace { location: Some(3), duration: 0.0, times: vec![TimeSpan::Window(TimeWindow::new(0.0, 2.0))] }], 20, 1),
        case08: (vec![JobPlace { location: Some(12), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] },
                      JobPlace { location: Some(11), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], 11, 1),
    }

    fn can_insert_job_with_location_into_tour_with_two_activities_and_variations_impl(
        job: Job,
        location: Location,
        index: usize,
    ) {
        let registry = create_test_registry();
        let mut route_ctx = RouteContext::new(registry.next().next().unwrap());
        route_ctx.route_mut().tour.insert_at(create_tour_activity_at(5), 1).insert_at(create_tour_activity_at(10), 2);
        let routes = vec![route_ctx];
        let constraint = create_constraint_pipeline_with_transport();
        let ctx = create_insertion_context(registry, constraint, routes);

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.activities.len(), 1);
            assert_eq!(success.activities.first().unwrap().1, index);
            assert_eq!(success.activities.first().unwrap().0.place.location, location);
        } else {
            unreachable!()
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
        let registry = Registry::new(
            &FleetBuilder::default()
                .add_driver(test_driver_with_costs(empty_costs()))
                .add_vehicles(vec![
                    VehicleBuilder::default()
                        .id("v1")
                        .details(vec![VehicleDetail {
                            start: Some(0),
                            end: v1_end_location,
                            time: Some(TimeWindow { start: 0.0, end: 100.0 }),
                        }])
                        .build(),
                    VehicleBuilder::default()
                        .id("v2")
                        .details(vec![VehicleDetail {
                            start: Some(20),
                            end: v2_end_location,
                            time: Some(TimeWindow { start: 0.0, end: 100.0 }),
                        }])
                        .build(),
                ])
                .build(),
        );
        let job = Job::Single(test_single_with_location(Some(job_location)));
        let ctx = create_test_insertion_context(registry);

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.activities.len(), 1);
            assert_eq!(get_vehicle_id(success.context.route.actor.vehicle.deref()), &expected_used_vehicle.to_owned());
            assert_eq!(compare_floats(success.cost, cost), Ordering::Equal);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn can_detect_and_return_insertion_violation() {
        let job = Job::Single(test_single_with_location(Some(1111)));
        let ctx = create_test_insertion_context(create_test_registry());

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Failure(failure) = result {
            assert_eq!(failure.constraint, 1);
        } else {
            unreachable!()
        }
    }
}

mod multi {
    use super::*;
    use crate::construction::heuristics::evaluators::InsertionPosition;

    type InsertionData = (usize, Location);

    fn assert_activities(success: InsertionSuccess, expected: Vec<InsertionData>) {
        assert_eq!(success.activities.len(), expected.len());
        success.activities.iter().zip(expected.iter()).for_each(|((activity, position), (index, location))| {
            assert_eq!(&activity.place.location, location);
            assert_eq!(position, index);
        });
    }

    #[test]
    fn can_insert_job_with_location_into_empty_tour() {
        let job = MultiBuilder::default()
            .job(SingleBuilder::default().id("s1").location(Some(3)).build())
            .job(SingleBuilder::default().id("s2").location(Some(7)).build())
            .build();
        let ctx = create_test_insertion_context(create_test_registry());

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.cost, 28.0);
            assert_activities(success, vec![(0, 3), (1, 7)]);
        } else {
            unreachable!()
        }
    }

    parameterized_test! {can_handle_activity_constraint_violation, activities, {
        can_handle_activity_constraint_violation_impl(activities);
    }}

    can_handle_activity_constraint_violation! {
        case1: vec![(0, 3), (1, 1111)],
        case2: vec![(0, 1111), (1, 3)],
    }
    fn can_handle_activity_constraint_violation_impl(singles: Vec<InsertionData>) {
        let mut job = MultiBuilder::default();
        singles.iter().zip(0usize..).for_each(|((_, loc), index)| {
            job.job(SingleBuilder::default().id(&index.to_string()).location(Some(*loc)).build());
        });
        let job = job.build();
        let ctx = create_test_insertion_context(create_test_registry());

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Failure(failure) = result {
            assert_eq!(failure.constraint, 1);
        } else {
            unreachable!()
        }
    }

    parameterized_test! {can_insert_job_with_singles_into_tour_with_activities, (existing, expected, cost), {
        can_insert_job_with_singles_into_tour_with_activities_impl(existing, expected, cost);
    }}

    can_insert_job_with_singles_into_tour_with_activities! {
        case01: (vec![(1, 5)], vec![(0, 3), (1, 7)], 8.0),                   // s 3  7 [5] e
        case02: (vec![(1, 5)], vec![(0, 7), (2, 3)], 8.0),                   // s 7 [5] 3  e
        case03: (vec![(1, 5), (2, 9)], vec![(0, 3), (2, 7), (3, 11)], 8.0),  // s 3 [5] 7 11 [9] e
        case04: (vec![(1, 3), (2, 7)], vec![(0, 1), (2, 9)], 8.0),           // s 1 [3] 9 [7] e,
        case05: (vec![(1, 7), (2, 3)], vec![(0, 9), (3, 1)], 8.0),           // s 9 [7] [3] 1  e
        case06: (vec![(1, 7), (2, 3)], vec![(0, 9), (2, 5)], 8.0),           // s 9 [7]  5 [3] e
    }

    fn can_insert_job_with_singles_into_tour_with_activities_impl(
        existing: Vec<InsertionData>,
        expected: Vec<InsertionData>,
        cost: Cost,
    ) {
        let registry = create_test_registry();
        let mut route_ctx = RouteContext::new(registry.next().next().unwrap());
        existing.iter().for_each(|&(index, loc)| {
            route_ctx.route_mut().tour.insert_at(create_tour_activity_at(loc), index);
        });
        let routes = vec![route_ctx];
        let constraint = create_constraint_pipeline_with_transport();
        let ctx = create_insertion_context(registry, constraint, routes);
        let mut job = MultiBuilder::default();
        expected.iter().zip(0usize..).for_each(|((_, loc), index)| {
            job.job(SingleBuilder::default().id(&index.to_string()).location(Some(*loc)).build());
        });
        let job = job.build();

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.cost, cost);
            assert_eq!(success.activities.len(), expected.len());
            assert_activities(success, expected);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn can_choose_cheaper_permutation_from_two() {
        let ctx = create_test_insertion_context(create_test_registry());
        let job = MultiBuilder::new_with_permutations(vec![vec![0, 1, 2], vec![1, 0, 2], vec![2, 1, 0]])
            .job(SingleBuilder::default().id("s1").location(Some(10)).build())
            .job(SingleBuilder::default().id("s2").location(Some(5)).build())
            .job(SingleBuilder::default().id("s3").location(Some(15)).build())
            .build();

        let result = evaluate_job_insertion(&job, &ctx, InsertionPosition::Any);

        if let InsertionResult::Success(success) = result {
            assert_eq!(success.cost, 60.0);
            assert_activities(success, vec![(0, 5), (1, 10), (2, 15)]);
        } else {
            unreachable!()
        }
    }
}
