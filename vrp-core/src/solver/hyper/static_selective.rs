use super::*;
use crate::algorithms::nsga2::Objective;
use crate::construction::heuristics::InsertionContext;
use crate::models::common::SingleDimLoad;
use crate::models::Problem;
use crate::solver::mutation::*;
use crate::solver::population::{Individual, SelectionPhase};
use crate::solver::RefinementContext;
use crate::utils::{parallel_into_collect, unwrap_from_result, Environment};
use std::cmp::Ordering;
use std::sync::Arc;

/// A type which specifies probability behavior for mutation selection.
pub type MutationProbability = Box<dyn Fn(&RefinementContext, &InsertionContext) -> bool + Send + Sync>;

/// A type which specifies a group of multiple mutation strategies with their probability.
pub type MutationGroup = Vec<(Arc<dyn Mutation + Send + Sync>, MutationProbability)>;

/// A simple hyper-heuristic which selects mutation operator from the list with fixed (static) probabilities.
pub struct StaticSelective {
    mutation_group: MutationGroup,
}

impl HyperHeuristic for StaticSelective {
    fn search(&mut self, refinement_ctx: &RefinementContext, individuals: Vec<&Individual>) -> Vec<Individual> {
        parallel_into_collect(individuals.iter().enumerate().collect(), |(idx, insertion_ctx)| {
            refinement_ctx
                .environment
                .parallelism
                .thread_pool_execute(idx, || self.mutate(refinement_ctx, insertion_ctx))
        })
    }
}

impl StaticSelective {
    /// Creates an instance of `StaticSelective` from mutation groups.
    pub fn new(mutation_group: MutationGroup) -> Self {
        Self { mutation_group }
    }

    /// Creates an instance of `StaticSelective` with default parameters.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        let default_mutation = Self::create_default_mutation(problem, environment.clone());
        let local_search = Arc::new(LocalSearch::new(Arc::new(CompositeLocalOperator::new(
            vec![
                (Arc::new(ExchangeInterRouteBest::default()), 100),
                (Arc::new(ExchangeInterRouteRandom::default()), 30),
                (Arc::new(ExchangeIntraRouteRandom::default()), 30),
                (Arc::new(RescheduleDeparture::default()), 20),
            ],
            1,
            2,
        ))));

        Self::new(vec![
            (
                Arc::new(DecomposeSearch::new(default_mutation.clone(), (2, 8), 4)),
                create_context_mutation_probability(
                    300,
                    10,
                    vec![(SelectionPhase::Exploration, 0.05), (SelectionPhase::Exploitation, 0.05)],
                    environment.random.clone(),
                ),
            ),
            (local_search.clone(), create_scalar_mutation_probability(0.05, environment.random.clone())),
            (default_mutation.clone(), create_scalar_mutation_probability(1., environment.random.clone())),
            (local_search, create_scalar_mutation_probability(0.05, environment.random.clone())),
            (
                Arc::new(InfeasibleSearch::new(default_mutation, 4, (0.05, 0.2), (0.05, 0.33))),
                create_scalar_mutation_probability(0.01, environment.random.clone()),
            ),
        ])
    }

    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        unwrap_from_result(
            self.mutation_group
                .iter()
                .filter(|(_, probability)| probability(refinement_ctx, insertion_ctx))
                // NOTE not more than two mutations in a row
                .take(2)
                .try_fold(insertion_ctx.deep_copy(), |ctx, (mutation, _)| {
                    let new_insertion_ctx = mutation.mutate(refinement_ctx, &ctx);

                    if refinement_ctx.problem.objective.total_order(insertion_ctx, &new_insertion_ctx)
                        == Ordering::Greater
                    {
                        // NOTE exit immediately as we don't want to lose improvement from original individual
                        Err(new_insertion_ctx)
                    } else {
                        Ok(new_insertion_ctx)
                    }
                }),
        )
    }

    /// Creates default mutation (ruin and recreate) with default parameters.
    pub fn create_default_mutation(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> Arc<dyn Mutation + Send + Sync> {
        // initialize recreate
        let recreate = Arc::new(WeightedRecreate::new(vec![
            (Arc::new(RecreateWithSkipBest::new(1, 2)), 50),
            (Arc::new(RecreateWithRegret::new(2, 3)), 20),
            (Arc::new(RecreateWithCheapest::default()), 20),
            (Arc::new(RecreateWithPerturbation::new_with_defaults(environment.random.clone())), 10),
            (Arc::new(RecreateWithSkipBest::new(3, 4)), 5),
            (Arc::new(RecreateWithGaps::default()), 5),
            // TODO use dimension size from problem
            (Arc::new(RecreateWithBlinks::<SingleDimLoad>::new_with_defaults(environment.random.clone())), 5),
            (Arc::new(RecreateWithFarthest::default()), 2),
            (Arc::new(RecreateWithSkipBest::new(4, 8)), 2),
            (Arc::new(RecreateWithNearestNeighbor::default()), 1),
            (
                Arc::new(RecreateWithSkipRandom::default_explorative_phased(Arc::new(RecreateWithCheapest::default()))),
                1,
            ),
        ]));

        // initialize ruin
        let close_route = Arc::new(CloseRouteRemoval::default());
        let random_route = Arc::new(RandomRouteRemoval::default());
        let random_job = Arc::new(RandomJobRemoval::new(RuinLimits::default()));
        let random_ruin = Self::create_default_random_ruin();

        let ruin = Arc::new(WeightedRuin::new(vec![
            (vec![(Arc::new(AdjustedStringRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 100),
            (vec![(Arc::new(NeighbourRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
            (vec![(Arc::new(WorstJobRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
            (vec![(Arc::new(ClusterRemoval::new_with_defaults(problem, environment)), 1.), (random_ruin, 0.1)], 5),
            (vec![(close_route, 1.), (random_job.clone(), 0.1)], 2),
            (vec![(random_route, 1.), (random_job, 0.1)], 1),
        ]));

        Arc::new(RuinAndRecreate::new(ruin, recreate))
    }

    /// Creates default random ruin method.
    pub fn create_default_random_ruin() -> Arc<dyn Ruin + Send + Sync> {
        Arc::new(WeightedRuin::new(vec![
            (vec![(Arc::new(CloseRouteRemoval::default()), 1.)], 100),
            (vec![(Arc::new(RandomRouteRemoval::default()), 1.)], 10),
            (vec![(Arc::new(RandomJobRemoval::new(RuinLimits::default())), 1.)], 2),
        ]))
    }
}

/// Creates a mutation probability which uses `is_hit` method from passed random object.
pub fn create_scalar_mutation_probability(
    scalar_probability: f64,
    random: Arc<dyn Random + Send + Sync>,
) -> MutationProbability {
    Box::new(move |_, _| random.is_hit(scalar_probability))
}

/// Creates a mutation probability which uses context state.
pub fn create_context_mutation_probability(
    jobs_threshold: usize,
    routes_threshold: usize,
    phases: Vec<(SelectionPhase, f64)>,
    random: Arc<dyn Random + Send + Sync>,
) -> MutationProbability {
    let phases = phases.into_iter().collect::<HashMap<_, _>>();
    Box::new(move |refinement_ctx, insertion_ctx| {
        let below_thresholds = insertion_ctx.problem.jobs.size() < jobs_threshold
            || insertion_ctx.solution.routes.len() < routes_threshold;

        if below_thresholds {
            return false;
        }

        let phase_probability = phases.get(&refinement_ctx.population.selection_phase()).cloned().unwrap_or(0.);

        random.is_hit(phase_probability)
    })
}
