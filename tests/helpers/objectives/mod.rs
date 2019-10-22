use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::objectives::Objective;

pub struct TestObjective {}

impl Objective for TestObjective {
    fn estimate(&self, _insertion_ctx: &InsertionContext) -> ObjectiveCost {
        unimplemented!()
    }
}

impl TestObjective {
    pub fn new() -> Self {
        Self {}
    }
}
