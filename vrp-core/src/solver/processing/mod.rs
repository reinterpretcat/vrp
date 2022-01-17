//! Contains pre and post processing logic.

use crate::construction::heuristics::InsertionContext;
use rosomaxa::prelude::*;

mod advance_departure;
pub use self::advance_departure::AdvanceDeparture;

mod unassignment_reason;
pub use self::unassignment_reason::UnassignmentReason;

mod vicinity_clustering;
pub use self::vicinity_clustering::{VicinityClustering, VicinityDimension};
