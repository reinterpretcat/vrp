use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::Problem;
use vrp_core::solver::mutation::ClusterRemoval;
use vrp_core::utils::Environment;
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::FormatError;

/// Gets job clusters.
pub fn get_clusters<F: Read>(
    problem_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
    min_points: Option<usize>,
) -> Result<String, Vec<FormatError>> {
    let problem = Arc::new(get_core_problem(problem_reader, matrices_readers)?);

    let environment = Arc::new(Environment::default());

    let _clusters = ClusterRemoval::create_clusters(problem, environment, min_points.unwrap_or(4));

    // TODO get geojson string with clusters

    unimplemented!()
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

    (problem.clone(), matrices.clone()).read_pragmatic()
}
