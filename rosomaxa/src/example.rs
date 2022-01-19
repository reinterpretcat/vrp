//! This module contains example models and logic to demonstrate practical usage of rosomaxa crate.

use crate::evolution::*;
use crate::get_default_population;
use crate::hyper::*;
use crate::population::{DominanceOrder, DominanceOrdered, RosomaxaWeighted, Shuffled};
use crate::prelude::*;
use hashbrown::HashMap;
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
    pub data: Vec<f64>,
    order: DominanceOrder,
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
        Box::new(self.data.iter().cloned())
    }

    fn deep_copy(&self) -> Self {
        Self::new(self.data.clone())
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
    pub fn new(data: Vec<f64>) -> Self {
        Self { data, order: DominanceOrder::default() }
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

    fn create(&self, _: &Self::Context) -> Self::Solution {
        Self::Solution::new(self.data.clone())
    }
}

pub struct Solver {
    environment: Arc<Environment>,
    initial_solutions: Vec<Vec<f64>>,
    initial_params: (usize, f64),
    objective_func: Option<VectorFunction>,
    max_time: Option<usize>,
    max_generations: Option<usize>,
    min_cv: Option<(String, usize, f64, bool)>,
}

impl Solver {
    /// Creates a new instance of `Solver`.
    pub fn new(environment: Arc<Environment>) -> Self {
        Self {
            environment,
            initial_solutions: vec![],
            initial_params: (4, 0.05),
            objective_func: None,
            max_time: Some(10),
            max_generations: Some(100),
            min_cv: None,
        }
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

    /// Runs the solver using configuration provided through fluent interface methods.
    pub fn solve(self) -> Result<(Vec<(Vec<f64>, f64)>, Option<TelemetryMetrics>), String> {
        // build instances of implementation types from submitted data
        let func = self.objective_func.ok_or("objective function must be set".to_string())?;
        let objective = Arc::new(VectorObjective::new(func));
        let heuristic = Box::new(MultiSelective::new(
            Box::new(DynamicSelective::new(operators, self.environment.random.clone())),
            Box::new(StaticSelective::new(heuristic_group)),
        ));
        let initial_operators =
            self.initial_solutions.into_iter().map(VectorInitialOperator::new).map(Box::new).map(|o| (o, 1)).collect();

        // build evolution config using fluent interface
        let config = EvolutionConfigBuilder::default()
            .with_heuristic(heuristic)
            // TODO replace `with_population` api with `with_context`
            .with_population(get_default_population::<VectorContext, _, _>(objective.clone(), self.environment.clone()))
            .with_min_cv(self.min_cv, 1)
            .with_max_time(self.max_time)
            .with_max_generations(self.max_generations)
            .with_initial(self.initial_params.0, self.initial_params.1, initial_operators)
            .build()?;

        // solve the problem
        let (solutions, metrics) = EvolutionSimulator::new(config, {
            move |population| VectorContext {
                objective,
                population,
                statistics: Default::default(),
                environment: Arc::new(Default::default()),
                state: Default::default(),
            }
        })?
        .run()?;

        let solutions =
            solutions.into_iter().map(|s| (s.data, s.get_fitness().next().expect("empty fitness"))).collect();

        Ok((solutions, metrics))
    }
}
