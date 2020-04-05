//! Specifies logic to read problem and routing matrix from json input.
//!

mod model;
pub use self::model::*;

mod reader;
pub use self::reader::FormatError;
pub use self::reader::PragmaticProblem;
