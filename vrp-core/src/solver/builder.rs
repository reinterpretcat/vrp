use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::{Problem, Solution};
use crate::refinement::mutation::*;
use crate::refinement::termination::*;
use crate::solver::evolution::EvolutionConfig;
use crate::solver::Solver;
use crate::utils::{DefaultRandom, TimeQuota};
use std::ops::Deref;
use std::sync::Arc;

/// Provides configurable way to build solver.
pub struct Builder {
    max_generations: Option<usize>,
    max_time: Option<usize>,
    problem: Option<Arc<Problem>>,
    config: EvolutionConfig,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            max_generations: None,
            max_time: None,
            problem: None,
            config: EvolutionConfig {
                mutation: Box::new(RuinAndRecreateMutation::default()),
                termination: Box::new(MaxTime::new(300.)),
                quota: None,
                population_size: 8,
                initial_size: 4,
                initial_methods: vec![
                    (Box::new(RecreateWithCheapest::default()), 10),
                    (Box::new(RecreateWithRegret::default()), 10),
                    (Box::new(RecreateWithRegret::new((5, 8))), 10),
                    (Box::new(RecreateWithBlinks::<i32>::default()), 5),
                    (Box::new(RecreateWithGaps::default()), 5),
                    (Box::new(RecreateWithNearestNeighbor::default()), 5),
                ],
                initial_individuals: vec![],
                random: Arc::new(DefaultRandom::default()),
                logger: Box::new(|msg| println!("{}", msg)),
            },
        }
    }
}

impl Builder {
    /// Sets max generations to be run.
    /// Default is 2000.
    pub fn with_max_generations(&mut self, limit: Option<usize>) -> &mut Self {
        self.max_generations = limit;
        self
    }

    /// Sets max running time limit.
    /// Default is 300 seconds.
    pub fn with_max_time(&mut self, limit: Option<usize>) -> &mut Self {
        self.max_time = limit;
        self
    }

    /// Sets problem.
    pub fn with_problem(&mut self, problem: Arc<Problem>) -> &mut Self {
        self.problem = Some(problem);
        self
    }

    /// Sets initial solutions.
    /// Default is none.
    pub fn with_solutions(&mut self, solutions: Vec<Arc<Solution>>) -> &mut Self {
        self.config.logger.deref()(format!("configured to use {} initial solutions", solutions.len()));
        self.config.initial_individuals = solutions
            .iter()
            .map(|solution| {
                InsertionContext::new_from_solution(
                    self.problem.as_ref().unwrap().clone(),
                    (solution.clone(), None),
                    Arc::new(DefaultRandom::default()),
                )
            })
            .collect();
        self
    }

    /// Sets population size.
    /// Default is 8.
    pub fn with_population_size(&mut self, size: usize) -> &mut Self {
        self.config.logger.deref()(format!("configured to use population size={} ", size));
        self.config.population_size = size;
        self
    }

    /// Sets initial population size. Each initial individual is constructed separately which
    /// used to take more time than normal refinement process.
    /// Default is 4.
    pub fn with_initial_size(&mut self, size: usize) -> &mut Self {
        self.config.logger.deref()(format!("configured to use initial population size={} ", size));
        self.config.initial_size = size;
        self
    }

    /// Builds solver with parameters specified.
    pub fn build(self) -> Result<Solver, String> {
        let problem = self.problem.ok_or_else(|| "problem is not specified".to_string())?;
        let mut config = self.config;

        let (criterias, quota): (Vec<Box<dyn Termination>>, _) = match (self.max_generations, self.max_time) {
            (None, None) => {
                config.logger.deref()(
                    "configured to use default max-generations (2000) and max-time (300secs)".to_string(),
                );
                (vec![Box::new(MaxGeneration::new(2000)), Box::new(MaxTime::new(300.))], None)
            }
            _ => {
                let mut criterias: Vec<Box<dyn Termination>> = vec![];

                if let Some(limit) = self.max_generations {
                    config.logger.deref()(format!("configured to use max-generations {}", limit));
                    criterias.push(Box::new(MaxGeneration::new(limit)))
                }

                let quota = if let Some(limit) = self.max_time {
                    config.logger.deref()(format!("configured to use max-time {}s", limit));
                    criterias.push(Box::new(MaxTime::new(limit as f64)));
                    create_time_quota(limit)
                } else {
                    None
                };

                (criterias, quota)
            }
        };

        config.termination = Box::new(CompositeTermination::new(criterias));
        config.quota = quota;

        Ok(Solver { problem, config })
    }
}

fn create_time_quota(limit: usize) -> Option<Box<dyn Quota + Sync + Send>> {
    Some(Box::new(TimeQuota::new(limit as f64)))
}
