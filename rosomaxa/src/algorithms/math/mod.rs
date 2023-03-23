//! This module contains some statistic related functionality.

mod distance;
pub use self::distance::*;

mod remedian;
pub use self::remedian::{Remedian, RemedianUsize};

mod statistics;
pub use self::statistics::*;
