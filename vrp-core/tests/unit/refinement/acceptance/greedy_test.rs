use crate::helpers::models::domain::{create_empty_insertion_context, create_empty_problem};
use crate::models::common::ObjectiveCost;
use crate::refinement::acceptance::greedy::Greedy;
use crate::refinement::acceptance::Acceptance;
use crate::refinement::RefinementContext;

parameterized_test! {can_identify_cheapest_solution, (new_cost, old_cost, expected), {
    can_identify_cheapest_solution_impl(ObjectiveCost::new(new_cost.0, new_cost.1), ObjectiveCost::new(old_cost.0, old_cost.1), expected);
}}

can_identify_cheapest_solution! {
    case_01: ((10., 0.), (20., 0.), true),
    case_02: ((20., 0.), (10., 0.), false),
    case_03: ((10., 20.), (20., 0.), false),
    case_04: ((20., 0.), (10., 20.), true),
}

fn can_identify_cheapest_solution_impl(new_cost: ObjectiveCost, old_cost: ObjectiveCost, expected: bool) {
    let mut refinement_ctx = RefinementContext::new(create_empty_problem());
    refinement_ctx.population.add((create_empty_insertion_context(), old_cost, 0));

    let result = Greedy::default().is_accepted(&refinement_ctx, (&create_empty_insertion_context(), new_cost));

    assert_eq!(result, expected);
}
