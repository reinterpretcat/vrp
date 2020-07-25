//! A crate for solving Vehicle Routing Problem using default metaheuristic.
//!
//!
//! This crate provides ready-to-use functionality to solve rich ***Vehicle Routing Problem***.
//!
//! For more details check the following resources:
//!
//! - [`user guide`](https://reinterpretcat.github.io/vrp) describes how to use cli
//!   application built from this crate
//! - `vrp-core` crate implements default metaheuristic

#![warn(missing_docs)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
mod helpers;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
#[path = "../tests/features/mod.rs"]
mod features;

pub use vrp_core as core;
pub use vrp_pragmatic as pragmatic;
pub use vrp_scientific as scientific;

pub mod extensions;

use crate::extensions::import::import_problem;
use crate::extensions::solve::config::{create_builder_from_config, Config};
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_core::models::Problem as CoreProblem;
use vrp_pragmatic::format::problem::{serialize_problem, PragmaticProblem, Problem};
use vrp_pragmatic::format::solution::PragmaticSolution;
use vrp_pragmatic::format::FormatError;
use vrp_pragmatic::get_unique_locations;

#[cfg(not(target_arch = "wasm32"))]
mod interop {
    use super::*;
    use crate::extensions::solve::config::read_config;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::panic;
    use std::panic::UnwindSafe;
    use std::slice;
    use vrp_pragmatic::format::problem::deserialize_problem;

    type Callback = extern "C" fn(*const c_char);

    fn to_string(pointer: *const c_char) -> String {
        let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
        std::str::from_utf8(slice).unwrap().to_string()
    }

    fn call_back(result: Result<String, String>, success: Callback, failure: Callback) {
        match result {
            Ok(ok) => {
                let ok = CString::new(ok.as_bytes()).unwrap();
                success(ok.as_ptr());
            }
            Err(err) => {
                let error = CString::new(err.as_bytes()).unwrap();
                failure(error.as_ptr());
            }
        };
    }

    fn catch_panic<F: FnOnce() + UnwindSafe>(failure: Callback, action: F) {
        if let Err(err) = panic::catch_unwind(|| action()) {
            let error = CString::new(format!("fatal: {:?}", err).as_bytes()).unwrap();
            failure(error.as_ptr());
        }
    }

    /// Returns a list of unique locations which can be used to request a routing matrix.
    /// A `problem` should be passed in `pragmatic` format.
    #[no_mangle]
    extern "C" fn get_routing_locations(problem: *const c_char, success: Callback, failure: Callback) {
        catch_panic(failure, || {
            let problem = to_string(problem);
            let problem = BufReader::new(problem.as_bytes());
            let result = deserialize_problem(problem)
                .map_err(|errors| get_errors_serialized(&errors))
                .and_then(|problem| get_locations_serialized(&problem));

            call_back(result, success, failure);
        });
    }

    /// Converts `problem` from format specified by `format` to `pragmatic` format.
    #[no_mangle]
    extern "C" fn convert_to_pragmatic(
        format: *const c_char,
        inputs: *const *const c_char,
        input_len: *const i32,
        success: Callback,
        failure: Callback,
    ) {
        catch_panic(failure, || {
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
                    let error = CString::new(err.as_bytes()).unwrap();
                    failure(error.as_ptr());
                }
            }
        });
    }

    /// Solves Vehicle Routing Problem passed in `pragmatic` format.
    #[no_mangle]
    extern "C" fn solve_pragmatic(
        problem: *const c_char,
        matrices: *const *const c_char,
        matrices_len: *const i32,
        config: *const c_char,
        success: Callback,
        failure: Callback,
    ) {
        catch_panic(failure, || {
            let problem = to_string(problem);
            let matrices = unsafe { slice::from_raw_parts(matrices, matrices_len as usize).to_vec() };
            let matrices = matrices.iter().map(|m| to_string(*m)).collect::<Vec<_>>();

            let result =
                if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }
                    .map_err(|errors| get_errors_serialized(&errors))
                    .and_then(|problem| {
                        read_config(BufReader::new(to_string(config).as_bytes()))
                            .map_err(|err| to_config_error(err.as_str()))
                            .map(|config| (problem, config))
                    })
                    .and_then(|(problem, config)| get_solution_serialized(Arc::new(problem), config));

            call_back(result, success, failure);
        });
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    extern crate serde_json;
    extern crate wasm_bindgen;

    use wasm_bindgen::prelude::*;

    use super::*;
    use vrp_pragmatic::format::problem::Matrix;

    /// Returns a list of unique locations which can be used to request a routing matrix.
    /// A `problem` should be passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn get_routing_locations(problem: &JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem = problem.into_serde().map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        get_locations_serialized(&problem)
            .map(|locations| JsValue::from_str(locations.as_str()))
            .map_err(|err| JsValue::from_str(err.to_string().as_str()))
    }

    /// Converts `problem` from format specified by `format` to `pragmatic` format.
    #[wasm_bindgen]
    pub fn convert_to_pragmatic(format: &str, inputs: &JsValue) -> Result<JsValue, JsValue> {
        let inputs: Vec<String> = inputs.into_serde().map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        let readers = inputs.iter().map(|input| BufReader::new(input.as_bytes())).collect();

        match import_problem(format, Some(readers)) {
            Ok(problem) => {
                let mut buffer = String::new();
                let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
                serialize_problem(writer, &problem).unwrap();

                Ok(JsValue::from_str(buffer.as_str()))
            }
            Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
        }
    }

    /// Solves Vehicle Routing Problem passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn solve_pragmatic(problem: &JsValue, matrices: &JsValue, config: &JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem = problem.into_serde().map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        let matrices: Vec<Matrix> = matrices.into_serde().map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        let problem = Arc::new(
            if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }.map_err(
                |errors| {
                    JsValue::from_str(errors.iter().map(|err| err.to_json()).collect::<Vec<_>>().join("\n").as_str())
                },
            )?,
        );

        let config: Config = config
            .into_serde()
            .map_err(|err| to_config_error(&err.to_string()))
            .map_err(|err| JsValue::from_str(err.as_str()))?;

        get_solution_serialized(problem, config)
            .map(|problem| JsValue::from_str(problem.as_str()))
            .map_err(|err| JsValue::from_str(err.as_str()))
    }
}

/// Gets locations serialized in json.
pub fn get_locations_serialized(problem: &Problem) -> Result<String, String> {
    // TODO validate the problem?

    let locations = get_unique_locations(&problem);
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    serde_json::to_writer_pretty(writer, &locations).map_err(|err| err.to_string())?;

    Ok(buffer)
}

/// Gets solution serialized in json.
pub fn get_solution_serialized(problem: Arc<CoreProblem>, config: Config) -> Result<String, String> {
    let (solution, _, metrics) = create_builder_from_config(problem.clone(), &config)
        .and_then(|builder| builder.build())
        .and_then(|solver| solver.solve())
        .map_err(|err| {
            FormatError::new(
                "E0003".to_string(),
                "cannot find any solution".to_string(),
                format!("please submit a bug and share original problem and routing matrix. Error: '{}'", err),
            )
            .to_json()
        })?;

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    if let Some(metrics) = metrics {
        (solution, metrics).write_pragmatic_json(&problem, writer)?;
    } else {
        solution.write_pragmatic_json(&problem, writer)?;
    }

    Ok(buffer)
}

/// Gets errors serialized in json.
pub fn get_errors_serialized(errors: &[FormatError]) -> String {
    errors.iter().map(|err| format!("{}", err)).collect::<Vec<_>>().join("\n")
}

fn to_config_error(err: &str) -> String {
    FormatError::new(
        "E0004".to_string(),
        "cannot read config".to_string(),
        format!("check config definition. Error: '{}'", err),
    )
    .to_json()
}
