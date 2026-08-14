use super::*;
use std::fs::File;
use std::io::BufReader;
use vrp_pragmatic::format::problem::{PragmaticProblem, deserialize_problem};

/// 100 deliveries served by `vehicle_1..vehicle_5`, none of which carries a driver id — so the
/// derivation keys on vehicle ids, which is the fallback the caller's fleet also relies on.
const PROBLEM_PATH: &str = "../examples/data/pragmatic/benches/simple.deliveries.100.json";

fn read_problem() -> CoreProblem {
    let reader = BufReader::new(File::open(PROBLEM_PATH).expect("cannot read problem file"));
    deserialize_problem(reader).unwrap().read_pragmatic().unwrap()
}

fn settings(balance: Option<BalancePeriodMetric>) -> TerritorySettings {
    TerritorySettings { proximity: TerritoryProximity::Distance, balance }
}

#[test]
fn can_derive_one_anchor_and_weight_per_driver() {
    let problem = read_problem();

    let json = get_territory_derivation(&problem, &settings(Some(BalancePeriodMetric::ProductionValue))).unwrap();
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();

    let anchors = report["anchors"].as_object().unwrap();
    let weights = report["weights"].as_object().unwrap();
    assert_eq!(anchors.len(), 5, "one anchor list per vehicle: {json}");
    assert_eq!(weights.len(), 5, "one weight per vehicle: {json}");
    for id in ["vehicle_1", "vehicle_2", "vehicle_3", "vehicle_4", "vehicle_5"] {
        assert_eq!(anchors[id].as_array().unwrap().len(), 1, "the derive path places exactly one seed per driver");
        assert!(weights[id].is_number());
    }
}

#[test]
fn derived_anchors_are_distinct_locations() {
    let problem = read_problem();

    let json = get_territory_derivation(&problem, &settings(Some(BalancePeriodMetric::ProductionValue))).unwrap();
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();

    let mut seeds = report["anchors"]
        .as_object()
        .unwrap()
        .values()
        .map(|list| list.as_array().unwrap()[0].as_u64().unwrap())
        .collect::<Vec<_>>();
    seeds.sort_unstable();
    let distinct = seeds.len();
    seeds.dedup();

    assert_eq!(seeds.len(), distinct, "each driver gets its own seed: the Hungarian match is injective");
}

#[test]
fn dump_is_byte_stable_across_runs() {
    // The whole point of the dump: a port of the derivation is checked against it with a plain
    // diff, which only works if repeated runs in one process agree byte for byte. Cross-process
    // stability (where hash-order randomisation would show up) is covered by the sorted maps.
    let problem = read_problem();
    let settings = settings(Some(BalancePeriodMetric::ProductionValue));

    let first = get_territory_derivation(&problem, &settings).unwrap();
    let second = get_territory_derivation(&problem, &settings).unwrap();

    assert_eq!(first, second);
}

#[test]
fn dump_keys_are_sorted() {
    let problem = read_problem();

    let json = get_territory_derivation(&problem, &settings(None)).unwrap();

    let anchor_keys = json
        .lines()
        .skip_while(|line| !line.contains("\"anchors\""))
        .take_while(|line| !line.contains("\"weights\""))
        .filter(|line| line.contains("vehicle_"))
        .collect::<Vec<_>>();
    let mut sorted = anchor_keys.clone();
    sorted.sort_unstable();

    assert_eq!(anchor_keys, sorted, "sorted output is what makes a diff meaningful: {json}");
}

#[test]
fn dump_echoes_the_settings_it_ran_under() {
    // Two dumps taken under different metrics are not comparable, so the metrics travel with them.
    let problem = read_problem();

    let json = get_territory_derivation(&problem, &settings(Some(BalancePeriodMetric::Duration))).unwrap();
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(report["proximity"], "distance");
    assert_eq!(report["balance"], "duration");
}

#[test]
fn dump_omits_an_absent_balance() {
    let problem = read_problem();

    let json = get_territory_derivation(&problem, &settings(None)).unwrap();
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(report.get("balance").is_none(), "no balance metric means no key: {json}");
}

#[test]
fn can_find_territory_settings_nested_in_a_multi_objective() {
    let api_problem = deserialize_problem(BufReader::new(
        r#"{
            "plan": { "jobs": [] },
            "fleet": { "vehicles": [], "profiles": [] },
            "objectives": [
                { "type": "minimize-unassigned" },
                { "type": "multi-objective", "strategy": { "name": "sum" }, "objectives": [
                    { "type": "minimize-cost" },
                    { "type": "territory", "proximity": "time", "balance": "production-value" }
                ]}
            ]
        }"#
        .as_bytes(),
    ))
    .unwrap();

    let settings = find_territory_settings(&api_problem).expect("territory objective should be found when nested");

    assert!(matches!(settings.proximity, TerritoryProximity::Time));
    assert!(matches!(settings.balance, Some(BalancePeriodMetric::ProductionValue)));
}

#[test]
fn finds_no_territory_settings_when_the_problem_has_no_territory_objective() {
    let api_problem = deserialize_problem(BufReader::new(
        r#"{
            "plan": { "jobs": [] },
            "fleet": { "vehicles": [], "profiles": [] },
            "objectives": [{ "type": "minimize-cost" }]
        }"#
        .as_bytes(),
    ))
    .unwrap();

    assert!(find_territory_settings(&api_problem).is_none());
}
