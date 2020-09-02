#[cfg(test)]
#[path = "../../../vrp-core/tests/helpers/macros.rs"]
#[macro_use]
pub mod macros;

pub trait ToLocation {
    fn to_loc(&self) -> Location;
}

impl ToLocation for Vec<f64> {
    fn to_loc(&self) -> Location {
        assert_eq!(self.len(), 2);
        Location::new_coordinate(*self.get(0).unwrap(), *self.get(1).unwrap())
    }
}

mod core;
pub use self::core::*;

mod fixtures;
pub use self::fixtures::*;

mod solver;
pub use self::solver::*;

pub mod problem;
pub use self::problem::*;

pub mod solution;
pub use self::solution::*;
use crate::format::Location;
