use super::*;
use crate::example::{VectorContext, VectorObjective, VectorSolution};
use crate::helpers::example::create_default_heuristic_context;
use std::ops::Range;
use std::time::Duration;

#[test]
fn can_estimate_median() {
    struct DelayableHeuristicOperator {
        delay_range: Range<i32>,
        random: Arc<dyn Random>,
    }
    impl HeuristicSearchOperator for DelayableHeuristicOperator {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            let delay = self.random.uniform_int(self.delay_range.start, self.delay_range.end);
            std::thread::sleep(Duration::from_millis(delay as u64));
            solution.deep_copy()
        }
    }
    impl HeuristicDiversifyOperator for DelayableHeuristicOperator {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
            vec![self.search(heuristic_ctx, solution)]
        }
    }
    let environment = Environment::default();
    let random = environment.random.clone();
    let solution = VectorSolution::new(vec![0., 0.], 0., vec![0., 0.]);
    let mut heuristic = DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(
        vec![
            (
                Arc::new(DelayableHeuristicOperator { delay_range: (2..3), random: random.clone() }),
                "first".to_string(),
                1.,
            ),
            (
                Arc::new(DelayableHeuristicOperator { delay_range: (7..10), random: random.clone() }),
                "second".to_string(),
                1.,
            ),
        ],
        vec![Arc::new(DelayableHeuristicOperator { delay_range: (2..3), random: random.clone() })],
        &environment,
    );

    heuristic.search_many(&create_default_heuristic_context(), (0..100).map(|_| &solution).collect());

    let median = heuristic.agent.tracker.approx_median().expect("cannot be None");
    assert!(median > 0);
}

parameterized_test! {can_compute_relative_distance, (fitness_a, fitness_b, expected), {
    can_compute_relative_distance_impl(fitness_a, fitness_b, expected);
}}

can_compute_relative_distance! {
    case_01_improvement: (vec![90.0], vec![100.0], 0.1),           // 10% distance: |100-90|/100 = 0.1
    case_02_regression: (vec![110.0], vec![100.0], 0.09),          // 9% distance: |110-100|/110 ≈ 0.09
    case_03_equal: (vec![100.0], vec![100.0], 0.0),                // Equal = no distance
    case_04_primary_priority: (vec![90.0, 100.0], vec![100.0, 90.0], 0.1), // Primary objective distance
}

fn can_compute_relative_distance_impl(fitness_a: Vec<Float>, fitness_b: Vec<Float>, expected: Float) {
    let solution_a = VectorSolution::new(vec![], fitness_a.first().copied().unwrap_or(0.0), fitness_a);
    let solution_b = VectorSolution::new(vec![], fitness_b.first().copied().unwrap_or(0.0), fitness_b);

    let result = get_relative_distance(&solution_a, &solution_b);

    assert!((result - expected).abs() < 0.02, "Expected ~{expected}, got {result}");
}

#[test]
fn can_display_heuristic_info() {
    let is_experimental = true;
    let environment = Environment { is_experimental, ..Environment::default() };
    let heuristic =
        DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(vec![], vec![], &environment);

    // Test that diagnostic system is properly initialized
    assert_eq!(heuristic.agent.tracker.telemetry_enabled(), is_experimental);

    let formatted = format!("{heuristic}");

    // Should contain TELEMETRY section when experimental mode is enabled
    if is_experimental {
        assert!(formatted.contains("TELEMETRY"));
    } else {
        // When not experimental, should be empty or minimal
        assert!(formatted.is_empty() || !formatted.contains("thompson_diagnostics:"));
    }
}

#[test]
fn can_handle_equal_fitness_solutions() {
    // Test that solutions with identical fitness return 0 distance.
    struct TestData;

    impl HeuristicSolution for TestData {
        fn fitness(&self) -> impl Iterator<Item = Float> {
            // fitness is the same
            Box::new(once(1.))
        }

        fn deep_copy(&self) -> Self {
            unreachable!()
        }
    }

    let distance = get_relative_distance(&TestData, &TestData);

    assert_eq!(distance, 0.)
}
