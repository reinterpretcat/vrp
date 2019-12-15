use crate::construction::states::InsertionContext;
use crate::refinement::selection::Selection;
use crate::refinement::RefinementContext;

pub struct SelectBest {}

impl Default for SelectBest {
    fn default() -> Self {
        Self {}
    }
}

impl Selection for SelectBest {
    fn select(&self, refinement_ctx: &RefinementContext) -> InsertionContext {
        refinement_ctx.population.best().unwrap().0.deep_copy()
    }
}
