use crate::construction::features::*;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::Location;
use crate::models::problem::Job;
use std::sync::Arc;

mod activity {
    use super::*;

    const VIOLATION_CODE: ViolationCode = 1;

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
            Job::Single(test_single_with_id("job1"))
        } else {
            Job::Multi(test_multi_job_with_locations((0..job_size).map(|idx| vec![Some(idx as Location)]).collect()))
        };
        let solution_ctx = create_empty_solution_context();
        let route_ctx = create_route_context_with_activities(
            &test_fleet(),
            "v1",
            (0..activities).map(|idx| test_activity_with_location(idx as Location)).collect(),
        );
        let constraint =
            create_activity_limit_feature(VIOLATION_CODE, Arc::new(move |_| limit)).unwrap().constraint.unwrap();

        let result = constraint.evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

        assert_eq!(result, expected);
    }
}

mod traveling {
    use super::*;
    use crate::construction::features::tour_limits::create_travel_limit_feature;
    use crate::models::common::*;
    use crate::models::problem::Actor;

    const DISTANCE_CODE: ViolationCode = 2;
    const DURATION_CODE: ViolationCode = 3;

    fn create_test_data(
        vehicle: &str,
        target: &str,
        limit: (Option<Distance>, Option<Duration>),
    ) -> (Feature, RouteContext) {
        let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build();
        let mut state = RouteState::default();
        state.put_route_state(TOTAL_DISTANCE_KEY, 50.);
        state.put_route_state(TOTAL_DURATION_KEY, 50.);
        let target = target.to_owned();
        let route_ctx = RouteContext::new_with_state(
            Arc::new(create_route_with_activities(&fleet, vehicle, vec![])),
            Arc::new(state),
        );
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
            tour_distance_limit,
            tour_duration_limit,
            DISTANCE_CODE,
            DURATION_CODE,
        )
        .unwrap();

        (feature, route_ctx)
    }

    parameterized_test! {can_check_traveling_limits, (vehicle, target, location, limit, expected), {
        can_check_traveling_limits_impl(vehicle, target, location, limit, expected);
    }}

    can_check_traveling_limits! {
        case01: ("v1", "v1", 76, (Some(100.), None), ConstraintViolation::skip(DISTANCE_CODE)),
        case02: ("v1", "v1", 74, (Some(100.), None), None),
        case03: ("v1", "v2", 76, (Some(100.), None), None),

        case04: ("v1", "v1", 76, (None, Some(100.)), ConstraintViolation::skip(DURATION_CODE)),
        case05: ("v1", "v1", 74, (None, Some(100.)), None),
        case06: ("v1", "v2", 76, (None, Some(100.)), None),
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
                prev: &test_activity_with_location(50),
                target: &test_activity_with_location(location),
                next: Some(&test_activity_with_location(50)),
            },
        ));

        assert_eq!(result, expected);
    }

    #[test]
    fn can_consider_waiting_time() {
        let (feature, route_ctx) = create_test_data("v1", "v1", (None, Some(100.)));

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &test_activity_with_location(50),
                target: &test_activity_with_location_and_tw(75, TimeWindow::new(100., 100.)),
                next: Some(&test_activity_with_location(50)),
            },
        ));

        assert_eq!(result, ConstraintViolation::skip(DURATION_CODE));
    }
}
