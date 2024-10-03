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
    let population = get_default_population(objective.clone(), environment, selection_size);
    let population = population.as_ref();

    let mut telemetry = Telemetry::new(TelemetryMode::None);
    let solution = VectorSolution::new(vec![], 0., vec![]);
    telemetry.on_initial(&solution, Timer::start());

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
