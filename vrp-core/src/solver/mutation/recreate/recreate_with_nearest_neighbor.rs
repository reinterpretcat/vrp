use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::{ConfigurableRecreate, Recreate};
use crate::solver::RefinementContext;

/// A recreate strategy which solution using nearest neighbor algorithm.
pub struct RecreateWithNearestNeighbor {
    recreate: ConfigurableRecreate,
}

impl Default for RecreateWithNearestNeighbor {
    fn default() -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(AllJobSelector::default()),
                Box::new(AllRouteSelector::default()),
                Box::new(AllLegSelector::default()),
                Box::new(BestResultSelector::default()),
                InsertionHeuristic::new(Box::new(PositionInsertionEvaluator::new(InsertionPosition::Last))),
            ),
        }
    }
}

impl Recreate for RecreateWithNearestNeighbor {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
