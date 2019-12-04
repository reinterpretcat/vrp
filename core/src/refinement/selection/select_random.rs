use crate::construction::states::InsertionContext;
use crate::refinement::selection::Selection;
use crate::refinement::RefinementContext;

pub struct SelectRandom {}

impl Default for SelectRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectRandom {
    pub fn new() -> Self {
        Self {}
    }

    fn get_index(refinement_ctx: &RefinementContext) -> usize {
        let size = refinement_ctx.population.size() as i32;
        let (insertion_ctx, _, _) = refinement_ctx.population.all().next().unwrap();

        insertion_ctx.random.uniform_int(0, size - 1) as usize
    }
}

impl Selection for SelectRandom {
    fn select(&self, refinement_ctx: &RefinementContext) -> InsertionContext {
        let index = Self::get_index(refinement_ctx);

        refinement_ctx.population.all().skip(index as usize).next().unwrap().0.deep_copy()
    }
}
