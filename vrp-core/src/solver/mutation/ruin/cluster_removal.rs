#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

extern crate rand;
use super::*;
use crate::algorithms::dbscan::{create_clusters, Cluster, NeighborhoodFn};
use crate::algorithms::geometry::Point;
use crate::construction::heuristics::InsertionContext;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::RefinementContext;
use crate::utils::{compare_floats, Random};
use hashbrown::HashSet;
use rand::prelude::*;
use std::ops::Range;
use std::sync::{Arc, RwLock};

/// A ruin strategy which removes job clusters using [`DBSCAN`] algorithm.
///
/// [`DBSCAN`]: ../../algorithms/dbscan/index.html
///
pub struct ClusterRemoval {
    /// Stores possible pairs of `min_point` and `epsilon` parameter values.
    params: Vec<(usize, f64)>,
    /// Specifies limitation for job removal.
    limit: JobRemovalLimit,
}

impl ClusterRemoval {
    /// Creates a new instance of `ClusterRemoval`.
    pub fn new(problem: Arc<Problem>, cluster_size: Range<usize>, limit: JobRemovalLimit) -> Self {
        let min = cluster_size.start.max(3);
        let max = cluster_size.end.min(problem.jobs.size()).max(min + 1);

        let params = (min..max).map(|min_pts| (min_pts, estimate_epsilon(&problem, min_pts))).collect::<Vec<_>>();

        Self { params, limit }
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &mut RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.random.clone();

        let mut clusters = create_job_clusters(&problem, &random, self.params.as_slice());
        clusters.shuffle(&mut rand::thread_rng());

        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let removed_jobs: RwLock<HashSet<Job>> = RwLock::new(HashSet::default());
        let locked = insertion_ctx.solution.locked.clone();
        let affected = get_removal_chunk_size(&insertion_ctx, &self.limit);

        clusters.iter_mut().take_while(|_| removed_jobs.read().unwrap().len() < affected).for_each(|cluster| {
            let left = affected - removed_jobs.read().unwrap().len();
            if cluster.len() > left {
                cluster.shuffle(&mut rand::thread_rng());
            }

            cluster.iter().filter(|job| !locked.contains(job)).take(left).for_each(|job| {
                if let Some(rc) = route_jobs.get_mut(job) {
                    // NOTE actual insertion context modification via route mut
                    if rc.route_mut().tour.remove(&job) {
                        removed_jobs.write().unwrap().insert((*job).clone());
                    }
                }
            });
        });

        removed_jobs.write().unwrap().iter().for_each(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx
    }
}

fn create_job_clusters<'a>(
    problem: &'a Problem,
    random: &Arc<dyn Random + Send + Sync>,
    params: &[(usize, f64)],
) -> Vec<Cluster<'a, Job>> {
    // get main parameters with some randomization
    let profile = problem.fleet.profiles[random.uniform_int(0, problem.fleet.profiles.len() as i32 - 1) as usize];
    let &(min_items, eps) = params.get(random.uniform_int(0, params.len() as i32 - 1) as usize).unwrap();
    let eps = random.uniform_real(eps * 0.9, eps * 1.1);

    let neighbor_fn: NeighborhoodFn<'a, Job> = Box::new(move |job, eps| {
        Box::new(once(job).chain(
            problem.jobs.neighbors(profile, job, 0.).take_while(move |(_, cost)| *cost < eps).map(|(job, _)| job),
        ))
    });

    create_clusters(problem.jobs.all_as_slice(), eps, min_items, &neighbor_fn)
}

/// Estimates DBSCAN epsilon parameter.
fn estimate_epsilon(problem: &Problem, min_points: usize) -> f64 {
    // for each job get distance to its nth neighbor
    let mut costs = get_average_costs(problem, min_points);

    // sort all distances in ascending order and form the curve
    costs.sort_by(|&a, &b| compare_floats(a, b));
    let curve = costs.into_iter().enumerate().map(|(idx, cost)| Point::new(idx as f64, cost)).collect::<Vec<_>>();

    // get max curvature approximation
    let curvature = get_max_curvature(curve.as_slice());

    // use max curvature as a guess for optimal epsilon value
    curvature
}

/// Gets average costs across all profiles.
fn get_average_costs(problem: &Problem, min_points: usize) -> Vec<f64> {
    let mut costs = problem.fleet.profiles.iter().fold(vec![0.; problem.jobs.size()], |mut acc, &profile| {
        problem.jobs.all().enumerate().for_each(|(idx, job)| {
            acc[idx] += problem
                .jobs
                .neighbors(profile, &job, 0.)
                .skip(min_points - 1)
                .next()
                // TODO consider time window difference as extra cost?
                .map(|(_, cost)| *cost)
                .unwrap_or(0.);
        });
        acc
    });

    costs.iter_mut().for_each(|cost| *cost /= problem.fleet.profiles.len() as f64);

    costs
}

/// Gets max curvature approximation: for each point p on the curve, find the one with the maximum
/// distance d to a line drawn from the first to the last point of the curves.
fn get_max_curvature(values: &[Point]) -> f64 {
    if values.is_empty() {
        return 0.;
    }

    let first = values.first().unwrap();
    let last = values.last().unwrap();

    values
        .iter()
        .fold((0., std::f64::MIN), |acc, p| {
            let distance = p.distance_to_line(&first, &last);

            if distance > acc.1 {
                (p.y, distance)
            } else {
                acc
            }
        })
        .0
}
