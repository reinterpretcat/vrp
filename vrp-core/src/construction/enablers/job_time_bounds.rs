#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/job_time_bounds_test.rs"]
mod job_time_bounds_test;

use crate::models::common::*;
use crate::models::problem::{ActivityCost, JobTimeConstraints, JobTimeConstraintsDimension, Single};
use crate::models::solution::{Activity, Route};
use std::ops::ControlFlow;
use std::sync::Arc;

/// A function which tells whether a job's activity is an appointment the shift's bounds apply to.
/// A break, a reload or a recharge is a job sitting on the tour too, but it is not an appointment,
/// and what makes one is a property of the format, not of the core — so the caller injects it.
pub type IsAppointmentFn = Arc<dyn Fn(&Single) -> bool + Sync + Send>;

/// Applies a shift's appointment bounds to the appointment activities on it.
///
/// The bounds are a property of the technician's day, not of the job, so they
/// cannot live in a job's time window. They are applied where the schedule is
/// actually produced: forward, by raising an appointment's service start to the
/// lower bound — which is what makes the vehicle wait rather than serve early —
/// and backward, by capping the latest departure, which tightens `latest_arrival`
/// for every activity before it.
///
/// Driving is untouched, and so is anything `is_appointment` rejects: a day
/// bounded to 08:00 does not push a 07:00 break into the appointment window.
///
/// ⚠️ Wraps the outside of a reserved-time cost, never the inside: the upper
/// bound has to be tested against the departure a required break has already
/// inflated.
///
/// ⚠️ Known limitation: on an open route — a shift that declares no `end` — the
/// upper bound does not propagate backwards, so a job inserted ahead of the last
/// appointment can push that appointment past `latest_last`. The backward pass in
/// `schedule_update` starts from `actor.detail.time.end`, which is `Float::MAX`
/// without a shift end, and in that case it takes each activity's own window end
/// and never asks `estimate_arrival` — the only place this cost could cap it.
pub struct JobTimeBoundsActivityCost {
    inner: Arc<dyn ActivityCost>,
    is_appointment: IsAppointmentFn,
}

impl JobTimeBoundsActivityCost {
    /// Creates a new instance of `JobTimeBoundsActivityCost`.
    pub fn new(inner: Arc<dyn ActivityCost>, is_appointment: IsAppointmentFn) -> Self {
        Self { inner, is_appointment }
    }

    /// The vehicle lookup comes first on purpose: it is constant for the whole route and cheap,
    /// while the injected predicate is a `dyn Fn` over a job-type dimension. Activities on a
    /// bound-free vehicle leave through the first `?` without paying for either.
    fn bounds(&self, route: &Route, activity: &Activity) -> Option<JobTimeConstraints> {
        let bounds = route.actor.vehicle.dimens.get_job_time_constraints().copied()?;

        if bounds.earliest_first.is_none() && bounds.latest_last.is_none() {
            return None;
        }

        let single = activity.job.as_ref()?;

        if (self.is_appointment)(single) { Some(bounds) } else { None }
    }
}

impl ActivityCost for JobTimeBoundsActivityCost {
    /// Forwarded unchanged. The bounds decide when an activity may happen, never what it costs, and
    /// the trait's default implementation is not the inner cost: it charges the driver's share,
    /// which `OnlyVehicleActivityCost` — the cost this decorator wraps in the pragmatic format —
    /// deliberately drops.
    fn cost(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Cost {
        self.inner.cost(route, activity, arrival)
    }

    fn estimate_departure(
        &self,
        route: &Route,
        activity: &Activity,
        arrival: Timestamp,
    ) -> ControlFlow<Timestamp, Timestamp> {
        let Some(bounds) = self.bounds(route, activity) else {
            return self.inner.estimate_departure(route, activity, arrival);
        };

        let arrival = bounds.earliest_first.map_or(arrival, |earliest| arrival.max(earliest));
        let departure = self.inner.estimate_departure(route, activity, arrival);

        match (departure, bounds.latest_last) {
            // waiting for the lower bound is legal, serving past the job's own window is not: a job
            // whose window closes before the bound cannot be served on this shift at all.
            (ControlFlow::Continue(departure), _) if arrival > activity.place.time.end => ControlFlow::Break(departure),
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
        let Some(bounds) = self.bounds(route, activity) else {
            return self.inner.estimate_arrival(route, activity, departure);
        };

        let departure = bounds.latest_last.map_or(departure, |latest| departure.min(latest));

        self.inner.estimate_arrival(route, activity, departure)
    }

    /// The same arithmetic `estimate_departure` performs, without the service duration on top: an
    /// appointment held back by the lower bound starts at the bound, and the vehicle idles until
    /// then. The departure optimiser reads this to see that idle — measuring the job's own window
    /// alone, it would leave the vehicle waiting at the first appointment instead of leaving the
    /// depot later, and the idle would be charged to the shift's duration limit.
    fn estimate_service_start(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp {
        let arrival = self
            .bounds(route, activity)
            .and_then(|bounds| bounds.earliest_first)
            .map_or(arrival, |earliest| arrival.max(earliest));

        self.inner.estimate_service_start(route, activity, arrival)
    }
}
