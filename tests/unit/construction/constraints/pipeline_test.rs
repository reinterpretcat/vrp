use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::helpers::models::solution::{test_actor, test_tour_activity_without_job};
use crate::models::common::Cost;
use std::slice::Iter;
use std::sync::Arc;

struct TestConstraintModule {
    state_keys: Vec<i32>,
    hard_route_constraint: Option<Arc<dyn HardRouteConstraint>>,
    hard_activity_constraint: Option<Arc<dyn HardActivityConstraint>>,
    soft_route_constraint: Option<Arc<dyn SoftRouteConstraint>>,
    soft_activity_constraint: Option<Arc<dyn SoftActivityConstraint>>,
}

impl ConstraintModule for TestConstraintModule {
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        unimplemented!()
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_hard_route_constraint(&self) -> Option<Arc<dyn HardRouteConstraint>> {
        self.hard_route_constraint.clone()
    }

    fn get_hard_activity_constraint(&self) -> Option<Arc<dyn HardActivityConstraint>> {
        self.hard_activity_constraint.clone()
    }

    fn get_soft_route_constraint(&self) -> Option<Arc<dyn SoftRouteConstraint>> {
        self.soft_route_constraint.clone()
    }

    fn get_soft_activity_constraint(&self) -> Option<Arc<dyn SoftActivityConstraint>> {
        self.soft_activity_constraint.clone()
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
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        self.violation.clone()
    }
}

impl SoftActivityConstraint for TestSoftActivityConstraint {
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Cost {
        self.cost
    }
}

#[test]
fn can_evaluate_hard_activity_constraints() {
    let mut pipeline = ConstraintPipeline::new();
    pipeline.add_module(TestConstraintModule {
        state_keys: vec![1, 2],
        hard_route_constraint: None,
        hard_activity_constraint: Some(Arc::new(TestHardActivityConstraint { violation: None })),
        soft_route_constraint: None,
        soft_activity_constraint: None,
    });
    pipeline.add_module(TestConstraintModule {
        state_keys: vec![3, 4],
        hard_route_constraint: None,
        hard_activity_constraint: Some(Arc::new(TestHardActivityConstraint {
            violation: Some(ActivityConstraintViolation {
                code: 5,
                stopped: true,
            }),
        })),
        soft_route_constraint: None,
        soft_activity_constraint: None,
    });

    let result = pipeline.evaluate_hard_activity(
        &RouteContext::new(test_actor()),
        &ActivityContext {
            index: 0,
            prev: test_tour_activity_without_job(),
            target: test_tour_activity_without_job(),
            next: None,
        },
    );

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.code, 5);
    assert_eq!(result.stopped, true);
}

#[test]
fn can_estimate_hard_activity_constraints() {
    let mut pipeline = ConstraintPipeline::new();
    pipeline.add_module(TestConstraintModule {
        state_keys: vec![1, 2],
        hard_route_constraint: None,
        hard_activity_constraint: None,
        soft_route_constraint: None,
        soft_activity_constraint: Some(Arc::new(TestSoftActivityConstraint { cost: 5.0 })),
    });
    pipeline.add_module(TestConstraintModule {
        state_keys: vec![3, 4],
        hard_route_constraint: None,
        hard_activity_constraint: None,
        soft_route_constraint: None,
        soft_activity_constraint: Some(Arc::new(TestSoftActivityConstraint { cost: 7.0 })),
    });

    let result = pipeline.evaluate_soft_activity(
        &RouteContext::new(test_actor()),
        &ActivityContext {
            index: 0,
            prev: test_tour_activity_without_job(),
            target: test_tour_activity_without_job(),
            next: None,
        },
    );

    assert_eq!(result, 12.0);
}
