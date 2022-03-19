use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;

fn get_best_fitness(population: &Greedy<VectorObjective, VectorSolution>) -> f64 {
    population.objective.fitness(population.ranked().next().unwrap().0)
}

#[test]
fn can_keep_best_solution() {
    let objective = create_example_objective();
    let mut population = Greedy::<_, _>::new(objective.clone(), 1, None);

    population.add(VectorSolution::new(vec![-1., -1.], objective.clone()));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 404.);

    population.add(VectorSolution::new(vec![2., 2.], objective.clone()));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 401.);

    population.add(VectorSolution::new(vec![-2., -2.], objective.clone()));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 401.);
}
