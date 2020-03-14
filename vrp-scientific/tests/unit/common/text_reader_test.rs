use crate::common::text_reader::read_init_solution;
use crate::helpers::{create_c101_100_problem, get_test_resource};
use std::io::BufReader;
use std::sync::Arc;
use vrp_core::construction::states::InsertionContext;
use vrp_core::refinement::objectives::{MultiObjective, Objective};
use vrp_core::refinement::RefinementContext;
use vrp_core::utils::DefaultRandom;

#[test]
pub fn can_read_init_solution() {
    let problem = Arc::new(create_c101_100_problem());
    let file = get_test_resource("../data/solomon/C101.100.best.txt").unwrap();

    let result = read_init_solution(BufReader::new(file), problem.clone());

    assert!(result.is_ok());
    let solution = Arc::new(result.unwrap());
    assert_eq!(solution.routes.len(), 10);
    assert_eq!(
        MultiObjective::default()
            .estimate(
                &mut RefinementContext::new(problem.clone()),
                &InsertionContext::new_from_solution(problem, (solution, None), Arc::new(DefaultRandom::default()))
            )
            .value()
            .round(),
        828.936f64.round()
    );
}
