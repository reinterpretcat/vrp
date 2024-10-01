//! This module provides functionality which clusters jobs using DBSCAN algorithm.

#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/dbscan_test.rs"]
mod dbscan_test;

use crate::algorithms::clustering::dbscan::create_clusters;
use crate::algorithms::geometry::Point;
use crate::models::common::Timestamp;
use crate::models::problem::{Job, Single};
use crate::models::Problem;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

/// Creates clusters of jobs using DBSCAN algorithm.
pub fn create_job_clusters(
    problem: &Problem,
    min_points: Option<usize>,
    epsilon: Option<Float>,
) -> GenericResult<Vec<HashSet<Job>>> {
    let min_points = min_points.unwrap_or(3).max(2);
    let epsilon = epsilon.unwrap_or_else(|| estimate_epsilon(problem, min_points));

    // get main parameters with some randomization
    let profile = problem.fleet.profiles.first().ok_or_else(|| GenericError::from("cannot find any profile"))?;
    // exclude jobs without locations from clustering
    let jobs = problem.jobs.all().iter().filter(|j| job_has_locations(j)).cloned().collect::<Vec<_>>();

    let neighbor_fn = move |job| {
        problem
            .jobs
            .neighbors(profile, job, 0.)
            .filter(move |(job, _)| job_has_locations(job))
            .take_while(move |(_, cost)| *cost < epsilon)
            .map(|(job, _)| job)
    };

    Ok(create_clusters(jobs.as_slice(), min_points, neighbor_fn)
        .into_iter()
        .map(|cluster| cluster.into_iter().cloned().collect::<HashSet<_>>())
        .collect())
}

/// Estimates DBSCAN epsilon parameter.
fn estimate_epsilon(problem: &Problem, min_points: usize) -> Float {
    let costs = get_average_costs(problem, min_points);
    let curve = costs.into_iter().enumerate().map(|(idx, cost)| Point::new(idx as Float, cost)).collect::<Vec<_>>();

    // get max curvature approximation and return it as a guess for optimal epsilon value
    get_max_curvature(curve.as_slice())
}

/// Gets average costs across all profiles.
fn get_average_costs(problem: &Problem, min_points: usize) -> Vec<Float> {
    let jobs = problem.jobs.as_ref();
    let mut costs = problem.fleet.profiles.iter().fold(vec![0.; jobs.size()], |mut acc, profile| {
        jobs.all().iter().enumerate().for_each(|(idx, job)| {
            let (sum, count) = jobs
                .neighbors(profile, job, Timestamp::default())
                .filter(|(j, _)| job_has_locations(j))
                .take(min_points)
                .map(|(_, cost)| cost)
                .fold((0., 1), |(sum, idx), cost| (sum + cost, idx + 1));

            acc[idx] += sum / count as Float;
        });
        acc
    });

    costs.iter_mut().for_each(|cost| *cost /= problem.fleet.profiles.len() as Float);

    // sort all distances in ascending order
    costs.sort_unstable_by(compare_floats_refs);
    costs.dedup_by(|a, b| compare_floats(*a, *b) == Ordering::Equal);

    costs
}

/// Gets max curvature approximation: for each point p on the curve, find the one with the maximum
/// distance d to a line drawn from the first to the last point of the curves.
fn get_max_curvature(values: &[Point]) -> Float {
    if values.is_empty() {
        return 0.;
    }

    let first = values.first().unwrap();
    let last = values.last().unwrap();

    values
        .iter()
        .fold((0., Float::MIN), |acc, p| {
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
