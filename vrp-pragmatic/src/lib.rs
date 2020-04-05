//! Pragmatic crates aims to solve real world VRP variations allowing users to specify their problems
//! via simple **pragmatic** json format.
//!

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

#[cfg(test)]
#[path = "../tests/checker/mod.rs"]
pub mod checker;

#[cfg(test)]
#[path = "../tests/generator/mod.rs"]
pub mod generator;

#[cfg(test)]
#[path = "../tests/features/mod.rs"]
pub mod features;

#[cfg(test)]
#[path = "../tests/slow/mod.rs"]
pub mod slow;

mod constraints;
mod extensions;
mod utils;
mod validation;

pub mod json;

use crate::json::coord_index::CoordIndex;
use crate::json::problem::{deserialize_problem, PragmaticProblem, Problem};
use crate::json::solution::PragmaticSolution;
use chrono::{DateTime, ParseError, SecondsFormat, TimeZone, Utc};
use std::ffi::{CStr, CString};
use std::io::{BufReader, BufWriter};
use std::os::raw::c_char;
use std::panic::catch_unwind;
use std::slice;
use std::sync::Arc;
use vrp_core::models::Problem as CoreProblem;
use vrp_core::models::Solution as CoreSolution;
use vrp_solver::SolverBuilder;

use crate::json::Location;
use std::io::Read;

/// Get lists of problem.
pub fn get_locations(problem: &Problem) -> Vec<Location> {
    CoordIndex::new(&problem).unique()
}

/// Returns serialized into json list of unique locations from serialized `problem` in order used
/// by routing matrix.
pub fn get_locations_serialized<R: Read>(problem: BufReader<R>) -> Result<String, String> {
    let problem = deserialize_problem(problem).map_err(|errors| {
        format!(
            "Problem has the following errors:\n{}",
            errors.iter().map(|err| err.to_string()).collect::<Vec<_>>().join("\t\n")
        )
    })?;

    let locations = get_locations(&problem);
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    serde_json::to_writer_pretty(writer, &locations).map_err(|err| err.to_string())?;

    Ok(buffer)
}

fn format_time(time: f64) -> String {
    Utc.timestamp(time as i64, 0).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_time(time: &String) -> f64 {
    parse_time_safe(time).unwrap()
}

fn parse_time_safe(time: &String) -> Result<f64, ParseError> {
    DateTime::parse_from_rfc3339(time).map(|time| time.timestamp() as f64)
}

fn solution_to_string(problem: &CoreProblem, solution: &CoreSolution) -> String {
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    solution.write_pragmatic_json(&problem, writer).ok();

    buffer
}

// TODO improve error propagation

type Callback = extern "C" fn(*const c_char);

fn to_string(pointer: *const c_char) -> String {
    let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
    std::str::from_utf8(slice).unwrap().to_string()
}

#[no_mangle]
extern "C" fn locations(problem: *const c_char, success: Callback, failure: Callback) {
    let result = catch_unwind(|| get_locations_serialized(BufReader::new(to_string(problem).as_bytes())).ok().unwrap());

    match result {
        Ok(locations) => {
            let locations = CString::new(locations.as_bytes()).unwrap();
            success(locations.as_ptr());
        }
        Err(_) => {
            let error = CString::new("Cannot get locations".as_bytes()).unwrap();
            failure(error.as_ptr());
        }
    };
}

#[no_mangle]
extern "C" fn solve(
    problem: *const c_char,
    matrices: *const *const c_char,
    matrices_len: *const i32,
    success: Callback,
    failure: Callback,
) {
    let result = catch_unwind(|| {
        let problem = to_string(problem);
        let matrices = unsafe { slice::from_raw_parts(matrices, matrices_len as usize).to_vec() };
        let matrices = matrices.iter().map(|m| to_string(*m)).collect::<Vec<_>>();

        let problem = Arc::new((problem, matrices).read_pragmatic().ok().unwrap());

        let (solution, _, _) = SolverBuilder::default().build().solve(problem.clone()).unwrap();

        solution_to_string(problem.as_ref(), &solution)
    });

    match result {
        Ok(solution) => {
            let solution = CString::new(solution.as_bytes()).unwrap();
            success(solution.as_ptr());
        }
        Err(_) => {
            let error = CString::new("Cannot solve".as_bytes()).unwrap();
            failure(error.as_ptr());
        }
    };
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    extern crate serde_json;
    extern crate wasm_bindgen;
    use wasm_bindgen::prelude::*;

    use super::*;
    use crate::json::problem::Matrix;

    #[wasm_bindgen]
    pub fn web_solve(problem: &JsValue, matrices: &JsValue) -> Result<JsValue, JsValue> {
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

        let (solution, _, _) = SolverBuilder::default()
            .build()
            .solve(problem.clone())
            .ok_or_else(|| JsValue::from_str("Cannot solve problem"))?;

        Ok(JsValue::from_str(solution_to_string(problem.as_ref(), &solution).as_str()))
    }
}
