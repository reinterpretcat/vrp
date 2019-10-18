use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};
use crate::refinement::objectives::ObjectiveFunction;

pub struct TestObjectiveFunction {}

impl ObjectiveFunction for TestObjectiveFunction {
    fn estimate(&self, problem: &Problem, solution: &Solution) -> ObjectiveCost {
        unimplemented!()
    }
}

impl TestObjectiveFunction {
    pub fn new() -> Self {
        Self {}
    }
}
