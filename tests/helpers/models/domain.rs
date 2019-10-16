use crate::construction::constraints::ConstraintPipeline;
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use crate::models::common::IdDimension;
use crate::models::problem::{Fleet, Job, Jobs};
use crate::models::{Problem, Solution};
use std::borrow::Borrow;
use std::sync::Arc;

pub fn create_empty_problem_with_constraint(constraint: ConstraintPipeline) -> Arc<Problem> {
    let transport = Arc::new(TestTransportCost::new());
    let fleet = Arc::new(Fleet::new(vec![], vec![]));
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

pub fn get_customer_ids_from_routes_sorted(solution: &Solution) -> Vec<Vec<String>> {
    let mut result = get_customer_ids_from_routes(solution);
    result.sort();
    result
}

pub fn get_customer_ids_from_jobs(jobs: &Vec<Arc<Job>>) -> Vec<String> {
    jobs.iter().map(|job| get_customer_id(&job)).collect::<Vec<String>>()
}

pub fn get_customer_ids_from_routes(solution: &Solution) -> Vec<Vec<String>> {
    solution
        .routes
        .iter()
        .map(|r| {
            r.tour
                .all_activities()
                .filter(|a| a.job.is_some())
                .map(|a| a.retrieve_job().unwrap())
                .map(|job| get_customer_id(&job))
                .collect::<Vec<String>>()
        })
        .collect()
}

fn get_customer_id(job: &Arc<Job>) -> String {
    match job.as_ref() {
        Job::Single(job) => &job.dimens,
        Job::Multi(job) => &job.dimens,
    }
    .get_id()
    .unwrap()
    .clone()
}
