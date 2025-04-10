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
#![deny(unsafe_code)] // NOTE: use deny instead forbid as we need allow unsafe code for c_interop
#![allow(clippy::items_after_test_module)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
mod helpers;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
#[path = "../tests/features/mod.rs"]
mod features;

#[cfg(test)]
#[path = "../tests/unit/lib_test.rs"]
mod lib_test;

pub use vrp_core as core;
pub use vrp_pragmatic as pragmatic;
#[cfg(feature = "scientific-format")]
pub use vrp_scientific as scientific;

pub mod extensions;

use crate::extensions::import::import_problem;
use crate::extensions::solve::config::{Config, create_builder_from_config};
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_core::models::Problem as CoreProblem;
use vrp_core::prelude::{GenericError, Solver};
use vrp_pragmatic::format::FormatError;
use vrp_pragmatic::format::problem::{PragmaticProblem, Problem, serialize_problem};
use vrp_pragmatic::format::solution::{PragmaticOutputType, write_pragmatic};
use vrp_pragmatic::get_unique_locations;
use vrp_pragmatic::validation::ValidationContext;

#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
mod c_interop {
    use super::*;
    use crate::extensions::solve::config::read_config;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::panic;
    use std::panic::UnwindSafe;
    use std::slice;
    use vrp_core::prelude::GenericError;
    use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem};
    use vrp_pragmatic::format::{CoordIndex, MultiFormatError};

    type Callback = extern "C" fn(*const c_char);

    fn to_string(pointer: *const c_char) -> String {
        let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
        std::str::from_utf8(slice).unwrap().to_string()
    }

    fn call_back(result: Result<String, GenericError>, success: Callback, failure: Callback) {
        match result {
            Ok(ok) => {
                let ok = CString::new(ok.as_bytes()).unwrap();
                success(ok.as_ptr());
            }
            Err(err) => {
                let error = CString::new(err.to_string().as_bytes()).unwrap();
                failure(error.as_ptr());
            }
        };
    }

    fn catch_panic<F: FnOnce() + UnwindSafe>(failure: Callback, action: F) {
        if let Err(err) = panic::catch_unwind(action) {
            let message = err
                .downcast_ref::<&str>()
                .cloned()
                .or_else(|| err.downcast_ref::<String>().map(|str| str.as_str()))
                .map(|msg| format!("panic: '{msg}'"))
                .unwrap_or_else(|| "panic with unknown type".to_string());

            let error = CString::new(message.as_bytes()).unwrap();
            failure(error.as_ptr());
        }
    }

    /// Returns a list of unique locations which can be used to request a routing matrix.
    /// A `problem` should be passed in `pragmatic` format.
    #[unsafe(no_mangle)]
    extern "C" fn get_routing_locations(problem: *const c_char, success: Callback, failure: Callback) {
        catch_panic(failure, || {
            let problem = to_string(problem);
            let problem = BufReader::new(problem.as_bytes());
            let result =
                deserialize_problem(problem).map_err(From::from).and_then(|problem| get_locations_serialized(&problem));

            call_back(result, success, failure);
        });
    }

    /// Converts `problem` from format specified by `format` to `pragmatic` format.
    #[unsafe(no_mangle)]
    extern "C" fn convert_to_pragmatic(
        format: *const c_char,
        inputs: *const *const c_char,
        input_len: usize,
        success: Callback,
        failure: Callback,
    ) {
        catch_panic(failure, || {
            let format = to_string(format);
            let inputs = unsafe { slice::from_raw_parts(inputs, input_len).to_vec() };
            let inputs = inputs.iter().map(|p| to_string(*p)).collect::<Vec<_>>();
            let readers = inputs.iter().map(|p| BufReader::new(p.as_bytes())).collect::<Vec<_>>();

            match import_problem(format.as_str(), Some(readers)) {
                Ok(problem) => {
                    let mut writer = BufWriter::new(Vec::new());
                    serialize_problem(&problem, &mut writer).unwrap();
                    let bytes = writer.into_inner().expect("cannot use writer");
                    let problem = CString::new(bytes).unwrap();

                    success(problem.as_ptr());
                }
                Err(err) => {
                    let error = CString::new(err.to_string().as_bytes()).unwrap();
                    failure(error.as_ptr());
                }
            }
        });
    }

    /// Validates Vehicle Routing Problem passed in `pragmatic` format.
    #[unsafe(no_mangle)]
    extern "C" fn validate_pragmatic(
        problem: *const c_char,
        matrices: *const *const c_char,
        matrices_len: usize,
        success: Callback,
        failure: Callback,
    ) {
        catch_panic(failure, || {
            let problem = to_string(problem);
            let matrices = unsafe { slice::from_raw_parts(matrices, matrices_len).to_vec() };
            let matrices = matrices.iter().map(|m| to_string(*m)).collect::<Vec<_>>();

            let problem = deserialize_problem(BufReader::new(problem.as_bytes()));
            let matrices = matrices
                .iter()
                .map(|matrix| deserialize_matrix(BufReader::new(matrix.as_bytes())))
                .collect::<Result<Vec<_>, _>>();

            let result = match (problem, matrices) {
                (Ok(problem), Ok(matrices)) => {
                    let matrices = if matrices.is_empty() { None } else { Some(&matrices) };
                    let coord_index = CoordIndex::new(&problem);

                    ValidationContext::new(&problem, matrices, &coord_index).validate()
                }
                (Err(errors), Ok(_)) | (Ok(_), Err(errors)) => Err(errors),
                (Err(errors1), Err(errors2)) => {
                    Err(MultiFormatError::from(errors1.into_iter().chain(errors2).collect::<Vec<_>>()))
                }
            }
            .map_err(From::from)
            .map(|_| "[]".to_string());

            call_back(result, success, failure);
        });
    }

    /// Solves Vehicle Routing Problem passed in `pragmatic` format.
    #[unsafe(no_mangle)]
    extern "C" fn solve_pragmatic(
        problem: *const c_char,
        matrices: *const *const c_char,
        matrices_len: usize,
        config: *const c_char,
        success: Callback,
        failure: Callback,
    ) {
        catch_panic(failure, || {
            let problem = to_string(problem);
            let matrices = unsafe { slice::from_raw_parts(matrices, matrices_len).to_vec() };
            let matrices = matrices.iter().map(|m| to_string(*m)).collect::<Vec<_>>();

            let result =
                if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }
                    .map_err(From::from)
                    .and_then(|problem| {
                        read_config(BufReader::new(to_string(config).as_bytes()))
                            .map_err(|err| GenericError::from(serialize_as_config_error(err.to_string().as_str())))
                            .map(|config| (problem, config))
                    })
                    .and_then(|(problem, config)| get_solution_serialized(Arc::new(problem), config));

            call_back(result, success, failure);
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::helpers::generate::SIMPLE_PROBLEM;

        #[test]
        fn can_use_to_string() {
            let c_str = CString::new("asd").unwrap();
            assert_eq!(to_string(c_str.as_ptr() as *const c_char), "asd".to_string());
        }

        #[test]
        fn can_use_callback() {
            // TODO: real check that data is passed is not really there

            extern "C" fn success1(_: *const c_char) {}
            extern "C" fn failure1(_: *const c_char) {
                unreachable!()
            }
            call_back(Ok("success".to_string()), success1, failure1);

            extern "C" fn success2(_: *const c_char) {
                unreachable!()
            }
            extern "C" fn failure2(_: *const c_char) {}
            call_back(Err("failure".into()), success2, failure2);
        }

        #[test]
        fn can_catch_panic_with_string_literal() {
            extern "C" fn callback(msg: *const c_char) {
                assert_eq!(to_string(msg), "panic: 'invaders detected!'");
            }
            catch_panic(callback, || panic!("invaders detected!"));
            catch_panic(callback, || panic!("invaders {}!", "detected"));
        }

        #[test]
        fn can_get_locations() {
            extern "C" fn success(locations: *const c_char) {
                let locations = to_string(locations);
                assert!(locations.starts_with('['));
                assert!(locations.ends_with(']'));
                assert!(locations.len() > 2);
            }
            extern "C" fn failure(err: *const c_char) {
                unreachable!("got {}", to_string(err))
            }

            let problem = CString::new(SIMPLE_PROBLEM).unwrap();
            get_routing_locations(problem.as_ptr() as *const c_char, success, failure)
        }

        #[test]
        fn can_validate_simple_problem() {
            extern "C" fn success(solution: *const c_char) {
                assert_eq!(to_string(solution), "[]")
            }
            extern "C" fn failure(err: *const c_char) {
                unreachable!("{}", to_string(err))
            }

            let problem = CString::new(SIMPLE_PROBLEM).unwrap();
            let matrices = CString::new("[]").unwrap();

            validate_pragmatic(
                problem.as_ptr() as *const c_char,
                matrices.as_ptr() as *const *const c_char,
                0,
                success,
                failure,
            );
        }

        #[test]
        fn can_validate_empty_problem() {
            extern "C" fn success(solution: *const c_char) {
                unreachable!("got {}", to_string(solution))
            }
            extern "C" fn failure(err: *const c_char) {
                let err = to_string(err);
                assert!(err.contains("E0000"));
                assert!(err.contains("cause"));
                assert!(err.contains("action"));
            }

            let problem = CString::new("").unwrap();
            let matrices = CString::new("[]").unwrap();

            validate_pragmatic(
                problem.as_ptr() as *const c_char,
                matrices.as_ptr() as *const *const c_char,
                0,
                success,
                failure,
            );
        }

        #[test]
        fn can_solve_problem() {
            extern "C" fn success(solution: *const c_char) {
                let solution = to_string(solution);
                assert!(solution.starts_with('{'));
                assert!(solution.ends_with('}'));
                assert!(solution.len() > 2);
            }
            extern "C" fn failure(err: *const c_char) {
                unreachable!("{}", to_string(err))
            }

            let problem = CString::new(SIMPLE_PROBLEM).unwrap();
            let matrices = CString::new("[]").unwrap();
            let config = CString::new("{\"termination\": {\"max-generations\": 1}}").unwrap();

            solve_pragmatic(
                problem.as_ptr() as *const c_char,
                matrices.as_ptr() as *const *const c_char,
                0,
                config.as_ptr() as *const c_char,
                success,
                failure,
            );
        }
    }
}

#[cfg(feature = "py_bindings")]
#[cfg(not(target_arch = "wasm32"))]
mod py_interop {
    use super::*;
    use crate::extensions::solve::config::read_config;
    use pyo3::exceptions::PyOSError;
    use pyo3::prelude::*;
    use std::io::BufReader;
    use vrp_pragmatic::format::CoordIndex;
    use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem};

    // TODO avoid duplications between 3 interop approaches

    /// Converts `problem` from format specified by `format` to `pragmatic` format.
    #[pyfunction]
    fn convert_to_pragmatic(format: &str, inputs: Vec<String>) -> PyResult<String> {
        let readers = inputs.iter().map(|p| BufReader::new(p.as_bytes())).collect::<Vec<_>>();
        import_problem(format, Some(readers))
            .and_then(|problem| {
                let mut writer = BufWriter::new(Vec::new());
                serialize_problem(&problem, &mut writer).unwrap();

                writer
                    .into_inner()
                    .map_err(|err| format!("BufWriter: {err}").into())
                    .and_then(|bytes| String::from_utf8(bytes).map_err(|err| format!("StringUTF8: {err}").into()))
            })
            .map_err(|err| PyOSError::new_err(err.to_string()))
    }

    /// Returns a list of unique locations which can be used to request a routing matrix.
    #[pyfunction]
    fn get_routing_locations(problem: String) -> PyResult<String> {
        deserialize_problem(BufReader::new(problem.as_bytes()))
            .map_err(From::from)
            .and_then(|problem| get_locations_serialized(&problem))
            .map_err(|err| PyOSError::new_err(err.to_string()))
    }

    /// Validates and solves Vehicle Routing Problem.
    #[pyfunction]
    fn solve_pragmatic(problem: String, matrices: Vec<String>, config: String) -> PyResult<String> {
        // validate first
        deserialize_problem(BufReader::new(problem.as_bytes()))
            .and_then(|problem| {
                matrices
                    .iter()
                    .map(|m| deserialize_matrix(BufReader::new(m.as_bytes())))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|matrices| (problem, matrices))
            })
            .and_then(|(problem, matrices)| {
                let matrices = if matrices.is_empty() { None } else { Some(&matrices) };
                let coord_index = CoordIndex::new(&problem);

                ValidationContext::new(&problem, matrices, &coord_index).validate()
            })
            .map_err(|errs| PyOSError::new_err(errs.to_string()))?;

        // try solve problem
        if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }
            .map_err(From::from)
            .and_then(|problem| {
                read_config(BufReader::new(config.as_bytes()))
                    .map_err(|err| GenericError::from(serialize_as_config_error(err.to_string().as_str())))
                    .map(|config| (problem, config))
            })
            .and_then(|(problem, config)| get_solution_serialized(Arc::new(problem), config))
            .map_err(|err| PyOSError::new_err(err.to_string()))
    }

    #[pymodule]
    fn vrp_cli(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(convert_to_pragmatic, m)?)?;
        m.add_function(wrap_pyfunction!(get_routing_locations, m)?)?;
        m.add_function(wrap_pyfunction!(solve_pragmatic, m)?)?;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    extern crate serde_json;
    extern crate wasm_bindgen;

    use super::*;
    use vrp_pragmatic::format::CoordIndex;
    use vrp_pragmatic::format::problem::Matrix;
    use wasm_bindgen::prelude::*;

    /// Returns a list of unique locations which can be used to request a routing matrix.
    /// A `problem` should be passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn get_routing_locations(problem: JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem =
            serde_wasm_bindgen::from_value(problem).map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        get_locations_serialized(&problem)
            .map(|locations| JsValue::from_str(locations.as_str()))
            .map_err(|err| JsValue::from_str(err.to_string().as_str()))
    }

    /// Validates Vehicle Routing Problem passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn validate_pragmatic(problem: JsValue, matrices: JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem =
            serde_wasm_bindgen::from_value(problem).map_err(|err| JsValue::from_str(err.to_string().as_str()))?;
        let matrices: Vec<Matrix> =
            serde_wasm_bindgen::from_value(matrices).map_err(|err| JsValue::from_str(err.to_string().as_str()))?;
        let coord_index = CoordIndex::new(&problem);

        let matrices = if matrices.is_empty() { None } else { Some(&matrices) };
        ValidationContext::new(&problem, matrices, &coord_index)
            .validate()
            .map_err(|errs| JsValue::from_str(errs.to_json().as_str()))
            .map(|_| JsValue::from_str("[]"))
    }

    /// Converts `problem` from format specified by `format` to `pragmatic` format.
    #[wasm_bindgen]
    pub fn convert_to_pragmatic(format: &str, inputs: JsValue) -> Result<JsValue, JsValue> {
        let inputs: Vec<String> =
            serde_wasm_bindgen::from_value(inputs).map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        let readers = inputs.iter().map(|input| BufReader::new(input.as_bytes())).collect();

        match import_problem(format, Some(readers)) {
            Ok(problem) => {
                let mut writer = BufWriter::new(Vec::new());
                serialize_problem(&problem, &mut writer).unwrap();

                let bytes = writer.into_inner().unwrap();
                let result = String::from_utf8(bytes).unwrap();

                Ok(JsValue::from_str(result.as_str()))
            }
            Err(err) => Err(JsValue::from_str(err.to_string().as_str())),
        }
    }

    /// Solves Vehicle Routing Problem passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn solve_pragmatic(problem: JsValue, matrices: JsValue, config: JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem =
            serde_wasm_bindgen::from_value(problem).map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        let matrices: Vec<Matrix> =
            serde_wasm_bindgen::from_value(matrices).map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        let problem = Arc::new(
            if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }
                .map_err(|errs| JsValue::from_str(errs.to_json().as_str()))?,
        );

        let config: Config = serde_wasm_bindgen::from_value(config)
            .map_err(|err| serialize_as_config_error(&err.to_string()))
            .map_err(|err| JsValue::from_str(err.as_str()))?;

        get_solution_serialized(problem, config)
            .map(|problem| JsValue::from_str(problem.as_str()))
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }
}

/// Gets locations serialized in json.
pub fn get_locations_serialized(problem: &Problem) -> Result<String, GenericError> {
    // TODO validate the problem?

    let locations = get_unique_locations(problem);
    serde_json::to_string_pretty(&locations).map_err(|err| err.to_string().into())
}

/// Gets solution serialized in json.
pub fn get_solution_serialized(problem: Arc<CoreProblem>, config: Config) -> Result<String, GenericError> {
    let solution = create_builder_from_config(problem.clone(), Default::default(), &config)
        .and_then(|builder| builder.build())
        .map(|config| Solver::new(problem.clone(), config))
        .and_then(|solver| solver.solve())
        .map_err(|err| {
            FormatError::new(
                "E0003".to_string(),
                "cannot find any solution".to_string(),
                format!("please submit a bug and share original problem and routing matrix. Error: '{err}'"),
            )
            .to_json()
        })?;

    let output_type = if config.output.and_then(|output_cfg| output_cfg.include_geojson).unwrap_or(false) {
        PragmaticOutputType::Combined
    } else {
        Default::default()
    };

    let mut writer = BufWriter::new(Vec::new());
    write_pragmatic(problem.as_ref(), &solution, output_type, &mut writer)?;

    let bytes = writer.into_inner().map_err(|err| format!("{err}"))?;
    let result = String::from_utf8(bytes).map_err(|err| format!("{err}"))?;

    Ok(result)
}

fn serialize_as_config_error(err: &str) -> String {
    FormatError::new(
        "E0004".to_string(),
        "cannot read config".to_string(),
        format!("check config definition. Error: '{err}'"),
    )
    .to_json()
}
