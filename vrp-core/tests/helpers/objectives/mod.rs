use crate::construction::states::InsertionContext;
use crate::refinement::objectives::{Objective, ObjectiveCost};
use crate::refinement::RefinementContext;

pub struct TestObjective {}

impl Objective for TestObjective {
    fn estimate(
        &self,
        _refinement_ctx: &mut RefinementContext,
        _insertion_ctx: &InsertionContext,
    ) -> Box<dyn ObjectiveCost + Send + Sync> {
        unimplemented!()
    }
}

impl TestObjective {
    pub fn new() -> Self {
        Self {}
    }
}
