//! Counts tagged visits served outside their own window, in one of its fallbacks.
//!
//! The visit-window activity cost lets a visit use its window's fallback chain when it does not fit
//! its own window. This objective, placed directly after minimizing unassigned jobs, makes that the
//! last resort: a solution only pays for an overflowed visit by placing a visit it otherwise could
//! not, never to save distance or cost.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/visit_window_overflow_test.rs"]
mod visit_window_overflow_test;

use super::*;
use crate::construction::enablers::{IsAppointmentFn, own_window_on};
use crate::models::solution::{Activity, Route};
use std::ops::ControlFlow;

/// What the overflow objective measures for a visit outside its own window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowMeasure {
    /// One per visit outside its own window.
    Visits,
    /// The service time that lies outside its own window.
    Minutes,
}

/// Creates a feature whose objective measures tagged visits outside their own window, the own window
/// bridged on its route.
pub fn create_visit_window_overflow_feature(
    name: &str,
    measure: OverflowMeasure,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    is_visit: IsAppointmentFn,
) -> GenericResult<Feature> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(VisitWindowOverflowObjective { measure, transport, activity, is_visit })
        .build()
}

struct VisitWindowOverflowObjective {
    measure: OverflowMeasure,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    is_visit: IsAppointmentFn,
}

/// The own window of a tagged visit on its shift, when the shift has that window.
fn own_window<'a>(windows: &'a VisitWindows, activity: &Activity) -> Option<&'a VisitWindow> {
    let tag = activity.job.as_ref().and_then(|job| job.dimens.get_visit_window_tag())?;

    windows.windows.get(tag)
}

impl VisitWindowOverflowObjective {
    /// How far a visit departing at `departure` lies outside its own window on `route`. Service start
    /// is read back from the departure: the schedule keeps arrival and departure, and the window or
    /// the job's own time window may have held service back past the arrival.
    fn overflow(&self, route: &Route, windows: &VisitWindows, activity: &Activity, departure: Timestamp) -> Cost {
        let Some(own) = own_window(windows, activity) else { return Cost::default() };
        let (earliest, latest) = own_window_on(route, own, &self.is_visit);
        let start = departure - activity.place.duration;
        let outside = ((earliest - start).max(0.) + (departure - latest).max(0.)).min(departure - start);

        match self.measure {
            OverflowMeasure::Visits if start < earliest || departure > latest => 1.,
            OverflowMeasure::Visits => Cost::default(),
            OverflowMeasure::Minutes => outside,
        }
    }
}

impl FeatureObjective for VisitWindowOverflowObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .routes
            .iter()
            .filter_map(|route_ctx| {
                let windows = route_ctx.route().actor.vehicle.dimens.get_visit_windows()?;

                let route = route_ctx.route();

                Some(
                    route
                        .tour
                        .all_activities()
                        .map(|activity| self.overflow(route, windows, activity, activity.schedule.departure))
                        .sum::<Cost>(),
                )
            })
            .sum()
    }

    /// The target's own schedule is not known yet when a move is estimated: it is computed here
    /// from the previous departure, the way the transport objective computes it.
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        let MoveContext::Activity { route_ctx, activity_ctx, .. } = move_ctx else { return Cost::default() };
        let target = activity_ctx.target;
        let route = route_ctx.route();

        let Some(windows) = route.actor.vehicle.dimens.get_visit_windows() else { return Cost::default() };
        if own_window(windows, target).is_none() {
            return Cost::default();
        }

        let prev = activity_ctx.prev;
        let arrival = prev.schedule.departure
            + self.transport.duration(
                route,
                prev.place.location,
                target.place.location,
                TravelTime::Departure(prev.schedule.departure),
            );
        let departure = match self.activity.estimate_departure(route, target, arrival) {
            ControlFlow::Continue(departure) | ControlFlow::Break(departure) => departure,
        };

        self.overflow(route, windows, target, departure)
    }
}
