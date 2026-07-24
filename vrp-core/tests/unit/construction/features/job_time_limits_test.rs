use crate::construction::features::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Schedule, TimeWindow};
use crate::models::problem::{JobTimeConstraints, JobTimeConstraintsDimension};
use crate::models::solution::{Activity, Place};

const VIOLATION_CODE: ViolationCode = ViolationCode(1);

fn create_feature() -> Feature {
    create_job_time_limits_feature(
        "job_time_limits",
        TestTransportCost::new_shared(),
        TestActivityCost::new_shared(),
        VIOLATION_CODE,
    )
    .unwrap()
}

fn create_fleet_with_job_time_constraints(id: &str, earliest_first: Option<f64>, latest_last: Option<f64>) -> Fleet {
    let mut builder = TestVehicleBuilder::default();
    builder.id(id);
    builder.dimens_mut().set_job_time_constraints(JobTimeConstraints { earliest_first, latest_last });

    FleetBuilder::default().add_driver(test_driver()).add_vehicle(builder.build()).build()
}

/// Creates a depot-like activity (no job) for testing
fn create_depot_activity(location: usize, departure: f64) -> Activity {
    Activity {
        place: Place { idx: 0, location, duration: 0.0, time: TimeWindow::new(0.0, 1000.0) },
        schedule: Schedule::new(departure, departure),
        job: None,
        commute: None,
    }
}

mod earliest_first_constraint {
    use super::*;

    #[test]
    fn allows_job_when_arrival_is_after_earliest_first() {
        // Vehicle can depart at 0, earliest_first is 5
        // Job at location 10 means arrival at 10 (distance = time)
        // 10 > 5, so should be allowed
        let fleet = create_fleet_with_job_time_constraints("v1", Some(5.0), None);
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(10, TimeWindow::new(0.0, 100.0)).build(),
                next: Some(&create_depot_activity(0, 20.0)), // End depot
            },
        ));

        assert_eq!(result, None);
    }

    #[test]
    fn allows_job_when_can_wait_until_earliest_first() {
        // Vehicle departs at 0, earliest_first is 15
        // Job at location 10 means arrival at 10
        // 10 < 15, but job time window extends to 100, so can wait
        let fleet = create_fleet_with_job_time_constraints("v1", Some(15.0), None);
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(10, TimeWindow::new(0.0, 100.0)).build(),
                next: Some(&create_depot_activity(0, 30.0)), // End depot
            },
        ));

        assert_eq!(result, None);
    }

    #[test]
    fn rejects_job_when_time_window_ends_before_earliest_first() {
        // Vehicle departs at 0, earliest_first is 15
        // Job at location 10 means arrival at 10
        // Job time window ends at 12, which is before earliest_first (15)
        // Cannot wait until earliest_first
        let fleet = create_fleet_with_job_time_constraints("v1", Some(15.0), None);
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(10, TimeWindow::new(0.0, 12.0)).build(),
                next: Some(&create_depot_activity(0, 30.0)), // End depot
            },
        ));

        assert_eq!(result, ConstraintViolation::skip(VIOLATION_CODE));
    }
}

mod latest_last_constraint {
    use super::*;

    #[test]
    fn allows_job_when_departure_is_before_latest_last() {
        // Job at location 10, service time is default (0 from TestActivityCost)
        // Arrival at 10, departure at 10
        // latest_last is 20, so 10 <= 20, should be allowed
        let fleet = create_fleet_with_job_time_constraints("v1", None, Some(20.0));
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(10, TimeWindow::new(0.0, 100.0)).build(),
                next: None, // Inserting at the end (will be last job)
            },
        ));

        assert_eq!(result, None);
    }

    #[test]
    fn rejects_job_when_departure_exceeds_latest_last() {
        // Job at location 50, arrival at 50, departure at 50 (no service time)
        // latest_last is 20, so departure 50 > 20, should be rejected
        let fleet = create_fleet_with_job_time_constraints("v1", None, Some(20.0));
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(50, TimeWindow::new(0.0, 100.0)).build(),
                next: None, // Inserting at the end (will be last job)
            },
        ));

        assert_eq!(result, ConstraintViolation::skip(VIOLATION_CODE));
    }

    #[test]
    fn rejects_job_when_duration_causes_departure_after_latest_last() {
        // Job at location 10, arrival at 10 (which is BEFORE latest_last of 15)
        // Service duration is 10, so departure = 10 + 10 = 20
        // latest_last is 15, so departure 20 > 15, should be rejected
        let fleet = create_fleet_with_job_time_constraints("v1", None, Some(15.0));
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                // Arrival at 10, duration 10, departure at 20
                target: &ActivityBuilder::with_location_tw_and_duration(10, TimeWindow::new(0.0, 100.0), 10.0).build(),
                next: None, // Inserting at the end (will be last job)
            },
        ));

        assert_eq!(result, ConstraintViolation::skip(VIOLATION_CODE));
    }

    #[test]
    fn does_not_apply_when_inserting_before_another_job() {
        // When inserting before another job, latest_last doesn't apply to the inserted job
        // because it won't be the last job
        let fleet = create_fleet_with_job_time_constraints("v1", None, Some(20.0));
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(50, TimeWindow::new(0.0, 100.0)).build(),
                // Next is another job activity (has job), so latest_last doesn't apply to target
                next: Some(&ActivityBuilder::with_location_and_tw(60, TimeWindow::new(0.0, 100.0)).build()),
            },
        ));

        // Should pass because this is not the last job (next is a job, not a depot)
        assert_eq!(result, None);
    }
}

mod combined_constraints {
    use super::*;

    #[test]
    fn applies_both_constraints_for_single_job_route() {
        // When there's only one job, it's both first and last
        // earliest_first = 5, latest_last = 20
        // Job at location 10, arrival = 10 (>5), departure = 10 (<20)
        let fleet = create_fleet_with_job_time_constraints("v1", Some(5.0), Some(20.0));
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(10, TimeWindow::new(0.0, 100.0)).build(),
                next: None, // Last job (also first job since only one)
            },
        ));

        assert_eq!(result, None);
    }

    #[test]
    fn rejects_when_earliest_first_violated_even_with_valid_latest_last() {
        // earliest_first = 15, but job time window ends at 12
        // Arrival at 10, cannot wait until 15 because TW ends at 12
        let fleet = create_fleet_with_job_time_constraints("v1", Some(15.0), Some(100.0));
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(10, TimeWindow::new(0.0, 12.0)).build(),
                next: None,
            },
        ));

        assert_eq!(result, ConstraintViolation::skip(VIOLATION_CODE));
    }
}

mod no_constraints {
    use super::*;

    #[test]
    fn allows_any_job_when_no_constraints_set() {
        // Vehicle has job time constraints dimension but both are None
        let fleet = create_fleet_with_job_time_constraints("v1", None, None);
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(100, TimeWindow::new(0.0, 5.0)).build(),
                next: None,
            },
        ));

        assert_eq!(result, None);
    }

    #[test]
    fn allows_any_job_when_vehicle_has_no_dimension() {
        // Use standard fleet without job time constraints dimension
        let fleet = test_fleet();
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let feature = create_feature();

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &create_depot_activity(0, 0.0), // Start depot
                target: &ActivityBuilder::with_location_and_tw(100, TimeWindow::new(0.0, 5.0)).build(),
                next: None,
            },
        ));

        assert_eq!(result, None);
    }
}
