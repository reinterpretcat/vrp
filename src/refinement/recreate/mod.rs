use crate::construction::heuristics::ResultSelector;
use crate::construction::states::{InsertionContext, InsertionResult};

pub trait Recreate {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// Selects best result.
struct BestResultSelector {}

impl ResultSelector for BestResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
    }
}

mod recreate_with_cheapest;
pub use self::recreate_with_cheapest::RecreateWithCheapest;

mod recreate_with_gaps;
pub use self::recreate_with_gaps::RecreateWithGaps;
