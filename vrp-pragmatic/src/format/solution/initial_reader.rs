#[cfg(test)]
#[path = "../../../tests/unit/format/solution/initial_reader_test.rs"]
mod initial_reader_test;

use crate::format::problem::JobIndex;
use crate::format::solution::deserialize_solution;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::common::{IdDimension, ValueDimension};
use vrp_core::models::problem::Actor;
use vrp_core::models::solution::{Registry, Route};
use vrp_core::models::{Problem, Solution};

use crate::format::solution::Activity as FormatActivity;

use vrp_core::models::solution::Activity as CoreActivity;
use vrp_core::models::solution::Tour as CoreTour;

/// Reads initial solution from buffer.
/// NOTE: Solution feasibility is not checked.
pub fn read_init_solution<R: Read>(solution: BufReader<R>, problem: Arc<Problem>) -> Result<Solution, String> {
    let solution = deserialize_solution(solution).map_err(|err| format!("cannot deserialize solution: {}", err))?;

    let mut registry = Registry::new(&problem.fleet);
    let actor_index = registry.all().map(|actor| (get_actor_key(actor.as_ref()), actor)).collect::<HashMap<_, _>>();
    let job_index = get_job_index(problem.as_ref());

    let routes = solution.tours.iter().try_fold::<_, _, Result<_, String>>(Default::default(), |routes, tour| {
        let actor_key = (tour.vehicle_id.clone(), tour.type_id.clone(), tour.shift_index);
        let actor =
            actor_index.get(&actor_key).ok_or_else(|| format!("cannot find vehicle for {:?}", actor_key))?.clone();
        registry.use_actor(&actor);

        let mut core_route = create_core_route(actor);

        tour.stops.iter().try_for_each(|stop| {
            stop.activities.iter().try_for_each::<_, Result<_, String>>(|activity| {
                let core_activity = create_core_activity(activity)?;
                core_route.tour.insert_last(core_activity);
                Ok(())
            })
        })?;

        Ok(routes)
    })?;

    let unassigned = solution.unassigned.unwrap_or_default().iter().try_fold::<Vec<_>, _, Result<_, String>>(
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

            acc.push((job, code));

            Ok(acc)
        },
    )?;

    Ok(Solution { registry, routes, unassigned, extras: problem.extras.clone() })
}

fn get_job_index(problem: &Problem) -> &JobIndex {
    problem
        .extras
        .get("job_index")
        .and_then(|s| s.downcast_ref::<JobIndex>())
        .unwrap_or_else(|| panic!("cannot get job index!"))
}

fn get_actor_key(actor: &Actor) -> (String, String, usize) {
    let dimens = &actor.vehicle.dimens;

    let vehicle_id = dimens.get_id().cloned().expect("cannot get vehicle id!");
    let type_id = dimens.get_value::<String>("type_id").cloned().expect("cannot get type id!");
    let shift_index = dimens.get_value::<usize>("shift_index").cloned().expect("cannot get shift index!");

    (vehicle_id, type_id, shift_index)
}

fn create_core_route(actor: Arc<Actor>) -> Route {
    let tour = CoreTour::new(&actor);
    Route { actor, tour }
}

fn create_core_activity(_activity: &FormatActivity) -> Result<CoreActivity, String> {
    unimplemented!()
}
