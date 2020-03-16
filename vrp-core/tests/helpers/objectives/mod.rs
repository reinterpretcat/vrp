use crate::construction::states::InsertionContext;
use crate::refinement::objectives::{Objective, ObjectiveCost};
use crate::refinement::RefinementContext;

pub struct TestObjective {}

impl Objective for TestObjective {
    fn estimate_cost(&self, _: &mut RefinementContext, _: &InsertionContext) -> Box<dyn ObjectiveCost + Send + Sync> {
        unimplemented!()
    }

    fn is_goal_satisfied(&self, _: &mut RefinementContext, _: &InsertionContext) -> Option<bool> {
        None
    }
}

impl TestObjective {
    pub fn new() -> Self {
        Self {}
    }
}
