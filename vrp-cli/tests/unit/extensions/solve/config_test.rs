use super::*;
use std::fs::File;

#[test]
fn can_read_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");

    let config = read_config(BufReader::new(file)).unwrap();

    assert!(config.population.is_some());
    assert!(config.termination.is_some());

    let MutationConfig::RuinRecreate { ruins, recreates } = config.mutation.expect("cannot get mutation");
    assert_eq!(ruins.len(), 7);
    assert_eq!(recreates.len(), 6);
}
