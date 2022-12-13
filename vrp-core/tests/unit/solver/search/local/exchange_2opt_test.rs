use super::*;
use crate::helpers::models::domain::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::helpers::solver::*;
use crate::models::problem::Multi;
use crate::models::solution::Registry;
use crate::models::GoalContext;

parameterized_test! {can_fix_order, (activities, is_open_vrp, job_order, expected), {
    can_fix_order_impl(activities, is_open_vrp, job_order, expected);
}}

can_fix_order! {
    case_01: (3, true, vec![vec!["c1", "c0", "c2"]], vec![vec!["c0", "c1", "c2"]]),
    case_02: (3, false, vec![vec!["c1", "c0", "c2"]], vec![vec!["c0", "c1", "c2"]]),
}

fn can_fix_order_impl(activities: usize, is_open_vrp: bool, job_order: Vec<Vec<&str>>, expected: Vec<Vec<&str>>) {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(activities, 1, is_open_vrp);
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order.as_slice());

    let insertion_ctx = ExchangeTwoOpt::default()
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    assert!(insertion_ctx.solution.unassigned.is_empty());
    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), expected.as_slice(), "");
}

#[test]
fn can_fix_multi_jobs() {
    let id = "job1";
    let valid_permutation = vec![0, 1, 2];
    let tour_permutation = vec![2, 1, 0];

    let fleet = test_fleet();
    let create_multi = |id: &str, singles: Vec<String>, permutations: Vec<Vec<usize>>| {
        test_multi_with_permutations(
            id,
            singles.iter().map(|id| SingleBuilder::default().id(id).build_shared()).collect(),
            permutations,
        )
    };
    let create_insertion_ctx = |permutation: &[usize], multi: &Multi| InsertionContext {
        solution: SolutionContext {
            routes: vec![create_route_context_with_activities(
                &fleet,
                "v1",
                permutation
                    .iter()
                    .map(|idx| multi.jobs.get(*idx).unwrap().clone())
                    .map(test_activity_with_job)
                    .collect(),
            )],
            registry: RegistryContext::new(Arc::new(GoalContext::default()), Registry::new(&fleet, test_random())),
            ..create_empty_solution_context()
        },
        ..create_empty_insertion_context()
    };

    let multi = create_multi(
        id,
        valid_permutation.iter().map(|idx| format!("s{}", idx)).collect(),
        vec![valid_permutation.clone()],
    );
    let orig_insertion_ctx = create_insertion_ctx(valid_permutation.as_slice(), multi.as_ref());
    let new_insertion_ctx = create_insertion_ctx(tour_permutation.as_slice(), multi.as_ref());
    let opt_ctx =
        OptContext { insertion_ctx: &orig_insertion_ctx, new_insertion_ctx: Some(new_insertion_ctx), route_idx: 0 };

    let insertion_ctx = opt_ctx.try_restore_solution().unwrap();

    assert!(insertion_ctx.solution.routes.is_empty());
    assert!(insertion_ctx.solution.required.is_empty());
    assert_eq!(insertion_ctx.solution.unassigned.len(), 1);
}
