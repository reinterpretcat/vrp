use crate::construction::constraints::{RouteConstraintViolation, TourSizeModule};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::{test_fleet, test_multi_job_with_locations, test_single_with_id};
use crate::helpers::models::solution::{create_route_context_with_activities, test_activity_with_location};
use crate::models::common::Location;
use crate::models::problem::Job;
use std::sync::Arc;

fn fail() -> Option<RouteConstraintViolation> {
    Some(RouteConstraintViolation { code: 1 })
}

parameterized_test! {can_limit_by_job_activities, (activities, job_size, limit, expected), {
    can_limit_by_job_activities_impl(activities, job_size, limit, expected);
}}

can_limit_by_job_activities! {
    case01: (3, 1, Some(3), fail()),
    case02: (3, 1, None, None),
    case03: (2, 1, Some(3), None),

    case04: (2, 2, Some(3), fail()),
    case05: (2, 2, None, None),
    case06: (1, 2, Some(3), None),
}

fn can_limit_by_job_activities_impl(
    activities: usize,
    job_size: usize,
    limit: Option<usize>,
    expected: Option<RouteConstraintViolation>,
) {
    let job = if job_size == 1 {
        Job::Single(test_single_with_id("job1"))
    } else {
        Job::Multi(test_multi_job_with_locations((0..job_size).map(|idx| vec![Some(idx as Location)]).collect()))
    };
    let route_ctx = create_route_context_with_activities(
        &test_fleet(),
        "v1",
        (0..activities).map(|idx| test_activity_with_location(idx as Location)).collect(),
    );

    let result = create_constraint_pipeline_with_module(Arc::new(TourSizeModule::new(Arc::new(move |_| limit), 1)))
        .evaluate_hard_route(&create_empty_solution_context(), &route_ctx, &job);

    assert_eq!(result, expected);
}
