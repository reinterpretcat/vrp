//! Import command helpers

mod csv;
pub use self::csv::*;

use std::io::{BufReader, Read};
use vrp_core::prelude::GenericError;
use vrp_pragmatic::format::problem::Problem;

/// Imports solution from specific format into pragmatic.
pub fn import_problem<R: Read>(
    input_format: &str,
    readers: Option<Vec<BufReader<R>>>,
) -> Result<Problem, GenericError> {
    match (input_format, readers) {
        ("csv", Some(mut readers)) if readers.len() == 2 => {
            let jobs = readers.swap_remove(0);
            let vehicles = readers.swap_remove(0);
            read_csv_problem(jobs, vehicles).map_err(|err| format!("cannot read csv: {err}").into())
        }
        ("csv", _) => Err("csv format expects two files with jobs and vehicles as an input".into()),
        _ => Err(format!("unknown format: '{input_format}'").into()),
    }
}
