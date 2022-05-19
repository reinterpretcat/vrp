#[cfg(test)]
#[path = "../../../tests/unit/solver/search/redistribute_search_test.rs"]
mod redistribute_search_test;

use super::*;
use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
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

impl HeuristicOperator for RedistributeSearch {
    type Context = RefinementContext;
    type Objective = ProblemObjective;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let routes_range = 2..3;
        let jobs_range = 4..12;

        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = create_target_insertion_ctx(solution, routes_range, jobs_range);

        let mut insertion_ctx = self.recreate.run(refinement_ctx, insertion_ctx);

        insertion_ctx.problem = solution.problem.clone();

        insertion_ctx.restore();
        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}

struct RedistributeHardConstraint {
    rules: HashMap<Job, Arc<Actor>>,
}

impl HardRouteConstraint for RedistributeHardConstraint {
    fn evaluate_job(
        &self,
        _: &SolutionContext,
        route_ctx: &RouteContext,
        job: &Job,
    ) -> Option<RouteConstraintViolation> {
        self.rules.get(job).and_then(|actor| {
            if actor == &route_ctx.route.actor {
                Some(RouteConstraintViolation { code: 0 })
            } else {
                None
            }
        })
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
        constraint: Arc::new(create_amended_constraint(problem.constraint.as_ref(), rules)),
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        objective: problem.objective.clone(),
        extras: problem.extras.clone(),
    });

    insertion_ctx
}

fn remove_jobs(
    insertion_ctx: &mut InsertionContext,
    routes_range: Range<i32>,
    jobs_range: Range<i32>,
) -> HashMap<Job, Arc<Actor>> {
    let constraint = insertion_ctx.problem.constraint.clone();
    let random = insertion_ctx.environment.random.clone();
    let locked = &insertion_ctx.solution.locked;
    let unassigned = &mut insertion_ctx.solution.unassigned;
    let sample = random.uniform_int(routes_range.start, routes_range.end) as usize;

    SelectionSamplingIterator::new(insertion_ctx.solution.routes.iter_mut(), sample, random.clone())
        .flat_map(|route_ctx| {
            #[allow(clippy::needless_collect)]
            let all_jobs = route_ctx.route.tour.jobs().filter(|job| !locked.contains(job)).collect::<Vec<_>>();
            let amount = random.uniform_int(jobs_range.start, jobs_range.end) as usize;

            let jobs = if random.is_head_not_tails() {
                let jobs =
                    SelectionSamplingIterator::new(all_jobs.into_iter(), amount, random.clone()).collect::<Vec<_>>();
                jobs.iter().for_each(|job| {
                    route_ctx.route_mut().tour.remove(job);
                    unassigned.insert(job.clone(), UnassignedCode::Unknown);
                });

                jobs
            } else {
                (0..amount).fold(Vec::new(), |mut acc, _| {
                    let job = if random.is_head_not_tails() {
                        route_ctx
                            .route
                            .tour
                            .all_activities()
                            .filter_map(|a| a.retrieve_job())
                            .find(|job| !locked.contains(job))
                    } else {
                        route_ctx
                            .route
                            .tour
                            .all_activities()
                            .rev()
                            .filter_map(|a| a.retrieve_job())
                            .find(|job| !locked.contains(job))
                    };

                    if let Some(job) = job {
                        route_ctx.route_mut().tour.remove(&job);
                        unassigned.insert(job.clone(), UnassignedCode::Unknown);
                        acc.push(job)
                    }

                    acc
                })
            };

            constraint.accept_route_state(route_ctx);

            jobs.into_iter().map(|job| (job, route_ctx.route.actor.clone()))
        })
        .collect()
}

fn create_amended_constraint(original: &ConstraintPipeline, rules: HashMap<Job, Arc<Actor>>) -> ConstraintPipeline {
    let mut pipeline = ConstraintPipeline {
        modules: original.modules.clone(),
        state_keys: original.state_keys.clone(),
        ..ConstraintPipeline::default()
    };

    pipeline.add_constraint(ConstraintVariant::HardRoute(Arc::new(RedistributeHardConstraint { rules })));
    original.get_constraints().for_each(|constraint| {
        pipeline.add_constraint(constraint);
    });

    pipeline
}
