use crate::construction::states::{InsertionContext, RouteContext, RouteState, SolutionContext};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_timing;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::Schedule;
use crate::models::problem::{Fleet, Jobs, SimpleActivityCost};
use crate::models::solution::Registry;
use crate::models::{Extras, Problem};
use crate::refinement::objectives::{Objective, PenalizeUnassigned};
use crate::utils::DefaultRandom;
use hashbrown::HashMap;
use std::sync::Arc;

#[test]
fn can_calculate_cost_with_penalty_properly() {
    let fleet = Arc::new(Fleet::new(
        vec![test_driver()],
        vec![
            VehicleBuilder::new().id("v1").costs(fixed_costs()).build(),
            VehicleBuilder::new().id("v2").costs(fixed_costs()).build(),
        ],
    ));
    let route1 = RouteContext {
        route: Arc::new(create_route_with_start_end_activities(
            &fleet,
            "v1",
            test_tour_activity_with_schedule(Schedule::new(0., 0.)),
            test_tour_activity_with_schedule(Schedule::new(40., 40.)),
            vec![
                test_tour_activity_with_location_and_duration(10, 5.),
                test_tour_activity_with_location_and_duration(15, 5.),
            ],
        )),
        state: Arc::new(RouteState::default()),
    };
    let route2 = RouteContext {
        route: Arc::new(create_route_with_start_end_activities(
            &fleet,
            "v2",
            test_tour_activity_with_schedule(Schedule::new(0., 0.)),
            test_tour_activity_with_schedule(Schedule::new(11., 11.)),
            vec![test_tour_activity_with_location_and_duration(5, 1.)],
        )),
        state: Arc::new(RouteState::default()),
    };
    let activity = Arc::new(SimpleActivityCost::default());
    let transport = Arc::new(TestTransportCost::new());
    let constraint = Arc::new(create_constraint_pipeline_with_timing());
    let mut unassigned = HashMap::new();
    unassigned.insert(Arc::new(test_single_job()), 1);
    let insertion_ctx = InsertionContext {
        problem: Arc::new(Problem {
            fleet: fleet.clone(),
            jobs: Arc::new(Jobs::new(&fleet, vec![], transport.as_ref())),
            locks: vec![],
            constraint,
            activity,
            transport,
            objective: Arc::new(PenalizeUnassigned::default()),
            extras: Arc::new(Extras::default()),
        }),
        solution: SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned,
            locked: Default::default(),
            routes: vec![route1, route2],
            registry: Registry::new(&fleet),
        },
        random: Arc::new(DefaultRandom::default()),
    };

    // vehicle + driver

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

    // total: (70 * 2 + 100) + (21 * 2 + 100) = 382

    let result = PenalizeUnassigned::new(1000.0).estimate(&insertion_ctx);

    assert_eq!(result.actual, 382.0);
    assert_eq!(result.penalty, 1000.0);
}
