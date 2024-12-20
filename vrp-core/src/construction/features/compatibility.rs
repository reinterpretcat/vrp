//! A compatibility feature provides the way to avoid assigning some jobs in the same tour.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/compatibility_test.rs"]
mod compatibility_test;

use super::*;

custom_dimension!(pub JobCompatibility typeof String);
custom_tour_state!(CurrentCompatibility typeof String);

/// Creates a compatibility feature as a hard constraint.
pub fn create_compatibility_feature(name: &str, code: ViolationCode) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(CompatibilityConstraint { code })
        .with_state(CompatibilityState {})
        .build()
}

struct CompatibilityConstraint {
    code: ViolationCode,
}

impl FeatureConstraint for CompatibilityConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => job.dimens().get_job_compatibility().and_then(|job_compat| {
                match route_ctx.state().get_current_compatibility() {
                    None => None,
                    Some(route_compat) if job_compat == route_compat => None,
                    _ => ConstraintViolation::fail(self.code),
                }
            }),
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match (source.dimens().get_job_compatibility(), candidate.dimens().get_job_compatibility()) {
            (None, None) => Ok(source),
            (Some(s_compat), Some(c_compat)) if s_compat == c_compat => Ok(source),
            _ => Err(self.code),
        }
    }
}

struct CompatibilityState {}

impl FeatureState for CompatibilityState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        if job.dimens().get_job_compatibility().is_some() {
            self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap())
        }
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let new_comp = get_route_compatibility(route_ctx);
        let current_compat = route_ctx.state().get_current_compatibility();

        match (new_comp, current_compat) {
            (None, None) => {}
            (None, Some(_)) => {
                route_ctx.state_mut().remove_current_compatibility();
            }
            (Some(value), None) | (Some(value), Some(_)) => route_ctx.state_mut().set_current_compatibility(value),
        }
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {}
}

fn get_route_compatibility(route_ctx: &RouteContext) -> Option<String> {
    route_ctx.route().tour.jobs().filter_map(|job| job.dimens().get_job_compatibility()).next().cloned()
}
