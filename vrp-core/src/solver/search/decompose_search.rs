#[cfg(test)]
#[path = "../../../tests/unit/solver/search/decompose_search_test.rs"]
mod decompose_search_test;

use crate::construction::heuristics::*;
use crate::models::GoalContext;
use crate::solver::search::create_environment_with_custom_quota;
use crate::solver::*;
use crate::utils::Either;
use rosomaxa::utils::parallel_into_collect;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::iter::{empty, once};

/// A search operator which decomposes an original solution into multiple partial solutions,
/// performs search independently, and then merges partial solutions back into one solution.
pub struct DecomposeSearch {
    inner_search: TargetSearchOperator,
    max_routes_range: (i32, i32),
    repeat_count: usize,
}

impl DecomposeSearch {
    /// Create a new instance of `DecomposeSearch`.
    pub fn new(inner_search: TargetSearchOperator, max_routes_range: (usize, usize), repeat_count: usize) -> Self {
        assert!(max_routes_range.0 > 1);
        let max_routes_range = (max_routes_range.0 as i32, max_routes_range.1 as i32);

        Self { inner_search, max_routes_range, repeat_count }
    }
}

impl HeuristicSearchOperator for DecomposeSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        decompose_insertion_context(refinement_ctx, insertion_ctx, self.max_routes_range, self.repeat_count)
            .map(|contexts| self.refine_decomposed(refinement_ctx, contexts))
            .unwrap_or_else(|| self.inner_search.search(heuristic_ctx, insertion_ctx))
    }
}

const GREEDY_ERROR: &str = "greedy population has no insertion_ctxs";

impl DecomposeSearch {
    fn refine_decomposed(
        &self,
        refinement_ctx: &RefinementContext,
        decomposed: Vec<(RefinementContext, HashSet<usize>)>,
    ) -> InsertionContext {
        // NOTE: validate decomposition in debug builds only
        #[cfg(debug_assertions)]
        decomposed.iter().enumerate().for_each(|(outer_ix, (_, outer))| {
            decomposed.iter().enumerate().filter(|(inner_idx, _)| outer_ix != *inner_idx).for_each(
                |(_, (_, inner))| {
                    debug_assert!(outer.intersection(inner).next().is_none());
                },
            );
        });

        // do actual refinement independently for each decomposed context
        let decomposed = parallel_into_collect(decomposed, |(mut refinement_ctx, route_indices)| {
            let actual_repeat_count = get_repeat_count(self.repeat_count, refinement_ctx.environment.random.as_ref());

            let _ = (0..actual_repeat_count).try_for_each(|_| {
                let insertion_ctx = refinement_ctx.selected().next().expect(GREEDY_ERROR);
                let insertion_ctx = self.inner_search.search(&refinement_ctx, insertion_ctx);
                let is_quota_reached =
                    refinement_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached());
                refinement_ctx.add_solution(insertion_ctx);

                if is_quota_reached { Err(()) } else { Ok(()) }
            });
            (refinement_ctx, route_indices)
        });

        // get new and old parts and detect if there was any improvement in any part
        let ((new_parts, old_parts), improvements): ((Vec<_>, Vec<_>), Vec<_>) =
            decomposed.into_iter().map(get_solution_parts).unzip();

        let has_improvements = improvements.iter().any(|is_improvement| *is_improvement);

        let mut insertion_ctx = if has_improvements {
            improvements.into_iter().zip(new_parts.into_iter().zip(old_parts)).fold(
                InsertionContext::new_empty(refinement_ctx.problem.clone(), refinement_ctx.environment.clone()),
                |accumulated, (is_improvement, (new_part, old_part))| {
                    merge_parts(if is_improvement { new_part } else { old_part }, accumulated)
                },
            )
        } else {
            new_parts.into_iter().fold(
                InsertionContext::new_empty(refinement_ctx.problem.clone(), refinement_ctx.environment.clone()),
                |accumulated, new_part| merge_parts(new_part, accumulated),
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

/// Selects a repeat count from 1 to max_repeat_count using exponential decay.
/// Uses stack-allocated arrays for common cases to avoid heap allocation.
fn get_repeat_count(max_repeat_count: usize, random: &dyn Random) -> usize {
    if max_repeat_count == 1 {
        return 1;
    }

    // create weights with exponential decay: [3^(n-1), 3^(n-2), ..., 3^1, 3^0]
    let index = match max_repeat_count {
        2 => random.weighted(&[3, 1]),
        3 => random.weighted(&[9, 3, 1]),
        4 => random.weighted(&[27, 9, 3, 1]),
        _ => {
            let weights: Vec<_> = (1..=max_repeat_count).map(|i| 3_usize.pow((max_repeat_count - i) as u32)).collect();
            random.weighted(&weights) + 1
        }
    };

    index + 1
}

fn create_multiple_insertion_contexts(
    insertion_ctx: &InsertionContext,
    environment: Arc<Environment>,
    max_routes_range: (i32, i32),
) -> Option<Vec<(InsertionContext, HashSet<usize>)>> {
    if insertion_ctx.solution.routes.is_empty() {
        return None;
    }

    let route_groups = group_routes_by_proximity(insertion_ctx);
    let (min, max) = max_routes_range;
    let max = if insertion_ctx.solution.routes.len() < max as usize { (max / 2).max(min) } else { max };

    // identify route groups and create contexts from them
    let mut used_indices: HashSet<usize> = HashSet::new();
    let insertion_ctxs = route_groups
        .into_iter()
        .enumerate()
        .filter_map(|(outer_idx, route_group)| {
            if used_indices.contains(&outer_idx) {
                return None;
            }

            let group_size = environment.random.uniform_int(min, max) as usize;
            let route_group = once(outer_idx)
                .chain(route_group.into_iter().filter(|inner_idx| !used_indices.contains(inner_idx)))
                .take(group_size)
                .collect::<HashSet<_>>();

            used_indices.extend(route_group.iter().copied());

            Some(create_partial_insertion_ctx(insertion_ctx, environment.clone(), route_group))
        })
        .chain(create_empty_insertion_ctxs(insertion_ctx, environment.clone()))
        .collect();

    Some(insertion_ctxs)
}

fn create_partial_insertion_ctx(
    insertion_ctx: &InsertionContext,
    environment: Arc<Environment>,
    route_indices: HashSet<usize>,
) -> (InsertionContext, HashSet<usize>) {
    let solution = &insertion_ctx.solution;

    let routes = route_indices.iter().map(|idx| solution.routes[*idx].deep_copy()).collect::<Vec<_>>();
    let actors = routes.iter().map(|route_ctx| route_ctx.route().actor.clone()).collect::<HashSet<_>>();
    let registry = solution.registry.deep_slice(|actor| actors.contains(actor));

    (
        InsertionContext {
            problem: insertion_ctx.problem.clone(),
            solution: SolutionContext {
                // NOTE we need to handle empty route indices case differently
                required: if route_indices.is_empty() { solution.required.clone() } else { Default::default() },
                ignored: if route_indices.is_empty() { solution.ignored.clone() } else { Default::default() },
                unassigned: if route_indices.is_empty() { solution.unassigned.clone() } else { Default::default() },
                locked: if route_indices.is_empty() {
                    let jobs = solution
                        .routes
                        .iter()
                        .flat_map(|route_ctx| route_ctx.route().tour.jobs())
                        .collect::<HashSet<_>>();
                    solution.locked.iter().filter(|job| !jobs.contains(*job)).cloned().collect()
                } else {
                    let jobs =
                        routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs()).collect::<HashSet<_>>();
                    solution.locked.iter().filter(|job| jobs.contains(*job)).cloned().collect()
                },
                routes,
                registry,
                state: Default::default(),
            },
            environment,
        },
        route_indices,
    )
}

fn create_empty_insertion_ctxs(
    insertion_ctx: &InsertionContext,
    environment: Arc<Environment>,
) -> impl Iterator<Item = (InsertionContext, HashSet<usize>)> + use<> {
    let solution = &insertion_ctx.solution;

    if solution.required.is_empty()
        && solution.unassigned.is_empty()
        && solution.ignored.is_empty()
        && solution.locked.is_empty()
    {
        Either::Left(empty())
    } else {
        Either::Right(once((
            InsertionContext {
                problem: insertion_ctx.problem.clone(),
                solution: SolutionContext {
                    required: solution.required.clone(),
                    ignored: solution.ignored.clone(),
                    unassigned: solution.unassigned.clone(),
                    locked: solution.locked.clone(),
                    routes: Default::default(),
                    registry: solution.registry.deep_copy(),
                    state: Default::default(),
                },
                environment,
            },
            HashSet::default(),
        )))
    }
}

fn decompose_insertion_context(
    refinement_ctx: &RefinementContext,
    insertion_ctx: &InsertionContext,
    max_routes_range: (i32, i32),
    repeat: usize,
) -> Option<Vec<(RefinementContext, HashSet<usize>)>> {
    // NOTE make limit a bit higher than median
    let median = refinement_ctx.statistics().speed.get_median();
    let limit = median.map(|median| (((median.max(10) * repeat) as f64) * 1.5) as usize);
    let environment = create_environment_with_custom_quota(limit, refinement_ctx.environment.as_ref());

    create_multiple_insertion_contexts(insertion_ctx, environment.clone(), max_routes_range)
        .map(|insertion_ctxs| {
            insertion_ctxs
                .into_iter()
                .map(|(insertion_ctx, indices)| {
                    (
                        RefinementContext::new(
                            refinement_ctx.problem.clone(),
                            create_population(insertion_ctx),
                            TelemetryMode::None,
                            environment.clone(),
                        ),
                        indices,
                    )
                })
                .collect::<Vec<_>>()
        })
        .and_then(|contexts| if contexts.len() > 1 { Some(contexts) } else { None })
}

fn get_solution_parts(decomposed: (RefinementContext, HashSet<usize>)) -> ((SolutionContext, SolutionContext), bool) {
    let (decomposed_ctx, _) = decomposed;
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
        if individuals.is_empty() {
            return false;
        }

        individuals.into_iter().any(|individual| self.add(individual))
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
        if let Some(best) = self.best.as_ref() {
            Box::new(once(best).chain(once(&self.baseline)))
        } else {
            Box::new(once(&self.baseline))
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
