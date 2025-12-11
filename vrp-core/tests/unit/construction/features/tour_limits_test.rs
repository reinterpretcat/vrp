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

mod min_activity {
    use super::*;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;

    #[test]
    fn can_create_min_activity_limit_feature() {
        let feature = create_min_activity_limit_feature("min_activity_limit", Arc::new(|_| Some(3)));
        assert!(feature.is_ok());
        let feature = feature.unwrap();
        // Now only has objective, no constraint
        assert!(feature.objective.is_some());
    }

    #[test]
    fn min_activity_objective_calculates_penalty_correctly() {
        // Route with 1 activity when minimum is 3 should have penalty of 2
        let insertion_ctx = TestInsertionContextBuilder::default()
            .with_routes(vec![
                RouteContextBuilder::default()
                    .with_route(
                        RouteBuilder::default()
                            .with_vehicle(&test_fleet(), "v1")
                            .add_activities((0..1).map(|idx| ActivityBuilder::with_location(idx).build()))
                            .build(),
                    )
                    .build(),
            ])
            .build();

        let objective = create_min_activity_limit_feature(
            "min_activity_limit",
            Arc::new(|_| Some(3)), // minimum 3, route has 1
        )
        .unwrap()
        .objective
        .unwrap();

        let fitness = objective.fitness(&insertion_ctx);

        // Penalty should be (3 - 1) = 2
        assert!((fitness - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn min_activity_objective_returns_zero_when_satisfied() {
        // Route with 3 activities when minimum is 3 should have zero penalty
        let insertion_ctx = TestInsertionContextBuilder::default()
            .with_routes(vec![
                RouteContextBuilder::default()
                    .with_route(
                        RouteBuilder::default()
                            .with_vehicle(&test_fleet(), "v1")
                            .add_activities((0..3).map(|idx| ActivityBuilder::with_location(idx).build()))
                            .build(),
                    )
                    .build(),
            ])
            .build();

        let objective = create_min_activity_limit_feature(
            "min_activity_limit",
            Arc::new(|_| Some(3)), // minimum 3, route has 3
        )
        .unwrap()
        .objective
        .unwrap();

        let fitness = objective.fitness(&insertion_ctx);

        // No penalty when constraint is satisfied
        assert!((fitness - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn min_activity_objective_ignores_empty_routes() {
        // Empty routes should not be penalized
        let insertion_ctx = TestInsertionContextBuilder::default()
            .with_routes(vec![
                RouteContextBuilder::default()
                    .with_route(RouteBuilder::default().with_vehicle(&test_fleet(), "v1").build())
                    .build(),
            ])
            .build();

        let objective =
            create_min_activity_limit_feature("min_activity_limit", Arc::new(|_| Some(3))).unwrap().objective.unwrap();

        let fitness = objective.fitness(&insertion_ctx);

        // No penalty for empty routes
        assert!((fitness - 0.0).abs() < f64::EPSILON);
    }
}

mod traveling {
    use super::*;
    use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState};
    use crate::construction::features::tour_limits::create_travel_limit_feature;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
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
        state.set_total_distance(50.);
        state.set_total_duration(50.);
        let target = target.to_owned();
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, vehicle_id).build())
            .with_state(state)
            .build();
        let tour_distance_limit = Arc::new({
            let target = target.clone();
            move |actor: &Actor| {
                if get_vehicle_id(actor.vehicle.as_ref()) == target.as_str() { limit.0 } else { None }
            }
        });
        let tour_duration_limit =
            Arc::new(
                move |actor: &Actor| {
                    if get_vehicle_id(actor.vehicle.as_ref()) == target.as_str() { limit.1 } else { None }
                },
            );
        let feature = create_travel_limit_feature(
            "travel_limit",
            TestTransportCost::new_shared(),
            TestActivityCost::new_shared(),
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
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
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
        let (feature, route_ctx) = create_test_data("v1", "v1", (None, Some(100.)));
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &ActivityContext {
                index: 0,
                prev: &ActivityBuilder::with_location(50).build(),
                target: &ActivityBuilder::with_location_and_tw(75, TimeWindow::new(100., 100.)).build(),
                next: Some(&ActivityBuilder::with_location(50).build()),
            },
        ));

        assert_eq!(result, ConstraintViolation::skip(DURATION_CODE));
    }
}

mod notify_failure {
    use super::super::*;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::helpers::models::domain::test_random;
    use crate::helpers::models::problem::*;
    use crate::models::solution::Registry;

    fn create_limit_state() -> TravelLimitState {
        TravelLimitState {
            tour_duration_limit_fn: Arc::new(|actor: &Actor| {
                let id = actor.vehicle.dimens.get_vehicle_id().unwrap();
                if actor.vehicle.dimens.get_vehicle_id().unwrap().contains("limit") {
                    println!("return Some(100): {id}");
                    Some(100.)
                } else {
                    println!("return None: {id}");
                    None
                }
            }),
            transport: TestTransportCost::new_shared(),
            activity: TestActivityCost::new_shared(),
        }
    }

    fn create_solution_context(vehicle_ids: &[&str]) -> SolutionContext {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vehicle_ids.iter().map(|id| TestVehicleBuilder::default().id(id).build()).collect())
            .build();

        TestInsertionContextBuilder::default()
            .with_registry(Registry::new(&fleet, test_random()))
            .with_fleet(Arc::new(fleet))
            .build()
            .solution
    }

    #[test]
    fn assigns_job_to_vehicle_with_duration_limit() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        // create job with time window that require departure time adjustment
        let job1 = SingleBuilder::default()
            .id("job1")
            .location(10)?
            .times(vec![TimeWindow::new(110., 200.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job1]);

        assert!(result, "should return true when successfully handling failure");
        assert_eq!(solution_ctx.routes.len(), 1, "should add one route to solution context");
        assert_eq!(solution_ctx.routes[0].route().tour[0].schedule.departure, 190., "should adjust departure time");
        Ok(())
    }

    #[test]
    fn assigns_job_to_vehicle_with_duration_limit_and_late_job_time_window() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        // create job with time window that require departure time adjustment
        let job1 = SingleBuilder::default()
            .id("job1")
            .location(10)?
            .times(vec![TimeWindow::new(110., 2000.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job1]);

        assert!(result, "should return true when successfully handling failure");
        assert_eq!(solution_ctx.routes.len(), 1, "should add one route to solution context");
        assert_eq!(solution_ctx.routes[0].route().tour[0].schedule.departure, 100., "should adjust departure time");
        Ok(())
    }

    #[test]
    fn returns_false_when_no_vehicle_with_duration_limit() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1", "v2"]);

        let job1 = SingleBuilder::default()
            .id("job1")
            .location(10)?
            .times(vec![TimeWindow::new(110., 200.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job1]);

        assert!(!result, "should return false when no vehicle with duration limit is available");
        assert_eq!(solution_ctx.routes.len(), 0, "should not add any route to solution context");
        Ok(())
    }

    #[test]
    fn returns_false_when_have_empty_routes_with_duration_limits() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit", "v2_limit"]);

        // Add a route with duration limit to solution context
        let actor = solution_ctx.registry.next_route().map(|route_ctx| route_ctx.route().actor.clone()).next().unwrap();
        let mut route_ctx = solution_ctx.registry.get_route(&actor).unwrap();
        route_ctx.state_mut().set_limit_duration(100.);
        solution_ctx.routes.push(route_ctx);

        let job1 = SingleBuilder::default()
            .id("job1")
            .location(10)?
            .times(vec![TimeWindow::new(110., 200.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[0], &[job1]);

        assert!(!result, "should return false when routes with duration limits already exist");
        assert_eq!(solution_ctx.routes.len(), 1, "should not add any additional route");
        Ok(())
    }

    #[test]
    fn returns_true_when_routes_have_no_limits() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit", "v2"]);

        // add a route without duration limit to solution context
        let actor = solution_ctx
            .registry
            .resources()
            .available()
            .find(|actor| actor.vehicle.dimens.get_vehicle_id().unwrap().contains("v2"))
            .unwrap();
        solution_ctx.routes.push(solution_ctx.registry.get_route(&actor).unwrap());

        let job1 = SingleBuilder::default()
            .id("job1")
            .location(10)?
            .times(vec![TimeWindow::new(110., 200.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[0], &[job1]);

        assert!(result, "should return true when has routes without duration limits");
        assert_eq!(solution_ctx.routes.len(), 2, "should add an additional route");
        assert_eq!(solution_ctx.registry.next_route().count(), 0, "should remove used actor from available");
        Ok(())
    }

    #[test]
    fn handles_multiple_jobs_with_different_time_windows() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        let job1 =
            SingleBuilder::default().id("job1").location(20)?.times(vec![TimeWindow::new(50., 80.)])?.build_as_job()?;

        let job2 = SingleBuilder::default()
            .id("job2")
            .location(10)?
            .times(vec![TimeWindow::new(110., 200.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job1, job2]);

        assert!(result, "should return true when successfully handling failure");
        assert_eq!(solution_ctx.routes.len(), 1, "should add one route to solution context");
        Ok(())
    }

    #[test]
    fn handles_job_with_multiple_time_windows() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        let job = SingleBuilder::default()
            .id("job1")
            .location(10)?
            .times(vec![TimeWindow::new(1100., 2000.), TimeWindow::new(110., 200.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job]);

        assert!(result, "should return true when successfully handling failure");
        assert_eq!(solution_ctx.routes.len(), 1, "should add one route to solution context");
        Ok(())
    }

    #[test]
    fn handles_jobs_with_no_time_windows() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        // Create job with no time windows
        let job = SingleBuilder::default().id("job1").location(10)?.times(vec![])?.build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job]);

        assert!(!result, "should return false when job has no time windows");
        assert!(solution_ctx.routes.is_empty(), "should not add any route to solution context");
        Ok(())
    }

    #[test]
    fn handles_jobs_with_max_time_window() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        // Create job with max time window
        let job = SingleBuilder::default().id("job1").location(10)?.times(vec![TimeWindow::max()])?.build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job]);

        assert!(!result, "should return false when job has max time window");
        assert!(solution_ctx.routes.is_empty(), "should not add any route to solution context");
        Ok(())
    }

    #[test]
    fn handles_job_outside_vehicle_shift_time() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1", "v2"]);

        // NOTE default shift time is [0, 1000]
        let job1 = SingleBuilder::default()
            .id("job1")
            .location(10)?
            .times(vec![TimeWindow::new(1100., 2000.)])?
            .build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job1]);

        assert!(!result, "should return false when job is outside vehicle shift time");
        assert_eq!(solution_ctx.routes.len(), 0, "should not add any route to solution context");
        Ok(())
    }

    #[test]
    fn handles_multi_jobs() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        // create multi job
        let single1 =
            SingleBuilder::default().id("s1").location(10)?.times(vec![TimeWindow::new(50., 80.)])?.build()?;

        let single2 =
            SingleBuilder::default().id("s2").location(20)?.times(vec![TimeWindow::new(110., 200.)])?.build()?;

        let job = MultiBuilder::default().id("job1").add_job(single1).add_job(single2).build_as_job()?;

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[job]);

        assert!(result, "should return true when successfully handling multi job");
        assert_eq!(solution_ctx.routes.len(), 1, "should add one route to solution context");
        Ok(())
    }

    #[test]
    fn handles_empty_jobs_list() -> GenericResult<()> {
        let mut solution_ctx = create_solution_context(&["v1_limit"]);

        let result = create_limit_state().notify_failure(&mut solution_ctx, &[], &[]);

        assert!(!result, "should return false with empty jobs list");
        assert_eq!(solution_ctx.routes.len(), 0, "should not add any route to solution context");
        Ok(())
    }
}
