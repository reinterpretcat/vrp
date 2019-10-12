use crate::helpers::construction::constraints::create_constraint_pipeline_with_timing;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::Schedule;
use crate::models::problem::{Fleet, Jobs, SimpleActivityCost};
use crate::models::solution::Registry;
use crate::models::{Extras, Problem, Solution};
use crate::objectives::{ObjectiveFunction, PenalizeUnassigned};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn can_calculate_cost_with_penalty_properly() {
    let fleet = Arc::new(Fleet::new(
        vec![test_driver()],
        vec![VehicleBuilder::new().id("v1").build(), VehicleBuilder::new().id("v2").build()],
    ));
    let route1 = create_route_with_start_end_activities(
        &fleet,
        "v1",
        test_tour_activity_with_schedule(Schedule::new(0., 0.)),
        test_tour_activity_with_schedule(Schedule::new(40., 40.)),
        vec![
            test_tour_activity_with_location_and_duration(10, 5.),
            test_tour_activity_with_location_and_duration(15, 5.),
        ],
    );
    let route2 = create_route_with_start_end_activities(
        &fleet,
        "v2",
        test_tour_activity_with_schedule(Schedule::new(0., 0.)),
        test_tour_activity_with_schedule(Schedule::new(11., 11.)),
        vec![test_tour_activity_with_location_and_duration(5, 1.)],
    );
    let activity = Arc::new(SimpleActivityCost::new());
    let transport = Arc::new(TestTransportCost::new());
    let constraint = Arc::new(create_constraint_pipeline_with_timing());
    let mut unassigned = HashMap::new();
    unassigned.insert(Arc::new(test_single_job()), 1);
    let problem = Problem {
        fleet: fleet.clone(),
        jobs: Arc::new(Jobs::new(&fleet, vec![], transport.as_ref())),
        locks: vec![],
        constraint,
        activity,
        transport,
        extras: Arc::new(Extras::default()),
    };
    let solution = Solution {
        registry: Registry::new(&fleet),
        routes: vec![route1, route2],
        unassigned,
        extras: Arc::new(Default::default()),
    };

    // vehicle or driver

    // route 1:
    // locations: 0 10 15 0
    // time: 40
    // driving: 30
    // fixed: 100

    // route 2:
    // locations: 0 5 0
    // time: 11
    // driving: 10
    // fixed: 100

    // total: 170 + 121 = 291

    let result = PenalizeUnassigned::new(1000.0).estimate(&problem, &solution);

    assert_eq!(result.actual, 582.0);
    assert_eq!(result.penalty, 1000.0);
}
