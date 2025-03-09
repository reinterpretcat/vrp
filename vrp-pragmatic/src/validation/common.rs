use crate::parse_time_safe;
use std::collections::HashSet;
use vrp_core::models::common::TimeWindow;

/// Checks time window rules.
pub fn check_raw_time_windows(tws: &[Vec<String>], skip_intersection_check: bool) -> bool {
    let tws = get_time_windows(tws);
    check_time_windows(&tws, skip_intersection_check)
}

/// Checks time window rules.
pub fn check_time_windows(tws: &[Option<TimeWindow>], skip_intersection_check: bool) -> bool {
    if tws.iter().any(|tw| tw.is_none()) {
        false
    } else {
        let mut tws = tws.iter().map(|tw| tw.clone().unwrap()).collect::<Vec<_>>();
        if let [a] = tws.as_slice() {
            a.start <= a.end
        } else {
            tws.sort_by(|a, b| a.start.total_cmp(&b.start));
            tws.windows(2).any(|pair| {
                if let [a, b] = pair {
                    a.start <= a.end && b.start <= b.end && (skip_intersection_check || !a.intersects(b))
                } else {
                    false
                }
            })
        }
    }
}

pub fn get_time_window(start: &str, end: &str) -> Option<TimeWindow> {
    let start = parse_time_safe(start);
    let end = parse_time_safe(end);

    if let (Some(start), Some(end)) = (start.ok(), end.ok()) { Some(TimeWindow::new(start, end)) } else { None }
}

/// Get time windows.
pub fn get_time_window_from_vec(tw: &[String]) -> Option<TimeWindow> {
    if tw.len() != 2 { None } else { get_time_window(tw.first().unwrap(), tw.last().unwrap()) }
}

/// Get time windows.
pub fn get_time_windows(tws: &[Vec<String>]) -> Vec<Option<TimeWindow>> {
    tws.iter().map(|tw| get_time_window_from_vec(tw)).collect::<Vec<_>>()
}

/// Returns a duplicates
pub fn get_duplicates<'a>(items: impl Iterator<Item = &'a String>) -> Option<Vec<String>> {
    let mut ids = HashSet::<_>::default();
    let duplicates =
        items.filter_map(move |id| if ids.insert(id) { None } else { Some(id.clone()) }).collect::<HashSet<_>>();

    if duplicates.is_empty() {
        None
    } else {
        let mut duplicates = duplicates.into_iter().collect::<Vec<_>>();
        duplicates.sort();
        Some(duplicates)
    }
}
