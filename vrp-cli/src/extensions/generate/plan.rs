#[cfg(test)]
#[path = "../../../tests/unit/extensions/generate/plan_test.rs"]
mod plan_test;

use vrp_core::utils::{DefaultRandom, Random};
use vrp_pragmatic::format::problem::{Job, JobPlace, JobTask, Plan, Problem};
use vrp_pragmatic::format::Location;

/// Generates a new plan for given problem with amount of jobs specified by`jobs_size` and
/// bounding box of size `area_size` (half size in meters). When not specified, jobs bounding
/// box is used.
pub fn generate_plan(problem_proto: &Problem, job_size: usize, area_size: Option<f64>) -> Result<Plan, String> {
    let rnd = DefaultRandom::default();

    let bounding_box = if let Some(area_size) = area_size {
        if area_size > 0. {
            get_bounding_box_from_size(&problem_proto.plan, area_size)
        } else {
            return Err("area size must be positive".to_string());
        }
    } else {
        get_bounding_box_from_plan(&problem_proto.plan)
    };

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
                        .map(|_| JobPlace {
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

    let jobs = (1..=job_size)
        .map(|job_idx| {
            let job_proto = get_random_item(problem_proto.plan.jobs.as_slice(), &rnd).unwrap();

            // TODO implement more sophisticated logic for jobs with pickup and delivery
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

    Ok(Plan { jobs, relations: None })
}

fn get_bounding_box_from_plan(plan: &Plan) -> (Location, Location) {
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

fn get_bounding_box_from_size(plan: &Plan, area_size: f64) -> (Location, Location) {
    const WGS84_A: f64 = 6378137.0;
    const WGS84_B: f64 = 6356752.3;
    let deg_to_rad = |deg| std::f64::consts::PI * deg / 180.;
    let rad_to_deg = |rad| 180. * rad / std::f64::consts::PI;

    let (min, max) = get_bounding_box_from_plan(plan);
    let center_lat = min.lat + (max.lat - min.lat) / 2.;
    let center_lng = min.lng + (max.lng - min.lng) / 2.;

    let lat = deg_to_rad(center_lat);
    let lng = deg_to_rad(center_lng);

    // NOTE copied from pragmatic
    let an = WGS84_A * WGS84_A * lat.cos();
    let bn = WGS84_B * WGS84_B * lat.sin();
    let ad = WGS84_A * lat.cos();
    let bd = WGS84_B * lat.sin();

    let half_size = area_size;

    let radius = ((an * an + bn * bn) / (ad * ad + bd * bd)).sqrt();
    let pradius = radius * lat.cos();

    let lat_min = rad_to_deg(lat - half_size / radius);
    let lat_max = rad_to_deg(lat + half_size / radius);
    let lon_min = rad_to_deg(lng - half_size / pradius);
    let lon_max = rad_to_deg(lng + half_size / pradius);

    (Location { lat: lat_min, lng: lon_min }, Location { lat: lat_max, lng: lon_max })
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
    let lat = rnd.uniform_real(bounding_box.0.lat, bounding_box.1.lat);
    let lng = rnd.uniform_real(bounding_box.0.lng, bounding_box.1.lng);

    Location { lat, lng }
}
