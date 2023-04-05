use crate::construction::heuristics::UnassignmentInfo;
use crate::models::problem::*;
use crate::models::solution::{Registry, Route};
use crate::utils::short_type_name;
use hashbrown::HashMap;
use rustc_hash::FxHasher;
use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::hash::BuildHasherDefault;
use std::sync::Arc;

/// Specifies a type used to store any values regarding problem and solution.
pub type Extras = HashMap<String, Arc<dyn Any + Send + Sync>, BuildHasherDefault<FxHasher>>;

/// Defines VRP problem.
pub struct Problem {
    /// Specifies used fleet.
    pub fleet: Arc<Fleet>,

    /// Specifies all jobs.
    pub jobs: Arc<Jobs>,

    /// Specifies jobs which preassigned to specific vehicles and/or drivers.
    pub locks: Vec<Arc<Lock>>,

    /// Specifies optimization goal with the corresponding global/local objectives and invariants.
    pub goal: Arc<GoalContext>,

    /// Specifies activity costs.
    pub activity: Arc<dyn ActivityCost + Send + Sync>,

    /// Specifies transport costs.
    pub transport: Arc<dyn TransportCost + Send + Sync>,

    /// Specifies index for storing extra parameters of arbitrary type.
    pub extras: Arc<Extras>,
}

impl Debug for Problem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("fleet", &self.fleet)
            .field("jobs", &self.jobs.size())
            .field("locks", &self.locks.len())
            .field("goal", self.goal.as_ref())
            .finish_non_exhaustive()
    }
}

/// Represents a VRP solution.
pub struct Solution {
    /// Actor's registry.
    pub registry: Registry,

    /// List of assigned routes.
    pub routes: Vec<Route>,

    /// List of unassigned jobs within reason code.
    pub unassigned: Vec<(Job, UnassignmentInfo)>,

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
    pub condition_fn: Arc<dyn Fn(&Actor) -> bool + Sync + Send>,
    /// Specifies lock details.
    pub details: Vec<LockDetail>,
    /// Specifies whether route is created or not in solution from beginning.
    /// True means that route is not created till evaluation.
    pub is_lazy: bool,
}

impl LockDetail {
    /// Creates a new instance of `LockDetail`.
    pub fn new(order: LockOrder, position: LockPosition, jobs: Vec<Job>) -> Self {
        Self { order, position, jobs }
    }
}

impl Lock {
    /// Creates a new instance of `Lock`.
    pub fn new(condition: Arc<dyn Fn(&Actor) -> bool + Sync + Send>, details: Vec<LockDetail>, is_lazy: bool) -> Self {
        Self { condition_fn: condition, details, is_lazy }
    }
}
