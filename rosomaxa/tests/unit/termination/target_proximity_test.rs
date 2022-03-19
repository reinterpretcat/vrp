use super::*;
use crate::helpers::example::create_heuristic_context_with_solutions;

parameterized_test! {can_use_target_proximity, (solutions, target_fitness, distance_threshold, expected), {
    can_use_target_proximity_impl(solutions, target_fitness, distance_threshold, expected);
}}

can_use_target_proximity! {
    case01: (vec![vec![0.5, 0.5]], vec![0.], 0.1, false),
    case02: (vec![vec![0., 0.]], vec![0.89], 0.1, false),
    case03: (vec![vec![0., 0.]], vec![0.91], 0.1, true),
}

fn can_use_target_proximity_impl(
    solutions: Vec<Vec<f64>>,
    target_fitness: Vec<f64>,
    distance_threshold: f64,
    expected: bool,
) {
    let mut context = create_heuristic_context_with_solutions(solutions);

    let result = TargetProximity::<_, _, _>::new(target_fitness, distance_threshold).is_termination(&mut context);

    assert_eq!(result, expected)
}
