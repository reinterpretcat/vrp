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

#[test]
fn can_exchange_estimates() {
    let create_action_estimates = |base_value: f64| (0..10)
        .map(|idx| (SearchAction::Mutate { mutation_index: idx }, idx as f64 * base_value))
        .collect::<HashMap<_, _>>();
    let mut simulator = Simulator::new(
        Box::new(MonteCarlo::new(0.1)),
        Box::new(EpsilonWeighted::new(0.1, Environment::default().random.clone())),
    );
    simulator.set_action_estimates(SearchState::BestKnown, ActionEstimates::from(create_action_estimates(-1.)));
    simulator.set_action_estimates(SearchState::Diverse, ActionEstimates::from(create_action_estimates(1.)));

    try_exchange_estimates(&mut simulator);
    simulator.set_action_estimates(SearchState::Diverse, ActionEstimates::from(create_action_estimates(-10.)));

    let estimate_values = simulator.get_state_estimates().get(&SearchState::BestKnown).unwrap();
    assert!(!estimate_values.data().is_empty());
    estimate_values.data().iter().for_each(|(_, value)| {
        assert_ne!(compare_floats(*value, 0.), Ordering::Less);
    });
}
