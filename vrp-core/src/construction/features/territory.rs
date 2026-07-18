//! A feature that builds balanced, capacity-aware territories around a per-driver anchor.
//!
//! PULL keeps each job on the nearest compatible driver that still has spare quota; PUSH
//! greedily moves over-quota surplus to the nearest under-quota driver at proximity cost. Both are
//! in proximity units, so there is no free weight between them. The anchor is supplied by the caller
//! (objective config), never derived here.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/territory_test.rs"]
mod territory_test;

use super::vehicle_distance::get_job_location;
use super::*;
use crate::models::problem::{DriverIdDimension, VehicleIdDimension};
use std::collections::HashMap;

pub use crate::construction::features::vehicle_distance::ActorJobCompatibilityFn;

custom_solution_state!(TerritoryFitness typeof TerritoryFitnessData);
custom_tour_state!(TerritoryRouteLoad typeof Float);

/// Distance metric used to measure how far a job sits from a driver's anchor.
#[derive(Clone, Copy, Debug)]
pub enum TerritoryProximity {
    /// Uses approximate travel distance between locations.
    Distance,
    /// Uses approximate travel time between locations.
    Time,
}

/// The metric used to size each driver's quota (capped share of total demand) when balancing
/// territories. `None` (see [`TerritoryFeatureBuilder::set_balance`]) disables quotas entirely,
/// giving every driver unlimited spare capacity and reducing PULL to pure nearest-anchor territory.
#[derive(Clone, Copy, Debug)]
pub enum TerritoryBalance {
    /// v1: bills each job's proximity to its nearest anchor (in the configured proximity
    /// metric); Distance and Duration are currently equivalent — true per-metric travel
    /// balancing is future work.
    Distance,
    /// v1: bills each job's proximity to its nearest anchor (in the configured proximity
    /// metric); Distance and Duration are currently equivalent — true per-metric travel
    /// balancing is future work.
    Duration,
    /// Balances on job (activity) count.
    Activities,
    /// Balances on a caller-supplied per-job production value (see
    /// [`TerritoryFeatureBuilder::set_job_value_fn`]).
    ProductionValue,
}

/// Cached, solution-level fitness contributions of the territory objective.
#[derive(Clone, Default)]
pub struct TerritoryFitnessData {
    /// Total PULL: excess proximity incurred by jobs served away from their nearest
    /// compatible, under-quota driver anchor.
    pub pull: Cost,
    /// Total PUSH: cost of moving over-quota surplus to the nearest under-quota driver.
    pub push: Cost,
}

/// A per-driver grouping key: `actor.vehicle.dimens.get_driver_id()`, falling back to the
/// vehicle id when no driver id dimension is set.
type DriverKey = String;

fn driver_key(actor: &Actor) -> DriverKey {
    actor
        .vehicle
        .dimens
        .get_driver_id()
        .cloned()
        .or_else(|| actor.vehicle.dimens.get_vehicle_id().cloned())
        .unwrap_or_default()
}

/// Provides a way to build a feature that keeps jobs within balanced, capacity-aware territories
/// around a per-driver anchor.
pub struct TerritoryFeatureBuilder {
    name: String,
    transport: Option<Arc<dyn TransportCost + Send + Sync>>,
    actors: Option<Vec<Arc<Actor>>>,
    jobs: Option<Arc<Jobs>>,
    compatibility_fn: Option<ActorJobCompatibilityFn>,
    proximity: TerritoryProximity,
    balance: Option<TerritoryBalance>,
    anchors: HashMap<DriverKey, Location>,
    job_value_fn: Option<Arc<dyn Fn(&Job) -> Float + Send + Sync>>,
}

impl TerritoryFeatureBuilder {
    /// Creates a new instance of `TerritoryFeatureBuilder`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            transport: None,
            actors: None,
            jobs: None,
            compatibility_fn: None,
            proximity: TerritoryProximity::Distance,
            balance: None,
            anchors: HashMap::new(),
            job_value_fn: None,
        }
    }

    /// Sets the transport cost model used to measure proximity.
    pub fn set_transport(mut self, t: Arc<dyn TransportCost + Send + Sync>) -> Self {
        self.transport = Some(t);
        self
    }

    /// Sets the fleet actors to consider when finding the nearest compatible driver.
    pub fn set_actors(mut self, a: Vec<Arc<Actor>>) -> Self {
        self.actors = Some(a);
        self
    }

    /// Sets the job set used to compute the self-normalization reference scale and, when
    /// `balance` is set, the total demand used to size quotas.
    pub fn set_jobs(mut self, j: Arc<Jobs>) -> Self {
        self.jobs = Some(j);
        self
    }

    /// Sets the compatibility function that checks if an actor can serve a job.
    pub fn set_compatibility_fn<F: Fn(&Job, &Actor) -> bool + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.compatibility_fn = Some(Arc::new(f));
        self
    }

    /// Sets the proximity metric (distance or time) used to measure how far a job sits from an
    /// anchor. Defaults to [`TerritoryProximity::Distance`].
    pub fn set_proximity(mut self, p: TerritoryProximity) -> Self {
        self.proximity = p;
        self
    }

    /// Sets the metric used to size each driver's quota. `None` (the default) disables quotas,
    /// so every driver has unlimited spare capacity and PULL reduces to pure territory.
    pub fn set_balance(mut self, b: Option<TerritoryBalance>) -> Self {
        self.balance = b;
        self
    }

    /// Sets the per-driver anchor locations, keyed by driver id (or vehicle id when no driver id
    /// dimension is set). Anchors are supplied by the caller, never derived from a dimension.
    pub fn set_anchors(mut self, a: HashMap<String, Location>) -> Self {
        self.anchors = a;
        self
    }

    /// Sets the function that reads a job's production value, used when balancing on
    /// [`TerritoryBalance::ProductionValue`]. Defaults to a constant `1.0` per job.
    pub fn set_job_value_fn<F: Fn(&Job) -> Float + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.job_value_fn = Some(Arc::new(f));
        self
    }

    /// Builds the feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let transport = self.transport.take().ok_or_else(|| GenericError::from("territory: transport required"))?;
        let actors = self.actors.take().ok_or_else(|| GenericError::from("territory: actors required"))?;
        let jobs = self.jobs.take().ok_or_else(|| GenericError::from("territory: jobs required"))?;
        let compatibility_fn =
            self.compatibility_fn.take().ok_or_else(|| GenericError::from("territory: compatibility_fn required"))?;
        let job_value_fn = self.job_value_fn.take().unwrap_or_else(|| Arc::new(|_| 1.0));

        let shared = Arc::new(TerritoryShared::new(
            transport,
            actors,
            jobs,
            compatibility_fn,
            self.proximity,
            self.balance,
            self.anchors,
            job_value_fn,
        ));

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_objective(TerritoryObjective { shared: shared.clone() })
            .with_state(TerritoryState { shared })
            .build()
    }
}

/// Shared compute logic and dependencies for the territory objective and state.
///
/// Both [`TerritoryObjective`] and [`TerritoryState`] go through the same PULL/PUSH calculation;
/// keeping it in one place avoids the trap of fixing the formula in one copy and forgetting the
/// other.
struct TerritoryShared {
    transport: Arc<dyn TransportCost + Send + Sync>,
    actors: Vec<Arc<Actor>>,
    compatibility_fn: ActorJobCompatibilityFn,
    proximity: TerritoryProximity,
    balance: Option<TerritoryBalance>,
    job_value_fn: Arc<dyn Fn(&Job) -> Float + Send + Sync>,
    profile: Profile,
    anchors: HashMap<DriverKey, Location>,
    /// Per-driver quota: the balance metric's ideal share, proportional to each driver's
    /// available time window. Empty when `balance` is `None` (unlimited spare capacity).
    quotas: HashMap<DriverKey, Float>,
    /// Reference magnitude used by `fitness_scale` to normalize PULL + PUSH.
    reference: Cost,
}

impl TerritoryShared {
    #[allow(clippy::too_many_arguments)]
    fn new(
        transport: Arc<dyn TransportCost + Send + Sync>,
        actors: Vec<Arc<Actor>>,
        jobs: Arc<Jobs>,
        compatibility_fn: ActorJobCompatibilityFn,
        proximity: TerritoryProximity,
        balance: Option<TerritoryBalance>,
        anchors: HashMap<DriverKey, Location>,
        job_value_fn: Arc<dyn Fn(&Job) -> Float + Send + Sync>,
    ) -> Self {
        let profile = actors.first().map(|a| a.vehicle.profile.clone()).unwrap_or_default();
        let mut shared = Self {
            transport,
            actors,
            compatibility_fn,
            proximity,
            balance,
            job_value_fn,
            profile,
            anchors,
            quotas: HashMap::new(),
            reference: 1.0,
        };
        shared.quotas = shared.compute_quotas(&jobs);
        shared.reference = shared.compute_reference(&jobs).max(1.0);
        shared
    }

    /// Proximity between two locations, per the configured metric and the fleet's (single)
    /// profile.
    fn proximity(&self, from: Location, to: Location) -> Float {
        match self.proximity {
            TerritoryProximity::Distance => self.transport.distance_approx(&self.profile, from, to),
            TerritoryProximity::Time => self.transport.duration_approx(&self.profile, from, to),
        }
    }

    /// The balance metric's contribution for a single job: `0.0` when balance is disabled.
    fn job_metric(&self, job: &Job) -> Float {
        match self.balance {
            None => 0.0,
            Some(TerritoryBalance::Activities) => 1.0,
            Some(TerritoryBalance::ProductionValue) => (self.job_value_fn)(job),
            Some(TerritoryBalance::Distance) | Some(TerritoryBalance::Duration) => {
                get_job_location(job).map(|loc| self.nearest_anchor_prox(loc, job)).unwrap_or(0.0)
            }
        }
    }

    /// Computes each driver's quota: the total balance metric spread over drivers proportionally
    /// to their available time window. Empty when balance is disabled (unlimited spare capacity).
    fn compute_quotas(&self, jobs: &Jobs) -> HashMap<DriverKey, Float> {
        if self.balance.is_none() {
            return HashMap::new();
        }
        let total_metric: Float = jobs.all().iter().map(|j| self.job_metric(j)).sum();
        let mut cap: HashMap<DriverKey, Float> = HashMap::new();
        for actor in self.actors.iter() {
            let window = (actor.detail.time.end - actor.detail.time.start).max(0.0);
            *cap.entry(driver_key(actor)).or_insert(0.0) += window;
        }
        let total_cap: Float = cap.values().sum::<Float>().max(1e-6);
        cap.into_iter().map(|(k, c)| (k, total_metric * c / total_cap)).collect()
    }

    /// The self-normalization reference: the sum, over all jobs, of the proximity to each job's
    /// nearest compatible anchor. Guarded to stay positive by the caller.
    fn compute_reference(&self, jobs: &Jobs) -> Cost {
        jobs.all().iter().filter_map(|job| get_job_location(job).map(|loc| self.nearest_anchor_prox(loc, job))).sum()
    }

    /// Proximity from a job's location to its nearest compatible anchor, ignoring quotas.
    fn nearest_anchor_prox(&self, job_loc: Location, job: &Job) -> Float {
        self.actors
            .iter()
            .filter(|a| (self.compatibility_fn)(job, a))
            .filter_map(|a| self.anchors.get(&driver_key(a)).copied())
            .map(|anchor| self.proximity(job_loc, anchor))
            .min_by(|x, y| x.total_cmp(y))
            .unwrap_or(0.0)
    }

    /// The balance metric total for a single route: the sum of `job_metric` across its jobs.
    /// Shared between `loads` (solution-wide) and the per-route `TerritoryRouteLoad` cache.
    fn route_load(&self, route_ctx: &RouteContext) -> Float {
        route_ctx.route().tour.jobs().map(|j| self.job_metric(j)).sum()
    }

    /// Current per-driver load: the sum of the balance metric across all jobs on that driver's
    /// route(s) in the given solution. Includes every driver with a quota, even if idle.
    fn loads(&self, solution: &SolutionContext) -> HashMap<DriverKey, Float> {
        let mut loads: HashMap<DriverKey, Float> = self.quotas.keys().map(|k| (k.clone(), 0.0)).collect();
        for route_ctx in solution.routes.iter() {
            let key = driver_key(&route_ctx.route().actor);
            *loads.entry(key).or_insert(0.0) += self.route_load(route_ctx);
        }
        loads
    }

    /// Proximity from a job's location to the nearest compatible anchor that still has spare
    /// quota. When balance is disabled, every driver has spare quota (unlimited capacity).
    fn nearest_spare_anchor(&self, job_loc: Location, job: &Job, loads: &HashMap<DriverKey, Float>) -> Option<Float> {
        self.actors
            .iter()
            .filter(|a| (self.compatibility_fn)(job, a))
            .filter_map(|a| {
                let key = driver_key(a);
                let has_spare = match self.balance {
                    None => true,
                    Some(_) => loads.get(&key).copied().unwrap_or(0.0) < *self.quotas.get(&key).unwrap_or(&Float::MAX),
                };
                if has_spare { self.anchors.get(&key).copied() } else { None }
            })
            .map(|anchor| self.proximity(job_loc, anchor))
            .min_by(|x, y| x.total_cmp(y))
    }

    /// Total PULL for the solution: for each assigned job, the excess proximity of its assigned
    /// driver's anchor over the nearest compatible, under-quota anchor.
    fn pull(&self, solution: &SolutionContext) -> Cost {
        let loads = self.loads(solution);
        let mut total = 0.0;
        for route_ctx in solution.routes.iter() {
            let actor = &route_ctx.route().actor;
            let Some(assigned_anchor) = self.anchors.get(&driver_key(actor)).copied() else { continue };
            for job in route_ctx.route().tour.jobs() {
                let Some(loc) = get_job_location(job) else { continue };
                let assigned = self.proximity(loc, assigned_anchor);
                let reference = self.nearest_spare_anchor(loc, job, &loads).unwrap_or(assigned);
                total += (assigned - reference).max(0.0);
            }
        }
        total
    }

    /// Total PUSH for the solution: a greedy lower bound on the cost of moving every over-quota
    /// driver's surplus to its *nearest* deficit driver's anchor (ignoring deficit capacity, i.e.
    /// not a full min-cost transport). Zero when no driver is over quota.
    fn push(&self, solution: &SolutionContext) -> Cost {
        if self.balance.is_none() || self.quotas.is_empty() {
            return 0.0;
        }
        let loads = self.loads(solution);

        // Deficit drivers (with an anchor) and their remaining room.
        let deficits: Vec<Location> = self
            .quotas
            .iter()
            .filter_map(|(key, &quota)| {
                let load = loads.get(key).copied().unwrap_or(0.0);
                if load + 1e-9 < quota { self.anchors.get(key).copied() } else { None }
            })
            .collect();
        if deficits.is_empty() {
            return 0.0;
        }

        let mut total = 0.0;
        for (key, &quota) in self.quotas.iter() {
            let load = loads.get(key).copied().unwrap_or(0.0);
            let surplus = load - quota;
            if surplus <= 1e-9 {
                continue;
            }
            let Some(anchor) = self.anchors.get(key).copied() else { continue };
            let nearest = deficits.iter().map(|&d| self.proximity(anchor, d)).min_by(|x, y| x.total_cmp(y)).unwrap_or(0.0);
            total += surplus * nearest;
        }
        total
    }

    /// The dual-price marginal contribution of assigning `job` to `route_ctx`'s driver: `0.0`
    /// while the driver is under quota (per the cached route load), otherwise the job's balance
    /// metric priced at the nearest other anchor (a proxy for the nearest deficit driver, kept
    /// cheap for the hot insertion-evaluation loop).
    fn push_marginal(&self, route_ctx: &RouteContext, job: &Job) -> Cost {
        if self.balance.is_none() {
            return 0.0;
        }
        let key = driver_key(&route_ctx.route().actor);
        let Some(anchor) = self.anchors.get(&key).copied() else { return 0.0 };
        let load = route_ctx.state().get_territory_route_load().copied().unwrap_or(0.0);
        if load < *self.quotas.get(&key).unwrap_or(&Float::MAX) {
            return 0.0;
        }

        let nearest_other = self
            .quotas
            .keys()
            .filter(|k| **k != key)
            .filter_map(|k| self.anchors.get(k).copied())
            .map(|a| self.proximity(anchor, a))
            .min_by(|x, y| x.total_cmp(y))
            .unwrap_or(0.0);

        self.job_metric(job) * nearest_other
    }
}

struct TerritoryObjective {
    shared: Arc<TerritoryShared>,
}
struct TerritoryState {
    shared: Arc<TerritoryShared>,
}

impl FeatureObjective for TerritoryObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .state
            .get_territory_fitness()
            .map(|d| d.pull + d.push)
            .unwrap_or_else(|| self.shared.pull(&solution.solution) + self.shared.push(&solution.solution))
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                let Some(loc) = get_job_location(job) else { return Cost::default() };
                let actor = &route_ctx.route().actor;
                let Some(assigned_anchor) = self.shared.anchors.get(&driver_key(actor)).copied() else {
                    return Cost::default();
                };
                let assigned = self.shared.proximity(loc, assigned_anchor);
                let reference = self.shared.nearest_anchor_prox(loc, job);
                let pull = (assigned - reference).max(0.0);
                pull + self.shared.push_marginal(route_ctx, job)
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }

    fn fitness_scale(&self) -> Cost {
        self.shared.reference
    }
}

impl FeatureState for TerritoryState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _job: &Job) {
        // Cheap: refresh only the affected route's load cache, mirroring `vehicle_distance.rs`.
        // PULL/PUSH are inherently solution-wide (every route's load feeds every other route's
        // deficit/surplus), so the full recompute is deferred to `accept_solution_state`; doing it
        // here would make every job insertion O(N) and construction O(N^2).
        let route_ctx = solution_ctx.routes.get_mut(route_index).expect("route_index out of bounds");
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let load = self.shared.route_load(route_ctx);
        route_ctx.state_mut().set_territory_route_load(load);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| self.accept_route_state(route_ctx));
        self.recompute(solution_ctx);
    }
}

impl TerritoryState {
    fn recompute(&self, solution_ctx: &mut SolutionContext) {
        let pull = self.shared.pull(solution_ctx);
        let push = self.shared.push(solution_ctx);
        solution_ctx.state.set_territory_fitness(TerritoryFitnessData { pull, push });
    }
}
