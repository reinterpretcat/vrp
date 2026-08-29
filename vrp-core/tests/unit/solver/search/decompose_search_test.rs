use super::*;
use crate::helpers::solver::generate_matrix_routes_with_defaults;
use crate::helpers::utils::random::FakeRandom;
use crate::solver::create_default_heuristic_operator;
use rosomaxa::prelude::*;

#[test]
fn can_get_repeat_count_above_precomputed_range() {
    assert_eq!(get_repeat_count(5, &FakeRandom::new(vec![0], vec![])), 1);
    assert_eq!(get_repeat_count(5, &FakeRandom::new(vec![4], vec![])), 5);
}

#[test]
fn can_create_multiple_insertion_ctxs_without_unassigned() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());

    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();

    assert_eq!(individuals.len(), 4);
    assert_eq!(individuals[0].0.solution.routes.len(), 2);
    assert_eq!(individuals[1].0.solution.routes.len(), 2);
    assert_eq!(individuals[2].0.solution.routes.len(), 2);
    assert_eq!(individuals[3].0.solution.routes.len(), 1);
}

#[test]
fn can_create_multiple_insertion_ctxs_with_unassigned() {
    let environment = Arc::new(Environment::default());
    let (problem, mut solution) = generate_matrix_routes_with_defaults(5, 6, false);
    solution.registry.free_actor(&solution.routes[0].actor);
    solution.unassigned.extend(solution.routes[0].tour.jobs().cloned().map(|job| (job, UnassignmentInfo::Unknown)));
    solution.routes.remove(0);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());

    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();

    assert_eq!(individuals.len(), 4);

    assert_eq!(individuals[0].0.solution.routes.len(), 2);
    assert_eq!(individuals[0].0.solution.unassigned.len(), 0);

    assert_eq!(individuals[1].0.solution.routes.len(), 2);
    assert_eq!(individuals[1].0.solution.unassigned.len(), 0);

    assert_eq!(individuals[2].0.solution.routes.len(), 1);
    assert_eq!(individuals[2].0.solution.unassigned.len(), 0);

    assert_eq!(individuals[3].0.solution.routes.len(), 0);
    assert_eq!(individuals[3].0.solution.unassigned.len(), 5);
}

#[test]
fn can_partition_locked_jobs() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(4, 3, false);
    let mut individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());
    individual.solution.locked =
        individual.solution.routes.iter().flat_map(|route| route.route().tour.jobs().take(1).cloned()).collect();

    let expected = individual.solution.locked.clone();
    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();
    let actual = individuals
        .iter()
        .flat_map(|(individual, _)| individual.solution.locked.iter().cloned())
        .collect::<HashSet<_>>();

    assert_eq!(actual, expected);
    assert_eq!(individuals.len(), 2);
    assert!(individuals.iter().all(|(individual, _)| {
        individual.solution.locked.iter().all(|job| {
            individual.solution.routes.iter().any(|route| route.route().tour.jobs().any(|candidate| candidate == job))
        })
    }));
}

#[test]
fn can_partition_assigned_and_unassigned_locked_jobs() {
    let environment = Arc::new(Environment::default());
    let (problem, mut solution) = generate_matrix_routes_with_defaults(3, 2, false);
    let assigned = solution.routes[1].tour.jobs().next().unwrap().clone();
    let unassigned = solution.routes[0].tour.jobs().next().unwrap().clone();
    solution.registry.free_actor(&solution.routes[0].actor);
    solution.unassigned.extend(solution.routes[0].tour.jobs().cloned().map(|job| (job, UnassignmentInfo::Unknown)));
    solution.routes.remove(0);

    let mut individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());
    individual.solution.locked.extend([assigned.clone(), unassigned.clone()]);
    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();
    let empty = individuals.iter().find(|(individual, _)| individual.solution.routes.is_empty()).unwrap();
    let assigned_part = individuals
        .iter()
        .find(|(individual, _)| individual.solution.routes.iter().any(|route| route.route().tour.contains(&assigned)))
        .unwrap();

    assert_eq!(empty.0.solution.locked, HashSet::from([unassigned]));
    assert_eq!(assigned_part.0.solution.locked, HashSet::from([assigned]));
}

#[test]
fn can_perform_search() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let problem = Arc::new(problem);
    let population = Box::new(GreedyPopulation::new(problem.goal.clone(), 1, None));

    let refinement_ctx = RefinementContext::new(problem.clone(), population, TelemetryMode::None, environment.clone());
    let insertion_ctx = InsertionContext::new_from_solution(problem.clone(), (solution, None), environment.clone());
    let inner_search = create_default_heuristic_operator(problem, environment);
    let decompose_search = DecomposeSearch::new(inner_search, (2, 2), 10);

    let result = decompose_search.search(&refinement_ctx, &insertion_ctx);

    let solution = &result.solution;
    assert!(solution.ignored.is_empty());
    assert!(solution.locked.is_empty());
    assert!(solution.required.is_empty());
    assert!(!solution.routes.is_empty());
    let total_jobs =
        solution.routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs()).collect::<HashSet<_>>().len()
            + solution.unassigned.len();
    assert_eq!(total_jobs, 35);
}
