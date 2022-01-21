//! This module contains example models and logic to demonstrate practical usage of rosomaxa crate.

use crate::evolution::*;
use crate::get_default_population;
use crate::hyper::*;
use crate::population::{DominanceOrder, DominanceOrdered, RosomaxaWeighted, Shuffled};
use crate::prelude::*;
use crate::utils::Noise;
use hashbrown::{HashMap, HashSet};
use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

/// An example objective function.
pub type VectorFunction = Arc<dyn Fn(&[f64]) -> f64 + Send + Sync>;

/// An example heuristic context.
pub struct VectorContext {
    objective: Arc<VectorObjective>,
    population: Box<dyn HeuristicPopulation<Objective = VectorObjective, Individual = VectorSolution>>,
    statistics: HeuristicStatistics,
    environment: Arc<Environment>,
    state: HashMap<i32, Box<dyn Any + Send + Sync>>,
}

/// An example heuristic objective.
pub struct VectorObjective {
    func: VectorFunction,
}

/// An example heuristic solution.
pub struct VectorSolution {
    /// Solution payload.
    pub data: Vec<f64>,
    objective: Arc<VectorObjective>,
    order: DominanceOrder,
}

impl VectorContext {
    /// Creates a new instance of `VectorContext`.
    pub fn new(
        objective: Arc<VectorObjective>,
        population: Box<dyn HeuristicPopulation<Objective = VectorObjective, Individual = VectorSolution>>,
        environment: Arc<Environment>,
    ) -> Self {
        Self { objective, population, statistics: Default::default(), environment, state: Default::default() }
    }
}

impl HeuristicContext for VectorContext {
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn objective(&self) -> &Self::Objective {
        &self.objective
    }

    fn population(&self) -> &dyn HeuristicPopulation<Objective = Self::Objective, Individual = Self::Solution> {
        self.population.as_ref()
    }

    fn population_mut(
        &mut self,
    ) -> &mut dyn HeuristicPopulation<Objective = Self::Objective, Individual = Self::Solution> {
        self.population.as_mut()
    }

    fn statistics(&self) -> &HeuristicStatistics {
        &self.statistics
    }

    fn statistics_mut(&mut self) -> &mut HeuristicStatistics {
        &mut self.statistics
    }

    fn environment(&self) -> &Environment {
        self.environment.as_ref()
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
    pub fn new(func: VectorFunction) -> Self {
        Self { func }
    }
}

impl HeuristicObjective for VectorObjective {}

impl Objective for VectorObjective {
    type Solution = VectorSolution;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        self.func.deref()(solution.data.as_slice())
    }
}

impl MultiObjective for VectorObjective {
    fn objectives<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a (dyn Objective<Solution = Self::Solution> + Send + Sync)> + 'a> {
        let objective: &(dyn Objective<Solution = Self::Solution> + Send + Sync) = self;

        Box::new(std::iter::once(objective))
    }
}

impl Shuffled for VectorObjective {
    fn get_shuffled(&self, _: &(dyn Random + Send + Sync)) -> Self {
        Self::new(self.func.clone())
    }
}

impl HeuristicSolution for VectorSolution {
    fn get_fitness<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.objective.objectives().map(move |objective| objective.fitness(self)))
        //Box::new(self.fitness.iter())
    }

    fn deep_copy(&self) -> Self {
        Self::new(self.data.clone(), self.objective.clone())
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
    fn weights(&self) -> Vec<f64> {
        // TODO:
        //  for the sake of experimentation, consider to provide some configuration here to allow
        //  usage of some noise, smoothing or optional weights, but not only direct mapping of data.
        self.data.clone()
    }
}

impl VectorSolution {
    /// Creates a new instance of `VectorSolution`.
    pub fn new(data: Vec<f64>, objective: Arc<VectorObjective>) -> Self {
        Self { data, objective, order: DominanceOrder::default() }
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
        Self::Solution::new(self.data.clone(), context.objective.clone())
    }
}

/// Specifies mode of heuristic operator.
pub enum VectorHeuristicOperatorMode {
    /// Adds some noice to all dimensions.
    JustNoise(Noise),
    /// Adds some noice to specific dimensions.
    DimensionNoise(Noise, HashSet<usize>),
}

/// A naive implementation of heuristic search operator in vector space.
struct VectorHeuristicOperator {
    mode: VectorHeuristicOperatorMode,
}

impl HeuristicOperator for VectorHeuristicOperator {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn search(&self, context: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        Self::Solution::new(
            match &self.mode {
                VectorHeuristicOperatorMode::JustNoise(noise) => solution.data.iter().map(|d| noise.add(*d)).collect(),
                VectorHeuristicOperatorMode::DimensionNoise(noise, dimens) => solution
                    .data
                    .iter()
                    .enumerate()
                    .map(|(idx, d)| if dimens.contains(&idx) { noise.add(*d) } else { *d })
                    .collect(),
            },
            context.objective.clone(),
        )
    }
}

type TargetInitialOperator = Box<
    dyn InitialOperator<Context = VectorContext, Objective = VectorObjective, Solution = VectorSolution> + Send + Sync,
>;

type TargetHeuristicOperator = Arc<
    dyn HeuristicOperator<Context = VectorContext, Objective = VectorObjective, Solution = VectorSolution>
        + Send
        + Sync,
>;

/// Specifies solver solution.
pub type SolverSolution = Vec<(Vec<f64>, f64)>;

/// An example of the optimization solver to solve trivial problems.
pub struct Solver {
    initial_solutions: Vec<Vec<f64>>,
    initial_params: (usize, f64),
    objective_func: Option<VectorFunction>,
    max_time: Option<usize>,
    max_generations: Option<usize>,
    min_cv: Option<(String, usize, f64, bool)>,
    operators: Vec<(TargetHeuristicOperator, String, f64)>,
}

impl Default for Solver {
    fn default() -> Self {
        Self {
            initial_solutions: vec![],
            initial_params: (4, 0.05),
            objective_func: None,
            max_time: Some(10),
            max_generations: Some(100),
            min_cv: None,
            operators: vec![],
        }
    }
}

impl Solver {
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
    ) -> Self {
        self.max_time = max_time;
        self.max_generations = max_generations;
        self.min_cv = min_cv;

        self
    }

    /// Sets search operator.
    pub fn with_operator(mut self, mode: VectorHeuristicOperatorMode, name: &str, probability: f64) -> Self {
        self.operators.push((Arc::new(VectorHeuristicOperator { mode }), name.to_string(), probability));
        self
    }

    /// Runs the solver using configuration provided through fluent interface methods.
    pub fn solve(self) -> Result<(SolverSolution, Option<TelemetryMetrics>), String> {
        let environment = Arc::new(Environment::new_with_time_quota(self.max_time));

        // build instances of implementation types from submitted data
        let func = self.objective_func.ok_or_else(|| "objective function must be set".to_string())?;
        let objective = Arc::new(VectorObjective::new(func));
        let heuristic = Box::new(MultiSelective::new(
            Box::new(DynamicSelective::new(
                self.operators.iter().map(|(op, name, _)| (op.clone(), name.clone())).collect(),
                environment.random.clone(),
            )),
            Box::new(StaticSelective::new(
                self.operators
                    .iter()
                    .map(|(op, _, probability)| {
                        let random = environment.random.clone();
                        let probability = *probability;
                        let probability_func: HeuristicProbability<VectorContext, VectorObjective, VectorSolution> =
                            (Box::new(move |_, _| random.is_hit(probability)), Default::default());
                        (op.clone(), probability_func)
                    })
                    .collect(),
            )),
        ));
        let initial_operators = self
            .initial_solutions
            .into_iter()
            .map(VectorInitialOperator::new)
            .map::<(TargetInitialOperator, _), _>(|o| (Box::new(o), 1))
            .collect();

        // create a heuristic context
        let context = VectorContext::new(
            objective.clone(),
            get_default_population::<VectorContext, _, _>(objective, environment.clone()),
            environment,
        );

        // build evolution config using fluent interface
        let config = EvolutionConfigBuilder::default()
            .with_heuristic(heuristic)
            .with_context(context)
            .with_min_cv(self.min_cv, 1)
            .with_max_time(self.max_time)
            .with_max_generations(self.max_generations)
            .with_initial(self.initial_params.0, self.initial_params.1, initial_operators)
            .build()?;

        // solve the problem
        let (solutions, metrics) = EvolutionSimulator::new(config)?.run()?;

        let solutions = solutions
            .into_iter()
            .map(|s| {
                let fitness = s.get_fitness().next().expect("empty fitness");
                (s.data, fitness)
            })
            .collect();

        Ok((solutions, metrics))
    }
}
