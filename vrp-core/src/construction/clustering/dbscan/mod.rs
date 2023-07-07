//! This module provides functionality which clusters jobs using DBSCAN algorithm.

#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/dbscan_test.rs"]
mod dbscan_test;

use crate::algorithms::clustering::dbscan::{create_clusters, NeighborhoodFn};
use crate::algorithms::geometry::Point;
use crate::models::common::Timestamp;
use crate::models::problem::{Job, Single};
use crate::models::Problem;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::sync::Arc;

/// Creates clusters of jobs using DBSCAN algorithm.
pub fn create_job_clusters(
    problem: &Problem,
    random: &(dyn Random + Send + Sync),
    min_points: Option<usize>,
    epsilon: Option<f64>,
) -> Vec<Vec<Job>> {
    let min_points = min_points.unwrap_or(3).max(2);
    let epsilon = epsilon.unwrap_or_else(|| estimate_epsilon(problem, min_points));

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
                .map(|(_, cost)| cost)
                .fold((0., 1), |(sum, idx), cost| (sum + cost, idx + 1));

            acc[idx] += sum / count as f64;
        });
        acc
    });

    costs.iter_mut().for_each(|cost| *cost /= problem.fleet.profiles.len() as f64);

    // sort all distances in ascending order
    costs.sort_by(compare_floats_refs);
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
