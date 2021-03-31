use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::models::Problem;
use crate::utils::Random;
use hashbrown::HashMap;
use std::iter::{empty, once};
use std::sync::Arc;

pub(crate) fn get_route_jobs(solution: &SolutionContext) -> HashMap<Job, RouteContext> {
    solution
        .routes
        .iter()
        .flat_map(|rc| rc.route.tour.jobs().collect::<Vec<_>>().into_iter().map(move |job| (job, rc.clone())))
        .collect()
}

/// Returns randomly selected job within all its neighbours.
pub(crate) fn select_seed_jobs<'a>(
    problem: &'a Problem,
    routes: &[RouteContext],
    random: &Arc<dyn Random + Send + Sync>,
) -> Box<dyn Iterator<Item = Job> + 'a> {
    let seed = select_seed_job(routes, random);

    if let Some((route_index, job)) = seed {
        return Box::new(
            once(job.clone()).chain(
                problem
                    .jobs
                    .neighbors(&routes.get(route_index).unwrap().route.actor.vehicle.profile, &job, Default::default())
                    .map(|(job, _)| job)
                    .cloned(),
            ),
        );
    }

    Box::new(empty())
}

/// Selects seed job from existing solution
pub(crate) fn select_seed_job(
    routes: &'_ [RouteContext],
    random: &Arc<dyn Random + Send + Sync>,
) -> Option<(usize, Job)> {
    if routes.is_empty() {
        return None;
    }

    let initial_route_index = random.uniform_int(0, (routes.len() - 1) as i32) as usize;
    let mut route_index = initial_route_index;

    loop {
        let rc = routes.get(route_index).unwrap();

        if rc.route.tour.has_jobs() {
            let job = select_random_job(rc, random);
            if let Some(job) = job {
                return Some((route_index, job));
            }
        }

        route_index = (route_index + 1) % routes.len();
        if route_index == initial_route_index {
            break;
        }
    }

    None
}

pub(crate) fn select_random_job(rc: &RouteContext, random: &Arc<dyn Random + Send + Sync>) -> Option<Job> {
    let size = rc.route.tour.activity_count();
    if size == 0 {
        return None;
    }

    let activity_index = random.uniform_int(1, size as i32) as usize;
    let mut ai = activity_index;

    loop {
        let job = rc.route.tour.get(ai).and_then(|a| a.retrieve_job());

        if job.is_some() {
            return job;
        }

        ai = (ai + 1) % (size + 1);
        if ai == activity_index {
            break;
        }
    }

    None
}
