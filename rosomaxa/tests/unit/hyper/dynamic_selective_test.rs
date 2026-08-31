use super::*;
use crate::example::{VectorContext, VectorObjective, VectorSolution};
use crate::helpers::example::{
    create_default_heuristic_context, create_example_objective, create_heuristic_context_with_solutions,
};
use crate::population::Greedy;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn can_run_intensify_operator_without_replacing_search() {
    struct Noop;
    struct CountingIntensify {
        count: Arc<AtomicUsize>,
    }

    impl HeuristicSearchOperator for Noop {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            solution.deep_copy()
        }
    }

    impl HeuristicIntensifyOperator for CountingIntensify {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn intensify(&self, _: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
            self.count.fetch_add(1, Ordering::Relaxed);
            vec![solution.deep_copy()]
        }
    }

    let environment = Arc::new(Environment::default());
    let objective = create_example_objective();
    let solution = VectorSolution::new(vec![0., 0.], 0., vec![0., 0.]);
    let population = Box::new(Greedy::new(objective.clone(), 1, Some(solution.deep_copy())));
    let heuristic_ctx = VectorContext::new(objective, population, TelemetryMode::None, environment.clone());
    let count = Arc::new(AtomicUsize::new(0));
    let mut heuristic = DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(
        vec![(Arc::new(Noop), "noop".to_string(), 1.)],
        environment.as_ref(),
    )
    .with_intensify_operators(vec![Arc::new(CountingIntensify { count: count.clone() })]);

    let search_offspring = heuristic.search_many(&heuristic_ctx, vec![&solution]);
    let intensify_offspring = heuristic.intensify_many(&heuristic_ctx, vec![&solution]);

    assert_eq!(search_offspring.len(), 1);
    assert_eq!(intensify_offspring.len(), 1);
    assert_eq!(count.load(Ordering::Relaxed), 1);
}

#[test]
fn can_apply_prior_policy() {
    assert_eq!(get_prior_alpha(1., 1.), 1.);
    assert_eq!(get_prior_alpha(10., 1.), PRIOR_ALPHA_MAX);
}

parameterized_test! {can_decide_when_to_reset, (generation, next_reset, improvement_ratio, expected), {
    let statistics = HeuristicStatistics { generation, improvement_1000_ratio: improvement_ratio, ..Default::default() };

    assert_eq!(should_reset(&statistics, next_reset), expected);
}}

can_decide_when_to_reset! {
    case_01: (1000, 1000, 0., true),
    case_02: (999, 1000, 0., false),
    case_03: (2000, 3000, 0., false),
    case_04: (2000, 1000, 0.01, false),
}

parameterized_test! {can_advance_stagnation_reset, (generation, interval, expected), {
    assert_eq!(advance_stagnation_reset(generation, interval), expected);
}}

can_advance_stagnation_reset! {
    case_01: (1000, 1000, (2000, 3000)),
    case_02: (3000, 2000, (3000, 6000)),
    case_03: (usize::MAX, usize::MAX, (usize::MAX, usize::MAX)),
}

#[test]
fn can_reset_both_search_states_during_stagnation() {
    struct Noop;

    impl HeuristicSearchOperator for Noop {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            solution.deep_copy()
        }
    }

    let environment = Environment::default();
    let mut heuristic = DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(
        vec![(Arc::new(Noop), "noop".to_string(), 1.)],
        &environment,
    );
    let feedback = SearchFeedback {
        sample: SearchSample {
            duration: 1,
            transition: (SearchState::BestKnown, SearchState::Diverse),
            is_parent_improvement: false,
            is_new_best: false,
        },
        slot_idx: 0,
        solution: None,
    };
    heuristic.agent.slot_machines.values_mut().for_each(|slots| slots[0].progress.update(&feedback));
    heuristic.agent.slot_machines.get_mut(&SearchState::Diverse).expect("missing diverse slots")[0]
        .promotion
        .as_mut()
        .expect("missing promotion posterior")
        .update(false);

    heuristic.agent.reset_if_stagnant(&HeuristicStatistics {
        generation: STAGNATION_WINDOW,
        improvement_1000_ratio: 0.,
        ..Default::default()
    });

    heuristic.agent.slot_machines.values().for_each(|slots| {
        let params = slots[0].progress.get_params();
        assert_eq!((params.alpha, params.beta, params.observations), (1., 1., 1));
    });
    let promotion = heuristic.agent.slot_machines[&SearchState::Diverse][0]
        .promotion
        .as_ref()
        .expect("missing promotion posterior")
        .params();
    assert_eq!((promotion.alpha, promotion.beta), (1., 1.));
}

#[test]
fn can_avoid_zero_duration_in_telemetry() {
    assert_eq!(get_duration_micros(Duration::ZERO), 1);
}

#[test]
fn can_treat_noop_as_unsuccessful_through_search_action() {
    struct Noop;

    impl HeuristicSearchOperator for Noop {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            solution.deep_copy()
        }
    }

    let heuristic_ctx = create_heuristic_context_with_solutions(vec![vec![0.0, 0.0]]);
    let solution = VectorSolution::new(vec![0.0, 0.0], 1.0, vec![0.0, 0.0]);
    let action = SearchAction { operator: Arc::new(Noop) };
    let best_known = heuristic_ctx.ranked().next();
    let feedback = action.take(SearchContext {
        heuristic_ctx: &heuristic_ctx,
        best_known,
        from: SearchState::BestKnown,
        slot_idx: 0,
        solution: &solution,
    });

    assert!(!feedback.sample.is_new_best);
    assert!(!feedback.is_success());
    assert!(feedback.sample.duration > 0);
}

#[test]
fn can_learn_diverse_progress_and_promotion_separately() {
    struct Noop;

    impl HeuristicSearchOperator for Noop {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            solution.deep_copy()
        }
    }

    let environment = Environment::default();
    let mut agent = SearchAgent::new(vec![(Arc::new(Noop), "noop".to_string(), 1.)], &environment);
    let create_feedback = |is_parent_improvement, is_new_best| SearchFeedback {
        sample: SearchSample {
            duration: 1,
            transition: (SearchState::Diverse, if is_new_best { SearchState::BestKnown } else { SearchState::Diverse }),
            is_parent_improvement,
            is_new_best,
        },
        slot_idx: 0,
        solution: None,
    };

    agent.update(1, &create_feedback(true, false));
    let slot = &agent.slot_machines[&SearchState::Diverse][0];
    let params = slot.progress.get_params();
    assert_eq!((params.alpha, params.beta), (2., 1.));
    let promotion = slot.promotion.as_ref().expect("missing promotion posterior").params();
    assert_eq!((promotion.alpha, promotion.beta), (1., 2.));

    agent.update(2, &create_feedback(false, false));
    let slot = &agent.slot_machines[&SearchState::Diverse][0];
    let params = slot.progress.get_params();
    assert_eq!((params.alpha, params.beta), (2., 2.));
    let promotion = slot.promotion.as_ref().expect("missing promotion posterior").params();
    assert_eq!((promotion.alpha, promotion.beta), (1., 2.));

    agent.update(3, &create_feedback(true, true));

    let slot = &agent.slot_machines[&SearchState::Diverse][0];
    let params = slot.progress.get_params();
    let promotion = slot.promotion.as_ref().expect("missing promotion posterior").params();
    assert_eq!((params.alpha, params.beta, params.observations), (3., 2., 3));
    assert_eq!((promotion.alpha, promotion.beta), (2., 2.));
}

#[test]
fn can_compute_product_variance() {
    let left = BernoulliParams { alpha: 1., beta: 1., mean: 0.5, variance: 0.1, observations: 0 };
    let right = BernoulliParams { alpha: 1., beta: 1., mean: 0.2, variance: 0.02, observations: 0 };

    assert!((product_variance(&left, &right) - 0.011).abs() < Float::EPSILON);
}

#[test]
fn can_display_heuristic_info() {
    struct Noop;

    impl HeuristicSearchOperator for Noop {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            solution.deep_copy()
        }
    }

    let is_experimental = true;
    let environment = Environment { is_experimental, ..Environment::default() };
    let mut heuristic = DynamicSelective::<VectorContext, VectorObjective, VectorSolution>::new(
        vec![(Arc::new(Noop), "noop".to_string(), 1.)],
        &environment,
    );
    let solution = VectorSolution::new(vec![0., 0.], 0., vec![0., 0.]);
    heuristic.search(&create_default_heuristic_context(), &solution);
    heuristic.agent.save_params(0);

    // Test that diagnostic system is properly initialized
    assert_eq!(heuristic.agent.tracker.telemetry_enabled(), is_experimental);

    let formatted = format!("{heuristic}");

    // Should contain TELEMETRY section when experimental mode is enabled
    if is_experimental {
        assert!(formatted.contains("TELEMETRY"));
        assert!(formatted.contains("duration_us"));
        assert!(formatted.contains("0,best,noop,"));
        assert!(formatted.contains("calls,incumbent_improvements,duration_us"));
        assert!(!formatted.contains("reward"));
    } else {
        // When not experimental, should be empty or minimal
        assert!(formatted.is_empty() || !formatted.contains("thompson_diagnostics:"));
    }
}

#[test]
fn can_track_exact_state_specific_summary() {
    let mut tracker = HeuristicTracker::new(true);
    tracker.observe_sample(
        1,
        "operator",
        &SearchSample {
            duration: 10,
            transition: (SearchState::BestKnown, SearchState::BestKnown),
            is_parent_improvement: true,
            is_new_best: true,
        },
    );
    tracker.observe_sample(
        2,
        "operator",
        &SearchSample {
            duration: 20,
            transition: (SearchState::BestKnown, SearchState::Diverse),
            is_parent_improvement: false,
            is_new_best: false,
        },
    );
    tracker.observe_sample(
        2,
        "operator",
        &SearchSample {
            duration: 30,
            transition: (SearchState::Diverse, SearchState::BestKnown),
            is_parent_improvement: true,
            is_new_best: true,
        },
    );

    let best = tracker.get_summary(&SearchState::BestKnown, "operator");
    let diverse = tracker.get_summary(&SearchState::Diverse, "operator");

    assert_eq!((best.incumbent_improvements, best.duration), (1, 30));
    assert_eq!((diverse.incumbent_improvements, diverse.duration), (1, 30));
    assert_eq!((best.parent_improvements, diverse.parent_improvements), (1, 1));
}

#[test]
fn can_compact_complete_parameter_snapshots() {
    let mut telemetry = (0..5).map(|generation| (generation, Vec::new())).collect();
    let mut recording_interval = 1;

    compact_params(&mut telemetry, &mut recording_interval, 4);

    assert_eq!(recording_interval, 2);
    assert_eq!(telemetry.into_iter().map(|(generation, _)| generation).collect::<Vec<_>>(), vec![0, 2, 4]);
}
