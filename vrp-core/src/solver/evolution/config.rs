use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::Problem;
use crate::solver::evolution::{EvolutionStrategy, RunSimple};
use crate::solver::mutation::*;
use crate::solver::population::*;
use crate::solver::telemetry::Telemetry;
use crate::solver::termination::*;
use crate::solver::TelemetryMode;
use crate::utils::{DefaultRandom, Random};
use std::sync::Arc;

/// A configuration which controls evolution execution.
pub struct EvolutionConfig {
    /// An original problem.
    pub problem: Arc<Problem>,

    /// A population configuration
    pub population: PopulationConfig,

    /// A mutation applied to population.
    pub mutation: Arc<dyn Mutation + Send + Sync>,

    /// A termination defines when evolution should stop.
    pub termination: Arc<dyn Termination + Send + Sync>,

    /// An evolution strategy.
    pub strategy: Arc<dyn EvolutionStrategy + Send + Sync>,

    /// A quota for evolution execution.
    pub quota: Option<Arc<dyn Quota + Send + Sync>>,

    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,

    /// A telemetry to be used.
    pub telemetry: Telemetry,
}

/// Contains population specific properties.
pub struct PopulationConfig {
    /// An initial solution config.
    pub initial: InitialConfig,

    /// Population algorithm variation.
    pub variation: Option<Box<dyn Population + Send + Sync>>,
}

/// An initial solutions configuration.
pub struct InitialConfig {
    /// Initial size of population to be generated.
    pub size: usize,

    /// Create methods to produce initial individuals.
    pub methods: Vec<(Box<dyn Recreate + Send + Sync>, usize)>,

    /// Initial individuals in population.
    pub individuals: Vec<InsertionContext>,
}

impl EvolutionConfig {
    /// Creates a new instance of `EvolutionConfig` using default settings.
    pub fn new(problem: Arc<Problem>) -> Self {
        let random = Arc::new(DefaultRandom::default());
        Self {
            problem: problem.clone(),
            population: PopulationConfig {
                initial: InitialConfig {
                    size: 1,
                    methods: vec![(Box::new(RecreateWithCheapest::default()), 10)],
                    individuals: vec![],
                },
                variation: Some(Rosomaxa::new_with_fallback(
                    problem.clone(),
                    random.clone(),
                    RosomaxaConfig::default(),
                )),
            },
            mutation: Arc::new(CompositeMutation::new(vec![(
                vec![
                    (Arc::new(LocalSearch::new(Box::new(CompositeLocalOperator::default()))), 0.05),
                    (Arc::new(RuinAndRecreate::new_from_problem(problem)), 1.),
                    (Arc::new(LocalSearch::new(Box::new(CompositeLocalOperator::default()))), 0.01),
                ],
                1,
            )])),
            termination: Arc::new(CompositeTermination::new(vec![
                Box::new(MaxTime::new(300.)),
                Box::new(MaxGeneration::new(3000)),
            ])),
            strategy: Arc::new(RunSimple::default()),
            quota: None,
            random,
            telemetry: Telemetry::new(TelemetryMode::None),
        }
    }
}
