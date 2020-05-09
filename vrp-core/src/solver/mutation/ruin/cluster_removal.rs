#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use crate::algorithms::geometry::Point;
use crate::construction::heuristics::InsertionContext;
use crate::models::Problem;
use crate::solver::mutation::Ruin;
use crate::solver::RefinementContext;
use crate::utils::compare_floats;
use std::ops::Range;
use std::sync::Arc;

/// A ruin strategy which removes job clusters using DBSCAN algorithm.
pub struct ClusterRemoval {
    /// Stores possible pairs of `min_point` and `epsilon` parameter values.
    params: Vec<(usize, f64)>,
    /// Threshold to control how many jobs can be removed.
    threshold: f64,
}

impl ClusterRemoval {
    pub fn new(problem: Arc<Problem>, cluster_size: Range<usize>, threshold: f64) -> Self {
        // TODO test on problem with zero or one job.

        let min = cluster_size.start.max(3);
        let max = cluster_size.end.min(problem.jobs.size()).max(min + 1);

        let params = (min..max).map(|min_pts| (min_pts, estimate_epsilon(&problem, min_pts - 1))).collect::<Vec<_>>();

        Self { params, threshold }
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &mut RefinementContext, _: InsertionContext) -> InsertionContext {
        unimplemented!()
    }
}

/// Estimates DBSCAN epsilon parameter.
fn estimate_epsilon(problem: &Problem, nth_neighbor: usize) -> f64 {
    // for each job get distance to its nth neighbor
    let mut costs = get_average_costs(problem, nth_neighbor);

    // sort all distances in ascending order and form the curve
    costs.sort_by(|&a, &b| compare_floats(a, b));
    let curve = costs.into_iter().enumerate().map(|(idx, cost)| Point::new(idx as f64, cost)).collect::<Vec<_>>();

    // get max curvature approximation
    let curvature = get_max_curvature(curve.as_slice());

    // use max curvature as a guess for optimal epsilon value
    curvature
}

/// Gets average costs across all profiles.
fn get_average_costs(problem: &Problem, nth_neighbor: usize) -> Vec<f64> {
    let mut costs = problem.fleet.profiles.iter().fold(vec![0.; problem.jobs.size()], |mut acc, &profile| {
        problem.jobs.all().enumerate().for_each(|(idx, job)| {
            acc[idx] += problem
                .jobs
                .neighbors(profile, &job, 0.)
                .skip(nth_neighbor - 1)
                .next()
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
