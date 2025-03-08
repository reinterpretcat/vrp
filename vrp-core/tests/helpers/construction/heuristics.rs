use crate::construction::heuristics::*;
use crate::helpers::models::domain::{TestGoalContextBuilder, test_logger, test_random};
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost, test_fleet};
use crate::models::problem::Job;
use crate::models::solution::Registry;
use crate::models::{Extras, GoalContext, Problem};
use crate::prelude::{Fleet, Jobs};
use rosomaxa::prelude::Environment;
use std::sync::Arc;

#[derive(Default)]
pub struct TestInsertionContextBuilder {
    problem: Option<Problem>,
    solution: Option<SolutionContext>,
    environment: Option<Environment>,
}

impl TestInsertionContextBuilder {
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

    pub fn with_fleet(&mut self, fleet: Arc<Fleet>) -> &mut Self {
        self.ensure_problem().fleet = fleet;
        self
    }

    pub fn with_registry(&mut self, registry: Registry) -> &mut Self {
        let goal = self.ensure_problem().goal.as_ref();
        self.ensure_solution().registry = RegistryContext::new(goal, registry);
        self
    }

    pub fn with_routes(&mut self, routes: Vec<RouteContext>) -> &mut Self {
        self.ensure_solution().routes = routes;

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

    pub fn with_state(&mut self, state_fn: impl FnOnce(&mut SolutionState)) -> &mut Self {
        state_fn(&mut self.ensure_solution().state);
        self
    }

    pub fn build(&mut self) -> InsertionContext {
        self.ensure_problem();
        self.ensure_solution();

        let problem = Arc::new(std::mem::take(&mut self.problem).unwrap());
        let solution = std::mem::take(&mut self.solution).unwrap();

        let environment = match std::mem::take(&mut self.environment) {
            Some(environment) => Arc::new(environment),
            _ => Arc::new(Environment::default()),
        };

        InsertionContext { problem, solution, environment }
    }
}

fn create_empty_problem() -> Problem {
    let transport = TestTransportCost::new_shared();
    let fleet = Arc::new(test_fleet());
    let jobs = Arc::new(Jobs::new(fleet.as_ref(), vec![], transport.as_ref(), &test_logger()).unwrap());
    Problem {
        fleet,
        jobs,
        locks: vec![],
        goal: Arc::new(TestGoalContextBuilder::default().build()),
        activity: Arc::new(TestActivityCost::default()),
        transport,
        extras: Arc::new(Extras::default()),
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
