use crate::helpers::models::domain::{create_empty_insertion_context, create_empty_problem};
use crate::models::common::ObjectiveCost;
use crate::refinement::termination::{Termination, VariationCoefficient};
use crate::refinement::RefinementContext;

parameterized_test! {can_detect_termination, (capacity, threshold, delta, expected), {
    can_detect_termination_impl(capacity, threshold, delta, expected);
}}

can_detect_termination! {
    case_01: (5, 0.1, 1E-2, vec![false, false, false, false, true]),
    case_02: (5, 0.1, 1E-1, vec![false, false, false, false, false]),
}

fn can_detect_termination_impl(capacity: usize, threshold: f64, delta: f64, expected: Vec<bool>) {
    let mut refinement_ctx = RefinementContext::new(create_empty_problem());
    let mut termination = VariationCoefficient::new(capacity, threshold);

    let result = (0..capacity)
        .map(|i| {
            refinement_ctx.generation = i;
            let solution = create_empty_insertion_context();
            let cost = ObjectiveCost::new(1. + (i + 1) as f64 * delta, 0.);

            termination.is_termination(&mut refinement_ctx, (&solution, cost, true))
        })
        .collect::<Vec<bool>>();

    assert_eq!(result, expected);
}
