use crate::construction::enablers::ScheduleKeys;
use crate::construction::features::create_minimize_transport_costs_feature;
use crate::construction::heuristics::*;
use crate::helpers::models::problem::{test_fleet, TestActivityCost, TestTransportCost};
use crate::models::common::IdDimension;
use crate::models::problem::{Fleet, Job, Jobs};
use crate::models::{ExtrasBuilder, Feature, Goal, GoalContext, Problem};
use rosomaxa::utils::{DefaultRandom, Random};
use std::sync::Arc;

pub fn test_random() -> Arc<dyn Random + Send + Sync> {
    Arc::new(DefaultRandom::default())
}

#[derive(Default)]
pub struct GoalContextBuilder {
    features: Vec<Feature>,
    goal: Option<Goal>,
}

impl GoalContextBuilder {
    pub fn with_transport_feature(schedule_keys: ScheduleKeys) -> Self {
        let mut builder = Self::default();
        builder
            .add_feature(
                create_minimize_transport_costs_feature(
                    "transport",
                    TestTransportCost::new_shared(),
                    TestActivityCost::new_shared(),
                    schedule_keys,
                    1,
                )
                .unwrap(),
            )
            .with_objectives(&["transport"]);

        builder
    }

    pub fn add_feature(&mut self, feature: Feature) -> &mut Self {
        self.features.push(feature);
        self
    }

    pub fn add_features(&mut self, feature: &[Feature]) -> &mut Self {
        self.features.extend(feature.iter().cloned());
        self
    }

    pub fn with_objectives(&mut self, objectives: &[&str]) -> &mut Self {
        let objectives: Vec<_> = objectives.iter().map(|name| name.to_string()).collect();

        self.goal = Some(Goal::no_alternatives(objectives.clone(), objectives));

        self
    }

    pub fn build(&mut self) -> GoalContext {
        let goal = if let Some(goal) = std::mem::take(&mut self.goal) {
            goal
        } else {
            Goal::no_alternatives::<&str, _>([], [])
        };
        GoalContext::new(self.features.as_ref(), goal).unwrap()
    }
}

/// Builds a problem. Please note, that the order of calling method matters.
pub struct ProblemBuilder(Problem);

impl Default for ProblemBuilder {
    fn default() -> Self {
        Self(create_empty_problem())
    }
}

impl ProblemBuilder {
    pub fn with_fleet(&mut self, fleet: Fleet) -> &mut Self {
        self.0.fleet = Arc::new(fleet);
        self
    }

    pub fn with_jobs(&mut self, jobs: Vec<Job>) -> &mut Self {
        self.0.jobs = Arc::new(Jobs::new(&self.0.fleet, jobs, self.0.transport.as_ref()));
        self
    }

    pub fn with_goal(&mut self, goal: GoalContext) -> &mut Self {
        self.0.goal = Arc::new(goal);
        self
    }

    pub fn build(&mut self) -> Problem {
        std::mem::replace(&mut self.0, create_empty_problem())
    }
}

pub fn get_customer_ids_from_routes_sorted(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    let mut result = get_customer_ids_from_routes(insertion_ctx);
    result.sort();
    result
}

pub fn get_sorted_customer_ids_from_jobs(jobs: &[Job]) -> Vec<String> {
    let mut ids = get_customer_ids_from_jobs(jobs);
    ids.sort();
    ids
}

pub fn get_customer_ids_from_jobs(jobs: &[Job]) -> Vec<String> {
    jobs.iter().map(get_customer_id).collect()
}

pub fn get_customer_ids_from_routes(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route_ctx| {
            route_ctx
                .route()
                .tour
                .all_activities()
                .filter(|a| a.job.is_some())
                .map(|a| a.retrieve_job().unwrap())
                .map(|job| get_customer_id(&job))
                .collect::<Vec<String>>()
        })
        .collect()
}

pub fn get_customer_ids_from_unassigned(insertion_ctx: &InsertionContext) -> Vec<String> {
    let mut job_ids = insertion_ctx.solution.unassigned.iter().map(|(job, _)| get_customer_id(job)).collect::<Vec<_>>();

    job_ids.sort();

    job_ids
}

pub fn get_customer_id(job: &Job) -> String {
    job.dimens().get_id().unwrap().clone()
}

fn create_empty_problem() -> Problem {
    let transport = TestTransportCost::new_shared();
    let fleet = test_fleet();
    let jobs = Jobs::new(&fleet, vec![], transport.as_ref());

    Problem {
        fleet: Arc::new(fleet),
        jobs: Arc::new(jobs),
        locks: vec![],
        goal: Arc::new(GoalContextBuilder::default().build()),
        activity: TestActivityCost::new_shared(),
        transport,
        extras: Arc::new(ExtrasBuilder::default().build().expect("cannot build default")),
    }
}
