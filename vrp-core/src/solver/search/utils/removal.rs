#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/utils/removal_test.rs"]
mod removal_test;

use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
use hashbrown::HashSet;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::Random;
use std::sync::Arc;

/// Specifies a limit for amount of jobs to be removed.
pub struct RuinLimitsEx {
    /// Specifies maximum amount of ruined (removed) jobs.
    pub max_ruined_jobs: usize,
    /// Specifies maximum amount of ruined (removed) jobs.
    pub max_ruined_activities: usize,
    /// Specifies maximum amount of affected routes.
    pub max_affected_routes: usize,
}

/// A helper logic to keep amount of jobs/routes removed under control.
#[derive(Clone)]
pub struct JobRemovalTracker {
    jobs_left: i32,
    activities_left: i32,
    routes_left: i32,
    has_fully_removed_routes: bool,
    affected_actors: HashSet<Arc<Actor>>,
    removed_jobs: HashSet<Job>,
}

impl JobRemovalTracker {
    /// Creates a new instance of `JobRemoval`.
    pub fn new(limits: &RuinLimitsEx) -> Self {
        Self {
            jobs_left: limits.max_ruined_jobs as i32,
            activities_left: limits.max_ruined_activities as i32,
            routes_left: limits.max_affected_routes as i32,
            has_fully_removed_routes: false,
            affected_actors: HashSet::default(),
            removed_jobs: HashSet::default(),
        }
    }

    pub fn is_affected_actor(&self, actor: &Actor) -> bool {
        self.affected_actors.contains(actor)
    }

    pub fn is_removed_job(&self, job: &Job) -> bool {
        self.removed_jobs.contains(job)
    }

    pub fn is_limit(&self) -> bool {
        self.activities_left == 0 || self.jobs_left == 0 || self.routes_left == 0
    }

    /// Tries to remove a job from the route.
    pub fn try_remove_job(&mut self, solution: &mut SolutionContext, route_ctx: &mut RouteContext, job: &Job) -> bool {
        if self.jobs_left == 0 || self.activities_left == 0 {
            return false;
        }

        self.activities_left = (self.activities_left - get_total_activities(job) as i32).max(0);
        self.jobs_left -= 1;

        if route_ctx.route_mut().tour.remove(job) {
            self.removed_jobs.insert(job.clone());
            self.affected_actors.insert(route_ctx.route.actor.clone());

            solution.required.push(job.clone());
            true
        } else {
            false
        }
    }

    /// Tries to remove jobs from the route. The route could be removed completely if there is no
    /// locked jobs and limits are not reached. However, to increase discoverability of the solution
    /// space, at least one route could be removed ignoring limits (but it should contain no locked
    /// jobs).
    pub fn try_remove_route(
        &mut self,
        solution: &mut SolutionContext,
        route_ctx: &mut RouteContext,
        random: &(dyn Random + Send + Sync),
    ) -> bool {
        if self.routes_left == 0 {
            return false;
        }

        if self.can_remove_full_route(solution, route_ctx, random) {
            self.remove_whole_route(solution, route_ctx);

            true
        } else {
            self.try_remove_part_route(solution, route_ctx, random)
        }
    }

    fn can_remove_full_route(
        &mut self,
        solution: &SolutionContext,
        route_ctx: &mut RouteContext,
        random: &(dyn Random + Send + Sync),
    ) -> bool {
        // check locked jobs
        let has_locked_jobs =
            !solution.locked.is_empty() && route_ctx.route.tour.jobs().any(|job| solution.locked.contains(&job));
        if has_locked_jobs {
            return false;
        }

        // can still remove activities
        if route_ctx.route.tour.job_activity_count() as i32 <= self.activities_left {
            return true;
        }

        // can still remove jobs
        if route_ctx.route.tour.job_count() as i32 <= self.jobs_left {
            return true;
        }

        // try at least once remove a route completely
        if !self.has_fully_removed_routes {
            return random.is_hit(1. / self.routes_left.max(1) as f64);
        }

        false
    }

    fn remove_whole_route(&mut self, solution: &mut SolutionContext, route_ctx: &mut RouteContext) {
        let jobs = route_ctx.route.tour.jobs().collect::<Vec<_>>();

        self.activities_left =
            (self.activities_left - jobs.iter().map(get_total_activities).sum::<usize>() as i32).max(0);
        self.jobs_left = (self.jobs_left - jobs.len() as i32).max(0);

        jobs.iter().for_each(|job| {
            self.removed_jobs.insert(job.clone());
        });

        solution.required.extend(jobs.into_iter());
        solution.routes.retain(|rc| rc != route_ctx);
        solution.registry.free_route(route_ctx);

        self.affected_actors.insert(route_ctx.route.actor.clone());
        self.routes_left = (self.routes_left - 1).max(0);

        self.has_fully_removed_routes = true;
    }

    fn try_remove_part_route(
        &mut self,
        solution: &mut SolutionContext,
        route_ctx: &mut RouteContext,
        random: &(dyn Random + Send + Sync),
    ) -> bool {
        let locked = solution.locked.clone();

        let mut jobs: Vec<Job> = route_ctx.route.tour.jobs().filter(|job| !locked.contains(job)).collect();
        jobs.shuffle(&mut random.get_rng());
        jobs.truncate(self.jobs_left as usize);

        let old_count = solution.required.len();
        jobs.retain(|job| self.try_remove_job(solution, route_ctx, job));

        self.routes_left = (self.routes_left - 1).max(0);

        // return false if no jobs was removed
        old_count != solution.required.len()
    }
}

fn get_total_activities(job: &Job) -> usize {
    job.as_multi().map_or(1, |multi| multi.jobs.len())
}
