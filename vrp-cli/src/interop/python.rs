//! Exposes the [`crate::interop::json`] contract to python.
//!
//! Errors are raised as `OSError` carrying the json representation of the problems found.

use crate::interop::json;
use pyo3::exceptions::PyOSError;
use pyo3::prelude::*;
use vrp_pragmatic::format::MultiFormatError;

/// Returns a list of unique locations which can be used to request a routing matrix.
#[pyfunction]
fn get_routing_locations(problem: String) -> PyResult<String> {
    json::routing_locations(&problem).map_err(to_py_err)
}

/// Converts `problem` from format specified by `format` to `pragmatic` format.
#[pyfunction]
fn convert_to_pragmatic(format: &str, inputs: Vec<String>) -> PyResult<String> {
    json::convert(format, &inputs).map_err(to_py_err)
}

/// Validates Vehicle Routing Problem passed in `pragmatic` format.
#[pyfunction]
fn validate_pragmatic(problem: String, matrices: Vec<String>) -> PyResult<String> {
    json::validate(&problem, &matrices).map(|_| "[]".to_string()).map_err(to_py_err)
}

/// Validates and solves Vehicle Routing Problem passed in `pragmatic` format.
#[pyfunction]
fn solve_pragmatic(problem: String, matrices: Vec<String>, config: String) -> PyResult<String> {
    json::solve(&problem, &matrices, &config).map_err(to_py_err)
}

fn to_py_err(err: MultiFormatError) -> PyErr {
    PyOSError::new_err(err.to_json())
}

// the `vrp_cli` package re-exports these from its `__init__.py`, which is what lets the generated
// models ship alongside as plain python
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(convert_to_pragmatic, m)?)?;
    m.add_function(wrap_pyfunction!(get_routing_locations, m)?)?;
    m.add_function(wrap_pyfunction!(validate_pragmatic, m)?)?;
    m.add_function(wrap_pyfunction!(solve_pragmatic, m)?)?;
    Ok(())
}
