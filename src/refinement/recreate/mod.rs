use crate::construction::heuristics::ResultSelector;
use crate::construction::states::{InsertionContext, InsertionResult};
use std::slice::Iter;

pub trait Recreate {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// Selects best result.
struct BestResultSelector {}

impl Default for BestResultSelector {
    fn default() -> Self {
        Self {}
    }
}

impl ResultSelector for BestResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
    }
}

mod recreate_with_cheapest;

pub use self::recreate_with_cheapest::RecreateWithCheapest;

mod recreate_with_gaps;

pub use self::recreate_with_gaps::RecreateWithGaps;

mod recreate_with_blinks;

/// Provides the way to run one of multiple recreate methods.
pub struct CompositeRecreate {
    recreates: Vec<Box<dyn Recreate>>,
    weights: Vec<usize>,
}

impl Default for CompositeRecreate {
    fn default() -> Self {
        Self::new(vec![(Box::new(RecreateWithCheapest::default()), 1000), (Box::new(RecreateWithGaps::default()), 1)])
    }
}

impl CompositeRecreate {
    fn new(recreates: Vec<(Box<dyn Recreate>, usize)>) -> Self {
        let weights = recreates.iter().map(|(_, weight)| *weight).collect();
        Self { recreates: recreates.into_iter().map(|(recreate, _)| recreate).collect(), weights }
    }
}

impl Recreate for CompositeRecreate {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.random.weighted(self.weights.iter());
        self.recreates.get(index).unwrap().run(insertion_ctx)
    }
}
