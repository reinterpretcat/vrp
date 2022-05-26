use super::*;
use crate::construction::heuristics::*;
use crate::models::common::{has_multi_dim_demand, MultiDimLoad, SingleDimLoad};
use crate::models::problem::ProblemObjective;
use crate::rosomaxa::get_default_selection_size;
use crate::solver::heuristic::dynamic::create_inner_heuristic_operator;
use crate::solver::search::*;
use rosomaxa::algorithms::gsom::Input;
use rosomaxa::hyper::*;
use rosomaxa::population::*;
use rosomaxa::termination::*;
use std::marker::PhantomData;

/// A type alias for domain specific population.
pub type TargetPopulation =
    Box<dyn HeuristicPopulation<Objective = ProblemObjective, Individual = InsertionContext> + Send + Sync>;
/// A type alias for domain specific heuristic.
pub type TargetHeuristic =
    Box<dyn HyperHeuristic<Context = RefinementContext, Objective = ProblemObjective, Solution = InsertionContext>>;
/// A type for domain specific heuristic operator.
pub type TargetSearchOperator = Arc<
    dyn HeuristicSearchOperator<Context = RefinementContext, Objective = ProblemObjective, Solution = InsertionContext>
        + Send
        + Sync,
>;

/// A type for greedy population.
pub type GreedyPopulation = Greedy<ProblemObjective, InsertionContext>;
/// A type for elitism population.
pub type ElitismPopulation = Elitism<ProblemObjective, InsertionContext>;
/// A type for rosomaxa population.
pub type RosomaxaPopulation = Rosomaxa<ProblemObjective, InsertionContext>;

/// A type alias for domain specific termination type.
pub type DynTermination = dyn Termination<Context = RefinementContext, Objective = ProblemObjective> + Send + Sync;
/// A type for composite termination.
pub type TargetCompositeTermination = CompositeTermination<RefinementContext, ProblemObjective, InsertionContext>;
/// A type for max time termination.
pub type MaxTimeTermination = MaxTime<RefinementContext, ProblemObjective, InsertionContext>;
/// A type for max generation termination.
pub type MaxGenerationTermination = MaxGeneration<RefinementContext, ProblemObjective, InsertionContext>;
/// A type for min variation termination.
pub type MinVariationTermination = MinVariation<RefinementContext, ProblemObjective, InsertionContext, String>;

/// A heuristic probability type alias.
pub type TargetHeuristicProbability = HeuristicProbability<RefinementContext, ProblemObjective, InsertionContext>;
/// A heuristic group type alias.
pub type TargetHeuristicGroup = HeuristicSearchGroup<RefinementContext, ProblemObjective, InsertionContext>;

/// A type alias for evolution config builder.
pub type ProblemConfigBuilder = EvolutionConfigBuilder<RefinementContext, ProblemObjective, InsertionContext, String>;

/// Creates config builder with default settings.
pub fn create_default_config_builder(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    telemetry_mode: TelemetryMode,
) -> ProblemConfigBuilder {
    let selection_size = get_default_selection_size(environment.as_ref());
    let population = get_default_population(problem.objective.clone(), environment.clone(), selection_size);

    ProblemConfigBuilder::default()
        .with_heuristic(get_default_heuristic(problem.clone(), environment.clone()))
        .with_context(RefinementContext::new(problem.clone(), population, telemetry_mode, environment.clone()))
        .with_initial(4, 0.05, create_default_init_operators(problem, environment))
        .with_processing(create_default_processing())
}

/// Creates default telemetry mode.B
pub fn get_default_telemetry_mode(logger: InfoLogger) -> TelemetryMode {
    TelemetryMode::OnlyLogging { logger, log_best: 100, log_population: 1000, dump_population: false }
}

/// Gets default heuristic.
pub fn get_default_heuristic(problem: Arc<Problem>, environment: Arc<Environment>) -> TargetHeuristic {
    get_dynamic_heuristic(problem, environment)
}

/// Gets static heuristic using default settings.
pub fn get_static_heuristic(problem: Arc<Problem>, environment: Arc<Environment>) -> TargetHeuristic {
    let default_operator = statik::create_default_heuristic_operator(problem.clone(), environment.clone());
    let local_search = statik::create_default_local_search(environment.clone());

    let heuristic_group: TargetHeuristicGroup = vec![
        (
            Arc::new(DecomposeSearch::new(default_operator.clone(), (2, 4), 4)),
            create_context_operator_probability(
                300,
                10,
                vec![(SelectionPhase::Exploration, 0.05), (SelectionPhase::Exploitation, 0.05)],
                environment.random.clone(),
            ),
        ),
        (local_search.clone(), create_scalar_operator_probability(0.05, environment.random.clone())),
        (default_operator.clone(), create_scalar_operator_probability(1., environment.random.clone())),
        (local_search, create_scalar_operator_probability(0.05, environment.random.clone())),
    ];

    get_static_heuristic_from_heuristic_group(problem, environment, heuristic_group)
}

/// Gets static heuristic using heuristic group.
pub fn get_static_heuristic_from_heuristic_group(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    heuristic_group: TargetHeuristicGroup,
) -> TargetHeuristic {
    Box::new(StaticSelective::<RefinementContext, ProblemObjective, InsertionContext>::new(
        heuristic_group,
        create_diversify_operators(problem, environment),
    ))
}

/// Gets dynamic heuristic using default settings.
pub fn get_dynamic_heuristic(problem: Arc<Problem>, environment: Arc<Environment>) -> TargetHeuristic {
    let search_operators = dynamic::get_operators(problem.clone(), environment.clone());
    let diversify_operators = create_diversify_operators(problem, environment.clone());

    Box::new(DynamicSelective::<RefinementContext, ProblemObjective, InsertionContext>::new(
        search_operators,
        diversify_operators,
        environment.as_ref(),
    ))
}

/// Creates elitism population algorithm.
pub fn create_elitism_population(objective: Arc<ProblemObjective>, environment: Arc<Environment>) -> TargetPopulation {
    let selection_size = get_default_selection_size(environment.as_ref());
    Box::new(Elitism::new(objective, environment.random.clone(), 4, selection_size))
}

impl RosomaxaWeighted for InsertionContext {
    fn init_weights(&mut self) {
        let weights = vec![
            get_max_load_variance(self),
            get_duration_mean(self),
            get_distance_mean(self),
            get_waiting_mean(self),
            get_longest_distance_between_customers_mean(self),
            get_average_distance_between_depot_customer_mean(self),
            get_distance_gravity_mean(self),
            get_customers_deviation(self),
            get_longest_distance_between_depot_customer_mean(self),
            self.solution.routes.len() as f64,
            self.solution.unassigned.len() as f64,
        ];
        self.solution.state.insert(SOLUTION_WEIGHTS_KEY, Arc::new(weights));
    }
}

impl Input for InsertionContext {
    fn weights(&self) -> &[f64] {
        self.solution.state.get(&SOLUTION_WEIGHTS_KEY).and_then(|s| s.downcast_ref::<Vec<f64>>()).unwrap().as_slice()
    }
}

impl DominanceOrdered for InsertionContext {
    fn get_order(&self) -> &DominanceOrder {
        self.solution.state.get(&SOLUTION_ORDER_KEY).and_then(|s| s.downcast_ref::<DominanceOrder>()).unwrap()
    }

    fn set_order(&mut self, order: DominanceOrder) {
        self.solution.state.insert(SOLUTION_ORDER_KEY, Arc::new(order));
    }
}

/// Creates a heuristic operator probability which uses `is_hit` method from passed random object.
pub fn create_scalar_operator_probability(
    scalar_probability: f64,
    random: Arc<dyn Random + Send + Sync>,
) -> TargetHeuristicProbability {
    (Box::new(move |_, _| random.is_hit(scalar_probability)), PhantomData::default())
}

/// Creates a heuristic operator probability which uses context state.
pub fn create_context_operator_probability(
    jobs_threshold: usize,
    routes_threshold: usize,
    phases: Vec<(SelectionPhase, f64)>,
    random: Arc<dyn Random + Send + Sync>,
) -> TargetHeuristicProbability {
    let phases = phases.into_iter().collect::<HashMap<_, _>>();
    (
        Box::new(move |refinement_ctx, insertion_ctx| {
            let below_thresholds = insertion_ctx.problem.jobs.size() < jobs_threshold
                || insertion_ctx.solution.routes.len() < routes_threshold;

            if below_thresholds {
                return false;
            }

            let phase_probability = phases.get(&refinement_ctx.population().selection_phase()).cloned().unwrap_or(0.);
            random.is_hit(phase_probability)
        }),
        PhantomData::default(),
    )
}

pub use self::builder::create_default_init_operators;
pub use self::builder::create_default_processing;
pub use self::statik::create_default_heuristic_operator;
pub use self::statik::create_default_random_ruin;

mod builder {
    use super::*;
    use crate::models::common::SingleDimLoad;
    use crate::rosomaxa::evolution::InitialOperators;
    use crate::solver::processing::*;
    use crate::solver::RecreateInitialOperator;

    /// Creates default init operators.
    pub fn create_default_init_operators(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> InitialOperators<RefinementContext, ProblemObjective, InsertionContext> {
        let random = environment.random.clone();
        let wrap = |recreate: Arc<dyn Recreate + Send + Sync>| Box::new(RecreateInitialOperator::new(recreate));

        vec![
            (wrap(Arc::new(RecreateWithCheapest::new(random.clone()))), 1),
            (wrap(Arc::new(RecreateWithFarthest::new(random.clone()))), 1),
            (wrap(Arc::new(RecreateWithRegret::new(2, 3, random.clone()))), 1),
            (wrap(Arc::new(RecreateWithGaps::new(1, (problem.jobs.size() / 10).max(1), random.clone()))), 1),
            (wrap(Arc::new(RecreateWithSkipBest::new(1, 2, random.clone()))), 1),
            (wrap(Arc::new(RecreateWithBlinks::<SingleDimLoad>::new_with_defaults(random.clone()))), 1),
            (wrap(Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone()))), 1),
            (wrap(Arc::new(RecreateWithNearestNeighbor::new(random.clone()))), 1),
        ]
    }

    /// Create default processing.
    pub fn create_default_processing() -> ProcessingConfig<RefinementContext, ProblemObjective, InsertionContext> {
        ProcessingConfig {
            context: vec![Box::new(VicinityClustering::default())],
            solution: vec![
                Box::new(AdvanceDeparture::default()),
                Box::new(UnassignmentReason::default()),
                Box::new(VicinityClustering::default()),
            ],
        }
    }
}

fn create_recreate_with_blinks(
    problem: &Problem,
    random: Arc<dyn Random + Send + Sync>,
) -> Arc<dyn Recreate + Send + Sync> {
    if has_multi_dim_demand(problem) {
        Arc::new(RecreateWithBlinks::<MultiDimLoad>::new_with_defaults(random))
    } else {
        Arc::new(RecreateWithBlinks::<SingleDimLoad>::new_with_defaults(random))
    }
}

fn create_diversify_operators(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> HeuristicDiversifyOperators<RefinementContext, ProblemObjective, InsertionContext> {
    let random = environment.random.clone();
    let inner_search = create_inner_heuristic_operator(problem, environment.clone());

    let recreates: Vec<(Arc<dyn Recreate + Send + Sync>, usize)> = vec![
        (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), 1),
        (Arc::new(RecreateWithRegret::new(1, 3, random.clone())), 1),
        (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), 1),
        (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), 1),
        (Arc::new(RecreateWithNearestNeighbor::new(random.clone())), 1),
        (Arc::new(RecreateWithSlice::new(random.clone())), 1),
    ];

    let redistribute_search = Arc::new(RedistributeSearch::new(Arc::new(WeightedRecreate::new(recreates))));
    let infeasible_search = Arc::new(InfeasibleSearch::new(inner_search, 2, (0.05, 0.2), (0.05, 0.33)));
    let local_search = Arc::new(LocalSearch::new(Arc::new(CompositeLocalOperator::new(
        vec![(Arc::new(ExchangeSequence::new(8, 0.25, 0.1)), 1)],
        2,
        4,
    ))));

    vec![Arc::new(WeightedHeuristicOperator::new(
        vec![redistribute_search, local_search, infeasible_search],
        vec![10, 2, 1],
    ))]
}

mod statik {
    use super::*;

    /// Creates default heuristic operator (ruin and recreate) with default parameters.
    pub fn create_default_heuristic_operator(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> TargetSearchOperator {
        let random = environment.random.clone();

        // initialize recreate
        let recreate = Arc::new(WeightedRecreate::new(vec![
            (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), 50),
            (Arc::new(RecreateWithRegret::new(2, 3, random.clone())), 20),
            (Arc::new(RecreateWithCheapest::new(random.clone())), 20),
            (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), 10),
            (Arc::new(RecreateWithSkipBest::new(3, 4, random.clone())), 5),
            (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), 5),
            (create_recreate_with_blinks(problem.as_ref(), random.clone()), 5),
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
        let worst_route = Arc::new(WorstRouteRemoval::default());
        let random_route = Arc::new(RandomRouteRemoval::default());
        let random_job = Arc::new(RandomJobRemoval::new(RuinLimits::default()));
        let random_ruin = create_default_random_ruin();

        let ruin = Arc::new(WeightedRuin::new(vec![
            (vec![(Arc::new(AdjustedStringRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 100),
            (vec![(Arc::new(NeighbourRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
            (vec![(Arc::new(WorstJobRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
            (
                vec![
                    (Arc::new(ClusterRemoval::new_with_defaults(problem, environment.clone())), 1.),
                    (random_ruin, 0.1),
                ],
                5,
            ),
            (vec![(close_route, 1.), (random_job.clone(), 0.1)], 2),
            (vec![(worst_route, 1.), (random_job.clone(), 0.1)], 1),
            (vec![(random_route, 1.), (random_job, 0.1)], 1),
        ]));

        Arc::new(WeightedHeuristicOperator::new(
            vec![Arc::new(RuinAndRecreate::new(ruin, recreate)), create_default_local_search(environment)],
            vec![100, 10],
        ))
    }

    /// Creates default random ruin method.
    pub fn create_default_random_ruin() -> Arc<dyn Ruin + Send + Sync> {
        Arc::new(WeightedRuin::new(vec![
            (vec![(Arc::new(CloseRouteRemoval::default()), 1.)], 100),
            (vec![(Arc::new(RandomRouteRemoval::default()), 1.)], 10),
            (vec![(Arc::new(WorstRouteRemoval::default()), 1.)], 5),
            (vec![(Arc::new(RandomJobRemoval::new(RuinLimits::default())), 1.)], 2),
        ]))
    }

    /// Creates default local search operator.
    pub fn create_default_local_search(environment: Arc<Environment>) -> TargetSearchOperator {
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
}

mod dynamic {
    use super::*;

    pub fn get_operators(problem: Arc<Problem>, environment: Arc<Environment>) -> Vec<(TargetSearchOperator, String)> {
        let random = environment.random.clone();
        let recreates: Vec<(Arc<dyn Recreate + Send + Sync>, String)> = vec![
            (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), "skip_best".to_string()),
            (Arc::new(RecreateWithRegret::new(1, 3, random.clone())), "regret".to_string()),
            (Arc::new(RecreateWithCheapest::new(random.clone())), "cheapest".to_string()),
            (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), "perturbation".to_string()),
            (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), "gaps".to_string()),
            (create_recreate_with_blinks(problem.as_ref(), random.clone()), "blinks".to_string()),
            (Arc::new(RecreateWithFarthest::new(random.clone())), "farthest".to_string()),
            (Arc::new(RecreateWithNearestNeighbor::new(random.clone())), "nearest".to_string()),
            (
                Arc::new(RecreateWithSkipRandom::default_explorative_phased(
                    Arc::new(RecreateWithCheapest::new(random.clone())),
                    random.clone(),
                )),
                "skip_random".to_string(),
            ),
            (Arc::new(RecreateWithSlice::new(random.clone())), "slice".to_string()),
        ];

        let ruins: Vec<(Arc<dyn Ruin + Send + Sync>, String)> = vec![
            (Arc::new(AdjustedStringRemoval::default()), "asr".to_string()),
            (Arc::new(NeighbourRemoval::default()), "neighbour_removal".to_string()),
            (
                Arc::new(ClusterRemoval::new_with_defaults(problem.clone(), environment.clone())),
                "cluster_removal".to_string(),
            ),
            (Arc::new(WorstJobRemoval::default()), "worst_job".to_string()),
            (Arc::new(RandomJobRemoval::new(RuinLimits::default())), "random_job_removal_1".to_string()),
            (Arc::new(RandomJobRemoval::new(RuinLimits::new(2, 8, 0.2, 2))), "random_job_removal_2".to_string()),
            (Arc::new(RandomRouteRemoval::default()), "random_route_removal".to_string()),
            (Arc::new(CloseRouteRemoval::default()), "close_route_removal".to_string()),
            (Arc::new(WorstRouteRemoval::default()), "worst_route_removal".to_string()),
        ];

        // NOTE we need to wrap any of ruin methods in composite which calls restore context before recreate
        let ruins = ruins
            .into_iter()
            .map::<(Arc<dyn Ruin + Send + Sync>, String), _>(|(ruin, name)| {
                (Arc::new(CompositeRuin::new(vec![(ruin, 1.)])), name)
            })
            .collect::<Vec<_>>();

        let inner_search = create_inner_heuristic_operator(problem, environment);

        let mutations: Vec<(TargetSearchOperator, String)> = vec![
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteBest::default()))),
                "local_exch_inter_route_best".to_string(),
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteRandom::default()))),
                "local_exch_inter_route_random".to_string(),
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeIntraRouteRandom::default()))),
                "local_exch_intra_route_random".to_string(),
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(RescheduleDeparture::default()))),
                "local_reschedule_departure".to_string(),
            ),
            (Arc::new(DecomposeSearch::new(inner_search, (2, 4), 2)), "decompose_search".to_string()),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeSwapStar::new(random.clone())))),
                "local_swap_star".to_string(),
            ),
        ];

        recreates
            .iter()
            .flat_map(|(recreate, recreate_name)| {
                ruins.iter().map::<(TargetSearchOperator, String), _>(move |(ruin, ruin_name)| {
                    (
                        Arc::new(RuinAndRecreate::new(ruin.clone(), recreate.clone())),
                        format!("{}+{}", ruin_name, recreate_name),
                    )
                })
            })
            .chain(mutations.into_iter())
            .collect::<Vec<_>>()
    }

    /// Creates inner heuristic operator with default parameters.
    pub fn create_inner_heuristic_operator(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> TargetSearchOperator {
        let random = environment.random.clone();
        // initialize recreate
        let cheapest = Arc::new(RecreateWithCheapest::new(random.clone()));
        let recreate = Arc::new(WeightedRecreate::new(vec![
            (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), 1),
            (Arc::new(RecreateWithRegret::new(2, 3, random.clone())), 1),
            (cheapest.clone(), 1),
            (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), 1),
            (Arc::new(RecreateWithSkipBest::new(3, 4, random.clone())), 1),
            (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), 1),
            (create_recreate_with_blinks(problem.as_ref(), random.clone()), 1),
            (Arc::new(RecreateWithFarthest::new(random.clone())), 1),
            (Arc::new(RecreateWithSlice::new(random.clone())), 1),
            (Arc::new(RecreateWithSkipRandom::default_explorative_phased(cheapest, random.clone())), 1),
        ]));

        // initialize ruin
        let random_route = Arc::new(RandomRouteRemoval::new(1, 2, 0.1));
        let random_job = Arc::new(RandomJobRemoval::new(RuinLimits::new(2, 6, 0.25, 1)));
        let random_ruin = Arc::new(WeightedRuin::new(vec![
            (vec![(random_job.clone(), 1.)], 2),
            (vec![(random_route.clone(), 1.)], 1),
        ]));

        let ruin = Arc::new(WeightedRuin::new(vec![
            (vec![(Arc::new(AdjustedStringRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
            (vec![(Arc::new(NeighbourRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
            (vec![(Arc::new(WorstJobRemoval::default()), 1.), (random_ruin.clone(), 0.1)], 10),
            (
                vec![
                    (Arc::new(ClusterRemoval::new_with_defaults(problem, environment.clone())), 1.),
                    (random_ruin, 0.1),
                ],
                5,
            ),
            (vec![(random_route, 1.), (random_job, 0.1)], 1),
        ]));
        let ruin_recreate = Arc::new(RuinAndRecreate::new(ruin, recreate));

        // initialize local search
        let local_search = Arc::new(LocalSearch::new(Arc::new(CompositeLocalOperator::new(
            vec![
                (Arc::new(ExchangeSwapStar::new(random)), 1),
                (Arc::new(ExchangeInterRouteBest::default()), 1),
                (Arc::new(ExchangeInterRouteRandom::default()), 1),
                (Arc::new(ExchangeIntraRouteRandom::default()), 1),
            ],
            1,
            1,
        ))));

        Arc::new(WeightedHeuristicOperator::new(vec![ruin_recreate, local_search], vec![100, 10]))
    }
}
