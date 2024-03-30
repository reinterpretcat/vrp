use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::search::{ConfigurableRecreate, Recreate};
use crate::solver::RefinementContext;
use rosomaxa::prelude::DefaultRandom;

/// A recreate strategy which solution using nearest neighbor algorithm.
pub struct RecreateWithNearestNeighbor {
    recreate: ConfigurableRecreate,
}

impl RecreateWithNearestNeighbor {
    /// Creates a new instance of `RecreateWithNearestNeighbor`.
    pub fn new(random: DefaultRandom) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::<AllJobSelector>::default(),
                Box::<AllRouteSelector>::default(),
                LegSelection::Stochastic(random),
                ResultSelection::Concrete(Box::<BestResultSelector>::default()),
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
