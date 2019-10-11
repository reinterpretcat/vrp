use crate::construction::heuristics::insertions::create_cheapest_insertion_heuristic;
use crate::construction::states::InsertionContext;
use crate::helpers::models::domain::get_customer_ids_from_routes_sorted;
use crate::helpers::streams::input::create_c101_25_problem;
use crate::models::Extras;
use std::io::BufWriter;

#[test]
fn can_solve_with_cheapest_insertion_heuristic() {
    let heuristic = create_cheapest_insertion_heuristic();

    let result = heuristic.process(InsertionContext::new(create_c101_25_problem()));
    let result = result.solution.into_solution(Extras::default());
    let result = get_customer_ids_from_routes_sorted(&result);

    assert_eq!(
        result,
        vec![
            vec!["c13", "c17", "c18", "c19", "c15"],
            vec!["c20", "c24", "c25", "c10", "c11", "c9", "c6", "c23", "c22", "c21"],
            vec!["c5", "c3", "c7", "c8", "c16", "c14", "c12", "c4", "c2", "c1"],
        ]
    );
}
