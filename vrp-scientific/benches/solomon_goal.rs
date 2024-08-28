//! This benchmark evaluates the goal pipeline for the Solomon problem variant (CVRPTW).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use vrp_core::construction::heuristics::ActivityContext;
use vrp_core::models::common::{Schedule, Timestamp};
use vrp_core::models::problem::{JobIdDimension, Single};
use vrp_core::models::solution::{Activity, Place};
use vrp_core::prelude::*;
use vrp_scientific::common::read_init_solution;
use vrp_scientific::solomon::SolomonProblem;

pub fn get_bench_resource(resource_path: &str) -> std::io::Result<File> {
    let mut path = std::env::current_dir()?;
    path.push("benches");
    path.push(resource_path);

    File::open(path)
}

/// Returns a partial insertion context: some jobs are excluded from the used initial solution.
fn get_partial_insertion_context() -> InsertionContext {
    let environment = Arc::new(Environment::default());
    let problem = Arc::new(
        BufReader::new(get_bench_resource("../../examples/data/scientific/solomon/C101.100.txt").unwrap())
            .read_solomon(false)
            .unwrap(),
    );
    assert_eq!(problem.jobs.size(), 100);

    let solution = read_init_solution(
        BufReader::new(get_bench_resource("../../examples/data/scientific/solomon/C101.100.partial.txt").unwrap()),
        problem.clone(),
        environment.random.clone(),
    )
    .expect("cannot read initial solution");

    assert!(!solution.routes.is_empty());
    assert!(!solution.unassigned.is_empty());

    InsertionContext::new_from_solution(problem, (solution, None), environment)
}

fn get_insertion_entities(solution_ctx: &SolutionContext) -> (&RouteContext, &Arc<Single>) {
    // get route for insertion
    let route_ctx = solution_ctx
        .routes
        .iter()
        .find(|route_ctx| {
            route_ctx
                .route()
                .tour
                .get(1)
                .and_then(|a| a.retrieve_job())
                .and_then(|job| job.dimens().get_job_id().cloned())
                .map_or(false, |job_id| job_id == "67")
        })
        .expect("cannot find expected route in the solution");
    assert_eq!(route_ctx.route().tour.job_count(), 11, "unexpected job count in the route");

    // get job for insertion
    let job = solution_ctx
        .unassigned
        .iter()
        .find(|(job, _)| job.dimens().get_job_id().map_or(false, |id| id == "45"))
        .and_then(|(job, _)| job.as_single())
        .expect("cannot find single job in the unassigned jobs");

    (route_ctx, job)
}

fn bench_evaluate_route(c: &mut Criterion) {
    c.bench_function("CVRPTW: run Goal::evaluate for route on C101.100", |b| {
        let insertion_ctx = get_partial_insertion_context();
        let solution_ctx = &insertion_ctx.solution;
        let (route_ctx, job) = get_insertion_entities(solution_ctx);

        b.iter(|| {
            insertion_ctx.problem.goal.evaluate(&MoveContext::Route {
                solution_ctx,
                route_ctx,
                job: &Job::Single(job.clone()),
            });
            black_box(())
        })
    });
}

fn bench_evaluate_activity(c: &mut Criterion) {
    c.bench_function("CVRPTW: run Goal::evaluate for activity on C101.100", |b| {
        let insertion_ctx = get_partial_insertion_context();
        let solution_ctx = &insertion_ctx.solution;
        let (route_ctx, job) = get_insertion_entities(solution_ctx);

        // get activities for insertion context
        let prev = route_ctx.route().tour.get(7).unwrap();
        let target = Activity {
            place: Place {
                idx: 7,
                location: job.places[0].location.unwrap(),
                duration: job.places[0].duration.clone(),
                time: job.places[0].times[0].to_time_window(Timestamp::default()),
            },
            schedule: Schedule { arrival: 0.0, departure: 0.0 },
            job: Some(job.clone()),
            commute: None,
        };
        let next = route_ctx.route().tour.get(8);

        b.iter(|| {
            insertion_ctx.problem.goal.evaluate(&MoveContext::Activity {
                route_ctx,
                activity_ctx: &ActivityContext { index: 0, prev, target: &target, next },
            });
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(64);
    targets = bench_evaluate_route,
              bench_evaluate_activity,
}
criterion_main!(benches);
