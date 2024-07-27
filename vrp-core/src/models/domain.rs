use crate::construction::heuristics::UnassignmentInfo;
use crate::models::common::{Cost, Location};
use crate::models::problem::*;
use crate::models::solution::{Registry, Route};
use crate::models::*;
use rosomaxa::evolution::TelemetryMetrics;
use rosomaxa::prelude::*;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Defines a VRP problem. You can use a [`ProblemBuilder`] to create the one.
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
    pub activity: Arc<dyn ActivityCost>,

    /// Specifies transport costs.
    pub transport: Arc<dyn TransportCost>,

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
    /// A total solution cost.
    /// Definition of the cost depends on VRP variant.
    pub cost: Cost,

    /// Actor's registry.
    pub registry: Registry,

    /// List of assigned routes.
    pub routes: Vec<Route>,

    /// List of unassigned jobs within reason code.
    pub unassigned: Vec<(Job, UnassignmentInfo)>,

    /// An optional telemetry metrics if available.
    pub telemetry: Option<TelemetryMetrics>,
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
    /// Specifies condition when locked jobs can be assigned to a specific actor
    pub condition_fn: Arc<dyn Fn(&Actor) -> bool + Sync + Send>,
    /// Specifies lock details.
    pub details: Vec<LockDetail>,
    /// Specifies whether route is created or not in solution from the beginning.
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

/// Specifies a function to group actors based on their similarity.
pub type FleetGroupKeyFn = dyn Fn(&Actor) -> usize + Send + Sync;

/// Provides way to build a VRP definition.
#[derive(Default)]
pub struct ProblemBuilder {
    jobs: Vec<Job>,
    vehicles: Vec<Vehicle>,
    #[allow(clippy::type_complexity)]
    group_key_fn: Option<Box<dyn Fn(&[Arc<Actor>]) -> Box<FleetGroupKeyFn>>>,
    goal: Option<Arc<GoalContext>>,
    activity: Option<Arc<dyn ActivityCost>>,
    transport: Option<Arc<dyn TransportCost>>,
    extras: Option<Arc<Extras>>,
}

impl ProblemBuilder {
    /// Adds a job to the collection of the things to be done.
    pub fn add_job(mut self, job: Job) -> Self {
        self.jobs.push(job);
        self
    }

    /// Adds multiple jobs to the collection of the things to be done.
    pub fn add_jobs(mut self, jobs: impl Iterator<Item = Job>) -> Self {
        self.jobs.extend(jobs);
        self
    }

    /// Add a vehicle to the fleet.
    /// At least one has to be provided.
    pub fn add_vehicle(mut self, vehicle: Vehicle) -> Self {
        self.vehicles.push(vehicle);
        self
    }

    /// Add multiple vehicles to the fleet.
    /// At least one has to be provided.
    pub fn add_vehicles(mut self, vehicles: impl Iterator<Item = Vehicle>) -> Self {
        self.vehicles.extend(vehicles);
        self
    }

    /// Sets a vehicle similarity function which allows grouping of similar vehicles together.
    /// That helps the solver to take more effective decisions job-vehicle assignment.
    /// Optional: when omitted, only vehicles with the same `profile.index` are grouped together.
    pub fn with_vehicle_similarity(
        mut self,
        group_key_fn: impl Fn(&[Arc<Actor>]) -> Box<FleetGroupKeyFn> + 'static,
    ) -> Self {
        self.group_key_fn = Some(Box::new(group_key_fn));
        self
    }

    /// Adds a goal of optimization. Use [GoalContextBuilder] to create the one.
    /// A required field.
    pub fn with_goal(mut self, goal: GoalContext) -> Self {
        self.goal = Some(Arc::new(goal));
        self
    }

    /// Adds a transport distance/duration estimation logic. A typical implementation will normally
    /// wrap routing distance/duration matrices.
    /// A required field.
    pub fn with_transport_cost(mut self, transport: Arc<dyn TransportCost>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Adds an activity service time estimation logic.
    /// An optional field: [SimpleActivityCost] will be used by default.
    pub fn with_activity_cost(mut self, activity: Arc<dyn ActivityCost>) -> Self {
        self.activity = Some(activity);
        self
    }

    /// Adds an extras: an extension mechanism to pass arbitrary properties associated within
    /// the problem definition.
    /// An optional field.
    pub fn with_extras(mut self, extras: Extras) -> Self {
        self.extras = Some(Arc::new(extras));
        self
    }

    /// Builds a problem definition.
    /// Returns [Err] in case of an invalid configuration.
    pub fn build(mut self) -> GenericResult<Problem> {
        if self.jobs.is_empty() {
            return Err("empty list of jobs: specify at least one job".into());
        }

        if self.vehicles.is_empty() {
            return Err("empty list of vehicles: specify at least one vehicle".into());
        }

        // analyze user input
        let transport = self.transport.take().ok_or_else(|| {
            GenericError::from("no information about routing data: use 'with_transport_cost' method to specify it")
        })?;
        let activity = self.activity.take().unwrap_or_else(|| Arc::new(SimpleActivityCost::default()));
        let goal = self
            .goal
            .take()
            .ok_or_else(|| GenericError::from("unknown goal of optimization: use 'with_goal' method to set it"))?;
        let extras = self.extras.take().unwrap_or_else(|| Arc::new(Extras::default()));

        // setup fleet
        // NOTE: driver concept is not fully supported yet, but we must provide at least one.
        let driver = Arc::new(Driver::empty());
        let vehicles = self.vehicles.into_iter().map(Arc::new).collect();
        let group_key = self.group_key_fn.take().unwrap_or_else(|| Box::new(|_| Box::new(|a| a.vehicle.profile.index)));
        let fleet = Arc::new(Fleet::new(vec![driver], vehicles, group_key));

        // setup jobs
        let jobs = Arc::new(Jobs::new(fleet.as_ref(), self.jobs, transport.as_ref()));

        Ok(Problem { fleet, jobs, locks: vec![], goal, activity, transport, extras })
    }
}

impl Solution {
    /// Iterates through all tours and returns locations of each activity in the order they are visited.
    pub fn get_locations(&self) -> impl Iterator<Item = impl Iterator<Item = Location> + '_> + '_ {
        self.routes.iter().map(|route| route.tour.all_activities().map(|activity| activity.place.location))
    }
}
