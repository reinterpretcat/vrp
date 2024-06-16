//! A feature to model group of jobs.

use super::*;
use std::collections::HashSet;

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/groups_test.rs"]
mod groups_test;

/// Provides a way to work with a job's groups.
pub trait GroupAspects: Clone + Send + Sync {
    /// Returns job group if present.
    fn get_job_group<'a>(&self, job: &'a Job) -> Option<&'a String>;

    /// Gets a dedicated state key.
    fn get_state_key(&self) -> StateKey;

    /// Gets a violation code.
    fn get_violation_code(&self) -> ViolationCode;
}

/// Creates a job group feature as a hard constraint.
pub fn create_group_feature<T: GroupAspects + 'static>(
    name: &str,
    total_jobs: usize,
    aspects: T,
) -> Result<Feature, GenericError> {
    let state_key = aspects.get_state_key();
    let code = aspects.get_violation_code();

    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(GroupConstraint { total_jobs, code, state_key, aspects: aspects.clone() })
        .with_state(GroupState { state_key, state_keys: vec![state_key], aspects })
        .build()
}

struct GroupConstraint<T: GroupAspects> {
    total_jobs: usize,
    code: ViolationCode,
    state_key: StateKey,
    aspects: T,
}

impl<T: GroupAspects> FeatureConstraint for GroupConstraint<T> {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => self.aspects.get_job_group(job).and_then(|group| {
                let is_partial_problem = solution_ctx.get_jobs_amount() != self.total_jobs;
                if is_partial_problem {
                    return ConstraintViolation::fail(self.code);
                }

                let other_route = solution_ctx
                    .routes
                    .iter()
                    .filter(|rc| rc.route().actor != route_ctx.route().actor)
                    .filter_map(|rc| rc.state().get_route_state::<HashSet<String>>(self.state_key))
                    .any(|groups| groups.contains(group));

                if other_route {
                    ConstraintViolation::fail(self.code)
                } else {
                    None
                }
            }),
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match (self.aspects.get_job_group(&source), self.aspects.get_job_group(&candidate)) {
            (None, None) => Ok(source),
            (Some(s_group), Some(c_group)) if s_group == c_group => Ok(source),
            _ => Err(self.code),
        }
    }
}

struct GroupState<T: GroupAspects> {
    state_key: StateKey,
    state_keys: Vec<StateKey>,
    aspects: T,
}

impl<T: GroupAspects> FeatureState for GroupState<T> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        if let Some(group) = self.aspects.get_job_group(job) {
            let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();

            let mut groups = self.get_groups(route_ctx);
            groups.insert(group.clone());

            route_ctx.state_mut().put_route_state(self.state_key, groups)
        }
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            let groups = self.get_groups(route_ctx);
            route_ctx.state_mut().put_route_state(self.state_key, groups);
        });
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}

impl<T: GroupAspects> GroupState<T> {
    fn get_groups(&self, route_ctx: &RouteContext) -> HashSet<String> {
        route_ctx.route().tour.jobs().filter_map(|job| self.aspects.get_job_group(job)).cloned().collect()
    }
}
