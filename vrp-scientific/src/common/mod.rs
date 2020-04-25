//! Contains common text reading and writing functionality.

use std::sync::Arc;
use vrp_core::models::SolutionObjective;

mod text_reader;
pub use self::text_reader::*;

mod text_writer;
pub use self::text_writer::write_text_solution;

fn create_default_objective() -> Arc<SolutionObjective> {
    unimplemented!()
}
