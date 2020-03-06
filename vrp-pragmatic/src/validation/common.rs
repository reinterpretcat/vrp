use crate::parse_time_safe;
use std::cmp::Ordering::Less;
use std::collections::HashSet;
use vrp_core::models::common::TimeWindow;

/// Check time window rules.
pub fn check_time_windows(tws: &Vec<Vec<String>>) -> bool {
    let tws = tws
        .iter()
        .map(|tw| {
            if tw.len() != 2 {
                (None, None)
            } else {
                let start = parse_time_safe(tw.first().unwrap());
                let end = parse_time_safe(tw.last().unwrap());
                (start.ok(), end.ok())
            }
        })
        .collect::<Vec<_>>();

    if tws.iter().any(|(start, end)| start.is_none() || end.is_none()) {
        false
    } else {
        let mut tws =
            tws.into_iter().map(|(start, end)| TimeWindow::new(start.unwrap(), end.unwrap())).collect::<Vec<_>>();
        if let &[a] = &tws.as_slice() {
            a.start <= a.end
        } else {
            tws.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(Less));
            tws.windows(2).any(|pair| {
                if let &[a, b] = &pair {
                    a.start <= a.end && b.start <= b.end && !a.intersects(b)
                } else {
                    false
                }
            })
        }
    }
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
