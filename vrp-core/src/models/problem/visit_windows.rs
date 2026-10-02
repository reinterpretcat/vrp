//! Per-shift windows a tagged visit has to lie in, and the tag a visit carries.

use crate::models::common::{Dimensions, Timestamp};

/// Which of a shift's visit windows a job keeps to. A job without one is fixed: its own time
/// window and the shift's job times bind it, no visit window does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisitWindowKind {
    /// A recurring visit with no preferred time.
    Recurring,
    /// Every non-recurring visit with no preferred time.
    NonRecurring,
}

/// A window a visit lies entirely inside: service starts at or after `earliest` and the visit is
/// departed at or before `latest`.
#[derive(Clone, Debug)]
pub struct VisitWindow {
    /// Earliest service start.
    pub earliest: Timestamp,
    /// Latest departure.
    pub latest: Timestamp,
}

/// The visit windows of one shift.
#[derive(Clone, Debug)]
pub struct VisitWindows {
    /// The window recurring visits keep to.
    pub recurring: Option<VisitWindow>,
    /// The window non-recurring visits keep to.
    pub non_recurring: Option<VisitWindow>,
    /// Whether a recurring visit may use the non-recurring window when it does not fit its own.
    pub overflow: bool,
}

impl VisitWindows {
    /// The window a job of `kind` keeps to on this shift, and the one it may fall back to when it
    /// does not fit: the non-recurring window for a recurring job with overflow, nothing otherwise.
    /// Overflow without an non-recurring window is no overflow.
    pub fn windows_for(
        &self,
        kind: &VisitWindowKind,
    ) -> Option<((Timestamp, Timestamp), Option<(Timestamp, Timestamp)>)> {
        let bounds = |window: &VisitWindow| (window.earliest, window.latest);

        match kind {
            VisitWindowKind::NonRecurring => self.non_recurring.as_ref().map(|window| (bounds(window), None)),
            VisitWindowKind::Recurring => self.recurring.as_ref().map(|recurring| {
                (bounds(recurring), self.non_recurring.as_ref().filter(|_| self.overflow).map(bounds))
            }),
        }
    }

    /// Everything a job of `kind` may lie in on this shift: with overflow, a recurring job may lie
    /// anywhere from the earlier start to the later end of both windows. Overflow without a non-recurring
    /// window is no overflow.
    pub fn bounds_for(&self, kind: &VisitWindowKind) -> Option<(Timestamp, Timestamp)> {
        match kind {
            VisitWindowKind::NonRecurring => self.non_recurring.as_ref().map(|w| (w.earliest, w.latest)),
            VisitWindowKind::Recurring => match (&self.recurring, &self.non_recurring, self.overflow) {
                (Some(r), Some(o), true) => Some((r.earliest.min(o.earliest), r.latest.max(o.latest))),
                (Some(r), _, _) => Some((r.earliest, r.latest)),
                (None, _, _) => None,
            },
        }
    }

    /// True when a recurring visit served from `start` to `end` lies outside its own window.
    pub fn is_overflow(&self, start: Timestamp, end: Timestamp) -> bool {
        self.recurring.as_ref().is_some_and(|r| start < r.earliest || end > r.latest)
    }
}

custom_dimension!(pub VisitWindows typeof VisitWindows);
custom_dimension!(pub VisitWindowKind typeof VisitWindowKind);
