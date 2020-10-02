//! Import command helpers

mod csv;
pub use self::csv::*;

mod hre;
pub use self::hre::*;

use std::io::{BufReader, Read};
use vrp_pragmatic::format::problem::Problem;

/// Imports solution from specific format into pragmatic.
pub fn import_problem<R: Read>(input_format: &str, readers: Option<Vec<BufReader<R>>>) -> Result<Problem, String> {
    match (input_format, readers) {
        ("csv", Some(mut readers)) if readers.len() == 2 => {
            let jobs = readers.swap_remove(0);
            let vehicles = readers.swap_remove(0);
            read_csv_problem(jobs, vehicles).map_err(|err| format!("cannot read csv: {}", err))
        }
        ("csv", _) => Err("csv format expects two files with jobs and vehicles as an input".to_string()),
        ("hre", Some(mut readers)) if readers.len() == 1 => {
            let problem = readers.swap_remove(0);
            deserialize_hre_problem(problem).map_err(|err| format!("cannot read problem from hre json: '{}'", err))
        }
        ("hre", _) => Err("hre format expects one input file".to_string()),
        _ => Err(format!("unknown format: '{}'", input_format)),
    }
}
