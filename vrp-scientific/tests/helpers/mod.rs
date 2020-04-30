#[cfg(test)]
#[path = "../../../vrp-core/tests/helpers/macros.rs"]
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
use std::fs::File;
use std::io::BufReader;
use vrp_core::models::Problem;

pub fn get_test_resource(resource_path: &str) -> std::io::Result<File> {
    let mut path = std::env::current_dir()?;
    path.push("tests");
    path.push(resource_path);

    File::open(path)
}

pub fn create_c101_25_problem() -> Problem {
    BufReader::new(get_test_resource("../../examples/data/scientific/solomon/C101.25.txt").unwrap())
        .read_solomon()
        .unwrap()
}

pub fn create_c101_100_problem() -> Problem {
    BufReader::new(get_test_resource("../../examples/data/scientific/solomon/C101.100.txt").unwrap())
        .read_solomon()
        .unwrap()
}

pub fn create_lc101_problem() -> Problem {
    BufReader::new(get_test_resource("../../examples/data/scientific/lilim/LC101.txt").unwrap()).read_lilim().unwrap()
}
