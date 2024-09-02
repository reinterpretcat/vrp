use super::*;
use crate::format::Location;
use crate::{format_time, parse_time};
use std::cmp::Ordering::Less;
use std::ops::Range;
use vrp_core::models::common::{Duration, TimeWindow, Timestamp};

/// Creates `std::time::Duration` from hours amount.
pub fn from_hours(hours: i32) -> std::time::Duration {
    std::time::Duration::from_secs((hours as u64) * 3600)
}

prop_compose! {
    /// Generates location inside given bounding box.
    pub fn generate_location(bounding_box: &((f64, f64), (f64, f64)))
    (
     lat in bounding_box.0.0..bounding_box.1.0,
     lng in bounding_box.0.1..bounding_box.1.1
    ) -> Location {
        Location::Coordinate { lat, lng }
    }
}

prop_compose! {
    /// Generates time window.
    fn generate_time_window_fixed_raw(day: Timestamp, start_offsets: Vec<Timestamp>, durations: Vec<Duration>)
    (
     start_offset in from_long(start_offsets),
     duration in from_long(durations)
    ) -> TimeWindow {

        let start = day + start_offset;
        let end = start + duration;

        TimeWindow::new(start, end)
    }
}

prop_compose! {
    /// Generates multiple time windows.
    pub fn generate_multiple_time_windows_fixed(start_date: &str,
                                           start_offsets: Vec<std::time::Duration>,
                                           durations: Vec<std::time::Duration>,
                                           amount_range: Range<usize>)
    (time_windows in prop::collection::vec(generate_time_window_fixed_raw(
                                            parse_time(start_date),
                                            start_offsets.iter().map(|d| d.as_secs() as Duration).collect(),
                                            durations.iter().map(|d| d.as_secs() as Duration).collect()),
                                           amount_range)
    .prop_filter("Filter out time window intersections.", |tws| {
        Some((0..).zip(tws.iter())).map(|tws| {
        let tws = tws.collect::<Vec<_>>();
        tws.iter().all(|(idx, tw)| tws.iter()
            .filter(|(idx_other, _)| *idx != *idx_other)
            .all(|(_, tw_other)| !tw.intersects(tw_other)))

        }).unwrap_or(false)
    })) -> Vec<Vec<String>> {

        let mut time_windows = time_windows;
        time_windows.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(Less));

        time_windows.iter().map(|tw| vec![format_time(tw.start), format_time(tw.end)]).collect()
    }
}

prop_compose! {
    /// Generates durations in range.
    pub fn generate_durations(range: Range<Duration>)(duration in range) -> Duration {
        duration
    }
}

prop_compose! {
    /// Generates no locations.
    pub fn generate_no_locations()(_ in ".*") -> Option<Location> {
        None
    }
}

prop_compose! {
    /// Generates no time windows.
    pub fn generate_no_time_windows()(_ in ".*") -> Option<Vec<Vec<String>>> {
        None
    }
}
