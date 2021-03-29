use crate::construction::constraints::TransportConstraintModule;
use crate::construction::heuristics::InsertionContext;
use crate::solver::mutation::LocalOperator;
use crate::solver::RefinementContext;

/// Reschedules departure time of the routes in the solution.
pub struct RescheduleDeparture {}

impl Default for RescheduleDeparture {
    fn default() -> Self {
        Self {}
    }
}

impl LocalOperator for RescheduleDeparture {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        let transport = refinement_ctx.problem.transport.as_ref();

        // TODO optionally, optimize only subset of the routes.

        let mut insertion_ctx = insertion_ctx.deep_copy();
        insertion_ctx.solution.routes.iter_mut().for_each(|route_ctx| {
            TransportConstraintModule::optimize_departure_time(route_ctx, transport);
        });

        // TODO check is_stale flag and return None

        refinement_ctx.problem.constraint.accept_solution_state(&mut insertion_ctx.solution);

        Some(insertion_ctx)
    }
}
