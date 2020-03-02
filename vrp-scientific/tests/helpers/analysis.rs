use vrp_core::construction::constraints::{CapacityDimension, Demand, DemandDimension};
use vrp_core::construction::states::InsertionContext;
use vrp_core::models::common::IdDimension;
use vrp_core::models::problem::Job;
use vrp_core::models::Problem;

pub fn get_customer_id(job: &Job) -> String {
    get_job_id(job).to_owned()
}

pub fn get_job_id(job: &Job) -> &String {
    job.dimens().get_id().unwrap()
}

pub fn get_customer_ids_from_routes_sorted(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    let mut result = get_customer_ids_from_routes(insertion_ctx);
    result.sort();
    result
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

pub fn get_vehicle_capacity(problem: &Problem) -> i32 {
    *problem.fleet.vehicles.iter().next().unwrap().dimens.get_capacity().unwrap()
}

pub fn get_job_time_windows(problem: &Problem) -> Vec<(f64, f64)> {
    problem
        .jobs
        .all()
        .map(|j| match j {
            Job::Single(j) => j
                .places
                .first()
                .unwrap()
                .times
                .first()
                .map(|span| span.as_time_window().unwrap())
                .map(|tw| (tw.start, tw.end))
                .unwrap(),
            _ => panic!(),
        })
        .collect()
}

pub fn get_job_ids(problem: &Problem) -> Vec<String> {
    problem.jobs.all().map(|j| get_job_id(&j).to_owned()).collect()
}

pub fn get_job_demands(problem: &Problem) -> Vec<i32> {
    problem.jobs.all().map(|j| get_job_simple_demand(&j).delivery.0).collect()
}

pub fn get_job_durations(problem: &Problem) -> Vec<f64> {
    problem
        .jobs
        .all()
        .map(|j| match j {
            Job::Single(j) => j.places.first().unwrap().duration,
            _ => panic!(),
        })
        .collect()
}

pub fn get_job_simple_demand(job: &Job) -> &Demand<i32> {
    match job {
        Job::Single(single) => &single.dimens,
        Job::Multi(multi) => &multi.dimens,
    }
    .get_demand()
    .unwrap()
}
