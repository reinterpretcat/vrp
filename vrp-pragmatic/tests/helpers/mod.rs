#[cfg(test)]
#[path = "../../../vrp-core/tests/helpers/macros.rs"]
#[macro_use]
pub mod macros;

use std::fs::File;

pub fn get_test_resource(resource_path: &str) -> std::io::Result<File> {
    let mut path = std::env::current_dir()?;
    path.push("tests");
    path.push(resource_path);

    File::open(path)
}

pub trait ToLocation {
    fn to_loc(&self) -> Location;
}

impl ToLocation for Vec<f64> {
    fn to_loc(&self) -> Location {
        assert_eq!(self.len(), 2);
        Location::new(*self.get(0).unwrap(), *self.get(1).unwrap())
    }
}

mod core;
pub use self::core::*;

mod solver;
pub use self::solver::*;

pub mod problem;
pub use self::problem::*;

pub mod solution;
pub use self::solution::*;
use crate::json::Location;
