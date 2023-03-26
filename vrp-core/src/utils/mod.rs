//! A collection of various utility helpers.

// Reimport rosomaxa utils
pub use rosomaxa::utils::*;

pub use self::comparison::*;
pub use self::types::Either;

mod comparison;
mod types;
