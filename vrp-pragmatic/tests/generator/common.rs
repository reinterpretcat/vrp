use super::*;
use crate::json::Location;
use crate::{format_time, parse_time};
use std::cmp::Ordering::Less;
use std::ops::Range;
use std::time::Duration;
use vrp_core::models::common::TimeWindow;

/// Creates `Duration` from hours amount.
pub fn from_hours(hours: i32) -> Duration {
    Duration::from_secs((hours as u64) * 3600)
}

prop_compose! {
    /// Generates locations.
    pub fn generate_simple_locations(range: Range<i32>)
        (latitude in range)
    -> Location {
        Location::new(latitude as f64, 0.)
    }
}

prop_compose! {
    /// Generates time window.
    fn generate_time_window_fixed_raw(day: f64, start_offsets: Vec<u64>, durations: Vec<u64>)
        (start_offset in from_uints(start_offsets.clone()), duration in from_uints(durations.clone()))
         -> TimeWindow {
        let start = day + start_offset as f64;
        let end = start + duration as f64;

        TimeWindow::new(start, end)
    }
}

prop_compose! {
    /// Generates multiple time windows.
    pub fn generate_multiple_time_windows_fixed(start_date: &str,
                                           start_offsets: Vec<Duration>,
                                           durations: Vec<Duration>,
                                           amount_range: Range<usize>)
        (time_windows in prop::collection::vec(generate_time_window_fixed_raw(
                                                parse_time(&start_date.to_string()),
                                                start_offsets.iter().map(|d| d.as_secs()).collect(),
                                                durations.iter().map(|d| d.as_secs()).collect()),
                                               amount_range)
            .prop_filter("Filter out time window intersections.", |tws| {
                Some((0..).zip(tws.iter())).map(|tws| {
                let tws = tws.collect::<Vec<_>>();
                tws.iter().all(|(idx, tw)| tws.iter()
                    .filter(|(idx_other, _)| *idx != *idx_other)
                    .all(|(_, tw_other)| !tw.intersects(&tw_other)))

                }).unwrap_or(false)
            })) -> Vec<Vec<String>> {

        let mut time_windows = time_windows;
        time_windows.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(Less));

        time_windows.iter().map(|tw| vec![format_time(tw.start), format_time(tw.end)]).collect()
    }
}
