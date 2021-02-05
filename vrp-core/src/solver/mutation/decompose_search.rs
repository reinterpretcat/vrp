#[cfg(test)]
#[path = "../../../tests/unit/solver/mutation/decompose_search_test.rs"]
mod decompose_search_test;

use super::super::rand::prelude::SliceRandom;
use crate::algorithms::nsga2::Objective;
use crate::construction::heuristics::{get_medoid, InsertionContext, SolutionContext};
use crate::solver::mutation::Mutation;
use crate::solver::population::{Greedy, Individual, Population};
use crate::solver::RefinementContext;
use crate::utils::{compare_floats, parallel_into_collect};
use hashbrown::HashSet;
use std::cmp::Ordering;
use std::iter::{empty, once};
use std::sync::{Arc, RwLock};

/// A mutation which decomposes original solution into multiple partial solutions,
/// preforms search independently, and then merges partial solution back into one solution.
pub struct DecomposeSearch {
    inner_mutation: Arc<dyn Mutation + Send + Sync>,
    max_routes_range: (i32, i32),
    repeat_count: usize,
}

impl DecomposeSearch {
    /// Create a new instance of `DecomposeSearch`.
    pub fn new(
        inner_mutation: Arc<dyn Mutation + Send + Sync>,
        max_routes_range: (usize, usize),
        repeat_count: usize,
    ) -> Self {
        let max_routes_range = (max_routes_range.0 as i32, max_routes_range.1 as i32);

        Self { inner_mutation, max_routes_range, repeat_count }
    }
}

impl Mutation for DecomposeSearch {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        decompose_individual(&refinement_ctx, insertion_ctx, self.max_routes_range)
            .map(|contexts| self.refine_decomposed(refinement_ctx, insertion_ctx, contexts))
            .unwrap_or_else(|| self.inner_mutation.mutate(refinement_ctx, insertion_ctx))
    }
}

const GREEDY_ERROR: &str = "greedy population has no individuals";

impl DecomposeSearch {
    fn refine_decomposed(
        &self,
        refinement_ctx: &RefinementContext,
        original_individual: &Individual,
        decomposed: Vec<(RefinementContext, HashSet<usize>)>,
    ) -> Individual {
        // NOTE: validate decomposition
        decomposed.iter().enumerate().for_each(|(outer_ix, (_, outer))| {
            decomposed.iter().enumerate().filter(|(inner_idx, _)| outer_ix != *inner_idx).for_each(
                |(_, (_, inner))| {
                    assert!(outer.intersection(&inner).next().is_none());
                },
            );
        });

        // do actual refinement independently for each decomposed context
        let decomposed = parallel_into_collect(decomposed, |mut decomposed| {
            (0..self.repeat_count).for_each(|_| {
                let insertion_ctx = decomposed.0.population.select().next().expect(GREEDY_ERROR);
                let insertion_ctx = self.inner_mutation.mutate(&decomposed.0, insertion_ctx);
                decomposed.0.population.add(insertion_ctx);
            });
            decomposed
        });

        // merge evolution results into one individual
        let mut individual = decomposed.into_iter().fold(
            Individual::new_empty(refinement_ctx.problem.clone(), refinement_ctx.environment.clone()),
            |individual, decomposed| merge_best(decomposed, original_individual, individual),
        );

        refinement_ctx.problem.constraint.accept_solution_state(&mut individual.solution);

        individual
    }
}

fn create_population(individual: Individual) -> Box<dyn Population + Send + Sync> {
    Box::new(Greedy::new(individual.problem.clone(), 1, Some(individual)))
}

fn create_multiple_individuals(
    individual: &Individual,
    max_routes_range: (i32, i32),
) -> Option<Vec<(Individual, HashSet<usize>)>> {
    let solution = &individual.solution;
    let profile = solution.routes.first().map(|route_ctx| route_ctx.route.actor.vehicle.profile)?;
    let transport = individual.problem.transport.as_ref();

    let indexed_medoids = solution
        .routes
        .iter()
        .enumerate()
        .map(|(idx, route_ctx)| (idx, get_medoid(route_ctx, transport)))
        .collect::<Vec<_>>();

    // estimate distances between all routes using their medoids
    let route_groups_distances = indexed_medoids
        .iter()
        .map(|(outer_idx, outer_medoid)| {
            let mut route_distances = indexed_medoids
                .iter()
                .filter(move |(inner_idx, _)| *outer_idx != *inner_idx)
                .map(move |(inner_idx, inner_medoid)| {
                    let distance = match (outer_medoid, inner_medoid) {
                        (Some(outer_medoid), Some(inner_medoid)) => {
                            let distance =
                                transport.distance(profile, *outer_medoid, *inner_medoid, Default::default());
                            if distance < 0. {
                                None
                            } else {
                                Some(distance)
                            }
                        }
                        _ => None,
                    };
                    (inner_idx, distance)
                })
                .collect::<Vec<_>>();

            route_distances.sort_by(|(_, a_distance), (_, b_distance)| match (a_distance, b_distance) {
                (Some(a_distance), Some(b_distance)) => compare_floats(*a_distance, *b_distance),
                (Some(_), None) => Ordering::Less,
                _ => Ordering::Greater,
            });

            let random = &individual.environment.random;
            let shuffle_count = random.uniform_int(2, (route_distances.len() as i32 / 4).max(2)) as usize;
            route_distances.partial_shuffle(&mut random.get_rng(), shuffle_count);

            route_distances
        })
        .collect::<Vec<_>>();

    // identify route groups and create individuals from them
    let used_indices = RwLock::new(HashSet::new());
    let individuals = route_groups_distances
        .iter()
        .enumerate()
        .filter(|(outer_idx, _)| !used_indices.read().unwrap().contains(outer_idx))
        .map(|(outer_idx, route_group_distance)| {
            let group_size = individual.environment.random.uniform_int(max_routes_range.0, max_routes_range.1) as usize;
            let route_group = once(outer_idx)
                .chain(
                    route_group_distance
                        .iter()
                        .cloned()
                        .filter(|(inner_idx, _)| !used_indices.read().unwrap().contains(*inner_idx))
                        .map(|(inner_idx, _)| *inner_idx),
                )
                .take(group_size)
                .collect::<HashSet<_>>();

            route_group.iter().for_each(|idx| {
                used_indices.write().unwrap().insert(*idx);
            });

            create_partial_individual(individual, route_group)
        })
        .chain(create_empty_individuals(individual))
        .collect();

    Some(individuals)
}

fn create_partial_individual(individual: &Individual, route_indices: HashSet<usize>) -> (Individual, HashSet<usize>) {
    let solution = &individual.solution;

    let routes = route_indices.iter().map(|idx| solution.routes[*idx].deep_copy()).collect::<Vec<_>>();
    let actors = routes.iter().map(|route_ctx| route_ctx.route.actor.clone()).collect::<HashSet<_>>();
    let registry = solution.registry.deep_slice(|actor| actors.contains(actor));

    (
        Individual {
            problem: individual.problem.clone(),
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
            environment: individual.environment.clone(),
        },
        route_indices,
    )
}

fn create_empty_individuals(individual: &Individual) -> Box<dyn Iterator<Item = (Individual, HashSet<usize>)>> {
    // TODO split into more individuals if too many required jobs are present
    //      this might increase overall refinement speed

    let solution = &individual.solution;

    if solution.required.is_empty()
        && solution.unassigned.is_empty()
        && solution.ignored.is_empty()
        && solution.locked.is_empty()
    {
        Box::new(empty())
    } else {
        Box::new(once((
            Individual {
                problem: individual.problem.clone(),
                solution: SolutionContext {
                    required: solution.required.clone(),
                    ignored: solution.ignored.clone(),
                    unassigned: solution.unassigned.clone(),
                    locked: solution.locked.clone(),
                    routes: Default::default(),
                    registry: solution.registry.deep_copy(),
                    state: Default::default(),
                },
                environment: individual.environment.clone(),
            },
            HashSet::default(),
        )))
    }
}

fn decompose_individual(
    refinement_ctx: &RefinementContext,
    individual: &Individual,
    max_routes_range: (i32, i32),
) -> Option<Vec<(RefinementContext, HashSet<usize>)>> {
    create_multiple_individuals(individual, max_routes_range)
        .map(|individuals| {
            individuals
                .into_iter()
                .map(|(individual, indices)| {
                    (
                        RefinementContext {
                            problem: refinement_ctx.problem.clone(),
                            population: create_population(individual),
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
    original_individual: &Individual,
    accumulated: Individual,
) -> Individual {
    let (decomposed_ctx, route_indices) = decomposed;
    let (decomposed_individual, _) = decomposed_ctx.population.ranked().next().expect(GREEDY_ERROR);

    let (partial_individual, _) = create_partial_individual(original_individual, route_indices);
    let objective = partial_individual.problem.objective.as_ref();

    let source_solution = if objective.total_order(decomposed_individual, &partial_individual) == Ordering::Less {
        &decomposed_individual.solution
    } else {
        &partial_individual.solution
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
