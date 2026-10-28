//! Exposes the [`crate::interop::json`] contract to javascript.
//!
//! Arguments accept a plain javascript object or a json string, results are returned as objects, and
//! errors are thrown as an `Error` whose message is the json representation of the problems found.

use crate::interop::json;
use js_sys::{Array, Error, JSON};
use vrp_pragmatic::format::MultiFormatError;
use wasm_bindgen::prelude::*;

// Embedded so that `wasm-pack` emits a typed `.d.ts` instead of one using `any`. This file is generated
// from the rust format types and is not committed; run `./vrp-cli/bindings/generate.sh` before a local
// wasm build. Release workflows generate it before packaging the crate.
#[wasm_bindgen(typescript_custom_section)]
const FORMAT_TYPES: &str = include_str!("../../bindings/typescript/format_types.d.ts");

#[wasm_bindgen]
extern "C" {
    /// A problem definition, as an object or a json string.
    #[wasm_bindgen(typescript_type = "Problem | string")]
    pub type ProblemArg;

    /// Routing matrices, as objects or json strings. May be omitted to approximate distances.
    #[wasm_bindgen(typescript_type = "(Matrix | string)[] | null | undefined")]
    pub type MatricesArg;

    /// A solver configuration, as an object or a json string. May be omitted to use the defaults.
    #[wasm_bindgen(typescript_type = "Config | string | null | undefined")]
    pub type ConfigArg;

    /// Unique locations, in the order the solver expects routing data in.
    #[wasm_bindgen(typescript_type = "Location[]")]
    pub type LocationsResult;

    /// Problems found while validating, empty when the input is valid.
    #[wasm_bindgen(typescript_type = "FormatError[]")]
    pub type ValidationResult;

    /// A problem definition converted to pragmatic format.
    #[wasm_bindgen(typescript_type = "Problem")]
    pub type ProblemResult;

    /// A solution to the given problem.
    #[wasm_bindgen(typescript_type = "Solution")]
    pub type SolutionResult;
}

// without this a panic reaches the caller as a bare `unreachable executed` with no message. It does
// not make a panic recoverable, the instance is still left unusable.
#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

/// Returns a list of unique locations which can be used to request a routing matrix.
/// A `problem` should be passed in `pragmatic` format.
#[wasm_bindgen]
pub fn get_routing_locations(problem: &ProblemArg) -> Result<LocationsResult, JsValue> {
    let problem = to_json_string(problem, "problem")?;

    to_js(json::routing_locations(&problem))
}

/// Converts `problem` from format specified by `format` to `pragmatic` format.
#[wasm_bindgen]
pub fn convert_to_pragmatic(format: &str, inputs: Vec<String>) -> Result<ProblemResult, JsValue> {
    to_js(json::convert(format, &inputs))
}

/// Validates Vehicle Routing Problem passed in `pragmatic` format.
#[wasm_bindgen]
pub fn validate_pragmatic(problem: &ProblemArg, matrices: &MatricesArg) -> Result<ValidationResult, JsValue> {
    let problem = to_json_string(problem, "problem")?;
    let matrices = to_json_strings(matrices)?;

    to_js(json::validate(&problem, &matrices).map(|_| "[]".to_string()))
}

/// Validates and solves Vehicle Routing Problem passed in `pragmatic` format.
#[wasm_bindgen]
pub fn solve_pragmatic(
    problem: &ProblemArg,
    matrices: &MatricesArg,
    config: &ConfigArg,
) -> Result<SolutionResult, JsValue> {
    let problem = to_json_string(problem, "problem")?;
    let matrices = to_json_strings(matrices)?;
    let config = to_json_string(config, "config")?;

    to_js(json::solve(&problem, &matrices, &config))
}

fn to_json_string(value: &impl AsRef<JsValue>, subject: &str) -> Result<String, JsValue> {
    let value = value.as_ref();

    if let Some(string) = value.as_string() {
        return Ok(string);
    }

    // an omitted argument is treated as an empty document, which keeps `config` optional
    if value.is_undefined() || value.is_null() {
        return Ok("{}".to_string());
    }

    JSON::stringify(value)
        .map_err(|err| {
            adapter_error(
                &format!("cannot serialize {subject} argument as json"),
                format!("pass a JSON-serializable value. Error: '{}'", js_error_message(&err)),
            )
        })?
        .as_string()
        .ok_or_else(|| {
            adapter_error(
                &format!("cannot serialize {subject} argument as json"),
                "pass a JSON-serializable value".to_string(),
            )
        })
}

fn to_json_strings(value: &impl AsRef<JsValue>) -> Result<Vec<String>, JsValue> {
    let value = value.as_ref();

    if value.is_undefined() || value.is_null() {
        return Ok(Vec::new());
    }

    if !Array::is_array(value) {
        return Err(adapter_error(
            "cannot read matrices argument",
            "pass matrices as an array of objects or json strings".to_string(),
        ));
    }

    Array::from(value).iter().map(|item| to_json_string(&item, "matrix")).collect()
}

fn to_js<T: JsCast>(result: Result<String, MultiFormatError>) -> Result<T, JsValue> {
    match result {
        Ok(json) => JSON::parse(&json).map(JsCast::unchecked_into).map_err(|err| {
            adapter_error(
                "cannot return result to javascript",
                format!("please submit a bug and share the original input. Error: '{}'", js_error_message(&err)),
            )
        }),
        Err(err) => Err(Error::new(&err.to_json()).into()),
    }
}

fn adapter_error(cause: &str, action: String) -> JsValue {
    Error::new(&json::interop_error(cause, action).to_json()).into()
}

fn js_error_message(err: &JsValue) -> String {
    err.as_string()
        .or_else(|| err.dyn_ref::<Error>().map(|error| error.message().into()))
        .unwrap_or_else(|| "unknown javascript error".to_string())
}
