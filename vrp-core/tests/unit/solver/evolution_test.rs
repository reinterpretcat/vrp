use super::*;
use crate::models::examples::create_example_problem;

parameterized_test! {can_enable_telemetry_metrics, mode, {
        can_enable_telemetry_metrics_impl(mode);
}}

can_enable_telemetry_metrics! {
        case01: TelemetryMode::OnlyMetrics { track_population: 100 },
        case02: TelemetryMode::All { logger: Arc::new(|_| {}), log_best: 100, log_population: 1000,  track_population: 100},
}

fn can_enable_telemetry_metrics_impl(mode: TelemetryMode) {
    let config = EvolutionConfig { telemetry: Telemetry::new(mode), ..EvolutionConfig::new(create_example_problem()) };
    let evolution = EvolutionSimulator::new(config).unwrap();

    let (_, metrics) = evolution.run().unwrap();

    let metrics = metrics.expect("metrics are empty");
    assert_eq!(metrics.generations, 3000);
    assert_eq!(metrics.evolution.len(), 30 + 1);
    assert!(metrics.duration > 0);
    assert!(metrics.speed > 0.);
}

parameterized_test! {can_disable_telemetry_metrics, mode, {
        can_disable_telemetry_metrics_impl(mode);
}}

can_disable_telemetry_metrics! {
        case01: TelemetryMode::None,
        case02: TelemetryMode::OnlyLogging { logger: Arc::new(|_| {}), log_best: 100, log_population: 1000},
}

fn can_disable_telemetry_metrics_impl(mode: TelemetryMode) {
    let config = EvolutionConfig { telemetry: Telemetry::new(mode), ..EvolutionConfig::new(create_example_problem()) };
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
        ..EvolutionConfig::new(create_example_problem())
    };
    let evolution = EvolutionSimulator::new(config).unwrap();

    let (_, metrics) = evolution.run().unwrap();

    let metrics = metrics.expect("metrics are empty");
    assert_eq!(metrics.generations, 0);
    assert!(metrics.evolution.is_empty());
}
