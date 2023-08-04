//! This module contains example models and logic to demonstrate practical usage of rosomaxa crate.

#[cfg(test)]
#[path = "../tests/unit/example_test.rs"]
mod example_test;

use crate::algorithms::gsom::Input;
use crate::evolution::*;
use crate::hyper::*;
use crate::population::{DominanceOrder, DominanceOrdered, RosomaxaWeighted, Shuffled};
use crate::prelude::*;
use crate::utils::Noise;
use crate::*;
use std::any::Any;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::iter::once;
use std::ops::Range;
use std::sync::Arc;

/// An objective function which calculates a fitness of a vector.
pub type FitnessFn = Arc<dyn Fn(&[f64]) -> f64 + Send + Sync>;
/// A weight function which calculates rosomaxa weights of a vector.
pub type WeightFn = Arc<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync>;
/// Specifies a population type which stores vector solutions.
pub type VectorPopulation = DynHeuristicPopulation<VectorObjective, VectorSolution>;

/// An example heuristic context.
pub struct VectorContext {
    inner_context: TelemetryHeuristicContext<VectorObjective, VectorSolution>,
    objective: Arc<VectorObjective>,
    state: HashMap<i32, Box<dyn Any + Send + Sync>>,
}

/// An example heuristic objective.
pub struct VectorObjective {
    fitness_fn: FitnessFn,
    weight_fn: WeightFn,
}

/// An example heuristic solution.
#[derive(Clone)]
pub struct VectorSolution {
    /// Solution payload.
    pub data: Vec<f64>,
    weights: Vec<f64>,
    objective: Arc<VectorObjective>,
    order: DominanceOrder,
}

impl VectorSolution {
    /// Returns a fitness value of given solution.
    pub fn fitness(&self) -> f64 {
        Objective::fitness(self.objective.as_ref(), self)
    }
}

impl VectorContext {
    /// Creates a new instance of `VectorContext`.
    pub fn new(
        objective: Arc<VectorObjective>,
        population: Box<VectorPopulation>,
        telemetry_mode: TelemetryMode,
        environment: Arc<Environment>,
    ) -> Self {
        Self {
            inner_context: TelemetryHeuristicContext::new(objective.clone(), population, telemetry_mode, environment),
            objective,
            state: Default::default(),
        }
    }
}

impl HeuristicContext for VectorContext {
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn objective(&self) -> &Self::Objective {
        self.inner_context.objective()
    }

    fn selected<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Solution> + 'a> {
        self.inner_context.population.select()
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Self::Solution, usize)> + 'a> {
        self.inner_context.population.ranked()
    }

    fn statistics(&self) -> &HeuristicStatistics {
        self.inner_context.statistics()
    }

    fn selection_phase(&self) -> SelectionPhase {
        self.inner_context.population.selection_phase()
    }

    fn environment(&self) -> &Environment {
        self.inner_context.environment()
    }

    fn on_initial(&mut self, solution: Self::Solution, item_time: Timer) {
        self.inner_context.on_initial(solution, item_time)
    }

    fn on_generation(&mut self, offspring: Vec<Self::Solution>, termination_estimate: f64, generation_time: Timer) {
        self.inner_context.on_generation(offspring, termination_estimate, generation_time)
    }

    fn on_result(self) -> HeuristicResult<Self::Objective, Self::Solution> {
        self.inner_context.on_result()
    }
}

impl Stateful for VectorContext {
    type Key = i32;

    fn set_state<T: 'static + Send + Sync>(&mut self, key: Self::Key, state: T) {
        self.state.insert(key, Box::new(state));
    }

    fn get_state<T: 'static + Send + Sync>(&self, key: &Self::Key) -> Option<&T> {
        self.state.get(key).and_then(|v| v.downcast_ref::<T>())
    }

    fn state_mut<T: 'static + Send + Sync, F: Fn() -> T>(&mut self, key: Self::Key, inserter: F) -> &mut T {
        self.state.entry(key).or_insert_with(|| Box::new(inserter())).downcast_mut::<T>().unwrap()
    }
}

impl VectorObjective {
    /// Creates a new instance `VectorObjective`.
    pub fn new(fitness_fn: FitnessFn, weight_fn: WeightFn) -> Self {
        Self { fitness_fn, weight_fn }
    }
}

impl HeuristicObjective for VectorObjective {}

impl Objective for VectorObjective {
    type Solution = VectorSolution;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        (self.fitness_fn)(solution.data.as_slice())
    }
}

impl MultiObjective for VectorObjective {
    type Solution = VectorSolution;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        Objective::total_order(self, a, b)
    }

    fn fitness<'a>(&self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(once((self.fitness_fn)(solution.data.as_slice())))
    }

    fn get_order(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<Ordering, GenericError> {
        if idx == 0 {
            Ok(Objective::total_order(self, a, b))
        } else {
            Err(format!("objective has only 1 inner, passed index: {idx}").into())
        }
    }

    fn get_distance(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<f64, GenericError> {
        if idx == 0 {
            Ok(Objective::distance(self, a, b))
        } else {
            Err(format!("objective has only 1 inner, passed index: {idx}").into())
        }
    }

    fn size(&self) -> usize {
        1
    }
}

impl Shuffled for VectorObjective {
    fn get_shuffled(&self, _: &(dyn Random + Send + Sync)) -> Self {
        Self::new(self.fitness_fn.clone(), self.weight_fn.clone())
    }
}

impl HeuristicSolution for VectorSolution {
    fn fitness<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a> {
        MultiObjective::fitness(self.objective.as_ref(), self)
    }

    fn deep_copy(&self) -> Self {
        Self {
            data: self.data.clone(),
            weights: self.weights.clone(),
            objective: self.objective.clone(),
            order: self.order.clone(),
        }
    }
}

impl DominanceOrdered for VectorSolution {
    fn get_order(&self) -> &DominanceOrder {
        &self.order
    }

    fn set_order(&mut self, order: DominanceOrder) {
        self.order = order
    }
}

impl RosomaxaWeighted for VectorSolution {
    fn init_weights(&mut self) {
        self.weights = (self.objective.weight_fn)(self.data.as_slice());
    }
}

impl Input for VectorSolution {
    fn weights(&self) -> &[f64] {
        self.weights.as_slice()
    }
}

impl VectorSolution {
    /// Creates a new instance of `VectorSolution`.
    pub fn new(data: Vec<f64>, objective: Arc<VectorObjective>) -> Self {
        Self { data, objective, weights: Vec::default(), order: DominanceOrder::default() }
    }
}

/// An example initial operator
pub struct VectorInitialOperator {
    data: Vec<f64>,
}

impl VectorInitialOperator {
    /// Creates a new instance of `VectorInitialOperator`.
    pub fn new(data: Vec<f64>) -> Self {
        Self { data }
    }
}

impl InitialOperator for VectorInitialOperator {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn create(&self, context: &Self::Context) -> Self::Solution {
        Self::Solution::new(self.data.clone(), context.inner_context.objective.clone())
    }
}

/// Specifies mode of heuristic operator.
pub enum VectorHeuristicOperatorMode {
    /// Adds some noice to all dimensions.
    JustNoise(Noise),
    /// Adds some noice to specific dimensions.
    DimensionNoise(Noise, HashSet<usize>),
    /// Adds a delta for each dimension.
    JustDelta(Range<f64>),
}

/// A naive implementation of heuristic search operator in vector space.
struct VectorHeuristicOperator {
    mode: VectorHeuristicOperatorMode,
}

impl HeuristicSearchOperator for VectorHeuristicOperator {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn search(&self, context: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        Self::Solution::new(
            match &self.mode {
                VectorHeuristicOperatorMode::JustNoise(noise) => {
                    solution.data.iter().map(|&d| d + noise.generate(d)).collect()
                }
                VectorHeuristicOperatorMode::DimensionNoise(noise, dimens) => solution
                    .data
                    .iter()
                    .enumerate()
                    .map(|(idx, &d)| if dimens.contains(&idx) { d + noise.generate(d) } else { d })
                    .collect(),
                VectorHeuristicOperatorMode::JustDelta(range) => solution
                    .data
                    .iter()
                    .map(|&d| d + context.environment().random.uniform_real(range.start, range.end))
                    .collect(),
            },
            context.objective.clone(),
        )
    }
}

impl HeuristicDiversifyOperator for VectorHeuristicOperator {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        // NOTE: just reuse search operator logic
        vec![self.search(heuristic_ctx, solution)]
    }
}

type TargetInitialOperator = Box<
    dyn InitialOperator<Context = VectorContext, Objective = VectorObjective, Solution = VectorSolution> + Send + Sync,
>;

type TargetSearchOperator = Arc<
    dyn HeuristicSearchOperator<Context = VectorContext, Objective = VectorObjective, Solution = VectorSolution>
        + Send
        + Sync,
>;

type TargetDiversifyOperator = Arc<
    dyn HeuristicDiversifyOperator<Context = VectorContext, Objective = VectorObjective, Solution = VectorSolution>
        + Send
        + Sync,
>;

type TargetHeuristic =
    Box<dyn HyperHeuristic<Context = VectorContext, Objective = VectorObjective, Solution = VectorSolution>>;

/// Specifies solver solutions.
pub type SolverSolutions = Vec<(Vec<f64>, f64)>;
/// Specifies heuristic context factory type.
pub type ContextFactory = Box<dyn FnOnce(Arc<VectorObjective>, Arc<Environment>) -> VectorContext>;

/// An example of the optimization solver to solve trivial problems.
pub struct Solver {
    is_experimental: bool,
    logger: Option<InfoLogger>,
    use_static_heuristic: bool,
    initial_solutions: Vec<Vec<f64>>,
    initial_params: (usize, f64),
    fitness_fn: Option<FitnessFn>,
    weight_fn: Option<WeightFn>,
    max_time: Option<usize>,
    max_generations: Option<usize>,
    min_cv: Option<(String, usize, f64, bool)>,
    target_proximity: Option<(Vec<f64>, f64)>,
    search_operators: Vec<(TargetSearchOperator, String, f64)>,
    diversify_operators: Vec<TargetDiversifyOperator>,
    context_factory: Option<ContextFactory>,
}

impl Default for Solver {
    fn default() -> Self {
        Self {
            is_experimental: false,
            logger: None,
            use_static_heuristic: false,
            initial_solutions: vec![],
            initial_params: (4, 0.05),
            fitness_fn: None,
            weight_fn: None,
            max_time: Some(10),
            max_generations: Some(100),
            min_cv: None,
            target_proximity: None,
            search_operators: vec![],
            diversify_operators: vec![],
            context_factory: None,
        }
    }
}

impl Solver {
    /// Sets experimental flag to true (false is default).
    pub fn set_experimental(mut self) -> Self {
        self.is_experimental = true;
        self
    }

    /// Sets logger.
    pub fn with_logger(mut self, logger: InfoLogger) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Use dynamic selective only
    pub fn use_static_heuristic(mut self) -> Self {
        self.use_static_heuristic = true;
        self
    }

    /// Sets initial parameters.
    pub fn with_init_params(mut self, max_size: usize, quota: f64) -> Self {
        self.initial_params = (max_size, quota);
        self
    }

    /// Sets initial solutions.
    pub fn with_init_solutions(mut self, init_solutions: Vec<Vec<f64>>) -> Self {
        self.initial_solutions = init_solutions;
        self
    }

    // TODO add termination to stop when solution close to some target

    /// Sets termination parameters.
    pub fn with_termination(
        mut self,
        max_time: Option<usize>,
        max_generations: Option<usize>,
        min_cv: Option<(String, usize, f64, bool)>,
        target_proximity: Option<(Vec<f64>, f64)>,
    ) -> Self {
        self.max_time = max_time;
        self.max_generations = max_generations;
        self.min_cv = min_cv;
        self.target_proximity = target_proximity;

        self
    }

    /// Sets search operator.
    pub fn with_search_operator(mut self, mode: VectorHeuristicOperatorMode, name: &str, probability: f64) -> Self {
        self.search_operators.push((Arc::new(VectorHeuristicOperator { mode }), name.to_string(), probability));
        self
    }

    /// Sets diversify operator.
    pub fn with_diversify_operator(mut self, mode: VectorHeuristicOperatorMode) -> Self {
        self.diversify_operators.push(Arc::new(VectorHeuristicOperator { mode }));
        self
    }

    /// Sets fitness function.
    pub fn with_fitness_fn(mut self, objective_fn: FitnessFn) -> Self {
        self.fitness_fn = Some(objective_fn);
        self
    }

    /// Sets weight function.
    pub fn with_weight_fn(mut self, weight_fn: WeightFn) -> Self {
        self.weight_fn = Some(weight_fn);
        self
    }

    /// Sets heuristic context factory.
    pub fn with_context_factory(mut self, context_factory: ContextFactory) -> Self {
        self.context_factory = Some(context_factory);
        self
    }

    /// Runs the solver using configuration provided through fluent interface methods.
    pub fn solve(self) -> Result<(SolverSolutions, Option<TelemetryMetrics>), GenericError> {
        // create an environment based on max_time and logger parameters supplied
        let environment =
            Environment { is_experimental: self.is_experimental, ..Environment::new_with_time_quota(self.max_time) };
        let environment = Arc::new(if let Some(logger) = self.logger.clone() {
            Environment { logger, ..environment }
        } else {
            environment
        });

        // build instances of implementation types from submitted data
        let heuristic = if self.use_static_heuristic {
            self.create_static_heuristic(environment.as_ref())
        } else {
            self.create_dynamic_heuristic(environment.as_ref())
        };
        let fitness_fn = self.fitness_fn.ok_or_else(|| "objective function must be set".to_string())?;
        let weight_fn = self.weight_fn.unwrap_or_else({
            let fitness_fn = fitness_fn.clone();
            move || Arc::new(move |data| data.iter().cloned().chain(once((fitness_fn)(data))).collect())
        });
        let objective = Arc::new(VectorObjective::new(fitness_fn, weight_fn));
        let initial_operators = self
            .initial_solutions
            .into_iter()
            .map(VectorInitialOperator::new)
            .map::<(TargetInitialOperator, _), _>(|o| (Box::new(o), 1))
            .collect();

        // create a heuristic context
        let context = {
            self.context_factory.map_or_else(
                || {
                    let selection_size = get_default_selection_size(environment.as_ref());
                    VectorContext::new(
                        objective.clone(),
                        get_default_population(objective.clone(), environment.clone(), selection_size),
                        TelemetryMode::OnlyLogging {
                            logger: environment.logger.clone(),
                            log_best: 100,
                            log_population: 500,
                            dump_population: false,
                        },
                        environment.clone(),
                    )
                },
                |context_factory| context_factory(objective.clone(), environment.clone()),
            )
        };

        // build evolution config using fluent interface
        let config = EvolutionConfigBuilder::default()
            .with_heuristic(heuristic)
            .with_objective(objective)
            .with_context(context)
            .with_min_cv(self.min_cv, 1)
            .with_max_time(self.max_time)
            .with_max_generations(self.max_generations)
            .with_target_proximity(self.target_proximity)
            .with_initial(self.initial_params.0, self.initial_params.1, initial_operators)
            .build()?;

        // solve the problem
        let (solutions, metrics) = EvolutionSimulator::new(config)?.run()?;

        let solutions = solutions
            .into_iter()
            .map(|s| {
                let fitness = s.fitness();
                (s.data, fitness)
            })
            .collect();

        Ok((solutions, metrics))
    }

    fn create_dynamic_heuristic(&self, environment: &Environment) -> TargetHeuristic {
        Box::new(DynamicSelective::new(
            self.search_operators.iter().map(|(op, name, weight)| (op.clone(), name.clone(), *weight)).collect(),
            self.diversify_operators.clone(),
            environment,
        ))
    }

    fn create_static_heuristic(&self, environment: &Environment) -> TargetHeuristic {
        Box::new(StaticSelective::new(
            self.search_operators
                .iter()
                .map(|(op, _, probability)| {
                    let random = environment.random.clone();
                    let probability = *probability;
                    let probability_fn: HeuristicProbability<VectorContext, VectorObjective, VectorSolution> =
                        (Box::new(move |_, _| random.is_hit(probability)), Default::default());
                    (op.clone(), probability_fn)
                })
                .collect(),
            self.diversify_operators.clone(),
        ))
    }
}

/// Creates multidimensional Rosenbrock function, also referred to as the Valley or Banana function.
/// The function is usually evaluated on the hypercube xi ∈ [-5, 10], for all i = 1, …, d, although
/// it may be restricted to the hypercube xi ∈ [-2.048, 2.048], for all i = 1, …, d.
pub fn create_rosenbrock_function() -> FitnessFn {
    Arc::new(|input| {
        assert!(input.len() > 1);

        input.windows(2).fold(0., |acc, pair| {
            let (x1, x2) = match pair {
                [x1, x2] => (*x1, *x2),
                _ => unreachable!(),
            };

            acc + 100. * (x2 - x1.powi(2)).powi(2) + (x1 - 1.).powi(2)
        })
    })
}
