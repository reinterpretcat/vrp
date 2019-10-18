use crate::helpers::models::domain::{create_empty_problem, create_empty_solution};
use crate::helpers::refinement::create_refinement_context;
use crate::models::common::ObjectiveCost;
use crate::refinement::termination::max_generation::MaxGeneration;
use crate::refinement::termination::Termination;
use crate::utils::DefaultRandom;
use std::sync::Arc;
parameterized_test! {can_detect_termination, (generation, limit, expected), {
    can_detect_termination_impl(generation, limit, expected);
}}

can_detect_termination! {
    case_01: (11, 10, true),
    case_02: (9, 10, false),
    case_03: (10, 10, false),
}

fn can_detect_termination_impl(generation: usize, limit: usize, expected: bool) {
    let mut refinement_ctx = create_refinement_context(create_empty_problem(), Arc::new(DefaultRandom::new()));
    refinement_ctx.generation = generation;

    let result = MaxGeneration::new(limit)
        .is_termination(&refinement_ctx, (create_empty_solution(), ObjectiveCost::new(0., 0.), true));

    assert_eq!(result, expected);
}
