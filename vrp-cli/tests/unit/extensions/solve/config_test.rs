use super::*;
use std::fs::File;
use vrp_core::models::examples::create_example_problem;

#[test]
fn can_read_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");

    let config = read_config(BufReader::new(file)).unwrap();

    let telemetry = config.telemetry.expect("no telemetry config");
    let logging = telemetry.logging.expect("no logging config");
    assert!(logging.enabled);
    assert_eq!(logging.log_best, Some(100));
    assert_eq!(logging.log_population, Some(1000));
    let metrics = telemetry.metrics.unwrap();
    assert!(!metrics.enabled);
    assert_eq!(metrics.track_population, Some(1000));

    let population = config.population.expect("no population config");
    assert_eq!(population.max_size, Some(2));

    let initial = population.initial.expect("no initial population config");
    assert_eq!(initial.methods.unwrap().len(), 1);
    assert_eq!(initial.size, Some(1));

    config.selection.expect("no selection config");

    let mutation_config = config.mutation.expect("cannot get mutation");
    match mutation_config {
        MutationType::Composite { inners, .. } => {
            assert_eq!(inners.len(), 3);
            match inners.first().unwrap() {
                MutationType::LocalSearch { probability, times, operators: inners } => {
                    assert_eq!(*probability, 0.05);
                    assert_eq!(*times, MinMaxConfig { min: 1, max: 2 });
                    assert_eq!(inners.len(), 4);
                }
                _ => unreachable!(),
            }

            match inners.get(1).unwrap() {
                MutationType::RuinRecreate { probability, ruins, recreates } => {
                    assert_eq!(*probability, 1.);
                    assert_eq!(ruins.len(), 6);
                    assert_eq!(recreates.len(), 8);
                }
                _ => unreachable!(),
            }

            match inners.last().unwrap() {
                MutationType::LocalSearch { probability, times, operators: inners } => {
                    assert_eq!(*probability, 0.01);
                    assert_eq!(*times, MinMaxConfig { min: 1, max: 2 });
                    assert_eq!(inners.len(), 4);
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    let termination = config.termination.expect("no termination config");
    assert_eq!(termination.max_time, Some(300));
    assert_eq!(termination.max_generations, Some(3000));
}

#[test]
fn can_create_builder_from_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");
    let config = read_config(BufReader::new(file)).unwrap();
    let problem = create_example_problem();

    let builder = create_builder_from_config(problem.clone(), &config).unwrap();

    assert_eq!(builder.config.problem.as_ref() as *const Problem, problem.as_ref() as *const Problem);
    assert_eq!(builder.config.population.max_size, 2);
    assert_eq!(builder.config.population.initial.size, 1);
    assert_eq!(builder.config.population.initial.individuals.len(), 0);
    assert_eq!(builder.config.population.initial.methods.len(), 1);
    assert_eq!(builder.max_time, Some(300));
    assert_eq!(builder.max_generations, Some(3000));
}

#[test]
fn can_create_default_config() {
    let config = Config::default();

    assert!(config.population.is_none());
    assert!(config.selection.is_none());
    assert!(config.mutation.is_none());
    assert!(config.termination.is_none());
    assert!(config.telemetry.is_none());
}
