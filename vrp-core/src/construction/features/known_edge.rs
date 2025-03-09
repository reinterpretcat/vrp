use crate::models::FeatureObjective;
use crate::models::common::FootprintSolutionState;
use crate::prelude::*;

custom_solution_state!(FootprintCost typeof Cost);

/// Creates a feature to penalize edges/transitions seen in multiple solutions.
/// This feature acts as a heuristic addition.
pub fn create_known_edge_feature(name: &str, keep_solution_fitness: bool) -> Result<Feature, GenericError> {
    if keep_solution_fitness {
        FeatureBuilder::default()
            .with_name(name)
            .with_objective(KnownEdgeObjective { keep_solution_fitness })
            .with_state(KnownEdgeState)
            .build()
    } else {
        FeatureBuilder::default().with_name(name).with_objective(KnownEdgeObjective { keep_solution_fitness }).build()
    }
}

struct KnownEdgeObjective {
    keep_solution_fitness: bool,
}

impl FeatureObjective for KnownEdgeObjective {
    fn fitness(&self, insertion_ctx: &InsertionContext) -> Cost {
        if !self.keep_solution_fitness {
            return Cost::default();
        }

        insertion_ctx.solution.state.get_footprint_cost().copied().unwrap_or_else(|| {
            debug_assert!(self.keep_solution_fitness);

            // NOTE: use sqrt/round to reduce sensitivity on solution fitness perturbations
            insertion_ctx
                .solution
                .state
                .get_footprint()
                .map_or(Cost::default(), |footprint| footprint.estimate_solution(&insertion_ctx.solution) as Cost)
                .sqrt()
                .round()
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { .. } => Cost::default(),
            MoveContext::Activity { solution_ctx, activity_ctx, .. } => {
                solution_ctx.state.get_footprint().map_or(Cost::default(), |footprint| {
                    let prev = activity_ctx.prev.place.location;
                    let target = activity_ctx.target.place.location;

                    // NOTE: scale down values to reduce sensitivity on solution fitness perturbations
                    let prev_to_target = (footprint.estimate_edge(prev, target) as Cost).sqrt().round();
                    let target_to_next = activity_ctx
                        .next
                        .as_ref()
                        .map_or(0., |next| footprint.estimate_edge(target, next.place.location) as Cost)
                        .sqrt()
                        .round();

                    prev_to_target + target_to_next
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
