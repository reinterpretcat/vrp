//! Solver configuration.

#![allow(missing_docs)]

#[cfg(test)]
#[path = "../../../tests/unit/extensions/solve/config_test.rs"]
mod config_test;

extern crate serde_json;

use serde::Deserialize;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::common::SingleDimLoad;
use vrp_core::models::Problem;
use vrp_core::solver::hyper::*;
use vrp_core::solver::mutation::*;
use vrp_core::solver::population::*;
use vrp_core::solver::{Builder, Telemetry, TelemetryMode};
use vrp_core::utils::{Environment, Parallelism, Random};

/// An algorithm configuration.
#[derive(Clone, Deserialize, Debug)]
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
}

/// An evolution configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionConfig {
    initial: Option<InitialConfig>,
    population: Option<PopulationType>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum PopulationType {
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
        /// Node population size. Default is 2.
        max_node_size: Option<usize>,
        /// Spread factor. Default is 0.25.
        spread_factor: Option<f64>,
        /// The reduction factor. Default is 0.1.
        reduction_factor: Option<f64>,
        /// Distribution factor. Default is 0.25.
        distribution_factor: Option<f64>,
        /// Learning rate. Default is 0.1.
        learning_rate: Option<f64>,
        /// A rebalance memory. Default is 500.
        rebalance_memory: Option<usize>,
        /// A rebalance count. Default is 10.
        rebalance_count: Option<usize>,
        /// An exploration phase ratio. Default is 0.9.
        exploration_ratio: Option<f64>,
    },
}

/// An initial solution configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitialConfig {
    pub size: Option<usize>,
    pub methods: Option<Vec<RecreateMethod>>,
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
    /// A hyper heuristic which selects one mutation from the list based on its probability.
    #[serde(rename(deserialize = "static-selective"))]
    StaticSelective {
        /// A collection of inner mutation operators (metaheuristics).
        mutations: Vec<MutationType>,
    },
}

/// A mutation operator configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MutationType {
    /// A metaheuristic which splits problem into smaller and solves them independently.
    #[serde(rename(deserialize = "decomposition"))]
    #[serde(rename_all = "camelCase")]
    Decomposition {
        /// Max routes to be selected in decomposed solution.
        routes: MinMaxConfig,
        /// Amount of attempts to repeat refinement.
        repeat: usize,
        /// Probability of mutation.
        probability: MutationProbabilityType,
    },

    /// A local search heuristic.
    #[serde(rename(deserialize = "local-search"))]
    LocalSearch {
        /// Probability of mutation.
        probability: MutationProbabilityType,
        /// Amount of times one of operators is applied.
        times: MinMaxConfig,
        /// Local search operator.
        operators: Vec<LocalOperatorType>,
    },

    /// A ruin and recreate metaheuristic settings.
    #[serde(rename(deserialize = "ruin-recreate"))]
    RuinRecreate {
        /// Probability.
        probability: MutationProbabilityType,
        /// Ruin methods.
        ruins: Vec<RuinGroupConfig>,
        /// Recreate methods.
        recreates: Vec<RecreateMethod>,
    },
}

/// A mutation method probability type
#[derive(Clone, Deserialize, Debug)]
#[serde(untagged)]
pub enum MutationProbabilityType {
    /// A scalar probability based type.
    Scalar {
        /// Probability value of the mutation.
        scalar: f64,
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
        chance: f64,
    },

    /// Exploration search phase.
    #[serde(rename(deserialize = "exploration"))]
    Exploration {
        /// A chance defined by probability.
        chance: f64,
    },

    /// Exploitation search phase.
    #[serde(rename(deserialize = "exploitation"))]
    Exploitation {
        /// A chance defined by probability.
        chance: f64,
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
    AdjustedString { probability: f64, lmax: usize, cavg: usize, alpha: f64 },
    /// Neighbour jobs method
    #[serde(rename(deserialize = "neighbour"))]
    Neighbour { probability: f64, min: usize, max: usize, threshold: f64 },
    /// Random job removal method.
    #[serde(rename(deserialize = "random-job"))]
    RandomJob { probability: f64, min: usize, max: usize, threshold: f64 },
    /// Random route removal method.
    #[serde(rename(deserialize = "random-route"))]
    RandomRoute { probability: f64, min: usize, max: usize, threshold: f64 },
    /// Worst job removal method.
    #[serde(rename(deserialize = "worst-job"))]
    WorstJob { probability: f64, min: usize, max: usize, threshold: f64, skip: usize },
    /// Clustered jobs removal method.
    #[serde(rename(deserialize = "cluster"))]
    Cluster { probability: f64, min: usize, max: usize, threshold: f64, cmin: usize, cmax: usize },
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
    #[serde(rename(deserialize = "blinks"))]
    /// Insertion with blinks method.
    Blinks { weight: usize },
    #[serde(rename(deserialize = "gaps"))]
    /// Insertion with gaps method.
    Gaps { weight: usize, min: usize },
    /// Nearest neighbour method.
    #[serde(rename(deserialize = "nearest"))]
    Nearest { weight: usize },
    /// Farthest insertion method.
    #[serde(rename(deserialize = "farthest"))]
    Farthest { weight: usize },
    #[serde(rename(deserialize = "perturbation"))]
    Perturbation { weight: usize, probability: f64, min: f64, max: f64 },
    #[serde(rename(deserialize = "regret"))]
    Regret { weight: usize, start: usize, end: usize },
}

/// A local search configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum LocalOperatorType {
    #[serde(rename(deserialize = "inter-route-best"))]
    InterRouteBest { weight: usize, noise: NoiseConfig },

    #[serde(rename(deserialize = "inter-route-random"))]
    InterRouteRandom { weight: usize, noise: NoiseConfig },

    #[serde(rename(deserialize = "intra-route-random"))]
    IntraRouteRandom { weight: usize, noise: NoiseConfig },
}

#[derive(Clone, Deserialize, Debug)]
pub struct NoiseConfig {
    probability: f64,
    min: f64,
    max: f64,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TerminationConfig {
    pub max_time: Option<usize>,
    pub max_generations: Option<usize>,
    pub variation: Option<VariationConfig>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct VariationConfig {
    sample: usize,
    cv: f64,
}

/// A telemetry config.
#[derive(Clone, Deserialize, Debug)]
pub struct TelemetryConfig {
    logging: Option<LoggingConfig>,
    metrics: Option<MetricsConfig>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Specifies whether logging is enabled. Default is false.
    enabled: bool,
    /// Specifies how often best individual is logged. Default is 100 (generations).
    log_best: Option<usize>,
    /// Specifies how often population is logged. Default is 1000 (generations).
    log_population: Option<usize>,
    /// Specifies whether population should be dumped.
    dump_population: Option<bool>,
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
pub struct EnvironmentConfig {
    /// Specifies a data parallelism configuration.
    pub parallelism: Option<ParallelismConfig>,
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

impl Default for Config {
    fn default() -> Self {
        Self { evolution: None, hyper: None, termination: None, environment: None, telemetry: None }
    }
}

fn configure_from_evolution(
    mut builder: Builder,
    population_config: &Option<EvolutionConfig>,
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> Result<Builder, String> {
    if let Some(config) = population_config {
        if let Some(initial) = &config.initial {
            builder = builder.with_init_params(
                initial.size,
                initial
                    .methods
                    .as_ref()
                    .map(|methods| methods.iter().map(|method| create_recreate_method(method)).collect()),
            );
        }

        if let Some(variation) = &config.population {
            let default_selection_size = get_default_selection_size(environment.as_ref());
            let population = match &variation {
                PopulationType::Elitism { max_size, selection_size } => Box::new(Elitism::new(
                    problem,
                    environment.random.clone(),
                    max_size.unwrap_or(4),
                    selection_size.unwrap_or(default_selection_size),
                ))
                    as Box<dyn Population + Send + Sync>,
                PopulationType::Rosomaxa {
                    max_elite_size,
                    max_node_size,
                    spread_factor,
                    reduction_factor,
                    distribution_factor,
                    learning_rate,
                    selection_size,
                    rebalance_memory,
                    rebalance_count,
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
                    if let Some(reduction_factor) = reduction_factor {
                        config.reduction_factor = *reduction_factor;
                    }
                    if let Some(distribution_factor) = distribution_factor {
                        config.distribution_factor = *distribution_factor;
                    }
                    if let Some(learning_rate) = learning_rate {
                        config.learning_rate = *learning_rate;
                    }
                    if let Some(rebalance_memory) = rebalance_memory {
                        config.rebalance_memory = *rebalance_memory;
                    }
                    if let Some(rebalance_count) = rebalance_count {
                        config.rebalance_count = *rebalance_count;
                    }
                    if let Some(exploration_ratio) = exploration_ratio {
                        config.exploration_ratio = *exploration_ratio;
                    }

                    Box::new(Rosomaxa::new(problem, environment, config)?)
                }
            };

            builder = builder.with_population(population);
        }
    }

    Ok(builder)
}

fn configure_from_hyper(
    mut builder: Builder,
    hyper_config: &Option<HyperType>,
    environment: Arc<Environment>,
) -> Result<Builder, String> {
    if let Some(config) = hyper_config {
        match config {
            HyperType::StaticSelective { mutations } => {
                let mutation_group = mutations
                    .iter()
                    .map(|mutation| create_mutation(&builder.config.problem, environment.random.clone(), mutation))
                    .collect::<Result<Vec<_>, _>>()?;
                let static_selective = vrp_core::solver::hyper::StaticSelective::new(mutation_group);
                builder = builder.with_hyper(Box::new(static_selective));
            }
        }
    }

    Ok(builder)
}

fn configure_from_termination(
    mut builder: Builder,
    termination_config: &Option<TerminationConfig>,
) -> Result<Builder, String> {
    if let Some(config) = termination_config {
        builder = builder.with_max_time(config.max_time);
        builder = builder.with_max_generations(config.max_generations);
        builder = builder.with_cost_variation(config.variation.as_ref().map(|v| (v.sample, v.cv)));
    }

    Ok(builder)
}

fn create_recreate_method(method: &RecreateMethod) -> (Arc<dyn Recreate + Send + Sync>, usize) {
    match method {
        RecreateMethod::Cheapest { weight } => (Arc::new(RecreateWithCheapest::default()), *weight),
        RecreateMethod::Farthest { weight } => (Arc::new(RecreateWithFarthest::default()), *weight),
        RecreateMethod::SkipBest { weight, start, end } => (Arc::new(RecreateWithSkipBest::new(*start, *end)), *weight),
        RecreateMethod::Blinks { weight } => (Arc::new(RecreateWithBlinks::<SingleDimLoad>::default()), *weight),
        RecreateMethod::Gaps { weight, min } => (Arc::new(RecreateWithGaps::new(*min)), *weight),
        RecreateMethod::Nearest { weight } => (Arc::new(RecreateWithNearestNeighbor::default()), *weight),
        RecreateMethod::Regret { weight, start, end } => (Arc::new(RecreateWithRegret::new(*start, *end)), *weight),
        RecreateMethod::Perturbation { weight, probability, min, max } => {
            (Arc::new(RecreateWithPerturbation::new(*probability, *min, *max)), *weight)
        }
    }
}

fn create_mutation(
    problem: &Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
    mutation: &MutationType,
) -> Result<(Arc<dyn Mutation + Send + Sync>, MutationProbability), String> {
    Ok(match mutation {
        MutationType::RuinRecreate { probability, ruins, recreates } => {
            let ruin = Arc::new(CompositeRuin::new(ruins.iter().map(|g| create_ruin_group(problem, g)).collect()));
            let recreate =
                Arc::new(CompositeRecreate::new(recreates.iter().map(|r| create_recreate_method(r)).collect()));
            (Arc::new(RuinAndRecreate::new(ruin, recreate)), create_mutation_probability(probability, random.clone()))
        }
        MutationType::LocalSearch { probability, times, operators: inners } => {
            let operator = create_local_search(times, inners);
            (Arc::new(LocalSearch::new(operator)), create_mutation_probability(probability, random.clone()))
        }
        MutationType::Decomposition { routes, repeat, probability } => {
            if *repeat < 1 {
                return Err(format!("repeat must be greater than 1. Specified: {}", repeat));
            }
            if routes.min < 2 {
                return Err(format!("min routes must be greater than 2. Specified: {}", routes.min));
            }

            let mutation = vrp_core::solver::hyper::StaticSelective::create_default_mutation(problem.clone());
            (
                Arc::new(DecomposeSearch::new(mutation, (routes.min, routes.max), *repeat)),
                create_mutation_probability(probability, random.clone()),
            )
        }
    })
}

fn create_mutation_probability(
    probability: &MutationProbabilityType,
    random: Arc<dyn Random + Send + Sync>,
) -> MutationProbability {
    match probability {
        MutationProbabilityType::Scalar { scalar } => create_scalar_mutation_probability(*scalar, random),
        MutationProbabilityType::Context { threshold, phases } => create_context_mutation_probability(
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

fn create_ruin_group(problem: &Arc<Problem>, group: &RuinGroupConfig) -> RuinGroup {
    (group.methods.iter().map(|r| create_ruin_method(problem, r)).collect(), group.weight)
}

fn create_ruin_method(problem: &Arc<Problem>, method: &RuinMethod) -> (Arc<dyn Ruin + Send + Sync>, f64) {
    match method {
        RuinMethod::AdjustedString { probability, lmax, cavg, alpha } => {
            (Arc::new(AdjustedStringRemoval::new(*lmax, *cavg, *alpha)), *probability)
        }
        RuinMethod::Neighbour { probability, min, max, threshold } => {
            (Arc::new(NeighbourRemoval::new(JobRemovalLimit::new(*min, *max, *threshold))), *probability)
        }
        RuinMethod::RandomJob { probability, min, max, threshold } => {
            (Arc::new(RandomJobRemoval::new(JobRemovalLimit::new(*min, *max, *threshold))), *probability)
        }
        RuinMethod::RandomRoute { probability, min, max, threshold } => {
            (Arc::new(RandomRouteRemoval::new(*min, *max, *threshold)), *probability)
        }
        RuinMethod::WorstJob { probability, min, max, threshold, skip: worst_skip } => {
            (Arc::new(WorstJobRemoval::new(*worst_skip, JobRemovalLimit::new(*min, *max, *threshold))), *probability)
        }
        RuinMethod::Cluster { probability, min, max, threshold, cmin, cmax } => (
            Arc::new(ClusterRemoval::new(problem.clone(), *cmin..*cmax, JobRemovalLimit::new(*min, *max, *threshold))),
            *probability,
        ),
    }
}

fn create_local_search(times: &MinMaxConfig, inners: &[LocalOperatorType]) -> Arc<dyn LocalOperator + Send + Sync> {
    let operators = inners
        .iter()
        .map::<(Arc<dyn LocalOperator + Send + Sync>, usize), _>(|op| match op {
            LocalOperatorType::InterRouteBest { weight, noise } => {
                (Arc::new(ExchangeInterRouteBest::new(noise.probability, noise.min, noise.max)), *weight)
            }
            LocalOperatorType::InterRouteRandom { weight, noise } => {
                (Arc::new(ExchangeInterRouteRandom::new(noise.probability, noise.min, noise.max)), *weight)
            }
            LocalOperatorType::IntraRouteRandom { weight, noise } => {
                (Arc::new(ExchangeIntraRouteRandom::new(noise.probability, noise.min, noise.max)), *weight)
            }
        })
        .collect::<Vec<_>>();

    Arc::new(CompositeLocalOperator::new(operators, times.min, times.max))
}

fn configure_from_telemetry(builder: Builder, telemetry_config: &Option<TelemetryConfig>) -> Result<Builder, String> {
    const LOG_BEST: usize = 100;
    const LOG_POPULATION: usize = 1000;
    const TRACK_POPULATION: usize = 1000;

    let create_logger = || Arc::new(|msg: &str| println!("{}", msg));

    let create_metrics = |track_population: &Option<usize>| TelemetryMode::OnlyMetrics {
        track_population: track_population.unwrap_or(TRACK_POPULATION),
    };

    let create_logging = |log_best: &Option<usize>, log_population: &Option<usize>, dump_population: &Option<bool>| {
        TelemetryMode::OnlyLogging {
            logger: create_logger(),
            log_best: log_best.unwrap_or(LOG_BEST),
            log_population: log_population.unwrap_or(LOG_POPULATION),
            dump_population: dump_population.unwrap_or(false),
        }
    };

    let telemetry_mode = match telemetry_config.as_ref().map(|t| (&t.logging, &t.metrics)) {
        Some((None, Some(MetricsConfig { enabled, track_population }))) if *enabled => create_metrics(track_population),
        Some((Some(LoggingConfig { enabled, log_best, log_population, dump_population }), None)) if *enabled => {
            create_logging(log_best, log_population, dump_population)
        }
        Some((
            Some(LoggingConfig { enabled: logging_enabled, log_best, log_population, dump_population }),
            Some(MetricsConfig { enabled: metrics_enabled, track_population }),
        )) => match (logging_enabled, metrics_enabled) {
            (true, true) => TelemetryMode::All {
                logger: create_logger(),
                log_best: log_best.unwrap_or(LOG_BEST),
                log_population: log_population.unwrap_or(LOG_POPULATION),
                track_population: track_population.unwrap_or(TRACK_POPULATION),
                dump_population: dump_population.unwrap_or(false),
            },
            (true, false) => create_logging(log_best, log_population, dump_population),
            (false, true) => create_metrics(track_population),
            _ => TelemetryMode::None,
        },
        _ => TelemetryMode::None,
    };

    Ok(builder.with_telemetry(Telemetry::new(telemetry_mode)))
}

fn configure_from_environment(environment_config: &Option<EnvironmentConfig>) -> Result<Arc<Environment>, String> {
    let mut environment = Environment::default();

    // TODO validate parameters
    if let Some(config) = environment_config.as_ref().and_then(|c| c.parallelism.as_ref()) {
        environment.parallelism = Parallelism::new(config.num_thread_pools, config.threads_per_pool);
    }

    Ok(Arc::new(environment))
}

/// Reads config from reader.
pub fn read_config<R: Read>(reader: BufReader<R>) -> Result<Config, String> {
    serde_json::from_reader(reader).map_err(|err| format!("cannot deserialize config: '{}'", err))
}

/// Creates a solver `Builder` from config file.
pub fn create_builder_from_config_file<R: Read>(
    problem: Arc<Problem>,
    reader: BufReader<R>,
) -> Result<Builder, String> {
    read_config(reader).and_then(|config| create_builder_from_config(problem, &config))
}

/// Creates a solver `Builder` from config.
pub fn create_builder_from_config(problem: Arc<Problem>, config: &Config) -> Result<Builder, String> {
    let environment = configure_from_environment(&config.environment)?;
    let mut builder = Builder::new(problem.clone(), environment.clone());

    builder = configure_from_telemetry(builder, &config.telemetry)?;
    builder = configure_from_evolution(builder, &config.evolution, problem, environment.clone())?;
    builder = configure_from_hyper(builder, &config.hyper, environment)?;
    builder = configure_from_termination(builder, &config.termination)?;

    Ok(builder)
}
