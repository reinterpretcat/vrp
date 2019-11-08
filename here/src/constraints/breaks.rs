use core::construction::constraints::*;
use core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use core::models::common::{IdDimension, ValueDimension};
use core::models::problem::{Job, Single};
use std::slice::Iter;
use std::sync::Arc;

pub struct BreakModule {
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

impl BreakModule {
    pub fn new(code: i32) -> Self {
        Self {
            conditional: ConditionalJobModule::new(Box::new(|ctx, job| is_required_job(ctx, job))),
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(BreakHardActivityConstraint { code }))],
        }
    }
}

impl ConstraintModule for BreakModule {
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.conditional.accept_route_state(ctx);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional.accept_solution_state(ctx);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.conditional.state_keys()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct BreakHardActivityConstraint {
    code: i32,
}

impl HardActivityConstraint for BreakHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let is_break =
            activity_ctx.target.job.as_ref().and_then(|job| Some(job.as_single())).map_or(false, |job| is_break(&job));

        // avoid assigning break right after departure
        if is_break && activity_ctx.prev.job.is_none() {
            Some(ActivityConstraintViolation { code: self.code, stopped: false })
        } else {
            None
        }
    }
}

/// Mark job as ignored only if it has break type and vehicle id is not present in routes
fn is_required_job(ctx: &SolutionContext, job: &Arc<Job>) -> bool {
    match job.as_ref() {
        Job::Single(job) => {
            if is_break(job) {
                let vehicle_id = job.dimens.get_value::<String>("vehicle_id").unwrap().clone();
                ctx.routes
                    .iter()
                    .any(move |rc| *rc.route.read().unwrap().actor.vehicle.dimens.get_id().unwrap() == vehicle_id)
            } else {
                true
            }
        }
        Job::Multi(_) => true,
    }
}

fn is_break(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "break")
}
