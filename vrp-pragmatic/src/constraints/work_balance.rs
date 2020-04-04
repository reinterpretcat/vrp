use crate::constraints::get_max_cost;
use std::cmp::Ordering::Less;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use vrp_core::models::problem::Job;
use vrp_core::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCostType};
use vrp_core::refinement::RefinementContext;
use vrp_core::utils::{get_mean, get_stdev};

/// Provides functionality needed to balance work across all routes.
pub struct WorkBalance {}

impl WorkBalance {
    /// Creates `WorkBalanceModule` which balances max load across all tours.
    pub fn new_load_balanced<Capacity>(
        threshold: Option<f64>,
        solution_tolerance: Option<f64>,
        route_tolerance: Option<f64>,
        load_func: Arc<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>,
    ) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn Objective + Send + Sync>)
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
            solution_tolerance,
            route_tolerance,
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
    pub fn new_activity_balanced(
        threshold: Option<usize>,
        solution_tolerance: Option<f64>,
        route_tolerance: Option<f64>,
    ) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn Objective + Send + Sync>) {
        let activity_balance = WorkBalanceObjectives {
            threshold: threshold.map(|t| t as f64),
            solution_tolerance,
            route_tolerance,
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
    pub fn new_distance_balanced(
        threshold: Option<f64>,
        solution_tolerance: Option<f64>,
        route_tolerance: Option<f64>,
    ) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn Objective + Send + Sync>) {
        Self::new_transport_balanced(threshold, solution_tolerance, route_tolerance, TOTAL_DISTANCE_KEY)
    }

    /// Creates `WorkBalanceModule` which balances travelled durations across all tours.
    pub fn new_duration_balanced(
        threshold: Option<f64>,
        solution_tolerance: Option<f64>,
        route_tolerance: Option<f64>,
    ) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn Objective + Send + Sync>) {
        Self::new_transport_balanced(threshold, solution_tolerance, route_tolerance, TOTAL_DURATION_KEY)
    }

    fn new_transport_balanced(
        threshold: Option<f64>,
        solution_tolerance: Option<f64>,
        route_tolerance: Option<f64>,
        state_key: i32,
    ) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn Objective + Send + Sync>) {
        let transport_balance = WorkBalanceObjectives {
            threshold,
            solution_tolerance,
            route_tolerance,
            value_func: Arc::new(move |rc| get_transport_value(rc, state_key)),
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
    solution_tolerance: Option<f64>,
    route_tolerance: Option<f64>,
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

        if ratio.is_normal() && ratio > self.route_tolerance.unwrap_or(0.) {
            ratio * get_max_cost(solution_ctx)
        } else {
            0.
        }
    }
}

impl Objective for WorkBalanceObjectives {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        let values = self.values_func.deref()(&insertion_ctx.solution);

        Box::new(MeasurableObjectiveCost::new_with_tolerance(get_stdev(&values), self.solution_tolerance.clone()))
    }

    fn is_goal_satisfied(&self, _: &mut RefinementContext, _: &InsertionContext) -> Option<bool> {
        None
    }
}

fn get_transport_value(route_ctx: &RouteContext, state_key: i32) -> f64 {
    assert!(state_key == TOTAL_DISTANCE_KEY || state_key == TOTAL_DURATION_KEY);

    route_ctx.state.get_route_state::<f64>(state_key).cloned().unwrap_or(0.)
}
