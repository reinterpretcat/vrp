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
    let json = r#"{"type":"territory","proximity":"time","balance":"production-value","anchors":{"drv-1":4,"drv-2":9}}"#;
    let obj: Objective = serde_json::from_str(json).unwrap();
    match obj {
        Objective::Territory {
            proximity: TerritoryProximity::Time,
            balance: Some(BalancePeriodMetric::ProductionValue),
            balance_tolerance,
            anchors,
            allow_idle_drivers,
        } => {
            assert_eq!(anchors.get("drv-1"), Some(&4));
            assert!(!allow_idle_drivers, "defaults to false when omitted from JSON");
            assert_eq!(balance_tolerance, 0.05, "omitted balance_tolerance defaults to 5%");
        }
        _ => panic!("wrong variant"),
    }
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
    // Omitted anchors deserialize to an empty map, which selects the solver-side derive path.
    let json = r#"{"type":"territory","proximity":"distance"}"#;
    let obj: Objective = serde_json::from_str(json).unwrap();
    match obj {
        Objective::Territory { anchors, .. } => assert!(anchors.is_empty(), "omitted anchors select the derive path"),
        _ => panic!("wrong variant"),
    }
}
