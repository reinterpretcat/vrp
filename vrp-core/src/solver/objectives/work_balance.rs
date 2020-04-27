use crate::construction::constraints::*;
use crate::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use crate::models::common::Objective;
use crate::models::problem::{Job, TargetConstraint, TargetObjective};
use crate::solver::objectives::{
    BALANCE_ACTIVITY_KEY, BALANCE_DISTANCE_KEY, BALANCE_DURATION_KEY, BALANCE_MAX_LOAD_KEY,
};
use crate::utils::{compare_floats, get_cv, get_mean};
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Less};
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;

/// Provides functionality needed to balance work across all routes.
pub struct WorkBalance {}

impl WorkBalance {
    /// Creates `WorkBalanceModule` which balances max load across all tours.
    pub fn new_load_balanced<Capacity>(
        threshold: Option<f64>,
        tolerance: Option<f64>,
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
                ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY).unwrap_or(&default_intervals);

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

        let value_func = Arc::new({
            let get_load_ratio = get_load_ratio.clone();
            move |rc: &RouteContext| get_load_ratio(rc)
        });
        let values_func = Arc::new({
            let get_load_ratio = get_load_ratio.clone();
            move |ctx: &SolutionContext| ctx.routes.iter().map(|rc| get_load_ratio(rc)).collect()
        });

        let objective = WorkBalanceObjectives {
            threshold,
            tolerance,
            state_key: BALANCE_MAX_LOAD_KEY,
            value_func: value_func.clone(),
            values_func: values_func.clone(),
        };

        let constraint = WorkBalanceModule {
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(objective.clone()))],
            value_func,
            values_func,
            state_key: BALANCE_MAX_LOAD_KEY,
            keys: vec![BALANCE_MAX_LOAD_KEY],
        };

        (Box::new(constraint), Box::new(objective))
    }

    /// Creates `WorkBalanceModule` which balances activities across all tours.
    pub fn new_activity_balanced(
        threshold: Option<f64>,
        tolerance: Option<f64>,
    ) -> (TargetConstraint, TargetObjective) {
        let value_func = Arc::new(|rc: &RouteContext| rc.route.tour.activity_count() as f64);
        let values_func = Arc::new(|ctx: &SolutionContext| {
            ctx.routes.iter().map(|rc| rc.route.tour.activity_count() as f64).collect()
        });

        let objective = WorkBalanceObjectives {
            threshold,
            tolerance,
            state_key: BALANCE_ACTIVITY_KEY,
            value_func: value_func.clone(),
            values_func: values_func.clone(),
        };

        let constraint = WorkBalanceModule {
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(objective.clone()))],
            state_key: BALANCE_ACTIVITY_KEY,
            value_func: value_func.clone(),
            values_func: values_func.clone(),
            keys: vec![BALANCE_ACTIVITY_KEY],
        };

        (Box::new(constraint), Box::new(objective))
    }

    /// Creates `WorkBalanceModule` which balances travelled distances across all tours.
    pub fn new_distance_balanced(
        threshold: Option<f64>,
        tolerance: Option<f64>,
    ) -> (TargetConstraint, TargetObjective) {
        Self::new_transport_balanced(threshold, tolerance, TOTAL_DISTANCE_KEY, BALANCE_DISTANCE_KEY)
    }

    /// Creates `WorkBalanceModule` which balances travelled durations across all tours.
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
        let value_func = Arc::new(move |rc: &RouteContext| {
            debug_assert!(transport_state_key == TOTAL_DISTANCE_KEY || transport_state_key == TOTAL_DURATION_KEY);
            rc.state.get_route_state::<f64>(transport_state_key).cloned().unwrap_or(0.)
        });

        let values_func = Arc::new(move |ctx: &SolutionContext| {
            ctx.routes
                .iter()
                .map(|rc| rc.state.get_route_state::<f64>(transport_state_key).cloned().unwrap_or(0.))
                .collect()
        });

        let objective = WorkBalanceObjectives {
            threshold,
            tolerance,
            state_key: memory_state_key,
            value_func: value_func.clone(),
            values_func: values_func.clone(),
        };

        let constraint = WorkBalanceModule {
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(objective.clone()))],
            value_func,
            values_func,
            state_key: memory_state_key,
            keys: vec![memory_state_key],
        };

        (Box::new(constraint), Box::new(objective))
    }
}

/// A module which provides way to balance work across all tours.
struct WorkBalanceModule {
    constraints: Vec<ConstraintVariant>,
    value_func: Arc<dyn Fn(&RouteContext) -> f64 + Send + Sync>,
    values_func: Arc<dyn Fn(&SolutionContext) -> Vec<f64> + Send + Sync>,
    state_key: i32,
    keys: Vec<i32>,
}

impl ConstraintModule for WorkBalanceModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Job) {}

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        let value = self.value_func.deref()(ctx);

        ctx.state_mut().put_route_state(self.state_key, value);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        let values = self.values_func.deref()(ctx);
        let cv = get_cv(&values);

        ctx.state.insert(self.state_key, Arc::new(cv));
    }

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
    tolerance: Option<f64>,
    state_key: i32,
    value_func: Arc<dyn Fn(&RouteContext) -> f64 + Send + Sync>,
    values_func: Arc<dyn Fn(&SolutionContext) -> Vec<f64> + Send + Sync>,
}

impl SoftRouteConstraint for WorkBalanceObjectives {
    fn estimate_job(&self, solution_ctx: &SolutionContext, route_ctx: &RouteContext, _job: &Job) -> f64 {
        let value = route_ctx
            .state
            .get_route_state::<f64>(self.state_key)
            .cloned()
            .unwrap_or_else(|| self.value_func.deref()(route_ctx));

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

        if self.tolerance.map_or(false, |tolerance| (fitness_a - fitness_b).abs() < tolerance) {
            Equal
        } else {
            compare_floats(fitness_a, fitness_b)
        }
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        let fitness_a = self.fitness(a);
        let fitness_b = self.fitness(b);

        fitness_a - fitness_b
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        let value = solution
            .solution
            .state
            .get(&self.state_key)
            .and_then(|s| s.downcast_ref::<f64>())
            .cloned()
            .unwrap_or_else(|| get_cv(&self.values_func.deref()(&solution.solution)));

        if value.is_nan() {
            1.
        } else {
            value
        }
    }
}
