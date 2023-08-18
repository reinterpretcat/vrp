use crate::construction::heuristics::{MoveContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::models::solution::{Activity, Route};
use crate::models::{ConstraintViolation, StateKey, ViolationCode};

/// This trait defines multi-trip strategy for constraint extension.
pub trait MultiTrip {
    /// Returns true if job is considered as multi trip marker.
    fn is_marker_job(&self, job: &Job) -> bool;

    /// Returns true if given job is multi trip marker and can be used with given route.
    fn is_marker_assignable(&self, route: &Route, job: &Job) -> bool;

    /// Checks whether vehicle can do a new multi trip.
    fn is_multi_trip_needed(&self, route_ctx: &RouteContext) -> bool;

    /// Gets route intervals split by marker jobs.
    fn get_marker_intervals<'a>(&self, route_ctx: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>>;

    /// Gets interval state key if present.
    fn get_interval_key(&self) -> Option<StateKey>;

    /// Gets all multi trip markers for specific route from jobs collection.
    fn filter_markers<'a>(
        &'a self,
        route: &'a Route,
        jobs: &'a [Job],
    ) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync>;

    /// Evaluates context for insertion possibility.
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation>;

    /// Tries to merge jobs together.
    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode>;

    /// Recalculates states for route context.
    fn accept_route_state(&self, route_ctx: &mut RouteContext);

    /// Accepts solution state, e.g. removes trivial marker jobs.
    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext);
}

/// Returns intervals between vehicle terminal and multi trip activities.
pub fn route_intervals(route: &Route, is_marker_activity: impl Fn(&Activity) -> bool) -> Vec<(usize, usize)> {
    let last_idx = route.tour.total() - 1;
    (0_usize..).zip(route.tour.all_activities()).fold(Vec::<(usize, usize)>::default(), |mut acc, (idx, a)| {
        let is_marker_activity = is_marker_activity(a);
        let is_last = idx == last_idx;

        if is_marker_activity || is_last {
            let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
            let end_idx = if is_last { last_idx } else { idx - 1 };

            if is_marker_activity && is_last {
                acc.push((start_idx, end_idx - 1));
                acc.push((end_idx, end_idx));
            } else {
                acc.push((start_idx, end_idx));
            }
        }

        acc
    })
}
