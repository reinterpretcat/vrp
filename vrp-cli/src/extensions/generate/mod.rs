//! Generate command helpers.

mod plan;
use self::plan::generate_plan;

mod fleet;
use self::fleet::generate_fleet;

mod prototype;
use self::prototype::generate_from_prototype;

use std::io::{BufReader, Read};
use vrp_core::prelude::GenericError;
use vrp_core::utils::Random;
use vrp_pragmatic::format::problem::*;

/// Generates a pragmatic problem.
pub fn generate_problem<R: Read>(
    input_format: &str,
    prototype_readers: Option<Vec<BufReader<R>>>,
    locations_reader: Option<BufReader<R>>,
    job_size: usize,
    vehicles_size: usize,
    area_size: Option<f64>,
) -> Result<Problem, GenericError> {
    let locations = if let Some(locations_reader) = locations_reader {
        Some(deserialize_locations(locations_reader).map_err(|errs| errs.to_string())?)
    } else {
        None
    };

    let problem_proto = match (input_format, prototype_readers) {
        (_, Some(readers)) if readers.len() != 1 => {
            Err(format!("expecting one input file, specified: '{}'", readers.len()))
        }
        ("pragmatic", Some(mut readers)) if readers.len() == 1 => {
            deserialize_problem(readers.swap_remove(0)).map_err(|errs| errs.to_string())
        }
        _ => Err(format!("unknown format: '{input_format}'")),
    }?;

    generate_from_prototype(&problem_proto, locations, job_size, vehicles_size, area_size)
}

fn get_random_item<'a, T>(items: &'a [T], rnd: &Random) -> Option<&'a T> {
    if items.is_empty() {
        return None;
    }

    let idx = rnd.uniform_int(0, items.len() as i32 - 1) as usize;
    items.get(idx)
}
