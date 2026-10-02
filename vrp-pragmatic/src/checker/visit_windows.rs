#[cfg(test)]
#[path = "../../tests/unit/checker/visit_windows_test.rs"]
mod visit_windows_test;

use super::*;
use crate::format_time;
use vrp_core::models::problem::{VisitWindow, VisitWindowKind, VisitWindows};
use vrp_core::prelude::GenericResult;

/// Checks that every tagged stop lies inside the visit window its shift gives it: service starts at
/// or after the window's start and the stop is departed at or before its end. With overflow, a
/// recurring stop may lie anywhere in its own window or the other one.
pub fn check_visit_windows(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    check_tours(context).map_err(|error| vec![error])
}

fn check_tours(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().try_for_each(|tour| {
        let shift = context.get_vehicle_shift(tour)?;
        let Some(windows) = shift.visit_windows.as_ref() else { return Ok(()) };
        let window = |window: &VisitWindowJson| VisitWindow {
            earliest: parse_time(&window.earliest),
            latest: parse_time(&window.latest),
        };
        let windows = VisitWindows {
            recurring: windows.recurring.as_ref().map(window),
            other: windows.other.as_ref().map(window),
            overflow: windows.overflow.unwrap_or(false),
        };

        tour.stops
            .iter()
            .flat_map(|stop| stop.activities().iter().map(move |activity| (stop, activity)))
            .filter(|(_, activity)| is_stop_activity(activity))
            .try_for_each(|(stop, activity)| {
                let Some(job) = context.get_job_by_id(&activity.job_id) else { return Ok(()) };
                let kind = match job.visit_window.as_deref() {
                    Some("recurring") => VisitWindowKind::Recurring,
                    Some("other") => VisitWindowKind::Other,
                    _ => return Ok(()),
                };
                let Some((earliest, latest)) = windows.bounds_for(&kind) else { return Ok(()) };

                // service start is read back from the departure: arriving early and waiting for the
                // window is legal, so the arrival says nothing about when service began.
                let departure = context.get_activity_time(stop, activity).end;
                let duration = match_job_task(&activity.activity_type, job, |tasks| tasks.first())
                    .and_then(|task| task.places.first())
                    .map_or(0., |place| place.duration);
                let service_start = departure - duration;

                if service_start < earliest || departure > latest {
                    Err(format!(
                        "visit window violation: job '{}' is served from {} to {}, its window is {} to {}, vehicle id '{}', shift index: {}",
                        activity.job_id,
                        format_time(service_start),
                        format_time(departure),
                        format_time(earliest),
                        format_time(latest),
                        tour.vehicle_id,
                        tour.shift_index
                    )
                    .into())
                } else {
                    Ok(())
                }
            })
    })
}
