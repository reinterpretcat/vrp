#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use super::*;
use crate::algorithms::dbscan::{create_clusters, NeighborhoodFn};
use crate::algorithms::geometry::Point;
use crate::construction::heuristics::InsertionContext;
use crate::models::common::Timestamp;
use crate::models::problem::{Job, Single};
use crate::models::Problem;
use crate::solver::mutation::get_route_jobs;
use crate::solver::RefinementContext;
use crate::utils::{compare_floats, Environment, Random};
use rand::prelude::*;
use std::cmp::Ordering;
use std::sync::Arc;

/// A ruin strategy which removes job clusters using [`DBSCAN`] algorithm.
///
/// [`DBSCAN`]: ../../algorithms/dbscan/index.html
pub struct ClusterRemoval {
    clusters: Vec<Vec<Job>>,
    limits: RuinLimits,
}

impl ClusterRemoval {
    /// Creates a new instance of `ClusterRemoval`.
    pub fn new(problem: Arc<Problem>, environment: Arc<Environment>, min_items: usize, limits: RuinLimits) -> Self {
        let mut clusters = Self::create_clusters(problem, environment.clone(), Some(min_items), None);

        clusters.shuffle(&mut environment.random.get_rng());

        Self { clusters, limits }
    }

    /// Creates a new instance of `ClusterRemoval` with default parameters.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        Self::new(problem, environment, 3, RuinLimits::default())
    }

    /// Creates clusters using DBSCAN algorithm.
    pub fn create_clusters(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
        min_points: Option<usize>,
        epsilon: Option<f64>,
    ) -> Vec<Vec<Job>> {
        let min_points = min_points.unwrap_or(3).max(3);
        let epsilon = epsilon.unwrap_or_else(|| estimate_epsilon(&problem, min_points));

        create_job_clusters(&problem, environment.random.as_ref(), min_points, epsilon)
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let locked = insertion_ctx.solution.locked.clone();

        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let max_removed_activities = self.limits.get_chunk_size(&insertion_ctx);
        let tracker = self.limits.get_tracker();

        let mut indices = (0..self.clusters.len()).into_iter().collect::<Vec<usize>>();
        indices.shuffle(&mut insertion_ctx.environment.random.get_rng());

        indices.into_iter().take_while(|_| tracker.is_not_limit(max_removed_activities)).for_each(|idx| {
            let cluster = self.clusters.get(idx).unwrap();
            let mut indices = (0..cluster.len()).into_iter().collect::<Vec<usize>>();
            indices.shuffle(&mut insertion_ctx.environment.random.get_rng());

            let left = max_removed_activities - tracker.get_removed_activities();

            indices
                .iter()
                .map(|idx| cluster.get(*idx).expect("invalid cluster index"))
                .filter(|job| !locked.contains(job))
                .take_while(|_| tracker.is_not_limit(max_removed_activities))
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

        tracker.iterate_removed_jobs(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx
    }
}

fn create_job_clusters(
    problem: &Problem,
    random: &(dyn Random + Send + Sync),
    min_points: usize,
    epsilon: f64,
) -> Vec<Vec<Job>> {
    // get main parameters with some randomization
    let profile = &problem.fleet.profiles[random.uniform_int(0, problem.fleet.profiles.len() as i32 - 1) as usize];
    // exclude jobs without locations from clustering
    let jobs = problem.jobs.all().filter(job_has_locations).collect::<Vec<_>>();

    let neighbor_fn: NeighborhoodFn<Job> = Box::new(move |job, eps| {
        Box::new(
            problem
                .jobs
                .neighbors(profile, job, 0.)
                .filter(move |(job, _)| job_has_locations(job))
                .take_while(move |(_, cost)| *cost < eps)
                .map(|(job, _)| job),
        )
    });

    create_clusters(jobs.as_slice(), epsilon, min_points, &neighbor_fn)
        .into_iter()
        .map(|cluster| cluster.into_iter().cloned().collect::<Vec<_>>())
        .collect::<Vec<_>>()
}

/// Estimates DBSCAN epsilon parameter.
fn estimate_epsilon(problem: &Problem, min_points: usize) -> f64 {
    let costs = get_average_costs(problem, min_points);
    let curve = costs.into_iter().enumerate().map(|(idx, cost)| Point::new(idx as f64, cost)).collect::<Vec<_>>();

    // get max curvature approximation and return it as a guess for optimal epsilon value
    get_max_curvature(curve.as_slice())
}

/// Gets average costs across all profiles.
fn get_average_costs(problem: &Problem, min_points: usize) -> Vec<f64> {
    let jobs = problem.jobs.as_ref();
    let mut costs = problem.fleet.profiles.iter().fold(vec![0.; jobs.size()], |mut acc, profile| {
        jobs.all().enumerate().for_each(|(idx, job)| {
            let (sum, count) = jobs
                .neighbors(profile, &job, Timestamp::default())
                .filter(|(j, _)| job_has_locations(j))
                .take(min_points)
                .map(|(_, cost)| *cost)
                .fold((0., 1), |(sum, idx), cost| (sum + cost, idx + 1));

            acc[idx] += sum / count as f64;
        });
        acc
    });

    costs.iter_mut().for_each(|cost| *cost /= problem.fleet.profiles.len() as f64);

    // sort all distances in ascending order
    costs.sort_by(|&a, &b| compare_floats(a, b));
    costs.dedup_by(|a, b| compare_floats(*a, *b) == Ordering::Equal);

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
            let distance = p.distance_to_line(first, last);

            if distance > acc.1 {
                (p.y, distance)
            } else {
                acc
            }
        })
        .0
}

fn job_has_locations(job: &Job) -> bool {
    let has_location = |single: &Arc<Single>| single.places.iter().any(|place| place.location.is_some());

    match &job {
        Job::Single(single) => has_location(single),
        Job::Multi(multi) => multi.jobs.iter().any(has_location),
    }
}
