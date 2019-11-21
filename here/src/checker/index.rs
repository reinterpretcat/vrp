use crate::checker::models::*;
use crate::json::problem::*;
use crate::json::solution::*;
use chrono::DateTime;
use std::collections::HashMap;
use std::sync::Arc;

pub fn create_solution_info(problem: &Problem, solution: &Solution) -> Result<SolutionInfo, String> {
    let job_map: HashMap<String, Arc<JobVariant>> =
        problem.plan.jobs.iter().fold(HashMap::default(), |mut acc, job| {
            let id = match &job {
                JobVariant::Single(job) => &job.id,
                JobVariant::Multi(job) => &job.id,
            }
            .clone();
            acc.insert(id, Arc::new(job.clone()));
            acc
        });

    let vehicle_map: HashMap<String, Arc<VehicleType>> =
        problem.fleet.types.iter().fold(HashMap::default(), |mut acc, vehicle_type| {
            acc.insert(vehicle_type.id.clone(), Arc::new(vehicle_type.clone()));
            acc
        });

    let tour_infos = create_tour_infos(&job_map, &vehicle_map, solution)?;
    let relation_infos = create_relation_infos(&job_map, &vehicle_map, problem)?;
    let unassigned_infos = create_unassigned_infos(&job_map, solution)?;

    Ok(SolutionInfo { jobs: job_map, tours: tour_infos, relations: relation_infos, unassigned: unassigned_infos })
}

fn create_tour_infos(
    job_map: &HashMap<String, Arc<JobVariant>>,
    vehicle_map: &HashMap<String, Arc<VehicleType>>,
    solution: &Solution,
) -> Result<Vec<TourInfo>, String> {
    solution.tours.iter().try_fold::<Vec<_>, _, Result<_, String>>(Default::default(), |mut acc, tour: &Tour| {
        let mut stops: Vec<StopInfo> = Default::default();

        let vehicle_type = vehicle_map.get(&tour.type_id).ok_or("".to_string())?;
        let vehicle_meta = VehicleMeta { vehicle_id: tour.vehicle_id.clone(), vehicle_type: vehicle_type.clone() };

        tour.stops.iter().for_each(|stop| {
            let schedule = parse_interval(&stop.time.arrival, &stop.time.departure);
            let mut activities: Vec<ActivityInfo> = Default::default();
            stop.activities.iter().for_each(|activity| {
                let job = job_map.get(&activity.job_id).cloned();
                activities.push(ActivityInfo {
                    activity: activity.clone(),
                    job_id: job.as_ref().map(|_| activity.job_id.clone()),
                    job,
                    vehicle_meta: vehicle_meta.clone(),
                    schedule: activity
                        .time
                        .as_ref()
                        .map(|t| parse_interval(&t.start, &t.end))
                        .unwrap_or(schedule.clone()),
                });
            });
            stops.push(StopInfo { stop: stop.clone(), activities, schedule })
        });

        let start = stops
            .iter()
            .flat_map(|s| s.activities.iter())
            .next()
            .map(|a| a.schedule.1)
            .ok_or(format!("No activities in the tour: {}", tour.vehicle_id))?;
        let end = stops.iter().flat_map(|s| s.activities.iter()).last().map(|a| a.schedule.0).unwrap();

        acc.push(TourInfo { vehicle_meta, stops, schedule: (start, end) });

        Ok(acc)
    })
}

fn create_relation_infos(
    job_map: &HashMap<String, Arc<JobVariant>>,
    vehicle_map: &HashMap<String, Arc<VehicleType>>,
    problem: &Problem,
) -> Result<Vec<RelationInfo>, String> {
    problem
        .plan
        .relations
        .as_ref()
        .map(|r| {
            r.iter().try_fold(Vec::<_>::default(), |mut acc, relation| {
                let vehicle_type = vehicle_map
                    .get(&relation.vehicle_id)
                    .ok_or(format!(
                        "Cannot find vehicle with id '{}' in one of problem's relations",
                        relation.vehicle_id
                    ))?
                    .clone();
                let jobs = relation.jobs.iter().map(|job_id| job_map.get(job_id).cloned()).collect();

                acc.push(RelationInfo { relation: relation.clone(), vehicle_type, jobs });
                Ok(acc)
            })
        })
        .unwrap_or_else(|| Ok(Vec::default()))
}

fn create_unassigned_infos(
    job_map: &HashMap<String, Arc<JobVariant>>,
    solution: &Solution,
) -> Result<Vec<UnassignedInfo>, String> {
    solution.unassigned.iter().try_fold::<Vec<_>, _, Result<_, String>>(Default::default(), |mut acc, unassigned| {
        let job = job_map
            .get(&unassigned.job_id)
            .ok_or(format!("Unknown job id in unassigned: '{}'", unassigned.job_id))?
            .clone();
        acc.push(UnassignedInfo { unassigned: unassigned.clone(), job });

        Ok(acc)
    })
}

fn parse_interval(start: &String, end: &String) -> (f64, f64) {
    (parse_time(start), parse_time(end))
}

fn parse_time(time: &String) -> f64 {
    DateTime::parse_from_rfc3339(time).unwrap().timestamp() as f64
}
