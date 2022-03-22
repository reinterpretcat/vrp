use super::*;
use crate::example::*;
use crate::helpers::example::{create_default_heuristic_context, create_example_objective};
use crate::utils::compare_floats;
use std::cmp::Ordering;

fn compare_statistic(statistics: &HeuristicStatistics, expected: (usize, f64, f64)) {
    assert_eq!(statistics.generation, expected.0);
    assert_eq!(compare_floats(statistics.improvement_all_ratio, expected.1), Ordering::Equal);
    assert_eq!(compare_floats(statistics.improvement_1000_ratio, expected.2), Ordering::Equal);
}

#[test]
fn can_update_statistic() {
    let mut heuristic_ctx = create_default_heuristic_context();
    let objective = create_example_objective();
    let mut telemetry = Telemetry::new(TelemetryMode::None);

    let solution = VectorSolution::new(vec![], objective);
    telemetry.on_initial(&solution, 0, 1, Timer::start());

    let statistics = telemetry.on_generation(&mut heuristic_ctx, 0., Timer::start(), true);
    compare_statistic(&statistics, (0, 1., 1.));

    let statistics = telemetry.on_generation(&mut heuristic_ctx, 0., Timer::start(), false);
    compare_statistic(&statistics, (1, 0.5, 0.5));

    telemetry.on_generation(&mut heuristic_ctx, 0., Timer::start(), false);
    let statistics = telemetry.on_generation(&mut heuristic_ctx, 0., Timer::start(), false);
    compare_statistic(&statistics, (3, 0.25, 0.25));

    let statistics =
        (0..996).fold(statistics, |_, _| telemetry.on_generation(&mut heuristic_ctx, 0., Timer::start(), false));
    compare_statistic(&statistics, (999, 0.001, 0.001));

    let statistics = telemetry.on_generation(&mut heuristic_ctx, 0., Timer::start(), true);
    compare_statistic(&statistics, (1000, 2. / 1001., 0.001));
}
