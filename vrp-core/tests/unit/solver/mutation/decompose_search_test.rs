use super::*;
use crate::helpers::solver::generate_matrix_routes_with_defaults;
use crate::solver::hyper::StaticSelective;
use crate::utils::Environment;

#[test]
fn can_create_multiple_individuals_without_unassigned() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);

    let individuals = create_multiple_individuals(&individual, (2, 2)).unwrap();

    assert_eq!(individuals.len(), 4);
    assert_eq!(individuals[0].0.solution.routes.len(), 2);
    assert_eq!(individuals[1].0.solution.routes.len(), 2);
    assert_eq!(individuals[2].0.solution.routes.len(), 2);
    assert_eq!(individuals[3].0.solution.routes.len(), 1);
}

#[test]
fn can_create_multiple_individuals_with_unassigned() {
    let environment = Arc::new(Environment::default());
    let (problem, mut solution) = generate_matrix_routes_with_defaults(5, 6, false);
    solution.registry.free_actor(&solution.routes[0].actor);
    solution.unassigned.extend(solution.routes[0].tour.jobs().map(|job| (job, 0)));
    solution.routes.remove(0);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);

    let individuals = create_multiple_individuals(&individual, (2, 2)).unwrap();

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
    let population = Box::new(Greedy::new(problem.clone(), 1, None));

    let refinement_ctx = RefinementContext::new(problem.clone(), population, environment.clone(), None);
    let insertion_ctx = InsertionContext::new_from_solution(problem.clone(), (solution, None), environment.clone());
    let decompose_search =
        DecomposeSearch::new(StaticSelective::create_default_mutation(problem.clone(), environment), (2, 2), 10);

    let result = decompose_search.mutate(&refinement_ctx, &insertion_ctx);

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
