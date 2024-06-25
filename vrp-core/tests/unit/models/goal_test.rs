use super::*;
use crate::construction::features::capacity::create_capacity_limit_feature;
use crate::construction::features::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::solution::{test_actor, ActivityBuilder};
use crate::models::common::SingleDimLoad;

fn create_feature(name: &str, cost: Cost, violation: Option<ConstraintViolation>) -> Feature {
    struct TestFeatureObjective {
        cost: Cost,
    }

    impl FeatureObjective for TestFeatureObjective {
        fn fitness(&self, _: &InsertionContext) -> f64 {
            self.cost
        }

        fn estimate(&self, _: &MoveContext<'_>) -> Cost {
            self.cost
        }
    }

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

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(TestFeatureObjective { cost })
        .with_constraint(TestFeatureConstraint { violation })
        .build()
        .unwrap()
}

type FitnessFn = Arc<dyn Fn(&str, &InsertionContext) -> f64 + Send + Sync>;

fn create_objective_feature_with_dynamic_cost(name: &str, fitness_fn: FitnessFn) -> Feature {
    struct TestFeatureObjective {
        name: String,
        fitness_fn: FitnessFn,
    }

    impl FeatureObjective for TestFeatureObjective {
        fn fitness(&self, solution: &InsertionContext) -> f64 {
            (self.fitness_fn)(self.name.as_str(), solution)
        }

        fn estimate(&self, _: &MoveContext<'_>) -> Cost {
            unimplemented!()
        }
    }

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(TestFeatureObjective { name: name.to_string(), fitness_fn })
        .build()
        .unwrap()
}

#[test]
pub fn can_create_goal_context_with_objective() -> GenericResult<()> {
    let features = vec![create_minimize_tours_feature("min_tours").unwrap()];

    GoalContextBuilder::with_features(features)?
        .set_goal(&["min_tours"], &["min_tours"])?
        .build()
        .expect("cannot build context");
    Ok(())
}

#[test]
pub fn cannot_create_goal_context_without_objectives() -> GenericResult<()> {
    let features = vec![create_capacity_limit_feature::<SingleDimLoad>("capacity", 0).unwrap()];

    assert!(GoalContextBuilder::with_features(features)?.build().is_err());
    Ok(())
}

#[test]
pub fn can_evaluate_constraints() -> GenericResult<()> {
    let route_ctx = RouteContext::new(test_actor());
    let activity_ctx = ActivityContext {
        index: 0,
        prev: &ActivityBuilder::default().job(None).build(),
        target: &ActivityBuilder::default().job(None).build(),
        next: None,
    };
    let move_ctx = MoveContext::activity(&route_ctx, &activity_ctx);

    assert_eq!(
        GoalContextBuilder::with_features(vec![create_feature("c_1", 0., ConstraintViolation::success())])?
            .set_goal(&["c_1"], &["c_1"])?
            .build()?
            .evaluate(&move_ctx),
        None
    );

    assert_eq!(
        GoalContextBuilder::with_features(vec![
            create_feature("c_1", 0., ConstraintViolation::success()),
            create_feature("c_2", 0., ConstraintViolation::fail(1)),
        ])?
        .set_goal(&["c_1"], &["c_1"])?
        .build()?
        .evaluate(&move_ctx),
        ConstraintViolation::fail(1)
    );

    assert_eq!(
        GoalContextBuilder::with_features(vec![
            create_feature("c_1", 0., ConstraintViolation::skip(1)),
            create_feature("c_2", 0., ConstraintViolation::success()),
        ])?
        .set_goal(&["c_1"], &["c_1"])?
        .build()?
        .evaluate(&move_ctx),
        ConstraintViolation::skip(1)
    );

    Ok(())
}

parameterized_test! {can_use_objective_estimate, (feature_map, expected_cost), {
    can_use_objective_estimate_impl(feature_map, expected_cost);
}}

can_use_objective_estimate! {
    case01_use_one: (
        &["o_1"], &[1.],
    ),
    case02_use_two: (
        &["o_1", "o_2"], &[1., 1.],
    ),
}

fn can_use_objective_estimate_impl(feature_map: &[&str], expected_cost: &[Cost]) {
    let route_ctx = RouteContext::new(test_actor());
    let activity_ctx = ActivityContext {
        index: 0,
        prev: &ActivityBuilder::default().job(None).build(),
        target: &ActivityBuilder::default().job(None).build(),
        next: None,
    };
    let move_ctx = MoveContext::activity(&route_ctx, &activity_ctx);
    let features = feature_map.iter().map(|name| create_feature(name, 1., None)).collect();

    let result = TestGoalContextBuilder::default()
        .add_features(features)
        .with_objectives(feature_map)
        .build()
        .estimate(&move_ctx);

    assert_eq!(result, InsertionCost::new(expected_cost));
}

parameterized_test! {can_use_objective_total_order, (left_fitness, right_fitness, expected), {
    can_use_objective_total_order_impl(left_fitness, right_fitness, expected);
}}

can_use_objective_total_order! {
    case01_equal: (vec![3., 5., 1., 1.], vec![3., 5., 1., 1.], Ordering::Equal),
    case02_less:  (vec![3., 3., 0., 0.], vec![3., 5., 0., 0.], Ordering::Less),
    case03_great: (vec![5., 5., 0., 0.], vec![5., 3., 0., 0.], Ordering::Greater),
}

fn can_use_objective_total_order_impl(left_fitness: Vec<f64>, right_fitness: Vec<f64>, expected: Ordering) {
    let fitness_fn = Arc::new(move |name: &str, insertion_ctx: &InsertionContext| {
        let idx = name.parse::<usize>().unwrap();
        insertion_ctx.solution.state.get_value::<(), Vec<f64>>().unwrap()[idx]
    });
    let create_insertion_ctx_with_fitness_state = |fitness: Vec<f64>| {
        let mut insertion_ctx = TestInsertionContextBuilder::default().build();
        insertion_ctx.solution.state.set_value::<(), _>(fitness);
        insertion_ctx
    };
    let goal_ctx = TestGoalContextBuilder::default()
        .add_feature(create_objective_feature_with_dynamic_cost("0", fitness_fn.clone()))
        .add_feature(create_objective_feature_with_dynamic_cost("1", fitness_fn.clone()))
        .add_feature(create_objective_feature_with_dynamic_cost("2", fitness_fn.clone()))
        .add_feature(create_objective_feature_with_dynamic_cost("3", fitness_fn))
        .with_objectives(&["0", "1", "2", "3"])
        .build();
    let left = create_insertion_ctx_with_fitness_state(left_fitness);
    let right = create_insertion_ctx_with_fitness_state(right_fitness);

    assert_eq!(goal_ctx.total_order(&left, &right), expected);
}

#[test]
fn can_detect_same_name_usage() {
    let goal_ctx = GoalContextBuilder::with_features(vec![
        create_objective_feature_with_dynamic_cost("name_1", Arc::new(|_, _| 1.)),
        create_objective_feature_with_dynamic_cost("name_2", Arc::new(|_, _| 1.)),
        create_objective_feature_with_dynamic_cost("name_1", Arc::new(|_, _| 1.)),
    ]);

    match goal_ctx {
        Ok(_) => unreachable!(),
        Err(message) => {
            assert_eq!(
                message,
                GenericError::from(
                    "some of the features are defined more than once, check ids list: name_1,name_2,name_1"
                )
            )
        }
    }
}
