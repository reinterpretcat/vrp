use super::*;
use crate::helpers::models::domain::create_simple_insertion_ctx;
use crate::models::examples::create_example_problem;

fn get_best_fitness(population: &Greedy) -> f64 {
    population.problem.objective.fitness(population.ranked().next().unwrap().0)
}

#[test]
fn can_keep_best_solution() {
    let problem = create_example_problem();
    let mut population = Greedy::new(problem.clone(), None);

    population.add(create_simple_insertion_ctx(100., 0));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 100.);

    population.add(create_simple_insertion_ctx(90., 0));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 90.);

    population.add(create_simple_insertion_ctx(120., 0));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 90.);
}
