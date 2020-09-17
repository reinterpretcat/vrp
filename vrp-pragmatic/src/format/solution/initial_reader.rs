#[cfg(test)]
#[path = "../../../tests/unit/format/solution/initial_reader_test.rs"]
mod initial_reader_test;

use crate::format::solution::activity_matcher::try_match_job;
use crate::format::solution::deserialize_solution;
use crate::format::{get_coord_index, get_job_index, CoordIndex, JobIndex};
use crate::parse_time;
use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::common::*;
use vrp_core::models::problem::{Actor, Job, Single};
use vrp_core::models::solution::{Activity, Place, Registry, Route};
use vrp_core::models::{Problem, Solution};

use crate::format::solution::Activity as FormatActivity;
use crate::format::solution::Stop as FormatStop;
use crate::format::solution::Tour as FormatTour;
use vrp_core::models::solution::Tour as CoreTour;

type ActorKey = (String, String, usize);

/// Reads initial solution from buffer.
/// NOTE: Solution feasibility is not checked.
pub fn read_init_solution<R: Read>(solution: BufReader<R>, problem: Arc<Problem>) -> Result<Solution, String> {
    let solution = deserialize_solution(solution).map_err(|err| format!("cannot deserialize solution: {}", err))?;

    let mut registry = Registry::new(&problem.fleet);
    let mut added_jobs = HashSet::default();

    let actor_index = registry.all().map(|actor| (get_actor_key(actor.as_ref()), actor)).collect::<HashMap<_, _>>();
    let coord_index = get_coord_index(problem.as_ref());
    let job_index = get_job_index(problem.as_ref());

    let routes =
        solution.tours.iter().try_fold::<_, _, Result<_, String>>(Vec::<_>::default(), |mut routes, tour| {
            let actor_key = (tour.vehicle_id.clone(), tour.type_id.clone(), tour.shift_index);
            let actor =
                actor_index.get(&actor_key).ok_or_else(|| format!("cannot find vehicle for {:?}", actor_key))?.clone();
            registry.use_actor(&actor);

            let mut core_route = create_core_route(actor, tour)?;

            tour.stops.iter().try_for_each(|stop| {
                stop.activities.iter().try_for_each::<_, Result<_, String>>(|activity| {
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
                .ok_or_else(|| format!("cannot get job id for: {:?}", unassigned_job))?;
            let code = unassigned_job
                .reasons
                .first()
                .map(|reason| reason.code)
                .ok_or_else(|| format!("cannot get reason for: {:?}", unassigned_job))?;

            added_jobs.insert(job.clone());
            acc.push((job, code));

            Ok(acc)
        },
    )?;

    unassigned.extend(problem.jobs.all().filter(|job| added_jobs.get(job).is_none()).map(|job| (job, 0)));

    Ok(Solution { registry, routes, unassigned, extras: problem.extras.clone() })
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
    if let Some((job, single, place, time)) = try_match_job(tour, stop, activity, job_index, coord_index)? {
        added_jobs.insert(job.clone());
        try_insert_new_activity(route, single, place, time)?;
    }

    Ok(())
}

fn get_actor_key(actor: &Actor) -> ActorKey {
    let dimens = &actor.vehicle.dimens;

    let vehicle_id = dimens.get_id().cloned().expect("cannot get vehicle id!");
    let type_id = dimens.get_value::<String>("type_id").cloned().expect("cannot get type id!");
    let shift_index = dimens.get_value::<usize>("shift_index").cloned().expect("cannot get shift index!");

    (vehicle_id, type_id, shift_index)
}

fn create_core_route(actor: Arc<Actor>, format_tour: &FormatTour) -> Result<Route, String> {
    let mut core_tour = CoreTour::new(&actor);

    // NOTE this is necessary to keep departure time optimization
    let departure_time =
        &format_tour.stops.first().as_ref().ok_or_else(|| "empty tour in init solution".to_string())?.time.departure;
    core_tour.all_activities_mut().next().expect("cannot get start activity from core tour").schedule.departure =
        parse_time(departure_time);

    Ok(Route { actor, tour: core_tour })
}

fn try_insert_new_activity(
    route: &mut Route,
    single: Arc<Single>,
    place: Place,
    time: TimeWindow,
) -> Result<(), String> {
    let activity =
        Activity { place, schedule: Schedule { arrival: time.start, departure: time.end }, job: Some(single) };
    route.tour.insert_last(activity);

    Ok(())
}
