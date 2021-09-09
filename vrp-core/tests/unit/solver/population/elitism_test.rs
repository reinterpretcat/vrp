use crate::algorithms::nsga2::Objective;
use crate::helpers::models::domain::*;
use crate::models::examples::create_example_problem;
use crate::solver::population::{Elitism, Population};
use crate::utils::{DefaultRandom, Random};
use std::sync::Arc;

fn create_random() -> Arc<dyn Random + Send + Sync> {
    Arc::new(DefaultRandom::default())
}

fn get_best_fitness(population: &Elitism) -> f64 {
    population.objective.fitness(population.ranked().next().unwrap().0)
}

fn get_all_fitness(population: &Elitism) -> Vec<f64> {
    population.ranked().map(|(individual, _)| population.objective.fitness(individual)).collect()
}

#[test]
fn can_maintain_best_order() {
    let problem = create_example_problem();
    let mut population = Elitism::new(problem.objective.clone(), create_random(), 3, 1);

    population.add(create_simple_insertion_ctx(100., 0));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 100.);

    population.add(create_simple_insertion_ctx(90., 0));
    assert_eq!(population.size(), 2);
    assert_eq!(get_best_fitness(&population), 90.);

    population.add(create_simple_insertion_ctx(120., 0));
    assert_eq!(population.size(), 3);
    assert_eq!(get_best_fitness(&population), 90.);
    assert_eq!(get_all_fitness(&population), &[90., 100., 120.]);
}

#[test]
fn can_maintain_diversity_with_one_objective() {
    let problem = create_example_problem();
    let mut population = Elitism::new(problem.objective.clone(), create_random(), 4, 1);

    population.add(create_simple_insertion_ctx(100., 0));
    assert_eq!(population.size(), 1);

    population.add(create_simple_insertion_ctx(200., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_simple_insertion_ctx(100., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_simple_insertion_ctx(200., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_simple_insertion_ctx(300., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200., 300.]);

    population.add(create_simple_insertion_ctx(50., 0));
    assert_eq!(get_all_fitness(&population), &[50., 100., 200., 300.]);

    population.add(create_simple_insertion_ctx(200., 0));
    assert_eq!(get_all_fitness(&population), &[50., 100., 200., 300.]);
}

#[test]
fn can_maintain_diversity_with_two_objectives() {
    let problem = create_example_problem();
    let mut population = Elitism::new(problem.objective.clone(), create_random(), 4, 1);

    population.add_all(vec![
        create_simple_insertion_ctx(100., 0),
        create_simple_insertion_ctx(100., 0),
        create_simple_insertion_ctx(25., 2),
        create_simple_insertion_ctx(100., 0),
    ]);

    assert_eq!(get_all_fitness(&population), &[100., 25.]);
}

#[test]
fn can_check_improvement() {
    let problem = create_example_problem();
    let mut population = Elitism::new(problem.objective.clone(), create_random(), 4, 1);

    assert_eq!(true, population.add(create_simple_insertion_ctx(100., 0)));
    assert_eq!(false, population.add(create_simple_insertion_ctx(100., 0)));
    assert_eq!(false, population.add(create_simple_insertion_ctx(200., 0)));
    assert_eq!(false, population.add(create_simple_insertion_ctx(100., 0)));
    assert_eq!(true, population.add(create_simple_insertion_ctx(50., 0)));
    assert_eq!(false, population.add(create_simple_insertion_ctx(90., 0)));
    assert_eq!(false, population.add(create_simple_insertion_ctx(60., 0)));
    assert_eq!(true, population.add(create_simple_insertion_ctx(20., 0)));

    assert_eq!(
        false,
        population.add_all(vec![
            create_simple_insertion_ctx(100., 0),
            create_simple_insertion_ctx(110., 0),
            create_simple_insertion_ctx(20., 0),
        ],)
    );
    assert_eq!(
        true,
        population.add_all(vec![
            create_simple_insertion_ctx(100., 0),
            create_simple_insertion_ctx(10., 0),
            create_simple_insertion_ctx(20., 0),
        ],)
    );

    assert_eq!(false, population.add(create_simple_insertion_ctx(20., 0)));
    assert_eq!(true, population.add(create_simple_insertion_ctx(5., 0)));
}

#[test]
fn can_select_individuals() {
    let problem = create_example_problem();
    let mut population = Elitism::new(problem.objective.clone(), create_random(), 4, 3);
    population.add_all(vec![
        create_empty_insertion_context(),
        create_empty_insertion_context(),
        create_empty_insertion_context(),
        create_empty_insertion_context(),
    ]);

    let parents = population.select().collect::<Vec<_>>();

    assert_eq!(parents.len(), 3);
}

#[test]
fn can_handle_empty() {
    let problem = create_example_problem();
    let mut population = Elitism::new(problem.objective.clone(), create_random(), 4, 3);

    population.add_all(vec![]);
    let parents = population.select().collect::<Vec<_>>();

    assert!(parents.is_empty());
}
