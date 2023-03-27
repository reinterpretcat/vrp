use crate::construction::heuristics::{RouteContext, SolutionContext, UnassignmentInfo};
use crate::models::problem::Job;
use crate::models::solution::{Activity, Route};
use hashbrown::HashSet;
use std::iter::empty;
use std::marker::PhantomData;

/// This trait defines multi-trip strategy for constraint extension.
pub trait MultiTrip {
    /// Specifies capacity type.
    type Constraint;

    /// Returns true if job is considered as multi trip marker.
    fn is_marker_job(&self, job: &Job) -> bool;

    /// Returns true if given job is multi trip marker and can be used with given route.
    fn is_assignable(&self, route: &Route, job: &Job) -> bool;

    /// Checks whether vehicle can do a new multi trip.
    fn is_multi_trip_needed(&self, route_ctx: &RouteContext) -> bool;

    /// Returns state code.
    fn get_state_code(&self) -> Option<i32>;

    /// Returns true if route context has multi trip markers.
    fn has_markers(&self, route_ctx: &RouteContext) -> bool {
        self.get_marker_intervals(route_ctx).map_or(false, |intervals| intervals.len() > 1)
    }

    /// Gets all multi trip markers for specific route from jobs collection.
    fn filter_markers<'a>(
        &'a self,
        route: &'a Route,
        jobs: &'a [Job],
    ) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync>;

    /// Returns marker intervals.
    fn get_marker_intervals<'a>(&self, route_ctx: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>> {
        self.get_state_code()
            .and_then(|state_code| route_ctx.state().get_route_state::<Vec<(usize, usize)>>(state_code))
    }

    /// Accepts insertion and promotes unassigned jobs with specific error code to unknown.
    fn accept_insertion(
        &self,
        solution_ctx: &mut SolutionContext,
        route_index: usize,
        job: &Job,
        unassigned_code: i32,
    ) {
        let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();

        if self.is_marker_job(job) {
            // move all unassigned marker jobs back to ignored
            let jobs = self.filter_markers(route_ctx.route(), &solution_ctx.required).collect::<HashSet<_>>();
            solution_ctx.required.retain(|job| !jobs.contains(job));
            solution_ctx.unassigned.retain(|job, _| !jobs.contains(job));
            solution_ctx.ignored.extend(jobs.into_iter());
            // NOTE reevaluate insertion of unassigned due to capacity constraint jobs
            solution_ctx.unassigned.iter_mut().for_each(|pair| match pair.1 {
                UnassignmentInfo::Simple(code) if *code == unassigned_code => {
                    *pair.1 = UnassignmentInfo::Unknown;
                }
                _ => {}
            });
        } else if self.is_multi_trip_needed(route_ctx) {
            // move all marker jobs for this shift to required
            let jobs = self
                .filter_markers(route_ctx.route(), &solution_ctx.ignored)
                .chain(self.filter_markers(route_ctx.route(), &solution_ctx.required))
                .collect::<HashSet<_>>();

            solution_ctx.ignored.retain(|job| !jobs.contains(job));
            solution_ctx.locked.extend(jobs.iter().cloned());
            solution_ctx.required.extend(jobs.into_iter());
        }
    }

    /// Accepts route state.
    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if let Some(state_code) = self.get_state_code() {
            let (route, state) = route_ctx.as_mut();
            let intervals = route_intervals(route, |a| {
                a.job.as_ref().map_or(false, |job| self.is_marker_job(&Job::Single(job.clone())))
            });

            state.put_route_state(state_code, intervals);
        }
    }

    /// Accepts solution state, e.g. removes trivial marker jobs.
    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext);
}

/// A no multi trip strategy.
pub struct NoMultiTrip<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for NoMultiTrip<T> {
    fn default() -> Self {
        NoMultiTrip { phantom: PhantomData }
    }
}

impl<T> MultiTrip for NoMultiTrip<T> {
    type Constraint = T;

    fn is_marker_job(&self, _: &Job) -> bool {
        false
    }

    fn is_assignable(&self, _: &Route, _: &Job) -> bool {
        false
    }

    fn is_multi_trip_needed(&self, _: &RouteContext) -> bool {
        false
    }

    fn get_state_code(&self) -> Option<i32> {
        None
    }

    fn has_markers(&self, _: &RouteContext) -> bool {
        false
    }

    fn filter_markers<'a>(&'a self, _: &'a Route, _: &'a [Job]) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync> {
        Box::new(empty())
    }

    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job, _: i32) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, _: &mut SolutionContext) {}
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
