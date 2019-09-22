use crate::construction::constraints::ConstraintPipeline;
use crate::models::problem::{ActivityCost, Fleet, Job, Jobs, TransportCost};
use crate::models::solution::{Registry, Route};
use crate::objectives::ObjectiveFunction;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Defines VRP problem.
pub struct Problem {
    /// Specifies used fleet.
    pub fleet: Arc<Fleet>,
    /// Specifies all jobs.
    pub jobs: Arc<Jobs>,
    /// Specifies jobs which preassigned to specific vehicles and/or drivers.
    pub locks: Vec<Arc<Lock>>,
    /// Specifies constraints pipeline.
    pub constraint: Arc<ConstraintPipeline>,
    /// Specifies objective function.
    pub objective: Arc<dyn ObjectiveFunction>,
    /// Specifies activity costs.
    pub activity: Arc<dyn ActivityCost>,
    /// Specifies transport costs.
    pub transport: Arc<dyn TransportCost>,
    /// Specifies index for storing extra parameters of arbitrary type.
    pub extras: Arc<HashMap<String, Box<dyn Any>>>,
}

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

/// Specifies jobs locked to specific actors.
pub struct Lock {}
