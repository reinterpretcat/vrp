//! Defines a json based contract which is shared by all language bindings.
//!
//! Every binding in [`crate::interop`] is a thin adapter over the functions defined here, which keeps
//! them in sync by construction. Errors are always [`MultiFormatError`], so a binding can pass on a
//! machine readable representation via [`MultiFormatError::to_json`] rather than a prose message.

#[cfg(test)]
#[path = "../../tests/unit/interop/json_test.rs"]
mod json_test;

use crate::extensions::import::import_problem;
use crate::extensions::solve::config::{Config, create_builder_from_config, read_config};
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_core::models::Problem as CoreProblem;
use vrp_core::prelude::{GenericError, Solver};
use vrp_pragmatic::format::problem::{
    Matrix, PragmaticProblem, Problem, deserialize_matrix, deserialize_problem, serialize_problem,
};
use vrp_pragmatic::format::solution::{PragmaticOutputType, write_pragmatic};
use vrp_pragmatic::format::{CoordIndex, FormatError, MultiFormatError};
use vrp_pragmatic::get_unique_locations;
use vrp_pragmatic::validation::ValidationContext;

/// A result type returned by all interop operations.
pub type InteropResult<T> = Result<T, MultiFormatError>;

/// Returns unique locations of the given problem as json, in the order the solver expects routing
/// data in, so that a matrix can be requested from an external routing service.
pub fn routing_locations(problem: &str) -> InteropResult<String> {
    locations_json(&deserialize_problem(BufReader::new(problem.as_bytes()))?)
}

/// Validates the given problem definition, returning `Ok(())` when it is valid.
pub fn validate(problem: &str, matrices: &[String]) -> InteropResult<()> {
    let (problem, matrices) = parse_problem(problem, matrices)?;

    validate_parsed(&problem, &matrices)
}

/// Converts a problem from the format specified by `format` into pragmatic json.
pub fn convert(format: &str, inputs: &[String]) -> InteropResult<String> {
    let readers = inputs.iter().map(|input| BufReader::new(input.as_bytes())).collect();

    let problem = import_problem(format, Some(readers)).map_err(|err| {
        error("E0000", "cannot deserialize problem", format!("check '{format}' input. Error: '{err}'"))
    })?;

    let mut writer = BufWriter::new(Vec::new());
    serialize_problem(&problem, &mut writer).map_err(|err| serialization_error("problem", &err))?;

    into_string("problem", writer)
}

/// Validates and solves the given problem, returning its solution as json.
///
/// Pass an empty slice of matrices to use the built-in distance approximation instead of real
/// routing data.
pub fn solve(problem: &str, matrices: &[String], config: &str) -> InteropResult<String> {
    let (problem, matrices) = parse_problem(problem, matrices)?;
    validate_parsed(&problem, &matrices)?;

    // NOTE: reuse the already parsed values instead of deserializing the raw input a second time
    let core_problem =
        if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }?;

    let config = read_config(BufReader::new(config.as_bytes()))
        .map_err(|err| error("E0004", "cannot read config", format!("check config definition. Error: '{err}'")))?;

    solution_json(Arc::new(core_problem), config)
}

/// Gets locations serialized in json.
pub fn get_locations_serialized(problem: &Problem) -> Result<String, GenericError> {
    locations_json(problem).map_err(as_generic)
}

/// Gets solution serialized in json.
pub fn get_solution_serialized(problem: Arc<CoreProblem>, config: Config) -> Result<String, GenericError> {
    solution_json(problem, config).map_err(as_generic)
}

fn locations_json(problem: &Problem) -> InteropResult<String> {
    serde_json::to_string_pretty(&get_unique_locations(problem)).map_err(|err| serialization_error("locations", &err))
}

fn solution_json(problem: Arc<CoreProblem>, config: Config) -> InteropResult<String> {
    let solution = create_builder_from_config(problem.clone(), Default::default(), &config)
        .and_then(|builder| builder.build())
        .map(|config| Solver::new(problem.clone(), config))
        .and_then(|solver| solver.solve())
        .map_err(|err| {
            error(
                "E0003",
                "cannot find any solution",
                format!("please submit a bug and share original problem and routing matrix. Error: '{err}'"),
            )
        })?;

    let output_type = if config.output.and_then(|output| output.include_geojson).unwrap_or(false) {
        PragmaticOutputType::Combined
    } else {
        Default::default()
    };

    let mut writer = BufWriter::new(Vec::new());
    write_pragmatic(problem.as_ref(), &solution, output_type, &mut writer)
        .map_err(|err| serialization_error("solution", &err))?;

    into_string("solution", writer)
}

/// Deserializes a problem with its routing matrices, reporting all errors found in one go.
fn parse_problem(problem: &str, matrices: &[String]) -> InteropResult<(Problem, Vec<Matrix>)> {
    let problem = deserialize_problem(BufReader::new(problem.as_bytes()));
    let matrices = matrices
        .iter()
        .map(|matrix| deserialize_matrix(BufReader::new(matrix.as_bytes())))
        .collect::<Result<Vec<_>, _>>();

    match (problem, matrices) {
        (Ok(problem), Ok(matrices)) => Ok((problem, matrices)),
        (Err(errors), Ok(_)) | (Ok(_), Err(errors)) => Err(errors),
        (Err(problem_errors), Err(matrix_errors)) => {
            Err(problem_errors.into_iter().chain(matrix_errors).collect::<Vec<_>>().into())
        }
    }
}

fn validate_parsed(problem: &Problem, matrices: &Vec<Matrix>) -> InteropResult<()> {
    let coord_index = CoordIndex::new(problem);
    let matrices = if matrices.is_empty() { None } else { Some(matrices) };

    ValidationContext::new(problem, matrices, &coord_index).validate()
}

fn into_string(subject: &str, writer: BufWriter<Vec<u8>>) -> InteropResult<String> {
    let bytes = writer.into_inner().map_err(|err| serialization_error(subject, &err))?;

    String::from_utf8(bytes).map_err(|err| serialization_error(subject, &err))
}

fn error(code: &str, cause: &str, action: String) -> MultiFormatError {
    vec![FormatError::new(code.to_string(), cause.to_string(), action)].into()
}

/// Creates a machine-readable error for failures in a language-binding adapter.
pub(crate) fn interop_error(cause: &str, action: String) -> MultiFormatError {
    error("E0006", cause, action)
}

fn serialization_error(subject: &str, err: &dyn std::fmt::Display) -> MultiFormatError {
    error(
        "E0005",
        &format!("cannot serialize {subject}"),
        format!("please submit a bug and share original problem and routing matrix. Error: '{err}'"),
    )
}

// keeps the json representation, so nothing is lost by widening to the generic error type
fn as_generic(err: MultiFormatError) -> GenericError {
    err.to_json().into()
}
