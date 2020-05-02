use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::{Problem, Solution};
use crate::solver::evolution::EvolutionConfig;
use crate::solver::mutation::*;
use crate::solver::termination::*;
use crate::solver::Solver;
use crate::utils::{DefaultRandom, TimeQuota};
use std::ops::Deref;
use std::sync::Arc;

/// Provides configurable way to build solver.
pub struct Builder {
    max_generations: Option<usize>,
    max_time: Option<usize>,
    cost_variation: Option<(usize, f64)>,
    problem: Option<Arc<Problem>>,
    config: EvolutionConfig,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            max_generations: None,
            max_time: None,
            cost_variation: None,
            problem: None,
            config: EvolutionConfig {
                mutation: Box::new(RuinAndRecreateMutation::default()),
                termination: Box::new(MaxTime::new(300.)),
                quota: None,
                population_size: 4,
                offspring_size: 4,
                elite_size: 2,
                initial_size: 2,
                initial_methods: vec![
                    (Box::new(RecreateWithCheapest::default()), 10),
                    (Box::new(RecreateWithRegret::default()), 10),
                    (Box::new(RecreateWithBlinks::<i32>::default()), 5),
                ],
                initial_individuals: vec![],
                random: Arc::new(DefaultRandom::default()),
                logger: Arc::new(|msg| println!("{}", msg)),
            },
        }
    }
}

impl Builder {
    /// Sets max generations to be run.
    /// Default is 2000.
    pub fn with_max_generations(mut self, limit: Option<usize>) -> Self {
        self.max_generations = limit;
        self
    }

    /// Sets cost variation termination criteria.
    /// Default is None.
    pub fn with_cost_variation(mut self, variation: Option<(usize, f64)>) -> Self {
        self.cost_variation = variation;
        self
    }

    /// Sets max running time limit.
    /// Default is 300 seconds.
    pub fn with_max_time(mut self, limit: Option<usize>) -> Self {
        self.max_time = limit;
        self
    }

    /// Sets problem.
    pub fn with_problem(mut self, problem: Arc<Problem>) -> Self {
        self.problem = Some(problem);
        self
    }

    /// Sets initial methods.
    pub fn with_initial_methods(mut self, initial_methods: Vec<(Box<dyn Recreate>, usize)>) -> Self {
        self.config.initial_methods = initial_methods;
        self
    }

    /// Sets initial solutions.
    /// Default is none.
    pub fn with_solutions(mut self, solutions: Vec<Arc<Solution>>) -> Self {
        self.config.logger.deref()(format!("provided {} initial solutions to start with", solutions.len()));
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
    /// Default is 4.
    pub fn with_population_size(mut self, size: usize) -> Self {
        self.config.logger.deref()(format!("configured to use population size={} ", size));
        self.config.population_size = size;
        self
    }

    /// Sets offspring size.
    /// Default is 4.
    pub fn with_offspring_size(mut self, size: usize) -> Self {
        self.config.logger.deref()(format!("configured to use offspring size={} ", size));
        self.config.offspring_size = size;
        self
    }

    /// Sets elite size.
    /// Default is 2.
    pub fn with_elite_size(mut self, size: usize) -> Self {
        self.config.logger.deref()(format!("configured to use elite size={} ", size));
        self.config.elite_size = size;
        self
    }

    /// Sets initial population size. Each initial individual is constructed separately which
    /// used to take more time than normal refinement process.
    /// Default is 2.
    pub fn with_initial_size(mut self, size: usize) -> Self {
        self.config.logger.deref()(format!("configured to use initial population size={} ", size));
        self.config.initial_size = size;
        self
    }

    /// Sets mutation algorithm.
    /// Default is ruin and recreate.
    pub fn with_mutation(mut self, mutation: Box<dyn Mutation>) -> Self {
        self.config.mutation = mutation;
        self
    }

    /// Sets termination algorithm.
    /// Default is max time and max generations.
    pub fn with_termination(mut self, termination: Box<dyn Termination>) -> Self {
        self.config.termination = termination;
        self
    }

    /// Builds solver with parameters specified.
    pub fn build(self) -> Result<Solver, String> {
        let problem = self.problem.ok_or_else(|| "problem is not specified".to_string())?;
        let mut config = self.config;

        let (criterias, quota): (Vec<Box<dyn Termination>>, _) =
            match (self.max_generations, self.max_time, self.cost_variation) {
                (None, None, None) => {
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

                    if let Some((sample, threshold)) = self.cost_variation {
                        config.logger.deref()(format!(
                            "configured to use cost variation with sample: {}, threshold: {}",
                            sample, threshold
                        ));
                        criterias.push(Box::new(CostVariation::new(sample, threshold)))
                    }

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
