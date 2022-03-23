use super::*;
use crate::helpers::solver::generate_matrix_routes_with_defaults;
use crate::solver::create_default_heuristic_operator;
use rosomaxa::prelude::*;

#[test]
fn can_create_multiple_insertion_ctxs_without_unassigned() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);

    let individuals = create_multiple_insertion_ctxs(&individual, (2, 2)).unwrap();

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
    solution.unassigned.extend(solution.routes[0].tour.jobs().map(|job| (job, 0)));
    solution.routes.remove(0);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);

    let individuals = create_multiple_insertion_ctxs(&individual, (2, 2)).unwrap();

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
fn can_mutate() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let problem = Arc::new(problem);
    let population = Box::new(GreedyPopulation::new(problem.objective.clone(), 1, None));

    let refinement_ctx = RefinementContext::new(problem.clone(), population, TelemetryMode::None, environment.clone());
    let insertion_ctx = InsertionContext::new_from_solution(problem.clone(), (solution, None), environment.clone());
    let inner_search = create_default_heuristic_operator(problem.clone(), environment);
    let decompose_search = DecomposeSearch::new(inner_search, (2, 2), 10);

    let result = decompose_search.search(&refinement_ctx, &insertion_ctx);

    let solution = &result.solution;
    assert!(solution.unassigned.is_empty());
    assert!(solution.ignored.is_empty());
    assert!(solution.locked.is_empty());
    assert!(solution.required.is_empty());
    assert!(!solution.routes.is_empty());
    assert_eq!(
        solution.routes.iter().flat_map(|route_ctx| route_ctx.route.tour.jobs()).collect::<HashSet<_>>().len(),
        35
    );
}
