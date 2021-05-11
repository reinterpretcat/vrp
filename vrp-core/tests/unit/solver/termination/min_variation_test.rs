use super::*;
use crate::helpers::models::domain::create_empty_problem;
use crate::helpers::solver::create_default_refinement_ctx;
use std::thread::sleep;
use std::time::Duration;

parameterized_test! {can_detect_termination_with_sample, (capacity, threshold, delta, no_other_variance, expected), {
    can_detect_termination_with_sample_impl(capacity, threshold, delta, no_other_variance, expected);
}}

can_detect_termination_with_sample! {
    case_01: (5, 0.1, 1E-2, true, vec![false, false, false, false, true]),
    case_02: (5, 0.1, 1E-2, false, vec![false, false, false, false, false]),
    case_03: (5, 0.1, 1E-1, true, vec![false, false, false, false, false]),
}

fn can_detect_termination_with_sample_impl(
    capacity: usize,
    threshold: f64,
    delta: f64,
    no_other_variance: bool,
    expected: Vec<bool>,
) {
    let mut refinement_ctx = create_default_refinement_ctx(create_empty_problem());
    let termination = MinVariation::new_with_sample(capacity, threshold, false);

    let result = (0..capacity)
        .map(|i| {
            refinement_ctx.statistics.generation = i;
            let other = if no_other_variance { 0. } else { i as f64 };
            let cost = 1. + (i + 1) as f64 * delta;

            termination.update_and_check(&mut refinement_ctx, vec![other, other, cost])
        })
        .collect::<Vec<bool>>();

    assert_eq!(result, expected);
}

#[test]
fn can_detect_termination_with_period() {
    let period = 2;
    let iterations = 3;
    let delta = 1E-2;
    let threshold = 0.1;
    let expected = vec![false, false, true];

    let mut refinement_ctx = create_default_refinement_ctx(create_empty_problem());
    let termination = MinVariation::new_with_period(period, threshold, false);

    let result = (0..iterations)
        .map(|i| {
            let cost = 1. + (i + 1) as f64 * delta;
            let result = termination.update_and_check(&mut refinement_ctx, vec![0., 0., cost]);
            sleep(Duration::from_secs(1));

            result
        })
        .collect::<Vec<bool>>();

    assert_eq!(result, expected);
}
