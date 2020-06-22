//! Generate command helpers.

mod plan;
use self::plan::generate_plan;

mod prototype;
pub use self::prototype::generate_from_prototype;

use std::io::{BufReader, Read};
use vrp_pragmatic::format::problem::{deserialize_problem, Problem};
use vrp_pragmatic::format::FormatError;

/// Generates a pragmatic problem.
pub fn generate_problem<R: Read>(
    input_format: &str,
    readers: Option<Vec<BufReader<R>>>,
    job_size: usize,
    area_size: Option<f64>,
) -> Result<Problem, String> {
    match (input_format, readers) {
        ("pragmatic", Some(readers)) if readers.len() != 1 => {
            Err(format!("expecting one input file, specified: '{}'", readers.len()))
        }
        ("pragmatic", Some(mut readers)) if readers.len() == 1 => {
            let problem_reader = readers.swap_remove(0);
            let problem_proto = deserialize_problem(problem_reader)
                .map_err(|errors| FormatError::format_many(errors.as_slice(), "\t\n"))?;
            generate_from_prototype(&problem_proto, job_size, area_size)
        }
        _ => Err(format!("unknown format: '{}'", input_format)),
    }
}
