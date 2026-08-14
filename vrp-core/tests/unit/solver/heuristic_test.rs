use super::*;
use crate::helpers::models::domain::ProblemBuilder;

#[test]
fn can_keep_default_cheapest_initial_operator_one_shot() {
    let problem = Arc::new(ProblemBuilder::default().build());
    let operators = create_default_init_operators(problem, Arc::new(Environment::default()));

    assert_eq!(operators.first().map(|(name, _, weight)| (name.as_str(), *weight)), Some(("cheapest", 0)));
}
