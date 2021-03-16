use crate::algorithms::statistics::get_cv;
use crate::construction::constraints::*;
use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::common::{CapacityDimension, Load};
use crate::models::problem::{TargetConstraint, TargetObjective};
use crate::solver::objectives::GenericValue;
use crate::solver::*;
use std::cmp::Ordering;
use std::ops::{Add, Deref, Sub};
use std::sync::Arc;

/// A type which provides functionality needed to balance work across all routes.
pub struct WorkBalance {}

impl WorkBalance {
    /// Creates _(constraint, objective)_  type pair which balances max load across all tours.
    pub fn new_load_balanced<T: Load + Add<Output = T> + Sub<Output = T> + 'static>(
        threshold: Option<f64>,
        tolerance: Option<f64>,
        load_func: Arc<dyn Fn(&T, &T) -> f64 + Send + Sync>,
    ) -> (TargetConstraint, TargetObjective) {
        let default_capacity = T::default();
        let default_intervals = vec![(0_usize, 0_usize)];

        let get_load_ratio = Arc::new(move |ctx: &RouteContext| {
            let capacity = ctx.route.actor.vehicle.dimens.get_capacity().unwrap();
            let intervals =
                ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY).unwrap_or(&default_intervals);

            intervals
                .iter()
                .map(|(start, _)| ctx.route.tour.get(*start).unwrap())
                .map(|activity| {
                    ctx.state.get_activity_state::<T>(MAX_FUTURE_CAPACITY_KEY, activity).unwrap_or(&default_capacity)
                })
                .map(|max_load| load_func.deref()(max_load, capacity))
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
                .unwrap_or(0_f64)
        });

        GenericValue::new(
            threshold,
            tolerance,
            Arc::new({
                let get_load_ratio = get_load_ratio.clone();
                move |rc: &RouteContext| get_load_ratio(rc)
            }),
            Arc::new({
                let get_load_ratio = get_load_ratio.clone();
                move |ctx: &SolutionContext| {
                    get_cv(ctx.routes.iter().map(|rc| get_load_ratio(rc)).collect::<Vec<_>>().as_slice())
                }
            }),
            Arc::new(|solution_ctx, _, _, value| value * solution_ctx.get_max_cost()),
            BALANCE_MAX_LOAD_KEY,
        )
    }

    /// Creates _(constraint, objective)_  type pair which balances activities across all tours.
    pub fn new_activity_balanced(
        threshold: Option<f64>,
        tolerance: Option<f64>,
    ) -> (TargetConstraint, TargetObjective) {
        GenericValue::new(
            threshold,
            tolerance,
            Arc::new(|rc: &RouteContext| rc.route.tour.activity_count() as f64),
            Arc::new(|ctx: &SolutionContext| {
                get_cv(ctx.routes.iter().map(|rc| rc.route.tour.activity_count() as f64).collect::<Vec<_>>().as_slice())
            }),
            Arc::new(|solution_ctx, _, _, value| value * solution_ctx.get_max_cost()),
            BALANCE_ACTIVITY_KEY,
        )
    }

    /// Creates _(constraint, objective)_  type pair which balances travelled distances across all tours.
    pub fn new_distance_balanced(
        threshold: Option<f64>,
        tolerance: Option<f64>,
    ) -> (TargetConstraint, TargetObjective) {
        Self::new_transport_balanced(threshold, tolerance, TOTAL_DISTANCE_KEY, BALANCE_DISTANCE_KEY)
    }

    /// Creates _(constraint, objective)_  type pair which balances travelled durations across all tours.
    pub fn new_duration_balanced(
        threshold: Option<f64>,
        tolerance: Option<f64>,
    ) -> (TargetConstraint, TargetObjective) {
        Self::new_transport_balanced(threshold, tolerance, TOTAL_DURATION_KEY, BALANCE_DURATION_KEY)
    }

    fn new_transport_balanced(
        threshold: Option<f64>,
        tolerance: Option<f64>,
        transport_state_key: i32,
        memory_state_key: i32,
    ) -> (TargetConstraint, TargetObjective) {
        GenericValue::new(
            threshold,
            tolerance,
            Arc::new(move |rc: &RouteContext| {
                debug_assert!(transport_state_key == TOTAL_DISTANCE_KEY || transport_state_key == TOTAL_DURATION_KEY);
                rc.state.get_route_state::<f64>(transport_state_key).cloned().unwrap_or(0.)
            }),
            Arc::new(move |ctx: &SolutionContext| {
                get_cv(
                    ctx.routes
                        .iter()
                        .map(|rc| rc.state.get_route_state::<f64>(transport_state_key).cloned().unwrap_or(0.))
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
            }),
            Arc::new(|solution_ctx, _, _, value| value * solution_ctx.get_max_cost()),
            memory_state_key,
        )
    }
}
