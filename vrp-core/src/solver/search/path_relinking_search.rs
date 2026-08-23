#[cfg(test)]
#[path = "../../../tests/unit/solver/search/path_relinking_search_test.rs"]
mod path_relinking_search_test;

use crate::construction::heuristics::*;
use crate::models::GoalContext;
use crate::models::problem::Job;
use crate::solver::search::Recreate;
use crate::solver::{RefinementContext, TargetSearchOperator};
use rosomaxa::algorithms::math::relative_distance;
use rosomaxa::hyper::HeuristicDiversifyOperator;
use rosomaxa::prelude::{Float, HeuristicContext, HeuristicObjective, HeuristicSolution};
use std::cmp::{Ordering, Reverse};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

/// A bounded route-based path relinking search.
///
/// The search waits until the population has had time to form, then pairs a selected parent with
/// a structurally different solution from the better half of the current selection. It starts from
/// the worse endpoint and copies one guiding route at a time onto a compatible source actor. Source
/// and guiding routes are used at most once, so every step commits another route block instead of
/// overwriting previous progress. Normal insertion constraints validate every copied route and the
/// configured recreate repairs displaced jobs.
///
/// Only the best repaired point on the path receives an additional ruin/recreate search. A child
/// is returned only when it meaningfully improves both parents under the configured objective;
/// unsuccessful recombination therefore cannot replace regular diversification. Calls are spaced
/// by a problem-sized generation interval; a lock-free reservation lets one of the parallel selected
/// parents attempt the search.
///
/// This follows the elite-reference-set and truncated trajectory ideas of path relinking, but uses
/// route blocks to remain useful with the framework's arbitrary constraints and objectives.
/// The implementation is inspired by the general review of Laguna et al. (2024) and VRP variants
/// by Schittekat and Sörensen (2013) and Tarantilis et al. (2013), rather than reproducing their
/// problem-specific moves.
///
/// Laguna et al. (2024): <https://arxiv.org/abs/2312.12663>
/// Schittekat and Sörensen (2013): <https://doi.org/10.1016/j.cor.2013.02.005>
/// Tarantilis et al. (2013): <https://doi.org/10.1287/trsc.1120.0439>
pub struct PathRelinkingSearch {
    recreate: Arc<dyn Recreate>,
    inner_search: TargetSearchOperator,
    max_affected_activities: usize,
    max_transitions: usize,
    next_generation: AtomicUsize,
}

impl PathRelinkingSearch {
    /// Creates a new instance of `PathRelinkingSearch`.
    pub fn new(
        recreate: Arc<dyn Recreate>,
        inner_search: TargetSearchOperator,
        max_affected_activities: usize,
        max_transitions: usize,
    ) -> Self {
        assert!(max_affected_activities > 0);
        assert!(max_transitions > 0);
        Self {
            recreate,
            inner_search,
            max_affected_activities,
            max_transitions,
            next_generation: AtomicUsize::default(),
        }
    }

    fn relink(
        &self,
        refinement_ctx: &RefinementContext,
        source: &InsertionContext,
        target: &InsertionContext,
    ) -> Option<InsertionContext> {
        let source_unassigned = source.solution.unassigned.len();
        let mut partial = source.deep_copy();
        let mut remaining = self.max_affected_activities;
        let mut best = None::<InsertionContext>;
        let mut committed_source_routes = HashSet::new();
        let mut committed_target_routes = HashSet::new();

        // `partial` is the monotone structural path. Repaired checkpoints are evaluated separately
        // and never become its next state, as repair can move jobs away from the guiding structure.
        for _ in 0..self.max_transitions {
            let Some((next_partial, route_pair)) =
                select_route_pairs(&partial, target, remaining, &committed_source_routes, &committed_target_routes)
                    .into_iter()
                    .find_map(|route_pair| {
                        transplant_route(&partial, route_pair.source_idx, route_pair.target_route)
                            .map(|candidate| (candidate, route_pair))
                    })
            else {
                break;
            };

            partial = next_partial;
            remaining = remaining.saturating_sub(route_pair.affected);
            // Guide routes are disjoint. Preventing reuse on both sides means a later transplant
            // cannot remove jobs from a route block which was already committed.
            committed_source_routes.insert(route_pair.source_idx);
            committed_target_routes.insert(route_pair.target_idx);

            // A route transplant deliberately leaves displaced jobs in `required`; recreate turns
            // this structural checkpoint into a complete solution through normal insertion rules.
            let mut candidate = self.recreate.run(refinement_ctx, partial.deep_copy());
            finalize_insertion_ctx(&mut candidate);

            // This operator starts from two feasible parents. Do not turn structural exploration into
            // a growing backlog which makes every subsequent insertion search more expensive.
            if candidate.solution.unassigned.len() > source_unassigned {
                continue;
            }

            let is_better = best
                .as_ref()
                .is_none_or(|best| refinement_ctx.objective().total_order(&candidate, best) == Ordering::Less);
            if is_better {
                best = Some(candidate);
            }

            if remaining == 0 {
                break;
            }
        }

        let mut candidate = best?;

        // The repaired intermediates do not define the next structural step, so improve only the
        // best one instead of spending a full ruin/recreate search at every point on the path.
        let mut improved = self.inner_search.search(refinement_ctx, &candidate);
        finalize_insertion_ctx(&mut improved);
        if improved.solution.unassigned.len() <= source_unassigned
            && refinement_ctx.objective().total_order(&improved, &candidate) == Ordering::Less
        {
            candidate = improved;
        }

        Some(candidate)
    }

    fn relink_pair(
        &self,
        refinement_ctx: &RefinementContext,
        first: &InsertionContext,
        second: &InsertionContext,
    ) -> Option<InsertionContext> {
        // A truncated path spends most of its work near the initiating endpoint. Start with the
        // worse parent and use the better one as the guide instead of paying for both directions.
        let (source, target) = if refinement_ctx.objective().total_order(first, second) == Ordering::Less {
            (second, first)
        } else {
            (first, second)
        };
        let candidate = self.relink(refinement_ctx, source, target)?;

        // A complete transition is attempted within one invocation. Keep it only when recombination
        // discovers something which neither parent already represents.
        if [first, second].into_iter().all(|parent| is_meaningfully_better(refinement_ctx, &candidate, parent)) {
            Some(candidate)
        } else {
            None
        }
    }

    fn try_relink(&self, refinement_ctx: &RefinementContext, source: &InsertionContext) -> Option<InsertionContext> {
        let generation = refinement_ctx.statistics().generation;
        let interval = refinement_ctx.problem.jobs.size().max(MIN_RELINK_INTERVAL);
        let next_generation = self.next_generation.load(AtomicOrdering::Relaxed);
        // The first call only arms the schedule, giving Rosomaxa time to form a reference set.
        // Afterwards the compare-exchange reserves one attempt among parallel selected parents.
        if next_generation == 0 {
            let _ = self.next_generation.compare_exchange(
                0,
                generation.saturating_add(interval),
                AtomicOrdering::Relaxed,
                AtomicOrdering::Relaxed,
            );
            return None;
        }
        if generation < next_generation
            || self
                .next_generation
                .compare_exchange(
                    next_generation,
                    generation.saturating_add(interval),
                    AtomicOrdering::Relaxed,
                    AtomicOrdering::Relaxed,
                )
                .is_err()
        {
            return None;
        }

        let (target, _) = select_target(refinement_ctx, source)?;

        self.relink_pair(refinement_ctx, source, target)
    }
}

fn is_meaningfully_better(
    refinement_ctx: &RefinementContext,
    candidate: &InsertionContext,
    parent: &InsertionContext,
) -> bool {
    refinement_ctx.objective().total_order(candidate, parent) == Ordering::Less
        && relative_distance(candidate.fitness(), parent.fitness()) > Float::EPSILON.sqrt()
}

// Rich problems can make the structurally closest source actor incompatible with a target route.
// Keep a small candidate set rather than rebuilding every possible route pair.
const ROUTE_PAIR_ATTEMPTS: usize = 8;
// Very short paths only revisit the neighborhoods in which both locally improved endpoints already lie.
const MIN_RELINK_ATTRIBUTES: usize = 4;
// Sample among a few distant guides to avoid repeatedly relinking the same pair of basins.
const GUIDE_CANDIDATE_LIST_SIZE: usize = 3;
// Small instances are cheap, but invoking a population-level search every few generations would
// spend more effort recombining the same basins than discovering new ones.
const MIN_RELINK_INTERVAL: usize = 100;

#[derive(Clone, Copy)]
struct RoutePair<'a> {
    source_idx: usize,
    target_idx: usize,
    target_route: &'a RouteContext,
    affected: usize,
    novelty: usize,
    changed: usize,
}

impl RoutePair<'_> {
    fn sort_key(&self) -> (usize, Reverse<usize>) {
        (self.changed, Reverse(self.novelty))
    }
}

struct SolutionStructure {
    // Route indices are used only to recover the partition. The distance below compares
    // intersections, so equivalent partitions do not depend on actor or route ordering.
    assignments: HashMap<Job, usize>,
    // Directed job edges retain sequence information which the route partition cannot express.
    edges: HashSet<(Job, Job)>,
    route_pairs: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct StructuralDistance {
    route_partition: usize,
    adjacency: usize,
    route_pair_scale: usize,
    adjacency_scale: usize,
}

impl StructuralDistance {
    fn attributes(self) -> usize {
        self.route_partition + self.adjacency
    }

    fn score(self) -> Float {
        let normalized = |value: usize, scale: usize| if scale == 0 { 0. } else { value as Float / scale as Float };

        normalized(self.route_partition, self.route_pair_scale) + normalized(self.adjacency, self.adjacency_scale)
    }
}

impl HeuristicDiversifyOperator for PathRelinkingSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        self.try_relink(heuristic_ctx, solution).into_iter().collect()
    }
}

fn select_target<'a>(
    refinement_ctx: &'a RefinementContext,
    source: &InsertionContext,
) -> Option<(&'a InsertionContext, StructuralDistance)> {
    // Establish the quality boundary before removing duplicates. Otherwise a set of leading clones
    // would let weak solutions outside the better half leak into the reference set.
    let candidates = select_quality_half(refinement_ctx.selected().collect(), |left, right| {
        refinement_ctx.objective().total_order(left, right)
    });
    let source_structure = SolutionStructure::new(source);
    let mut candidates = candidates
        .into_iter()
        .filter_map(|candidate| {
            let distance = source_structure.distance(&SolutionStructure::new(candidate));
            (distance.attributes() >= MIN_RELINK_ATTRIBUTES).then_some((candidate, distance))
        })
        .collect::<Vec<_>>();

    candidates.sort_unstable_by(|(_, left), (_, right)| right.score().total_cmp(&left.score()));
    candidates.truncate(GUIDE_CANDIDATE_LIST_SIZE);

    if candidates.is_empty() {
        return None;
    }

    // A small restricted candidate list keeps guides structurally distant without repeatedly
    // attracting every path to the same farthest solution.
    let index = refinement_ctx.environment.random.uniform_int(0, candidates.len() as i32 - 1) as usize;
    candidates.into_iter().nth(index)
}

fn select_quality_half<T>(mut candidates: Vec<T>, mut compare: impl FnMut(&T, &T) -> Ordering) -> Vec<T> {
    candidates.sort_unstable_by(|left, right| compare(left, right));
    candidates.truncate(candidates.len().div_ceil(2));
    candidates
}

fn select_route_pairs<'a>(
    source: &InsertionContext,
    target: &'a InsertionContext,
    max_affected_activities: usize,
    committed_source_routes: &HashSet<usize>,
    committed_target_routes: &HashSet<usize>,
) -> Vec<RoutePair<'a>> {
    // Prefer a source route which already shares most guide jobs. This makes each transition small;
    // new guide edges break ties without assuming that transport cost is part of the objective.
    let source_edges = get_solution_edges(source);
    let is_locked = |job: &Job| source.solution.locked.contains(job) || target.solution.locked.contains(job);
    let source_routes = source
        .solution
        .routes
        .iter()
        .enumerate()
        .filter(|(route_idx, _)| !committed_source_routes.contains(route_idx))
        .filter(|(_, route)| !route.route().tour.jobs().any(&is_locked))
        .map(|(route_idx, route)| (route_idx, route.route().tour.jobs().cloned().collect::<HashSet<_>>()))
        .collect::<Vec<_>>();
    let mut pairs = Vec::with_capacity(ROUTE_PAIR_ATTEMPTS);

    for (target_idx, target_route) in target
        .solution
        .routes
        .iter()
        .enumerate()
        .filter(|(route_idx, _)| !committed_target_routes.contains(route_idx))
        .filter(|(_, route)| !route.route().tour.jobs().any(&is_locked))
    {
        let target_jobs = target_route.route().tour.jobs().cloned().collect::<HashSet<_>>();
        if target_jobs.is_empty() {
            continue;
        }
        let novelty = get_route_edges(target_route).filter(|edge| !source_edges.contains(edge)).count();

        for (source_route_idx, source_jobs) in &source_routes {
            let affected = source_jobs.union(&target_jobs).map(get_job_activity_count).sum();
            if affected == 0 || affected > max_affected_activities {
                continue;
            }

            let changed = source_jobs.symmetric_difference(&target_jobs).count();
            if changed == 0 && novelty == 0 {
                continue;
            }

            keep_route_pair(
                &mut pairs,
                RoutePair { source_idx: *source_route_idx, target_idx, target_route, affected, novelty, changed },
                ROUTE_PAIR_ATTEMPTS,
            );
        }
    }

    pairs
}

fn keep_route_pair<'a>(pairs: &mut Vec<RoutePair<'a>>, candidate: RoutePair<'a>, capacity: usize) {
    let candidate_key = candidate.sort_key();
    let insert_idx = pairs.partition_point(|pair| pair.sort_key() <= candidate_key);

    if insert_idx < capacity {
        pairs.insert(insert_idx, candidate);
        pairs.truncate(capacity);
    }
}

fn transplant_route(
    source: &InsertionContext,
    source_route_idx: usize,
    target_route: &RouteContext,
) -> Option<InsertionContext> {
    let target_jobs = target_route.route().tour.jobs().cloned().collect::<HashSet<_>>();
    if target_jobs.is_empty() {
        return None;
    }
    let mut candidate = source.deep_copy();
    let source_jobs = candidate.solution.routes.get(source_route_idx)?.route().tour.jobs().cloned().collect::<Vec<_>>();
    let displaced = source_jobs.iter().filter(|job| !target_jobs.contains(*job)).cloned().collect::<Vec<_>>();

    // Remove guide jobs globally before recreating their target order on the chosen source actor.
    // This preserves job uniqueness even when the parents assign the jobs to different routes.
    for (route_idx, route) in candidate.solution.routes.iter_mut().enumerate() {
        let removed = route
            .route()
            .tour
            .jobs()
            .filter(|job| route_idx == source_route_idx || target_jobs.contains(*job))
            .cloned()
            .collect::<Vec<_>>();
        if removed.is_empty() {
            continue;
        }

        for job in removed {
            if !route.route_mut().tour.remove(&job) {
                return None;
            }
        }
        candidate.problem.goal.accept_route_state(route);
    }

    candidate.solution.required.retain(|job| !target_jobs.contains(job));
    candidate.solution.ignored.retain(|job| !target_jobs.contains(job));
    candidate.solution.unassigned.retain(|job, _| !target_jobs.contains(job));
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    // Reinsert jobs rather than copying activities: the regular feature pipeline owns feasibility
    // for capacity, time windows, pickup/delivery, breaks, and custom constraints.
    let result_selector = BestResultSelector::default();
    for job in get_ordered_jobs(target_route) {
        let eval_ctx = EvaluationContext {
            goal: candidate.problem.goal.as_ref(),
            job: &job,
            leg_selection: &LegSelection::Exhaustive,
            result_selector: &result_selector,
        };
        let route = candidate.solution.routes.get(source_route_idx)?;
        let result = eval_job_insertion_in_route(
            &candidate,
            &eval_ctx,
            route,
            InsertionPosition::Last,
            InsertionResult::make_failure(),
        );
        let InsertionResult::Success(success) = result else {
            return None;
        };
        apply_insertion_success(&mut candidate, success);
    }

    candidate.solution.required.extend(displaced);
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    Some(candidate)
}

fn get_ordered_jobs(route: &RouteContext) -> Vec<Job> {
    let mut seen = HashSet::new();

    route
        .route()
        .tour
        .all_activities()
        .filter_map(|activity| activity.retrieve_job())
        .filter(|job| seen.insert(job.clone()))
        .collect()
}

fn get_job_activity_count(job: &Job) -> usize {
    job.as_multi().map_or(1, |multi| multi.jobs.len())
}

impl SolutionStructure {
    fn new(insertion_ctx: &InsertionContext) -> Self {
        let mut route_pairs = 0;
        let mut assignments = HashMap::new();

        for (route_idx, route) in insertion_ctx.solution.routes.iter().enumerate() {
            route_pairs += get_pair_count(route.route().tour.job_count());
            assignments.extend(route.route().tour.jobs().cloned().map(|job| (job, route_idx)));
        }

        Self { assignments, edges: get_solution_edges(insertion_ctx), route_pairs }
    }

    fn distance(&self, other: &Self) -> StructuralDistance {
        let mut intersections = HashMap::<(usize, usize), usize>::new();
        self.assignments
            .iter()
            .filter_map(|(job, left_route)| other.assignments.get(job).map(|right_route| (*left_route, *right_route)))
            .for_each(|route_pair| *intersections.entry(route_pair).or_default() += 1);
        let common_route_pairs = intersections.into_values().map(get_pair_count).sum::<usize>();

        StructuralDistance {
            route_partition: self.route_pairs + other.route_pairs - 2 * common_route_pairs,
            adjacency: self.edges.symmetric_difference(&other.edges).count(),
            route_pair_scale: self.route_pairs + other.route_pairs,
            adjacency_scale: self.edges.len() + other.edges.len(),
        }
    }
}

fn get_pair_count(size: usize) -> usize {
    size.saturating_mul(size.saturating_sub(1)) / 2
}

fn get_solution_edges(insertion_ctx: &InsertionContext) -> HashSet<(Job, Job)> {
    insertion_ctx.solution.routes.iter().flat_map(get_route_edges).collect()
}

fn get_route_edges(route: &RouteContext) -> impl Iterator<Item = (Job, Job)> + '_ {
    route
        .route()
        .tour
        .all_activities()
        .filter_map(|activity| activity.retrieve_job())
        .scan(None, |previous, current| {
            let edge = previous.take().map(|previous| (previous, current.clone()));
            *previous = Some(current);
            Some(edge)
        })
        .flatten()
}
