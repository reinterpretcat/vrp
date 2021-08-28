#[cfg(test)]
#[path = "../../../tests/unit/extensions/analyze/clusters_test.rs"]
mod clusters_test;

use std::io::{BufReader, BufWriter, Read};
use std::sync::Arc;
use vrp_core::models::common::IdDimension;
use vrp_core::models::problem::get_job_locations;
use vrp_core::models::Problem;
use vrp_core::solver::mutation::ClusterRemoval;
use vrp_core::utils::Environment;
use vrp_pragmatic::format::get_coord_index;
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::solution::serialize_named_locations_as_geojson;
use vrp_pragmatic::format::FormatError;

/// Gets job clusters.
pub fn get_clusters<F: Read>(
    problem_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
    min_points: Option<usize>,
    epsilon: Option<f64>,
) -> Result<String, String> {
    let problem = Arc::new(
        get_core_problem(problem_reader, matrices_readers).map_err(|errs| FormatError::format_many(&errs, ","))?,
    );

    let coord_index = get_coord_index(&problem);
    let environment = Arc::new(Environment::default());

    let clusters = ClusterRemoval::create_clusters(problem.clone(), environment, min_points, epsilon);

    let locations = clusters
        .iter()
        .enumerate()
        .flat_map(|(cluster_idx, jobs)| {
            jobs.iter()
                .filter_map(move |job| {
                    job.dimens().get_id().cloned().map(|job_id| {
                        get_job_locations(job)
                            .flatten()
                            .filter_map(move |l_idx| coord_index.get_by_idx(l_idx))
                            .map(move |location| (job_id.clone(), location, cluster_idx))
                    })
                })
                .flatten()
        })
        .collect::<Vec<_>>();

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };

    serialize_named_locations_as_geojson(writer, locations.as_slice())
        .map_err(|err| format!("cannot write named locations as geojson: '{}'", err))?;

    Ok(buffer)
}

fn get_core_problem<F: Read>(
    problem_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
) -> Result<Problem, Vec<FormatError>> {
    let problem = deserialize_problem(problem_reader)?;

    let matrices = matrices_readers.map(|matrices| {
        matrices.into_iter().map(|file| deserialize_matrix(BufReader::new(file))).collect::<Result<Vec<_>, _>>()
    });

    let matrices = if let Some(matrices) = matrices { Some(matrices?) } else { None };

    (problem, matrices).read_pragmatic()
}
