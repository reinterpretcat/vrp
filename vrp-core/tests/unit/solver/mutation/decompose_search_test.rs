use super::*;
use crate::helpers::models::domain::test_random;
use crate::helpers::solver::generate_matrix_routes_with_defaults;

#[test]
fn can_create_multiple_individuals() {
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), test_random());

    let individuals = create_multiple_individuals(&individual).unwrap();

    assert_eq!(individuals.len(), 3);
}
