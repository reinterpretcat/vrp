use super::*;
use crate::construction::features::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::solution::{ActivityBuilder, test_actor};
use crate::models::common::SingleDimLoad;

fn create_feature(name: &str, cost: Cost, violation: Option<ConstraintViolation>) -> Feature {
    struct TestFeatureObjective {
        cost: Cost,
    }

    impl FeatureObjective for TestFeatureObjective {
        fn fitness(&self, _: &InsertionContext) -> Cost {
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

type FitnessFn = Arc<dyn Fn(&str, &InsertionContext) -> Float + Send + Sync>;

fn create_objective_feature_with_dynamic_cost(name: &str, fitness_fn: FitnessFn) -> Feature {
    struct TestFeatureObjective {
        name: String,
        fitness_fn: FitnessFn,
    }

    impl FeatureObjective for TestFeatureObjective {
        fn fitness(&self, solution: &InsertionContext) -> Cost {
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

struct TestRelaxableConstraint {
    violation: Float,
    strict: Option<ConstraintViolation>,
}

impl FeatureConstraint for TestRelaxableConstraint {
    fn evaluate(&self, _: &MoveContext<'_>) -> Option<ConstraintViolation> {
        self.strict.clone()
    }

    fn relaxation(&self) -> Option<&dyn RelaxedFeatureConstraint> {
        Some(self)
    }
}

impl RelaxedFeatureConstraint for TestRelaxableConstraint {
    fn evaluate_relaxed(&self, _: &MoveContext<'_>) -> Option<ConstraintViolation> {
        None
    }

    fn solution_violation(&self, _: &SolutionContext) -> Float {
        self.violation
    }

    fn estimate_violation(&self, _: &MoveContext<'_>) -> Float {
        self.violation
    }
}

#[test]
pub fn can_create_goal_context_with_objective() -> GenericResult<()> {
    GoalContextBuilder::with_features(&[create_minimize_tours_feature("min-tours").unwrap()])?
        .build()
        .expect("cannot build context");
    Ok(())
}

#[test]
pub fn cannot_create_goal_context_without_objectives() -> GenericResult<()> {
    let features = vec![CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build().unwrap()];

    assert!(GoalContextBuilder::with_features(&features).is_err());
    Ok(())
}

#[test]
fn cannot_relax_goal_without_capability() -> GenericResult<()> {
    let goal = GoalContextBuilder::with_features(&[create_feature("hard", 0., None)])?.build()?;

    assert!(goal.relaxed(0., 0.).is_none());

    Ok(())
}

#[test]
pub fn can_evaluate_constraints() -> GenericResult<()> {
    let solution_ctx = TestInsertionContextBuilder::default().build().solution;
    let route_ctx = RouteContext::new(test_actor());
    let activity_ctx = ActivityContext {
        index: 0,
        prev: &ActivityBuilder::default().job(None).build(),
        target: &ActivityBuilder::default().job(None).build(),
        next: None,
    };
    let move_ctx = MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx);

    let features = vec![create_feature("c_1", 0., ConstraintViolation::success())];
    assert_eq!(
        GoalContextBuilder::with_features(&features)?
            .set_main_goal(Goal::subset_of(&features, &["c_1"])?)
            .build()?
            .evaluate(&move_ctx),
        None
    );

    let features = vec![
        create_feature("c_1", 0., ConstraintViolation::success()),
        create_feature("c_2", 0., ConstraintViolation::fail(ViolationCode(1))),
    ];
    assert_eq!(
        GoalContextBuilder::with_features(&features)?
            .set_main_goal(Goal::subset_of(&features, &["c_1"])?)
            .build()?
            .evaluate(&move_ctx),
        ConstraintViolation::fail(ViolationCode(1))
    );

    let features = vec![
        create_feature("c_1", 0., ConstraintViolation::skip(ViolationCode(1))),
        create_feature("c_2", 0., ConstraintViolation::success()),
    ];
    assert_eq!(
        GoalContextBuilder::with_features(&features)?
            .set_main_goal(Goal::subset_of(&features, &["c_1"])?)
            .build()?
            .evaluate(&move_ctx),
        ConstraintViolation::skip(ViolationCode(1))
    );

    Ok(())
}

#[test]
fn can_relax_only_opted_in_constraint_and_track_exact_violation() -> GenericResult<()> {
    let relaxable =
        FeatureBuilder::from_feature(create_feature("relaxable", 0., ConstraintViolation::fail(ViolationCode(1))))
            .with_constraint(TestRelaxableConstraint {
                violation: 0.25,
                strict: ConstraintViolation::fail(ViolationCode(1)),
            })
            .build()?;
    let hard = create_feature("hard", 1., ConstraintViolation::fail(ViolationCode(2)));
    let goal = GoalContextBuilder::with_features(&[relaxable, hard])?.build()?;
    let relaxed = goal.relaxed(0., 0.).expect("a controlled relaxation should be available");
    let mut insertion_ctx = TestInsertionContextBuilder::default().build();
    let route_ctx = RouteContext::new(test_actor());
    let activity = ActivityBuilder::default().job(None).build();
    let activity_ctx = ActivityContext { index: 0, prev: &activity, target: &activity, next: None };

    assert_eq!(
        relaxed.evaluate(&MoveContext::activity(&insertion_ctx.solution, &route_ctx, &activity_ctx)),
        ConstraintViolation::fail(ViolationCode(2))
    );

    relaxed.accept_solution_state(&mut insertion_ctx.solution);
    assert_eq!(insertion_ctx.solution.state.get_relaxed_violation(), Some(&0.25));

    goal.accept_solution_state(&mut insertion_ctx.solution);
    assert!(insertion_ctx.solution.state.get_relaxed_violation().is_none());

    Ok(())
}

#[test]
fn can_preserve_relaxation_when_combining_features() -> GenericResult<()> {
    let create_relaxable = |name, violation| {
        FeatureBuilder::from_feature(create_feature(name, 0., ConstraintViolation::fail(ViolationCode(1))))
            .with_constraint(TestRelaxableConstraint { violation, strict: ConstraintViolation::fail(ViolationCode(1)) })
            .build()
    };
    let combined = FeatureCombinator::default()
        .add_features(&[
            create_relaxable("first", 0.1)?,
            create_relaxable("second", 0.2)?,
            create_feature("hard", 0., ConstraintViolation::fail(ViolationCode(2))),
        ])
        .combine()?;
    let goal = GoalContextBuilder::with_features(&[combined])?.build()?.relaxed(0., 0.).unwrap();
    let mut insertion_ctx = TestInsertionContextBuilder::default().build();
    let route_ctx = RouteContext::new(test_actor());
    let activity = ActivityBuilder::default().job(None).build();
    let activity_ctx = ActivityContext { index: 0, prev: &activity, target: &activity, next: None };

    assert_eq!(
        goal.evaluate(&MoveContext::activity(&insertion_ctx.solution, &route_ctx, &activity_ctx)),
        ConstraintViolation::fail(ViolationCode(2))
    );
    goal.accept_solution_state(&mut insertion_ctx.solution);
    assert!((insertion_ctx.solution.state.get_relaxed_violation().unwrap() - 0.3).abs() < 1E-9);

    Ok(())
}

#[test]
fn can_compare_relaxed_solutions_inside_violation_tolerance() -> GenericResult<()> {
    let relaxable = FeatureBuilder::default()
        .with_name("relaxable")
        .with_constraint(TestRelaxableConstraint { violation: 0., strict: None })
        .build()?;
    let goal = GoalContextBuilder::with_features(&[create_minimize_tours_feature("tours")?, relaxable])?.build()?;
    let create_solution = |route_count: usize, violation: Float| {
        TestInsertionContextBuilder::default()
            .with_routes((0..route_count).map(|_| RouteContext::new(test_actor())).collect())
            .with_state(|state| {
                state.set_relaxed_violation(violation);
            })
            .build()
    };
    let one_route = create_solution(1, 0.2);
    let two_routes = create_solution(2, 0.1);

    assert_eq!(goal.relaxed(0.2, 0.2).unwrap().total_order(&one_route, &two_routes), Ordering::Less);
    assert_eq!(goal.relaxed(0.05, 0.2).unwrap().total_order(&one_route, &two_routes), Ordering::Greater);

    Ok(())
}

#[test]
fn can_prioritize_projected_violation_outside_relaxed_tolerance() -> GenericResult<()> {
    let relaxable = FeatureBuilder::from_feature(create_feature("relaxable", 2., None))
        .with_constraint(TestRelaxableConstraint { violation: 0.25, strict: None })
        .build()?;
    let goal = GoalContextBuilder::with_features(&[relaxable])?.build()?.relaxed(0.2, 0.2).unwrap();
    let mut solution_ctx = TestInsertionContextBuilder::default().build().solution;
    solution_ctx.state.set_relaxed_violation(0.1);
    let route_ctx = RouteContext::new(test_actor());
    let activity = ActivityBuilder::default().job(None).build();
    let activity_ctx = ActivityContext { index: 0, prev: &activity, target: &activity, next: None };
    let move_ctx = MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx);
    let costs = goal.estimate(&move_ctx).iter().collect::<Vec<_>>();

    assert!((costs[0] - 0.15).abs() < 1E-9);
    assert_eq!(costs[1], 2.);
    // An estimate guides candidate ranking but cannot safely reject a compound move whose other route changes are
    // not represented by this insertion context. The complete refreshed solution decides admission.
    assert_eq!(goal.evaluate(&move_ctx), None);

    Ok(())
}

#[test]
fn can_check_exact_relaxed_admission_limit() -> GenericResult<()> {
    let relaxable = FeatureBuilder::default()
        .with_name("relaxable")
        .with_constraint(TestRelaxableConstraint { violation: 0., strict: None })
        .build()?;
    let goal = GoalContextBuilder::with_features(&[create_minimize_tours_feature("tours")?, relaxable])?
        .build()?
        .relaxed(0.05, 0.2)
        .unwrap();
    let create_solution = |violation| {
        TestInsertionContextBuilder::default()
            .with_state(|state| {
                state.set_relaxed_violation(violation);
            })
            .build()
    };

    assert!(goal.is_relaxed_admissible(&create_solution(0.2)));
    assert!(!goal.is_relaxed_admissible(&create_solution(0.21)));

    Ok(())
}

#[test]
fn can_admit_complete_repair_after_temporarily_removing_violation() -> GenericResult<()> {
    let relaxable = FeatureBuilder::default()
        .with_name("relaxable")
        .with_constraint(TestRelaxableConstraint { violation: 0.03, strict: None })
        .build()?;
    let goal = GoalContextBuilder::with_features(&[create_minimize_tours_feature("tours")?, relaxable])?
        .build()?
        .relaxed(0.02, 0.04)
        .unwrap();
    let create_solution = |violation| {
        TestInsertionContextBuilder::default()
            .with_state(|state| {
                state.set_relaxed_violation(violation);
            })
            .build()
    };
    let parent = create_solution(0.04);
    let candidate = create_solution(0.03);
    let partial = create_solution(0.);
    let route_ctx = RouteContext::new(test_actor());
    let activity = ActivityBuilder::default().job(None).build();
    let activity_ctx = ActivityContext { index: 0, prev: &activity, target: &activity, next: None };
    let move_ctx = MoveContext::activity(&partial.solution, &route_ctx, &activity_ctx);

    // The insertion preview cannot see the relief obtained from the removed source route. It may rank the move,
    // but only the refreshed 0.03 candidate is authoritative for the frozen 0.04 segment limit.
    assert_eq!(goal.evaluate(&move_ctx), None);
    assert!(goal.is_relaxed_admissible(&candidate));
    assert_eq!(goal.total_order(&candidate, &parent), Ordering::Less);

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
    let solution_ctx = TestInsertionContextBuilder::default().build().solution;
    let route_ctx = RouteContext::new(test_actor());
    let activity_ctx = ActivityContext {
        index: 0,
        prev: &ActivityBuilder::default().job(None).build(),
        target: &ActivityBuilder::default().job(None).build(),
        next: None,
    };
    let move_ctx = MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx);
    let features = feature_map.iter().map(|name| create_feature(name, 1., None)).collect();

    let result = TestGoalContextBuilder::empty().add_features(features).build().estimate(&move_ctx);

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

fn can_use_objective_total_order_impl(left_fitness: Vec<Float>, right_fitness: Vec<Float>, expected: Ordering) {
    let fitness_fn = Arc::new(move |name: &str, insertion_ctx: &InsertionContext| {
        let idx = name.parse::<usize>().unwrap();
        insertion_ctx.solution.state.get_value::<(), Vec<Float>>().unwrap()[idx]
    });
    let create_insertion_ctx_with_fitness_state = |fitness: Vec<Float>| {
        let mut insertion_ctx = TestInsertionContextBuilder::default().build();
        insertion_ctx.solution.state.set_value::<(), _>(fitness);
        insertion_ctx
    };
    let goal_ctx = TestGoalContextBuilder::default()
        .add_feature(create_objective_feature_with_dynamic_cost("0", fitness_fn.clone()))
        .add_feature(create_objective_feature_with_dynamic_cost("1", fitness_fn.clone()))
        .add_feature(create_objective_feature_with_dynamic_cost("2", fitness_fn.clone()))
        .add_feature(create_objective_feature_with_dynamic_cost("3", fitness_fn))
        .build();
    let left = create_insertion_ctx_with_fitness_state(left_fitness);
    let right = create_insertion_ctx_with_fitness_state(right_fitness);

    assert_eq!(goal_ctx.total_order(&left, &right), expected);
}

#[test]
fn can_detect_same_name_usage() {
    let goal_ctx = GoalContextBuilder::with_features(&[
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
