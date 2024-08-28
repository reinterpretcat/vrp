use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use vrp_pragmatic::core::construction::heuristics::InsertionContext;
use vrp_pragmatic::core::prelude::*;
use vrp_pragmatic::core::rosomaxa::evolution::TelemetryMode;
use vrp_pragmatic::core::solver::search::{Recreate, RecreateWithCheapest};
use vrp_pragmatic::core::solver::{create_elitism_population, RefinementContext};
use vrp_pragmatic::format::problem::PragmaticProblem;

fn get_problem(problem_path: &str) -> Arc<Problem> {
    let file = File::open(problem_path).unwrap_or_else(|err| panic!("cannot open {} file: '{}'", problem_path, err));
    Arc::new(
        BufReader::new(file)
            .read_pragmatic()
            .unwrap_or_else(|errs| panic!("cannot create pragmatic problem: {}", errs)),
    )
}

fn get_solver(problem_path: &str, generations: usize) -> Solver {
    let problem = get_problem(problem_path);

    VrpConfigBuilder::new(problem.clone())
        .set_telemetry_mode(TelemetryMode::None)
        .prebuild()
        .expect("cannot prebuild configuration")
        .with_max_generations(Some(generations))
        .build()
        .map(|config| Solver::new(problem, config))
        .expect("cannot build solver")
}

fn get_refinement_ctx(problem_path: &str) -> RefinementContext {
    let problem = get_problem(problem_path);

    let environment = Arc::new(Environment::default());
    RefinementContext::new(
        problem.clone(),
        Box::new(create_elitism_population(problem.goal.clone(), environment.clone())),
        TelemetryMode::None,
        environment.clone(),
    )
}

/// Solve problem using cheapest insertion heuristic and returns one solution.
fn solve_problem_with_recreate_cheapest(refinement_ctx: &RefinementContext) -> InsertionContext {
    RecreateWithCheapest::new(refinement_ctx.environment.random.clone())
        .run(&refinement_ctx, InsertionContext::new(refinement_ctx.problem.clone(), refinement_ctx.environment.clone()))
}

fn simple_deliveries_100_benchmark(c: &mut Criterion) {
    c.bench_function("a problem with 100 trivial deliveries", |b| {
        let solver = get_solver("../data/pragmatic/benches/simple.deliveries.100.json", 100);
        b.iter(|| black_box(solver.solve().expect("cannot solve the problem")))
    });
}

fn simple_reload_100_benchmark(c: &mut Criterion) {
    c.bench_function("a problem with 100 trivial deliveries and one reload", |b| {
        let solver = get_solver("../data/pragmatic/benches/simple.reload.100.json", 100);
        b.iter(|| black_box(solver.solve().expect("cannot solve the problem")))
    });
}

fn simple_multi_job_100_benchmark(c: &mut Criterion) {
    c.bench_function("a problem with 50 multi jobs", |b| {
        let solver = get_solver("../data/pragmatic/benches/multi-job.100.json", 100);
        b.iter(|| black_box(solver.solve().expect("cannot solve the problem")))
    });
}

fn init_deliveries_100_benchmark(c: &mut Criterion) {
    c.bench_function("init solution for a problem with 100 trivial deliveries", |b| {
        let insertion_ctx = get_refinement_ctx("../data/pragmatic/benches/simple.deliveries.100.json");
        b.iter(|| black_box(solve_problem_with_recreate_cheapest(&insertion_ctx)))
    });
}

fn init_multi_job_100_benchmark(c: &mut Criterion) {
    c.bench_function("init solution for a problem with 50 multi jobs", |b| {
        let insertion_ctx = get_refinement_ctx("../data/pragmatic/benches/multi-job.100.json");
        b.iter(|| black_box(solve_problem_with_recreate_cheapest(&insertion_ctx)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(15);
    targets = simple_deliveries_100_benchmark,
              simple_reload_100_benchmark,
              simple_multi_job_100_benchmark,
              init_deliveries_100_benchmark,
              init_multi_job_100_benchmark
}
criterion_main!(benches);
