use crate::extensions::TimeQuota;
use crate::Solver;
use std::ops::Deref;
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::construction::Quota;
use vrp_core::models::{Problem, Solution};
use vrp_core::refinement::termination::*;
use vrp_core::refinement::RefinementContext;
use vrp_core::utils::DefaultRandom;

/// Provides configurable way to build solver.
pub struct SolverBuilder {
    solver: Solver,
    max_generations: Option<usize>,
    max_time: Option<usize>,
    init_solution: Option<(Arc<Problem>, Arc<Solution>)>,
}

impl Default for SolverBuilder {
    fn default() -> Self {
        Self { solver: Solver::default(), max_generations: None, max_time: None, init_solution: None }
    }
}

impl SolverBuilder {
    /// Sets max generations to be run.
    /// Default is 2000.
    pub fn with_max_generations(&mut self, limit: Option<usize>) -> &mut Self {
        self.max_generations = limit;
        self
    }

    /// Sets max running time limit.
    /// Default is none.
    pub fn with_max_time(&mut self, limit: Option<usize>) -> &mut Self {
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
        let (criterias, quota): (Vec<Box<dyn Termination>>, _) = match (self.max_generations, self.max_time) {
            (None, None) => {
                self.solver.logger.deref()(
                    "configured to use default max-generations (2000) and max-time (300secs)".to_string(),
                );
                (vec![Box::new(MaxGeneration::default()), Box::new(QuotaReached::default())], create_time_quota(300))
            }
            _ => {
                let mut criterias: Vec<Box<dyn Termination>> = vec![];

                if let Some(limit) = self.max_generations {
                    self.solver.logger.deref()(format!("configured to use max-generations {}", limit));
                    criterias.push(Box::new(MaxGeneration::new(limit)))
                }

                let quota = if let Some(limit) = self.max_time {
                    self.solver.logger.deref()(format!("configured to use max-time {}s", limit));
                    criterias.push(Box::new(QuotaReached::default()));
                    create_time_quota(limit)
                } else {
                    None
                };

                (criterias, quota)
            }
        };

        self.solver.termination = Box::new(CompositeTermination::new(criterias));
        self.solver.quota = quota;

        if let Some((problem, solution)) = &self.init_solution {
            let insertion_ctx = InsertionContext::new_from_solution(
                problem.clone(),
                (solution.clone(), None),
                Arc::new(DefaultRandom::default()),
            );

            let cost = problem.objective.estimate_cost(&mut RefinementContext::new(problem.clone()), &insertion_ctx);
            self.solver.logger.deref()(format!(
                "configured to use initial solution with cost: {:.2}, routes: {}",
                cost.value(),
                solution.routes.len()
            ));

            self.solver.initial = Some(insertion_ctx);
        }
        std::mem::replace(&mut self.solver, Solver::default())
    }
}

fn create_time_quota(limit: usize) -> Option<Box<dyn Quota + Sync + Send>> {
    Some(Box::new(TimeQuota::new(limit as f64)))
}
