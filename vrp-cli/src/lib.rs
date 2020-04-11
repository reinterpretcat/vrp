//! A VRP library public API.

pub mod import;
pub mod solve;

extern crate clap;
use crate::import::import_problem;
use clap::{App, Arg, ArgMatches, Values};
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter, Write};
use std::process;
use std::sync::Arc;
use vrp_core::models::Problem as CoreProblem;
use vrp_pragmatic::get_unique_locations;
use vrp_pragmatic::json::problem::{deserialize_problem, serialize_problem, FormatError, PragmaticProblem, Problem};
use vrp_pragmatic::json::solution::PragmaticSolution;
use vrp_solver::SolverBuilder;

#[cfg(not(target_arch = "wasm32"))]
mod interop {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::slice;

    type Callback = extern "C" fn(*const c_char);

    fn to_string(pointer: *const c_char) -> String {
        let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
        std::str::from_utf8(slice).unwrap().to_string()
    }

    /// Returns a list of unique locations to request a routing matrix.
    /// Problem should be passed in `pragmatic` format.
    #[no_mangle]
    extern "C" fn get_routing_locations(problem: *const c_char, success: Callback, failure: Callback) {
        let problem = to_string(problem);
        let problem = BufReader::new(problem.as_bytes());
        let result = deserialize_problem(problem)
            .map_err(|errors| get_errors_serialized(&errors))
            .and_then(|problem| get_locations_serialized(&problem));

        match result {
            Ok(locations) => {
                let locations = CString::new(locations.as_bytes()).unwrap();
                success(locations.as_ptr());
            }
            Err(err) => {
                let error = CString::new(format!("Cannot get locations: '{}'", err).as_bytes()).unwrap();
                failure(error.as_ptr());
            }
        };
    }

    /// Converts problem from format specified by `format` to `pragmatic` format.
    #[no_mangle]
    extern "C" fn convert_to_pragmatic(
        format: *const c_char,
        inputs: *const *const c_char,
        input_len: *const i32,
        success: Callback,
        failure: Callback,
    ) {
        let format = to_string(format);
        let inputs = unsafe { slice::from_raw_parts(inputs, input_len as usize).to_vec() };
        let inputs = inputs.iter().map(|p| to_string(*p)).collect::<Vec<_>>();
        let readers = inputs.iter().map(|p| BufReader::new(p.as_bytes())).collect::<Vec<_>>();

        match import_problem(format.as_str(), Some(readers)) {
            Ok(problem) => {
                let mut buffer = String::new();
                let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
                serialize_problem(writer, &problem).unwrap();
                let problem = CString::new(buffer.as_bytes()).unwrap();

                success(problem.as_ptr());
            }
            Err(err) => {
                let error = CString::new(format!("Cannot convert to pragmatic: '{}'", err).as_bytes()).unwrap();
                failure(error.as_ptr());
            }
        }
    }

    /// Solves Vehicle Routing Problem passed in `pragmatic` format.
    #[no_mangle]
    extern "C" fn solve_pragmatic(
        problem: *const c_char,
        matrices: *const *const c_char,
        matrices_len: *const i32,
        success: Callback,
        failure: Callback,
    ) {
        let problem = to_string(problem);
        let matrices = unsafe { slice::from_raw_parts(matrices, matrices_len as usize).to_vec() };
        let matrices = matrices.iter().map(|m| to_string(*m)).collect::<Vec<_>>();

        let result = if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }
            .map_err(|errors| get_errors_serialized(&errors))
            .and_then(|problem| get_solution_serialized(&Arc::new(problem)));

        match result {
            Ok(solution) => {
                let solution = CString::new(solution.as_bytes()).unwrap();
                success(solution.as_ptr());
            }
            Err(err) => {
                let error = CString::new(format!("Cannot solve: '{}'", err).as_bytes()).unwrap();
                failure(error.as_ptr());
            }
        };
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    extern crate serde_json;
    extern crate wasm_bindgen;

    use wasm_bindgen::prelude::*;

    use super::*;
    use vrp_pragmatic::json::problem::Matrix;

    /// Returns a list of unique locations to request a routing matrix.
    /// Problem should be passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn get_routing_locations(problem: &JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem = problem
            .into_serde()
            .map_err(|err| JsValue::from_str(format!("Cannot read problem: '{}'", err).as_str()))?;

        get_locations_serialized(&problem)
            .map(|locations| JsValue::from_str(locations.as_str()))
            .map_err(|err| JsValue::from_str(format!("Cannot get locations: '{}'", err).as_str()))
    }

    /// Converts problem from format specified by `format` to `pragmatic` format.
    #[wasm_bindgen]
    pub fn convert_to_pragmatic(format: &str, inputs: &JsValue) -> Result<JsValue, JsValue> {
        let inputs: Vec<String> =
            inputs.into_serde().map_err(|err| JsValue::from_str(format!("Cannot read inputs: '{}'", err).as_str()))?;

        let readers = inputs.iter().map(|input| BufReader::new(input.as_bytes())).collect();

        match import_problem(format, Some(readers)) {
            Ok(problem) => {
                let mut buffer = String::new();
                let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
                serialize_problem(writer, &problem).unwrap();

                Ok(JsValue::from_str(buffer.as_str()))
            }
            Err(err) => Err(JsValue::from_str(format!("Cannot convert to pragmatic: '{}'", err).as_str())),
        }
    }

    /// Solves Vehicle Routing Problem passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn solve_pragmatic(problem: &JsValue, matrices: &JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem = problem
            .into_serde()
            .map_err(|err| JsValue::from_str(format!("Cannot read problem: '{}'", err).as_str()))?;

        let matrices: Vec<Matrix> = matrices
            .into_serde()
            .map_err(|err| JsValue::from_str(format!("Cannot read matrix array: '{}'", err).as_str()))?;

        let problem = Arc::new(
            if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }.map_err(
                |errors| {
                    JsValue::from_str(
                        errors.iter().map(|err| format!("{}", err)).collect::<Vec<_>>().join("\n").as_str(),
                    )
                },
            )?,
        );

        get_solution_serialized(&problem)
            .map(|problem| JsValue::from_str(problem.as_str()))
            .map_err(|err| JsValue::from_str(format!("Cannot solve problem: '{}'", err).as_str()))
    }
}

fn open_file(path: &str, description: &str) -> File {
    File::open(path).unwrap_or_else(|err| {
        eprintln!("Cannot open {} file '{}': '{}'", description, path, err.to_string());
        process::exit(1);
    })
}

fn create_file(path: &str, description: &str) -> File {
    File::create(path).unwrap_or_else(|err| {
        eprintln!("Cannot create {} file '{}': '{}'", description, path, err.to_string());
        process::exit(1);
    })
}

fn create_write_buffer(out_file: Option<File>) -> BufWriter<Box<dyn Write>> {
    if let Some(out_file) = out_file {
        BufWriter::new(Box::new(out_file))
    } else {
        BufWriter::new(Box::new(stdout()))
    }
}

fn get_locations_serialized(problem: &Problem) -> Result<String, String> {
    // TODO validate the problem?

    let locations = get_unique_locations(&problem);
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    serde_json::to_writer_pretty(writer, &locations).map_err(|err| err.to_string())?;

    Ok(buffer)
}

fn get_solution_serialized(problem: &Arc<CoreProblem>) -> Result<String, String> {
    let (solution, _, _) =
        SolverBuilder::default().build().solve(problem.clone()).ok_or_else(|| "Cannot solve problem".to_string())?;

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    solution.write_pragmatic_json(&problem, writer)?;

    Ok(buffer)
}

pub fn get_errors_serialized(errors: &Vec<FormatError>) -> String {
    errors.iter().map(|err| format!("{}", err)).collect::<Vec<_>>().join("\n")
}
