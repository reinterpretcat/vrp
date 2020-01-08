//! Specifies logic to read problem and routing matrix from json input.
//!
//! Please refer to [documentation](concepts/pragmatic/index.md) problem and routing matrix definitions.

mod deserializer;
pub use self::deserializer::*;

mod reader;
pub use self::reader::PragmaticProblem;
