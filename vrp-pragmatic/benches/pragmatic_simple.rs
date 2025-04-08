use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use vrp_pragmatic::core::construction::heuristics::InsertionContext;
use vrp_pragmatic::core::prelude::*;
use vrp_pragmatic::core::rosomaxa::evolution::TelemetryMode;
use vrp_pragmatic::core::solver::search::{Recreate, RecreateWithCheapest};
use vrp_pragmatic::core::solver::{RefinementContext, create_elitism_population};
use vrp_pragmatic::format::problem::PragmaticProblem;

pub fn get_bench_resource(resource_path: &str) -> std::io::Result<File> {
    let mut path = std::env::current_dir()?;
    path.push("benches");
    path.push(resource_path);

    File::open(path)
}

fn get_problem(problem_path: &str) -> Arc<Problem> {
    let file =
        get_bench_resource(problem_path).unwrap_or_else(|err| panic!("cannot open {} file: '{}'", problem_path, err));
    Arc::new(
        BufReader::new(file)
            .read_pragmatic()
            .unwrap_or_else(|errs| panic!("cannot create pragmatic problem: {}", errs)),
    )
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
        .run(refinement_ctx, InsertionContext::new(refinement_ctx.problem.clone(), refinement_ctx.environment.clone()))
}

fn bench_init_deliveries_100_benchmark(c: &mut Criterion) {
    c.bench_function("building init solution for a problem with 100 trivial deliveries", |b| {
        let insertion_ctx = get_refinement_ctx("../../examples/data/pragmatic/benches/simple.deliveries.100.json");
        b.iter(|| black_box(solve_problem_with_recreate_cheapest(&insertion_ctx)))
    });
}

fn bench_init_multi_job_100_benchmark(c: &mut Criterion) {
    c.bench_function("building init solution for a problem with 50 multi jobs", |b| {
        let insertion_ctx = get_refinement_ctx("../../examples/data/pragmatic/benches/multi-job.100.json");
        b.iter(|| black_box(solve_problem_with_recreate_cheapest(&insertion_ctx)))
    });
}

fn bench_init_reload_100_benchmark(c: &mut Criterion) {
    c.bench_function("building init solution for a problem 100 trivial deliveries and one reload", |b| {
        let insertion_ctx = get_refinement_ctx("../../examples/data/pragmatic/benches/simple.reload.100.json");
        b.iter(|| black_box(solve_problem_with_recreate_cheapest(&insertion_ctx)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(512).noise_threshold(0.05);
    targets = bench_init_deliveries_100_benchmark,
              bench_init_multi_job_100_benchmark,
              bench_init_reload_100_benchmark,
}
criterion_main!(benches);
