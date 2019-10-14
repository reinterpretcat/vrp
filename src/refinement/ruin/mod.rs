use crate::construction::states::InsertionContext;
use crate::models::Solution;
use crate::refinement::RefinementContext;

/// Specifies ruin strategy.
pub trait RuinStrategy {
    fn ruin_solution(ctx: &RefinementContext, solution: &Solution) -> InsertionContext;
}

mod adjusted_string_removal;
