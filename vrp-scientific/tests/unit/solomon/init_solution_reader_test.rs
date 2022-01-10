use super::*;
use crate::helpers::{create_c101_100_problem, get_test_resource};
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::rosomaxa::prelude::Objective;
use vrp_core::utils::Environment;

#[test]
pub fn can_read_init_solution() {
    let environment = Arc::new(Environment::default());
    let problem = Arc::new(create_c101_100_problem());
    let file = get_test_resource("../../examples/data/scientific/solomon/C101.100.best.txt").unwrap();

    let solution = read_init_solution(BufReader::new(file), problem.clone(), environment.random.clone())
        .expect("Cannot read initial solution");
    assert_eq!(solution.routes.len(), 10);

    let ctx = InsertionContext::new_from_solution(problem, (solution, None), environment);
    assert_eq!(ProblemObjective::default().fitness(&ctx).round(), 828.936f64.round());
}
