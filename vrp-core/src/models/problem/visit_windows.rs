//! Per-shift windows a tagged visit has to lie in, and the tag a visit carries.

use crate::models::common::{Dimensions, Timestamp};
use rustc_hash::FxHashMap;

/// A named window a tagged visit lies entirely inside: service starts at or after `earliest` and
/// the visit is departed at or before `latest`.
#[derive(Clone, Debug)]
pub struct VisitWindow {
    /// Earliest service start.
    pub earliest: Timestamp,
    /// Latest departure.
    pub latest: Timestamp,
    /// The window a visit may use instead when it does not fit this one. Need not contain it.
    pub fallback: Option<String>,
    /// Whether the window reaches out to the untagged visits on the visit's route.
    pub bridge: bool,
}

/// The named visit windows of one shift. A job names the window it keeps to with its tag; a job
/// without a tag is fixed: its own time window and the shift's job times bind it, no visit window does.
#[derive(Clone, Debug, Default)]
pub struct VisitWindows {
    /// The windows by name. Read for every visit the schedule estimates, so the tag is hashed with
    /// FxHash rather than SipHash.
    pub windows: FxHashMap<String, VisitWindow>,
}

impl VisitWindows {
    /// The window `tag` names and its fallback chain, own window first. The chain stops at a name
    /// the shift does not have and never visits a window twice.
    pub fn chain(&self, tag: &str) -> Vec<(&str, &VisitWindow)> {
        self.chain_iter(tag).collect()
    }

    /// `chain` without collecting it: read for every visit the schedule estimates, where a vector per
    /// read cost more than the walk. A chain is a handful of windows, so a revisit is found by walking
    /// it again from the start instead of keeping what was seen.
    pub fn chain_iter<'a>(&'a self, tag: &str) -> impl Iterator<Item = (&'a str, &'a VisitWindow)> + Clone + use<'a> {
        let first = self.windows.get_key_value(tag).map(|(key, window)| (key.as_str(), window));
        let mut next = first;
        let mut step = 0_usize;

        std::iter::from_fn(move || {
            let (key, window) = next?;

            if first.is_some_and(|first| self.walk(first).take(step).any(|(_, seen)| std::ptr::eq(seen, window))) {
                next = None;
                return None;
            }

            step += 1;
            next = self.next_after(window);

            Some((key, window))
        })
    }

    /// The window `window` falls back to, when the shift has it.
    fn next_after<'a>(&'a self, window: &VisitWindow) -> Option<(&'a str, &'a VisitWindow)> {
        window
            .fallback
            .as_deref()
            .and_then(|name| self.windows.get_key_value(name))
            .map(|(key, window)| (key.as_str(), window))
    }

    /// The windows along the fallbacks from `first`, as far as the shift has them, revisits included.
    fn walk<'a>(&'a self, first: (&'a str, &'a VisitWindow)) -> impl Iterator<Item = (&'a str, &'a VisitWindow)> + 'a {
        std::iter::successors(Some(first), |(_, window)| self.next_after(window))
    }
}

custom_dimension!(pub VisitWindows typeof VisitWindows);
custom_dimension!(pub VisitWindowTag typeof String);
