#[cfg(test)]
#[path = "../../../tests/unit/extensions/generate/plan_test.rs"]
mod plan_test;

use super::get_random_item;
use vrp_core::utils::{DefaultRandom, Random};
use vrp_pragmatic::format::problem::{Job, JobPlace, JobTask, Plan, Problem};
use vrp_pragmatic::format::Location;

/// Generates a new plan for given problem with amount of jobs specified by`jobs_size` and
/// bounding box of size `area_size` (half size in meters). When not specified, jobs bounding
/// box is used.
pub(crate) fn generate_plan(
    problem_proto: &Problem,
    locations: Option<Vec<Location>>,
    jobs_size: usize,
    area_size: Option<f64>,
) -> Result<Plan, String> {
    let rnd = DefaultRandom::default();

    let get_location_fn = get_location_fn(problem_proto, locations, area_size)?;

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
                            location: get_location_fn(&rnd),
                            duration: get_random_item(durations.as_slice(), &rnd).cloned().unwrap(),
                            times: get_random_item(time_windows.as_slice(), &rnd).cloned(),
                            tag: place.tag.clone(),
                        })
                        .collect(),
                    demand: if keep_original_demand {
                        task.demand.clone()
                    } else {
                        get_random_item(demands.as_slice(), &rnd).cloned()
                    },
                    order: task.order,
                })
                .collect::<Vec<_>>()
        })
    };

    let jobs = (1..=jobs_size)
        .map(|job_idx| {
            let job_proto = get_random_item(problem_proto.plan.jobs.as_slice(), &rnd).unwrap();

            // TODO implement more sophisticated logic for jobs with pickup and delivery
            let keep_original_demand = job_proto.pickups.as_ref().map_or(false, |t| !t.is_empty())
                && job_proto.deliveries.as_ref().map_or(false, |t| !t.is_empty());

            Job {
                id: format!("job{}", job_idx),
                pickups: generate_tasks(&job_proto.pickups, keep_original_demand),
                deliveries: generate_tasks(&job_proto.deliveries, keep_original_demand),
                replacements: generate_tasks(&job_proto.replacements, false),
                services: generate_tasks(&job_proto.services, true),
                skills: job_proto.skills.clone(),
                value: job_proto.value,
                group: job_proto.group.clone(),
                compatibility: job_proto.compatibility.clone(),
            }
        })
        .collect();

    Ok(Plan { jobs, relations: None, clustering: None })
}

type LocationFn = Box<dyn Fn(&DefaultRandom) -> Location>;

fn get_location_fn(
    problem_proto: &Problem,
    locations: Option<Vec<Location>>,
    area_size: Option<f64>,
) -> Result<LocationFn, String> {
    if let Some(locations) = locations {
        Ok(Box::new(move |rnd| get_random_item(locations.as_slice(), rnd).cloned().expect("cannot get any location")))
    } else {
        let bounding_box = if let Some(area_size) = area_size {
            if area_size > 0. {
                get_bounding_box_from_size(&problem_proto.plan, area_size)
            } else {
                return Err("area size must be positive".to_string());
            }
        } else {
            get_bounding_box_from_plan(&problem_proto.plan)
        };
        Ok(Box::new(move |rnd| {
            // TODO allow to configure distribution
            let lat = rnd.uniform_real((bounding_box.0).0, (bounding_box.1).0);
            let lng = rnd.uniform_real((bounding_box.0).1, (bounding_box.1).1);

            Location::Coordinate { lat, lng }
        }))
    }
}

fn get_bounding_box_from_plan(plan: &Plan) -> ((f64, f64), (f64, f64)) {
    let mut lat_min = f64::MAX;
    let mut lat_max = f64::MIN;
    let mut lng_min = f64::MAX;
    let mut lng_max = f64::MIN;

    get_plan_places(plan).map(|job_place| job_place.location.to_lat_lng()).for_each(|(lat, lng)| {
        lat_min = lat_min.min(lat);
        lat_max = lat_max.max(lat);

        lng_min = lng_min.min(lng);
        lng_max = lng_max.max(lng);
    });

    ((lat_min, lng_min), (lat_max, lng_max))
}

fn get_bounding_box_from_size(plan: &Plan, area_size: f64) -> ((f64, f64), (f64, f64)) {
    const WGS84_A: f64 = 6_378_137.0;
    const WGS84_B: f64 = 6_356_752.3;
    let deg_to_rad = |deg| std::f64::consts::PI * deg / 180.;
    let rad_to_deg = |rad| 180. * rad / std::f64::consts::PI;

    let ((min_lat, min_lng), (max_lat, max_lng)) = get_bounding_box_from_plan(plan);
    let center_lat = min_lat + (max_lat - min_lat) / 2.;
    let center_lng = min_lng + (max_lng - min_lng) / 2.;

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

    ((lat_min, lon_min), (lat_max, lon_max))
}

fn get_plan_time_windows(plan: &Plan) -> Vec<Vec<Vec<String>>> {
    get_plan_places(plan).flat_map(|job_place| job_place.times.iter()).cloned().collect()
}

fn get_plan_demands(plan: &Plan) -> Vec<Vec<i32>> {
    plan.jobs.iter().flat_map(get_job_tasks).filter_map(|job_task| job_task.demand.as_ref()).cloned().collect()
}

fn get_plan_durations(plan: &Plan) -> Vec<f64> {
    get_plan_places(plan).map(|job_place| job_place.duration).collect()
}

fn get_plan_places(plan: &Plan) -> impl Iterator<Item = &JobPlace> {
    plan.jobs.iter().flat_map(get_job_tasks).flat_map(|job_task| job_task.places.iter())
}

fn get_job_tasks(job: &Job) -> impl Iterator<Item = &JobTask> {
    job.pickups
        .iter()
        .flat_map(|tasks| tasks.iter())
        .chain(job.deliveries.iter().flat_map(|tasks| tasks.iter()))
        .chain(job.replacements.iter().flat_map(|tasks| tasks.iter()))
        .chain(job.services.iter().flat_map(|tasks| tasks.iter()))
}
