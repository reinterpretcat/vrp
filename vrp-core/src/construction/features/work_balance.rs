//! Provides features to balance work.

use super::*;
use crate::models::common::{CapacityDimension, LoadOps};
use rosomaxa::algorithms::math::get_cv_safe;
use std::cmp::Ordering;

/// Specifies load function type.
pub type LoadBalanceFn<T> = Arc<dyn Fn(&T, &T) -> f64 + Send + Sync>;

/// Creates a feature which balances max load across all tours.
pub fn create_max_load_balanced_feature<T: LoadOps>(
    name: &str,
    threshold: Option<f64>,
    load_balance_fn: LoadBalanceFn<T>,
) -> Result<Feature, GenericError> {
    let default_capacity = T::default();
    let default_intervals = vec![(0_usize, 0_usize)];

    let get_load_ratio = Arc::new(move |route_ctx: &RouteContext| {
        let capacity = route_ctx.route().actor.vehicle.dimens.get_capacity().unwrap();
        let intervals = route_ctx
            .state()
            .get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY)
            .unwrap_or(&default_intervals);

        intervals
            .iter()
            .map(|(start_idx, _)| {
                route_ctx
                    .state()
                    .get_activity_state::<T>(MAX_FUTURE_CAPACITY_KEY, *start_idx)
                    .unwrap_or(&default_capacity)
            })
            .map(|max_load| (load_balance_fn)(max_load, capacity))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
            .unwrap_or(0_f64)
    });

    let route_estimate_fn = get_load_ratio.clone();
    let solution_estimate_fn = Arc::new(move |ctx: &SolutionContext| {
        get_cv_safe(ctx.routes.iter().map(|route_ctx| get_load_ratio(route_ctx)).collect::<Vec<_>>().as_slice())
    });

    create_feature(name, threshold, BALANCE_MAX_LOAD_KEY, route_estimate_fn, solution_estimate_fn)
}

/// Creates a feature which balances activities across all tours.
pub fn create_activity_balanced_feature(name: &str, threshold: Option<f64>) -> Result<Feature, GenericError> {
    let route_estimate_fn = Arc::new(|route_ctx: &RouteContext| route_ctx.route().tour.job_activity_count() as f64);
    let solution_estimate_fn = Arc::new(|solution_ctx: &SolutionContext| {
        get_cv_safe(
            solution_ctx
                .routes
                .iter()
                .map(|route_ctx| route_ctx.route().tour.job_activity_count() as f64)
                .collect::<Vec<_>>()
                .as_slice(),
        )
    });

    create_feature(name, threshold, BALANCE_ACTIVITY_KEY, route_estimate_fn, solution_estimate_fn)
}

/// Creates a feature which which balances travelled durations across all tours.
pub fn create_duration_balanced_feature(name: &str, threshold: Option<f64>) -> Result<Feature, GenericError> {
    create_transport_balanced_feature(name, threshold, TOTAL_DURATION_KEY, BALANCE_DURATION_KEY)
}

/// Creates a feature which which balances travelled distances across all tours.
pub fn create_distance_balanced_feature(name: &str, threshold: Option<f64>) -> Result<Feature, GenericError> {
    create_transport_balanced_feature(name, threshold, TOTAL_DISTANCE_KEY, BALANCE_DISTANCE_KEY)
}

fn create_transport_balanced_feature(
    name: &str,
    threshold: Option<f64>,
    value_key: StateKey,
    state_key: StateKey,
) -> Result<Feature, GenericError> {
    let route_estimate_fn = Arc::new(move |route_ctx: &RouteContext| {
        route_ctx.state().get_route_state::<f64>(value_key).cloned().unwrap_or(0.)
    });

    let solution_estimate_fn = Arc::new(move |ctx: &SolutionContext| {
        get_cv_safe(
            ctx.routes
                .iter()
                .map(|route_ctx| route_ctx.state().get_route_state::<f64>(value_key).cloned().unwrap_or(0.))
                .collect::<Vec<_>>()
                .as_slice(),
        )
    });

    create_feature(name, threshold, state_key, route_estimate_fn, solution_estimate_fn)
}

fn create_feature(
    name: &str,
    threshold: Option<f64>,
    state_key: StateKey,
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> f64 + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> f64 + Send + Sync>,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(WorkBalanceObjective {
            threshold,
            state_key,
            route_estimate_fn: route_estimate_fn.clone(),
            solution_estimate_fn: solution_estimate_fn.clone(),
        })
        .with_state(WorkBalanceState {
            state_key,
            state_keys: vec![state_key],
            route_estimate_fn,
            solution_estimate_fn,
        })
        .build()
}

struct WorkBalanceObjective {
    threshold: Option<f64>,
    state_key: StateKey,
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> f64 + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> f64 + Send + Sync>,
}

impl Objective for WorkBalanceObjective {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        let fitness_a = self.fitness(a);
        let fitness_b = self.fitness(b);

        if let Some(threshold) = self.threshold {
            if fitness_a < threshold && fitness_b < threshold {
                return Ordering::Equal;
            }

            if fitness_a < threshold {
                return Ordering::Less;
            }

            if fitness_b < threshold {
                return Ordering::Greater;
            }
        }

        compare_floats(fitness_a, fitness_b)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .state
            .get(&self.state_key)
            .and_then(|s| s.downcast_ref::<f64>())
            .cloned()
            .unwrap_or_else(|| (self.solution_estimate_fn)(&solution.solution))
    }
}

impl FeatureObjective for WorkBalanceObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => {
                let value = route_ctx
                    .state()
                    .get_route_state::<f64>(self.state_key)
                    .cloned()
                    .unwrap_or_else(|| (self.route_estimate_fn)(route_ctx));

                // NOTE: this value doesn't consider a route state after insertion of given job
                if value.is_finite() && self.threshold.map_or(true, |threshold| value > threshold) {
                    value
                } else {
                    Cost::default()
                }
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct WorkBalanceState {
    state_key: StateKey,
    state_keys: Vec<StateKey>,
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> f64 + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> f64 + Send + Sync>,
}

impl FeatureState for WorkBalanceState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let value = (self.route_estimate_fn)(route_ctx);

        route_ctx.state_mut().put_route_state(self.state_key, value);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let value = (self.solution_estimate_fn)(solution_ctx);

        solution_ctx.state.insert(self.state_key, Arc::new(value));
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}
