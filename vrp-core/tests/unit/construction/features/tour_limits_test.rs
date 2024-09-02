use crate::construction::features::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::Location;
use crate::models::problem::Job;
use std::sync::Arc;

mod activity {
    use super::*;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;

    const VIOLATION_CODE: ViolationCode = ViolationCode(1);

    parameterized_test! {can_limit_by_job_activities, (activities, job_size, limit, expected), {
        can_limit_by_job_activities_impl(activities, job_size, limit, expected);
    }}

    can_limit_by_job_activities! {
        case01: (3, 1, Some(3), ConstraintViolation::fail(VIOLATION_CODE)),
        case02: (3, 1, None, None),
        case03: (2, 1, Some(3), None),

        case04: (2, 2, Some(3), ConstraintViolation::fail(VIOLATION_CODE)),
        case05: (2, 2, None, None),
        case06: (1, 2, Some(3), None),
    }

    fn can_limit_by_job_activities_impl(
        activities: usize,
        job_size: usize,
        limit: Option<usize>,
        expected: Option<ConstraintViolation>,
    ) {
        let job = if job_size == 1 {
            TestSingleBuilder::default().id("job1").build_as_job_ref()
        } else {
            Job::Multi(test_multi_job_with_locations((0..job_size).map(|idx| vec![Some(idx as Location)]).collect()))
        };
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&test_fleet(), "v1")
                    .add_activities((0..activities).map(|idx| ActivityBuilder::with_location(idx).build()))
                    .build(),
            )
            .build();
        let constraint = create_activity_limit_feature("activity_limit", VIOLATION_CODE, Arc::new(move |_| limit))
            .unwrap()
            .constraint
            .unwrap();

        let result = constraint.evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

        assert_eq!(result, expected);
    }
}

mod traveling {
    use super::*;
    use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState};
    use crate::construction::features::tour_limits::create_travel_limit_feature;
    use crate::models::common::*;
    use crate::models::problem::Actor;

    const DISTANCE_CODE: ViolationCode = ViolationCode(2);
    const DURATION_CODE: ViolationCode = ViolationCode(3);

    fn create_test_data(
        vehicle_id: &str,
        target: &str,
        limit: (Option<Distance>, Option<Duration>),
    ) -> (Feature, RouteContext) {
        let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build();
        let mut state = RouteState::default();
        state.set_total_distance(50);
        state.set_total_duration(50);
        let target = target.to_owned();
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, vehicle_id).build())
            .with_state(state)
            .build();
        let transport = TestTransportCost::new_shared();
        let tour_distance_limit = Arc::new({
            let target = target.clone();
            move |actor: &Actor| {
                if get_vehicle_id(actor.vehicle.as_ref()) == target.as_str() {
                    limit.0
                } else {
                    None
                }
            }
        });
        let tour_duration_limit =
            Arc::new(
                move |actor: &Actor| {
                    if get_vehicle_id(actor.vehicle.as_ref()) == target.as_str() {
                        limit.1
                    } else {
                        None
                    }
                },
            );
        let feature = create_travel_limit_feature(
            "travel_limit",
            transport,
            DISTANCE_CODE,
            DURATION_CODE,
            tour_distance_limit,
            tour_duration_limit,
        )
        .unwrap();

        (feature, route_ctx)
    }

    parameterized_test! {can_check_traveling_limits, (vehicle, target, location, limit, expected), {
        can_check_traveling_limits_impl(vehicle, target, location, limit, expected);
    }}

    can_check_traveling_limits! {
        case01: ("v1", "v1", 76, (Some(100), None), ConstraintViolation::skip(DISTANCE_CODE)),
        case02: ("v1", "v1", 74, (Some(100), None), None),
        case03: ("v1", "v2", 76, (Some(100), None), None),

        case04: ("v1", "v1", 76, (None, Some(100)), ConstraintViolation::skip(DURATION_CODE)),
        case05: ("v1", "v1", 74, (None, Some(100)), None),
        case06: ("v1", "v2", 76, (None, Some(100)), None),
    }

    fn can_check_traveling_limits_impl(
        vehicle: &str,
        target: &str,
        location: Location,
        limit: (Option<Distance>, Option<Duration>),
        expected: Option<ConstraintViolation>,
    ) {
        let (feature, route_ctx) = create_test_data(vehicle, target, limit);

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &ActivityBuilder::with_location(50).build(),
                target: &ActivityBuilder::with_location(location).build(),
                next: Some(&ActivityBuilder::with_location(50).build()),
            },
        ));

        assert_eq!(result, expected);
    }

    #[test]
    fn can_consider_waiting_time() {
        let (feature, route_ctx) = create_test_data("v1", "v1", (None, Some(100)));

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &ActivityBuilder::with_location(50).build(),
                target: &ActivityBuilder::with_location_and_tw(75, TimeWindow::new(100, 100)).build(),
                next: Some(&ActivityBuilder::with_location(50).build()),
            },
        ));

        assert_eq!(result, ConstraintViolation::skip(DURATION_CODE));
    }
}
