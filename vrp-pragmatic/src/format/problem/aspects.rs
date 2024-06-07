use crate::construction::enablers::{BreakTie, JobTie, VehicleTie};
use vrp_core::construction::features::{BreakAspects, BreakCandidate, BreakPolicy};
use vrp_core::construction::heuristics::RouteContext;
use vrp_core::models::common::IdDimension;
use vrp_core::models::problem::Single;
use vrp_core::models::solution::Route;

/// Provides way to use break feature.
#[derive(Clone, Copy)]
pub struct PragmaticBreakAspects;

impl BreakAspects for PragmaticBreakAspects {
    fn is_break_job(&self, candidate: BreakCandidate<'_>) -> bool {
        candidate
            .as_single()
            .and_then(|break_single| break_single.dimens.get_job_type())
            .map_or(false, |job_type| job_type == "break")
    }

    fn belongs_to_route(&self, route_ctx: &RouteContext, candidate: BreakCandidate<'_>) -> bool {
        if self.is_break_job(candidate) {
            candidate.as_single().map_or(false, |single| is_correct_vehicle(route_ctx.route(), single))
        } else {
            false
        }
    }

    fn get_policy(&self, candidate: BreakCandidate<'_>) -> Option<BreakPolicy> {
        candidate.as_single().and_then(|single| single.dimens.get_break_policy())
    }
}

fn is_correct_vehicle(route: &Route, single: &Single) -> bool {
    let job_vehicle_id = single.dimens.get_vehicle_id();
    let job_shift_idx = single.dimens.get_shift_index();

    let vehicle = &route.actor.vehicle;
    let vehicle_id = vehicle.dimens.get_id();
    let vehicle_shift_idx = vehicle.dimens.get_shift_index();

    job_vehicle_id == vehicle_id && job_shift_idx == vehicle_shift_idx
}
