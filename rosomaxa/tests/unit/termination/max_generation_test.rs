use super::*;
use crate::Timer;
use crate::helpers::example::create_default_heuristic_context;

parameterized_test! {can_detect_termination, (generation, limit, expected), {
    can_detect_termination_impl(generation, limit, expected);
}}

can_detect_termination! {
    case_01: (11, 10, true),
    case_02: (9, 10, false),
    case_03: (10, 10, true),
}

fn can_detect_termination_impl(generation: usize, limit: usize, expected: bool) {
    let mut context = create_default_heuristic_context();

    (0..=generation).for_each(|_| {
        context.on_generation(vec![], 0.1, Timer::start());
    });

    let result = MaxGeneration::<_, _, _>::new(limit).is_termination(&mut context);

    assert_eq!(result, expected);
}
