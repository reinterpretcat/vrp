use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;

fn get_best_fitness(population: &Elitism<VectorObjective, VectorSolution>) -> f64 {
    population.ranked().next().unwrap().fitness()
}

fn get_all_fitness(population: &Elitism<VectorObjective, VectorSolution>) -> Vec<f64> {
    population.ranked().map(|s| s.fitness()).collect()
}

fn create_objective_population(
    max_population_size: usize,
    selection_size: usize,
) -> (Arc<VectorObjective>, Elitism<VectorObjective, VectorSolution>) {
    let objective = create_example_objective();
    let population =
        Elitism::<_, _>::new(objective.clone(), Environment::default().random, max_population_size, selection_size);

    (objective, population)
}

#[test]
fn can_maintain_best_order() {
    let (objective, mut population) = create_objective_population(3, 1);

    population.add(VectorSolution::new(vec![0.5, 0.5], objective.clone()));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 6.5);

    population.add(VectorSolution::new(vec![0., 0.], objective.clone()));
    assert_eq!(population.size(), 2);
    assert_eq!(get_best_fitness(&population), 1.);

    population.add(VectorSolution::new(vec![-0.5, -0.5], objective));
    assert_eq!(population.size(), 3);
    assert_eq!(get_best_fitness(&population), 1.);
    assert_eq!(get_all_fitness(&population), &[1., 6.5, 58.5]);
}

#[test]
fn can_maintain_diversity_with_one_objective() {
    let (objective, mut population) = create_objective_population(4, 1);

    population.add(VectorSolution::new(vec![0., 0.], objective.clone()));
    assert_eq!(population.size(), 1);

    population.add(VectorSolution::new(vec![0.5, 0.5], objective.clone()));
    assert_eq!(get_all_fitness(&population), &[1., 6.5]);

    population.add(VectorSolution::new(vec![0., 0.], objective.clone()));
    assert_eq!(get_all_fitness(&population), &[1., 6.5]);

    population.add(VectorSolution::new(vec![0.5, 0.5], objective.clone()));
    assert_eq!(get_all_fitness(&population), &[1., 6.5]);

    population.add(VectorSolution::new(vec![0.5, 0.5], objective.clone()));
    assert_eq!(get_all_fitness(&population), &[1., 6.5]);

    population.add(VectorSolution::new(vec![-0.5, -0.5], objective.clone()));
    assert_eq!(get_all_fitness(&population), &[1., 6.5, 58.5]);

    population.add(VectorSolution::new(vec![1., 1.], objective.clone()));
    assert_eq!(get_all_fitness(&population), &[0., 1., 6.5, 58.5]);

    population.add(VectorSolution::new(vec![0.5, 0.5], objective));
    assert_eq!(get_all_fitness(&population), &[0., 1., 6.5, 58.5])
}

#[test]
fn can_check_improvement() {
    let (objective, mut population) = create_objective_population(4, 1);

    assert!(population.add(VectorSolution::new(vec![-1., -1.], objective.clone())));
    assert!(!population.add(VectorSolution::new(vec![-1., -1.], objective.clone())));
    assert!(!population.add(VectorSolution::new(vec![-2., -2.], objective.clone())));
    assert!(!population.add(VectorSolution::new(vec![-1., -1.], objective.clone())));
    assert!(population.add(VectorSolution::new(vec![0.5, 0.5], objective.clone())));
    assert!(!population.add(VectorSolution::new(vec![2., 2.], objective.clone())));
    assert!(!population.add(VectorSolution::new(vec![-0.5, -0.5], objective.clone())));
    assert!(population.add(VectorSolution::new(vec![0., 0.], objective.clone())));

    assert!(!population.add_all(vec![
        VectorSolution::new(vec![-1., -1.], objective.clone()),
        VectorSolution::new(vec![-2., -2.], objective.clone()),
        VectorSolution::new(vec![0., 0.], objective.clone()),
    ],));
    assert!(population.add_all(vec![
        VectorSolution::new(vec![-1., -1.], objective.clone()),
        VectorSolution::new(vec![1., 1.], objective.clone()),
        VectorSolution::new(vec![0., 0.], objective),
    ]));
}

#[test]
fn can_select_individuals() {
    let (objective, mut population) = create_objective_population(4, 3);

    population.add_all(vec![
        VectorSolution::new(vec![-1., -1.], objective.clone()),
        VectorSolution::new(vec![1., 1.], objective.clone()),
        VectorSolution::new(vec![0., 0.], objective.clone()),
        VectorSolution::new(vec![-2., -2.], objective),
    ]);

    let parents = population.select().count();

    assert_eq!(parents, 3);
}

#[test]
fn can_handle_empty() {
    let (_, mut population) = create_objective_population(4, 3);

    population.add_all(vec![]);

    assert!(population.select().next().is_none());
}

parameterized_test! {can_detect_improvement, (new_individuals, expected), {
    can_detect_improvement_impl(new_individuals, expected);
}}

can_detect_improvement! {
    case_01_add_one_same: (vec![vec![0.5, 0.5]], false),
    case_02_add_one_worse: (vec![vec![0.7, 0.5]], false),
    case_03_add_one_worse: (vec![vec![0.5, 0.7]], false),

    case_04_add_one_better: (vec![vec![0.4, 0.5]], true),
    case_05_add_one_better: (vec![vec![0.5, 0.4]], true),

    case_06_add_more_worse: (vec![vec![0.5, 0.7], vec![0.6, 0.6]], false),
    case_07_add_more_same: (vec![vec![0.5, 0.5], vec![0.5, 0.5]], false),

    case_08_add_more_mixed: (vec![vec![0.5, 0.4], vec![0.5, 0.7], vec![0.5, 0.5]], true),
}

fn can_detect_improvement_impl(new_individuals: Vec<Vec<f64>>, expected: bool) {
    let objective = Arc::new(VectorObjective::new(
        Arc::new(|data| data.iter().map(|&a| a * a).sum::<f64>().sqrt()),
        Arc::new(|data: &[f64]| data.to_vec()),
    ));
    let mut population = Elitism::<_, _>::new(objective.clone(), Environment::default().random, 2, 1);
    population.add(VectorSolution::new(vec![0.5, 0.5], objective.clone()));

    assert_eq!(
        population.add_with_iter(new_individuals.into_iter().map(|data| VectorSolution::new(data, objective.clone()))),
        expected
    )
}
