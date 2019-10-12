use crate::construction::heuristics::insertions::create_cheapest_insertion_heuristic;
use crate::construction::states::InsertionContext;
use crate::helpers::streams::input::create_c101_25_problem;

#[test]
fn can_solve_with_cheapest_insertion_heuristic() {
    let heuristic = create_cheapest_insertion_heuristic();

    let result = heuristic.process(InsertionContext::new(create_c101_25_problem()));

    assert_eq!(result.solution.unassigned.len(), 0);
}
