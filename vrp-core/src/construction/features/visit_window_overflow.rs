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
use crate::models::solution::Activity;
use std::ops::ControlFlow;

/// Creates a feature whose objective counts tagged visits outside their own window.
pub fn create_visit_window_overflow_feature(
    name: &str,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
) -> GenericResult<Feature> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(VisitWindowOverflowObjective { transport, activity })
        .build()
}

struct VisitWindowOverflowObjective {
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
}

/// The own window of a tagged visit on its shift, when the shift has that window.
fn own_window<'a>(windows: &'a VisitWindows, activity: &Activity) -> Option<&'a VisitWindow> {
    let tag = activity.job.as_ref().and_then(|job| job.dimens.get_visit_window_tag())?;

    windows.windows.get(tag)
}

/// Service start is read back from the departure: the schedule keeps arrival and departure, and
/// the window or the job's own time window may have held service back past the arrival.
fn overflows(windows: &VisitWindows, activity: &Activity, departure: Timestamp) -> bool {
    own_window(windows, activity).is_some_and(|own| {
        let start = departure - activity.place.duration;
        start < own.earliest || departure > own.latest
    })
}

impl FeatureObjective for VisitWindowOverflowObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .routes
            .iter()
            .filter_map(|route_ctx| {
                let windows = route_ctx.route().actor.vehicle.dimens.get_visit_windows()?;

                Some(
                    route_ctx
                        .route()
                        .tour
                        .all_activities()
                        .filter(|activity| overflows(windows, activity, activity.schedule.departure))
                        .count() as Cost,
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

        if overflows(windows, target, departure) { 1. } else { Cost::default() }
    }
}
