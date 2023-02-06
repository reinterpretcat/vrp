use super::*;
use crate::example::{VectorContext, VectorObjective, VectorSolution};
use crate::helpers::example::{create_default_heuristic_context, create_example_objective};
use std::ops::Range;

parameterized_test! {can_evaluate_state_reward, (ratio, value, expected), {
    can_evaluate_state_reward_impl(ratio, value, expected);
}}

can_evaluate_state_reward! {
    case_01: (1.0, 1000., 1000.),
    case_02: (1.0, 0., 0.),
    case_03: (1.5, 0., -3.),
    case_04: (1.5, -10., -15.),
    case_05: (1.5, 30., 20.),
    case_06: (3., 30., 15.),
}

fn can_evaluate_state_reward_impl(ratio: f64, value: f64, expected: f64) {
    let median = MedianRatio { ratio };

    let result = median.eval(value);

    assert_eq!(result, expected);
}

#[test]
fn can_estimate_median() {
    struct DelayableHeuristicOperator {
        delay_range: Range<i32>,
        random: Arc<dyn Random + Send + Sync>,
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
    let solution = VectorSolution::new(vec![0., 0.], create_example_objective());
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

    heuristic.search(&create_default_heuristic_context(), (0..100).map(|_| &solution).collect());

    let median = heuristic.tracker.approx_median().expect("cannot be None");
    assert!(median > 0);
}

parameterized_test! {can_display_heuristic_info, is_experimental, {
    can_display_heuristic_info_impl(is_experimental);
}}

can_display_heuristic_info! {
    case_01: true,
    case_02: false,
}

fn can_display_heuristic_info_impl(is_experimental: bool) {
    let environment = Environment { is_experimental, ..Environment::default() };
    let mut heuristic =
        DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(vec![], vec![], &environment);
    heuristic.tracker.observation(
        1,
        "name1".to_string(),
        Duration::from_millis(100),
        1.,
        SearchState::Stagnated(MedianRatio { ratio: 1. }),
    );
    heuristic.tracker.observation(
        2,
        "name1".to_string(),
        Duration::from_millis(101),
        1.,
        SearchState::BestMajorImprovement(MedianRatio { ratio: 1. }),
    );
    heuristic.tracker.observation(
        1,
        "name2".to_string(),
        Duration::from_millis(102),
        1.,
        SearchState::DiverseImprovement(MedianRatio { ratio: 1. }),
    );

    let formatted = format!("{heuristic}");

    assert_eq!(!formatted.is_empty(), is_experimental);
}
