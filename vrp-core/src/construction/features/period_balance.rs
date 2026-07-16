//! Provides a feature to balance a work metric per employee across the whole planning period.

use super::*;
use rosomaxa::algorithms::math::get_cv_safe;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Creates a feature which balances a work metric (e.g. distance, duration, activity count or
/// production value) per employee across the whole planning period, instead of per tour.
///
/// Tours are grouped by `group_key_fn` (typically the employee's `vehicle_id`, which is shared
/// across all `VehicleType` splits of the same employee). For each group, the summed
/// `tour_metric_fn` value across all of its tours is normalized by its available capacity (e.g.
/// amount of available shifts) taken from `group_capacities`. Every key present in
/// `group_capacities` contributes a ratio to the balance calculation, even if the employee has no
/// tours in the current solution (ratio of zero), so that idle employees are not invisible to the
/// objective.
pub fn create_period_balanced_feature(
    name: &str,
    group_capacities: HashMap<String, usize>,
    group_key_fn: impl Fn(&Actor) -> Option<String> + Send + Sync + 'static,
    tour_metric_fn: impl Fn(&RouteContext) -> Float + Send + Sync + 'static,
) -> Result<Feature, GenericError> {
    struct PeriodBalanceKey;

    let route_estimate_fn = Arc::new(tour_metric_fn);

    let solution_estimate_fn = Arc::new({
        let route_estimate_fn = route_estimate_fn.clone();
        move |solution_ctx: &SolutionContext| {
            let mut group_usage: HashMap<String, Float> = HashMap::new();

            for route_ctx in solution_ctx.routes.iter() {
                if let Some(key) = group_key_fn(&route_ctx.route().actor) {
                    *group_usage.entry(key).or_insert(0.) += route_estimate_fn(route_ctx);
                }
            }

            let ratios = group_capacities
                .iter()
                .map(|(key, &capacity)| group_usage.get(key).copied().unwrap_or(0.) / capacity as Float)
                .collect::<Vec<_>>();

            get_cv_safe(ratios.as_slice())
        }
    });

    create_feature::<PeriodBalanceKey>(name, route_estimate_fn, solution_estimate_fn)
}

fn create_feature<K: Send + Sync + 'static>(
    name: &str,
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> Float + Send + Sync>,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(PeriodBalanceObjective {
            route_estimate_fn: route_estimate_fn.clone(),
            solution_estimate_fn: solution_estimate_fn.clone(),
            phantom_data: PhantomData::<K>,
        })
        .with_state(PeriodBalanceState { route_estimate_fn, solution_estimate_fn, phantom_data: PhantomData::<K> })
        .build()
}

struct PeriodBalanceObjective<K: Send + Sync + 'static> {
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> Float + Send + Sync>,
    phantom_data: PhantomData<K>,
}

impl<K: Send + Sync + 'static> FeatureObjective for PeriodBalanceObjective<K> {
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
                if value.is_finite() { value } else { Cost::default() }
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct PeriodBalanceState<K: Send + Sync + 'static> {
    route_estimate_fn: Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>,
    solution_estimate_fn: Arc<dyn Fn(&SolutionContext) -> Float + Send + Sync>,
    phantom_data: PhantomData<K>,
}

impl<K: Send + Sync + 'static> FeatureState for PeriodBalanceState<K> {
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
