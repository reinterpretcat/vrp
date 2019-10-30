#[cfg(test)]
#[path = "../../../core/tests/helpers/macros.rs"]
#[macro_use]
pub mod macros;

mod analysis;
pub use self::analysis::*;

mod solomon;
pub use self::solomon::SolomonBuilder;

mod lilim;
pub use self::lilim::LilimBuilder;

use crate::lilim::LilimProblem;
use crate::solomon::SolomonProblem;
use core::models::Problem;
use std::fs::File;

pub fn get_test_resource(resource_path: &str) -> std::io::Result<File> {
    let mut path = std::env::current_dir()?;
    path.push("tests");
    path.push(resource_path);

    File::open(path)
}

pub fn create_c101_25_problem() -> Problem {
    get_test_resource("data/solomon/C101.25.txt").unwrap().parse_solomon().unwrap()
}

pub fn create_c101_100_problem() -> Problem {
    get_test_resource("data/solomon/C101.100.txt").unwrap().parse_solomon().unwrap()
}

pub fn create_lc101_problem() -> Problem {
    get_test_resource("data/lilim/LC101.txt").unwrap().parse_lilim().unwrap()
}
