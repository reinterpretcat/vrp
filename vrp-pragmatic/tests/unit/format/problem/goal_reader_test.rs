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
                balance_tolerance: 0.0,
                anchors: HashMap::from([("v1_1".to_string(), vec![0])]),
                weights: None,
                allow_idle_drivers: false,
                quota: None,
            },
            Objective::MinimizeCost,
        ]),
    };
    let matrix = create_matrix_from_problem(&problem);

    let result = (problem, vec![matrix]).read_pragmatic();

    assert!(result.is_ok(), "expected goal with territory objective to build, got: {:?}", result.err());
}

// An empty `anchors` map used to select a solver-side derive path (medoid seeds + Hungarian
// driver→seed matching). That derivation is gone: the caller owns the territory, so an empty map
// is now simply a territory nobody holds. The problem must still READ — the format accepts it, and
// the objective it builds is inert (pinned at the feature level by
// `an_empty_anchor_map_leaves_the_objective_inert`) rather than rejected. Making the field
// mandatory is deliberately NOT done here: the caller emits `anchors: {}` on its degenerate paths
// (a chunk whose fleet or plan comes out empty), so rejecting it would turn a no-op solve into a
// failed run.
#[test]
fn builds_goal_with_territory_objective_when_anchors_are_empty() {
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
                balance_tolerance: 0.0,
                anchors: HashMap::new(),
                weights: None,
                allow_idle_drivers: false,
                quota: None,
            },
            Objective::MinimizeCost,
        ]),
    };
    let matrix = create_matrix_from_problem(&problem);

    let result = (problem, vec![matrix]).read_pragmatic();

    assert!(result.is_ok(), "expected an empty-anchor territory goal to build, got: {:?}", result.err());
}
