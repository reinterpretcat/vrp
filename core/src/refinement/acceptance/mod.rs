use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::RefinementContext;

pub trait Acceptance {
    fn is_accepted(&self, refinement_ctx: &RefinementContext, solution: (&InsertionContext, ObjectiveCost)) -> bool;
}

mod greedy;
pub use self::greedy::Greedy;

mod random;
pub use self::random::SmoothRandom;
