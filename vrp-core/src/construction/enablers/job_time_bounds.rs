#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/job_time_bounds_test.rs"]
mod job_time_bounds_test;

use crate::models::common::*;
use crate::models::problem::{ActivityCost, JobTimeConstraints, JobTimeConstraintsDimension};
use crate::models::solution::{Activity, Route};
use std::ops::ControlFlow;
use std::sync::Arc;

/// Applies a shift's appointment bounds to every job activity on it.
///
/// The bounds are a property of the technician's day, not of the job, so they
/// cannot live in a job's time window. They are applied where the schedule is
/// actually produced: forward, by raising a job's service start to the lower
/// bound — which is what makes the vehicle wait rather than serve early — and
/// backward, by capping the latest departure, which tightens `latest_arrival`
/// for every activity before it.
///
/// ⚠️ Wraps the outside of a reserved-time cost, never the inside: the upper
/// bound has to be tested against the departure a required break has already
/// inflated.
pub struct JobTimeBoundsActivityCost {
    inner: Arc<dyn ActivityCost>,
}

impl JobTimeBoundsActivityCost {
    /// Creates a new instance of `JobTimeBoundsActivityCost`.
    pub fn new(inner: Arc<dyn ActivityCost>) -> Self {
        Self { inner }
    }

    fn bounds(route: &Route, activity: &Activity) -> Option<JobTimeConstraints> {
        activity.job.as_ref()?;

        let bounds = route.actor.vehicle.dimens.get_job_time_constraints().copied()?;

        if bounds.earliest_first.is_none() && bounds.latest_last.is_none() { None } else { Some(bounds) }
    }
}

impl ActivityCost for JobTimeBoundsActivityCost {
    fn estimate_departure(
        &self,
        route: &Route,
        activity: &Activity,
        arrival: Timestamp,
    ) -> ControlFlow<Timestamp, Timestamp> {
        let Some(bounds) = Self::bounds(route, activity) else {
            return self.inner.estimate_departure(route, activity, arrival);
        };

        let arrival = bounds.earliest_first.map_or(arrival, |earliest| arrival.max(earliest));
        let departure = self.inner.estimate_departure(route, activity, arrival);

        match (departure, bounds.latest_last) {
            (ControlFlow::Continue(departure), Some(latest_last)) if departure > latest_last => {
                ControlFlow::Break(departure)
            }
            (departure, _) => departure,
        }
    }

    fn estimate_arrival(
        &self,
        route: &Route,
        activity: &Activity,
        departure: Timestamp,
    ) -> ControlFlow<Timestamp, Timestamp> {
        let Some(bounds) = Self::bounds(route, activity) else {
            return self.inner.estimate_arrival(route, activity, departure);
        };

        let departure = bounds.latest_last.map_or(departure, |latest| departure.min(latest));

        self.inner.estimate_arrival(route, activity, departure)
    }
}
