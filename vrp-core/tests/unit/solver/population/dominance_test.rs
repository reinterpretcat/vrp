use crate::algorithms::nsga2::Objective;
use crate::construction::constraints::{TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
use crate::helpers::models::domain::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::create_route_context_with_activities;
use crate::models::examples::create_example_problem;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::population::{DominancePopulation, Individual, Population};
use crate::solver::Statistics;
use crate::utils::{DefaultRandom, Random};
use std::sync::Arc;

fn create_problem() -> Arc<Problem> {
    let jobs = vec![Job::Single(Arc::new(test_single()))];
    let fleet = FleetBuilder::default()
        .add_driver(test_driver_with_costs(empty_costs()))
        .add_vehicle(test_vehicle_with_id("v1"))
        .build();
    create_problem_with_constraint_jobs_and_fleet(create_constraint_pipeline_with_transport(), jobs, fleet)
}

fn create_random() -> Arc<dyn Random + Send + Sync> {
    Arc::new(DefaultRandom::default())
}

fn create_individual(problem: &Arc<Problem>, fitness: f64, unassigned: usize) -> Individual {
    let mut insertion_ctx = create_empty_insertion_context();

    let mut route_ctx = create_route_context_with_activities(problem.fleet.as_ref(), "v1", vec![]);

    route_ctx.state_mut().put_route_state(TOTAL_DISTANCE_KEY, fitness);
    route_ctx.state_mut().put_route_state(TOTAL_DURATION_KEY, 0.);

    insertion_ctx.solution.routes.push(route_ctx);

    (0..unassigned).for_each(|_| {
        insertion_ctx
            .solution
            .unassigned
            .insert(problem.jobs.all().next().clone().expect("at least one job expected"), 0);
    });

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
    let mut population = DominancePopulation::new(problem.clone(), create_random(), 3, 1);

    population.add(create_individual(&problem, 100., 0));
    assert_eq!(population.size(), 1);
    assert_eq!(get_best_fitness(&population), 100.);

    population.add(create_individual(&problem, 90., 0));
    assert_eq!(population.size(), 2);
    assert_eq!(get_best_fitness(&population), 90.);

    population.add(create_individual(&problem, 120., 0));
    assert_eq!(population.size(), 3);
    assert_eq!(get_best_fitness(&population), 90.);
    assert_eq!(get_all_fitness(&population), &[90., 100., 120.]);
}

#[test]
fn can_maintain_diversity_with_one_objective() {
    let problem = create_problem();
    let mut population = DominancePopulation::new(problem.clone(), create_random(), 4, 1);

    population.add(create_individual(&problem, 100., 0));
    assert_eq!(population.size(), 1);

    population.add(create_individual(&problem, 200., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_individual(&problem, 100., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_individual(&problem, 200., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200.]);

    population.add(create_individual(&problem, 300., 0));
    assert_eq!(get_all_fitness(&population), &[100., 200., 300.]);

    population.add(create_individual(&problem, 50., 0));
    assert_eq!(get_all_fitness(&population), &[50., 100., 200., 300.]);

    population.add(create_individual(&problem, 200., 0));
    assert_eq!(get_all_fitness(&population), &[50., 100., 200., 300.]);
}

#[test]
fn can_maintain_diversity_with_two_objectives() {
    let problem = create_problem();
    let mut population = DominancePopulation::new(problem.clone(), create_random(), 4, 1);

    population.add_all(vec![
        create_individual(&problem, 100., 0),
        create_individual(&problem, 100., 0),
        create_individual(&problem, 25., 2),
        create_individual(&problem, 100., 0),
    ]);

    assert_eq!(get_all_fitness(&population), &[100., 25.]);
}

#[test]
fn can_check_improvement() {
    let problem = create_problem();
    let mut population = DominancePopulation::new(problem.clone(), create_random(), 4, 1);

    assert_eq!(true, population.add(create_individual(&problem, 100., 0)));
    assert_eq!(false, population.add(create_individual(&problem, 100., 0)));
    assert_eq!(false, population.add(create_individual(&problem, 200., 0)));
    assert_eq!(false, population.add(create_individual(&problem, 100., 0)));
    assert_eq!(true, population.add(create_individual(&problem, 50., 0)));
    assert_eq!(false, population.add(create_individual(&problem, 90., 0)));
    assert_eq!(false, population.add(create_individual(&problem, 60., 0)));
    assert_eq!(true, population.add(create_individual(&problem, 20., 0)));

    assert_eq!(
        false,
        population.add_all(vec![
            create_individual(&problem, 100., 0),
            create_individual(&problem, 110., 0),
            create_individual(&problem, 20., 0),
        ])
    );
    assert_eq!(
        true,
        population.add_all(vec![
            create_individual(&problem, 100., 0),
            create_individual(&problem, 10., 0),
            create_individual(&problem, 20., 0),
        ])
    );

    assert_eq!(false, population.add(create_individual(&problem, 20., 0)));
    assert_eq!(true, population.add(create_individual(&problem, 5., 0)));
}

#[test]
fn can_select_individuals() {
    let problem = create_example_problem();
    let mut population = DominancePopulation::new(problem.clone(), create_random(), 4, 3);
    population.add_all(vec![
        create_empty_insertion_context(),
        create_empty_insertion_context(),
        create_empty_insertion_context(),
        create_empty_insertion_context(),
    ]);

    let parents = population.select(&Statistics::default()).collect::<Vec<_>>();

    assert_eq!(parents.len(), 3);
}
