//! Exposes the [`crate::interop::json`] contract over the C ABI.
//!
//! Results and errors are passed to the caller through the `success` and `failure` callbacks. An
//! error is always the json representation of the encountered problems, see
//! [`crate::interop::json`].

#[cfg(test)]
#[path = "../../tests/unit/interop/c_api_test.rs"]
mod c_api_test;

use crate::interop::json;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic;
use std::panic::UnwindSafe;
use std::slice;
use vrp_pragmatic::format::MultiFormatError;

type Callback = extern "C" fn(*const c_char);

/// Returns a list of unique locations which can be used to request a routing matrix.
/// A `problem` should be passed in `pragmatic` format.
#[unsafe(no_mangle)]
extern "C" fn get_routing_locations(problem: *const c_char, success: Callback, failure: Callback) {
    catch_panic(failure, || {
        call_back(json::routing_locations(&to_string(problem)), success, failure);
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
        let inputs = to_strings(inputs, input_len);

        call_back(json::convert(&format, &inputs), success, failure);
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
        let matrices = to_strings(matrices, matrices_len);

        call_back(json::validate(&problem, &matrices).map(|_| "[]".to_string()), success, failure);
    });
}

/// Validates and solves Vehicle Routing Problem passed in `pragmatic` format.
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
        let matrices = to_strings(matrices, matrices_len);
        let config = to_string(config);

        call_back(json::solve(&problem, &matrices, &config), success, failure);
    });
}

fn to_string(pointer: *const c_char) -> String {
    assert!(!pointer.is_null(), "received a null string pointer");
    let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
    std::str::from_utf8(slice).unwrap().to_string()
}

fn to_strings(pointer: *const *const c_char, len: usize) -> Vec<String> {
    // A null pointer is the conventional C representation of an empty array. `from_raw_parts`
    // requires a non-null, aligned pointer even when the length is zero, so return before touching
    // it. This also makes the documented "no matrices" case natural for C callers.
    if len == 0 {
        return Vec::new();
    }

    assert!(!pointer.is_null(), "received a null array pointer with a non-zero length");
    let pointers = unsafe { slice::from_raw_parts(pointer, len).to_vec() };

    pointers.iter().map(|pointer| to_string(*pointer)).collect()
}

fn call_back(result: Result<String, MultiFormatError>, success: Callback, failure: Callback) {
    match result {
        Ok(ok) => {
            let ok = CString::new(ok.as_bytes()).unwrap();
            success(ok.as_ptr());
        }
        Err(err) => call_failure(err, failure),
    };
}

fn call_failure(err: MultiFormatError, failure: Callback) {
    let error = CString::new(err.to_json().as_bytes()).unwrap();
    failure(error.as_ptr());
}

fn catch_panic<F: FnOnce() + UnwindSafe>(failure: Callback, action: F) {
    if let Err(err) = panic::catch_unwind(action) {
        let message = err
            .downcast_ref::<&str>()
            .cloned()
            .or_else(|| err.downcast_ref::<String>().map(|str| str.as_str()))
            .map(|msg| format!("panic: '{msg}'"))
            .unwrap_or_else(|| "panic with unknown type".to_string());

        call_failure(
            json::interop_error(
                "language binding panicked",
                format!("please submit a bug and share the input passed to the binding. Error: '{message}'"),
            ),
            failure,
        );
    }
}
