use crate::construction::heuristics::InsertionContext;
use crate::refinement::selection::Selection;
use crate::refinement::RefinementContext;

/// Selects a weighted random solution from population.
pub struct SelectRandom {}

impl Default for SelectRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectRandom {
    /// Creates a new instance of [`SelectRandom`].
    pub fn new() -> Self {
        Self {}
    }

    fn get_index(refinement_ctx: &RefinementContext) -> usize {
        let size = refinement_ctx.population.size() as i32;
        let weights: Vec<_> =
            (0..size).rev().map(|idx| (std::f64::consts::E.powi(idx) * 100.).round() as usize).collect();

        let (insertion_ctx, _, _) = refinement_ctx.population.best().unwrap();

        // NOTE random weighted
        insertion_ctx.random.weighted(weights.iter()) as usize
    }
}

impl Selection for SelectRandom {
    fn select(&self, refinement_ctx: &mut RefinementContext) -> InsertionContext {
        let index = Self::get_index(refinement_ctx);

        refinement_ctx.population.all().skip(index as usize).next().unwrap().0.deep_copy()
    }
}
