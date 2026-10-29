#[cfg(test)]
#[path = "../../tests/unit/checker/visit_windows_test.rs"]
mod visit_windows_test;

use super::*;
use crate::format_time;
use vrp_core::models::problem::{VisitWindow, VisitWindows};
use vrp_core::prelude::GenericResult;

/// Checks that every tagged stop lies inside the window its tag names or one of that window's
/// fallbacks: service starts at or after the window's start and the stop is departed at or before
/// its end.
pub fn check_visit_windows(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    check_tours(context).map_err(|error| vec![error])
}

fn check_tours(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().try_for_each(|tour| {
        let shift = context.get_vehicle_shift(tour)?;
        let Some(windows) = shift.visit_windows.as_ref() else { return Ok(()) };
        let windows = VisitWindows {
            windows: windows
                .iter()
                .map(|(name, window)| {
                    (
                        name.clone(),
                        VisitWindow {
                            earliest: parse_time(&window.earliest),
                            latest: parse_time(&window.latest),
                            fallback: window.fallback.clone(),
                            bridge: window.bridge,
                        },
                    )
                })
                .collect(),
        };

        // the window bounds of the tour's untagged visits: a bridged own window reaches out to them,
        // as `own_window_on` does while the tour is built.
        let fixed_bounds = tour
            .stops
            .iter()
            .flat_map(|stop| stop.activities().iter().map(move |activity| (stop, activity)))
            .filter(|(_, activity)| is_stop_activity(activity))
            .filter_map(|(stop, activity)| {
                let job = context.get_job_by_id(&activity.job_id)?;
                if job.visit_window.is_some() {
                    return None;
                }
                let served = context.get_activity_time(stop, activity);
                let times = match_job_task(&activity.activity_type, job, |tasks| tasks.first())
                    .and_then(|task| task.places.first())
                    .and_then(|place| place.times.clone())?;

                times
                    .iter()
                    .filter_map(|time| Some((parse_time(time.first()?), parse_time(time.last()?))))
                    .find(|&(start, end)| served.start <= end && served.end >= start)
            })
            .fold(None, |bounds: Option<(f64, f64)>, (start, end)| {
                Some(bounds.map_or((start, end), |(earliest, latest)| (earliest.min(start), latest.max(end))))
            });

        tour.stops
            .iter()
            .flat_map(|stop| stop.activities().iter().map(move |activity| (stop, activity)))
            .filter(|(_, activity)| is_stop_activity(activity))
            .try_for_each(|(stop, activity)| {
                let Some(job) = context.get_job_by_id(&activity.job_id) else { return Ok(()) };
                let Some(tag) = job.visit_window.as_deref() else { return Ok(()) };
                let chain = windows.chain(tag);
                if chain.is_empty() {
                    return Ok(());
                }

                // service start is read back from the departure: arriving early and waiting for the
                // window is legal, so the arrival says nothing about when service began.
                let departure = context.get_activity_time(stop, activity).end;
                let duration = match_job_task(&activity.activity_type, job, |tasks| tasks.first())
                    .and_then(|task| task.places.first())
                    .map_or(0., |place| place.duration);
                let service_start = departure - duration;

                let fits = |(earliest, latest): (f64, f64)| service_start >= earliest && departure <= latest;
                let bounds = |idx: usize, window: &VisitWindow| match (idx, window.bridge, fixed_bounds) {
                    (0, true, Some((earliest, latest))) => (window.earliest.min(earliest), window.latest.max(latest)),
                    _ => (window.earliest, window.latest),
                };

                if chain.iter().enumerate().all(|(idx, (_, window))| !fits(bounds(idx, window))) {
                    let (_, own) = chain[0];
                    Err(format!(
                        "visit window violation: job '{}' is served from {} to {}, its window is {} to {}, vehicle id '{}', shift index: {}",
                        activity.job_id,
                        format_time(service_start),
                        format_time(departure),
                        format_time(own.earliest),
                        format_time(own.latest),
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
