use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};
use crate::utils::compare_floats;

#[test]
fn can_search_individual() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let problem = Arc::new(problem);
    let individual = InsertionContext::new_from_solution(problem.clone(), (solution, None), environment.clone());
    let refinement_ctx = create_default_refinement_ctx(problem.clone());
    let mut heuristic = DynamicSelective::new_with_defaults(problem, environment);

    let results = heuristic.search(&refinement_ctx, vec![&individual]);

    assert_eq!(results.len(), 1);
    let individual = results.first().unwrap();
    assert!(!individual.solution.routes.is_empty());
    assert!(individual.solution.unassigned.is_empty());
    assert!(!heuristic.initial_estimates.is_empty());
    for state in &[SearchState::Diverse, SearchState::BestKnown] {
        let actions = heuristic.initial_estimates.get(state).expect("cannot get state");
        assert!(!actions.data().is_empty());
        assert!(actions.data().iter().any(|(_, estimate)| compare_floats(*estimate, 0.) != Ordering::Less));
    }
}
