//! Per-shift windows a tagged visit has to lie in, and the tag a visit carries.

use crate::models::common::{Dimensions, Timestamp};

/// Which of a shift's visit windows a job keeps to. A job without one is fixed: its own time
/// window and the shift's job times bind it, no visit window does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisitWindowKind {
    /// A recurring visit with no preferred time.
    Recurring,
    /// Every other visit with no preferred time.
    Other,
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
    /// The window every other tagged visit keeps to.
    pub other: Option<VisitWindow>,
    /// Whether a recurring visit may use the other window when it does not fit its own.
    pub overflow: bool,
}

impl VisitWindows {
    /// The window a job of `kind` may lie in on this shift. With overflow, a recurring job may lie
    /// anywhere from the earlier start to the later end of both windows; the overflow objective
    /// counts it when it leaves its own. Overflow without an other window is no overflow.
    pub fn bounds_for(&self, kind: &VisitWindowKind) -> Option<(Timestamp, Timestamp)> {
        match kind {
            VisitWindowKind::Other => self.other.as_ref().map(|w| (w.earliest, w.latest)),
            VisitWindowKind::Recurring => match (&self.recurring, &self.other, self.overflow) {
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
