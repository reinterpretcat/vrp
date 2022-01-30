use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use vrp_core::models::problem::{Job, TransportCost, TravelTime};

pub struct ReachableModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl ReachableModule {
    pub fn new(transport: Arc<dyn TransportCost + Send + Sync>, code: i32) -> Self {
        Self {
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(ReachableHardActivityConstraint {
                transport,
                code,
            }))],
            keys: vec![],
        }
    }
}

impl ConstraintModule for ReachableModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_index: usize, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn merge(&self, source: Job, _candidate: Job) -> Result<Job, i32> {
        // NOTE it is responsibility of the caller to check whether jobs are reachable
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct ReachableHardActivityConstraint {
    transport: Arc<dyn TransportCost + Send + Sync>,
    code: i32,
}

impl HardActivityConstraint for ReachableHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let actor = &route_ctx.route.actor;

        let prev_to_target = self.transport.distance(
            actor,
            prev.place.location,
            target.place.location,
            TravelTime::Departure(prev.schedule.departure),
        );

        if prev_to_target < 0. {
            return Some(ActivityConstraintViolation { code: self.code, stopped: false });
        }

        if let Some(next) = next {
            let target_to_next = self.transport.distance(
                actor,
                target.place.location,
                next.place.location,
                TravelTime::Departure(target.schedule.departure),
            );
            if target_to_next < 0. {
                return Some(ActivityConstraintViolation { code: self.code, stopped: false });
            }
        }

        None
    }
}
