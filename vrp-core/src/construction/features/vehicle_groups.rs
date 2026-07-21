//! A feature keeping a group of jobs on one vehicle across all of its shifts.

use super::*;
use std::collections::HashSet;

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/vehicle_groups_test.rs"]
mod vehicle_groups_test;

custom_dimension!(pub VehicleGroup typeof String);
custom_tour_state!(CurrentVehicleGroups typeof HashSet<String>);

/// Creates a vehicle-group feature as a hard constraint: every job sharing a
/// group value must be served by the same vehicle (any of its shifts/tours).
pub fn create_vehicle_group_feature(
    name: &str,
    total_jobs: usize,
    code: ViolationCode,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(VehicleGroupConstraint { total_jobs, code })
        .with_state(VehicleGroupState {})
        .build()
}

struct VehicleGroupConstraint {
    total_jobs: usize,
    code: ViolationCode,
}

impl FeatureConstraint for VehicleGroupConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => job.dimens().get_vehicle_group().and_then(|group| {
                // Same guard as the stock group feature: during decomposition the
                // sub-problem cannot verify the group globally, so refuse placement.
                let is_partial_problem = solution_ctx.get_jobs_amount() != self.total_jobs;
                if is_partial_problem {
                    return ConstraintViolation::fail(self.code);
                }

                let this_vehicle = route_ctx.route().actor.vehicle.dimens.get_vehicle_id();

                let on_other_vehicle = solution_ctx
                    .routes
                    .iter()
                    .filter(|rc| rc.route().actor.vehicle.dimens.get_vehicle_id() != this_vehicle)
                    .filter_map(|rc| rc.state().get_current_vehicle_groups())
                    .any(|groups| groups.contains(group));

                if on_other_vehicle { ConstraintViolation::fail(self.code) } else { None }
            }),
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match (source.dimens().get_vehicle_group(), candidate.dimens().get_vehicle_group()) {
            (None, None) => Ok(source),
            (Some(s), Some(c)) if s == c => Ok(source),
            _ => Err(self.code),
        }
    }
}

struct VehicleGroupState {}

impl FeatureState for VehicleGroupState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        if let Some(group) = job.dimens().get_vehicle_group() {
            let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
            let mut groups = collect_vehicle_groups(route_ctx);
            groups.insert(group.clone());
            route_ctx.state_mut().set_current_vehicle_groups(groups);
        }
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            let groups = collect_vehicle_groups(route_ctx);
            route_ctx.state_mut().set_current_vehicle_groups(groups);
        });
    }
}

fn collect_vehicle_groups(route_ctx: &RouteContext) -> HashSet<String> {
    route_ctx.route().tour.jobs().filter_map(|job| job.dimens().get_vehicle_group()).cloned().collect()
}
