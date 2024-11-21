use crate::models::common::FootprintSolutionState;
use crate::models::FeatureObjective;
use crate::prelude::*;

custom_solution_state!(FootprintCost typeof Cost);

/// Creates a feature to penalize edges/transitions seen in multiple solutions.
pub fn create_known_edge_feature(name: &str) -> Result<Feature, GenericError> {
    FeatureBuilder::default().with_name(name).with_objective(KnownEdgeObjective).with_state(KnownEdgeState).build()
}

struct KnownEdgeObjective;

impl FeatureObjective for KnownEdgeObjective {
    fn fitness(&self, insertion_ctx: &InsertionContext) -> Cost {
        insertion_ctx.solution.state.get_footprint_cost().copied().unwrap_or_else(|| {
            insertion_ctx
                .solution
                .state
                .get_footprint()
                .map_or(Cost::default(), |footprint| footprint.estimate_solution(&insertion_ctx.solution) as Cost)
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { .. } => Cost::default(),
            MoveContext::Activity { solution_ctx, activity_ctx, .. } => {
                solution_ctx.state.get_footprint().map_or(Cost::default(), |footprint| {
                    let prev = activity_ctx.prev.place.location;
                    let target = activity_ctx.target.place.location;

                    (footprint.estimate_edge(prev, target)
                        + activity_ctx
                            .next
                            .as_ref()
                            .map_or(0, |next| footprint.estimate_edge(target, next.place.location)))
                        as Cost
                })
            }
        }
    }
}

struct KnownEdgeState;

impl FeatureState for KnownEdgeState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        if let Some(footprint) = solution_ctx.state.get_footprint() {
            let cost = footprint.estimate_solution(solution_ctx);
            solution_ctx.state.set_footprint_cost(cost as Cost);
        }
    }
}
