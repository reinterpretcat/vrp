use super::*;
use crate::helpers::algorithms::nsga2::*;
use std::f64::consts::PI;
use std::sync::Arc;

fn fitness(individual: &Vec<f64>) -> Vec<f64> {
    const SCALE: f64 = 10.;

    let r = individual[0];
    let h = individual[1];

    let sh = (r * r + h * h).sqrt();

    let s = (PI * r * sh) * SCALE;
    let t = PI * r * (r + sh) * SCALE;

    vec![s.round(), t.round()]
}

#[test]
fn can_use_select_and_rank() {
    let population = vec![
        vec![10.0, 19.61],
        vec![4.99, 5.10],
        vec![6.09, 0.79],
        vec![6.91, 10.62],
        vec![5.21, 18.87],
        vec![7.90, 8.98],
        vec![9.84, 0.78],
        vec![4.96, 0.60],
        vec![6.24, 19.66],
        vec![6.90, 15.09],
        vec![5.20, 18.86],
        vec![7.89, 8.97],
    ];
    let mo = SliceMultiObjective::new(vec![
        Arc::new(SliceDimensionObjective::new(0)),
        Arc::new(SliceDimensionObjective::new(1)),
    ]);

    // rate population (calculate fitness)
    let rated_population = population.iter().map(fitness).collect::<Vec<_>>();
    let ranked_population = select_and_rank(&rated_population, 7, &mo);

    let results = ranked_population.iter().map(|s| vec![s.index, s.rank]).collect::<Vec<_>>();

    assert_eq!(results.len(), 7);

    assert_eq!(results[0], &[7, 0]);

    assert_eq!(results[1], &[1, 1]);

    assert_eq!(results[2], &[2, 2]);

    assert_eq!(results[3], &[10, 3]);
    assert_eq!(results[4], &[3, 3]);

    assert_eq!(results[5], &[4, 4]);
    assert_eq!(results[6], &[11, 4]);
}

parameterized_test! {can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective, solutions, {
        can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective_impl(solutions);
}}

can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective! {
        case01: &[
            vec![6., 6., 101.], vec![7., 5., 102.], vec![5., 6., 103.], vec![8., 8., 108.], vec![9., 9., 109.],
        ],
        case02: &[
            vec![2., 3., 101.], vec![4., 3., 103.], vec![2., 4., 104.], vec![3., 4., 102.],
        ],
        case03: &[
            vec![2., 5., 102.], vec![3., 5., 101.], vec![2., 5., 102.],
        ],
        case04: &[
            vec![164., 5., 407.451], vec![166., 5., 393.545], vec![166., 5., 395.197],
            vec![166., 5., 395.558], vec![164., 5., 407.451], vec![164., 5., 407.5]
        ],
}

fn can_use_select_and_rank_with_non_transient_relationship_by_hierarchical_objective_impl(solutions: &[Vec<f64>]) {
    let ranked = select_and_rank(
        solutions,
        solutions.len(),
        &SliceHierarchicalObjective::new(
            vec![Arc::new(SliceDimensionObjective::new(0)), Arc::new(SliceDimensionObjective::new(1))],
            vec![Arc::new(SliceDimensionObjective::new(2))],
        ),
    );

    assert_eq!(ranked.len(), solutions.len())
}
