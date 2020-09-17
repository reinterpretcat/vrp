use crate::format::JobIndex;
use std::sync::Arc;
use vrp_core::construction::constraints::ConstraintPipeline;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::{IdDimension, ValueDimension};
use vrp_core::utils::compare_floats;

/// Returns route modifier.
pub fn get_route_modifier(constraint: Arc<ConstraintPipeline>, job_index: JobIndex) -> RouteModifier {
    RouteModifier::new(move |route_ctx: RouteContext| {
        let actor = &route_ctx.route.actor;
        let vehicle = &actor.vehicle;

        let shift_index = vehicle.dimens.get_value::<usize>("shift_index").expect("cannot find shift index");
        let vehicle_id = vehicle.dimens.get_id().expect("cannot get vehicle id");

        let result = (1..)
            .map(|idx| format!("{}_depot_{}_{}", vehicle_id, shift_index, idx))
            .map(|job_id| job_index.get(&job_id))
            .take_while(|job| job.is_some())
            .filter_map(|job| {
                job.map(|job| evaluate_job_constraint_in_route(job, &constraint, &route_ctx, InsertionPosition::Last))
                    .and_then(|result| match result {
                        InsertionResult::Success(success) => Some(success),
                        _ => None,
                    })
            })
            .min_by(|a, b| compare_floats(a.cost, b.cost));

        if let Some(success) = result {
            let mut route_ctx = success.context;
            let route = route_ctx.route_mut();
            success.activities.into_iter().for_each(|(activity, index)| {
                route.tour.insert_at(activity, index + 1);
            });
            constraint.accept_route_state(&mut route_ctx);

            route_ctx
        } else {
            route_ctx
        }
    })
}
