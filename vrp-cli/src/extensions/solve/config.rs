//! Solver configuration.

#![allow(missing_docs)]

#[cfg(test)]
#[path = "../../../tests/unit/extensions/solve/config_test.rs"]
mod config_test;

extern crate serde_json;

use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::common::SingleDimLoad;
use vrp_core::models::Problem;
use vrp_core::solver::mutation::*;
use vrp_core::solver::selection::NaiveSelection;
use vrp_core::solver::{Builder, Telemetry, TelemetryMode};

/// An algorithm configuration.
#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    /// Specifies population configuration.
    population: Option<PopulationConfig>,
    /// Specifies mutation operator configuration.
    selection: Option<SelectionConfig>,
    /// Specifies mutation operator configuration.
    mutation: Option<MutationConfig>,
    /// Specifies algorithm termination configuration.
    termination: Option<TerminationConfig>,
    /// Specifies telemetry configuration.
    telemetry: Option<TelemetryConfig>,
}

/// A population configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PopulationConfig {
    initial: Option<InitialConfig>,
    max_size: Option<usize>,
}

/// An initial solution configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitialConfig {
    pub size: Option<usize>,
    pub methods: Option<Vec<RecreateMethod>>,
}

/// A selection configuration.
#[derive(Clone, Deserialize, Debug)]
pub struct SelectionConfig {
    /// A name of used selection from the collection.
    name: String,
    /// Collection of available selection algorithms.
    collection: Vec<SelectionType>,
}

/// A selection operator configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SelectionType {
    #[serde(rename(deserialize = "naive"))]
    Naive {
        /// A name of selection operator.
        name: String,
        /// A size of offspring.
        offspring_size: usize,
    },
}

/// A mutation configuration.
#[derive(Clone, Deserialize, Debug)]
pub struct MutationConfig {
    /// A name of used mutation from the collection.
    name: String,
    /// Collection of available mutation metaheuristics.
    collection: Vec<MutationType>,
}

/// A mutation operator configuration.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MutationType {
    /// A naive branching metaheurstic settings.
    #[serde(rename(deserialize = "naive-branching"))]
    NaiveBranching {
        /// A name of metaheurisic instance.
        name: String,
        /// A name of inner metaheurisic instance.
        inner: String,
        /// Specifies branching chance settings.
        chance: BranchingChance,
        /// Specifies steepness parameter for acceptance probability.
        steepness: f64,
        /// Specifies generations range inside a branch.
        generations: MinMaxConfig,
    },

    /// A metaheuristic which is composition of other metaheuristics with their
    /// probability weights.
    #[serde(rename(deserialize = "weighted-composite"))]
    WeightedComposite {
        /// A name of metaheurisic instance.
        name: String,
        /// A collection of inner metaheuristics.
        inners: Vec<NameWeight>,
    },

    /// A ruin and recreate metaheuristic settings.
    #[serde(rename(deserialize = "ruin-recreate"))]
    RuinRecreate {
        /// A name of metaheurisic instance.
        name: String,
        /// Ruin methods.
        ruins: Vec<ConfigRuinGroup>,
        /// Recreate methods.
        recreates: Vec<RecreateMethod>,
    },
}

/// A ruin method configuration
#[derive(Clone, Deserialize, Debug)]
pub struct ConfigRuinGroup {
    methods: Vec<RuinMethod>,
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
    /// Regret insertion method.
    #[serde(rename(deserialize = "regret"))]
    Regret { weight: usize, start: usize, end: usize },
    #[serde(rename(deserialize = "blinks"))]
    /// Insertion with blinks method.
    Blinks { weight: usize },
    #[serde(rename(deserialize = "gaps"))]
    /// Insertion with gaps method.
    Gaps { weight: usize, min: usize },
    /// Nearest neighbour method.
    #[serde(rename(deserialize = "nearest"))]
    Nearest { weight: usize },
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TerminationConfig {
    max_time: Option<usize>,
    max_generations: Option<usize>,
    variation: Option<VariationConfig>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct VariationConfig {
    sample: usize,
    cv: f64,
}

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
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetricsConfig {
    /// Specifies whether metrics collection is enabled. Default is false.
    enabled: bool,
    /// Specifies how often population is tracked. Default is 1000 (generations).
    track_population: Option<usize>,
}

#[derive(Clone, Deserialize, Debug)]

pub struct BranchingConfig {
    pub chance: BranchingChance,
    pub steepness: f64,
    pub generations: MinMaxConfig,
}

#[derive(Clone, Deserialize, Debug)]
pub struct BranchingChance {
    pub normal: f64,
    pub intensive: f64,
    pub threshold: f64,
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

fn configure_from_population(
    mut builder: Builder,
    population_config: &Option<PopulationConfig>,
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

        if let Some(population_size) = &config.max_size {
            builder = builder.with_population_size(*population_size);
        }
    }

    Ok(builder)
}

fn configure_from_selection(
    mut builder: Builder,
    selection_config: &Option<SelectionConfig>,
) -> Result<Builder, String> {
    if let Some(config) = selection_config {
        let selections = config
            .collection
            .iter()
            .map(|item| match item {
                SelectionType::Naive { name, offspring_size } => (name, Arc::new(NaiveSelection::new(*offspring_size))),
            })
            .collect::<HashMap<_, _>>();

        let selection =
            selections.get(&config.name).cloned().ok_or_else(|| format!("cannot find {} selection", config.name))?;

        builder = builder.with_selection(selection);
    }

    Ok(builder)
}

type NamedMutations = HashMap<String, Arc<dyn Mutation + Send + Sync>>;

fn configure_from_mutation(mut builder: Builder, mutation_config: &Option<MutationConfig>) -> Result<Builder, String> {
    if let Some(config) = mutation_config {
        let get_mutation_by_name = |mutations: &NamedMutations, name: &String| {
            mutations
                .get(name)
                .cloned()
                .ok_or_else(|| format!("cannot find {} mutation, make sure that it is defined before used", name))
        };
        let mutations = config.collection.iter().try_fold::<_, _, Result<_, String>>(
            NamedMutations::default(),
            |mut mutations, type_cfg| {
                let (name, mutation): (_, Arc<dyn Mutation + Send + Sync>) = match type_cfg {
                    MutationType::RuinRecreate { name, ruins, recreates } => {
                        let ruin = Box::new(CompositeRuin::new(
                            ruins.iter().map(|g| create_ruin_group(&builder.config.problem, g)).collect(),
                        ));
                        let recreate = Box::new(CompositeRecreate::new(
                            recreates.iter().map(|r| create_recreate_method(r)).collect(),
                        ));
                        (name.clone(), Arc::new(RuinAndRecreate::new(recreate, ruin)))
                    }
                    MutationType::WeightedComposite { name, inners } => {
                        let inners = inners
                            .iter()
                            .map(|nw| get_mutation_by_name(&mutations, &nw.name).map(|mutation| (mutation, nw.weight)))
                            .collect::<Result<Vec<_>, _>>()?;
                        (name.clone(), Arc::new(WeightedComposite::new(inners)))
                    }
                    MutationType::NaiveBranching { name, inner, chance, steepness, generations } => (
                        name.clone(),
                        Arc::new(NaiveBranching::new(
                            get_mutation_by_name(&mutations, inner)?,
                            (chance.normal, chance.intensive, chance.threshold),
                            *steepness,
                            generations.min..generations.max,
                        )),
                    ),
                };

                mutations.insert(name, mutation);

                Ok(mutations)
            },
        )?;

        let mutation = get_mutation_by_name(&mutations, &config.name)?.clone();

        builder = builder.with_mutation(mutation);
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

fn create_recreate_method(method: &RecreateMethod) -> (Box<dyn Recreate + Send + Sync>, usize) {
    match method {
        RecreateMethod::Cheapest { weight } => (Box::new(RecreateWithCheapest::default()), *weight),
        RecreateMethod::Regret { weight, start, end } => (Box::new(RecreateWithRegret::new(*start, *end)), *weight),
        RecreateMethod::Blinks { weight } => (Box::new(RecreateWithBlinks::<SingleDimLoad>::default()), *weight),
        RecreateMethod::Gaps { weight, min } => (Box::new(RecreateWithGaps::new(*min)), *weight),
        RecreateMethod::Nearest { weight } => (Box::new(RecreateWithNearestNeighbor::default()), *weight),
    }
}

fn create_ruin_group(problem: &Arc<Problem>, group: &ConfigRuinGroup) -> RuinGroup {
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

fn configure_from_telemetry(builder: Builder, telemetry_config: &Option<TelemetryConfig>) -> Result<Builder, String> {
    const LOG_BEST: usize = 100;
    const LOG_POPULATION: usize = 1000;
    const TRACK_POPULATION: usize = 1000;

    let create_logger = || Arc::new(|msg: &str| println!("{}", msg));

    let create_metrics = |track_population: &Option<usize>| TelemetryMode::OnlyMetrics {
        track_population: track_population.unwrap_or(TRACK_POPULATION),
    };

    let create_logging = |log_best: &Option<usize>, log_population: &Option<usize>| TelemetryMode::OnlyLogging {
        logger: create_logger(),
        log_best: log_best.unwrap_or(LOG_BEST),
        log_population: log_population.unwrap_or(LOG_POPULATION),
    };

    let telemetry_mode = match telemetry_config.as_ref().map(|t| (&t.logging, &t.metrics)) {
        Some((None, Some(MetricsConfig { enabled, track_population }))) if *enabled => create_metrics(track_population),
        Some((Some(LoggingConfig { enabled, log_best, log_population }), None)) if *enabled => {
            create_logging(log_best, log_population)
        }
        Some((
            Some(LoggingConfig { enabled: logging_enabled, log_best, log_population }),
            Some(MetricsConfig { enabled: metrics_enabled, track_population }),
        )) => match (logging_enabled, metrics_enabled) {
            (true, true) => TelemetryMode::All {
                logger: create_logger(),
                log_best: log_best.unwrap_or(LOG_BEST),
                log_population: log_population.unwrap_or(LOG_POPULATION),
                track_population: track_population.unwrap_or(TRACK_POPULATION),
            },
            (true, false) => create_logging(log_best, log_population),
            (false, true) => create_metrics(track_population),
            _ => TelemetryMode::None,
        },
        _ => TelemetryMode::None,
    };

    Ok(builder.with_telemetry(Telemetry::new(telemetry_mode)))
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
    let mut builder = Builder::new(problem);

    builder = configure_from_telemetry(builder, &config.telemetry)?;
    builder = configure_from_population(builder, &config.population)?;
    builder = configure_from_selection(builder, &config.selection)?;
    builder = configure_from_mutation(builder, &config.mutation)?;
    builder = configure_from_termination(builder, &config.termination)?;

    Ok(builder)
}
