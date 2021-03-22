use super::*;
use crate::helpers::solver::create_default_refinement_ctx;
use crate::models::examples::create_example_problem;
use crate::utils::compare_floats;
use std::cmp::Ordering;

fn compare_statistic(refinement_ctx: &RefinementContext, expected: (usize, f64, f64)) {
    assert_eq!(refinement_ctx.statistics.generation, expected.0);
    assert_eq!(compare_floats(refinement_ctx.statistics.improvement_all_ratio, expected.1), Ordering::Equal);
    assert_eq!(compare_floats(refinement_ctx.statistics.improvement_1000_ratio, expected.2), Ordering::Equal);
}

#[test]
fn can_update_statistic() {
    let mut refinement_ctx = create_default_refinement_ctx(create_example_problem());
    let mut telemetry = Telemetry::new(TelemetryMode::None);
    telemetry.start();
    telemetry.on_initial(0, 1, Timer::start(), 0.);

    telemetry.on_generation(&mut refinement_ctx, 0., Timer::start(), true);
    compare_statistic(&refinement_ctx, (0, 1., 1.));

    telemetry.on_generation(&mut refinement_ctx, 0., Timer::start(), false);
    compare_statistic(&refinement_ctx, (1, 0.5, 0.5));

    telemetry.on_generation(&mut refinement_ctx, 0., Timer::start(), false);
    telemetry.on_generation(&mut refinement_ctx, 0., Timer::start(), false);
    compare_statistic(&refinement_ctx, (3, 0.25, 0.25));

    (0..996).for_each(|_| {
        telemetry.on_generation(&mut refinement_ctx, 0., Timer::start(), false);
    });
    compare_statistic(&refinement_ctx, (999, 0.001, 0.001));

    telemetry.on_generation(&mut refinement_ctx, 0., Timer::start(), true);
    compare_statistic(&refinement_ctx, (1000, 2. / 1001., 0.001));
}
