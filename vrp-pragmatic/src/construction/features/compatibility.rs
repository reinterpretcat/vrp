//! A compatibility feature provides the way to avoid assigning some jobs in the same tour.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/compatibility_test.rs"]
mod compatibility_test;

use crate::extensions::JobTie;
use std::slice::Iter;
use vrp_core::construction::features::*;
use vrp_core::construction::heuristics::*;
use vrp_core::models::problem::Job;

/// Creates a compatibility feature as hard constraint.
pub fn create_compatibility_constraint(code: ViolationCode, state_key: StateKey) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_constraint(CompatibilityConstraint { code, state_key })
        .with_state(CompatibilityState { state_key, keys: vec![state_key] })
        .build()
}

struct CompatibilityConstraint {
    code: ViolationCode,
    state_key: StateKey,
}

impl FeatureConstraint for CompatibilityConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => job.dimens().get_job_compatibility().and_then(|job_compat| {
                match route_ctx.state.get_route_state::<Option<String>>(self.state_key) {
                    None | Some(None) => None,
                    Some(Some(route_compat)) if job_compat == route_compat => None,
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

struct CompatibilityState {
    state_key: StateKey,
    keys: Vec<StateKey>,
}

impl FeatureState for CompatibilityState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        if job.dimens().get_job_compatibility().is_some() {
            self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap())
        }
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let new_comp = get_route_compatibility(route_ctx);
        let current_compat = route_ctx.state.get_route_state::<Option<String>>(self.state_key);

        match (new_comp, current_compat) {
            (None, None) => {}
            (None, Some(_)) => {
                route_ctx.state_mut().put_route_state::<Option<String>>(self.state_key, None);
            }
            (value, None) | (value, Some(_)) => route_ctx.state_mut().put_route_state(self.state_key, value),
        }
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<StateKey> {
        self.keys.iter()
    }
}

fn get_route_compatibility(route_ctx: &RouteContext) -> Option<String> {
    route_ctx.route.tour.jobs().filter_map(|job| job.dimens().get_job_compatibility().cloned()).next()
}
