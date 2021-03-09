use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::mutation::ConfigurableRecreate;
use crate::solver::RefinementContext;

/// A recreate method which is equivalent to cheapest insertion heuristic.
pub struct RecreateWithCheapest {
    recreate: ConfigurableRecreate,
}

impl Default for RecreateWithCheapest {
    fn default() -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(AllJobSelector::default()),
                Box::new(AllRouteSelector::default()),
                Box::new(BestResultSelector::default()),
                Default::default(),
            ),
        }
    }
}

impl Recreate for RecreateWithCheapest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
