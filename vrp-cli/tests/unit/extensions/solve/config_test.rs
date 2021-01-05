use super::*;
use std::fs::File;
use vrp_core::models::examples::create_example_problem;

#[test]
fn can_read_full_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");

    let config = read_config(BufReader::new(file)).unwrap();

    let telemetry = config.telemetry.expect("no telemetry config");
    let logging = telemetry.logging.expect("no logging config");
    assert!(logging.enabled);
    assert_eq!(logging.log_best, Some(100));
    assert_eq!(logging.log_population, Some(1000));
    assert_eq!(logging.dump_population, Some(false));
    let metrics = telemetry.metrics.unwrap();
    assert!(!metrics.enabled);
    assert_eq!(metrics.track_population, Some(1000));

    let evolution_config = config.evolution.expect("no evolution config");

    let initial = evolution_config.initial.expect("no initial population config");
    assert_eq!(initial.methods.unwrap().len(), 1);
    assert_eq!(initial.size, Some(1));

    match evolution_config.population.expect("no population config") {
        PopulationType::Rosomaxa {
            selection_size,
            max_elite_size,
            max_node_size,
            spread_factor,
            reduction_factor,
            distribution_factor,
            learning_rate,
            rebalance_memory,
            rebalance_count,
            exploration_ratio,
        } => {
            assert_eq!(selection_size, Some(12));
            assert_eq!(max_elite_size, Some(2));
            assert_eq!(max_node_size, Some(2));
            assert_eq!(spread_factor, Some(0.25));
            assert_eq!(reduction_factor, Some(0.1));
            assert_eq!(distribution_factor, Some(0.25));
            assert_eq!(learning_rate, Some(0.1));
            assert_eq!(rebalance_memory, Some(500));
            assert_eq!(rebalance_count, Some(4));
            assert_eq!(exploration_ratio, Some(0.9));
        }
        PopulationType::Elitism { .. } => unreachable!(),
    }

    let mutation_config = config.mutation.expect("cannot get mutation");
    match mutation_config {
        MutationType::Composite { inners, .. } => {
            assert_eq!(inners.len(), 4);
            match inners.first().unwrap() {
                MutationType::Decomposition { inner, repeat, probability } => {
                    assert!(inner.is_none());
                    assert_eq!(*repeat, 10);
                    match probability {
                        MutationProbabilityType::Context { threshold, phases } => {
                            assert_eq!(threshold.jobs, 800);
                            assert_eq!(threshold.routes, 6);
                            assert_eq!(phases.len(), 2);
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }

            match inners.get(1).unwrap() {
                MutationType::LocalSearch { probability, times, operators: inners } => {
                    assert_eq!(as_scalar_probability(probability), 0.05);
                    assert_eq!(*times, MinMaxConfig { min: 1, max: 2 });
                    assert_eq!(inners.len(), 3);
                }
                _ => unreachable!(),
            }

            match inners.get(2).unwrap() {
                MutationType::RuinRecreate { probability, ruins, recreates } => {
                    assert_eq!(as_scalar_probability(probability), 1.);
                    assert_eq!(ruins.len(), 6);
                    assert_eq!(recreates.len(), 10);
                }
                _ => unreachable!(),
            }

            match inners.last().unwrap() {
                MutationType::LocalSearch { probability, times, operators: inners } => {
                    assert_eq!(as_scalar_probability(probability), 0.01);
                    assert_eq!(*times, MinMaxConfig { min: 1, max: 2 });
                    assert_eq!(inners.len(), 3);
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    let termination = config.termination.expect("no termination config");
    assert_eq!(termination.max_time, Some(300));
    assert_eq!(termination.max_generations, Some(3000));

    let parallelism = config.environment.expect("no environment config").parallelism.expect("no parallelism config");
    assert_eq!(parallelism.num_thread_pools, 6);
    assert_eq!(parallelism.threads_per_pool, 8);
}

#[test]
fn can_create_builder_from_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");
    let config = read_config(BufReader::new(file)).unwrap();
    let problem = create_example_problem();

    let builder = create_builder_from_config(problem.clone(), &config).unwrap();

    assert!(builder.config.population.variation.is_some());
    assert_eq!(builder.config.problem.as_ref() as *const Problem, problem.as_ref() as *const Problem);
    assert_eq!(builder.config.population.initial.size, 1);
    assert_eq!(builder.config.population.initial.individuals.len(), 0);
    assert_eq!(builder.config.population.initial.methods.len(), 1);
    assert_eq!(builder.max_time, Some(300));
    assert_eq!(builder.max_generations, Some(3000));
}

#[test]
fn can_create_default_config() {
    let config = Config::default();

    assert!(config.evolution.is_none());
    assert!(config.mutation.is_none());
    assert!(config.termination.is_none());
    assert!(config.telemetry.is_none());
}

fn as_scalar_probability(probability: &MutationProbabilityType) -> f64 {
    match probability {
        MutationProbabilityType::Scalar { scalar } => *scalar,
        _ => unreachable!(),
    }
}
