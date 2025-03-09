//! This benchmark evaluates the goal pipeline for the Solomon problem variant (CVRPTW).

use criterion::{Criterion, black_box, criterion_group, criterion_main};
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
                .is_some_and(|job_id| job_id == "67")
        })
        .expect("cannot find expected route in the solution");
    assert_eq!(route_ctx.route().tour.job_count(), 11, "unexpected job count in the route");

    // get job for insertion
    let job = solution_ctx
        .unassigned
        .iter()
        .find(|(job, _)| job.dimens().get_job_id().is_some_and(|id| id == "45"))
        .and_then(|(job, _)| job.as_single())
        .expect("cannot find single job in the unassigned jobs");

    (route_ctx, job)
}

fn bench_route_template<F, R>(c: &mut Criterion, id: &str, actual_fn: F)
where
    F: Fn(&GoalContext, &MoveContext) -> R,
{
    c.bench_function(id, |b| {
        let insertion_ctx = get_partial_insertion_context();
        let solution_ctx = &insertion_ctx.solution;
        let (route_ctx, job) = get_insertion_entities(solution_ctx);

        b.iter(|| {
            black_box(actual_fn(
                &insertion_ctx.problem.goal,
                &MoveContext::Route { solution_ctx, route_ctx, job: &Job::Single(job.clone()) },
            ))
        })
    });
}

fn bench_evaluate_route(c: &mut Criterion) {
    bench_route_template(c, "CVRPTW: run Goal::evaluate for route on C101.100", |goal_ctx, move_ctx| {
        goal_ctx.evaluate(move_ctx)
    });
}

fn bench_estimate_route(c: &mut Criterion) {
    bench_route_template(c, "CVRPTW: run Goal::estimate for route on C101.100", |goal_ctx, move_ctx| {
        goal_ctx.estimate(move_ctx)
    });
}

fn bench_activity_template<F, R>(c: &mut Criterion, id: &str, actual_fn: F)
where
    F: Fn(&GoalContext, &MoveContext) -> R,
{
    c.bench_function(id, |b| {
        let insertion_ctx = get_partial_insertion_context();
        let solution_ctx = &insertion_ctx.solution;
        let (route_ctx, job) = get_insertion_entities(solution_ctx);

        // get activities for insertion context
        let prev = route_ctx.route().tour.get(7).unwrap();
        let target = Activity {
            place: Place {
                idx: 0,
                location: job.places[0].location.unwrap(),
                duration: job.places[0].duration,
                time: job.places[0].times[0].to_time_window(Timestamp::default()),
            },
            schedule: Schedule { arrival: 0.0, departure: 0.0 },
            job: Some(job.clone()),
            commute: None,
        };
        let next = route_ctx.route().tour.get(8);

        b.iter(|| {
            black_box(actual_fn(
                &insertion_ctx.problem.goal,
                &MoveContext::Activity {
                    solution_ctx,
                    route_ctx,
                    activity_ctx: &ActivityContext { index: 7, prev, target: &target, next },
                },
            ))
        })
    });
}

fn bench_evaluate_activity(c: &mut Criterion) {
    bench_activity_template(c, "CVRPTW: run Goal::evaluate for activity on C101.100", |goal_ctx, move_ctx| {
        goal_ctx.evaluate(move_ctx)
    });
}

fn bench_estimate_activity(c: &mut Criterion) {
    bench_activity_template(c, "CVRPTW: run Goal::estimate for activity on C101.100", |goal_ctx, move_ctx| {
        goal_ctx.estimate(move_ctx)
    });
}

fn bench_accept_solution(c: &mut Criterion) {
    c.bench_function("CVRPTW: run Goal::accept_solution_state on C101.100", |b| {
        let insertion_ctx = get_partial_insertion_context();
        let mut solution_ctx = insertion_ctx.solution;

        // mark all routes as stale
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            black_box(route_ctx.route_mut());
        });

        b.iter(|| {
            insertion_ctx.problem.goal.accept_solution_state(&mut solution_ctx);
            black_box(())
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(512);
    targets = bench_evaluate_route,
              bench_estimate_route,
              bench_evaluate_activity,
              bench_estimate_activity,
              bench_accept_solution
}
criterion_main!(benches);
