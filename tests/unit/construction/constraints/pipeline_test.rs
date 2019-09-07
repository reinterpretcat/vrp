use crate::construction::constraints::{
    ActivityConstraintViolation, Constraint, ConstraintPipeline, HardActivityConstraint,
    SoftActivityConstraint,
};
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::Cost;
use std::slice::Iter;
use std::sync::Arc;

//struct Constraint1 {
//    keys: Vec<i32>
//}
//
//impl Constraint for Constraint1 {
//    fn accept_route(&self, ctx: &mut RouteContext) {}
//
//    fn accept_solution(&self, ctx: &mut SolutionContext) {}
//
//    fn state_keys(&self) -> Iter<i32> {
//        self.keys.iter()
//    }
//}

struct TestConstraint {
    violation: Option<ActivityConstraintViolation>,
    cost: Cost,
    keys: Vec<i32>,
}

impl HardActivityConstraint for TestConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        self.violation.clone()
    }
}

impl SoftActivityConstraint for TestConstraint {
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Cost {
        self.cost
    }
}

impl Constraint for TestConstraint {
    fn accept_route(&self, ctx: &mut RouteContext) {}

    fn accept_solution(&self, ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }
}

#[test]
fn can_evaluate_hard_activity_constraints() {
    let mut pipeline = ConstraintPipeline::new();
    pipeline.add_hard_activity(&Arc::new(TestConstraint {
        violation: None,
        cost: 10.0,
        keys: vec![1, 2],
    }));
    pipeline.add_hard_activity(&Arc::new(TestConstraint {
        violation: Some(ActivityConstraintViolation {
            code: 5,
            stopped: false,
        }),
        cost: 10.0,
        keys: vec![1, 2],
    }));

    //pipeline.evaluate_hard_activity()
}
