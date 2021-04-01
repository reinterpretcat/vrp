#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use super::*;
use crate::algorithms::dbscan::{create_clusters, Cluster, NeighborhoodFn};
use crate::algorithms::geometry::Point;
use crate::construction::heuristics::InsertionContext;
use crate::models::common::Timestamp;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::mutation::get_route_jobs;
use crate::solver::RefinementContext;
use crate::utils::{compare_floats, Environment, Random};
use rand::prelude::*;
use std::sync::Arc;

/// A ruin strategy which removes job clusters using [`DBSCAN`] algorithm.
///
/// [`DBSCAN`]: ../../algorithms/dbscan/index.html
///
pub struct ClusterRemoval {
    clusters: Vec<Vec<Job>>,
    limits: RuinLimits,
}

impl ClusterRemoval {
    /// Creates a new instance of `ClusterRemoval`.
    pub fn new(problem: Arc<Problem>, environment: Arc<Environment>, min_items: usize, limits: RuinLimits) -> Self {
        let min_items = min_items.max(3);
        let epsilon = estimate_epsilon(&problem, min_items);

        let mut clusters = create_job_clusters(&problem, environment.random.as_ref(), min_items, epsilon)
            .into_iter()
            .map(|cluster| cluster.into_iter().cloned().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        clusters.shuffle(&mut environment.random.get_rng());

        Self { clusters, limits }
    }

    /// Creates a new instance of `ClusterRemoval` with default parameters.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        Self::new(problem, environment, 4, RuinLimits::default())
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let locked = insertion_ctx.solution.locked.clone();

        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let max_affected = self.limits.get_chunk_size(&insertion_ctx);
        let tracker = self.limits.get_tracker();

        let mut indices = (0..self.clusters.len()).into_iter().collect::<Vec<usize>>();
        indices.shuffle(&mut insertion_ctx.environment.random.get_rng());

        indices.into_iter().take_while(|_| tracker.is_not_limit(max_affected)).for_each(|idx| {
            let cluster = self.clusters.get(idx).unwrap();
            let left = max_affected - tracker.removed_jobs.read().unwrap().len();

            cluster
                .iter()
                .filter(|job| !locked.contains(job))
                .take_while(|_| tracker.is_not_limit(max_affected))
                .take(left)
                .for_each(|job| {
                    if let Some(rc) = route_jobs.get_mut(job) {
                        // NOTE actual insertion context modification via route mut
                        if rc.route.tour.contains(job) {
                            rc.route_mut().tour.remove(job);

                            tracker.add_actor(rc.route.actor.clone());
                            tracker.add_job((*job).clone());
                        }
                    }
                });
        });

        tracker.removed_jobs.write().unwrap().iter().for_each(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx
    }
}

fn create_job_clusters<'a>(
    problem: &'a Problem,
    random: &(dyn Random + Send + Sync),
    min_items: usize,
    epsilon: f64,
) -> Vec<Cluster<'a, Job>> {
    // get main parameters with some randomization
    let profile = &problem.fleet.profiles[random.uniform_int(0, problem.fleet.profiles.len() as i32 - 1) as usize];

    let neighbor_fn: NeighborhoodFn<'a, Job> = Box::new(move |job, eps| {
        Box::new(once(job).chain(
            problem.jobs.neighbors(profile, job, 0.).take_while(move |(_, cost)| *cost < eps).map(|(job, _)| job),
        ))
    });

    create_clusters(problem.jobs.all_as_slice(), epsilon, min_items, &neighbor_fn)
}

/// Estimates DBSCAN epsilon parameter.
fn estimate_epsilon(problem: &Problem, min_points: usize) -> f64 {
    // for each job get distance to its nth neighbor
    let mut costs = get_average_costs(problem, min_points);

    // sort all distances in ascending order and form the curve
    costs.sort_by(|&a, &b| compare_floats(a, b));
    let curve = costs.into_iter().enumerate().map(|(idx, cost)| Point::new(idx as f64, cost)).collect::<Vec<_>>();

    // get max curvature approximation and return it as a guess for optimal epsilon value
    get_max_curvature(curve.as_slice())
}

/// Gets average costs across all profiles.
fn get_average_costs(problem: &Problem, min_points: usize) -> Vec<f64> {
    let mut costs = problem.fleet.profiles.iter().fold(vec![0.; problem.jobs.size()], |mut acc, profile| {
        problem.jobs.all().enumerate().for_each(|(idx, job)| {
            acc[idx] += problem
                .jobs
                .neighbors(profile, &job, Timestamp::default())
                .filter(|(_, cost)| *cost > 0.)
                .nth(min_points - 1)
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
        .fold((0., f64::MIN), |acc, p| {
            let distance = p.distance_to_line(&first, &last);

            if distance > acc.1 {
                (p.y, distance)
            } else {
                acc
            }
        })
        .0
}
