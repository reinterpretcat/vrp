#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/reserved_time_test.rs"]
mod reserved_time_test;

use crate::models::common::*;
use crate::models::problem::{ActivityCost, Actor, TransportCost, TravelTime};
use crate::models::solution::{Activity, Route};
use rosomaxa::prelude::GenericError;
use std::collections::HashMap;
use std::sync::Arc;

/// Represent a reserved time span entity.
#[derive(Clone, Debug)]
pub struct ReservedTimeSpan {
    /// A specific time span when an extra reserved duration should be applied.
    pub time: TimeSpan,
    /// An extra duration to be applied at given time.
    pub duration: Duration,
}

impl ReservedTimeSpan {
    /// Converts `ReservedTimeSpan` to `ReservedTimeWindow`.
    pub fn to_reserved_time_window(&self, offset: Timestamp) -> ReservedTimeWindow {
        ReservedTimeWindow { time: self.time.to_time_window(offset), duration: self.duration }
    }
}

/// Represent a reserved time window entity.
#[derive(Clone, Debug)]
pub struct ReservedTimeWindow {
    /// A specific time window when an extra reserved duration should be applied.
    pub time: TimeWindow,
    /// An extra duration to be applied at given time.
    pub duration: Duration,
}

/// Specifies reserved time index type.
pub type ReservedTimesIndex = HashMap<Arc<Actor>, Vec<ReservedTimeSpan>>;

/// Specifies a function which returns an extra reserved time window for given actor. This reserved
/// time should be considered for planning.
pub(crate) type ReservedTimesFn = Arc<dyn Fn(&Route, &TimeWindow) -> Option<ReservedTimeWindow> + Send + Sync>;

/// Provides way to calculate activity costs which might contain reserved time.
pub struct DynamicActivityCost {
    reserved_times_fn: ReservedTimesFn,
}

impl DynamicActivityCost {
    /// Creates a new instance of `DynamicActivityCost` with given reserved time function.
    pub fn new(reserved_times_index: ReservedTimesIndex) -> Result<Self, GenericError> {
        Ok(Self { reserved_times_fn: create_reserved_times_fn(reserved_times_index)? })
    }
}

impl ActivityCost for DynamicActivityCost {
    fn estimate_departure(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp {
        let activity_start = arrival.max(activity.place.time.start);
        let departure = activity_start + activity.place.duration;
        let schedule = TimeWindow::new(arrival, departure);

        (self.reserved_times_fn)(route, &schedule).map_or(departure, |reserved_time| {
            // NOTE we ignore reserved_time.time.start and consider the latest possible time only
            let reserved_tw = &reserved_time.time;
            let reserved_tw = TimeWindow::new(reserved_tw.end, reserved_tw.end + reserved_time.duration);

            assert!(reserved_tw.intersects(&schedule));

            let activity_tw = &activity.place.time;

            let extra_duration = if reserved_tw.start < activity_tw.start {
                let waiting_time = TimeWindow::new(arrival, activity_tw.start);
                let overlapping = waiting_time.overlapping(&reserved_tw).map(|tw| tw.duration()).unwrap_or_default();

                reserved_time.duration - overlapping
            } else {
                reserved_time.duration
            };

            // NOTE: do not allow to start or restart work after break finished
            if activity_start + extra_duration > activity.place.time.end {
                // TODO this branch is the reason why departure rescheduling is disabled.
                //      theoretically, rescheduling should be aware somehow about dynamic costs
                Duration::MAX
            } else {
                departure + extra_duration
            }
        })
    }

    fn estimate_arrival(&self, route: &Route, activity: &Activity, departure: Timestamp) -> Timestamp {
        let arrival = activity.place.time.end.min(departure - activity.place.duration);
        let schedule = TimeWindow::new(arrival, departure);

        (self.reserved_times_fn)(route, &schedule)
            .map_or(arrival, |reserved_time| (arrival - reserved_time.duration).max(activity.place.time.start))
    }
}

/// Provides way to calculate transport costs which might contain reserved time.
pub struct DynamicTransportCost {
    reserved_times_fn: ReservedTimesFn,
    inner: Arc<dyn TransportCost>,
}

impl DynamicTransportCost {
    /// Creates a new instance of `DynamicTransportCost`.
    pub fn new(reserved_times_index: ReservedTimesIndex, inner: Arc<dyn TransportCost>) -> Result<Self, GenericError> {
        Ok(Self { reserved_times_fn: create_reserved_times_fn(reserved_times_index)?, inner })
    }
}

impl TransportCost for DynamicTransportCost {
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration {
        self.inner.duration_approx(profile, from, to)
    }

    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        self.inner.distance_approx(profile, from, to)
    }

    fn duration(&self, route: &Route, from: Location, to: Location, travel_time: TravelTime) -> Duration {
        let duration = self.inner.duration(route, from, to, travel_time);

        let time_window = match travel_time {
            TravelTime::Arrival(arrival) => TimeWindow::new(arrival - duration, arrival),
            TravelTime::Departure(departure) => TimeWindow::new(departure, departure + duration),
        };

        (self.reserved_times_fn)(route, &time_window)
            .map_or(duration, |reserved_time| duration + reserved_time.duration)
    }

    fn distance(&self, route: &Route, from: Location, to: Location, travel_time: TravelTime) -> Distance {
        self.inner.distance(route, from, to, travel_time)
    }
}

/// Optimizes reserved time schedules by rescheduling it to earlier time (e.g. to avoid transit stops,
/// reduce waiting time).
pub(crate) fn optimize_reserved_times_schedule(route: &mut Route, reserved_times_fn: &ReservedTimesFn) {
    // NOTE run in this order as reducing waiting time can be also applied on top of avoiding travel time
    avoid_reserved_time_when_driving(route, reserved_times_fn);
    reduce_waiting_by_reserved_time(route, reserved_times_fn);
}

fn avoid_reserved_time_when_driving(route: &mut Route, reserved_times_fn: &ReservedTimesFn) {
    // NOTE assume reserved times has no intersection
    let schedule_shifts = route
        .tour
        .legs()
        .filter_map(|(leg, idx)| match &leg {
            &[from, to] => Some((from, to, idx)),
            _ => None,
        })
        .filter_map(|(from, to, idx)| {
            let travel_tw = TimeWindow::new(from.schedule.departure, to.schedule.arrival);
            reserved_times_fn(route, &travel_tw).map(|reserved_time| (idx, from, reserved_time))
        })
        .filter(|(_, from, reserved_time)| from.schedule.departure > reserved_time.time.start)
        .map(|(idx, _, reserved_time)| (idx, reserved_time.duration))
        .collect::<Vec<_>>();

    schedule_shifts.into_iter().for_each(|(idx, duration)| {
        route.tour.get_mut(idx).unwrap().schedule.departure += duration;
    });
}

fn reduce_waiting_by_reserved_time(_route: &mut Route, _reserved_times_fn: &ReservedTimesFn) {
    // TODO: could be added if necessary, but it should be thought carefully to keep solution feasibility
}

/// Creates a reserved time function from reserved time index.
pub(crate) fn create_reserved_times_fn(
    reserved_times_index: ReservedTimesIndex,
) -> Result<ReservedTimesFn, GenericError> {
    if reserved_times_index.is_empty() {
        return Ok(Arc::new(|_, _| None));
    }

    let reserved_times = reserved_times_index.into_iter().try_fold(
        HashMap::<_, (Vec<_>, Vec<_>)>::new(),
        |mut acc, (actor, mut times)| {
            // NOTE do not allow different types to simplify interval searching
            let are_same_types = times.windows(2).all(|pair| {
                if let [ReservedTimeSpan { time: a, .. }, ReservedTimeSpan { time: b, .. }] = pair {
                    matches!(
                        (a, b),
                        (TimeSpan::Window(_), TimeSpan::Window(_)) | (TimeSpan::Offset(_), TimeSpan::Offset(_))
                    )
                } else {
                    false
                }
            });

            if !are_same_types {
                return Err("has reserved types of different time span types".to_string());
            }

            times.sort_by(|ReservedTimeSpan { time: a, .. }, ReservedTimeSpan { time: b, .. }| match (a, b) {
                (TimeSpan::Window(a), TimeSpan::Window(b)) => a.start.cmp(&b.start),
                (TimeSpan::Offset(a), TimeSpan::Offset(b)) => a.start.cmp(&b.start),
                _ => unreachable!(),
            });
            let has_no_intersections = times.windows(2).all(|pair| {
                if let [ReservedTimeSpan { time: a, .. }, ReservedTimeSpan { time: b, .. }] = pair {
                    !a.intersects(Timestamp::default(), &b.to_time_window(Timestamp::default()))
                } else {
                    false
                }
            });

            if has_no_intersections {
                let (indices, intervals): (Vec<_>, Vec<_>) = times
                    .into_iter()
                    .map(|span| {
                        let start = match &span.time {
                            TimeSpan::Window(time) => time.end,
                            TimeSpan::Offset(time) => time.end,
                        };

                        (start as u64, span)
                    })
                    .unzip();
                acc.insert(actor, (indices, intervals));

                Ok(acc)
            } else {
                Err("reserved times have intersections".to_string())
            }
        },
    )?;

    // NOTE: this function considers only latest time from reserved time
    //       reserved_time.time.start is ignored and should be handled by post processing
    Ok(Arc::new(move |route: &Route, time_window: &TimeWindow| {
        reserved_times.get(&route.actor).and_then(|(indices, intervals)| {
            let offset = route.tour.start().map(|a| a.schedule.departure).unwrap_or_default();

            // NOTE map external absolute time window to time span's start/end
            let (interval_start, interval_end) = match intervals.first().map(|rt| &rt.time) {
                Some(TimeSpan::Offset(_)) => (time_window.start - offset, time_window.end - offset),
                Some(TimeSpan::Window(_)) => (time_window.start, time_window.end),
                _ => unreachable!(),
            };

            match indices.binary_search(&(interval_start as u64)) {
                Ok(idx) => intervals.get(idx),
                Err(idx) => (idx.max(1) - 1..=idx) // NOTE left (earliest) wins
                    .map(|idx| intervals.get(idx))
                    .find(|reserved_time| {
                        reserved_time.map_or(false, |reserved_time| {
                            let (reserved_start, reserved_end) = match &reserved_time.time {
                                TimeSpan::Offset(to) => (to.end, to.end + reserved_time.duration),
                                TimeSpan::Window(tw) => (tw.end, tw.end + reserved_time.duration),
                            };

                            // NOTE use exclusive intersection
                            interval_start < reserved_end && reserved_start < interval_end
                        })
                    })
                    .flatten(),
            }
            .map(|reserved_time| reserved_time.to_reserved_time_window(offset))
        })
    }))
}
