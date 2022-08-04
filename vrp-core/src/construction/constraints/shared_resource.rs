use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::problem::{Job, Single};
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;

// TODO consider interval demand and capacity inside trivial multi trip removal logic

/// Represents a shared unique resource.
pub trait SharedResource: Add + Sub + Copy + Ord + Sized + Send + Sync + Default + 'static {}

/// Provides way to define and use shared across multiple routes resource.
pub struct SharedResourceModule<T>
where
    T: SharedResource + Add<Output = T>,
{
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    interval_fn: Arc<dyn Fn(&RouteContext) -> &[(usize, usize)] + Send + Sync>,
    resource_demand_fn: Arc<dyn Fn(&Single) -> T + Send + Sync>,
    resource_capacity_fn: Arc<dyn Fn(&RouteContext, usize) -> Option<(T, usize)> + Send + Sync>,
}

impl<T: SharedResource + Add<Output = T>> SharedResourceModule<T> {
    /// Creates a new instance of `SharedResourceModule`.
    pub fn new(
        code: i32,
        interval_fn: Arc<dyn Fn(&RouteContext) -> &[(usize, usize)] + Send + Sync>,
        resource_capacity_fn: Arc<dyn Fn(&RouteContext, usize) -> Option<(T, usize)> + Send + Sync>,
        resource_demand_fn: Arc<dyn Fn(&Single) -> T + Send + Sync>,
        resource_capacity_key: i32,
    ) -> Self {
        Self {
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(SharedResourceHardActivityConstraint {
                code,
                interval_fn: interval_fn.clone(),
                resource_demand_fn: resource_demand_fn.clone(),
                resource_capacity_key,
            }))],
            interval_fn,
            resource_demand_fn,
            state_keys: vec![resource_capacity_key],
            resource_capacity_fn,
        }
    }

    fn update_resources_consumption(&self, solution_ctx: &mut SolutionContext) {
        // get total demand for each shared resource
        let total_demand = solution_ctx.routes.iter().fold(HashMap::<usize, T>::default(), |acc, route_ctx| {
            self.interval_fn.deref()(route_ctx).iter().fold(acc, |mut acc, &(start_idx, end_idx)| {
                // get total resource demand for given interval
                let resource_demand_with_id =
                    self.resource_capacity_fn.deref()(route_ctx, start_idx).map(|(capacity, id)| {
                        let resource_demand = (start_idx..=end_idx)
                            .filter_map(|idx| route_ctx.route.tour.get(idx))
                            .filter_map(|activity| activity.job.as_ref())
                            .fold(T::default(), |acc, job| acc + self.resource_demand_fn.deref()(job));
                        (resource_demand, id)
                    });

                if let Some((resource_demand, id)) = resource_demand_with_id {
                    let entry = acc.entry(id).or_insert_with(T::default);
                    *entry = *entry + resource_demand;
                }

                acc
            })
        });

        // second pass to store amount of available resources
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            self.interval_fn.deref()(route_ctx).iter().for_each(|&(start_idx, end_idx)| {
                // TODO
            });
        });
    }
}

impl<T: SharedResource + Add<Output = T>> ConstraintModule for SharedResourceModule<T> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
        self.update_resources_consumption(solution_ctx);
    }

    fn accept_route_state(&self, _route_ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        self.update_resources_consumption(solution_ctx);
    }

    fn merge(&self, source: Job, _candidate: Job) -> Result<Job, i32> {
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct SharedResourceHardActivityConstraint<T: SharedResource> {
    code: i32,
    interval_fn: Arc<dyn Fn(&RouteContext) -> &[(usize, usize)] + Send + Sync>,
    resource_demand_fn: Arc<dyn Fn(&Single) -> T + Send + Sync>,
    resource_capacity_key: i32,
}

impl<T: SharedResource> HardActivityConstraint for SharedResourceHardActivityConstraint<T> {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        self.interval_fn.deref()(route_ctx)
            .iter()
            // TODO what's about OVRP?
            .find(|(_, end_idx)| activity_ctx.index <= *end_idx)
            .and_then(|&(start_idx, _)| {
                route_ctx
                    .state
                    .get_activity_state::<T>(
                        self.resource_capacity_key,
                        route_ctx.route.tour.get(start_idx).expect("cannot get resource activity"),
                    )
                    .and_then(|resource_available| {
                        let resource_demand = route_ctx
                            .route
                            .tour
                            .get(activity_ctx.index)
                            .expect("cannot get demand activity")
                            .job
                            .as_ref()
                            .map_or(T::default(), |job| self.resource_demand_fn.deref()(job.as_ref()));

                        if resource_available < &resource_demand {
                            Some(ActivityConstraintViolation { code: self.code, stopped: false })
                        } else {
                            None
                        }
                    })
            })
    }
}
