use super::*;
use crate::helpers::{SIMPLE_MATRIX, SIMPLE_PROBLEM};
use serde_json::from_str;
use std::io::BufReader;

fn assert_time_windows(actual: &Option<Vec<Vec<String>>>, expected: (&str, &str)) {
    let actual = actual.as_ref().unwrap();
    assert_eq!(actual.len(), 1);
    assert_eq!(actual.first().unwrap().len(), 2);
    assert_eq!(actual.first().unwrap().first().unwrap(), expected.0);
    assert_eq!(actual.first().unwrap().last().unwrap(), expected.1);
}

fn assert_location(actual: &Location, expected: (f64, f64)) {
    let (lat, lng) = actual.to_lat_lng();

    assert_eq!(lat, expected.0);
    assert_eq!(lng, expected.1);
}

fn assert_demand(actual: &Option<Vec<i32>>, expected: i32) {
    let actual = actual.as_ref().expect("Empty demand!");
    assert_eq!(actual.len(), 1);
    assert_eq!(*actual.first().unwrap(), expected);
}

#[test]
fn can_deserialize_problem() {
    let problem = deserialize_problem(BufReader::new(SIMPLE_PROBLEM.as_bytes())).ok().unwrap();

    assert_eq!(problem.plan.jobs.len(), 2);
    assert_eq!(problem.fleet.vehicles.len(), 1);
    assert!(problem.plan.relations.is_none());

    // validate jobs
    let job = problem.plan.jobs.first().unwrap();
    assert_eq!(job.id, "single_job");
    assert!(job.pickups.is_none());
    assert!(job.deliveries.is_some());
    assert!(job.skills.is_none());

    let deliveries = job.deliveries.as_ref().unwrap();
    assert_eq!(deliveries.len(), 1);
    let delivery = deliveries.first().unwrap();
    assert_demand(&delivery.demand, 1);
    assert!(delivery.places.first().unwrap().tag.is_none());

    assert_eq!(delivery.places.len(), 1);
    let place = delivery.places.first().unwrap();
    assert_eq!(place.duration, 240.);
    assert_location(&place.location, (52.5622847f64, 13.4023099f64));
    assert_time_windows(&place.times, ("2019-07-04T10:00:00Z", "2019-07-04T16:00:00Z"));

    let job = problem.plan.jobs.last().unwrap();
    assert_eq!(job.id, "multi_job");
    assert!(job.skills.is_none());
    assert_eq!(job.pickups.as_ref().unwrap().len(), 2);
    assert_eq!(job.deliveries.as_ref().unwrap().len(), 1);
}

#[test]
fn can_deserialize_matrix() {
    let matrix = deserialize_matrix(BufReader::new(SIMPLE_MATRIX.as_bytes())).ok().unwrap();

    assert_eq!(matrix.distances.len(), 16);
    assert_eq!(matrix.travel_times.len(), 16);
}

#[test]
fn can_deserialize_job_production_value() {
    let job: Job = serde_json::from_str(r#"{ "id": "job1", "productionValue": 12.5 }"#).unwrap();

    assert_eq!(job.production_value, Some(12.5));
}

#[test]
fn can_deserialize_job_vehicle_group() {
    let job: Job = serde_json::from_str(r#"{ "id": "job1", "vehicleGroup": "sub-1" }"#).unwrap();

    assert_eq!(job.vehicle_group, Some("sub-1".to_string()));

    // The snake_case wire key must NOT be honoured: the field is camelCase-only on the wire.
    let job: Job = serde_json::from_str(r#"{ "id": "job1", "vehicle_group": "sub-1" }"#).unwrap();

    assert_eq!(job.vehicle_group, None);
}

#[test]
fn can_deserialize_balance_production_value_objective() {
    let objective: Objective = serde_json::from_str(r#"{ "type": "balance-production-value" }"#).unwrap();

    assert!(matches!(objective, Objective::BalanceProductionValue));
}

#[test]
fn can_deserialize_balance_period_objective_with_production_value_metric() {
    let objective: Objective = serde_json::from_str(r#"{ "type": "balance-period", "metric": "production-value" }"#)
        .expect("failed to deserialize objective");

    assert!(matches!(objective, Objective::BalancePeriod { metric: BalancePeriodMetric::ProductionValue }));
}

#[test]
fn can_deserialize_balance_period_objective_with_distance_metric() {
    let objective: Objective = serde_json::from_str(r#"{ "type": "balance-period", "metric": "distance" }"#)
        .expect("failed to deserialize objective");

    assert!(matches!(objective, Objective::BalancePeriod { metric: BalancePeriodMetric::Distance }));
}

#[test]
fn can_deserialize_balance_shifts_objective_with_saturation() {
    let objective: Objective = from_str(r#"{ "type": "balance-shifts", "saturation": 0.2, "weight": 3.5 }"#)
        .expect("failed to deserialize objective");

    match objective {
        Objective::BalanceShifts { saturation, weight } => {
            assert!((saturation.unwrap() - 0.2).abs() < 1e-9);
            assert!((weight.unwrap() - 3.5).abs() < 1e-9);
        }
        _ => panic!("unexpected objective variant"),
    }
}

#[test]
fn can_deserialize_territory_objective_with_anchors() {
    // Documents the wire shape: each driver's anchors are an ARRAY of routing-matrix indices, one
    // per patch of ground the driver works. `drv-2` holds two.
    let json = r#"{"type":"territory","proximity":"time","balance":"production-value","anchors":{"drv-1":[4],"drv-2":[9,11]}}"#;
    let obj: Objective = serde_json::from_str(json).unwrap();
    match obj {
        Objective::Territory {
            proximity: TerritoryProximity::Time,
            balance: Some(BalancePeriodMetric::ProductionValue),
            balance_tolerance,
            anchors,
            weights,
            allow_idle_drivers,
            quota,
        } => {
            assert_eq!(anchors.get("drv-1"), Some(&vec![4]));
            assert_eq!(anchors.get("drv-2"), Some(&vec![9, 11]));
            assert!(!allow_idle_drivers, "defaults to false when omitted from JSON");
            assert_eq!(balance_tolerance, 0.05, "omitted balance_tolerance defaults to 5%");
            assert!(quota.is_none(), "omitted quota selects the derive path");
            assert!(weights.is_none(), "omitted weights leave every power weight at 0");
        }
        _ => panic!("wrong variant"),
    }
}

/// Pins the exact payload a caller emits for the `territory` objective, with every optional field
/// present at once and in the spelling the field serializer produces. The three per-driver inputs
/// -- `anchors`, `weights`, `quota` -- are keyed identically (driver id, else vehicle id): `anchors`
/// to an ARRAY of routing-matrix indices, `weights` to the power weight `w_d`, and `quota` to a
/// scalar in the unit of `balance`. `balanceTolerance` and `allowIdleDrivers` arrive in camelCase
/// and are accepted through their aliases.
#[test]
fn can_deserialize_the_full_territory_objective_payload() {
    let json = r#"{
        "type": "territory",
        "proximity": "distance",
        "balance": "production-value",
        "balanceTolerance": 0.05,
        "allowIdleDrivers": true,
        "anchors": { "emp-17": [412, 986], "emp-23": [77] },
        "weights": { "emp-17": 1250.0, "emp-23": 0.0 },
        "quota": { "emp-17": 1840.5, "emp-23": 920.25 }
    }"#;

    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::Territory {
            proximity: TerritoryProximity::Distance,
            balance: Some(BalancePeriodMetric::ProductionValue),
            balance_tolerance,
            anchors,
            weights: Some(weights),
            allow_idle_drivers,
            quota: Some(quota),
        } => {
            assert_eq!(balance_tolerance, 0.05);
            assert!(allow_idle_drivers);
            assert_eq!(anchors.get("emp-17"), Some(&vec![412, 986]));
            assert_eq!(anchors.get("emp-23"), Some(&vec![77]));
            assert_eq!(weights.get("emp-17"), Some(&1250.0));
            assert_eq!(weights.get("emp-23"), Some(&0.0));
            assert_eq!(quota.get("emp-17"), Some(&1840.5));
            assert_eq!(quota.get("emp-23"), Some(&920.25));
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn can_deserialize_territory_objective_with_weights() {
    // Documents the wire shape: `weights` is a per-driver map keyed exactly like `anchors`, one
    // scalar per driver (not per anchor). Single-word key, so no camelCase alias is needed.
    let json = r#"{"type":"territory","proximity":"distance","anchors":{"drv-1":[4],"drv-2":[9]},"weights":{"drv-1":12.5,"drv-2":-3.0}}"#;
    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::Territory { weights: Some(weights), .. } => {
            assert_eq!(weights.get("drv-1"), Some(&12.5));
            assert_eq!(weights.get("drv-2"), Some(&-3.0), "a negative weight shrinks a cell and is legal");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn territory_objective_omits_absent_weights_when_serialized() {
    // Same reasoning as `quota`: `skip_serializing_if` keeps a round-tripped problem byte-identical
    // to what it was before the field existed, so it cannot silently gain an empty weight map.
    let objective = Objective::Territory {
        proximity: TerritoryProximity::Distance,
        balance: None,
        balance_tolerance: 0.05,
        anchors: Default::default(),
        weights: None,
        allow_idle_drivers: false,
        quota: None,
    };

    let json = serde_json::to_string(&objective).unwrap();

    assert!(!json.contains("weights"), "absent weights must not be serialized: {json}");
}

#[test]
fn can_deserialize_territory_objective_with_an_empty_anchor_list() {
    // An empty list is a first-class value on the wire: it says "this driver holds no contested
    // ground", which the objective reads as "takes no part in the territory". It must survive
    // deserialization as an empty list rather than being dropped or rejected.
    let json = r#"{"type":"territory","proximity":"distance","anchors":{"drv-1":[4],"drv-2":[]}}"#;
    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::Territory { anchors, .. } => {
            assert_eq!(anchors.get("drv-2"), Some(&Vec::new()));
            assert!(!anchors.is_empty(), "an empty list per driver is not an empty map");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn can_deserialize_territory_objective_with_quota() {
    // Documents the wire shape: `quota` is a per-driver map keyed the same way `anchors` is, in the
    // unit of the chosen balance metric. Single-word key, so no camelCase alias is needed.
    let json = r#"{"type":"territory","proximity":"distance","balance":"production-value","quota":{"drv-1":120.5,"drv-2":80.0}}"#;
    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::Territory { quota: Some(quota), .. } => {
            assert_eq!(quota.get("drv-1"), Some(&120.5));
            assert_eq!(quota.get("drv-2"), Some(&80.0));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn territory_objective_omits_an_absent_quota_when_serialized() {
    // `skip_serializing_if` keeps the derive path's serialized form byte-identical to what it was
    // before `quota` existed, so a round-trip cannot silently switch a problem onto the input path.
    let objective = Objective::Territory {
        proximity: TerritoryProximity::Distance,
        balance: None,
        balance_tolerance: 0.05,
        anchors: Default::default(),
        weights: None,
        allow_idle_drivers: false,
        quota: None,
    };

    let json = serde_json::to_string(&objective).unwrap();

    assert!(!json.contains("quota"), "an absent quota must not be serialized: {json}");
}

#[test]
fn can_deserialize_territory_objective_with_explicit_balance_tolerance() {
    // Documents the wire key: the Objective enum's `rename_all = "kebab-case"` renames variants,
    // not struct-variant fields, so the field stays snake_case `balance_tolerance` on the wire.
    let json = r#"{"type":"territory","proximity":"distance","balance_tolerance":0.3}"#;
    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::Territory { balance_tolerance, .. } => assert_eq!(balance_tolerance, 0.3),
        _ => panic!("wrong variant"),
    }

    // The camelCase alias is accepted too, matching the field serializer fieldrouting emits.
    let camel = r#"{"type":"territory","proximity":"distance","balanceTolerance":0.2}"#;
    match serde_json::from_str::<Objective>(camel).unwrap() {
        Objective::Territory { balance_tolerance, .. } => assert_eq!(balance_tolerance, 0.2),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn can_deserialize_territory_objective_with_camelcase_allow_idle_drivers() {
    // Regression: the camelCase key must be honoured, not silently dropped to the false default.
    let json = r#"{"type":"territory","proximity":"distance","allowIdleDrivers":true}"#;
    match serde_json::from_str::<Objective>(json).unwrap() {
        Objective::Territory { allow_idle_drivers, .. } => assert!(allow_idle_drivers),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn can_deserialize_territory_objective_without_anchors() {
    // Omitted anchors deserialize to an empty map. Nothing is derived from that any more: an empty
    // map is a territory no driver holds, so the objective it builds is inert.
    let json = r#"{"type":"territory","proximity":"distance"}"#;
    let obj: Objective = serde_json::from_str(json).unwrap();
    match obj {
        Objective::Territory { anchors, .. } => assert!(anchors.is_empty(), "omitted anchors read as an empty map"),
        _ => panic!("wrong variant"),
    }
}
