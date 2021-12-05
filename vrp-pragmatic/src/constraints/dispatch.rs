#[cfg(test)]
#[path = "../../tests/unit/constraints/dispatch_test.rs"]
mod dispatch_test;

use crate::constraints::*;
use crate::format::UNASSIGNABLE_ROUTE_KEY;
use std::iter::once;
use std::slice::Iter;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use vrp_core::models::problem::Job;

pub struct DispatchModule {
    code: i32,
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl DispatchModule {
    pub fn new(code: i32) -> Self {
        Self {
            code,
            conditional: ConditionalJobModule::new(create_job_transition()),
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(DispatchHardRouteConstraint { code })),
                ConstraintVariant::HardActivity(Arc::new(DispatchHardActivityConstraint { code })),
            ],
            keys: vec![],
        }
    }
}

impl ConstraintModule for DispatchModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.conditional.accept_route_state(ctx);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        // NOTE enforce propagation to locked
        ctx.locked.extend(ctx.routes.iter().flat_map(|route| route.route.tour.jobs().filter(is_dispatch_job)));

        self.conditional.accept_solution_state(ctx);

        // NOTE remove tour with dispatch only
        let registry = &mut ctx.registry;
        ctx.routes.retain(|rc| {
            let tour = &rc.route.tour;
            if tour.job_count() == 1 {
                let is_dispatch = tour.jobs().next().unwrap().as_single().map_or(false, is_dispatch_single);

                if is_dispatch {
                    registry.free_route(rc);
                    return false;
                }
            }

            true
        });
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, i32> {
        let any_is_dispatch = once(&source).chain(once(&candidate)).any(is_dispatch_job);

        if any_is_dispatch {
            Err(self.code)
        } else {
            Ok(source)
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct DispatchHardActivityConstraint {
    code: i32,
}

/// Locks break jobs to specific vehicles.
struct DispatchHardRouteConstraint {
    code: i32,
}

impl HardRouteConstraint for DispatchHardRouteConstraint {
    fn evaluate_job(
        &self,
        _: &SolutionContext,
        route_ctx: &RouteContext,
        job: &Job,
    ) -> Option<RouteConstraintViolation> {
        if let Some(single) = job.as_single() {
            if is_dispatch_single(single) {
                return if !is_single_belongs_to_route(route_ctx, single) {
                    Some(RouteConstraintViolation { code: self.code })
                } else {
                    None
                };
            } else if is_unassignable_route(route_ctx) {
                return Some(RouteConstraintViolation { code: self.code });
            }
        }

        None
    }
}

impl HardActivityConstraint for DispatchHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        if is_dispatch_activity(&activity_ctx.next) {
            Some(ActivityConstraintViolation { code: self.code, stopped: false })
        } else {
            None
        }
    }
}

fn create_job_transition() -> Box<dyn JobContextTransition + Send + Sync> {
    Box::new(ConcreteJobContextTransition {
        remove_required: |_, _, job| is_dispatch_job(job),
        promote_required: |_, _, _| false,
        remove_locked: |_, _, _| false,
        promote_locked: |_, _, job| is_dispatch_job(job),
    })
}

fn is_dispatch_job(job: &Job) -> bool {
    job.as_single().and_then(|single| single.dimens.get_value::<String>("type")).map_or(false, |t| t == "dispatch")
}

fn is_dispatch_single(single: &Arc<Single>) -> bool {
    single.dimens.get_value::<String>("type").map_or(false, |t| t == "dispatch")
}

fn is_dispatch_activity(activity: &Option<&Activity>) -> bool {
    activity.and_then(|activity| activity.job.as_ref()).map_or(false, is_dispatch_single)
}

fn is_unassignable_route(route_ctx: &RouteContext) -> bool {
    route_ctx.state.get_route_state::<bool>(UNASSIGNABLE_ROUTE_KEY).cloned().unwrap_or(false)
}
