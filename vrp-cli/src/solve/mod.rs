//! A command line interface to solve variations of *Vehicle Routing Problem*.
//!
//! For more details please check [docs](cli/index.html)

mod app;
pub use self::app::get_solve_app;

mod command;
pub use self::command::run_solve;

use super::*;
