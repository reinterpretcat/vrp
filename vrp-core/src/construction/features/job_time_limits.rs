//! A feature to enforce job time constraints on shifts.
//!
//! This allows configuring:
//! - `earliest_first`: The earliest time a vehicle can arrive at its first job
//! - `latest_last`: The latest time a vehicle can depart from its last job

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/job_time_limits_test.rs"]
mod job_time_limits_test;

use super::*;
use crate::models::problem::{Job, JobTimeConstraintsDimension, TransportCost, TravelTime};

/// Creates a feature that enforces job time constraints on shifts.
/// This is a hard constraint - jobs that violate the constraints remain unassigned.
///
/// # Arguments
/// * `name` - Feature name
/// * `transport` - Transport cost provider for calculating travel times
/// * `activity` - Activity cost provider for estimating departures
/// * `violation_code` - Code returned when constraint is violated
pub fn create_job_time_limits_feature(
    name: &str,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    violation_code: ViolationCode,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(JobTimeLimitsConstraint { transport, activity, violation_code })
        .build()
}

struct JobTimeLimitsConstraint {
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    violation_code: ViolationCode,
}

impl JobTimeLimitsConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let actor = route_ctx.route().actor.as_ref();
        let constraints = actor.vehicle.dimens.get_job_time_constraints().copied()?;

        // Skip if no constraints are set
        if constraints.earliest_first.is_none() && constraints.latest_last.is_none() {
            return None;
        }

        let route = route_ctx.route();
        let prev = activity_ctx.prev;
        let target = activity_ctx.target;

        // Skip if target is not a job (e.g., it's a depot or break)
        target.job.as_ref()?;

        let departure = prev.schedule.departure;
        let arr_time_at_target = departure
            + self.transport.duration(
                route,
                prev.place.location,
                target.place.location,
                TravelTime::Departure(departure),
            );

        // Check earliest_first constraint: applies when this is the first job
        // (prev is the start depot, which has no job)
        if let Some(earliest_first) = constraints.earliest_first {
            let is_first_job = prev.job.is_none() && activity_ctx.index == 0;
            if is_first_job && arr_time_at_target < earliest_first {
                // Vehicle would arrive before earliest allowed time
                // Check if we can wait - job's time window must extend past earliest_first
                if target.place.time.end < earliest_first {
                    return ConstraintViolation::skip(self.violation_code);
                }
                // We can wait, but we need to ensure the adjusted arrival still works
                // The actual arrival will be max(arr_time_at_target, earliest_first)
                // which needs to be <= target.place.time.end (already checked above)
            }
        }

        // Check latest_last constraint: applies when this becomes the last job
        // (next is the end depot or None for open routes)
        if let Some(latest_last) = constraints.latest_last {
            let is_last_job = activity_ctx.next.is_none_or(|next| next.job.is_none());
            if is_last_job {
                // Calculate when we would depart from this job
                let actual_arr_time = if let Some(earliest_first) = constraints.earliest_first {
                    let is_first_job = prev.job.is_none() && activity_ctx.index == 0;
                    if is_first_job { arr_time_at_target.max(earliest_first) } else { arr_time_at_target }
                } else {
                    arr_time_at_target
                };

                // Respect the job's time window (might need to wait)
                let service_start = actual_arr_time.max(target.place.time.start);
                let departure_result = self.activity.estimate_departure(route, target, service_start);

                // Extract departure time from ControlFlow (use the value regardless of Continue/Break)
                let departure_from_target = match departure_result {
                    std::ops::ControlFlow::Continue(t) | std::ops::ControlFlow::Break(t) => t,
                };

                if departure_from_target > latest_last {
                    return ConstraintViolation::skip(self.violation_code);
                }
            }
        }

        None
    }
}

impl FeatureConstraint for JobTimeLimitsConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { .. } => None,
            MoveContext::Activity { route_ctx, activity_ctx, .. } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}
