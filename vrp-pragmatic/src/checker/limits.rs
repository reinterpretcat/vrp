#[cfg(test)]
#[path = "../../tests/unit/checker/limits_test.rs"]
mod limits_test;

use super::*;

/// Check that shift limits are not violated:
/// * max shift time
/// * max distance
///
/// NOTE to ensure distance/duration correctness, routing check should be performed first.
pub fn check_limits(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each::<_, Result<_, String>>(|tour| {
        let vehicle = context.get_vehicle(&tour.vehicle_id)?;

        if let Some(ref limits) = vehicle.limits {
            if let Some(max_distance) = limits.max_distance {
                if tour.statistic.distance as f64 > max_distance {
                    return Err(format!(
                        "max distance limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        max_distance, tour.statistic.distance, tour.vehicle_id, tour.shift_index
                    ));
                }
            }

            if let Some(shift_time) = limits.shift_time {
                if tour.statistic.duration as f64 > shift_time {
                    return Err(format!(
                        "shift time limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        shift_time, tour.statistic.duration, tour.vehicle_id, tour.shift_index
                    ));
                }
            }

            if let Some(tour_size_limit) = limits.tour_size {
                let shift = context.get_vehicle_shift(tour)?;

                let extra_activities = if shift.end.is_some() { 2 } else { 1 };
                let tour_activities = tour.stops.iter().flat_map(|stop| stop.activities.iter()).count();
                let tour_activities = if tour_activities > extra_activities { tour_activities - extra_activities } else { 0 };

                if tour_activities > tour_size_limit {
                    return Err(format!(
                        "tour size limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        tour_size_limit, tour_activities, tour.vehicle_id, tour.shift_index
                    ))
                }
            }
        }

        Ok(())
    })
}
