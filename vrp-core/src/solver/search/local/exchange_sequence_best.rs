#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_sequence_best_test.rs"]
mod exchange_sequence_best_test;

use crate::construction::heuristics::*;
use crate::models::common::{Cost, Timestamp};
use crate::models::problem::{Job, TravelTime};
use crate::models::solution::{Activity, Route};
use crate::solver::RefinementContext;
use crate::solver::search::LocalOperator;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::{HeuristicObjective, HeuristicSolution};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

// Route-local checks are deliberately bounded: the transport delta is only a proposal ranking, while
// the common insertion pipeline remains responsible for feasibility and final objective acceptance.
const SCREEN_CANDIDATE_THRESHOLD: usize = 8;

/// A cost-guided sequence neighborhood inspired by the classical HGS local-search moves.
///
/// One scan samples consecutive source pairs and proposes four related inter-route moves around nearby
/// jobs: ordered and reversed relocation of two jobs, exchange of two jobs against one, and exchange
/// of two jobs against two. Candidate generation is granular and transport-guided, but every moved
/// job is handled through the normal constraint pipeline and only a strict configured-objective
/// improvement is returned. Locked jobs are never moved.
pub struct ExchangeSequenceBest {
    source_pair_threshold: usize,
    neighbor_threshold: usize,
    target_threshold: usize,
    move_types: MoveTypes,
}

impl ExchangeSequenceBest {
    /// Creates a sequence search with bounded source, proximity, and target coverage.
    pub fn new(source_pair_threshold: usize, neighbor_threshold: usize, target_threshold: usize) -> Self {
        assert!(source_pair_threshold > 0);
        assert!(neighbor_threshold > 0);
        assert!(target_threshold > 0);

        Self { source_pair_threshold, neighbor_threshold, target_threshold, move_types: MoveTypes::all() }
    }

    /// Creates a sequence search which considers every movable source pair.
    pub fn new_global(neighbor_threshold: usize, target_threshold: usize) -> Self {
        Self::new(usize::MAX, neighbor_threshold, target_threshold)
    }

    #[cfg(test)]
    fn with_move_types(move_types: MoveTypes) -> Self {
        Self { source_pair_threshold: 32, neighbor_threshold: 32, target_threshold: 2, move_types }
    }
}

impl Default for ExchangeSequenceBest {
    fn default() -> Self {
        // VND calls this neighborhood repeatedly, so small random samples provide broad coverage
        // without repeating a full source scan each time.
        Self::new(8, 32, 2)
    }
}

impl LocalOperator for ExchangeSequenceBest {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        if insertion_ctx.solution.routes.len() < 2
            || insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached())
        {
            return None;
        }

        let sequence_move = select_sequence_move(
            insertion_ctx,
            self.source_pair_threshold,
            self.neighbor_threshold,
            self.target_threshold,
            self.move_types,
        )?;
        let candidate = apply_sequence_move(insertion_ctx, sequence_move)?;

        (insertion_ctx.problem.goal.total_order(&candidate, insertion_ctx) == Ordering::Less).then_some(candidate)
    }
}

#[derive(Clone, Copy)]
struct MoveTypes {
    relocate: bool,
    exchange_two_with_one: bool,
    exchange_two_with_two: bool,
}

impl MoveTypes {
    fn all() -> Self {
        Self { relocate: true, exchange_two_with_one: true, exchange_two_with_two: true }
    }
}

#[derive(Clone, Copy)]
enum RelativePosition {
    Before,
    After,
}

#[derive(Clone)]
enum SequenceMove {
    Relocate {
        source_route_idx: usize,
        target_route_idx: usize,
        jobs: [Job; 2],
        anchor: Job,
        position: RelativePosition,
    },
    Exchange {
        first_route_idx: usize,
        second_route_idx: usize,
        first_jobs: [Job; 2],
        second_jobs: JobSequence,
    },
}

#[derive(Clone)]
enum JobSequence {
    One(Job),
    Two([Job; 2]),
}

impl JobSequence {
    fn as_slice(&self) -> &[Job] {
        match self {
            Self::One(job) => std::slice::from_ref(job),
            Self::Two(jobs) => jobs,
        }
    }
}

struct MoveCandidate {
    estimated_cost: Cost,
    sequence_move: SequenceMove,
}

struct JobPosition {
    route_idx: usize,
    position: usize,
}

fn select_sequence_move(
    insertion_ctx: &InsertionContext,
    source_pair_threshold: usize,
    neighbor_threshold: usize,
    target_threshold: usize,
    move_types: MoveTypes,
) -> Option<SequenceMove> {
    let ordered_routes =
        insertion_ctx.solution.routes.iter().map(|route_ctx| get_ordered_jobs(route_ctx.route())).collect::<Vec<_>>();
    let job_count = ordered_routes.iter().map(Vec::len).sum();
    let mut job_positions = HashMap::with_capacity(job_count);
    ordered_routes.iter().enumerate().for_each(|(route_idx, jobs)| {
        jobs.iter().enumerate().for_each(|(position, job)| {
            job_positions.insert(job.clone(), JobPosition { route_idx, position });
        });
    });
    let is_quota_reached = || insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached());
    let locked = &insertion_ctx.solution.locked;
    let mut candidates = Vec::with_capacity(SCREEN_CANDIDATE_THRESHOLD);
    let mut source_pairs = Vec::with_capacity(job_count.saturating_sub(ordered_routes.len()));
    ordered_routes.iter().enumerate().for_each(|(route_idx, jobs)| {
        source_pairs.extend(
            jobs.windows(2)
                .enumerate()
                .filter(|(_, jobs)| jobs.iter().all(|job| !locked.contains(job)))
                .map(|(position, _)| (route_idx, position)),
        );
    });
    let sample_size = source_pair_threshold.min(source_pairs.len());
    let (source_pairs, _) = source_pairs.partial_shuffle(&mut insertion_ctx.environment.random.get_rng(), sample_size);

    for &(source_route_idx, source_position) in source_pairs.iter() {
        if is_quota_reached() {
            return None;
        }

        let source_jobs = &ordered_routes[source_route_idx];
        let profile = &insertion_ctx.solution.routes[source_route_idx].route().actor.vehicle.profile;
        let jobs = &source_jobs[source_position..source_position + 2];
        let first_jobs = [jobs[0].clone(), jobs[1].clone()];
        // The removed path is identical for all four relocation orientations around every target.
        // Calculate it once per source pair; on large routes this avoids most repeated route scans.
        let relocation_removal = if move_types.relocate {
            estimate_replacement_cost(insertion_ctx, source_route_idx, jobs, None)
        } else {
            None
        };
        let targets = insertion_ctx
            .problem
            .jobs
            .neighbors(profile, &first_jobs[0], Timestamp::default())
            .take(neighbor_threshold)
            .filter_map(|(job, cost)| job_positions.get(job).map(|position| (job, position, cost)))
            .filter(|(_, position, _)| position.route_idx != source_route_idx)
            .take(target_threshold);

        for (anchor, target, neighbor_cost) in targets {
            if is_quota_reached() {
                return None;
            }

            if move_types.relocate {
                for position in [RelativePosition::Before, RelativePosition::After] {
                    for jobs in [first_jobs.clone(), [first_jobs[1].clone(), first_jobs[0].clone()]] {
                        let sequence_move = SequenceMove::Relocate {
                            source_route_idx,
                            target_route_idx: target.route_idx,
                            jobs,
                            anchor: anchor.clone(),
                            position,
                        };
                        add_candidate(
                            &mut candidates,
                            create_candidate(insertion_ctx, sequence_move, neighbor_cost, relocation_removal),
                        );
                    }
                }
            }

            if move_types.exchange_two_with_one && !locked.contains(anchor) {
                let sequence_move = SequenceMove::Exchange {
                    first_route_idx: source_route_idx,
                    second_route_idx: target.route_idx,
                    first_jobs: first_jobs.clone(),
                    second_jobs: JobSequence::One(anchor.clone()),
                };
                add_candidate(&mut candidates, create_candidate(insertion_ctx, sequence_move, neighbor_cost, None));
            }

            if move_types.exchange_two_with_two
                && (source_route_idx, source_position) < (target.route_idx, target.position)
                && ordered_routes[target.route_idx]
                    .get(target.position..target.position + 2)
                    .is_some_and(|jobs| jobs.iter().all(|job| !locked.contains(job)))
            {
                let second_jobs = &ordered_routes[target.route_idx][target.position..target.position + 2];
                let sequence_move = SequenceMove::Exchange {
                    first_route_idx: source_route_idx,
                    second_route_idx: target.route_idx,
                    first_jobs: first_jobs.clone(),
                    second_jobs: JobSequence::Two([second_jobs[0].clone(), second_jobs[1].clone()]),
                };
                add_candidate(&mut candidates, create_candidate(insertion_ctx, sequence_move, neighbor_cost, None));
            }
        }
    }

    candidates.sort_unstable_by(|left, right| left.estimated_cost.total_cmp(&right.estimated_cost));
    candidates
        .into_iter()
        .find(|candidate| !is_quota_reached() && is_route_locally_feasible(insertion_ctx, &candidate.sequence_move))
        .map(|candidate| candidate.sequence_move)
}

fn add_candidate(candidates: &mut Vec<MoveCandidate>, candidate: MoveCandidate) {
    if candidates.len() < SCREEN_CANDIDATE_THRESHOLD {
        candidates.push(candidate);
        return;
    }

    let (worst_idx, worst) = candidates
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.estimated_cost.total_cmp(&right.estimated_cost))
        .expect("candidate list cannot be empty");

    if candidate.estimated_cost.total_cmp(&worst.estimated_cost) == Ordering::Less {
        candidates[worst_idx] = candidate;
    }
}

fn create_candidate(
    insertion_ctx: &InsertionContext,
    sequence_move: SequenceMove,
    neighbor_cost: Cost,
    relocation_removal: Option<Cost>,
) -> MoveCandidate {
    let estimated_cost = estimate_move_cost(insertion_ctx, &sequence_move, relocation_removal).unwrap_or(neighbor_cost);

    MoveCandidate { estimated_cost, sequence_move }
}

fn estimate_move_cost(
    insertion_ctx: &InsertionContext,
    sequence_move: &SequenceMove,
    relocation_removal: Option<Cost>,
) -> Option<Cost> {
    match sequence_move {
        SequenceMove::Relocate { source_route_idx, target_route_idx, jobs, anchor, position } => {
            let removal = relocation_removal?;
            let insertion = estimate_insertion_cost(
                insertion_ctx,
                *target_route_idx,
                anchor,
                *position,
                *source_route_idx,
                jobs.as_slice(),
            )?;

            Some(removal + insertion)
        }
        SequenceMove::Exchange { first_route_idx, second_route_idx, first_jobs, second_jobs } => {
            let first = estimate_replacement_cost(
                insertion_ctx,
                *first_route_idx,
                first_jobs.as_slice(),
                Some((*second_route_idx, second_jobs.as_slice())),
            )?;
            let second = estimate_replacement_cost(
                insertion_ctx,
                *second_route_idx,
                second_jobs.as_slice(),
                Some((*first_route_idx, first_jobs.as_slice())),
            )?;

            Some(first + second)
        }
    }
}

fn estimate_replacement_cost(
    insertion_ctx: &InsertionContext,
    route_idx: usize,
    removed_jobs: &[Job],
    inserted_jobs: Option<(usize, &[Job])>,
) -> Option<Cost> {
    let route = insertion_ctx.solution.routes.get(route_idx)?.route();
    let (first_idx, last_idx) = get_block_bounds(route, removed_jobs)?;
    let previous = route.tour.get(first_idx.checked_sub(1)?)?;
    let next = route.tour.get(last_idx + 1)?;
    let old_cost = get_path_cost(
        insertion_ctx,
        route,
        std::iter::once(previous)
            .chain(route.tour.activities_slice(first_idx, last_idx).iter())
            .chain(std::iter::once(next)),
    );
    let new_cost = get_path_cost(
        insertion_ctx,
        route,
        std::iter::once(previous)
            .chain(
                inserted_jobs
                    .into_iter()
                    .flat_map(|(inserted_route_idx, jobs)| get_job_activities(insertion_ctx, inserted_route_idx, jobs)),
            )
            .chain(std::iter::once(next)),
    );

    Some(new_cost - old_cost)
}

fn estimate_insertion_cost(
    insertion_ctx: &InsertionContext,
    route_idx: usize,
    anchor: &Job,
    position: RelativePosition,
    inserted_route_idx: usize,
    inserted_jobs: &[Job],
) -> Option<Cost> {
    let route = insertion_ctx.solution.routes.get(route_idx)?.route();
    let insertion_idx = get_insertion_position(route, anchor, position)?;
    let previous = route.tour.get(insertion_idx)?;
    let next = route.tour.get(insertion_idx + 1)?;
    let old_cost = get_path_cost(insertion_ctx, route, [previous, next]);
    let new_cost = get_path_cost(
        insertion_ctx,
        route,
        std::iter::once(previous)
            .chain(get_job_activities(insertion_ctx, inserted_route_idx, inserted_jobs))
            .chain(std::iter::once(next)),
    );

    Some(new_cost - old_cost)
}

fn get_block_bounds(route: &Route, jobs: &[Job]) -> Option<(usize, usize)> {
    let first_idx = jobs.iter().filter_map(|job| route.tour.index(job)).min()?;
    let last_idx = jobs.iter().filter_map(|job| route.tour.index_last(job)).max()?;
    route
        .tour
        .activities_slice(first_idx, last_idx)
        .iter()
        .all(|activity| activity.retrieve_job().as_ref().is_some_and(|job| jobs.contains(job)))
        .then_some((first_idx, last_idx))
}

fn get_job_activities<'a>(
    insertion_ctx: &'a InsertionContext,
    route_idx: usize,
    jobs: &'a [Job],
) -> impl Iterator<Item = &'a Activity> + 'a {
    let route = insertion_ctx.solution.routes.get(route_idx).expect("invalid route index").route();
    jobs.iter().flat_map(move |job| route.tour.job_activities(job))
}

fn get_path_cost<'a>(
    insertion_ctx: &InsertionContext,
    route: &Route,
    activities: impl IntoIterator<Item = &'a Activity>,
) -> Cost {
    let mut activities = activities.into_iter();
    let Some(first) = activities.next() else {
        return Cost::default();
    };

    activities
        .fold((Cost::default(), first), |(acc, previous), current| {
            let cost = insertion_ctx.problem.transport.cost(
                route,
                previous.place.location,
                current.place.location,
                TravelTime::Departure(previous.schedule.departure),
            );

            (acc + cost, current)
        })
        .0
}

fn get_insertion_position(route: &Route, anchor: &Job, position: RelativePosition) -> Option<usize> {
    match position {
        RelativePosition::Before => route.tour.index(anchor)?.checked_sub(1),
        RelativePosition::After => route.tour.index_last(anchor),
    }
}

fn is_route_locally_feasible(insertion_ctx: &InsertionContext, sequence_move: &SequenceMove) -> bool {
    match sequence_move {
        SequenceMove::Relocate { source_route_idx, target_route_idx, jobs, anchor, position } => {
            let Some(mut source_route) =
                insertion_ctx.solution.routes.get(*source_route_idx).map(|route| route.deep_copy())
            else {
                return false;
            };
            let Some(mut target_route) =
                insertion_ctx.solution.routes.get(*target_route_idx).map(|route| route.deep_copy())
            else {
                return false;
            };
            let Some(insertion_position) = get_insertion_position(target_route.route(), anchor, *position) else {
                return false;
            };

            remove_jobs_from_route(insertion_ctx, &mut source_route, jobs.as_slice())
                && insert_jobs_into_route(insertion_ctx, &mut target_route, jobs.as_slice(), insertion_position)
        }
        SequenceMove::Exchange { first_route_idx, second_route_idx, first_jobs, second_jobs } => {
            let Some(mut first_route) =
                insertion_ctx.solution.routes.get(*first_route_idx).map(|route| route.deep_copy())
            else {
                return false;
            };
            let Some(mut second_route) =
                insertion_ctx.solution.routes.get(*second_route_idx).map(|route| route.deep_copy())
            else {
                return false;
            };
            let Some(first_position) =
                first_route.route().tour.index(&first_jobs[0]).and_then(|idx| idx.checked_sub(1))
            else {
                return false;
            };
            let Some(second_position) =
                second_route.route().tour.index(&second_jobs.as_slice()[0]).and_then(|idx| idx.checked_sub(1))
            else {
                return false;
            };

            remove_jobs_from_route(insertion_ctx, &mut first_route, first_jobs.as_slice())
                && remove_jobs_from_route(insertion_ctx, &mut second_route, second_jobs.as_slice())
                && insert_jobs_into_route(insertion_ctx, &mut first_route, second_jobs.as_slice(), first_position)
                && insert_jobs_into_route(insertion_ctx, &mut second_route, first_jobs.as_slice(), second_position)
        }
    }
}

fn remove_jobs_from_route(insertion_ctx: &InsertionContext, route_ctx: &mut RouteContext, jobs: &[Job]) -> bool {
    if jobs.iter().any(|job| !route_ctx.route_mut().tour.remove(job)) {
        return false;
    }
    insertion_ctx.problem.goal.accept_route_state(route_ctx);

    true
}

fn insert_jobs_into_route(
    insertion_ctx: &InsertionContext,
    route_ctx: &mut RouteContext,
    jobs: &[Job],
    mut position: usize,
) -> bool {
    let result_selector = BestResultSelector::default();

    for job in jobs {
        if insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return false;
        }

        let eval_ctx = EvaluationContext {
            goal: insertion_ctx.problem.goal.as_ref(),
            job,
            leg_selection: &LegSelection::Exhaustive,
            result_selector: &result_selector,
        };
        let result = eval_job_insertion_in_route(
            insertion_ctx,
            &eval_ctx,
            route_ctx,
            InsertionPosition::Concrete(position),
            InsertionResult::make_failure(),
        );
        let InsertionResult::Success(success) = result else {
            return false;
        };

        success.activities.into_iter().for_each(|(activity, index)| {
            route_ctx.route_mut().tour.insert_at(activity, index + 1);
        });
        insertion_ctx.problem.goal.accept_route_state(route_ctx);
        let Some(next_position) = route_ctx.route().tour.index_last(job) else {
            return false;
        };
        position = next_position;
    }

    true
}

fn apply_sequence_move(insertion_ctx: &InsertionContext, sequence_move: SequenceMove) -> Option<InsertionContext> {
    let mut candidate = insertion_ctx.deep_copy();

    match sequence_move {
        SequenceMove::Relocate { source_route_idx, target_route_idx, jobs, anchor, position } => {
            remove_jobs_from_solution(&mut candidate, source_route_idx, jobs.as_slice())?;
            candidate.problem.goal.accept_solution_state(&mut candidate.solution);
            let insertion_position =
                get_insertion_position(candidate.solution.routes.get(target_route_idx)?.route(), &anchor, position)?;
            insert_jobs_into_solution(&mut candidate, target_route_idx, jobs.as_slice(), insertion_position)?;
        }
        SequenceMove::Exchange { first_route_idx, second_route_idx, first_jobs, second_jobs } => {
            let first_position =
                candidate.solution.routes.get(first_route_idx)?.route().tour.index(&first_jobs[0])?.checked_sub(1)?;
            let second_position = candidate
                .solution
                .routes
                .get(second_route_idx)?
                .route()
                .tour
                .index(&second_jobs.as_slice()[0])?
                .checked_sub(1)?;

            remove_jobs_from_solution(&mut candidate, first_route_idx, first_jobs.as_slice())?;
            remove_jobs_from_solution(&mut candidate, second_route_idx, second_jobs.as_slice())?;
            candidate.problem.goal.accept_solution_state(&mut candidate.solution);
            insert_jobs_into_solution(&mut candidate, first_route_idx, second_jobs.as_slice(), first_position)?;
            insert_jobs_into_solution(&mut candidate, second_route_idx, first_jobs.as_slice(), second_position)?;
        }
    }

    candidate.solution.remove_empty_routes();
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    Some(candidate)
}

fn remove_jobs_from_solution(insertion_ctx: &mut InsertionContext, route_idx: usize, jobs: &[Job]) -> Option<()> {
    {
        let route_ctx = insertion_ctx.solution.routes.get_mut(route_idx)?;
        if jobs.iter().any(|job| !route_ctx.route_mut().tour.remove(job)) {
            return None;
        }
        insertion_ctx.problem.goal.accept_route_state(route_ctx);
    }
    insertion_ctx.solution.required.extend(jobs.iter().cloned());

    Some(())
}

fn insert_jobs_into_solution(
    insertion_ctx: &mut InsertionContext,
    route_idx: usize,
    jobs: &[Job],
    mut position: usize,
) -> Option<()> {
    let result_selector = BestResultSelector::default();

    for job in jobs {
        if insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return None;
        }

        let eval_ctx = EvaluationContext {
            goal: insertion_ctx.problem.goal.as_ref(),
            job,
            leg_selection: &LegSelection::Exhaustive,
            result_selector: &result_selector,
        };
        let route_ctx = insertion_ctx.solution.routes.get(route_idx)?;
        let result = eval_job_insertion_in_route(
            insertion_ctx,
            &eval_ctx,
            route_ctx,
            InsertionPosition::Concrete(position),
            InsertionResult::make_failure(),
        );
        let InsertionResult::Success(success) = result else {
            return None;
        };

        apply_insertion_success(insertion_ctx, success);
        position = insertion_ctx.solution.routes.get(route_idx)?.route().tour.index_last(job)?;
    }

    Some(())
}

fn get_ordered_jobs(route: &Route) -> Vec<Job> {
    let capacity = route.tour.job_count();
    let mut used = HashSet::with_capacity(capacity);
    let mut jobs = Vec::with_capacity(capacity);

    route.tour.all_activities().filter_map(Activity::retrieve_job).for_each(|job| {
        if used.insert(job.clone()) {
            jobs.push(job);
        }
    });

    jobs
}
