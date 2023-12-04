use crate::construction::enablers::VehicleTie;
use crate::format::JobIndex;
use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::GoalContext;

/// Returns route modifier.
pub fn get_route_modifier(goal: Arc<GoalContext>, job_index: Arc<JobIndex>) -> RouteModifier {
    RouteModifier::new(move |route_ctx: RouteContext| {
        let actor = &route_ctx.route().actor;
        let vehicle = &actor.vehicle;

        let shift_index = vehicle.dimens.get_shift_index().expect("cannot find shift index");
        let vehicle_id = vehicle.dimens.get_vehicle_id().expect("cannot get vehicle id");

        let candidates = (1..)
            .map(|idx| format!("{vehicle_id}_dispatch_{shift_index}_{idx}"))
            .map(|job_id| job_index.get(&job_id))
            .take_while(|job| job.is_some())
            .collect::<Vec<_>>();

        let leg_selection = LegSelection::Exhaustive;
        let result_selector = BestResultSelector::default();

        let result = candidates
            .iter()
            .filter_map(|job| {
                job.map(|job| {
                    let eval_ctx = EvaluationContext {
                        goal: goal.as_ref(),
                        job,
                        leg_selection: &leg_selection,
                        result_selector: &result_selector,
                    };

                    eval_job_constraint_in_route(
                        &eval_ctx,
                        &route_ctx,
                        InsertionPosition::Last,
                        InsertionCost::default(),
                        None,
                    )
                })
                .and_then(|result| match result {
                    InsertionResult::Success(success) => Some(success),
                    _ => None,
                })
            })
            .min_by(|a, b| a.cost.cmp(&b.cost));

        if let Some(success) = result {
            let mut route_ctx = route_ctx.deep_copy();
            let route = route_ctx.route_mut();
            success.activities.into_iter().for_each(|(activity, index)| {
                route.tour.insert_at(activity, index + 1);
            });
            goal.accept_route_state(&mut route_ctx);

            route_ctx
        } else {
            let mut route_ctx = route_ctx;

            if !candidates.is_empty() {
                route_ctx.state_mut().set_flag(state_flags::UNASSIGNABLE);
            }

            route_ctx
        }
    })
}
