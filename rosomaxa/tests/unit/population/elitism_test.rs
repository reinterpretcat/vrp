use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;

fn get_best_fitness(population: &Elitism<VectorFitness, VectorObjective, VectorSolution>) -> f64 {
    population.ranked().next().unwrap().fitness()
}

fn get_all_fitness(population: &Elitism<VectorFitness, VectorObjective, VectorSolution>) -> Vec<f64> {
    population.ranked().map(|s| s.fitness()).collect()
}

fn create_objective_population(
    max_population_size: usize,
    selection_size: usize,
) -> (Arc<VectorObjective>, Elitism<VectorFitness, VectorObjective, VectorSolution>) {
    let objective = create_example_objective();
    let population =
        Elitism::<_, _, _>::new(objective.clone(), Environment::default().random, max_population_size, selection_size);

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
