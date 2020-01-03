//! Models to keep track of state during solution evaluation.

const OP_START_MSG: &str = "Optional start is not yet implemented.";

mod route;
pub use self::route::*;

mod models;
pub use self::models::*;
