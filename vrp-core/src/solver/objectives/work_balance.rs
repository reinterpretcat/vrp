use crate::construction::constraints::*;
use crate::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use crate::models::common::Objective;
use crate::models::problem::{Job, TargetConstraint, TargetObjective};
use crate::utils::{compare_floats, get_mean, get_stdev};
use std::cmp::Ordering;
use std::cmp::Ordering::Less;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;

/// Provides functionality needed to balance work across all routes.
pub struct WorkBalance {}

impl WorkBalance {
    /// Creates `WorkBalanceModule` which balances max load across all tours.
    pub fn new_load_balanced<Capacity>(
        threshold: Option<f64>,
        load_func: Arc<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>,
    ) -> (TargetConstraint, TargetObjective)
    where
        Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static,
    {
        let default_capacity = Capacity::default();
        let default_intervals = vec![(0_usize, 0_usize)];

        let get_load_ratio = Arc::new(move |ctx: &RouteContext| {
            let capacity = ctx.route.actor.vehicle.dimens.get_capacity().unwrap();
            let intervals =
                ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS).unwrap_or(&default_intervals);

            intervals
                .iter()
                .map(|(start, _)| ctx.route.tour.get(*start).unwrap())
                .map(|activity| {
                    ctx.state
                        .get_activity_state::<Capacity>(MAX_FUTURE_CAPACITY_KEY, activity)
                        .unwrap_or_else(|| &default_capacity)
                })
                .map(|max_load| load_func.deref()(max_load, capacity))
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
                .unwrap_or(0_f64)
        });

        let load_balance = WorkBalanceObjectives {
            threshold,
            value_func: Arc::new({
                let get_load_ratio = get_load_ratio.clone();
                move |rc| get_load_ratio(rc)
            }),
            values_func: Arc::new({
                let get_load_ratio = get_load_ratio.clone();
                move |ctx| ctx.routes.iter().map(|rc| get_load_ratio(rc)).collect()
            }),
        };

        (
            Box::new(WorkBalanceModule {
                constraints: vec![ConstraintVariant::SoftRoute(Arc::new(load_balance.clone()))],
                keys: vec![],
            }),
            Box::new(load_balance),
        )
    }

    /// Creates `WorkBalanceModule` which balances activities across all tours.
    pub fn new_activity_balanced(threshold: Option<usize>) -> (TargetConstraint, TargetObjective) {
        let activity_balance = WorkBalanceObjectives {
            threshold: threshold.map(|t| t as f64),
            value_func: Arc::new(|rc| rc.route.tour.activity_count() as f64),
            values_func: Arc::new(|ctx| ctx.routes.iter().map(|rc| rc.route.tour.activity_count() as f64).collect()),
        };

        (
            Box::new(WorkBalanceModule {
                constraints: vec![ConstraintVariant::SoftRoute(Arc::new(activity_balance.clone()))],
                keys: vec![],
            }),
            Box::new(activity_balance),
        )
    }

    /// Creates `WorkBalanceModule` which balances travelled distances across all tours.
    pub fn new_distance_balanced(threshold: Option<f64>) -> (TargetConstraint, TargetObjective) {
        Self::new_transport_balanced(threshold, TOTAL_DISTANCE_KEY)
    }

    /// Creates `WorkBalanceModule` which balances travelled durations across all tours.
    pub fn new_duration_balanced(threshold: Option<f64>) -> (TargetConstraint, TargetObjective) {
        Self::new_transport_balanced(threshold, TOTAL_DURATION_KEY)
    }

    fn new_transport_balanced(threshold: Option<f64>, state_key: i32) -> (TargetConstraint, TargetObjective) {
        let transport_balance = WorkBalanceObjectives {
            threshold,
            value_func: Arc::new(move |rc| {
                debug_assert!(state_key == TOTAL_DISTANCE_KEY || state_key == TOTAL_DURATION_KEY);
                rc.state.get_route_state::<f64>(state_key).cloned().unwrap_or(0.)
            }),
            values_func: Arc::new(move |ctx| {
                ctx.routes.iter().map(|rc| rc.state.get_route_state::<f64>(state_key).cloned().unwrap_or(0.)).collect()
            }),
        };

        (
            Box::new(WorkBalanceModule {
                constraints: vec![ConstraintVariant::SoftRoute(Arc::new(transport_balance.clone()))],
                keys: vec![],
            }),
            Box::new(transport_balance),
        )
    }
}

/// A module which provides way to balance work across all tours.
struct WorkBalanceModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl ConstraintModule for WorkBalanceModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

#[derive(Clone)]
struct WorkBalanceObjectives {
    threshold: Option<f64>,
    value_func: Arc<dyn Fn(&RouteContext) -> f64 + Send + Sync>,
    values_func: Arc<dyn Fn(&SolutionContext) -> Vec<f64> + Send + Sync>,
}

impl SoftRouteConstraint for WorkBalanceObjectives {
    fn estimate_job(&self, solution_ctx: &SolutionContext, route_ctx: &RouteContext, _job: &Job) -> f64 {
        let value = self.value_func.deref()(route_ctx);

        if self.threshold.map_or(false, |threshold| value < threshold) {
            return 0.;
        }

        let values = self.values_func.deref()(solution_ctx);

        let mean = get_mean(&values);
        let ratio = (value - mean).max(0.) / mean;

        if ratio.is_normal() {
            ratio * solution_ctx.get_max_cost()
        } else {
            0.
        }
    }
}

impl Objective for WorkBalanceObjectives {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        let fitness_a = self.fitness(a);
        let fitness_b = self.fitness(b);

        compare_floats(fitness_a, fitness_b)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        let fitness_a = self.fitness(a);
        let fitness_b = self.fitness(b);

        fitness_a - fitness_b
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        let value = get_stdev(&self.values_func.deref()(&solution.solution));

        if value.is_nan() {
            1.
        } else {
            value
        }
    }
}
