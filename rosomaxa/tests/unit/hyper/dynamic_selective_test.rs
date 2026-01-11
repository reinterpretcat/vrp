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

parameterized_test! {can_estimate_reward_multiplier, (improvement_ratio, approx_median, duration, expected), {
    can_estimate_reward_multiplier_impl(improvement_ratio, approx_median, duration, expected);
}}

can_estimate_reward_multiplier! {
    case_01_fast_with_improvement: (0.1, Some(10), 5, 1.104),    // Fast operator during improvement: ln(2.0)*0.15*1.0 ≈ 0.104
    case_02_slow_with_improvement: (0.1, Some(10), 20, 0.896),   // Slow operator during improvement: ln(0.5)*0.15*1.0 ≈ -0.104
    case_03_fast_partial_improvement: (0.05, Some(10), 5, 1.052), // Fast with 50% damping: ln(2.0)*0.15*0.5 ≈ 0.052
    case_04_no_improvement: (0.0, Some(10), 5, 1.0),             // No improvement = no bonus
    case_05_no_median: (0.1, None, 5, 1.0),                      // No median = baseline
    case_06_clamped_fast: (0.1, Some(10), 1, 1.2),               // Very fast but clamped to PERF_TOLERANCE (0.2)
    case_07_clamped_slow: (0.1, Some(10), 100, 0.8),             // Very slow but clamped to -PERF_TOLERANCE (-0.2)
}

fn can_estimate_reward_multiplier_impl(
    improvement_ratio: Float,
    approx_median: Option<usize>,
    duration: usize,
    expected: Float,
) {
    // Create a mock context with the specified improvement ratio
    let mut heuristic_ctx = create_default_heuristic_context();

    // Simulate improvement ratio by triggering generations
    if improvement_ratio > 0.0 {
        let num_improvements = (improvement_ratio * 1000.0) as usize;

        // Add improving solutions
        for i in 0..num_improvements {
            let solution = VectorSolution::new(vec![], -(i as Float), vec![]);
            heuristic_ctx.on_generation(vec![solution], 0.0, Timer::start());
        }

        // Add non-improving solutions
        for _ in num_improvements..1000 {
            let solution = VectorSolution::new(vec![], 100.0, vec![]);
            heuristic_ctx.on_generation(vec![solution], 0.0, Timer::start());
        }
    }

    let solution = VectorSolution::new(vec![], 0., vec![]);
    let search_ctx = SearchContext {
        heuristic_ctx: &heuristic_ctx,
        from: SearchState::BestKnown,
        slot_idx: 0,
        solution: &solution,
        approx_median,
    };

    let result = estimate_reward_perf_multiplier(&search_ctx, duration);

    assert!((result - expected).abs() < 0.001, "Expected {expected}, got {result}");
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
fn can_handle_when_objective_lies() {
    struct LiarObjective;

    impl HeuristicObjective for LiarObjective {
        type Solution = TestData;

        fn total_order(&self, _: &Self::Solution, _: &Self::Solution) -> Ordering {
            // that is where it lies based on some non-fitness related factors for total order
            Ordering::Greater
        }
    }

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

    let distance = get_relative_distance(&LiarObjective, &TestData, &TestData);

    assert_eq!(distance, 0.)
}
