use super::*;
use std::fs::File;
use vrp_core::models::examples::create_example_problem;

#[test]
fn can_read_full_config() {
    let file = File::open("../examples/data/config/config.full.json").expect("cannot read config from file");

    let config = read_config(BufReader::new(file)).unwrap();

    let telemetry = config.telemetry.expect("no telemetry config");
    let logging = telemetry.progress.expect("no logging config");
    assert!(logging.enabled);
    assert_eq!(logging.log_best, Some(100));
    assert_eq!(logging.log_population, Some(1000));
    assert_eq!(logging.dump_population, Some(false));
    let metrics = telemetry.metrics.unwrap();
    assert!(!metrics.enabled);
    assert_eq!(metrics.track_population, Some(1000));

    let evolution_config = config.evolution.expect("no evolution config");

    let initial = evolution_config.initial.expect("no initial population config");

    match initial.method {
        RecreateMethod::Cheapest { weight: 1 } => {}
        _ => unreachable!(),
    }
    assert_eq!(initial.alternatives.methods.len(), 7);
    assert_eq!(initial.alternatives.max_size, 4);
    assert_eq!(initial.alternatives.quota, 0.05);

    match evolution_config.population.expect("no population config") {
        PopulationType::Rosomaxa {
            selection_size,
            max_elite_size,
            max_node_size,
            spread_factor,
            distribution_factor,
            rebalance_memory,
            exploration_ratio,
        } => {
            assert_eq!(selection_size, Some(8));
            assert_eq!(max_elite_size, Some(2));
            assert_eq!(max_node_size, Some(2));
            assert_eq!(spread_factor, Some(0.75));
            assert_eq!(distribution_factor, Some(0.75));
            assert_eq!(rebalance_memory, Some(100));
            assert_eq!(exploration_ratio, Some(0.9));
        }
        _ => unreachable!(),
    }

    let hyper_config = config.hyper.expect("cannot get hyper");
    match hyper_config {
        HyperType::StaticSelective { operators } => {
            let operators = operators.expect("cannot get operators");
            assert_eq!(operators.len(), 4);
            match operators.first().unwrap() {
                SearchOperatorType::Decomposition { routes, repeat, probability } => {
                    assert_eq!(*repeat, 4);
                    assert_eq!(routes.min, 2);
                    assert_eq!(routes.max, 4);
                    match probability {
                        OperatorProbabilityType::Context { threshold, phases } => {
                            assert_eq!(threshold.jobs, 300);
                            assert_eq!(threshold.routes, 10);
                            assert_eq!(phases.len(), 2);
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }

            match operators.get(1).unwrap() {
                SearchOperatorType::LocalSearch { probability, times, operators: inners } => {
                    assert_eq!(as_scalar_probability(probability), 0.05);
                    assert_eq!(*times, MinMaxConfig { min: 1, max: 2 });
                    assert_eq!(inners.len(), 4);
                }
                _ => unreachable!(),
            }

            match operators.get(2).unwrap() {
                SearchOperatorType::RuinRecreate { probability, ruins, recreates } => {
                    assert_eq!(as_scalar_probability(probability), 1.);
                    assert_eq!(ruins.len(), 7);
                    assert_eq!(recreates.len(), 12);
                }
                _ => unreachable!(),
            }

            match operators.last().unwrap() {
                SearchOperatorType::LocalSearch { probability, times, operators: inners } => {
                    assert_eq!(as_scalar_probability(probability), 0.01);
                    assert_eq!(*times, MinMaxConfig { min: 1, max: 2 });
                    assert_eq!(inners.len(), 4);
                }
                _ => unreachable!(),
            }
        }
        HyperType::DynamicSelective => unreachable!(),
    }

    let termination = config.termination.expect("no termination config");
    assert_eq!(termination.max_time, Some(300));
    assert_eq!(termination.max_generations, Some(3000));

    let environment = config.environment.expect("no environment config");
    assert_eq!(environment.is_experimental, Some(false));

    let parallelism = environment.parallelism.expect("no parallelism config");
    assert_eq!(parallelism.num_thread_pools, 6);
    assert_eq!(parallelism.threads_per_pool, 8);

    let logging = environment.logging.expect("no logging config");
    assert!(logging.enabled);
    assert_eq!(logging.prefix, Some("[config.full]".to_string()));

    let output_cfg = config.output.expect("cannot read output config");
    assert_eq!(output_cfg.include_geojson, Some(true));
}

#[test]
fn can_create_default_config() {
    let config = Config::default();

    assert!(config.evolution.is_none());
    assert!(config.hyper.is_none());
    assert!(config.termination.is_none());
    assert!(config.telemetry.is_none());
}

#[test]
fn can_configure_telemetry_metrics() {
    let config = Config {
        evolution: None,
        hyper: None,
        termination: Some(TerminationConfig { max_time: None, max_generations: Some(100), variation: None }),
        environment: None,
        telemetry: Some(TelemetryConfig {
            progress: None,
            metrics: Some(MetricsConfig { enabled: true, track_population: Some(10) }),
        }),
        output: None,
    };

    let solution = create_builder_from_config(create_example_problem(), Vec::default(), &config)
        .and_then(|config_builder| config_builder.build())
        .map(|evolution_config| Solver::new(create_example_problem(), evolution_config))
        .and_then(|solver| solver.solve())
        .unwrap();

    let metrics = solution.telemetry.expect("no metrics");
    assert_eq!(metrics.generations, 100);
    assert_eq!(metrics.evolution.len(), 10 + 1);
}

fn as_scalar_probability(probability: &OperatorProbabilityType) -> f64 {
    match probability {
        OperatorProbabilityType::Scalar { scalar } => *scalar,
        _ => unreachable!(),
    }
}
