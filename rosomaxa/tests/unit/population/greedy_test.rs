use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;

fn get_best_fitness(population: &Greedy<VectorObjective, VectorSolution>) -> f64 {
    population.ranked().next().unwrap().0.fitness()
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

    population.add(VectorSolution::new(vec![-2., -2.], objective));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 401.);
}

#[test]
fn can_format_empty_population() {
    let population = Greedy::<_, _>::new(create_example_objective(), 1, None);

    let formatted = format!("{}", population);

    assert_eq!(formatted, "[]")
}

#[test]
fn can_format_filled_population() {
    let objective = create_example_objective();
    let solution = VectorSolution::new(vec![-1., -1.], objective.clone());
    let population = Greedy::<_, _>::new(objective, 1, Some(solution));

    let formatted = format!("{}", population);

    assert_eq!(formatted, "[404.0000000]")
}

#[test]
fn can_select_when_empty() {
    let objective = create_example_objective();

    let population = Greedy::<_, _>::new(objective, 1, None);

    assert_eq!(population.select().count(), 0);
    assert_eq!(population.all().count(), 0);
}

#[test]
fn can_compare_individuals() {
    let objective = create_example_objective();
    let create_individual = |data: Vec<f64>| VectorSolution::new(data, objective.clone());
    let population = Greedy::<_, _>::new(objective.clone(), 1, None);

    assert_eq!(population.cmp(&create_individual(vec![-1., -1.]), &create_individual(vec![-1., -1.])), Ordering::Equal);
    assert_eq!(population.cmp(&create_individual(vec![0., 0.]), &create_individual(vec![-1., -1.])), Ordering::Less);
    assert_eq!(population.cmp(&create_individual(vec![-1., -1.]), &create_individual(vec![0., 0.])), Ordering::Greater);
}
