use super::*;
use std::fs::File;

#[test]
fn can_read_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");

    let config = read_config(BufReader::new(file)).unwrap();

    let telemetry = config.telemetry.expect("cannot get telemetry");
    let logging = telemetry.logging.unwrap();
    assert!(!logging.enabled);
    assert_eq!(logging.log_best, Some(100));
    assert_eq!(logging.log_population, Some(1000));
    let metrics = telemetry.metrics.unwrap();
    assert!(!metrics.enabled);
    assert_eq!(metrics.track_population, Some(1000));

    assert!(config.population.is_some());
    let population = config.population.unwrap();
    assert_eq!(population.initial_methods.unwrap().len(), 3);
    assert_eq!(population.initial_size, Some(2));
    assert_eq!(population.population_size, Some(4));
    assert_eq!(population.offspring_size, Some(4));
    assert_eq!(population.elite_size, Some(2));

    assert!(config.termination.is_some());
    let termination = config.termination.unwrap();
    assert_eq!(termination.max_time, Some(300));
    assert_eq!(termination.max_generations, Some(2000));

    let MutationConfig::RuinRecreate { ruins, recreates } = config.mutation.expect("cannot get mutation");
    assert_eq!(ruins.len(), 10);
    assert_eq!(recreates.len(), 6);
}
