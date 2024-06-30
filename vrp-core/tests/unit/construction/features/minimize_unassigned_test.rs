use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::solution::RouteContextBuilder;

#[test]
fn can_properly_estimate_empty_solution() {
    let empty = TestInsertionContextBuilder::default().build();
    let non_empty =
        TestInsertionContextBuilder::default().with_routes(vec![RouteContextBuilder::default().build()]).build();
    let objective = MinimizeUnassignedBuilder::new("minimize_unassigned").build().unwrap().objective.unwrap();

    assert_eq!(objective.fitness(&empty), 0.);
    assert_eq!(objective.fitness(&non_empty), 0.);
}
