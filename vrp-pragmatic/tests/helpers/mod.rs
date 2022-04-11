#[cfg(test)]
#[path = "../../../vrp-core/tests/helpers/macros.rs"]
#[macro_use]
pub mod macros;

/// A helper trait to create Location from some type.
pub trait ToLocation {
    fn to_loc(self) -> Location;
}

impl ToLocation for (f64, f64) {
    fn to_loc(self) -> Location {
        let (lat, lng) = self;
        Location::new_coordinate(lat, lng)
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
