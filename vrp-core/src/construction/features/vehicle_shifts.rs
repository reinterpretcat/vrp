//! Provides a feature to enforce minimum shift usage per vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/vehicle_shifts_test.rs"]
mod vehicle_shifts_test;

use super::*;
use std::collections::{HashMap, HashSet};

custom_solution_state!(pub VehicleShiftSummary typeof VehicleShiftInfo);

/// Provides a way to build a feature which enforces minimum shift usage per vehicle.
pub struct MinVehicleShiftsFeatureBuilder {
    name: String,
    violation_code: ViolationCode,
    requirements: Option<HashMap<String, MinShiftRequirement>>,
}

/// Represents minimum shift requirements per vehicle id.
#[derive(Clone)]
pub struct MinShiftRequirement {
    /// Minimum number of shifts that must be used.
    pub minimum: usize,
    /// When true, usage of zero shifts is allowed without violating the minimum requirement.
    pub allow_zero_usage: bool,
}

impl MinVehicleShiftsFeatureBuilder {
    /// Creates a new builder instance.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), violation_code: ViolationCode::default(), requirements: None }
    }

    /// Sets a violation code which is used when constraint forbids an insertion.
    pub fn with_violation_code(mut self, violation_code: ViolationCode) -> Self {
        self.violation_code = violation_code;
        self
    }

    /// Sets a map with required shifts per vehicle id.
    pub fn with_requirements(mut self, requirements: HashMap<String, MinShiftRequirement>) -> Self {
        self.requirements = Some(requirements);
        self
    }

    /// Builds a feature instance.
    pub fn build(self) -> GenericResult<Feature> {
        let requirements = self.requirements.ok_or_else(|| "requirements map is not defined".to_string())?;

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_constraint(MinVehicleShiftsConstraint { violation_code: self.violation_code })
            .with_state(MinVehicleShiftsState { requirements })
            .build()
    }
}

struct MinVehicleShiftsConstraint {
    violation_code: ViolationCode,
}

impl FeatureConstraint for MinVehicleShiftsConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, .. } => {
                let summary = solution_ctx.state.get_vehicle_shift_summary()?;

                if summary.missing_vehicle_ids.is_empty() {
                    return None;
                }

                route_ctx.route().actor.vehicle.dimens.get_vehicle_id().and_then(|vehicle_id| {
                    if summary.missing_vehicle_ids.contains(vehicle_id) {
                        None
                    } else {
                        ConstraintViolation::skip(self.violation_code)
                    }
                })
            }
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

struct MinVehicleShiftsState {
    requirements: HashMap<String, MinShiftRequirement>,
}

impl FeatureState for MinVehicleShiftsState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
        self.accept_solution_state(solution_ctx);
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let summary = build_vehicle_shift_summary(solution_ctx.routes.as_slice(), &self.requirements);

        solution_ctx.state.set_vehicle_shift_summary(summary);
    }
}

fn build_vehicle_shift_summary(
    routes: &[RouteContext],
    requirements: &HashMap<String, MinShiftRequirement>,
) -> VehicleShiftInfo {
    let usage = routes.iter().fold(HashMap::new(), |mut used, route_ctx| {
        if let Some(vehicle_id) = route_ctx.route().actor.vehicle.dimens.get_vehicle_id().cloned() {
            if requirements.contains_key(&vehicle_id) && route_ctx.route().tour.has_jobs() {
                *used.entry(vehicle_id).or_insert(0) += 1;
            }
        }

        used
    });

    let missing_vehicle_ids = requirements
        .iter()
        .filter_map(|(vehicle_id, requirement)| {
            let used = usage.get(vehicle_id).copied().unwrap_or(0);
            let below_minimum = used < requirement.minimum;
            let zero_allowed = requirement.allow_zero_usage && used == 0;
            if below_minimum && !zero_allowed { Some(vehicle_id.clone()) } else { None }
        })
        .collect();

    VehicleShiftInfo { missing_vehicle_ids }
}

/// Provides aggregated vehicle shift usage information.
#[derive(Clone, Default)]
pub struct VehicleShiftInfo {
    /// Vehicle ids that still require additional shifts.
    pub missing_vehicle_ids: HashSet<String>,
}
