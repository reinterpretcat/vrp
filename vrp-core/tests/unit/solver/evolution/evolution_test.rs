use super::*;
use crate::construction::Quota;
use crate::models::examples::create_example_problem;
use crate::solver::TelemetryMode;
use crate::utils::Environment;
use std::sync::Arc;

parameterized_test! {can_enable_telemetry_metrics, (mode, evolution_size), {
        can_enable_telemetry_metrics_impl(mode, evolution_size);
}}

can_enable_telemetry_metrics! {
        case01: (TelemetryMode::OnlyMetrics { track_population: 100 }, 31),
        case02: (TelemetryMode::OnlyMetrics { track_population: 99 }, 32),
        case03: (TelemetryMode::All {
            logger: Arc::new(|_| {}), log_best: 100, log_population: 1000,
            track_population: 100, dump_population: false,
        }, 31),
}

fn can_enable_telemetry_metrics_impl(mode: TelemetryMode, evolution_size: usize) {
    let config = EvolutionConfig {
        telemetry: Telemetry::new(mode),
        ..EvolutionConfig::new(create_example_problem(), Arc::new(Environment::default()))
    };
    let evolution = EvolutionSimulator::new(config).unwrap();

    let (_, metrics) = evolution.run().unwrap();

    let metrics = metrics.expect("metrics are empty");
    assert_eq!(metrics.generations, 3000);
    assert_eq!(metrics.evolution.len(), evolution_size);
    assert!(metrics.duration > 0);
    assert!(metrics.speed > 0.);
}

parameterized_test! {can_disable_telemetry_metrics, mode, {
        can_disable_telemetry_metrics_impl(mode);
}}

can_disable_telemetry_metrics! {
        case01: TelemetryMode::None,
        case02: TelemetryMode::OnlyLogging {
            logger: Arc::new(|_| {}), log_best: 100, log_population: 1000, dump_population: false
        },
}

fn can_disable_telemetry_metrics_impl(mode: TelemetryMode) {
    let config = EvolutionConfig {
        telemetry: Telemetry::new(mode),
        ..EvolutionConfig::new(create_example_problem(), Arc::new(Environment::default()))
    };
    let evolution = EvolutionSimulator::new(config).unwrap();

    let (_, metrics) = evolution.run().unwrap();

    assert!(metrics.is_none())
}

#[test]
fn can_use_quota() {
    struct FullQuota {}

    impl Quota for FullQuota {
        fn is_reached(&self) -> bool {
            true
        }
    }

    let config = EvolutionConfig {
        quota: Some(Arc::new(FullQuota {})),
        telemetry: Telemetry::new(TelemetryMode::OnlyMetrics { track_population: 100 }),
        ..EvolutionConfig::new(create_example_problem(), Arc::new(Environment::default()))
    };
    let evolution = EvolutionSimulator::new(config).unwrap();

    let (_, metrics) = evolution.run().unwrap();

    let metrics = metrics.expect("metrics are empty");
    assert_eq!(metrics.generations, 0);
    assert_eq!(metrics.evolution.len(), 1);
}
