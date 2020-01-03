use crate::Solver;
use std::ops::Deref;
use std::sync::Arc;
use vrp_core::construction::states::InsertionContext;
use vrp_core::models::{Problem, Solution};
use vrp_core::refinement::acceptance::{Greedy, RandomProbability};
use vrp_core::refinement::termination::*;
use vrp_core::utils::DefaultRandom;

/// Provides configurable way to build solver.
pub struct SolverBuilder {
    solver: Solver,
    minimize_routes: Option<bool>,

    max_generations: Option<usize>,
    variation_coefficient: Option<(usize, f64)>,
    max_time: Option<f64>,

    init_solution: Option<(Arc<Problem>, Arc<Solution>)>,
}

impl Default for SolverBuilder {
    fn default() -> Self {
        Self {
            solver: Solver::default(),
            minimize_routes: None,
            max_generations: None,
            variation_coefficient: None,
            max_time: None,
            init_solution: None,
        }
    }
}

impl SolverBuilder {
    /// Sets whether route minimization should be preferred over cost.
    /// Default is false.
    pub fn with_minimize_routes(&mut self, value: bool) -> &mut Self {
        self.minimize_routes = Some(value);
        self
    }

    /// Sets max generations to be run.
    /// Default is 2000.
    pub fn with_max_generations(&mut self, limit: Option<usize>) -> &mut Self {
        self.max_generations = limit;
        self
    }

    /// Sets variation coefficient parameters.
    /// Default is none.
    pub fn with_variation_coefficient(&mut self, params: Option<Vec<f64>>) -> &mut Self {
        if let Some(params) = params {
            let sample = params.get(0).map(|s| s.round() as usize).unwrap_or_else(|| panic!("Cannot get sample size"));
            let threshold = *params.get(1).unwrap_or_else(|| panic!("Cannot get threshold"));
            self.variation_coefficient = Some((sample, threshold));
        }
        self
    }

    /// Sets max running time limit.
    /// Default is none.
    pub fn with_max_time(&mut self, limit: Option<f64>) -> &mut Self {
        self.max_time = limit;
        self
    }

    /// Sets initial solution.
    /// Default is none.
    pub fn with_init_solution(&mut self, solution: Option<(Arc<Problem>, Arc<Solution>)>) -> &mut Self {
        self.init_solution = solution;
        self
    }

    /// Builds solver with parameters specified.
    pub fn build(&mut self) -> Solver {
        self.solver.termination = Box::new(CompositeTermination::new(
            match (self.max_generations, self.variation_coefficient, self.max_time) {
                (None, None, None) => {
                    self.solver.logger.deref()("configured to use default max-generations (2000)".to_string());
                    vec![Box::new(MaxGeneration::default())]
                }
                _ => {
                    let mut criterias: Vec<Box<dyn Termination>> = vec![];

                    if let Some(limit) = self.max_generations {
                        self.solver.logger.deref()(format!("configured to use max-generations {}", limit));
                        criterias.push(Box::new(MaxGeneration::new(limit)))
                    }

                    if let Some((sample, threshold)) = self.variation_coefficient {
                        self.solver.logger.deref()(format!("configured to use variation ({}, {})", sample, threshold));
                        criterias.push(Box::new(VariationCoefficient::new(sample, threshold)));
                    }

                    if let Some(limit) = self.max_time {
                        self.solver.logger.deref()(format!("configured to use max-time {}s", limit));
                        criterias.push(Box::new(MaxTime::new(limit)));
                    }

                    criterias
                }
            },
        ));

        if let Some(value) = self.minimize_routes {
            self.solver.logger.deref()(format!("configured to use minimize routes: {}", value));
            self.solver.acceptance = Box::new(RandomProbability::new(Box::new(Greedy::new(value)), 0.001));
            self.solver.settings.minimize_routes = value;
        }

        if let Some((problem, solution)) = &self.init_solution {
            let insertion_ctx = InsertionContext::new_from_solution(
                problem.clone(),
                (solution.clone(), None),
                Arc::new(DefaultRandom::default()),
            );

            let cost = problem.objective.estimate(&insertion_ctx);
            self.solver.logger.deref()(format!(
                "configured to use initial solution with cost: ({:.2},{:.2}), routes: {}",
                cost.actual,
                cost.penalty,
                solution.routes.len()
            ));

            self.solver.settings.init_insertion_ctx = Some((insertion_ctx, cost));
        }
        std::mem::replace(&mut self.solver, Solver::default())
    }
}
