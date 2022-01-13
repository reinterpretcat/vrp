use super::*;
use crate::rosomaxa::evolution::{InitialOperator, Telemetry};
use crate::solver::search::Recreate;

/// Provides configurable way to build Vehile Routing Problem [`Solver`] instance using fluent
/// interface style.
///
/// A newly created builder instance is pre-configured with some reasonable defaults for mid-size
/// problems (~200), so there is no need to call any of its methods.
///
/// [`Solver`]: ./struct.Solver.html
///
/// # Examples
///
/// This example shows how to override some of default metaheuristic parameters using fluent
/// interface methods:
///
/// ```
/// # use vrp_core::models::examples::create_example_problem;
/// # use std::sync::Arc;
/// use vrp_core::solver::Builder;
/// use vrp_core::models::Problem;
/// use vrp_core::utils::Environment;
///
/// // create your VRP problem
/// let problem: Arc<Problem> = create_example_problem();
/// let environment = Arc::new(Environment::default());
/// // build solver using builder with overridden parameters
/// let solver = SolverBuilder::new(problem, environment)
///     .with_max_time(Some(60))
///     .with_max_generations(Some(100))
///     .build()?;
/// // run solver and get the best known solution within its cost.
/// let (solution, cost, _) = solver.solve()?;
///
/// assert_eq!(cost, 42.);
/// assert_eq!(solution.routes.len(), 1);
/// assert_eq!(solution.unassigned.len(), 0);
/// # Ok::<(), String>(())
/// ```
pub struct SolverBuilder {
    /// A problem.
    pub problem: Arc<Problem>,
    /// An environment
    pub environment: Arc<Environment>,
    config_builder: ProblemConfigBuilder,
    processing: Option<Box<dyn Processing + Send + Sync>>,
}

impl SolverBuilder {
    /// Creates a new instance of `Builder`.
    pub fn new(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        let config_builder = ProblemConfigBuilder::new(
            get_default_heuristic(problem.clone(), environment.clone()),
            get_default_population(problem.objective.clone(), environment.clone()),
            create_default_init_operators(problem.clone(), environment.clone()),
            environment.clone(),
        );

        let processing = create_default_processing();

        Self { problem, environment, config_builder, processing }
    }

    /// Sets max generations to be run by evolution.
    pub fn with_max_generations(mut self, limit: Option<usize>) -> Self {
        self.config_builder = self.config_builder.with_max_generations(limit);
        self
    }

    /// Sets max running time limit for evolution.
    pub fn with_max_time(mut self, limit: Option<usize>) -> Self {
        self.config_builder = self.config_builder.with_max_time(limit);
        self
    }

    /// Sets variation coefficient termination criteria.
    pub fn with_min_cv(mut self, min_cv: Option<(String, usize, f64, bool)>) -> Self {
        self.config_builder = self.config_builder.with_min_cv(min_cv, "min_cv".to_string());
        self
    }

    /// Sets initial parameters used to construct initial population.
    pub fn with_initial(
        mut self,
        max_size: usize,
        quota: f64,
        recreates: Vec<(Box<dyn Recreate + Send + Sync>, usize)>,
    ) -> Self {
        let operators = recreates
            .into_iter()
            .map::<(
                Box<
                    dyn InitialOperator<
                            Context = RefinementContext,
                            Objective = ProblemObjective,
                            Solution = InsertionContext,
                        > + Send
                        + Sync,
                >,
                usize,
            ), _>(|(recreate, weight)| {
                (
                    Box::new(RecreateInitialOperator {
                        problem: self.problem.clone(),
                        environment: self.environment.clone(),
                        recreate,
                    }),
                    weight,
                )
            })
            .collect();

        self.config_builder = self.config_builder.with_initial(max_size, quota, operators);
        self
    }

    /// Sets initial solutions in population.
    pub fn with_init_solutions(mut self, solutions: Vec<InsertionContext>, max_init_size: Option<usize>) -> Self {
        self.config_builder = self.config_builder.with_init_solutions(solutions, max_init_size);
        self
    }

    /// Sets population algorithm.
    pub fn with_population(
        mut self,
        population: Box<dyn HeuristicPopulation<Objective = ProblemObjective, Individual = InsertionContext>>,
    ) -> Self {
        self.config_builder = self.config_builder.with_population(population);
        self
    }

    /// Sets termination algorithm.
    pub fn with_termination(
        mut self,
        termination: Box<dyn Termination<Context = RefinementContext, Objective = ProblemObjective>>,
    ) -> Self {
        self.config_builder = self.config_builder.with_termination(termination);
        self
    }

    /// Sets telemetry.
    pub fn with_telemetry(
        mut self,
        telemetry: Telemetry<RefinementContext, ProblemObjective, InsertionContext>,
    ) -> Self {
        self.config_builder = self.config_builder.with_telemetry(telemetry);
        self
    }

    /// Sets problem processing logic.
    pub fn with_processing(mut self, processing: Option<Box<dyn Processing + Send + Sync>>) -> Self {
        self.processing = processing;
        self
    }

    /// Builds a solver.
    pub fn build(self) -> Result<Solver, String> {
        let config = self.config_builder.build()?;

        Ok(Solver { problem: self.problem, config, processing: self.processing })
    }
}

/// A type alias for problem specific evolution config.
type ProblemConfigBuilder = EvolutionConfigBuilder<RefinementContext, ProblemObjective, InsertionContext, String>;

pub(crate) struct RecreateInitialOperator {
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    recreate: Box<dyn Recreate + Send + Sync>,
}

impl RecreateInitialOperator {
    pub fn new(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
        recreate: Box<dyn Recreate + Send + Sync>,
    ) -> Self {
        Self { problem, environment, recreate }
    }
}

impl InitialOperator for RecreateInitialOperator {
    type Context = RefinementContext;
    type Objective = ProblemObjective;
    type Solution = InsertionContext;

    fn create(&self, heuristic_ctx: &Self::Context) -> Self::Solution {
        let insertion_ctx = InsertionContext::new(self.problem.clone(), self.environment.clone());
        self.recreate.run(heuristic_ctx, insertion_ctx)
    }
}
