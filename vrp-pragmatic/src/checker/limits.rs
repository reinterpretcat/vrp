#[cfg(test)]
#[path = "../../tests/unit/checker/limits_test.rs"]
mod limits_test;

use super::*;
use crate::utils::combine_error_results;

/// NOTE to ensure distance/duration correctness, routing check should be performed first.
pub fn check_limits(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[check_shift_limits(context), check_shift_time(context)])
}

/// Check that shift limits are not violated:
/// * max shift time
/// * max distance
fn check_shift_limits(context: &CheckerContext) -> Result<(), GenericError> {
    context.solution.tours.iter().try_for_each::<_, Result<_, GenericError>>(|tour| {
        let vehicle = context.get_vehicle(&tour.vehicle_id)?;

        if let Some(ref limits) = vehicle.limits {
            if let Some(max_distance) = limits.max_distance {
                if tour.statistic.distance as f64 > max_distance {
                    return Err(format!(
                        "max distance limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        max_distance, tour.statistic.distance, tour.vehicle_id, tour.shift_index
                    ).into());
                }
            }

            if let Some(max_duration) = limits.max_duration {
                if tour.statistic.duration as f64 > max_duration {
                    return Err(format!(
                        "shift time limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        max_duration, tour.statistic.duration, tour.vehicle_id, tour.shift_index
                    ).into());
                }
            }

            if let Some(tour_size_limit) = limits.tour_size {
                let shift = context.get_vehicle_shift(tour)?;

                let extra_activities = if shift.end.is_some() { 2 } else { 1 };
                let tour_activities = tour.stops.iter().flat_map(|stop| stop.activities()).count();
                let tour_activities = if tour_activities > extra_activities { tour_activities - extra_activities } else { 0 };

                if tour_activities > tour_size_limit {
                    return Err(format!(
                        "tour size limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        tour_size_limit, tour_activities, tour.vehicle_id, tour.shift_index
                    ).into())
                }
            }
        }

        Ok(())
    })
}

fn check_shift_time(context: &CheckerContext) -> Result<(), GenericError> {
    context.solution.tours.iter().try_for_each::<_, Result<_, GenericError>>(|tour| {
        let vehicle = context.get_vehicle(&tour.vehicle_id)?;

        let (start, end) = tour.stops.first().zip(tour.stops.last()).ok_or("empty tour")?;

        let departure = parse_time(&start.schedule().departure);
        let arrival = parse_time(&end.schedule().arrival);

        let has_match = vehicle
            .shifts
            .iter()
            .map(|shift| {
                let start = parse_time(&shift.start.earliest);
                let end = shift.end.as_ref().map(|end| parse_time(&end.latest)).unwrap_or(f64::MAX);

                (start, end)
            })
            .any(|(start, end)| departure >= start && arrival <= end);

        if !has_match {
            Err(format!(
                "tour time is outside shift time, vehicle id '{}', shift index: {}",
                tour.vehicle_id, tour.shift_index
            )
            .into())
        } else {
            Ok(())
        }
    })
}
