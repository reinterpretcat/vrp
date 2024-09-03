use super::*;
use crate::common::RoutingMode;
use crate::helpers::{create_c101_100_problem, get_test_resource};
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::utils::{Environment, Float};

const SCALE: Float = 1000.;

#[test]
pub fn can_read_init_solution() {
    let environment = Arc::new(Environment::default());
    let problem = Arc::new(create_c101_100_problem(RoutingMode::ScaleNoRound(SCALE)));
    let file = get_test_resource("../../examples/data/scientific/solomon/C101.100.best.txt").unwrap();

    let solution = read_init_solution(BufReader::new(file), problem.clone(), environment.random.clone())
        .expect("cannot read initial solution");
    assert_eq!(solution.routes.len(), 10);
    assert!(solution.unassigned.is_empty());

    let insertion_ctx = InsertionContext::new_from_solution(problem, (solution, None), environment);
    assert_eq!(insertion_ctx.get_total_cost(SCALE).unwrap_or_default().round(), (828.936 as Float).round());
}
