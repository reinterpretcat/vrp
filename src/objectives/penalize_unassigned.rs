use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};
use crate::objectives::ObjectiveFunction;

pub struct PenalizeUnassigned {}

impl PenalizeUnassigned {
    pub fn new() -> Self {
        Self {}
    }
}

impl ObjectiveFunction for PenalizeUnassigned {
    fn estimate(&self, problem: &Problem, solution: &Solution) -> ObjectiveCost {
        unimplemented!()
    }
}
