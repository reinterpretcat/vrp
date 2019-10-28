use crate::construction::constraints::ConstraintPipeline;
use crate::models::problem::{ActivityCost, Actor, Fleet, Job, Jobs, TransportCost};
use crate::models::solution::{Registry, Route};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

pub type Extras = Arc<HashMap<String, Box<dyn Any + Send + Sync>>>;

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
    /// Specifies activity costs.
    pub activity: Arc<dyn ActivityCost + Send + Sync>,
    /// Specifies transport costs.
    pub transport: Arc<dyn TransportCost + Send + Sync>,
    /// Specifies index for storing extra parameters of arbitrary type.
    pub extras: Arc<Extras>,
}

/// Represents a VRP solution.
pub struct Solution {
    /// Actor's registry.
    pub registry: Registry,

    /// List of assigned routes.
    pub routes: Vec<Route>,

    /// Map of unassigned jobs within reason code.
    pub unassigned: HashMap<Arc<Job>, i32>,

    /// Specifies index for storing extra data of arbitrary type.
    pub extras: Arc<Extras>,
}

/// Specifies how jobs should be ordered in tour.
pub enum LockOrder {
    /// Jobs can be reshuffled in any order.
    Any,
    /// Jobs cannot be reshuffled, but new job can be inserted in between.
    Sequence,
    /// Jobs cannot be reshuffled and no jobs can be inserted in between.
    Strict,
}

/// Specifies how other jobs can be inserted in tour.
#[derive(Clone)]
pub enum LockPosition {
    /// No specific position.
    Any,
    /// First job follows departure.
    Departure,
    /// Last job is before arrival.
    Arrival,
    /// First and last jobs should be between departure and arrival.
    Fixed,
}

/// Specifies lock details.
pub struct LockDetail {
    pub order: LockOrder,
    pub position: LockPosition,
    pub jobs: Vec<Arc<Job>>,
}

/// Specifies jobs locked to specific actors.
pub struct Lock {
    /// Specifies condition when locked jobs can be assigned to specific actor
    pub condition: Arc<dyn Fn(&Arc<Actor>) -> bool + Sync + Send>,
    /// Specifies lock details.
    pub details: Vec<LockDetail>,
}
