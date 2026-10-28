//! A feature that builds balanced, capacity-aware territories around a per-driver anchor list.
//!
//! PULL is a per-driver *weighted* overlap penalty: each job is billed the excess of its driver's
//! power distance (`proximity − weight`) over the minimum power distance across the job's
//! compatible anchors, so a job reaching into a foreign power cell is penalized and a job in its
//! own cell is free. PUSH greedily moves over-quota surplus to the nearest under-quota driver at
//! proximity cost. Anchors and weights are supplied by the caller (objective config) and are never
//! derived anywhere — an empty anchor map is a territory nobody holds, not a request to invent one.
//! The per-driver quota may be supplied too (see [`TerritoryFeatureBuilder::set_quotas`]), and is
//! otherwise derived from total demand.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/territory_test.rs"]
mod territory_test;

use super::vehicle_distance::get_job_location;
use super::*;
use crate::construction::enablers::{PaidWorkingDurationTourState, TotalDistanceTourState};
use crate::models::problem::{JobIdDimension, driver_key};
use std::collections::HashMap;

pub use crate::construction::features::vehicle_distance::ActorJobCompatibilityFn;

custom_solution_state!(TerritoryFitness typeof TerritoryFitnessData);
custom_solution_state!(TerritoryAvgLoad typeof Float);
custom_tour_state!(TerritoryRouteLoad typeof Float);
custom_tour_state!(TerritoryRouteQuota typeof Float);
custom_tour_state!(TerritoryRouteAnchor typeof Location);

/// Restores the quadratic term's weight in the regime the feature's own tests exercise.
/// `surplus²/quota` alone is below `surplus` for every surplus under quota — at a surplus of a
/// quarter to a third of quota, which is what the territory fixtures produce, it drops the balance
/// term to 27-33% of its linear weight and transport cost outbids it.
///
/// Bracketed from both sides by the vrp-pragmatic fixtures, and there is no room to round it off.
/// Both edges are the tipping point of one discrete "move this job or leave it" decision on a
/// six-job problem, so both are analytic and both were located to four decimals:
///
/// - **Low, 35/12 ≈ 2.9167**, from `territory_balances_for_each_metric`'s ProductionValue arm.
///   Values 2,3 | 3,3,2,2 over two equal drivers ⇒ `avg_metric` 2.5, quota 7.5 each, anchors 60
///   apart, and the cost-only split is 5/10. Leaving it there is a surplus of 2.5 ⇒
///   `(2.5/2.5) × GAIN × (2.5/7.5) × 60 = 20 × GAIN`, with no PULL. Moving driver1's boundary job
///   (value 3, sitting 2 from its own anchor and 58 from driver0's) leaves a surplus of 0.5 ⇒
///   `(0.5/2.5) × GAIN × (0.5/7.5) × 60 = 0.8 × GAIN`, plus `58 − 2 = 56` of PULL. Balancing wins
///   only while `20 × GAIN > 0.8 × GAIN + 56`, i.e. `GAIN > 56/19.2`. Below it the arm settles back
///   on 5/10.
/// - **High, 10/3 ≈ 3.3333**, from `supplied_weights_pull_work_toward_the_heavier_driver` (with
///   `supplied_weights_ignore_unknown_keys_and_default_absent_drivers_to_zero` failing alongside
///   it). That fixture's second job crossing costs `60 × GAIN` against a power weight worth 200, so
///   above 10/3 PUSH outbids the supplied weights and the split settles on 2/4 instead of 1/5.
///
/// Measured: 2.9166 red / 2.9167 green, 3.33 green / 3.34 red, each side unanimous over repeated
/// runs. 3.0 sits inside. The vrp-core unit tests then pin it exactly rather than bracket it —
/// `derived_quotas_produce_exact_pull_and_push` and friends assert PUSH totals worked out at this
/// value — so moving the gain means recomputing those numbers, not just re-probing the window.
///
/// Normalizing the PUSH fitness by `avg_metric` (see [`TerritoryShared::push`]) left the *upper*
/// edge and every Activities-driven edge exactly where they were — `avg_metric` is 1 when the metric
/// is activity count, so the division is the identity there, and
/// `territory_forms_balanced_clusters_from_shared_start` still fails at 2.8 and passes at 2.85 as
/// before. What moved is which fixture binds from below. The same equation without the
/// `/avg_metric` reads `50 × GAIN > 2 × GAIN + 56`, i.e. `GAIN > 7/6` — well under the Activities
/// edge, so the ProductionValue arm never bound. Normalizing lifted it by exactly `avg_metric` (2.5)
/// to 35/12, which lands just past that edge. So the window is near enough the one the raw-sum
/// formula had, now held from below by a different constraint — which is the point: with both terms
/// in jobs × distance, this gain no longer means something different per balance metric.
///
/// Those pragmatic fixtures drive a stochastic solver (`Environment::default()` seeds
/// `DefaultRandom` from entropy, not a fixed seed), so treat a single red run near an edge as
/// evidence, not proof — though on these two edges the seed never changed the verdict.
const PUSH_CONVEXITY_GAIN: Float = 3.0;

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
    /// Balances on time spent AT customers — service only, no travel.
    ///
    /// The cheap sibling of [`Self::Duration`]. Duration balances service plus drive, which is what
    /// the technician is on the clock for, but it is self-referential: moving a job to a distant
    /// technician adds drive time to the very load being levelled, so the objective partly undoes
    /// itself and buys its fairness in miles. Service time is invariant to who performs it, so
    /// levelling it has no such feedback and leaves the transport objective free to keep the plan
    /// tight.
    Service,
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

/// A per-driver grouping key — see [`driver_key`], which every feature that means "the same
/// person" shares so they cannot drift apart on what a person is.
type DriverKey = String;
type JobValueFn = Arc<dyn Fn(&Job) -> Float + Send + Sync>;

/// The keys of a per-driver map, ascending — see [`TerritoryShared::driver_order`] for why that
/// order is pinned. Takes the map it orders as an argument rather than reading a field, so the
/// "must be built from `caps`, after `caps`" dependency is in the signature instead of a comment.
fn sorted_driver_keys(caps: &HashMap<DriverKey, Float>) -> Vec<DriverKey> {
    let mut keys: Vec<DriverKey> = caps.keys().cloned().collect();
    keys.sort_unstable();
    keys
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
    balance_tolerance: Float,
    anchors: HashMap<DriverKey, Vec<Location>>,
    weights: HashMap<DriverKey, Float>,
    deficit_weight: Float,
    quotas: HashMap<DriverKey, Float>,
    quota_shares: HashMap<DriverKey, Float>,
    quota_pools: HashMap<DriverKey, String>,
    job_value_fn: Option<JobValueFn>,
    allow_idle_drivers: bool,
    day_patch: bool,
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
            balance_tolerance: 0.0,
            anchors: HashMap::new(),
            weights: HashMap::new(),
            deficit_weight: 0.0,
            quotas: HashMap::new(),
            quota_shares: HashMap::new(),
            quota_pools: HashMap::new(),
            job_value_fn: None,
            allow_idle_drivers: false,
            day_patch: false,
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

    /// Sets the balance deadband: a driver is only billed PUSH once its load exceeds
    /// `quota * (1 + tolerance)`, and only counts as a deficit below `quota * (1 - tolerance)`.
    /// A neutral band around the quota means the solver stops shaving the last few percent of
    /// imbalance by exiling jobs into foreign cells — the imbalance those exiles bought was tiny,
    /// the overlap they created was not. `0.0` (the default) restores the exact, zero-slack balance.
    pub fn set_balance_tolerance(mut self, tolerance: Float) -> Self {
        self.balance_tolerance = tolerance.max(0.0);
        self
    }

    /// Sets the per-driver anchor locations, keyed by driver id (or vehicle id when no driver id
    /// dimension is set). Anchors are supplied by the caller, never derived from a dimension.
    ///
    /// A driver holds a *list* of anchors, one per patch of ground it works, because a technician
    /// may hold several separate service areas — each contested with a different colleague — and a
    /// single anchor would be a compromise that loses the tiebreak in whichever patch it sits
    /// further from. Every proximity to a driver is therefore the minimum over that driver's list:
    /// `π(job, driver) = min_k π(job, anchor_k)`, and the PUSH ground cost between two drivers is
    /// the minimum over their anchor pairs.
    ///
    /// A driver with **no** anchor — absent key or empty list — takes no part in the territory at
    /// all: it accrues no PULL, is never a PUSH source or target, and is never a job's overlap
    /// reference. That is deliberate, not a gap: a technician who is the sole holder of their
    /// ground has no tiebreak to make, so no anchor is sent and none is synthesised here.
    pub fn set_anchors(mut self, a: HashMap<String, Vec<Location>>) -> Self {
        self.anchors = a;
        self
    }

    /// Sets the function that reads a job's production value, used when balancing on
    /// [`TerritoryBalance::ProductionValue`]. Defaults to a constant `1.0` per job.
    pub fn set_job_value_fn<F: Fn(&Job) -> Float + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.job_value_fn = Some(Arc::new(f));
        self
    }

    /// Sets how much a DEFICIT costs, as a fraction of the same gap billed as a surplus. `0.0` (the
    /// default) is PUSH as it has always been: a driver below its band is a destination for other
    /// drivers' surplus and is never billed itself.
    ///
    /// ⚠️ Work is conserved, so a deficit and a surplus are two readings of ONE misallocation and
    /// pricing both counts it twice. What the second reading buys is where the pressure sits: the
    /// band is per driver, so a deficit concentrated on one technician appears as a small surplus
    /// spread across the rest and can sit inside every band at once. Measured on a six-technician tenant, run 40:
    /// one technician 17% short with nobody else more than 10.6% over, and narrowing the band to 1%
    /// did not move it — the deficit had to become expensive, not merely visible.
    pub fn set_deficit_weight(mut self, w: Float) -> Self {
        self.deficit_weight = w.max(0.0);

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

    /// Sets per-driver quotas supplied by the caller, keyed like anchors (driver id, else vehicle
    /// id). A non-empty map is used verbatim in place of the derived quota, following the same
    /// principle as the anchor: only the caller knows which work a driver can actually reach.
    /// Deriving the quota from the problem's *total* demand inflates it for a driver a hard
    /// constraint keeps off part of that work — the deficit then never closes, PUSH floors at a
    /// non-zero constant, and the balance axis stops producing a gradient. Empty (the default) ⇒
    /// derive from total demand as before.
    ///
    /// Three edge cases, all defined rather than left to chance:
    /// - A key matching no driver in the fleet is **ignored**. It can never be keyed to a route, so
    ///   it can only be noise. A map whose keys *all* miss therefore leaves no driver with a quota,
    ///   which switches the balance term off rather than silently reverting to the derived quotas
    ///   the caller explicitly replaced.
    /// - A fleet driver **absent** from a non-empty map gets **no quota**, which leaves it out of
    ///   the balance entirely: never a surplus, never a deficit, no per-insertion shedding
    ///   pressure. Omission stays a no-op instead of being read as "quota 0", which would order
    ///   that driver to shed every job it holds.
    /// - Ignored when balance is `None`: quotas are meaningless without a balance metric, so that
    ///   mode keeps its "no quotas at all" invariant.
    pub fn set_quotas(mut self, q: HashMap<String, Float>) -> Self {
        self.quotas = q;
        self
    }

    /// Sets per-driver quota SHARES, keyed like `anchors` and `quota`, to be read together with
    /// the pool each driver competes in ([`Self::set_quota_pools`]). A share is the fraction of its
    /// pool's load that driver is expected to carry, and the shares inside one pool sum to 1.0.
    ///
    /// This is how a ROUTE-level balance metric gets a quota at all. `Distance` and `Duration` are
    /// properties of a route, so the problem holds no total to split before the solve: only the
    /// caller knows the ratio — capacity, and which work each driver may actually reach — and only
    /// a solution knows the level. A non-empty map therefore replaces both a supplied `quota` and
    /// the derived one.
    ///
    /// Same three edge cases as [`Self::set_quotas`]: a key matching no driver is ignored, a fleet
    /// driver absent from a non-empty map is left out of the balance entirely, and the whole thing
    /// is ignored when balance is `None`.
    pub fn set_quota_shares(mut self, shares: HashMap<String, Float>) -> Self {
        self.quota_shares = shares;
        self
    }

    /// Sets the pool each driver's share is measured inside: the drivers it competes with for the
    /// same work. A driver the caller did not place falls into one shared default pool, which is
    /// the right answer when no hard gate splits the fleet.
    ///
    /// Pools are what keep the share gate-aware. A driver a service area keeps off half the ground
    /// must be quoted against the half it can reach, not against the whole chunk — the failure
    /// otherwise is a deficit that never closes.
    pub fn set_quota_pools(mut self, pools: HashMap<String, String>) -> Self {
        self.quota_pools = pools;
        self
    }

    /// When `true`, drivers that end up with no jobs are left out of the balance entirely (quotas
    /// are re-based over the drivers actually used), so leaving a driver idle is allowed rather than
    /// treated as a deficit. Defaults to `false` (balance spans every driver).
    pub fn set_allow_idle_drivers(mut self, allow: bool) -> Self {
        self.allow_idle_drivers = allow;
        self
    }

    /// When true, a route is measured against ONE of its driver's anchors, the one nearest its
    /// jobs as a whole, instead of each job against the nearest of them. A day then stays in one
    /// patch of its driver's ground rather than mixing patches that lie far apart.
    pub fn set_day_patch(mut self, day_patch: bool) -> Self {
        self.day_patch = day_patch;
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

        let mut shared = TerritoryShared::new(
            transport,
            actors,
            jobs,
            compatibility_fn,
            self.proximity,
            self.balance,
            self.balance_tolerance,
            self.deficit_weight,
            self.anchors,
            self.weights,
            self.quotas,
            self.quota_shares,
            self.quota_pools,
            job_value_fn,
            self.allow_idle_drivers,
        );
        shared.day_patch = self.day_patch;
        let shared = Arc::new(shared);

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
    /// Balance deadband; see [`TerritoryFeatureBuilder::set_balance_tolerance`]. A driver is over
    /// quota only above `quota * (1 + balance_tolerance)` and a deficit only below
    /// `quota * (1 - balance_tolerance)`.
    balance_tolerance: Float,
    job_value_fn: JobValueFn,
    profile: Profile,
    /// Per-driver anchor list; see [`TerritoryFeatureBuilder::set_anchors`]. A missing key or an
    /// empty list means "no anchor", which keeps that driver out of the territory entirely.
    anchors: HashMap<DriverKey, Vec<Location>>,
    /// See [`TerritoryFeatureBuilder::set_deficit_weight`]. `0.0` ⇒ a deficit costs nobody anything
    /// and only steers where somebody else's surplus should go.
    deficit_weight: Float,
    /// Per-driver boundary weight `w_i`; missing entries are `0.0` (unweighted cell). Keyed like
    /// `anchors`: one weight per driver = per territory (however many patches that territory has).
    weights: HashMap<DriverKey, Float>,
    /// Per-driver quota: the caller-supplied map when one was given (see
    /// [`TerritoryFeatureBuilder::set_quotas`]), otherwise the balance metric's ideal share,
    /// proportional to each driver's available time window. Empty when `balance` is `None`
    /// (unlimited spare capacity).
    quotas: HashMap<DriverKey, Float>,
    /// Per-driver quota shares and the pool each is measured in — the route-level alternative to
    /// [`Self::quotas`]. See [`TerritoryFeatureBuilder::set_quota_shares`].
    quota_shares: HashMap<DriverKey, Float>,
    quota_pools: HashMap<DriverKey, String>,
    /// True when [`Self::quota_shares`] came from the caller. Shares outrank quotas: a caller that
    /// sent both means the ratio, and a level it could not know.
    shares_supplied: bool,
    /// True when [`Self::quotas`] came from the caller instead of [`Self::compute_quotas`].
    /// Supplied quotas are already the intended targets, so they are never re-based (see
    /// [`Self::effective_quotas`]).
    quotas_supplied: bool,
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
    /// Precomputed per job id: the *second* smallest power distance over its compatible anchors
    /// (`+∞` when a job has only one compatible anchor). With `job_nearest_power` this gives the
    /// per-job power gap — how much deeper a job sits in its own cell than in the next-best one —
    /// which the location-aware PUSH marginal uses to prefer shedding boundary jobs over deep ones.
    job_second_power: HashMap<String, Float>,
    /// The average balance metric per job (`total_metric / job_count`, floored positive). Divides
    /// the raw job metric in the PUSH marginal AND the surplus in the PUSH fitness, so both live on
    /// the same (distance) scale as PULL instead of `value × distance`, which otherwise dwarfs PULL
    /// by the value magnitude and makes the balance pressure ignore where a job sits. Estimate and
    /// fitness must share this normalization or the estimate steers on a different exchange rate
    /// than the fitness it approximates.
    avg_metric: Float,
    /// Precomputed per job id: the shortest round trip from any COMPATIBLE vehicle start to the
    /// job and back, in the balance metric's unit.
    ///
    /// Not what the balance measures — that is [`Self::route_load`] — but a per-job quantity in the
    /// same unit, which two places need: [`Self::compute_avg_metric`], to express PUSH's surplus in
    /// jobs' worth so [`PUSH_CONVEXITY_GAIN`] means the same thing whatever is balanced, and
    /// [`Self::push_marginal`]'s value factor. Static, so `avg_metric` stays a build-time constant.
    job_travel_estimate: HashMap<String, Float>,
    /// The PUSH marginal's reach: the median per-job power gap. A job whose gap exceeds this sits
    /// too deep in its cell to be worth shedding for balance, so its PUSH marginal is zero (it stays
    /// home); jobs within reach of a boundary are the ones balance may push to a neighbour.
    push_reach: Float,
    /// Per-driver capacity (summed available shift time). Used to re-base quotas over the used
    /// drivers when `allow_idle_drivers` is set.
    caps: HashMap<DriverKey, Float>,
    /// Every driver key in the fleet, ascending — the canonical order for per-driver reductions.
    /// A `HashMap` iterates in hash order and the standard library seeds every map from a
    /// per-thread counter, so the *n*-th map a process builds gets its own order; a float sum or a
    /// pick taken straight off `caps`/`quotas` therefore depends on how many maps were built
    /// earlier, which supplying anchors instead of deriving them changes. Folding in this order
    /// pins the arithmetic to the input.
    driver_order: Vec<DriverKey>,
    /// See [`TerritoryFeatureBuilder::set_allow_idle_drivers`].
    allow_idle_drivers: bool,
    /// See [`TerritoryFeatureBuilder::set_day_patch`].
    day_patch: bool,
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
        balance_tolerance: Float,
        deficit_weight: Float,
        anchors: HashMap<DriverKey, Vec<Location>>,
        weights: HashMap<DriverKey, Float>,
        supplied_quotas: HashMap<DriverKey, Float>,
        supplied_shares: HashMap<DriverKey, Float>,
        quota_pools: HashMap<DriverKey, String>,
        job_value_fn: JobValueFn,
        allow_idle_drivers: bool,
    ) -> Self {
        let profile = actors.first().map(|a| a.vehicle.profile.clone()).unwrap_or_default();
        // Quotas are meaningless without a balance metric, so that mode keeps deriving nothing.
        let shares_supplied = balance.is_some() && !supplied_shares.is_empty();
        let quotas_supplied = balance.is_some() && !supplied_quotas.is_empty();
        let mut shared = Self {
            transport,
            actors,
            compatibility_fn,
            proximity,
            balance,
            balance_tolerance,
            deficit_weight,
            job_value_fn,
            profile,
            anchors,
            weights,
            quotas: HashMap::new(),
            quota_shares: HashMap::new(),
            quota_pools,
            shares_supplied,
            quotas_supplied,
            reference: 1.0,
            job_anchor_ranking: HashMap::new(),
            job_travel_estimate: HashMap::new(),
            job_nearest_power: HashMap::new(),
            job_second_power: HashMap::new(),
            avg_metric: 1.0,
            push_reach: 0.0,
            caps: HashMap::new(),
            driver_order: Vec::new(),
            allow_idle_drivers,
            day_patch: false,
        };
        // Precompute the static anchor lookups first; quotas/reference/power reuse them.
        shared.job_anchor_ranking = shared.compute_job_anchor_ranking(&jobs);
        shared.job_travel_estimate = shared.compute_job_travel_estimate(&jobs);
        shared.job_nearest_power = shared.compute_job_nearest_power();
        shared.job_second_power = shared.compute_job_second_power();
        shared.avg_metric = shared.compute_avg_metric(&jobs);
        shared.push_reach = shared.compute_push_reach();
        shared.caps = shared.compute_caps();
        shared.driver_order = sorted_driver_keys(&shared.caps);
        // `filter_supplied_quotas` reads `caps` to tell a real driver from an unknown key, so both
        // must run after it.
        shared.quota_shares = shared.filter_supplied_quotas(supplied_shares);
        shared.quotas = if shares_supplied {
            // Shares replace the amount entirely: the level is read off each solution, so there is
            // no static quota to hold. `effective_quotas` is where it appears.
            HashMap::new()
        } else if quotas_supplied {
            shared.filter_supplied_quotas(supplied_quotas)
        } else {
            shared.compute_quotas(&jobs)
        };
        shared.reference = shared.compute_reference(&jobs).max(1.0);
        shared
    }

    /// The threshold above which a driver's load counts as over quota (billed by PUSH): the quota
    /// widened by the balance deadband.
    fn over_quota(&self, quota: Float) -> Float {
        quota * (1.0 + self.balance_tolerance)
    }

    /// The threshold below which a driver's load counts as a deficit (a PUSH target): the quota
    /// narrowed by the balance deadband.
    fn under_quota(&self, quota: Float) -> Float {
        quota * (1.0 - self.balance_tolerance)
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
            Some(TerritoryBalance::Service) => self.service_share(job),
            Some(TerritoryBalance::ProductionValue) => (self.job_value_fn)(job),
            // The per-job ESTIMATE, not the measurement: see [`Self::job_travel_estimate`].
            Some(TerritoryBalance::Distance) | Some(TerritoryBalance::Duration) => {
                job.dimens().get_job_id().and_then(|id| self.job_travel_estimate.get(id)).copied().unwrap_or(0.0)
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
        // Summed in `driver_order`, not hash order: the rounding of a float sum depends on the
        // order it is folded in, and every quota is scaled by this total.
        let total_cap: Float = self.driver_order.iter().filter_map(|key| self.caps.get(key)).sum::<Float>().max(1e-6);
        self.caps.iter().map(|(k, &c)| (k.clone(), total_metric * c / total_cap)).collect()
    }

    /// The caller-supplied quota map, narrowed to drivers that exist in the fleet. A key matching
    /// no driver is dropped (it could never be keyed to a route) and a driver the caller omitted
    /// stays absent, which leaves it out of the balance — see
    /// [`TerritoryFeatureBuilder::set_quotas`] for why each of those is the chosen behaviour.
    fn filter_supplied_quotas(&self, supplied: HashMap<DriverKey, Float>) -> HashMap<DriverKey, Float> {
        supplied.into_iter().filter(|(key, _)| self.caps.contains_key(key)).collect()
    }

    /// The quota map the balance is actually measured against for a given solution.
    /// - quotas supplied by the caller: used as they are, in every mode. They are already the
    ///   intended targets, so re-basing them would overwrite the caller's numbers with a capacity
    ///   share it deliberately did not ask for.
    /// - derived, `allow_idle_drivers` off: the static, all-driver quotas.
    /// - derived, `allow_idle_drivers` on: quotas re-based over the *used* drivers (load > 0), so
    ///   idle drivers carry no quota and never count as a deficit, while the used drivers stay
    ///   balanced among themselves.
    fn effective_quotas(&self, loads: &HashMap<DriverKey, Float>) -> HashMap<DriverKey, Float> {
        if self.shares_supplied {
            return self.quotas_from_shares(loads);
        }

        if !self.allow_idle_drivers || self.quotas_supplied {
            return self.quotas.clone();
        }
        // Walked in `driver_order`, so the two sums below fold in a fixed order (see
        // [`Self::driver_order`]).
        let used: Vec<&DriverKey> = self
            .driver_order
            .iter()
            .filter(|k| self.quotas.contains_key(*k))
            .filter(|k| loads.get(*k).copied().unwrap_or(0.0) > 1e-9)
            .collect();
        let used_cap: Float = used.iter().filter_map(|k| self.caps.get(*k)).sum::<Float>().max(1e-6);
        let used_load: Float = used.iter().filter_map(|k| loads.get(*k)).sum();
        used.into_iter().map(|k| (k.clone(), used_load * self.caps.get(k).copied().unwrap_or(0.0) / used_cap)).collect()
    }

    /// Quotas built from the caller's shares: the ratio is theirs, the level is this solution's.
    ///
    /// Per pool, the quota is `share_e x (the pool's own total load)`. That keeps the two quota
    /// invariants a supplied map has to satisfy — every participating driver holds a key, and the
    /// quotas of a pool sum to exactly that pool's total, so a deficit always exists somewhere
    /// while anybody is over the band.
    ///
    /// The target therefore moves with the solution, which is the point rather than a compromise:
    /// balance is a dispersion measure and should be level-invariant. The level belongs to the
    /// transport objective, and pinning it to a pre-solve estimate would either silence PUSH (an
    /// estimate above the truth) or bill everybody at once (one below it, which is what any
    /// lower-bound estimate of travel is).
    ///
    /// Walked in `driver_order` so both sums fold in a fixed order — a float sum rounds by the
    /// order it is folded in, and every quota is scaled by these totals.
    fn quotas_from_shares(&self, loads: &HashMap<DriverKey, Float>) -> HashMap<DriverKey, Float> {
        let mut pool_load: HashMap<&str, Float> = HashMap::new();

        for key in self.driver_order.iter() {
            if !self.quota_shares.contains_key(key) {
                continue;
            }
            *pool_load.entry(self.pool_of(key)).or_insert(0.0) += loads.get(key).copied().unwrap_or(0.0);
        }

        self.driver_order
            .iter()
            .filter_map(|key| {
                let share = self.quota_shares.get(key)?;
                Some((key.clone(), share * pool_load.get(self.pool_of(key)).copied().unwrap_or(0.0)))
            })
            .collect()
    }

    /// The pool a driver competes in. Drivers the caller did not place share one default pool,
    /// which is the whole fleet when no hard gate splits it.
    fn pool_of(&self, key: &DriverKey) -> &str {
        self.quota_pools.get(key).map(String::as_str).unwrap_or("")
    }

    /// Every driver taking part in the balance, whichever way its quota is expressed.
    fn quota_keys(&self) -> impl Iterator<Item = &DriverKey> {
        let (shares, quotas) = if self.shares_supplied {
            (Some(self.quota_shares.keys()), None)
        } else {
            (None, Some(self.quotas.keys()))
        };

        shares.into_iter().flatten().chain(quotas.into_iter().flatten())
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

    /// A driver's anchor list, or `None` when it holds no anchor at all — an absent key and an empty
    /// list are the same thing, and both mean the driver takes no part in the territory (see
    /// [`TerritoryFeatureBuilder::set_anchors`]). Every anchor read goes through here so that
    /// "no anchor ⇒ no participation" cannot be lost at one call site.
    fn driver_anchors(&self, key: &DriverKey) -> Option<&[Location]> {
        self.anchors.get(key).map(Vec::as_slice).filter(|anchors| !anchors.is_empty())
    }

    /// The anchor a route is measured against under `day_patch`: of its driver's anchors, the one
    /// with the smallest summed proximity to the route's jobs. `None` for an empty route or a
    /// driver without anchors.
    fn route_anchor(&self, route_ctx: &RouteContext) -> Option<Location> {
        let anchors = self.driver_anchors(&driver_key(&route_ctx.route().actor))?;
        let locations: Vec<Location> = route_ctx.route().tour.jobs().filter_map(get_job_location).collect();
        if locations.is_empty() {
            return None;
        }
        anchors
            .iter()
            .map(|&anchor| (anchor, locations.iter().map(|&loc| self.proximity(loc, anchor)).sum::<Float>()))
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .map(|(anchor, _)| anchor)
    }

    /// Proximity from a location to a driver's *nearest* anchor: `min_k π(loc, anchor_k)`. `None`
    /// when the driver holds no anchor.
    fn driver_prox(&self, key: &DriverKey, loc: Location) -> Option<Float> {
        self.driver_anchors(key).map(|anchors| self.min_prox_to(anchors, loc))
    }

    /// The smallest proximity from `loc` to any of `anchors`. `0.0` for an empty slice, which
    /// `driver_anchors` already rules out for every caller that matters.
    fn min_prox_to(&self, anchors: &[Location], loc: Location) -> Float {
        anchors.iter().map(|&a| self.proximity(loc, a)).min_by(|x, y| x.total_cmp(y)).unwrap_or(0.0)
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
                let np =
                    ranking.iter().map(|(k, prox)| prox - self.weight(k)).min_by(|a, b| a.total_cmp(b)).unwrap_or(0.0);
                (id.clone(), np)
            })
            .collect()
    }

    /// Precompute, per job id, the second-smallest power distance over its compatible anchors
    /// (`+∞` when fewer than two). Reuses `job_anchor_ranking`, so it must run after it. Note the
    /// ranking is sorted by raw proximity; with per-driver weights the power order can differ, so
    /// the powers are re-sorted here.
    fn compute_job_second_power(&self) -> HashMap<String, Float> {
        self.job_anchor_ranking
            .iter()
            .map(|(id, ranking)| {
                let mut powers: Vec<Float> = ranking.iter().map(|(k, prox)| prox - self.weight(k)).collect();
                powers.sort_by(|a, b| a.total_cmp(b));
                (id.clone(), powers.get(1).copied().unwrap_or(Float::INFINITY))
            })
            .collect()
    }

    /// See [`Self::job_travel_estimate`]. Compatibility-aware, so a job counts only the vehicles
    /// that may actually serve it; a job no vehicle may serve estimates at 0.0 and never enters a
    /// route anyway.
    fn compute_job_travel_estimate(&self, jobs: &Jobs) -> HashMap<String, Float> {
        jobs.all()
            .iter()
            .filter_map(|job| {
                let id = job.dimens().get_job_id()?.clone();
                let loc = get_job_location(job)?;
                let travel = self
                    .actors
                    .iter()
                    .filter(|actor| (self.compatibility_fn)(job, actor))
                    .filter_map(|actor| actor.detail.start.as_ref().map(|place| place.location))
                    .map(|start| self.travel(start, loc) + self.travel(loc, start))
                    .min_by(|a, b| a.total_cmp(b))
                    .unwrap_or(0.0);
                Some((id, travel + self.service_share(job)))
            })
            .collect()
    }

    /// What inserting ONE job into a route actually adds to that route's load.
    ///
    /// ⚠️ Deliberately not [`Self::job_travel_estimate`], which answers a different question: what a
    /// job is worth to the problem's TOTAL, and so what a derived quota should be sized from.
    ///
    /// Inserting a job adds its service exactly, and its drive only as a DETOUR — a few minutes
    /// between two neighbours, not the round trip from a depot. Estimating `Duration` at the round
    /// trip made the marginal large, roughly equal for every job, and therefore nearly free of the
    /// discrimination a marginal exists to provide, while mis-scaling it against PULL. Service
    /// alone is the part exactly knowable here, and on a field-service day it is the part that
    /// dominates.
    ///
    /// `Distance` keeps the travel: there the drive IS the load, and an estimate without it would
    /// be zero.
    fn insertion_metric(&self, job: &Job) -> Float {
        match self.balance {
            None => 0.0,
            Some(TerritoryBalance::Duration) | Some(TerritoryBalance::Service) => self.service_share(job),
            _ => self.job_metric(job),
        }
    }

    /// What the job itself adds to the balance quantity, beside the travel to reach it.
    ///
    /// ⚠️ For `Duration` that is its SERVICE time, and leaving it out is not a rounding error:
    /// the route load is the paid span, which is service plus idle plus the drive between jobs,
    /// and on a field-service day service dominates. An estimate made of travel alone would
    /// price a two-hour visit next door as cheaper than a ten-minute visit across town, and
    /// `.ai/rules/solver.md` is explicit that an estimate disagreeing with its fitness in
    /// QUANTITY is mis-scaled against everything sharing its layer.
    ///
    /// For `Distance` it is zero: standing still drives no miles.
    fn service_share(&self, job: &Job) -> Float {
        if !matches!(self.balance, Some(TerritoryBalance::Duration) | Some(TerritoryBalance::Service)) {
            return 0.0;
        }

        match job {
            Job::Single(single) => single.places.first().map(|place| place.duration).unwrap_or(0.0),
            Job::Multi(multi) => multi.jobs.iter().filter_map(|s| s.places.first().map(|place| place.duration)).sum(),
        }
    }

    /// Travel between two locations in the BALANCE metric's unit.
    ///
    /// ⚠️ Deliberately distinct from [`Self::proximity`], which answers in the TERRITORY metric's
    /// unit. The two are configured separately and a solve may well balance on duration while its
    /// territories are drawn by distance.
    fn travel(&self, from: Location, to: Location) -> Float {
        match self.balance {
            Some(TerritoryBalance::Duration) => self.transport.duration_approx(&self.profile, from, to),
            // Service balances a quantity travel is no part of, so the estimate carries none
            // either — see the caller, which adds only `service_share`.
            Some(TerritoryBalance::Service) => 0.0,
            _ => self.transport.distance_approx(&self.profile, from, to),
        }
    }

    /// The average balance metric per job (floored positive). Used to normalize the PUSH marginal
    /// onto the PULL (distance) scale. `1.0` when balance is disabled (metric is then unused).
    fn compute_avg_metric(&self, jobs: &Jobs) -> Float {
        if self.balance.is_none() {
            return 1.0;
        }
        let all = jobs.all();
        let n = all.len().max(1);
        // Averaged over the INSERTION metric, because that is what `push_marginal` divides by:
        // the ratio has to be dimensionless, so both sides must measure the same thing.
        let total: Float = all.iter().map(|j| self.insertion_metric(j)).sum();
        (total / n as Float).max(1e-9)
    }

    /// The median per-job power gap (`second_power − nearest_power`) over jobs with at least two
    /// compatible anchors — the PUSH marginal's reach. `0.0` when no job has an alternative anchor
    /// (then the marginal is always zero, i.e. no per-insertion balance pressure).
    fn compute_push_reach(&self) -> Float {
        let mut gaps: Vec<Float> = self
            .job_nearest_power
            .iter()
            .filter_map(|(id, &np)| {
                let sp = self.job_second_power.get(id).copied().unwrap_or(Float::INFINITY);
                sp.is_finite().then_some(sp - np)
            })
            .collect();
        if gaps.is_empty() {
            return 0.0;
        }
        gaps.sort_by(|a, b| a.total_cmp(b));
        gaps[gaps.len() / 2]
    }

    /// Actor scan producing, per compatible driver, the proximity to that driver's NEAREST anchor,
    /// sorted ascending. Used to precompute `job_anchor_ranking` and as the uncached fallback for
    /// the lookups above. A driver holding no anchor never enters the ranking, so it is invisible to
    /// PULL's reference and to the power lookups built on top of it.
    fn scan_sorted_anchors(&self, job_loc: Location, job: &Job) -> Vec<(DriverKey, Float)> {
        let mut seen: HashMap<DriverKey, Float> = HashMap::new();
        for actor in self.actors.iter() {
            if !(self.compatibility_fn)(job, actor) {
                continue;
            }
            let key = driver_key(actor);
            if seen.contains_key(&key) {
                continue;
            }
            if let Some(prox) = self.driver_prox(&key, job_loc) {
                seen.insert(key, prox);
            }
        }
        let mut ranking: Vec<(DriverKey, Float)> = seen.into_iter().collect();
        // Two drivers exactly equidistant from a job are a real tie (mirrored anchors, a shared
        // depot, a coarse matrix). `seen` is a `HashMap` and the sort is stable, so without a
        // second key the tied pair would keep hash order — which is the order of the *n*-th map
        // this process happened to build, not a property of the input. The driver key breaks it.
        // Every reader of this ranking today takes only a proximity, so the tie is currently
        // invisible; it is broken here so that a reader of the *key* at a tied position — the
        // obvious next use of a ranking — inherits a defined order instead of the hash seed.
        ranking.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
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

    /// The balance quantity for a single route. Shared between `loads` (solution-wide) and the
    /// per-route `TerritoryRouteLoad` cache.
    ///
    /// `Activities` and `ProductionValue` are sums over the route's jobs, so they stay per-job
    /// sums — a stop is a stop wherever it sits in the tour. `Distance` and `Duration` are
    /// properties of the ROUTE: travel depends on the order the stops are visited, so no per-job
    /// term can express it, and the anchor-proximity proxy this used to sum was blind to the very
    /// thing it claimed to measure (two hours of driving between two ten-minute jobs read as two
    /// short hops). Both are read from the state the transport feature already maintains —
    /// `get_total_duration()` is the span the vehicle is PAID for, under its own `RouteCostSpan`,
    /// and `get_total_distance()` is the whole route, because miles are vehicle cost and are
    /// incurred on the commute legs whoever is on the clock for them.
    fn route_load(&self, route_ctx: &RouteContext) -> Float {
        match self.balance {
            None => 0.0,
            Some(TerritoryBalance::Activities)
            | Some(TerritoryBalance::ProductionValue)
            | Some(TerritoryBalance::Service) => route_ctx.route().tour.jobs().map(|j| self.job_metric(j)).sum(),
            Some(TerritoryBalance::Distance) => route_ctx.state().get_total_distance().copied().unwrap_or(0.0),
            Some(TerritoryBalance::Duration) => route_ctx.state().get_paid_working_duration().copied().unwrap_or(0.0),
        }
    }

    /// Current per-driver load: the sum of the balance metric across all jobs on that driver's
    /// route(s) in the given solution. Includes every driver with a quota, even if idle.
    fn loads(&self, solution: &SolutionContext) -> HashMap<DriverKey, Float> {
        let mut loads: HashMap<DriverKey, Float> = self.quota_keys().map(|k| (k.clone(), 0.0)).collect();
        for route_ctx in solution.routes.iter() {
            let key = driver_key(&route_ctx.route().actor);
            *loads.entry(key).or_insert(0.0) += self.route_load(route_ctx);
        }
        loads
    }

    /// Total PULL (overlap penalty) for the solution: for each assigned job, the excess of its
    /// driver's power distance over the minimum power distance across the job's compatible anchors.
    /// Zero when every job sits in its own power cell (no cross-boundary reaching). Depot start/end
    /// activities are not jobs, so `tour.jobs()` already excludes the shared office from this sum.
    fn pull(&self, solution: &SolutionContext) -> Cost {
        let mut total = 0.0;
        for route_ctx in solution.routes.iter() {
            let actor = &route_ctx.route().actor;
            let key = driver_key(actor);
            // A driver holding no anchor takes no part in the territory, so it accrues no PULL.
            let Some(assigned_anchors) = self.driver_anchors(&key) else { continue };
            let weight = self.weight(&key);
            let route_anchor = if self.day_patch { self.route_anchor(route_ctx) } else { None };
            for job in route_ctx.route().tour.jobs() {
                let Some(loc) = get_job_location(job) else { continue };
                let assigned_prox = match route_anchor {
                    Some(anchor) => self.proximity(loc, anchor),
                    None => self.min_prox_to(assigned_anchors, loc),
                };
                let assigned_power = assigned_prox - weight;
                let reference = self.nearest_power(loc, job);
                total += (assigned_power - reference).max(0.0);
            }
        }
        total
    }

    /// Total PUSH for the solution: a convex imbalance penalty per over-quota driver, weighted by
    /// how far that driver's surplus would have to travel to reach the *nearest* deficit driver's
    /// anchor. It is not a transport bound — nothing is routed and deficit capacity is ignored — it
    /// prices imbalance and lets distance say where imbalance hurts most. Zero when no driver is
    /// over quota.
    ///
    /// The surplus is expressed in *jobs*, not in raw balance metric: it is divided by `avg_metric`
    /// exactly as [`Self::push_marginal`] divides the job metric. PULL sums a proximity per job, so
    /// without that division a solution balancing production value would add dollars × metres to
    /// metres, and the ratio between the two terms would be set by the average job value — a
    /// property of the caller's price list, not of how much balance was asked for. Normalized, both
    /// terms are jobs × metres and [`PUSH_CONVEXITY_GAIN`] means the same thing whatever the metric
    /// is measured in.
    fn push(&self, solution: &SolutionContext) -> Cost {
        if self.balance.is_none() || (self.quotas.is_empty() && self.quota_shares.is_empty()) {
            return 0.0;
        }
        let loads = self.loads(solution);
        // With `allow_idle_drivers`, this spans only the used drivers, so idle drivers are neither
        // deficits (targets) nor surplus (sources) — leaving one idle is not an imbalance.
        let quotas = self.effective_quotas(&loads);

        // Every anchor held by a deficit driver. The deadband narrows the deficit threshold, so a
        // driver only just below quota is not treated as needing more work. Flattening the lists is
        // exactly right for the ground cost below: the minimum over this flat set, taken from each
        // source anchor, IS the minimum over the (source anchor, deficit anchor) pairs. A driver
        // holding no anchor contributes nothing and so is never a target.
        // Walked in `driver_order` rather than the quota map's hash order, so the flattened anchor
        // list is the same sequence in every process (see [`Self::driver_order`]). Only a `min`
        // reads it below, which is order-free, so this is hygiene rather than a live fix — but it
        // keeps every per-driver walk in this feature on one order, and the next reduction added
        // here is then right by default.
        let deficits: Vec<Location> = self
            .driver_order
            .iter()
            .filter_map(|key| quotas.get(key).map(|quota| (key, quota)))
            .filter(|(key, quota)| loads.get(*key).copied().unwrap_or(0.0) + 1e-9 < self.under_quota(**quota))
            .filter_map(|(key, _)| self.driver_anchors(key))
            .flatten()
            .copied()
            .collect();
        if deficits.is_empty() {
            return 0.0;
        }

        let mut total = 0.0;
        // The leading surplus is divided by `avg_metric`, so it reads as "how many jobs' worth of
        // value sits in the wrong hands" rather than a raw amount of the balance metric. That puts
        // PUSH in the same jobs × distance unit as PULL (which sums a proximity per job) and as
        // [`Self::push_marginal`] (which already divides by `avg_metric`), so the three can be added
        // and compared without the tenant's price list setting the exchange rate between them.
        // The second factor is the shape, not the unit: `surplus / quota` is dimensionless and
        // independent of problem size, and [`PUSH_CONVEXITY_GAIN`] keeps convexity from being bought
        // by lowering the curve. A term linear in surplus has its optimum in a corner: stacking all
        // the imbalance on the one driver nearest a deficit anchor is then cheaper than spreading it,
        // so the objective produced the outlier it exists to prevent. A rising marginal price makes
        // spreading cheaper than stacking.
        // Again `driver_order`: this is a float sum, so its rounding depends on the fold order.
        for key in self.driver_order.iter() {
            let Some(&quota) = quotas.get(key) else { continue };
            let load = loads.get(key).copied().unwrap_or(0.0);
            // Only the load beyond the widened (deadband) quota is surplus: small imbalances inside
            // the band are free, so the solver stops exiling jobs to shave the last few percent.
            let surplus = load - self.over_quota(quota);
            if surplus <= 1e-9 {
                continue;
            }
            // A driver holding no anchor is never a source either.
            let Some(anchors) = self.driver_anchors(key) else { continue };
            // Ground cost = min over (source anchor, deficit anchor) pairs. Kept in the source →
            // deficit direction, because `proximity` is not required to be symmetric.
            let nearest = anchors
                .iter()
                .flat_map(|&s| deficits.iter().map(move |&d| (s, d)))
                .map(|(s, d)| self.proximity(s, d))
                .min_by(|x, y| x.total_cmp(y))
                .unwrap_or(0.0);
            // A zero quota has no scale to normalise against, and it is already maximal imbalance —
            // nothing about it should be softened, so the term stays linear there.
            let convexity = if quota > 1e-9 { PUSH_CONVEXITY_GAIN * surplus / quota } else { 1.0 };
            total += (surplus / self.avg_metric) * convexity * nearest;
        }

        if self.deficit_weight <= 0.0 {
            return total;
        }

        // The mirror: what a driver's own shortfall costs, rather than only what somebody else's
        // surplus costs. The two are readings of ONE misallocation, because work is conserved — so
        // this is not new information, it is the same information seen from the side the deadband
        // does not hide. A shortfall concentrated on one technician shows up as a small surplus
        // spread over the rest, and every one of those can sit inside its own band while the
        // shortfall is large. Measured on a six-technician tenant, run 40: one technician 17% short, nobody else
        // more than 10.6% over, and narrowing the band to 1% did not move it.
        //
        // Counterparties are the over-band drivers, mirroring the surplus side's use of deficit
        // anchors. When nobody is over the band there is still work to fetch and it has to come
        // from somewhere, so the reference widens to every OTHER anchored driver rather than
        // collapsing to zero and silently disabling the term in exactly the case it exists for.
        let sources: Vec<Location> = self
            .driver_order
            .iter()
            .filter_map(|key| quotas.get(key).map(|quota| (key, quota)))
            .filter(|(key, quota)| loads.get(*key).copied().unwrap_or(0.0) > self.over_quota(**quota) + 1e-9)
            .filter_map(|(key, _)| self.driver_anchors(key))
            .flatten()
            .copied()
            .collect();

        for key in self.driver_order.iter() {
            let Some(&quota) = quotas.get(key) else { continue };
            let load = loads.get(key).copied().unwrap_or(0.0);
            let deficit = self.under_quota(quota) - load;
            if deficit <= 1e-9 {
                continue;
            }
            let Some(anchors) = self.driver_anchors(key) else { continue };

            let counterparties: Vec<Location> = if sources.is_empty() {
                self.driver_order
                    .iter()
                    .filter(|other| *other != key)
                    .filter_map(|other| self.driver_anchors(other))
                    .flatten()
                    .copied()
                    .collect()
            } else {
                sources.clone()
            };

            // Kept in the deficit → source direction, because `proximity` is not required to be
            // symmetric and this is the leg the work would travel back along.
            let nearest = anchors
                .iter()
                .flat_map(|&d| counterparties.iter().map(move |&s| (d, s)))
                .map(|(d, s)| self.proximity(d, s))
                .min_by(|x, y| x.total_cmp(y))
                .unwrap_or(0.0);

            let convexity = if quota > 1e-9 { PUSH_CONVEXITY_GAIN * deficit / quota } else { 1.0 };
            total += self.deficit_weight * (deficit / self.avg_metric) * convexity * nearest;
        }

        total
    }

    /// The dual-price marginal contribution of assigning `job` to `route_ctx`'s driver while that
    /// driver is over quota (per the cached route load): a *location-aware* shedding pressure.
    ///
    /// The old marginal was `job_metric × nearest_other_anchor` — a per-driver constant, so it
    /// repelled every extra job from an over-quota driver by the same amount (scaled only by value),
    /// deep-in-cell jobs as hard as boundary ones, and the value factor made it dwarf PULL. Both are
    /// fixed here:
    /// - the metric is normalized by `avg_metric`, putting the pressure on PULL's (distance) scale;
    /// - it is priced by `max(0, push_reach − gap)` where `gap` is how much deeper this job sits in
    ///   this driver's cell than in its next-best one. A boundary job (small/negative gap) is cheap
    ///   to shed and carries pressure; a job deeper than `push_reach` carries none, so an over-quota
    ///   driver rebalances by giving up its border jobs, not the ones buried in its territory.
    ///
    /// An estimate has to measure the same quantity, in the same unit, at the same AGGREGATION LEVEL
    /// as the fitness it estimates. The deadband here read one route's load — one actor is one shift
    /// is one route — against the driver's whole-horizon quota, which a single day can never exceed,
    /// so in any multi-day problem this returned zero always and the per-insertion balance pressure
    /// was dead code. The quota is scaled to this route's share of the driver's capacity first. The
    /// returned pressure is then the derivative of the convex PUSH fitness (see [`Self::push`]),
    /// `d/ds [GAIN · s²/q] = 2 · GAIN · s/q` — it grows with how far over the band this route already
    /// is — rather than a flat step. Note the version before the convex fitness already read as a
    /// derivative without being one: it was off by the factor of 2, and carried no gain at all, so
    /// the estimate steered construction on a weaker price than the fitness it approximates.
    /// The shedding pressure one INSERTION carries, priced where the position is known.
    ///
    /// ⚠️ Evaluated at the activity, not at the route. What an insertion adds to a route's load is
    /// its service plus a DETOUR between two neighbours — not the job's standalone round trip from
    /// a depot, which is several times larger, and which discriminates by distance-from-depot
    /// rather than by distance-from-this-route. A technician whose whole territory sits far from
    /// the depot saw every job as expensive under that reading.
    ///
    /// The detour is what `estimate_leg` already computes for the transport feature, so this
    /// prices an insertion with the same arithmetic the plan is costed by.
    fn push_marginal(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext, avg_load: Float) -> Cost {
        if self.balance.is_none() {
            return 0.0;
        }
        // When idle drivers are allowed, the estimate should not spread work off drivers (that would
        // fill idle ones): concentration to the feasible minimum is fine and the fitness still
        // balances the used drivers. So there is no per-insertion push signal in that mode.
        if self.allow_idle_drivers {
            return 0.0;
        }
        let Some(single) = activity_ctx.target.job.as_ref() else { return 0.0 };
        let job = Job::Single(single.clone());
        let actor = &route_ctx.route().actor;
        let key = driver_key(actor);
        let Some(assigned_anchors) = self.driver_anchors(&key) else {
            return 0.0;
        };
        let load = route_ctx.state().get_territory_route_load().copied().unwrap_or(0.0);
        // This route's own slice of its driver's quota, cached by `cache_route_quotas`. Absent
        // before the first solution-state pass, which reads as no shedding pressure: the slice
        // depends on which routes the driver actually has, and before a solution exists there is
        // no answer to that.
        let route_quota = route_ctx.state().get_territory_route_quota().copied().unwrap_or(0.0);
        if route_quota <= 0.0 {
            return 0.0;
        }
        // Deadband: no shedding pressure until the route is over the widened quota.
        if load <= self.over_quota(route_quota) {
            return 0.0;
        }
        let loc = activity_ctx.target.place.location;

        // gap = (nearest power among OTHER drivers) − (this driver's power for the job). Large gap ⇒
        // this driver is much the better home ⇒ the job is deep in its cell; small/negative ⇒ it is
        // a boundary/foreign job with a cheap alternative.
        let assigned_power = self.min_prox_to(assigned_anchors, loc) - self.weight(&key);
        let reference = self.nearest_power(loc, &job);
        let min_other = if assigned_power <= reference + 1e-9 {
            single.dimens.get_job_id().and_then(|id| self.job_second_power.get(id)).copied().unwrap_or(Float::INFINITY)
        } else {
            reference
        };
        let gap = min_other - assigned_power;
        let value_factor = self.insertion_delta(activity_ctx) / avg_load;
        // Derivative of the convex PUSH, `2 · GAIN · surplus / quota`: the price rises with how far
        // over the band this route sits, so a flat step is replaced by pressure that grows with the
        // imbalance it prices — on the same scale as the fitness rather than a fraction of it.
        let surplus_ratio = (load - self.over_quota(route_quota)) / route_quota.max(1e-9);
        value_factor * 2.0 * PUSH_CONVEXITY_GAIN * surplus_ratio * (self.push_reach - gap).max(0.0)
    }

    /// The travel this insertion adds between its two neighbours:
    /// `prev -> target -> next` less the `prev -> next` it replaces. Zero at an open end, where
    /// nothing is replaced.
    ///
    /// Measured with the approximate lookups, as [`Self::proximity`] is — an estimate does not
    /// need the routed, departure-time-dependent figure the transport feature computes, and paying
    /// for it once per candidate position would be the most expensive thing in the search.
    fn detour(&self, activity_ctx: &ActivityContext, as_duration: bool) -> Float {
        let travel = |from: Location, to: Location| {
            if as_duration {
                self.transport.duration_approx(&self.profile, from, to)
            } else {
                self.transport.distance_approx(&self.profile, from, to)
            }
        };

        let prev = activity_ctx.prev.place.location;
        let target = activity_ctx.target.place.location;

        match activity_ctx.next {
            Some(next) => {
                let next = next.place.location;
                (travel(prev, target) + travel(target, next) - travel(prev, next)).max(0.0)
            }
            None => travel(prev, target),
        }
    }

    /// What this insertion adds to the route's balance load, at this position.
    ///
    /// The counterpart of [`Self::route_load`], one insertion at a time: the same quantity the
    /// fitness measures, which is what `.ai/rules/solver.md` means by an estimate agreeing with its
    /// fitness in quantity, unit and aggregation level.
    fn insertion_delta(&self, activity_ctx: &ActivityContext) -> Float {
        let service = activity_ctx.target.place.duration;

        match self.balance {
            None => 0.0,
            Some(TerritoryBalance::Activities) => 1.0,
            Some(TerritoryBalance::Service) => service,
            Some(TerritoryBalance::ProductionValue) => activity_ctx
                .target
                .job
                .as_ref()
                .map(|single| (self.job_value_fn)(&Job::Single(single.clone())))
                .unwrap_or(0.0),
            Some(TerritoryBalance::Distance) => self.detour(activity_ctx, false),
            // Service is added on top of the drive, because the paid working duration this
            // balances is both.
            Some(TerritoryBalance::Duration) => service + self.detour(activity_ctx, true),
        }
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
            // PULL is a property of WHICH driver takes the job, not of where in their day it
            // lands, so it is priced once per route.
            MoveContext::Route { route_ctx, job, .. } => {
                let Some(loc) = get_job_location(job) else { return Cost::default() };
                let actor = &route_ctx.route().actor;
                let key = driver_key(actor);
                let Some(assigned_prox) = self.shared.driver_prox(&key, loc) else {
                    return Cost::default();
                };
                let assigned_prox = match route_ctx.state().get_territory_route_anchor() {
                    Some(&anchor) if self.shared.day_patch => self.shared.proximity(loc, anchor),
                    _ => assigned_prox,
                };
                let assigned_power = assigned_prox - self.shared.weight(&key);
                let reference = self.shared.nearest_power(loc, job);
                (assigned_power - reference).max(0.0)
            }
            // PUSH is not: what an insertion adds to a route's load is the detour it causes, and
            // that is only knowable once the position is.
            MoveContext::Activity { solution_ctx, route_ctx, activity_ctx } => {
                let avg_load =
                    solution_ctx.state.get_territory_avg_load().copied().unwrap_or(self.shared.avg_metric).max(1e-9);

                self.shared.push_marginal(route_ctx, activity_ctx, avg_load)
            }
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
        if self.shared.day_patch {
            // A route that empties holds no patch any more: its next first job may pick any of them.
            match self.shared.route_anchor(route_ctx) {
                Some(anchor) => route_ctx.state_mut().set_territory_route_anchor(anchor),
                None => {
                    route_ctx.state_mut().remove_territory_route_anchor();
                }
            }
        }
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| self.accept_route_state(route_ctx));
        self.cache_route_quotas(solution_ctx);
        self.recompute(solution_ctx);
    }
}

impl TerritoryState {
    /// Writes each route's own slice of its driver's quota into the route state.
    ///
    /// `push_marginal` runs during insertion with only a `RouteContext` in hand. Under supplied
    /// shares the driver's quota does not exist until a solution supplies the level, and even under
    /// a static quota the route's slice had to be recomputed on every estimate. Caching it here —
    /// the one place that already walks every route, after their loads are refreshed — makes the
    /// marginal a route-local read and puts the scaling in a single place.
    fn cache_route_quotas(&self, solution_ctx: &mut SolutionContext) {
        let loads = self.shared.loads(solution_ctx);
        let quotas = self.shared.effective_quotas(&loads);

        // ⚠️ Spread over the routes the driver ACTUALLY has, not over every shift they own.
        //
        // A quota spans the horizon while a route is one day, so the two have to be brought onto
        // one scale — but dividing by the driver's whole capacity assumes every shift becomes a
        // route. It does not: a technician with nineteen shifts may work ten. Their load then lands
        // on ten routes while each is measured against a nineteenth of the quota, so EVERY route
        // reads as over quota, permanently and for every driver — including the ones who are far
        // under their share and ought to be receiving work. The marginal then sheds from everybody
        // and discriminates between nobody.
        //
        // Against the active capacity the two sides add up: the driver's route quotas sum to their
        // quota exactly as their route loads sum to their load, so a driver sitting at quota has no
        // route over it, and a day carrying more than its peers still does.
        let mut active_capacity: HashMap<DriverKey, Float> = HashMap::new();
        for route_ctx in solution_ctx.routes.iter() {
            let actor = &route_ctx.route().actor;
            *active_capacity.entry(driver_key(actor)).or_insert(0.0) +=
                (actor.detail.time.end - actor.detail.time.start).max(0.0);
        }

        for route_ctx in solution_ctx.routes.iter_mut() {
            let actor = &route_ctx.route().actor;
            let key = driver_key(actor);
            let capacity = active_capacity.get(&key).copied().unwrap_or(0.0);
            let route_capacity = (actor.detail.time.end - actor.detail.time.start).max(0.0);

            let route_quota = match quotas.get(&key) {
                Some(&driver_quota) if capacity > 0.0 => driver_quota * route_capacity / capacity,
                _ => 0.0,
            };

            route_ctx.state_mut().set_territory_route_quota(route_quota);
        }
    }

    fn recompute(&self, solution_ctx: &mut SolutionContext) {
        let pull = self.shared.pull(solution_ctx);
        let push = self.shared.push(solution_ctx);
        solution_ctx.state.set_territory_fitness(TerritoryFitnessData { pull, push });

        // What one job typically contributes to a route's load, measured rather than estimated.
        //
        // It is the divisor that turns PUSH's surplus into "jobs' worth", and the marginal now
        // measures a DETOUR — so the divisor has to be the same kind of quantity, or the two are
        // on different scales and `PUSH_CONVEXITY_GAIN` stops meaning one thing. For `Activities`
        // it is identically 1.0 and for `ProductionValue` the mean job value, so the two metrics
        // the gain is calibrated against are unmoved.
        let jobs: usize = solution_ctx.routes.iter().map(|route_ctx| route_ctx.route().tour.job_count()).sum();
        let load: Float = solution_ctx.routes.iter().map(|route_ctx| self.shared.route_load(route_ctx)).sum();
        solution_ctx.state.set_territory_avg_load(if jobs == 0 {
            self.shared.avg_metric
        } else {
            (load / jobs as Float).max(1e-9)
        });
    }
}
