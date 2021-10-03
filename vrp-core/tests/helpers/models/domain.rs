use crate::construction::constraints::{ConstraintPipeline, TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::construction::heuristics::{InsertionContext, RegistryContext, SolutionContext};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::create_route_context_with_activities;
use crate::models::common::IdDimension;
use crate::models::examples::create_example_problem;
use crate::models::problem::{Fleet, Job, Jobs, ObjectiveCost};
use crate::models::solution::Registry;
use crate::models::{Problem, Solution};
use crate::utils::{DefaultRandom, Environment, Random};
use std::sync::Arc;

pub fn test_random() -> Arc<dyn Random + Send + Sync> {
    Arc::new(DefaultRandom::default())
}

pub fn create_empty_problem_with_constraint(constraint: ConstraintPipeline) -> Arc<Problem> {
    create_problem_with_constraint_jobs_and_fleet(constraint, vec![], test_fleet())
}

pub fn create_empty_problem() -> Arc<Problem> {
    create_empty_problem_with_constraint(ConstraintPipeline::default())
}

pub fn create_problem_with_constraint_jobs_and_fleet(
    constraint: ConstraintPipeline,
    jobs: Vec<Job>,
    fleet: Fleet,
) -> Arc<Problem> {
    let transport = TestTransportCost::new_shared();
    let fleet = Arc::new(fleet);
    let jobs = Arc::new(Jobs::new(fleet.as_ref(), jobs, &transport));
    Arc::new(Problem {
        fleet,
        jobs,
        locks: vec![],
        constraint: Arc::new(constraint),
        activity: Arc::new(TestActivityCost::default()),
        transport,
        objective: Arc::new(ObjectiveCost::default()),
        extras: Arc::new(Default::default()),
    })
}

pub fn create_empty_solution() -> Solution {
    Solution {
        registry: Registry::new(&test_fleet(), test_random()),
        routes: vec![],
        unassigned: Default::default(),
        extras: Arc::new(Default::default()),
    }
}

pub fn create_empty_solution_context() -> SolutionContext {
    SolutionContext {
        required: vec![],
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes: vec![],
        registry: RegistryContext::new(Registry::new(&test_fleet(), test_random())),
        state: Default::default(),
    }
}

pub fn create_empty_insertion_context() -> InsertionContext {
    InsertionContext {
        problem: create_empty_problem(),
        solution: create_empty_solution_context(),
        environment: Arc::new(Environment::default()),
    }
}

/// Creates a simple insertion context with given distance and amount of unassigned jobs.
pub fn create_simple_insertion_ctx(distance: f64, unassigned: usize) -> InsertionContext {
    let problem = create_example_problem();

    let mut insertion_ctx = create_empty_insertion_context();

    let mut route_ctx = create_route_context_with_activities(problem.fleet.as_ref(), "v1", vec![]);

    route_ctx.state_mut().put_route_state(TOTAL_DISTANCE_KEY, distance);
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

pub fn get_customer_ids_from_routes_sorted(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    let mut result = get_customer_ids_from_routes(insertion_ctx);
    result.sort();
    result
}

pub fn get_sorted_customer_ids_from_jobs(jobs: &[Job]) -> Vec<String> {
    let mut ids = get_customer_ids_from_jobs(jobs);
    ids.sort();
    ids
}

pub fn get_customer_ids_from_jobs(jobs: &[Job]) -> Vec<String> {
    jobs.iter().map(|job| get_customer_id(&job)).collect()
}

pub fn get_customer_ids_from_routes(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(|rc| {
            rc.route
                .tour
                .all_activities()
                .filter(|a| a.job.is_some())
                .map(|a| a.retrieve_job().unwrap())
                .map(|job| get_customer_id(&job))
                .collect::<Vec<String>>()
        })
        .collect()
}

pub fn get_customer_ids_from_unassigned(insertion_ctx: &InsertionContext) -> Vec<String> {
    let mut job_ids = insertion_ctx.solution.unassigned.iter().map(|(job, _)| get_customer_id(job)).collect::<Vec<_>>();

    job_ids.sort();

    job_ids
}

pub fn get_customer_id(job: &Job) -> String {
    job.dimens().get_id().unwrap().clone()
}
