#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/visit_windows_test.rs"]
mod visit_windows_test;

use super::IsAppointmentFn;
use crate::models::common::*;
use crate::models::problem::{ActivityCost, VisitWindow, VisitWindowTagDimension, VisitWindows, VisitWindowsDimension};
use crate::models::solution::{Activity, Route};
use std::ops::ControlFlow;
use std::sync::Arc;

/// A visit window as `(earliest service start, latest departure)`.
type Window = (Timestamp, Timestamp);

/// Keeps a tagged visit inside the visit window of the shift it lands on.
///
/// A shift names its windows; a job names the one it keeps to. The window is a property of the
/// technician's day, not of the job, so it cannot live in the job's time window. Same mechanism as
/// `JobTimeBoundsActivityCost`: forward, service is held back to the window's start; backward, the
/// latest departure is capped at its end; a visit that cannot fit is refused.
///
/// A window may name a fallback: a visit that does not fit its own window from where it arrives is
/// served in the first window of the chain it fits; one that fits its own waits for it. The overflow
/// objective counts the visits that end up outside their own window. A bridged own window reaches out
/// to the untagged visits on the route (`own_window_on`).
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

    /// The windows a visit may lie in on this route: the window its tag names, then that window's
    /// fallback chain. The vehicle lookup comes first: it is cheap and constant per route, and a shift
    /// without windows leaves through it before the job is looked at.
    fn windows(&self, route: &Route, activity: &Activity) -> Option<Vec<Window>> {
        let windows: &VisitWindows = route.actor.vehicle.dimens.get_visit_windows()?;
        let single = activity.job.as_ref()?;
        let tag: &String = single.dimens.get_visit_window_tag()?;

        if !(self.is_visit)(single) {
            return None;
        }

        let chain =
            windows
                .chain(tag)
                .into_iter()
                .enumerate()
                .map(|(idx, (_, window))| {
                    if idx == 0 {
                        own_window_on(route, window, &self.is_visit)
                    } else {
                        (window.earliest, window.latest)
                    }
                })
                .collect::<Vec<_>>();

        (!chain.is_empty()).then_some(chain)
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

    /// The window the visit is served in from `arrival`: the first of its chain it fits, its own
    /// window first. A visit that could wait for its own window waits, so a fallback never serves a
    /// visit early just because it opens sooner. A visit that fits none is measured against the last.
    fn chosen(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Option<Window> {
        let chain = self.windows(route, activity)?;

        chain
            .iter()
            .copied()
            .find(|&window| matches!(self.serve_in(route, activity, arrival, window), ControlFlow::Continue(_)))
            .or_else(|| chain.last().copied())
    }
}

/// The own window of a tagged visit on `route`. A bridged window reaches out to the untagged visits
/// on the route: from the earlier of its start and their earliest window start, to the later of its
/// end and their latest window end. Window bounds, not scheduled times: the window then depends on
/// which visits are on the route, never on the schedule being built.
pub fn own_window_on(route: &Route, window: &VisitWindow, is_visit: &IsAppointmentFn) -> Window {
    if !window.bridge {
        return (window.earliest, window.latest);
    }

    route
        .tour
        .all_activities()
        .filter_map(|activity| activity.job.as_ref().map(|single| (activity, single)))
        .filter(|(_, single)| is_visit(single) && single.dimens.get_visit_window_tag().is_none())
        .fold((window.earliest, window.latest), |(earliest, latest), (activity, _)| {
            (earliest.min(activity.place.time.start), latest.max(activity.place.time.end))
        })
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
        // the latest the visit may be departed from at all: the latest end along its chain.
        let departure = self.windows(route, activity).map_or(departure, |chain| {
            departure.min(chain.iter().map(|&(_, latest)| latest).fold(Timestamp::MIN, Timestamp::max))
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
