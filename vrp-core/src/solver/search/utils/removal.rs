#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/utils/removal_test.rs"]
mod removal_test;

use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
use crate::solver::search::RemovalLimits;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::Random;
use std::collections::HashSet;
use std::sync::Arc;

/// A helper logic to keep amount of jobs/routes removed under control.
pub struct JobRemovalTracker {
    activities_left: i32,
    routes_left: i32,
    has_fully_removed_routes: bool,
    affected_actors: HashSet<Arc<Actor>>,
    removed_jobs: HashSet<Job>,
}

impl JobRemovalTracker {
    /// Creates a new instance of `JobRemoval`.
    pub fn new(limits: &RemovalLimits, random: &dyn Random) -> Self {
        Self {
            activities_left: random
                .uniform_int(limits.removed_activities_range.start as i32, limits.removed_activities_range.end as i32),
            routes_left: random
                .uniform_int(limits.affected_routes_range.start as i32, limits.affected_routes_range.end as i32),
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
        self.activities_left == 0 || self.routes_left == 0
    }

    pub fn get_affected_actors(&self) -> usize {
        self.affected_actors.len()
    }

    /// Tries to remove a job from the route of given index.
    pub fn try_remove_job(&mut self, solution: &mut SolutionContext, route_idx: usize, job: &Job) -> bool {
        if self.activities_left == 0 {
            return false;
        }

        if solution.locked.contains(job) {
            return false;
        }

        if let Some(route_ctx) = solution.routes.get_mut(route_idx) {
            if route_ctx.route_mut().tour.remove(job) {
                self.removed_jobs.insert(job.clone());
                self.affected_actors.insert(route_ctx.route().actor.clone());
                self.activities_left = (self.activities_left - get_total_activities(job) as i32).max(0);

                solution.required.push(job.clone());
                return true;
            }
        }

        false
    }

    /// Tries to remove jobs from the route with given index. The route could be removed completely
    /// if there is no locked jobs and limits are not reached. However, to increase discoverability
    /// of the solution space, at least one route could be removed ignoring limits (but it should
    /// contain no locked jobs).
    pub fn try_remove_route(
        &mut self,
        solution: &mut SolutionContext,
        route_idx: usize,
        random: &(dyn Random),
    ) -> bool {
        if self.routes_left == 0 || self.activities_left == 0 {
            return false;
        }

        if self.can_remove_full_route(solution, route_idx, random) {
            self.remove_whole_route(solution, route_idx);

            true
        } else {
            self.try_remove_part_route(solution, route_idx, random)
        }
    }

    fn can_remove_full_route(&self, solution: &SolutionContext, route_idx: usize, random: &(dyn Random)) -> bool {
        let route_ctx = solution.routes.get(route_idx).expect("invalid route index");

        // check locked jobs
        let has_locked_jobs =
            !solution.locked.is_empty() && route_ctx.route().tour.jobs().any(|job| solution.locked.contains(job));
        if has_locked_jobs {
            return false;
        }

        if route_ctx.route().tour.job_activity_count() as i32 <= self.activities_left {
            return true;
        }

        // try at least once remove a route completely
        if !self.has_fully_removed_routes {
            return random.is_hit(1. / self.routes_left.max(1) as f64);
        }

        false
    }

    fn remove_whole_route(&mut self, solution: &mut SolutionContext, route_idx: usize) {
        let route_ctx = solution.routes.get_mut(route_idx).expect("invalid route index");
        let actor = route_ctx.route().actor.clone();
        let jobs = route_ctx.route().tour.jobs().cloned().collect::<Vec<_>>();

        self.activities_left =
            (self.activities_left - jobs.iter().map(get_total_activities).sum::<usize>() as i32).max(0);

        jobs.iter().for_each(|job| {
            self.removed_jobs.insert(job.clone());
        });

        solution.required.extend(jobs);
        solution.keep_routes(&|route_ctx| route_ctx.route().actor != actor);

        self.affected_actors.insert(actor);
        self.routes_left = (self.routes_left - 1).max(0);

        self.has_fully_removed_routes = true;
    }

    fn try_remove_part_route(
        &mut self,
        solution: &mut SolutionContext,
        route_idx: usize,
        random: &(dyn Random),
    ) -> bool {
        let locked = solution.locked.clone();
        let route_ctx = solution.routes.get(route_idx).expect("invalid route index");
        let mut jobs: Vec<Job> = route_ctx.route().tour.jobs().filter(|job| !locked.contains(*job)).cloned().collect();
        jobs.shuffle(&mut random.get_rng());
        jobs.truncate(self.activities_left as usize);

        let old_count = solution.required.len();
        jobs.retain(|job| self.try_remove_job(solution, route_idx, job));

        self.routes_left = (self.routes_left - 1).max(0);

        // return false if no jobs was removed
        old_count != solution.required.len()
    }
}

fn get_total_activities(job: &Job) -> usize {
    job.as_multi().map_or(1, |multi| multi.jobs.len())
}
