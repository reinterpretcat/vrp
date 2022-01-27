use crate::constraints::BreakModule;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::construction::constraints::ConstraintModule;
use vrp_core::construction::constraints::ConstraintPipeline;
use vrp_core::construction::heuristics::{RouteContext, RouteState, SolutionContext};
use vrp_core::models::common::{IdDimension, Location, ValueDimension};
use vrp_core::models::problem::Job;
use vrp_core::models::problem::Single;

fn create_single(id: &str) -> Arc<Single> {
    let mut single = create_single_with_location(Some(DEFAULT_JOB_LOCATION));
    single.dimens.set_id(id);

    Arc::new(single)
}

fn create_break(vehicled_id: &str, location: Option<Location>) -> Arc<Single> {
    let mut single = create_single_with_location(location);
    single.dimens.set_id("break");
    single.dimens.set_value("type", "break".to_string());
    single.dimens.set_value("vehicle_id", vehicled_id.to_string());
    single.dimens.set_value("shift_index", 0_usize);

    Arc::new(single)
}

parameterized_test! {can_remove_orphan_break, (break_job_loc, break_activity_loc, break_removed), {
    can_remove_orphan_break_impl(break_job_loc, break_activity_loc, break_removed);
}}

can_remove_orphan_break! {
    case01: (None, 2, true),
    case02: (None, 1, false),
    case03: (Some(2), 2, false),
}

fn can_remove_orphan_break_impl(break_job_loc: Option<Location>, break_activity_loc: Location, break_removed: bool) {
    let (transport, activity) = get_costs();
    let fleet = test_fleet();
    let mut solution_ctx = SolutionContext {
        routes: vec![RouteContext::new_with_state(
            Arc::new(create_route_with_activities(
                &fleet,
                "v1",
                vec![
                    create_activity_with_job_at_location(create_single("job1"), 1),
                    create_activity_with_job_at_location(create_break("v1", break_job_loc), break_activity_loc),
                    create_activity_with_job_at_location(create_single("job2"), 3),
                ],
            )),
            Arc::new(RouteState::default()),
        )],
        ..create_solution_context_for_fleet(&fleet)
    };

    ConstraintPipeline::default()
        .add_module(Arc::new(BreakModule::new(activity, transport, 0)))
        .accept_solution_state(&mut solution_ctx);

    if break_removed {
        assert_eq!(solution_ctx.unassigned.len(), 1);
        assert_eq!(
            solution_ctx.unassigned.iter().next().unwrap().0.to_single().dimens.get_id().unwrap().clone(),
            "break"
        );
    } else {
        assert!(solution_ctx.unassigned.is_empty());
    }
    assert!(solution_ctx.required.is_empty());
    assert_eq!(solution_ctx.routes.first().unwrap().route.tour.job_count(), (if break_removed { 2 } else { 3 }));
    assert_eq!(
        solution_ctx.routes.first().unwrap().route.tour.all_activities().len(),
        (if break_removed { 4 } else { 5 })
    );
}

parameterized_test! {can_skip_merge_breaks, (source, candidate, expected), {
    can_skip_merge_breaks_impl(Job::Single(source), Job::Single(candidate), expected);
}}

can_skip_merge_breaks! {
    case_01: (create_single("source"), create_break("v1", None), Err(0)),
    case_02: (create_break("v1", None), create_single("candidate"), Err(0)),
    case_03: (create_single("source"), create_single("candidate"), Ok(())),
}

fn can_skip_merge_breaks_impl(source: Job, candidate: Job, expected: Result<(), i32>) {
    let (transport, activity) = get_costs();
    let constraint = BreakModule::new(activity, transport, 0);

    let result = constraint.merge(source, candidate).map(|_| ());

    assert_eq!(result, expected);
}
