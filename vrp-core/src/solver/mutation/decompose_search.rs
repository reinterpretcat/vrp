use crate::construction::heuristics::{get_medoid, InsertionContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::mutation::Mutation;
use crate::solver::population::{Greedy, Individual, Population};
use crate::solver::RefinementContext;
use crate::utils::{parallel_into_collect, Random};
use hashbrown::HashMap;
use std::sync::Arc;

/// A mutation which decomposes original solution into multiple partial solutions,
/// preforms search independently, and then merges partial solution back into one solution.
pub struct DecomposeSearch {
    inner_mutation: Arc<dyn Mutation + Send + Sync>,
    // TODO different repeat count depending on generation in refinement ctx
    repeat_count: usize,
}

impl Mutation for DecomposeSearch {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        refinement_ctx
            .population
            .ranked()
            .next()
            .and_then(|(individual, _)| {
                decompose_individual(&refinement_ctx, individual).map(|result| (individual.random.clone(), result))
            })
            .map(|(random, decomposed_contexts)| self.refine_decomposed(refinement_ctx, random, decomposed_contexts))
            .unwrap_or_else(|| self.inner_mutation.mutate_one(refinement_ctx, insertion_ctx))
    }

    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<&InsertionContext>,
    ) -> Vec<InsertionContext> {
        individuals.into_iter().map(|individual| self.mutate_one(refinement_ctx, individual)).collect()
    }
}

const GREEDY_ERROR: &str = "greedy population has no individuals";

impl DecomposeSearch {
    fn refine_decomposed(
        &self,
        refinement_ctx: &RefinementContext,
        random: Arc<dyn Random + Send + Sync>,
        decomposed_contexts: Vec<RefinementContext>,
    ) -> Individual {
        // do actual refinement independently for each decomposed context
        let decomposed_populations = parallel_into_collect(decomposed_contexts, |mut decomposed_ctx| {
            (0..self.repeat_count).for_each(|_| {
                let insertion_ctx = decomposed_ctx.population.select().next().expect(GREEDY_ERROR);
                let insertion_ctx = self.inner_mutation.mutate_one(&decomposed_ctx, insertion_ctx);
                decomposed_ctx.population.add(insertion_ctx);
            });
            decomposed_ctx.population
        });

        // merge evolution results into one individual
        let mut individual = decomposed_populations.into_iter().fold(
            Individual::new(refinement_ctx.problem.clone(), random),
            |mut individual, decomposed_population| {
                let (decomposed_individual, _) = decomposed_population.ranked().next().expect(GREEDY_ERROR);

                let acc_solution = &mut individual.solution;
                let dec_solution = &decomposed_individual.solution;

                // NOTE theoretically, we can avoid deep copy here, but this would require extension in Population trait
                acc_solution.routes.extend(dec_solution.routes.iter().map(|route_ctx| route_ctx.deep_copy()));
                acc_solution.ignored.extend(dec_solution.ignored.iter().cloned());
                acc_solution.required.extend(dec_solution.required.iter().cloned());
                acc_solution.locked.extend(dec_solution.locked.iter().cloned());
                acc_solution.unassigned.extend(dec_solution.unassigned.iter().map(|(k, v)| (k.clone(), v.clone())));

                individual
            },
        );

        refinement_ctx.problem.constraint.accept_solution_state(&mut individual.solution);

        individual
    }
}

fn create_population(individual: Individual) -> Box<dyn Population + Send + Sync> {
    Box::new(Greedy::new(individual.problem.clone(), Some(individual)))
}

fn create_multiple_individuals(individual: &Individual) -> Option<Vec<Individual>> {
    // Individual { problem: individual.problem.clone(), solution, random: individual.random.clone() }

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
    let route_groups = indexed_medoids
        .iter()
        .map(|(outer_idx, outer_medoid)| {
            indexed_medoids
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
                    (outer_idx, inner_idx, distance)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    unimplemented!()
}

fn decompose_individual(refinement_ctx: &RefinementContext, individual: &Individual) -> Option<Vec<RefinementContext>> {
    create_multiple_individuals(individual)
        .map(|individuals| {
            individuals
                .into_iter()
                .map(|individual| RefinementContext {
                    problem: refinement_ctx.problem.clone(),
                    population: create_population(individual),
                    state: Default::default(),
                    quota: refinement_ctx.quota.clone(),
                    statistics: Default::default(),
                })
                .collect::<Vec<_>>()
        })
        .and_then(|contexts| if contexts.len() > 1 { Some(contexts) } else { None })
}
