use crate::construction::constraints::ConstraintPipeline;
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use crate::helpers::objectives::TestObjectiveFunction;
use crate::models::problem::{Fleet, Jobs};
use crate::models::Problem;
use std::borrow::Borrow;
use std::sync::Arc;

pub fn create_empty_problem_with_constraint(constraint: ConstraintPipeline) -> Arc<Problem> {
    let transport = Arc::new(TestTransportCost::new());
    let fleet = Arc::new(Fleet::new(vec![], vec![]));
    let jobs = Arc::new(Jobs::new(fleet.borrow(), vec![], transport.as_ref()));
    Arc::new(Problem {
        fleet,
        jobs,
        locks: vec![],
        constraint: Arc::new(constraint),
        objective: Arc::new(TestObjectiveFunction::new()),
        activity: Arc::new(TestActivityCost::new()),
        transport,
        extras: Arc::new(Default::default()),
    })
}

pub fn create_empty_problem() -> Arc<Problem> {
    create_empty_problem_with_constraint(ConstraintPipeline::new())
}
