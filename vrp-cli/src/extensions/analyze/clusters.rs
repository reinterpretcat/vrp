#[cfg(test)]
#[path = "../../../tests/unit/extensions/analyze/clusters_test.rs"]
mod clusters_test;

use std::io::{BufReader, BufWriter, Read};
use std::sync::Arc;
use vrp_core::construction::clustering::dbscan::create_job_clusters;
use vrp_core::models::common::Timestamp;
use vrp_core::models::problem::{get_job_locations, JobIdDimension};
use vrp_core::models::Problem;
use vrp_core::prelude::{Float, GenericResult};
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::solution::serialize_named_locations_as_geojson;
use vrp_pragmatic::format::{CoordIndexExtraProperty, MultiFormatError};

/// Gets job clusters.
pub fn get_clusters<F: Read>(
    problem_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
    min_points: Option<usize>,
    epsilon: Option<Float>,
) -> GenericResult<String> {
    let problem = Arc::new(get_core_problem(problem_reader, matrices_readers).map_err(|errs| errs.to_string())?);

    let coord_index = problem.extras.get_coord_index().expect("cannot find coord index");
    let coord_index = coord_index.as_ref();

    let clusters = create_job_clusters(problem.jobs.all(), &problem.fleet, min_points, epsilon, |profile, job| {
        problem.jobs.neighbors(profile, job, Timestamp::default())
    })?;

    let locations = clusters
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
        .collect::<Vec<_>>();

    let mut writer = BufWriter::new(Vec::new());

    serialize_named_locations_as_geojson(locations.as_slice(), &mut writer)
        .map_err(|err| format!("cannot write named locations as geojson: '{err}'"))?;

    let bytes = writer.into_inner().map_err(|err| format!("{err}"))?;
    let result = String::from_utf8(bytes).map_err(|err| format!("{err}"))?;

    Ok(result)
}

fn get_core_problem<F: Read>(
    problem_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
) -> Result<Problem, MultiFormatError> {
    let problem = deserialize_problem(problem_reader)?;

    let matrices = matrices_readers.map(|matrices| {
        matrices.into_iter().map(|file| deserialize_matrix(BufReader::new(file))).collect::<Result<Vec<_>, _>>()
    });

    let matrices = if let Some(matrices) = matrices { Some(matrices?) } else { None };

    (problem, matrices).read_pragmatic()
}
