use crate::construction::extensions::{advance_departure_time, recede_departure_time};
use crate::construction::heuristics::InsertionContext;
use crate::models::solution::Activity;
use crate::solver::search::LocalOperator;
use crate::solver::RefinementContext;
use rosomaxa::prelude::*;
use std::cmp::Ordering;

/// Reschedules departure time of the routes in the solution.
#[derive(Default)]
pub struct RescheduleDeparture {}

impl LocalOperator for RescheduleDeparture {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        let activity = refinement_ctx.problem.activity.as_ref();
        let transport = refinement_ctx.problem.transport.as_ref();

        // TODO optionally, optimize only subset of the routes.

        let random = insertion_ctx.environment.random.clone();

        let mut insertion_ctx = insertion_ctx.deep_copy();
        insertion_ctx.solution.routes.iter_mut().for_each(|route_ctx| {
            let earliest = route_ctx.route.actor.detail.start.as_ref().and_then(|start| start.time.earliest);

            match (route_ctx.route.tour.start(), earliest, random.is_head_not_tails()) {
                (Some(start), Some(earliest), true) if can_recede_departure(start, earliest) => {
                    recede_departure_time(route_ctx, activity, transport)
                }
                _ => advance_departure_time(route_ctx, activity, transport, true),
            };
        });

        // TODO check is_stale flag and return None

        refinement_ctx.problem.constraint.accept_solution_state(&mut insertion_ctx.solution);

        Some(insertion_ctx)
    }
}

fn can_recede_departure(start: &Activity, earliest: f64) -> bool {
    compare_floats(start.schedule.departure, earliest) != Ordering::Equal
}
