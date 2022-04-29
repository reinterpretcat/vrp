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
    impl HeuristicOperator for DelayableHeuristicOperator {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            let delay = self.random.uniform_int(self.delay_range.start, self.delay_range.end);
            std::thread::sleep(Duration::from_millis(delay as u64));
            solution.deep_copy()
        }
    }
    let random = Environment::default().random.clone();
    let solution = VectorSolution::new(vec![0., 0.], create_example_objective());
    let mut heuristic = DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(
        vec![
            (Arc::new(DelayableHeuristicOperator { delay_range: (2..3), random: random.clone() }), "first".to_string()),
            (
                Arc::new(DelayableHeuristicOperator { delay_range: (7..10), random: random.clone() }),
                "second".to_string(),
            ),
        ],
        random,
    );

    heuristic.search(&create_default_heuristic_context(), (0..100).map(|_| &solution).collect());

    let median = heuristic.heuristic_median.approx_median().expect("cannot be None");
    assert!(median > 0);
}
