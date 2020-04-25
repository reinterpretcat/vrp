use crate::helpers::models::domain::*;
use crate::helpers::solver::create_default_refinement_ctx;
use crate::solver::termination::max_generation::MaxGeneration;
use crate::solver::termination::Termination;

parameterized_test! {can_detect_termination, (generation, limit, expected), {
    can_detect_termination_impl(generation, limit, expected);
}}

can_detect_termination! {
    case_01: (11, 10, true),
    case_02: (9, 10, false),
    case_03: (10, 10, true),
}

fn can_detect_termination_impl(generation: usize, limit: usize, expected: bool) {
    let mut refinement_ctx = create_default_refinement_ctx(create_empty_problem());
    refinement_ctx.generation = generation;

    let result = MaxGeneration::new(limit).is_termination(&mut refinement_ctx);

    assert_eq!(result, expected);
}
