use crate::construction::heuristics::{RouteContext, SolutionContext, UnassignedCode};
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Route};
use std::iter::empty;
use std::marker::PhantomData;
use std::sync::Arc;
use hashbrown::HashSet;

/// This trait defines multi-trip strategy for constraint extension.
pub trait MultiTrip {
    /// Specifies capacity type.
    type Capacity;

    /// Returns true if job is reload.
    fn is_reload_job(&self, job: &Job) -> bool;

    /// Returns true if single job is reload.
    fn is_reload_single(&self, single: &Single) -> bool;

    /// Returns true if given job is reload and can be used with given route.
    fn is_assignable(&self, route: &Route, job: &Job) -> bool;

    /// Checks whether vehicle is full
    fn is_vehicle_full(&self, ctx: &RouteContext) -> bool;

    /// Returns true if route context has reloads.
    fn has_reloads(&self, route_ctx: &RouteContext) -> bool;

    /// Returns reload job from activity or None.
    fn get_reload<'a>(&self, activity: &'a Activity) -> Option<&'a Arc<Single>>;

    /// Gets all reloads for specific route from jobs collection.
    fn get_reloads<'a>(&'a self, route: &'a Route, jobs: &'a [Job])
        -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync>;

    /// Accepts insertion and promotes unassigned jobs with specific error code to unknown.
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job, unassigned_code: i32) {
        let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();

        if self.is_reload_job(job) {
            // move all unassigned reloads back to ignored
            let jobs = self.get_reloads(&route_ctx.route, &solution_ctx.required).collect::<HashSet<_>>();
            solution_ctx.required.retain(|job| !jobs.contains(job));
            solution_ctx.unassigned.retain(|job, _| !jobs.contains(job));
            solution_ctx.ignored.extend(jobs.into_iter());
            // NOTE reevaluate insertion of unassigned due to capacity constraint jobs
            solution_ctx.unassigned.iter_mut().for_each(|pair| match pair.1 {
                UnassignedCode::Simple(code) if *code == unassigned_code => {
                    *pair.1 = UnassignedCode::Unknown;
                }
                _ => {}
            });
        } else if self.is_vehicle_full(route_ctx) {
            // move all reloads for this shift to required
            let jobs = self
                .get_reloads(&route_ctx.route, &solution_ctx.ignored)
                .chain(self.get_reloads(&route_ctx.route, &solution_ctx.required))
                .collect::<HashSet<_>>();

            solution_ctx.ignored.retain(|job| !jobs.contains(job));
            solution_ctx.locked.extend(jobs.iter().cloned());
            solution_ctx.required.extend(jobs.into_iter());
        }
    }
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
    type Capacity = T;

    fn is_reload_job(&self, _: &Job) -> bool {
        false
    }

    fn is_reload_single(&self, _: &Single) -> bool {
        false
    }

    fn is_assignable(&self, _: &Route, _: &Job) -> bool {
        false
    }

    fn is_vehicle_full(&self, _: &RouteContext) -> bool {
        false
    }

    fn has_reloads(&self, _: &RouteContext) -> bool {
        false
    }

    fn get_reload<'a>(&self, _: &'a Activity) -> Option<&'a Arc<Single>> {
        None
    }

    fn get_reloads<'a>(&'a self, _: &'a Route, _: &'a [Job]) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync> {
        Box::new(empty())
    }

    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job, _: i32) { }
}
