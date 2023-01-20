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

#[cfg(test)]
#[path = "../tests/unit/lib_test.rs"]
mod lib_test;

pub use vrp_core as core;
pub use vrp_pragmatic as pragmatic;
#[cfg(feature = "scientific-format")]
pub use vrp_scientific as scientific;

pub mod extensions;

use crate::extensions::import::import_problem;
use crate::extensions::solve::config::{create_builder_from_config, Config};
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_core::models::Problem as CoreProblem;
use vrp_core::prelude::Solver;
use vrp_pragmatic::format::problem::{serialize_problem, PragmaticProblem, Problem};
use vrp_pragmatic::format::solution::PragmaticSolution;
use vrp_pragmatic::format::FormatError;
use vrp_pragmatic::get_unique_locations;
use vrp_pragmatic::validation::ValidationContext;

#[cfg(not(target_arch = "wasm32"))]
mod c_interop {
    use super::*;
    use crate::extensions::solve::config::read_config;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::panic;
    use std::panic::UnwindSafe;
    use std::slice;
    use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem};
    use vrp_pragmatic::format::CoordIndex;

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
        if let Err(err) = panic::catch_unwind(action) {
            let message = err
                .downcast_ref::<&str>()
                .cloned()
                .or_else(|| err.downcast_ref::<String>().map(|str| str.as_str()))
                .map(|msg| format!("panic: '{}'", msg))
                .unwrap_or_else(|| "panic with unknown type".to_string());

            let error = CString::new(message.as_bytes()).unwrap();
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

    /// Validates Vehicle Routing Problem passed in `pragmatic` format.
    #[no_mangle]
    extern "C" fn validate_pragmatic(
        problem: *const c_char,
        matrices: *const *const c_char,
        matrices_len: *const i32,
        success: Callback,
        failure: Callback,
    ) {
        catch_panic(failure, || {
            let problem = to_string(problem);
            let matrices = unsafe { slice::from_raw_parts(matrices, matrices_len as usize).to_vec() };
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
                (Err(errors), Ok(_)) | (Ok(_), Err(errors)) => Err(errors.into_iter().collect()),
                (Err(errors1), Err(errors2)) => Err(errors1.into_iter().chain(errors2.into_iter()).collect()),
            }
            .map_err(|err| FormatError::format_many_to_json(&err))
            .map(|_| "[]".to_string());

            call_back(result, success, failure);
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
            extern "C" fn success1(_: *const c_char) {}
            extern "C" fn failure1(_: *const c_char) {
                unreachable!()
            }
            call_back(Ok("success".to_string()), success1, failure1);

            extern "C" fn success2(_: *const c_char) {
                unreachable!()
            }
            extern "C" fn failure2(_: *const c_char) {}
            call_back(Err("failure".to_string()), success2, failure2);

            let result = std::panic::catch_unwind(|| {
                call_back(Err("failure".to_string()), success1, failure1);
            });
            assert!(result.is_err());
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
                std::ptr::null::<i32>(),
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
                assert!(err.starts_with('['));
                assert!(err.contains("code"));
                assert!(err.contains("cause"));
                assert!(err.contains("action"));
                assert!(err.ends_with(']'));
            }

            let problem = CString::new("").unwrap();
            let matrices = CString::new("[]").unwrap();

            validate_pragmatic(
                problem.as_ptr() as *const c_char,
                matrices.as_ptr() as *const *const c_char,
                std::ptr::null::<i32>(),
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
                std::ptr::null::<i32>(),
                config.as_ptr() as *const c_char,
                success,
                failure,
            );
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(tarpaulin)))]
mod py_interop {
    use super::*;
    use crate::extensions::solve::config::read_config;
    use pyo3::exceptions::PyOSError;
    use pyo3::prelude::*;
    use std::io::BufReader;
    use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem};
    use vrp_pragmatic::format::CoordIndex;

    // TODO avoid duplications between 3 interop approaches

    /// Converts `problem` from format specified by `format` to `pragmatic` format.
    #[pyfunction]
    fn convert_to_pragmatic(format: &str, inputs: Vec<String>) -> PyResult<String> {
        let readers = inputs.iter().map(|p| BufReader::new(p.as_bytes())).collect::<Vec<_>>();
        import_problem(format, Some(readers))
            .map(|problem| {
                let mut buffer = String::new();
                serialize_problem(unsafe { BufWriter::new(buffer.as_mut_vec()) }, &problem).unwrap();

                buffer
            })
            .map_err(PyOSError::new_err)
    }

    /// Returns a list of unique locations which can be used to request a routing matrix.
    #[pyfunction]
    fn get_routing_locations(problem: String) -> PyResult<String> {
        deserialize_problem(BufReader::new(problem.as_bytes()))
            .map_err(|errors| get_errors_serialized(&errors))
            .and_then(|problem| get_locations_serialized(&problem))
            .map_err(PyOSError::new_err)
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
            .map_err(|errs| PyOSError::new_err(FormatError::format_many_to_json(&errs)))?;

        // try solve problem
        if matrices.is_empty() { problem.read_pragmatic() } else { (problem, matrices).read_pragmatic() }
            .map_err(|errors| get_errors_serialized(&errors))
            .and_then(|problem| {
                read_config(BufReader::new(config.as_bytes()))
                    .map_err(|err| to_config_error(err.as_str()))
                    .map(|config| (problem, config))
            })
            .and_then(|(problem, config)| get_solution_serialized(Arc::new(problem), config))
            .map_err(PyOSError::new_err)
    }

    #[pymodule]
    fn vrp_cli(_py: Python, m: &PyModule) -> PyResult<()> {
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
    use vrp_pragmatic::format::problem::Matrix;
    use vrp_pragmatic::format::CoordIndex;
    use wasm_bindgen::prelude::*;

    /// Returns a list of unique locations which can be used to request a routing matrix.
    /// A `problem` should be passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn get_routing_locations(problem: &JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem = problem.into_serde().map_err(|err| JsValue::from_str(err.to_string().as_str()))?;

        get_locations_serialized(&problem)
            .map(|locations| JsValue::from_str(locations.as_str()))
            .map_err(|err| JsValue::from_str(err.to_string().as_str()))
    }

    /// Validates Vehicle Routing Problem passed in `pragmatic` format.
    #[wasm_bindgen]
    pub fn validate_pragmatic(problem: &JsValue, matrices: &JsValue) -> Result<JsValue, JsValue> {
        let problem: Problem = problem.into_serde().map_err(|err| JsValue::from_str(err.to_string().as_str()))?;
        let matrices: Vec<Matrix> = matrices.into_serde().map_err(|err| JsValue::from_str(err.to_string().as_str()))?;
        let coord_index = CoordIndex::new(&problem);

        let matrices = if matrices.is_empty() { None } else { Some(&matrices) };
        ValidationContext::new(&problem, matrices, &coord_index)
            .validate()
            .map_err(|err| JsValue::from_str(FormatError::format_many_to_json(&err).as_str()))
            .map(|_| JsValue::from_str("[]"))
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

    let locations = get_unique_locations(problem);
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    serde_json::to_writer_pretty(writer, &locations).map_err(|err| err.to_string())?;

    Ok(buffer)
}

/// Gets solution serialized in json.
pub fn get_solution_serialized(problem: Arc<CoreProblem>, config: Config) -> Result<String, String> {
    let (solution, cost, metrics) = create_builder_from_config(problem.clone(), Default::default(), &config)
        .and_then(|builder| builder.build())
        .map(|config| Solver::new(problem.clone(), config))
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
        (&solution, cost, &metrics).write_pragmatic_json(&problem, writer)?;
    } else {
        (&solution, cost).write_pragmatic_json(&problem, writer)?;
    }

    Ok(buffer)
}

/// Gets errors serialized in free form.
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
