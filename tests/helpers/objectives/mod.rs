use crate::objectives::ObjectiveFunction;

pub struct TestObjectiveFunction {}

impl ObjectiveFunction for TestObjectiveFunction {}

impl TestObjectiveFunction {
    pub fn new() -> Self {
        Self {}
    }
}
