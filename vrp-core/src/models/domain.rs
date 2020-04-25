use crate::construction::constraints::ConstraintPipeline;
use crate::construction::heuristics::InsertionContext;
use crate::models::problem::{ActivityCost, Actor, Fleet, Job, Jobs, TransportCost};
use crate::models::solution::{Registry, Route};
use crate::refinement::objectives::Objective;
use hashbrown::HashMap;
use std::any::Any;
use std::sync::Arc;

/// Specifies a type used to store any values regarding problem and solution.
pub type Extras = HashMap<String, Arc<dyn Any + Send + Sync>>;

pub type SolutionObjective = dyn Objective<Solution = InsertionContext> + Send + Sync;

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

    /// Specifies objective function for the problem.
    pub objective: Arc<SolutionObjective>,

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
    pub unassigned: HashMap<Job, i32>,

    /// Specifies index for storing extra data of arbitrary type.
    pub extras: Arc<Extras>,
}

/// An enumeration which specifies how jobs should be ordered in tour.
pub enum LockOrder {
    /// Jobs can be reshuffled in any order.
    Any,
    /// Jobs cannot be reshuffled, but new job can be inserted in between.
    Sequence,
    /// Jobs cannot be reshuffled and no jobs can be inserted in between.
    Strict,
}

/// An enumeration which specifies how other jobs can be inserted in tour.
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
    /// Lock order.
    pub order: LockOrder,
    /// Lock position.
    pub position: LockPosition,
    /// Jobs affected by the lock.
    pub jobs: Vec<Job>,
}

/// Contains information about jobs locked to specific actors.
pub struct Lock {
    /// Specifies condition when locked jobs can be assigned to specific actor
    pub condition: Arc<dyn Fn(&Actor) -> bool + Sync + Send>,
    /// Specifies lock details.
    pub details: Vec<LockDetail>,
}

impl LockDetail {
    /// Creates a new instance of [`LockDetail`].
    pub fn new(order: LockOrder, position: LockPosition, jobs: Vec<Job>) -> Self {
        Self { order, position, jobs }
    }
}

impl Lock {
    /// Creates a new instance of [`Lock`].
    pub fn new(condition: Arc<dyn Fn(&Actor) -> bool + Sync + Send>, details: Vec<LockDetail>) -> Self {
        Self { condition, details }
    }
}
