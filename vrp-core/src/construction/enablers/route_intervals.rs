use crate::construction::heuristics::{RouteContext, RouteState, SolutionContext};
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Route};
use crate::utils::Either;
use std::collections::HashSet;
use std::iter::once;
use std::ops::Range;
use std::sync::Arc;

/// Provides the way to get/set route intervals on the route state.
/// Depending on the feature, route intervals can be different. So, each feature needs to implement
/// state management independently.
pub trait RouteIntervalsState: Send + Sync {
    /// Gets route indices specified by `start` end `end` activity index.
    fn get_route_intervals<'a>(&self, route_state: &'a RouteState) -> Option<&'a Vec<(usize, usize)>>;

    /// Sets route indices specified by `start` end `end` activity index.
    fn set_route_intervals(&self, route_state: &mut RouteState, values: Vec<(usize, usize)>);
}

/// Provides a way to logically split route into intervals using specific marker jobs.
#[derive(Clone)]
pub enum RouteIntervals {
    /// The whole route is considered as a single route interval with no special markers in the middle.
    Single,

    /// The route can be split into multiple intervals by marker jobs.
    #[allow(clippy::type_complexity)]
    Multiple {
        /// Checks whether specified single job is of marker type.
        is_marker_single_fn: Arc<dyn Fn(&Single) -> bool + Send + Sync>,

        /// Specifies a function which checks whether a new interval is needed for given route.
        is_new_interval_needed_fn: Arc<dyn Fn(&RouteContext) -> bool + Send + Sync>,

        /// Specifies an obsolete interval function which takes left and right interval range. These intervals are separated by marker job activity.
        is_obsolete_interval_fn: Arc<dyn Fn(&RouteContext, Range<usize>, Range<usize>) -> bool + Send + Sync>,

        /// Specifies a function which checks whether job can be assigned to a given route.
        is_assignable_fn: Arc<dyn Fn(&Route, &Job) -> bool + Send + Sync>,

        /// Specifies a specific implementation to get/set route intervals on `RouteState`.
        intervals_state: Arc<dyn RouteIntervalsState>,
    },
}

// Public API
impl RouteIntervals {
    /// Returns true if job is considered as a route interval marker.
    pub fn is_marker_job(&self, job: &Job) -> bool {
        match self {
            RouteIntervals::Single => false,
            RouteIntervals::Multiple { is_marker_single_fn, .. } => {
                job.as_single().map_or(false, |single| (is_marker_single_fn)(single))
            }
        }
    }

    /// Returns true if given job is a marker job and can be used with given route.
    pub fn is_marker_assignable(&self, route: &Route, job: &Job) -> bool {
        match self {
            RouteIntervals::Single => false,
            RouteIntervals::Multiple { is_assignable_fn, .. } => {
                self.is_marker_job(job) && (is_assignable_fn)(route, job)
            }
        }
    }

    /// Checks whether vehicle can do a new route interval.
    pub fn is_new_interval_needed(&self, route_ctx: &RouteContext) -> bool {
        match self {
            RouteIntervals::Single => false,
            RouteIntervals::Multiple { is_new_interval_needed_fn, .. } => (is_new_interval_needed_fn)(route_ctx),
        }
    }

    /// Gets route intervals split by marker jobs.
    pub fn get_marker_intervals<'a>(&self, route_ctx: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>> {
        match self {
            RouteIntervals::Single => None,
            RouteIntervals::Multiple { .. } => {
                self.get_interval_fn().and_then(|interval_fn| interval_fn.get_route_intervals(route_ctx.state()))
            }
        }
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

    /// Gets interval function if present.
    pub fn get_interval_fn(&self) -> Option<Arc<dyn RouteIntervalsState>> {
        match self {
            RouteIntervals::Single => None,
            RouteIntervals::Multiple { intervals_state: intervals_fn, .. } => Some(intervals_fn.clone()),
        }
    }

    /// Update route intervals on solution level.
    pub fn update_solution_intervals(&self, solution_ctx: &mut SolutionContext) {
        match self {
            RouteIntervals::Single => {}
            RouteIntervals::Multiple { .. } => {
                self.promote_markers_when_needed(solution_ctx);
                self.remove_trivial_markers(solution_ctx);
            }
        }
    }
}

// Private API
impl RouteIntervals {
    fn has_markers(&self, route_ctx: &RouteContext) -> bool {
        self.get_marker_intervals(route_ctx).map_or(false, |intervals| intervals.len() > 1)
    }

    fn filter_markers<'a>(&'a self, route: &'a Route, jobs: &'a [Job]) -> impl Iterator<Item = Job> + 'a {
        jobs.iter().filter(|job| self.is_marker_assignable(route, job)).cloned()
    }

    fn remove_trivial_markers(&self, solution_ctx: &mut SolutionContext) {
        let is_obsolete_interval_fn = match self {
            RouteIntervals::Single => return,
            RouteIntervals::Multiple { is_obsolete_interval_fn, .. } => is_obsolete_interval_fn,
        };

        let mut extra_ignored = Vec::new();
        solution_ctx.routes.iter_mut().filter(|route_ctx| self.has_markers(route_ctx)).for_each(|route_ctx| {
            let intervals = self.get_marker_intervals(route_ctx).cloned().unwrap_or_default();

            let _ = intervals.windows(2).try_for_each(|item| {
                let ((left_start, left_end), (right_start, right_end)) = match item {
                    &[left, right] => (left, right),
                    _ => unreachable!(),
                };

                assert_eq!(left_end + 1, right_start);

                if (is_obsolete_interval_fn)(route_ctx, left_start..left_end, right_start..right_end) {
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
