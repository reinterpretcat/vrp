#[cfg(test)]
#[path = "../../tests/unit/checker/job_times_test.rs"]
mod job_times_test;

use super::*;
use crate::format_time;
use crate::utils::combine_error_results;
use vrp_core::prelude::GenericResult;

/// Checks that a tour keeps the appointment bounds its shift declares:
/// the first job does not start before `earliest_first`, and the last job's
/// departure is not after `latest_last`.
pub fn check_job_times(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[check_job_time_bounds(context)])
}

fn check_job_time_bounds(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().try_for_each(|tour| {
        let shift = context.get_vehicle_shift(tour)?;
        let Some(job_times) = shift.job_times.as_ref() else { return Ok(()) };

        let job_activities = tour
            .stops
            .iter()
            .flat_map(|stop| stop.activities().iter().map(move |activity| (stop, activity)))
            .filter(|(_, activity)| is_stop_activity(activity))
            .collect::<Vec<_>>();

        let (Some(&(first_stop, first_activity)), Some(&(last_stop, last_activity))) =
            (job_activities.first(), job_activities.last())
        else {
            return Ok(());
        };

        if let Some(earliest_first) = job_times.earliest_first.as_ref() {
            let earliest = parse_time(earliest_first);
            // The service start, not the raw arrival: arriving early and waiting for the
            // job's own window is legal, so the bound must be checked against the moment
            // service actually begins.
            let service_start = context.get_activity_time(first_stop, first_activity).start;

            if service_start < earliest {
                return Err(format!(
                    "job time bound violation: first job starts at {}, earliest allowed is {}, vehicle id '{}', shift index: {}",
                    format_time(service_start),
                    earliest_first,
                    tour.vehicle_id,
                    tour.shift_index
                )
                .into());
            }
        }

        if let Some(latest_last) = job_times.latest_last.as_ref() {
            let latest = parse_time(latest_last);
            // The appointment's own departure, not the stop's: a break, a reload or a recharge
            // sharing the last appointment's stop carries the stop past the bound, and the bound
            // never governed it. Reading the stop here reports a violation against a tour the
            // clamp built correctly.
            let departure = context.get_activity_time(last_stop, last_activity).end;

            if departure > latest {
                return Err(format!(
                    "job time bound violation: last job departs at {}, latest allowed is {}, vehicle id '{}', shift index: {}",
                    format_time(departure),
                    latest_last,
                    tour.vehicle_id,
                    tour.shift_index
                )
                .into());
            }
        }

        Ok(())
    })
}
