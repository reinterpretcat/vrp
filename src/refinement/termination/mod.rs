use crate::models::common::ObjectiveCost;
use crate::models::Solution;
use crate::refinement::RefinementContext;
use std::sync::Arc;

pub trait TerminationCriteria {
    fn is_termination(
        &self,
        refinement_ctx: &RefinementContext,
        solution: (Arc<Solution>, ObjectiveCost, bool),
    ) -> bool;
}

mod max_generation;
