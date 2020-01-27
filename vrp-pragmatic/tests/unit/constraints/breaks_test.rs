use crate::constraints::BreakModule;
use crate::extensions::create_typed_actor_groups;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::construction::constraints::ConstraintPipeline;
use vrp_core::construction::states::{RouteContext, RouteState, SolutionContext};
use vrp_core::models::common::{IdDimension, Location};
use vrp_core::models::problem::{Fleet, SimpleActivityCost, Single};
use vrp_core::models::solution::Registry;

fn create_single(id: &str) -> Arc<Single> {
    let mut single = create_single_with_location(Some(DEFAULT_JOB_LOCATION));
    single.dimens.set_id(id);

    Arc::new(single)
}

fn create_break(vehicled_id: &str, location: Option<Location>) -> Arc<Single> {
    let mut single = create_single_with_location(location);
    single.dimens.set_id("break");
    single.dimens.insert("type".to_string(), Arc::new("break".to_string()));
    single.dimens.insert("vehicle_id".to_string(), Arc::new(vehicled_id.to_string()));

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
    let activity = Arc::new(SimpleActivityCost::default());
    let transport = Arc::new(TestTransportCost::new());
    let fleet = Fleet::new(
        vec![Arc::new(test_driver())],
        vec![Arc::new(test_vehicle("v1"))],
        Box::new(|actors| create_typed_actor_groups(actors)),
    );
    let mut solution_ctx = SolutionContext {
        required: vec![],
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes: vec![RouteContext {
            route: Arc::new(create_route_with_activities(
                &fleet,
                "v1",
                vec![
                    create_activity_with_job_at_location(create_single("job1"), 1),
                    create_activity_with_job_at_location(create_break("v1", break_job_loc), break_activity_loc),
                    create_activity_with_job_at_location(create_single("job2"), 3),
                ],
            )),
            state: Arc::new(RouteState::default()),
        }],
        registry: Registry::new(&fleet),
    };

    ConstraintPipeline::default()
        .add_module(Box::new(BreakModule::new(activity, transport, 0, None, false)))
        .accept_solution_state(&mut solution_ctx);

    if break_removed {
        assert_eq!(solution_ctx.required.len(), 1);
        assert_eq!(solution_ctx.required.first().unwrap().to_single().dimens.get_id().unwrap().clone(), "break");
    } else {
        assert!(solution_ctx.required.is_empty());
    }
    assert_eq!(solution_ctx.routes.first().unwrap().route.tour.job_count(), (if break_removed { 2 } else { 3 }));
    assert_eq!(
        solution_ctx.routes.first().unwrap().route.tour.all_activities().len(),
        (if break_removed { 4 } else { 5 })
    );
}
