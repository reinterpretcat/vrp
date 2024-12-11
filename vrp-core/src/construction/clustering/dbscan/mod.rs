//! This module provides functionality which clusters jobs using DBSCAN algorithm.

mod neighbour_clusters;
pub use self::neighbour_clusters::create_job_clusters;

use crate::algorithms::clustering::dbscan::create_clusters;
use crate::algorithms::geometry::Point;
use crate::models::common::Profile;
use crate::models::problem::{Job, Single};
use crate::prelude::{Cost, Fleet};
use rosomaxa::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

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
