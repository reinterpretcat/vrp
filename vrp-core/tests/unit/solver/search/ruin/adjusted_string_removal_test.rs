/* Uses predefined values to control algorithm execution.
int distribution values:
1. route index in solution
2*. job index in selected route tour
3*. selected algorithm: 1: sequential algorithm(**)
4*. string removal index(-ies)
double distribution values:
1. string count
2*. string size(-s)
(*) - specific for each route.
(**) - calls more int and double distributions:
    int 5. split start
    dbl 3. alpha param
*/

use super::{AdjustedStringRemoval, lower_bounds, preserved_string, sequential_string};
use crate::construction::heuristics::InsertionContext;
use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::solver::search::{RemovalLimits, Ruin};
use rosomaxa::prelude::{Float, Random, RandomGen};
use std::sync::Arc;

parameterized_test! {can_ruin_solution_with_matrix_routes, (matrix, ints, reals, expected_ids), {
    can_ruin_solution_with_matrix_routes_impl(matrix, ints, reals, expected_ids);
}}

can_ruin_solution_with_matrix_routes! {
    // NOTE [10,5] in ints is for limits ranges
    case_01_sequential: ((10, 1), vec![10, 5, 0, 3, 1, 2], vec![1., 5.], vec!["c1", "c2", "c3", "c4", "c5"]),
    case_02_preserved: ((10, 1), vec![10, 5, 0, 2, 2, 1, 4], vec![1., 5., 0.5, 0.005], vec!["c0", "c1", "c2", "c5", "c6"]),
    case_03_preserved: ((10, 1), vec![10, 5, 0, 2, 2, 1, 4], vec![1., 5., 0.5, 0.5, 0.005], vec!["c0", "c1", "c2", "c6", "c7"]),
    case_04_preserved: ((10, 1), vec![10, 5, 0, 3, 2, 3, 4], vec![1., 5., 0.5, 0.5, 0.005], vec!["c2", "c6", "c7", "c8", "c9"]),
    case_05_sequential: ((5, 3), vec![10, 5, 1, 2, 1, 2], vec![1., 3.], vec!["c6", "c7", "c8"]),
    case_06_sequential: ((5, 3), vec![10, 5, 0, 2, 1, 2, 1, 2], vec![2., 3., 2.], vec!["c1", "c2", "c3", "c6", "c7"]),
    case_07_sequential: ((5, 3), vec![10, 5, 1, 2, 1, 2, 1, 2, 1, 2], vec![3., 3., 3., 3.], vec!["c1", "c11", "c12", "c13", "c2", "c3", "c6", "c7", "c8"]),
    case_08_preserved: ((5, 3), vec![10, 5, 1, 1, 2, 1, 3], vec![1., 3., 0.5], vec!["c5", "c6", "c9"]),
    case_09_preserved: ((5, 3), vec![10, 5, 1, 3, 2, 1, 3], vec![1., 3., 0.5], vec!["c5", "c6", "c7"]),
}

fn can_ruin_solution_with_matrix_routes_impl(
    matrix: (usize, usize),
    ints: Vec<i32>,
    reals: Vec<Float>,
    expected_ids: Vec<&str>,
) {
    let limits = RemovalLimits { removed_activities_range: 10..=10, affected_routes_range: 5..=5 };
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, false);
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(CheckedRandom(FakeRandom::new(ints, reals)))),
    );

    let insertion_ctx = AdjustedStringRemoval::new_with_defaults(limits)
        .run(&create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}

#[test]
fn can_select_exactly_the_string_starts_containing_seed() {
    for tour_size in 1..=32 {
        for cardinality in 1..=tour_size {
            for seed in 1..=tour_size {
                let (begin, end) = lower_bounds(cardinality, tour_size, seed);
                assert!(begin <= end);

                for start in 1..=tour_size {
                    let contains_seed = (start..start + cardinality).contains(&seed);
                    let fits_tour = start + cardinality <= tour_size + 1;
                    assert_eq!((begin..=end).contains(&start), contains_seed && fits_tour);
                }
            }
        }
    }
}

#[test]
fn can_select_sequential_strings_in_open_and_closed_tours() {
    for is_open in [false, true] {
        for tour_size in 1..=10 {
            let (_, solution) = generate_matrix_routes_with_defaults(tour_size, 1, is_open);
            let tour = &solution.routes[0].tour;

            for cardinality in 1..=tour_size {
                for seed in 1..=tour_size {
                    let (begin, end) = lower_bounds(cardinality, tour_size, seed);
                    for start in begin..=end {
                        let random: Arc<dyn Random> =
                            Arc::new(CheckedRandom(FakeRandom::new(vec![start as i32], vec![])));
                        let selected = sequential_string((tour, seed), cardinality, &random).collect::<Vec<_>>();

                        assert_eq!(selected.len(), cardinality);
                        assert!(selected.contains(&tour.get(seed).unwrap().retrieve_job().unwrap()));
                        assert_eq!(selected.first(), tour.get(start).unwrap().retrieve_job().as_ref());
                        assert_eq!(selected.last(), tour.get(start + cardinality - 1).unwrap().retrieve_job().as_ref());
                    }
                }
            }
        }
    }
}

#[test]
fn can_preserve_string_cardinality_and_seed_in_open_and_closed_tours() {
    for is_open in [false, true] {
        for tour_size in 1..=8 {
            let (_, solution) = generate_matrix_routes_with_defaults(tour_size, 1, is_open);
            let tour = &solution.routes[0].tour;

            for cardinality in 1..=tour_size {
                let max_preserved = tour_size - cardinality;
                for preserved in usize::from(max_preserved > 0)..=max_preserved {
                    let mut reals = vec![0.75; preserved.saturating_sub(1)];
                    if preserved < max_preserved {
                        reals.push(0.25);
                    }

                    for seed in 1..=tour_size {
                        let (begin, end) = lower_bounds(cardinality + preserved, tour_size, seed);
                        for start in begin..=end {
                            for split in start..start + cardinality {
                                let random: Arc<dyn Random> = Arc::new(CheckedRandom(FakeRandom::new(
                                    vec![start as i32, split as i32],
                                    reals.clone(),
                                )));
                                let selected =
                                    preserved_string((tour, seed), cardinality, 0.5, &random).collect::<Vec<_>>();

                                assert_eq!(selected.len(), cardinality);
                                assert!(selected.contains(&tour.get(seed).unwrap().retrieve_job().unwrap()));
                                assert!(selected.iter().all(|job| tour.has_job(job)));
                            }
                        }
                    }
                }
            }
        }
    }
}

// These tests enumerate valid random choices: accepting an out-of-range fixture would hide bad bounds.
struct CheckedRandom(FakeRandom);

impl Random for CheckedRandom {
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        let value = self.0.uniform_int(min, max);
        assert!((min..=max).contains(&value), "{value} is outside [{min}, {max}]");
        value
    }

    fn uniform_real(&self, min: Float, max: Float) -> Float {
        let value = self.0.uniform_real(min, max);
        assert!((min..max).contains(&value), "{value} is outside [{min}, {max})");
        value
    }

    fn is_head_not_tails(&self) -> bool {
        self.uniform_int(1, 2) == 1
    }

    fn is_hit(&self, probability: Float) -> bool {
        self.uniform_real(0., 1.) < probability
    }

    fn weighted(&self, weights: &[usize]) -> usize {
        self.uniform_int(0, weights.len() as i32 - 1) as usize
    }

    fn get_rng(&self) -> RandomGen {
        self.0.get_rng()
    }
}
