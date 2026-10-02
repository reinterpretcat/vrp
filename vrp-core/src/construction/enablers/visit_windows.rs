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

/// A visit window as `(earliest service start, latest departure)`.
type Window = (Timestamp, Timestamp);

/// Keeps a tagged visit inside the visit window of the shift it lands on.
///
/// A shift says when its recurring visits and its other visits may happen; a job says which of the
/// two it keeps to. The window is a property of the technician's day, not of the job, so it cannot
/// live in the job's time window. Same mechanism as `JobTimeBoundsActivityCost`: forward, service
/// is held back to the window's start; backward, the latest departure is capped at its end; a visit
/// that cannot fit is refused.
///
/// With overflow, a recurring visit that does not fit its own window from where it arrives is
/// served in the other window instead; one that fits waits for its own. The overflow objective
/// counts the visits that end up outside their own window.
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
    fn windows(&self, route: &Route, activity: &Activity) -> Option<(Window, Option<Window>)> {
        let windows: &VisitWindows = route.actor.vehicle.dimens.get_visit_windows()?;
        let single = activity.job.as_ref()?;
        let kind: &VisitWindowKind = single.dimens.get_visit_window_kind()?;

        if (self.is_visit)(single) { windows.windows_for(kind) } else { None }
    }

    /// Serves the visit inside `window` from `arrival`: held back to its start, refused when it
    /// would depart after its end or start after the job's own time window closed.
    fn serve_in(
        &self,
        route: &Route,
        activity: &Activity,
        arrival: Timestamp,
        (earliest, latest): Window,
    ) -> ControlFlow<Timestamp, Timestamp> {
        let arrival = arrival.max(earliest);

        match self.inner.estimate_departure(route, activity, arrival) {
            ControlFlow::Continue(departure) if arrival > activity.place.time.end => ControlFlow::Break(departure),
            ControlFlow::Continue(departure) if departure > latest => ControlFlow::Break(departure),
            departure => departure,
        }
    }

    /// The window the visit is served in from `arrival`: its own when it fits there, the fallback
    /// otherwise. A visit that could wait for its own window waits, so overflow never serves a
    /// recurring visit early just because the other hours open sooner.
    fn chosen(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Option<Window> {
        let (own, fallback) = self.windows(route, activity)?;

        match (self.serve_in(route, activity, arrival, own), fallback) {
            (ControlFlow::Break(_), Some(fallback)) => Some(fallback),
            _ => Some(own),
        }
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
        match self.chosen(route, activity, arrival) {
            Some(window) => self.serve_in(route, activity, arrival, window),
            None => self.inner.estimate_departure(route, activity, arrival),
        }
    }

    fn estimate_arrival(
        &self,
        route: &Route,
        activity: &Activity,
        departure: Timestamp,
    ) -> ControlFlow<Timestamp, Timestamp> {
        // the latest the visit may be departed from at all: the later end of its own window and
        // its fallback.
        let departure = self.windows(route, activity).map_or(departure, |((_, own), fallback)| {
            departure.min(fallback.map_or(own, |(_, fallback)| own.max(fallback)))
        });

        self.inner.estimate_arrival(route, activity, departure)
    }

    /// The arithmetic of `estimate_departure` without the service: a visit held back by its window
    /// starts at the window, and the departure optimiser has to see that idle.
    fn estimate_service_start(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp {
        let arrival = self.chosen(route, activity, arrival).map_or(arrival, |(earliest, _)| arrival.max(earliest));

        self.inner.estimate_service_start(route, activity, arrival)
    }
}
