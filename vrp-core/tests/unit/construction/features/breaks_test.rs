use super::*;
use crate::construction::heuristics::RouteContext;
use crate::helpers::construction::heuristics::InsertionContextBuilder;
use crate::helpers::models::problem::SingleBuilder;
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::common::Location;
use crate::models::problem::Job;
use crate::models::problem::Single;
use std::sync::Arc;

const VIOLATION_CODE: ViolationCode = 1;

#[derive(Clone)]
struct TestBreakAspects;

impl BreakAspects for TestBreakAspects {
    fn belongs_to_route(&self, route_ctx: &RouteContext, candidate: BreakCandidate<'_>) -> bool {
        if !self.is_break_job(candidate) {
            return false;
        }

        let Some(single) = candidate.as_single() else { return false };

        let job_vehicle_id = single.dimens.get_value::<String>("vehicle_id");
        let vehicle_id = route_ctx.route().actor.vehicle.dimens.get_id();

        job_vehicle_id.zip(vehicle_id).map_or(false, |(a, b)| a == b)
    }

    fn is_break_job(&self, candidate: BreakCandidate<'_>) -> bool {
        candidate
            .as_single()
            .and_then(|break_single| break_single.dimens.get_value::<String>("type"))
            .map_or(false, |job_type| job_type == "break")
    }

    fn get_policy(&self, _: BreakCandidate<'_>) -> Option<BreakPolicy> {
        None
    }
}

fn create_single(id: &str, location: Location) -> Arc<Single> {
    SingleBuilder::default().id(id).location(Some(location)).build_shared()
}

fn create_break(vehicle_id: &str, location: Option<Location>) -> Arc<Single> {
    SingleBuilder::default()
        .id("break")
        .location(location)
        .duration(3600.)
        .property("type", "break".to_string())
        .property("vehicle_id", vehicle_id.to_string())
        .build_shared()
}

parameterized_test! {can_remove_orphan_break, (break_job_loc, break_activity_loc, break_removed), {
    can_remove_orphan_break_impl(break_job_loc, break_activity_loc, break_removed);
}}

can_remove_orphan_break! {
    case01_break_no_location_activity_at_another: (None, 2, true),
    case02_break_no_location_activity_at_prev: (None, 1, false),
    case03_break_with_location: (Some(2), 2, false),
}

fn can_remove_orphan_break_impl(break_job_loc: Option<Location>, break_activity_loc: Location, break_removed: bool) {
    let mut solution_ctx = InsertionContextBuilder::default()
        .with_routes(vec![RouteContextBuilder::default()
            .with_route(
                RouteBuilder::with_default_vehicle()
                    .add_activity(ActivityBuilder::with_location(1).job(Some(create_single("job1", 1))).build())
                    .add_activity(
                        ActivityBuilder::with_location(break_activity_loc)
                            .job(Some(create_break("v1", break_job_loc)))
                            .build(),
                    )
                    .add_activity(ActivityBuilder::with_location(3).job(Some(create_single("job2", 3))).build())
                    .build(),
            )
            .build()])
        .build()
        .solution;
    let feature = create_optional_break_feature("break", VIOLATION_CODE, TestBreakAspects).unwrap();

    feature.state.unwrap().accept_solution_state(&mut solution_ctx);

    if break_removed {
        assert_eq!(solution_ctx.unassigned.len(), 1);
        assert_eq!(solution_ctx.unassigned.iter().next().unwrap().0.dimens().get_id().unwrap().clone(), "break");
    } else {
        assert!(solution_ctx.unassigned.is_empty());
    }
    assert!(solution_ctx.required.is_empty());
    assert_eq!(solution_ctx.routes.first().unwrap().route().tour.job_count(), (if break_removed { 2 } else { 3 }));
    assert_eq!(
        solution_ctx.routes.first().unwrap().route().tour.all_activities().len(),
        (if break_removed { 4 } else { 5 })
    );
}

parameterized_test! {can_skip_merge_breaks, (source, candidate, expected), {
    can_skip_merge_breaks_impl(Job::Single(source), Job::Single(candidate), expected);
}}

can_skip_merge_breaks! {
    case_01: (create_single("source", 0), create_break("v1", None), Err(VIOLATION_CODE)),
    case_02: (create_break("v1", None), create_single("candidate", 0), Err(VIOLATION_CODE)),
    case_03: (create_single("source", 0), create_single("candidate", 1), Ok(())),
}

fn can_skip_merge_breaks_impl(source: Job, candidate: Job, expected: Result<(), i32>) {
    let constraint =
        create_optional_break_feature("break", VIOLATION_CODE, TestBreakAspects).unwrap().constraint.unwrap();

    let result = constraint.merge(source, candidate).map(|_| ());

    assert_eq!(result, expected);
}
