#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/visit_windows_test.rs"]
mod visit_windows_test;

use super::IsAppointmentFn;
use crate::models::common::*;
use crate::models::problem::{
    ActivityCost, VisitWindowKind, VisitWindowKindDimension, VisitWindows, VisitWindowsDimension,
};
use crate::models::solution::{Activity, Route};
use std::ops::ControlFlow;
use std::sync::Arc;

/// Keeps a tagged visit inside the visit window of the shift it lands on.
///
/// A shift says when its recurring visits and its other visits may happen; a job says which of the
/// two it keeps to. The window is a property of the technician's day, not of the job, so it cannot
/// live in the job's time window. Same mechanism as `JobTimeBoundsActivityCost`: forward, service
/// is held back to the window's start; backward, the latest departure is capped at its end; a visit
/// that cannot fit is refused.
///
/// An untagged job (a preferred or pinned time), a job on a shift without windows, and anything
/// `is_visit` rejects (a break, a reload, a recharge) pass through to the inner cost untouched.
///
/// ⚠️ Wraps the outside of the job-times cost: a visit keeps to both.
pub struct VisitWindowsActivityCost {
    inner: Arc<dyn ActivityCost>,
    is_visit: IsAppointmentFn,
}

impl VisitWindowsActivityCost {
    /// Creates a new instance of `VisitWindowsActivityCost`.
    pub fn new(inner: Arc<dyn ActivityCost>, is_visit: IsAppointmentFn) -> Self {
        Self { inner, is_visit }
    }

    /// The vehicle lookup comes first: it is cheap and constant per route, and a shift without
    /// windows leaves through it before the job is looked at.
    fn bounds(&self, route: &Route, activity: &Activity) -> Option<(Timestamp, Timestamp)> {
        let windows: &VisitWindows = route.actor.vehicle.dimens.get_visit_windows()?;
        let single = activity.job.as_ref()?;
        let kind: &VisitWindowKind = single.dimens.get_visit_window_kind()?;

        if (self.is_visit)(single) { windows.bounds_for(kind) } else { None }
    }
}

impl ActivityCost for VisitWindowsActivityCost {
    /// Forwarded unchanged: the window decides when a visit may happen, never what it costs.
    fn cost(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Cost {
        self.inner.cost(route, activity, arrival)
    }

    fn estimate_departure(
        &self,
        route: &Route,
        activity: &Activity,
        arrival: Timestamp,
    ) -> ControlFlow<Timestamp, Timestamp> {
        let Some((earliest, latest)) = self.bounds(route, activity) else {
            return self.inner.estimate_departure(route, activity, arrival);
        };

        let arrival = arrival.max(earliest);
        let departure = self.inner.estimate_departure(route, activity, arrival);

        match departure {
            // waiting for the window is legal, serving past the job's own window is not.
            ControlFlow::Continue(departure) if arrival > activity.place.time.end => ControlFlow::Break(departure),
            ControlFlow::Continue(departure) if departure > latest => ControlFlow::Break(departure),
            departure => departure,
        }
    }

    fn estimate_arrival(
        &self,
        route: &Route,
        activity: &Activity,
        departure: Timestamp,
    ) -> ControlFlow<Timestamp, Timestamp> {
        let departure = self.bounds(route, activity).map_or(departure, |(_, latest)| departure.min(latest));

        self.inner.estimate_arrival(route, activity, departure)
    }

    /// The arithmetic of `estimate_departure` without the service: a visit held back by its window
    /// starts at the window, and the departure optimiser has to see that idle.
    fn estimate_service_start(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp {
        let arrival = self.bounds(route, activity).map_or(arrival, |(earliest, _)| arrival.max(earliest));

        self.inner.estimate_service_start(route, activity, arrival)
    }
}
