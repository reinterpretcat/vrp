use crate::format::problem::*;
use crate::helpers::*;
use std::collections::HashMap;

// `GoalContext` does not expose feature names after it is built (no test in this repo, in either
// vrp-core or vrp-pragmatic, introspects post-build feature names — they only check `is_ok()`/
// `is_err()` or a feature's behavioral effect), so this test follows that same convention: it
// checks that a problem with a `territory` objective builds successfully end-to-end via
// `read_pragmatic()`. Before Task 6, the goal reader's `Objective::Territory` arm was a stub that
// unconditionally returned `Err`, so this failed red for that reason; wiring the real
// `TerritoryFeatureBuilder` arm makes it build.
#[test]
fn builds_goal_with_territory_objective() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (2., 0.))], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1_1".to_string()],
                shifts: vec![create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))],
                ..create_vehicle_with_capacity("v1", vec![10])
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            Objective::MinimizeUnassigned { breaks: None },
            Objective::Territory {
                proximity: TerritoryProximity::Distance,
                balance: Some(BalancePeriodMetric::Distance),
                anchors: HashMap::from([("v1_1".to_string(), 0)]),
                allow_idle_drivers: false,
            },
            Objective::MinimizeCost,
        ]),
    };
    let matrix = create_matrix_from_problem(&problem);

    let result = (problem, vec![matrix]).read_pragmatic();

    assert!(result.is_ok(), "expected goal with territory objective to build, got: {:?}", result.err());
}

// checks that a `territory` objective with NO anchors triggers the solver-side derive path
// (medoid seeds + value-balancing weights + Hungarian driver→seed matching) end-to-end, and that
// the goal builds. Two drivers and two jobs exercise a 2×2 matching.
#[test]
fn builds_goal_with_derived_territory_anchors_when_omitted() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (2., 0.)), create_delivery_job("job2", (8., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1_1".to_string(), "v1_2".to_string()],
                shifts: vec![create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))],
                ..create_vehicle_with_capacity("v1", vec![10])
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![
            Objective::MinimizeUnassigned { breaks: None },
            Objective::Territory {
                proximity: TerritoryProximity::Distance,
                balance: Some(BalancePeriodMetric::ProductionValue),
                anchors: HashMap::new(),
                allow_idle_drivers: false,
            },
            Objective::MinimizeCost,
        ]),
    };
    let matrix = create_matrix_from_problem(&problem);

    let result = (problem, vec![matrix]).read_pragmatic();

    assert!(result.is_ok(), "expected derived-anchor territory goal to build, got: {:?}", result.err());
}
