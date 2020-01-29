use crate::construction::states::InsertionContext;
use crate::refinement::selection::Selection;
use crate::refinement::RefinementContext;

/// Selects a best solution from population.
pub struct SelectBest {}

impl Default for SelectBest {
    fn default() -> Self {
        Self {}
    }
}

impl Selection for SelectBest {
    fn select(&self, refinement_ctx: &mut RefinementContext) -> InsertionContext {
        refinement_ctx.population.best().unwrap().0.deep_copy()
    }
}
