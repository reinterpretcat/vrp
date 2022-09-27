use super::*;
use crate::construction::features::*;
use crate::helpers::construction::features::{create_goal_ctx_with_feature, create_goal_ctx_with_features};
use crate::helpers::models::solution::{test_activity_without_job, test_actor};
use crate::models::common::SingleDimLoad;

fn create_constraint_feature(name: &str, violation: Option<ConstraintViolation>) -> Feature {
    struct TestFeatureConstraint {
        violation: Option<ConstraintViolation>,
    }

    impl FeatureConstraint for TestFeatureConstraint {
        fn evaluate(&self, _: &MoveContext<'_>) -> Option<ConstraintViolation> {
            self.violation.clone()
        }

        fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
            Ok(source)
        }
    }

    FeatureBuilder::default().with_name(name).with_constraint(TestFeatureConstraint { violation }).build().unwrap()
}

fn create_objective_feature(name: &str, cost: Cost) -> Feature {
    struct TestFeatureObjective {
        cost: Cost,
    }

    impl Objective for TestFeatureObjective {
        type Solution = InsertionContext;

        fn fitness(&self, _: &Self::Solution) -> f64 {
            self.cost
        }
    }

    impl FeatureObjective for TestFeatureObjective {
        fn estimate(&self, _: &MoveContext<'_>) -> Cost {
            self.cost
        }
    }

    FeatureBuilder::default().with_name(name).with_objective(TestFeatureObjective { cost }).build().unwrap()
}

#[test]
pub fn can_create_goal_context_with_objective() {
    let features = &[create_minimize_tours_feature("min_tours").unwrap()];
    let objectives_map = &[vec!["min_tours".to_string()]];

    GoalContext::new(features, objectives_map, objectives_map).expect("cannot create goal context");
}

#[test]
pub fn can_create_goal_context_without_objectives() {
    let features = &[create_capacity_limit_feature::<SingleDimLoad>("capacity", 0).unwrap()];
    let objectives_map = &[];

    GoalContext::new(features, objectives_map, objectives_map).expect("cannot create goal context");
}

#[test]
pub fn can_evaluate_constraints() {
    let route_ctx = RouteContext::new(test_actor());
    let activity_ctx = ActivityContext {
        index: 0,
        prev: &test_activity_without_job(),
        target: &test_activity_without_job(),
        next: None,
    };
    let move_ctx = MoveContext::activity(&route_ctx, &activity_ctx);

    assert_eq!(
        GoalContext::new(&[create_constraint_feature("c_1", ConstraintViolation::success())], &[], &[])
            .unwrap()
            .evaluate(&move_ctx),
        None
    );

    assert_eq!(
        GoalContext::new(
            &[
                create_constraint_feature("c_1", ConstraintViolation::success()),
                create_constraint_feature("c_2", ConstraintViolation::fail(1)),
            ],
            &[],
            &[]
        )
        .unwrap()
        .evaluate(&move_ctx),
        ConstraintViolation::fail(1)
    );

    assert_eq!(
        GoalContext::new(
            &[
                create_constraint_feature("c_1", ConstraintViolation::skip(1)),
                create_constraint_feature("c_2", ConstraintViolation::success()),
            ],
            &[],
            &[]
        )
        .unwrap()
        .evaluate(&move_ctx),
        ConstraintViolation::skip(1)
    );
}

#[test]
pub fn can_use_objective_estimate() {
    let route_ctx = RouteContext::new(test_actor());
    let activity_ctx = ActivityContext {
        index: 0,
        prev: &test_activity_without_job(),
        target: &test_activity_without_job(),
        next: None,
    };
    let move_ctx = MoveContext::activity(&route_ctx, &activity_ctx);

    assert_eq!(create_goal_ctx_with_feature(create_objective_feature("o_1", 1.)).estimate(&move_ctx), 1.);

    assert_eq!(
        create_goal_ctx_with_features(
            vec![create_objective_feature("o_1", 1.), create_objective_feature("o_2", 1.)],
            vec![vec!["o_1", "o_2"]],
        )
        .estimate(&move_ctx),
        2.
    );
}
