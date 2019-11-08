use crate::construction::states::InsertionContext;
use crate::models::{Problem, Solution};
use crate::refinement::acceptance::Greedy;
use crate::refinement::termination::{CompositeTermination, MaxGeneration, VariationCoefficient};
use crate::solver::Solver;
use crate::utils::DefaultRandom;
use std::ops::Deref;
use std::sync::Arc;

/// Provides configurable way to build solver.
pub struct SolverBuilder {
    solver: Solver,
    minimize_routes: Option<bool>,
    max_generations: Option<usize>,
    variation_coefficient: Option<(usize, f64)>,
    init_solution: Option<(Arc<Problem>, Arc<Solution>)>,
}

impl SolverBuilder {
    pub fn new() -> Self {
        Self {
            solver: Solver::default(),
            minimize_routes: None,
            max_generations: None,
            variation_coefficient: None,
            init_solution: None,
        }
    }

    pub fn with_minimize_routes(&mut self, value: bool) -> &mut Self {
        self.minimize_routes = Some(value);
        self
    }

    pub fn with_max_generations(&mut self, limit: usize) -> &mut Self {
        self.max_generations = Some(limit);
        self
    }

    pub fn with_variation_coefficient(&mut self, params: Vec<f64>) -> &mut Self {
        let sample =
            params.get(0).and_then(|s| Some(s.round() as usize)).unwrap_or_else(|| panic!("Cannot get sample size"));
        let threshold = *params.get(1).unwrap_or_else(|| panic!("Cannot get threshold"));
        self.variation_coefficient = Some((sample, threshold));
        self
    }

    pub fn with_init_solution(&mut self, solution: Option<(Arc<Problem>, Arc<Solution>)>) -> &mut Self {
        self.init_solution = solution;
        self
    }

    pub fn build(&mut self) -> Solver {
        self.solver.termination =
            Box::new(CompositeTermination::new(match (self.max_generations, self.variation_coefficient) {
                (Some(limit), Some((sample, threshold))) => {
                    self.solver.logger.deref()(format!(
                        "configured to use max-generations {} and variation ({}, {}) limits",
                        limit, sample, threshold
                    ));
                    vec![Box::new(MaxGeneration::new(limit)), Box::new(VariationCoefficient::new(sample, threshold))]
                }
                (None, Some((sample, threshold))) => {
                    self.solver.logger.deref()(format!(
                        "configured to use variation ({}, {}) limit",
                        sample, threshold
                    ));
                    vec![Box::new(MaxGeneration::default()), Box::new(VariationCoefficient::new(sample, threshold))]
                }
                (Some(limit), None) => {
                    self.solver.logger.deref()(format!("configured to use generation {} limit", limit));
                    vec![Box::new(MaxGeneration::new(limit)), Box::new(VariationCoefficient::default())]
                }
                _ => vec![Box::new(MaxGeneration::default()), Box::new(VariationCoefficient::default())],
            }));

        if let Some(value) = self.minimize_routes {
            self.solver.logger.deref()(format!("configured to use minimize routes: {}", value));
            self.solver.acceptance = Box::new(Greedy::new(value));
            self.solver.settings.minimize_routes = value;
        }

        if let Some((problem, solution)) = &self.init_solution {
            self.solver.logger.deref()(format!(
                "configured to use initial solution with {} routes",
                solution.routes.len()
            ));
            let insertion_ctx = InsertionContext::new_from_solution(
                problem.clone(),
                (solution.clone(), None),
                Arc::new(DefaultRandom::new()),
            );
            self.solver.settings.init_insertion_ctx = Some(insertion_ctx);
        }
        std::mem::replace(&mut self.solver, Solver::default())
    }
}
