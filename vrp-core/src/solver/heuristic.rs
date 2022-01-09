use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::models::common::SingleDimLoad;
use crate::models::problem::ObjectiveCost;
use crate::solver::mutation::*;
use rosomaxa::heuristics::hyper::*;
use rosomaxa::heuristics::population::*;
use std::marker::PhantomData;

// TODO add type aliases for greedy, elitism, rosomaxa populations?

pub type TargetPopulation =
    Box<dyn HeuristicPopulation<Objective = ObjectiveCost, Individual = InsertionContext> + Send + Sync>;
pub type TargetHeuristic = Box<dyn HyperHeuristic<Context = RefinementContext, Solution = InsertionContext>>;

pub type GreedyPopulation = Greedy<ObjectiveCost, InsertionContext>;
pub type ElitismPopulation = Elitism<ObjectiveCost, InsertionContext>;
pub type RosomaxaPopulation = Rosomaxa<ObjectiveCost, InsertionContext>;

pub type MutationProbability<P> = HeuristicProbability<RefinementContext, ObjectiveCost, P, InsertionContext>;

/// Gets default population selection size.
pub fn get_default_selection_size(environment: &Environment) -> usize {
    environment.parallelism.available_cpus().min(8)
}

/// Gets default population algorithm.
pub fn get_default_population(objective: Arc<ObjectiveCost>, environment: Arc<Environment>) -> TargetPopulation {
    let selection_size = get_default_selection_size(environment.as_ref());
    if selection_size == 1 {
        Box::new(Greedy::new(objective, 1, None))
    } else {
        let config = RosomaxaConfig::new_with_defaults(selection_size);
        let population =
            Rosomaxa::new(objective, environment, config).expect("cannot create rosomaxa with default configuration");

        Box::new(population)
    }
}

/// Gets default heuristic.
pub fn get_default_heuristic(_problem: Arc<Problem>, _environment: Arc<Environment>) -> TargetHeuristic {
    todo!()
}

pub fn get_static_heuristic<Population>(
    _problem: Arc<Problem>,
    _environment: Arc<Environment>,
) -> StaticSelective<RefinementContext, ObjectiveCost, RosomaxaPopulation, InsertionContext>
where
    Population: HeuristicPopulation<Objective = ObjectiveCost, Individual = InsertionContext>,
{
    todo!()
}

/// Creates elitism population algorithm.
pub fn create_elitism_population(objective: Arc<ObjectiveCost>, environment: Arc<Environment>) -> TargetPopulation {
    let selection_size = get_default_selection_size(environment.as_ref());
    Box::new(Elitism::new(objective, environment.random.clone(), 4, selection_size))
}

impl RosomaxaWeighted for InsertionContext {
    fn weights(&self) -> Vec<f64> {
        todo!()
    }
}

impl DominanceOrdered for InsertionContext {
    fn get_order(&self) -> &DominanceOrder {
        todo!()
    }

    fn set_order(&mut self, order: DominanceOrder) {
        todo!()
    }
}

/// Creates default mutation (ruin and recreate) with default parameters.
pub fn create_default_mutation(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> Arc<dyn Mutation + Send + Sync> {
    let random = environment.random.clone();
    // initialize recreate
    let recreate = Arc::new(WeightedRecreate::new(vec![
        (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), 50),
        (Arc::new(RecreateWithRegret::new(2, 3, random.clone())), 20),
        (Arc::new(RecreateWithCheapest::new(random.clone())), 20),
        (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), 10),
        (Arc::new(RecreateWithSkipBest::new(3, 4, random.clone())), 5),
        (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), 5),
        // TODO use dimension size from problem
        (Arc::new(RecreateWithBlinks::<SingleDimLoad>::new_with_defaults(random.clone())), 5),
        (Arc::new(RecreateWithFarthest::new(random.clone())), 2),
        (Arc::new(RecreateWithSkipBest::new(4, 8, random.clone())), 2),
        (Arc::new(RecreateWithNearestNeighbor::new(random.clone())), 1),
        (Arc::new(RecreateWithSlice::new(random.clone())), 1),
        (
            Arc::new(RecreateWithSkipRandom::default_explorative_phased(
                Arc::new(RecreateWithCheapest::new(random.clone())),
                random.clone(),
            )),
            1,
        ),
    ]));

    // initialize ruin
    let close_route = Arc::new(CloseRouteRemoval::default());
    let random_route = Arc::new(RandomRouteRemoval::default());
    let random_job = Arc::new(RandomJobRemoval::new(RuinLimits::default()));
    let random_ruin = create_default_random_ruin();

    let ruin = Arc::new(WeightedRuin::new(vec![
        (vec![(Arc::new(AdjustedStringRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 100),
        (vec![(Arc::new(NeighbourRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
        (vec![(Arc::new(WorstJobRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
        (vec![(Arc::new(ClusterRemoval::new_with_defaults(problem, environment.clone())), 1.), (random_ruin, 0.1)], 5),
        (vec![(close_route, 1.), (random_job.clone(), 0.1)], 2),
        (vec![(random_route, 1.), (random_job, 0.1)], 1),
    ]));

    Arc::new(WeightedMutation::new(
        vec![Arc::new(RuinAndRecreate::new(ruin, recreate)), create_default_local_search(environment)],
        vec![100, 10],
    ))
}

/// Creates default random ruin method.
pub fn create_default_random_ruin() -> Arc<dyn Ruin + Send + Sync> {
    Arc::new(WeightedRuin::new(vec![
        (vec![(Arc::new(CloseRouteRemoval::default()), 1.)], 100),
        (vec![(Arc::new(RandomRouteRemoval::default()), 1.)], 10),
        (vec![(Arc::new(RandomJobRemoval::new(RuinLimits::default())), 1.)], 2),
    ]))
}

/// Creates a mutation probability which uses `is_hit` method from passed random object.
pub fn create_scalar_mutation_probability<P>(
    scalar_probability: f64,
    random: Arc<dyn Random + Send + Sync>,
) -> MutationProbability<P> {
    (Box::new(move |_, _| random.is_hit(scalar_probability)), PhantomData::default(), PhantomData::default())
}

/// Creates a mutation probability which uses context state.
pub fn create_context_mutation_probability<P>(
    jobs_threshold: usize,
    routes_threshold: usize,
    phases: Vec<(SelectionPhase, f64)>,
    random: Arc<dyn Random + Send + Sync>,
) -> MutationProbability<P> {
    let phases = phases.into_iter().collect::<HashMap<_, _>>();
    (
        Box::new(move |refinement_ctx, insertion_ctx| {
            let below_thresholds = insertion_ctx.problem.jobs.size() < jobs_threshold
                || insertion_ctx.solution.routes.len() < routes_threshold;

            if below_thresholds {
                return false;
            }

            let phase_probability = phases.get(&refinement_ctx.population.selection_phase()).cloned().unwrap_or(0.);
            random.is_hit(phase_probability)
        }),
        PhantomData::default(),
        PhantomData::default(),
    )
}

fn create_default_local_search(environment: Arc<Environment>) -> Arc<dyn Mutation + Send + Sync> {
    let random = environment.random.clone();

    Arc::new(LocalSearch::new(Arc::new(CompositeLocalOperator::new(
        vec![
            (Arc::new(ExchangeSwapStar::new(random)), 200),
            (Arc::new(ExchangeInterRouteBest::default()), 100),
            (Arc::new(ExchangeSequence::default()), 100),
            (Arc::new(ExchangeInterRouteRandom::default()), 30),
            (Arc::new(ExchangeIntraRouteRandom::default()), 30),
            (Arc::new(RescheduleDeparture::default()), 20),
        ],
        1,
        2,
    ))))
}
