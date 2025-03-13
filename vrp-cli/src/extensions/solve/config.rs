//! Solver configuration.

#![allow(missing_docs)]

#[cfg(test)]
#[path = "../../../tests/unit/extensions/solve/config_test.rs"]
mod config_test;

extern crate serde_json;

use serde::Deserialize;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::models::GoalContext;
use vrp_core::models::common::Footprint;
use vrp_core::prelude::*;
use vrp_core::rosomaxa::evolution::{InitialOperator, TelemetryMode};
use vrp_core::rosomaxa::get_default_selection_size;
use vrp_core::rosomaxa::prelude::*;
use vrp_core::rosomaxa::utils::*;
use vrp_core::solver::RecreateInitialOperator;
use vrp_core::solver::search::*;
use vrp_core::solver::*;

/// An algorithm configuration.
#[derive(Clone, Default, Deserialize, Debug)]
pub struct Config {
    /// Specifies evolution configuration.
    pub evolution: Option<EvolutionConfig>,
    /// Specifies hyper heuristic type.
    pub hyper: Option<HyperType>,
    /// Specifies algorithm termination configuration.
    pub termination: Option<TerminationConfig>,
    /// Specifies environment configuration.
    pub environment: Option<EnvironmentConfig>,
    /// Specifies telemetry configuration.
    pub telemetry: Option<TelemetryConfig>,
    /// Specifies output configuration.
    pub output: Option<OutputConfig>,
}

/// An evolution configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionConfig {
    pub initial: Option<InitialConfig>,
    pub population: Option<PopulationType>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum PopulationType {
    /// A greedy population keeps track only of one best-known individual.
    #[serde(rename(deserialize = "greedy"))]
    #[serde(rename_all = "camelCase")]
    Greedy {
        /// Selection size. Default is number of cpus.
        selection_size: Option<usize>,
    },

    /// A basic population which sorts individuals based on their
    /// dominance order.
    #[serde(rename(deserialize = "elitism"))]
    #[serde(rename_all = "camelCase")]
    Elitism {
        /// Max population size. Default is 4.
        max_size: Option<usize>,
        /// Selection size. Default is number of cpus.
        selection_size: Option<usize>,
    },

    /// A population algorithm based on SOM.
    #[serde(rename(deserialize = "rosomaxa"))]
    #[serde(rename_all = "camelCase")]
    Rosomaxa {
        /// Selection size. Default is number of cpus.
        selection_size: Option<usize>,
        /// Elite population size. Default is 2.
        max_elite_size: Option<usize>,
        /// Node population size. Default is 4.
        max_node_size: Option<usize>,
        /// Spread factor. Default is 0.75.
        spread_factor: Option<Float>,
        /// Distribution factor. Default is 0.75.
        distribution_factor: Option<Float>,
        /// A rebalance memory. Default is 100.
        rebalance_memory: Option<usize>,
        /// An exploration phase ratio. Default is 0.9.
        exploration_ratio: Option<Float>,
    },
}

/// An initial solution configuration.
#[derive(Clone, Deserialize, Debug)]
pub struct InitialConfig {
    pub method: RecreateMethod,
    pub alternatives: InitialAlternativesConfig,
}

/// An initial solution alternatives configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitialAlternativesConfig {
    pub methods: Vec<RecreateMethod>,
    pub max_size: usize,
    pub quota: Float,
}

/// A selection operator configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum SelectionType {
    #[serde(rename(deserialize = "naive"))]
    Naive {
        /// A size of offspring.
        offspring_size: Option<usize>,
    },
}

/// A hyper heuristic configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum HyperType {
    /// A hyper heuristic which selects one operator from the list based on its predefined probability.
    #[serde(rename(deserialize = "static-selective"))]
    StaticSelective {
        /// A collection of inner operators (metaheuristics).
        operators: Option<Vec<SearchOperatorType>>,
    },

    /// A hyper heuristic which selects operator from the predefined list using reinforcement
    /// learning technics.
    #[serde(rename(deserialize = "dynamic-selective"))]
    DynamicSelective,
}

/// A operator configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SearchOperatorType {
    /// A metaheuristic which splits problem into smaller and solves them independently.
    #[serde(rename(deserialize = "decomposition"))]
    #[serde(rename_all = "camelCase")]
    Decomposition {
        /// Max routes to be selected in decomposed solution.
        routes: MinMaxConfig,
        /// Amount of attempts to repeat refinement.
        repeat: usize,
        /// Probability of operator.
        probability: OperatorProbabilityType,
    },

    /// A local search heuristic.
    #[serde(rename(deserialize = "local-search"))]
    LocalSearch {
        /// Probability of operator.
        probability: OperatorProbabilityType,
        /// Amount of times one of operators is applied.
        times: MinMaxConfig,
        /// Local search operator.
        operators: Vec<LocalOperatorType>,
    },

    /// A ruin and recreate metaheuristic settings.
    #[serde(rename(deserialize = "ruin-recreate"))]
    RuinRecreate {
        /// Probability.
        probability: OperatorProbabilityType,
        /// Ruin methods.
        ruins: Vec<RuinGroupConfig>,
        /// Recreate methods.
        recreates: Vec<RecreateMethod>,
    },
}

/// A operator probability type
#[derive(Clone, Deserialize, Debug)]
#[serde(untagged)]
pub enum OperatorProbabilityType {
    /// A scalar probability based type.
    Scalar {
        /// Probability value of the operator.
        scalar: Float,
    },

    /// A context specific probability type.
    Context {
        /// Threshold parameters.
        threshold: ContextThreshold,
        /// Selection phase specific parameters.
        phases: Vec<ContextPhase>,
    },
}

/// A context condition for `MutationProbabilityType`.
#[derive(Clone, Deserialize, Debug)]
pub struct ContextThreshold {
    /// Min amount of jobs in individual.
    pub jobs: usize,
    /// Min amount of routes in individual.
    pub routes: usize,
}

/// A selection phase filter for `MutationProbabilityType`.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ContextPhase {
    /// Initial selection phase.
    #[serde(rename(deserialize = "initial"))]
    Initial {
        /// A chance defined by probability.
        chance: Float,
    },

    /// Exploration search phase.
    #[serde(rename(deserialize = "exploration"))]
    Exploration {
        /// A chance defined by probability.
        chance: Float,
    },

    /// Exploitation search phase.
    #[serde(rename(deserialize = "exploitation"))]
    Exploitation {
        /// A chance defined by probability.
        chance: Float,
    },
}

/// A ruin method configuration.
#[derive(Clone, Deserialize, Debug)]
pub struct RuinGroupConfig {
    /// Ruin methods.
    methods: Vec<RuinMethod>,
    /// Weight of the group.
    weight: usize,
}

/// Specifies ruin methods with their probability weight and specific parameters.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum RuinMethod {
    /// Adjusted string removal method.
    #[serde(rename(deserialize = "adjusted-string"))]
    AdjustedString { probability: Float, lmax: usize, cavg: usize, alpha: Float },
    /// Neighbour jobs method
    #[serde(rename(deserialize = "neighbour"))]
    Neighbour { probability: Float, min: usize, max: usize },
    /// Random job removal method.
    #[serde(rename(deserialize = "random-job"))]
    RandomJob { probability: Float, min: usize, max: usize },
    /// Random route removal method.
    #[serde(rename(deserialize = "random-route"))]
    RandomRoute { probability: Float, min: usize, max: usize },
    /// Close route removal method.
    #[serde(rename(deserialize = "close-route"))]
    CloseRoute { probability: Float },
    #[serde(rename(deserialize = "worst-route"))]
    WorstRoute { probability: Float },
    /// Worst job removal method.
    #[serde(rename(deserialize = "worst-job"))]
    WorstJob { probability: Float, min: usize, max: usize, skip: usize },
    /// Clustered jobs removal method.
    #[serde(rename(deserialize = "cluster"))]
    #[serde(rename_all = "camelCase")]
    Cluster { probability: Float, min: usize, max: usize },
}

/// Specifies recreate methods with their probability weight and specific parameters.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum RecreateMethod {
    /// Cheapest insertion method.
    #[serde(rename(deserialize = "cheapest"))]
    Cheapest { weight: usize },
    /// SkipBest insertion method.
    #[serde(rename(deserialize = "skip-best"))]
    SkipBest { weight: usize, start: usize, end: usize },
    /// Insertion with blinks method.
    #[serde(rename(deserialize = "blinks"))]
    Blinks { weight: usize },
    /// Insertion with gaps method.
    #[serde(rename(deserialize = "gaps"))]
    Gaps { weight: usize, min: usize, max: usize },
    /// Nearest neighbour method.
    #[serde(rename(deserialize = "nearest"))]
    Nearest { weight: usize },
    /// Insertion with skip random method.
    #[serde(rename(deserialize = "skip-random"))]
    SkipRandom { weight: usize },
    /// Insertion with slice method.
    #[serde(rename(deserialize = "slice"))]
    Slice { weight: usize },
    /// Farthest insertion method.
    #[serde(rename(deserialize = "farthest"))]
    Farthest { weight: usize },
    /// Insertion with perturbation method.
    #[serde(rename(deserialize = "perturbation"))]
    Perturbation { weight: usize, probability: Float, min: Float, max: Float },
    /// Insertion with regret method.
    #[serde(rename(deserialize = "regret"))]
    Regret { weight: usize, start: usize, end: usize },
}

/// A local search configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum LocalOperatorType {
    #[serde(rename(deserialize = "swap-star"))]
    SwapStar { weight: usize },

    #[serde(rename(deserialize = "inter-route-best"))]
    InterRouteBest { weight: usize, noise: NoiseConfig },

    #[serde(rename(deserialize = "inter-route-random"))]
    InterRouteRandom { weight: usize, noise: NoiseConfig },

    #[serde(rename(deserialize = "intra-route-random"))]
    IntraRouteRandom { weight: usize, noise: NoiseConfig },

    #[serde(rename(deserialize = "sequence"))]
    Sequence { weight: usize },
}

#[derive(Clone, Deserialize, Debug)]
pub struct NoiseConfig {
    probability: Float,
    min: Float,
    max: Float,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TerminationConfig {
    pub max_time: Option<usize>,
    pub max_generations: Option<usize>,
    pub variation: Option<VariationConfig>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VariationConfig {
    interval_type: String,
    value: usize,
    cv: Float,
    is_global: bool,
}

/// A telemetry config.
#[derive(Clone, Deserialize, Debug)]
pub struct TelemetryConfig {
    progress: Option<ProgressConfig>,
    metrics: Option<MetricsConfig>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProgressConfig {
    /// Specifies whether logging is enabled. Default is false.
    enabled: bool,
    /// Specifies how often best individual is logged. Default is 100 (generations).
    log_best: Option<usize>,
    /// Specifies how often population is logged. Default is 1000 (generations).
    log_population: Option<usize>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetricsConfig {
    /// Specifies whether metrics collection is enabled. Default is false.
    enabled: bool,
    /// Specifies how often population is tracked. Default is 1000 (generations).
    track_population: Option<usize>,
}

/// An environment specific configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentConfig {
    /// Specifies a data parallelism configuration.
    pub parallelism: Option<ParallelismConfig>,

    /// Specifies a logging configuration.
    pub logging: Option<LoggingConfig>,

    /// Specifies experimental behavior flag.
    pub is_experimental: Option<bool>,
}

/// Data parallelism configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ParallelismConfig {
    /// Number of thread pools.
    pub num_thread_pools: usize,
    /// Specifies amount of threads in each thread pool.
    pub threads_per_pool: usize,
}

/// Global logging configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Specifies whether logging is enabled. Default is false.
    enabled: bool,
    /// Prefix of logging messages.
    prefix: Option<String>,
}

#[derive(Clone, Deserialize, Debug, Eq, PartialEq)]
pub struct MinMaxConfig {
    pub min: usize,
    pub max: usize,
}

#[derive(Clone, Deserialize, Debug, Eq, PartialEq)]
pub struct NameWeight {
    pub name: String,
    pub weight: usize,
}

/// Specifies output configuration.
#[derive(Clone, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputConfig {
    /// True if the solution, serialized as geojson features, should be included in solution.extras.
    pub include_geojson: Option<bool>,
}

fn configure_from_evolution(
    mut builder: ProblemConfigBuilder,
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    telemetry_mode: TelemetryMode,
    population_config: &Option<EvolutionConfig>,
) -> Result<ProblemConfigBuilder, GenericError> {
    if let Some(config) = population_config {
        if let Some(initial) = &config.initial {
            builder = builder.with_initial(
                initial.alternatives.max_size,
                initial.alternatives.quota,
                std::iter::once(create_recreate_method(&initial.method, environment.clone()))
                    .chain(
                        initial
                            .alternatives
                            .methods
                            .iter()
                            .map(|method| create_recreate_method(method, environment.clone())),
                    )
                    .map::<(
                        Box<
                            dyn InitialOperator<
                                    Context = RefinementContext,
                                    Objective = GoalContext,
                                    Solution = InsertionContext,
                                > + Send
                                + Sync,
                        >,
                        _,
                    ), _>(|(recreate, weight)| {
                        (Box::new(RecreateInitialOperator::new(recreate)), weight)
                    })
                    .collect(),
            );
        }

        if let Some(variation) = &config.population {
            let default_selection_size = get_default_selection_size(environment.as_ref());
            let population = match &variation {
                PopulationType::Greedy { selection_size } => Box::new(GreedyPopulation::new(
                    problem.goal.clone(),
                    selection_size.unwrap_or(default_selection_size),
                    None,
                )),
                PopulationType::Elitism { max_size, selection_size } => Box::new(ElitismPopulation::new(
                    problem.goal.clone(),
                    environment.random.clone(),
                    max_size.unwrap_or(4),
                    selection_size.unwrap_or(default_selection_size),
                )) as TargetPopulation,
                PopulationType::Rosomaxa {
                    max_elite_size,
                    max_node_size,
                    spread_factor,
                    distribution_factor,
                    selection_size,
                    rebalance_memory,
                    exploration_ratio,
                } => {
                    let mut config = RosomaxaConfig::new_with_defaults(default_selection_size);
                    if let Some(selection_size) = selection_size {
                        config.selection_size = *selection_size;
                    }
                    if let Some(max_elite_size) = max_elite_size {
                        config.elite_size = *max_elite_size;
                    }
                    if let Some(max_node_size) = max_node_size {
                        config.node_size = *max_node_size;
                    }
                    if let Some(spread_factor) = spread_factor {
                        config.spread_factor = *spread_factor;
                    }
                    if let Some(distribution_factor) = distribution_factor {
                        config.distribution_factor = *distribution_factor;
                    }
                    if let Some(rebalance_memory) = rebalance_memory {
                        config.rebalance_memory = *rebalance_memory;
                    }
                    if let Some(exploration_ratio) = exploration_ratio {
                        config.exploration_ratio = *exploration_ratio;
                    }

                    let footprint = Footprint::new(problem.as_ref());
                    Box::new(RosomaxaPopulation::new(footprint, problem.goal.clone(), environment.clone(), config)?)
                }
            };

            builder = builder.with_context(RefinementContext::new(problem, population, telemetry_mode, environment));
        }
    }

    Ok(builder)
}

fn configure_from_hyper(
    mut builder: ProblemConfigBuilder,
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    hyper_config: &Option<HyperType>,
) -> Result<ProblemConfigBuilder, GenericError> {
    if let Some(config) = hyper_config {
        match config {
            HyperType::StaticSelective { operators } => {
                let static_selective = if let Some(operators) = operators {
                    let heuristic_group = operators
                        .iter()
                        .map(|operator| create_operator(problem.clone(), environment.clone(), operator))
                        .collect::<Result<Vec<_>, _>>()?;
                    get_static_heuristic_from_heuristic_group(problem.clone(), environment.clone(), heuristic_group)
                } else {
                    get_static_heuristic(problem, environment)
                };

                builder = builder.with_heuristic(Box::new(static_selective));
            }
            HyperType::DynamicSelective => {
                let dynamic_selective = get_dynamic_heuristic(problem, environment);
                builder = builder.with_heuristic(Box::new(dynamic_selective));
            }
        }
    }

    Ok(builder)
}

fn configure_from_termination(
    mut builder: ProblemConfigBuilder,
    termination_config: &Option<TerminationConfig>,
) -> ProblemConfigBuilder {
    if let Some(config) = termination_config {
        builder = builder.with_max_time(config.max_time).with_max_generations(config.max_generations).with_min_cv(
            config.variation.as_ref().map(|v| (v.interval_type.clone(), v.value, v.cv, v.is_global)),
            "min_cv".to_string(),
        );
    }

    builder
}

fn create_recreate_method(method: &RecreateMethod, environment: Arc<Environment>) -> (Arc<dyn Recreate>, usize) {
    let random = environment.random.clone();
    match method {
        RecreateMethod::Cheapest { weight } => (Arc::new(RecreateWithCheapest::new(random)), *weight),
        RecreateMethod::Farthest { weight } => (Arc::new(RecreateWithFarthest::new(random)), *weight),
        RecreateMethod::SkipBest { weight, start, end } => {
            (Arc::new(RecreateWithSkipBest::new(*start, *end, random)), *weight)
        }
        RecreateMethod::Slice { weight } => (Arc::new(RecreateWithSlice::new(random)), *weight),
        RecreateMethod::Blinks { weight } => (Arc::new(RecreateWithBlinks::new_with_defaults(random.clone())), *weight),
        RecreateMethod::SkipRandom { weight } => (Arc::new(RecreateWithSkipRandom::new(random)), *weight),
        RecreateMethod::Gaps { weight, min, max } => (Arc::new(RecreateWithGaps::new(*min, *max, random)), *weight),
        RecreateMethod::Nearest { weight } => (Arc::new(RecreateWithNearestNeighbor::new(random)), *weight),
        RecreateMethod::Regret { weight, start, end } => {
            (Arc::new(RecreateWithRegret::new(*start, *end, random)), *weight)
        }
        RecreateMethod::Perturbation { weight, probability, min, max } => {
            let noise = Noise::new_with_addition(*probability, (*min, *max), random.clone());
            (Arc::new(RecreateWithPerturbation::new(noise, random.clone())), *weight)
        }
    }
}

fn create_operator(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    operator: &SearchOperatorType,
) -> Result<(TargetSearchOperator, TargetHeuristicProbability), GenericError> {
    Ok(match operator {
        SearchOperatorType::RuinRecreate { probability, ruins, recreates } => {
            let ruins = ruins.iter().map(|g| create_ruin_group(&problem, g)).collect::<Vec<_>>();

            let ruin = Arc::new(WeightedRuin::new(ruins));
            let recreate = Arc::new(WeightedRecreate::new(
                recreates.iter().map(|r| create_recreate_method(r, environment.clone())).collect(),
            ));
            (
                Arc::new(RuinAndRecreate::new(ruin, recreate)),
                create_operator_probability(probability, environment.random.clone()),
            )
        }
        SearchOperatorType::LocalSearch { probability, times, operators: inners } => {
            let operator = create_local_search(times, inners, environment.random.clone());
            (Arc::new(LocalSearch::new(operator)), create_operator_probability(probability, environment.random.clone()))
        }
        SearchOperatorType::Decomposition { routes, repeat, probability } => {
            if *repeat < 1 {
                return Err(format!("repeat must be greater than 1. Specified: {repeat}").into());
            }
            if routes.min < 2 {
                return Err(format!("min routes must be greater than 2. Specified: {}", routes.min).into());
            }

            let operator = create_default_heuristic_operator(problem, environment.clone());
            (
                Arc::new(DecomposeSearch::new(operator, (routes.min, routes.max), *repeat, 200)),
                create_operator_probability(probability, environment.random.clone()),
            )
        }
    })
}

fn create_operator_probability(
    probability: &OperatorProbabilityType,
    random: Arc<dyn Random>,
) -> TargetHeuristicProbability {
    match probability {
        OperatorProbabilityType::Scalar { scalar } => create_scalar_operator_probability(*scalar, random),
        OperatorProbabilityType::Context { threshold, phases } => create_context_operator_probability(
            threshold.jobs,
            threshold.routes,
            phases
                .iter()
                .map(|phase| match phase {
                    ContextPhase::Initial { chance } => (SelectionPhase::Initial, *chance),
                    ContextPhase::Exploration { chance } => (SelectionPhase::Exploration, *chance),
                    ContextPhase::Exploitation { chance } => (SelectionPhase::Exploitation, *chance),
                })
                .collect(),
            random,
        ),
    }
}

fn create_ruin_group(problem: &Arc<Problem>, group: &RuinGroupConfig) -> (Arc<dyn Ruin>, usize) {
    (Arc::new(CompositeRuin::new(group.methods.iter().map(|r| create_ruin_method(problem, r)).collect())), group.weight)
}

fn create_ruin_method(problem: &Arc<Problem>, method: &RuinMethod) -> (Arc<dyn Ruin>, Float) {
    let limits = RemovalLimits::new(problem.as_ref());
    let get_limits = |min: usize, max: usize| RemovalLimits {
        removed_activities_range: min..max,
        ..RemovalLimits::new(problem.as_ref())
    };

    match method {
        RuinMethod::AdjustedString { probability, lmax, cavg, alpha } => {
            (Arc::new(AdjustedStringRemoval::new(*lmax, *cavg, *alpha, limits)), *probability)
        }
        RuinMethod::Neighbour { probability, min, max } => {
            (Arc::new(NeighbourRemoval::new(get_limits(*min, *max))), *probability)
        }
        RuinMethod::RandomJob { probability, min, max } => {
            (Arc::new(RandomJobRemoval::new(get_limits(*min, *max))), *probability)
        }
        RuinMethod::RandomRoute { probability, min, max } => {
            (Arc::new(RandomRouteRemoval::new(get_limits(*min, *max))), *probability)
        }
        RuinMethod::WorstJob { probability, min, max, skip: worst_skip } => {
            (Arc::new(WorstJobRemoval::new(*worst_skip, get_limits(*min, *max))), *probability)
        }
        RuinMethod::Cluster { probability, min, max } => (
            // TODO: remove unwrap
            Arc::new(ClusterRemoval::new(problem.clone(), get_limits(*min, *max)).unwrap()),
            *probability,
        ),
        RuinMethod::CloseRoute { probability } => (Arc::new(CloseRouteRemoval::new(limits)), *probability),
        RuinMethod::WorstRoute { probability } => (Arc::new(WorstRouteRemoval::new(limits)), *probability),
    }
}

fn create_local_search(
    times: &MinMaxConfig,
    inners: &[LocalOperatorType],
    random: Arc<dyn Random>,
) -> Arc<dyn LocalOperator> {
    let operators = inners
        .iter()
        .map::<(Arc<dyn LocalOperator>, usize), _>(|op| match op {
            LocalOperatorType::SwapStar { weight } => (Arc::new(ExchangeSwapStar::new(random.clone(), 200)), *weight),
            LocalOperatorType::InterRouteBest { weight, noise } => {
                (Arc::new(ExchangeInterRouteBest::new(noise.probability, noise.min, noise.max)), *weight)
            }
            LocalOperatorType::InterRouteRandom { weight, noise } => {
                (Arc::new(ExchangeInterRouteRandom::new(noise.probability, noise.min, noise.max)), *weight)
            }
            LocalOperatorType::IntraRouteRandom { weight, noise } => {
                (Arc::new(ExchangeIntraRouteRandom::new(noise.probability, noise.min, noise.max)), *weight)
            }
            LocalOperatorType::Sequence { weight } => (Arc::new(ExchangeSequence::default()), *weight),
        })
        .collect::<Vec<_>>();

    Arc::new(CompositeLocalOperator::new(operators, times.min, times.max))
}

fn get_telemetry_mode(environment: Arc<Environment>, telemetry_config: &Option<TelemetryConfig>) -> TelemetryMode {
    const LOG_BEST: usize = 100;
    const LOG_POPULATION: usize = 1000;
    const TRACK_POPULATION: usize = 1000;

    let create_metrics = |track_population: &Option<usize>| TelemetryMode::OnlyMetrics {
        track_population: track_population.unwrap_or(TRACK_POPULATION),
    };

    let create_progress = |log_best: &Option<usize>, log_population: &Option<usize>| TelemetryMode::OnlyLogging {
        logger: environment.logger.clone(),
        log_best: log_best.unwrap_or(LOG_BEST),
        log_population: log_population.unwrap_or(LOG_POPULATION),
    };

    match telemetry_config.as_ref().map(|t| (&t.progress, &t.metrics)) {
        Some((None, Some(MetricsConfig { enabled, track_population }))) if *enabled => create_metrics(track_population),
        Some((Some(ProgressConfig { enabled, log_best, log_population }), None)) if *enabled => {
            create_progress(log_best, log_population)
        }
        Some((
            Some(ProgressConfig { enabled: progress_enabled, log_best, log_population }),
            Some(MetricsConfig { enabled: metrics_enabled, track_population }),
        )) => match (progress_enabled, metrics_enabled) {
            (true, true) => TelemetryMode::All {
                logger: environment.logger.clone(),
                log_best: log_best.unwrap_or(LOG_BEST),
                log_population: log_population.unwrap_or(LOG_POPULATION),
                track_population: track_population.unwrap_or(TRACK_POPULATION),
            },
            (true, false) => create_progress(log_best, log_population),
            (false, true) => create_metrics(track_population),
            _ => TelemetryMode::None,
        },
        _ => TelemetryMode::None,
    }
}

fn configure_from_environment(
    environment_config: &Option<EnvironmentConfig>,
    max_time: Option<usize>,
) -> Arc<Environment> {
    let mut environment = Environment::new_with_time_quota(max_time);

    if let Some(parallelism) = environment_config.as_ref().and_then(|c| c.parallelism.as_ref()) {
        // TODO validate parameters
        environment.parallelism = Parallelism::new(parallelism.num_thread_pools, parallelism.threads_per_pool);
    }

    if let Some(logging) = environment_config.as_ref().and_then(|c| c.logging.as_ref()) {
        environment.logger = match (logging.enabled, logging.prefix.clone()) {
            (true, Some(prefix)) => Arc::new(move |msg: &str| println!("{prefix}{msg}")),
            (true, None) => Arc::new(|msg: &str| println!("{msg}")),
            _ => Arc::new(|_: &str| {}),
        };
    }

    if let Some(is_experimental) = environment_config.as_ref().and_then(|c| c.is_experimental) {
        environment.is_experimental = is_experimental;
    }

    Arc::new(environment)
}

/// Reads config from reader.
pub fn read_config<R: Read>(reader: BufReader<R>) -> GenericResult<Config> {
    serde_json::from_reader(reader).map_err(|err| format!("cannot deserialize config: '{err}'").into())
}

/// Creates a solver `Builder` from config file.
pub fn create_builder_from_config_file<R>(
    problem: Arc<Problem>,
    solutions: Vec<InsertionContext>,
    reader: BufReader<R>,
) -> GenericResult<ProblemConfigBuilder>
where
    R: Read,
{
    read_config(reader).and_then(|config| create_builder_from_config(problem, solutions, &config))
}

/// Creates a solver `Builder` from config.
pub fn create_builder_from_config(
    problem: Arc<Problem>,
    solutions: Vec<InsertionContext>,
    config: &Config,
) -> GenericResult<ProblemConfigBuilder> {
    let environment =
        configure_from_environment(&config.environment, config.termination.as_ref().and_then(|t| t.max_time));
    let telemetry_mode = get_telemetry_mode(environment.clone(), &config.telemetry);
    let mut builder = VrpConfigBuilder::new(problem.clone())
        .set_environment(environment.clone())
        .set_telemetry_mode(telemetry_mode.clone())
        .prebuild()?
        .with_init_solutions(solutions, None);

    builder =
        configure_from_evolution(builder, problem.clone(), environment.clone(), telemetry_mode, &config.evolution)?;
    builder = configure_from_hyper(builder, problem, environment, &config.hyper)?;
    builder = configure_from_termination(builder, &config.termination);

    Ok(builder)
}
