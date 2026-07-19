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
use crate::models::problem::{DriverIdDimension, JobIdDimension, VehicleIdDimension};
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
type JobValueFn = Arc<dyn Fn(&Job) -> Float + Send + Sync>;

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
    weights: HashMap<DriverKey, Float>,
    job_value_fn: Option<JobValueFn>,
    allow_idle_drivers: bool,
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
            weights: HashMap::new(),
            job_value_fn: None,
            allow_idle_drivers: false,
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

    /// Sets per-driver boundary weights `w_i` used to form power cells: a job's power distance to
    /// a driver is `proximity − w_i`, and a job is assigned overlap-free to the driver minimizing
    /// it. A larger weight enlarges that driver's cell (so a sparse-value driver can reach further
    /// for equal value). Defaults to `0.0` per driver, which makes power distance equal to raw
    /// nearest-anchor proximity. Keyed like anchors (driver id, else vehicle id).
    pub fn set_weights(mut self, w: HashMap<String, Float>) -> Self {
        self.weights = w;
        self
    }

    /// When `true`, drivers that end up with no jobs are left out of the balance entirely (quotas
    /// are re-based over the drivers actually used), so leaving a driver idle is allowed rather than
    /// treated as a deficit. Defaults to `false` (balance spans every driver).
    pub fn set_allow_idle_drivers(mut self, allow: bool) -> Self {
        self.allow_idle_drivers = allow;
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
            self.weights,
            job_value_fn,
            self.allow_idle_drivers,
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
    job_value_fn: JobValueFn,
    profile: Profile,
    anchors: HashMap<DriverKey, Location>,
    /// Per-driver boundary weight `w_i`; missing entries are `0.0` (unweighted cell). Keyed like
    /// `anchors`: one weight per driver = per territory = per anchor.
    weights: HashMap<DriverKey, Float>,
    /// Per-driver quota: the balance metric's ideal share, proportional to each driver's
    /// available time window. Empty when `balance` is `None` (unlimited spare capacity).
    quotas: HashMap<DriverKey, Float>,
    /// Reference magnitude used by `fitness_scale` to normalize PULL + PUSH.
    reference: Cost,
    /// Precomputed per job id: its compatible drivers' anchors sorted by proximity (ascending).
    /// The nearest anchor and nearest-spare anchor are pure functions of the (static) fleet anchors
    /// and job compatibility, so they are computed once here instead of rescanning every actor on
    /// each hot-loop insertion `estimate` — the scan was the fleet-scale construction bottleneck.
    job_anchor_ranking: HashMap<String, Vec<(DriverKey, Float)>>,
    /// Precomputed per job id: the minimum power distance `min_d (prox(loc, anchor_d) − w_d)` over
    /// its compatible drivers — the overlap-penalty reference, static because anchors and weights
    /// are fixed at build time.
    job_nearest_power: HashMap<String, Float>,
    /// Precomputed per driver: the proximity to its nearest *other* driver's anchor (static).
    driver_nearest_other: HashMap<DriverKey, Float>,
    /// Per-driver capacity (summed available shift time). Used to re-base quotas over the used
    /// drivers when `allow_idle_drivers` is set.
    caps: HashMap<DriverKey, Float>,
    /// See [`TerritoryFeatureBuilder::set_allow_idle_drivers`].
    allow_idle_drivers: bool,
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
        weights: HashMap<DriverKey, Float>,
        job_value_fn: JobValueFn,
        allow_idle_drivers: bool,
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
            weights,
            quotas: HashMap::new(),
            reference: 1.0,
            job_anchor_ranking: HashMap::new(),
            job_nearest_power: HashMap::new(),
            driver_nearest_other: HashMap::new(),
            caps: HashMap::new(),
            allow_idle_drivers,
        };
        // Precompute the static anchor lookups first; quotas/reference/power reuse them.
        shared.job_anchor_ranking = shared.compute_job_anchor_ranking(&jobs);
        shared.job_nearest_power = shared.compute_job_nearest_power();
        shared.driver_nearest_other = shared.compute_driver_nearest_other();
        shared.caps = shared.compute_caps();
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
    /// Per-driver capacity: the summed available shift-time window across the driver's actors.
    fn compute_caps(&self) -> HashMap<DriverKey, Float> {
        let mut caps: HashMap<DriverKey, Float> = HashMap::new();
        for actor in self.actors.iter() {
            let window = (actor.detail.time.end - actor.detail.time.start).max(0.0);
            *caps.entry(driver_key(actor)).or_insert(0.0) += window;
        }
        caps
    }

    /// The static per-driver quota: the total demand spread over ALL drivers proportionally to
    /// capacity. Empty when balance is disabled. When `allow_idle_drivers` is set this static map is
    /// re-based per solution over the used drivers only (see [`Self::effective_quotas`]).
    fn compute_quotas(&self, jobs: &Jobs) -> HashMap<DriverKey, Float> {
        if self.balance.is_none() {
            return HashMap::new();
        }
        let total_metric: Float = jobs.all().iter().map(|j| self.job_metric(j)).sum();
        let total_cap: Float = self.caps.values().sum::<Float>().max(1e-6);
        self.caps.iter().map(|(k, &c)| (k.clone(), total_metric * c / total_cap)).collect()
    }

    /// The quota map the balance is actually measured against for a given solution.
    /// - `allow_idle_drivers` off: the static, all-driver quotas.
    /// - `allow_idle_drivers` on: quotas re-based over the *used* drivers (load > 0), so idle drivers
    ///   carry no quota and never count as a deficit, while the used drivers stay balanced among
    ///   themselves.
    fn effective_quotas(&self, loads: &HashMap<DriverKey, Float>) -> HashMap<DriverKey, Float> {
        if !self.allow_idle_drivers {
            return self.quotas.clone();
        }
        let used: Vec<&DriverKey> =
            self.quotas.keys().filter(|k| loads.get(*k).copied().unwrap_or(0.0) > 1e-9).collect();
        let used_cap: Float = used.iter().filter_map(|k| self.caps.get(*k)).sum::<Float>().max(1e-6);
        let used_load: Float = used.iter().filter_map(|k| loads.get(*k)).sum();
        used.into_iter()
            .map(|k| (k.clone(), used_load * self.caps.get(k).copied().unwrap_or(0.0) / used_cap))
            .collect()
    }

    /// The set of drivers PULL may steer jobs toward: with balance off, every anchored driver;
    /// otherwise the effective-quota drivers still under their quota (which excludes idle drivers
    /// entirely when `allow_idle_drivers` is set).
    fn spare_set(&self, loads: &HashMap<DriverKey, Float>) -> std::collections::HashSet<DriverKey> {
        match self.balance {
            None => self.anchors.keys().cloned().collect(),
            Some(_) => self
                .effective_quotas(loads)
                .into_iter()
                .filter(|(k, q)| loads.get(k).copied().unwrap_or(0.0) < *q)
                .map(|(k, _)| k)
                .collect(),
        }
    }

    /// The self-normalization reference: the sum, over all jobs, of the proximity to each job's
    /// nearest compatible anchor. Guarded to stay positive by the caller.
    fn compute_reference(&self, jobs: &Jobs) -> Cost {
        jobs.all().iter().filter_map(|job| get_job_location(job).map(|loc| self.nearest_anchor_prox(loc, job))).sum()
    }

    /// Proximity from a job's location to its nearest compatible anchor, ignoring quotas. O(1) via
    /// the precomputed ranking; falls back to an actor scan for a job not seen at build time (e.g. a
    /// synthetic job without an id).
    fn nearest_anchor_prox(&self, job_loc: Location, job: &Job) -> Float {
        if let Some(ranking) = job.dimens().get_job_id().and_then(|id| self.job_anchor_ranking.get(id)) {
            return ranking.first().map(|(_, p)| *p).unwrap_or(0.0);
        }
        self.scan_sorted_anchors(job_loc, job).first().map(|(_, p)| *p).unwrap_or(0.0)
    }

    /// The boundary weight for a driver; `0.0` when unset (unweighted cell).
    fn weight(&self, key: &DriverKey) -> Float {
        self.weights.get(key).copied().unwrap_or(0.0)
    }

    /// The minimum power distance from a job's location to any compatible driver's anchor:
    /// `min_d (prox(loc, anchor_d) − w_d)`. This is the overlap-penalty reference (a job in its
    /// power cell reaches it exactly). O(1) via the precomputed map; scans as a fallback for a job
    /// absent at build time (e.g. a synthetic job without an id).
    fn nearest_power(&self, job_loc: Location, job: &Job) -> Float {
        if let Some(&p) = job.dimens().get_job_id().and_then(|id| self.job_nearest_power.get(id)) {
            return p;
        }
        self.scan_sorted_anchors(job_loc, job)
            .into_iter()
            .map(|(k, prox)| prox - self.weight(&k))
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0)
    }

    /// Precompute, per job id, its minimum power distance over compatible anchors (static input to
    /// the overlap penalty). Reuses `job_anchor_ranking`, so it must run after it.
    fn compute_job_nearest_power(&self) -> HashMap<String, Float> {
        self.job_anchor_ranking
            .iter()
            .map(|(id, ranking)| {
                let np = ranking
                    .iter()
                    .map(|(k, prox)| prox - self.weight(k))
                    .min_by(|a, b| a.total_cmp(b))
                    .unwrap_or(0.0);
                (id.clone(), np)
            })
            .collect()
    }

    /// Actor scan producing a job's compatible drivers' anchors sorted by proximity (ascending).
    /// Used to precompute `job_anchor_ranking` and as the uncached fallback for the lookups above.
    fn scan_sorted_anchors(&self, job_loc: Location, job: &Job) -> Vec<(DriverKey, Float)> {
        let mut seen: HashMap<DriverKey, Float> = HashMap::new();
        for actor in self.actors.iter() {
            if !(self.compatibility_fn)(job, actor) {
                continue;
            }
            let key = driver_key(actor);
            if let Some(&anchor) = self.anchors.get(&key) {
                seen.entry(key).or_insert_with(|| self.proximity(job_loc, anchor));
            }
        }
        let mut ranking: Vec<(DriverKey, Float)> = seen.into_iter().collect();
        ranking.sort_by(|a, b| a.1.total_cmp(&b.1));
        ranking
    }

    /// Precompute, per job id, its sorted compatible-anchor list — the static hot-loop input.
    fn compute_job_anchor_ranking(&self, jobs: &Jobs) -> HashMap<String, Vec<(DriverKey, Float)>> {
        jobs.all()
            .iter()
            .filter_map(|job| {
                let id = job.dimens().get_job_id()?.clone();
                let loc = get_job_location(job)?;
                Some((id, self.scan_sorted_anchors(loc, job)))
            })
            .collect()
    }

    /// Precompute, per driver, the proximity to its nearest *other* driver's anchor.
    fn compute_driver_nearest_other(&self) -> HashMap<DriverKey, Float> {
        self.anchors
            .iter()
            .map(|(key, &anchor)| {
                let nearest = self
                    .anchors
                    .iter()
                    .filter(|(other_key, _)| *other_key != key)
                    .map(|(_, &other)| self.proximity(anchor, other))
                    .min_by(|x, y| x.total_cmp(y))
                    .unwrap_or(0.0);
                (key.clone(), nearest)
            })
            .collect()
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
    fn nearest_spare_anchor(
        &self,
        job_loc: Location,
        job: &Job,
        spare: &std::collections::HashSet<DriverKey>,
    ) -> Option<Float> {
        // Ranking is proximity-ascending, so the first spare driver in it is the nearest spare one.
        if let Some(ranking) = job.dimens().get_job_id().and_then(|id| self.job_anchor_ranking.get(id)) {
            return ranking.iter().find(|(k, _)| spare.contains(k)).map(|(_, p)| *p);
        }
        self.scan_sorted_anchors(job_loc, job).into_iter().find(|(k, _)| spare.contains(k)).map(|(_, p)| p)
    }

    /// Total PULL for the solution: for each assigned job, the excess proximity of its assigned
    /// driver's anchor over the nearest compatible, under-quota anchor.
    fn pull(&self, solution: &SolutionContext) -> Cost {
        let loads = self.loads(solution);
        let spare = self.spare_set(&loads);
        let mut total = 0.0;
        for route_ctx in solution.routes.iter() {
            let actor = &route_ctx.route().actor;
            let Some(assigned_anchor) = self.anchors.get(&driver_key(actor)).copied() else { continue };
            for job in route_ctx.route().tour.jobs() {
                let Some(loc) = get_job_location(job) else { continue };
                let assigned = self.proximity(loc, assigned_anchor);
                let reference = self.nearest_spare_anchor(loc, job, &spare).unwrap_or(assigned);
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
        // With `allow_idle_drivers`, this spans only the used drivers, so idle drivers are neither
        // deficits (targets) nor surplus (sources) — leaving one idle is not an imbalance.
        let quotas = self.effective_quotas(&loads);

        // Deficit drivers (with an anchor) and their remaining room.
        let deficits: Vec<Location> = quotas
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
        for (key, &quota) in quotas.iter() {
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
        // When idle drivers are allowed, the estimate should not spread work off drivers (that would
        // fill idle ones): concentration to the feasible minimum is fine and the fitness still
        // balances the used drivers. So there is no per-insertion push signal in that mode.
        if self.allow_idle_drivers {
            return 0.0;
        }
        let key = driver_key(&route_ctx.route().actor);
        if !self.anchors.contains_key(&key) {
            return 0.0;
        }
        let load = route_ctx.state().get_territory_route_load().copied().unwrap_or(0.0);
        if load < *self.quotas.get(&key).unwrap_or(&Float::MAX) {
            return 0.0;
        }

        // Nearest other anchor is static per driver — precomputed.
        let nearest_other = self.driver_nearest_other.get(&key).copied().unwrap_or(0.0);
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
                let key = driver_key(actor);
                let Some(assigned_anchor) = self.shared.anchors.get(&key).copied() else {
                    return Cost::default();
                };
                let assigned_power = self.shared.proximity(loc, assigned_anchor) - self.shared.weight(&key);
                let reference = self.shared.nearest_power(loc, job);
                let pull = (assigned_power - reference).max(0.0);
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
