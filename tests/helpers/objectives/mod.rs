use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};
use crate::refinement::objectives::Objective;

pub struct TestObjective {}

impl Objective for TestObjective {
    fn estimate(&self, problem: &Problem, solution: &Solution) -> ObjectiveCost {
        unimplemented!()
    }
}

impl TestObjective {
    pub fn new() -> Self {
        Self {}
    }
}
