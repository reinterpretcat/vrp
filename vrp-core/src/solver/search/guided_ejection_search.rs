#[cfg(test)]
#[path = "../../../tests/unit/solver/search/guided_ejection_search_test.rs"]
mod guided_ejection_search_test;

use crate::construction::heuristics::*;
use crate::models::GoalContext;
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use rosomaxa::hyper::HeuristicDiversifyOperator;
use rosomaxa::prelude::*;
use rosomaxa::utils::{ParallelismPolicy, ParallelismScope, parallel_collect};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::iter::once;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_ATTEMPTS: usize = 1_000;
// Ejection candidates differ in cost depending on route size and constraints. Keep enough tasks for
// load balancing without letting this inner loop compete with every coarse search for tiny work.
const EJECTION_EVALUATION_TASKS_PER_WORKER: usize = 4;
// Consecutive failures double the retry interval up to eight problem-sized intervals. This gives a
// new local optimum an early second chance, while repeated hard failures become rare.
const FAILED_SEARCH_INTERVALS: usize = 8;

/// A bounded route-elimination search based on guided ejection.
///
/// The search starts from the incumbent and works as follows:
///
/// 1. Remove one of the routes with the fewest jobs, excluding routes which contain locked jobs.
///    Put all jobs from that route into a last-in-first-out ejection pool.
/// 2. Pop a job and try an exhaustive feasible insertion into the remaining routes.
/// 3. If direct insertion fails, try replacing one served job with the pool job. Only when every
///    single ejection fails, try a bounded number of same-route job pairs.
/// 4. Put ejected jobs back into the pool and repeat. The route is eliminated when the pool becomes
///    empty; an unfinished partial solution is never returned.
///
/// Every time a pool job is processed, its insertion-attempt counter is increased. Ejections first
/// minimize the counter of a single job or the counter sum of a pair, avoiding jobs which have
/// already proved difficult to reinsert. Generic insertion cost breaks ties inside the lowest
/// feasible counter tier. No objective dimension is interpreted here; the population decides
/// whether the completed candidate is valuable.
///
/// Unlike some published variants, routes remain feasible throughout this search. Its temporary
/// infeasibility consists only of jobs in the ejection pool. A call is bounded by problem size, up
/// to 1,000 pool-processing attempts, observes the global quota, and gives the quadratic pair
/// neighborhood a separate evaluation budget.
///
/// The scientific references are relevant as the algorithm's ancestry, not as an exact
/// specification of this implementation. [Lim and Zhang (2007)] provides the smallest-route and
/// ejection-pool scheme used as the starting point, while [Nagata and Bräysy (2009)] guides
/// route-minimizing ejections by how difficult jobs are to reinsert. [Curtois et al. (2018)]
/// describes the closely related LIFO, single-then-pair procedure. Those algorithms also use
/// problem-specific infeasible insertion, perturbation, or local-search phases which are
/// deliberately left to the existing solver here.
///
/// Parallel callers use a lock-free generation schedule, so at most one call performs the expensive
/// search in a generation. Successful route eliminations are retried after one problem-sized
/// interval; consecutive failures back off to at most eight such intervals.
///
/// [Lim and Zhang (2007)]: https://doi.org/10.1287/ijoc.1060.0186
/// [Nagata and Bräysy (2009)]: https://doi.org/10.1016/j.orl.2009.04.006
/// [Curtois et al. (2018)]: https://doi.org/10.1007/s13676-017-0115-6
pub struct GuidedEjectionSearch {
    schedule: SearchSchedule,
}

#[derive(Default)]
struct SearchSchedule {
    next_generation: AtomicUsize,
    failures: AtomicUsize,
}

impl SearchSchedule {
    fn try_reserve(&self, generation: usize) -> bool {
        let next_generation = self.next_generation.load(Ordering::Relaxed);
        generation >= next_generation
            && self
                .next_generation
                .compare_exchange(next_generation, generation.saturating_add(1), Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
    }

    fn complete(&self, generation: usize, problem_size: usize, is_success: bool) {
        let intervals = if is_success {
            self.failures.store(0, Ordering::Relaxed);
            1
        } else {
            let failures = self.failures.fetch_add(1, Ordering::Relaxed);
            1_usize << failures.min(FAILED_SEARCH_INTERVALS.ilog2() as usize)
        };
        let interval = problem_size.max(1).saturating_mul(intervals);

        self.next_generation.store(generation.saturating_add(interval), Ordering::Relaxed);
    }
}

impl Default for GuidedEjectionSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl GuidedEjectionSearch {
    /// Creates a guided ejection search.
    pub fn new() -> Self {
        Self { schedule: SearchSchedule::default() }
    }

    fn try_search(&self, heuristic_ctx: &RefinementContext, solution: &InsertionContext) -> Option<InsertionContext> {
        // Route elimination starts from a feasible incumbent. Mixing its ejection pool with jobs
        // which were already unassigned makes feasibility repair harder and the attempt needlessly large.
        let incumbent = heuristic_ctx.ranked().next().unwrap_or(solution);
        if !incumbent.solution.required.is_empty() || !incumbent.solution.unassigned.is_empty() {
            return None;
        }

        let generation = heuristic_ctx.statistics().generation;
        if !self.schedule.try_reserve(generation) {
            return None;
        }

        // A deep route-elimination attempt is most useful on the incumbent. The regular additive
        // diversifier still works on the selected parent and therefore preserves population variety.
        let problem_size = incumbent.problem.jobs.size();
        let max_attempts = problem_size.clamp(1, MAX_ATTEMPTS);
        let source_idx = select_source_route(incumbent);
        let candidate = source_idx.and_then(|source_idx| eliminate_route(incumbent, source_idx, max_attempts));
        self.schedule.complete(generation, problem_size, candidate.is_some());

        candidate
    }
}

impl HeuristicSearchOperator for GuidedEjectionSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        self.try_search(heuristic_ctx, solution).unwrap_or_else(|| solution.deep_copy())
    }
}

impl HeuristicDiversifyOperator for GuidedEjectionSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        self.try_search(heuristic_ctx, solution).into_iter().collect()
    }
}

fn select_source_route(insertion_ctx: &InsertionContext) -> Option<usize> {
    if insertion_ctx.solution.routes.len() < 2 {
        return None;
    }

    let locked = &insertion_ctx.solution.locked;
    let mut candidates = insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        // Reinsertion constraints cannot undo the ruin of a locked job, so its route is not eligible.
        .filter(|(_, route)| !route.route().tour.jobs().any(|job| locked.contains(job)))
        .map(|(route_idx, route)| (route_idx, route.route().tour.job_count()))
        .collect::<Vec<_>>();
    candidates.sort_unstable_by_key(|(_, job_count)| *job_count);

    let min_job_count = candidates.first().map(|(_, job_count)| *job_count)?;
    let candidate_count = candidates.iter().take_while(|(_, job_count)| *job_count == min_job_count).count();
    let selected = insertion_ctx.environment.random.uniform_int(0, candidate_count as i32 - 1) as usize;

    candidates.get(selected).map(|(route_idx, _)| *route_idx)
}

fn eliminate_route(original: &InsertionContext, source_idx: usize, max_attempts: usize) -> Option<InsertionContext> {
    let mut candidate = original.deep_copy();
    let source = candidate.solution.routes.get(source_idx)?;
    let actor = source.route().actor.clone();
    let mut pool = source.route().tour.jobs().cloned().collect::<Vec<_>>();

    // Removing the route creates the initial partial solution; all remaining routes stay feasible.
    candidate.solution.keep_routes(&|route| route.route().actor != actor);
    pool.iter().for_each(|job| {
        candidate.solution.unassigned.insert(job.clone(), UnassignmentInfo::Unknown);
    });
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    let mut insertion_attempts = HashMap::<Job, usize>::new();
    // Pair ejections are a fallback. Giving them one evaluated candidate per pool attempt on average
    // keeps the quadratic neighborhood bounded without reducing the depth of single-job chains.
    let mut ejection_budget = EjectionEvaluationBudget::new(max_attempts);

    for _ in 0..max_attempts {
        if candidate.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return None;
        }

        // Removing or inserting a job can activate conditional jobs such as breaks or reloads.
        // They are part of the partial solution and have to be reinserted before it can be returned.
        pool.retain(|job| !candidate.solution.ignored.contains(job));
        pool.extend(candidate.solution.required.drain(..));
        add_unassigned(&candidate.solution, &mut pool);
        if pool.is_empty() {
            candidate.restore();
            pool.retain(|job| !candidate.solution.ignored.contains(job));
            pool.extend(candidate.solution.required.drain(..));
            add_unassigned(&candidate.solution, &mut pool);
            if pool.is_empty() {
                return Some(candidate);
            }
        }

        let job = pool.pop().unwrap();
        *insertion_attempts.entry(job.clone()).or_default() += 1;
        // Prefer repairing the pool without disturbing another route.
        if let Some(success) = evaluate_job(&candidate, &job, None) {
            apply_insertion_success(&mut candidate, success);
            continue;
        }

        // Otherwise exchange one or two served jobs for this job and continue the resulting chain.
        let ejected = match find_ejection(&candidate, &job, &insertion_attempts, &mut ejection_budget) {
            Some(ejection) => {
                // Feature transitions can make the real insertion differ from its route-only
                // evaluation. The candidate is isolated, so discard the whole attempt instead of
                // trying to roll back arbitrary feature state.
                if !apply_ejection(&mut candidate, &job, &ejection) {
                    return None;
                }
                Some(ejection.into_jobs())
            }
            None => None,
        };

        if ejected.is_none() && ejection_budget.is_exhausted() {
            return None;
        }

        match ejected {
            Some(ejected) => pool.extend(ejected),
            None => {
                // With no other pending job, neither the partial solution nor the ejection
                // penalties can change in a way which makes this job insertable on another try.
                if pool.is_empty() && candidate.solution.required.is_empty() {
                    return None;
                }
                pool.insert(0, job);
            }
        }
    }

    None
}

fn add_unassigned(solution: &SolutionContext, pool: &mut Vec<Job>) {
    let activated = solution.unassigned.keys().filter(|job| !pool.contains(*job)).cloned().collect::<Vec<_>>();
    pool.extend(activated);
}

fn evaluate_job(insertion_ctx: &InsertionContext, job: &Job, route_idx: Option<usize>) -> Option<InsertionSuccess> {
    let result_selector = BestResultSelector::default();
    let evaluator = PositionInsertionEvaluator::default();
    let jobs = [job];
    let routes = match route_idx {
        Some(route_idx) => insertion_ctx.solution.routes.get(route_idx).into_iter().collect::<Vec<_>>(),
        None => insertion_ctx.solution.routes.iter().collect::<Vec<_>>(),
    };

    evaluator.evaluate_all(insertion_ctx, &jobs, &routes, &LegSelection::Exhaustive, &result_selector).try_into().ok()
}

struct Ejection {
    route_idx: usize,
    first: Job,
    second: Option<Job>,
}

impl Ejection {
    fn single(route_idx: usize, job: Job) -> Self {
        Self { route_idx, first: job, second: None }
    }

    fn pair(route_idx: usize, first: Job, second: Job) -> Self {
        Self { route_idx, first, second: Some(second) }
    }

    fn jobs(&self) -> impl Iterator<Item = &Job> {
        once(&self.first).chain(self.second.iter())
    }

    fn into_jobs(self) -> impl Iterator<Item = Job> {
        once(self.first).chain(self.second)
    }
}

struct EjectionEvaluationBudget {
    remaining_pairs: usize,
}

impl EjectionEvaluationBudget {
    fn new(limit: usize) -> Self {
        Self { remaining_pairs: limit }
    }

    fn try_consume_pair(&mut self) -> bool {
        if self.remaining_pairs == 0 {
            false
        } else {
            self.remaining_pairs -= 1;
            true
        }
    }

    fn is_exhausted(&self) -> bool {
        self.remaining_pairs == 0
    }
}

fn find_ejection(
    insertion_ctx: &InsertionContext,
    job: &Job,
    attempts: &HashMap<Job, usize>,
    budget: &mut EjectionEvaluationBudget,
) -> Option<Ejection> {
    // Penalty is the primary guide. Route and tour order provide a stable tie break, while source
    // route selection and the surrounding search still provide variation between calls.
    let mut singles = Vec::with_capacity(insertion_ctx.problem.jobs.size());
    for (route_idx, route) in insertion_ctx.solution.routes.iter().enumerate() {
        for (job_idx, candidate) in route
            .route()
            .tour
            .jobs()
            .filter(|candidate| !insertion_ctx.solution.locked.contains(*candidate))
            .enumerate()
        {
            singles.push((attempts.get(candidate).copied().unwrap_or_default(), route_idx, job_idx, candidate));
        }
    }
    singles.sort_unstable_by_key(|(penalty, route_idx, job_idx, _)| (*penalty, *route_idx, *job_idx));

    let evaluate = |(_, route_idx, _, candidate): &(usize, usize, usize, &Job)| {
        if insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return None;
        }

        let ejection = Ejection::single(*route_idx, (*candidate).clone());
        evaluate_ejection(insertion_ctx, job, &insertion_ctx.solution.routes[*route_idx], ejection.jobs())
            .map(|cost| (cost, ejection))
    };
    // Penalty tiers depend on the history of the ejection chain, so they are considered in order.
    // Candidates inside one tier are independent and expensive enough to evaluate concurrently.
    let mut tier_start = 0;
    while tier_start < singles.len() {
        let penalty = singles[tier_start].0;
        let tier_end = tier_start
            + singles[tier_start..].partition_point(|(candidate_penalty, _, _, _)| *candidate_penalty == penalty);
        let tier = &singles[tier_start..tier_end];
        let evaluated = parallel_collect(
            tier,
            ParallelismScope::Local,
            ParallelismPolicy::adaptive(EJECTION_EVALUATION_TASKS_PER_WORKER),
            evaluate,
        );
        let best_single = evaluated.into_iter().flatten().fold(None, |best, (cost, ejection)| match best {
            Some((best_cost, best_ejection)) if best_cost <= cost => Some((best_cost, best_ejection)),
            _ => Some((cost, ejection)),
        });

        if insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return None;
        }
        if let Some((_, ejection)) = best_single {
            return Some(ejection);
        }
        tier_start = tier_end;
    }

    // Keep jobs ordered by penalty so pair candidates can be generated lazily in increasing total
    // penalty. The heap stores one head for each pair sequence instead of materializing the complete
    // quadratic neighborhood. Per-route collections are delayed until this fallback is actually used.
    let mut route_jobs = insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .map(|(route_idx, route)| {
            let jobs = route
                .route()
                .tour
                .jobs()
                .filter(|job| !insertion_ctx.solution.locked.contains(*job))
                .collect::<Vec<_>>();
            (route_idx, jobs)
        })
        .collect::<Vec<_>>();
    route_jobs.iter_mut().for_each(|(_, jobs)| jobs.sort_by_key(|job| attempts.get(job).copied().unwrap_or_default()));
    let mut pairs = BinaryHeap::new();
    for (route_pos, (_, jobs)) in route_jobs.iter().enumerate() {
        for first in 0..jobs.len().saturating_sub(1) {
            let second = first + 1;
            let penalty = attempts.get(&jobs[first]).copied().unwrap_or_default()
                + attempts.get(&jobs[second]).copied().unwrap_or_default();
            pairs.push(Reverse((penalty, route_pos, first, second)));
        }
    }

    let mut best_pair = None;
    while let Some(Reverse((penalty, route_pos, first, second))) = pairs.pop() {
        if best_pair.as_ref().is_some_and(|(_, best_penalty, _)| *best_penalty < penalty) {
            break;
        }

        let (route_idx, jobs) = &route_jobs[route_pos];
        let ejection = Ejection::pair(*route_idx, jobs[first].clone(), jobs[second].clone());
        let next = second + 1;
        if next < jobs.len() {
            let next_penalty = attempts.get(&jobs[first]).copied().unwrap_or_default()
                + attempts.get(&jobs[next]).copied().unwrap_or_default();
            pairs.push(Reverse((next_penalty, route_pos, first, next)));
        }

        if !budget.try_consume_pair() {
            break;
        }
        if let Some(cost) =
            evaluate_ejection(insertion_ctx, job, &insertion_ctx.solution.routes[*route_idx], ejection.jobs())
            && best_pair.as_ref().is_none_or(|(best_cost, _, _)| cost < *best_cost)
        {
            best_pair = Some((cost, penalty, ejection));
        }

        if insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return None;
        }
    }

    best_pair.map(|(_, _, ejection)| ejection)
}

fn evaluate_ejection<'a>(
    insertion_ctx: &InsertionContext,
    job: &Job,
    route: &RouteContext,
    ejected: impl Iterator<Item = &'a Job>,
) -> Option<InsertionCost> {
    let mut route = route.deep_copy();
    if ejected.into_iter().any(|job| !route.route_mut().tour.remove(job)) {
        return None;
    }
    insertion_ctx.problem.goal.accept_route_state(&mut route);

    let result_selector = BestResultSelector::default();
    let eval_ctx = EvaluationContext {
        goal: insertion_ctx.problem.goal.as_ref(),
        job,
        leg_selection: &LegSelection::Exhaustive,
        result_selector: &result_selector,
    };

    eval_job_insertion_in_route(
        insertion_ctx,
        &eval_ctx,
        &route,
        InsertionPosition::Any,
        InsertionResult::make_failure(),
    )
    .as_success()
    .map(|success| success.cost.clone())
}

fn apply_ejection(insertion_ctx: &mut InsertionContext, job: &Job, ejection: &Ejection) -> bool {
    {
        let route = &mut insertion_ctx.solution.routes[ejection.route_idx];
        for ejected in ejection.jobs() {
            if !route.route_mut().tour.remove(ejected) {
                return false;
            }
        }
        insertion_ctx.problem.goal.accept_route_state(route);
    }
    ejection.jobs().for_each(|job| {
        insertion_ctx.solution.unassigned.insert(job.clone(), UnassignmentInfo::Unknown);
    });
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);

    // Re-evaluate against the complete solution state: conditional features can differ from the
    // route copy used while screening ejection candidates.
    if let Some(success) = evaluate_job(insertion_ctx, job, Some(ejection.route_idx)) {
        apply_insertion_success(insertion_ctx, success);
        true
    } else {
        false
    }
}
