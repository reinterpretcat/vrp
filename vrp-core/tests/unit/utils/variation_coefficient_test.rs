use crate::helpers::models::domain::create_empty_problem;
use crate::helpers::solver::create_default_refinement_ctx;
use crate::solver::RefinementContext;
use crate::utils::variation_coefficient::VariationCoefficient;

parameterized_test! {can_detect_termination, (capacity, threshold, delta, expected), {
    can_detect_termination_impl(capacity, threshold, delta, expected);
}}

can_detect_termination! {
    case_01: (5, 0.1, 1E-2, vec![false, false, false, false, true]),
    case_02: (5, 0.1, 1E-1, vec![false, false, false, false, false]),
}

fn can_detect_termination_impl(capacity: usize, threshold: f64, delta: f64, expected: Vec<bool>) {
    let mut refinement_ctx = create_default_refinement_ctx(create_empty_problem());
    let termination = VariationCoefficient::new(capacity, threshold, "test_cv");

    let result = (0..capacity)
        .map(|i| {
            refinement_ctx.generation = i;
            let cost = 1. + (i + 1) as f64 * delta;

            termination.update_and_check(&mut refinement_ctx, cost)
        })
        .collect::<Vec<bool>>();

    assert_eq!(result, expected);
}
