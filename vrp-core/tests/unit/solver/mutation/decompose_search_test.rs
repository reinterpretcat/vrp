use super::*;
use crate::helpers::models::domain::test_random;
use crate::helpers::solver::generate_matrix_routes_with_defaults;

#[test]
fn can_create_multiple_individuals_without_unassigned() {
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), test_random());

    let individuals = create_multiple_individuals(&individual).unwrap();

    assert_eq!(individuals.len(), 3);
    assert_eq!(individuals[0].solution.routes.len(), 3);
    assert_eq!(individuals[1].solution.routes.len(), 3);
    assert_eq!(individuals[2].solution.routes.len(), 1);
}

#[test]
fn can_create_multiple_individuals_with_unassigned() {
    let (problem, mut solution) = generate_matrix_routes_with_defaults(5, 6, false);
    solution.registry.free_actor(&solution.routes[0].actor);
    solution.unassigned.extend(solution.routes[0].tour.jobs().map(|job| (job, 0)));
    solution.routes.remove(0);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), test_random());

    let individuals = create_multiple_individuals(&individual).unwrap();

    assert_eq!(individuals.len(), 3);

    assert_eq!(individuals[0].solution.routes.len(), 3);
    assert_eq!(individuals[0].solution.unassigned.len(), 0);

    assert_eq!(individuals[1].solution.routes.len(), 2);
    assert_eq!(individuals[1].solution.unassigned.len(), 0);

    assert_eq!(individuals[2].solution.routes.len(), 0);
    assert_eq!(individuals[2].solution.unassigned.len(), 5);
}
