//! A command line interface to import VRP problem from various formats.
//!

use super::*;

mod app;
pub use self::app::get_import_app;

mod command;
pub use self::command::run_import;
