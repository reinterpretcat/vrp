//! Per-shift windows a tagged visit has to lie in, and the tag a visit carries.

use crate::models::common::{Dimensions, Timestamp};
use std::collections::HashMap;

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
    /// The windows by name.
    pub windows: HashMap<String, VisitWindow>,
}

impl VisitWindows {
    /// The window `tag` names and its fallback chain, own window first. The chain stops at a name
    /// the shift does not have and never visits a window twice.
    pub fn chain(&self, tag: &str) -> Vec<(&str, &VisitWindow)> {
        let mut chain: Vec<(&str, &VisitWindow)> = Vec::new();
        let mut next = Some(tag);

        while let Some(name) = next {
            let Some((key, window)) = self.windows.get_key_value(name) else { break };

            if chain.iter().any(|(seen, _)| *seen == key.as_str()) {
                break;
            }

            chain.push((key.as_str(), window));
            next = window.fallback.as_deref();
        }

        chain
    }
}

custom_dimension!(pub VisitWindows typeof VisitWindows);
custom_dimension!(pub VisitWindowTag typeof String);
