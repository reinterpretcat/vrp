use super::*;
use vrp_core::construction::enablers::ReservedTimesIndex;
use vrp_core::models::common::TimeWindow;
use vrp_core::models::solution::Route;

/// Converts reserved time duration applied to activity or travel time to break activity.
pub(crate) fn insert_reserved_times_as_breaks(
    route: &Route,
    tour: &mut Tour,
    reserved_times_index: &ReservedTimesIndex,
) {
    let shift_time = route
        .tour
        .start()
        .zip(route.tour.end())
        .map(|(start, end)| TimeWindow::new(start.schedule.departure, end.schedule.arrival))
        .expect("empty tour");

    reserved_times_index
        .get(&route.actor)
        .iter()
        .flat_map(|times| times.iter())
        .map(|reserved_time| reserved_time.to_reserved_time_window(shift_time.start))
        // NOTE: we ignore reserved.time.start here as it is ignored in core implementation
        .map(|rt| (TimeWindow::new(rt.time.end, rt.time.end + rt.duration), rt))
        .filter(|(reserved_tw, _)| shift_time.intersects(reserved_tw))
        .for_each(|(reserved_tw, reserved_time)| {
            // NOTE scan and insert a new stop if necessary
            if let Some((leg_idx, load)) = tour
                .stops
                .windows(2)
                .enumerate()
                .filter_map(|(leg_idx, stops)| {
                    if let &[prev, next] = &stops {
                        let travel_tw = TimeWindow::new(
                            parse_time(&prev.schedule().departure),
                            parse_time(&next.schedule().arrival),
                        );

                        if travel_tw.intersects_exclusive(&reserved_tw) {
                            return Some((leg_idx, prev.load().clone()));
                        }
                    }

                    None
                })
                .next()
            {
                tour.stops.insert(
                    leg_idx + 1,
                    Stop::Transit(TransitStop {
                        time: ApiSchedule {
                            arrival: format_time(reserved_tw.start),
                            departure: format_time(reserved_tw.end),
                        },
                        load,
                        activities: vec![],
                    }),
                )
            }

            let break_time = reserved_time.duration as i64;

            // NOTE insert activity
            tour.stops.iter_mut().for_each(|stop| {
                let stop_tw =
                    TimeWindow::new(parse_time(&stop.schedule().arrival), parse_time(&stop.schedule().departure));

                if stop_tw.intersects_exclusive(&reserved_tw) {
                    let break_idx = stop
                        .activities()
                        .iter()
                        .enumerate()
                        .filter_map(|(activity_idx, activity)| {
                            let activity_tw = activity.time.as_ref().map_or(stop_tw.clone(), |interval| {
                                TimeWindow::new(parse_time(&interval.start), parse_time(&interval.end))
                            });

                            if activity_tw.intersects(&reserved_tw) {
                                Some(activity_idx + 1)
                            } else {
                                None
                            }
                        })
                        .next()
                        .unwrap_or(0);

                    // TODO costs may not match?
                    let activities = match stop {
                        Stop::Point(point) => {
                            tour.statistic.cost += break_time as f64 * route.actor.vehicle.costs.per_service_time;
                            &mut point.activities
                        }
                        Stop::Transit(transit) => {
                            tour.statistic.times.driving -= break_time;
                            &mut transit.activities
                        }
                    };

                    activities.insert(
                        break_idx,
                        ApiActivity {
                            job_id: "break".to_string(),
                            activity_type: "break".to_string(),
                            location: None,
                            time: Some(Interval {
                                start: format_time(reserved_tw.start),
                                end: format_time(reserved_tw.end),
                            }),
                            job_tag: None,
                            commute: None,
                        },
                    );

                    activities.iter_mut().enumerate().filter(|(idx, _)| *idx != break_idx).for_each(|(_, activity)| {
                        if let Some(time) = &mut activity.time {
                            let start = parse_time(&time.start);
                            let end = parse_time(&time.end);
                            let overlap = TimeWindow::new(start, end).overlapping(&reserved_tw);

                            if let Some(overlap) = overlap {
                                let extra_time = reserved_tw.end - overlap.end + overlap.duration();
                                time.end = format_time(end + extra_time);
                            }
                        }
                    });
                }
            });

            tour.statistic.times.break_time += break_time;
        });
}
