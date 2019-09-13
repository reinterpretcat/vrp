use crate::construction::heuristics::evaluators::InsertionEvaluator;
use crate::construction::states::{InsertionContext, InsertionResult, RouteContext, RouteState, SolutionContext};
use crate::helpers::construction::states::test_insertion_progress;
use crate::helpers::models::domain::create_empty_problem;
use crate::helpers::models::problem::*;
use crate::models::problem::Fleet;
use crate::models::solution::Registry;
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn can_insert_service_with_location_into_empty_tour() {
    let registry = Registry::new(&Fleet::new(
        vec![test_driver_with_costs(empty_costs())],
        vec![VehicleBuilder::new().id("v1").build()],
    ));
    let mut routes: HashSet<RouteContext> = HashSet::new();
    routes.insert(RouteContext::new(registry.next().next().unwrap()));
    let ctx = InsertionContext {
        progress: test_insertion_progress(),
        problem: create_empty_problem(),
        solution: Arc::new(SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned: Default::default(),
            routes,
            registry: Arc::new(registry),
        }),
        random: Arc::new("".to_string()),
    };

    let result = InsertionEvaluator::new().evaluate(&Arc::new(test_single_job()), &ctx);

    if let InsertionResult::Success(success) = result {
        assert_eq!(success.activities.len(), 1);
        assert_eq!(success.activities.first().unwrap().1, 0);
        assert_eq!(success.activities.first().unwrap().0.place.location, DEFAULT_JOB_LOCATION);
    } else {
        assert!(false);
    }
}
