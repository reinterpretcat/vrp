use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;

const CURRENT_DEMAND_KEY: i32 = 11;
const MAX_FUTURE_DEMAND_KEY: i32 = 12;
const MAX_PAST_DEMAND_KEY: i32 = 13;

// TODO to avoid code duplication in generic type definition and implementation,
// TODO consider to use TODO trait aliases once they are stabilized (or macro?).

/// Checks whether vehicle can handle activity's demand.
/// Demand can be interpreted as vehicle capacity change after visiting specific activity.
pub struct DemandConstraintModule<Demand: Add + Sub + Send + Sync + 'static> {
    code: i32,
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    phantom: PhantomData<Demand>,
}

impl<Demand: Add<Output = Demand> + Sub<Output = Demand> + Send + Sync + 'static> DemandConstraintModule<Demand> {
    pub fn new(code: i32) -> Self {
        Self {
            code,
            state_keys: vec![CURRENT_DEMAND_KEY, MAX_FUTURE_DEMAND_KEY, MAX_PAST_DEMAND_KEY],
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(DemandHardRouteConstraint::<Demand> {
                    code,
                    phantom: PhantomData,
                })),
                ConstraintVariant::HardActivity(Arc::new(DemandHardActivityConstraint::<Demand> {
                    code,
                    phantom: PhantomData,
                })),
            ],
            phantom: PhantomData,
        }
    }
}

impl<Demand: Add<Output = Demand> + Sub<Output = Demand> + Send + Sync + 'static> ConstraintModule
    for DemandConstraintModule<Demand>
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

struct DemandHardRouteConstraint<Demand: Add + Sub + Send + Sync + 'static> {
    code: i32,
    phantom: PhantomData<Demand>,
}

impl<Demand: Add<Output = Demand> + Sub<Output = Demand> + Send + Sync + 'static> HardRouteConstraint
    for DemandHardRouteConstraint<Demand>
{
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation> {
        unimplemented!()
    }
}

struct DemandHardActivityConstraint<Demand: Add + Sub + Send + Sync + 'static> {
    code: i32,
    phantom: PhantomData<Demand>,
}

impl<Demand: Add<Output = Demand> + Sub<Output = Demand> + Send + Sync + 'static> HardActivityConstraint
    for DemandHardActivityConstraint<Demand>
{
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        unimplemented!()
    }
}
