#[cfg(test)]
#[path = "../../../tests/unit/extensions/analyze/clusters_test.rs"]
mod clusters_test;

use vrp_core::construction::clustering::dbscan::create_job_clusters;
use vrp_core::models::common::Timestamp;
use vrp_core::models::problem::{get_job_locations, JobIdDimension};
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
