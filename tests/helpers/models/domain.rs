use crate::construction::constraints::ConstraintPipeline;
use crate::construction::states::{InsertionContext, InsertionProgress, SolutionContext};
use crate::helpers::models::problem::{test_driver, test_vehicle, TestActivityCost, TestTransportCost, test_fleet};
use crate::models::common::IdDimension;
use crate::models::problem::{Fleet, Job, Jobs};
use crate::models::solution::Registry;
use crate::models::{Problem, Solution};
use crate::utils::DefaultRandom;
use std::borrow::Borrow;
use std::sync::Arc;

pub fn create_empty_problem_with_constraint(constraint: ConstraintPipeline) -> Arc<Problem> {
    let transport = Arc::new(TestTransportCost::new());
    let fleet = Arc::new(test_fleet());
    let jobs = Arc::new(Jobs::new(fleet.borrow(), vec![], transport.as_ref()));
    Arc::new(Problem {
        fleet,
        jobs,
        locks: vec![],
        constraint: Arc::new(constraint),
        activity: Arc::new(TestActivityCost::new()),
        transport,
        extras: Arc::new(Default::default()),
    })
}

pub fn create_empty_problem() -> Arc<Problem> {
    create_empty_problem_with_constraint(ConstraintPipeline::new())
}

pub fn create_empty_solution() -> Solution {
    Solution {
        registry: Registry::new(&Fleet::new(vec![test_driver()], vec![test_vehicle(0)])),
        routes: vec![],
        unassigned: Default::default(),
        extras: Arc::new(Default::default()),
    }
}

pub fn create_empty_insertion_context() -> InsertionContext {
    InsertionContext {
        progress: InsertionProgress { cost: None, completeness: 0.0, total: 0 },
        problem: create_empty_problem(),
        solution: SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned: Default::default(),
            routes: vec![],
            registry: Registry::new(&Fleet::new(vec![test_driver()], vec![test_vehicle(0)])),
        },
        locked: Arc::new(Default::default()),
        random: Arc::new(DefaultRandom::new()),
    }
}

pub fn get_customer_ids_from_routes_sorted(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    let mut result = get_customer_ids_from_routes(insertion_ctx);
    result.sort();
    result
}

pub fn get_sorted_customer_ids_from_jobs(jobs: &Vec<Arc<Job>>) -> Vec<String> {
    let mut ids = jobs.iter().map(|job| get_customer_id(&job)).collect::<Vec<String>>();
    ids.sort();
    ids
}

pub fn get_customer_ids_from_routes(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(|rc| {
            rc.route
                .read()
                .unwrap()
                .tour
                .all_activities()
                .filter(|a| a.job.is_some())
                .map(|a| a.retrieve_job().unwrap())
                .map(|job| get_customer_id(&job))
                .collect::<Vec<String>>()
        })
        .collect()
}

pub fn get_customer_id(job: &Arc<Job>) -> String {
    match job.as_ref() {
        Job::Single(job) => &job.dimens,
        Job::Multi(job) => &job.dimens,
    }
    .get_id()
    .unwrap()
    .clone()
}
