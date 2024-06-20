use crate::construction::enablers::ScheduleKeys;
use crate::construction::heuristics::*;
use crate::helpers::models::domain::{test_random, TestGoalContextBuilder};
use crate::helpers::models::problem::{test_fleet, TestActivityCost, TestTransportCost};
use crate::models::problem::Job;
use crate::models::solution::Registry;
use crate::models::{ExtrasBuilder, GoalContext, Problem};
use crate::prelude::Jobs;
use rosomaxa::prelude::Environment;
use std::sync::Arc;

pub fn create_state_key() -> StateKey {
    StateKeyRegistry::default().next_key()
}

pub fn create_schedule_keys() -> ScheduleKeys {
    ScheduleKeys::from(&mut StateKeyRegistry::default())
}

#[derive(Default)]
pub struct InsertionContextBuilder {
    problem: Option<Problem>,
    solution: Option<SolutionContext>,
    environment: Option<Environment>,
}

impl InsertionContextBuilder {
    fn ensure_problem(&mut self) -> &mut Problem {
        if self.problem.is_none() {
            self.problem = Some(create_empty_problem());
        }

        self.problem.as_mut().unwrap()
    }

    fn ensure_solution(&mut self) -> &mut SolutionContext {
        if self.solution.is_none() {
            self.solution = Some(create_empty_solution_ctx());
        }

        self.solution.as_mut().unwrap()
    }

    pub fn with_problem(&mut self, problem: Problem) -> &mut Self {
        self.problem = Some(problem);
        self
    }

    pub fn with_solution(&mut self, solution: SolutionContext) -> &mut Self {
        self.solution = Some(solution);
        self
    }

    pub fn with_registry(&mut self, registry: Registry) -> &mut Self {
        let goal = self.ensure_problem().goal.as_ref();
        self.ensure_solution().registry = RegistryContext::new(goal, registry);
        self
    }

    pub fn with_registry_context(&mut self, registry: RegistryContext) -> &mut Self {
        self.ensure_solution().registry = registry;
        self
    }

    pub fn with_routes(&mut self, routes: Vec<RouteContext>) -> &mut Self {
        self.ensure_solution().routes = routes;

        self
    }

    pub fn with_required(&mut self, required: Vec<Job>) -> &mut Self {
        self.ensure_solution().required = required;

        self
    }

    pub fn with_unassigned(&mut self, required: Vec<(Job, UnassignmentInfo)>) -> &mut Self {
        let solution = self.ensure_solution();

        solution.unassigned = required.into_iter().collect();

        self
    }

    pub fn with_goal(&mut self, goal: GoalContext) -> &mut Self {
        self.ensure_problem().goal = Arc::new(goal);
        self
    }

    pub fn with_state<T: 'static + Sync + Send>(&mut self, state_key: StateKey, value: T) -> &mut Self {
        self.ensure_solution().state.insert(state_key, Arc::new(value));
        self
    }

    pub fn build(&mut self) -> InsertionContext {
        self.ensure_problem();
        self.ensure_solution();

        let problem = Arc::new(std::mem::take(&mut self.problem).unwrap());
        let solution = std::mem::take(&mut self.solution).unwrap();

        let environment = if let Some(environment) = std::mem::take(&mut self.environment) {
            Arc::new(environment)
        } else {
            Arc::new(Environment::default())
        };

        InsertionContext { problem, solution, environment }
    }
}

fn create_empty_problem() -> Problem {
    let transport = TestTransportCost::new_shared();
    let fleet = Arc::new(test_fleet());
    let jobs = Arc::new(Jobs::new(fleet.as_ref(), vec![], transport.as_ref()));
    Problem {
        fleet,
        jobs,
        locks: vec![],
        goal: Arc::new(TestGoalContextBuilder::default().build()),
        activity: Arc::new(TestActivityCost::default()),
        transport,
        extras: Arc::new(ExtrasBuilder::default().build().expect("cannot build default extras")),
    }
}

fn create_empty_solution_ctx() -> SolutionContext {
    let goal = TestGoalContextBuilder::default().build();
    let registry = Registry::new(&test_fleet(), test_random());

    SolutionContext {
        required: vec![],
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes: vec![],
        registry: RegistryContext::new(&goal, registry),
        state: Default::default(),
    }
}
