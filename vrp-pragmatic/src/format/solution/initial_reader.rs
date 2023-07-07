#[cfg(test)]
#[path = "../../../tests/unit/format/solution/initial_reader_test.rs"]
mod initial_reader_test;

use crate::construction::enablers::VehicleTie;
use crate::format::solution::activity_matcher::{try_match_point_job, JobInfo};
use crate::format::solution::Activity as FormatActivity;
use crate::format::solution::Stop as FormatStop;
use crate::format::solution::Tour as FormatTour;
use crate::format::solution::{deserialize_solution, map_reason_code};
use crate::format::{get_coord_index, get_job_index, CoordIndex, JobIndex};
use crate::parse_time;
use hashbrown::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::heuristics::UnassignmentInfo;
use vrp_core::models::common::*;
use vrp_core::models::problem::{Actor, Job, Single};
use vrp_core::models::solution::Tour as CoreTour;
use vrp_core::models::solution::{Activity, Place, Registry, Route};
use vrp_core::prelude::*;

type ActorKey = (String, String, usize);

/// Reads initial solution from buffer.
/// NOTE: Solution feasibility is not checked.
pub fn read_init_solution<R: Read>(
    solution: BufReader<R>,
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
) -> Result<Solution, String> {
    let solution = deserialize_solution(solution).map_err(|err| format!("cannot deserialize solution: {err}"))?;

    let mut registry = Registry::new(&problem.fleet, random);
    let mut added_jobs = HashSet::default();

    let actor_index = registry.all().map(|actor| (get_actor_key(actor.as_ref()), actor)).collect::<HashMap<_, _>>();
    let coord_index = get_coord_index(problem.as_ref());
    let job_index = get_job_index(problem.as_ref());

    let routes =
        solution.tours.iter().try_fold::<_, _, Result<_, String>>(Vec::<_>::default(), |mut routes, tour| {
            let actor_key = (tour.vehicle_id.clone(), tour.type_id.clone(), tour.shift_index);
            let actor =
                actor_index.get(&actor_key).ok_or_else(|| format!("cannot find vehicle for {actor_key:?}"))?.clone();
            registry.use_actor(&actor);

            let mut core_route = create_core_route(actor, tour)?;

            tour.stops.iter().try_for_each(|stop| {
                stop.activities().iter().try_for_each::<_, Result<_, String>>(|activity| {
                    try_insert_activity(&mut core_route, tour, stop, activity, job_index, coord_index, &mut added_jobs)
                })
            })?;

            routes.push(core_route);

            Ok(routes)
        })?;

    let mut unassigned = solution.unassigned.unwrap_or_default().iter().try_fold::<Vec<_>, _, Result<_, String>>(
        Default::default(),
        |mut acc, unassigned_job| {
            let job = job_index
                .get(&unassigned_job.job_id)
                .cloned()
                .ok_or_else(|| format!("cannot get job id for: {unassigned_job:?}"))?;
            // NOTE we take the first reason only and map it to simple variant
            let code = unassigned_job
                .reasons
                .first()
                .map(|reason| UnassignmentInfo::Simple(map_reason_code(&reason.code)))
                .ok_or_else(|| format!("cannot get reason for: {unassigned_job:?}"))?;

            added_jobs.insert(job.clone());
            acc.push((job, code));

            Ok(acc)
        },
    )?;

    unassigned.extend(
        problem.jobs.all().filter(|job| added_jobs.get(job).is_none()).map(|job| (job, UnassignmentInfo::Unknown)),
    );

    Ok(Solution { registry, routes, unassigned })
}

fn try_insert_activity(
    route: &mut Route,
    tour: &FormatTour,
    stop: &FormatStop,
    activity: &FormatActivity,
    job_index: &JobIndex,
    coord_index: &CoordIndex,
    added_jobs: &mut HashSet<Job>,
) -> Result<(), String> {
    if activity.commute.is_some() {
        return Err("commute property in initial solution is not supported".to_string());
    }

    let stop = match stop {
        FormatStop::Transit(_) => return Err("transit property in initial solution is not yet supported".to_string()),
        FormatStop::Point(stop) => stop,
    };

    if let Some(JobInfo(job, single, place, time)) = try_match_point_job(tour, stop, activity, job_index, coord_index)?
    {
        added_jobs.insert(job);
        insert_new_activity(route, single, place, time);
    } else if activity.activity_type != "departure" && activity.activity_type != "arrival" {
        return Err(format!("cannot match activity with job id '{}' in tour: '{}'", activity.job_id, tour.vehicle_id));
    }

    Ok(())
}

fn get_actor_key(actor: &Actor) -> ActorKey {
    let dimens = &actor.vehicle.dimens;

    let vehicle_id = dimens.get_vehicle_id().cloned().expect("cannot get vehicle id!");
    let type_id = dimens.get_vehicle_type().cloned().expect("cannot get type id!");
    let shift_index = dimens.get_shift_index().expect("cannot get shift index!");

    (vehicle_id, type_id, shift_index)
}

fn create_core_route(actor: Arc<Actor>, format_tour: &FormatTour) -> Result<Route, String> {
    let mut core_tour = CoreTour::new(&actor);

    // NOTE this is necessary to keep departure time optimization
    let set_activity_time = |format_stop: &FormatStop,
                             format_activity: &FormatActivity,
                             core_activity: &mut Activity|
     -> Result<(), String> {
        let time = &format_stop.schedule();
        let (arrival, departure) = format_activity
            .time
            .as_ref()
            .map_or((&time.arrival, &time.departure), |interval| (&interval.start, &interval.end));

        core_activity.schedule.arrival = parse_time(arrival);
        core_activity.schedule.departure = parse_time(departure);

        Ok(())
    };

    let start_stop = format_tour.stops.first().ok_or_else(|| "empty tour in init solution".to_string())?;
    let start_activity = start_stop.activities().first().ok_or_else(|| "start stop has no activities".to_string())?;
    let core_start = core_tour.all_activities_mut().next().expect("cannot get start activity from core tour");

    set_activity_time(start_stop, start_activity, core_start)?;

    if core_tour.end().is_some() {
        let end_stop = format_tour.stops.last().unwrap();
        let end_activity = end_stop.activities().first().ok_or_else(|| "end stop has no activities".to_string())?;
        let core_end = core_tour.all_activities_mut().last().unwrap();

        set_activity_time(end_stop, end_activity, core_end)?;
    }

    Ok(Route { actor, tour: core_tour })
}

fn insert_new_activity(route: &mut Route, single: Arc<Single>, place: Place, time: TimeWindow) {
    let activity = Activity {
        place,
        schedule: Schedule { arrival: time.start, departure: time.end },
        job: Some(single),
        commute: None,
    };
    route.tour.insert_last(activity);
}
