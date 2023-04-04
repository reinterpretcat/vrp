use super::*;
use crate::construction::features::*;
use crate::helpers::construction::features::{create_goal_ctx_with_feature, create_goal_ctx_with_features};
use crate::helpers::models::domain::create_empty_insertion_context;
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

fn create_objective_feature_with_fixed_cost(name: &str, cost: Cost) -> Feature {
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

type FitnessFn = Arc<dyn Fn(&str, &InsertionContext) -> f64 + Send + Sync>;

fn create_objective_feature_with_dynamic_cost(name: &str, fitness_fn: FitnessFn) -> Feature {
    struct TestFeatureObjective {
        name: String,
        fitness_fn: FitnessFn,
    }

    impl Objective for TestFeatureObjective {
        type Solution = InsertionContext;

        fn fitness(&self, solution: &Self::Solution) -> f64 {
            (self.fitness_fn)(self.name.as_str(), solution)
        }
    }

    impl FeatureObjective for TestFeatureObjective {
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

    assert_eq!(
        create_goal_ctx_with_feature(create_objective_feature_with_fixed_cost("o_1", 1.)).estimate(&move_ctx),
        InsertionCost::new(&[1.])
    );

    assert_eq!(
        create_goal_ctx_with_features(
            vec![
                create_objective_feature_with_fixed_cost("o_1", 1.),
                create_objective_feature_with_fixed_cost("o_2", 1.)
            ],
            vec![vec!["o_1", "o_2"]],
        )
        .estimate(&move_ctx),
        InsertionCost::new(&[1., 1.])
    );
}

parameterized_test! {can_use_objective_total_order, (feature_map, left_fitness, right_fitness, expected), {
    can_use_objective_total_order_impl(feature_map, left_fitness, right_fitness, expected);
}}

can_use_objective_total_order! {
    case01: (
        vec![vec!["0"], vec!["1"], vec!["2", "3"]],
        vec![3., 5., 0., 1.], vec![3., 5., 1., 0.],
        Ordering::Equal,
    ),
    case02: (
        vec![vec!["0", "1"], vec!["2"], vec!["3"]],
        vec![3., 5., 0., 0.], vec![5., 3., 0., 0.],
        Ordering::Equal,
    ),
    case03: (
        vec![vec!["0", "1"], vec!["2"], vec!["3"]],
        vec![3., 3., 0., 0.], vec![5., 3., 0., 0.],
        Ordering::Less,
    ),
    case04: (
        vec![vec!["0", "1"], vec!["2"], vec!["3"]],
        vec![5., 5., 0., 0.], vec![5., 3., 0., 0.],
        Ordering::Greater,
    ),
}

fn can_use_objective_total_order_impl(
    feature_map: Vec<Vec<&str>>,
    left_fitness: Vec<f64>,
    right_fitness: Vec<f64>,
    expected: Ordering,
) {
    let fitness_fn = Arc::new(|name: &str, insertion_ctx: &InsertionContext| {
        insertion_ctx
            .solution
            .state
            .get(&name.parse::<i32>().unwrap())
            .and_then(|s| s.downcast_ref::<f64>())
            .copied()
            .unwrap()
    });
    let create_insertion_ctx_with_fitness_state = |fitness: Vec<f64>| {
        let mut insertion_ctx = create_empty_insertion_context();
        fitness.into_iter().enumerate().for_each(|(idx, value)| {
            insertion_ctx.solution.state.insert(idx as i32, Arc::new(value));
        });
        insertion_ctx
    };
    let goal_ctx = create_goal_ctx_with_features(
        vec![
            create_objective_feature_with_dynamic_cost("0", fitness_fn.clone()),
            create_objective_feature_with_dynamic_cost("1", fitness_fn.clone()),
            create_objective_feature_with_dynamic_cost("2", fitness_fn.clone()),
            create_objective_feature_with_dynamic_cost("3", fitness_fn),
        ],
        feature_map,
    );
    let left = create_insertion_ctx_with_fitness_state(left_fitness);
    let right = create_insertion_ctx_with_fitness_state(right_fitness);

    assert_eq!(goal_ctx.total_order(&left, &right), expected);
}
