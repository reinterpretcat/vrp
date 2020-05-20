use super::*;
use std::fs::File;

#[test]
fn can_read_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");

    let config = read_config(BufReader::new(file)).unwrap();

    assert!(config.logging.is_some());
    assert!(config.logging.unwrap().enabled);

    assert!(config.population.is_some());
    let population = config.population.unwrap();
    assert_eq!(population.initial_methods.unwrap().len(), 3);
    assert_eq!(population.initial_size.unwrap(), 2);
    assert_eq!(population.population_size.unwrap(), 4);
    assert_eq!(population.offspring_size.unwrap(), 4);
    assert_eq!(population.elite_size.unwrap(), 2);

    assert!(config.termination.is_some());
    let termination = config.termination.unwrap();
    assert_eq!(termination.max_time.unwrap(), 300);
    assert_eq!(termination.max_generations.unwrap(), 2000);

    let MutationConfig::RuinRecreate { ruins, recreates } = config.mutation.expect("cannot get mutation");
    assert_eq!(ruins.len(), 10);
    assert_eq!(recreates.len(), 6);
}
