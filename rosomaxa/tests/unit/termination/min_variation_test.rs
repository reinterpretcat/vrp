use super::*;
use crate::Timer;
use crate::helpers::example::*;
use std::thread::sleep;
use std::time::Duration;

parameterized_test! {can_detect_termination_with_sample, (sample, threshold, delta, no_other_variance, expected), {
    can_detect_termination_with_sample_impl(sample, threshold, delta, no_other_variance, expected);
}}

can_detect_termination_with_sample! {
    case_01: (5, 0.1, 1E-2, true, vec![false, false, false, false, true]),
    case_02: (5, 0.1, 1E-2, false, vec![false, false, false, false, false]),
    case_03: (5, 0.1, 1E-1, true, vec![false, false, false, false, false]),
}

fn can_detect_termination_with_sample_impl(
    sample: usize,
    threshold: Float,
    delta: Float,
    no_other_variance: bool,
    expected: Vec<bool>,
) {
    let mut context = create_default_heuristic_context();
    let termination = MinVariation::<_, _, _, _>::new_with_sample(sample, threshold, false, 0);

    let result = (0..sample)
        .map(|i| {
            let other = if no_other_variance { 0. } else { i as Float };
            let cost = 1. + (i + 1) as Float * delta;

            context.on_generation(vec![], 0.1, Timer::start());

            termination.update_and_check(&mut context, vec![other, other, cost])
        })
        .collect::<Vec<bool>>();

    assert_eq!(result, expected)
}

#[test]
fn can_detect_termination_with_period() {
    let period = 2;
    let iterations = 3;
    let delta = 1E-2;
    let threshold = 0.1;
    let expected = vec![false, false, true];

    let mut context = create_default_heuristic_context();
    let termination = MinVariation::<_, _, _, _>::new_with_period(period, threshold, false, 0);

    let result = (0..iterations)
        .map(|i| {
            let cost = 1. + (i + 1) as Float * delta;
            let result = termination.update_and_check(&mut context, vec![0., 0., cost]);
            sleep(Duration::from_secs(1));

            result
        })
        .collect::<Vec<bool>>();

    assert_eq!(result, expected);
}

parameterized_test! {can_maintain_period_buffer_size, (size, check_sorted), {
    can_maintain_period_buffer_size_impl(size, check_sorted);
}}

can_maintain_period_buffer_size! {
    case_01: (0, false),
    case_02: (1, false),
    case_03: (1000, true),
    case_04: (2000, true),
}

fn can_maintain_period_buffer_size_impl(size: u128, check_sorted: bool) {
    let key = 0;
    let mut context = create_default_heuristic_context();
    context.set_state(key, (0..size).map(|i| (i, vec![Float::default(), Float::default()])).collect::<Vec<_>>());
    let termination = MinVariation::<_, _, _, _>::new_with_period(300, 0.01, false, key);

    termination.update_and_check(&mut context, vec![Float::default(), Float::default()]);

    let values = context.get_state::<Vec<(u128, Vec<Float>)>>(&key).unwrap();
    if check_sorted {
        let all_sorted = values.windows(2).all(|data| {
            let (a, b) = match data {
                &[(a, _), (b, _)] => (a, b),
                _ => unreachable!(),
            };

            a <= b
        });
        assert!(all_sorted);
    }
    assert!(values.len() < 1000);
}
