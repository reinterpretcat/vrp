#[cfg(test)]
#[path = "../../../tests/unit/solver/search/decompose_search_test.rs"]
mod decompose_search_test;

use crate::construction::heuristics::*;
use crate::models::GoalContext;
use crate::solver::search::create_environment_with_custom_quota;
use crate::solver::*;
use crate::utils::Either;
use rosomaxa::utils::parallel_into_collect;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::iter::{empty, once};

/// A search operator which decomposes original solution into multiple partial solutions,
/// preforms search independently, and then merges partial solution back into one solution.
pub struct DecomposeSearch {
    inner_search: TargetSearchOperator,
    max_routes_range: (i32, i32),
    repeat_count: usize,
    quota_limit: usize,
}

impl DecomposeSearch {
    /// Create a new instance of `DecomposeSearch`.
    pub fn new(
        inner_search: TargetSearchOperator,
        max_routes_range: (usize, usize),
        repeat_count: usize,
        quota_limit: usize,
    ) -> Self {
        assert!(max_routes_range.0 > 1);
        let max_routes_range = (max_routes_range.0 as i32, max_routes_range.1 as i32);

        Self { inner_search, max_routes_range, repeat_count, quota_limit }
    }
}

impl HeuristicSearchOperator for DecomposeSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        decompose_insertion_context(
            refinement_ctx,
            insertion_ctx,
            self.max_routes_range,
            self.repeat_count,
            self.quota_limit,
        )
        .map(|contexts| self.refine_decomposed(refinement_ctx, insertion_ctx, contexts))
        .unwrap_or_else(|| self.inner_search.search(heuristic_ctx, insertion_ctx))
    }
}

const GREEDY_ERROR: &str = "greedy population has no insertion_ctxs";

impl DecomposeSearch {
    fn refine_decomposed(
        &self,
        refinement_ctx: &RefinementContext,
        original_insertion_ctx: &InsertionContext,
        decomposed: Vec<(RefinementContext, HashSet<usize>)>,
    ) -> InsertionContext {
        // NOTE: validate decomposition
        decomposed.iter().enumerate().for_each(|(outer_ix, (_, outer))| {
            decomposed.iter().enumerate().filter(|(inner_idx, _)| outer_ix != *inner_idx).for_each(
                |(_, (_, inner))| {
                    assert!(outer.intersection(inner).next().is_none());
                },
            );
        });

        // do actual refinement independently for each decomposed context
        let decomposed = parallel_into_collect(decomposed, |(mut refinement_ctx, route_indices)| {
            let _ = (0..self.repeat_count).try_for_each(|_| {
                let insertion_ctx = refinement_ctx.selected().next().expect(GREEDY_ERROR);
                let insertion_ctx = self.inner_search.search(&refinement_ctx, insertion_ctx);
                let is_quota_reached =
                    refinement_ctx.environment.quota.as_ref().map_or(false, |quota| quota.is_reached());
                refinement_ctx.add_solution(insertion_ctx);

                if is_quota_reached {
                    Err(())
                } else {
                    Ok(())
                }
            });
            (refinement_ctx, route_indices)
        });

        // merge evolution results into one insertion_ctx
        let mut insertion_ctx = decomposed.into_iter().fold(
            InsertionContext::new_empty(refinement_ctx.problem.clone(), refinement_ctx.environment.clone()),
            |insertion_ctx, decomposed| merge_best(decomposed, original_insertion_ctx, insertion_ctx),
        );

        insertion_ctx.restore();
        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}

fn create_population(insertion_ctx: InsertionContext) -> TargetPopulation {
    Box::new(GreedyPopulation::new(insertion_ctx.problem.goal.clone(), 1, Some(insertion_ctx)))
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
    let max = if insertion_ctx.solution.routes.len() < 4 { 2 } else { max };

    // identify route groups and create contexts from them
    let used_indices = RefCell::new(HashSet::new());
    let insertion_ctxs = route_groups
        .into_iter()
        .enumerate()
        .filter(|(outer_idx, _)| !used_indices.borrow().contains(outer_idx))
        .map(|(outer_idx, route_group)| {
            let group_size = environment.random.uniform_int(min, max) as usize;
            let route_group = once(outer_idx)
                .chain(route_group.into_iter().filter(|inner_idx| !used_indices.borrow().contains(inner_idx)))
                .take(group_size)
                .collect::<HashSet<_>>();

            used_indices.borrow_mut().extend(route_group.iter().cloned());

            create_partial_insertion_ctx(insertion_ctx, environment.clone(), route_group)
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
) -> impl Iterator<Item = (InsertionContext, HashSet<usize>)> {
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
    quota_limit: usize,
) -> Option<Vec<(RefinementContext, HashSet<usize>)>> {
    // NOTE make limit a bit higher than median
    let median = refinement_ctx.statistics().speed.get_median();
    let limit = median.map(|median| (median * repeat).max(quota_limit));
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

fn merge_best(
    decomposed: (RefinementContext, HashSet<usize>),
    original_insertion_ctx: &InsertionContext,
    accumulated: InsertionContext,
) -> InsertionContext {
    let (decomposed_ctx, route_indices) = decomposed;
    let decomposed_insertion_ctx = decomposed_ctx.ranked().next().expect(GREEDY_ERROR);
    let environment = original_insertion_ctx.environment.clone();

    let (partial_insertion_ctx, _) = create_partial_insertion_ctx(original_insertion_ctx, environment, route_indices);
    let goal = partial_insertion_ctx.problem.goal.as_ref();

    let source_solution = if goal.total_order(decomposed_insertion_ctx, &partial_insertion_ctx) == Ordering::Less {
        &decomposed_insertion_ctx.solution
    } else {
        &partial_insertion_ctx.solution
    };

    let mut accumulated = accumulated;
    let dest_solution = &mut accumulated.solution;

    // NOTE theoretically, we can avoid deep copy here, but this would require an extension in Population trait
    dest_solution.routes.extend(source_solution.routes.iter().map(|route_ctx| route_ctx.deep_copy()));
    dest_solution.ignored.extend(source_solution.ignored.iter().cloned());
    dest_solution.required.extend(source_solution.required.iter().cloned());
    dest_solution.locked.extend(source_solution.locked.iter().cloned());
    dest_solution.unassigned.extend(source_solution.unassigned.iter().map(|(k, v)| (k.clone(), v.clone())));

    source_solution.routes.iter().for_each(|route_ctx| {
        assert!(dest_solution.registry.use_route(route_ctx), "attempt to use route more than once");
    });

    accumulated
}
