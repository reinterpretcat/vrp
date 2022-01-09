#[cfg(test)]
#[path = "../../../tests/unit/solver/search/decompose_search_test.rs"]
mod decompose_search_test;

use crate::construction::heuristics::*;
use crate::solver::*;
use hashbrown::HashSet;
use rand::prelude::SliceRandom;
use rosomaxa::utils::parallel_into_collect;
use std::cmp::Ordering;
use std::iter::{empty, once};
use std::sync::RwLock;

/// A search operator which decomposes original solution into multiple partial solutions,
/// preforms search independently, and then merges partial solution back into one solution.
pub struct DecomposeSearch {
    inner_search: TargetHeuristicOperator,
    max_routes_range: (i32, i32),
    repeat_count: usize,
}

impl DecomposeSearch {
    /// Create a new instance of `DecomposeSearch`.
    pub fn new(inner_search: TargetHeuristicOperator, max_routes_range: (usize, usize), repeat_count: usize) -> Self {
        let max_routes_range = (max_routes_range.0 as i32, max_routes_range.1 as i32);

        Self { inner_search, max_routes_range, repeat_count }
    }
}

impl HeuristicOperator for DecomposeSearch {
    type Context = RefinementContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        decompose_insertion_ctx(refinement_ctx, insertion_ctx, self.max_routes_range)
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
        let decomposed = parallel_into_collect(decomposed, |mut decomposed| {
            (0..self.repeat_count).for_each(|_| {
                let insertion_ctx = decomposed.0.population.select().next().expect(GREEDY_ERROR);
                let insertion_ctx = self.inner_search.search(&decomposed.0, insertion_ctx);
                decomposed.0.population.add(insertion_ctx);
            });
            decomposed
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
    Box::new(GreedyPopulation::new(insertion_ctx.problem.objective.clone(), 1, Some(insertion_ctx)))
}

fn create_multiple_insertion_ctxs(
    insertion_ctx: &InsertionContext,
    max_routes_range: (i32, i32),
) -> Option<Vec<(InsertionContext, HashSet<usize>)>> {
    let mut route_groups_distances = group_routes_by_proximity(insertion_ctx)?;
    route_groups_distances.iter_mut().for_each(|route_distances| {
        let random = &insertion_ctx.environment.random;
        let shuffle_count = random.uniform_int(2, (route_distances.len() as i32 / 4).max(2)) as usize;
        route_distances.partial_shuffle(&mut random.get_rng(), shuffle_count);
    });

    // identify route groups and create insertion_ctxs from them
    let used_indices = RwLock::new(HashSet::new());
    let insertion_ctxs = route_groups_distances
        .iter()
        .enumerate()
        .filter(|(outer_idx, _)| !used_indices.read().unwrap().contains(outer_idx))
        .map(|(outer_idx, route_group_distance)| {
            let group_size =
                insertion_ctx.environment.random.uniform_int(max_routes_range.0, max_routes_range.1) as usize;
            let route_group = once(outer_idx)
                .chain(
                    route_group_distance
                        .iter()
                        .cloned()
                        .filter(|(inner_idx, _)| !used_indices.read().unwrap().contains(inner_idx))
                        .map(|(inner_idx, _)| inner_idx),
                )
                .take(group_size)
                .collect::<HashSet<_>>();

            route_group.iter().for_each(|idx| {
                used_indices.write().unwrap().insert(*idx);
            });

            create_partial_insertion_ctx(insertion_ctx, route_group)
        })
        .chain(create_empty_insertion_ctxs(insertion_ctx))
        .collect();

    Some(insertion_ctxs)
}

fn create_partial_insertion_ctx(
    insertion_ctx: &InsertionContext,
    route_indices: HashSet<usize>,
) -> (InsertionContext, HashSet<usize>) {
    let solution = &insertion_ctx.solution;

    let routes = route_indices.iter().map(|idx| solution.routes[*idx].deep_copy()).collect::<Vec<_>>();
    let actors = routes.iter().map(|route_ctx| route_ctx.route.actor.clone()).collect::<HashSet<_>>();
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
                    let jobs = solution.routes.iter().flat_map(|rc| rc.route.tour.jobs()).collect::<HashSet<_>>();
                    solution.locked.iter().filter(|job| !jobs.contains(job)).cloned().collect()
                } else {
                    let jobs = routes.iter().flat_map(|route_ctx| route_ctx.route.tour.jobs()).collect::<HashSet<_>>();
                    solution.locked.iter().filter(|job| jobs.contains(job)).cloned().collect()
                },
                routes,
                registry,
                state: Default::default(),
            },
            environment: insertion_ctx.environment.clone(),
        },
        route_indices,
    )
}

fn create_empty_insertion_ctxs(
    insertion_ctx: &InsertionContext,
) -> Box<dyn Iterator<Item = (InsertionContext, HashSet<usize>)>> {
    // TODO split into more insertion_ctxs if too many required jobs are present
    //      this might increase overall refinement speed

    let solution = &insertion_ctx.solution;

    if solution.required.is_empty()
        && solution.unassigned.is_empty()
        && solution.ignored.is_empty()
        && solution.locked.is_empty()
    {
        Box::new(empty())
    } else {
        Box::new(once((
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
                environment: insertion_ctx.environment.clone(),
            },
            HashSet::default(),
        )))
    }
}

fn decompose_insertion_ctx(
    refinement_ctx: &RefinementContext,
    insertion_ctx: &InsertionContext,
    max_routes_range: (i32, i32),
) -> Option<Vec<(RefinementContext, HashSet<usize>)>> {
    create_multiple_insertion_ctxs(insertion_ctx, max_routes_range)
        .map(|insertion_ctxs| {
            insertion_ctxs
                .into_iter()
                .map(|(insertion_ctx, indices)| {
                    (
                        RefinementContext {
                            problem: refinement_ctx.problem.clone(),
                            population: create_population(insertion_ctx),
                            state: Default::default(),
                            quota: refinement_ctx.quota.clone(),
                            environment: refinement_ctx.environment.clone(),
                            statistics: Default::default(),
                        },
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
    let (decomposed_insertion_ctx, _) = decomposed_ctx.population.ranked().next().expect(GREEDY_ERROR);

    let (partial_insertion_ctx, _) = create_partial_insertion_ctx(original_insertion_ctx, route_indices);
    let objective = partial_insertion_ctx.problem.objective.as_ref();

    let source_solution = if objective.total_order(decomposed_insertion_ctx, &partial_insertion_ctx) == Ordering::Less {
        &decomposed_insertion_ctx.solution
    } else {
        &partial_insertion_ctx.solution
    };

    let mut accumulated = accumulated;
    let dest_solution = &mut accumulated.solution;

    // NOTE theoretically, we can avoid deep copy here, but this would require extension in Population trait
    dest_solution.routes.extend(source_solution.routes.iter().map(|route_ctx| route_ctx.deep_copy()));
    dest_solution.ignored.extend(source_solution.ignored.iter().cloned());
    dest_solution.required.extend(source_solution.required.iter().cloned());
    dest_solution.locked.extend(source_solution.locked.iter().cloned());
    dest_solution.unassigned.extend(source_solution.unassigned.iter().map(|(k, v)| (k.clone(), *v)));

    source_solution.routes.iter().for_each(|route_ctx| {
        dest_solution.registry.use_route(route_ctx);
    });

    accumulated
}
