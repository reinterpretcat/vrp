//! A feature to model group of jobs.

use super::*;
use std::collections::HashSet;

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/groups_test.rs"]
mod groups_test;

custom_dimension!(pub JobGroup typeof String);
custom_tour_state!(CurrentGroups typeof HashSet<String>);

/// Creates a job group feature as a hard constraint.
pub fn create_group_feature(name: &str, total_jobs: usize, code: ViolationCode) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(GroupConstraint { total_jobs, code })
        .with_state(GroupState {})
        .build()
}

struct GroupConstraint {
    total_jobs: usize,
    code: ViolationCode,
}

impl FeatureConstraint for GroupConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => job.dimens().get_job_group().and_then(|group| {
                let is_partial_problem = solution_ctx.get_jobs_amount() != self.total_jobs;
                if is_partial_problem {
                    return ConstraintViolation::fail(self.code);
                }

                let other_route = solution_ctx
                    .routes
                    .iter()
                    .filter(|rc| rc.route().actor != route_ctx.route().actor)
                    .filter_map(|rc| rc.state().get_current_groups())
                    .any(|groups| groups.contains(group));

                if other_route { ConstraintViolation::fail(self.code) } else { None }
            }),
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match (source.dimens().get_job_group(), candidate.dimens().get_job_group()) {
            (None, None) => Ok(source),
            (Some(s_group), Some(c_group)) if s_group == c_group => Ok(source),
            _ => Err(self.code),
        }
    }
}

struct GroupState {}

impl FeatureState for GroupState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        if let Some(group) = job.dimens().get_job_group() {
            let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();

            let mut groups = get_groups(route_ctx);
            groups.insert(group.clone());

            route_ctx.state_mut().set_current_groups(groups);
        }
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            let groups = get_groups(route_ctx);
            route_ctx.state_mut().set_current_groups(groups);
        });
    }
}

fn get_groups(route_ctx: &RouteContext) -> HashSet<String> {
    route_ctx.route().tour.jobs().filter_map(|job| job.dimens().get_job_group()).cloned().collect()
}
