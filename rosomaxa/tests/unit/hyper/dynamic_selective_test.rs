use super::*;
use crate::example::{VectorContext, VectorObjective, VectorSolution};
use crate::helpers::example::{create_default_heuristic_context, create_example_objective};
use std::ops::Range;
use std::time::Duration;

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

    heuristic.search_many(&create_default_heuristic_context(), (0..100).map(|_| &solution).collect());

    let median = heuristic.agent.tracker.approx_median().expect("cannot be None");
    assert!(median > 0);
}

parameterized_test! {can_estimate_reward_multiplier, (approx_median, duration, has_improvement, expected), {
    can_estimate_reward_multiplier_impl(approx_median, duration, has_improvement, expected);
}}

can_estimate_reward_multiplier! {
    case_01_moderate: (Some(1), 1, false, 1.),
    case_02_allegro: (Some(2), 1, false, 1.5),
    case_03_allegretto: (Some(10), 8, false, 1.25),
    case_04_andante: (Some(8), 13, false, 0.75),
    case_05_moderato_improvement: (Some(1), 1, true, 2.),
}

fn can_estimate_reward_multiplier_impl(
    approx_median: Option<usize>,
    duration: usize,
    has_improvement: bool,
    expected: f64,
) {
    let heuristic_ctx = create_default_heuristic_context();
    let objective = create_example_objective();
    let solution = VectorSolution::new(vec![], objective);
    let search_ctx = SearchContext {
        heuristic_ctx: &heuristic_ctx,
        from: SearchState::BestKnown,
        slot_idx: 0,
        solution: &solution,
        approx_median,
    };

    let result = estimate_reward_perf_multiplier(&search_ctx, duration, has_improvement);

    assert_eq!(result, expected);
}

#[test]
fn can_display_heuristic_info() {
    let is_experimental = true;
    let environment = Environment { is_experimental, ..Environment::default() };
    let duration = 1;
    let reward = 1.;
    let transition = (SearchState::Diverse, SearchState::BestKnown);
    let mut heuristic =
        DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(vec![], vec![], &environment);

    heuristic.agent.tracker.observe_sample(1, SearchSample { name: "name1".to_string(), duration, reward, transition });

    let formatted = format!("{heuristic}");

    assert!(!formatted.is_empty());
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

        fn fitness<'a>(&'a self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a> {
            Box::new(solution.fitness())
        }
    }

    struct TestData;

    impl HeuristicSolution for TestData {
        fn fitness(&self) -> impl Iterator<Item = f64> {
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
