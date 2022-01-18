use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use vrp_pragmatic::core::models::Solution;
use vrp_pragmatic::core::prelude::{create_default_config_builder, Solver};
use vrp_pragmatic::core::utils::Environment;
use vrp_pragmatic::format::problem::PragmaticProblem;
use vrp_pragmatic::format::FormatError;

fn solve_problem_with_max_generations(problem_path: &str, generations: usize) -> Solution {
    let file = File::open(problem_path)
        .unwrap_or_else(|err| panic!("cannot open {} file: '{}'", problem_path, err.to_string()));
    let problem = Arc::new(BufReader::new(file).read_pragmatic().unwrap_or_else(|errs| {
        panic!("cannot create pragmatic problem: {}", FormatError::format_many(errs.as_slice(), ","))
    }));

    let (solution, _, _) = create_default_config_builder(problem.clone(), Arc::new(Environment::default()))
        .with_max_generations(Some(generations))
        .build()
        .map(|config| Solver::new(problem, config))
        .unwrap_or_else(|err| panic!("cannot build solver: {}", err))
        .solve()
        .unwrap_or_else(|err| panic!("cannot solver problem: {}", err));

    solution
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

fn multi_job_100_benchmark(c: &mut Criterion) {
    c.bench_function("a problem with 50 multi jobs", |b| {
        b.iter(|| solve_problem_with_max_generations("../data/pragmatic/benches/multi-job.100.json", black_box(10)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(15);
    targets = simple_deliveries_100_benchmark,
              simple_reload_100_benchmark,
              multi_job_100_benchmark
}
criterion_main!(benches);
