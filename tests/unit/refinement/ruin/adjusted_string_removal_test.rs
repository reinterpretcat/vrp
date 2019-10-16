/* Uses predefined values to control algorithm execution.
int distribution values:
1. route index in solution
2*. job index in selected route tour
3*. selected algorithm: 1: sequential algorithm(**)
4*. string removal index(-ies)
double distribution values:
1. string count
2*. string size(-s)
(*) - specific for each route.
(**) - calls more int and double distributions:
    int 5. split start
    dbl 3. alpha param
*/

use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::refinement::generate_matrix_routes;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::ObjectiveCost;
use crate::refinement::ruin::{AdjustedStringRemoval, RuinStrategy};
use crate::refinement::RefinementContext;
use std::sync::Arc;

#[test]
fn can_ruin_solution_with_one_route() {
    let fake_random = FakeRandom::new(vec![0, 3, 1, 2], vec![1., 5.]);
    let (problem, solution) = generate_matrix_routes(10, 1);
    let solution = Arc::new(solution);
    let refinement_ctx = RefinementContext {
        problem: Arc::new(problem),
        locked: Default::default(),
        population: vec![(solution.clone(), ObjectiveCost::new(0., 0.))],
        random: Arc::new(FakeRandom::new(vec![0, 3, 1, 2], vec![1., 5.])),
        generation: 0,
    };

    let insertion_ctx = AdjustedStringRemoval::default().ruin_solution(&refinement_ctx, &solution);

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), vec!["c1", "c2", "c3", "c4", "c5"]);
}
