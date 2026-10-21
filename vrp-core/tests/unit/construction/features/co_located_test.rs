use crate::construction::features::create_co_located_feature;
use crate::construction::heuristics::{InsertionContext, MoveContext};
use crate::helpers::solver::{generate_matrix_routes_with_defaults, get_job_by_id};
use crate::models::Feature;
use rosomaxa::prelude::Float;
use rosomaxa::utils::Environment;
use std::sync::Arc;

// Two routes of three jobs each:
//   route 0: c0 c1 c2
//   route 1: c3 c4 c5
fn create_insertion_ctx() -> InsertionContext {
    let (problem, solution) = generate_matrix_routes_with_defaults(3, 2, false);

    InsertionContext::new_from_solution(Arc::new(problem), (solution, None), Arc::new(Environment::default()))
}

fn create_feature(insertion_ctx: &InsertionContext, pairs: &[(&str, &str)]) -> Feature {
    let pairs = pairs
        .iter()
        .map(|(a, b)| {
            (get_job_by_id(insertion_ctx, a).unwrap().clone(), get_job_by_id(insertion_ctx, b).unwrap().clone())
        })
        .collect();

    create_co_located_feature("co_located", pairs).expect("cannot create feature")
}

fn get_fitness(insertion_ctx: &mut InsertionContext, pairs: &[(&str, &str)]) -> Float {
    let feature = create_feature(insertion_ctx, pairs);

    feature.state.as_ref().unwrap().accept_solution_state(&mut insertion_ctx.solution);

    feature.objective.as_ref().unwrap().fitness(insertion_ctx)
}

parameterized_test! {can_count_split_pairs, (pairs, expected), {
    let mut insertion_ctx = create_insertion_ctx();

    assert_eq!(get_fitness(&mut insertion_ctx, pairs), expected);
}}

can_count_split_pairs! {
    case_01_together:          (&[("c0", "c1")], 0.),
    case_02_split:             (&[("c0", "c3")], 1.),
    case_03_mixed:             (&[("c0", "c1"), ("c0", "c3"), ("c2", "c5")], 2.),
    case_04_repeated:          (&[("c0", "c3"), ("c0", "c3"), ("c3", "c0")], 1.),
}

/// Takes `job_id` out of its tour and leaves it waiting for insertion.
fn unassign(insertion_ctx: &mut InsertionContext, job_id: &str) {
    let job = get_job_by_id(insertion_ctx, job_id).unwrap().clone();
    let route_ctx =
        insertion_ctx.solution.routes.iter_mut().find(|route_ctx| route_ctx.route().tour.has_job(&job)).unwrap();

    route_ctx.route_mut().tour.remove(&job);
    insertion_ctx.solution.required.push(job);
}

parameterized_test! {can_estimate_change_in_split_pairs, (pairs, route_idx, expected), {
    can_estimate_change_in_split_pairs_impl(pairs, route_idx, expected);
}}

can_estimate_change_in_split_pairs! {
    case_01_joins_partner:       (&[("c2", "c5")], 0, 0.),
    case_02_leaves_partner:      (&[("c2", "c5")], 1, 1.),
    case_03_two_partners:        (&[("c2", "c5"), ("c0", "c5"), ("c3", "c5")], 0, 1.),
    case_04_no_partner:          (&[("c0", "c3")], 0, 0.),
    case_05_repeated_partner:    (&[("c2", "c5"), ("c5", "c2")], 1, 1.),
}

/// `c5` is taken out of route 1 and offered to `route_idx`. The estimate is checked against the
/// fitness the solution actually has once the insertion is made, which is the independent truth.
fn can_estimate_change_in_split_pairs_impl(pairs: &[(&str, &str)], route_idx: usize, expected: Float) {
    let mut insertion_ctx = create_insertion_ctx();
    unassign(&mut insertion_ctx, "c5");
    let before = get_fitness(&mut insertion_ctx, pairs);
    let feature = create_feature(&insertion_ctx, pairs);
    let job = get_job_by_id(&insertion_ctx, "c5").unwrap().clone();

    let estimate = feature.objective.as_ref().unwrap().estimate(&MoveContext::Route {
        solution_ctx: &insertion_ctx.solution,
        route_ctx: &insertion_ctx.solution.routes[route_idx],
        job: &job,
    });

    let mut activity = insertion_ctx.solution.routes[1].route().tour.get(1).unwrap().deep_copy();
    activity.job = job.as_single().cloned();
    insertion_ctx.solution.routes[route_idx].route_mut().tour.insert_last(activity);
    insertion_ctx.solution.required.retain(|required| *required != job);
    let after = get_fitness(&mut insertion_ctx, pairs);

    assert_eq!(estimate, expected);
    assert_eq!(after - before, expected);
}

/// A decomposed sub-problem holds only some of the routes. The pair whose partner sits in a route
/// the sub-problem does not hold is neither counted nor refused.
#[test]
fn can_count_what_a_partial_solution_holds() {
    let mut insertion_ctx = create_insertion_ctx();
    unassign(&mut insertion_ctx, "c5");
    insertion_ctx.solution.routes.truncate(1);
    let pairs = [("c0", "c3"), ("c2", "c5"), ("c0", "c1")];

    let fitness = get_fitness(&mut insertion_ctx, &pairs);
    let feature = create_feature(&insertion_ctx, &pairs);
    let estimate = feature.objective.as_ref().unwrap().estimate(&MoveContext::Route {
        solution_ctx: &insertion_ctx.solution,
        route_ctx: &insertion_ctx.solution.routes[0],
        job: get_job_by_id(&insertion_ctx, "c3").unwrap(),
    });

    assert_eq!(fitness, 0.);
    assert_eq!(estimate, 0.);
}

#[test]
fn can_ignore_pair_with_unassigned_job() {
    let mut insertion_ctx = create_insertion_ctx();
    unassign(&mut insertion_ctx, "c5");

    assert_eq!(get_fitness(&mut insertion_ctx, &[("c0", "c3"), ("c2", "c5")]), 1.);
}
