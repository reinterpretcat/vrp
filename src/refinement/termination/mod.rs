use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::RefinementContext;
use std::sync::Arc;

pub trait Termination {
    fn is_termination(
        &self,
        refinement_ctx: &RefinementContext,
        solution: (&InsertionContext, ObjectiveCost, bool),
    ) -> bool;
}

mod max_generation;
pub use self::max_generation::MaxGeneration;
