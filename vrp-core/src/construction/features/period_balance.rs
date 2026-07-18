//! Provides a feature to balance a work metric per employee across the whole planning period.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/period_balance_test.rs"]
mod period_balance_test;

use super::*;
use rosomaxa::algorithms::math::get_stdev_safe;
use std::collections::HashMap;
use std::marker::PhantomData;

type GroupKeyFn = Arc<dyn Fn(&Actor) -> Option<String> + Send + Sync>;
type RouteMetricFn = Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>;
type SolutionFitnessFn = Arc<dyn Fn(&SolutionContext) -> Float + Send + Sync>;

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
///
/// Both the solution fitness (standard deviation of those per-shift ratios) and the insertion
/// estimate (a route's own per-shift load) are expressed in the metric's unit and divided by the
/// same fixed `reference` via [`FeatureObjective::fitness_scale`]. This keeps the objective on a
/// comparable, dimensionless scale to the other self-normalizing objectives (compact-tour,
/// vehicle-distance), so a scalarizing multi-objective can weight it as a pure preference rather
/// than have its raw magnitude dominate. `reference` should be the ideal work per shift.
pub fn create_period_balanced_feature(
    name: &str,
    group_capacities: HashMap<String, usize>,
    group_key_fn: impl Fn(&Actor) -> Option<String> + Send + Sync + 'static,
    tour_metric_fn: impl Fn(&RouteContext) -> Float + Send + Sync + 'static,
    reference: Float,
) -> Result<Feature, GenericError> {
    struct PeriodBalanceKey;

    let group_capacities = Arc::new(group_capacities);
    let group_key_fn: GroupKeyFn = Arc::new(group_key_fn);
    let route_metric_fn: RouteMetricFn = Arc::new(tour_metric_fn);

    let solution_fitness_fn = Arc::new({
        let route_metric_fn = route_metric_fn.clone();
        let group_capacities = group_capacities.clone();
        let group_key_fn = group_key_fn.clone();
        move |solution_ctx: &SolutionContext| {
            let mut group_usage: HashMap<String, Float> = HashMap::new();

            for route_ctx in solution_ctx.routes.iter() {
                if let Some(key) = group_key_fn(&route_ctx.route().actor) {
                    *group_usage.entry(key).or_insert(0.) += route_metric_fn(route_ctx);
                }
            }

            let ratios = group_capacities
                .iter()
                .map(|(key, &capacity)| group_usage.get(key).copied().unwrap_or(0.) / capacity as Float)
                .collect::<Vec<_>>();

            get_stdev_safe(ratios.as_slice())
        }
    });

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(PeriodBalanceObjective::<PeriodBalanceKey> {
            route_metric_fn: route_metric_fn.clone(),
            solution_fitness_fn: solution_fitness_fn.clone(),
            group_capacities: group_capacities.clone(),
            group_key_fn: group_key_fn.clone(),
            reference,
            phantom_data: PhantomData,
        })
        .with_state(PeriodBalanceState::<PeriodBalanceKey> {
            route_metric_fn,
            solution_fitness_fn,
            phantom_data: PhantomData,
        })
        .build()
}

struct PeriodBalanceObjective<K: Send + Sync + 'static> {
    route_metric_fn: RouteMetricFn,
    solution_fitness_fn: SolutionFitnessFn,
    group_capacities: Arc<HashMap<String, usize>>,
    group_key_fn: GroupKeyFn,
    reference: Float,
    phantom_data: PhantomData<K>,
}

impl<K: Send + Sync + 'static> FeatureObjective for PeriodBalanceObjective<K> {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .state
            .get_value::<K, Float>()
            .cloned()
            .unwrap_or_else(|| (self.solution_fitness_fn)(&solution.solution))
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => {
                let metric = route_ctx
                    .state()
                    .get_tour_state::<K, Float>()
                    .cloned()
                    .unwrap_or_else(|| (self.route_metric_fn)(route_ctx));

                let capacity = (self.group_key_fn)(&route_ctx.route().actor)
                    .and_then(|key| self.group_capacities.get(&key).copied())
                    .unwrap_or(1)
                    .max(1);

                // Per-shift load in the metric's unit, matching the fitness unit so that dividing
                // by `reference` yields a dimensionless term. Guides insertions towards the groups
                // that are currently least loaded per shift.
                // NOTE: this value doesn't consider a route state after insertion of given job.
                let ratio = metric / capacity as Float;
                if ratio.is_finite() { ratio } else { Cost::default() }
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }

    fn fitness_scale(&self) -> Cost {
        self.reference
    }
}

struct PeriodBalanceState<K: Send + Sync + 'static> {
    route_metric_fn: RouteMetricFn,
    solution_fitness_fn: SolutionFitnessFn,
    phantom_data: PhantomData<K>,
}

impl<K: Send + Sync + 'static> FeatureState for PeriodBalanceState<K> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let value = (self.route_metric_fn)(route_ctx);

        route_ctx.state_mut().set_tour_state::<K, _>(value);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let value = (self.solution_fitness_fn)(solution_ctx);

        solution_ctx.state.set_value::<K, _>(value);
    }
}
