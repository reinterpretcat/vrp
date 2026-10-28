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
                deficit_weight: 0.0,
                anchors: HashMap::from([("v1_1".to_string(), vec![0])]),
                weights: None,
                allow_idle_drivers: false,
                day_patch: false,
                quota: None,
                shares: None,
                pools: None,
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
// is a territory nobody holds and the objective it would build is inert — PULL and PUSH are zero.
// The caller no longer sends one (a chunk with no technicians or no jobs never reaches its writer),
// so this is refused rather than silently solved as if the objective had not been asked for. This
// test pins the refusal end-to-end through `read_pragmatic()`, i.e. that E1610 is actually reached
// on the real read path and not only when the check is called directly.
#[test]
fn refuses_goal_with_territory_objective_when_anchors_are_empty() {
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
                deficit_weight: 0.0,
                anchors: HashMap::new(),
                weights: None,
                allow_idle_drivers: false,
                day_patch: false,
                quota: None,
                shares: None,
                pools: None,
            },
            Objective::MinimizeCost,
        ]),
    };
    let matrix = create_matrix_from_problem(&problem);

    let result = (problem, vec![matrix]).read_pragmatic();

    let Err(errors) = result else {
        panic!("expected an empty-anchor territory objective to be refused");
    };
    assert!(
        errors.errors.iter().any(|err| err.code == "E1610"),
        "expected E1610 among the reported errors, got: {errors}"
    );
}

/// Both new keys are single words, so their snake_case and camelCase spellings coincide and
/// neither needs a serde `alias` — the trap `balance_tolerance` and `allow_idle_drivers` carry.
/// This pins that they survive a round trip through the wire rather than being silently dropped.
#[test]
fn territory_objective_reads_quota_shares_and_pools() {
    let json = r#"{
        "type": "territory",
        "proximity": "distance",
        "balance": "duration",
        "anchors": { "d1": [1], "d2": [2] },
        "shares": { "d1": 0.6, "d2": 0.4 },
        "pools": { "d1": "north", "d2": "north" }
    }"#;

    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::Territory { shares: Some(shares), pools: Some(pools), quota: None, .. } => {
            assert_eq!(shares.get("d1").copied(), Some(0.6));
            assert_eq!(shares.get("d2").copied(), Some(0.4));
            assert_eq!(pools.get("d1").map(String::as_str), Some("north"));
            assert_eq!(pools.get("d2").map(String::as_str), Some("north"));
        }
        other => unreachable!("expected a territory objective carrying shares and pools, got {other:?}"),
    }
}

fn read_problem_with_objective(objective: Objective) -> Result<vrp_core::models::Problem, MultiFormatError> {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (2., 0.)), create_delivery_job("job2", (8., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1_1".to_string()],
                shifts: vec![create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))],
                ..create_vehicle_with_capacity("v1", vec![10])
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![Objective::MinimizeUnassigned { breaks: None }, objective, Objective::MinimizeCost]),
    };
    let matrix = create_matrix_from_problem(&problem);

    (problem, vec![matrix]).read_pragmatic()
}

#[test]
fn route_targets_objective_reads_the_full_contract() {
    let json = r#"{
        "type": "route-targets",
        "aggregate": "max",
        "targets": [
            { "metric": "distance", "span": "depot-to-depot", "points": [[10000, 0], [20000, 1]] },
            { "metric": "driving-time", "span": "depot-to-last-job", "points": [[3600, 0], [7200, 1]] },
            { "metric": "duration", "span": "first-job-to-depot", "breaks": "include", "points": [[0, 0], [1, 1]] },
            { "metric": "service-time", "per": "hour", "points": [[1800, 1], [3600, 0]] },
            { "metric": "value", "per": "activity", "points": [[0, 2], [100, 0]] },
            { "metric": "activities", "points": [[0, 3], [10, 0]] }
        ]
    }"#;

    let objective = serde_json::from_str::<Objective>(json).unwrap();

    let Objective::RouteTargets { targets, aggregate: Some(RouteTargetAggregateJson::Max) } = &objective else {
        unreachable!("expected a route-targets objective with the max aggregate, got {objective:?}");
    };
    assert_eq!(targets.len(), 6);
    assert!(matches!(targets[0].span, Some(RouteCostSpan::DepotToDepot)));
    assert!(matches!(targets[1].span, Some(RouteCostSpan::DepotToLastJob)));
    assert_eq!(targets[1].metric, RouteTargetMetricJson::DrivingTime);
    assert_eq!(targets[2].breaks, Some(RouteTargetBreaksJson::Include));
    assert_eq!(targets[3].per, Some(RouteTargetPerJson::Hour));
    assert_eq!(targets[4].per, Some(RouteTargetPerJson::Activity));
    assert_eq!(targets[5].points, vec![(0., 3.), (10., 0.)]);
    assert!(read_problem_with_objective(objective).is_ok());
}

#[test]
fn route_targets_objective_leaves_omitted_options_to_their_defaults() {
    let json = r#"{"type": "route-targets", "targets": [{"metric": "distance", "points": [[0, 0], [1, 1]]}]}"#;

    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::RouteTargets { targets, aggregate: None } => {
            assert!(targets[0].span.is_none() && targets[0].breaks.is_none() && targets[0].per.is_none());
        }
        other => unreachable!("expected a route-targets objective without options, got {other:?}"),
    }
}

#[test]
fn route_targets_objective_accepts_total_distance() {
    let json = r#"{"type": "route-targets", "aggregate": "sum", "targets": [
        {"metric": "total-distance", "points": [[0, 0], [10000, 1]]}
    ]}"#;

    let objective = serde_json::from_str::<Objective>(json).unwrap();

    assert!(matches!(
        &objective,
        Objective::RouteTargets { targets, aggregate: Some(RouteTargetAggregateJson::Sum) }
            if targets[0].metric == RouteTargetMetricJson::TotalDistance
    ));
    assert!(read_problem_with_objective(objective).is_ok());
}

parameterized_test! {route_targets_objective_refuses_invalid_curve, (points, expected_message), {
    route_targets_objective_refuses_invalid_curve_impl(points, expected_message);
}}

route_targets_objective_refuses_invalid_curve! {
    case01_single_point: (vec![(0., 0.)], "at least two points"),
    case02_descending: (vec![(20., 0.), (10., 1.)], "strictly ascending"),
    case03_repeated_value: (vec![(10., 0.), (10., 1.)], "strictly ascending"),
}

fn route_targets_objective_refuses_invalid_curve_impl(points: Vec<(Float, Float)>, expected_message: &str) {
    let objective = Objective::RouteTargets {
        targets: vec![RouteTargetJson {
            metric: RouteTargetMetricJson::Distance,
            span: None,
            breaks: None,
            per: None,
            points,
        }],
        aggregate: None,
    };

    let Err(errors) = read_problem_with_objective(objective) else {
        panic!("expected an invalid curve to be refused");
    };
    let message = errors.to_string();
    assert!(message.contains("route-targets objective, target 0"), "unexpected error: {message}");
    assert!(message.contains(expected_message), "unexpected error: {message}");
}
