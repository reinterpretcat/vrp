use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::RefinementContext;

pub trait Termination {
    fn is_termination(
        &mut self,
        refinement_ctx: &RefinementContext,
        solution: (&InsertionContext, ObjectiveCost, bool),
    ) -> bool;
}

mod max_generation;
pub use self::max_generation::MaxGeneration;

mod variation_coefficient;
pub use self::variation_coefficient::VariationCoefficient;
