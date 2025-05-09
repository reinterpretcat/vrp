use crate::construction::heuristics::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::ActivityBuilder;
use crate::helpers::models::solution::{RouteBuilder, RouteContextBuilder, create_test_registry};
use crate::models::common::{Cost, Location, Schedule, TimeSpan, TimeWindow, Timestamp};
use crate::models::problem::{Job, Single, VehicleDetail};
use crate::models::solution::{Activity, Place, Registry};
use std::sync::Arc;

type JobPlace = crate::models::problem::Place;

fn create_test_insertion_ctx() -> InsertionContext {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver_with_costs(empty_costs()))
        .add_vehicle(TestVehicleBuilder::default().id("v1").build())
        .build();
    let route =
        RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build()).build();

    TestInsertionContextBuilder::default()
        .with_goal(TestGoalContextBuilder::with_transport_feature().build())
        .with_routes(vec![route])
        .build()
}

fn create_activity_at(loc_and_time: usize) -> Activity {
    ActivityBuilder::default()
        .place(Place { idx: 0, location: loc_and_time, duration: 0.0, time: DEFAULT_JOB_TIME_SPAN.to_time_window(0.) })
        .schedule(Schedule { arrival: loc_and_time as Timestamp, departure: loc_and_time as Timestamp })
        .build()
}

fn evaluate_job_insertion(
    insertion_ctx: &mut InsertionContext,
    job: &Job,
    insertion_position: InsertionPosition,
) -> InsertionResult {
    let route_selector = AllRouteSelector::default();
    let leg_selection = LegSelection::Stochastic(insertion_ctx.environment.random.clone());
    let result_selector = BestResultSelector::default();
    let routes = route_selector.select(insertion_ctx, &[]).collect::<Vec<_>>();

    let eval_ctx = EvaluationContext {
        goal: &insertion_ctx.problem.goal,
        job,
        leg_selection: &leg_selection,
        result_selector: &result_selector,
    };

    routes.iter().fold(InsertionResult::make_failure(), |acc, route_ctx| {
        eval_job_insertion_in_route(insertion_ctx, &eval_ctx, route_ctx, insertion_position, acc)
    })
}

mod single {
    use super::*;
    use crate::construction::heuristics::evaluators::InsertionPosition;
    use crate::helpers::models::domain::test_random;
    use crate::helpers::models::solution::RouteBuilder;
    use crate::models::common::TimeInterval;
    use crate::models::problem::VehiclePlace;
    use crate::prelude::ViolationCode;

    parameterized_test! {can_insert_job_with_location_into_empty_tour, (job, position, has_result), {
        can_insert_job_with_location_into_empty_tour_impl(job, position, has_result);
    }}

    can_insert_job_with_location_into_empty_tour! {
        case01: (TestSingleBuilder::default().build_as_job_ref(), InsertionPosition::Any, true),
        case02: (TestSingleBuilder::default().location(None).build_as_job_ref(), InsertionPosition::Any, true),

        case03: (TestSingleBuilder::default().build_as_job_ref(), InsertionPosition::Concrete(0), true),
        case04: (TestSingleBuilder::default().location(None).build_as_job_ref(), InsertionPosition::Concrete(0), true),
        case05: (TestSingleBuilder::default().build_as_job_ref(), InsertionPosition::Concrete(1), false),

        case06: (TestSingleBuilder::default().build_as_job_ref(), InsertionPosition::Last, true),
        case07: (TestSingleBuilder::default().location(None).build_as_job_ref(), InsertionPosition::Last, true),
    }

    fn can_insert_job_with_location_into_empty_tour_impl(job: Job, position: InsertionPosition, has_result: bool) {
        let mut ctx = create_test_insertion_ctx();

        let result = evaluate_job_insertion(&mut ctx, &job, position);

        match result {
            InsertionResult::Success(success) => {
                assert_eq!(success.activities.len(), 1);
                assert_eq!(success.activities.first().unwrap().1, 0);
                assert_eq!(success.activities.first().unwrap().0.place.location, DEFAULT_JOB_LOCATION);
            }
            _ => {
                assert!(!has_result)
            }
        }
    }

    parameterized_test! {can_insert_job_with_location_into_tour_with_two_activities_and_variations, (places, location, position, index), {
        let job = Job::Single(Arc::new(Single { places, dimens: Default::default() }));
        can_insert_job_with_location_into_tour_with_two_activities_and_variations_impl(job, location, position, index);
    }}

    can_insert_job_with_location_into_tour_with_two_activities_and_variations! {
        // vary times
        case01: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], InsertionPosition::Any, 3, 0),
        case02: (vec![JobPlace { location: Some(8), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], InsertionPosition::Any, 8, 1),
        case03: (vec![JobPlace { location: Some(7), duration: 0.0, times: vec![TimeSpan::Window(TimeWindow::new(15.0, 20.0))] }], InsertionPosition::Any, 7, 2),
        case04: (vec![JobPlace { location: Some(7), duration: 0.0, times: vec![TimeSpan::Window(TimeWindow::new(15.0, 20.0)),
                                                                               TimeSpan::Window(TimeWindow::new(7.0, 8.0))] }], InsertionPosition::Any, 7, 1),

        // vary locations
        case05: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], InsertionPosition::Any, 3, 0),
        case06: (vec![JobPlace { location: Some(20), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] },
                      JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], InsertionPosition::Any, 3, 0),

        // vary locations and times
        case07: (vec![JobPlace { location: Some(20), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] },
                      JobPlace { location: Some(3), duration: 0.0, times: vec![TimeSpan::Window(TimeWindow::new(0.0, 2.0))] }], InsertionPosition::Any, 20, 1),
        case08: (vec![JobPlace { location: Some(12), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] },
                      JobPlace { location: Some(11), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], InsertionPosition::Any, 11, 1),

        // vary insertion position
        case09: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], InsertionPosition::Last, 3, 2),
        case10: (vec![JobPlace { location: Some(3), duration: 0.0, times: vec![DEFAULT_JOB_TIME_SPAN] }], InsertionPosition::Concrete(1), 3, 1),
    }

    fn can_insert_job_with_location_into_tour_with_two_activities_and_variations_impl(
        job: Job,
        insertion_position: InsertionPosition,
        location: Location,
        index: usize,
    ) {
        let mut registry = create_test_registry();
        let mut route_ctx = RouteContext::new(registry.next().next().unwrap());
        registry.use_actor(&route_ctx.route().actor);
        route_ctx.route_mut().tour.insert_at(create_activity_at(5), 1).insert_at(create_activity_at(10), 2);
        let mut ctx = TestInsertionContextBuilder::default()
            .with_goal(TestGoalContextBuilder::with_transport_feature().build())
            .with_registry(registry)
            .with_routes(vec![route_ctx])
            .build();

        let result = evaluate_job_insertion(&mut ctx, &job, insertion_position);

        let success: InsertionSuccess = result.try_into().ok().unwrap();
        assert_eq!(success.activities.len(), 1);
        assert_eq!(success.activities.first().unwrap().1, index);
        assert_eq!(success.activities.first().unwrap().0.place.location, location);
    }

    parameterized_test! {can_insert_job_with_two_vehicles_and_various_time_constraints, (job_location, v1_end_location, v2_end_location, expected_used_vehicle, cost), {
        can_insert_job_with_two_vehicles_and_various_time_constraints_impl(job_location, v1_end_location, v2_end_location, expected_used_vehicle, cost);
    }}

    can_insert_job_with_two_vehicles_and_various_time_constraints! {
        case1: (3, 0, 20, "v1", (3.0 + 3.0) * 2.0),
        case2: (27, 0, 20, "v2", (7.0 + 7.0) * 2.0),
        case3: (11, 12, 20, "v1", (12.0 + 12.0)),
    }

    fn can_insert_job_with_two_vehicles_and_various_time_constraints_impl(
        job_location: Location,
        v1_end_location: Location,
        v2_end_location: Location,
        expected_used_vehicle: &str,
        cost: Cost,
    ) {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vec![
                TestVehicleBuilder::default()
                    .id("v1")
                    .details(vec![VehicleDetail {
                        start: Some(VehiclePlace {
                            location: 0,
                            time: TimeInterval { earliest: Some(0.), latest: None },
                        }),
                        end: Some(VehiclePlace {
                            location: v1_end_location,
                            time: TimeInterval { earliest: None, latest: Some(100.) },
                        }),
                    }])
                    .build(),
                TestVehicleBuilder::default()
                    .id("v2")
                    .details(vec![VehicleDetail {
                        start: Some(VehiclePlace {
                            location: 20,
                            time: TimeInterval { earliest: Some(0.), latest: None },
                        }),
                        end: Some(VehiclePlace {
                            location: v2_end_location,
                            time: TimeInterval { earliest: None, latest: Some(100.) },
                        }),
                    }])
                    .build(),
            ])
            .build();
        let registry = Registry::new(&fleet, test_random());
        let job = TestSingleBuilder::default().location(Some(job_location)).build_as_job_ref();
        let mut ctx = TestInsertionContextBuilder::default()
            .with_goal(TestGoalContextBuilder::with_transport_feature().build())
            .with_registry(registry)
            .with_routes(vec![
                RouteContextBuilder::default()
                    .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
                    .build(),
            ])
            .build();

        let result = evaluate_job_insertion(&mut ctx, &job, InsertionPosition::Any);

        let success: InsertionSuccess = result.try_into().ok().unwrap();
        assert_eq!(success.activities.len(), 1);
        assert_eq!(get_vehicle_id(&success.actor.vehicle), &expected_used_vehicle.to_owned());
        assert_eq!(success.cost, InsertionCost::new(&[cost]));
    }

    #[test]
    fn can_detect_and_return_insertion_violation() {
        let job = TestSingleBuilder::default().location(Some(1111)).build_as_job_ref();
        let mut ctx = create_test_insertion_ctx();

        let result = evaluate_job_insertion(&mut ctx, &job, InsertionPosition::Any);

        match result {
            InsertionResult::Failure(failure) => {
                assert_eq!(failure.constraint, ViolationCode(1));
            }
            _ => {
                unreachable!()
            }
        }
    }
}

mod multi {
    use super::*;
    use crate::construction::heuristics::evaluators::InsertionPosition;
    use crate::models::ViolationCode;

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
        let job = Job::Multi(test_multi_with_id(
            "multi",
            vec![
                TestSingleBuilder::default().id("s1").location(Some(3)).build_shared(),
                TestSingleBuilder::default().id("s2").location(Some(7)).build_shared(),
            ],
        ));
        let mut ctx = create_test_insertion_ctx();

        let result = evaluate_job_insertion(&mut ctx, &job, InsertionPosition::Any);

        let success: InsertionSuccess = result.try_into().ok().unwrap();
        assert_eq!(success.cost, InsertionCost::new(&[28.0]));
        assert_activities(success, vec![(0, 3), (1, 7)]);
    }

    parameterized_test! {can_handle_activity_constraint_violation, activities, {
        can_handle_activity_constraint_violation_impl(activities);
    }}

    can_handle_activity_constraint_violation! {
        case1: vec![(0, 3), (1, 1111)],
        case2: vec![(0, 1111), (1, 3)],
    }

    fn can_handle_activity_constraint_violation_impl(singles: Vec<InsertionData>) {
        let job = Job::Multi(test_multi_with_id(
            "multi",
            singles
                .iter()
                .zip(0..)
                .map(|((_, loc), index)| {
                    TestSingleBuilder::default().id(&index.to_string()).location(Some(*loc)).build_shared()
                })
                .collect(),
        ));
        let mut ctx = create_test_insertion_ctx();

        let result = evaluate_job_insertion(&mut ctx, &job, InsertionPosition::Any);

        match result {
            InsertionResult::Failure(failure) => {
                assert_eq!(failure.constraint, ViolationCode(1));
            }
            _ => {
                unreachable!()
            }
        }
    }

    parameterized_test! {can_insert_job_with_singles_into_tour_with_activities, (existing, position, expected, cost), {
        can_insert_job_with_singles_into_tour_with_activities_impl(existing, position, expected, cost);
    }}

    can_insert_job_with_singles_into_tour_with_activities! {
        // any position
        case01: (vec![(1, 5)], InsertionPosition::Any, vec![(0, 3), (1, 7)], 8.),                   // s 3  7 [5] e
        case02: (vec![(1, 5)], InsertionPosition::Any, vec![(0, 7), (2, 3)], 8.),                   // s 7 [5] 3  e
        case03: (vec![(1, 5), (2, 9)], InsertionPosition::Any, vec![(0, 3), (2, 7), (3, 11)], 8.),  // s 3 [5] 7 11 [9] e
        case04: (vec![(1, 3), (2, 7)], InsertionPosition::Any, vec![(0, 1), (2, 9)], 8.),           // s 1 [3] 9 [7] e,
        case05: (vec![(1, 7), (2, 3)], InsertionPosition::Any, vec![(0, 9), (3, 1)], 8.),           // s 9 [7] [3] 1  e
        case06: (vec![(1, 7), (2, 3)], InsertionPosition::Any, vec![(0, 9), (2, 5)], 8.),           // s 9 [7]  5 [3] e

        // last position
        case07: (vec![(1, 5)], InsertionPosition::Last, vec![(1, 3), (2, 7)], 16.),                 // s [5] 3 7 e
        case08: (vec![(1, 7), (2, 3)], InsertionPosition::Last, vec![(2, 9), (3, 5)], 24.),         // s [7] [3] 9 5 e

        // concrete position
        case09: (vec![(1, 5)], InsertionPosition::Concrete(1), vec![(1, 3), (2, 7)], 16.),          // s [5] 3 7 e
        case10: (vec![(1, 7), (2, 3)], InsertionPosition::Concrete(1), vec![(1, 9), (2, 5)], 8.),   // s [7] 9 5 [3] e
    }

    fn can_insert_job_with_singles_into_tour_with_activities_impl(
        existing: Vec<InsertionData>,
        position: InsertionPosition,
        expected: Vec<InsertionData>,
        cost: Cost,
    ) {
        let registry = create_test_registry();
        let mut route_ctx = RouteContext::new(registry.next().next().unwrap());
        existing.iter().for_each(|&(index, loc)| {
            route_ctx.route_mut().tour.insert_at(create_activity_at(loc), index);
        });
        let mut ctx = TestInsertionContextBuilder::default()
            .with_goal(TestGoalContextBuilder::with_transport_feature().build())
            .with_routes(vec![route_ctx])
            .build();

        let job = Job::Multi(test_multi_with_id(
            "multi",
            expected
                .iter()
                .zip(0usize..)
                .map(|((_, loc), index)| {
                    TestSingleBuilder::default().id(&index.to_string()).location(Some(*loc)).build_shared()
                })
                .collect(),
        ));

        let result = evaluate_job_insertion(&mut ctx, &job, position);

        let success: InsertionSuccess = result.try_into().ok().unwrap();
        assert_eq!(success.cost, InsertionCost::new(&[cost]));
        assert_eq!(success.activities.len(), expected.len());
        assert_activities(success, expected);
    }

    #[test]
    fn can_choose_cheaper_permutation_from_two() {
        let mut ctx = create_test_insertion_ctx();
        let job = Job::Multi(test_multi_with_permutations(
            "multi",
            vec![
                TestSingleBuilder::default().id("s1").location(Some(10)).build_shared(),
                TestSingleBuilder::default().id("s2").location(Some(5)).build_shared(),
                TestSingleBuilder::default().id("s3").location(Some(15)).build_shared(),
            ],
            vec![vec![0, 1, 2], vec![1, 0, 2], vec![2, 1, 0]],
        ));

        let result = evaluate_job_insertion(&mut ctx, &job, InsertionPosition::Any);

        let success: InsertionSuccess = result.try_into().ok().unwrap();
        assert_eq!(success.cost, InsertionCost::new(&[60.]));
        assert_activities(success, vec![(0, 5), (1, 10), (2, 15)]);
    }
}
