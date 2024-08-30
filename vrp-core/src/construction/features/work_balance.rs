//! Provides the way to build one of the flavors of the work balance feature.

use super::*;
use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState};
use crate::construction::features::capacity::MaxFutureCapacityActivityState;
use crate::models::common::LoadOps;
use rosomaxa::algorithms::math::get_cv_safe;
use std::cmp::Ordering;
use std::marker::PhantomData;

/// Creates a feature which balances max load across all tours.
pub fn create_max_load_balanced_feature<T>(
    name: &str,
    load_balance_fn: impl Fn(&T, &T) -> Float + Send + Sync + 'static,
    vehicle_capacity_fn: impl Fn(&Vehicle) -> &T + Send + Sync + 'static,
) -> Result<Feature, GenericError>
where
    T: LoadOps,
{
    struct MaxLoadBalancedKey;

    let default_capacity = T::default();
    let default_intervals = vec![(0_usize, 0_usize)];

    let get_load_ratio = Arc::new(move |route_ctx: &RouteContext| {
        let capacity = vehicle_capacity_fn(&route_ctx.route().actor.vehicle);
        let intervals = route_ctx.state().get_reload_intervals().unwrap_or(&default_intervals);

        intervals
            .iter()
            .map(|(start_idx, _)| route_ctx.state().get_max_future_capacity_at(*start_idx).unwrap_or(&default_capacity))
            .map(|max_load| (load_balance_fn)(max_load, capacity))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
            .unwrap_or(Float::default())
    });

    let route_estimate_fn = get_load_ratio.clone();
    let solution_estimate_fn = Arc::new(move |ctx: &SolutionContext| {
        get_cv_safe(ctx.routes.iter().map(|route_ctx| get_load_ratio(route_ctx)).collect::<Vec<_>>().as_slice())
    });

    create_feature::<MaxLoadBalancedKey>(name, route_estimate_fn, solution_estimate_fn)
}

/// Creates a feature which balances activities across all tours.
pub fn create_activity_balanced_feature(name: &str) -> Result<Feature, GenericError> {
    struct ActivityBalancedKey;

    let route_estimate_fn = Arc::new(|route_ctx: &RouteContext| route_ctx.route().tour.job_activity_count() as Float);
    let solution_estimate_fn = Arc::new(|solution_ctx: &SolutionContext| {
        get_cv_safe(
            solution_ctx
                .routes
                .iter()
                .map(|route_ctx| route_ctx.route().tour.job_activity_count() as Float)
                .collect::<Vec<_>>()
                .as_slice(),
        )
    });

    create_feature::<ActivityBalancedKey>(name, route_estimate_fn, solution_estimate_fn)
}

/// Creates a feature which which balances travelled durations across all tours.
pub fn create_duration_balanced_feature(name: &str) -> Result<Feature, GenericError> {
    struct DurationBalancedKey;

    create_transport_balanced_feature::<DurationBalancedKey>(name, |state| state.get_total_duration())
}

/// Creates a feature which which balances travelled distances across all tours.
pub fn create_distance_balanced_feature(name: &str) -> Result<Feature, GenericError> {
    struct DistanceBalancedKey;
    create_transport_balanced_feature::<DistanceBalancedKey>(name, |state| state.get_total_distance())
}

fn create_transport_balanced_feature<K: Send + Sync + 'static>(
    name: &str,
    value_fn: impl Fn(&RouteState) -> Option<&Float> + Send + Sync + 'static,
) -> Result<Feature, GenericError> {
    let route_estimate_fn =
        Arc::new(move |route_ctx: &RouteContext| value_fn(route_ctx.state()).cloned().unwrap_or(0.));

    let solution_estimate_fn = Arc::new({
        let route_estimate_fn = route_estimate_fn.clone();
        move |ctx: &SolutionContext| {
            get_cv_safe(ctx.routes.iter().map(|route_ctx| route_estimate_fn(route_ctx)).collect::<Vec<_>>().as_slice())
        }
    });

    create_feature::<K>(name, route_estimate_fn, solution_estimate_fn)
}

fn create_feature<K: Send + Sync + 'static>(
    name: &str,
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> Float + Send + Sync>,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(WorkBalanceObjective {
            route_estimate_fn: route_estimate_fn.clone(),
            solution_estimate_fn: solution_estimate_fn.clone(),
            phantom_data: PhantomData::<K>,
        })
        .with_state(WorkBalanceState { route_estimate_fn, solution_estimate_fn, phantom_data: PhantomData::<K> })
        .build()
}

struct WorkBalanceObjective<K: Send + Sync + 'static> {
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> Float + Send + Sync>,
    phantom_data: PhantomData<K>,
}

impl<K: Send + Sync + 'static> FeatureObjective for WorkBalanceObjective<K> {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .state
            .get_value::<K, Float>()
            .cloned()
            .unwrap_or_else(|| (self.solution_estimate_fn)(&solution.solution))
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => {
                let value = route_ctx
                    .state()
                    .get_tour_state::<K, Float>()
                    .cloned()
                    .unwrap_or_else(|| (self.route_estimate_fn)(route_ctx));

                // NOTE: this value doesn't consider a route state after insertion of given job
                if value.is_finite() {
                    value
                } else {
                    Cost::default()
                }
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct WorkBalanceState<K: Send + Sync + 'static> {
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> Float + Send + Sync>,
    phantom_data: PhantomData<K>,
}

impl<K: Send + Sync + 'static> FeatureState for WorkBalanceState<K> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let value = (self.route_estimate_fn)(route_ctx);

        route_ctx.state_mut().set_tour_state::<K, _>(value);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let value = (self.solution_estimate_fn)(solution_ctx);

        solution_ctx.state.set_value::<K, _>(value);
    }
}
