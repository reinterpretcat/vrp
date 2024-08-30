use crate::evolution::*;
use crate::hyper::*;
use crate::termination::*;
use std::hash::Hash;
use std::sync::Arc;

/// A configuration which controls evolution execution.
pub struct EvolutionConfig<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// An initial solution config.
    pub initial: InitialConfig<C, O, S>,

    /// A pre/post processing config.
    pub processing: ProcessingConfig<C, O, S>,

    /// A heuristic context.
    pub context: C,

    /// An evolution strategy.
    pub strategy: Box<dyn EvolutionStrategy<Context = C, Objective = O, Solution = S>>,

    /// A termination defines when evolution should stop.
    pub termination: Box<dyn Termination<Context = C, Objective = O>>,
}

/// Specifies an operator which builds initial solution.
pub trait InitialOperator {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Creates an initial solution from scratch.
    fn create(&self, heuristic_ctx: &Self::Context) -> Self::Solution;
}

/// A collection of initial operators.
pub type InitialOperators<C, O, S> =
    Vec<(Box<dyn InitialOperator<Context = C, Objective = O, Solution = S> + Send + Sync>, usize)>;

/// An initial solutions configuration.
pub struct InitialConfig<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Create methods to produce initial individuals.
    pub operators: InitialOperators<C, O, S>,
    /// Initial size of population to be generated.
    pub max_size: usize,
    /// Quota for initial solution generation.
    pub quota: Float,
    /// Initial individuals in population.
    pub individuals: Vec<S>,
}

/// Specifies pre/post processing logic which is run before and after the solver.
pub struct ProcessingConfig<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// A heuristic context creating pre processing.
    pub context: Vec<Box<dyn HeuristicContextProcessing<Context = C, Objective = O, Solution = S> + Send + Sync>>,
    /// A solution post processing.
    pub solution: Vec<Box<dyn HeuristicSolutionProcessing<Solution = S> + Send + Sync>>,
}

/// Provides configurable way to build evolution configuration using fluent interface style.
pub struct EvolutionConfigBuilder<C, O, S, K>
where
    C: HeuristicContext<Objective = O, Solution = S> + Stateful<Key = K> + 'static,
    O: HeuristicObjective<Solution = S> + 'static,
    S: HeuristicSolution + 'static,
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    max_generations: Option<usize>,
    max_time: Option<usize>,
    min_cv: Option<(String, usize, Float, bool, K)>,
    target_proximity: Option<(Vec<Float>, Float)>,
    heuristic: Option<Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>>,
    context: Option<C>,
    termination: Option<Box<dyn Termination<Context = C, Objective = O>>>,
    strategy: Option<Box<dyn EvolutionStrategy<Context = C, Objective = O, Solution = S>>>,

    search_operators: Option<HeuristicSearchOperators<C, O, S>>,
    diversify_operators: Option<HeuristicDiversifyOperators<C, O, S>>,

    objective: Option<Arc<dyn HeuristicObjective<Solution = S>>>,

    initial: InitialConfig<C, O, S>,
    processing: ProcessingConfig<C, O, S>,
}

impl<C, O, S, K> Default for EvolutionConfigBuilder<C, O, S, K>
where
    C: HeuristicContext<Objective = O, Solution = S> + Stateful<Key = K> + 'static,
    O: HeuristicObjective<Solution = S> + 'static,
    S: HeuristicSolution + 'static,
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            max_generations: None,
            max_time: None,
            min_cv: None,
            target_proximity: None,
            heuristic: None,
            context: None,
            termination: None,
            strategy: None,
            search_operators: None,
            diversify_operators: None,
            objective: None,
            initial: InitialConfig { operators: vec![], max_size: 4, quota: 0.05, individuals: vec![] },
            processing: ProcessingConfig { context: vec![], solution: vec![] },
        }
    }
}

impl<C, O, S, K> EvolutionConfigBuilder<C, O, S, K>
where
    C: HeuristicContext<Objective = O, Solution = S> + Stateful<Key = K> + 'static,
    O: HeuristicObjective<Solution = S> + 'static,
    S: HeuristicSolution + 'static,
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    /// Sets max generations to be run by evolution. Default is 3000.
    pub fn with_max_generations(mut self, limit: Option<usize>) -> Self {
        self.max_generations = limit;
        self
    }

    /// Sets max running time limit for evolution. Default is 300 seconds.
    pub fn with_max_time(mut self, limit: Option<usize>) -> Self {
        self.max_time = limit;
        self
    }

    /// Sets variation coefficient termination criteria. Default is None.
    pub fn with_min_cv(mut self, min_cv: Option<(String, usize, Float, bool)>, key: K) -> Self {
        self.min_cv = min_cv.map(|min_cv| (min_cv.0, min_cv.1, min_cv.2, min_cv.3, key));
        self
    }

    /// Sets target fitness and distance threshold as termination criteria.
    pub fn with_target_proximity(mut self, target_proximity: Option<(Vec<Float>, Float)>) -> Self {
        self.target_proximity = target_proximity;
        self
    }

    /// Sets initial parameters used to construct initial population.
    pub fn with_initial(mut self, max_size: usize, quota: Float, operators: InitialOperators<C, O, S>) -> Self {
        self.initial.max_size = max_size;
        self.initial.quota = quota;
        self.initial.operators = operators;

        self
    }

    /// Specifies processing configuration.
    pub fn with_processing(mut self, processing: ProcessingConfig<C, O, S>) -> Self {
        self.processing = processing;
        self
    }

    /// Sets initial solutions in population. Default is no solutions in population.
    pub fn with_init_solutions(mut self, solutions: Vec<S>, max_init_size: Option<usize>) -> Self {
        if let Some(max_size) = max_init_size {
            self.initial.max_size = max_size;
        }
        self.initial.individuals = solutions;

        self
    }

    /// Sets objective.
    pub fn with_objective(mut self, objective: Arc<dyn HeuristicObjective<Solution = S>>) -> Self {
        self.objective = Some(objective);
        self
    }

    /// Sets heuristic context.
    pub fn with_context(mut self, context: C) -> Self {
        self.context = Some(context);
        self
    }

    /// Sets termination.
    pub fn with_termination(mut self, termination: Box<dyn Termination<Context = C, Objective = O>>) -> Self {
        self.termination = Some(termination);
        self
    }

    /// Sets a different heuristic replacing initial.
    pub fn with_heuristic(
        mut self,
        heuristic: Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>,
    ) -> Self {
        self.heuristic = Some(heuristic);
        self
    }

    /// Sets a different heuristic replacing initial.
    pub fn with_strategy(
        mut self,
        strategy: Box<dyn EvolutionStrategy<Context = C, Objective = O, Solution = S>>,
    ) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Sets search operators for dynamic heuristic.
    pub fn with_search_operators(mut self, search_operators: HeuristicSearchOperators<C, O, S>) -> Self {
        self.search_operators = Some(search_operators);
        self
    }

    /// Sets diversify operators for dynamic heuristic.
    pub fn with_diversify_operators(mut self, diversify_operators: HeuristicDiversifyOperators<C, O, S>) -> Self {
        self.diversify_operators = Some(diversify_operators);
        self
    }

    /// Gets termination criterias.
    #[allow(clippy::type_complexity)]
    fn get_termination(
        logger: &InfoLogger,
        max_generations: Option<usize>,
        max_time: Option<usize>,
        min_cv: Option<(String, usize, Float, bool, K)>,
        target_proximity: Option<(Vec<Float>, Float)>,
    ) -> Result<Box<dyn Termination<Context = C, Objective = O>>, GenericError> {
        let terminations: Vec<Box<dyn Termination<Context = C, Objective = O>>> = match (
            max_generations,
            max_time,
            &min_cv,
            &target_proximity,
        ) {
            (None, None, None, None) => {
                (logger)("configured to use default max-generations (3000) and max-time (300secs)");
                vec![Box::new(MaxGeneration::new(3000)), Box::new(MaxTime::new(300.))]
            }
            _ => {
                let mut terminations: Vec<Box<dyn Termination<Context = C, Objective = O>>> = vec![];

                if let Some(limit) = max_generations {
                    (logger)(format!("configured to use max-generations: {limit}").as_str());
                    terminations.push(Box::new(MaxGeneration::new(limit)))
                }

                if let Some(limit) = max_time {
                    (logger)(format!("configured to use max-time: {limit}s").as_str());
                    terminations.push(Box::new(MaxTime::new(limit as Float)));
                }

                if let Some((interval_type, value, threshold, is_global, key)) = min_cv.clone() {
                    (logger)(
                            format!(
                                "configured to use variation coefficient {interval_type} with sample: {value}, threshold: {threshold}",
                            )
                            .as_str(),
                        );

                    let variation: Box<dyn Termination<Context = C, Objective = O>> = match interval_type.as_str() {
                        "sample" => {
                            Box::new(MinVariation::<C, O, S, K>::new_with_sample(value, threshold, is_global, key))
                        }
                        "period" => {
                            Box::new(MinVariation::<C, O, S, K>::new_with_period(value, threshold, is_global, key))
                        }
                        _ => return Err(format!("unknown variation interval type: {interval_type}").into()),
                    };

                    terminations.push(variation)
                }

                if let Some((target_fitness, distance_threshold)) = target_proximity.clone() {
                    (logger)(
                            format!(
                                "configured to use target fitness: {target_fitness:?}, distance threshold: {distance_threshold}",
                            )
                            .as_str(),
                        );
                    terminations.push(Box::new(TargetProximity::new(target_fitness, distance_threshold)));
                }

                terminations
            }
        };

        Ok(Box::new(CompositeTermination::new(terminations)))
    }

    /// Builds the evolution config.
    pub fn build(self) -> Result<EvolutionConfig<C, O, S>, GenericError> {
        let context = self.context.ok_or_else(|| "missing heuristic context".to_string())?;
        let logger = context.environment().logger.clone();
        let termination =
            Self::get_termination(&logger, self.max_generations, self.max_time, self.min_cv, self.target_proximity)?;

        Ok(EvolutionConfig {
            initial: self.initial,
            strategy: if let Some(strategy) = self.strategy {
                (logger)("configured to use a custom strategy");
                strategy
            } else {
                let heuristic = if let Some(heuristic) = self.heuristic {
                    (logger)("configured to use custom heuristic");
                    heuristic
                } else {
                    Box::new(DynamicSelective::new(
                        self.search_operators.ok_or_else(|| "missing search operators or heuristic".to_string())?,
                        self.diversify_operators
                            .ok_or_else(|| "missing diversify operators or heuristic".to_string())?,
                        context.environment(),
                    ))
                };
                Box::new(strategies::Iterative::new(heuristic, 1))
            },
            context,
            termination,
            processing: self.processing,
        })
    }
}
