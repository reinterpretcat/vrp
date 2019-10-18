use crate::models::common::ObjectiveCost;
use crate::models::Solution;
use crate::refinement::RefinementContext;
use std::sync::Arc;

pub trait Acceptance {
    fn is_accepted(&self, refinement_ctx: &RefinementContext, solution: (Arc<Solution>, ObjectiveCost)) -> bool;
}

mod greedy;
pub use self::greedy::Greedy;
