#[cfg(test)]
#[path = "../../../tests/unit/extensions/generate/plan_test.rs"]
mod plan_test;

use vrp_core::utils::{DefaultRandom, Random};
use vrp_pragmatic::format::problem::{Job, JobPlace, JobTask, Plan, Problem};
use vrp_pragmatic::format::Location;

/// Generates a new plan for given problem.
pub fn generate_plan(problem_proto: &Problem, job_count: usize) -> Plan {
    let rnd = DefaultRandom::default();

    let bounding_box = get_plan_bounding_box(&problem_proto.plan);
    let time_windows = get_plan_time_windows(&problem_proto.plan);
    let demands = get_plan_demands(&problem_proto.plan);
    let durations = get_plan_durations(&problem_proto.plan);

    let generate_tasks = |tasks: &Option<Vec<JobTask>>, keep_original_demand: bool| {
        tasks.as_ref().map(|tasks| {
            tasks
                .iter()
                .map(|task| JobTask {
                    places: task
                        .places
                        .iter()
                        .map(|place| JobPlace {
                            location: get_random_location(&bounding_box, &rnd),
                            duration: get_random_item(durations.as_slice(), &rnd).cloned().unwrap(),
                            times: get_random_item(time_windows.as_slice(), &rnd).cloned(),
                        })
                        .collect(),
                    demand: if keep_original_demand {
                        task.demand.clone()
                    } else {
                        Some(get_random_item(demands.as_slice(), &rnd).cloned().unwrap())
                    },

                    tag: None,
                })
                .collect::<Vec<_>>()
        })
    };

    let jobs = (1..=job_count)
        .map(|job_idx| {
            let job_proto = get_random_item(problem_proto.plan.jobs.as_slice(), &rnd).unwrap();

            let keep_original_demand = job_proto.pickups.as_ref().map_or(false, |t| t.len() > 0)
                && job_proto.deliveries.as_ref().map_or(false, |t| t.len() > 0);

            Job {
                id: format!("job{}", job_idx),
                pickups: generate_tasks(&job_proto.pickups, keep_original_demand),
                deliveries: generate_tasks(&job_proto.deliveries, keep_original_demand),
                replacements: generate_tasks(&job_proto.replacements, false),
                services: generate_tasks(&job_proto.services, true),
                priority: job_proto.priority,
                skills: job_proto.skills.clone(),
            }
        })
        .collect();

    Plan { jobs, relations: None }
}

fn get_plan_bounding_box(plan: &Plan) -> (Location, Location) {
    let mut lat_min = std::f64::MAX;
    let mut lat_max = std::f64::MIN;
    let mut lng_min = std::f64::MAX;
    let mut lng_max = std::f64::MIN;

    get_plan_places(&plan).map(|job_place| &job_place.location).cloned().for_each(|Location { lat, lng }| {
        lat_min = lat_min.min(lat);
        lat_max = lat_max.max(lat);

        lng_min = lng_min.min(lng);
        lng_max = lng_max.max(lng);
    });

    (Location { lat: lat_min, lng: lng_min }, Location { lat: lat_max, lng: lng_max })
}

fn get_plan_time_windows(plan: &Plan) -> Vec<Vec<Vec<String>>> {
    get_plan_places(&plan).flat_map(|job_place| job_place.times.iter()).cloned().collect()
}

fn get_plan_demands(plan: &Plan) -> Vec<Vec<i32>> {
    plan.jobs
        .iter()
        .flat_map(|job| get_job_tasks(job))
        .filter_map(|job_task| job_task.demand.as_ref())
        .cloned()
        .collect()
}

fn get_plan_durations(plan: &Plan) -> Vec<f64> {
    get_plan_places(&plan).map(|job_place| job_place.duration).collect()
}

fn get_plan_places(plan: &Plan) -> impl Iterator<Item = &JobPlace> {
    plan.jobs.iter().flat_map(|job| get_job_tasks(job)).flat_map(|job_task| job_task.places.iter())
}

fn get_job_tasks(job: &Job) -> impl Iterator<Item = &JobTask> {
    job.pickups
        .iter()
        .flat_map(|tasks| tasks.iter())
        .chain(job.deliveries.iter().flat_map(|tasks| tasks.iter()))
        .chain(job.replacements.iter().flat_map(|tasks| tasks.iter()))
        .chain(job.services.iter().flat_map(|tasks| tasks.iter()))
}

fn get_random_item<'a, T>(items: &'a [T], rnd: &DefaultRandom) -> Option<&'a T> {
    if items.is_empty() {
        return None;
    }

    let idx = rnd.uniform_int(0, items.len() as i32 - 1) as usize;
    items.get(idx)
}

fn get_random_location(bounding_box: &(Location, Location), rnd: &DefaultRandom) -> Location {
    unimplemented!()
}
