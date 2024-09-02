#[cfg(test)]
#[path = "../../tests/unit/common/init_solution_reader_test.rs"]
mod init_solution_reader_test;

use crate::common::read_line;
use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::heuristics::UnassignmentInfo;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::solution::{Activity, Registry, Route, Tour};
use vrp_core::prelude::*;

/// Reads initial solution from a buffer.
/// NOTE: Solution feasibility is not checked.
pub fn read_init_solution<R: Read>(
    mut reader: BufReader<R>,
    problem: Arc<Problem>,
    random: Arc<dyn Random>,
) -> Result<Solution, GenericError> {
    let mut buffer = String::new();

    let mut solution = Solution {
        cost: Cost::default(),
        registry: Registry::new(&problem.fleet, random),
        routes: vec![],
        unassigned: Default::default(),
        telemetry: None,
    };

    let mut not_used_jobs = problem.jobs.all().collect::<HashSet<_>>();

    loop {
        match read_line(&mut reader, &mut buffer) {
            Ok(read) if read > 0 => {
                let route: Vec<_> = buffer.split(':').collect();
                if route.len() != 2 {
                    continue;
                }

                let id_map = problem.jobs.all().fold(HashMap::<String, Arc<Single>>::new(), |mut acc, job| {
                    let single = job.to_single().clone();
                    acc.insert(single.dimens.get_job_id().unwrap().to_string(), single);
                    acc
                });

                let actor = solution.registry.next().next().unwrap();
                let mut tour = Tour::new(&actor);

                route.last().unwrap().split_whitespace().for_each(|id| {
                    let single = id_map.get(id).unwrap();
                    let place_idx = 0;
                    let place = &single.places[place_idx];
                    tour.insert_last(Activity {
                        place: vrp_core::models::solution::Place {
                            idx: place_idx,
                            location: place.location.unwrap(),
                            duration: place.duration,
                            time: place.times.first().and_then(|span| span.as_time_window()).unwrap(),
                        },
                        schedule: Schedule::new(0, 0),
                        job: Some(single.clone()),
                        commute: None,
                    });

                    not_used_jobs.remove(&Job::Single(single.clone()));
                });

                solution.registry.use_actor(&actor);
                solution.routes.push(Route { actor, tour });
            }
            Ok(_) => break,
            Err(error) => {
                if buffer.is_empty() {
                    break;
                } else {
                    return Err(error);
                }
            }
        }
    }

    solution.unassigned = not_used_jobs.into_iter().map(|job| (job, UnassignmentInfo::Unknown)).collect();

    Ok(solution)
}
