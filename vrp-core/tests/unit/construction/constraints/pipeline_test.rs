use crate::construction::constraints::*;
use crate::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use crate::helpers::models::solution::{test_activity_without_job, test_actor};
use crate::models::common::Cost;
use crate::models::problem::Job;
use std::slice::Iter;
use std::sync::Arc;

struct TestConstraintModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConstraintModule for TestConstraintModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_index: usize, _job: &Job) {
        unimplemented!()
    }

    fn accept_route_state(&self, _ctx: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {
        unimplemented!()
    }

    fn merge(&self, _source: Job, _candidate: Job) -> Result<Job, i32> {
        unimplemented!()
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct TestHardActivityConstraint {
    violation: Option<ActivityConstraintViolation>,
}

struct TestSoftActivityConstraint {
    cost: Cost,
}

impl HardActivityConstraint for TestHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _route_ctx: &RouteContext,
        _activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        self.violation.clone()
    }
}

impl SoftActivityConstraint for TestSoftActivityConstraint {
    fn estimate_activity(&self, _route_ctx: &RouteContext, _activity_ctx: &ActivityContext) -> Cost {
        self.cost
    }
}

#[test]
fn can_evaluate_hard_activity_constraints() {
    let mut pipeline = ConstraintPipeline::default();
    pipeline.add_module(Arc::new(TestConstraintModule {
        state_keys: vec![1, 2],
        constraints: vec![ConstraintVariant::HardActivity(Arc::new(TestHardActivityConstraint { violation: None }))],
    }));
    pipeline.add_module(Arc::new(TestConstraintModule {
        state_keys: vec![3, 4],
        constraints: vec![ConstraintVariant::HardActivity(Arc::new(TestHardActivityConstraint {
            violation: Some(ActivityConstraintViolation { code: 5, stopped: true }),
        }))],
    }));

    let result = pipeline.evaluate_hard_activity(
        &RouteContext::new(test_actor()),
        &ActivityContext {
            index: 0,
            prev: &test_activity_without_job(),
            target: &test_activity_without_job(),
            next: None,
        },
    );

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.code, 5);
    assert!(result.stopped);
}

#[test]
fn can_estimate_hard_activity_constraints() {
    let mut pipeline = ConstraintPipeline::default();
    pipeline.add_module(Arc::new(TestConstraintModule {
        state_keys: vec![1, 2],
        constraints: vec![ConstraintVariant::SoftActivity(Arc::new(TestSoftActivityConstraint { cost: 5.0 }))],
    }));
    pipeline.add_module(Arc::new(TestConstraintModule {
        state_keys: vec![3, 4],
        constraints: vec![ConstraintVariant::SoftActivity(Arc::new(TestSoftActivityConstraint { cost: 7.0 }))],
    }));

    let result = pipeline.evaluate_soft_activity(
        &RouteContext::new(test_actor()),
        &ActivityContext {
            index: 0,
            prev: &test_activity_without_job(),
            target: &test_activity_without_job(),
            next: None,
        },
    );

    assert_eq!(result, 12.0);
}
