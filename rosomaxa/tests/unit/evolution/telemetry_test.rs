use super::*;
use crate::example::*;
use crate::helpers::example::create_example_objective;
use crate::{get_default_population, get_default_selection_size};
use std::sync::Arc;

fn compare_statistic(statistics: &HeuristicStatistics, expected: (usize, Float, Float)) {
    assert_eq!(statistics.generation, expected.0);
    assert_eq!(statistics.improvement_all_ratio, expected.1);
    assert_eq!(statistics.improvement_1000_ratio, expected.2);
}

#[test]
fn can_update_statistic() {
    let environment = Arc::new(Environment::default());
    let objective = create_example_objective();
    let selection_size = get_default_selection_size(environment.as_ref());
    let population = get_default_population(objective.clone(), VectorRosomaxaContext, environment, selection_size);
    let population = population.as_ref();

    let mut telemetry = Telemetry::new(TelemetryMode::None);
    let solution = VectorSolution::new(vec![], 0., vec![]);
    telemetry.on_initial(&solution, "test", Timer::start());

    telemetry.on_generation(population, 0., Timer::start(), true);
    compare_statistic(telemetry.get_statistics(), (0, 1., 1.));

    telemetry.on_generation(population, 0., Timer::start(), false);
    compare_statistic(telemetry.get_statistics(), (1, 0.5, 0.5));

    telemetry.on_generation(population, 0., Timer::start(), false);
    telemetry.on_generation(population, 0., Timer::start(), false);
    compare_statistic(telemetry.get_statistics(), (3, 0.25, 0.25));

    (0..996).for_each(|_| telemetry.on_generation(population, 0., Timer::start(), false));
    compare_statistic(telemetry.get_statistics(), (999, 0.001, 0.001));

    telemetry.on_generation(population, 0., Timer::start(), true);
    compare_statistic(telemetry.get_statistics(), (1000, 2. / 1001., 0.001));
}

#[test]
fn can_recover_from_slow_speed() {
    let mut tracker = SpeedTracker::default();

    tracker.track_elapsed(0, 0, 0., SelectionPhase::Initial);
    tracker.track_elapsed(1, 1, 0.15, SelectionPhase::Initial);

    assert!(matches!(tracker.get_current_speed(), HeuristicSpeed::Slow { ratio, .. } if ratio == 0.1));

    tracker.track_elapsed(200, 2, 0.15, SelectionPhase::Initial);

    assert!(matches!(tracker.get_current_speed(), HeuristicSpeed::Moderate { .. }));
}

#[test]
fn can_track_sub_millisecond_generation_duration() {
    let mut tracker = SpeedTracker::default();

    tracker.track_elapsed(0, 0, 0., SelectionPhase::Exploration);
    (1..=11).for_each(|generation| {
        tracker.track_elapsed(generation, generation as u128 * 250, 0., SelectionPhase::Exploration);
    });

    assert_eq!(tracker.duration_median.approx_median(), Some(250));
    assert_eq!(tracker.get_current_speed().get_median(), Some(1));
}

#[test]
fn can_track_generation_duration_per_phase() {
    let mut tracker = SpeedTracker::default();

    tracker.track_elapsed(0, 0, 0., SelectionPhase::Exploration);
    (1..=11).for_each(|generation| {
        tracker.track_elapsed(generation, generation as u128 * 12_000, 0., SelectionPhase::Exploration);
    });
    assert_eq!(tracker.duration_median.approx_median(), Some(12_000));

    tracker.track_elapsed(12, 132_250, 0., SelectionPhase::Exploitation);

    assert_eq!(tracker.duration_median.approx_median(), Some(250));
    assert_eq!(tracker.get_current_speed().get_median(), Some(1));
}

#[test]
fn can_convert_duration_for_public_speed_and_log() {
    assert_eq!(duration_to_millis(250), 1);
    assert_eq!(duration_to_millis(12_000), 12);
    assert_eq!(duration_to_millis(12_001), 13);
    assert_eq!(format_duration(250), "250µs");
    assert_eq!(format_duration(12_345), "12.35ms");
}
