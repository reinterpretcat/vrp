#[cfg(test)]
#[path = "../../../tests/unit/solver/search/decompose_search_test.rs"]
mod decompose_search_test;

use crate::construction::heuristics::*;
use crate::models::GoalContext;
use crate::solver::search::create_environment_with_custom_quota;
use crate::solver::*;
use crate::utils::Either;
use rand::prelude::SliceRandom;
use rosomaxa::utils::{ParallelismPolicy, parallel_collect};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::iter::{empty, once};

/// A search operator which decomposes an original solution into multiple partial solutions,
/// performs search independently, and then merges partial solutions back into one solution.
pub struct DecomposeSearch {
    inner_search: TargetSearchOperator,
    max_routes_range: (i32, i32),
    max_attempts: usize,
}

impl DecomposeSearch {
    /// Create a new instance of `DecomposeSearch`.
    pub fn new(inner_search: TargetSearchOperator, max_routes_range: (usize, usize), max_attempts: usize) -> Self {
        assert!(max_routes_range.0 > 1);
        assert!(max_routes_range.0 <= max_routes_range.1);
        assert!(max_attempts > 0);
        let max_routes_range = (max_routes_range.0 as i32, max_routes_range.1 as i32);

        Self { inner_search, max_routes_range, max_attempts }
    }
}

impl HeuristicSearchOperator for DecomposeSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        decompose_insertion_context(refinement_ctx, insertion_ctx, self.max_routes_range, self.max_attempts)
            .map(|contexts| self.refine_decomposed(refinement_ctx, insertion_ctx, contexts))
            .unwrap_or_else(|| self.inner_search.search(heuristic_ctx, insertion_ctx))
    }
}

const GREEDY_ERROR: &str = "greedy population has no insertion_ctxs";

impl DecomposeSearch {
    fn refine_decomposed(
        &self,
        refinement_ctx: &RefinementContext,
        original: &InsertionContext,
        decomposed: Vec<RefinementContext>,
    ) -> InsertionContext {
        // do actual refinement independently for each decomposed context
        let decomposed = parallel_collect(decomposed, ParallelismPolicy::Coarse, |mut refinement_ctx| {
            let _ = (0..self.max_attempts).try_for_each(|attempt| {
                let insertion_ctx = refinement_ctx.selected().next().expect(GREEDY_ERROR);
                let candidate = self.inner_search.search(&refinement_ctx, insertion_ctx);
                let improved = insertion_ctx.problem.goal.total_order(&candidate, insertion_ctx).is_lt();
                let is_quota_reached =
                    refinement_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached());
                refinement_ctx.add_solution(candidate);

                if is_quota_reached
                    || !should_retry(attempt, self.max_attempts, improved, refinement_ctx.environment.random.as_ref())
                {
                    Err(())
                } else {
                    Ok(())
                }
            });
            refinement_ctx
        });

        // get new and old parts and detect if there was any improvement in any part
        let ((new_parts, old_parts), improvements): ((Vec<_>, Vec<_>), Vec<_>) =
            decomposed.into_iter().map(get_solution_parts).unzip();

        let has_improvements = improvements.iter().any(|is_improvement| *is_improvement);
        let create_accumulator = || InsertionContext {
            problem: refinement_ctx.problem.clone(),
            solution: SolutionContext {
                required: Default::default(),
                ignored: Default::default(),
                unassigned: Default::default(),
                locked: Default::default(),
                routes: Default::default(),
                registry: original.solution.registry.deep_copy_with_all_available(),
                state: Default::default(),
            },
            environment: refinement_ctx.environment.clone(),
        };

        let mut insertion_ctx = if has_improvements {
            improvements.into_iter().zip(new_parts.into_iter().zip(old_parts)).fold(
                create_accumulator(),
                |accumulated, (is_improvement, (new_part, old_part))| {
                    merge_parts(if is_improvement { new_part } else { old_part }, accumulated)
                },
            )
        } else {
            // Keep a localized perturbation: merging every non-improving part makes damage grow with problem size.
            let random = refinement_ctx.environment.random.as_ref();
            let (first_idx, second_idx) = sample_fallback_part_indices(new_parts.len(), random);
            new_parts.into_iter().zip(old_parts).enumerate().fold(
                create_accumulator(),
                |accumulated, (idx, (new_part, old_part))| {
                    let is_selected = idx == first_idx || second_idx == Some(idx);
                    merge_parts(if is_selected { new_part } else { old_part }, accumulated)
                },
            )
        };

        insertion_ctx.restore();
        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}

fn create_population(insertion_ctx: InsertionContext) -> TargetPopulation {
    // Keep baseline and (optionally) best/last candidate without reconstructing baseline later.
    Box::new(DecomposePopulation::new(insertion_ctx.problem.goal.clone(), 1, insertion_ctx))
}

fn should_retry(attempt: usize, max_attempts: usize, improved: bool, random: &dyn Random) -> bool {
    const RETRY_AFTER_FAILURE_PROBABILITY: Float = 0.2;

    // Follow a productive descent, but occasionally restart after a failure to preserve alternative outcomes.
    attempt + 1 < max_attempts && (improved || random.is_hit(RETRY_AFTER_FAILURE_PROBABILITY))
}

/// Selects one non-improving part, and a second only when this changes no more than half of the decomposition.
fn sample_fallback_part_indices(part_count: usize, random: &dyn Random) -> (usize, Option<usize>) {
    const MAX_SELECTED_PARTS: usize = 2;

    debug_assert!(part_count > 1);

    let first_idx = random.uniform_int(0, part_count as i32 - 1) as usize;
    let second_idx = (part_count >= MAX_SELECTED_PARTS * 2).then(|| {
        let idx = random.uniform_int(0, part_count as i32 - 2) as usize;
        if idx >= first_idx { idx + 1 } else { idx }
    });

    (first_idx, second_idx)
}

fn create_multiple_insertion_contexts(
    insertion_ctx: &InsertionContext,
    environment: Arc<Environment>,
    max_routes_range: (i32, i32),
) -> Option<Vec<InsertionContext>> {
    if insertion_ctx.solution.routes.is_empty() {
        return None;
    }

    let mut route_groups = group_routes_by_proximity(insertion_ctx).into_iter().enumerate().collect::<Vec<_>>();
    // A route which is visited first claims its closest unused neighbours. Vary this order so repeated
    // decomposition can search across boundaries left by earlier partitions.
    route_groups.shuffle(&mut environment.random.get_rng());
    let (min, max) = max_routes_range;
    let max = if insertion_ctx.solution.routes.len() < max as usize { (max / 2).max(min) } else { max };

    // identify route groups and create contexts from them
    let mut used_indices = vec![false; insertion_ctx.solution.routes.len()];
    let insertion_ctxs = route_groups
        .into_iter()
        .filter_map(|(outer_idx, route_group)| {
            if used_indices[outer_idx] {
                return None;
            }

            let group_size = environment.random.uniform_int(min, max) as usize;
            let route_group = once(outer_idx)
                .chain(route_group.into_iter().filter(|inner_idx| !used_indices[*inner_idx]))
                .take(group_size)
                .collect::<HashSet<_>>();

            route_group.iter().for_each(|idx| {
                debug_assert!(!used_indices[*idx]);
                used_indices[*idx] = true;
            });

            Some(create_partial_insertion_ctx(insertion_ctx, environment.clone(), route_group))
        })
        .chain(create_empty_insertion_ctxs(insertion_ctx, environment.clone()))
        .collect();

    debug_assert!(used_indices.iter().all(|is_used| *is_used));

    Some(insertion_ctxs)
}

fn create_partial_insertion_ctx(
    insertion_ctx: &InsertionContext,
    environment: Arc<Environment>,
    route_indices: HashSet<usize>,
) -> InsertionContext {
    debug_assert!(!route_indices.is_empty());
    let solution = &insertion_ctx.solution;

    let routes = route_indices.iter().map(|idx| solution.routes[*idx].deep_copy()).collect::<Vec<_>>();
    let registry = solution
        .registry
        .deep_slice(|actor| routes.iter().any(|route_ctx| std::ptr::eq(route_ctx.route().actor.as_ref(), actor)));
    let locked = if solution.locked.is_empty() {
        HashSet::default()
    } else {
        let jobs = routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs()).collect::<HashSet<_>>();
        solution.locked.iter().filter(|job| jobs.contains(*job)).cloned().collect()
    };

    initialize_decomposed_context(InsertionContext {
        problem: insertion_ctx.problem.clone(),
        solution: SolutionContext {
            required: Default::default(),
            ignored: Default::default(),
            unassigned: Default::default(),
            locked,
            routes,
            registry,
            state: Default::default(),
        },
        environment,
    })
}

fn create_empty_insertion_ctxs(
    insertion_ctx: &InsertionContext,
    environment: Arc<Environment>,
) -> impl Iterator<Item = InsertionContext> + use<> {
    let solution = &insertion_ctx.solution;
    let locked = if solution.locked.is_empty() {
        HashSet::default()
    } else {
        let assigned =
            solution.routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs()).collect::<HashSet<_>>();
        solution.locked.iter().filter(|job| !assigned.contains(*job)).cloned().collect()
    };

    if solution.required.is_empty()
        && solution.unassigned.is_empty()
        && solution.ignored.is_empty()
        && locked.is_empty()
    {
        Either::Left(empty())
    } else {
        Either::Right(once(initialize_decomposed_context(InsertionContext {
            problem: insertion_ctx.problem.clone(),
            solution: SolutionContext {
                required: solution.required.clone(),
                ignored: solution.ignored.clone(),
                unassigned: solution.unassigned.clone(),
                locked,
                routes: Default::default(),
                registry: solution.registry.deep_copy(),
                state: Default::default(),
            },
            environment,
        })))
    }
}

fn initialize_decomposed_context(mut insertion_ctx: InsertionContext) -> InsertionContext {
    // Global state from the original solution cannot be reused by a route subset. Rebuild it before
    // the first objective, constraint, or search evaluation sees the decomposed solution.
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
    insertion_ctx
}

fn decompose_insertion_context(
    refinement_ctx: &RefinementContext,
    insertion_ctx: &InsertionContext,
    max_routes_range: (i32, i32),
    max_attempts: usize,
) -> Option<Vec<RefinementContext>> {
    const QUOTA_MULTIPLIER: Float = 1.5;

    // Keep the local quota as a runaway guard rather than a normal stopping condition.
    let median = refinement_ctx.statistics().speed.get_median();
    let limit = median.map(|median| ((median.max(10) * max_attempts) as Float * QUOTA_MULTIPLIER) as usize);
    let environment = create_environment_with_custom_quota(limit, refinement_ctx.environment.as_ref());

    create_multiple_insertion_contexts(insertion_ctx, environment.clone(), max_routes_range)
        .map(|insertion_ctxs| {
            insertion_ctxs
                .into_iter()
                .map(|insertion_ctx| {
                    RefinementContext::new(
                        refinement_ctx.problem.clone(),
                        create_population(insertion_ctx),
                        TelemetryMode::None,
                        environment.clone(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .and_then(|contexts| if contexts.len() > 1 { Some(contexts) } else { None })
}

fn get_solution_parts(decomposed_ctx: RefinementContext) -> ((SolutionContext, SolutionContext), bool) {
    let mut individuals = decomposed_ctx.into_individuals();

    // Baseline is preserved by `DecomposePopulation` and yielded first.
    let baseline = individuals.next().expect(GREEDY_ERROR);
    // The second individual is always present:
    // - if there was an improvement: best improved solution
    // - otherwise: last non-improving solution (used for diversity)
    let candidate = individuals.next().expect(GREEDY_ERROR);

    let goal = baseline.problem.goal.as_ref();

    // When there is no improvement, `candidate` is the last non-improving solution for diversity.
    // When there is an improvement, `candidate` is the best improved solution.
    let is_improvement = goal.total_order(&candidate, &baseline) == Ordering::Less;

    ((candidate.solution, baseline.solution), is_improvement)
}

fn merge_parts(source_solution: SolutionContext, accumulated: InsertionContext) -> InsertionContext {
    let mut accumulated = accumulated;
    let dest_solution = &mut accumulated.solution;

    // register routes in registry before moving them
    source_solution.routes.iter().for_each(|route_ctx| {
        assert!(dest_solution.registry.use_route(route_ctx), "attempt to use route more than once");
    });

    dest_solution.routes.extend(source_solution.routes);
    dest_solution.ignored.extend(source_solution.ignored);
    dest_solution.required.extend(source_solution.required);
    dest_solution.locked.extend(source_solution.locked);
    dest_solution.unassigned.extend(source_solution.unassigned);

    accumulated
}

/// A small population implementation used only by `DecomposeSearch`.
///
/// It preserves the original (baseline) individual so we can compare and (optionally) reuse it
/// later without reconstructing/deep-copying it again from the original full solution.
///
/// Additionally, when there is no improvement, it keeps the last non-improving candidate which
/// can be used to build a more diverse combined solution.
struct DecomposePopulation {
    objective: Arc<GoalContext>,
    selection_size: usize,

    baseline: InsertionContext,
    best: Option<InsertionContext>,
    last_non_improving: Option<InsertionContext>,
}

impl DecomposePopulation {
    fn new(objective: Arc<GoalContext>, selection_size: usize, baseline: InsertionContext) -> Self {
        Self { objective, selection_size, baseline, best: None, last_non_improving: None }
    }

    fn best_ref(&self) -> &InsertionContext {
        self.best.as_ref().unwrap_or(&self.baseline)
    }
}

impl HeuristicPopulation for DecomposePopulation {
    type Objective = GoalContext;
    type Individual = InsertionContext;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        let mut is_improved = false;
        for individual in individuals {
            is_improved = self.add(individual) || is_improved;
        }

        is_improved
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        // Greedy update: replace best only when a strictly better solution is found.
        if self.objective.total_order(self.best_ref(), &individual) == Ordering::Greater {
            self.best = Some(individual);
            // Once we found an improvement, we don't need to keep non-improving candidates.
            self.last_non_improving = None;
            true
        } else {
            // Keep the last non-improving candidate for diversity (used only when no improvements happen).
            if self.best.is_none() {
                self.last_non_improving = Some(individual);
            }
            false
        }
    }

    fn on_generation(&mut self, _: &HeuristicStatistics) {}

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.objective.total_order(a, b)
    }

    fn select(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        Box::new(std::iter::repeat_n(self.best_ref(), self.selection_size))
    }

    fn ranked(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        // Not used by `DecomposeSearch`, but provide a deterministic iteration order.
        match (&self.best, &self.last_non_improving) {
            (Some(best), _) => Box::new(once(best).chain(once(&self.baseline))),
            (None, Some(last)) => Box::new(once(&self.baseline).chain(once(last))),
            (None, None) => Box::new(once(&self.baseline)),
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        self.ranked()
    }

    fn into_iter(self: Box<Self>) -> Box<dyn Iterator<Item = Self::Individual>>
    where
        Self::Individual: 'static,
    {
        // Contract used by `get_solution_parts`:
        // - Always yield baseline first.
        // - Then yield either best (if any) or last non-improving (if any).
        Box::new(once(self.baseline).chain(self.best.or(self.last_non_improving)))
    }

    fn size(&self) -> usize {
        1 + usize::from(self.best.is_some() || self.last_non_improving.is_some())
    }

    fn selection_phase(&self) -> SelectionPhase {
        SelectionPhase::Exploitation
    }
}
