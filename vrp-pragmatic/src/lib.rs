//! Pragmatic crates aims to solve real world VRP variations allowing users to specify their problems
//! via simple **pragmatic** json format.
//!
//!
//! For list of supported variants, please refer to [documentation](getting-started/features.md)
//!
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

mod constraints;
mod extensions;
mod utils;

pub mod json;

use crate::json::coord_index::CoordIndex;
use crate::json::problem::{deserialize_problem, PragmaticProblem};
use crate::json::solution::PragmaticSolution;
use chrono::{SecondsFormat, TimeZone, Utc};
use std::ffi::{CStr, CString};
use std::io::{BufReader, BufWriter};
use std::os::raw::c_char;
use std::panic::catch_unwind;
use std::slice;
use std::sync::Arc;
use vrp_solver::SolverBuilder;

use std::io::Read;
use std::slice::Iter;

struct StringReader<'a> {
    iter: Iter<'a, u8>,
}

impl<'a> StringReader<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { iter: data.as_bytes().iter() }
    }
}

impl<'a> Read for StringReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for i in 0..buf.len() {
            if let Some(x) = self.iter.next() {
                buf[i] = *x;
            } else {
                return Ok(i);
            }
        }
        Ok(buf.len())
    }
}

/// Returns serialized into json list of unique locations from serialized `problem` in order used
/// by routing matrix.
pub fn get_locations<R: Read>(problem: BufReader<R>) -> Result<String, String> {
    let problem = deserialize_problem(problem).map_err(|err| err.to_string())?;
    let locations = CoordIndex::new(&problem).unique();
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    serde_json::to_writer_pretty(writer, &locations).map_err(|err| err.to_string())?;

    Ok(buffer)
}

fn format_time(time: f64) -> String {
    Utc.timestamp(time as i64, 0).to_rfc3339_opts(SecondsFormat::Secs, true)
}

// TODO improve error propagation

type Callback = extern "C" fn(*const c_char);

fn to_string(pointer: *const c_char) -> String {
    let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
    std::str::from_utf8(slice).unwrap().to_string()
}

#[no_mangle]
extern "C" fn locations(problem: *const c_char, success: Callback, failure: Callback) {
    let result = catch_unwind(|| get_locations(BufReader::new(StringReader::new(&to_string(problem)))).ok().unwrap());

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

        let problem = Arc::new((problem, matrices).read_pragmatic().unwrap());

        let (solution, _, _) = SolverBuilder::default().build().solve(problem.clone()).unwrap();

        let mut buffer = String::new();
        let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
        solution.write_pragmatic(&problem, writer).ok();

        buffer
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
