use crate::refinement::termination::Termination;
use crate::models::common::ObjectiveCost;
use crate::refinement::RefinementContext;
use crate::construction::states::InsertionContext;

pub struct VariationCoefficient {}

impl Termination for VariationCoefficient {
    fn is_termination(&mut self, refinement_ctx: &RefinementContext, solution: (&InsertionContext, ObjectiveCost, bool)) -> bool {
        unimplemented!()
    }
}