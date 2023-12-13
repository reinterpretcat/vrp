use crate::construction::heuristics::{RouteContext, SolutionContext, StateKey};
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Route};
use crate::utils::Either;
use hashbrown::HashSet;
use std::iter::once;
use std::ops::Range;

/// This trait defines a logic to split route into logical intervals by marker jobs.
pub trait RouteIntervals {
    /// Returns true if job is considered as a route interval marker.
    fn is_marker_job(&self, job: &Job) -> bool;

    /// Returns true if given job is a marker job and can be used with given route.
    fn is_marker_assignable(&self, route: &Route, job: &Job) -> bool;

    /// Checks whether vehicle can do a new route interval.
    fn is_new_interval_needed(&self, route_ctx: &RouteContext) -> bool;

    /// Gets route intervals split by marker jobs.
    fn get_marker_intervals<'a>(&self, route_ctx: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>>;

    /// Gets interval state key if present.
    fn get_interval_key(&self) -> Option<StateKey>;

    /// Update route intervals on solution level.
    fn update_solution_intervals(&self, solution_ctx: &mut SolutionContext);
}

/// A no-op implementation of `RouteIntervals`.
#[derive(Default)]
pub struct NoRouteIntervals {}

impl RouteIntervals for NoRouteIntervals {
    fn is_marker_job(&self, _: &Job) -> bool {
        false
    }

    fn is_marker_assignable(&self, _: &Route, _: &Job) -> bool {
        false
    }

    fn is_new_interval_needed(&self, _: &RouteContext) -> bool {
        false
    }

    fn get_marker_intervals<'a>(&self, _: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>> {
        None
    }

    fn get_interval_key(&self) -> Option<StateKey> {
        None
    }

    fn update_solution_intervals(&self, _: &mut SolutionContext) {}
}

/// Provides a basic implementation of route intervals functionality.
#[allow(clippy::type_complexity)]
pub struct FixedRouteIntervals {
    /// Checks whether specified single job is of marker type.
    pub is_marker_single_fn: Box<dyn Fn(&Single) -> bool + Send + Sync>,

    /// Specifies a function which checks whether a new interval is needed for given route.
    pub is_new_interval_needed_fn: Box<dyn Fn(&RouteContext) -> bool + Send + Sync>,

    /// Specifies an obsolete interval function which takes left and right interval range. These intervals are separated by marker job activity.
    pub is_obsolete_interval_fn: Box<dyn Fn(&RouteContext, Range<usize>, Range<usize>) -> bool + Send + Sync>,

    /// Specifies a function which checks whether job can be assigned to a given route.
    pub is_assignable_fn: Box<dyn Fn(&Route, &Job) -> bool + Send + Sync>,

    /// An intervals state key.
    pub intervals_key: StateKey,
}

impl RouteIntervals for FixedRouteIntervals {
    fn is_marker_job(&self, job: &Job) -> bool {
        job.as_single().map_or(false, |single| (self.is_marker_single_fn)(single))
    }

    fn is_marker_assignable(&self, route: &Route, job: &Job) -> bool {
        self.is_marker_job(job) && (self.is_assignable_fn)(route, job)
    }

    fn is_new_interval_needed(&self, route_ctx: &RouteContext) -> bool {
        (self.is_new_interval_needed_fn)(route_ctx)
    }

    fn get_marker_intervals<'a>(&self, route_ctx: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>> {
        self.get_interval_key()
            .and_then(|state_code| route_ctx.state().get_route_state::<Vec<(usize, usize)>>(state_code))
    }

    fn get_interval_key(&self) -> Option<StateKey> {
        Some(self.intervals_key)
    }

    fn update_solution_intervals(&self, solution_ctx: &mut SolutionContext) {
        self.promote_markers_when_needed(solution_ctx);
        self.remove_trivial_markers(solution_ctx);
    }
}

impl FixedRouteIntervals {
    fn has_markers(&self, route_ctx: &RouteContext) -> bool {
        self.get_marker_intervals(route_ctx).map_or(false, |intervals| intervals.len() > 1)
    }

    fn filter_markers<'a>(&'a self, route: &'a Route, jobs: &'a [Job]) -> impl Iterator<Item = Job> + 'a {
        jobs.iter().filter(|job| self.is_marker_assignable(route, job)).cloned()
    }

    fn remove_trivial_markers(&self, solution_ctx: &mut SolutionContext) {
        let mut extra_ignored = Vec::new();
        solution_ctx.routes.iter_mut().filter(|route_ctx| self.has_markers(route_ctx)).for_each(|route_ctx| {
            let intervals = self.get_marker_intervals(route_ctx).cloned().unwrap_or_default();

            let _ = intervals.windows(2).try_for_each(|item| {
                let ((left_start, left_end), (right_start, right_end)) = match item {
                    &[left, right] => (left, right),
                    _ => unreachable!(),
                };

                assert_eq!(left_end + 1, right_start);

                if (self.is_obsolete_interval_fn)(route_ctx, left_start..left_end, right_start..right_end) {
                    // NOTE: we remove only one reload per tour, state update should be handled externally
                    extra_ignored.push(route_ctx.route_mut().tour.remove_activity_at(right_start));
                    Err(())
                } else {
                    Ok(())
                }
            });
        });

        solution_ctx.ignored.extend(extra_ignored);
    }

    fn promote_markers_when_needed(&self, solution_ctx: &mut SolutionContext) {
        let candidate_jobs = solution_ctx
            .routes
            .iter()
            .filter(|route_ctx| self.is_new_interval_needed(route_ctx))
            .flat_map(|route_ctx| {
                self.filter_markers(route_ctx.route(), &solution_ctx.ignored)
                    .chain(self.filter_markers(route_ctx.route(), &solution_ctx.required))
            })
            .collect::<HashSet<_>>();

        // NOTE: get already assigned jobs to guarantee locking them
        let assigned_job = solution_ctx
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route().tour.jobs())
            .filter(|job| self.is_marker_job(job))
            .cloned();

        solution_ctx.ignored.retain(|job| !candidate_jobs.contains(job));
        solution_ctx.locked.extend(candidate_jobs.iter().cloned().chain(assigned_job));
        solution_ctx.required.extend(candidate_jobs);
    }

    /// Returns marker intervals or default interval [0, tour_size).
    pub fn resolve_marker_intervals<'a>(
        &self,
        route_ctx: &'a RouteContext,
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        let last_idx = route_ctx.route().tour.total() - 1;

        self.get_marker_intervals(route_ctx)
            .map(|intervals| Either::Left(intervals.iter().copied()))
            .unwrap_or_else(|| Either::Right(once((0, last_idx))))
    }
}

/// Returns intervals between vehicle terminal and marker job activities.
pub fn get_route_intervals(route: &Route, is_marker_activity: impl Fn(&Activity) -> bool) -> Vec<(usize, usize)> {
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
