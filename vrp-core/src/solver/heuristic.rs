use super::*;
use crate::construction::heuristics::*;
use crate::models::common::FootprintSolutionState;
use crate::models::{Extras, GoalContext};
use crate::rosomaxa::get_default_selection_size;
use crate::solver::search::*;
use rosomaxa::algorithms::gsom::Input;
use rosomaxa::hyper::*;
use rosomaxa::population::*;
use rosomaxa::termination::*;
use std::marker::PhantomData;

/// A type alias for domain specific evolution strategy.
pub type TargetEvolutionStrategy =
    Box<dyn EvolutionStrategy<Context = RefinementContext, Objective = GoalContext, Solution = InsertionContext>>;
/// A type alias for domain specific population.
pub type TargetPopulation =
    Box<dyn HeuristicPopulation<Objective = GoalContext, Individual = InsertionContext> + Send + Sync>;
/// A type alias for domain specific heuristic.
pub type TargetHeuristic =
    Box<dyn HyperHeuristic<Context = RefinementContext, Objective = GoalContext, Solution = InsertionContext>>;
/// A type for domain specific heuristic operator.
pub type TargetSearchOperator = Arc<
    dyn HeuristicSearchOperator<Context = RefinementContext, Objective = GoalContext, Solution = InsertionContext>
        + Send
        + Sync,
>;

/// A type for greedy population.
pub type GreedyPopulation = Greedy<GoalContext, InsertionContext>;
/// A type for elitism population.
pub type ElitismPopulation = Elitism<GoalContext, InsertionContext>;
/// A type for rosomaxa population.
pub type RosomaxaPopulation = Rosomaxa<Footprint, GoalContext, InsertionContext>;

/// A type alias for domain specific termination type.
pub type DynTermination = dyn Termination<Context = RefinementContext, Objective = GoalContext> + Send + Sync;
/// A type for composite termination.
pub type TargetCompositeTermination = CompositeTermination<RefinementContext, GoalContext, InsertionContext>;
/// A type for max time termination.
pub type MaxTimeTermination = MaxTime<RefinementContext, GoalContext, InsertionContext>;
/// A type for max generation termination.
pub type MaxGenerationTermination = MaxGeneration<RefinementContext, GoalContext, InsertionContext>;
/// A type for min variation termination.
pub type MinVariationTermination = MinVariation<RefinementContext, GoalContext, InsertionContext, String>;

/// A heuristic probability type alias.
pub type TargetHeuristicProbability = HeuristicProbability<RefinementContext, GoalContext, InsertionContext>;
/// A heuristic group type alias.
pub type TargetHeuristicGroup = HeuristicSearchGroup<RefinementContext, GoalContext, InsertionContext>;

/// A type alias for evolution config builder.
pub type ProblemConfigBuilder = EvolutionConfigBuilder<RefinementContext, GoalContext, InsertionContext, String>;

/// A type to filter meta heuristics by name. Returns true if heuristic can be used.
pub type HeuristicFilterFn = Arc<dyn Fn(&str) -> bool + Send + Sync>;

custom_extra_property!(pub HeuristicFilter typeof HeuristicFilterFn);

/// Provides the way to get [ProblemConfigBuilder] with reasonable defaults for VRP domain.
pub struct VrpConfigBuilder {
    problem: Arc<Problem>,
    environment: Option<Arc<Environment>>,
    heuristic: Option<TargetHeuristic>,
    telemetry_mode: Option<TelemetryMode>,
}

impl VrpConfigBuilder {
    /// Creates a new instance of `VrpConfigBuilder`.
    pub fn new(problem: Arc<Problem>) -> Self {
        Self { problem, environment: None, heuristic: None, telemetry_mode: None }
    }

    /// Sets [Environment] instance to be used.
    pub fn set_environment(mut self, environment: Arc<Environment>) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Sets [TelemetryMode] to be used.
    pub fn set_telemetry_mode(mut self, mode: TelemetryMode) -> Self {
        self.telemetry_mode = Some(mode);
        self
    }

    /// Sets [TargetHeuristic] to be used.
    /// By default, it is used what is returned by [get_default_heuristic].
    pub fn set_heuristic(mut self, heuristic: TargetHeuristic) -> Self {
        self.heuristic = Some(heuristic);
        self
    }

    /// Builds a preconfigured instance of [ProblemConfigBuilder] for further usage.
    pub fn prebuild(self) -> GenericResult<ProblemConfigBuilder> {
        let problem = self.problem;
        let environment = self.environment.unwrap_or_else(|| Arc::new(Environment::default()));
        let telemetry_mode =
            self.telemetry_mode.unwrap_or_else(|| get_default_telemetry_mode(environment.logger.clone()));

        let heuristic = self.heuristic.unwrap_or_else(|| get_default_heuristic(problem.clone(), environment.clone()));

        let selection_size = get_default_selection_size(environment.as_ref());
        let footprint = Footprint::new(problem.as_ref());
        let population = get_default_population(problem.goal.clone(), footprint, environment.clone(), selection_size);

        Ok(ProblemConfigBuilder::default()
            .with_heuristic(heuristic)
            .with_context(RefinementContext::new(problem.clone(), population, telemetry_mode, environment.clone()))
            .with_processing(create_default_processing())
            .with_initial(4, 0.05, create_default_init_operators(problem, environment)))
    }
}

/// Creates default telemetry mode.B
pub fn get_default_telemetry_mode(logger: InfoLogger) -> TelemetryMode {
    TelemetryMode::OnlyLogging { logger, log_best: 100, log_population: 1000 }
}

/// Gets default heuristic.
pub fn get_default_heuristic(problem: Arc<Problem>, environment: Arc<Environment>) -> TargetHeuristic {
    Timer::measure_duration_with_callback(
        || Box::new(get_dynamic_heuristic(problem, environment.clone())),
        |duration| (environment.logger)(format!("getting default heuristic took: {}ms", duration.as_millis()).as_str()),
    )
}

/// Gets static heuristic using default settings.
pub fn get_static_heuristic(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> StaticSelective<RefinementContext, GoalContext, InsertionContext> {
    let default_operator = statik::create_default_heuristic_operator(problem.clone(), environment.clone());
    let local_search = statik::create_default_local_search(environment.random.clone());

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
        (
            Arc::new(LKHSearch::new(LKHSearchMode::ImprovementOnly)),
            create_scalar_operator_probability(0.05, environment.random.clone()),
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
) -> StaticSelective<RefinementContext, GoalContext, InsertionContext> {
    StaticSelective::<RefinementContext, GoalContext, InsertionContext>::new(
        heuristic_group,
        create_diversify_operators(problem, environment),
    )
}

/// Gets dynamic heuristic using default settings.
pub fn get_dynamic_heuristic(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> DynamicSelective<RefinementContext, GoalContext, InsertionContext> {
    let search_operators = dynamic::get_operators(problem.clone(), environment.clone());
    let diversify_operators = create_diversify_operators(problem, environment.clone());

    DynamicSelective::<RefinementContext, GoalContext, InsertionContext>::new(
        search_operators,
        diversify_operators,
        environment.as_ref(),
    )
}

/// Creates elitism population algorithm.
pub fn create_elitism_population(
    objective: Arc<GoalContext>,
    environment: Arc<Environment>,
) -> Elitism<GoalContext, InsertionContext> {
    let selection_size = get_default_selection_size(environment.as_ref());
    Elitism::new(objective, environment.random.clone(), 4, selection_size)
}

custom_solution_state!(SolutionWeights typeof Vec<Float>);

impl RosomaxaSolution for InsertionContext {
    type Context = Footprint;

    fn on_init(&mut self, context: &Self::Context) {
        // built a feature vector which is used to classify solution in population
        let weights = vec![
            // load related features
            get_max_load_variance(self),
            get_max_load_mean(self),
            get_full_load_ratio(self),
            // time related features
            get_duration_mean(self),
            get_waiting_mean(self),
            // distance related features
            get_distance_mean(self),
            get_longest_distance_between_customers_mean(self),
            get_first_distance_customer_mean(self),
            get_last_distance_customer_mean(self),
            // depot related features
            get_average_distance_between_depot_customer_mean(self),
            get_longest_distance_between_depot_customer_mean(self),
            // tour related features
            get_customers_deviation(self),
            // default objective related
            self.solution.unassigned.len() as Float,
            self.solution.routes.len() as Float,
            self.get_total_cost().unwrap_or_default(),
        ];

        self.solution.state.set_solution_weights(weights);
        self.on_update(context);
    }

    fn on_update(&mut self, context: &Self::Context) {
        self.solution.state.set_footprint(context.clone());
    }
}

impl Input for InsertionContext {
    fn weights(&self) -> &[Float] {
        self.solution.state.get_solution_weights().unwrap().as_slice()
    }
}

/// Creates a heuristic operator probability which uses `is_hit` method from passed random object.
pub fn create_scalar_operator_probability(
    scalar_probability: Float,
    random: Arc<dyn Random>,
) -> TargetHeuristicProbability {
    (Box::new(move |_, _| random.is_hit(scalar_probability)), PhantomData)
}

/// Creates a heuristic operator probability which uses context state.
pub fn create_context_operator_probability(
    jobs_threshold: usize,
    routes_threshold: usize,
    phases: Vec<(SelectionPhase, Float)>,
    random: Arc<dyn Random>,
) -> TargetHeuristicProbability {
    let phases = phases.into_iter().collect::<HashMap<_, _>>();
    (
        Box::new(move |refinement_ctx, insertion_ctx| {
            let below_thresholds = insertion_ctx.problem.jobs.size() < jobs_threshold
                || insertion_ctx.solution.routes.len() < routes_threshold;

            if below_thresholds {
                return false;
            }

            let phase_probability = phases.get(&refinement_ctx.selection_phase()).cloned().unwrap_or(0.);
            random.is_hit(phase_probability)
        }),
        PhantomData,
    )
}

fn get_limits(problem: &Problem) -> (RemovalLimits, RemovalLimits) {
    let normal_limits = RemovalLimits::new(problem);
    let activities_range = normal_limits.removed_activities_range.clone();
    let small_limits = RemovalLimits {
        removed_activities_range: (activities_range.start / 3).max(2)..(activities_range.end / 3).max(8),
        affected_routes_range: 1..2,
    };

    (normal_limits, small_limits)
}

pub use self::builder::create_default_init_operators;
pub use self::builder::create_default_processing;
pub use self::statik::create_default_heuristic_operator;

mod builder {
    use super::*;
    use crate::rosomaxa::evolution::InitialOperators;
    use crate::solver::RecreateInitialOperator;
    use crate::solver::processing::*;

    /// Creates default init operators.
    pub fn create_default_init_operators(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> InitialOperators<RefinementContext, GoalContext, InsertionContext> {
        type VrpInitialOperator = dyn InitialOperator<Context = RefinementContext, Objective = GoalContext, Solution = InsertionContext>
            + Send
            + Sync;

        let random = environment.random.clone();
        let wrap: fn(Arc<dyn Recreate>) -> Box<VrpInitialOperator> =
            |recreate| Box::new(RecreateInitialOperator::new(recreate));

        std::iter::once({
            // main stable constructive heuristics
            (wrap(Arc::new(RecreateWithCheapest::new(random.clone()))), 1)
        })
        .chain(
            // alternative constructive heuristics
            get_recreate_with_alternative_goal(problem.goal.as_ref(), {
                let random = random.clone();
                move || RecreateWithCheapest::new(random.clone())
            })
            .map(|recreate| (wrap(recreate), 1)),
        )
        .chain([
            // additional constructive heuristics
            (wrap(Arc::new(RecreateWithFarthest::new(random.clone()))), 1),
            (wrap(Arc::new(RecreateWithBlinks::new_with_defaults(random.clone()))), 1),
            (wrap(Arc::new(RecreateWithRegret::new(2, 3, random.clone()))), 1),
            (wrap(Arc::new(RecreateWithGaps::new(1, (problem.jobs.size() / 10).max(1), random.clone()))), 1),
            (wrap(Arc::new(RecreateWithSkipBest::new(1, 2, random.clone()))), 1),
            (wrap(Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone()))), 1),
            (wrap(Arc::new(RecreateWithNearestNeighbor::new(random.clone()))), 1),
        ])
        .collect()
    }

    /// Create default processing.
    pub fn create_default_processing() -> ProcessingConfig<RefinementContext, GoalContext, InsertionContext> {
        ProcessingConfig {
            context: vec![Box::<VicinityClustering>::default()],
            solution: vec![
                Box::new(AdvanceDeparture::default()),
                Box::<RescheduleReservedTime>::default(),
                Box::<UnassignmentReason>::default(),
                Box::<VicinityClustering>::default(),
            ],
        }
    }
}

fn create_diversify_operators(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> HeuristicDiversifyOperators<RefinementContext, GoalContext, InsertionContext> {
    let random = environment.random.clone();

    let recreates: Vec<(Arc<dyn Recreate>, usize)> = vec![
        (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), 1),
        (Arc::new(RecreateWithRegret::new(1, 3, random.clone())), 1),
        (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), 1),
        (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), 1),
        (Arc::new(RecreateWithNearestNeighbor::new(random.clone())), 1),
        (Arc::new(RecreateWithSlice::new(random.clone())), 1),
    ];

    let redistribute_search = Arc::new(RedistributeSearch::new(Arc::new(WeightedRecreate::new(recreates))));
    let infeasible_search = Arc::new(InfeasibleSearch::new(
        Arc::new(WeightedHeuristicOperator::new(
            vec![
                dynamic::create_default_inner_ruin_recreate(problem, environment.clone()),
                dynamic::create_default_local_search(random.clone()),
            ],
            vec![10, 1],
        )),
        Arc::new(RecreateWithCheapest::new(random)),
        4,
        (0.05, 0.2),
        (0.33, 0.75),
    ));
    let local_search = Arc::new(LocalSearch::new(Arc::new(CompositeLocalOperator::new(
        vec![(Arc::new(ExchangeSequence::new(8, 0.5, 0.1)), 1)],
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
    /// NOTE: should not contain heuristics which rely on repair_solution_from_unknown as this funciton is used
    /// from decompose search which creates partial solutions.
    pub fn create_default_heuristic_operator(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> TargetSearchOperator {
        let (normal_limits, small_limits) = get_limits(problem.as_ref());
        let random = environment.random.clone();

        // initialize recreate
        let recreate = Arc::new(WeightedRecreate::new(vec![
            (Arc::new(RecreateWithBlinks::new_with_defaults(random.clone())), 50),
            (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), 20),
            (Arc::new(RecreateWithRegret::new(2, 3, random.clone())), 20),
            (Arc::new(RecreateWithCheapest::new(random.clone())), 20),
            (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), 10),
            (Arc::new(RecreateWithSkipBest::new(3, 4, random.clone())), 5),
            (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), 5),
            (Arc::new(RecreateWithFarthest::new(random.clone())), 2),
            (Arc::new(RecreateWithSkipBest::new(4, 8, random.clone())), 2),
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
        let close_route = Arc::new(CloseRouteRemoval::new(normal_limits.clone()));
        let worst_route = Arc::new(WorstRouteRemoval::new(normal_limits.clone()));
        let random_route = Arc::new(RandomRouteRemoval::new(normal_limits.clone()));

        let random_job = Arc::new(RandomJobRemoval::new(normal_limits.clone()));
        let extra_random_job = Arc::new(RandomJobRemoval::new(small_limits));

        let ruin = Arc::new(WeightedRuin::new(vec![
            (
                Arc::new(CompositeRuin::new(vec![
                    (Arc::new(AdjustedStringRemoval::new_with_defaults(normal_limits.clone())), 2.),
                    (extra_random_job.clone(), 0.1),
                ])),
                100,
            ),
            (
                Arc::new(CompositeRuin::new(vec![
                    (Arc::new(NeighbourRemoval::new(normal_limits.clone())), 1.),
                    (extra_random_job.clone(), 0.1),
                ])),
                10,
            ),
            (
                Arc::new(CompositeRuin::new(vec![
                    (Arc::new(WorstJobRemoval::new(4, normal_limits)), 1.),
                    (extra_random_job.clone(), 0.1),
                ])),
                10,
            ),
            (
                Arc::new(CompositeRuin::new(vec![
                    // TODO avoid unwrap
                    (Arc::new(ClusterRemoval::new_with_defaults(problem.clone()).unwrap()), 1.),
                    (extra_random_job.clone(), 0.1),
                ])),
                5,
            ),
            (Arc::new(CompositeRuin::new(vec![(close_route, 1.), (extra_random_job.clone(), 0.1)])), 2),
            (Arc::new(CompositeRuin::new(vec![(worst_route, 1.), (extra_random_job.clone(), 0.1)])), 1),
            (Arc::new(CompositeRuin::new(vec![(random_route, 1.), (extra_random_job.clone(), 0.1)])), 1),
            (Arc::new(CompositeRuin::new(vec![(random_job, 1.), (extra_random_job, 0.1)])), 1),
        ]));

        Arc::new(WeightedHeuristicOperator::new(
            vec![
                Arc::new(RuinAndRecreate::new(ruin, recreate)),
                create_default_local_search(environment.random.clone()),
            ],
            vec![100, 10],
        ))
    }

    /// Creates default local search operator.
    pub fn create_default_local_search(random: Arc<dyn Random>) -> TargetSearchOperator {
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

    /// Bandit prior weight for the SISR pair (asr, blinks). Boosted over other strong arms.
    const SISR_BOOST_WEIGHT: Float = 3.0;
    /// Bandit prior weight for non-boosted strong-tier operators.
    const STRONG_WEIGHT: Float = 1.0;
    /// Bandit prior weight for weak-tier arms.
    const WEAK_ARM_PRIOR: Float = 0.5;

    /// Internal `WeightedRuin`/`WeightedRecreate` weight for strong members (used in weak-tier bundles
    /// and in non-bandit inner R&R). Combined with [`WEAK_BUNDLE_WEIGHT`] to give a strong:weak = 2:1 ratio.
    const STRONG_BUNDLE_WEIGHT: usize = 2;
    /// Internal `WeightedRuin`/`WeightedRecreate` weight for weak members.
    const WEAK_BUNDLE_WEIGHT: usize = 1;

    /// Wraps every primary ruin in a `CompositeRuin` with a small-probability `extra_random_job`
    /// companion to mimic today's "small chaos" baseline. `random_job` is its own primary, so we
    /// skip the wrapper for it to avoid double-counting random destruction.
    fn wrap_with_extra(ruin: Arc<dyn Ruin>, name: &str, extra: Arc<dyn Ruin>) -> Arc<dyn Ruin> {
        if name == "random_job" {
            ruin
        } else {
            Arc::new(CompositeRuin::new(vec![(ruin, 1.), (extra, 0.1)]))
        }
    }

    fn get_strong_ruins(
        problem: Arc<Problem>,
        normal_limits: &RemovalLimits,
        small_limits: &RemovalLimits,
    ) -> Vec<(Arc<dyn Ruin>, String, Float)> {
        // Combines normal and small limits (2:1, favoring normal).
        let create_weighted = |factory: fn(RemovalLimits) -> Arc<dyn Ruin>| {
            Arc::new(WeightedRuin::new(vec![(factory(normal_limits.clone()), 2), (factory(small_limits.clone()), 1)]))
                as Arc<dyn Ruin>
        };

        vec![
            (
                create_weighted(|limits| Arc::new(AdjustedStringRemoval::new_with_defaults(limits))),
                "asr".to_string(),
                SISR_BOOST_WEIGHT,
            ),
            (Arc::new(ClusterRemoval::new_with_defaults(problem).unwrap()), "cluster".to_string(), STRONG_WEIGHT),
            (
                create_weighted(|limits| Arc::new(WorstJobRemoval::new(4, limits))),
                "worst_job".to_string(),
                STRONG_WEIGHT,
            ),
            (
                create_weighted(|limits| Arc::new(WorstRouteRemoval::new(limits))),
                "worst_route".to_string(),
                STRONG_WEIGHT,
            ),
            (
                create_weighted(|limits| Arc::new(CloseRouteRemoval::new(limits))),
                "close_route".to_string(),
                STRONG_WEIGHT,
            ),
        ]
    }

    fn get_weak_ruins(
        normal_limits: &RemovalLimits,
        small_limits: &RemovalLimits,
    ) -> Vec<(Arc<dyn Ruin>, String, Float)> {
        let create_weighted = |factory: fn(RemovalLimits) -> Arc<dyn Ruin>| {
            Arc::new(WeightedRuin::new(vec![(factory(normal_limits.clone()), 2), (factory(small_limits.clone()), 1)]))
                as Arc<dyn Ruin>
        };

        vec![
            (
                create_weighted(|limits| Arc::new(NeighbourRemoval::new(limits))),
                "neighbour".to_string(),
                WEAK_ARM_PRIOR,
            ),
            (
                create_weighted(|limits| Arc::new(RandomJobRemoval::new(limits))),
                "random_job".to_string(),
                WEAK_ARM_PRIOR,
            ),
            (
                create_weighted(|limits| Arc::new(RandomRouteRemoval::new(limits))),
                "random_route".to_string(),
                WEAK_ARM_PRIOR,
            ),
        ]
    }

    fn get_strong_recreates(
        problem: &Problem,
        random: Arc<dyn Random>,
    ) -> Vec<(Arc<dyn Recreate>, String, Float)> {
        let blinks: Arc<dyn Recreate> = Arc::new(RecreateWithBlinks::new_with_defaults(random.clone()));
        let cheapest: Arc<dyn Recreate> = Arc::new(RecreateWithCheapest::new(random.clone()));
        let regret: Arc<dyn Recreate> = Arc::new(RecreateWithRegret::new(1, 3, random.clone()));
        vec![
            (blinks, "blinks".to_string(), SISR_BOOST_WEIGHT),
            (cheapest, "cheapest".to_string(), STRONG_WEIGHT),
            (regret, "regret".to_string(), STRONG_WEIGHT),
        ]
        .into_iter()
        .chain(
            get_recreate_with_alternative_goal(problem.goal.as_ref(), {
                let random = random.clone();
                move || RecreateWithCheapest::new(random.clone())
            })
            .enumerate()
            .map(|(idx, recreate)| (recreate, format!("alternative_{idx}"), STRONG_WEIGHT)),
        )
        .collect()
    }

    fn get_weak_recreates(random: Arc<dyn Random>) -> Vec<(Arc<dyn Recreate>, String, Float)> {
        let cheapest: Arc<dyn Recreate> = Arc::new(RecreateWithCheapest::new(random.clone()));
        let skip_best: Arc<dyn Recreate> = Arc::new(RecreateWithSkipBest::new(1, 2, random.clone()));
        let perturbation: Arc<dyn Recreate> = Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone()));
        let gaps: Arc<dyn Recreate> = Arc::new(RecreateWithGaps::new(2, 20, random.clone()));
        let farthest: Arc<dyn Recreate> = Arc::new(RecreateWithFarthest::new(random.clone()));
        let skip_random: Arc<dyn Recreate> =
            Arc::new(RecreateWithSkipRandom::default_explorative_phased(cheapest, random.clone()));
        let slice: Arc<dyn Recreate> = Arc::new(RecreateWithSlice::new(random));

        vec![
            (skip_best, "skip_best".to_string(), WEAK_ARM_PRIOR),
            (perturbation, "perturbation".to_string(), WEAK_ARM_PRIOR),
            (gaps, "gaps".to_string(), WEAK_ARM_PRIOR),
            (farthest, "farthest".to_string(), WEAK_ARM_PRIOR),
            (skip_random, "skip_random".to_string(), WEAK_ARM_PRIOR),
            (slice, "slice".to_string(), WEAK_ARM_PRIOR),
        ]
    }

    /// Builds a `WeightedRecreate` bundle (strong:weak = 2:1) used as the recreate side of a
    /// weak-ruin arm — keeps weak ruins paired mostly with strong recreates while preserving
    /// reachability of weak×weak combinations.
    fn build_weak_recreate_bundle(
        strong: &[(Arc<dyn Recreate>, String, Float)],
        weak: &[(Arc<dyn Recreate>, String, Float)],
    ) -> Arc<dyn Recreate> {
        let mut bundle: Vec<(Arc<dyn Recreate>, usize)> = Vec::with_capacity(strong.len() + weak.len());
        bundle.extend(strong.iter().map(|(r, _, _)| (r.clone(), STRONG_BUNDLE_WEIGHT)));
        bundle.extend(weak.iter().map(|(r, _, _)| (r.clone(), WEAK_BUNDLE_WEIGHT)));
        Arc::new(WeightedRecreate::new(bundle))
    }

    /// Builds a `WeightedRuin` bundle (strong:weak = 2:1) used as the ruin side of a weak-recreate arm.
    /// Each member is wrapped with the `extra_random_job` companion (except `random_job` itself).
    fn build_weak_ruin_bundle(
        strong: &[(Arc<dyn Ruin>, String, Float)],
        weak: &[(Arc<dyn Ruin>, String, Float)],
        extra: Arc<dyn Ruin>,
    ) -> Arc<dyn Ruin> {
        let mut bundle: Vec<(Arc<dyn Ruin>, usize)> = Vec::with_capacity(strong.len() + weak.len());
        bundle.extend(
            strong
                .iter()
                .map(|(r, n, _)| (wrap_with_extra(r.clone(), n, extra.clone()), STRONG_BUNDLE_WEIGHT)),
        );
        bundle.extend(
            weak.iter().map(|(r, n, _)| (wrap_with_extra(r.clone(), n, extra.clone()), WEAK_BUNDLE_WEIGHT)),
        );
        Arc::new(WeightedRuin::new(bundle))
    }

    fn get_search_operators(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> Vec<(TargetSearchOperator, String, Float)> {
        vec![
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteBest::default()))),
                "local_exch_inter_route_best".to_string(),
                1.,
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteRandom::default()))),
                "local_exch_inter_route_random".to_string(),
                1.,
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeIntraRouteRandom::default()))),
                "local_exch_intra_route_random".to_string(),
                1.,
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(RescheduleDeparture::default()))),
                "local_reschedule_departure".to_string(),
                1.,
            ),
            (Arc::new(LKHSearch::new(LKHSearchMode::ImprovementOnly)), "lkh_strict".to_string(), 1.),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeSwapStar::new(environment.random.clone())))),
                "local_swap_star".to_string(),
                2.,
            ),
            (
                create_variable_search_decompose_search(problem.clone(), environment.clone()),
                "variable_decompose_search".to_string(),
                2.,
            ),
            (create_composite_decompose_search(problem, environment), "composite_decompose_search".to_string(), 2.),
        ]
    }

    pub fn get_operators(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> Vec<(TargetSearchOperator, String, Float)> {
        let (normal_limits, small_limits) = get_limits(problem.as_ref());
        let random = environment.random.clone();

        // NOTE: consider checking usage of names within heuristic filter before changing them

        let strong_ruins = get_strong_ruins(problem.clone(), &normal_limits, &small_limits);
        let weak_ruins = get_weak_ruins(&normal_limits, &small_limits);
        let strong_recreates = get_strong_recreates(problem.as_ref(), random.clone());
        let weak_recreates = get_weak_recreates(random.clone());

        let extra_random_job: Arc<dyn Ruin> = Arc::new(RandomJobRemoval::new(small_limits));

        // Strong cartesian: every strong ruin × every strong recreate. Ruins are wrapped with the
        // small `extra_random_job` companion (today's "small chaos" baseline).
        let wrapped_strong_ruins = strong_ruins
            .iter()
            .map(|(ruin, name, weight)| {
                (wrap_with_extra(ruin.clone(), name, extra_random_job.clone()), name.clone(), *weight)
            })
            .collect::<Vec<_>>();

        let strong_cartesian_ops = strong_recreates
            .iter()
            .flat_map(|(recreate, recreate_name, recreate_weight)| {
                wrapped_strong_ruins.iter().map::<(TargetSearchOperator, String, Float), _>(
                    move |(ruin, ruin_name, ruin_weight)| {
                        (
                            Arc::new(RuinAndRecreate::new(ruin.clone(), recreate.clone())),
                            format!("{ruin_name}+{recreate_name}"),
                            ruin_weight + recreate_weight,
                        )
                    },
                )
            })
            .collect::<Vec<_>>();

        // Weak ruin arms: each weak ruin paired with a strong-heavy WeightedRecreate bundle.
        let weak_ruin_recreate_bundle = build_weak_recreate_bundle(&strong_recreates, &weak_recreates);
        let weak_ruin_ops = weak_ruins
            .iter()
            .map::<(TargetSearchOperator, String, Float), _>(|(ruin, name, weight)| {
                let primary = wrap_with_extra(ruin.clone(), name, extra_random_job.clone());
                (
                    Arc::new(RuinAndRecreate::new(primary, weak_ruin_recreate_bundle.clone())),
                    name.clone(),
                    *weight,
                )
            })
            .collect::<Vec<_>>();

        // Weak recreate arms: each weak recreate paired with a strong-heavy WeightedRuin bundle.
        let weak_recreate_ruin_bundle =
            build_weak_ruin_bundle(&strong_ruins, &weak_ruins, extra_random_job);
        let weak_recreate_ops = weak_recreates
            .iter()
            .map::<(TargetSearchOperator, String, Float), _>(|(recreate, name, weight)| {
                (
                    Arc::new(RuinAndRecreate::new(weak_recreate_ruin_bundle.clone(), recreate.clone())),
                    name.clone(),
                    *weight,
                )
            })
            .collect::<Vec<_>>();

        let operators = get_search_operators(problem.clone(), environment.clone());
        let heuristic_filter = problem.extras.get_heuristic_filter();

        strong_cartesian_ops
            .into_iter()
            .chain(weak_ruin_ops)
            .chain(weak_recreate_ops)
            .chain(operators)
            .filter(|(_, name, _)| heuristic_filter.as_ref().is_none_or(|filter| (filter)(name.as_str())))
            .collect::<Vec<_>>()
    }

    /// Creates a default operator which is good for general use.
    pub fn create_default_good_operator(problem: Arc<Problem>, environment: Arc<Environment>) -> TargetSearchOperator {
        Arc::new(RuinAndRecreate::new(
            Arc::new(AdjustedStringRemoval::new_with_defaults(get_limits(problem.as_ref()).0)),
            Arc::new(RecreateWithBlinks::new_with_defaults(environment.random.clone())),
        ))
    }

    /// Creates a default ruin-and-recreate operator for internal use (e.g., decompose search, infeasible search).
    /// Uses the same tier philosophy as the main bandit: strong members dominate (2:1 over weak)
    /// inside both the ruin and recreate bundles, with the SISR pair (asr, blinks) further boosted.
    pub fn create_default_inner_ruin_recreate(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> Arc<RuinAndRecreate> {
        let (normal_limits, small_limits) = get_limits(problem.as_ref());
        let random = environment.random.clone();

        let strong_ruins = get_strong_ruins(problem.clone(), &normal_limits, &small_limits);
        let weak_ruins = get_weak_ruins(&normal_limits, &small_limits);
        let strong_recreates = get_strong_recreates(problem.as_ref(), random.clone());
        let weak_recreates = get_weak_recreates(random);

        let extra_random_job: Arc<dyn Ruin> = Arc::new(RandomJobRemoval::new(small_limits));

        // Map bandit-prior weights to integer bundle weights, preserving the SISR boost
        // (asr=3, blinks=3) and the strong/weak split. Weak members at half-weight after rounding,
        // so we use an explicit table instead of casting Float→usize.
        let to_bundle_weight = |float_weight: Float| -> usize {
            if float_weight >= SISR_BOOST_WEIGHT {
                SISR_BOOST_WEIGHT as usize * STRONG_BUNDLE_WEIGHT
            } else if float_weight >= STRONG_WEIGHT {
                STRONG_BUNDLE_WEIGHT
            } else {
                WEAK_BUNDLE_WEIGHT
            }
        };

        let weighted_ruins: Vec<(Arc<dyn Ruin>, usize)> = strong_ruins
            .iter()
            .chain(weak_ruins.iter())
            .map(|(ruin, name, weight)| {
                (wrap_with_extra(ruin.clone(), name, extra_random_job.clone()), to_bundle_weight(*weight))
            })
            .collect();

        let weighted_recreates: Vec<(Arc<dyn Recreate>, usize)> = strong_recreates
            .iter()
            .chain(weak_recreates.iter())
            .map(|(recreate, _, weight)| (recreate.clone(), to_bundle_weight(*weight)))
            .collect();

        Arc::new(RuinAndRecreate::new(
            Arc::new(WeightedRuin::new(weighted_ruins)),
            Arc::new(WeightedRecreate::new(weighted_recreates)),
        ))
    }

    pub fn create_default_local_search(random: Arc<dyn Random>) -> Arc<LocalSearch> {
        Arc::new(LocalSearch::new(Arc::new(CompositeLocalOperator::new(
            vec![
                (Arc::new(ExchangeSwapStar::new(random)), 2),
                (Arc::new(ExchangeInterRouteBest::default()), 1),
                (Arc::new(ExchangeInterRouteRandom::default()), 1),
                (Arc::new(ExchangeIntraRouteRandom::default()), 1),
                (Arc::new(ExchangeSequence::default()), 1),
            ],
            1,
            1,
        ))))
    }

    fn create_variable_search_decompose_search(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> TargetSearchOperator {
        Arc::new(DecomposeSearch::new(
            Arc::new(WeightedHeuristicOperator::new(
                vec![
                    create_default_inner_ruin_recreate(problem.clone(), environment.clone()),
                    create_default_good_operator(problem, environment.clone()),
                    create_default_local_search(environment.random.clone()),
                ],
                vec![9, 3, 1],
            )),
            (2, 4),
            2,
        ))
    }

    fn create_composite_decompose_search(problem: Arc<Problem>, environment: Arc<Environment>) -> TargetSearchOperator {
        let limits = RemovalLimits { removed_activities_range: (10..100), affected_routes_range: 1..2 };
        let ruin = WeightedRuin::new(vec![
            (Arc::new(RandomRouteRemoval::new(limits.clone())), 1),
            (Arc::new(WorstRouteRemoval::new(limits)), 1),
        ]);
        let route_removal_operator = Arc::new(RuinAndRecreate::new(Arc::new(ruin), Arc::new(DummyRecreate)));

        Arc::new(DecomposeSearch::new(
            Arc::new(CompositeHeuristicOperator::new(vec![
                (route_removal_operator, 1.),
                (create_default_inner_ruin_recreate(problem.clone(), environment.clone()), 1.),
            ])),
            (2, 4),
            2,
        ))
    }
}

fn get_recreate_with_alternative_goal<T, F>(
    original_goal: &GoalContext,
    recreate_fn: F,
) -> impl Iterator<Item = Arc<dyn Recreate>> + '_
where
    T: Recreate + Send + Sync + 'static,
    F: Fn() -> T + 'static,
{
    original_goal
        .get_alternatives()
        .map::<Arc<dyn Recreate>, _>(move |goal| Arc::new(RecreateWithGoal::new(Arc::new(goal), recreate_fn())))
}
