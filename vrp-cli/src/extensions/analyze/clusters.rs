#[cfg(test)]
#[path = "../../../tests/unit/extensions/analyze/clusters_test.rs"]
mod clusters_test;

use vrp_core::algorithms::clustering::kmedoids::create_kmedoids;
use vrp_core::construction::clustering::dbscan::create_job_clusters;
use vrp_core::models::common::Timestamp;
use vrp_core::models::problem::{JobIdDimension, get_job_locations};
use vrp_core::prelude::*;
use vrp_pragmatic::format::{CoordIndexExtraProperty, Location as ApiLocation};

/// Gets job clusters with DBSCAN algorithm.
pub fn get_dbscan_clusters(
    problem: &Problem,
    min_points: Option<usize>,
    epsilon: Option<Float>,
) -> GenericResult<Vec<(String, ApiLocation, usize)>> {
    let coord_index = problem.extras.get_coord_index().expect("cannot find coord index");
    let coord_index = coord_index.as_ref();

    let clusters = create_job_clusters(problem.jobs.all(), &problem.fleet, min_points, epsilon, |profile, job| {
        problem.jobs.neighbors(profile, job, Timestamp::default())
    })?;

    Ok(clusters
        .iter()
        .enumerate()
        .flat_map(|(cluster_idx, jobs)| {
            jobs.iter()
                .filter_map(move |job| {
                    job.dimens().get_job_id().cloned().map(|job_id| {
                        get_job_locations(job)
                            .flatten()
                            .filter_map(move |l_idx| coord_index.get_by_idx(l_idx))
                            .map(move |location| (job_id.clone(), location, cluster_idx))
                    })
                })
                .flatten()
        })
        .collect())
}

/// Gets k-medoids clusters for all locations in the given problem.
pub fn get_k_medoids_clusters(problem: &Problem, k: usize) -> GenericResult<Vec<(String, ApiLocation, usize)>> {
    let points = (0..problem.transport.size()).collect::<Vec<_>>();
    let profile = problem.fleet.profiles.first().ok_or_else(|| GenericError::from("cannot find any profile"))?;
    let coord_index = problem.extras.get_coord_index().ok_or_else(|| GenericError::from("cannot find coord index"))?;
    let coord_index = coord_index.as_ref();

    let clusters = create_kmedoids(&points, k, |from, to| problem.transport.distance_approx(profile, *from, *to));

    clusters
        .into_iter()
        .flat_map(move |(medoid, location_ids)| {
            location_ids.into_iter().map(move |id| {
                coord_index
                    .get_by_idx(id)
                    .ok_or_else(|| GenericError::from(format!("cannot find location {id}")))
                    .map(|location| (format!("m={medoid}"), location, medoid))
            })
        })
        .collect::<Result<Vec<_>, _>>()
}
