use crate::algorithms::nsga2::Objective;
use crate::construction::constraints::{TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
use crate::helpers::models::domain::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::create_route_context_with_activities;
use crate::models::Problem;
use crate::solver::{DominancePopulation, Individual, Population};
use std::sync::Arc;

fn create_problem() -> Arc<Problem> {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver_with_costs(empty_costs()))
        .add_vehicle(test_vehicle_with_id("v1"))
        .build();
    create_empty_problem_with_constraint_and_fleet(create_constraint_pipeline_with_transport(), fleet)
}

fn create_individual(problem: &Arc<Problem>, fitness: f64) -> Individual {
    let mut insertion_ctx = create_empty_insertion_context();

    let mut route_ctx = create_route_context_with_activities(problem.fleet.as_ref(), "v1", vec![]);

    route_ctx.state_mut().put_route_state(TOTAL_DISTANCE_KEY, fitness);
    route_ctx.state_mut().put_route_state(TOTAL_DURATION_KEY, 0.);

    insertion_ctx.solution.routes.push(route_ctx);

    insertion_ctx
}

fn get_best_fitness(population: &DominancePopulation) -> f64 {
    population.problem.objective.fitness(population.ranked().next().unwrap().0)
}

fn get_all_fitness(population: &DominancePopulation) -> Vec<f64> {
    population.ranked().map(|(individual, _)| population.problem.objective.fitness(individual)).collect()
}

#[test]
fn can_maintain_best_order() {
    let problem = create_problem();
    let mut population = DominancePopulation::new(problem.clone(), 3);

    population.add(create_individual(&problem, 100.));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 100.);

    population.add(create_individual(&problem, 90.));
    assert_eq!(population.size(), 2);
    assert_eq!(get_best_fitness(&population), 90.);

    population.add(create_individual(&problem, 120.));
    assert_eq!(population.size(), 3);
    assert_eq!(get_best_fitness(&population), 90.);
    assert_eq!(get_all_fitness(&population), &[90., 100., 120.]);
}

#[test]
fn can_maintain_diversity() {
    let problem = create_problem();
    let mut population = DominancePopulation::new(problem.clone(), 4);

    population.add(create_individual(&problem, 100.));
    assert_eq!(population.size(), 1);

    population.add(create_individual(&problem, 200.));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_individual(&problem, 100.));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_individual(&problem, 200.));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_individual(&problem, 300.));
    assert_eq!(get_all_fitness(&population), &[100., 200., 300.]);

    population.add(create_individual(&problem, 50.));
    assert_eq!(get_all_fitness(&population), &[50., 100., 200., 300.]);

    population.add(create_individual(&problem, 200.));
    assert_eq!(get_all_fitness(&population), &[50., 100., 200., 300.]);
}

#[test]
fn can_check_improvement() {
    let problem = create_problem();
    let mut population = DominancePopulation::new(problem.clone(), 4);

    assert_eq!(true, population.add(create_individual(&problem, 100.)));
    assert_eq!(false, population.add(create_individual(&problem, 100.)));
    assert_eq!(false, population.add(create_individual(&problem, 200.)));
    assert_eq!(false, population.add(create_individual(&problem, 100.)));
    assert_eq!(true, population.add(create_individual(&problem, 50.)));
    assert_eq!(false, population.add(create_individual(&problem, 90.)));
    assert_eq!(false, population.add(create_individual(&problem, 60.)));
    assert_eq!(true, population.add(create_individual(&problem, 20.)));

    assert_eq!(
        false,
        population.add_all(vec![
            create_individual(&problem, 100.),
            create_individual(&problem, 110.),
            create_individual(&problem, 20.),
        ])
    );
    assert_eq!(
        true,
        population.add_all(vec![
            create_individual(&problem, 100.),
            create_individual(&problem, 10.),
            create_individual(&problem, 20.),
        ])
    );

    assert_eq!(false, population.add(create_individual(&problem, 20.)));
    assert_eq!(true, population.add(create_individual(&problem, 5.)));
}
