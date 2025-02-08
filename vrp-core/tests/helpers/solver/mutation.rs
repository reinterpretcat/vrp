use super::*;
use crate::construction::heuristics::*;
use crate::models::common::Schedule;
use crate::models::solution::{Activity, Place};
use std::collections::HashMap;

/// Promotes given job ids to locked in given context.
pub fn promote_to_locked(mut insertion_ctx: InsertionContext, job_ids: &[&str]) -> InsertionContext {
    let ids = insertion_ctx
        .problem
        .jobs
        .all()
        .iter()
        .filter(|job| job_ids.contains(&job.dimens().get_job_id().unwrap().as_str()))
        .cloned();
    insertion_ctx.solution.locked.extend(ids);

    insertion_ctx
}

/// Compares given strings as ids using ignore id.
pub fn compare_with_ignore(left: &[Vec<String>], right: &[Vec<&str>], ignore: &str) {
    if left.len() != right.len() {
        assert_eq!(left, right);
    }

    left.iter().zip(right.iter()).for_each(|(a_vec, b_vec)| {
        if a_vec.len() != b_vec.len() {
            assert_eq!(left, right);
        }

        a_vec.iter().zip(b_vec.iter()).for_each(|(a_value, b_value)| {
            if a_value != ignore && *b_value != ignore && a_value != b_value {
                assert_eq!(left, right);
            }
        });
    })
}

/// Get jobs from insertion context with given ids in the given order.
pub fn get_jobs_by_ids(insertion_ctx: &InsertionContext, job_ids: &[&str]) -> Vec<Job> {
    let mut ids = insertion_ctx
        .problem
        .jobs
        .all()
        .iter()
        .filter_map(|job| {
            let job_id = job.dimens().get_job_id().unwrap().clone();
            if job_ids.contains(&job_id.as_str()) {
                Some((job_id, job.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let map = job_ids.iter().enumerate().map(|(idx, job)| (*job, idx)).collect::<HashMap<_, _>>();
    ids.sort_by(|(a, _), (b, _)| map.get(a.as_str()).unwrap().cmp(map.get(b.as_str()).unwrap()));

    let (_, jobs): (Vec<_>, Vec<_>) = ids.into_iter().unzip();

    jobs
}

/// Gets all jobs with their id in a map.
pub fn get_jobs_map_by_ids(insertion_ctx: &InsertionContext) -> HashMap<String, Job> {
    insertion_ctx
        .problem
        .jobs
        .all()
        .iter()
        .map(|job| {
            let job_id = job.dimens().get_job_id().unwrap().clone();
            (job_id, job.clone())
        })
        .collect()
}

/// Rearranges jobs in routes to match specified job order.
pub fn rearrange_jobs_in_routes(insertion_ctx: &mut InsertionContext, job_order: &[Vec<&str>]) {
    assert_eq!(insertion_ctx.solution.routes.len(), job_order.len());
    let jobs_map = get_jobs_map_by_ids(insertion_ctx);

    insertion_ctx.solution.routes.iter_mut().zip(job_order.iter()).for_each(|(route_ctx, order)| {
        let jobs = route_ctx.route().tour.jobs().cloned().collect::<Vec<_>>();
        jobs.iter().for_each(|job| {
            route_ctx.route_mut().tour.remove(job);
        });
        assert_eq!(route_ctx.route().tour.job_count(), 0);

        order.iter().for_each(|job_id| {
            let job = jobs_map.get(*job_id).unwrap().to_single().clone();
            let place_idx = 0;
            let place = &job.places[place_idx];
            route_ctx.route_mut().tour.insert_last(Activity {
                place: Place {
                    idx: place_idx,
                    location: place.location.unwrap(),
                    duration: place.duration,
                    time: place.times.first().unwrap().to_time_window(0.),
                },
                schedule: Schedule::new(0., 0.),
                job: Some(job),
                commute: None,
            });
        });

        insertion_ctx.problem.goal.accept_route_state(route_ctx);
    });

    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
}
