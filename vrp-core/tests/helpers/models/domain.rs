use crate::construction::features::TransportFeatureBuilder;
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost, test_fleet};
use crate::models::problem::JobIdDimension;
use crate::prelude::*;
use std::sync::Arc;

pub fn test_random() -> Arc<dyn Random> {
    Arc::new(DefaultRandom::default())
}

pub fn test_logger() -> InfoLogger {
    Arc::new(|_| ())
}

pub struct TestGoalContextBuilder {
    features: Vec<Feature>,
}

impl Default for TestGoalContextBuilder {
    fn default() -> Self {
        Self {
            features: vec![
                FeatureBuilder::default().with_name("default").with_objective(TestObjective).build().unwrap(),
            ],
        }
    }
}

impl TestGoalContextBuilder {
    pub fn empty() -> Self {
        Self { features: vec![] }
    }

    pub fn with_transport_feature() -> Self {
        Self {
            features: vec![
                TransportFeatureBuilder::new("transport")
                    .set_violation_code(ViolationCode(1))
                    .set_transport_cost(TestTransportCost::new_shared())
                    .set_activity_cost(TestActivityCost::new_shared())
                    .build_minimize_cost()
                    .unwrap(),
            ],
        }
    }

    pub fn add_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }

    pub fn add_features(mut self, feature: Vec<Feature>) -> Self {
        self.features.extend(feature);
        self
    }

    pub fn build(self) -> GoalContext {
        GoalContextBuilder::with_features(&self.features)
            .expect("cannot create builder")
            .build()
            .expect("cannot build context")
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
        self.0.jobs = Arc::new(Jobs::new(&self.0.fleet, jobs, self.0.transport.as_ref(), &test_logger()).unwrap());
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
    let mut job_ids = insertion_ctx.solution.unassigned.keys().map(get_customer_id).collect::<Vec<_>>();

    job_ids.sort();

    job_ids
}

pub fn get_customer_id(job: &Job) -> String {
    job.dimens().get_job_id().unwrap().clone()
}

fn create_empty_problem() -> Problem {
    let transport = TestTransportCost::new_shared();
    let fleet = test_fleet();
    let jobs = Jobs::new(&fleet, vec![], transport.as_ref(), &test_logger()).unwrap();

    Problem {
        fleet: Arc::new(fleet),
        jobs: Arc::new(jobs),
        locks: vec![],
        goal: Arc::new(TestGoalContextBuilder::default().build()),
        activity: TestActivityCost::new_shared(),
        transport,
        extras: Arc::new(Extras::default()),
    }
}

struct TestObjective;

impl FeatureObjective for TestObjective {
    fn fitness(&self, _: &InsertionContext) -> Cost {
        Cost::default()
    }

    fn estimate(&self, _: &MoveContext<'_>) -> Cost {
        Cost::default()
    }
}
