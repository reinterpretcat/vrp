use crate::construction::heuristics::ResultSelector;
use crate::construction::states::{InsertionContext, InsertionResult};

pub trait Recreate {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// Selects best result.
struct BestResultSelector {}

impl Default for BestResultSelector {
    fn default() -> Self {
        Self {}
    }
}

impl ResultSelector for BestResultSelector {
    fn select(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
    }
}

mod recreate_with_cheapest;

pub use self::recreate_with_cheapest::RecreateWithCheapest;

mod recreate_with_gaps;

pub use self::recreate_with_gaps::RecreateWithGaps;
use crate::refinement::recreate::recreate_with_blinks::RecreateWithBlinks;
use crate::refinement::RefinementContext;

mod recreate_with_blinks;

/// Provides the way to run one of multiple recreate methods.
pub struct CompositeRecreate {
    recreates: Vec<Box<dyn Recreate>>,
    weights: Vec<usize>,
}

impl Default for CompositeRecreate {
    fn default() -> Self {
        Self::new(vec![
            (Box::new(RecreateWithBlinks::<i32>::default()), 2),
            (Box::new(RecreateWithCheapest::default()), 10),
            (Box::new(RecreateWithGaps::default()), 1),
        ])
    }
}

impl CompositeRecreate {
    pub fn new(recreates: Vec<(Box<dyn Recreate>, usize)>) -> Self {
        let weights = recreates.iter().map(|(_, weight)| *weight).collect();
        Self { recreates: recreates.into_iter().map(|(recreate, _)| recreate).collect(), weights }
    }
}

impl Recreate for CompositeRecreate {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.random.weighted(self.weights.iter());
        self.recreates.get(index).unwrap().run(refinement_ctx, insertion_ctx)
    }
}
