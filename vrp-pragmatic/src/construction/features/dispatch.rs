//! A feature to model dispatch activity at tour start.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/dispatch_test.rs"]
mod dispatch_test;

use super::*;
use crate::construction::enablers::is_single_belongs_to_route;
use crate::construction::enablers::JobTie;
use std::iter::once;
use vrp_core::construction::enablers::*;
use vrp_core::models::solution::Activity;

/// Creates a dispatch feature as a hard constraint.
pub fn create_dispatch_feature(name: &str, code: ViolationCode) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(DispatchConstraint { code })
        .with_state(DispatchState {
            context_transition: Box::new(ConcreteJobContextTransition {
                remove_required: |_, _, job| is_dispatch_job(job),
                promote_required: |_, _, _| false,
                remove_locked: |_, _, _| false,
                promote_locked: |_, _, job| is_dispatch_job(job),
            }),
            state_keys: vec![],
        })
        .build()
}

struct DispatchConstraint {
    code: ViolationCode,
}

impl DispatchConstraint {
    fn evaluate_route(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        if let Some(single) = job.as_single() {
            if is_dispatch_single(single) {
                return if !is_single_belongs_to_route(route_ctx.route(), single) {
                    ConstraintViolation::fail(self.code)
                } else {
                    None
                };
            } else if route_ctx.state().has_flag(state_flags::UNASSIGNABLE) {
                return ConstraintViolation::fail(self.code);
            }
        }

        None
    }

    fn evaluate_activity(&self, activity_ctx: &ActivityContext) -> Option<ConstraintViolation> {
        if is_dispatch_activity(&activity_ctx.next) {
            ConstraintViolation::skip(self.code)
        } else {
            None
        }
    }
}

impl FeatureConstraint for DispatchConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_route(route_ctx, job),
            MoveContext::Activity { activity_ctx, .. } => self.evaluate_activity(activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        let any_is_dispatch = once(&source).chain(once(&candidate)).any(is_dispatch_job);

        if any_is_dispatch {
            Err(self.code)
        } else {
            Ok(source)
        }
    }
}

struct DispatchState {
    context_transition: Box<dyn JobContextTransition + Send + Sync>,
    state_keys: Vec<StateKey>,
}

impl FeatureState for DispatchState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        process_conditional_jobs(solution_ctx, Some(route_index), self.context_transition.as_ref());
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        // NOTE enforce propagation to locked
        solution_ctx.locked.extend(
            solution_ctx.routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs().filter(is_dispatch_job)),
        );

        process_conditional_jobs(solution_ctx, None, self.context_transition.as_ref());

        // NOTE remove tour with dispatch only
        solution_ctx.keep_routes(&|route_ctx| {
            let tour = &route_ctx.route().tour;
            if tour.job_count() == 1 {
                !tour.jobs().next().unwrap().as_single().map_or(false, is_dispatch_single)
            } else {
                true
            }
        });
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}

fn is_dispatch_job(job: &Job) -> bool {
    job.as_single().and_then(|single| single.dimens.get_job_type()).map_or(false, |t| t == "dispatch")
}

fn is_dispatch_single(single: &Arc<Single>) -> bool {
    single.dimens.get_job_type().map_or(false, |t| t == "dispatch")
}

fn is_dispatch_activity(activity: &Option<&Activity>) -> bool {
    activity.and_then(|activity| activity.job.as_ref()).map_or(false, is_dispatch_single)
}
