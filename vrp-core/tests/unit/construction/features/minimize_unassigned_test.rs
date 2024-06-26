use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::solution::RouteContextBuilder;
use std::cmp::Ordering;

#[test]
fn can_properly_estimate_empty_solution() {
    let empty = TestInsertionContextBuilder::default().build();
    let non_empty =
        TestInsertionContextBuilder::default().with_routes(vec![RouteContextBuilder::default().build()]).build();
    let objective = MinimizeUnassignedBuilder::new("minimize_unassigned").build().unwrap().objective.unwrap();

    let result = objective.total_order(&empty, &non_empty);

    assert_eq!(result, Ordering::Greater);
}
