//! A command line interface to solve variations of *Vehicle Routing Problem*.
//!

use super::*;

mod app;
pub use self::app::get_solve_app;

mod command;
pub use self::command::run_solve;
