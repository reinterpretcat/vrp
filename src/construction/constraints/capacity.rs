use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::Dimensions;
use crate::models::problem::Job;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;

const CURRENT_CAPACITY_KEY: i32 = 11;
const MAX_FUTURE_CAPACITY_KEY: i32 = 12;
const MAX_PAST_CAPACITY_KEY: i32 = 13;

// TODO to avoid code duplication in generic type definition and implementation,
// TODO consider to use TODO trait aliases once they are stabilized (or macro?).

/// Checks whether vehicle can handle activity's demand.
/// Capacity can be interpreted as vehicle capacity change after visiting specific activity.
pub struct CapacityConstraintModule<Capacity: Add + Sub + PartialOrd + Send + Sync + 'static> {
    code: i32,
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + PartialOrd + Send + Sync + 'static>
    CapacityConstraintModule<Capacity>
{
    pub fn new(code: i32) -> Self {
        Self {
            code,
            state_keys: vec![CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY],
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(CapacityHardRouteConstraint::<Capacity> {
                    code,
                    phantom: PhantomData,
                })),
                ConstraintVariant::HardActivity(Arc::new(CapacityHardActivityConstraint::<Capacity> {
                    code,
                    phantom: PhantomData,
                })),
            ],
            phantom: PhantomData,
        }
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + PartialOrd + Send + Sync + 'static> ConstraintModule
    for CapacityConstraintModule<Capacity>
{
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

pub trait CapacityDimension<Capacity: Add + Sub + PartialOrd + Send + Sync + 'static> {
    fn set_capacity(&mut self, demand: Capacity) -> &mut Self;
    fn get_capacity(&self) -> Option<&Capacity>;
}

const CAPACITY_DIMENSION_KEY: &str = "cpc";

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + PartialOrd + Send + Sync + 'static>
    CapacityDimension<Capacity> for Dimensions
{
    fn set_capacity(&mut self, demand: Capacity) -> &mut Self {
        self.insert(CAPACITY_DIMENSION_KEY.to_string(), Box::new(demand));
        self
    }

    fn get_capacity(&self) -> Option<&Capacity> {
        self.get(CAPACITY_DIMENSION_KEY).and_then(|any| any.downcast_ref::<Capacity>())
    }
}

struct CapacityHardRouteConstraint<Capacity: Add + Sub + PartialOrd + Send + Sync + 'static> {
    code: i32,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + PartialOrd + Send + Sync + 'static> HardRouteConstraint
    for CapacityHardRouteConstraint<Capacity>
{
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation> {
        unimplemented!()
    }
}

struct CapacityHardActivityConstraint<Capacity: Add + Sub + PartialOrd + Send + Sync + 'static> {
    code: i32,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + PartialOrd + Send + Sync + 'static>
    HardActivityConstraint for CapacityHardActivityConstraint<Capacity>
{
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        unimplemented!()
    }
}
