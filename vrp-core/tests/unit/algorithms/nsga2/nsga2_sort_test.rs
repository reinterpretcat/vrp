use super::*;
use crate::helpers::algorithms::nsga2::*;
use std::f64::consts::PI;

struct Individual(pub f64, pub f64);

fn fitness(individual: &Individual) -> Pair {
    const SCALE: f64 = 10.;

    let Individual(r, h) = *individual;

    let sh = (r * r + h * h).sqrt();

    let s = (PI * r * sh) * SCALE;
    let t = PI * r * (r + sh) * SCALE;

    Pair(s.round() as usize, t.round() as usize)
}

#[test]
fn can_use_select_and_rank() {
    let population = vec![
        Individual(10.0, 19.61),
        Individual(4.99, 5.10),
        Individual(6.09, 0.79),
        Individual(6.91, 10.62),
        Individual(5.21, 18.87),
        Individual(7.90, 8.98),
        Individual(9.84, 0.78),
        Individual(4.96, 0.60),
        Individual(6.24, 19.66),
        Individual(6.90, 15.09),
        Individual(5.20, 18.86),
        Individual(7.89, 8.97),
    ];
    let mo = PairMultiObjective::new(vec![Box::new(PairObjective1), Box::new(PairObjective2)]);

    // rate population (calculate fitness)
    let rated_population = population.iter().map(fitness).collect::<Vec<_>>();
    let ranked_population = select_and_rank(&rated_population, 7, &mo);

    let results = ranked_population.iter().map(|s| (s.index, s.rank)).collect::<Vec<_>>();

    assert_eq!(results.len(), 7);

    assert_eq!(results[0], (7, 0));

    assert_eq!(results[1], (1, 1));

    assert_eq!(results[2], (2, 2));

    assert_eq!(results[3], (10, 3));
    assert_eq!(results[4], (3, 3));

    assert_eq!(results[5], (4, 4));
    assert_eq!(results[6], (11, 4));
}

parameterized_test! {can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective, solutions, {
        can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective_impl(solutions);
}}

can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective! {
        case01: &[
            Triple(6., 6., 101.), Triple(7., 5., 102.), Triple(5., 6., 103.), Triple(8., 8., 108.), Triple(9., 9., 109.),
        ],
        case02: &[
            Triple(2., 3., 101.), Triple(4., 3., 103.), Triple(2., 4., 104.), Triple(3., 4., 102.),
        ],
        case03: &[
            Triple(2., 5., 102.), Triple(3., 5., 101.), Triple(2., 5., 102.),
        ],
        case04: &[
            Triple(164., 5., 407.451), Triple(166., 5., 393.545), Triple(166., 5., 395.197),
            Triple(166., 5., 395.558), Triple(164., 5., 407.451), Triple(164., 5., 407.5)
        ],
}

fn can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective_impl(solutions: &[Triple]) {
    let ranked = select_and_rank(
        solutions,
        solutions.len(),
        &TupleHierarchicalObjective::new(
            vec![Box::new(TripleObjective1), Box::new(TripleObjective2)],
            vec![Box::new(TripleObjective3)],
        ),
    );

    assert_eq!(ranked.len(), solutions.len())
}
