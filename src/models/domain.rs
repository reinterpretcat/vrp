use crate::models::problem::Job;
use crate::models::solution::{Registry, Route};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Problem {}

/// Represents a VRP solution.
pub struct Solution {
    /// Actor's registry.
    pub registry: Arc<Registry>,

    /// List of assigned routes.
    pub routes: Vec<Arc<Route>>,

    /// Map of unassigned jobs within reason code.
    pub unassigned: HashMap<Arc<Job>, i32>,

    /// Specifies index for storing extra data of arbitrary type.
    pub extras: HashMap<String, Box<dyn Any>>,
}
