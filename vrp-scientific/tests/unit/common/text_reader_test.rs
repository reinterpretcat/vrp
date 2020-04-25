use crate::common::create_default_objective;
use crate::common::text_reader::read_init_solution;
use crate::helpers::{create_c101_100_problem, get_test_resource};
use std::io::BufReader;
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::refinement::objectives::Objective;
use vrp_core::utils::DefaultRandom;

#[test]
pub fn can_read_init_solution() {
    let problem = Arc::new(create_c101_100_problem());
    let file = get_test_resource("../data/solomon/C101.100.best.txt").unwrap();

    let solution =
        Arc::new(read_init_solution(BufReader::new(file), problem.clone()).expect("Cannot read initial solution"));

    assert_eq!(solution.routes.len(), 10);
    assert_eq!(
        create_default_objective()
            .fitness(&InsertionContext::new_from_solution(
                problem,
                (solution, None),
                Arc::new(DefaultRandom::default())
            ))
            .round(),
        828.936f64.round()
    );
}
