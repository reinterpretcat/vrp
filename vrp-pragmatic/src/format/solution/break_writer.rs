use super::*;
use std::cmp::Ordering;
use vrp_core::construction::enablers::ReservedTimesIndex;
use vrp_core::models::common::TimeWindow;
use vrp_core::models::solution::Route;
use vrp_core::utils::compare_floats;

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
        .map(|rt| (TimeWindow::new(rt.time.end, rt.time.end + rt.duration), rt))
        .filter(|(reserved_tw, _)| shift_time.intersects(reserved_tw))
        .for_each(|(reserved_tw, reserved_time)| {
            // NOTE scan and insert a new stop if necessary
            let travel_break_info = tour.stops.windows(2).enumerate().find_map(|(leg_idx, stops)| {
                if let &[prev, next] = &stops {
                    let travel_tw =
                        TimeWindow::new(parse_time(&prev.schedule().departure), parse_time(&next.schedule().arrival));

                    if travel_tw.intersects_exclusive(&reserved_tw) {
                        // NOTE: should be moved to the last activity on previous stop by post-processing
                        return if reserved_time.time.start < travel_tw.start {
                            let break_tw = TimeWindow::new(travel_tw.start - reserved_tw.duration(), travel_tw.start);
                            Some(BreakInsertionInfo::TransitBreakMoved { leg_idx, break_tw })
                        } else {
                            Some(BreakInsertionInfo::TransitBreakUsed { leg_idx, load: prev.load().clone() })
                        };
                    }
                }

                None
            });

            if let Some(BreakInsertionInfo::TransitBreakUsed { leg_idx, load }) = travel_break_info.clone() {
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
            tour.stops.iter_mut().enumerate().for_each(|(current_idx, stop)| {
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
                        .unwrap_or(stop.activities().len());

                    let break_cost = break_time as f64 * route.actor.vehicle.costs.per_service_time;

                    let activities = match stop {
                        Stop::Point(point) => {
                            tour.statistic.cost += break_cost;
                            &mut point.activities
                        }
                        Stop::Transit(transit) => {
                            tour.statistic.times.driving -= break_time;
                            &mut transit.activities
                        }
                    };

                    let activity_time = match &travel_break_info {
                        Some(BreakInsertionInfo::TransitBreakMoved { break_tw, leg_idx })
                            if *leg_idx == current_idx =>
                        {
                            tour.statistic.cost -= break_cost;
                            tour.statistic.times.driving -= break_time;
                            break_tw
                        }
                        _ => &reserved_tw,
                    };

                    activities.insert(
                        break_idx,
                        ApiActivity {
                            job_id: "break".to_string(),
                            activity_type: "break".to_string(),
                            location: None,
                            time: Some(Interval {
                                start: format_time(activity_time.start),
                                end: format_time(activity_time.end),
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

                    activities.sort_by(|a, b| match (&a.time, &b.time) {
                        (Some(a), Some(b)) => compare_floats(parse_time(&a.start), parse_time(&b.start)),
                        (Some(_), None) => Ordering::Greater,
                        (None, Some(_)) => Ordering::Less,
                        (None, None) => Ordering::Equal,
                    })
                }
            });

            tour.statistic.times.break_time += break_time;
        });
}

#[derive(Clone)]
enum BreakInsertionInfo {
    TransitBreakUsed { leg_idx: usize, load: Vec<i32> },
    TransitBreakMoved { leg_idx: usize, break_tw: TimeWindow },
}
