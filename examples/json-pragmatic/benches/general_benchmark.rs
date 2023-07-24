use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use vrp_pragmatic::core::construction::heuristics::InsertionContext;
use vrp_pragmatic::core::models::Solution;
use vrp_pragmatic::core::prelude::{create_default_config_builder, Problem, Solver};
use vrp_pragmatic::core::rosomaxa::evolution::TelemetryMode;
use vrp_pragmatic::core::solver::search::{Recreate, RecreateWithCheapest};
use vrp_pragmatic::core::solver::{create_elitism_population, RefinementContext};
use vrp_pragmatic::core::utils::Environment;
use vrp_pragmatic::format::problem::PragmaticProblem;

fn get_problem(problem_path: &str) -> Arc<Problem> {
    let file = File::open(problem_path).unwrap_or_else(|err| panic!("cannot open {} file: '{}'", problem_path, err));
    Arc::new(
        BufReader::new(file)
            .read_pragmatic()
            .unwrap_or_else(|errs| panic!("cannot create pragmatic problem: {}", errs)),
    )
}

/// Runs solver with specific amount of generations. It involves some non-determenism.
fn solve_problem_with_max_generations(problem_path: &str, generations: usize) -> Solution {
    let problem = get_problem(problem_path);

    let (solution, _, _) =
        create_default_config_builder(problem.clone(), Arc::new(Environment::default()), TelemetryMode::None)
            .with_max_generations(Some(generations))
            .build()
            .map(|config| Solver::new(problem, config))
            .unwrap_or_else(|err| panic!("cannot build solver: {}", err))
            .solve()
            .unwrap_or_else(|err| panic!("cannot solver problem: {}", err));

    solution
}

/// Solve problem using cheapest insertion heuristic and returns one solution.
fn solve_problem_with_init(problem_path: &str) {
    let problem = get_problem(problem_path);

    let environment = Arc::new(Environment::default());
    let refinement_ctx = RefinementContext::new(
        problem.clone(),
        Box::new(create_elitism_population(problem.goal.clone(), environment.clone())),
        TelemetryMode::None,
        environment.clone(),
    );

    let _ = RecreateWithCheapest::new(environment.random.clone())
        .run(&refinement_ctx, InsertionContext::new(problem.clone(), environment));
}

fn simple_deliveries_100_benchmark(c: &mut Criterion) {
    c.bench_function("a problem with 100 trivial deliveries", |b| {
        b.iter(|| {
            solve_problem_with_max_generations("../data/pragmatic/benches/simple.deliveries.100.json", black_box(100))
        })
    });
}

fn simple_reload_100_benchmark(c: &mut Criterion) {
    c.bench_function("a problem with 100 trivial deliveries and one reload", |b| {
        b.iter(|| {
            solve_problem_with_max_generations("../data/pragmatic/benches/simple.reload.100.json", black_box(100))
        })
    });
}

fn simple_multi_job_100_benchmark(c: &mut Criterion) {
    c.bench_function("a problem with 50 multi jobs", |b| {
        b.iter(|| solve_problem_with_max_generations("../data/pragmatic/benches/multi-job.100.json", black_box(10)))
    });
}

fn init_deliveries_100_benchmark(c: &mut Criterion) {
    c.bench_function("init solution for a problem with 100 trivial deliveries", |b| {
        b.iter(|| {
            solve_problem_with_init("../data/pragmatic/benches/simple.deliveries.100.json");
            black_box(())
        })
    });
}

fn init_multi_job_100_benchmark(c: &mut Criterion) {
    c.bench_function("init solution for a problem with 50 multi jobs", |b| {
        b.iter(|| {
            solve_problem_with_init("../data/pragmatic/benches/multi-job.100.json");
            black_box(())
        })
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
