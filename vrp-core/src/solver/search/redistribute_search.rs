#[cfg(test)]
#[path = "../../../tests/unit/solver/search/redistribute_search_test.rs"]
mod redistribute_search_test;

use super::*;
use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
use crate::models::*;
use crate::prelude::Problem;
use hashbrown::HashMap;
use rosomaxa::prelude::*;
use rosomaxa::utils::SelectionSamplingIterator;
use std::ops::Range;
use std::sync::Arc;

/// A search operator which removes jobs from existing routes and prevents their insertion into
/// the same routes again.
/// The main idea is to introduce a bit more diversity in the population.
pub struct RedistributeSearch {
    recreate: Arc<dyn Recreate + Send + Sync>,
}

impl RedistributeSearch {
    /// Creates a new instance of `RedistributeSearch`.
    pub fn new(recreate: Arc<dyn Recreate + Send + Sync>) -> Self {
        Self { recreate }
    }
}

impl HeuristicSearchOperator for RedistributeSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let routes_range = 1..8;
        let per_routes_jobs_range = 4..8;

        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = create_target_insertion_ctx(solution, routes_range, per_routes_jobs_range);

        let mut insertion_ctx = self.recreate.run(refinement_ctx, insertion_ctx);

        insertion_ctx.problem = solution.problem.clone();

        insertion_ctx.restore();
        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}

struct RedistributeFeatureConstraint {
    rules: HashMap<Job, Arc<Actor>>,
}

impl FeatureConstraint for RedistributeFeatureConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.rules.get(*job).and_then(|actor| {
                if actor == &route_ctx.route().actor {
                    Some(ConstraintViolation { code: 0, stopped: true })
                } else {
                    None
                }
            }),
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

fn create_target_insertion_ctx(
    original_ctx: &InsertionContext,
    routes_range: Range<i32>,
    jobs_range: Range<i32>,
) -> InsertionContext {
    let problem = original_ctx.problem.clone();

    let mut insertion_ctx = original_ctx.deep_copy();
    let rules = remove_jobs(&mut insertion_ctx, routes_range, jobs_range);

    insertion_ctx.problem = Arc::new(Problem {
        fleet: problem.fleet.clone(),
        jobs: problem.jobs.clone(),
        locks: problem.locks.clone(),
        goal: create_amended_variant(problem.goal.as_ref(), rules),
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        extras: problem.extras.clone(),
    });

    insertion_ctx
}

fn remove_jobs(
    insertion_ctx: &mut InsertionContext,
    routes_range: Range<i32>,
    jobs_range: Range<i32>,
) -> HashMap<Job, Arc<Actor>> {
    let constraint = insertion_ctx.problem.goal.clone();
    let random = insertion_ctx.environment.random.clone();
    let locked = &insertion_ctx.solution.locked;
    let unassigned = &mut insertion_ctx.solution.unassigned;
    let sample = random.uniform_int(routes_range.start, routes_range.end) as usize;

    SelectionSamplingIterator::new(insertion_ctx.solution.routes.iter_mut(), sample, random.clone())
        .flat_map(|route_ctx| {
            #[allow(clippy::needless_collect)]
            let all_jobs = route_ctx.route().tour.jobs().filter(|job| !locked.contains(job)).collect::<Vec<_>>();
            let amount = random.uniform_int(jobs_range.start, jobs_range.end) as usize;

            let jobs = if random.is_head_not_tails() {
                let jobs =
                    SelectionSamplingIterator::new(all_jobs.into_iter(), amount, random.clone()).collect::<Vec<_>>();
                jobs.iter().for_each(|job| {
                    route_ctx.route_mut().tour.remove(job);
                    unassigned.insert(job.clone(), UnassignmentInfo::Unknown);
                });

                jobs
            } else {
                (0..amount).fold(Vec::new(), |mut acc, _| {
                    let job = if random.is_head_not_tails() {
                        route_ctx
                            .route()
                            .tour
                            .all_activities()
                            .filter_map(|a| a.retrieve_job())
                            .find(|job| !locked.contains(job))
                    } else {
                        route_ctx
                            .route()
                            .tour
                            .all_activities()
                            .rev()
                            .filter_map(|a| a.retrieve_job())
                            .find(|job| !locked.contains(job))
                    };

                    if let Some(job) = job {
                        route_ctx.route_mut().tour.remove(&job);
                        unassigned.insert(job.clone(), UnassignmentInfo::Unknown);
                        acc.push(job)
                    }

                    acc
                })
            };

            constraint.accept_route_state(route_ctx);

            jobs.into_iter().map(|job| (job, route_ctx.route().actor.clone()))
        })
        .collect()
}

fn create_amended_variant(original: &GoalContext, rules: HashMap<Job, Arc<Actor>>) -> Arc<GoalContext> {
    let mut constraints = original.constraints.clone();
    constraints.push(Arc::new(RedistributeFeatureConstraint { rules }));

    Arc::new(GoalContext { constraints, ..original.clone() })
}
