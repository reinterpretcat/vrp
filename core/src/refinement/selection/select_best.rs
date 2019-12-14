use crate::construction::states::InsertionContext;
use crate::refinement::selection::Selection;
use crate::refinement::RefinementContext;

pub struct SelectBest {
    is_minimize_routes: bool,
}

impl SelectBest {
    pub fn new(is_minimize_routes: bool) -> Self {
        Self { is_minimize_routes }
    }
}

impl Selection for SelectBest {
    fn select(&self, refinement_ctx: &RefinementContext) -> InsertionContext {
        refinement_ctx.population.best(self.is_minimize_routes).unwrap().0.deep_copy()
    }
}
