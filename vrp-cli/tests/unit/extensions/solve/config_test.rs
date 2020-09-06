use super::*;
use std::fs::File;

#[test]
fn can_read_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");

    let config = read_config(BufReader::new(file)).unwrap();

    let telemetry = config.telemetry.expect("no telemetry config");
    let logging = telemetry.logging.expect("no logging config");
    assert!(!logging.enabled);
    assert_eq!(logging.log_best, Some(100));
    assert_eq!(logging.log_population, Some(1000));
    let metrics = telemetry.metrics.unwrap();
    assert!(!metrics.enabled);
    assert_eq!(metrics.track_population, Some(1000));

    let population = config.population.expect("no population config");
    assert_eq!(population.size, Some(4));

    let initial = population.initial.expect("no initial population config");
    assert_eq!(initial.methods.unwrap().len(), 1);
    assert_eq!(initial.size, Some(1));

    let offspring = population.offspring.expect("no offspring config");
    assert_eq!(offspring.size, Some(4));

    let branching = offspring.branching.expect("no branching config");
    assert_eq!(branching.chance, Some(0.2));
    assert_eq!(branching.steepness, Some(3));
    assert_eq!(branching.generations, Some(MinMaxConfig { min: 4, max: 6 }));

    let termination = config.termination.expect("no termination config");
    assert_eq!(termination.max_time, Some(300));
    assert_eq!(termination.max_generations, Some(3000));

    let MutationConfig::RuinRecreate { ruins, recreates } = config.mutation.expect("cannot get mutation");
    assert_eq!(ruins.len(), 7);
    assert_eq!(recreates.len(), 6);
}
