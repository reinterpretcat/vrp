#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

#[cfg(test)]
#[path = "../tests/features/mod.rs"]
pub mod features;

mod constraints;
mod extensions;
mod utils;

pub mod json;

use crate::json::problem::HereProblem;
use crate::json::solution::HereSolution;
use solver::SolverBuilder;
use std::ffi::{CStr, CString};
use std::io::BufWriter;
use std::os::raw::c_char;
use std::panic::catch_unwind;
use std::slice;
use std::sync::Arc;

// TODO improve error propagation

type Callback = extern "C" fn(*const c_char);

fn to_string(pointer: *const c_char) -> String {
    let slice = unsafe { CStr::from_ptr(pointer).to_bytes() };
    std::str::from_utf8(slice).unwrap().to_string()
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

        let problem = Arc::new((problem, matrices).read_here().unwrap());

        let (solution, _, _) = SolverBuilder::default().build().solve(problem.clone()).unwrap();

        let mut buffer = String::new();
        let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
        solution.write_here(&problem, writer).ok();

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
